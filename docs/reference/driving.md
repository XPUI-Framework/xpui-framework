# Driving a screen

A harness that drives a screen the way a person does, finding controls by the
text on the panel, and the functions that feed the fake host one frame of input
at a time. Either way a test reaches focus, routing and dispatch as a device
does, under `cargo test`, and neither is compiled unless the crate's `testing`
feature is on.

[Testing a screen without a screen](../testing.md) is the guide, and
[its second layer](../testing.md#2-driving-it-the-way-a-person-does) is this
page at work. Installing the fake host, and the numbers it answers with, is
[Testing](testing.md); what was drawn, and how to assert on it, is
[Recording](recording.md). This page is what each piece does.

## Topics

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
| [`testing::next_frame`](#testingnext_frame) | Ends the frame the fake is reporting input for. |
| [`testing::set_swipe`](#testingset_swipe) | Reports one swipe to the next frame the runtime reads input, so navigation can be tested without a finger. |
| [`testing::set_millis`](#testingset_millis) | Moves the fake clock, so repeat timing is deterministic. |
| [`testing::set_has_left_right_keys`](#testingset_has_left_right_keys) | Says whether the device the fake stands for has a Left/Right pair, so a control that branches on it can be tested both ways. |
| [`testing::set_swipe_moves_selection`](#testingset_swipe_moves_selection) | Chooses which way a swipe moves focus, so both readings can be tested. |

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
through [`Drive`](#testingdrive), the log is [`reset`](testing.md#testingreset), the app
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
feature. [`TestHost`](testing.md#testingtesthost) does not.

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
what the fake's `InputSource` reads on the next frame; [`reset`](testing.md#testingreset)
clears all of it.

**A press or a swipe lasts one frame, and reading it consumes nothing.** The
runtime may ask the same question twice in a frame and gets the same answer.
The frame ends at [`next_frame`](#testingnext_frame), at `reset`, or at the
first `press`, `hold`, `release`, `set_swipe` or `set_millis` after the frame
has read input. So each write describes the frame that reads it, and two writes
with no frame between them land in the same one.

| Test | What each frame sees |
|---|---|
| `press(Down)`, tick, `press(Up)`, tick | Down, then Up |
| `hold(Down)`, tick, `set_millis(600)`, tick | the press, then Down still held with no new press |
| `press(Confirm)`, tick, tick | Confirm **twice**: nothing was written between them |
| `press(Confirm)`, tick, `next_frame()`, tick | Confirm, then a quiet frame |

## `testing::press`

Reports one button press to the next frame the runtime reads input.

```text
pub fn press(button: Button)
```

It reads the same for the whole frame, however often that frame asks, and is
cleared when the frame ends. A second `press` before a frame reads the first
replaces it.

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
It stays down until [`release`](#testingrelease) or [`reset`](testing.md#testingreset).

## `testing::release`

Lifts whatever `hold` put down.

```text
pub fn release()
```

## `testing::next_frame`

Ends the frame the fake is reporting input for.

```text
pub fn next_frame()
```

A pending press or swipe is cleared; a button put down with
[`hold`](#testinghold) stays down, and nothing else changes. Needed only
between two frames with no input written between them, since any write after a
read already ends the frame.

**Example — a quiet frame after a press**

```rust
use xpui::{Button, Input, testing};

testing::install();
testing::reset();

testing::press(Button::Confirm);
assert!(Input::was_pressed(Button::Confirm));
assert!(Input::was_pressed(Button::Confirm), "reading consumes nothing");

testing::next_frame();
assert!(!Input::was_pressed(Button::Confirm), "an edge lasts one frame");
```

## `testing::set_swipe`

Reports one swipe to the next frame the runtime reads input, so navigation can be tested without a finger.

```text
pub fn set_swipe(direction: SwipeDir)
```

It reads the same for the whole frame, however often that frame asks, and is
cleared when the frame ends, as a [press](#testingpress) is.

## `testing::set_millis`

Moves the fake clock, so repeat timing is deterministic.

```text
pub fn set_millis(value: u32)
```

The clock reads `0` until this is called, and never moves by itself.
[`reset`](testing.md#testingreset) leaves it alone.

## `testing::set_has_left_right_keys`

Says whether the device the fake stands for has a Left/Right pair, so a control that branches on it can be tested both ways.

```text
pub fn set_has_left_right_keys(present: bool)
```

`false` until set, and reset to `false` by [`reset`](testing.md#testingreset).

## `testing::set_swipe_moves_selection`

Chooses which way a swipe moves focus, so both readings can be tested.

```text
pub fn set_swipe_moves_selection(enabled: bool)
```

| Parameter | Meaning |
|---|---|
| `enabled` | `false`, the default, moves the content: swiping up walks down the list. `true` moves the focus with the swipe. |

`false` until set, and reset to `false` by [`reset`](testing.md#testingreset).
