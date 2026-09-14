//! What a key does once it reaches the runtime: every button is offered to the
//! screen, Back under a dialog belongs to the dialog, and a stepper answers the
//! keys with nothing but `on_change`.
//!
//! `App` installs itself as the process-wide navigator, so these serialise on a
//! lock rather than racing each other for that global.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

use xpui::screen::Screen;
use xpui::{
    App, Button, InputMask, Interactions, List, ListRow, Modal, Modifiers, NavigationScreen, Point,
    Stepper, Text, View, testing, vstack,
};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    let guard = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    testing::install();
    testing::reset();
    guard
}

/// One frame with `button` pressed in it.
fn press(app: &mut App, button: Button) {
    testing::press(button);
    app.tick();
    app.render_if_dirty();
}

/// Every message a screen was handed, in order.
type Log<M> = Rc<RefCell<Vec<M>>>;

// -- every button reaches the screen ------------------------------------------

const ALL: [Button; 15] = [
    Button::Back,
    Button::Confirm,
    Button::Left,
    Button::Right,
    Button::Up,
    Button::Down,
    Button::Power,
    Button::PageBack,
    Button::PageForward,
    Button::NavNext,
    Button::NavPrevious,
    Button::ScreenLeft,
    Button::ScreenRight,
    Button::ScreenUp,
    Button::ScreenDown,
];

/// Claims every key it is offered.
struct Claims {
    log: Log<Button>,
}

impl Screen for Claims {
    type Message = Button;

    fn body(&self) -> impl View<Button> {
        NavigationScreen::new(vstack![0; Text::new("keys")])
    }

    fn update(&mut self, message: Button) {
        self.log.borrow_mut().push(message);
    }

    fn on_key(&self, key: Button) -> Option<Button> {
        Some(key)
    }
}

/// A reader's side keys, the power key and the orientation-fixed directions
/// have no meaning of the runtime's, and used to be dropped before the screen
/// could see them.
#[test]
fn every_button_is_offered_to_the_screen() {
    let _guard = serial();
    let log = Log::default();
    let mut app = App::new(Claims { log: log.clone() });
    app.render();

    for button in ALL {
        press(&mut app, button);
    }

    assert_eq!(*log.borrow(), ALL, "each key, once, in the order pressed");
    assert_eq!(app.depth(), 1, "Back was claimed, so nothing finished");
}

// -- Back under a dialog -----------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pick {
    Chose(usize),
    Dismiss,
    Claimed,
}

/// A screen with a dialog open, optionally dismissable, optionally claiming
/// Back itself.
struct Picking {
    dismissable: bool,
    claims_back: bool,
    log: Log<Pick>,
}

impl Screen for Picking {
    type Message = Pick;

    fn body(&self) -> impl View<Pick> {
        let dialog = Modal::picker("Typeface", ["Serif", "Sans"]).on_select(Pick::Chose);
        let dialog = if self.dismissable {
            dialog.on_dismiss(Pick::Dismiss)
        } else {
            dialog
        };
        NavigationScreen::new(List::new().push(ListRow::new("Typeface").on_tap(Pick::Chose(9))))
            .overlay(dialog)
    }

    fn update(&mut self, message: Pick) {
        self.log.borrow_mut().push(message);
    }

    fn on_key(&self, key: Button) -> Option<Pick> {
        (self.claims_back && key == Button::Back).then_some(Pick::Claimed)
    }
}

fn picking(dismissable: bool, claims_back: bool) -> (App, Log<Pick>) {
    let log = Log::default();
    let mut app = App::new(Leaf);
    app.push(Picking {
        dismissable,
        claims_back,
        log: log.clone(),
    });
    app.render();
    (app, log)
}

/// Underneath the screen with the dialog, so a finish shows as a pop.
struct Leaf;

impl Screen for Leaf {
    type Message = ();

    fn body(&self) -> impl View<()> {
        NavigationScreen::new(vstack![0; Text::new("under")])
    }

    fn update(&mut self, _message: ()) {}
}

/// Back closes the dialog, through the message the dialog was given, and the
/// screen stays.
#[test]
fn back_under_a_dialog_sends_its_dismiss_message() {
    let _guard = serial();
    let (mut app, log) = picking(true, false);

    press(&mut app, Button::Back);

    assert_eq!(*log.borrow(), [Pick::Dismiss]);
    assert_eq!(app.depth(), 2, "the screen under the dialog is still there");
}

/// A dialog with no way to dismiss it swallows Back: leaving the screen from
/// under an open question is never what the key meant.
#[test]
fn back_under_a_dialog_without_on_dismiss_does_nothing() {
    let _guard = serial();
    let (mut app, log) = picking(false, false);

    press(&mut app, Button::Back);
    press(&mut app, Button::Back);

    assert!(log.borrow().is_empty(), "nothing was dispatched");
    assert_eq!(app.depth(), 2, "and the screen did not finish");
}

/// The screen is still asked first, as it is for any key.
#[test]
fn a_screen_claiming_back_keeps_priority_over_its_dialog() {
    let _guard = serial();
    let (mut app, log) = picking(true, true);

    press(&mut app, Button::Back);

    assert_eq!(*log.borrow(), [Pick::Claimed]);
    assert_eq!(app.depth(), 2);
}

