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
nothing. This page is installing the fake, what it counted and the fake's
numbers. Driving a screen and feeding it input is
[Driving a screen](driving.md), and what was drawn, and how to assert on it, is
[Recording](recording.md).

## Topics

### Installing the fake host

| | |
|---|---|
| [`testing::install`](#testinginstall) | Installs the fake as the host and the navigator, each only if none is installed yet. |
| [`testing::install_forced`](#testinginstall_forced) | Installs the fake **over** whatever host is already there. |
| [`testing::TestHost`](#testingtesthost) | The fake host. |
| [`testing::reset`](#testingreset) | Forgets every recorded draw, pending input, input flag and counter. |

### What the fake counted

| | |
|---|---|
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
| `InputSource` | Reports only what a test injected with [`press`](driving.md#testingpress), [`hold`](driving.md#testinghold) and [`set_swipe`](driving.md#testingset_swipe). Nothing is consumed by reading: an edge reads the same all frame, until [the frame ends](driving.md#testingnext_frame). No touch panel, no taps and no gestures. |
| `Clock` | Reads the time [`set_millis`](driving.md#testingset_millis) set; `0` until then. |

It does not implement [`Drive`](driving.md#testingdrive), so [`Ui`](driving.md#testingui) does not
run on it. To drive a screen through the fake, call
[`press`](driving.md#testingpress) and tick an `App` yourself.

**See also:** [`testing::install`](#testinginstall), [`testing::Recorder`](recording.md#testingrecorder)

## `testing::reset`

Forgets every recorded draw, pending input, input flag and counter.

```text
pub fn reset()
```

Call it at the start of each test, and again just before the frame a test is
about to assert on, so the log holds that frame only. It ends the frame, as
[`next_frame`](driving.md#testingnext_frame) does, and clears the log, a pending press,
swipe or hold, both flags (`set_has_left_right_keys`,
`set_swipe_moves_selection`) and the three counters. The clock
[`set_millis`](driving.md#testingset_millis) moved stays where it is.

**See also:** [`testing::ops_log`](recording.md#testingops_log), [`testing::install`](#testinginstall)

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
