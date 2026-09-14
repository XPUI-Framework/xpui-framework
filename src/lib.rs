//! A declarative UI framework for e-ink screens.
//!
//! Screens are described as a tree of [`View`]s and painted through whatever
//! host the application installs. The API is deliberately SwiftUI-shaped:
//!
//! ```rust
//! # use xpui::{NavigationScreen, Spacer, Text, vstack};
//! # xpui::testing::install();
//! # let version = "1.4.2";
//! # let _: NavigationScreen<()> =
//! NavigationScreen::new(vstack![20;
//!     Text::new("Firmware"),
//!     Text::new(version).bold(),
//!     Spacer::new(),
//! ])
//! # ;
//! ```
//!
//! See [the tutorial](https://github.com/XPUI-Framework/xpui-framework/blob/main/docs/tutorial.md)
//! for a walkthrough, and
//! [the architecture](https://github.com/XPUI-Framework/xpui-framework/blob/main/docs/architecture.md)
//! for how a frame runs.
//!
//! # Layers
//!
//! - [`host`] — the traits a backend implements, and the façades widgets call.
//! - [`geometry`] — [`Point`], [`Size`], [`Rect`], [`Insets`].
//! - [`view`] — the [`View`] trait and the interaction model.
//! - [`layout`] — containers that position children: stacks, spacer, modifiers.
//! - [`widgets`] — leaves that draw: text, lists, sliders, icons.
//! - [`screen`] — the [`Screen`] contract, its runtime, and the root views.
//!
//! `unsafe` is confined to three places and stays there: the installed-host
//! globals in [`host`], the single-threaded cells in [`app::AppShell`], and the
//! `testing` module — its doubles, and the counting allocator one of its own
//! tests installs.
//!
//! # Portability
//!
//! Builds `no_std` for bare-metal targets (using `alloc` against whatever heap
//! the application provides) and against `std` on a desktop so the simulator
//! and `cargo test` work unchanged. Nothing in this crate may reference a
//! product, a device or a drawing library — those belong in a backend, and
//! screens belong in the application.

#![cfg_attr(target_os = "none", no_std)]
#![deny(missing_docs)]

extern crate alloc;

pub mod app;
pub mod geometry;
pub mod host;
pub mod layout;
pub mod screen;
pub mod view;
pub mod widgets;

#[cfg(any(test, feature = "testing"))]
pub mod testing;

/// The guides, compiled: every ```rust block in these files is a doctest, so
/// a snippet that stops matching the API fails `cargo test`. `cfg(doctest)`
/// keeps the markdown out of a real build.
#[cfg(doctest)]
mod guides {
    #[doc = include_str!("../docs/reference.md")]
    pub mod reference {}
    #[doc = include_str!("../docs/reference/lists.md")]
    pub mod reference_lists {}
    #[doc = include_str!("../docs/reference/navigation.md")]
    pub mod reference_navigation {}
    #[doc = include_str!("../docs/reference/screens.md")]
    pub mod reference_screens {}
    #[doc = include_str!("../docs/reference/views.md")]
    pub mod reference_views {}
    #[doc = include_str!("../docs/reference/layout.md")]
    pub mod reference_layout {}
    #[doc = include_str!("../docs/reference/modifiers.md")]
    pub mod reference_modifiers {}
    #[doc = include_str!("../docs/reference/text-and-images.md")]
    pub mod reference_text_and_images {}
    #[doc = include_str!("../docs/reference/controls.md")]
    pub mod reference_controls {}
    #[doc = include_str!("../docs/reference/dialogs.md")]
    pub mod reference_dialogs {}
    #[doc = include_str!("../docs/reference/indicators.md")]
    pub mod reference_indicators {}
    #[doc = include_str!("../docs/reference/input.md")]
    pub mod reference_input {}
    #[doc = include_str!("../docs/reference/theme.md")]
    pub mod reference_theme {}
    #[doc = include_str!("../docs/reference/geometry.md")]
    pub mod reference_geometry {}
    #[doc = include_str!("../docs/reference/backend-contract.md")]
    pub mod reference_backend_contract {}
    #[doc = include_str!("../docs/reference/testing.md")]
    pub mod reference_testing {}
    #[doc = include_str!("../docs/reference/toggles.md")]
    pub mod reference_toggles {}
    #[doc = include_str!("../docs/reference/app.md")]
    pub mod reference_app {}
    #[doc = include_str!("../docs/reference/interactions.md")]
    pub mod reference_interactions {}
    #[doc = include_str!("../docs/reference/canvas-and-chrome.md")]
    pub mod reference_canvas_and_chrome {}
    #[doc = include_str!("../docs/reference/recording.md")]
    pub mod reference_recording {}
    #[doc = include_str!("../docs/architecture.md")]
    pub mod architecture {}
    #[doc = include_str!("../docs/design.md")]
    pub mod design {}
    #[doc = include_str!("../docs/host.md")]
    pub mod host {}
    #[doc = include_str!("../docs/writing-a-backend.md")]
    pub mod writing_a_backend {}
    #[doc = include_str!("../docs/writing-a-widget.md")]
    pub mod writing_a_widget {}
    #[doc = include_str!("../docs/testing.md")]
    pub mod testing_guide {}
    #[doc = include_str!("../README.md")]
    pub mod readme {}
    #[doc = include_str!("../docs/tutorial.md")]
    pub mod tutorial {}
    #[doc = include_str!("../docs/a-second-screen.md")]
    pub mod a_second_screen {}
}

pub use app::App;
pub use geometry::{Insets, Point, Rect, Size};
pub use host::{
    Button, ControlState, Font, FontId, FontRole, FontStyle, Hint, HintWord, IconRef, Input,
    KeyRow, Navigator, Renderer, RowKey, ScreenChrome, SwipeDir, Theme, ThemeMetric, finish_screen,
    millis, present, request_update,
};
pub use layout::{
    Alignment, Flexible, Frame, HStack, Modifiers, Padding, ScrollView, Spacer, Tappable,
    UNBOUNDED, VStack,
};
pub use screen::{NavigationScreen, OverlayPanel, Screen};
pub use view::{InputMask, Interaction, Interactions, Scrim, Trigger, View, ViewExt, value_at};
pub use widgets::{
    Divider, Icon, IconToggle, Image, List, ListRow, Modal, ProgressBar, Section, Slider, Stepper,
    Text, Toggle,
};
