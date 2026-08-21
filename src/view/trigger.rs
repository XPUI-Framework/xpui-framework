//! What a declaration turns into: the message a touch or a key produces.
//!
//! A widget says *what kind of thing* it is — a row, an absolute value, a
//! relative nudge — and the runtime turns one frame of input into the screen's
//! own message. Separate from the declaration itself because the arithmetic
//! lives here: converting a touch position into a value, nudging an absolute
//! control by one step, and moving the copy an open edit holds.

use alloc::boxed::Box;

use crate::geometry::Rect;
use crate::host::{Theme, ThemeMetric};

/// What an interaction produces when it fires.
///
/// [`Trigger::Message`] covers buttons, rows and toggles: the widget knows what
/// it means, so it builds the message when the tree is built. [`Trigger::Value`]
/// is for controls whose message depends on *where* the touch landed — the
/// framework converts the position and calls the constructor, so no screen
/// re-derives slider geometry.
pub enum Trigger<M> {
    Message(M),
    Value {
        make: fn(i32) -> M,
        max: i32,
        /// What the control reads right now.
        ///
        /// Carried because an absolute control cannot be *nudged* without it:
        /// Left and Right ask for one step, and one step of an absolute value
        /// is `value + delta`.
        ///
        /// Rebuilt with the tree every frame, so it is the screen's own
        /// reading and never a stale copy the framework kept.
        value: i32,
    },
    /// A relative nudge: `-1` or `+1` from Left/Right, or from a `-`/`+` glyph.
    /// Distinct from [`Trigger::Value`] because the screen adds the delta to
    /// whatever it currently holds, rather than being handed an absolute.
    Step {
        make: fn(i32) -> M,
        /// The top of the control's own range, so a held edit can clamp.
        ///
        /// A relative control does not need it to nudge — the screen adds the
        /// delta to whatever it holds and clamps however it likes. An open edit
        /// does: the framework owns the value while it is open, and a working
        /// copy nothing bounds marches past the end of the track and commits a
        /// number the screen never showed.
        max: i32,
        /// The message that sets this control outright, when it has one.
        ///
        /// **What makes a relative control editable.** `make` takes a *nudge*,
        /// and a nudge is worth whatever the screen decides — a frontlight row
        /// reads one as five units. So a nudge cannot express "the value is
        /// this now", which is the one message an open edit sends: it holds the
        /// value while the keys move it and commits an absolute at Confirm.
        /// Without a setter a control can still be nudged; it just cannot be
        /// edited, because there would be no way to commit what was shown.
        set: Option<fn(i32) -> M>,
        /// What the control reads right now — see [`Trigger::Value::value`].
        ///
        /// A relative control does not need it to step, but an editor does: it
        /// is what an edit opens on.
        value: i32,
    },
    /// A value control seen through [`ViewExt::map`](crate::view::ViewExt::map).
    ///
    /// Composing two function pointers is not itself a function pointer, so a
    /// mapped value control is the one case that needs a closure. It costs one
    /// small allocation per touch frame, and only for components that actually
    /// wrap a slider — the alternative was dropping the interaction, which
    /// would silently make the control dead.
    MappedValue {
        make: Box<dyn Fn(i32) -> M>,
        max: i32,
        value: i32,
    },
    /// A step control seen through [`ViewExt::map`](crate::view::ViewExt::map); see [`Trigger::MappedValue`].
    MappedStep {
        make: Box<dyn Fn(i32) -> M>,
        set: Option<Box<dyn Fn(i32) -> M>>,
        max: i32,
        value: i32,
    },
}

impl<M: Clone> Trigger<M> {
    /// Resolves to a message. `x` is the touch position, ignored by controls
    /// that do not depend on it.
    ///
    /// Public so a test can assert what a control *would* send without driving
    /// the whole runtime.
    pub fn resolve(&self, rect: Rect, x: i32) -> M {
        match self {
            Trigger::Message(message) => message.clone(),
            Trigger::Value { make, max, .. } => make(value_at(rect, x, *max)),
            Trigger::MappedValue { make, max, .. } => make(value_at(rect, x, *max)),
            // A nudge has no position to resolve, so a `Confirm` that reached
            // one would ask for a step of nothing. It does not: `focused_message`
            // declines anything carrying `ADJUST`, which is every control that
            // gets here.
            Trigger::Step { make, .. } => make(0),
            Trigger::MappedStep { make, .. } => make(0),
        }
    }

