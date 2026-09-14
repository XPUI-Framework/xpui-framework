//! One frame of input, in the order a screen is offered it: touch, then a
//! swipe, then the buttons and the back gesture — and the screen gets first
//! refusal of each.

use super::{Editing, Repeat, Runtime};
use crate::host::{Button, Input, SwipeDir, finish_screen, millis, request_update};
use crate::screen::Screen;
use crate::screen::routing::{adjustable, focused, focused_message, focused_step};

/// How long a press is held before it starts repeating.
const REPEAT_DELAY_MS: u32 = 500;
/// The gap between one repeat and the next.
const REPEAT_INTERVAL_MS: u32 = 500;

/// Every button, each offered to the screen before the runtime gives it a
/// meaning. The last seven have none of the runtime's; a screen that wants one
/// claims it, and one that does not loses nothing.
const KEYS: [Button; 15] = [
    Button::Left,
    Button::Right,
    Button::Up,
    Button::Down,
    Button::Confirm,
    Button::Back,
    Button::PageBack,
    Button::PageForward,
    Button::Power,
    Button::NavNext,
    Button::NavPrevious,
    Button::ScreenLeft,
    Button::ScreenRight,
    Button::ScreenUp,
    Button::ScreenDown,
];

impl<S: Screen> Runtime<S> {
    /// Which button, if any, should act this frame — including auto-repeat for
    /// one held down.
    fn active_key(&mut self) -> Option<Button> {
        let now = millis();
        // Every frame that reaches here has looked at input, whatever it
        // decides — so the gap below measures blindness, not inactivity.
        let seen_at = core::mem::replace(&mut self.repeat.seen_at, now);

        // The back gesture is Back, arriving by another road: offered to the
        // screen, cancelling an edit, dismissing a dialog, leaving. An edge
        // with nothing held behind it, so it never repeats.
        if Input::was_back_gesture() {
            self.repeat.button = None;
            return Some(Button::Back);
        }

        for key in KEYS {
            if Input::was_pressed(key) {
                self.repeat = Repeat {
                    button: Some(key),
                    pressed_at: now,
                    fired_at: now,
                    seen_at: now,
                };
                return Some(key);
            }
        }

        // Repeat only while the same button is still down.
        let held = self.repeat.button?;
        if !Input::is_pressed(held) {
            self.repeat.button = None;
            return None;
        }
        // A gap longer than a repeat period re-arms the hold rather than
        // firing it. A panel that blocks for most of a second while it
        // refreshes leaves the loop blind, and a button released during the
        // refresh still reads as down on the frame after — input is sampled
        // at the top of a frame. Crediting that gap to the hold turns one tap
        // into a run; what is lost is a repeat earned by holding through a
        // refresh, what is kept is that a tap moves by one.
        if now.wrapping_sub(seen_at) >= REPEAT_INTERVAL_MS {
            self.repeat.pressed_at = now;
            self.repeat.fired_at = now;
            return None;
        }
        if now.wrapping_sub(self.repeat.pressed_at) < REPEAT_DELAY_MS {
            return None;
        }
        if now.wrapping_sub(self.repeat.fired_at) < REPEAT_INTERVAL_MS {
            return None;
        }
        self.repeat.fired_at = now;
        Some(held)
    }

    /// One frame of input while a value is open.
    ///
    /// Up **raises** and Down lowers, which is the opposite of what they mean
    /// in a list — there Up walks towards the top, here it walks the number
    /// upwards. Reading the two side by side looks like a sign error and is
    /// not.
    fn edit_key(&mut self, key: Button) {
        let Some(&Editing { focus, value }) = self.editing.as_ref() else {
            return;
        };

        match key {
            Button::Up | Button::Down | Button::PageBack | Button::PageForward => {
                let delta = if matches!(key, Button::Up | Button::PageBack) {
                    1
                } else {
                    -1
                };
                self.step_open(focus, value, delta);
            }
            // A board with this pair never opens an edit, so these arrive only
            // from a host that sends them while answering that it has no pair.
            // Moved like everything else, so the value tracks the keys.
            Button::Left | Button::Right => {
                let delta = if key == Button::Left { -1 } else { 1 };
                self.step_open(focus, value, delta);
            }
            // **Commit: the screen's one and only sight of this edit.** Every
            // key up to here moved the framework's copy and told the screen
            // nothing, so a screen that writes to flash on every change writes
            // once, and what it writes is the number the panel was showing.
            Button::Confirm => {
                self.editing = None;
                let interactions = self.collect_settled();
                if let Some(item) = focused(&interactions, focus)
                    && let Some(message) = item.trigger.set_to(value)
                {
                    self.dispatch(message);
                }
                request_update();
            }
            // **Cancel dispatches nothing at all.** The screen's value never
            // moved, so dropping the copy is already the value it opened on —
            // exact for a screen that clamps and one that scales alike, which
            // no sum of undo steps could be.
            //
            // And **do not leave the screen**: Back is the first key on the
            // boards this mode exists for, so a stray press costs the edit and
            // nothing more.
            Button::Back => {
                self.editing = None;
                request_update();
            }
            _ => {}
        }
    }

