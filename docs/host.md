# The host contract

`xpui` draws nothing by itself. It describes what it needs through five traits in
[`src/host/`](../src/host/); you implement them and install the result once.

The crates under [`crates/backend/`](../../backend/) implement these against
real drawing substrates. This page describes what each trait owes `xpui` —
worth reading if you are changing one of those, or writing a new one.

## The five traits a backend implements

| Trait | You provide | File |
|---|---|---|
| `Canvas` | Fill and stroke rectangles, draw text, lines, bitmaps, icons | [canvas.rs](../src/host/canvas.rs) |
| `TextMetrics` | Width and line height for a string in a font | [metrics.rs](../src/host/metrics.rs) |
| `Chrome` | Header, button hints, list rows, dialogs, and "repaint please" — your theme | [chrome.rs](../src/host/chrome.rs) |
| `InputSource` | Buttons, taps, drags, gestures for one frame | [input.rs](../src/host/input.rs) |
| `Clock` | Milliseconds since boot | [clock.rs](../src/host/clock.rs) |

Implement all five on one type and it satisfies `Host` automatically:

```rust,no_run
# use xpui::host::{
#     Button, Canvas, Chrome, Clock, FontId, FontRole, FontStyle, Hint, IconRef, InputSource,
#     RowField, SwipeDir, TextMetrics, ThemeMetric,
# };
# use xpui::{Point, Rect, Size};
pub struct MyBackend;

impl Canvas for MyBackend {
#   fn screen_size(&self) -> Size { Size::new(480, 800) }
#   fn clear(&self) {}
#   fn draw_text(&self, _at: Point, _text: &str, _font: FontId, _style: FontStyle) {}
#   fn fill_rect(&self, _rect: Rect, _black: bool) {}
#   fn stroke_rect(&self, _rect: Rect) {}
#   fn draw_line(&self, _from: Point, _to: Point) {}
#   fn fill_rect_dither(&self, _rect: Rect, _light: bool) {}
#   fn scrim(&self, _rect: Rect) {}
#   fn set_clip(&self, _rect: Option<Rect>) {}
#   fn draw_image(&self, _at: Point, _data: &[u8], _size: Size) {}
#   fn draw_icon(&self, _at: Point, _icon: IconRef) {}
#   fn icon_size(&self, _icon: IconRef) -> i32 { 0 }
    /* ... */
}

impl TextMetrics for MyBackend {
#   fn font(&self, _role: FontRole) -> FontId { FontId::UNAVAILABLE }
#   fn text_width(&self, _font: FontId, _text: &str, _style: FontStyle) -> i32 { 0 }
#   fn line_height(&self, _font: FontId) -> i32 { 0 }
    /* ... */
}

impl Chrome for MyBackend {
#   fn metric(&self, _metric: ThemeMetric) -> i32 { 0 }
#   fn draw_header(&self, _title: Option<&str>, _subtitle: Option<&str>) {}
#   fn draw_sub_header(&self, _rect: Rect, _label: &str, _right: Option<&str>) {}
#   fn draw_button_hints(&self, _back: &Hint, _confirm: &Hint, _prev: &Hint, _next: &Hint) {}
#   fn draw_progress_bar(&self, _rect: Rect, _current: u32, _total: u32) {}
#   fn draw_slider(&self, _rect: Rect, _value: i32, _max: i32) {}
#   fn draw_scroll_indicator(&self, _r: Rect, _content: i32, _visible: i32, _offset: i32) {}
#   fn draw_list<'a>(
#       &self,
#       _rect: Rect,
#       _rows: usize,
#       _selected: i32,
#       _row: &dyn Fn(usize, RowField) -> Option<&'a str>,
#   ) {}
#   fn draw_option_popup<'a>(
#       &self,
#       _title: &str,
#       _options: &dyn Fn(usize) -> Option<&'a str>,
#       _count: usize,
#       _selected: i32,
#   ) {}
#   fn option_popup_row_rect<'a>(
#       &self,
#       _title: &str,
#       _options: &dyn Fn(usize) -> Option<&'a str>,
#       _count: usize,
#       _index: usize,
#   ) -> Option<Rect> { None }
#   fn request_update(&self) {}
    /* ... */
}

impl InputSource for MyBackend {
#   fn was_pressed(&self, _button: Button) -> bool { false }
#   fn is_pressed(&self, _button: Button) -> bool { false }
#   fn was_released(&self, _button: Button) -> bool { false }
#   fn has_touch(&self) -> bool { false }
#   fn tap(&self) -> Option<Point> { None }
#   fn touch_held(&self) -> Option<Point> { None }
#   fn touch_released(&self) -> bool { false }
#   fn swipe(&self) -> SwipeDir { SwipeDir::None }
#   fn was_back_gesture(&self) -> bool { false }
#   fn was_home_gesture(&self) -> bool { false }
    /* ... */
}

impl Clock for MyBackend {
#   fn millis(&self) -> u32 { 0 }
    /* ... */
}

static BACKEND: MyBackend = MyBackend;

// `install` is unsafe: it must run before the first measure, render or
// interactions pass, and never alongside one.
unsafe { xpui::host::install(&BACKEND) };
```

