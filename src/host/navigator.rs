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
//! Keeping them off [`Chrome`](super::Chrome) lets a backend crate ship a
//! drawing implementation without an opinion about navigation. Named
//! `Navigator` rather than `Shell`: in UI vocabulary a shell *is* the chrome.

use alloc::boxed::Box;

use crate::screen::{Driver, Screen};

/// The navigation a screen sits inside.
pub trait Navigator: Sync {
    /// This screen's own title, already translated.
    ///
    /// `'static` on purpose: a navigator that owns the stack and returned a
    /// title borrowed out of the top screen would dangle the moment
    /// [`finish`](Navigator::finish) dropped it. A translation table on one
    /// side and a stored `&'static str` on the other satisfy it.
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
    /// it back when it cannot, so a caller can tell the difference. A stack
    /// outside Rust can take one — the screen crosses as an opaque handle the
    /// host only hands back; a refusal means nowhere to put it: a
    /// single-screen host, or a push already queued for this frame.
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
pub fn present<S: Screen + 'static>(screen: S) -> bool {
    super::navigator()
        .present(Box::new(crate::screen::Runtime::new(screen)))
        .is_none()
}
