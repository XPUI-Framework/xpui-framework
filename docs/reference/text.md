# Text and fonts

A line of text, and the fonts it is drawn in. A font is named by the role the
text plays, never by typeface, and the host answers with a face it registered,
so every width and line height here comes from the font engine that paints the
text and nothing is estimated.

![A book row: a book icon in outline beside Middlemarch in bold over George Eliot in the small interface face, and a bookmark ribbon at the far edge](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/text_overview.png)

[Images](images.md) is the other half of what puts something on the panel: a
bitmap you ship and an icon the host draws. [Canvas and chrome](canvas-and-chrome.md)
is how a host measures and draws the text these name. This page is what each
piece does.

## Topics

| | |
|---|---|
| [`Text`](#text) | Draws a single line of text. |
| [`Font`](#font) | A font plus the style it is drawn in, as widgets reach for it. |
| [`FontId`](#fontid-1) | A font the host has registered, as a number only the host can interpret. |
| [`FontRole`](#fontrole-1) | What a piece of text is *for*, rather than which typeface it uses. |
| [`FontStyle`](#fontstyle) | Weight and slant, as a host-independent value. |

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

**It draws nothing in a missing face.** In
[`Font::UNAVAILABLE`](#fontunavailable) it measures zero and never hands its
string to the host, so no backend paints it in another face over space nothing
reserved.

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
size nobody measured, and a [`Text`](#text) in it draws nothing: the string
never reaches the host, which might otherwise paint it in a face it does know.

A widget of your own that calls `Renderer::draw_text` directly makes the same
check with [`is_available`](#fontis_available).

```rust
use xpui::{Font, Point, Text, View, testing};

testing::install();
testing::reset();

let missing = Font::UNAVAILABLE;
assert!(!missing.is_available());
assert_eq!(missing.text_width("Battery"), 0);
assert_eq!(missing.line_height(), 0);

let mut label = Text::new("Battery").font(missing);
View::<()>::measure(&mut label, testing::screen());
View::<()>::render(&label, Point::ORIGIN);
assert!(testing::drawn_text().is_empty());
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