A compile-time assertion is worth adding so a missing trait is caught at the
definition rather than at the install site:

```rust
# use xpui::testing::TestHost as MyBackend;
const _: fn() = || {
    fn assert_host<T: xpui::host::Host>() {}
    assert_host::<MyBackend>();
};
```

## The sixth trait, which is not a backend's job

`Navigator` — [navigator.rs](../src/host/navigator.rs) — answers "what is this
screen called" and "go back". Those depend on who owns the screen stack, which
is a different question from what paints the pixels, so it is installed
separately:

```rust
# static SHELL: xpui::testing::TestHost = xpui::testing::TestHost;
unsafe { xpui::host::install_navigator(&SHELL) };
```

A backend crate does **not** implement it. Either the application does, or
[`App`](../src/app.rs) does it for you — `App::new` installs itself. A C++
firmware whose own activity manager owns the stack implements it over the FFI.

Note what deliberately stayed on `Chrome`: `request_update`. A repaint is a
display concern, anything that can paint can ask to paint again, and the
framework calls it on every dispatch — so a host that forgot to install a
navigator gets a dead Back button, not a screen that never refreshes.

## Things that are easy to get wrong

**Install before anything runs.** That is the safety contract on `install`, not
a style note. Rendering and input run on different tasks on the device, so the
lifecycle installs from both entry points rather than assuming which wakes first.

**`Chrome` is your theme, and `xpui` has no opinion about it.** The framework
never decides what a list row looks like; it asks, and you answer. A backend
sitting on a component library — FreeInkUI, say — answers by calling that
library, so a screen written here and a native one are the same pixels.

That leaves a backend sitting on a *drawing* library with eight components to
paint and no toolkit to paint them with. It does not have to write them: the
`chrome` backend paints all eight from `Canvas` and `TextMetrics` alone, so
such a backend implements `Canvas`, `TextMetrics`, `InputSource` and `Clock`,
and takes `Chrome` from there. Shared implementation, not a default — you still
choose it explicitly.

**Ask for metrics honestly.** `TextMetrics` must reflect the font you will
actually paint with. If it does not, everything measures correctly and draws
wrongly.

**Icons are roles, not files.** `Canvas::draw_icon` receives an opaque number
meaning "the thing you use for *sun*", and you choose the asset. That keeps
asset names out of the framework.

**Report zero rather than guessing.** If a font or icon is missing from a build,
return `0` for its size. `xpui` then draws nothing, rather than painting garbage
at an arbitrary size.

## Testing without hardware

`xpui` ships a fake host behind the `testing` feature:

```toml
[dev-dependencies]
xpui = { workspace = true, features = ["testing"] }
```

```rust
xpui::testing::install();
```

It reports fixed screen and theme dimensions and records everything drawn, so
you can assert on layout and touch behaviour in an ordinary `cargo test`. The
framework's own tests use nothing else.

## Worked examples

The crates under [`crates/backend/`](../../backend/) are the real ones, and
they are deliberately different shapes:

| Backend | Satisfies | How |
|---|---|---|
| `embedded_graphics` | all five | `Canvas` and `TextMetrics` over a `DrawTarget`; `InputSource` and `Clock` from what the frame loop feeds it; `Chrome` from `chrome` |
| `fui` | all five | over an FFI boundary, into C++ FreeInkUI |
| `chrome` | `Chrome` | from `Canvas` primitives, for backends that have no toolkit |

`xpui`'s own `unsafe` is confined to three places: installing and reading the
host globals, the single-threaded cells `App` keeps for its navigator, and the
testing doubles. A backend crossing an FFI boundary holds the rest.
