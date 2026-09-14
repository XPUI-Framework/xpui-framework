//! The framework-owned screen stack.
//!
//! Most of these exist because of one hazard: `finish()` and `present()` are
//! called from *inside* a screen's own frame, while `App` holds `&mut` on the
//! screen running it. Anything that edited the stack there would free the
//! screen currently executing. So the tests below are mostly about when things
//! happen, not what they return.
//!
//! `App` installs itself as the process-wide navigator, so these serialise on
//! a lock rather than racing each other for that global.

use std::sync::{Mutex, MutexGuard};

use xpui::screen::Screen;
use xpui::{
    App, Button, List, ListRow, NavigationScreen, Text, View, finish_screen, present, testing,
    vstack,
};

static SERIAL: Mutex<()> = Mutex::new(());

/// One `App` at a time: the installed navigator is process-wide.
fn serial() -> MutexGuard<'static, ()> {
    let guard = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    testing::install();
    testing::reset();
    guard
}

// -- screens ---------------------------------------------------------------

/// Finishes itself the moment Confirm is pressed, from inside its own frame.
struct Leaf {
    name: &'static str,
}

impl Screen for Leaf {
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![0; Text::new(self.name)])
    }

    fn update(&mut self, _message: Self::Message) {}

    fn title(&self) -> Option<&'static str> {
        Some(self.name)
    }

    fn on_key(&self, key: Button) -> Option<Self::Message> {
        if key == Button::Confirm {
            // Straight out of the frame the runtime is currently running.
            finish_screen();
        }
        None
    }
}

/// Pushes another screen from inside its own frame.
struct Opener;

impl Screen for Opener {
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![0; Text::new("open me")])
    }

    fn update(&mut self, _message: Self::Message) {}

    fn title(&self) -> Option<&'static str> {
        Some("Opener")
    }

    fn on_key(&self, key: Button) -> Option<Self::Message> {
        if key == Button::Confirm {
            present(Leaf { name: "Pushed" });
        }
        None
    }
}

/// Paints over whatever is beneath it.
struct Sheet;

impl Screen for Sheet {
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![0; Text::new("sheet")])
    }

    fn update(&mut self, _message: Self::Message) {}

    fn is_overlay(&self) -> bool {
        true
    }
}

/// Calls back into the navigator from `Drop`, which runs while the stack is
/// being edited.
struct NoisyOnDrop;

impl Screen for NoisyOnDrop {
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![0; Text::new("bye")])
    }

    fn update(&mut self, _message: Self::Message) {}

    fn on_key(&self, key: Button) -> Option<Self::Message> {
        if key == Button::Confirm {
            finish_screen();
        }
        None
    }
}

impl Drop for NoisyOnDrop {
    fn drop(&mut self) {
        // A screen dropped mid-navigation asking to navigate again. It must
        // not re-enter a stack that is still being edited.
        finish_screen();
    }
}

// -- the tests -------------------------------------------------------------

#[test]
fn an_app_starts_on_its_root() {
    let _guard = serial();
    let app = App::new(Leaf { name: "Root" });

    assert!(app.is_running());
    assert_eq!(app.depth(), 1);
    assert!(app.is_dirty(), "a screen just entered has not been painted");
}

/// The whole reason for the deferred flag. `finish()` fires while the runtime
/// holds the screen; the pop must wait until the frame is over.
#[test]
fn finishing_pops_after_the_frame_not_during_it() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Root" });
    app.push(Leaf { name: "Second" });
    assert_eq!(app.depth(), 2);

    testing::press(Button::Confirm);
    app.tick();

    assert_eq!(app.depth(), 1, "the pop happened, once the frame was over");
    assert!(app.is_running());
}

#[test]
fn finishing_the_last_screen_ends_the_app() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Only" });

    testing::press(Button::Confirm);
    app.tick();

    assert_eq!(app.depth(), 0);
    assert!(!app.is_running(), "the loop has nothing left to run");
}

// -- a root a device cannot leave --------------------------------------------

/// Finishes itself and opens a replacement in the same frame.
struct Replacer;

impl Screen for Replacer {
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![0; Text::new("splash")])
    }

    fn update(&mut self, _message: Self::Message) {}

    fn on_key(&self, key: Button) -> Option<Self::Message> {
        if key == Button::Confirm {
            finish_screen();
            present(Leaf { name: "Menu" });
        }
        None
    }
}

/// Counts the Backs that reach it, without claiming them.
///
/// Deliberately returns `None`: a screen that *claimed* Back would never reach
/// the policy under test. What this proves is that the key arrives at all —
/// which a host withholding it at the pin is what takes away.
struct Rooted {
    backs: std::rc::Rc<std::cell::Cell<u32>>,
}

impl Screen for Rooted {
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![0; Text::new("root")])
    }

    fn update(&mut self, _message: Self::Message) {}

    fn on_key(&self, key: Button) -> Option<Self::Message> {
        if key == Button::Back {
            self.backs.set(self.backs.get() + 1);
        }
        None
    }
}

