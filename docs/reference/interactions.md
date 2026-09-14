# Interactions

How a view declares where it can be touched and focused. A screen never
hit-tests: while the tree is walked, each widget declares the regions it owns,
the kinds of input each one accepts and the `Trigger` it produces when it fires,
and `Interactions` collects them, telling each one as it is declared whether it
holds focus. Every frame the runtime resolves input against that list, so the
same declaration answers a finger and a key.

[Input](input.md) is the buttons, the swipes and the frame of input these
declarations are resolved against. [Writing a widget](../writing-a-widget.md)
builds a control that declares its own regions. What a declaration produces
when it fires is [Triggers](triggers.md). This page is what each piece of a
declaration does.

## Topics

| | |
|---|---|
| [`InputMask`](#inputmask) | Which kinds of input an interaction accepts. |
| [`Interaction`](#interaction) | One interactive region, as declared by the widget that owns it. |
| [`Interactions`](#interactions) | Collects a tree's interactions, telling each one whether it has focus as it is declared. |

## `InputMask`

Which kinds of input an interaction accepts.

```text
pub struct InputMask(u8)
```

This is what stops a finger resting on a button re-firing it every frame: only
`DRAG` interactions are offered every held frame, `LONG_PRESS` acts once when a
hold reaches 500 ms, and everything else acts once, on release. **The mask is a widget's most important choice.** Combine masks with
`|` or [`union`](#inputmaskunion).

| Constant | Meaning |
|---|---|
| `InputMask::TAP` | A tap: press and release inside the control. |
| `InputMask::FOCUS` | Reachable by Up/Down, activated by Confirm. |
| `InputMask::DRAG` | Receives every frame the finger is down, with its position. |
| `InputMask::LONG_PRESS` | A press held past the long-press threshold. |
| `InputMask::ADJUST` | This control is moved one step at a time rather than fired. |
| `InputMask::DEFAULT` | What an ordinary control wants: tappable, and reachable by button. |

What each asks of the runtime:

| Mask | Means |
|---|---|
| `TAP` | A completed tap. Held frames never arrive. |
| `DRAG` | Every frame while a finger is down: a slider's track wants this. |
| `FOCUS` | Joins the Up/Down focus order, and Confirm fires it. |
| `LONG_PRESS` | A finger held for 500 ms: fires once, and its release fires nothing. `Modifiers::on_long_press` declares it. |
| `ADJUST` | Left and Right nudge it while it holds focus, Confirm declines to fire it, and on a device with no Left/Right pair Confirm opens it for editing instead. |
| `DEFAULT` | `TAP` plus `FOCUS`. |

**How a touch resolves.** Every frame a finger is down, the runtime offers it to
the innermost `DRAG` region under it. When none takes it and the finger has
rested for 500 ms, the innermost `LONG_PRESS` region under the point where it
went down fires, provided the finger is still inside it, and the rest of that
hold is ignored: the release that ends a long press is not a tap. Otherwise the
release is a tap, for the innermost `TAP` region under it. A region carrying
both `TAP` and `LONG_PRESS` sends its message for either, and never twice for
one press. The threshold is the delay before a held key repeats, so a finger
and a key become a hold at the same moment.

**The rect declared with `FOCUS` is what gets scrolled into view.** The runtime
scrolls the least that brings it there, so a control that declares only its
moving part settles with the rest of it clipped off the panel. Declare the whole
control for `FOCUS`, and a second, smaller rect for `TAP` or `DRAG` when a
finger should land only on part of it. `Slider` does both.

**Example — combining masks**

```rust
use xpui::InputMask;

let track = InputMask::TAP | InputMask::DRAG;
assert!(track.contains(InputMask::DRAG));
assert!(!track.contains(InputMask::FOCUS));

assert_eq!(InputMask::DEFAULT, InputMask::TAP.union(InputMask::FOCUS));
assert_eq!(InputMask::DEFAULT.without(InputMask::FOCUS), InputMask::TAP);
```

### Combining masks

#### `InputMask::union`

This mask with `other`'s bits added.

```text
pub const fn union(self, other: InputMask) -> InputMask
```

`const`, so a widget can name a combination as a constant. `a | b` is the same.

#### `InputMask::without`

This mask with `other`'s bits removed.

```text
pub const fn without(self, other: InputMask) -> InputMask
```

#### `InputMask::contains`

Whether every bit of `other` is set here.

```text
pub const fn contains(self, other: InputMask) -> bool
```

`contains(InputMask::DEFAULT)` is true only when both `TAP` and `FOCUS` are set.

**See also:** [`Interactions::declare`](#interactionsdeclare),
[`Tappable::accepting`](modifiers.md#tappableaccepting)

## `Interaction`

One interactive region, as declared by the widget that owns it.

```text
pub struct Interaction<M>
```

A widget does not build one: [`Interactions::declare`](#interactionsdeclare)
does, and [`Interactions::items`](#interactionsitems) reads them back, in tree
order. A test reads them to check what a widget declared without driving the
runtime.

| Field | Meaning |
|---|---|
| `Interaction::rect` | The region, in screen pixels. |
| `Interaction::mask` | Which kinds of input it accepts. |
| `Interaction::trigger` | What it produces when it fires. |

When two regions under a touch accept it, the one declared **last** wins, so a
container that declares its own region before its children resolves to the
innermost control.

**See also:** [`Interactions`](#interactions), [`InputMask`](#inputmask),
[`Trigger`](triggers.md#trigger)

## `Interactions`

Collects a tree's interactions, telling each one whether it has focus as it is declared.

```text
pub struct Interactions<M>
```

The runtime builds one each frame it needs one, starting at the index of the
control holding focus, and walks the tree through
[`View::interactions`](views.md#viewinteractions). Focus is an index into the
focusable interactions **in tree order**. A widget learns whether it holds
focus from the return value of [`declare`](#interactionsdeclare) rather than
being told in a later pass, so there is no third walk to keep in step with
`render`.

Most of this type is for containers and for the runtime. A widget of your own
calls `declare`, and a value widget reads
[`is_editing`](#interactionsis_editing) and
[`editing_value`](#interactionsediting_value).

**Example — a widget that declares one region**

```rust
use xpui::{InputMask, Interactions, Point, Rect, Size, Trigger, View};

struct Chip<M> {
    size: Size,
    message: M,
    focused: bool,
}

impl<M: Clone> View<M> for Chip<M> {
    fn measure(&mut self, available: Size) {
        self.size = Size::new(available.width, 40);
    }

    fn size(&self) -> Size {
        self.size
    }

    fn render(&self, _origin: Point) {
        // A real chip paints itself highlighted when `self.focused`.
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<M>) {
        let rect = Rect { origin, size: self.size };
        let message = Trigger::Message(self.message.clone());
        self.focused = out.declare(rect, InputMask::DEFAULT, message);
    }
}

let mut chip = Chip { size: Size::default(), message: "open", focused: false };
chip.measure(Size::new(200, 800));

let mut out = Interactions::new(0);
chip.interactions(Point::new(16, 100), &mut out);

assert!(chip.focused, "the first focus stop, and focus is at 0");
let item = &out.items()[0];
assert_eq!(item.rect, Rect::new(16, 100, 200, 40));
assert!(item.mask.contains(InputMask::TAP));
assert_eq!(item.trigger.resolve(item.rect, 20), "open");
```

**Example — one focus stop, a smaller touch target**

The pattern `Slider` uses: the whole control is the focus stop, so scrolling
to it brings its label along, and only the track takes a finger.

```rust
use xpui::{InputMask, Interactions, Rect, Trigger};

#[derive(Clone, Copy)]
enum Msg {
    Warmth(i32),
}

let whole = Rect::new(16, 100, 448, 64);
let track = Rect::new(16, 124, 448, 40);
let mut out = Interactions::new(0);

let focused = out.declare(
    whole,
    InputMask::FOCUS | InputMask::ADJUST,
    Trigger::Value { make: Msg::Warmth, max: 100, value: 25 },
);
out.declare(
    track,
    InputMask::TAP | InputMask::DRAG,
    Trigger::Value { make: Msg::Warmth, max: 100, value: 25 },
);

assert!(focused);
assert_eq!((out.len(), out.focusable_count()), (2, 1));
assert_eq!(out.focused_rect(0), Some(whole));
```

### Creating a collector

#### `Interactions::new`

A collector for a tree whose focused control is at `focus`.

```text
pub fn new(focus: usize) -> Self
```

| Parameter | Meaning |
|---|---|
| `focus` | The index, among focusable interactions in tree order, of the one holding focus. |

#### `Interactions::capturing`

A pass that ignores everything until a view calls `capture`.

```text
pub fn capturing(focus: usize) -> Self
```

The runtime uses this once it knows the frame contains a capturing view: a
first pass finds out, and a second collects only what is reachable. Everything
declared before `capture` is dropped and told it has no focus, so the rows
behind a dialog never paint themselves focused.

#### `Interactions::scrolled`

Starts the walk with a scroll offset the scroll view should apply.

```text
pub fn scrolled(mut self, offset: i32) -> Self
```

The view tree is rebuilt every frame, so a scroll view cannot remember how far
it has scrolled. The runtime hands the offset down here.

### Declaring regions

#### `Interactions::declare`

Registers an interaction, returning whether it currently holds focus.

```text
pub fn declare(&mut self, rect: Rect, mask: InputMask, trigger: Trigger<M>) -> bool
```

| Parameter | Meaning |
|---|---|
| `rect` | The region, in screen pixels: the `origin` the view was given plus its size. |
| `mask` | What it accepts. Only a mask containing `FOCUS` takes a focus stop. |
| `trigger` | What it sends. |

Returns `false` for a region without `FOCUS`, and for every region declared
during a capturing pass before `capture`.

#### `Interactions::capture`

Discards everything declared so far: this view is the only thing reachable while it is present.

```text
pub fn capture(&mut self, preferred_focus: usize)
```

| Parameter | Meaning |
|---|---|
| `preferred_focus` | Where focus sits while the view is up: a picker opens on the value already chosen. |

A dialog drawn over a list shares the list's tree, so without this the side
buttons would walk out of the dialog into the rows behind it. A capturing view
calls it **before** declaring its own regions: the tree is walked in draw
order, so everything behind has already been collected.

**Example — a dialog over two rows**

```rust
use xpui::{InputMask, Interactions, Rect, Trigger};

let mut out = Interactions::new(0);
for y in [100, 144] {
    out.declare(Rect::new(0, y, 480, 40), InputMask::DEFAULT, Trigger::Message("row"));
}

out.capture(1);
for y in [300, 344, 388] {
    out.declare(Rect::new(40, y, 400, 40), InputMask::DEFAULT, Trigger::Message("choice"));
}

assert_eq!(out.len(), 3, "the rows behind are gone");
assert_eq!(out.captured_focus(), Some(1));
```

#### `Interactions::dismiss_with`

Says what dismissing the view that just captured input sends.

```text
pub fn dismiss_with(&mut self, message: M)
```

| Parameter | Meaning |
|---|---|
| `message` | Sent for a Back the screen does not claim, and for a tap on nothing the capturing view declared. |

Call it after [`capture`](#interactionscapture), which clears it, so a view that
captures without saying never inherits the dismissal of a view beneath it.
Called while nothing has captured, it is ignored.
[`Modal::on_dismiss`](dialogs.md#modalon_dismiss) is how a dialog calls it.

**Example — a dialog over a dialog**

```rust
use xpui::{InputMask, Interactions, Rect, Trigger};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Msg {
    Choice(usize),
    Dismiss,
}

let mut out = Interactions::new(0);
out.capture(0);
out.dismiss_with(Msg::Dismiss);
out.declare(Rect::new(40, 300, 400, 40), InputMask::DEFAULT, Trigger::Message(Msg::Choice(0)));
assert_eq!(out.dismissal(), Some(&Msg::Dismiss));

out.capture(0); // a second dialog, over the first, that says nothing
assert_eq!(out.dismissal(), None);
```

#### `Interactions::restrict_outside`

Withdraws `mask` from every interaction declared since `from` that falls outside `visible`.

```text
pub fn restrict_outside(&mut self, from: usize, visible: Rect, mask: InputMask)
```

| Parameter | Meaning |
|---|---|
| `from` | The `len` before the child was walked, so only the child's regions change. |
| `visible` | The band that can be touched. |
| `mask` | What to withdraw: a scroll view passes `TAP` and `DRAG`. |

A scrolled-away control keeps its focus stop, so it can still be reached, but
stops accepting touches aimed at whatever now occupies that part of the screen.

**Example — a row scrolled out of the viewport**

```rust
use xpui::{InputMask, Interactions, Rect, Trigger};

let viewport = Rect::new(0, 60, 480, 700);
let mut out = Interactions::new(0);
let before = out.len();
out.declare(Rect::new(0, 100, 480, 40), InputMask::DEFAULT, Trigger::Message(0));
out.declare(Rect::new(0, 900, 480, 40), InputMask::DEFAULT, Trigger::Message(1));
out.restrict_outside(before, viewport, InputMask::TAP | InputMask::DRAG);

let hidden = &out.items()[1];
assert!(!hidden.mask.contains(InputMask::TAP));
assert!(hidden.mask.contains(InputMask::FOCUS));
assert_eq!(out.focusable_count(), 2);
```

### Focus in a composite control

#### `Interactions::parent_focused`

Whether the control being walked into is the one holding focus.

```text
pub fn parent_focused(&self) -> bool
```

For a widget that declares no focus stop of its own. A `Stepper` takes one stop
for the whole control and embeds a track that takes none, so the track has
nothing to learn from its own declaration and asks the control that wrapped it.

#### `Interactions::set_parent_focused`

Says the subtree about to be walked belongs to a focused control.

```text
pub fn set_parent_focused(&mut self, focused: bool)
```

Set it around the walk and put it back afterwards, so a second control further
down the tree does not inherit the answer.

**Example — a wrapper that owns the stop**

```rust
use xpui::{InputMask, Interactions, Rect, Trigger};

let mut out: Interactions<()> = Interactions::new(0);
let focused = out.declare(Rect::new(0, 0, 480, 40), InputMask::FOCUS, Trigger::Message(()));

out.set_parent_focused(focused);
// The inner track declares touch only, and paints from what its parent said.
out.declare(Rect::new(40, 0, 400, 40), InputMask::TAP | InputMask::DRAG, Trigger::Message(()));
let track_paints_focused = out.parent_focused();
out.set_parent_focused(false);

assert!(track_paints_focused);
assert_eq!(out.focusable_count(), 1);
```

#### `Interactions::is_editing`

Whether the control at `focus` is open for editing.

```text
pub fn is_editing(&self) -> bool
```

The runtime owns the edit, and a widget cannot tell from its own declaration
that the keys have changed meaning. It is a widget's question rather than a
screen's, so a value widget can paint the working copy while the mode stays
the framework's.

#### `Interactions::editing_value`

The open edit's working value, for the control the edit is open on.

```text
pub fn editing_value(&self) -> Option<i32>
```

A control that holds focus while this is `Some` **is** that control, since there
is one focus, so it paints this rather than the value it was built with.

### Scrolling

#### `Interactions::scroll`

The offset a scroll view should apply to its content.

```text
pub fn scroll(&self) -> i32
```

#### `Interactions::set_viewport`

Published by a scroll view: the visible band, and how tall its content is.

```text
pub fn set_viewport(&mut self, viewport: Rect, content_height: i32)
```

The runtime reads it back to keep the focused control in sight.

#### `Interactions::viewport`

The scrolling viewport and its content height, if a scroll view published one.

```text
pub fn viewport(&self) -> Option<(Rect, i32)>
```

`None` means nothing on this screen scrolls.

### Reading what was declared

#### `Interactions::len`

How many interactions have been declared so far.

```text
pub fn len(&self) -> usize
```

A container reads it before walking a child, to find the ones the child added.

#### `Interactions::is_empty`

Whether nothing has been declared.

```text
pub fn is_empty(&self) -> bool
```

#### `Interactions::items`

Everything declared so far, in tree order.

```text
pub fn items(&self) -> &[Interaction<M>]
```

#### `Interactions::focusable_count`

How many interactions can hold focus.

```text
pub fn focusable_count(&self) -> usize
```

The runtime wraps its cursor on this.

#### `Interactions::focused_rect`

Rect of the focusable interaction at `focus`, in tree order.

```text
pub fn focused_rect(&self, focus: usize) -> Option<Rect>
```

The runtime scrolls to bring this into view.

#### `Interactions::captured_focus`

The focus index a capturing view asked for, or `None` if none captured.

```text
pub fn captured_focus(&self) -> Option<usize>
```

#### `Interactions::dismissal`

What dismissing the capturing view sends, if it said.

```text
pub fn dismissal(&self) -> Option<&M>
```

The runtime reads it only while [`captured_focus`](#interactionscaptured_focus)
is `Some`. `None` there means Back does nothing and a tap outside goes to
`Screen::on_background_tap`.

**See also:** [`Interaction`](#interaction), [`View::interactions`](views.md#viewinteractions),
[writing a widget](../writing-a-widget.md#responding-to-touch)
