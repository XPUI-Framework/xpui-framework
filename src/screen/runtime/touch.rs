//! One frame of touch: a drag, a long press, then a tap.

use super::Runtime;
use crate::geometry::Point;
use crate::host::{Input, millis};
use crate::screen::Screen;
use crate::screen::routing::{long_press, resolve};
use crate::view::InputMask;

/// How long a finger rests before a hold becomes a long press.
///
/// The delay before a held key starts repeating, so a finger and a key cross
/// from a press into a hold at the same moment. Time rather than a theme
/// metric: every metric the theme answers is a length in pixels.
const LONG_PRESS_MS: u32 = 500;

/// A finger that has stayed down, and what became of it.
pub(super) struct Hold {
    /// Where the finger was first seen down. A long press resolves here, the
    /// way a tap resolves where the finger went down.
    origin: Point,
    /// When the hold started, or last re-armed.
    since: u32,
    /// When a held frame was last seen, so a blind loop cannot time the hold.
    seen: u32,
    /// Whether the threshold has been crossed, and whether that fired.
    outcome: Outcome,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Outcome {
    /// Still under the threshold.
    Waiting,
    /// A long press fired. The release that ends the hold fires nothing.
    Fired,
    /// Crossed the threshold with nothing to fire, so the release is a tap.
    Declined,
}

impl<S: Screen> Runtime<S> {
    /// One frame of touch. Returns whether it consumed the frame.
    pub(super) fn touch(&mut self) -> bool {
        if let Some(point) = Input::touch_held() {
            let interactions = self.collect_settled();
            if let Some(message) = resolve(&interactions, point, InputMask::DRAG) {
                self.dragging = true;
                self.hold = None;
                self.dispatch(message);
                return true;
            }
            if let Some(origin) = self.crossed(point) {
                let message = long_press(&interactions, origin, point);
                if let Some(hold) = self.hold.as_mut() {
                    hold.outcome = match message {
                        Some(_) => Outcome::Fired,
                        None => Outcome::Declined,
                    };
                }
                if let Some(message) = message {
                    self.dispatch(message);
                    return true;
                }
            }
        }

        let released = Input::touch_released();
        if released && self.dragging {
            // Swallow the release that ended a drag: otherwise it reads as
            // a tap elsewhere and, on an overlay, closes the panel.
            self.dragging = false;
            self.hold = None;
            return true;
        }

        let tap = Input::tap();
        let fired = self
            .hold
            .as_ref()
            .is_some_and(|hold| hold.outcome == Outcome::Fired);
        if released || tap.is_some() {
            self.hold = None;
        }
        // The same finger already said what it meant. Its release is the end
        // of the long press, not a second, shorter one.
        if fired && (released || tap.is_some()) {
            return true;
        }

        let Some(point) = tap else {
            return false;
        };
        let interactions = self.collect_settled();
        if let Some(message) = resolve(&interactions, point, InputMask::TAP) {
            self.dispatch(message);
            return true;
        }
        // Outside every row of a dialog that says how to dismiss it. A dialog
        // without one leaves the tap to the screen, as it always did.
        if interactions.captured_focus().is_some()
            && let Some(message) = interactions.dismissal().cloned()
        {
            self.dispatch(message);
            return true;
        }
        if let Some(message) = self.screen.on_background_tap(point) {
            self.dispatch(message);
            return true;
        }
        false
    }

    /// Times a held finger. Returns where it went down on the frame the hold
    /// crosses the threshold, and `None` on every other.
    ///
    /// The caller settles the outcome, so this answers once per hold.
    fn crossed(&mut self, point: Point) -> Option<Point> {
        let now = millis();
        let hold = self.hold.get_or_insert(Hold {
            origin: point,
            since: now,
            seen: now,
            outcome: Outcome::Waiting,
        });
        let gap = now.wrapping_sub(core::mem::replace(&mut hold.seen, now));
        if hold.outcome != Outcome::Waiting {
            return None;
        }
        // The guard auto-repeat has, for the same reason: a panel blocking
        // through a refresh leaves the loop blind, and crediting that gap to
        // the hold turns a tap into a long press.
        if gap >= LONG_PRESS_MS {
            hold.since = now;
            return None;
        }
        (now.wrapping_sub(hold.since) >= LONG_PRESS_MS).then_some(hold.origin)
    }
}