/// A host with nothing under its root keeps running when Back reaches it.
///
/// A board whose stack empties stops its frame loop, and a board that stops
/// answering looks exactly like one that crashed — every other key goes quiet
/// with it.
#[test]
fn a_kept_root_survives_the_key_that_would_finish_it() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Only" }).keep_root();

    testing::press(Button::Confirm);
    app.tick();

    assert_eq!(app.depth(), 1, "the root is still on the stack");
    assert!(app.is_running(), "and the loop still has something to run");
}

/// Keeping the root does not take the key away from the screen.
///
/// This is the half a host-side guard destroys. Declining the *pop* leaves Back
/// free to mean what a screen or an open edit wants it to mean; withholding the
/// key means a screen cannot dismiss its own picker and a value opened here
/// could be committed but never cancelled.
#[test]
fn a_kept_root_still_sees_back() {
    let _guard = serial();
    let backs = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut app = App::new(Rooted {
        backs: std::rc::Rc::clone(&backs),
    })
    .keep_root();

    testing::press(Button::Back);
    app.tick();
    testing::press(Button::Back);
    app.tick();

    assert_eq!(backs.get(), 2, "both presses reached the screen's on_key");
    assert!(app.is_running(), "and neither ended the app");
}

/// The root is kept; anything above it pops as it always did.
///
/// The bound matters as much as the policy. Without it a board would open a
/// screen from its menu and never get back — worse than the fault this
/// replaces, and every board using it would be stuck until it was re-flashed.
#[test]
fn keeping_the_root_does_not_pin_the_screens_above_it() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Root" }).keep_root();
    app.push(Leaf { name: "Second" });
    assert_eq!(app.depth(), 2);

    testing::press(Button::Confirm);
    app.tick();

    assert_eq!(app.depth(), 1, "the screen above the root still finishes");
    assert!(app.is_running());
}

/// A root that replaces itself is replaced, not stacked on.
///
/// Finishing and presenting in one frame is how a splash becomes a menu. The
/// root is kept from being *emptied*, not from being swapped — pinning it here
/// too would leave the splash alive underneath for the life of the firmware,
/// and Back from the menu would land on it.
#[test]
fn a_kept_root_can_still_replace_itself() {
    let _guard = serial();
    let mut app = App::new(Replacer).keep_root();

    testing::press(Button::Confirm);
    app.tick();

    assert_eq!(
        app.depth(),
        1,
        "the replacement is the root, not a screen on it"
    );
    assert!(app.is_running());
}

/// A request declined at the root does not fire later.
///
/// `finish_screen` sets a flag the shell clears when it acts on it. Left set
/// while the root declines it, the next push would be popped straight back off
/// by a request made frames earlier against a different screen.
#[test]
fn a_declined_finish_is_not_remembered() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Only" }).keep_root();

    testing::press(Button::Confirm);
    app.tick();
    assert_eq!(app.depth(), 1);

    app.push(Leaf { name: "Second" });
    assert_eq!(app.depth(), 2);

    // A frame with no key pressed. The stale request, if it were kept, would
    // pop the screen just pushed. Reading does not consume the fake's press,
    // so the frame that carried it is ended first.
    testing::next_frame();
    app.tick();
    assert_eq!(app.depth(), 2, "the declined request did not carry over");
}

/// Same hazard in the other direction: a screen pushing from inside its frame.
#[test]
fn presenting_pushes_after_the_frame() {
    let _guard = serial();
    let mut app = App::new(Opener);

    testing::press(Button::Confirm);
    app.tick();

    assert_eq!(app.depth(), 2, "the pushed screen arrived");
}

/// A `Drop` that navigates runs after the stack edit is finished, so it lands
/// on the next frame rather than re-entering this one.
#[test]
fn a_screen_that_navigates_from_drop_does_not_re_enter() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Root" });
    app.push(NoisyOnDrop);

    testing::press(Button::Confirm);
    app.tick(); // pops NoisyOnDrop; its Drop asks to finish again

    assert_eq!(app.depth(), 1, "exactly one screen came off this frame");

    // The Drop's request was recorded, and is honoured on the next frame.
    app.tick();
    assert_eq!(app.depth(), 0, "and the deferred request lands next frame");
}

/// A screen presenting twice in one frame: the second is handed back rather
/// than silently replacing the first.
#[test]
fn only_one_screen_is_accepted_per_frame() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Root" });

    assert!(present(Leaf { name: "First" }), "the first is taken");
    assert!(
        !present(Leaf { name: "Second" }),
        "the second is refused rather than losing the first"
    );

    app.tick();
    assert_eq!(app.depth(), 2);
}