    /// The message for a relative nudge, or `None` for a control that has no
    /// meaningful step.
    ///
    /// An absolute control is nudged by resolving `value + delta` against its
    /// own bounds, so Left and Right drive a `Slider` and a `Stepper` the same
    /// way from a screen's point of view. Clamped here rather than left to the
    /// screen: a screen that clamps is common, one that wraps or rejects is
    /// not, and the framework must not need to know which it got.
    pub fn resolve_step(&self, delta: i32) -> Option<M> {
        match self {
            Trigger::Step { make, .. } => Some(make(delta)),
            Trigger::MappedStep { make, .. } => Some(make(delta)),
            // `max.max(0)` for the same reason as `stepped`: `i32::clamp`
            // panics when its low bound exceeds its high, and a control with no
            // room to move is empty everywhere else rather than an error.
            Trigger::Value { make, max, value } => {
                Some(make(value.saturating_add(delta).clamp(0, (*max).max(0))))
            }
            Trigger::MappedValue { make, max, value } => {
                Some(make(value.saturating_add(delta).clamp(0, (*max).max(0))))
            }
            Trigger::Message(_) => None,
        }
    }

    /// What the control reads now, for an editor to open on.
    ///
    /// Read once, on the frame an edit opens, to seed the copy the framework
    /// then owns. Cancel needs nothing from it: the screen's value never moved.
    ///
    /// `None` for a control with no reading at all — a row, a button — which is
    /// also every control an edit can never open on.
    pub fn reading(&self) -> Option<i32> {
        match self {
            Trigger::Value { value, .. }
            | Trigger::Step { value, .. }
            | Trigger::MappedValue { value, .. }
            | Trigger::MappedStep { value, .. } => Some(*value),
            Trigger::Message(_) => None,
        }
    }

    /// The message that sets this control to `value` outright, or `None` for
    /// one that can only be nudged.
    ///
    /// Absolute by construction. A relative control has one only if it was
    /// given one — see [`Trigger::Step::set`].
    pub fn set_to(&self, value: i32) -> Option<M> {
        match self {
            Trigger::Value { make, .. } => Some(make(value)),
            Trigger::MappedValue { make, .. } => Some(make(value)),
            Trigger::Step { set, .. } => set.map(|set| set(value)),
            Trigger::MappedStep { set, .. } => set.as_ref().map(|set| set(value)),
            Trigger::Message(_) => None,
        }
    }

    /// Whether an edit can open on this control at all.
    ///
    /// It needs a reading to open on and a way to be **set outright**, because
    /// that is what Confirm dispatches. A control that can only be nudged would
    /// give a person a mode they could enter and move and never commit, which
    /// is worse than not offering the mode.
    pub fn is_editable(&self) -> bool {
        match self {
            // Absolute by construction: it can always be set to what it read.
            Trigger::Value { .. } | Trigger::MappedValue { .. } => true,
            Trigger::Step { set, .. } => set.is_some(),
            Trigger::MappedStep { set, .. } => set.is_some(),
            Trigger::Message(_) => false,
        }
    }

    /// One step from `from`, held inside the control's own range.
    ///
    /// **For a value the framework is holding, not one the screen owns.** An
    /// open edit keeps its own copy and paints from it, so the arithmetic has
    /// to happen here rather than in the screen's `update`; `resolve_step` is
    /// the other half, for the boards that nudge a value in place.
    ///
    /// A step is one unit of the control's range, including for
    /// [`Trigger::Step`], whose `make` a screen is free to scale. That scale is
    /// what a *nudge* is worth to a screen holding its own value; inside an
    /// open edit the framework holds it, and a unit is a unit of the track the
    /// knob is moving along.
    ///
    /// `None` for a control with no value at all — a row, a button.
    ///
    /// A control whose `max` is not positive has a value and no room to move
    /// it, and answers `0`: the rest of the framework treats such a control as
    /// empty rather than as an error — `Slider::render` and the chrome's
    /// `draw_slider` both decline to paint one — and `i32::clamp` **panics**
    /// when its low bound exceeds its high, which on a device is an abort.
    pub fn stepped(&self, from: i32, delta: i32) -> Option<i32> {
        let max = match self {
            Trigger::Value { max, .. }
            | Trigger::MappedValue { max, .. }
            | Trigger::Step { max, .. }
            | Trigger::MappedStep { max, .. } => *max,
            Trigger::Message(_) => return None,
        };
        Some(from.saturating_add(delta).clamp(0, max.max(0)))
    }
}

/// The value a touch at `x` represents within `track`.
///
/// Rounds to nearest, matching the `(permille * range + 500) / 1000` the C++
/// slider screens use, so dragging feels identical in both.
///
/// The inset and knob width come from the theme rather than from constants
/// here: the host draws the knob, and a copy of its dimensions would keep
/// converting touches against the old geometry the day the theme changed it.
pub fn value_at(track: Rect, x: i32, max: i32) -> i32 {
    let inset = Theme::metric(ThemeMetric::SliderSideInset);
    let knob = Theme::metric(ThemeMetric::SliderKnobWidth);

    let usable = (track.width() - inset * 2 - knob).max(1);
    let offset = (x - track.x() - inset - knob / 2).clamp(0, usable);
    (offset * max + usable / 2) / usable
}
