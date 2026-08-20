//! Buttons, touch and gestures.

use crate::geometry::Point;

/// A button by meaning, never by physical position.
///
/// The host applies the user's front-button remapping and the screen
/// orientation, so a screen asking for `Confirm` gets whatever the user has
/// decided that is.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Button {
    Back = 0,
    Confirm = 1,
    Left = 2,
    Right = 3,
    Up = 4,
    Down = 5,
    Power = 6,
    /// Page navigation, honouring the user's side-button swap.
    PageBack = 7,
    PageForward = 8,
    NavNext = 9,
    NavPrevious = 10,
    /// Direction as seen on the rendered screen, whatever the orientation.
    ScreenLeft = 11,
    ScreenRight = 12,
    ScreenUp = 13,
    ScreenDown = 14,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum SwipeDir {
    #[default]
    None = 0,
    Left = 1,
    Right = 2,
    Up = 3,
    Down = 4,
}

/// One frame of input, and what the device it came from can do.
///
/// Edge queries are true for exactly one frame. Nothing is consumed by reading,
/// so the framework may ask the same question more than once per frame.
///
/// One of these is not about a frame at all:
/// [`has_left_right_keys`](InputSource::has_left_right_keys) describes the
/// device and answers the same thing every time it is asked. Everything else
/// here is about *this* frame — [`has_touch`](InputSource::has_touch) included,
/// which reports whether this frame carries any touch at all, down or just
/// lifted, and never whether the panel has a digitiser.
pub trait InputSource {
    fn was_pressed(&self, button: Button) -> bool;
    fn is_pressed(&self, button: Button) -> bool;
    fn was_released(&self, button: Button) -> bool;

    fn has_touch(&self) -> bool;

    /// Whether the device has a Left/Right pair to nudge a value with.
    ///
    /// A control that changes a value needs to know. With the pair, Left and
    /// Right move the value where it stands. Without one, the keys that would
    /// do the nudging are already busy walking between rows, so the value has
    /// to be entered and left again instead.
    ///
    /// **Deliberately not defaulted.** A backend that forgot to answer would
    /// inherit whichever behaviour the default picked, on every device it
    /// drives, with nothing to notice.
    ///
    /// Answering it is not the same as acting on it: a control has to read this
    /// and branch. Nothing in the framework does so on the caller's behalf.
    fn has_left_right_keys(&self) -> bool;

    /// A completed tap, at the position the finger went down.
    fn tap(&self) -> Option<Point>;

    /// True while a finger is down, reporting where it is now — the signal a
    /// slider drag needs.
    fn touch_held(&self) -> Option<Point>;

    fn touch_released(&self) -> bool;

    fn swipe(&self) -> SwipeDir;

    fn was_back_gesture(&self) -> bool;
    fn was_home_gesture(&self) -> bool;

    /// Which way a vertical swipe moves focus.
    ///
    /// `false` — the default, and what the C++ screens do today — means the
    /// swipe moves the *content*: swiping up walks **down** the list, as though
    /// dragging the page upwards. `true` reverses it, so a swipe up moves focus
    /// up, which is what someone expects if they read the gesture as moving the
    /// selection rather than the page.
    ///
    /// Defaulted so a host need not implement it until there is a setting
    /// behind it.
    fn swipe_moves_selection(&self) -> bool {
        false
    }
}

/// One frame of input, as screens reach for it.
pub struct Input;

impl Input {
    pub fn was_pressed(button: Button) -> bool {
        super::current().was_pressed(button)
    }

    pub fn is_pressed(button: Button) -> bool {
        super::current().is_pressed(button)
    }

    pub fn was_released(button: Button) -> bool {
        super::current().was_released(button)
    }

    pub fn has_touch() -> bool {
        super::current().has_touch()
    }

    /// See [`InputSource::has_left_right_keys`].
    pub fn has_left_right_keys() -> bool {
        super::current().has_left_right_keys()
    }

    pub fn tap() -> Option<Point> {
        super::current().tap()
    }

    pub fn touch_held() -> Option<Point> {
        super::current().touch_held()
    }

    pub fn touch_released() -> bool {
        super::current().touch_released()
    }

    pub fn swipe() -> SwipeDir {
        super::current().swipe()
    }

    pub fn was_back_gesture() -> bool {
        super::current().was_back_gesture()
    }

    pub fn was_home_gesture() -> bool {
        super::current().was_home_gesture()
    }

    /// See [`InputSource::swipe_moves_selection`].
    pub fn swipe_moves_selection() -> bool {
        super::current().swipe_moves_selection()
    }
}
