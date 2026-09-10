//! Screens: the contract, the runtime that drives one, and the root views
//! that own a whole screen's chrome.

mod navigation;
mod overlay;

pub use navigation::NavigationScreen;
pub use overlay::OverlayPanel;

use crate::geometry::Point;
use crate::host::{Button, SwipeDir};
use crate::view::View;

mod driver;
mod routing;
mod runtime;

pub use driver::Driver;
pub use runtime::Runtime;

/// A screen.
///
/// ```rust
/// # use xpui::{Screen, Slider, Toggle, View, vstack};
/// # struct Panel { on: bool, brightness: i32 }
/// #[derive(Clone, Copy)]
/// enum Msg { Toggle(bool), Brightness(i32) }
///
/// impl Screen for Panel {
///     type Message = Msg;
///
///     fn body(&self) -> impl View<Msg> {
///         vstack![12;
///             Toggle::new("Frontlight", self.on, "On", "Off").on_change(Msg::Toggle),
///             Slider::new(self.brightness, 100).on_change(Msg::Brightness),
///         ]
///     }
///
///     fn update(&mut self, message: Msg) {
///         match message {
///             // The state being moved to, so this is never `!self.on`.
///             Msg::Toggle(next) => self.on = next,
///             Msg::Brightness(v) => self.brightness = v,
///         }
///     }
/// }
/// ```
pub trait Screen {
    /// What this screen's controls send back. One enum per screen, matched
    /// exhaustively in [`update`](Screen::update).
    type Message: Clone;

    /// Describes the screen. A pure function of `self` — no device writes.
    ///
    /// Called once per paint and once per frame that carries input. Built
    /// fresh rather than stored because `loop_` and `render` may run on
    /// different tasks with no lock between them: a stored tree is one task
    /// walking what the other is replacing.
    fn body(&self) -> impl View<Self::Message>;

    /// Applies a message. The only place state changes; the runtime repaints
    /// afterwards, so no screen calls `request_update` itself.
    fn update(&mut self, message: Self::Message);

    /// A key the runtime has not claimed, offered before it applies its own
    /// meaning. Return a message to consume it.
    ///
    /// Consulted **first**, so a screen that wants Up/Down for something other
    /// than moving focus simply says so. Auto-repeat applies to whatever is
    /// claimed here.
    fn on_key(&self, key: Button) -> Option<Self::Message> {
        let _ = key;
        None
    }

    /// A swipe, offered before the runtime gives it its own meaning.
    /// Return a message to consume it.
    ///
    /// Consulted **first**, so a screen that pages on a swipe — a reader, say —
    /// simply claims it and the runtime does not move focus.
    fn on_swipe(&self, _direction: SwipeDir) -> Option<Self::Message> {
        None
    }

    /// A touch that no control claimed. Return a message to consume it.
    ///
    /// An overlay uses this to close when the scrim is tapped.
    fn on_background_tap(&self, point: Point) -> Option<Self::Message> {
        let _ = point;
        None
    }

    /// Whether this screen paints over what is already on the panel, leaving
    /// it visible beneath.
    fn is_overlay(&self) -> bool {
        false
    }

    /// This screen's title, for a host that keeps a stack of them.
    ///
    /// `'static` because a host may hold it past this frame. A screen whose
    /// title is computed at run time — a file name — passes it to
    /// [`NavigationScreen::title`] inside `body()` instead, where no such
    /// constraint applies.
    ///
    /// Leaving it `None` while using a [`NavigationScreen`] with no explicit
    /// title of its own draws an **empty header band**, because that is
    /// literally what the screen asked for. Either give one here, or pass one
    /// to `NavigationScreen::title`. An [`OverlayPanel`] is the case where
    /// `None` is the right answer.
    fn title(&self) -> Option<&'static str> {
        None
    }

    /// A frame happened.
    ///
    /// Called once per frame, before any input is considered, and on frames
    /// where nothing arrived at all — which is the point: a countdown, an
    /// auto-refresh, an action held back for a second press have nowhere else
    /// to notice a deadline passed. No argument on purpose; a screen that
    /// wants the time asks the clock. Ask for a repaint if something changed.
    fn tick(&mut self) {}

    /// The screen was pushed, before its first frame. A screen uncovered by
    /// a pop is not told.
    fn on_enter(&mut self) {}

    /// The screen is being popped. A screen covered by a push is not told.
    fn on_exit(&mut self) {}

    /// The system home gesture. Return `true` to consume it; an overlay does,
    /// so the gesture dismisses the overlay rather than the screen below.
    fn handle_home_gesture(&mut self) -> bool {
        false
    }
}
