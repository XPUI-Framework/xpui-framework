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
//! // Safety: before the first frame, and never concurrently with one.
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
mod value_mode;

pub use canvas::{Canvas, IconRef, Renderer};
pub use chrome::{
    Chrome, ControlState, Hint, HintWord, RowField, ScreenChrome, Theme, ThemeMetric,
    request_update,
};
pub use clock::{Clock, millis};
pub use input::{Button, Input, InputSource, KeyRow, RowKey, SwipeDir};
pub use metrics::{Font, FontId, FontRole, FontStyle, TextMetrics};
pub use navigator::{Navigator, finish_screen, present};
pub(crate) use value_mode::{ValueMode, set_value_mode, value_mode};

/// Everything a backend must provide, as one object implementing all five traits.
///
/// A host installs a single value and the framework keeps one pointer. Nothing
/// implements it by hand: any `Sync` type that implements the five is a `Host`.
pub trait Host: Canvas + TextMetrics + Chrome + InputSource + Clock + Sync {}

impl<T> Host for T where T: Canvas + TextMetrics + Chrome + InputSource + Clock + Sync {}

/// The installed host.
///
/// Written between frames and only read inside them. A plain static rather
/// than a lock: the readers may be separate tasks, none of them writes, and
/// taking a lock on every text measurement would cost more than the whole
/// layout pass.
static mut HOST: Option<&'static dyn Host> = None;

/// Installs the host, before any view is measured or drawn.
///
/// Installing again replaces it — another panel size is another backend —
/// and the old `&'static` stays valid for anything that read it.
///
/// **The host is process-wide.** A test that installs a second corrupts what
/// the first was serving, which shows up as flakiness; every test here takes
/// the same mutex first, and `testing::Ui` holds it.
///
/// # Safety
/// One thread, and no frame in flight — no `measure`, `render` or
/// `interactions` running on any task. The write is unsynchronised, so
/// overlapping it with a read is a data race: undefined behaviour, not a
/// stale pointer you could notice.
pub unsafe fn install(host: &'static dyn Host) {
    // Safety: the caller's, as documented above.
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
        // Safety: as above; `testing::install` has just written it.
        if let Some(host) = unsafe { HOST } {
            return host;
        }
    }

    panic!("xpui::host::install was never called")
}

/// Whether a host has been installed, so tests can assert wiring.
pub fn is_installed() -> bool {
    // Safety: a read of a static that is written once, before any reader.
    unsafe { HOST }.is_some()
}

/// The installed navigator. Separate from [`HOST`] because the two have
/// different owners: a backend crate supplies the host, the application
/// supplies this.
static mut NAVIGATOR: Option<&'static dyn Navigator> = None;

/// Installs the navigator, before the first frame.
///
/// # Safety
/// Same contract as [`install`]: before any frame runs, and never concurrently
/// with one. A host with a separate render task must install from both entry
/// points, since either may wake first.
pub unsafe fn install_navigator(navigator: &'static dyn Navigator) {
    // Safety: the caller's, as documented above.
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
    static NONE: navigator::NoNavigator = navigator::NoNavigator;
    // Safety: written once by `install_navigator` before any reader exists.
    unsafe { NAVIGATOR }.unwrap_or(&NONE)
}

/// Whether a navigator has been installed, so a host can assert its wiring
/// and stay idempotent across two entry points.
pub fn is_navigator_installed() -> bool {
    // Safety: a read of a static that is written once, before any reader.
    unsafe { NAVIGATOR }.is_some()
}
