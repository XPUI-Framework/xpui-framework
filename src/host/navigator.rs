//! Who owns the screen stack.
//!
//! Two of the things a screen needs are not drawing at all: what this screen
//! is called, and "I am done, go back". They depend on whoever owns the
//! navigation, which is a different question from what paints the pixels:
//!
//! | Owner | `Navigator` is |
//! |---|---|
//! | a C++ firmware with its own activity stack | a call across the FFI |
//! | [`App`](crate::App) | the stack it holds |
//!
//! Keeping them off [`Chrome`](super::Chrome) is what lets a backend crate
//! ship a ready-made drawing implementation without also having an opinion
//! about navigation. `request_update` deliberately stayed on `Chrome`: a
//! repaint is a *display* concern, every backend that can paint can ask for
//! one, and the framework calls it on every dispatch — a host that forgot to
//! install a navigator should not end up with a screen that never refreshes.
//!
//! The name is `Navigator` rather than `Shell` on purpose. In UI vocabulary
//! "shell" and "chrome" mean the same thing — the frame around the content —
//! so a reader would have no way to guess which trait a method lived on.

use alloc::boxed::Box;

use crate::screen::Driver;

/// The navigation a screen sits inside.
pub trait Navigator: Sync {
    /// This screen's own title, already translated.
    ///
    /// `'static` is explicit rather than elided, and that is the whole point:
    /// a navigator that owned the screen stack and returned a title borrowed
    /// out of the top screen would hand back a dangling reference the moment
    /// [`finish`](Navigator::finish) dropped it. Requiring `'static` makes
    /// that unwriteable rather than merely discouraged. Both real
    /// implementations satisfy it easily — a compiled-in translation table on
    /// one side, a `&'static str` the app stored on the other.
    fn screen_title(&self) -> &'static str;

    /// Pops this screen.
    ///
    /// Called from inside a screen's own frame, so an implementation that owns
    /// the stack must **record** the request and act on it once the frame is
    /// over. Popping here would free the screen currently running.
    fn finish(&self);

    /// Pushes a screen on top of this one.
    ///
    /// Returns `None` when the navigator took it, and `Some(screen)` handing
    /// it back when it cannot. Returning it rather than dropping it means a
    /// caller can tell the difference; silently swallowing the screen would
    /// look identical to working.
    ///
    /// A navigator whose stack lives outside Rust **can** take one: the screen
    /// is a trait object, so it crosses as an opaque handle that the host only
    /// ever hands back. What a refusal means is that this particular navigator
    /// has nowhere to put it — a single-screen host, or a stack that already
    /// has a push queued for this frame.
    ///
    /// Same re-entrancy rule as [`finish`](Navigator::finish): record, then
    /// act after the frame.
    fn present(&self, screen: Box<dyn Driver>) -> Option<Box<dyn Driver>>;
}

/// What a host gets when it installs no navigator.
///
/// Not automatically an error: a backend with exactly one screen and nowhere
/// to go back to needs none of this. It does mean Back does nothing, so the
/// debug build says so rather than leaving you holding a device with a dead
/// button and no clue why.
pub(super) struct NoNavigator;

impl Navigator for NoNavigator {
    fn screen_title(&self) -> &'static str {
        ""
    }

    fn finish(&self) {
        debug_assert!(
            false,
            "a screen asked to be finished, but no Navigator is installed — \
             call xpui::host::install_navigator, or xpui::App::new which does it for you"
        );
    }

    fn present(&self, screen: Box<dyn Driver>) -> Option<Box<dyn Driver>> {
        debug_assert!(
            false,
            "a screen asked to present another, but no Navigator is installed"
        );
        Some(screen)
    }
}

/// Pops the current screen.
pub fn finish_screen() {
    super::navigator().finish()
}

/// Pushes `screen` on top of the current one.
///
/// Returns `false` when the host's navigation is not the framework's to drive
/// — see [`Navigator::present`]. The screen is dropped in that case, having
/// gone nowhere.
pub fn present<S: crate::screen::Screen + 'static>(screen: S) -> bool {
    super::navigator()
        .present(Box::new(crate::screen::Runtime::new(screen)))
        .is_none()
}
