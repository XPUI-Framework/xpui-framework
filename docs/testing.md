# Testing a screen without a screen

Four layers. Each catches what the one before it cannot, and the reason there
are four is that each of the first three has shipped a bug the next one found.

| Layer | Asks | Fails when |
|---|---|---|
| **Behaviour** | did `update` do the right thing? | the logic is wrong |
| **Driving it** | does tapping *that row* open *that screen*? | the wiring is wrong |
| **Draw calls** | what was painted, in what order, in what state? | the order or the clipping is wrong |
| **Pixels** | what is actually on the panel? | one pixel moved |

All four run under `cargo test`. None needs hardware, and none opens a window.

```bash
cargo test --workspace --features xpui/testing
```

Every item in `xpui::testing`, with its declaration, is in
[the testing reference](reference/testing.md).

## 1. Behaviour

`update` is an ordinary method. Call it.

```rust
# #[derive(Clone, Copy)] enum Msg { Step(i32) }
# struct Brightness { level: i32 }
# impl Brightness { fn update(&mut self, m: Msg) { match m { Msg::Step(d) => self.level = (self.level + d).clamp(0, 100) } } }
let mut screen = Brightness { level: 40 };
screen.update(Msg::Step(-1));
assert_eq!(screen.level, 39);
```

No host, no harness, no feature flag. Most of what a screen does should be
testable this way, and a screen that is not is usually one keeping state where
`update` cannot see it.

## 2. Driving it the way a person does

The layer worth the most, and the one most often skipped.

[`Ui`](../src/testing/ui/mod.rs) builds an `App` on a real host, renders it, and
finds things **by the text you can see**. `tap_text("Lists")` looks up where
that string was actually drawn and taps those coordinates — so it exercises hit
testing, focus, scrolling and dispatch, exactly as a thumb does.

```rust
# use xpui::host::Host;
# use xpui::testing::{Drive, Ui};
# use xpui::{NavigationScreen, Screen, Text, View, vstack};
# struct Menu;
# impl Menu { fn new() -> Menu { Menu } }
# impl Screen for Menu {
#     type Message = ();
#     fn body(&self) -> impl View<()> { NavigationScreen::new(vstack![0; Text::new("Lists")]) }
#     fn update(&mut self, _message: ()) {}
# }
fn opens_the_list_screen<H: Host + Drive + 'static>(backend: &'static H) {
    let mut ui = Ui::new(Menu::new(), backend);
    ui.tap_text("Lists");
    assert!(ui.visible_text().iter().any(|line| line == "Rows, subtitles, values"));
}
```

Every method `Ui` has, from tapping a label to reading the stack's depth, is in
[the reference](reference/testing.md#testingui).

**Assert on what is on the screen, not on the depth.** `depth() == 2` says
*something* opened. It does not say the right thing did — and that exact
weakness let two menu rows open the wrong example for as long as nobody looked.
`visible_text()` would have failed on the first run.

It is generic over the host, so the same test runs against the fake host or a
real `embedded_graphics` backend; anything implementing `Drive` can be fed
input. The fake host has fixed 480x800 geometry, so a test about a *particular
panel* wants a real backend.

## 3. Draw calls

`xpui::testing` installs a fake host that records every call: rectangles, text,
clips, in order. Whole frames are compared against text goldens in
`tests/snapshots/`.

Text, deliberately. These assert what no image can show — **call order**, clip
lifecycle, and the state a widget was drawn in. A golden small enough to read
in a diff is a golden somebody reviews.

```rust
# use xpui::{Screen, Text, View};
# struct Settings;
# impl Settings { fn new() -> Settings { Settings } }
# impl Screen for Settings {
#     type Message = ();
#     fn body(&self) -> impl View<()> { Text::new("Settings") }
#     fn update(&mut self, _message: ()) {}
# }
# fn main() {
xpui::testing::install();
xpui::testing::reset();
# }
```

## 4. Pixels

Nothing in this repository draws a pixel — it has no backend — so no test
here compares one. `xpui-screenshot` renders to memory and compares the panel
against a committed PNG, pixel for pixel, and it lives beside the backends:
[`xpui-backends`' screenshot guide](https://github.com/XPUI-Framework/xpui-backends/blob/main/embedded_graphics/docs/screenshots.md)
is how such a test is written and a golden accepted, and
[`xpui-gallery`](https://github.com/XPUI-Framework/xpui-gallery) runs every
screen through it on all seven boards.

## Three things that will bite

**The host is process-wide.** `install` writes a static, so tests that install
one must not run concurrently. `Ui` holds a lock for as long as it exists, so
tests built on it take turns.

**A test that cannot fail is worse than none**, because it is counted — and
believed. Before keeping a test, break the thing it covers and watch it go red.
The habit that finds these is mutation, not review.

**Prefer an assertion that pins a relationship over one that pins a number.**
"The label sits at the same offset within every row" catches drift that "the
label is inside its row" cannot.
