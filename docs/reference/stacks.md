# Stacks

The containers that lay children along one axis: a column, a row, the macros
that build either from a list of views, and where children sit across the axis.
Stacks take views **by value**, so a tree is built without ever writing
`Box::new`.

![A settings screen whose content is a column: the heading Storage, the rows Books reading 128 and Free space reading 182 KB, and the line Last synced at 09:14 pushed down to the bottom of the band](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_vstack_footer.png)

[The tutorial](../tutorial.md) builds a screen from these. The space between
and around views, and scrolling, are in [Layout](layout.md), and the chainable
wrappers, `.frame`, `.flexible` and `.on_tap`, are in
[Modifiers](modifiers.md). This page is how a stack measures, and what each
piece does.

## Topics

| | |
|---|---|
| [`VStack`](#vstack) | Lays children out top to bottom. |
| [`HStack`](#hstack) | Lays children out left to right. |
| [`vstack!`](#vstack-1) | A vertical stack. |
| [`hstack!`](#hstack-1) | A horizontal stack, written the same way as `vstack!`. |
| [`Alignment`](#alignment) | Where children sit across the stacking axis. |

## How a stack measures

`VStack` and `HStack` are one algorithm with the axes swapped, and it runs in
two passes. Most surprises in a layout come from one of them.

1. **Fixed children first.** Every child that is not flexible is measured
   against the *whole* space the stack was offered, not against what the
   children before it left.
2. **Flexible children share the rest.** A [`Spacer`](layout.md#spacer), or any view
   made [`.flexible()`](modifiers.md#modifiersflexible), is measured last,
   against an equal share of what the fixed children and the gaps left. A share
   is never negative, and a remainder that does not divide evenly goes unused.

The stack is then as long as its children plus a gap between each pair, and as
wide across as the widest child that counts. A spacer does not count across: it
records the whole cross extent it was offered, and counting it would make an
`hstack!` as tall as the screen.

| When | What happens |
|---|---|
| The fixed children are longer than the space | The stack is longer than it was offered, and nothing clips it. Flexible children get nothing. Wrap the content in a [`ScrollView`](layout.md#scrollview). |
| A stack holds a spacer | The stack is exactly as long as it was offered. |
| A child measures zero | It still has a gap on each side. [`push_if`](#vstackpush_if) leaves out the child and its gap. |
| Children are aligned | Across the stack's own extent, which is its widest child, not the panel. See [`Alignment`](#alignment). |

```rust
use xpui::{Size, Spacer, Text, VStack, View, vstack};

# xpui::testing::install();
let offered = Size::new(480, 800);
let mut line = Text::new("A longer line");
View::<()>::measure(&mut line, offered);
let line = View::<()>::size(&line);

let mut column: VStack<()> = vstack![10; Text::new("Title"), Text::new("A longer line")];
column.measure(offered);
assert_eq!(column.size(), Size::new(line.width, 2 * line.height + 10));

let mut pushed: VStack<()> = vstack![10; Text::new("Title"), Spacer::new(), Text::new("Footer")];
pushed.measure(offered);
assert_eq!(pushed.size().height, 800, "a spacer makes the stack as tall as its offer");
```

`Text` is a view for every message type, so measuring a bare one names the type,
`View::<()>::measure`. A stack knows its own and passes it down.

## `VStack`

Lays children out top to bottom.

```text
pub struct VStack<M>(Stack<M>)
```

![A column of three lines eight pixels apart: Wi-Fi in bold, Connected to Home, and Signal strong](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_vstack.png)

`M` is the screen's message type. Children are drawn in the order they were
added, the first at the top, each against the left edge unless
[`align`](#vstackalign) says otherwise. How tall the column is, and what a
spacer in it does, is [how a stack measures](#how-a-stack-measures).

| Builder | Sets | When not called |
|---|---|---|
| [`VStack::new`](#vstacknew) | the gap between children, in pixels | — |
| [`push`](#vstackpush) · [`push_if`](#vstackpush_if) · [`push_some`](#vstackpush_some) · [`extend`](#vstackextend) | the children, in order | no children: the stack measures zero |
| [`align`](#vstackalign) | where children sit across the column | `Alignment::Start`, the left edge |

The spacing is the framework's to lay out, not the theme's: pass a value from
`Theme::metric` when the gap should follow the device.

**Example — a column built by hand**

```rust
use xpui::{Text, VStack};

# xpui::testing::install();
let column: VStack<()> = VStack::new(8)
    .push(Text::new("Wi-Fi").bold())
    .push(Text::new("Connected to Home"))
    .push(Text::new("Signal strong"));
```

**Example — a line only while it applies**

![A column reading Library in bold, then Syncing…, then 128 books, with no line for the absent warning](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_push_if.png)

```rust
use xpui::{Text, VStack};

# xpui::testing::install();
let syncing = true;
let warning: Option<&str> = None;

let column: VStack<()> = VStack::new(8)
    .push(Text::new("Library").bold())
    .push_if(syncing, Text::new("Syncing…"))
    .push_some(warning.map(Text::new))
    .push(Text::new("128 books"));
```

The chain keeps one type whichever way the conditions go, so `body` needs no
`if` whose branches would have to agree.

**Example — rows from data**

![A column reading Networks in bold, then Home, Office and Library](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_extend.png)

```rust
use xpui::{Text, VStack};

# xpui::testing::install();
let networks = ["Home", "Office", "Library"];

let column: VStack<()> = VStack::new(8)
    .push(Text::new("Networks").bold())
    .extend(networks.into_iter().map(Text::new));
```

**Example — children of different types from data**

```rust
use xpui::{Divider, Text, VStack, View, ViewExt};

# xpui::testing::install();
let rows: Vec<Box<dyn View<()>>> = vec![
    Text::new("Wi-Fi").boxed(),
    Divider::new().boxed(),
    Text::new("Bluetooth").boxed(),
];
let column = VStack::new(4).extend(rows);
```

### Creating a stack

#### `VStack::new`

A vertical stack with `spacing` pixels between children.

```text
pub fn new(spacing: i32) -> Self
```

| Parameter | Meaning |
|---|---|
| `spacing` | Pixels between each pair of children, and none before the first or after the last. |

### Adding children

#### `VStack::push`

Appends a child.

```text
pub fn push(self, child: impl View<M> + 'static) -> Self
```

Takes any view by value and boxes it, so a tree is built without `Box::new`.
The child must be a view for this stack's message type.

#### `VStack::push_if`

Appends a child only when `condition` holds, so callers can build conditional trees without wrapping the whole chain in an `if`.

```text
pub fn push_if(self, condition: bool, child: impl View<M> + 'static) -> Self
```

The child is built either way, and dropped when `condition` is `false`. A child
left out takes no gap.

#### `VStack::push_some`

Appends a child when there is one.

```text
pub fn push_some(self, child: Option<impl View<M> + 'static>) -> Self
```

`None` adds nothing, not even a gap.

#### `VStack::extend`

Appends every view an iterator yields, for lists built at run time.

```text
pub fn extend<V: View<M> + 'static>(self, children: impl IntoIterator<Item = V>) -> Self
```

Every item is one type. To mix types, box each to `Box<dyn View<M>>` first,
which is itself a view; [`ViewExt::boxed`](views.md) is the short way.

### Arranging children

#### `VStack::align`

Positions children across the stacking axis, at `Alignment::Start` unless this is called.

```text
pub fn align(self, alignment: Alignment) -> Self
```

In a column, `Start` is the left edge, `Center` the middle and `End` the right
edge, all of the column's own width.

**See also:** [`HStack`](#hstack), [`vstack!`](#vstack-1), [`Spacer`](layout.md#spacer), [`Alignment`](#alignment)

## `HStack`

Lays children out left to right.

```text
pub struct HStack<M>(Stack<M>)
```

![A row of three words twenty-four pixels apart: Books, Fonts and Notes](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_hstack.png)

The same stack as [`VStack`](#vstack) on the other axis, with the same builders.
Children run from the left, each against the top edge unless
[`align`](#hstackalign) says otherwise: a row mixing an icon with a line of text
almost always wants `Alignment::Center`.

A row with no spacer in it is as wide as its children. A row with one is as wide
as it was offered, which in a column is the width of the band.

**Example — a row built by hand**

```rust
use xpui::{HStack, Text};

# xpui::testing::install();
let row: HStack<()> = HStack::new(24)
    .push(Text::new("Books"))
    .push(Text::new("Fonts"))
    .push(Text::new("Notes"));
```

### Creating a stack

#### `HStack::new`

A horizontal stack with `spacing` pixels between children.

```text
pub fn new(spacing: i32) -> Self
```

### Adding children

#### `HStack::push`

Appends a child.

```text
pub fn push(self, child: impl View<M> + 'static) -> Self
```

#### `HStack::push_if`

Appends a child only when `condition` holds, so callers can build conditional trees without wrapping the whole chain in an `if`.

```text
pub fn push_if(self, condition: bool, child: impl View<M> + 'static) -> Self
```

#### `HStack::push_some`

Appends a child when there is one.

```text
pub fn push_some(self, child: Option<impl View<M> + 'static>) -> Self
```

#### `HStack::extend`

Appends every view an iterator yields, for lists built at run time.

```text
pub fn extend<V: View<M> + 'static>(self, children: impl IntoIterator<Item = V>) -> Self
```

Each behaves as its [`VStack`](#adding-children) counterpart does.

### Arranging children

#### `HStack::align`

Positions children across the stacking axis, at `Alignment::Start` unless this is called.

```text
pub fn align(self, alignment: Alignment) -> Self
```

In a row, `Start` is the top edge, `Center` the middle and `End` the bottom, all
of the row's own height: the tallest child's.

**See also:** [`VStack`](#vstack), [`hstack!`](#hstack-1), [`Alignment`](#alignment)

## `vstack!`

A vertical stack.

```text
macro_rules! vstack
```

```text
vstack![spacing; child, child, …]
vstack![spacing]
```

Shorthand for a column whose children are fixed: it expands to
`VStack::new(spacing)` followed by one `.push(child)` per child, and hides
nothing. A trailing comma is allowed. The second form is an empty stack. Use the
builder when a child depends on a condition or comes from data:
[`push_if`](#vstackpush_if), [`push_some`](#vstackpush_some),
[`extend`](#vstackextend).

**Example — a titled column with a footer**

![A settings screen whose content is a column: the heading Storage, the rows Books reading 128 and Free space reading 182 KB, and the line Last synced at 09:14 pushed down to the bottom of the band](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_vstack_footer.png)

```rust
use xpui::{Spacer, Text, VStack, hstack, vstack};

# xpui::testing::install();
let column: VStack<()> = vstack![12;
    Text::new("Storage").bold(),
    hstack![8; Text::new("Books"), Spacer::new(), Text::new("128")],
    hstack![8; Text::new("Free space"), Spacer::new(), Text::new("182 KB")],
    Spacer::new(),
    Text::new("Last synced at 09:14"),
];
```

The spacer takes the height the other children leave, so the footer sits on the
bottom of whatever band the column is given. Each row's own spacer pushes its
value to the right edge.

> [!WARNING]
> Not inside a [`ScrollView`](layout.md#scrollview). Its content is offered
> [`UNBOUNDED`](layout.md#unbounded) height, the spacer takes nearly all of it, and the
> footer lands sixteen million pixels down.

**Example — a macro stack in a screen**

```rust
use xpui::{NavigationScreen, Screen, Text, View, vstack};

struct About;

impl Screen for About {
    type Message = ();

    fn body(&self) -> impl View<()> {
        NavigationScreen::new(vstack![16;
            Text::new("Firmware").bold(),
            Text::new("1.4.2"),
        ])
    }

    fn update(&mut self, _message: ()) {}

    fn title(&self) -> Option<&'static str> {
        Some("About")
    }
}
```

**See also:** [`VStack`](#vstack), [`hstack!`](#hstack-1)

## `hstack!`

A horizontal stack, written the same way as [`vstack!`](#vstack-1).

```text
macro_rules! hstack
```

```text
hstack![spacing; child, child, …]
hstack![spacing]
```

![A row reading Battery, then 72% in bold, eight pixels to its right](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_hstack_label.png)

It expands to `HStack::new(spacing)` followed by one `.push(child)` per child.

**Example — a label and its value**

```rust
use xpui::{HStack, Text, hstack};

# xpui::testing::install();
let row: HStack<()> = hstack![8; Text::new("Battery"), Text::new("72%").bold()];
```

**Example — an icon-sized frame beside text**

![A sun icon centred in a 44-pixel square, with Frontlight beside it, centred on the same line](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_hstack_icon.png)

```rust
use xpui::{Alignment, HStack, Icon, IconRef, Modifiers, Text, hstack};
# #[derive(Copy, Clone)]
# enum Glyph { Sun }
# impl From<Glyph> for IconRef {
#     fn from(glyph: Glyph) -> IconRef { IconRef::new(glyph as u16) }
# }

# xpui::testing::install();
let row: HStack<()> = hstack![12;
    Modifiers::<()>::frame(Icon::new(Glyph::Sun), 44, 44),
    Text::new("Frontlight"),
]
.align(Alignment::Center);
```

Without `Alignment::Center` the text sits against the top of the 44-pixel row.
The frame names its message type because `Icon` is a view for every one; see
[`Frame`](modifiers.md#frame).

**Example — a spacer between**

![A row reading Battery at the left edge and 72% at the right edge](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_hstack_spacer.png)

```rust
use xpui::{HStack, Spacer, Text, hstack};

# xpui::testing::install();
let row: HStack<()> = hstack![8; Text::new("Battery"), Spacer::new(), Text::new("72%")];
```

**See also:** [`HStack`](#hstack), [`Spacer`](layout.md#spacer), [`Alignment`](#alignment)

## `Alignment`

Where children sit across the stacking axis.

```text
pub enum Alignment
```

![Three rows separated by rules, each a sun icon in a 56-pixel square beside a word: Start with the word against the top of its row, Center with it level with the icon, and End with it against the bottom](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_alignment.png)

| Variant | |
|---|---|
| `Alignment::Start` | Against the leading edge: left in a `vstack!`, top in an `hstack!`. |
| `Alignment::Center` | Centred across the stack. |
| `Alignment::End` | Against the trailing edge: right in a `vstack!`, bottom in an `hstack!`. |

`Start` is the default, which a row mixing a 32-pixel icon with a line of text
almost never wants: the text hangs off the top. It is `Copy`, `Default`,
`Debug` and `Eq`.

> [!NOTE]
> Alignment places a child within **the stack's own extent**, which is its
> widest child, not within the panel. A column of short lines centred is centred
> on the longest line. To centre on the panel, give the column a child as wide
> as the band, such as a row holding a spacer, or centre each line in its own
> row between two spacers, as [`Spacer`](layout.md#spacer) does.

**Example — the three alignments side by side**

```rust
use xpui::{Alignment, Divider, HStack, Icon, IconRef, Modifiers, Text, VStack, hstack, vstack};
# #[derive(Copy, Clone)]
# enum Glyph { Sun }
# impl From<Glyph> for IconRef {
#     fn from(glyph: Glyph) -> IconRef { IconRef::new(glyph as u16) }
# }

fn aligned(alignment: Alignment, label: &'static str) -> HStack<()> {
    hstack![12; Modifiers::<()>::frame(Icon::new(Glyph::Sun), 56, 56), Text::new(label)]
        .align(alignment)
}

# xpui::testing::install();
let rows: VStack<()> = vstack![0;
    aligned(Alignment::Start, "Start"),
    Divider::new(),
    aligned(Alignment::Center, "Center"),
    Divider::new(),
    aligned(Alignment::End, "End"),
];
assert_eq!(Alignment::default(), Alignment::Start);
```

**See also:** [`VStack::align`](#vstackalign), [`HStack::align`](#hstackalign)
