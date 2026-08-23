# Writing a backend

You have hardware nobody here supports and you want a screen on it. This is the
path: six traits, in the order you need them, and the obligations the compiler
cannot check.

[`host.md`](host.md) is the contract itself, method by method.
[`reference.md`](reference.md) is the API. This is a route through them.

**Two exist already**, and a third will be one of their two shapes:

| | Implements | Depends on |
|---|---|---|
| `xpui-embedded-graphics` | four traits by hand, `Chrome` from a macro | `xpui`, `xpui-chrome`, `embedded-graphics` |
| `xpui-fui` | five by hand, over a C ABI into C++ | `xpui` and nothing else |

## What you do not have to write

Before the list gets long, three things are not yours:

- **`Chrome`** — the eleven methods that paint a list, a dialog, a slider and
  the rest. `xpui-chrome`'s `plain_chrome!` writes all eleven from the
  primitives you already implemented. Write it yourself only if your host draws
  those components already, as a C++ firmware with its own widget set does.
- **`Navigator`** — who owns the screen stack. That is the *application's*, not
  the backend's, and `xpui::App` supplies it for anything that does not have a
  stack of its own.
- **Anything about boards.** Nothing below the caller knows what a board is.
  Measurements come from the panel's size, and whoever wires your backend
  hands them over.

## 1. `Canvas`, and the one thing everybody gets wrong

Twelve methods, and nothing paints without them. Sizes, rectangles, lines, a
clip, an image, an icon — and text:

```rust
# use xpui::{Point, Rect, Size};
# use xpui::host::{FontId, FontStyle, IconRef};
# struct MyCanvas;
impl MyCanvas {
    fn draw_text(&self, origin: Point, text: &str, font: FontId, style: FontStyle) {
        // `origin` is the **top-left** of the line, not a baseline.
        let ascent = self.ascent_of(font);
        self.device_draw_from_baseline(origin.x, origin.y + ascent, text, style);
    }
#     fn ascent_of(&self, _: FontId) -> i32 { 0 }
#     fn device_draw_from_baseline(&self, _: i32, _: i32, _: &str, _: FontStyle) {}
}
```

Most drawing libraries take a baseline. u8g2 does; `embedded-graphics` will if
you ask. Passing `origin.y` straight through compiles, runs, and puts every
glyph one line too high — and a test that counts draw calls will not notice.
The framework reserved `[y, y + line_height)`; fill that.

`set_clip` is the other one worth care: it must actually clip. The simulator
draws a device body around the panel, and what stops a mis-measured widget
painting over the bezel is the backend honouring the clip rather than the
framework trusting it.

## 2. `TextMetrics`, which must agree with what you just drew

Three methods, and the contract between them and `Canvas` is the whole point:

```rust
# use xpui::host::{FontId, FontRole, FontStyle};
# struct MyMetrics;
# impl MyMetrics {
#     fn face_for(&self, _: FontRole) -> &'static [u8] { b"" }
#     fn hash_of_face_bytes(&self, _: &[u8]) -> i32 { 0 }
fn font(&self, role: FontRole) -> FontId {
    // The id is a hash of the face's own bytes. Not the role, not a counter:
    // a consumer keys a glyph cache on it, so a stable id over changed bytes
    // serves the old face for ever with nothing anywhere to notice.
    FontId(self.hash_of_face_bytes(self.face_for(role)))
}
# }
```

`text_width` must return what `draw_text` will occupy — the advance, summed the
same way, for the same string. A label that measures narrower than it paints
overruns the space reserved for it, and the framework has no way to find out.

**Never estimate.** Measure through the real face. An estimate drifts from what
is painted and pushes content off the panel.

## 3. `Clock`

One method, `millis`. A real clock: key auto-repeat is timed against it, and
answering `0` for ever is a fixed lie rather than a missing feature. Wrapping is
fine — the framework only ever takes differences.

## 4. `InputSource`

Twelve methods, and most of them answer from whatever your event loop last saw:
which buttons were pressed, released, held; a tap; a swipe; the two gestures.

One has no default **on purpose**:

```rust
# struct MyInput;
# impl MyInput {
fn has_left_right_keys(&self) -> bool {
    // Whether this device physically has the pair. A default would let a host
    // forget to answer and inherit a guess — and a value control told the pair
    // exists when it does not cannot be changed by any key.
    false
}
# }
```

`false` is the safe direction, not the accurate one: a control told the keys are
missing can still be entered and left, which costs a keystroke. Told they exist
when they do not, it is unusable.

## 5. Installing it

One `unsafe`, and it is the piece to get right:

```rust
# use xpui::testing::TestHost;
static BACKEND: TestHost = TestHost;
// Safety: one thread, before the first frame, and never while one is running.
unsafe { xpui::host::install(&BACKEND) };
```

**The host is process-wide.** It is written once and read from every frame, on
whatever tasks you have. Install from your entry point before the render loop
starts. In tests, take a shared lock first — `xpui::testing::Ui` holds one for
you — or two tests corrupt each other in ways that look like flakiness.

## 6. Proving it without hardware

You do not need the panel to know the backend is right:

```rust
use xpui::screen::Screen;
use xpui::{App, NavigationScreen, Text, View, testing, vstack};

struct Sleep;

impl Screen for Sleep {
    type Message = ();
    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![14; Text::new("Sleep after")]).title("Sleep timer")
    }
    fn update(&mut self, _message: Self::Message) {}
}

testing::install();
testing::reset();

App::new(Sleep).render();

// Every `draw_text` the host received, in order, as
// (x, y, text, font id, style).
let drawn = testing::drawn_text();
assert!(drawn.iter().any(|(_, _, text, _, _)| text == "Sleep after"));
```

That is the framework's own fake host, and it is what every test here uses. For
pixels, `xpui-screenshot` renders to memory and compares against a committed
PNG — see [`testing.md`](testing.md) for the four layers and, more usefully,
for the eight tests this repository shipped that could not fail.

## Where to look when you are stuck

- [`host.md`](host.md) — every method, and what it must promise
- [`architecture.md`](architecture.md) — how one frame runs, and where you sit
  in it
- `xpui-fui`'s `src/platform.rs` — the cleanest example of a contract written
  where an implementer meets it
