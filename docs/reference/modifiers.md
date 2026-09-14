# Modifiers

Wrappers that change how any view is sized, or what it does when touched,
without the view knowing. Every view has them as methods, so a tree reads as a
chain: `Modifiers::<Msg>::frame(Text::new("-"), 44, 44).on_tap(Msg::Down)` is a
glyph in a square that sends a message.

![A brightness row: a minus glyph in a 44-pixel square, a slider at 30% filling the width between, and a plus glyph in a 44-pixel square](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/modifiers_overview.png)

The containers they sit in are in [Layout](layout.md). Translating a
component's messages into a screen's, `.map`, is
[`ViewExt::map`](views.md#viewextmap).

## Topics

| | |
|---|---|
| [`Modifiers`](#modifiers) | Chainable modifiers, available on every view. |
| [`Frame`](#frame) | Gives a view a fixed size, centring it in the space that makes. |
| [`Flexible`](#flexible) | Makes any view absorb leftover space along its parent's stacking axis. |
| [`Tappable`](#tappable) | Makes any view report a touch as a message. |

## The modifiers at a glance

| Modifier | Wraps the view in | Effect |
|---|---|---|
| [`.on_tap(message)`](#modifierson_tap) | `Tappable` | A touch, and Confirm while focused, send `message`. |
| [`.on_touch(message)`](#modifierson_touch) | `Tappable` | A touch sends `message`; the view stays out of the focus order. |
| [`.on_long_press(message)`](#modifierson_long_press) | `Tappable` | As `.on_tap`, and a finger held for 500 ms sends `message` too, once. |
| [`.flexible()`](#modifiersflexible) | `Flexible` | Absorbs leftover space in a stack, the way a `Spacer` does. |
| [`.frame(width, height)`](#modifiersframe) | `Frame` | Fixes the size and centres the view in it. |
| [`.map(convert)`](views.md#viewextmap) | `Mapped` | Folds a component's messages into this screen's. |

**Order matters.** Each modifier wraps what is to its left, so the chain reads
inside out:

| Chain | What it does |
|---|---|
| `.frame(44, 44).on_tap(m)` | The whole 44-pixel square is the touch target. What a glyph a few pixels wide wants. |
| `.on_tap(m).frame(44, 44)` | The target grows from the glyph to the theme's minimum touch size, centred on the glyph, and not necessarily to the square. |
| `.on_tap(m).flexible()` · `.flexible().on_tap(m)` | The same: a tappable view that is flexible. `Tappable` passes flexibility through. |
| `.flexible().frame(w, h)` | **Not flexible.** A frame states a size, and hides the flexibility of what it holds. |

## `Modifiers`

Chainable modifiers, available on every view.

```text
pub trait Modifiers<M>: View<M> + Sized
```

Every view implements it, through a blanket implementation for anything that is
a `View<M>`. Nothing implements it by hand.

> [!NOTE]
> **Some views need their message type named.** `Text`, `Icon`, `Image`,
> `Divider` and `Spacer` are views for *every* message type. `.on_tap(message)`
> learns the type from `message`, but `.frame` and `.flexible` have nothing to
> learn it from, even inside a stack, and the compiler asks for type
> annotations. Name it: `Modifiers::<Msg>::frame(Text::new("-"), 44, 44)`. A
> view that carries its own type, such as a `Slider<Msg>` or a `VStack<Msg>`,
> chains directly.

**Example — a brightness row**

```rust
use xpui::{Alignment, HStack, Modifiers, Slider, Text, hstack};

#[derive(Clone, Copy)]
enum Msg {
    Set(i32),
    Down,
    Up,
}

# xpui::testing::install();
let row: HStack<Msg> = hstack![8;
    Modifiers::<Msg>::frame(Text::new("-"), 44, 44).on_tap(Msg::Down),
    Slider::new(30, 100).on_change(Msg::Set).flexible(),
    Modifiers::<Msg>::frame(Text::new("+"), 44, 44).on_tap(Msg::Up),
]
.align(Alignment::Center);
```

**Example — a tap on a bare view**

```rust
use xpui::{Modifiers, Text, Tappable};

#[derive(Clone, Copy)]
enum Msg {
    Decrement,
}

# xpui::testing::install();
// `message` says which type is meant, so nothing needs naming.
let minus: Tappable<Text, Msg> = Text::new("-").on_tap(Msg::Decrement);
```

### Required methods

None: every method has a body.

### Provided methods

#### `Modifiers::on_tap`

Report a touch on this view, or Confirm while it holds focus, as `message`.

```text
fn on_tap(self, message: M) -> Tappable<Self, M>
```

The view becomes a focus stop, in the order the tree reads, so Up and Down reach
it and a finger and a button produce the same message. The message is cloned
each time it is sent, so it is a value, never a closure.

#### `Modifiers::on_touch`

Report a touch as `message`, **without** joining the focus order.

```text
fn on_touch(self, message: M) -> Tappable<Self, M>
```

For controls that are meant for a finger and would otherwise add a stop that
buttons have no way to activate. On a device with no touch panel the view does
nothing at all.

#### `Modifiers::on_long_press`

Report a press held past the long-press threshold as `message`, in addition to an ordinary tap.

```text
fn on_long_press(self, message: M) -> Tappable<Self, M>
```

It declares a tap, a focus stop and a long press, all sending the same
`message`. A quick tap sends it on release, Confirm sends it while the view
holds focus, and a finger resting on the view sends it once the hold reaches
500 ms, the delay before a held key starts repeating.

**One press sends one message.** A hold fires while the finger is still down,
and the release that ends it sends nothing, so a view never hears a long press
followed by a tap. A finger that slides off the view before the threshold fires
no long press. A hold the loop could not see, because the panel was busy
refreshing, starts timing again from the frame that next sees it.

**A hold that means something else.** Keep the tap on the view and put a
hold-only region around it with [`Tappable::accepting`](#tappableaccepting). A
tap resolves to the innermost region that takes a tap, and a hold to the
innermost region that takes a hold, so each finds its own message.

**Example — open on a tap, a menu on a hold**

```rust
use xpui::{InputMask, Interactions, Modifiers, Point, Tappable, Text, View, testing};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Msg {
    Open,
    Menu,
}

testing::install();
let mut book: Tappable<_, Msg> =
    Tappable::new(Text::new("Middlemarch").on_tap(Msg::Open), Msg::Menu)
        .accepting(InputMask::LONG_PRESS);
book.measure(testing::screen());
let mut out = Interactions::new(0);
book.interactions(Point::ORIGIN, &mut out);

// Resolved the way the runtime resolves them: the innermost region that takes it.
let innermost = |kind: InputMask| {
    let item = out.items().iter().rev().find(|item| item.mask.contains(kind))?;
    Some(item.trigger.resolve(item.rect, item.rect.x()))
};
assert_eq!(innermost(InputMask::TAP), Some(Msg::Open));
assert_eq!(innermost(InputMask::LONG_PRESS), Some(Msg::Menu));
```

#### `Modifiers::flexible`

Absorb the space fixed-size siblings leave along the stacking axis.

```text
fn flexible(self) -> Flexible<Self>
```

See [`Flexible`](#flexible) for which views grow into that space and which do
not.

#### `Modifiers::frame`

Fix this view's size, centring it in the frame; an axis of zero or less stays natural.

```text
fn frame(self, width: i32, height: i32) -> Frame<Self>
```

| Parameter | Meaning |
|---|---|
| `width` | The frame's width in pixels, or `0` to keep the view's own. |
| `height` | The frame's height in pixels, or `0` to keep the view's own. |

Shorthand for `Frame::new(view).width(width).height(height)`.

**See also:** [`Frame`](#frame), [`Flexible`](#flexible), [`Tappable`](#tappable), [`ViewExt::map`](views.md#viewextmap)

## `Frame`

Gives a view a fixed size, centring it in the space that makes.

```text
pub struct Frame<V>
```

![Above a rule, a minus and a plus glyph touching; below it, the same two glyphs each centred in a 44-pixel square, so they sit well apart](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/modifiers_frame.png)

A "-" glyph is a few pixels wide, but the control it stands for is a row-height
square. Framing it keeps the glyph where the eye expects it, and gives
[`Tappable`](#tappable) a sensible rectangle to grow from. A frame draws
nothing of its own.

| Builder | Sets | When not called |
|---|---|---|
| [`Frame::new`](#framenew) | the view inside | — |
| [`width`](#framewidth) | a fixed width | the view's own width |
| [`height`](#frameheight) | a fixed height | the view's own height |

How it measures:

- A fixed axis is what the child is offered, so a flexible child fills the frame
  rather than overflowing it. A natural axis offers the child what the frame was
  offered.
- **A frame is never smaller than its child.** A child larger than the fixed
  size makes the frame grow to fit it; nothing is clipped.
- The child is centred on both axes, and its touch targets move with it.
- A frame always counts towards its stack's extent across, even around a
  `Spacer`, because it states a size on purpose. It is never flexible, whatever
  it holds.

**Example — two glyphs in square tap zones**

```rust
use xpui::{Divider, Modifiers, Text, VStack, hstack, vstack};

# xpui::testing::install();
let comparison: VStack<()> = vstack![8;
    hstack![0; Text::new("-"), Text::new("+")],
    Divider::new(),
    hstack![0;
        Modifiers::<()>::frame(Text::new("-"), 44, 44),
        Modifiers::<()>::frame(Text::new("+"), 44, 44),
    ],
];
```

**Example — one axis fixed, and a child larger than the frame**

```rust
use xpui::{Frame, Size, Text, View};

# xpui::testing::install();
let offered = Size::new(480, 800);
let mut text = Text::new("Battery");
View::<()>::measure(&mut text, offered);
let natural = View::<()>::size(&text);

let mut row_height = Frame::new(Text::new("Battery")).height(44);
View::<()>::measure(&mut row_height, offered);
assert_eq!(View::<()>::size(&row_height), Size::new(natural.width, 44), "the width stays natural");

let mut too_small = Frame::new(Text::new("Battery")).width(4).height(4);
View::<()>::measure(&mut too_small, offered);
assert_eq!(View::<()>::size(&too_small), natural, "the frame grows to its child");
```

### Creating a frame

#### `Frame::new`

`child` with neither dimension fixed yet.

```text
pub fn new(child: V) -> Self
```

A frame with neither axis fixed is exactly its child's size. Usually reached
through [`Modifiers::frame`](#modifiersframe) instead.

### Sizing

#### `Frame::width`

Fixes the width, or leaves it natural when `width` is zero or less.

```text
pub fn width(self, width: i32) -> Self
```

#### `Frame::height`

Fixes the height, or leaves it natural when `height` is zero or less.

```text
pub fn height(self, height: i32) -> Self
```

**See also:** [`Modifiers::frame`](#modifiersframe), [`Padding`](layout.md#padding), [`Tappable`](#tappable)

## `Flexible`

Makes any view absorb leftover space along its parent's stacking axis.

```text
pub struct Flexible<V>
```

![Two rows of minus, slider and plus: in the first the slider is not flexible, fills the whole width, and pushes the plus off the panel; in the second it is flexible and the plus sits at the right edge](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/modifiers_flexible.png)

A stack measures its fixed children against the whole space it was offered, and
its flexible children afterwards, against a share of what is left; see
[how a stack measures](stacks.md#how-a-stack-measures). A slider fills whatever
width it is offered, so between fixed `-` and `+` glyphs it takes the whole row
unless it is flexible, and the `+` lands past the edge.

> [!NOTE]
> **Flexible changes what a view is offered, not what it takes.** A slider, a
> spacer or a stack holding a spacer grows into its share. A `Text` reports its
> natural width whatever it is offered, so a flexible text is no wider, and
> pushes nothing along.

Outside a stack a flexible view is measured as it would be without the wrapper.
It draws, declares and measures exactly as its child does.

**Example — a slider between two glyphs**

```rust
use xpui::{HStack, Modifiers, Slider, Text, hstack};

# xpui::testing::install();
let row: HStack<()> = hstack![8; Text::new("-"), Slider::new(40, 100).flexible(), Text::new("+")];
```

**Example — a flexible text grows no wider**

```rust
use xpui::{HStack, Modifiers, Size, Text, View, hstack};

# xpui::testing::install();
let mut text = Text::new("Battery");
View::<()>::measure(&mut text, Size::new(480, 800));

let mut row: HStack<()> = hstack![0; Modifiers::<()>::flexible(Text::new("Battery"))];
row.measure(Size::new(480, 800));
assert_eq!(row.size().width, View::<()>::size(&text).width);
```

### Creating a flexible view

#### `Flexible::new`

`child`, grown into whatever its stack has left.

```text
pub fn new(child: V) -> Self
```

Usually reached through [`Modifiers::flexible`](#modifiersflexible) instead.

**See also:** [`Spacer`](layout.md#spacer), [`Modifiers::flexible`](#modifiersflexible)

## `Tappable`

Makes any view report a touch as a message.

```text
pub struct Tappable<V, M>
```

The visible size is unchanged; only the *hit* area grows, to at least the
theme's minimum touch target on each axis, centred on what was drawn. A 6-pixel
"-" glyph is a legitimate control, and an impossible thing to hit with a
finger, so the target is widened around it. Nothing is drawn to show the
target, or to show focus: a view that should look focused draws that itself.

| Builder | Sets | When not called |
|---|---|---|
| [`Tappable::new`](#tappablenew) | the view and the message it sends | — |
| [`accepting`](#tappableaccepting) | which kinds of input it answers | a tap, and focus with Confirm |

The view's own touch targets are declared before the tappable's, so wrapping a
container still resolves a touch to the innermost control when both cover the
point. A tappable is as flexible as its child. It is a view only when `M` is
`Clone`, because the message is cloned each time it is sent.

**Example — the three ways to respond**

```rust
use xpui::{Icon, IconRef, Modifiers, Text};
# #[derive(Copy, Clone)]
# enum Glyph { Sun }
# impl From<Glyph> for IconRef {
#     fn from(glyph: Glyph) -> IconRef { IconRef::new(glyph as u16) }
# }

#[derive(Clone, Copy)]
enum Msg {
    Decrement,
    ShowContextMenu,
    Toggle,
}

# xpui::testing::install();
Text::new("-").on_tap(Msg::Decrement);
Text::new("Wi-Fi").on_long_press(Msg::ShowContextMenu);
Icon::new(Glyph::Sun).on_touch(Msg::Toggle); // touch only, out of the focus order
```

**Example — built directly**

```rust
use xpui::{InputMask, Tappable, Text};

# xpui::testing::install();
// The same as `Text::new("Close").on_touch(())`.
let close: Tappable<Text, ()> = Tappable::new(Text::new("Close"), ()).accepting(InputMask::TAP);
```

### Creating a tappable view

#### `Tappable::new`

`child`, sending `message` when tapped or activated.

```text
pub fn new(child: V, message: M) -> Self
```

### Choosing the input

#### `Tappable::accepting`

Replaces the accepted input kinds, which are [`InputMask::DEFAULT`](interactions.md#inputmask) (a tap plus button focus) until this is called.

```text
pub fn accepting(self, mask: InputMask) -> Self
```

| Parameter | Meaning |
|---|---|
| `mask` | The kinds of input, combined with `union` or `\|`. `TAP` is a touch released inside the target. `FOCUS` makes it a stop Up and Down reach and Confirm fires. `LONG_PRESS` sends the message once, when a finger has rested on the target for 500 ms; see [`on_long_press`](#modifierson_long_press). |

`InputMask::DRAG` offers every frame the finger is down to its target, so a
tappable accepting it sends its message on each of those frames. It is for a
control that reads the position, which a `Tappable` does not.

**See also:** [`Modifiers::on_tap`](#modifierson_tap), [`Frame`](#frame)
