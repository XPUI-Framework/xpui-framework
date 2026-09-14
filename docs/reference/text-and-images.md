# Text and images

The pieces that put words and pictures on the panel: a line of text in a font
the host names by role, a bitmap you ship, and an icon the host draws. Every
size here comes from the host, the font engine for text and the icon registry
for icons, so nothing is estimated and nothing drifts from what is painted.

![A book row: a book icon in outline beside Middlemarch in bold over George Eliot in the small interface face, and a bookmark ribbon at the far edge](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/text_overview.png)

## Topics

| | |
|---|---|
| [`Text`](#text) | Draws a single line of text. |
| [`Font`](#font) | A font plus the style it is drawn in, as widgets reach for it. |
| [`FontId`](#fontid) | A font the host has registered, as a number only the host can interpret. |
| [`FontRole`](#fontrole-1) | What a piece of text is *for*, rather than which typeface it uses. |
| [`FontStyle`](#fontstyle) | Weight and slant, as a host-independent value. |
| [`Image`](#image) | A 1-bit bitmap. |
| [`Icon`](#icon) | An icon from the host's registry. |
| [`IconRef`](#iconref) | An icon the host owns. |

## `Text`

Draws a single line of text.

```text
pub struct Text
```

![Four lines in the interface face: Regular, Bold, Italic drawn upright, and Bold italic drawn bold, because this host has no italic cut](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/text_styles.png)

| Builder | Sets | When not called |
|---|---|---|
| [`Text::new`](#textnew) | the string | — |
| [`font`](#textfont) | the font | `Font::ui()`, the interface face |
| [`bold`](#textbold) · [`italic`](#textitalic) | the style, replacing any set before | regular |

The italic lines in the picture are upright. The family its host draws was
never cut in italic, so the host draws the nearest style it has, which is
[`FontStyle`](#fontstyle)'s rule.

**It is measured by the host.** While the tree is laid out, a `Text` asks its
font for the width of its string and the height of one line, and keeps both.
That is the size it reports, whatever it is offered.

**It is one line, and never shortened.** A string wider than the space it is
given reports its whole width and draws past the edge. Shorten it in `update`,
or put it in a [`ListRow`](lists.md#listrow), whose theme decides what a long
title does.

**It holds the string it was built with.** Build the string in `update` and
keep it. A `format!` inside `body` runs on every paint and every frame carrying
input, and brings `core::fmt` with it.

> [!NOTE]
> `bold` and `italic` each replace the style, so `.bold().italic()` is italic.
> For both, pass `Font::ui().with_style(FontStyle::BoldItalic)` to
> [`font`](#textfont).

**Example — styles**

```rust
use xpui::{Font, FontStyle, Text, VStack, vstack};

# xpui::testing::install();
let styles: VStack<()> = vstack![8;
    Text::new("Regular"),
    Text::new("Bold").bold(),
    Text::new("Italic").italic(),
    Text::new("Bold italic").font(Font::ui().with_style(FontStyle::BoldItalic)),
];
```

**Example — a figure formatted once**

```rust
use xpui::{NavigationScreen, Screen, Text, View, vstack};

struct Battery {
    label: String, // "72%", rebuilt only when the level changes
}

impl Screen for Battery {
    type Message = u8;

    fn body(&self) -> impl View<u8> {
        NavigationScreen::new(vstack![8;
            Text::new("Battery"),
            Text::new(self.label.as_str()).bold(),
        ])
    }

    fn update(&mut self, percent: u8) {
        self.label = format!("{percent}%");
    }

    fn title(&self) -> Option<&'static str> {
        Some("Power")
    }
}

let mut screen = Battery { label: String::new() };
screen.update(72);
assert_eq!(screen.label, "72%");
```

### Creating text

#### `Text::new`

Text in the default interface font.

```text
pub fn new(content: impl Into<String>) -> Self
```

It asks the installed host for the interface font, so a host must be installed
first; the examples on this page install the test host for that reason. A
string containing an interior NUL may render as empty: a host with a C boundary
cannot pass it.

### Font and style

#### `Text::font`

Draws in `font` instead of the default.

```text
pub fn font(self, font: Font) -> Self
```

```rust
use xpui::{Font, Text};

# xpui::testing::install();
let caption = Text::new("Last synced an hour ago").font(Font::ui_small());
let page = Text::new("It is a truth universally acknowledged").font(Font::reader());
```

#### `Text::bold`

Draws bold.

```text
pub fn bold(self) -> Self
```

#### `Text::italic`

Draws italic.

```text
pub fn italic(self) -> Self
```

**See also:** [`Font`](#font), [`FontRole`](#fontrole-1), [`ListRow`](lists.md#listrow)

## `Font`

A font plus the style it is drawn in, as widgets reach for it.

```text
pub struct Font
```

Fonts are named by what the text is *for*, never by typeface. Which face a
[`FontRole`](#fontrole-1) resolves to is the host's business, and a role survives
the host's assets being changed. A `Font` is `Copy` and compares by face and
style.

| To | Call |
|---|---|
| choose a face | [`role`](#fontrole) · [`ui`](#fontui) · [`ui_small`](#fontui_small) · [`reader`](#fontreader) |
| set the style | [`bold`](#fontbold) · [`italic`](#fontitalic) · [`with_style`](#fontwith_style) |
| measure | [`text_width`](#fonttext_width) · [`line_height`](#fontline_height) · [`is_available`](#fontis_available) |

**Never estimate a width.** Measure through the font: an estimate drifts from
what is painted, and content drifts off the panel with it. [`Text`](#text)
measures this way for you.

**Example — naming fonts by role**

```rust
use xpui::{Font, FontRole, FontStyle};

xpui::testing::install();
let ui = Font::ui(); // interface text: the widget default
let small = Font::ui_small(); // captions, secondary labels
let reader = Font::reader(); // the face the user reads in
let bold = Font::ui().bold(); // also .italic()
let both = Font::ui().with_style(FontStyle::BoldItalic);

assert_eq!(Font::role(FontRole::Ui), ui);
assert_eq!(both.id(), ui.id(), "a style is not another face");
assert!(ui.text_width("Battery") > 0);
assert!(ui.line_height() > 0);
```

**Example — choosing the face that fits**

```rust
use xpui::{Font, Text, Theme};

xpui::testing::install();
let title = "The Collected Poems of Wallace Stevens";
let room = Theme::content_area().width();
let font = if Font::ui().text_width(title) <= room { Font::ui() } else { Font::ui_small() };
let label = Text::new(title).font(font);
```

### Choosing a face

#### `Font::role`

The host's font for `role`, in regular style.

```text
pub fn role(role: FontRole) -> Self
```

It asks the installed host when it is called, and keeps the answer.

#### `Font::ui`

Interface text: the default for every widget.

```text
pub fn ui() -> Self
```

#### `Font::ui_small`

Smaller interface text, for captions and secondary labels.

```text
pub fn ui_small() -> Self
```

#### `Font::reader`

The face the user chose for reading.

```text
pub fn reader() -> Self
```

#### `Font::UNAVAILABLE`

A font this build does not ship, which measures zero.

```text
pub const UNAVAILABLE: Font = Font
```

A build may compile a face out, and the host answers its role with this. A
missing face therefore takes no room in a layout, rather than being given a
size nobody measured.

> [!WARNING]
> **It measures zero; it does not promise to draw nothing.** A `Text` in this
> font still hands its string to the host, and what the host paints for a face
> it does not know is the host's decision: one draws nothing, another falls
> back to its interface face, over a space nothing reserved. Check
> [`is_available`](#fontis_available) before putting text in a face that may be
> missing.

```rust
use xpui::Font;

let missing = Font::UNAVAILABLE;
assert!(!missing.is_available());
assert_eq!(missing.text_width("Battery"), 0);
assert_eq!(missing.line_height(), 0);
```

### Style

#### `Font::bold`

The same font in bold, replacing any style set before.

```text
pub fn bold(self) -> Self
```

#### `Font::italic`

The same font in italic, replacing any style set before.

```text
pub fn italic(self) -> Self
```

#### `Font::with_style`

The same font in `style`.

```text
pub fn with_style(self, style: FontStyle) -> Self
```

The only way to ask for [`FontStyle::BoldItalic`](#fontstyle).

### Measuring

#### `Font::text_width`

The width `text` paints at, or 0 for a face this build lacks.

```text
pub fn text_width(self, text: &str) -> i32
```

In pixels, in this font's style: bold text is usually wider.

#### `Font::line_height`

The height one line occupies, or 0 for a face this build lacks.

```text
pub fn line_height(self) -> i32
```

Ascent and descent included, so lines stacked at this pitch do not touch.

#### `Font::is_available`

Whether this build ships the face.

```text
pub fn is_available(self) -> bool
```

### Reading a font

What a host is handed when it draws. A screen has no need of these.

#### `Font::id`

The host's id for the face.

```text
pub fn id(self) -> FontId
```

#### `Font::style`

The weight and slant it is drawn in.

```text
pub fn style(self) -> FontStyle
```

**See also:** [`Text`](#text), [`FontRole`](#fontrole-1), [`FontStyle`](#fontstyle), [`FontId`](#fontid)

## `FontId`

A font the host has registered, as a number only the host can interpret.

```text
pub struct FontId(pub i32)
```

The host's side of a [`Font`](#font). A host's `TextMetrics::font` answers a
role with one, and receives it back with every measurement and every string it
draws; see [writing a backend](../writing-a-backend.md). `0` means "this build
does not ship that font". The number is public, `FontId(7)`, because a host
makes them; a screen only ever receives them.

```rust
use xpui::{Font, FontId};

assert!(FontId(7).is_available());
assert!(!FontId::UNAVAILABLE.is_available());
assert_eq!(Font::UNAVAILABLE.id(), FontId::UNAVAILABLE);
```

### Constants and checks

#### `FontId::UNAVAILABLE`

The font a build compiled out, which measures zero.

```text
pub const UNAVAILABLE: FontId = FontId(0)
```

#### `FontId::is_available`

Whether this build ships the font.

```text
pub fn is_available(self) -> bool
```

**See also:** [`Font`](#font), [`Font::UNAVAILABLE`](#fontunavailable)

## `FontRole`

What a piece of text is *for*, rather than which typeface it uses.

```text
pub enum FontRole
```

![Three lines: Interface text, a smaller caption beneath it, and a line in the face chosen for reading](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/text_roles.png)

The host decides what each role means. Naming families here would tie the
framework to one product's assets, and a role survives those assets being
changed.

| Variant | |
|---|---|
| `FontRole::Ui` | Interface text: labels, list rows, values. |
| `FontRole::UiSmall` | Smaller interface text, for captions and secondary labels. |
| `FontRole::Reader` | The face the user chose for reading, including fonts loaded from storage. |

Each has a shorthand on `Font`: [`ui`](#fontui), [`ui_small`](#fontui_small) and
[`reader`](#fontreader).

```rust
use xpui::{Font, FontRole, Text, VStack, vstack};

# xpui::testing::install();
assert_eq!(Font::role(FontRole::UiSmall), Font::ui_small());

let roles: VStack<()> = vstack![8;
    Text::new("Interface text"),
    Text::new("A caption beneath it").font(Font::role(FontRole::UiSmall)),
    Text::new("The face chosen for reading").font(Font::role(FontRole::Reader)),
];
```

**See also:** [`Font::role`](#fontrole), [`FontStyle`](#fontstyle)

## `FontStyle`

Weight and slant, as a host-independent value.

```text
pub enum FontStyle
```

| Variant | |
|---|---|
| `FontStyle::Regular` | Upright, regular weight. |
| `FontStyle::Bold` | Heavier weight. |
| `FontStyle::Italic` | Slanted. |
| `FontStyle::BoldItalic` | Both. |

`Regular` is the default. The type is `#[repr(u8)]`, and the numbers are what a
host with a C boundary receives: `Regular` is 0, `Bold` 1, `Italic` 2 and
`BoldItalic` 3. A host without a face in that style draws what it has.

```rust
use xpui::{Font, FontStyle};

# xpui::testing::install();
assert_eq!(FontStyle::default(), FontStyle::Regular);
assert_eq!(Font::ui().bold().italic().style(), FontStyle::Italic, "the last one wins");
assert_eq!(Font::ui().with_style(FontStyle::BoldItalic).style() as u8, 3);
```

**See also:** [`Font::with_style`](#fontwith_style), [`FontRole`](#fontrole-1)

## `Image`

A 1-bit bitmap.

```text
pub struct Image
```

![A small solid bookmark ribbon beside the word Bookmarked](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/text_image.png)

A picture you ship yourself, where an [`Icon`](#icon) is one the host ships.

**The buffer format.** Row-major, top row first, most significant bit first
within each byte, `(width + 7) / 8` bytes per row, with each row padded on its
own. **Bit 0 is ink and bit 1 is white.** That is inverted from the usual
convention, so a buffer produced elsewhere will very likely render as a
negative.

**It is borrowed, not copied.** Assets are arrays in flash, and copying one
onto a small heap to draw it is waste, so the data is `&'static`.

**It is exactly its own size.** Nothing scales a bitmap, so it measures as its
width and height whatever it is offered. A buffer too short for those
dimensions measures zero and draws nothing, rather than being read past its
end.

**Example — a bookmark from sixteen rows**

Written with 1 as ink, which reads, and inverted as it is packed.

```rust
use xpui::{Alignment, HStack, Image, Text, hstack};

const RIBBON: [u16; 16] = [
    0b0011111111111100,
    0b0011111111111100,
    0b0011111111111100,
    0b0011111111111100,
    0b0011111111111100,
    0b0011111111111100,
    0b0011111111111100,
    0b0011111111111100,
    0b0011111111111100,
    0b0011111111111100,
    0b0011111111111100,
    0b0011111001111100,
    0b0011110000111100,
    0b0011100000011100,
    0b0011000000001100,
    0b0010000000000100,
];

static BOOKMARK: [u8; 32] = {
    let mut bytes = [0; 32];
    let mut row = 0;
    while row < 16 {
        let [high, low] = (!RIBBON[row]).to_be_bytes(); // bit 0 is ink
        bytes[row * 2] = high;
        bytes[row * 2 + 1] = low;
        row += 1;
    }
    bytes
};

# xpui::testing::install();
let row: HStack<()> =
    hstack![8; Image::new(&BOOKMARK, 16, 16), Text::new("Bookmarked")].align(Alignment::Center);
```

**Example — a buffer too short for its size**

```rust
use xpui::{Image, Size, View};

static SHORT: [u8; 4] = [0; 4];

let mut image = Image::new(&SHORT, 16, 16); // needs 32 bytes
View::<()>::measure(&mut image, Size::new(480, 800));
assert_eq!(View::<()>::size(&image), Size::ZERO);
```

### Creating an image

#### `Image::new`

Wraps a bitmap of `width` x `height` pixels.

```text
pub fn new(data: &'static [u8], width: i32, height: i32) -> Self
```

| Parameter | Meaning |
|---|---|
| `data` | The packed rows, in the format above. Longer than needed is fine. |
| `width` | Pixels across. Each row takes `(width + 7) / 8` bytes. |
| `height` | Rows. |

**See also:** [`Icon`](#icon)

## `Icon`

An icon from the host's registry.

```text
pub struct Icon
```

![Four icons in a row: a sun in outline, the same sun solid, a book in outline, and a smaller solid battery](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/text_icons.png)

**An icon is chosen by what it means, not by file name.** The framework passes
the host an opaque [`IconRef`](#iconref), and the host decides which glyph that
is. A backend publishes its own names and converts them into an `IconRef`, so a
screen writes `Icon::new(Glyph::Sun)` and never a number. The picture above is
drawn by the gallery's backend, whose glyphs are lines and rectangles.

| Builder | Sets | When not called |
|---|---|---|
| [`Icon::new`](#iconnew) | which icon | — |
| [`filled`](#iconfilled) | solid rather than outline | the icon's first variant |
| [`size`](#iconsize) | the edge length asked for | 32 pixels |

**Its size is the host's answer.** An icon measures as the edge the host says it
would draw, square. An icon the host ships nothing for measures zero and draws
nothing, rather than painting something arbitrary at a guessed size.

It is kept apart from [`Image`](#image) because the two use different asset
conventions and are not interchangeable. An icon to switch on and off is
`IconToggle`, in [controls](controls.md).

**Example — a backend's glyphs**

```rust
use xpui::{Icon, IconRef};

/// What a backend publishes. The framework only ever sees the number.
#[derive(Copy, Clone)]
enum Glyph {
    Sun = 0,
    Book = 11,
}

impl From<Glyph> for IconRef {
    fn from(glyph: Glyph) -> IconRef {
        IconRef::new(glyph as u16)
    }
}

# xpui::testing::install();
let light_on = true;
let outline_or_solid = Icon::new(Glyph::Sun).filled(light_on); // solid means on
let small = Icon::new(Glyph::Book).size(24); // outline, the first variant
```

**Example — an icon beside its label**

Centre a row that mixes an icon with a line of text, or the text hangs off the
top.

```rust
use xpui::{Alignment, Font, HStack, Icon, IconRef, Spacer, Text, VStack, hstack, vstack};
# #[derive(Copy, Clone)]
# enum Glyph { Book = 11 }
# impl From<Glyph> for IconRef {
#     fn from(glyph: Glyph) -> IconRef { IconRef::new(glyph as u16) }
# }

# xpui::testing::install();
let book: HStack<()> = hstack![12;
    Icon::new(Glyph::Book),
    vstack![4; Text::new("Middlemarch").bold(), Text::new("George Eliot").font(Font::ui_small())],
    Spacer::new(),
]
.align(Alignment::Center);
```

### Creating an icon

#### `Icon::new`

An icon for a role, asking the host for a 32-pixel edge.

```text
pub fn new(icon: impl Into<IconRef>) -> Self
```

### Appearance

#### `Icon::filled`

Solid rather than outline, where the role has both variants.

```text
pub fn filled(self, filled: bool) -> Self
```

| Parameter | Meaning |
|---|---|
| `filled` | `true` asks for variant 1, solid. `false` asks for variant 0, outline. |

Solid for on and outline for off is the convention, which is why this takes a
`bool`: `.filled(self.light_on)`. An icon with one variant draws the same
either way.

#### `Icon::size`

Preferred edge length; the host draws the nearest size it ships.

```text
pub fn size(self, size: i32) -> Self
```

| Parameter | Meaning |
|---|---|
| `size` | In pixels. The host may round it, or answer zero for a size too small to draw. |

**See also:** [`IconRef`](#iconref), [`Image`](#image)

## `IconRef`

An icon the host owns.

```text
pub struct IconRef
```

Deliberately opaque numbers: which glyph they select is the host's business, and
naming them in the framework would tie it to one product's assets. This is what
an [`Icon`](#icon) hands the host to measure and to draw.

| Field | |
|---|---|
| `IconRef::kind` | Which icon, in whatever numbering the host uses. |
| `IconRef::variant` | A variant of the same icon, where the host offers one (solid vs outline). |
| `IconRef::size` | Preferred edge length; the host picks the nearest size it ships. |

A backend's own enum of glyphs, converted with `From`, is how a screen names
one; see the [example on `Icon`](#icon). The fields are public so a host can
read them.

```rust
use xpui::IconRef;

let battery = IconRef::new(9);
assert_eq!((battery.kind, battery.variant, battery.size), (9, 0, 32));

let solid_and_small = IconRef { variant: 1, size: 16, ..battery };
assert_eq!(solid_and_small.kind, 9);
```

### Creating a reference

#### `IconRef::new`

Icon `kind` in its first variant, asking for a 32-pixel edge.

```text
pub fn new(kind: u16) -> Self
```

**See also:** [`Icon`](#icon)
