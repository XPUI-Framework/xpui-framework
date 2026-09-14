# Theme

What a widget or a screen asks the installed host for: the theme's geometry,
the furniture the theme paints itself, the framebuffer, the header and hints,
the clock, and a repaint. Each is a façade of associated functions on a
zero-sized type, or a free function, so nothing threads a host reference
through every call.

| Façade | For |
|---|---|
| [`Theme`](#theme) | Themed furniture and the metrics behind it. |
| [`Renderer`](#renderer) | Drawing primitives, `screen_size` and `screen_bounds`. Mostly for widget authors. |
| [`ScreenChrome`](#screenchrome) | The header band and the button hints. Used by the screen roots. |
| [`Input`](input.md#input-1) | One frame of buttons, touch and gestures. |

Plus two free functions, [`millis`](#millis) and
[`request_update`](#request_update), and the navigation pair in
[navigation](navigation.md).

[Writing a widget](../writing-a-widget.md) builds a view that draws through
`Renderer`. [The host guide](../host.md) is the other side of these calls:
the traits a backend implements to answer them, listed on
[the backend contract](backend-contract.md).

## Topics

| | |
|---|---|
| [`Theme`](#theme) | The active theme, as widgets reach for it. |
| [`ThemeMetric`](#thememetric) | A geometry value from the active theme. |
| [`Renderer`](#renderer) | The framebuffer, as widgets reach for it. |
| [`ScreenChrome`](#screenchrome) | The screen's own furniture: header band and button hints. |
| [`ControlState`](#controlstate) | What the keys will do to a control next, so a person can see it: a value row on a device with no Left/Right pair changes what four keys mean when it opens, and the panel has to say so. |
| [`millis`](#millis) | Milliseconds since boot. |
| [`request_update`](#request_update) | Asks for a repaint, which e-ink never does on its own. |

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
`Chrome`, not by the framework, so a Rust screen and a native one are the same
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

**See also:** [`ThemeMetric`](#thememetric), [`ControlState`](#controlstate), [`Renderer`](#renderer), [`Chrome`](canvas-and-chrome.md#hostchrome)

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
fills exactly what it reserved. See [`Font`](text-and-images.md#font).

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
| `font` | The face, from [`Font::id`](text-and-images.md#fontid). |
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

[`Image`](text-and-images.md#image) wraps this.

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

Confines drawing to `rect`.

```text
pub fn clip(rect: Rect)
```

Pair every call with [`Renderer::clear_clip`](#rendererclear_clip). [`ScrollView`](layout.md#scrollview)
clips to its viewport so content taller than the space it was given stays off
the header and the hints.

> [!WARNING]
> A clip is not a stack. A second `clip` replaces the first, and `clear_clip`
> lifts every clip, so a clipping widget inside a `ScrollView` leaves the rest
> of the scroll view unclipped once it clears its own.

#### `Renderer::clear_clip`

Lifts the clip set by [`Renderer::clip`](#rendererclip).

```text
pub fn clear_clip()
```

**See also:** [`Theme`](#theme), [`Canvas`](canvas-and-chrome.md#hostcanvas), [`View`](views.md#view), [geometry](geometry.md)

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

**See also:** [`Theme::draw_slider`](#themedraw_slider), [`Slider`](controls.md#slider), [`Stepper`](controls.md#stepper)

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
