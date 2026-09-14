# Interactions

How a view declares where it can be touched and focused. A screen never
hit-tests: while the tree is walked, each widget declares the regions it owns,
the kinds of input each one accepts and the `Trigger` it produces when it fires,
and `Interactions` collects them, telling each one as it is declared whether it
holds focus. Every frame the runtime resolves input against that list, so the
same declaration answers a finger and a key.

[Input](input.md) is the buttons, the swipes and the frame of input these
declarations are resolved against. [Writing a widget](../writing-a-widget.md)
builds a control that declares its own regions. This page is what each piece of
a declaration does.

## Topics

| | |
|---|---|
| [`InputMask`](#inputmask) | Which kinds of input an interaction accepts. |
| [`Interaction`](#interaction) | One interactive region, as declared by the widget that owns it. |
| [`Interactions`](#interactions) | Collects a tree's interactions, telling each one whether it has focus as it is declared. |
| [`Trigger`](#trigger) | What an interaction produces when it fires. |
| [`value_at`](#value_at) | The value a touch at `x` represents within `track`, rounded to nearest. |

## `InputMask`

Which kinds of input an interaction accepts.

```text
pub struct InputMask(u8)
```

This is what stops a finger resting on a button re-firing it every frame: only
`DRAG` interactions are offered held touches, and everything else acts once, on
release. **The mask is a widget's most important choice.** Combine masks with
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
| `LONG_PRESS` | A press held past the threshold, declared by `Modifiers::on_long_press`. |
| `ADJUST` | Left and Right nudge it while it holds focus, Confirm declines to fire it, and on a device with no Left/Right pair Confirm opens it for editing instead. |
| `DEFAULT` | `TAP` plus `FOCUS`. |

> [!NOTE]
> The runtime resolves touches only against `TAP` and `DRAG`. Nothing reads
> `LONG_PRESS` yet, so a region declaring it fires on an ordinary tap, through
> the `TAP` it also carries, and never on a hold.

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
[`Trigger`](#trigger)

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

**See also:** [`Interaction`](#interaction), [`View::interactions`](views.md#viewinteractions),
[writing a widget](../writing-a-widget.md#responding-to-touch)

## `Trigger`

What an interaction produces when it fires.

```text
pub enum Trigger<M>
```

A widget says what kind of thing it is, and the runtime turns one frame of
input into the screen's own message. `Message` covers buttons, rows and
toggles: the widget knows what it means, so it builds the message when the tree
is built. `Value` is for controls whose message depends on **where** the touch
landed: the framework converts the position and calls the constructor, so no
screen re-derives slider geometry. For anything continuous, never send one
message per pixel.

| Variant | Meaning |
|---|---|
| `Trigger::Message` | A message the widget built when the tree was built. |
| `Trigger::Value` | An absolute value, converted from where the touch landed. |
| `Trigger::Step` | A relative nudge: `-1` or `+1` from Left/Right, or from a `-`/`+` glyph. |
| `Trigger::MappedValue` | A value control seen through `ViewExt::map`. |
| `Trigger::MappedStep` | A step control seen through `ViewExt::map`; see `Trigger::MappedValue`. |

`Value` and `Step` carry the same fields, and the mapped variants carry them
boxed:

| Field | In | Meaning |
|---|---|---|
| `make` | all four | Builds the message from the resolved value, or from the delta. A function pointer in `Value` and `Step`, a `Box<dyn Fn(i32) -> M>` in the mapped two. |
| `max` | all four | The top of the control's range. The bottom is always 0. |
| `value` | all four | What the control reads right now, rebuilt with the tree every frame. |
| `set` | `Step`, `MappedStep` | The message that sets the control outright, when it has one. |

**`value` is what the control reads right now.** An absolute control needs it to
be nudged, since one step of an absolute value is `value + delta`; without it,
Left and Right could focus a control and still not change it. A relative
control needs it for an edit to open on.

**`set` is what makes a relative control editable.** A nudge is worth whatever
the screen decides, so it cannot express "the value is this now", the one
message an open edit commits. A `Step` without a setter can be nudged but never
opened.

**`max` bounds the value while the framework holds it.** Outside an edit a
screen clamps; inside one the framework owns the copy, and an unbounded copy
would commit a number the screen never showed.

`Step` is distinct from `Value` because the screen adds the delta to whatever it
currently holds, rather than being handed an absolute. The mapped variants
exist because composing two function pointers is not a function pointer: a
component that wraps a value control, seen through
[`ViewExt::map`](views.md#viewextmap), costs one small allocation per touch
frame.

**Example — what each kind sends**

```rust
use xpui::{Rect, Trigger, testing};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Msg {
    Open,
    Set(i32),
    Nudge(i32),
}

testing::install();
let track = Rect::new(0, 0, 200, 40);

let row = Trigger::Message(Msg::Open);
assert_eq!(row.resolve(track, 120), Msg::Open);
assert_eq!(row.resolve_step(1), None);

// Absolute: a touch becomes a value, and a nudge is held inside 0..=max.
let slider = Trigger::Value { make: Msg::Set, max: 100, value: 40 };
assert_eq!(slider.resolve(track, 100), Msg::Set(50));
assert_eq!(slider.resolve_step(1), Some(Msg::Set(41)));
assert_eq!(slider.resolve_step(500), Some(Msg::Set(100)));

// Relative: the screen gets the delta and does the adding.
let stepper = Trigger::Step { make: Msg::Nudge, set: Some(Msg::Set), max: 100, value: 40 };
assert_eq!(stepper.resolve_step(-1), Some(Msg::Nudge(-1)));
```

### Resolving input

#### `Trigger::resolve`

Resolves to a message.

```text
pub fn resolve(&self, rect: Rect, x: i32) -> M
```

| Parameter | Meaning |
|---|---|
| `rect` | The region the interaction declared. |
| `x` | The touch position, ignored by controls that do not depend on it. |

`Value` and `MappedValue` convert `x` through [`value_at`](#value_at). A
`Step` resolves to `make(0)`, which Confirm never asks for, because it declines
anything carrying `InputMask::ADJUST`. Public so a test can assert what a
control would send without driving the whole runtime.

#### `Trigger::resolve_step`

The message for a relative nudge, or `None` for a control that has no meaningful step.

```text
pub fn resolve_step(&self, delta: i32) -> Option<M>
```

An absolute control is nudged by resolving `value + delta` against its own
bounds, so Left and Right drive a `Slider` and a `Stepper` the same way from a
screen's point of view. It is clamped here rather than left to the screen: a
screen that clamps is common, one that wraps or rejects is not, and the
framework must not need to know which it got.

### Editing a value

#### `Trigger::reading`

What the control reads now, for an editor to open on.

```text
pub fn reading(&self) -> Option<i32>
```

Read once, on the frame an edit opens, to seed the copy the framework then
owns. `None` for a control with no reading, a row or a button, which is also
every control an edit can never open on.

#### `Trigger::is_editable`

Whether an edit can open on this control at all.

```text
pub fn is_editable(&self) -> bool
```

It needs a reading to open on and a way to be **set outright**, because that is
what Confirm dispatches. `Value` and `MappedValue` always are; a `Step` is when
it has `set`; a `Message` never is.

#### `Trigger::stepped`

One step from `from`, held inside the control's own range.

```text
pub fn stepped(&self, from: i32, delta: i32) -> Option<i32>
```

| Parameter | Meaning |
|---|---|
| `from` | The edit's working copy. |
| `delta` | `-1` or `+1`. |

For a value the framework is holding, not one the screen owns. A step is one
unit of the range, including for a `Step` whose `make` the screen scales: that
scale is what a nudge is worth to a screen, and inside an edit the framework
holds the value. `None` for a `Message`; a `max` that is not positive answers
`0`.

#### `Trigger::set_to`

The message that sets this control to `value` outright, or `None` for one that can only be nudged.

```text
pub fn set_to(&self, value: i32) -> Option<M>
```

What Confirm sends when it closes an edit. Absolute controls always have one;
a `Step` has one only if it was given `set`.

**Example — an edit on a stepper, and one that cannot open**

```rust
use xpui::Trigger;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Msg {
    Set(i32),
    Nudge(i32),
}

let editable = Trigger::Step { make: Msg::Nudge, set: Some(Msg::Set), max: 100, value: 98 };
assert!(editable.is_editable());
let start = editable.reading().unwrap();
let moved = editable.stepped(start, 1).and_then(|v| editable.stepped(v, 1));
let moved = moved.and_then(|v| editable.stepped(v, 1));
assert_eq!(moved, Some(100), "held inside 0..=max");
assert_eq!(editable.set_to(100), Some(Msg::Set(100)));

let nudge_only = Trigger::Step { make: Msg::Nudge, set: None, max: 100, value: 98 };
assert!(!nudge_only.is_editable());
assert_eq!(nudge_only.set_to(100), None);
```

**See also:** [`value_at`](#value_at), [`Interactions::declare`](#interactionsdeclare),
[reporting a value](../writing-a-widget.md#reporting-a-value)

## `value_at`

The value a touch at `x` represents within `track`, rounded to nearest.

```text
pub fn value_at(track: Rect, x: i32, max: i32) -> i32
```

| Parameter | Meaning |
|---|---|
| `track` | The track's region, in screen pixels. |
| `x` | The touch position, in the same coordinates. |
| `max` | The top of the range; the result is always in `0..=max`. |

The usable length is the track's width less the theme's side inset at each end
and one knob width, since the knob's centre is what a finger places. A touch
outside it clamps to the nearer end. The inset and knob width come from the
theme, through `ThemeMetric::SliderSideInset` and
`ThemeMetric::SliderKnobWidth`, rather than from constants: the host draws the
knob, and a copy of its dimensions would convert touches against the old
geometry the day the theme changed it.

A widget rarely calls this: a `Trigger::Value` does, inside
[`Trigger::resolve`](#triggerresolve). Call it directly for a control of your own
whose track is drawn by the same theme.

**Example — converting a touch under the fake host**

```rust
use xpui::{Rect, testing, value_at};

testing::install();
let track = Rect::new(0, 0, 200, 40);
// 200 less two insets of 8 and a knob of 14 leaves 170 usable pixels,
// starting 15 pixels in.
let start = testing::SLIDER_SIDE_INSET + testing::SLIDER_KNOB_WIDTH / 2;
assert_eq!(start, 15);

assert_eq!(value_at(track, 0, 100), 0); // before the track clamps
assert_eq!(value_at(track, start, 100), 0);
assert_eq!(value_at(track, 100, 100), 50);
assert_eq!(value_at(track, 185, 100), 100);
assert_eq!(value_at(track, 480, 100), 100); // past the end clamps
```

**See also:** [`Trigger::Value`](#trigger), [`Slider`](controls.md#slider)