    /// Moves the open edit's own copy by one step, telling the screen nothing.
    fn step_open(&mut self, focus: usize, from: i32, delta: i32) {
        let interactions = self.collect_settled();
        let Some(item) = focused(&interactions, focus) else {
            return;
        };
        let Some(value) = item.trigger.stepped(from, delta) else {
            return;
        };
        if value == from {
            // Already against the end of the track. Repainting an identical
            // frame is a second of a slow panel's life for nothing.
            return;
        }
        self.editing = Some(Editing { focus, value });
        request_update();
    }

    /// One frame of input, in priority order. See the module docs.
    pub(in crate::screen) fn loop_(&mut self) {
        // Before anything else, and on every frame including the quiet ones:
        // a screen holding a deadline has to hear that one passed.
        self.screen.tick();

        // -- touch ----------------------------------------------------------
        //
        // **Declined while a value is open.** Touch and swipe run ahead of the
        // keys and know nothing about the edit, so a finger could set the value
        // the keys are driving, or walk the focus out from under it and leave
        // the keys acting on a control the highlight has left. A mode only some
        // inputs respect is not a mode.
        if self.editing.is_none() && self.painted && Input::has_touch() {
            if self.touch() {
                return;
            }
        } else {
            // A hold this frame cannot see is over, whatever ended it.
            self.hold = None;
        }

        // -- swipe ----------------------------------------------------------
        // A vertical swipe anywhere moves focus. Declined during an edit, as
        // touch is. Which way is the host's preference: see
        // `InputSource::swipe_moves_selection`.
        if self.editing.is_none() && self.painted {
            let direction = Input::swipe();
            if direction != SwipeDir::None {
                if let Some(message) = self.screen.on_swipe(direction) {
                    self.dispatch(message);
                    return;
                }

                // Left/Right are left alone: the back and home gestures own
                // that axis, and claiming it here would break navigation.
                let forward = if Input::swipe_moves_selection() {
                    -1
                } else {
                    1
                };
                let delta = match direction {
                    SwipeDir::Up => forward,
                    SwipeDir::Down => -forward,
                    _ => 0,
                };
                if delta != 0 {
                    let count = self.collect_settled().focusable_count();
                    if self.move_focus(delta, count) {
                        request_update();
                    }
                    return;
                }
            }
        }

        // -- buttons --------------------------------------------------------
        let Some(key) = self.active_key() else { return };

        // The screen gets first refusal, so a screen wanting Up/Down for
        // something other than focus simply claims them.
        if let Some(message) = self.screen.on_key(key) {
            self.dispatch(message);
            return;
        }

        // While a value is open, the same four keys mean something else. Kept
        // ahead of the ordinary match rather than threaded through it, so the
        // two readings never half-apply.
        if self.editing.is_some() {
            self.edit_key(key);
            return;
        }

        match key {
            Button::Confirm => {
                let interactions = self.collect_settled();

                // Confirm opens an adjustable control rather than firing it —
                // and only where there is no pair to nudge it with, because
                // that pair is otherwise the way in. `focused_message` declines
                // the same controls, which is what leaves this branch free.
                if !Input::has_left_right_keys()
                    && let Some(item) = focused(&interactions, self.focus)
                    && adjustable(item)
                    && let Some(start) = item.trigger.reading()
                    && item.trigger.is_editable()
                {
                    self.editing = Some(Editing {
                        focus: self.focus,
                        value: start,
                    });
                    request_update();
                    return;
                }

                if let Some(message) = focused_message(&interactions, self.focus) {
                    self.dispatch(message);
                }
            }
            // The page pair. On a reader these turn pages; everywhere else they
            // are the second way to walk a list, which is what the devices with
            // only two side keys rely on.
            Button::Up | Button::Down | Button::PageBack | Button::PageForward => {
                let interactions = self.collect_settled();
                let count = interactions.focusable_count();
                let delta = if matches!(key, Button::Up | Button::PageBack) {
                    -1
                } else {
                    1
                };
                if self.move_focus(delta, count) || self.scroll_by(delta, &interactions) {
                    request_update();
                }
            }
            Button::Left | Button::Right => {
                // Nudge whatever holds focus, so one pair of keys drives every
                // adjustable control instead of the screen wiring them to one.
                // When nothing under the focus adjusts, they walk the list: a
                // screen with no slider on it would otherwise have two dead
                // keys.
                let interactions = self.collect_settled();
                let delta = if key == Button::Left { -1 } else { 1 };

                if let Some(message) = focused_step(&interactions, self.focus, delta) {
                    self.dispatch(message);
                } else {
                    let count = interactions.focusable_count();
                    let step = delta as isize;
                    if self.move_focus(step, count) || self.scroll_by(step, &interactions) {
                        request_update();
                    }
                }
            }
            Button::Back => self.back(),
            _ => {}
        }
    }

    /// Back that nothing claimed and no edit took.
    ///
    /// **Never finishes the screen from under a dialog.** The dialog is what
    /// the reader is looking at, so Back is about the dialog: it sends the
    /// dialog's dismiss message, or, when it has none, does nothing at all.
    fn back(&mut self) {
        let interactions = self.collect_settled();
        if interactions.captured_focus().is_none() {
            finish_screen();
            return;
        }
        if let Some(message) = interactions.dismissal().cloned() {
            self.dispatch(message);
        }
    }
}
