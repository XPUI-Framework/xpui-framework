//! A screen stack the framework owns, for a host that has none of its own.
//!
//! A C++ firmware with an activity stack drives one [`Driver`] at a time and
//! needs nothing here. Everything else — a desktop simulator, a bare-metal
//! Rust binary, the example gallery — has no such owner, and this is it.
//!
//! ```rust,ignore
//! let mut app = App::new(MainMenu::new());
//! while app.is_running() {
//!     app.tick();          // input, then any navigation it asked for
//!     app.render_if_dirty();
//! }
//! ```
//!
//! # Why the state is split in two
//!
//! [`Navigator`] is installed as a `&'static`, and `App` needs `&mut self` to
//! pump frames. One object cannot be both: leaking the `App` to get the
//! `&'static` leaves a shared borrow alive for the rest of the program, and
//! there is no `&mut` left to tick with. So the parts the navigator writes
//! live in a separate, leaked [`AppShell`], and `App` drains it between
//! frames.
//!
//! That split is also what makes the re-entrancy safe. `finish()` is called
//! from inside `Runtime::loop_`, while `App` holds `&mut` on the top of the
//! stack. Popping there would free the screen currently running. Recording the
//! request in a different allocation and acting on it after the frame is what
//! keeps the two apart.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::host::{self, Navigator};
use crate::screen::{Driver, Runtime, Screen};

/// What the navigator writes and [`App`] reads, in its own allocation.
pub struct AppShell {
    finish: AtomicBool,
    /// Only ever `store`d and `load`ed, never swapped: RISC-V without the `A`
    /// extension — the ESP32-C3 — has atomic loads and stores but no
    /// read-modify-write, so `swap` does not compile for that target.
    pending: UnsafeCell<Option<Box<dyn Driver>>>,
    title: UnsafeCell<&'static str>,
}

// Safety: `App` pumps frames from one thread, and the navigator is only ever
// touched from inside a frame — so the `UnsafeCell` fields have exactly one
// accessor at a time. A host that renders on a second task must not use `App`;
// it should implement `Navigator` itself over whatever synchronisation it
// already has, which is what the C++ firmware does.
unsafe impl Sync for AppShell {}

impl AppShell {
    fn new() -> Self {
        AppShell {
            finish: AtomicBool::new(false),
            pending: UnsafeCell::new(None),
            title: UnsafeCell::new(""),
        }
    }

    /// Reads and clears the finish request.
    fn take_finish(&self) -> bool {
        if self.finish.load(Ordering::Relaxed) {
            self.finish.store(false, Ordering::Relaxed);
            return true;
        }
        false
    }

    /// Takes the queued screen out before any of it runs.
    ///
    /// Moved to a local deliberately: `on_enter` on the new screen may call
    /// `present` again, and finding the slot still occupied would either lose
    /// that screen or re-enter a borrow.
    fn take_pending(&self) -> Option<Box<dyn Driver>> {
        // Safety: see the `Sync` impl — one accessor at a time.
        unsafe { (*self.pending.get()).take() }
    }

    fn set_title(&self, title: &'static str) {
        // Safety: see the `Sync` impl.
        unsafe { *self.title.get() = title }
    }
}

impl Navigator for AppShell {
    fn screen_title(&self) -> &'static str {
        // Safety: see the `Sync` impl. The value is a `&'static str` the app
        // stored, never a borrow out of the stack, so popping cannot dangle it.
        unsafe { *self.title.get() }
    }

    fn finish(&self) {
        self.finish.store(true, Ordering::Relaxed);
    }

    fn present(&self, screen: Box<dyn Driver>) -> Option<Box<dyn Driver>> {
        // Safety: see the `Sync` impl.
        let slot = unsafe { &mut *self.pending.get() };
        match slot {
            // One push per frame. Handing the second back rather than
            // overwriting means a screen that pushed twice finds out.
            Some(_) => Some(screen),
            None => {
                *slot = Some(screen);
                None
            }
        }
    }
}

/// A stack of screens, and the frame loop that drives the top one.
pub struct App {
    stack: Vec<Box<dyn Driver>>,
    shell: &'static AppShell,
    dirty: bool,
}

