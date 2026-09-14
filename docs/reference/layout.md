# Layout

The containers that put space between and around other views, and make a tree
taller than the panel reachable: a spacer, padding and a scroll view.
Containers take views **by value**, so a tree is built without ever writing
`Box::new`.

![A settings screen whose content is a column: the heading Storage, the rows Books reading 128 and Free space reading 182 KB, and the line Last synced at 09:14 pushed down to the bottom of the band](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/layout_vstack_footer.png)

[The tutorial](../tutorial.md) builds a screen from these, and
[a second screen](../a-second-screen.md) adds scrolling. The stacks these sit
in are in [Stacks](stacks.md), and the chainable wrappers, `.frame`, `.flexible`
and `.on_tap`, are in [Modifiers](modifiers.md). This page is what each piece
does.

## Topics

| | |
|---|---|
| [`Spacer`](#spacer) | Absorbs the space its siblings do not use, pushing whatever follows to the far end of the stack. |
| [`Padding`](#padding) | Surrounds a child with empty space. |
| [`ScrollView`](#scrollview) | Shows as much of its content as fits, and lets the rest be scrolled to. |
| [`UNBOUNDED`](#unbounded) | The height offered to content that may be as tall as it likes. |

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

See [`vstack!`](stacks.md#vstack-1): a spacer before the last child puts that child on
the bottom edge.

### Creating a spacer

#### `Spacer::new`

A spacer that takes whatever its stack has left.

```text
pub fn new() -> Self
```

**See also:** [`VStack`](stacks.md#vstack), [`HStack`](stacks.md#hstack), [`Flexible`](modifiers.md#flexible)

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
[`vstack!`](stacks.md#vstack-1) of sections.

**See also:** [`UNBOUNDED`](#unbounded), [`List`](lists.md#list), [`NavigationScreen`](navigation.md#navigationscreen)

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