/// A capturing dialog's dismissal is read by the runtime from what the dialog
/// declared, and a second dialog does not inherit the first one's.
#[test]
fn a_dialog_declares_its_dismissal_and_only_its_own() {
    testing::install();
    let mut out: Interactions<Pick> = Interactions::new(0);

    let mut first = Modal::picker("One", ["a"]).on_dismiss(Pick::Dismiss);
    first.measure(testing::screen());
    first.interactions(Point::ORIGIN, &mut out);
    assert_eq!(out.dismissal(), Some(&Pick::Dismiss));

    let mut second = Modal::picker("Two", ["b"]);
    second.measure(testing::screen());
    second.interactions(Point::ORIGIN, &mut out);
    assert_eq!(out.dismissal(), None, "the dialog on top says nothing");

    let mut loose: Interactions<Pick> = Interactions::new(0);
    loose.dismiss_with(Pick::Dismiss);
    assert_eq!(
        loose.dismissal(),
        None,
        "nothing captured, so nothing to dismiss"
    );
}

// -- a stepper with only on_change ---------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Set(i32),
    Step(i32),
    Row,
}

/// A stepper over 0..=10 above a plain row, with `on_step` only if asked.
struct Levels {
    value: i32,
    stepped: bool,
    log: Log<Level>,
}

impl Screen for Levels {
    type Message = Level;

    fn body(&self) -> impl View<Level> {
        let stepper = Stepper::ranged(self.value, 10).on_change(Level::Set);
        let stepper = if self.stepped {
            stepper.on_step(Level::Step)
        } else {
            stepper
        };
        NavigationScreen::new(vstack![8; stepper, Text::new("row").on_tap(Level::Row)])
    }

    fn update(&mut self, message: Level) {
        self.log.borrow_mut().push(message);
        match message {
            Level::Set(value) => self.value = value,
            Level::Step(delta) => self.value += delta,
            Level::Row => {}
        }
    }
}

fn levels(value: i32, stepped: bool) -> (App, Log<Level>) {
    let log = Log::default();
    let mut app = App::new(Levels {
        value,
        stepped,
        log: log.clone(),
    });
    app.render();
    (app, log)
}

/// With the pair, Left and Right move the value one step, held inside the
/// range, through the only message the stepper was given.
#[test]
fn left_and_right_drive_a_stepper_with_only_on_change() {
    let _guard = serial();
    testing::set_has_left_right_keys(true);
    let (mut app, log) = levels(9, false);

    press(&mut app, Button::Right);
    press(&mut app, Button::Right);
    press(&mut app, Button::Left);

    assert_eq!(
        *log.borrow(),
        [Level::Set(10), Level::Set(10), Level::Set(9)],
        "absolute values, clamped at the top of the range"
    );
}

/// It is a focus stop: Down leaves it for the row below, and Confirm then fires
/// that row.
#[test]
fn a_stepper_with_only_on_change_is_a_focus_stop() {
    let _guard = serial();
    testing::set_has_left_right_keys(true);
    let (mut app, log) = levels(0, false);

    press(&mut app, Button::Left);
    press(&mut app, Button::Down);
    press(&mut app, Button::Confirm);

    assert_eq!(
        *log.borrow(),
        [Level::Set(0), Level::Row],
        "clamped at the bottom, then the row the focus moved to"
    );
}

/// Without the pair, Confirm opens it and commits one absolute value.
#[test]
fn a_stepper_with_only_on_change_opens_and_commits() {
    let _guard = serial();
    let (mut app, log) = levels(4, false);

    for key in [Button::Confirm, Button::Up, Button::Up, Button::Confirm] {
        press(&mut app, key);
    }

    assert_eq!(*log.borrow(), [Level::Set(6)], "one message for the edit");
}

/// Given both, `on_step` keeps the keys, as it always did.
#[test]
fn a_stepper_with_both_messages_still_steps() {
    let _guard = serial();
    testing::set_has_left_right_keys(true);
    let (mut app, log) = levels(4, true);

    press(&mut app, Button::Right);

    assert_eq!(*log.borrow(), [Level::Step(1)]);
}

/// The glyphs are drawn, and a touch on each sends the value beside the one
/// shown.
#[test]
fn a_stepper_with_only_on_change_has_glyphs() {
    let _guard = serial();
    let mut stepper: Stepper<Level> = Stepper::ranged(0, 10).on_change(Level::Set);
    stepper.measure(testing::screen());
    stepper.render(Point::ORIGIN);

    let drawn: Vec<String> = testing::drawn_text()
        .into_iter()
        .map(|(_, _, text, _, _)| text)
        .collect();
    assert!(drawn.iter().any(|text| text == "-"), "a minus: {drawn:?}");
    assert!(drawn.iter().any(|text| text == "+"), "a plus: {drawn:?}");

    let mut out = Interactions::new(0);
    stepper.interactions(Point::ORIGIN, &mut out);
    let touched: Vec<Level> = out
        .items()
        .iter()
        .filter(|item| item.mask == InputMask::TAP)
        .map(|item| item.trigger.resolve(item.rect, item.rect.x()))
        .collect();
    assert_eq!(
        touched,
        [Level::Set(0), Level::Set(1)],
        "minus is held at the bottom of the range, plus is one up"
    );
}