impl App {
    /// Starts an app showing `root`, and installs the navigator behind it.
    ///
    /// The shell is leaked on purpose: it must outlive every screen that could
    /// call into it, it is one small allocation, and it lives as long as the
    /// program does anyway.
    pub fn new<S: Screen + 'static>(root: S) -> Self {
        let shell: &'static AppShell = Box::leak(Box::new(AppShell::new()));
        // Safety: nothing has rendered yet, and `App` is single-threaded.
        unsafe { host::install_navigator(shell) };

        let mut app = App {
            stack: Vec::new(),
            shell,
            dirty: true,
        };
        app.push(root);
        app
    }

    /// Pushes a screen and shows it.
    pub fn push<S: Screen + 'static>(&mut self, screen: S) {
        self.push_driver(Box::new(Runtime::new(screen)));
    }

    fn push_driver(&mut self, mut screen: Box<dyn Driver>) {
        self.shell.set_title(screen.title().unwrap_or(""));
        screen.on_enter();
        self.stack.push(screen);
        self.dirty = true;
    }

    /// Pops the top screen, returning whether anything was there.
    ///
    /// The popped screen is returned to the caller rather than dropped here:
    /// its `Drop` may run screen code that calls back into the navigator, and
    /// that must not happen while the stack is mid-edit.
    fn pop(&mut self) -> Option<Box<dyn Driver>> {
        let mut screen = self.stack.pop()?;
        screen.on_exit();
        self.shell
            .set_title(self.stack.last().and_then(|top| top.title()).unwrap_or(""));
        self.dirty = true;
        Some(screen)
    }

    /// Whether any screen is left. The loop ends when the last one finishes.
    pub fn is_running(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Whether the screen has changed since it was last painted.
    pub fn is_dirty(&self) -> bool {
        self.dirty || crate::host::chrome::needs_paint()
    }

    /// Marks the screen as needing a repaint, for a host reacting to something
    /// the framework cannot see — a window resize, say.
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    /// One frame of input, then whatever navigation it asked for.
    pub fn tick(&mut self) {
        // The borrow on the stack ends here, before anything can edit it.
        if let Some(top) = self.stack.last_mut() {
            top.loop_();
        }

        // Order is pop-then-push, so a screen that finishes itself and opens a
        // replacement in one frame ends up with the replacement on top of the
        // screen it came from, not on top of itself.
        let finished = self.shell.take_finish().then(|| self.pop()).flatten();
        if let Some(next) = self.shell.take_pending() {
            self.push_driver(next);
        }
        // Last, so a screen's `Drop` cannot re-enter a stack still being
        // edited. Anything it asks for now lands on the next frame.
        drop(finished);
    }

    /// Paints the top screen, and whatever it is transparent over.
    pub fn render(&mut self) {
        // Cleared before painting, not after: this paint answers the requests
        // that exist now, and one made *during* it is about the next frame.
        self.dirty = false;
        crate::host::chrome::clear_needs_paint();

        if self.stack.is_empty() {
            return;
        }

        // An overlay does not clear, so whatever sits under it has to be
        // painted first or it overlays a stale panel. Capped one short of the
        // stack, since the bottom screen has nothing beneath it to reveal.
        let overlays = self
            .stack
            .iter()
            .rev()
            .take_while(|screen| screen.is_overlay())
            .count()
            .min(self.stack.len() - 1);

        for index in (self.stack.len() - overlays - 1)..self.stack.len() {
            self.stack[index].render();
        }
    }

    /// Paints only when something changed. E-ink takes a second or more to
    /// refresh, so an unconditional repaint is not free.
    pub fn render_if_dirty(&mut self) -> bool {
        if !self.is_dirty() || self.stack.is_empty() {
            return false;
        }
        self.render();
        true
    }

    /// Offers the system home gesture to the top screen, and pops everything
    /// down to the root if nothing claims it.
    pub fn home_gesture(&mut self) {
        if let Some(top) = self.stack.last_mut()
            && top.handle_home_gesture()
        {
            return;
        }
        let mut popped = Vec::new();
        while self.stack.len() > 1 {
            popped.extend(self.pop());
        }
        drop(popped);
    }
}
