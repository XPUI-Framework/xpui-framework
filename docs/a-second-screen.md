# A second screen

[`tutorial.md`](tutorial.md) took you from an empty file to one working screen.
This is the next four things, in the order you will want them: a list, a screen
that opens another, a page taller than the panel, and a widget of your own.

Everything below runs with no backend at all — `xpui::testing` is a host that
draws into memory and records what it drew. Put a window under it whenever you
like; you will not need one to follow this.

`gallery/` in the [gallery repository](https://github.com/XPUI-Framework/xpui-gallery)
is the worked version of all of it, running on seven panels.

## 1. A list is not a stack of rows

The obvious thing is a `VStack` of `Text`s. It will look close and behave
wrong: no selection marker, no scroll indicator, and rows that do not match
what the rest of the device paints.

A `List` goes through the backend's own `Chrome::draw_list`, so a row here and
a row in a screen somebody else wrote are the same row:

```rust
use xpui::screen::Screen;
use xpui::{App, List, ListRow, NavigationScreen, View, testing};

#[derive(Clone, Copy)]
enum Message {
    Open(usize),
}

struct Settings;

impl Screen for Settings {
    type Message = Message;

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(
            List::new()
                .push(ListRow::new("Wi-Fi").value("Off").on_tap(Message::Open(0)))
                .push(ListRow::new("Storage").subtitle("3.1 GB free").on_tap(Message::Open(1))),
        )
        .title("Settings")
    }

    fn update(&mut self, _message: Self::Message) {}
}

testing::install();
testing::reset();
App::new(Settings).render();

// One entry per list drawn, each holding that list's rows.
let lists = testing::drawn_list_rows();
assert_eq!(lists.len(), 1, "one list");
assert_eq!(lists[0].len(), 2, "and the theme drew both of its rows");

// Each row is [title, subtitle, value] — which is the paragraph below, made
// checkable. A count alone would pass with the two swapped.
assert_eq!(lists[0][0][0].as_deref(), Some("Wi-Fi"));
assert_eq!(lists[0][0][2].as_deref(), Some("Off"));
assert_eq!(lists[0][1][1].as_deref(), Some("3.1 GB free"));
```

Three things worth knowing:

- **`value` is the right-hand text**, and `subtitle` is the second line. A row
  with a subtitle is taller, and the theme decides by how much — you do not.
- **`on_tap` takes your message**, not a closure. The runtime delivers it to
  `update` when the row is chosen, whether by a finger or by a key.
- **Rows are pulled at paint time, not handed over.** The theme asks for cell
  *n* through a callback rather than being given a buffer, so nothing is
  copied for the backend. The rows themselves are not free: `ListRow::new`
  owns its `String`, and building the list inside `body()` allocates one per
  cell on every frame. §4 is about that.

## 2. Opening a screen, and coming back

Navigation is not a function you call on a parent. A screen asks the
application to `present` another, and the application owns the stack:

```rust
# use xpui::screen::Screen;
# use xpui::{List, ListRow, NavigationScreen, Text, View, present, vstack};
# #[derive(Clone, Copy)] enum Message { Open }
struct Detail;
# impl Screen for Detail {
#     type Message = ();
#     fn body(&self) -> impl View<()> {
#         NavigationScreen::new(vstack![0; Text::new("Wi-Fi")]).title("Wi-Fi")
#     }
#     fn update(&mut self, _: ()) {}
# }

# struct Settings;
# impl Screen for Settings {
#     type Message = Message;
#     fn body(&self) -> impl View<Message> {
#         NavigationScreen::new(List::new().push(ListRow::new("Wi-Fi").on_tap(Message::Open)))
#     }
fn update(&mut self, message: Message) {
    match message {
        Message::Open => {
            present(Detail);
        }
    }
}
# }
```

That is the same shape `xpui-gallery`'s `gallery/src/menu.rs` uses for all seven of its
examples.

### `Back` means four things, in order

This is the one that catches people, and it caught this repository:

1. **A screen claims it.** `Screen::on_key` is asked first, for Back as for any key.
2. **Leave a value being edited.** A stepper that is open cancels.
3. **Close a dialog.** An open picker sends its `on_dismiss`, or swallows the
   key when it has none.
4. **Pop the stack.** Nothing above claimed it, so the screen finishes.

The system back gesture is the same Back, and goes through the same four.

A firmware once fixed a root screen finishing — pressing Back on the first
screen and ending the app — by suppressing the key at the pin. That killed the
other meanings on every host: a screen could not dismiss its own picker,
and a value opened on a root screen could be committed but never cancelled.

The answer is one call, and it is not discoverable from the reference:

```rust
# use xpui::screen::Screen;
# use xpui::{App, Button, NavigationScreen, Text, View, testing, vstack};
# struct Menu;
# impl Screen for Menu {
#     type Message = ();
#     fn body(&self) -> impl View<()> { NavigationScreen::new(vstack![0; Text::new("Menu")]) }
#     fn update(&mut self, _: ()) {}
# }
# testing::install();
# testing::reset();
// Back still arrives everywhere. The root simply declines to finish.
let mut app = App::new(Menu).keep_root();
testing::press(Button::Back);
app.tick();
assert!(app.is_running(), "the root kept it");

// Without it, the same key ends the application.
let mut plain = App::new(Menu);
testing::press(Button::Back);
plain.tick();
assert!(!plain.is_running(), "and this is what a firmware went to the pin over");
```

## 3. A page taller than the panel

Wrap it in a `ScrollView`. The runtime keeps the offset beside focus, so moving
focus off the bottom scrolls on its own:

```rust
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use xpui::screen::Screen;
use xpui::{App, NavigationScreen, ScrollView, Text, View, testing, vstack};

struct About {
    lines: Vec<String>,
}

impl About {
    fn new() -> Self {
        // Built once, when the screen is. `body` runs on every frame.
        Self { lines: (1..=40).map(|n| alloc::format!("Line {n}")).collect() }
    }
}

impl Screen for About {
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        let mut lines = vstack![10];
        // Taller than the panel on purpose. Content that fits scrolls no
        // further than content with no ScrollView around it, so a test built
        // on it cannot tell the two apart.
        for line in &self.lines {
            lines = lines.push(Text::new(line.as_str()));
        }
        NavigationScreen::new(ScrollView::new(lines)).title("About")
    }

    fn update(&mut self, _message: Self::Message) {}
}

testing::install();
testing::reset();
App::new(About::new()).render();

// The theme is told what to draw a thumb from: how tall the content is, how
// much of it fits, and where the offset sits. Nothing reports this but a
// ScrollView, so removing the wrapper empties the vector.
let indicators = testing::drawn_indicators();
assert_eq!(indicators.len(), 1, "one ScrollView reports one thumb; nothing else reports any");
let (content, visible, offset) = indicators[0];
assert!(content > visible, "{content} of content, {visible} of panel");
assert_eq!(offset, 0, "and it opens at the top");
```

**One `ScrollView` per screen.** The offset lives in the runtime beside focus,
so a second one would share the first one's position.

A `ScrollView` measures its content against `UNBOUNDED` height — a large
sentinel rather than `i32::MAX`, because several views echo the height they
were offered and three of them added together must not wrap into a negative.
That is why layout arithmetic here saturates, and it is worth knowing before
you write a view that reports its own height.

## 4. Your own widget

When a `List`, a `Stepper`, a `Toggle` and a `Modal` are not the thing you
need, write a `View`. That is a trait with three required
methods — `measure`, `size` and `render` — and
[`writing-a-widget.md`](writing-a-widget.md) is the walk through them,
including the one about tap targets that a stack-depth test cannot catch.

Before you do, one rule that bites hardest here:

**Keep `format!` off `body()`.** It runs on every repaint *and* every frame
carrying input, so a `format!` in it allocates while a finger is dragging. Build
the `String` in `update`, when the value actually changes. `Slider` and
`Stepper` format into a stack buffer for exactly this reason — see
[`reference.md`](reference.md) for what they do instead.

## Where next

- [`reference.md`](reference.md) — every widget, modifier and host trait
- [`writing-a-widget.md`](writing-a-widget.md) — adding to the framework
- [`testing.md`](testing.md) — the four layers, and eight tests this repository
  shipped that could not fail
- [`architecture.md`](architecture.md) — how one frame actually runs
