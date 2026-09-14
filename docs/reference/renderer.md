# Drawing and repainting

What a widget or a screen draws with, and when the panel is painted again: the
framebuffer, the header band and the button hints, the clock, and a repaint.
`Renderer` and `ScreenChrome` are façades of associated functions on zero-sized
types, and `millis` and `request_update` are free functions, so nothing threads
a host reference through every call.

[How a façade reaches the host](theme.md#how-a-façade-reaches-the-host) is how each of
these finds the installed host. [Writing a widget](../writing-a-widget.md)
builds a view that draws through `Renderer`. [The host guide](../host.md) is
the other side of these calls: the traits a backend implements to answer them,
listed on [the backend contract](backend-contract.md). This page is what each
piece does.

## Topics

| | |
|---|---|
| [`Renderer`](#renderer) | The framebuffer, as widgets reach for it. |
| [`ScreenChrome`](#screenchrome) | The screen's own furniture: header band and button hints. |
| [`millis`](#millis) | Milliseconds since boot. |
| [`request_update`](#request_update) | Asks for a repaint, which e-ink never does on its own. |

## `Renderer`

The framebuffer, as widgets reach for it.

```text
pub struct Renderer
```

A thin façade over the installed [`Canvas`](canvas-and-chrome.md#hostcanvas), so a
widget writes `Renderer::fill_rect(..)` rather than plumbing a host reference
through every call. Each function forwards to the `Canvas` method of the same
name, and the contract is the backend's: what follows is what a widget can rely
on from any conforming backend.

| Primitive | Behaves |
|---|---|
| [`draw_text`](#rendererdraw_text) | `origin` is the text's **top-left** corner, not a baseline |
| [`fill_rect`](#rendererfill_rect) | `black: false` fills with background, which erases |
| [`stroke_rect`](#rendererstroke_rect) | one pixel wide, inside the rect's bounds |
| [`draw_line`](#rendererdraw_line) | one pixel wide, both ends included |
| [`fill_rect_dither`](#rendererfill_rect_dither) | clears, then a 50% checkerboard that reads as grey |
| [`scrim`](#rendererscrim) | adds ink on one parity only, so what is behind stays legible |
| [`draw_image`](#rendererdraw_image) | 1 bpp, row-major, MSB first, and **bit 0 is ink** |
| [`draw_icon`](#rendererdraw_icon) | at the size [`icon_size`](#renderericon_size) answers, which is 0 for an icon the build lacks |

**Measure with the font, draw with the same font.** A widget that reserved
`Font::text_width` for a label and draws it with the same `Font`'s id and style
fills exactly what it reserved. See [`Font`](text.md#font).

**Example — a custom widget drawing through `Renderer`**

```rust
use xpui::{Font, Point, Rect, Renderer, Size, Theme, ThemeMetric, View, testing};

/// A label in a one-pixel box, padded by the theme's small step.
struct Badge {
    label: &'static str,
    size: Size,
}

impl<M> View<M> for Badge {
    fn measure(&mut self, _available: Size) {
        let font = Font::ui();
        let pad = Theme::metric(ThemeMetric::SpacingSmall);
        self.size = Size::new(
            font.text_width(self.label) + 2 * pad,
            font.line_height() + 2 * pad,
        );
    }

    fn size(&self) -> Size {
        self.size
    }

    fn render(&self, origin: Point) {
        // Always at `origin`: a parent decides where this goes.
        let font = Font::ui();
        let pad = Theme::metric(ThemeMetric::SpacingSmall);
        Renderer::stroke_rect(Rect { origin, size: self.size });
        Renderer::draw_text(origin.offset(pad, pad), self.label, font.id(), font.style());
    }
}

testing::install();
testing::reset();

let mut badge = Badge { label: "New", size: Size::ZERO };
View::<()>::measure(&mut badge, Size::new(200, 100));
View::<()>::render(&badge, Point::new(20, 30));

let pad = testing::SPACING_SMALL;
let rects = testing::drawn_rects();
assert_eq!((rects[0].0, rects[0].1), (20, 30));
let text = testing::drawn_text();
assert_eq!((text[0].0, text[0].1), (20 + pad, 30 + pad));
assert_eq!(text[0].2, "New");
```

**Example — keeping an overflow off the chrome**

```rust
use xpui::{Rect, Renderer, Theme, testing};

testing::install();
testing::reset();

let viewport = Theme::content_area();
Renderer::clip(viewport);
// Taller than the viewport: only the part inside it reaches the panel.
Renderer::fill_rect(Rect::new(viewport.x(), 0, viewport.width(), 2000), true);
Renderer::clear_clip();

assert_eq!(testing::clips(), vec![Some(viewport), None]);
```

### Reading the panel

#### `Renderer::screen_size`

See [`Canvas::screen_size`](canvas-and-chrome.md#hostcanvasscreen_size).

```text
pub fn screen_size() -> Size
```

In logical pixels, in the current orientation, so it changes when the device is
turned. Never assume it.

#### `Renderer::screen_bounds`

The whole panel, as a rect at the origin.

```text
pub fn screen_bounds() -> Rect
```

**Example — the panel as a rect**

```rust
use xpui::{Point, Renderer, testing};

testing::install();

let panel = Renderer::screen_bounds();
assert_eq!(panel.origin, Point::ORIGIN);
assert_eq!(panel.size, Renderer::screen_size());
assert!(!panel.contains(Point::new(panel.width(), 0)));
```

### Clearing

#### `Renderer::clear`

See [`Canvas::clear`](canvas-and-chrome.md#hostcanvasclear).

```text
pub fn clear()
```

Clears the whole panel to background. The runtime calls it before painting
every screen that is not an overlay; a view painting over what is already there
must not.

### Drawing shapes

#### `Renderer::fill_rect`

See [`Canvas::fill_rect`](canvas-and-chrome.md#hostcanvasfill_rect).

```text
pub fn fill_rect(rect: Rect, black: bool)
```

| Parameter | Meaning |
|---|---|
| `rect` | The area to fill. |
| `black` | `true` for ink, `false` for background. |

#### `Renderer::stroke_rect`

See [`Canvas::stroke_rect`](canvas-and-chrome.md#hostcanvasstroke_rect).

```text
pub fn stroke_rect(rect: Rect)
```

#### `Renderer::draw_line`

See [`Canvas::draw_line`](canvas-and-chrome.md#hostcanvasdraw_line).

```text
pub fn draw_line(from: Point, to: Point)
```

A rect's [`right`](geometry.md#rectright) and [`bottom`](geometry.md#rectbottom)
are outside it, so a rule along the bottom row of `rect` runs at
`rect.bottom() - 1`.

**Example — a rule under a row**

```rust
use xpui::{Point, Rect, Renderer, testing};

testing::install();
testing::reset();

let row = Rect::new(16, 60, 448, 40);
let y = row.bottom() - 1; // the last row inside
Renderer::draw_line(Point::new(row.x(), y), Point::new(row.right() - 1, y));

assert_eq!(testing::ops_log().len(), 1);
```

#### `Renderer::fill_rect_dither`

See [`Canvas::fill_rect_dither`](canvas-and-chrome.md#hostcanvasfill_rect_dither).

```text
pub fn fill_rect_dither(rect: Rect, light: bool)
```

| Parameter | Meaning |
|---|---|
| `light` | Which checkerboard parity takes ink, so two dithers side by side can differ. |

#### `Renderer::scrim`

See [`Canvas::scrim`](canvas-and-chrome.md#hostcanvasscrim).

```text
pub fn scrim(rect: Rect)
```

Unlike [`fill_rect_dither`](#rendererfill_rect_dither), which clears first,
this only adds ink, so it pushes a background back behind an overlay without
repainting it. [`OverlayPanel`](navigation.md#overlaypanel) uses it.

### Drawing text and pictures

#### `Renderer::draw_text`

See [`Canvas::draw_text`](canvas-and-chrome.md#hostcanvasdraw_text).

```text
pub fn draw_text(origin: Point, text: &str, font: FontId, style: FontStyle)
```

| Parameter | Meaning |
|---|---|
| `origin` | The **top-left** corner of the line. The line occupies `origin.y` to `origin.y + line_height`. |
| `text` | What to draw, on one line. |
| `font` | The face, from [`Font::id`](text.md#fontid). |
| `style` | The weight and slant, from `Font::style`. |

#### `Renderer::draw_image`

See [`Canvas::draw_image`](canvas-and-chrome.md#hostcanvasdraw_image).

```text
pub fn draw_image(origin: Point, data: &[u8], size: Size)
```

| Parameter | Meaning |
|---|---|
| `origin` | The image's top-left corner. |
| `data` | `(width + 7) / 8` bytes per row, row-major, MSB first, bit 0 ink. |
| `size` | The image's width and height in pixels. |

[`Image`](images.md#image) wraps this.

#### `Renderer::draw_icon`

See [`Canvas::draw_icon`](canvas-and-chrome.md#hostcanvasdraw_icon).

```text
pub fn draw_icon(origin: Point, icon: IconRef)
```

#### `Renderer::icon_size`

See [`Canvas::icon_size`](canvas-and-chrome.md#hostcanvasicon_size).

```text
pub fn icon_size(icon: IconRef) -> i32
```

The host picks the nearest size it ships to `icon.size`. Reserve what this
answers, not what was asked for, and draw nothing when it is 0.

**Example — reserving an icon's real size**

```rust
use xpui::{IconRef, Point, Renderer, testing};

testing::install();

let icon = IconRef::new(3);
let edge = Renderer::icon_size(icon);
if edge > 0 {
    Renderer::draw_icon(Point::new(16, 60), icon);
}
assert_eq!(edge, icon.size); // the test host ships every size
```

### Clipping

#### `Renderer::clip`

Confines drawing to `rect`, inside whatever clip is already set.

```text
pub fn clip(rect: Rect)
```

Pair every call with [`Renderer::clear_clip`](#rendererclear_clip). [`ScrollView`](layout.md#scrollview)
clips to its viewport so content taller than the space it was given stays off
the header and the hints.

**Clips nest.** The host is given `rect` intersected with the clip in force, so
a clipping widget inside a `ScrollView` cannot draw past either, and clearing
its clip gives the scroll view's back. A rect that misses the outer clip
entirely confines drawing to nothing.

| Nesting | What the host is given |
|---|---|
| no clip set | `rect` |
| inside another clip | `rect` intersected with that clip |
| past eight deep | `rect` intersected with the eighth; clearing it restores the eighth |

The stack is fixed at eight and allocates nothing.

**Example — a clip inside a clip**

```rust
use xpui::{Rect, Renderer, testing};

testing::install();
testing::reset();

let viewport = Rect::new(0, 60, 480, 700);
Renderer::clip(viewport);
Renderer::clip(Rect::new(16, 40, 200, 100)); // pokes above the viewport
Renderer::clear_clip();
Renderer::clear_clip();

assert_eq!(
    testing::clips(),
    vec![
        Some(viewport),
        Some(Rect::new(16, 60, 200, 80)), // only the part inside the viewport
        Some(viewport),                   // the outer clip, given back
        None,
    ]
);
```

#### `Renderer::clear_clip`

Lifts the innermost clip, restoring the one it was set inside.

```text
pub fn clear_clip()
```

With no clip outside it, the host's clip is lifted entirely. A call with no clip
set lifts the host's clip and is otherwise ignored.

**See also:** [`Theme`](theme.md#theme), [`Canvas`](canvas-and-chrome.md#hostcanvas), [`View`](views.md#view), [geometry](geometry.md)

## `ScreenChrome`

The screen's own furniture: header band and button hints.

```text
pub struct ScreenChrome
```

**The screen roots call it.** [`NavigationScreen`](navigation.md#navigationscreen)
draws the header and the hints around its content, so a screen built on one
never calls `ScreenChrome`. A root of your own, a view that frames a whole
screen, calls it from `render`.

The hints are given by meaning, never by position: the host puts `back`,
`confirm`, `previous` and `next` under whichever buttons the user has mapped
to them. Each is a [`Hint`](navigation.md#hint).

**Example — a root drawing its own frame**

```rust
use xpui::{Hint, Point, Renderer, ScreenChrome, Size, View, testing};

/// A full-screen frame: the header, then the hints, with nothing between.
struct Frame;

impl<M> View<M> for Frame {
    fn measure(&mut self, _available: Size) {}

    fn size(&self) -> Size {
        Renderer::screen_size()
    }

    fn render(&self, _origin: Point) {
        Renderer::clear();
        ScreenChrome::draw_screen_header();
        ScreenChrome::draw_button_hints(
            &Hint::Standard,
            &Hint::text("Save"),
            &Hint::None,
            &Hint::None,
        );
    }
}

testing::install();
testing::reset();

View::<()>::render(&Frame, Point::ORIGIN);

// The test host's navigator calls every screen "Test".
assert_eq!(testing::drawn_headers(), vec![Some("Test".to_string())]);
assert_eq!(testing::drawn_hints().len(), 1);
```

### Drawing the header

#### `ScreenChrome::draw_header`

Draws the header with an explicit title.

```text
pub fn draw_header(title: &str)
```

The band includes whatever the host puts in it, a battery indicator for one.

#### `ScreenChrome::draw_screen_header`

Draws the header with the screen's own translated title, fetched here rather than passed as `None`: to a `Chrome` implementation `None` means no title, and the band comes out empty.

```text
pub fn draw_screen_header()
```

The title comes from [`screen_title`](#screenchromescreen_title). With no
navigator installed that is an empty string, so the band is drawn with an empty
title.

#### `ScreenChrome::screen_title`

The running screen's own title, from the installed [`Navigator`](navigation.md#navigator).

```text
pub fn screen_title() -> &'static str
```

Under [`App`](app.md#app), the title the top screen returns from
`Screen::title`. A C++ host answers from its own activity manager.

**Example — a title in the header**

```rust
use xpui::{ScreenChrome, testing};

testing::install();
testing::reset();

ScreenChrome::draw_header("Wi-Fi");
ScreenChrome::draw_screen_header();

assert_eq!(
    testing::drawn_headers(),
    vec![Some("Wi-Fi".to_string()), Some(ScreenChrome::screen_title().to_string())],
);
```

### Drawing the hints

#### `ScreenChrome::draw_button_hints`

See [`Chrome::draw_button_hints`](canvas-and-chrome.md#hostchromedraw_button_hints).

```text
pub fn draw_button_hints(back: &Hint, confirm: &Hint, previous: &Hint, next: &Hint)
```

| Parameter | Meaning |
|---|---|
| `back` | The slot over Back. |
| `confirm` | The slot over Confirm. |
| `previous` | The slot over the key that moves up or back. |
| `next` | The slot over the key that moves down or forward. |

`Hint::Standard` asks for the host's own translated label, and `Hint::None`
leaves the slot blank. While a value control is open, the runtime replaces the
Back and Confirm slots itself: see [the value mode](controls.md#the-value-mode).

**See also:** [`NavigationScreen`](navigation.md#navigationscreen), [`Hint`](navigation.md#hint), [`Navigator`](navigation.md#navigator)

## `millis`

Milliseconds since boot.

```text
pub fn millis() -> u32
```

From the installed `Clock`, which is monotonic. It is a `u32`, so it wraps
after about 49.7 days: compare two readings with `wrapping_sub`, never with
`-`, and a timeout still fires across the wrap. How often it advances is the
host's business.

**Nothing arrives to say time has passed.** A timeout, a countdown or an
auto-refresh reads `millis` in [`Screen::tick`](screens.md#screentick), which
runs once per frame whether or not any input arrived, and calls
[`request_update`](#request_update) when what the screen shows has changed.

**Example — a message that disappears after three seconds**

```rust
use xpui::{Screen, Text, View, millis, request_update, testing};

const SHOWN_MS: u32 = 3_000;

struct Saved {
    shown_at: Option<u32>,
}

impl Screen for Saved {
    type Message = ();

    fn tick(&mut self) {
        if let Some(at) = self.shown_at
            && millis().wrapping_sub(at) >= SHOWN_MS
        {
            self.shown_at = None;
            request_update(); // the panel still shows "Saved"
        }
    }

    fn body(&self) -> impl View<()> {
        Text::new(if self.shown_at.is_some() { "Saved" } else { "" })
    }

    fn update(&mut self, _message: ()) {}
}

testing::install();
testing::reset();

// Shown just before the counter wraps.
let mut screen = Saved { shown_at: Some(u32::MAX - 1_000) };

testing::set_millis(u32::MAX);
screen.tick();
assert_eq!(testing::updates(), 0, "one second in");

testing::set_millis(2_500); // wrapped: 3.5 seconds in
screen.tick();
assert_eq!(screen.shown_at, None);
assert_eq!(testing::updates(), 1);
```

**See also:** [`request_update`](#request_update), [`Screen::tick`](screens.md#screentick), [`Clock`](backend-contract.md#hostclock)

## `request_update`

Asks for a repaint, which e-ink never does on its own.

```text
pub fn request_update()
```

It marks the frame dirty for [`App`](app.md#app) and passes the request on
to the host's `Chrome`. Several requests before the next paint are one paint.
A request made during a paint is for the frame after it.

| From | Call it? |
|---|---|
| a screen's `update` | No: the runtime repaints after every message it dispatches. |
| `Screen::tick`, or anything that changes what shows without a message | Yes. |
| pushing or finishing a screen | No: navigation is a repaint by definition. |

> [!WARNING]
> An e-ink refresh can take a second or more. Call this when
> what the screen shows has changed, not on every tick.

**Example — one request, one paint**

```rust
use xpui::{App, Screen, Text, View, request_update, testing};

struct Clock;

impl Screen for Clock {
    type Message = ();

    fn body(&self) -> impl View<()> {
        Text::new("12:00")
    }

    fn update(&mut self, _message: ()) {}
}

testing::install();
let mut app = App::new(Clock);
app.render();
assert!(!app.is_dirty());

request_update();
request_update();
assert!(app.is_dirty());
assert!(app.render_if_dirty());
assert!(!app.render_if_dirty(), "two requests, one paint");
```

**See also:** [`millis`](#millis), [`App::render_if_dirty`](app.md#apprender_if_dirty), [`Chrome::request_update`](canvas-and-chrome.md#hostchromerequest_update)
