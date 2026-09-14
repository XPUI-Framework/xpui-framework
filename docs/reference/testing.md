# Testing

A fake host that records every draw call, a wrapper that records a real
backend's, and a harness that drives a screen the way a person does, so a
screen can be tested under `cargo test` with no device, no simulator and no
window. None of it is compiled unless the crate's `testing` feature is on:

```bash
cargo test --features xpui/testing
```

A crate testing its own screens, or its own backend, turns the feature on in
its `[dev-dependencies]`. Inside `xpui` the module is also on under `cfg(test)`.

[Testing a screen without a screen](../testing.md) is the guide: the four
layers of test, what each catches, and the tests that passed while proving
nothing. This page is installing the fake, driving a screen, feeding input and
the fake's numbers; what was drawn, and how to assert on it, is
[Recording](recording.md).

## Topics

### Installing the fake host

| | |
|---|---|
| [`testing::install`](#testinginstall) | Installs the fake as the host and the navigator, each only if none is installed yet. |
| [`testing::install_forced`](#testinginstall_forced) | Installs the fake **over** whatever host is already there. |
| [`testing::TestHost`](#testingtesthost) | The fake host. |
| [`testing::reset`](#testingreset) | Forgets every recorded draw, pending input, input flag and counter. |

### Driving a screen like a person

| | |
|---|---|
| [`testing::Ui`](#testingui) | A screen under test, with the app and host that drive it. |
| [`testing::Drive`](#testingdrive) | A host that a test can feed input to. |

### Feeding input and reading state

| | |
|---|---|
| [`testing::press`](#testingpress) | Reports one button press to the next frame the runtime reads input. |
| [`testing::hold`](#testinghold) | Reports the edge *and* leaves the button down, as a finger does. |
| [`testing::release`](#testingrelease) | Lifts whatever `hold` put down. |
| [`testing::set_swipe`](#testingset_swipe) | Reports one swipe to the next frame the runtime reads input, so navigation can be tested without a finger. |
| [`testing::set_millis`](#testingset_millis) | Moves the fake clock, so repeat timing is deterministic. |
| [`testing::set_has_left_right_keys`](#testingset_has_left_right_keys) | Says whether the device the fake stands for has a Left/Right pair, so a control that branches on it can be tested both ways. |
| [`testing::set_swipe_moves_selection`](#testingset_swipe_moves_selection) | Chooses which way a swipe moves focus, so both readings can be tested. |
| [Counters](#counters) | How many times a screen finished, asked for a repaint, or offered a screen to the navigator. |

### The fake metrics

| | |
|---|---|
| [Metrics](#metrics) | The panel size, chrome geometry and font metrics the fake answers with. |

## `testing::install`

Installs the fake as the host and the navigator, each only if none is installed yet.

```text
pub fn install()
```

Call it first in every test that renders, measures or reads input. Calling it
again costs nothing.

> [!WARNING]
> **It will not take the global back off another host.** A test binary that
> installs a real backend and then calls `install` keeps the backend, and every
> [draw accessor](recording.md#draw-accessors) comes back empty while the assertions read
> as though the screen drew nothing. Put behaviour tests and pixel tests in
> separate files under `tests/`: Cargo runs each file in its own process.

The host is process-wide, but what it records is **per thread**: the draw log,
injected input and counters are thread-locals. A test that renders on one thread
and reads the log on another reads an empty log.

**Example — measuring a widget**

```rust
use xpui::{Text, View, testing};

testing::install();

let mut label = Text::new("Wi-Fi");
View::<()>::measure(&mut label, testing::screen());

// The fake's interface font is 17px tall and 6px per character.
let size = View::<()>::size(&label);
assert_eq!(size.width, 5 * 6);
assert_eq!(size.height, 17);
```

**See also:** [`testing::install_forced`](#testinginstall_forced), [`testing::reset`](#testingreset), [`testing::TestHost`](#testingtesthost)

## `testing::install_forced`

Installs the fake **over** whatever host is already there.

```text
pub unsafe fn install_forced()
```

For a test binary that must alternate between the fake and a real backend in
one process. The navigator is still installed only if none is. Prefer separate
test files; this is the escape hatch.

> [!WARNING]
> The contract is `host::install`'s: call it before any frame runs, and never
> while one is running on another thread.

**See also:** [`testing::install`](#testinginstall)

## `testing::TestHost`

The fake host.

```text
pub struct TestHost
```

One static instance, which [`install`](#testinginstall) puts behind the host and
the navigator. It implements every host trait, and draws nothing: each call is
pushed onto the log that [`ops_log`](recording.md#testingops_log) returns.

| Trait | What the fake does |
|---|---|
| `Canvas` | Records every call as a [`DrawOp`](recording.md#testingdrawop). The panel is [`SCREEN_WIDTH`](#metrics) by [`SCREEN_HEIGHT`](#metrics). Text in font id `0`, a font the build omitted, is not recorded. An icon measures its requested size. |
| `TextMetrics` | Fixed metrics per font role; see [`line_height`](#metrics) and [`text_width`](#metrics). |
| `Chrome` | Answers every `ThemeMetric` with the [constants](#metrics), and records the header, hints, lists, dialogs and bars instead of painting them. A dialog's rows stack from a quarter of the way down, one list row each. Each `request_update` adds one to [`updates`](#counters). |
| `Navigator` | Counts: `finish` adds to [`finishes`](#counters), and `present` adds to [`presents`](#counters) and hands the screen back, refused. The title is `"Test"`. |
| `InputSource` | Reports only what a test injected with [`press`](#testingpress), [`hold`](#testinghold) and [`set_swipe`](#testingset_swipe). No touch panel, no taps and no gestures. |
| `Clock` | Reads the time [`set_millis`](#testingset_millis) set; `0` until then. |

It does not implement [`Drive`](#testingdrive), so [`Ui`](#testingui) does not
run on it. To drive a screen through the fake, call
[`press`](#testingpress) and tick an `App` yourself.

**See also:** [`testing::install`](#testinginstall), [`testing::Recorder`](recording.md#testingrecorder)

## `testing::reset`

Forgets every recorded draw, pending input, input flag and counter.

```text
pub fn reset()
```

Call it at the start of each test, and again just before the frame a test is
about to assert on, so the log holds that frame only. It clears the log, a
pending press, swipe or hold, both flags (`set_has_left_right_keys`,
`set_swipe_moves_selection`) and the three counters. The clock
[`set_millis`](#testingset_millis) moved stays where it is.

**See also:** [`testing::ops_log`](recording.md#testingops_log), [`testing::install`](#testinginstall)

## `testing::Ui`

A screen under test, with the app and host that drive it.

```text
pub struct Ui<H: Host + Drive + 'static>
```

`Ui` builds an `App` on a real backend, wrapped in a
[`Recorder`](recording.md#testingrecorder), and finds controls **by the text on the panel**.
[`tap_text`](#testinguitap_text) looks up where that string was painted and
taps the middle of it, so a test goes through hit testing, focus, scrolling and
dispatch exactly as a thumb does. Layout comes from the backend, so a test about
a particular panel size runs at that size.

Every action is one frame: the clock moves on 16 ms, the input is delivered
through [`Drive`](#testingdrive), the log is [`reset`](#testingreset), the app
ticks, and it repaints if anything asked. [`frame`](#testinguiframe) and
[`visible_text`](#testinguivisible_text) read the last frame that painted.

> [!NOTE]
> **One `Ui` at a time.** It installs the process-wide host, so a `Ui` holds a
> lock for its whole life and the next one waits for it. Tests in one file need
> no lock of their own. A second `Ui` alive in the *same* test waits a minute
> and then panics, saying so.

`Ui` is not built for bare-metal targets (`target_os = "none"`): it needs `std`
for the lock.

The fake host does not implement [`Drive`](#testingdrive). A backend that does is
`xpui-embedded-graphics`'s `Backend`, with its own `testing` feature on, so the
harness is written generic over the host and a test in that crate passes its
backend in.

**Example — opening a screen from a menu**

```rust
use xpui::host::Host;
use xpui::testing::{Drive, Ui};
use xpui::{Button, NavigationScreen, Screen, Text, View, vstack};

struct Menu;

impl Screen for Menu {
    type Message = ();
    fn body(&self) -> impl View<()> {
        NavigationScreen::new(vstack![0; Text::new("Lists")])
    }
    fn update(&mut self, _message: ()) {}
}

/// `backend` is a host that implements `Drive`, already sized for its panel.
fn opens_the_list_screen<H: Host + Drive + 'static>(backend: &'static H) {
    let mut ui = Ui::new(Menu, backend);
    ui.tap_text("Lists");
    assert!(ui.visible_text().iter().any(|line| line == "Rows, subtitles, values"));

    ui.press(Button::Back);
    assert_eq!(ui.depth(), 1);
}
```

**Assert on what is on the panel, not on the depth.** `depth() == 2` says
something opened; `visible_text` says the right thing did.

### Starting

#### `testing::Ui::new`

Installs `host` behind a recorder, starts `screen`, and paints once.

```text
pub fn new<S: Screen + 'static>(screen: S, host: &'static H) -> Ui<H>
```

| Parameter | Meaning |
|---|---|
| `screen` | The root screen, taken by value. The `App` owns it from here. |
| `host` | The backend. It is installed as the host, wrapped so its draw calls are recorded. |

The first paint is not optional: the runtime ignores taps until it has painted,
so without it the first tap of every test would be swallowed. The `App`
installs itself as the navigator.

### Acting

Each of these is one frame, and returns the `Ui` so actions chain:
`ui.tap_text("Wi-Fi").press(Button::Back)`.

#### `testing::Ui::tap_text`

Taps the middle of whatever control shows `label`.

```text
pub fn tap_text(&mut self, label: &str) -> &mut Self
```

Panics if nothing shows that text, listing what is on the panel, and panics if
the text is in more than one place, rather than choosing by draw order. Use
[`tap_nth_text`](#testinguitap_nth_text) for a string that repeats.

#### `testing::Ui::tap_nth_text`

Taps the `index`th place `label` appears, in draw order.

```text
pub fn tap_nth_text(&mut self, label: &str, index: usize) -> &mut Self
```

| Parameter | Meaning |
|---|---|
| `label` | The text, matched whole. |
| `index` | Zero-based, in the order [`rects_of_text`](#testinguirects_of_text) returns. Panics when there are fewer. |

#### `testing::Ui::tap_at`

Taps a panel coordinate.

```text
pub fn tap_at(&mut self, point: Point) -> &mut Self
```

For what has no text: a background, a scrim, a place outside a dialog.

#### `testing::Ui::press`

Presses and releases `button` in one frame.

```text
pub fn press(&mut self, button: Button) -> &mut Self
```

#### `testing::Ui::swipe`

Swipes in `direction`.

```text
pub fn swipe(&mut self, direction: SwipeDir) -> &mut Self
```

### Reading the frame

#### `testing::Ui::visible_text`

Every string the last painted frame actually showed, in draw order.

```text
pub fn visible_text(&self) -> Vec<String>
```

Text clipped away is left out, since a `ScrollView` draws all its content and
lets the clip hide what does not fit. Each string appears once, however many
times it was drawn.

#### `testing::Ui::rects_of_text`

Every place `label` is painted, as something tappable.

```text
pub fn rects_of_text(&self, label: &str) -> Vec<Rect>
```

A place is left out when it is clipped away, or thinner than half the theme's
minimum touch size. A row and the label inside it count once, as the label.
While a dialog is up only the dialog is searched, because it captures input: a
row behind it is visible and cannot be tapped. Header text is visible and never
tappable.

#### `testing::Ui::rect_of_text`

Where `label` is, when it is in exactly one place.

```text
pub fn rect_of_text(&self, label: &str) -> Option<Rect>
```

`None` covers both "not on the panel" and "in more than one place".

#### `testing::Ui::changed`

Whether the last action repainted, and painted something different.

```text
pub fn changed(&self) -> bool
```

Draw calls are compared, not pixels, so the same ink in different places counts
as a change.

#### `testing::Ui::frame`

The draw calls of the last painted frame, for assertions this harness has no word for.

```text
pub fn frame(&self) -> &[DrawOp]
```

### Reading the app

#### `testing::Ui::depth`

How many screens are on the stack.

```text
pub fn depth(&self) -> usize
```

#### `testing::Ui::is_running`

Whether any screen is left.

```text
pub fn is_running(&self) -> bool
```

### Reaching the host

#### `testing::Ui::host`

The host underneath, for reading pixels or anything else backend-shaped.

```text
pub fn host(&self) -> &'static H
```

**See also:** [`testing::Drive`](#testingdrive), [`testing::Recorder`](recording.md#testingrecorder), [the guide's second layer](../testing.md#2-driving-it-the-way-a-person-does)

## `testing::Drive`

A host that a test can feed input to.

```text
pub trait Drive
```

`InputSource` only reads input. A backend already has a way to write it, for its
simulator or its event loop; `Drive` names those methods so
[`Ui`](#testingui) can reach them without knowing which backend it drives.
`xpui-embedded-graphics`'s `Backend` implements it under that crate's `testing`
feature. [`TestHost`](#testingtesthost) does not.

### Required methods

#### `testing::Drive::begin`

Starts a frame at `millis`, clearing the previous frame's edges.

```text
fn begin(&self, millis: u32)
```

Without it a press stays "just pressed" forever, and every frame acts on it
again. `Ui` calls it before each action's input, with a clock that moves on
16 ms a frame.

#### `testing::Drive::inject_press`

Reports `button` as pressed this frame.

```text
fn inject_press(&self, button: Button)
```

#### `testing::Drive::inject_release`

Reports `button` as released this frame.

```text
fn inject_release(&self, button: Button)
```

#### `testing::Drive::inject_tap`

Reports a completed tap at `point`.

```text
fn inject_tap(&self, point: Point)
```

#### `testing::Drive::inject_swipe`

Reports a completed swipe.

```text
fn inject_swipe(&self, direction: SwipeDir)
```

**See also:** [`testing::Ui`](#testingui)

## Feeding input and reading state

The fake reports only the input a test put there. Each function below writes
what the fake's `InputSource` reads on the next frame; [`reset`](#testingreset)
clears all of it.

## `testing::press`

Reports one button press to the next frame the runtime reads input.

```text
pub fn press(button: Button)
```

Cleared by [`reset`](#testingreset), and consumed when read, so it fires exactly
once. A second `press` before a frame reads the first replaces it.

**Example — moving the focus down a list**

```rust
use xpui::{App, Button, List, ListRow, NavigationScreen, Screen, View, testing};

struct Networks;

impl Screen for Networks {
    type Message = usize;
    fn body(&self) -> impl View<usize> {
        NavigationScreen::new(
            List::new()
                .push(ListRow::new("Home").on_tap(0))
                .push(ListRow::new("Office").on_tap(1)),
        )
    }
    fn update(&mut self, _index: usize) {}
}

testing::install();
let mut app = App::new(Networks);
app.render();

testing::press(Button::Down);
app.tick();
testing::reset();
app.render_if_dirty();
assert_eq!(testing::drawn_lists(), [(2, 1)], "the focus is on Office");
```

**See also:** [`testing::hold`](#testinghold), [`testing::set_swipe`](#testingset_swipe)

## `testing::hold`

Reports the edge *and* leaves the button down, as a finger does.

```text
pub fn hold(button: Button)
```

[`press`](#testingpress) alone is a key tapped so briefly that no frame saw it
held, which is not what hardware sends. A held key is what auto-repeat reads:
hold, then move the clock with [`set_millis`](#testingset_millis) and tick.
It stays down until [`release`](#testingrelease) or [`reset`](#testingreset).

## `testing::release`

Lifts whatever `hold` put down.

```text
pub fn release()
```

## `testing::set_swipe`

Reports one swipe to the next frame the runtime reads input, so navigation can be tested without a finger.

```text
pub fn set_swipe(direction: SwipeDir)
```

Cleared by [`reset`](#testingreset), and consumed when read.

## `testing::set_millis`

Moves the fake clock, so repeat timing is deterministic.

```text
pub fn set_millis(value: u32)
```

The clock reads `0` until this is called, and never moves by itself.
[`reset`](#testingreset) leaves it alone.

## `testing::set_has_left_right_keys`

Says whether the device the fake stands for has a Left/Right pair, so a control that branches on it can be tested both ways.

```text
pub fn set_has_left_right_keys(present: bool)
```

`false` until set, and reset to `false` by [`reset`](#testingreset).

## `testing::set_swipe_moves_selection`

Chooses which way a swipe moves focus, so both readings can be tested.

```text
pub fn set_swipe_moves_selection(enabled: bool)
```

| Parameter | Meaning |
|---|---|
| `enabled` | `false`, the default, moves the content: swiping up walks down the list. `true` moves the focus with the swipe. |

`false` until set, and reset to `false` by [`reset`](#testingreset).

## Counters

What a screen asked of the fake's `Navigator` and `Chrome`, counted since the
last [`reset`](#testingreset). They count only while the fake is the navigator:
`App::new` installs its own, so a screen inside an `App` is counted by
[`updates`](#counters) and not by the other two.

| Counter | Abstract | Returns |
|---|---|---|
| `testing::finishes` | How many times a screen asked to be finished since the last `reset`. | `u32` |
| `testing::updates` | How many repaints were requested since the last `reset`. | `u32` |
| `testing::presents` | How many screens were offered to the navigator since the last `reset`. | `u32` |

## Metrics

The numbers the fake answers with, in the proportions a real theme uses: a
portrait 480×800 panel, and chrome sized to match. The slider's are a real
backend's own defaults. A test that asserts a position works it out from these,
never from a literal, so it reads as the rule it checks.

| Metric | Abstract | Value |
|---|---|---|
| `testing::SCREEN_WIDTH` | The width of the panel the fake reports: a portrait e-reader. | `480` |
| `testing::SCREEN_HEIGHT` | The height of the panel the fake reports. | `800` |
| `testing::TOP_PADDING` | Gap above the header band. | `8` |
| `testing::HEADER_HEIGHT` | Height of the header band. | `40` |
| `testing::VERTICAL_SPACING` | The gap between stacked elements. | `12` |
| `testing::CONTENT_TOP` | First y below the header that content may use. | `TOP_PADDING + HEADER_HEIGHT + VERTICAL_SPACING`, `60` |
| `testing::CONTENT_BOTTOM` | First y occupied by the button hints. | `SCREEN_HEIGHT - BUTTON_HINTS_HEIGHT`, `760` |
| `testing::BUTTON_HINTS_HEIGHT` | The band reserved at the bottom for the button hints. | `40` |
| `testing::SIDE_PADDING` | Space between the panel's side edges and the content. | `16` |
| `testing::MIN_TOUCH_SIZE` | The smallest comfortably tappable dimension. | `44` |
| `testing::LIST_ROW_HEIGHT` | Height of a one-line list row. | `40` |
| `testing::LIST_ROW_HEIGHT_WITH_SUBTITLE` | Height of a list row carrying a subtitle. | `56` |
| `testing::LIST_ROW_GAP` | Gap between list rows, as a real theme leaves one. | `4` |
| `testing::SUB_HEADER_HEIGHT` | A heading's own line. | `17` |
| `testing::SPACING_SMALL` | The theme's small step, within a group. | `4` |
| `testing::PROGRESS_BAR_HEIGHT` | Height of the progress bar. | `6` |
| `testing::SLIDER_KNOB_WIDTH` | The slider knob's width. | `14` |
| `testing::SLIDER_KNOB_HEIGHT` | The slider knob's height. | `22` |
| `testing::SLIDER_SIDE_INSET` | The padding a slider's track is inset by at each end. | `8` |

Font metrics are fixed per font id. The interface font is 17 px tall with 6 px
per character, the small interface font 14 px and 5 px, and the reader font
19 px and 8 px; any other id measures as the interface font.

| Function | Abstract | Returns |
|---|---|---|
| `testing::screen` | The screen size, for `measure`. | `Size::new(SCREEN_WIDTH, SCREEN_HEIGHT)` |
| `testing::line_height` | Line height the fake reports for a font id. | `i32` |
| `testing::text_width` | Width the fake reports for `text` in a font id. | `i32`: characters, not bytes, times the font's advance |

**Example — a row's position from the metrics**

```rust
use xpui::testing::{self, CONTENT_TOP, LIST_ROW_GAP, LIST_ROW_HEIGHT};

// The third row of a list that starts at the top of the content area.
let third_row_top = CONTENT_TOP + 2 * (LIST_ROW_HEIGHT + LIST_ROW_GAP);
assert_eq!(third_row_top, 60 + 2 * 44);
assert_eq!(testing::text_width("Wi-Fi", 0), testing::text_width("abcde", 0));
```
