# Recording

What the framework drew, as data. Every draw call made on the fake host, or on
a real backend wrapped in a [`testing::Recorder`](#testingrecorder), lands in
one log, in the order it was made. A test reads the log whole with
[`testing::ops_log`](#testingops_log), keeps one kind of call with a
[draw accessor](#draw-accessors), or renders it as text and compares it with a
golden file, so call order, clipping and the state a widget was drawn in can be
asserted without a pixel. Like the rest of `xpui::testing`, none of it is
compiled unless the crate's `testing` feature is on.

[Draw calls](../testing.md#3-draw-calls) is the guide's layer of test built on
it. [Testing](testing.md) installs the fake host, resets the log and drives a
screen. This page is what each piece of the recording does.

## Topics

| | |
|---|---|
| [`testing::Recorder`](#testingrecorder) | Wraps a host, recording every draw call before passing it on. |
| [`testing::DrawOp`](#testingdrawop) | One call the framework made on the host. |
| [`testing::RectKind`](#testingrectkind) | How a rectangle was painted. |
| [`testing::ops_log`](#testingops_log) | Everything drawn since the last `reset`, in order. |
| [`testing::render`](#testingrender) | Every op as text, one per line, which is exactly a golden file's body. |
| [`testing::assert_snapshot`](#testingassert_snapshot) | Asserts that everything drawn since the last `reset` matches the golden. |
| [Draw accessors](#draw-accessors) | One view onto the log per kind of call: text, rectangles, headers, lists, dialogs, hints, sliders, bars, indicators and clips. |
| [`testing::TextDraw`](#testingtextdraw) | One recorded `draw_text`: position, text, font id and style. |
| [`testing::RectDraw`](#testingrectdraw) | One recorded rectangle: x, y, width, height, and how it was painted. |
| [`testing::RowCells`](#testingrowcells) | The title, subtitle and value of a list row, as the theme asked for them. |

## `testing::Recorder`

Wraps a host, recording every draw call before passing it on.

```text
pub struct Recorder<H: Host + 'static>
```

Every call is forwarded, and nothing is answered from the recording, so a screen
measures, lays out and paints exactly as it would without the wrapper. The
record lands in the same log as the fake's, so [`ops_log`](#testingops_log), the
[draw accessors](#draw-accessors) and [snapshots](#testingassert_snapshot) all
read a real backend's frame. [`Ui`](driving.md#testingui) installs one for you.

A repaint request is counted by [`updates`](testing.md#counters), as the fake
counts its own, and then forwarded to the backend. A dither is recorded as the
fake records it, with `light` in `black`, so a snapshot marks the same shade
through either host. Wrap a real backend, never the fake itself: that records
every call twice.

**Example — recording a real backend**

```rust
use xpui::host::Host;
use xpui::testing::Recorder;

/// `backend` is the real host, already sized for the panel under test.
fn install_recording<H: Host + 'static>(backend: &'static H) {
    let recorded = Recorder::wrap(backend);
    // Safety: before the first frame, and never concurrently with one.
    unsafe { xpui::host::install(recorded) };
}
```

### Creating a recorder

#### `testing::Recorder::wrap`

Wraps `inner` and leaks the wrapper, because a host must be `'static`.

```text
pub fn wrap(inner: &'static H) -> &'static Recorder<H>
```

The installed host outlives every screen that could draw through it, and a test
process exits anyway.

### Reaching the host

#### `testing::Recorder::inner`

The host underneath, for anything this wrapper does not model — reading the framebuffer, say.

```text
pub fn inner(&self) -> &'static H
```

**See also:** [`testing::Ui`](driving.md#testingui), [`testing::ops_log`](#testingops_log)

## `testing::DrawOp`

One call the framework made on the host.

```text
pub enum DrawOp
```

The log is a `Vec<DrawOp>` in call order, so it answers what no per-kind list
can: whether the header was drawn before the content, and whether a clip was
lifted before the chrome. `DrawOp` is `Clone`, `Debug` and `PartialEq`, so a
test can compare a whole frame.

| Variant | Meaning | Fields |
|---|---|---|
| `testing::DrawOp::Clear` | The whole panel cleared to background. | — |
| `testing::DrawOp::Text` | A line of text. | `origin`, the top-left corner, not a baseline; `text`; `font`, the host's font id; `style` |
| `testing::DrawOp::Rect` | A rectangle, painted one of four ways. | `rect`; `kind`, a [`RectKind`](#testingrectkind); `black`, ink or background for a filled one, and `light` for a dither |
| `testing::DrawOp::Line` | A one-pixel line. | `from`, `to` |
| `testing::DrawOp::Image` | A 1-bit bitmap. | `origin`; `size`; `bytes`, how many were handed over, without the pixels |
| `testing::DrawOp::Icon` | An icon the host owns. | `origin`; `icon`, which icon and at what size |
| `testing::DrawOp::Clip` | A clip set on the host, or lifted when `None`. | `Option<Rect>` |
| `testing::DrawOp::Header` | The header band. | `title`, `None` for an empty band; `subtitle` |
| `testing::DrawOp::SubHeader` | A section heading. | `rect`, the band; `label`; `right`, a right-aligned value |
| `testing::DrawOp::Hints` | The button hints in meaning order: back, confirm, previous, next. | `[Option<String>; 4]`: `None` is the host's standard label, `"<edit>"`, `"<done>"` or `"<cancel>"` a word the host owns, and `""` a blank slot |
| `testing::DrawOp::ProgressBar` | A determinate progress bar. | `rect`; `current`, out of `total`; `total` |
| `testing::DrawOp::Slider` | A themed slider. | `rect`; `value`, out of `max`; `max`; `state`, idle, focused or editing |
| `testing::DrawOp::ScrollIndicator` | The scroll indicator beside a scrolling region. | `rect`; `content`, how tall; `visible`, how much shows; `offset`, how far down |
| `testing::DrawOp::List` | A themed list. | `rect`; `selected`, `-1` for none; `rows`, each a [`RowCells`](#testingrowcells) |
| `testing::DrawOp::OptionPopup` | A themed option dialog. | `title`; `selected`, the highlighted option; `options`, each label in order |

**Example — asserting on the order of a frame**

```rust
use xpui::testing::{self, DrawOp};
use xpui::{App, NavigationScreen, Screen, Text, View, vstack};

struct Storage;

impl Screen for Storage {
    type Message = ();
    fn body(&self) -> impl View<()> {
        NavigationScreen::new(vstack![0; Text::new("182 KB free")])
    }
    fn update(&mut self, _message: ()) {}
    fn title(&self) -> Option<&'static str> {
        Some("Storage")
    }
}

testing::install();
let mut app = App::new(Storage);
testing::reset();
app.render();

let log = testing::ops_log();
let header = log.iter().position(|op| matches!(op, DrawOp::Header { .. }));
let content = log
    .iter()
    .position(|op| matches!(op, DrawOp::Text { text, .. } if text == "182 KB free"));
assert!(header.is_some() && content.is_some());
```

### Writing a snapshot line

#### `testing::DrawOp::to_line`

One line of a snapshot.

```text
pub fn to_line(&self) -> String
```

Flat and readable rather than serialised, so a reviewer sees what changed in the
diff. A kind name, padded to twelve columns, is followed by the call's
arguments; a rectangle is `(x,y WxH)` and an absent string is `-`. A list and a
dialog continue on indented lines, one per row or option.

```rust
use xpui::testing::{DrawOp, RectKind};
use xpui::Rect;

assert_eq!(DrawOp::Clear.to_line(), "clear");
let rule = DrawOp::Rect { rect: Rect::new(16, 60, 448, 1), kind: RectKind::Filled, black: true };
assert_eq!(rule.to_line(), "rect        (16,60 448x1) filled");
```

**See also:** [`testing::render`](#testingrender), [`testing::ops_log`](#testingops_log), [`testing::RectKind`](#testingrectkind)

## `testing::RectKind`

How a rectangle was painted.

```text
pub enum RectKind
```

| Variant | Meaning | In a snapshot |
|---|---|---|
| `testing::RectKind::Filled` | Solid ink or background. | `filled`, then `background` when it was not ink |
| `testing::RectKind::Stroked` | An outline, one pixel wide. | `stroked` |
| `testing::RectKind::Dither` | Dithered, which reads as grey on a 1-bit panel. | `dither`, then `light` for the light pattern |
| `testing::RectKind::Scrim` | Darkened without erasing, so what was behind stays legible. | `scrim` |

**See also:** [`testing::DrawOp`](#testingdrawop), [`testing::RectDraw`](#testingrectdraw)

## `testing::ops_log`

Everything drawn since the last `reset`, in order.

```text
pub fn ops_log() -> Vec<DrawOp>
```

A copy, so a test may hold it across later frames. Every
[draw accessor](#draw-accessors) reads this same log, so "was a list drawn" and
"what did the frame look like" never disagree.

**See also:** [`testing::reset`](testing.md#testingreset), [`testing::render`](#testingrender)

## `testing::render`

Every op as text, one per line, which is exactly a golden file's body.

```text
pub fn render(ops: &[DrawOp]) -> String
```

Each op is its [`to_line`](#testingdrawopto_line), and every line ends in a
newline. Useful in a failure message, or to assert on a few lines without
committing a golden.

```rust
use xpui::{Point, Text, VStack, View, testing, vstack};

testing::install();
testing::reset();
let mut view: VStack<()> = vstack![8; Text::new("Wi-Fi"), Text::new("Bluetooth")];
view.measure(testing::screen());
view.render(Point::ORIGIN);

let text = testing::render(&testing::ops_log());
assert!(text.contains("\"Wi-Fi\""), "{text}");
assert!(text.contains("\"Bluetooth\""), "{text}");
```

**See also:** [`testing::assert_snapshot`](#testingassert_snapshot)

## `testing::assert_snapshot`

Asserts that everything drawn since the last `reset` matches the golden.

```text
pub fn assert_snapshot(name: &str)
```

| Parameter | Meaning |
|---|---|
| `name` | The golden's file name without `.txt`, under `tests/snapshots/` in the crate being tested. |

The golden is [`render`](#testingrender) of the log, committed beside the test,
so a change to layout shows up as a diff a reviewer reads. It asserts call
order, the clip lifecycle and the state a widget was drawn in, none of which a
picture shows.

| Situation | What happens |
|---|---|
| The golden matches | The assertion passes. |
| The golden differs | It panics with a line diff. |
| `UPDATE_SNAPSHOTS` is set, and not `0` | It writes the golden and passes. |
| No golden exists | It writes one and panics, so the new file is read before a second run trusts it. |

> [!NOTE]
> An accepted snapshot is an assertion you have made. Read the diff before
> committing `UPDATE_SNAPSHOTS=1 cargo test --features testing`.

**Example — a whole screen against its golden**

`no_run`, because the first run writes a file:

```rust,no_run
use xpui::{App, NavigationScreen, Screen, Text, View, testing, vstack};

struct Settings;

impl Screen for Settings {
    type Message = ();
    fn body(&self) -> impl View<()> {
        NavigationScreen::new(vstack![0; Text::new("Settings")])
    }
    fn update(&mut self, _message: ()) {}
}

testing::install();
let mut app = App::new(Settings);
testing::reset();
app.render();
testing::assert_snapshot("settings_screen");
```

**See also:** [`testing::render`](#testingrender), [`testing::DrawOp::to_line`](#testingdrawopto_line)

## Draw accessors

Each reads [`ops_log`](#testingops_log) and keeps one kind of call, in order, so
a test can assert on the kind it cares about without matching `DrawOp` itself.

| Accessor | Abstract | Returns |
|---|---|---|
| `testing::drawn_text` | Every `draw_text` recorded since the last `reset`. | `Vec<TextDraw>` |
| `testing::drawn_rects` | Every rectangle recorded since the last `reset`, in draw order. | `Vec<RectDraw>` |
| `testing::drawn_headers` | The title passed to each `draw_header` since the last `reset`. | `Vec<Option<String>>`; `None` is a header with no title, which paints an empty band |
| `testing::drawn_sub_headers` | Every sub-header (a `Section` title) drawn since the last `reset`, with the rect it was given. | `Vec<(Rect, String)>` |
| `testing::drawn_lists` | Every list drawn since the last `reset`, as `(rows, selected)`. | `Vec<(usize, i32)>`; `selected` is `-1` for a list behind a dialog |
| `testing::drawn_list_rows` | The full cell contents of every list drawn since the last `reset`. | `Vec<Vec<RowCells>>` |
| `testing::drawn_popups` | Every option dialog drawn since the last `reset`, as `(title, options, highlighted)`. | `Vec<(String, usize, i32)>` |
| `testing::drawn_hints` | Which of the four button hints were asked for, per draw: `true` where the slot carries a label, `false` where it was left blank. | `Vec<[bool; 4]>` |
| `testing::drawn_sliders` | Every slider drawn since the last `reset`, as `(rect, value, max)`. | `Vec<(Rect, i32, i32)>` |
| `testing::drawn_progress_bars` | Every progress bar drawn since the last `reset`, as `(rect, current, total)`. | `Vec<(Rect, u32, u32)>` |
| `testing::drawn_indicators` | Every scroll indicator asked for since the last `reset`, as `(content, visible, offset)`. | `Vec<(i32, i32, i32)>` |
| `testing::clips` | Every clip set or lifted since the last `reset`, in order. | `Vec<Option<Rect>>`; `None` is a lift, which a scroll view must leave behind |

**Example — asserting a screen's draw calls**

```rust
use xpui::{App, List, ListRow, NavigationScreen, Screen, View, testing};

struct Settings;

impl Screen for Settings {
    type Message = ();
    fn body(&self) -> impl View<()> {
        NavigationScreen::new(
            List::new()
                .push(ListRow::new("Wi-Fi").value("On"))
                .push(ListRow::new("Sleep after").value("5 min")),
        )
    }
    fn update(&mut self, _message: ()) {}
    fn title(&self) -> Option<&'static str> {
        Some("Settings")
    }
}

testing::install();
let mut app = App::new(Settings);
testing::reset();
app.render();

assert_eq!(testing::drawn_headers(), [Some("Settings".to_string())]);
assert_eq!(testing::drawn_lists(), [(2, -1)], "two read-outs, nothing highlighted");
assert_eq!(
    testing::drawn_list_rows()[0][1],
    [Some("Sleep after".to_string()), None, Some("5 min".to_string())],
);
```

## `testing::TextDraw`

One recorded `draw_text`: position, text, font id and style.

```text
pub type TextDraw = (i32, i32, String, i32, u8)
```

The position is the top-left corner of the text, not a baseline. The style is
`FontStyle` as its `u8`.

## `testing::RectDraw`

One recorded rectangle: x, y, width, height, and how it was painted.

```text
pub type RectDraw = (i32, i32, i32, i32, RectKind)
```

## `testing::RowCells`

The title, subtitle and value of a list row, as the theme asked for them.

```text
pub type RowCells = [Option<String>; 3]
```

`None` is a field the row omitted, which is how a host chooses a one- or
two-line row.

**See also:** [`testing::drawn_list_rows`](#draw-accessors), [`testing::DrawOp`](#testingdrawop)
