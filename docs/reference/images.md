# Images

The two ways to put a picture on the panel: an `Image`, a 1-bit bitmap you ship
with the application, and an `Icon`, which the host draws from its own registry
when handed an `IconRef`. An image is exactly its own bits and its own size; an
icon's glyph and size are the host's answer, so nothing is scaled and nothing is
guessed.

![A book row: a book icon in outline beside Middlemarch in bold over George Eliot in the small interface face, and a bookmark ribbon at the far edge](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/text_overview.png)

[Text and fonts](text.md) is the other half of what puts something on the
panel. [Canvas and chrome](canvas-and-chrome.md) is how a host draws the bitmaps
and icons these name. This page is what each piece does.

## Topics

| | |
|---|---|
| [`Image`](#image) | A 1-bit bitmap. |
| [`Icon`](#icon) | An icon from the host's registry. |
| [`IconRef`](#iconref) | An icon the host owns. |

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
