# Canvas and chrome

How a host paints. Three of the five traits a backend implements are about
pixels: [`host::Canvas`](#hostcanvas) is the framebuffer,
[`host::TextMetrics`](#hosttextmetrics) finds and measures fonts, and
[`host::Chrome`](#hostchrome) draws the header, the hints, lists, dialogs and
bars on the framework's behalf, asking for a list row a piece at a time by
[`host::RowField`](#hostrowfield). Each is a supertrait of
[`host::Host`](backend-contract.md#hosthost). A screen never calls them: it
reaches them through [`Renderer`](theme.md#renderer) and
[`Theme`](theme.md#theme).

[The backend contract](backend-contract.md) is the rest: the `Host` object,
input, the clock, and installing a host. [Writing a
backend](../writing-a-backend.md) walks through building one, a trait at a
time. This page is each drawing trait, method by method.

## Topics

| | |
|---|---|
| [`host::Canvas`](#hostcanvas) | The framebuffer, as the framework sees it. |
| [`host::TextMetrics`](#hosttextmetrics) | Font lookup and measurement. |
| [`host::Chrome`](#hostchrome) | Chrome the host draws on the framework's behalf. |
| [`host::RowField`](#hostrowfield) | Which piece of a list row is being asked for. |

## `host::Canvas`

The framebuffer, as the framework sees it.

```text
pub trait Canvas
```

Twelve primitives in logical pixels, with the origin at the top-left of the
panel in its current orientation. "Ink" is the foreground colour and
"background" the paper; on a 1-bit panel they are black and white, and a host
with a palette chooses what each means. A widget reaches these through
[`Renderer`](theme.md#renderer), which forwards to the installed host, rather
than holding a host of its own.

Every drawing call honours the clip set by
[`host::Canvas::set_clip`](#hostcanvasset_clip). `xpui-embedded-graphics` makes
one exception, [`host::Canvas::clear`](#hostcanvasclear), which is a
whole-screen act.

### Required methods

#### `host::Canvas::screen_size`

The panel's size in logical pixels, in the current orientation.

```text
fn screen_size(&self) -> Size
```

A rotated panel reports its rotated size. Every screen is laid out against this.

#### `host::Canvas::clear`

Clears to background.

```text
fn clear(&self)
```

A screen painting over what is already there — an overlay — must not call this.
The runtime calls it at the start of each frame of a screen whose
`Screen::is_overlay` is `false`, and skips it otherwise.

#### `host::Canvas::draw_text`

Draws `text` with its **top-left** corner at `origin`.

```text
fn draw_text(&self, origin: Point, text: &str, font: FontId, style: FontStyle)
```

| Parameter | Meaning |
|---|---|
| `origin` | The top-left of the line the text occupies. The line fills `[y, y + line_height)`. |
| `text` | The string to draw, already translated. |
| `font` | A [`FontId`](text-and-images.md#fontid) this host handed out from [`host::TextMetrics::font`](#hosttextmetricsfont). |
| `style` | The [`FontStyle`](text-and-images.md#fontstyle) to draw it in. |

> [!WARNING]
> **Not a baseline.** Most drawing libraries take one. Passing `origin.y`
> straight through compiles, runs, and puts every glyph one line too high,
> while every test that counts draw calls still passes. Add the face's ascent
> on the way in.

What this paints must be as wide as
[`host::TextMetrics::text_width`](#hosttextmetricstext_width) says, or a label
reserved to fit overruns the space it was given. A font this build does not
ship, `FontId::UNAVAILABLE`, measures zero; the fake host draws nothing for it.

#### `host::Canvas::fill_rect`

Fills `rect`; `black` false means background.

```text
fn fill_rect(&self, rect: Rect, black: bool)
```

`true` fills in ink, `false` in background, which is how a widget erases a
region before drawing over it.

#### `host::Canvas::stroke_rect`

Outlines `rect` in ink, one pixel wide, inside its bounds.

```text
fn stroke_rect(&self, rect: Rect)
```

The outline never extends past `rect`, so a stroked and a filled rect of the
same bounds cover the same pixels.

#### `host::Canvas::draw_line`

A one-pixel line in ink from `from` to `to`, both ends included.

```text
fn draw_line(&self, from: Point, to: Point)
```

#### `host::Canvas::fill_rect_dither`

Fills `rect` with a 50% dither, which reads as grey on a 1-bit panel.

```text
fn fill_rect_dither(&self, rect: Rect, light: bool)
```

`light` picks which checkerboard parity takes ink, so two adjacent dithers can
differ. It is a fill: whatever was behind goes, and the pattern is drawn on
background. The shared chrome dithers a slider's track and a scroll indicator's
track with it.

#### `host::Canvas::scrim`

Darkens `rect` while leaving what is already drawn there legible.

```text
fn scrim(&self, rect: Rect)
```

Unlike [`host::Canvas::fill_rect_dither`](#hostcanvasfill_rect_dither), which
clears before it patterns, this only adds ink, on one checkerboard parity, so
about half of what was behind survives. A `Modal` given `Scrim::Dim` pushes
the screen behind it back with this, without repainting that screen.

#### `host::Canvas::set_clip`

Confines drawing to `rect`, or lifts the clip when it is `None`.

```text
fn set_clip(&self, rect: Option<Rect>)
```

A view taller than the space it was given - a scrolling one - relies on this to
keep its overflow off the chrome around it. There is one clip, not a stack:
`Some` replaces whatever was set, and `None` lifts it entirely. A widget pairs
`Renderer::clip` with `Renderer::clear_clip`.

It must really clip. Nothing in the framework checks what a mis-measured widget
paints outside its bounds; the clip is what keeps it off the header, the hints,
and the simulator's drawn bezel.

#### `host::Canvas::draw_image`

Draws a 1-bpp bitmap of `size` with its top-left corner at `origin`.

```text
fn draw_image(&self, origin: Point, data: &[u8], size: Size)
```

| Parameter | Meaning |
|---|---|
| `origin` | Where the bitmap's top-left pixel lands. |
| `data` | The pixels: row-major, MSB first, `(w + 7) / 8` bytes per row. |
| `size` | The bitmap's width and height in pixels, which is what `w` above is. |

> [!WARNING]
> **Bit 0 is ink**, inverted from the usual convention: a set bit is
> background, as on the e-ink panels the format comes from. Only the cleared
> bits are drawn, so a set bit leaves what is behind it.

`xpui-embedded-graphics` refuses a buffer shorter than `size` needs and draws
nothing, rather than a fragment that reads as a corrupt asset.

#### `host::Canvas::draw_icon`

Draws `icon` with its top-left corner at `origin`, at the size `icon_size` answers.

```text
fn draw_icon(&self, origin: Point, icon: IconRef)
```

An [`IconRef`](text-and-images.md#iconref) is an opaque number meaning "the
thing you use for this", and the host chooses the asset. A backend with no
assets draws them from lines and rectangles; `xpui-embedded-graphics` takes
`xpui_chrome::draw_icon`.

#### `host::Canvas::icon_size`

Edge length the host would actually draw, or 0 if it ships nothing for this icon.

```text
fn icon_size(&self, icon: IconRef) -> i32
```

`IconRef::size` is a preference, and the host answers with the nearest size it
ships. `0` makes the icon take no room in a layout, rather than a hole of a
guessed size. The fake host echoes `icon.size`, so icon layout in a test is
predictable.

**See also:** [`host::TextMetrics`](#hosttextmetrics), [`Renderer`](theme.md#renderer)

## `host::TextMetrics`

Font lookup and measurement.

```text
pub trait TextMetrics
```

The framework never estimates a glyph's size. An estimate drifts from what is
painted and pushes content off the panel, so every width and height is asked of
the same font engine [`host::Canvas::draw_text`](#hostcanvasdraw_text) paints
with. A widget reaches these through [`Font`](text-and-images.md#font), which
also answers zero for a font this build lacks without asking the host.

### Required methods

#### `host::TextMetrics::font`

The font for a role, or `FontId::UNAVAILABLE` when this build does not ship one.

```text
fn font(&self, role: FontRole) -> FontId
```

A [`FontRole`](text-and-images.md#fontrole) says what the text is for; the host
decides which face that means.

> [!IMPORTANT]
> **The id must be derived from the face's own bytes**, not from the role and
> not from a counter. A consumer keys a glyph cache on it, so a stable id over
> changed bytes keeps every such cache serving the old face, and nothing
> notices. Two faces that draw the same string differently must not share an id.

**Example — an id from the face's bytes**

```rust
use xpui::host::FontId;

/// FNV-1a over the face, kept clear of 0, which means "not shipped".
fn id_of(face: &[u8]) -> FontId {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in face {
        hash = (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193);
    }
    FontId(match hash as i32 {
        0 => 1,
        id => id,
    })
}

let regular = id_of(b"a face's bytes");
assert!(regular.is_available());
assert_eq!(regular, id_of(b"a face's bytes"), "the same face, the same id");
assert_ne!(regular, id_of(b"a face's other bytes"), "a changed face, a new id");
```

#### `host::TextMetrics::text_width`

The width `text` paints at in `font` and `style`, in pixels.

```text
fn text_width(&self, font: FontId, text: &str, style: FontStyle) -> i32
```

The advance, summed the way `draw_text` sums it, for the same string. A label
that measures narrower than it paints overruns what was reserved for it, and
the framework has no way to find out.

#### `host::TextMetrics::line_height`

The height one line of `font` occupies, ascent and descent included.

```text
fn line_height(&self, font: FontId) -> i32
```

It is asked without a style, so it must hold the tallest style the face is
drawn in: `xpui-embedded-graphics` answers with the band of the face's
tallest style, so a bold taller than its regular stays inside it.

**See also:** [`host::Canvas::draw_text`](#hostcanvasdraw_text), [`Font`](text-and-images.md#font)

## `host::Chrome`

Chrome the host draws on the framework's behalf.

```text
pub trait Chrome
```

The header, the button hints, a list, a dialog, a slider: the framework lays
them out and the host paints them, so a Rust screen and a native one are the
same pixels, and both follow the user's theme without the framework knowing
what a theme is. A host sitting on a component library answers by calling it.
One sitting on a drawing library takes all eleven painting methods from
`xpui-chrome`'s `plain_chrome!`, which paints them with its own `Canvas`.

A screen reaches these through [`Theme`](theme.md#theme) and
[`ScreenChrome`](theme.md#screenchrome), never through the trait.

### Required methods

#### `host::Chrome::metric`

One geometry value from the active theme, in pixels.

```text
fn metric(&self, metric: ThemeMetric) -> i32
```

Asked one [`ThemeMetric`](theme.md#thememetric) at a time rather than mirrored
as a struct. The slider and list metrics must be the numbers the host actually
draws with: the framework converts a touch to a value, and measures a list's
height, from them.

#### `host::Chrome::draw_header`

Draws the header band, including whatever the host puts in it (a battery indicator, say).

```text
fn draw_header(&self, title: Option<&str>, subtitle: Option<&str>)
```

| Parameter | Meaning |
|---|---|
| `title` | The title, or `None` for no title. |
| `subtitle` | A second, smaller label, or `None` for none. `xpui-chrome` right-aligns it in at most half the band. |

A `None` title means no title, and a `None` subtitle no subtitle: with both,
the band is drawn empty. `None` never stands for the screen's own title. The
framework fetches that from the navigator and passes it as `Some`, which is why
both of its own calls, `ScreenChrome::draw_header` and
`ScreenChrome::draw_screen_header`, pass a title. A C++ host can still receive a
null one across the ABI.

**Example — what `None` paints**

```rust
use xpui::host::Chrome;
use xpui::testing::{self, TestHost};

testing::install();
testing::reset();

TestHost.draw_header(None, Some("12:04"));     // the band, with no title
xpui::ScreenChrome::draw_screen_header();      // the screen's own, fetched first

assert_eq!(testing::drawn_headers(), [None, Some("Test".to_string())]);
```

#### `host::Chrome::draw_sub_header`

A section heading in `rect`, with an optional right-aligned value.

```text
fn draw_sub_header(&self, rect: Rect, label: &str, right_label: Option<&str>)
```

`Section` asks for it. The band's height is `ThemeMetric::SubHeaderHeight`, the
heading's own line, and the theme draws the label top-aligned in it.

#### `host::Chrome::draw_button_hints`

The four button hints, given by meaning rather than by position.

```text
fn draw_button_hints(&self, back: &Hint, confirm: &Hint, previous: &Hint, next: &Hint)
```

The host reorders them to match the user's button layout, so a screen never
names a position. Each is a [`Hint`](navigation.md#hint): `Hint::Standard`
means "your standard label for this slot", and `Hint::label` and `Hint::word`
say what to draw. A `None` label means a word of the host's own, and `word`
says which one: the slot's standard label, or Edit, Done or Cancel while a
value control is in play.

#### `host::Chrome::draw_progress_bar`

The themed progress bar, `current` of `total` along.

```text
fn draw_progress_bar(&self, rect: Rect, current: u32, total: u32)
```

`current` can exceed `total`, and `total` can be zero. `xpui-chrome` clamps the
first and draws an empty bar for the second, rather than dividing by it.

#### `host::Chrome::draw_slider`

The themed slider: a track, a fill up to `value`, and a knob over both.

```text
fn draw_slider(&self, rect: Rect, value: i32, max: i32, state: ControlState)
```

The host owns every dimension of it; the framework says only where it goes, how
far along it is, and what the keys will do to it next.

| Parameter | Meaning |
|---|---|
| `rect` | The slider's track bounds. |
| `value` | How far along, in `0..=max`. |
| `max` | The top of the range. A `max` of 0 or less has no position to show. |
| `state` | [`ControlState`](theme.md#controlstate): idle, focused, or open for editing. Each must look different. |

The knob's position must come from `ThemeMetric::SliderKnobWidth` and
`ThemeMetric::SliderSideInset`, the numbers the framework converts a touch
with, or the knob lands somewhere other than the finger.

#### `host::Chrome::draw_scroll_indicator`

The scroll indicator beside a scrolling region: how much of `content` the `rect`-sized window shows, and how far down it sits.

```text
fn draw_scroll_indicator(&self, rect: Rect, content: i32, visible: i32, offset: i32)
```

The host draws nothing when everything already fits.

| Parameter | Meaning |
|---|---|
| `rect` | The scrolling region's viewport. The indicator sits at its edge. |
| `content` | The full height of what scrolls. |
| `visible` | How much of it the viewport shows. |
| `offset` | How far down the content is scrolled, in pixels. |

`ScrollView` calls it after lifting its clip, so the indicator is not trimmed
by the viewport it sits beside.

#### `host::Chrome::draw_list`

The themed list: `rows` rows laid out in `rect`, with the one at `selected` highlighted.

```text
fn draw_list<'a>(&self, rect: Rect, rows: usize, selected: i32, row: &dyn Fn(usize, RowField) -> Option<&'a str>)
```

`row` is called back per visible row and field; returning `None` omits that
field, which is how the host decides between a one- and two-line row. Each
field is a [`host::RowField`](#hostrowfield).

| Parameter | Meaning |
|---|---|
| `rect` | The list's bounds, as tall as its rows measured. |
| `rows` | How many rows there are. |
| `selected` | The index of the highlighted row. An index matching no row highlights none. |
| `row` | The text of row `index`'s field, asked while the list is drawn. |

The callback borrows from the widget that built it, so a host reads the strings
during this call and keeps none of them. `xpui-chrome` stops at the first row
that does not fully fit, and leaves "there is more" to the scroll indicator.

#### `host::Chrome::draw_option_popup`

The themed modal: `title` over `count` options, with the one at `selected` highlighted.

```text
fn draw_option_popup<'a>(&self, title: &str, options: &dyn Fn(usize) -> Option<&'a str>, count: usize, selected: i32)
```

`options` is called back per row. The host centres the dialog, sizes it to its
content, and paints its own background so it stays legible over what is
behind. `Modal` asks for it, and paints any scrim first.

#### `host::Chrome::option_popup_row_rect`

Screen rect of one modal row, so hit-testing does not re-derive the dialog geometry the host already owns.

```text
fn option_popup_row_rect<'a>(&self, title: &str, options: &dyn Fn(usize) -> Option<&'a str>, count: usize, index: usize) -> Option<Rect>
```

Given the same `title`, `options` and `count` as
[`host::Chrome::draw_option_popup`](#hostchromedraw_option_popup), it answers
where row `index` was painted. `None` means the row is not on the panel: past
`count`, or off the bottom of a dialog too tall to show every option. A row the
host did not paint is not touchable.

#### `host::Chrome::request_update`

Asks for a repaint, since e-ink does not refresh on its own.

```text
fn request_update(&self)
```

On the painting trait, not [`Navigator`](navigation.md#navigator): a repaint is
a display concern, and a host that installed no navigator must still refresh.
The framework calls it through `xpui::request_update` on every dispatch that
changes what the panel shows. A host sets a flag here and repaints from its own
loop; it does not paint inside the call.

**Example — counting repaints**

```rust
use xpui::testing;

testing::install();
testing::reset();

xpui::request_update();
assert_eq!(testing::updates(), 1);
```

**See also:** [`Theme`](theme.md#theme), [`ScreenChrome`](theme.md#screenchrome), [`host::RowField`](#hostrowfield)

## `host::RowField`

Which piece of a list row is being asked for.

```text
pub enum RowField
```

The second argument to the callback [`host::Chrome::draw_list`](#hostchromedraw_list)
receives. A host asks for the fields its theme shows, and a `None` answer omits
that field.

| Variant | |
|---|---|
| `host::RowField::Title` | The row's main text. |
| `host::RowField::Subtitle` | The second line, when the row has one. |
| `host::RowField::Value` | A right-aligned value, when the row has one. |

**Example — one line or two**

```rust
use xpui::host::RowField;

/// A host's row height: two lines when the row has a subtitle.
fn row_height<'a>(row: &dyn Fn(usize, RowField) -> Option<&'a str>, index: usize) -> i32 {
    if row(index, RowField::Subtitle).is_some() { 56 } else { 40 }
}

let rows = [("Wi-Fi", Some("Home"), Some("On")), ("Bluetooth", None, Some("Off"))];
let row = |index: usize, field: RowField| match field {
    RowField::Title => Some(rows[index].0),
    RowField::Subtitle => rows[index].1,
    RowField::Value => rows[index].2,
};

assert_eq!(row_height(&row, 0), 56);
assert_eq!(row_height(&row, 1), 40);
```
