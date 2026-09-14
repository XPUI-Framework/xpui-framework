//! Turning one frame of input into a message.
//!
//! Pure functions over the interactions a tree declared. Kept apart from the
//! runtime so the *policy* — which control a touch resolves to, what Confirm
//! fires, which control Left/Right nudge — can be read and tested without the
//! lifecycle around it.

use crate::Point;
use crate::view::{InputMask, Interaction, Interactions};

/// The message from the first interaction under `point` that accepts `kind`.
///
/// Later declarations win: a container declares its own region before its
/// children in a `Tappable`, so scanning backwards resolves to the innermost
/// control.
pub(crate) fn resolve<M: Clone>(
    interactions: &Interactions<M>,
    point: Point,
    kind: InputMask,
) -> Option<M> {
    interactions
        .items()
        .iter()
        .rev()
        .find(|item| item.mask.contains(kind) && item.rect.contains(point))
        .map(|item| item.trigger.resolve(item.rect, point.x))
}

/// The message from the innermost `LONG_PRESS` region under `origin`, where the
/// finger went down, while the finger at `now` is still inside it.
///
/// A finger that slid off the control fires nothing, as a tap released outside
/// its control does not.
pub(crate) fn long_press<M: Clone>(
    interactions: &Interactions<M>,
    origin: Point,
    now: Point,
) -> Option<M> {
    let item = interactions
        .items()
        .iter()
        .rev()
        .find(|item| item.mask.contains(InputMask::LONG_PRESS) && item.rect.contains(origin))?;
    item.rect
        .contains(now)
        .then(|| item.trigger.resolve(item.rect, origin.x))
}

/// The focused interaction, if any.
pub(crate) fn focused<M>(
    interactions: &Interactions<M>,
    focus: usize,
) -> Option<&crate::view::Interaction<M>> {
    interactions
        .items()
        .iter()
        .filter(|item| item.mask.contains(InputMask::FOCUS))
        .nth(focus)
}

/// Whether this is a control the keys move rather than fire.
///
/// **One question, asked in three places**: a control that is adjustable to
/// one of them and not the others opens a mode that nothing can drive.
/// Whether an edit may *open* on it needs a way back as well, and
/// [`Trigger::is_editable`](crate::view::Trigger::is_editable) asks that.
pub(crate) fn adjustable<M: Clone>(item: &Interaction<M>) -> bool {
    item.mask.contains(InputMask::ADJUST)
}

/// A relative nudge for the focused control, for Left/Right.
pub(crate) fn focused_step<M: Clone>(
    interactions: &Interactions<M>,
    focus: usize,
    delta: i32,
) -> Option<M> {
    let item = focused(interactions, focus)?;
    if !adjustable(item) {
        return None;
    }
    item.trigger.resolve_step(delta)
}

/// The focused interaction's message, for Confirm.
pub(crate) fn focused_message<M: Clone>(interactions: &Interactions<M>, focus: usize) -> Option<M> {
    let item = focused(interactions, focus)?;

    // **Confirm never fires a control the keys move.** Asked through
    // `adjustable` rather than of the trigger's shape: an absolute trigger
    // fired from a focus rather than a touch has no position to resolve, and
    // resolves at the centre of its own track.
    if adjustable(item) {
        return None;
    }

    let x = item.rect.x() + item.rect.width() / 2;
    Some(item.trigger.resolve(item.rect, x))
}
