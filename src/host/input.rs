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
    /// Leaves the screen, cancels an open edit, or dismisses a dialog.
    Back = 0,
    /// Acts on whatever has focus.
    Confirm = 1,
    /// Nudges the focused value down; where nothing under the focus
    /// adjusts, moves focus back.
    Left = 2,
    /// Nudges the focused value up; where nothing under the focus adjusts,
    /// moves focus forward.
    Right = 3,
    /// Moves focus to the previous control, or raises an open value.
    Up = 4,
    /// Moves focus to the next control, or lowers an open value.
    Down = 5,
    /// The power key.
    Power = 6,
    /// Page navigation backwards, honouring the user's side-button swap.
    PageBack = 7,
    /// Page navigation forwards, honouring the user's side-button swap.
    PageForward = 8,
    /// The next item, as a reader's side key means it.
    NavNext = 9,
    /// The previous item, as a reader's side key means it.
    NavPrevious = 10,
    /// Left as seen on the rendered screen, whatever the orientation.
    ScreenLeft = 11,
    /// Right as seen on the rendered screen, whatever the orientation.
    ScreenRight = 12,
    /// Up as seen on the rendered screen, whatever the orientation.
    ScreenUp = 13,
    /// Down as seen on the rendered screen, whatever the orientation.
    ScreenDown = 14,
}

/// The direction a completed swipe travelled.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum SwipeDir {
    /// No swipe this frame.
    #[default]
    None = 0,
    /// Towards the left edge.
    Left = 1,
    /// Towards the right edge.
    Right = 2,
    /// Towards the top edge.
    Up = 3,
    /// Towards the bottom edge.
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
    /// Whether `button` went down this frame.
    fn was_pressed(&self, button: Button) -> bool;
    /// Whether `button` is down, this frame included.
    fn is_pressed(&self, button: Button) -> bool;
    /// Whether `button` came up this frame.
    fn was_released(&self, button: Button) -> bool;

    /// Whether this frame carries any touch at all, down or just lifted.
    fn has_touch(&self) -> bool;

    /// Whether the device has a Left/Right pair to nudge a value with.
    ///
    /// With the pair, Left and Right move a value where it stands. Without
    /// one, those keys are busy walking between rows, so the value has to be
    /// entered and left again. A control reads this and branches; nothing in
    /// the framework does so on its behalf.
    ///
    /// **Deliberately not defaulted.** A backend that forgot to answer would
    /// inherit whichever behaviour the default picked, with nothing to notice.
    fn has_left_right_keys(&self) -> bool;

    /// A completed tap, at the position the finger went down.
    fn tap(&self) -> Option<Point>;

    /// Where a finger is while one is down, the signal a slider drag needs.
    fn touch_held(&self) -> Option<Point>;

    /// Whether a finger lifted this frame.
    fn touch_released(&self) -> bool;

    /// A completed swipe, or [`SwipeDir::None`].
    fn swipe(&self) -> SwipeDir;

    /// The system back gesture, an edge swipe on a touch device.
    fn was_back_gesture(&self) -> bool;
    /// The system home gesture, offered to the screen before the host acts.
    fn was_home_gesture(&self) -> bool;

    /// Which way a vertical swipe moves focus.
    ///
    /// `false` means the swipe moves the *content*: swiping up walks **down**
    /// the list, as though dragging the page. `true` moves the focus with the
    /// swipe. Defaulted so a host need not implement it until there is a
    /// setting behind it.
    fn swipe_moves_selection(&self) -> bool {
        false
    }
}

/// One frame of input, as screens reach for it.
pub struct Input;

impl Input {
    /// See [`InputSource::was_pressed`].
    pub fn was_pressed(button: Button) -> bool {
        super::current().was_pressed(button)
    }

    /// See [`InputSource::is_pressed`].
    pub fn is_pressed(button: Button) -> bool {
        super::current().is_pressed(button)
    }

    /// See [`InputSource::was_released`].
    pub fn was_released(button: Button) -> bool {
        super::current().was_released(button)
    }

    /// See [`InputSource::has_touch`].
    pub fn has_touch() -> bool {
        super::current().has_touch()
    }

    /// See [`InputSource::has_left_right_keys`].
    pub fn has_left_right_keys() -> bool {
        super::current().has_left_right_keys()
    }

    /// See [`InputSource::tap`].
    pub fn tap() -> Option<Point> {
        super::current().tap()
    }

    /// See [`InputSource::touch_held`].
    pub fn touch_held() -> Option<Point> {
        super::current().touch_held()
    }

    /// See [`InputSource::touch_released`].
    pub fn touch_released() -> bool {
        super::current().touch_released()
    }

    /// See [`InputSource::swipe`].
    pub fn swipe() -> SwipeDir {
        super::current().swipe()
    }

    /// See [`InputSource::was_back_gesture`].
    pub fn was_back_gesture() -> bool {
        super::current().was_back_gesture()
    }

    /// See [`InputSource::was_home_gesture`].
    pub fn was_home_gesture() -> bool {
        super::current().was_home_gesture()
    }

    /// See [`InputSource::swipe_moves_selection`].
    pub fn swipe_moves_selection() -> bool {
        super::current().swipe_moves_selection()
    }
}

/// What one key along the bottom edge of a device does.
///
/// The vocabulary a [`KeyRow`] is written in. Which *word* each of these
/// paints is not here: only a product knows what language its user reads, so
/// the words are supplied alongside the row.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RowKey {
    /// Leaves the screen, or the value being edited.
    Back,
    /// Acts on whatever has focus.
    Confirm,
    /// Walks a list backwards.
    ///
    /// `Up` on a device with a reader's four keys.
    Previous,
    /// Walks a list forwards.
    ///
    /// `Down` on a device with a reader's four keys.
    Next,
    /// A key with no word in the hint vocabulary, drawn blank.
    ///
    /// Either nothing is mapped to it, or what is has no label, as a power key
    /// does.
    Unassigned,
}

/// What the keys along the bottom edge mean, left to right.
///
/// A hint bar asks two questions — how many slots to divide its band into, and
/// which word goes in each — and a device answers both with its row. A slot
/// with nothing behind it is [`RowKey::Unassigned`] and stays blank: naming a
/// key the device does not have sends a person looking for it.
///
/// Here rather than beside the components that paint it because it is a fact
/// about hardware, and the crate describing a device should not have to depend
/// on the one drawing it to state it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct KeyRow(&'static [RowKey]);

impl KeyRow {
    /// A reader's four keys, Back leftmost.
    pub const READER: KeyRow = KeyRow(&[
        RowKey::Back,
        RowKey::Confirm,
        RowKey::Previous,
        RowKey::Next,
    ]);

    /// A row of a device's own.
    ///
    /// `const`, so a board table can hold one.
    pub const fn new(keys: &'static [RowKey]) -> KeyRow {
        KeyRow(keys)
    }

    /// How many slots the hint band divides into.
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the device has no bottom row at all.
    ///
    /// A device without one reserves no band, and there is nothing to label.
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether some slot carries `key`.
    pub fn contains(&self, key: RowKey) -> bool {
        self.0.contains(&key)
    }

    /// The slots, left to right.
    pub fn iter(&self) -> impl Iterator<Item = RowKey> + use<> {
        self.0.iter().copied()
    }
}