/// Navigation makes the screen stale without anyone asking.
///
/// It is not the only thing that does — see `moving_the_focus_repaints`, which
/// is what keeps the arrow keys from doing nothing.
#[test]
fn navigating_marks_the_screen_dirty() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Root" });
    app.render();
    assert!(!app.is_dirty(), "painting clears it");

    app.push(Leaf { name: "Second" });
    assert!(app.is_dirty(), "and a push sets it again");

    app.render();
    testing::press(Button::Confirm);
    app.tick();
    assert!(
        app.is_dirty(),
        "a pop reveals a screen last painted frames ago — it must repaint"
    );
}

#[test]
fn render_if_dirty_paints_only_once() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Root" });

    assert!(app.render_if_dirty(), "the first frame always paints");
    assert!(
        !app.render_if_dirty(),
        "and an unchanged screen does not repaint — e-ink is slow"
    );
}

/// An overlay deliberately does not clear, so the screen beneath it has to be
/// painted first or it overlays whatever was left on the panel.
#[test]
fn an_overlay_repaints_the_screen_beneath_it() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Under" });
    app.push(Sheet);

    testing::reset();
    app.render();

    let headers = testing::drawn_headers();
    assert_eq!(
        headers.len(),
        2,
        "both the screen underneath and the overlay painted: {headers:?}"
    );

    let clears = testing::ops_log()
        .iter()
        .filter(|op| matches!(op, testing::DrawOp::Clear))
        .count();
    assert_eq!(clears, 1, "cleared once, by the screen underneath");
}

/// A plain screen on top of another paints alone — compositing is for
/// overlays, and repainting the whole stack on e-ink would be visible.
#[test]
fn an_opaque_screen_paints_alone() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Under" });
    app.push(Leaf { name: "Over" });

    testing::reset();
    app.render();

    assert_eq!(
        testing::drawn_headers().len(),
        1,
        "only the top screen painted"
    );
}

/// The title follows the stack, so a header with no explicit title of its own
/// shows the screen that is actually on top.
#[test]
fn the_title_follows_the_top_of_the_stack() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Root" });
    assert_eq!(xpui::ScreenChrome::screen_title(), "Root");

    app.push(Leaf { name: "Second" });
    assert_eq!(xpui::ScreenChrome::screen_title(), "Second");

    testing::press(Button::Confirm);
    app.tick();
    assert_eq!(
        xpui::ScreenChrome::screen_title(),
        "Root",
        "popping restores the title underneath"
    );
}

/// The home gesture unwinds to the root rather than quitting.
#[test]
fn the_home_gesture_unwinds_to_the_root() {
    let _guard = serial();
    let mut app = App::new(Leaf { name: "Root" });
    app.push(Leaf { name: "Second" });
    app.push(Leaf { name: "Third" });

    app.home_gesture();

    assert_eq!(app.depth(), 1);
    assert!(app.is_running(), "home goes to the root, it does not quit");
}

// -- a keypress that changes nothing but the focus -------------------------

/// Three rows, so Up and Down have somewhere to go.
struct Rows;

impl Screen for Rows {
    type Message = usize;

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(
            List::new().extend((0..3).map(|index| ListRow::new("row").on_tap(index))),
        )
    }

    fn update(&mut self, _message: Self::Message) {}

    fn title(&self) -> Option<&'static str> {
        Some("Rows")
    }
}

/// Moving the focus must repaint.
///
/// This is the one the suite was missing. Every other focus test drives
/// `Runtime` and asserts `focused_index()`, which moves correctly even when
/// nothing reaches the panel — so a screen could change and never be shown.
/// That is exactly what happened: `request_update()` sets the *host's* dirty
/// flag, and `App` was consulting only its own, which nothing but navigation
/// ever set.
#[test]
fn moving_the_focus_repaints() {
    let _guard = serial();
    let mut app = App::new(Rows);

    app.render();
    assert!(!app.is_dirty(), "painting clears it");

    testing::press(Button::Down);
    app.tick();

    assert!(
        app.render_if_dirty(),
        "focus moved, so the panel is stale — without this the arrow keys \
         appear dead: the selection moves internally and is never drawn"
    );
}

// -- a screen that watches the clock ---------------------------------------

/// Counts the frames it was told about.
struct Ticker {
    frames: std::rc::Rc<std::cell::Cell<u32>>,
}

impl Screen for Ticker {
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![0; Text::new("tick")])
    }

    fn update(&mut self, _message: Self::Message) {}

    fn tick(&mut self) {
        self.frames.set(self.frames.get() + 1);
    }
}

/// A screen hears about frames where nothing happened.
///
/// That is the whole point of the hook: an action held back to see whether a
/// second press is coming has no other way to learn that the wait is over. A
/// version called only alongside input would look correct and never fire.
#[test]
fn a_screen_is_told_about_quiet_frames() {
    let _guard = serial();
    let frames = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut app = App::new(Ticker {
        frames: frames.clone(),
    });

    for _ in 0..5 {
        app.tick();
    }

    assert_eq!(
        frames.get(),
        5,
        "no key was pressed in any of those frames, and the screen still has to \
         hear that time passed"
    );
}
