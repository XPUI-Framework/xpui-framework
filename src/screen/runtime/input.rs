//! One frame of input, in the order a screen is offered it.

use super::{Repeat, Runtime};
use crate::host::{Button, Input, SwipeDir, finish_screen, millis, request_update};
use crate::screen::Screen;
use crate::screen::routing::{focused_message, focused_step, resolve};
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

        for key in KEYS {
            if Input::was_pressed(key) {
                self.repeat = Repeat {
                    button: Some(key),
                    pressed_at: now,
                    fired_at: now,
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
        if now.wrapping_sub(self.repeat.pressed_at) < REPEAT_DELAY_MS {
            return None;
        }
        if now.wrapping_sub(self.repeat.fired_at) < REPEAT_INTERVAL_MS {
            return None;
        }
        self.repeat.fired_at = now;
        Some(held)
    }

    /// One frame of input, in priority order. See the module docs.
    pub(in crate::screen) fn loop_(&mut self) {
        // Before anything else, and on every frame including the quiet ones:
        // a screen holding a deadline has to hear that one passed.
        self.screen.tick();

        // -- touch ----------------------------------------------------------
        if self.painted && Input::has_touch() {
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
        // do (HomeActivity). Which way is a preference, because both readings
        // are defensible: by default the swipe drags the *content*, so swiping
        // up walks down the list; with `swipe_moves_selection` it drags the
        // *selection*, so swiping up moves focus up like Button::Up.
        if self.painted {
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

        match key {
            Button::Confirm => {
                let interactions = self.collect_settled();
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
