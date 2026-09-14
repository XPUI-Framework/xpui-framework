# Geometry

The four value types layout and drawing share: a position, a size, a rectangle
and an inset. All coordinates are `i32` logical screen pixels in the current
orientation, the same space the host draws in, with the origin at the top-left
corner and `y` growing downwards.

Never assume a panel size or shape. The panels this runs on are portrait and
landscape, from 296x128 to 800x480; ask
[`Renderer::screen_size`](theme.md#rendererscreen_size), or better, lay out
from [`Theme::content_area`](theme.md#themecontent_area).
[Writing a widget](../writing-a-widget.md) uses all four.

## Topics

| | |
|---|---|
| [`Point`](#point) | A position on screen. |
| [`Size`](#size) | A width and height, clamped at zero by every constructor. |
| [`Rect`](#rect) | A positioned, sized region. |
| [`Insets`](#insets) | Space inset equally or individually on each edge. |

## Edges and clamping

**A rect's right and bottom edges are outside it.** A rect at `x` with width
`w` covers the columns `x` to `x + w - 1`; [`Rect::right`](#rectright) is
`x + w`, the first column outside. [`contains`](#rectcontains) and
[`intersects`](#rectintersects) agree on this, so two rects that merely touch
do not overlap, and rects laid edge to edge tile without a shared pixel.

**A size never goes below zero.** [`Size::new`](#sizenew),
[`Size::shrink`](#sizeshrink) and everything built on them clamp each dimension
at zero, so a layout that over-subtracts gives an empty rectangle rather than
an inverted one. A position is not clamped, and neither is an inset.

> [!WARNING]
> The fields are public, and a struct literal is not clamped:
> `Size { width: -4, height: 10 }` is negative, and `is_empty` says it is not
> empty. Build sizes from computed numbers with `Size::new`.

All four types are `Copy`, `Default` (all zeros), `Debug` and `Eq`.

## `Point`

A position on screen.

```text
pub struct Point
```

| Field or constant | Abstract |
|---|---|
| `Point::x` | Pixels from the left edge. |
| `Point::y` | Pixels from the top edge. |
| `Point::ORIGIN` | The top-left corner of the screen. |

A point may be negative or beyond the panel: a view scrolled up past the top
of its viewport renders at a negative `y`.

**Example — moving a point**

```rust
use xpui::Point;

let origin = Point::new(16, 60);
assert_eq!(origin.offset(4, -8), Point::new(20, 52));
assert_eq!(Point::default(), Point::ORIGIN);
```

### Creating a point

#### `Point::new`

A point at `x`, `y`.

```text
pub const fn new(x: i32, y: i32) -> Self
```

### Moving a point

#### `Point::offset`

This point moved by `dx`/`dy`.

```text
pub const fn offset(self, dx: i32, dy: i32) -> Self
```

Either may be negative. A widget uses it to place a child inside its own
padding: `origin.offset(pad, pad)`.

**See also:** [`Rect`](#rect), [`Rect::contains`](#rectcontains)

## `Size`

A width and height, clamped at zero by every constructor.

```text
pub struct Size
```

| Field or constant | Abstract |
|---|---|
| `Size::width` | Pixels across. |
| `Size::height` | Pixels down. |
| `Size::ZERO` | Nothing at all. |

A size is what a view's `measure` receives and what its `size` answers. It has
no position: a [`Rect`](#rect) pairs one with an origin.

**Example — clamping at zero**

```rust
use xpui::Size;

assert_eq!(Size::new(-10, 20), Size::new(0, 20));
assert_eq!(Size::new(0, 0), Size::ZERO);
assert!(Size::new(-10, 20).is_empty());
```

### Creating a size

#### `Size::new`

A size of `width` by `height`, each clamped at zero.

```text
pub fn new(width: i32, height: i32) -> Self
```

### Shrinking a size

#### `Size::shrink`

This size shrunk by `dw`/`dh`, clamped at zero.

```text
pub fn shrink(self, dw: i32, dh: i32) -> Self
```

It saturates at zero rather than going negative. A negative `dw` or `dh`
grows the size instead.

**Example — shrinking past zero**

```rust
use xpui::Size;

let available = Size::new(100, 40);
assert_eq!(available.shrink(32, 8), Size::new(68, 32));

// Over-subtracting saturates, one dimension at a time.
assert_eq!(available.shrink(120, 0), Size::new(0, 40));

// A negative amount grows it.
assert_eq!(available.shrink(-10, -10), Size::new(110, 50));
```

### Testing a size

#### `Size::is_empty`

Whether either dimension is zero, so nothing could be drawn in it.

```text
pub fn is_empty(self) -> bool
```

**Example — a row with no height is empty**

```rust
use xpui::Size;

assert!(Size::new(200, 0).is_empty());
assert!(Size::ZERO.is_empty());
assert!(!Size::new(1, 1).is_empty());
```

**See also:** [`Rect`](#rect), [`Insets`](#insets)

## `Rect`

A positioned, sized region.

```text
pub struct Rect
```

| Field | Abstract |
|---|---|
| `Rect::origin` | The top-left corner. |
| `Rect::size` | The width and height. |

The origin may be anywhere, negative included; the size is clamped at zero by
[`Rect::new`](#rectnew) and [`Rect::inset`](#rectinset). Its right and bottom
edges are outside it: see [Edges and clamping](#edges-and-clamping). A view
builds one from what `render` receives, `Rect { origin, size: self.size }`.

**Example — edges**

```rust
use xpui::{Point, Rect, Size};

let card = Rect::new(10, 20, 200, 80);

assert_eq!((card.x(), card.y()), (10, 20));
assert_eq!((card.width(), card.height()), (200, 80));
assert_eq!((card.right(), card.bottom()), (210, 100));
assert_eq!(card.origin, Point::new(10, 20));
assert_eq!(card.size, Size::new(200, 80));

// A negative size is clamped; a negative origin is not.
assert_eq!(Rect::new(-5, 0, -1, 10).size, Size::new(0, 10));
assert_eq!(Rect::new(-5, 0, -1, 10).x(), -5);
```

**Example — hit-testing a tap**

```rust
use xpui::{Point, Rect};

let button = Rect::new(0, 0, 200, 80);

assert!(button.contains(Point::new(0, 0)));      // the top-left corner is inside
assert!(button.contains(Point::new(199, 79)));   // the last pixel is inside
assert!(!button.contains(Point::new(200, 10)));  // the right edge is outside
assert!(!button.contains(Point::new(10, 80)));   // the bottom edge is outside
```

### Creating a rect

#### `Rect::new`

A rect with its top-left corner at `x`, `y`.

```text
pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self
```

`width` and `height` are clamped at zero, as by [`Size::new`](#sizenew).

### Reading its edges

#### `Rect::x`

The left edge.

```text
pub const fn x(&self) -> i32
```

#### `Rect::y`

The top edge.

```text
pub const fn y(&self) -> i32
```

#### `Rect::width`

Pixels across.

```text
pub const fn width(&self) -> i32
```

#### `Rect::height`

Pixels down.

```text
pub const fn height(&self) -> i32
```

#### `Rect::right`

The first column outside the rect.

```text
pub const fn right(&self) -> i32
```

`x() + width()`. The last column inside is one less.

#### `Rect::bottom`

The first row outside the rect.

```text
pub const fn bottom(&self) -> i32
```

`y() + height()`. The next row of a stack starts here, with no gap and no
overlap.

**Example — stacking rows edge to edge**

```rust
use xpui::Rect;

let first = Rect::new(16, 60, 448, 40);
let second = Rect::new(16, first.bottom(), 448, 40);

assert_eq!(second.y(), 100);
assert!(!first.intersects(second));
```

### Insetting

#### `Rect::inset`

This rect pulled inward on every edge by `insets`.

```text
pub fn inset(&self, insets: Insets) -> Self
```

The origin moves by `left` and `top`, and the size shrinks by
[`horizontal`](#insetshorizontal) and [`vertical`](#insetsvertical), clamped at
zero. Negative insets push the rect outward.

**Example — padding, too much padding, and an outset**

```rust
use xpui::{Insets, Rect, Size};

let card = Rect::new(0, 0, 200, 80);

assert_eq!(card.inset(Insets::all(8)), Rect::new(8, 8, 184, 64));

// More inset than there is room: the size stops at zero.
let crushed = card.inset(Insets::symmetric(120, 0));
assert_eq!(crushed.size, Size::new(0, 80));
assert!(crushed.size.is_empty());

// Negative insets grow it, as a touch target larger than its glyph does.
assert_eq!(card.inset(Insets::all(-4)), Rect::new(-4, -4, 208, 88));
```

### Hit-testing

#### `Rect::contains`

Whether `point` is inside; the right and bottom edges are outside.

```text
pub fn contains(&self, point: Point) -> bool
```

An empty rect contains nothing.

#### `Rect::intersects`

Whether the two overlap at all.

```text
pub fn intersects(&self, other: Rect) -> bool
```

Touching edges do not count, matching [`contains`](#rectcontains), which treats
the right and bottom edges as outside.

> [!NOTE]
> Only the edges are compared, so an empty rect strictly inside another
> intersects it, although it contains no point. Check
> [`Size::is_empty`](#sizeis_empty) first where that matters.

**Example — overlap**

```rust
use xpui::{Point, Rect};

let a = Rect::new(0, 0, 100, 100);

assert!(a.intersects(Rect::new(99, 99, 10, 10)));   // one pixel shared
assert!(!a.intersects(Rect::new(100, 0, 10, 10)));  // touching the right edge
assert!(!a.intersects(Rect::new(0, 100, 10, 10)));  // touching the bottom edge
assert!(a.intersects(Rect::new(-10, -10, 200, 200))); // around it
assert!(a.intersects(Rect::new(50, 50, 0, 0)));     // empty, but inside
assert!(!Rect::new(50, 50, 0, 0).contains(Point::new(50, 50)));
```

**See also:** [`Point`](#point), [`Size`](#size), [`Insets`](#insets), [`Renderer::clip`](theme.md#rendererclip)

## `Insets`

Space inset equally or individually on each edge.

```text
pub struct Insets
```

| Field or constant | Abstract |
|---|---|
| `Insets::top` | Pixels taken from the top edge. |
| `Insets::right` | Pixels taken from the right edge. |
| `Insets::bottom` | Pixels taken from the bottom edge. |
| `Insets::left` | Pixels taken from the left edge. |
| `Insets::ZERO` | No inset on any edge. |

Insets are not clamped: a negative one grows a rect under
[`Rect::inset`](#rectinset). [`Padding`](layout.md#padding) takes one.

**Example — building insets**

```rust
use xpui::Insets;

let even = Insets::all(8);
assert_eq!((even.top, even.right, even.bottom, even.left), (8, 8, 8, 8));

let wide = Insets::symmetric(16, 4);
assert_eq!((wide.top, wide.right, wide.bottom, wide.left), (4, 16, 4, 16));

let header = Insets { top: 40, ..Insets::ZERO };
assert_eq!(header.vertical(), 40);
assert_eq!(Insets::default(), Insets::ZERO);
```

### Creating insets

#### `Insets::all`

The same inset on all four edges.

```text
pub const fn all(value: i32) -> Self
```

#### `Insets::symmetric`

Independent horizontal and vertical insets.

```text
pub const fn symmetric(horizontal: i32, vertical: i32) -> Self
```

| Parameter | Meaning |
|---|---|
| `horizontal` | Taken from the left and from the right, each. |
| `vertical` | Taken from the top and from the bottom, each. |

### Reading totals

#### `Insets::horizontal`

Left and right together: the width an inset rect loses.

```text
pub const fn horizontal(&self) -> i32
```

#### `Insets::vertical`

Top and bottom together: the height an inset rect loses.

```text
pub const fn vertical(&self) -> i32
```

**Example — the size inside the insets**

```rust
use xpui::{Insets, Size};

let insets = Insets::symmetric(16, 4);
assert_eq!((insets.horizontal(), insets.vertical()), (32, 8));

let inner = Size::new(480, 40).shrink(insets.horizontal(), insets.vertical());
assert_eq!(inner, Size::new(448, 32));
```

**See also:** [`Rect::inset`](#rectinset), [`Padding`](layout.md#padding)
