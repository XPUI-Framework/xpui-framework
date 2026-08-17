//! What the UI needs from whatever hosts it.
//!
//! `xpui` never talks to a backend directly. It declares this contract and a
//! host installs an implementation once at startup, so the framework can be
//! built, tested and reasoned about without a device — and so nothing in the
//! UI can reach for a backend by accident.
//!
//! There are two installable things, because they answer to different owners:
//!
//! ```rust
//! # use xpui::testing::TestHost;
//! # static BACKEND: TestHost = TestHost;
//! # static SHELL: TestHost = TestHost;
//! // Safety: once, before the first frame, and never concurrently with one.
//! unsafe {
//!     xpui::host::install(&BACKEND);          // what paints
//!     xpui::host::install_navigator(&SHELL);  // what owns the screen stack
//! }
//! ```
//!
//! A backend crate supplies the first and knows nothing about the second.

mod canvas;
pub(crate) mod chrome;
mod clock;
mod input;
mod metrics;
mod navigator;

pub use canvas::{Canvas, IconRef, Renderer};
pub use chrome::{Chrome, Hint, RowField, ScreenChrome, Theme, ThemeMetric, request_update};
pub use clock::{Clock, millis};
pub use input::{Button, Input, InputSource, SwipeDir};
pub use metrics::{Font, FontId, FontRole, FontStyle, TextMetrics};
pub use navigator::{Navigator, finish_screen, present};

/// Everything a backend must provide. One object implements all five, so a
/// host installs a single value and the framework keeps one pointer.
pub trait Host: Canvas + TextMetrics + Chrome + InputSource + Clock + Sync {}

impl<T> Host for T where T: Canvas + TextMetrics + Chrome + InputSource + Clock + Sync {}

/// The installed host.
///
/// Written once before the first frame and only read afterwards. A plain static
/// rather than a lock: the two callers are separate FreeRTOS tasks, but neither
/// writes, and taking a lock on every text measurement would cost more than the
/// whole layout pass.
static mut HOST: Option<&'static dyn Host> = None;

/// Installs the host. Call once, before any view is measured or drawn.
///
/// # Safety
/// Must be called before the first `measure`/`render`/`interactions`, and never
/// concurrently with them. In practice that means from the activity's entry
/// point, on the main task, before the render task is started.
pub unsafe fn install(host: &'static dyn Host) {
    unsafe {
        HOST = Some(host);
    }
}

/// The installed host, for the framework's own use.
///
/// # Panics
/// If nothing was installed. That is a wiring mistake, not a runtime condition:
/// a screen cannot be measured before its host exists.
pub(crate) fn current() -> &'static dyn Host {
    // Safety: written once by `install` before any reader exists.
    if let Some(host) = unsafe { HOST } {
        return host;
    }

    // In a test build, fall back to the fake rather than making every test
    // remember to install before it constructs its first widget. Widgets
    // resolve fonts in their constructors, so the ordering trap is easy to hit
    // and the failure looks nothing like its cause.
    #[cfg(any(test, feature = "testing"))]
    {
        crate::testing::install();
        if let Some(host) = unsafe { HOST } {
            return host;
        }
    }

    panic!("xpui::host::install was never called")
}

/// Whether a host has been installed, so tests can assert wiring.
pub fn is_installed() -> bool {
    unsafe { HOST }.is_some()
}

/// The installed navigator. Separate from [`HOST`] because the two have
/// different owners: a backend crate supplies the host, the application
/// supplies this.
static mut NAVIGATOR: Option<&'static dyn Navigator> = None;

/// Installs the navigator. Call once, before the first frame.
///
/// # Safety
/// Same contract as [`install`]: before any frame runs, and never concurrently
/// with one. A host with a separate render task must install from both entry
/// points, since either may wake first.
pub unsafe fn install_navigator(navigator: &'static dyn Navigator) {
    unsafe {
        NAVIGATOR = Some(navigator);
    }
}

/// The installed navigator, for the framework's own use.
///
/// Falls back to a no-op rather than panicking: a single-screen host has
/// nowhere to go back to and should not have to say so. The no-op trips a
/// `debug_assert` if a screen ever actually asks it to navigate.
pub(crate) fn navigator() -> &'static dyn Navigator {
    // Safety: written once by `install_navigator` before any reader exists.
    static NONE: navigator::NoNavigator = navigator::NoNavigator;
    unsafe { NAVIGATOR }.unwrap_or(&NONE)
}

/// Whether a navigator has been installed, so a host can assert its wiring
/// and stay idempotent across two entry points.
pub fn is_navigator_installed() -> bool {
    unsafe { NAVIGATOR }.is_some()
}
