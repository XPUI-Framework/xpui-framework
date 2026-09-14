# Theme

What a widget asks the installed theme for: the geometry it lays out with, the
furniture the theme paints itself, and the way a control shows what the keys
will do to it next. `Theme` is a façade of associated functions on a
zero-sized type, so nothing threads a host reference through every call.

| Façade | For |
|---|---|
| [`Theme`](#theme) | Themed furniture and the metrics behind it. |
| [`Renderer`](renderer.md#renderer) | Drawing primitives, `screen_size` and `screen_bounds`. Mostly for widget authors. |
| [`ScreenChrome`](renderer.md#screenchrome) | The header band and the button hints. Used by the screen roots. |
| [`Input`](input.md#input-1) | One frame of buttons, touch and gestures. |

`Renderer`, `ScreenChrome` and the two free functions, [`millis`](renderer.md#millis) and
[`request_update`](renderer.md#request_update), are in
[drawing and repainting](renderer.md); the navigation pair is in
[navigation](navigation.md).

[Writing a widget](../writing-a-widget.md) builds a view that draws with the
theme. [The host guide](../host.md) is the other side of these calls: the
traits a backend implements to answer them, listed on
[the backend contract](backend-contract.md). This page is what each piece does.

## Topics

| | |
|---|---|
| [`Theme`](#theme) | The active theme, as widgets reach for it. |
| [`ThemeMetric`](#thememetric-1) | A geometry value from the active theme. |
| [`ControlState`](#controlstate) | What the keys will do to a control next, so a person can see it: a value row on a device with no Left/Right pair changes what four keys mean when it opens, and the panel has to say so. |

## How a façade reaches the host

**Every call goes to whatever was installed.** `Theme::metric` asks the
installed `Chrome`, `Renderer::fill_rect` the installed `Canvas`, `millis` the
installed `Clock`. The façades hold nothing and cache nothing, so a theme the
user changes between frames is the theme the next frame measures with.

**Nothing works before a host is installed.** A call with no host panics,
because a screen cannot be measured before its host exists. A build with the
`testing` feature falls back to the fake host instead, so a test or a doctest
can call these straight away; `testing::install()` says so explicitly.

**Coordinates are logical screen pixels** in the current orientation, with the
origin at the top-left: see [geometry](geometry.md). A view's `render` receives
its absolute origin, so a widget draws at `origin`, never at `0, 0`.

## `Theme`

The active theme, as widgets reach for it.

```text
pub struct Theme
```

**Ask the theme for geometry rather than writing pixel offsets.** A header,
a list row and a slider knob are as tall as the host's theme says, and the
panels this runs on range from 296x128 to 800x480. A screen laid out from
[`content_area`](#themecontent_area) and [`metric`](#thememetric-1) needs no
changes when either the theme or the panel does.

**The host paints the furniture.** A list, a slider, a progress bar, a
sub-header, a scroll indicator and an option popup are drawn by the host's
`Chrome`, not by the framework, so a [Rust](https://rust-lang.org/) screen and a native one are the same
pixels and follow the user's theme. The framework says where each goes and what
it shows; the widgets on the other reference pages call these functions for
you, and a custom widget calls them directly.

| Member | Answers or draws |
|---|---|
| [`metric`](#thememetric-1) | one [`ThemeMetric`](#thememetric), in pixels |
| [`content_area`](#themecontent_area) | the rect between the header and the hints |
| [`draw_sub_header`](#themedraw_sub_header) | a section heading |
| [`draw_progress_bar`](#themedraw_progress_bar) | a progress bar |
| [`draw_slider`](#themedraw_slider) | a slider track and knob |
| [`draw_scroll_indicator`](#themedraw_scroll_indicator) | a scroll indicator |
| [`draw_list`](#themedraw_list) | a list of rows |
| [`draw_option_popup`](#themedraw_option_popup) · [`option_popup_row_rect`](#themeoption_popup_row_rect) | an option popup, and where its rows are |

**Example — laying out from the theme, not from pixels**

```rust
use xpui::{Rect, Renderer, Theme, ThemeMetric, testing};

testing::install();

let content = Theme::content_area(); // between header and hints, already inset
let row = Theme::metric(ThemeMetric::ListRowHeight);
let gap = Theme::metric(ThemeMetric::ListRowGap);

// The third row of a list that starts at the top of the content area.
let third = Rect::new(content.x(), content.y() + 2 * (row + gap), content.width(), row);

assert!(content.contains(third.origin));
assert!(third.bottom() <= content.bottom());
assert!(content.width() < Renderer::screen_size().width);
```

**Example — a heading widget drawn by the theme**

```rust
use xpui::{Point, Rect, Size, Theme, ThemeMetric, View, testing};

/// A section heading, as tall as the theme's heading line.
struct Heading {
    label: &'static str,
    size: Size,
}

impl<M> View<M> for Heading {
    fn measure(&mut self, available: Size) {
        let height = Theme::metric(ThemeMetric::SubHeaderHeight);
        self.size = Size::new(available.width, height);
    }

    fn size(&self) -> Size {
        self.size
    }

    fn render(&self, origin: Point) {
        Theme::draw_sub_header(Rect { origin, size: self.size }, self.label, None);
    }
}

testing::install();
testing::reset();

let content = Theme::content_area();
let mut heading = Heading { label: "Wi-Fi", size: Size::ZERO };
View::<()>::measure(&mut heading, content.size);
View::<()>::render(&heading, content.origin);

let drawn = testing::drawn_sub_headers();
assert_eq!(drawn[0].0.origin, content.origin);
assert_eq!(drawn[0].0.height(), testing::SUB_HEADER_HEIGHT);
assert_eq!(drawn[0].1, "Wi-Fi");
```

### Reading the theme's geometry

#### `Theme::metric`

See [`Chrome::metric`](canvas-and-chrome.md#hostchromemetric).

```text
pub fn metric(metric: ThemeMetric) -> i32
```

One value, in pixels, asked for each time. The table under
[`ThemeMetric`](#thememetric) lists every value and what in the framework reads
it.

#### `Theme::content_area`

The region between the header and the button hints, already inset horizontally by the theme's side padding.

```text
pub fn content_area() -> Rect
```

Its left edge is `ContentSidePadding`, its top `ContentTop`, its bottom
`ContentBottom`, and it is as wide as the panel less the padding on both sides.
It is not inset vertically: `ContentTop` already sits below the header's own
spacing. A theme whose padding leaves no room gives an empty rect, never an
inverted one.

**Example — the content area on the test host**

```rust
use xpui::{Renderer, Theme, ThemeMetric, testing};

testing::install();

let content = Theme::content_area();
let side = Theme::metric(ThemeMetric::ContentSidePadding);

assert_eq!(content.x(), side);
assert_eq!(content.y(), Theme::metric(ThemeMetric::ContentTop));
assert_eq!(content.bottom(), Theme::metric(ThemeMetric::ContentBottom));
assert_eq!(content.right(), Renderer::screen_size().width - side);
```

### Drawing themed controls

These draw into a rect the caller has already laid out. The widgets that wrap
them, [`Section`](lists.md#section), [`ProgressBar`](indicators.md#progressbar),
[`Slider`](controls.md#slider) and [`ScrollView`](layout.md#scrollview), are
what a screen normally uses.

#### `Theme::draw_sub_header`

See [`Chrome::draw_sub_header`](canvas-and-chrome.md#hostchromedraw_sub_header).

```text
pub fn draw_sub_header(rect: Rect, label: &str, right_label: Option<&str>)
```

| Parameter | Meaning |
|---|---|
| `rect` | Where the heading goes. Give it `SubHeaderHeight`, not a list row's height: the theme draws the label top-aligned. |
| `label` | The heading, at the leading edge. |
| `right_label` | A value at the trailing edge, or `None`. |

#### `Theme::draw_progress_bar`

See [`Chrome::draw_progress_bar`](canvas-and-chrome.md#hostchromedraw_progress_bar).

```text
pub fn draw_progress_bar(rect: Rect, current: u32, total: u32)
```

**Example — a bar at the foot of the content area**

```rust
use xpui::{Rect, Theme, ThemeMetric, testing};

testing::install();
testing::reset();

let content = Theme::content_area();
let height = Theme::metric(ThemeMetric::ProgressBarHeight);
let bar = Rect::new(content.x(), content.bottom() - height, content.width(), height);

Theme::draw_progress_bar(bar, 3, 10);
assert_eq!(testing::drawn_progress_bars(), vec![(bar, 3, 10)]);
```

#### `Theme::draw_slider`

See [`Chrome::draw_slider`](canvas-and-chrome.md#hostchromedraw_slider).

```text
pub fn draw_slider(rect: Rect, value: i32, max: i32, state: ControlState)
```

| Parameter | Meaning |
|---|---|
| `rect` | The track's rect. The host draws the knob inside it, `SliderSideInset` in from each end. |
| `value` | How far along, from `0` to `max`. |
| `max` | The value at the right-hand end. |
| `state` | What the keys will do to it next: see [`ControlState`](#controlstate). |

#### `Theme::draw_scroll_indicator`

See [`Chrome::draw_scroll_indicator`](canvas-and-chrome.md#hostchromedraw_scroll_indicator).

```text
pub fn draw_scroll_indicator(rect: Rect, content: i32, visible: i32, offset: i32)
```

| Parameter | Meaning |
|---|---|
| `rect` | The window the content scrolls in. |
| `content` | The content's full height. |
| `visible` | How much of it the window shows. |
| `offset` | How far down the window sits. |

The host draws nothing when `content` already fits in `visible`.

### Drawing lists and popups

#### `Theme::draw_list`

The themed list — see [`Chrome::draw_list`](canvas-and-chrome.md#hostchromedraw_list).

```text
pub fn draw_list<'a>(
    rect: Rect,
    rows: usize,
    selected: i32,
    row: &dyn Fn(usize, RowField) -> Option<&'a str>,
)
```

| Parameter | Meaning |
|---|---|
| `rect` | The space the rows fill. The host draws only rows that fit entirely. |
| `rows` | How many rows there are. |
| `selected` | The highlighted row, or `-1` for none. |
| `row` | Called per visible row and field. `None` omits the field, which is how the host tells a one-line row from a two-line one. |

[`List`](lists.md#list) calls this, and adds focus, taps and the height the
rows need. `RowField` is in `xpui::host`.

**Example — rows from a table**

```rust
use xpui::host::RowField;
use xpui::{Theme, testing};

testing::install();
testing::reset();

let rows = [("Wi-Fi", Some("Home")), ("Bluetooth", None)];
Theme::draw_list(Theme::content_area(), rows.len(), 0, &|index, field| {
    let (title, value) = rows.get(index)?;
    match field {
        RowField::Title => Some(*title),
        RowField::Value => *value,
        RowField::Subtitle => None,
    }
});

assert_eq!(testing::drawn_lists(), vec![(2, 0)]);
```

#### `Theme::draw_option_popup`

See [`Chrome::draw_option_popup`](canvas-and-chrome.md#hostchromedraw_option_popup).

```text
pub fn draw_option_popup<'a>(
    title: &str,
    options: &dyn Fn(usize) -> Option<&'a str>,
    count: usize,
    selected: i32,
)
```

| Parameter | Meaning |
|---|---|
| `title` | The popup's heading. |
| `options` | Called per row for its label. |
| `count` | How many options there are. |
| `selected` | The highlighted option. |

#### `Theme::option_popup_row_rect`

See [`Chrome::option_popup_row_rect`](canvas-and-chrome.md#hostchromeoption_popup_row_rect).

```text
pub fn option_popup_row_rect<'a>(
    title: &str,
    options: &dyn Fn(usize) -> Option<&'a str>,
    count: usize,
    index: usize,
) -> Option<Rect>
```

Pass the same `title`, `options` and `count` the popup was drawn with: the host
lays the dialog out from them. `None` means the row is not on screen.

**Example — which option a tap landed on**

```rust
use xpui::{Theme, testing};

testing::install();

let options = ["Off", "5 min", "15 min"];
let label = |index: usize| options.get(index).copied();
Theme::draw_option_popup("Sleep after", &label, options.len(), 0);

// A tap somewhere inside the third row.
let third = Theme::option_popup_row_rect("Sleep after", &label, options.len(), 2).unwrap();
let tap = third.origin.offset(2, 2);

let hit = (0..options.len()).find(|&index| {
    Theme::option_popup_row_rect("Sleep after", &label, options.len(), index)
        .is_some_and(|row| row.contains(tap))
});
assert_eq!(hit, Some(2));
```

**See also:** [`ThemeMetric`](#thememetric), [`ControlState`](#controlstate), [`Renderer`](renderer.md#renderer), [`Chrome`](canvas-and-chrome.md#hostchrome)

## `ThemeMetric`

A geometry value from the active theme.

```text
pub enum ThemeMetric
```

**Asked for one at a time, by tag.** A host's metrics table has dozens of
fields, and a struct mirroring its layout here would silently read the wrong
value the day one was inserted. Each variant carries a fixed `u8` tag that
crosses the C ABI, which is why the numbers are not in declaration order:
`ListRowGap` was given the next free tag rather than renumbering the rest.

| Variant | Abstract | Tag | Read by |
|---|---|---|---|
| `ThemeMetric::TopPadding` | Gap above the header band. | 0 | `OverlayPanel` |
| `ThemeMetric::HeaderHeight` | Height of the header band. | 1 | `OverlayPanel` |
| `ThemeMetric::VerticalSpacing` | The gap between stacked elements. | 2 | `Stepper`, `OverlayPanel` |
| `ThemeMetric::ButtonHintsHeight` | The band reserved at the bottom for the button hints. | 3 | nothing in the framework: for a screen's own layout |
| `ThemeMetric::ContentSidePadding` | Space between the panel's side edges and the content. | 4 | `Theme::content_area`, `OverlayPanel` |
| `ThemeMetric::ContentTop` | First y below the header that content may use. | 5 | `Theme::content_area` |
| `ThemeMetric::ContentBottom` | First y occupied by the button hints; content must stay above. | 6 | `Theme::content_area` |
| `ThemeMetric::ListRowHeight` | Height of a one-line list row. | 7 | `List`, `Slider`, `Stepper` |
| `ThemeMetric::ListRowHeightWithSubtitle` | Height of a list row carrying a subtitle. | 8 | `List` |
| `ThemeMetric::ListRowGap` | Space the theme leaves between one row and the next. | 14 | `List` |
| `ThemeMetric::ProgressBarHeight` | Height of the themed progress bar. | 9 | `ProgressBar`, when not given a height |
| `ThemeMetric::MinTouchSize` | The smallest comfortably tappable dimension. | 10 | `IconToggle`, and every touch target too small for a finger |
| `ThemeMetric::SliderKnobWidth` | A slider knob's width. | 11 | the conversion from a touch to a slider's value |
| `ThemeMetric::SliderKnobHeight` | A slider knob's height, and so the least height its track needs. | 12 | `Slider` |
| `ThemeMetric::SliderSideInset` | The padding a slider's track is inset by at each end. | 13 | the conversion from a touch to a slider's value |
| `ThemeMetric::SubHeaderHeight` | Height of the band a sub-header needs: the heading's own line, not a list row. | 15 | `Section` |
| `ThemeMetric::SpacingSmall` | The theme's small step, for space *within* a group. | 16 | `Section`, a control's title line |

> [!WARNING]
> A list measured without `ListRowGap` asks for less height than the host
> needs, and the host draws only rows that fit entirely, so the last row
> silently vanishes. The same goes for any widget of your own that stacks
> themed rows.

> [!NOTE]
> `SliderKnobWidth` and `SliderSideInset` are how the framework turns a touch
> into a value, so a host must answer the numbers it actually draws with, or a
> finger lands on one value and the knob shows another.

**Example — the height a list of rows needs**

```rust
use xpui::{List, ListRow, Size, Theme, ThemeMetric, View, testing};

testing::install();

let row = Theme::metric(ThemeMetric::ListRowHeight);
let gap = Theme::metric(ThemeMetric::ListRowGap);

let mut list: List<()> = List::new()
    .push(ListRow::new("Wi-Fi"))
    .push(ListRow::new("Bluetooth"))
    .push(ListRow::new("Airplane mode"));
list.measure(Size::new(400, 1000));

// Three rows and the two gaps between them: none after the last.
assert_eq!(list.size().height, 3 * row + 2 * gap);
```

**Example — separating groups by more than the small step**

```rust
use xpui::{Theme, ThemeMetric, testing};

testing::install();

let within = Theme::metric(ThemeMetric::SpacingSmall);
let between = Theme::metric(ThemeMetric::VerticalSpacing);

// A heading must sit closer to what it heads than to what is above it.
assert!(between > within);
```

**See also:** [`Theme::metric`](#thememetric-1), [`Theme::content_area`](#themecontent_area)

## `ControlState`

What the keys will do to a control next, so a person can see it: a value row on a device with no Left/Right pair changes what four keys mean when it opens, and the panel has to say so.

```text
pub enum ControlState
```

**The framework decides the state and the host draws it.** The runtime passes
one to [`Theme::draw_slider`](#themedraw_slider) on every paint, from what holds
focus and whether a value is open; what each state looks like is the backend's
choice. [The value mode](controls.md#the-value-mode) is what opens a control.

It crosses the C ABI as a `u8`, so a host written in C++ can switch on it. The
default is `Idle`.

| Variant | Abstract | Tag |
|---|---|---|
| `ControlState::Idle` | The keys are elsewhere, so the control shows the value it holds and nothing more. | 0 |
| `ControlState::Focused` | The keys would act on this control if they moved a value now. | 1 |
| `ControlState::Editing` | The control is open: the keys that walked the list are moving this value, Confirm keeps it, and Back drops the edit rather than leaving the screen. | 2 |

> [!NOTE]
> A backend draws `Focused` the way it draws a focused list row, rather than
> inventing a second idiom, and draws `Editing` so that it is distinguishable
> from `Focused` at a glance, or the mode is still invisible.

**Example — a custom track drawn in each state**

```rust
use xpui::{ControlState, Rect, Theme, ThemeMetric, testing};

testing::install();
testing::reset();

let content = Theme::content_area();
let height = Theme::metric(ThemeMetric::SliderKnobHeight);
let track = Rect::new(content.x(), content.y(), content.width(), height);

for state in [ControlState::Idle, ControlState::Focused, ControlState::Editing] {
    Theme::draw_slider(track, 60, 100, state);
}

assert_eq!(testing::drawn_sliders().len(), 3);
assert_eq!(ControlState::default(), ControlState::Idle);
assert_eq!(ControlState::Editing as u8, 2);
```

**See also:** [`Theme::draw_slider`](#themedraw_slider), [`Slider`](controls.md#slider), [`Stepper`](steppers.md#stepper)
