# The backend contract

What `xpui` needs from whatever hosts it, as five traits and the functions that
install them. The framework never talks to a backend directly: a host
implements [`host::Canvas`](canvas-and-chrome.md#hostcanvas),
[`host::TextMetrics`](canvas-and-chrome.md#hosttextmetrics),
[`host::Chrome`](canvas-and-chrome.md#hostchrome),
[`host::InputSource`](#hostinputsource) and
[`host::Clock`](#hostclock) on one type, which makes it a
[`host::Host`](#hosthost), and hands a `'static` reference to
[`host::install`](#hostinstall) before the first frame. None of these names is
re-exported at the crate root, so a backend writes `xpui::host::Canvas`, and a
screen never has a reason to.

[The host contract](../host.md) explains what each trait owes the framework and
what is easy to get wrong. [Writing a backend](../writing-a-backend.md) walks
through building one, a trait at a time. This page is the `Host` object, input,
the clock and installation, method by method; the three drawing traits are
[Canvas and chrome](canvas-and-chrome.md). The navigation trait a shell
installs beside the host is [`Navigator`](navigation.md#navigator).

## Topics

| | |
|---|---|
| [`host::Host`](#hosthost) | Everything a backend must provide, as one object implementing all five traits. |
| [`host::InputSource`](#hostinputsource) | One frame of input, and what the device it came from can do. |
| [`host::Clock`](#hostclock) | The host's monotonic clock. |
| [`host::install`](#hostinstall) | Installs the host, before any view is measured or drawn. |
| [`host::is_installed`](#hostis_installed) | Whether a host has been installed, so tests can assert wiring. |
| [`host::install_navigator`](#hostinstall_navigator) | Installs the navigator, before the first frame. |
| [`host::is_navigator_installed`](#hostis_navigator_installed) | Whether a navigator has been installed, so a host can assert its wiring and stay idempotent across two entry points. |

## Who implements what

| Implementation | Canvas, TextMetrics, InputSource, Clock | Chrome |
|---|---|---|
| `xpui-embedded-graphics` | by hand, over a `DrawTarget` | from `xpui_chrome::plain_chrome!`, painted with its own `Canvas` |
| `xpui-fui` | by hand, across a C ABI into FreeInkUI | by hand, the same way |
| the fake host, `xpui::testing::TestHost` | by hand, recording every call | by hand, recording every call |

Every method takes `&self`. The framework holds one `&'static dyn Host` and
calls it from wherever a frame runs, so a host that keeps per-frame state keeps
it behind interior mutability, and the whole type must be `Sync`.

## `host::Host`

Everything a backend must provide, as one object implementing all five traits.

```text
pub trait Host: Canvas + TextMetrics + Chrome + InputSource + Clock + Sync
```

A host installs a single value and the framework keeps one pointer. Nothing
implements `Host` by hand: a blanket implementation covers every `Sync` type
that implements the five, so a missing method shows up as a missing trait.

### Required methods

None of its own. Every method belongs to one of the five supertraits.

### Provided methods

None.

**Example — a skeleton host**

Every method answers "nothing": no fonts, no icons, no input, and paint that
goes nowhere. It is the shape a new backend starts from, and it installs.

```rust
use xpui::host::{
    self, Button, Canvas, Chrome, Clock, ControlState, FontId, FontRole, FontStyle, Hint,
    IconRef, InputSource, RowField, TextMetrics, ThemeMetric,
};
use xpui::{Point, Rect, Size, SwipeDir};

struct Skeleton;

impl Canvas for Skeleton {
    fn screen_size(&self) -> Size { Size::new(480, 800) }
    fn clear(&self) {}
    fn draw_text(&self, _origin: Point, _text: &str, _font: FontId, _style: FontStyle) {}
    fn fill_rect(&self, _rect: Rect, _black: bool) {}
    fn stroke_rect(&self, _rect: Rect) {}
    fn draw_line(&self, _from: Point, _to: Point) {}
    fn fill_rect_dither(&self, _rect: Rect, _light: bool) {}
    fn scrim(&self, _rect: Rect) {}
    fn set_clip(&self, _rect: Option<Rect>) {}
    fn draw_image(&self, _origin: Point, _data: &[u8], _size: Size) {}
    fn draw_icon(&self, _origin: Point, _icon: IconRef) {}
    fn icon_size(&self, _icon: IconRef) -> i32 { 0 }
}

impl TextMetrics for Skeleton {
    fn font(&self, _role: FontRole) -> FontId { FontId::UNAVAILABLE }
    fn text_width(&self, _font: FontId, _text: &str, _style: FontStyle) -> i32 { 0 }
    fn line_height(&self, _font: FontId) -> i32 { 0 }
}

impl Chrome for Skeleton {
    fn metric(&self, _metric: ThemeMetric) -> i32 { 0 }
    fn draw_header(&self, _title: Option<&str>, _subtitle: Option<&str>) {}
    fn draw_sub_header(&self, _rect: Rect, _label: &str, _right_label: Option<&str>) {}
    fn draw_button_hints(&self, _back: &Hint, _confirm: &Hint, _previous: &Hint, _next: &Hint) {}
    fn draw_progress_bar(&self, _rect: Rect, _current: u32, _total: u32) {}
    fn draw_slider(&self, _rect: Rect, _value: i32, _max: i32, _state: ControlState) {}
    fn draw_scroll_indicator(&self, _rect: Rect, _content: i32, _visible: i32, _offset: i32) {}
    fn draw_list<'a>(
        &self,
        _rect: Rect,
        _rows: usize,
        _selected: i32,
        _row: &dyn Fn(usize, RowField) -> Option<&'a str>,
    ) {
    }
    fn draw_option_popup<'a>(
        &self,
        _title: &str,
        _options: &dyn Fn(usize) -> Option<&'a str>,
        _count: usize,
        _selected: i32,
    ) {
    }
    fn option_popup_row_rect<'a>(
        &self,
        _title: &str,
        _options: &dyn Fn(usize) -> Option<&'a str>,
        _count: usize,
        _index: usize,
    ) -> Option<Rect> {
        None
    }
    fn request_update(&self) {}
}

impl InputSource for Skeleton {
    fn was_pressed(&self, _button: Button) -> bool { false }
    fn is_pressed(&self, _button: Button) -> bool { false }
    fn was_released(&self, _button: Button) -> bool { false }
    fn has_touch(&self) -> bool { false }
    fn has_left_right_keys(&self) -> bool { false }
    fn tap(&self) -> Option<Point> { None }
    fn touch_held(&self) -> Option<Point> { None }
    fn touch_released(&self) -> bool { false }
    fn swipe(&self) -> SwipeDir { SwipeDir::None }
    fn was_back_gesture(&self) -> bool { false }
    fn was_home_gesture(&self) -> bool { false }
}

impl Clock for Skeleton {
    fn millis(&self) -> u32 { 0 }
}

// Caught at the definition, not at the install site, if a trait is missing.
const _: fn() = || {
    fn assert_host<T: host::Host>() {}
    assert_host::<Skeleton>();
};

static BACKEND: Skeleton = Skeleton;

// Safety: this is the only thread, and no frame has started.
unsafe { host::install(&BACKEND) };

assert_eq!(xpui::Renderer::screen_size(), Size::new(480, 800));
assert_eq!(xpui::Font::ui().text_width("Wi-Fi"), 0, "no font, so no width");
```

**See also:** [`host::install`](#hostinstall), [the host contract](../host.md)

## `host::InputSource`

One frame of input, and what the device it came from can do.

```text
pub trait InputSource
```

Edge queries are true for exactly one frame. Nothing is consumed by reading, so
the framework may ask the same question more than once per frame. A host
latches what its event loop saw at the top of a frame, and answers from that
until the next one.

One method is not about a frame at all:
[`host::InputSource::has_left_right_keys`](#hostinputsourcehas_left_right_keys)
describes the device, and answers the same every time. A screen reaches the
rest through [`Input`](input.md#input-1).

### Required methods

#### `host::InputSource::was_pressed`

Whether `button` went down this frame.

```text
fn was_pressed(&self, button: Button) -> bool
```

An edge. The runtime starts key auto-repeat from it, timed against
[`host::Clock::millis`](#hostclockmillis). `button` is a
[`Button`](input.md#button), by meaning: a host maps its physical keys, and the
user's remapping, onto it.

#### `host::InputSource::is_pressed`

Whether `button` is down, this frame included.

```text
fn is_pressed(&self, button: Button) -> bool
```

A level, true on the frame `was_pressed` is and on every frame after until the
key comes up. Auto-repeat reads it.

#### `host::InputSource::was_released`

Whether `button` came up this frame.

```text
fn was_released(&self, button: Button) -> bool
```

#### `host::InputSource::has_touch`

Whether this frame carries any touch at all, down or just lifted.

```text
fn has_touch(&self) -> bool
```

About this frame, never whether the panel has a digitiser. A device with no
touch panel answers `false` every frame.

#### `host::InputSource::has_left_right_keys`

Whether the device has a Left/Right pair to nudge a value with.

```text
fn has_left_right_keys(&self) -> bool
```

With the pair, Left and Right move a value where it stands. Without one, those
keys are busy walking between rows, so Confirm opens the value and closes it
again. A control reads this and branches; nothing in the framework does so on
its behalf.

> [!IMPORTANT]
> **Deliberately not defaulted.** A backend that forgot to answer would inherit
> whichever behaviour a default picked, with nothing to notice. When unsure,
> `false` is the safe answer: a control told the keys are missing costs a
> keystroke, and one told they exist when they do not cannot be changed at all.

#### `host::InputSource::tap`

A completed tap, at the position the finger went down.

```text
fn tap(&self) -> Option<Point>
```

`Some` on the one frame the tap completes, and `None` on every other.

#### `host::InputSource::touch_held`

Where a finger is while one is down, the signal a slider drag needs.

```text
fn touch_held(&self) -> Option<Point>
```

A level: `Some` on every frame the finger stays down, with its current
position, and `None` once it lifts.

#### `host::InputSource::touch_released`

Whether a finger lifted this frame.

```text
fn touch_released(&self) -> bool
```

#### `host::InputSource::swipe`

A completed swipe, or `SwipeDir::None`.

```text
fn swipe(&self) -> SwipeDir
```

An edge, like a tap: a [`SwipeDir`](input.md#swipedir) on the one frame the
swipe completes, and `SwipeDir::None` otherwise.

#### `host::InputSource::was_back_gesture`

The system back gesture, an edge swipe on a touch device.

```text
fn was_back_gesture(&self) -> bool
```

#### `host::InputSource::was_home_gesture`

The system home gesture, offered to the screen before the host acts.

```text
fn was_home_gesture(&self) -> bool
```

The screen on top sees it first, through `Screen::handle_home_gesture`, and
consumes it by returning `true`, as an overlay does to dismiss itself.

### Provided methods

#### `host::InputSource::swipe_moves_selection`

Which way a vertical swipe moves focus.

```text
fn swipe_moves_selection(&self) -> bool
```

`false`, the default, means the swipe moves the *content*: swiping up walks
**down** the list, as though dragging the page. `true` moves the focus with the
swipe. Defaulted so a host need not implement it until there is a setting
behind it.

**See also:** [`Input`](input.md#input-1), [`Button`](input.md#button), [`SwipeDir`](input.md#swipedir)

## `host::Clock`

The host's monotonic clock.

```text
pub trait Clock
```

Anything the framework paces itself with reads this. A screen reads it through
`xpui::millis`.

### Required methods

#### `host::Clock::millis`

Milliseconds since boot.

```text
fn millis(&self) -> u32
```

Key auto-repeat is timed against it. The caller only takes wrapping
differences, so a host may return a plain counter that wraps. It must be a real
clock, though: answering `0` for ever means a held key never repeats.

**Example — a counter a timer advances**

```rust
use core::sync::atomic::{AtomicU32, Ordering};
use xpui::host::Clock;

/// Milliseconds, advanced by the board's timer interrupt.
struct Ticks(AtomicU32);

impl Ticks {
    fn advance(&self, elapsed: u32) {
        self.0.fetch_add(elapsed, Ordering::Relaxed); // wraps at u32::MAX
    }
}

impl Clock for Ticks {
    fn millis(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
}

static CLOCK: Ticks = Ticks(AtomicU32::new(u32::MAX - 4));

let before = CLOCK.millis();
CLOCK.advance(10);
assert!(CLOCK.millis() < before, "the counter wrapped");
assert_eq!(CLOCK.millis().wrapping_sub(before), 10, "the difference did not");
```

## `host::install`

Installs the host, before any view is measured or drawn.

```text
pub unsafe fn install(host: &'static dyn Host)
```

| Parameter | Meaning |
|---|---|
| `host` | The host, for the rest of the process. A `static`, or a value leaked with `Box::leak`. |

Installing again replaces it — another panel size is another backend — and the
old `&'static` stays valid for anything that read it.

**The host is process-wide.** A test that installs a second corrupts what the
first was serving, which shows up as flakiness. Put tests that install a real
backend in their own integration test file, which Cargo runs as its own
process, or take the lock `xpui::testing::Ui` holds.

> [!WARNING]
> **Safety.** Call it from one thread, with no frame in flight: no `measure`,
> `render` or `interactions` running on any task. The write is unsynchronised,
> so overlapping it with a read is a data race, which is undefined behaviour,
> not a stale pointer you could notice.

Nothing reads the host before a view exists, so installing first thing in the
entry point, before the render loop starts, satisfies the contract. In a build
with the `testing` feature, a read that finds no host installs the fake instead
of panicking; without it, that read panics with "xpui::host::install was never
called".

**Example — installing from an entry point**

```rust
use xpui::host;
use xpui::testing::TestHost;

static BACKEND: TestHost = TestHost;

fn main() {
    // Safety: this is the only thread, and nothing has been measured, rendered
    // or asked for its interactions yet.
    unsafe { host::install(&BACKEND) };
    assert!(host::is_installed());

    // A host built at run time lives for the process, too.
    let built: &'static TestHost = Box::leak(Box::new(TestHost));
    // Safety: still the only thread, and still no frame in flight.
    unsafe { host::install(built) };
}
```

**See also:** [`host::is_installed`](#hostis_installed), [`host::install_navigator`](#hostinstall_navigator), [writing a backend](../writing-a-backend.md#5-installing-it)

## `host::is_installed`

Whether a host has been installed, so tests can assert wiring.

```text
pub fn is_installed() -> bool
```

`xpui::testing::install` reads it to stay idempotent: it installs the fake only
when nothing is installed, so it never takes the global back off a real
backend a test installed first.

**Example — the fake host, installed**

```rust
use xpui::{host, testing};

testing::install();
assert!(host::is_installed());
assert!(host::is_navigator_installed(), "the fake is a navigator as well");

testing::install(); // idempotent: the host already there stays
assert!(host::is_installed());
```

## `host::install_navigator`

Installs the navigator, before the first frame.

```text
pub unsafe fn install_navigator(navigator: &'static dyn Navigator)
```

The second installable thing, apart from the host because it has a different
owner: a backend crate supplies the host, and whoever owns the screen stack
supplies the [`Navigator`](navigation.md#navigator). `App::new` installs itself,
and a C++ firmware installs one that calls across its FFI. With none installed,
`present` and `finish_screen` do nothing, and a debug build says so.

> [!WARNING]
> **Safety.** The same contract as [`host::install`](#hostinstall): before any
> frame runs, and never concurrently with one. A host with a separate render
> task must install from both entry points, since either may wake first.

**Example — installing from two entry points**

```rust
use xpui::host;
use xpui::testing::TestHost;

static SHELL: TestHost = TestHost;

/// Called first thing by both the render task and the input task.
fn wire_navigation() {
    if !host::is_navigator_installed() {
        // Safety: no frame has started on either task, and the two entry
        // points do not run at the same time.
        unsafe { host::install_navigator(&SHELL) };
    }
}

wire_navigation();
wire_navigation(); // the second finds it installed, and writes nothing
assert!(host::is_navigator_installed());
```

## `host::is_navigator_installed`

Whether a navigator has been installed, so a host can assert its wiring and stay idempotent across two entry points.

```text
pub fn is_navigator_installed() -> bool
```

**See also:** [`host::install_navigator`](#hostinstall_navigator), [`Navigator`](navigation.md#navigator)
