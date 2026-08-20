//! One frame of input, in the order a screen is offered it.

use super::{Editing, Repeat, Runtime};
use crate::host::{Button, Input, SwipeDir, finish_screen, millis, request_update};
use crate::screen::Screen;
use crate::screen::routing::{adjustable, focused, focused_message, focused_step, resolve};
use crate::view::InputMask;

/// Fire once on press, then repeat after this hold, at this interval.
/// Mirrors `ButtonNavigator` (continuousStartMs / continuousIntervalMs).
const REPEAT_DELAY_MS: u32 = 500;
const REPEAT_INTERVAL_MS: u32 = 500;

/// Buttons the runtime offers a screen before claiming them itself.
const KEYS: [Button; 8] = [
    Button::Left,
    Button::Right,
    Button::Up,
    Button::Down,
    Button::Confirm,
    Button::Back,
    Button::PageBack,
    Button::PageForward,
];

impl<S: Screen> Runtime<S> {
    /// Which button, if any, should act this frame — including auto-repeat for
    /// one held down.
    fn active_key(&mut self) -> Option<Button> {
        let now = millis();
        // Every frame that reaches here has looked at input, whatever it
        // decides — so the gap below measures blindness, not inactivity.
        let seen_at = core::mem::replace(&mut self.repeat.seen_at, now);

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
        // A hold is something the loop watches, not something it works out
        // afterwards from a clock. A panel that blocks for most of a second
        // while it refreshes leaves the loop blind for that whole time, and a
        // button released during it still reads as down on the frame after —
        // input is sampled at the top of a frame, and the frame that presented
        // sampled it before the press had done anything. Crediting that gap to
        // the hold turns one press into a run of them: on hardware with a
        // slow panel, a single tap of Down walked the selection several rows.
        //
        // So a gap longer than a whole repeat period re-arms the hold instead
        // of firing it. What is lost is a repeat the person had genuinely
        // earned by holding through a refresh; what is kept is that a tap
        // moves by one. Below the threshold nothing changes, and a display
        // that draws straight through never reaches it.
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
        let Some(editing) = self.editing.as_ref() else {
            return;
        };
        let focus = editing.focus;

        match key {
            Button::Up | Button::Down | Button::PageBack | Button::PageForward => {
                let delta = if matches!(key, Button::Up | Button::PageBack) {
                    1
                } else {
                    -1
                };
                let interactions = self.collect_settled();
                if let Some(message) = focused_step(&interactions, focus, delta) {
                    self.dispatch(message);
                }
            }
            // A board that has this pair never opens an edit, so these arrive
            // only from a host that sends them while answering that it has no
            // pair — the simulator's keyboard does exactly that. Routed to the
            // same `focused_step` as everything else so the value moves the way
            // it would anywhere, rather than being silently dropped.
            Button::Left | Button::Right => {
                let delta = if key == Button::Left { -1 } else { 1 };
                let interactions = self.collect_settled();
                if let Some(message) = focused_step(&interactions, focus, delta) {
                    self.dispatch(message);
                }
            }
            // Keep what it now reads.
            Button::Confirm => {
                self.editing = None;
                request_update();
            }
            // Put it back, and **do not leave the screen**. Back is the first
            // key on the boards this mode exists for, so a stray press must
            // cost the edit and nothing more.
            Button::Back => {
                let start = self.editing.take().map(|editing| editing.start);
                if let Some(start) = start {
                    let interactions = self.collect_settled();
                    if let Some(item) = focused(&interactions, focus)
                        && let Some(message) = item.trigger.restore(start)
                    {
                        self.dispatch(message);
                    }
                }
                request_update();
            }
            _ => {}
        }
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
            if let Some(point) = Input::touch_held() {
                let interactions = self.collect_settled();
                if let Some(message) = resolve(&interactions, point, InputMask::DRAG) {
                    self.dragging = true;
                    self.dispatch(message);
                    return;
                }
            }

            if Input::touch_released() && self.dragging {
                // Swallow the release that ended a drag: otherwise it reads as
                // a tap elsewhere and, on an overlay, closes the panel.
                self.dragging = false;
                return;
            }

            if let Some(point) = Input::tap() {
                let interactions = self.collect_settled();
                if let Some(message) = resolve(&interactions, point, InputMask::TAP) {
                    self.dispatch(message);
                    return;
                }
                if let Some(message) = self.screen.on_background_tap(point) {
                    self.dispatch(message);
                    return;
                }
            }
        }

        // -- swipe ----------------------------------------------------------
        // A vertical swipe anywhere moves focus, which is what the C++ screens
        // do (HomeActivity). Declined during an edit, as touch is. Which way is a preference, because both readings
        // are defensible: by default the swipe drags the *content*, so swiping
        // up walks down the list; with `swipe_moves_selection` it drags the
        // *selection*, so swiping up moves focus up like Button::Up.
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
                        start,
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
                //
                // When nothing under the focus adjusts, they walk the list
                // instead. These are the third and fourth keys of a reader's
                // bottom row, and the firmware labels them Up and Down for
                // exactly that reason — a screen with no slider on it would
                // otherwise have two dead keys.
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
            Button::Back => finish_screen(),
            _ => {}
        }
    }
}
