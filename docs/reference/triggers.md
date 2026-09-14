# Triggers

What an interaction produces when it fires. A widget says what kind of thing it
is as it declares a region, and every frame the runtime turns input into the
screen's own message: one the widget built with the tree, or one built from a
value where a touch landed or a step the keys asked for.

[Interactions](interactions.md) is how a widget declares the regions a trigger
belongs to. [Controls](controls.md) is how a slider uses these, and
[Steppers](steppers.md) how a stepper does. This page is what each piece does.

## Topics

| | |
|---|---|
| [`Trigger`](#trigger) | What an interaction produces when it fires. |
| [`value_at`](#value_at) | The value a touch at `x` represents within `track`, rounded to nearest. |

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

**See also:** [`value_at`](#value_at), [`Interactions::declare`](interactions.md#interactionsdeclare),
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
