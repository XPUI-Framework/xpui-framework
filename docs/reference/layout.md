# Layout

The containers that size and place other views. Stacks lay children along one
axis, spacers and padding put space between and around them, and a scroll view
makes a tree taller than the panel reachable. Containers take views **by
value**, so a tree is built without ever writing `Box::new`.

![A settings screen whose content is a column: the heading Storage, the rows Books reading 128 and Free space reading 182 KB, and the line Last synced at 09:14 pushed down to the bottom of the band](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_vstack_footer.png)

[The tutorial](../tutorial.md) builds a screen from these, and
[a second screen](../a-second-screen.md) adds scrolling. This page is what each
piece does. The chainable wrappers, `.frame`, `.flexible` and `.on_tap`, are in
[Modifiers](modifiers.md).

## Topics

| | |
|---|---|
| [`VStack`](#vstack) | Lays children out top to bottom. |
| [`HStack`](#hstack) | Lays children out left to right. |
| [`vstack!`](#vstack-1) | A vertical stack. |
| [`hstack!`](#hstack-1) | A horizontal stack, written the same way as `vstack!`. |
| [`Spacer`](#spacer) | Absorbs the space its siblings do not use, pushing whatever follows to the far end of the stack. |
| [`Padding`](#padding) | Surrounds a child with empty space. |
| [`ScrollView`](#scrollview) | Shows as much of its content as fits, and lets the rest be scrolled to. |
| [`Alignment`](#alignment) | Where children sit across the stacking axis. |
| [`UNBOUNDED`](#unbounded) | The height offered to content that may be as tall as it likes. |

## How a stack measures

`VStack` and `HStack` are one algorithm with the axes swapped, and it runs in
two passes. Most surprises in a layout come from one of them.

1. **Fixed children first.** Every child that is not flexible is measured
   against the *whole* space the stack was offered, not against what the
   children before it left.
2. **Flexible children share the rest.** A [`Spacer`](#spacer), or any view
   made [`.flexible()`](modifiers.md#modifiersflexible), is measured last,
   against an equal share of what the fixed children and the gaps left. A share
   is never negative, and a remainder that does not divide evenly goes unused.

The stack is then as long as its children plus a gap between each pair, and as
wide across as the widest child that counts. A spacer does not count across: it
records the whole cross extent it was offered, and counting it would make an
`hstack!` as tall as the screen.

| When | What happens |
|---|---|
| The fixed children are longer than the space | The stack is longer than it was offered, and nothing clips it. Flexible children get nothing. Wrap the content in a [`ScrollView`](#scrollview). |
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

**See also:** [`HStack`](#hstack), [`vstack!`](#vstack-1), [`Spacer`](#spacer), [`Alignment`](#alignment)

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
> Not inside a [`ScrollView`](#scrollview). Its content is offered
> [`UNBOUNDED`](#unbounded) height, the spacer takes nearly all of it, and the
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

**See also:** [`HStack`](#hstack), [`Spacer`](#spacer), [`Alignment`](#alignment)

## `Spacer`

Absorbs the space its siblings do not use, pushing whatever follows to the far end of the stack.

```text
pub struct Spacer
```

![Page 12 of 240, centred on its row by a spacer on each side](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_spacer.png)

A spacer draws nothing and takes no input. A stack measures it after its fixed
children, against only what they left, so a spacer never pushes content past
the edge. Several spacers in one stack share that space equally. A spacer
carries no message type, so it sits in any stack, and it is `Default`.

Outside a stack a spacer is as big as whatever it is offered. Across its stack
it never counts: a row holding one is as tall as its other children.

**Example — centring with two spacers**

```rust
use xpui::{HStack, Spacer, Text, hstack};

# xpui::testing::install();
let row: HStack<()> = hstack![0; Spacer::new(), Text::new("Page 12 of 240"), Spacer::new()];
```

**Example — a footer**

See [`vstack!`](#vstack-1): a spacer before the last child puts that child on
the bottom edge.

### Creating a spacer

#### `Spacer::new`

A spacer that takes whatever its stack has left.

```text
pub fn new() -> Self
```

**See also:** [`VStack`](#vstack), [`HStack`](#hstack), [`Flexible`](modifiers.md#flexible)

## `Padding`

Surrounds a child with empty space.

```text
pub struct Padding<M>
```

![Sleep after 5 min between two rules, twelve pixels clear of each and of the left edge](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_padding_all.png)

The child is offered what the parent offers, less the insets, and the padding is
as big as the child plus the insets. It draws nothing itself, and moves the
child's touch targets with it.

Padding is as flexible as its child: padding around a spacer is still a spacer.

> [!NOTE]
> Insets are not clamped. Insets larger than the space offered leave the child
> a negative size to measure against.

**Example — the same inset on every edge**

```rust
use xpui::{Divider, Padding, Text, VStack, vstack};

# xpui::testing::install();
let group: VStack<()> = vstack![0;
    Divider::new(),
    Padding::all(Text::new("Sleep after 5 min"), 12),
    Divider::new(),
];
```

**Example — wide sides and a thin top and bottom**

![Sleep after 5 min between two rules, four pixels clear of each and forty pixels in from the left edge](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_padding_symmetric.png)

```rust
use xpui::{Divider, Padding, Text, VStack, vstack};

# xpui::testing::install();
let group: VStack<()> = vstack![0;
    Divider::new(),
    Padding::symmetric(Text::new("Sleep after 5 min"), 40, 4),
    Divider::new(),
];
```

**Example — one edge at a time**

```rust
use xpui::{Insets, Padding, Size, Text, View};

# xpui::testing::install();
let mut text = Text::new("Indented");
View::<()>::measure(&mut text, Size::new(480, 800));
let natural = View::<()>::size(&text);

let insets = Insets { top: 0, right: 0, bottom: 8, left: 24 };
let mut padded: Padding<()> = Padding::new(Text::new("Indented"), insets);
padded.measure(Size::new(480, 800));
assert_eq!(padded.size(), Size::new(natural.width + 24, natural.height + 8));
```

### Creating padding

#### `Padding::all`

The same inset on every edge.

```text
pub fn all(child: impl View<M> + 'static, inset: i32) -> Self where M: 'static,
```

#### `Padding::symmetric`

Independent horizontal and vertical insets.

```text
pub fn symmetric(child: impl View<M> + 'static, horizontal: i32, vertical: i32) -> Self where M: 'static,
```

| Parameter | Meaning |
|---|---|
| `horizontal` | Pixels on the left and on the right. |
| `vertical` | Pixels on the top and on the bottom. |

#### `Padding::new`

`child` inset by `insets`.

```text
pub fn new(child: impl View<M> + 'static, insets: Insets) -> Self where M: 'static,
```

`Insets` has a public field for each edge, and `Insets::all` and
`Insets::symmetric` build the two shapes above; see
[Geometry](geometry.md).

**See also:** [`Frame`](modifiers.md#frame), [`Spacer`](#spacer)

## `ScrollView`

Shows as much of its content as fits, and lets the rest be scrolled to.

```text
pub struct ScrollView<M>
```

![A screen titled Contents listing Chapter 1 to Chapter 15 with Chapter 16 cut off at the bottom of the band, the focus on Chapter 1, and a scroll indicator at the right edge](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_scroll_top.png)

Wrap a screen's content in one and everything below the fold becomes reachable.
**The runtime owns the offset**, beside focus, so a screen never tracks a
scroll position:

- Moving focus scrolls by the least that brings the focused control fully into
  view. Reaching the first control scrolls to the very top, so a heading above
  it is not stranded, and reaching the last scrolls to the very bottom.
- Where focus cannot move, because nothing on the screen is focusable, Up, Down
  and the page keys scroll by half the view instead.
- A vertical swipe moves focus, and so scrolls only as focus does. Content with
  nothing focusable does not scroll by swipe.

The scroll view fills the space it is offered and measures its content against
[`UNBOUNDED`](#unbounded) height, so rows keep their natural size rather than
being squeezed to fit. The furthest it scrolls puts the end of the content at
the bottom of the view, never into blank space. Content is drawn under a clip
the full width of the panel, so what overflows is discarded rather than painted
over the header and the button hints, and the theme draws the scroll indicator
in the side margin. It draws nothing when the content fits.

Anything scrolled out of sight keeps its focus stop, which is how it can be
reached at all, but stops taking touches: its rectangle now lies over the
chrome, where a tap means something else.

> [!IMPORTANT]
> **One per screen, as the root of the content.** The runtime keeps a single
> offset, so a second scroll view would share the first one's position. A
> scroll view is not flexible and takes the whole height it is offered, so
> beside other children in a stack it pushes them off the panel.

> [!WARNING]
> No [`Spacer`](#spacer) in the stack a scroll view holds. Offered unbounded
> height, the spacer takes all of it, and the content scrolls into sixteen
> million pixels of nothing.

**Example — a long list that scrolls**

```rust
use xpui::{List, ListRow, NavigationScreen, Screen, ScrollView, View};

#[derive(Clone, Copy)]
enum Msg {
    Open(usize),
}

struct Contents {
    chapters: Vec<String>, // built once, not on every frame
}

impl Screen for Contents {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        NavigationScreen::new(ScrollView::new(
            List::new().extend(
                self.chapters
                    .iter()
                    .enumerate()
                    .map(|(index, title)| ListRow::new(title.as_str()).on_tap(Msg::Open(index))),
            ),
        ))
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Open(_chapter) => {}
        }
    }

    fn title(&self) -> Option<&'static str> {
        Some("Contents")
    }
}

let screen = Contents {
    chapters: (1..=24).map(|n| format!("Chapter {n}")).collect(),
};
```

After Down is pressed twenty times, the focus is on Chapter 21 and the list has
scrolled just far enough to show it:

![The same screen scrolled five rows: Chapter 6 at the top, the focus on Chapter 21 at the bottom of the band, and the scroll indicator moved down](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_scroll_scrolled.png)

**Example — the band a scroll view occupies**

```rust
use xpui::{List, ListRow, ScrollView, Size, View};

xpui::testing::install();
let rows = (0..40).map(|index| ListRow::new("Setting").on_tap(index));
let mut scroll = ScrollView::new(List::new().extend(rows));

View::<usize>::measure(&mut scroll, Size::new(480, 400));
assert_eq!(
    View::<usize>::size(&scroll),
    Size::new(480, 400),
    "the view occupies the band it was given, however tall its content is"
);
```

### Creating a scroll view

#### `ScrollView::new`

A scrolling window over `content`.

```text
pub fn new(content: impl View<M> + 'static) -> Self
```

The content may be any view: a [`List`](lists.md#list), or a
[`vstack!`](#vstack-1) of sections.

**See also:** [`UNBOUNDED`](#unbounded), [`List`](lists.md#list), [`NavigationScreen`](navigation.md#navigationscreen)

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
> row between two spacers, as [`Spacer`](#spacer) does.

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

## `UNBOUNDED`

The height offered to content that may be as tall as it likes.

```text
pub const UNBOUNDED: i32
```

Sixteen million pixels, more than any real content reaches. A
[`ScrollView`](#scrollview) offers it to its content. A view of your own that
hosts content of any height offers it too.

It is not `i32::MAX`, and the difference matters. A few views echo the height
they were offered straight back, `Spacer`, `Modal` and a nested `ScrollView`
among them, and a stack adds that to its siblings. With `i32::MAX` that
overflows: a panic in a debug build, and a *negative* height or a scroll offset
of two billion in a release one.

```rust
use xpui::{Size, Spacer, Text, UNBOUNDED, VStack, View, vstack};

# xpui::testing::install();
let mut column: VStack<()> = vstack![8; Text::new("Title"), Spacer::new(), Text::new("Footer")];
column.measure(Size::new(480, UNBOUNDED));
assert_eq!(column.size().height, UNBOUNDED, "the spacer took it all, and nothing overflowed");
```

**See also:** [`ScrollView`](#scrollview)
