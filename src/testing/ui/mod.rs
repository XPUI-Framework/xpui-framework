//! Driving a screen the way a person does.
//!
//! ```rust
//! # use xpui::host::Host;
//! # use xpui::testing::{Drive, Ui};
//! # use xpui::{NavigationScreen, Screen, Text, View, vstack};
//! # struct Menu;
//! # impl Menu { fn new() -> Menu { Menu } }
//! # impl Screen for Menu {
//! #     type Message = ();
//! #     fn body(&self) -> impl View<()> { NavigationScreen::new(vstack![0; Text::new("Lists")]) }
//! #     fn update(&mut self, _message: ()) {}
//! # }
//! /// `backend` is whatever host the test drives — see [`Drive`].
//! fn opens_the_list_screen<H: Host + Drive + 'static>(backend: &'static H) {
//!     let mut ui = Ui::new(Menu::new(), backend);
//!     ui.tap_text("Lists");
//!     assert!(ui.visible_text().iter().any(|line| line == "Rows, subtitles, values"));
//! }
//! ```
//!
//! Everything goes through [`App`] and the installed host: a harness that
//! drove the runtime directly reports the arrow keys working when nothing
//! reaches the panel, because the focus index moves either way.

mod inspect;

use alloc::vec::Vec;
use std::sync::{Mutex, MutexGuard};

use crate::app::App;
use crate::geometry::Point;
use crate::host::{Button, Host, SwipeDir};
use crate::screen::Screen;
use crate::testing::ops::DrawOp;
use crate::testing::{Recorder, ops_log, reset};

/// A host that a test can feed input to.
///
/// [`InputSource`](crate::host::InputSource) only reads; something has to write.
/// A backend already has these — this names them so the harness can reach them
/// without knowing which backend it is driving.
pub trait Drive {
    /// Starts a frame, clearing the previous frame's edges. Without this a
    /// press stays "just pressed" forever and every frame acts on it again.
    fn begin(&self, millis: u32);
    /// Reports `button` as pressed this frame.
    fn inject_press(&self, button: Button);
    /// Reports `button` as released this frame.
    fn inject_release(&self, button: Button);
    /// Reports a completed tap at `point`.
    fn inject_tap(&self, point: Point);
    /// Reports a completed swipe.
    fn inject_swipe(&self, direction: SwipeDir);
}

/// Held for as long as a `Ui` exists, because it installs the process-wide
/// host. Two at once would race on the backend's state, which is a confusing
/// way to learn about a rule, so the type enforces it.
static SERIAL: Mutex<()> = Mutex::new(());

/// A screen under test, with the app and host that drive it.
pub struct Ui<H: Host + Drive + 'static> {
    app: App,
    host: &'static H,
    millis: u32,
    frame: Vec<DrawOp>,
    previous: Vec<DrawOp>,
    /// Whether the *last* action repainted, so an action that drew nothing
    /// does not report the previous action's answer.
    repainted: bool,
    /// Dropped last, releasing the lock when the test ends.
    _guard: MutexGuard<'static, ()>,
}

impl<H: Host + Drive + 'static> Ui<H> {
    /// Installs `host` behind a recorder, starts `screen`, and paints once.
    ///
    /// The first paint is not optional: the runtime ignores taps until it has
    /// painted, so without it the first tap of every test is swallowed and the
    /// screen looks broken for a reason that is not its fault.
    ///
    /// Only one `Ui` runs at a time. The rest of the test harness blocks here
    /// rather than racing, so tests in one file need no lock of their own.
    pub fn new<S: Screen + 'static>(screen: S, host: &'static H) -> Ui<H> {
        // Waits, but not forever. Tests in one binary run on several threads,
        // so a `Ui` legitimately queues behind another; two alive in the *same*
        // test is a mistake, and blocking on it would hang with no message —
        // much harder to diagnose than a panic that says what you did.
        //
        // A panicking test poisons the lock. Recover rather than cascading: the
        // next `Ui` installs its own host and resets the log, so there is no
        // state left to inherit.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let guard = loop {
            match SERIAL.try_lock() {
                Ok(guard) => break guard,
                Err(std::sync::TryLockError::Poisoned(poisoned)) => break poisoned.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "waited a minute for another `Ui` to finish. Only one at \
                         a time: it installs the process-wide host, so two would \
                         draw through each other. If both are in one test, drop \
                         the first before building the second."
                    );
                    std::thread::yield_now();
                }
            }
        };

        let recorder = Recorder::wrap(host);
        // Safety: `guard` is held for this `Ui`'s whole life, so nothing else
        // is drawing through the installed host while this one replaces it.
        unsafe { crate::host::install(recorder) };

        reset();
        host.begin(0);
        let mut app = App::new(screen);
        app.render();

        Ui {
            app,
            host,
            millis: 0,
            frame: ops_log(),
            previous: Vec::new(),
            repainted: true,
            _guard: guard,
        }
    }

    // -- acting ------------------------------------------------------------

    /// Taps the middle of whatever control shows `label`.
    ///
    /// # Panics
    /// If nothing shows that text, listing what is on screen. A silent miss
    /// leaves a test passing because nothing happened.
    pub fn tap_text(&mut self, label: &str) -> &mut Self {
        let found = self.rects_of_text(label);
        let rect = match found.as_slice() {
            [only] => *only,
            [] => panic!(
                "nothing on screen shows {label:?}.\nVisible text: {:#?}",
                self.visible_text()
            ),
            many => panic!(
                "{label:?} is on screen {} times, at {many:?}. Say which one \
                 with `tap_nth_text`, because picking for you would make this \
                 test pass or fail depending on draw order.",
                many.len()
            ),
        };
        self.tap_at(Point::new(
            rect.x() + rect.width() / 2,
            rect.y() + rect.height() / 2,
        ))
    }

    /// Taps the `index`th place `label` appears, in draw order.
    ///
    /// # Panics
    /// If there are fewer than `index + 1` of them.
    pub fn tap_nth_text(&mut self, label: &str, index: usize) -> &mut Self {
        let found = self.rects_of_text(label);
        let rect = *found.get(index).unwrap_or_else(|| {
            panic!(
                "{label:?} appears {} times; asked for number {index}",
                found.len()
            )
        });
        self.tap_at(Point::new(
            rect.x() + rect.width() / 2,
            rect.y() + rect.height() / 2,
        ))
    }

    /// Taps a panel coordinate.
    pub fn tap_at(&mut self, point: Point) -> &mut Self {
        self.advance(|host| host.inject_tap(point))
    }

    /// Presses and releases `button` in one frame.
    pub fn press(&mut self, button: Button) -> &mut Self {
        self.advance(|host| {
            host.inject_press(button);
            host.inject_release(button);
        })
    }

    /// Swipes in `direction`.
    pub fn swipe(&mut self, direction: SwipeDir) -> &mut Self {
        self.advance(|host| host.inject_swipe(direction))
    }

    /// One frame: deliver the input, let the screen react, repaint if asked.
    fn advance(&mut self, feed: impl FnOnce(&H)) -> &mut Self {
        self.millis += 16;
        self.host.begin(self.millis);
        feed(self.host);

        reset();
        self.app.tick();
        self.repainted = self.app.render_if_dirty();
        if self.repainted {
            self.previous = core::mem::take(&mut self.frame);
            self.frame = ops_log();
        }
        self
    }
}
