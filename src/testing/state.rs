//! Everything the fake remembers between calls, and the only place a test
//! writes to it.

use alloc::vec::Vec;
use core::cell::RefCell;

use super::ops::DrawOp;
use crate::{Button, SwipeDir};

thread_local! {
    static OPS: RefCell<Vec<DrawOp>> = const { RefCell::new(Vec::new()) };
    pub(super) static NOW: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
    pub(super) static SWIPE: core::cell::Cell<SwipeDir> = const { core::cell::Cell::new(SwipeDir::None) };
    pub(super) static PRESSED: core::cell::Cell<Option<Button>> = const { core::cell::Cell::new(None) };
    pub(super) static HELD: core::cell::Cell<Option<Button>> = const { core::cell::Cell::new(None) };
    pub(super) static SWIPE_MOVES_SELECTION: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    pub(super) static HAS_LEFT_RIGHT_KEYS: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    pub(super) static FINISHES: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
    pub(super) static UPDATES: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
    pub(super) static PRESENTS: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
    /// Whether the frame now being reported has looked at its edges yet.
    static READ: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    pub(super) static UNAVAILABLE_TEXT: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
}

pub(super) fn push(op: DrawOp) {
    OPS.with(|ops| ops.borrow_mut().push(op));
}

/// Forgets every recorded draw, pending input, input flag and counter.
///
/// Call at the start of each test. It also ends the frame, as [`next_frame`]
/// does. The clock [`set_millis`] moved stays where it is.
pub fn reset() {
    OPS.with(|ops| ops.borrow_mut().clear());
    next_frame();
    HELD.with(|held| held.set(None));
    SWIPE_MOVES_SELECTION.with(|flag| flag.set(false));
    HAS_LEFT_RIGHT_KEYS.with(|flag| flag.set(false));
    FINISHES.with(|count| count.set(0));
    UPDATES.with(|count| count.set(0));
    PRESENTS.with(|count| count.set(0));
    UNAVAILABLE_TEXT.with(|count| count.set(0));
}

/// Ends the frame the fake is reporting input for.
///
/// A press or swipe is an edge: it reads the same however often one frame asks,
/// and is gone on the next. Call this between two frames that should not both
/// see it. A button put down with [`hold`] stays down.
///
/// Rarely needed by name. The first [`press`], [`hold`], [`release`],
/// [`set_swipe`] or [`set_millis`] after a frame has read input starts the next
/// frame on its own, so each write describes the frame that reads it.
pub fn next_frame() {
    SWIPE.with(|swipe| swipe.set(SwipeDir::None));
    PRESSED.with(|pressed| pressed.set(None));
    READ.with(|read| read.set(false));
}

/// Notes that a frame has read its edges, so the next write belongs to another.
pub(super) fn mark_read() {
    READ.with(|read| read.set(true));
}

/// Starts a new frame if the current one has already been read.
fn after_read() {
    if READ.with(core::cell::Cell::get) {
        next_frame();
    }
}

/// How many times text in font id `0` was handed to the fake since the last
/// [`reset`]. The fake records none of them, so this is the only trace.
#[cfg(test)]
pub(crate) fn unavailable_text_draws() -> u32 {
    UNAVAILABLE_TEXT.with(core::cell::Cell::get)
}

/// Everything drawn since the last [`reset`], in order.
pub fn ops_log() -> Vec<DrawOp> {
    OPS.with(|ops| ops.borrow().clone())
}

/// Reports one swipe to the next frame the runtime reads input, so navigation
/// can be tested without a finger.
///
/// It reads the same for the whole frame, however often that frame asks, and is
/// cleared when the frame ends: see [`next_frame`].
pub fn set_swipe(direction: SwipeDir) {
    after_read();
    SWIPE.with(|swipe| swipe.set(direction));
}

/// Reports one button press to the next frame the runtime reads input.
///
/// It reads the same for the whole frame, however often that frame asks, and is
/// cleared when the frame ends: see [`next_frame`].
pub fn press(button: Button) {
    after_read();
    PRESSED.with(|pressed| pressed.set(Some(button)));
}

/// Reports the edge *and* leaves the button down, as a finger does.
///
/// [`press`] alone is a key tapped so briefly that no frame ever saw it held,
/// which is not what hardware sends. Ended with [`release`].
pub fn hold(button: Button) {
    press(button);
    HELD.with(|held| held.set(Some(button)));
}

/// Lifts whatever [`hold`] put down.
pub fn release() {
    after_read();
    HELD.with(|held| held.set(None));
}

/// Chooses which way a swipe moves focus, so both readings can be tested.
///
/// `false` until set, and reset to `false` by [`reset`].
/// See [`InputSource::swipe_moves_selection`](crate::host::InputSource::swipe_moves_selection).
pub fn set_swipe_moves_selection(enabled: bool) {
    SWIPE_MOVES_SELECTION.with(|flag| flag.set(enabled));
}

/// Says whether the device the fake stands for has a Left/Right pair, so a
/// control that branches on it can be tested both ways.
///
/// `false` until set, and reset to `false` by [`reset`].
/// See [`InputSource::has_left_right_keys`](crate::host::InputSource::has_left_right_keys).
pub fn set_has_left_right_keys(present: bool) {
    HAS_LEFT_RIGHT_KEYS.with(|flag| flag.set(present));
}

/// Moves the fake clock, so repeat timing is deterministic.
pub fn set_millis(value: u32) {
    after_read();
    NOW.with(|now| now.set(value));
}

/// How many times a screen asked to be finished since the last [`reset`].
pub fn finishes() -> u32 {
    FINISHES.with(|count| count.get())
}

/// How many repaints were requested since the last [`reset`].
pub fn updates() -> u32 {
    UPDATES.with(|count| count.get())
}

/// How many screens were offered to the navigator since the last [`reset`].
pub fn presents() -> u32 {
    PRESENTS.with(|count| count.get())
}
