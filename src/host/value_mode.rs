//! What the runtime wants the hint bar to say, over whatever the screen asked
//! for.
//!
//! The screen builds the bar — [`NavigationScreen`](crate::NavigationScreen)
//! carries four [`Hint`](super::Hint)s and paints them — but the screen does
//! not know a value is open, and must not: a screen that could read the mode
//! would start branching on it, and the mode belongs to the framework. So the
//! runtime writes here and the chrome reads it while painting, which is the one
//! place both are in scope.

/// What the focused control is doing, for the hint bar.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum ValueMode {
    /// Nothing is focused that a key could open. The screen's hints stand.
    #[default]
    None = 0,
    /// The focused control can be opened: Confirm offers to.
    Openable = 1,
    /// The focused control is open: Confirm keeps it, Back drops it.
    Open = 2,
}

/// Says what the focused control is doing. Called once a frame by the runtime,
/// before the tree that holds the bar paints.
pub(crate) fn set_value_mode(mode: ValueMode) {
    storage::set(mode as u8);
}

/// What the focused control is doing, as the chrome paints the bar.
pub(crate) fn value_mode() -> ValueMode {
    match storage::get() {
        1 => ValueMode::Openable,
        2 => ValueMode::Open,
        _ => ValueMode::None,
    }
}

/// A plain static on a device, which has one thread and one panel.
///
/// Load and store only, never a read-modify-write: Cortex-M0+ has no atomic
/// compare-and-swap, so `swap` and `fetch_or` do not compile for `thumbv6m`.
#[cfg(not(all(any(test, feature = "testing"), not(target_os = "none"))))]
mod storage {
    use core::sync::atomic::{AtomicU8, Ordering};

    static VALUE_MODE: AtomicU8 = AtomicU8::new(0);

    pub(super) fn set(mode: u8) {
        VALUE_MODE.store(mode, Ordering::Relaxed);
    }

    pub(super) fn get() -> u8 {
        VALUE_MODE.load(Ordering::Relaxed)
    }
}

/// **Thread-local under test**, because a test binary runs its cases in
/// parallel threads that share every static.
///
/// A mode one case set while another case painted is a failure that passes on
/// its own and fails in a suite, which is the hardest kind to read: it looks
/// like the frame is wrong rather than like the harness is.
///
/// Nothing about the device path changes: the `not(target_os = "none")` keeps
/// this off bare metal by construction rather than by nobody having enabled the
/// feature there, and `thread_local!` is `std` in any case.
#[cfg(all(any(test, feature = "testing"), not(target_os = "none")))]
mod storage {
    use core::cell::Cell;

    thread_local! {
        static VALUE_MODE: Cell<u8> = const { Cell::new(0) };
    }

    pub(super) fn set(mode: u8) {
        VALUE_MODE.with(|mode_| mode_.set(mode));
    }

    pub(super) fn get() -> u8 {
        VALUE_MODE.with(Cell::get)
    }
}
