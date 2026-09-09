//! A screen with its type erased.
//!
//! [`Screen`](super::Screen) returns `impl View`, so it is deliberately never a
//! trait object. `Runtime<S>` erases the screen type behind this instead, which
//! is what a host's lifecycle entry points dispatch through.

use super::{Runtime, Screen};

/// A screen with its type erased, so a host can drive one without knowing
/// which `Screen` it is. The lifecycle entry points live in the host crate.
pub trait Driver {
    fn on_enter(&mut self);
    fn loop_(&mut self);
    fn on_exit(&mut self);
    fn render(&mut self);
    fn handle_home_gesture(&mut self) -> bool;

    /// This screen's title, for a host that shows one.
    ///
    /// A host owning a stack of these has no other way to ask: `Screen` is
    /// never a trait object, so the title has to come through here.
    fn title(&self) -> Option<&'static str>;

    /// Whether this screen paints over what is already on the panel.
    ///
    /// A host stacking screens needs this to know it must repaint whatever
    /// sits underneath before drawing this one — an overlay deliberately does
    /// not clear, so without the screen beneath it there is nothing to overlay.
    fn is_overlay(&self) -> bool;
}

impl<S: Screen> Driver for Runtime<S> {
    fn on_enter(&mut self) {
        self.screen.on_enter();
    }

    fn loop_(&mut self) {
        Runtime::loop_(self);
    }

    fn on_exit(&mut self) {
        self.screen.on_exit();
    }

    fn render(&mut self) {
        Runtime::render(self);
    }

    fn handle_home_gesture(&mut self) -> bool {
        self.screen.handle_home_gesture()
    }

    fn title(&self) -> Option<&'static str> {
        self.screen.title()
    }

    fn is_overlay(&self) -> bool {
        self.screen.is_overlay()
    }
}
