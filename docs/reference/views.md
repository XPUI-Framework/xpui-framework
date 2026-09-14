# Views

Everything a screen's `body` returns is a view: a text, a list, a stack of
either, a modifier around any of them. A view is told how much room it has,
says how big it wants to be, declares what can be touched and paints itself.
It is built for one frame and dropped.

![A custom battery gauge, an outline with a terminal nub, filled nearly three quarters, beside the text 72%](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/views_battery.png)

[Writing a widget](../writing-a-widget.md) is the guide to building one, and
[how a frame runs](../architecture.md) is where the passes fit. This page is
what each piece does.

## Topics

| | |
|---|---|
| [`View`](#view) | A node in the UI tree. |
| [`ViewExt`](#viewext) | Chainable helpers available on every view. |
| [`view::Mapped`](#viewmapped) | A component's view, with its messages translated into its parent's. |

## `View`

A node in the UI tree.

```text
pub trait View<M>
```

`M` is the message type of the screen the view sits in: a control carries
values of `M` and hands them back when touched, so a screen matches on its own
enum. A view that sends nothing, such as `Text` or `Divider`, is a `View<M>`
for every `M`.

Every frame walks the tree in three passes:

| Pass | Method | What it does |
|---|---|---|
| 1 | [`measure`](#viewmeasure) | records the size the view wants within what it is offered |
| 2 | [`interactions`](#viewinteractions) | declares what can be touched, where it finally sits, and learns which declaration holds focus |
| 3 | [`render`](#viewrender) | paints |

> [!WARNING]
> Passes 2 and 3 walk the same geometry. A container that places a child at
> one origin in `interactions` and another in `render` puts a touch on the
> control next door.

Implemented by every widget, stack and modifier; by `Box<V>` for any view `V`,
including `Box<dyn View<M>>`; and by [`view::Mapped`](#viewmapped).

**Example — a battery gauge**

```rust
use xpui::{Point, Rect, Renderer, Size, View};

struct Battery {
    percent: i32,
    measured: Size,
}

impl Battery {
    fn new(percent: i32) -> Self {
        Battery { percent: percent.clamp(0, 100), measured: Size::ZERO }
    }
}

impl<M> View<M> for Battery {
    fn measure(&mut self, available: Size) {
        // 64 by 28 when there is room, and never more than was offered.
        self.measured = Size::new(64.min(available.width), 28.min(available.height));
    }

    fn size(&self) -> Size {
        self.measured
    }

    fn render(&self, origin: Point) {
        let Size { width, height } = self.measured;
        Renderer::stroke_rect(Rect::new(origin.x, origin.y, width - 4, height));
        Renderer::fill_rect(Rect::new(origin.x + width - 4, origin.y + height / 3, 4, height / 3), true);
        let fill = (width - 10) * self.percent / 100;
        Renderer::fill_rect(Rect::new(origin.x + 3, origin.y + 3, fill, height - 6), true);
    }
}

# xpui::testing::install();
let row: xpui::HStack<()> = xpui::hstack![12; Battery::new(72), xpui::Text::new("72%")]
    .align(xpui::Alignment::Center);
```

**Example — a control that draws its own focus**

```rust
use xpui::{InputMask, Interactions, Point, Rect, Renderer, Size, Trigger, View};

struct Swatch<M> {
    message: M,
    focused: bool,
    measured: Size,
}

impl<M: Clone> View<M> for Swatch<M> {
    fn measure(&mut self, available: Size) {
        self.measured = Size::new(40.min(available.width), 40.min(available.height));
    }

    fn size(&self) -> Size {
        self.measured
    }

    // Runs before `render` on every paint, so the answer is this frame's.
    fn interactions(&mut self, origin: Point, out: &mut Interactions<M>) {
        let trigger = Trigger::Message(self.message.clone());
        self.focused = out.declare(self.bounds(origin), InputMask::DEFAULT, trigger);
    }

    fn render(&self, origin: Point) {
        let rect = self.bounds(origin);
        Renderer::stroke_rect(rect);
        if self.focused {
            Renderer::fill_rect(Rect::new(rect.x() + 4, rect.y() + 4, rect.width() - 8, rect.height() - 8), true);
        }
    }
}

let mut swatch = Swatch { message: "ink", focused: false, measured: Size::ZERO };
swatch.measure(Size::new(480, 800));

let mut out = Interactions::new(0); // focus is on the first stop
swatch.interactions(Point::ORIGIN, &mut out);
assert!(swatch.focused);
assert_eq!(out.focusable_count(), 1);
```

**Example — measuring a view outside a tree**

```rust
use xpui::{Size, Text, View};

xpui::testing::install();
let mut text = Text::new("Battery");
View::<()>::measure(&mut text, Size::new(480, 800));
assert!(View::<()>::size(&text).width > 0);
```

`Text` is a `View<M>` for every `M`, so a call outside a tree has to say which
one it means, and `()` is the usual choice. Inside a tree this never comes up:
the stack knows its message type and passes it down. Text measures through the
host's font metrics, which is why the host is installed first.

### Required methods

#### `View::measure`

Compute and store the size this view wants within `available`.

```text
fn measure(&mut self, available: Size)
```

| Parameter | Meaning |
|---|---|
| `available` | The most room the parent can give. A scroll view offers `UNBOUNDED` height, a large number rather than `i32::MAX`, so a sum of children cannot overflow. |

Store the answer, since `size` is asked afterwards and more than once. A stack
measures its fixed children first and offers flexible ones what they left, so a
flexible view's `available` is the remainder.

#### `View::size`

The size recorded by the most recent [`measure`](#viewmeasure).

```text
fn size(&self) -> Size
```

A parent reads it after `measure`, to place the view and to size itself.

#### `View::render`

Paint with the view's top-left corner at `origin`.

```text
fn render(&self, origin: Point)
```

Paint through `Renderer`, within the size recorded by `measure`. A container
renders each child at the origin it gave that child in `interactions`.

### Provided methods

#### `View::interactions`

Declare any interactive regions, given this view sits at `origin`.

```text
fn interactions(&mut self, origin: Point, out: &mut Interactions<M>)
```

Non-interactive leaves keep the default and declare nothing.

| Parameter | Meaning |
|---|---|
| `origin` | Where the view sits on the panel: the same origin `render` receives. |
| `out` | The frame's table. `declare` adds a rectangle, a mask and a trigger, and returns whether that declaration holds focus. |

It takes `&mut self` so a view can keep that answer for `render`, which runs
after it. [Writing a widget](../writing-a-widget.md#responding-to-touch)
explains the masks.

#### `View::is_flexible`

Whether this view absorbs leftover space along its parent's stacking axis.

```text
fn is_flexible(&self) -> bool
```

Flexible views are measured in a second pass, against only what the fixed-size
siblings left behind. `Spacer` and the `flexible()` modifier say yes.

#### `View::contributes_cross_size`

Whether this view's measured size counts towards its parent's extent *across* the stacking axis.

```text
fn contributes_cross_size(&self) -> bool
```

Only `Spacer` says no. A spacer records the whole cross extent it is offered,
so counting it would make an `HStack` as tall as the space it was offered rather
than as tall as its content.

#### `View::bounds`

This view's own bounds when placed at `origin`.

```text
fn bounds(&self, origin: Point) -> Rect
```

The `origin` and the measured `size`, as a `Rect`: what most views declare as
their touch target.

**See also:** [`ViewExt`](#viewext), [`Screen::body`](screens.md#screenbody)

## `ViewExt`

Chainable helpers available on every view.

```text
pub trait ViewExt<M>: View<M> + Sized
```

Implemented for every sized view, so importing the trait is all it takes. The
modifiers that change layout or add a tap, such as `frame` and `on_tap`, are a
separate trait, [`Modifiers`](modifiers.md).

**Example — one stack of different views**

```rust
use xpui::{Divider, Text, VStack, View, ViewExt};

# xpui::testing::install();
let rows: Vec<Box<dyn View<()>>> = vec![
    Text::new("Wi-Fi").boxed(),
    Divider::new().boxed(),
    Text::new("Bluetooth").boxed(),
];
let tree: VStack<()> = VStack::new(8).extend(rows);
```

![Wi-Fi and Bluetooth separated by a one-pixel rule, three views of two types in one stack](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/views_boxed.png)

**Example — a component with its own messages**

```rust
use xpui::{Modifiers, Screen, Text, View, ViewExt, vstack};

#[derive(Clone, Copy)]
enum UnitMsg {
    Cycle,
}

struct Units {
    binary: bool,
}

impl Units {
    /// Tapping the figure cycles the units, a message the screen never sees.
    fn view(&self, bytes: i32) -> impl View<UnitMsg> + use<> {
        let text = if self.binary {
            format!("{} KiB", bytes / 1024)
        } else {
            format!("{} kB", bytes / 1000)
        };
        Text::new(text).on_tap(UnitMsg::Cycle)
    }

    fn update(&mut self, message: UnitMsg) {
        match message {
            UnitMsg::Cycle => self.binary = !self.binary,
        }
    }
}

#[derive(Clone, Copy)]
enum Msg {
    Units(UnitMsg),
}

struct Storage {
    units: Units,
    free: i32,
}

impl Screen for Storage {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        vstack![8;
            Text::new("Free space"),
            self.units.view(self.free).map(Msg::Units),
        ]
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Units(inner) => self.units.update(inner),
        }
    }
}

let mut storage = Storage { units: Units { binary: false }, free: 182_000 };
storage.update(Msg::Units(UnitMsg::Cycle));
assert!(storage.units.binary);
```

![Free space above the figure 182 kB, which cycles its own units when tapped](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/views_units.png)

A component owns its state and declares its own message type, and the parent
folds it in with `.map()`. Its controls keep their place in the parent's focus
order, exactly where they appear in the tree. There is no registration and no
trait to implement: a plain `fn thing(..) -> impl View<M> + use<M>` is a
component too.

> [!NOTE]
> The `+ use<>` is not decoration. In edition 2024 an `impl Trait` return
> captures every lifetime in scope, including the `&self` the method was called
> on, and a stack requires `'static` children. `use<>` says the returned view
> borrows nothing, which is true because `Text` owns its string. A component
> that does borrow cannot go in a stack, and this is where the compiler says
> so.

### Provided methods

#### `ViewExt::boxed`

Erase this view's type, for collecting views of different shapes.

```text
fn boxed(self) -> Box<dyn View<M>>
where
    Self: 'static,
```

A `Box<dyn View<M>>` is itself a view, so the result goes anywhere a view goes.
Containers take views by value, and a tree whose shape is fixed never needs
this. It is for a `Vec` of mixed views, or the two branches of an `if` that
build different types.

#### `ViewExt::map`

Translate this view's messages into another type.

```text
fn map<P>(self, convert: fn(M) -> P) -> Mapped<Self, M, P>
```

| Parameter | Meaning |
|---|---|
| `convert` | A function from the view's message to the parent's, usually the parent's enum variant: `Msg::Units`. A `fn` pointer, not a closure, so it captures nothing. |

A message a control built with the tree is converted as it is declared; a value
resolved from a touch or a key is converted when it resolves. Layout, painting
and focus pass through unchanged.

**See also:** [`view::Mapped`](#viewmapped), [`Modifiers`](modifiers.md)

## `view::Mapped`

A component's view, with its messages translated into its parent's.

```text
pub struct Mapped<V, N, M>
```

Built by [`ViewExt::map`](#viewextmap), and named only when a function returns
one without `impl View`.

| Parameter | Meaning |
|---|---|
| `V` | The component's view. |
| `N` | The component's message type, what `V` sends. |
| `M` | The parent's message type, what the `Mapped` sends. |

It measures, renders and reports flexibility exactly as `V` does. While
declaring, it collects `V`'s interactions into a table of its own and converts
each trigger into `M`, so every focus stop inside stays a focus stop, in
order.

**Example — the same size, another message type**

```rust
use xpui::view::Mapped;
use xpui::{Size, Text, View, ViewExt};

#[derive(Clone, Copy)]
enum UnitMsg {
    Cycle,
}

#[derive(Clone, Copy)]
enum Msg {
    Units(UnitMsg),
}

xpui::testing::install();
let mut plain = Text::new("182 kB");
let mut mapped: Mapped<Text, UnitMsg, Msg> = Text::new("182 kB").map(Msg::Units);

View::<UnitMsg>::measure(&mut plain, Size::new(480, 800));
mapped.measure(Size::new(480, 800));
assert_eq!(mapped.size(), View::<UnitMsg>::size(&plain));
```

**See also:** [`ViewExt::map`](#viewextmap), [`View`](#view)
