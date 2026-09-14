//! A finger held, a tap outside a dialog, and the two system gestures.
//!
//! The fake host reports no touch and no gestures, so these run over a host of
//! their own: every drawing and measuring call goes to the fake, and the input
//! is whatever the test put in the atomics below. `Ui::new` is the one safe way
//! to install such a host. It is built and dropped straight away, which leaves
//! the host installed, and the frames are then driven by hand: `Ui` has no verb
//! for a finger resting on the panel, or for a gesture.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use xpui::host::{
    Canvas, Chrome, Clock, ControlState, FontId, FontRole, FontStyle, Hint, IconRef, InputSource,
    RowField, TextMetrics, ThemeMetric,
};
use xpui::screen::Screen;
use xpui::testing::{self, Drive, SCREEN_HEIGHT, TestHost, Ui};
use xpui::{
    App, Button, InputMask, List, ListRow, Modal, Modifiers, NavigationScreen, Point, Rect, Size,
    SwipeDir, Tappable, Text, Theme, View, finish_screen, vstack,
};

// -- the host ------------------------------------------------------------------

/// Input a test sets, over the fake's drawing.
struct Finger {
    now: AtomicU32,
    /// `Button as u8 + 1`, or `0` for none.
    pressed: AtomicU8,
    down: AtomicBool,
    released: AtomicBool,
    tapped: AtomicBool,
    x: AtomicI32,
    y: AtomicI32,
    back: AtomicBool,
    home: AtomicBool,
}

static FINGER: Finger = Finger {
    now: AtomicU32::new(0),
    pressed: AtomicU8::new(0),
    down: AtomicBool::new(false),
    released: AtomicBool::new(false),
    tapped: AtomicBool::new(false),
    x: AtomicI32::new(0),
    y: AtomicI32::new(0),
    back: AtomicBool::new(false),
    home: AtomicBool::new(false),
};

static FAKE: TestHost = TestHost;

impl Finger {
    /// Forgets this frame's input. The clock stays where it is.
    fn clear(&self) {
        self.pressed.store(0, Ordering::Relaxed);
        self.down.store(false, Ordering::Relaxed);
        self.released.store(false, Ordering::Relaxed);
        self.tapped.store(false, Ordering::Relaxed);
        self.back.store(false, Ordering::Relaxed);
        self.home.store(false, Ordering::Relaxed);
    }

    fn at(&self, point: Point) {
        self.x.store(point.x, Ordering::Relaxed);
        self.y.store(point.y, Ordering::Relaxed);
    }

    fn point(&self) -> Point {
        Point::new(
            self.x.load(Ordering::Relaxed),
            self.y.load(Ordering::Relaxed),
        )
    }
}

/// One frame at `millis`: `input` sets what the frame carries, then the app
/// ticks and paints if asked, then the frame's input is forgotten.
fn frame(app: &mut App, millis: u32, input: impl FnOnce(&Finger)) {
    FINGER.now.store(millis, Ordering::Relaxed);
    input(&FINGER);
    app.tick();
    app.render_if_dirty();
    FINGER.clear();
}

/// A finger resting at `point`.
fn held(point: Point) -> impl FnOnce(&Finger) {
    move |finger| {
        finger.at(point);
        finger.down.store(true, Ordering::Relaxed);
    }
}

/// The finger lifting at `point`, completing a tap there.
fn lifted(point: Point) -> impl FnOnce(&Finger) {
    move |finger| {
        finger.at(point);
        finger.released.store(true, Ordering::Relaxed);
        finger.tapped.store(true, Ordering::Relaxed);
    }
}

impl InputSource for Finger {
    fn was_pressed(&self, button: Button) -> bool {
        self.pressed.load(Ordering::Relaxed) == button as u8 + 1
    }
    fn is_pressed(&self, _button: Button) -> bool {
        false
    }
    fn was_released(&self, _button: Button) -> bool {
        false
    }
    fn has_touch(&self) -> bool {
        self.down.load(Ordering::Relaxed)
            || self.released.load(Ordering::Relaxed)
            || self.tapped.load(Ordering::Relaxed)
    }
    fn has_left_right_keys(&self) -> bool {
        false
    }
    fn tap(&self) -> Option<Point> {
        self.tapped.load(Ordering::Relaxed).then(|| self.point())
    }
    fn touch_held(&self) -> Option<Point> {
        self.down.load(Ordering::Relaxed).then(|| self.point())
    }
    fn touch_released(&self) -> bool {
        self.released.load(Ordering::Relaxed)
    }
    fn swipe(&self) -> SwipeDir {
        SwipeDir::None
    }
    fn was_back_gesture(&self) -> bool {
        self.back.load(Ordering::Relaxed)
    }
    fn was_home_gesture(&self) -> bool {
        self.home.load(Ordering::Relaxed)
    }
}

impl Clock for Finger {
    fn millis(&self) -> u32 {
        self.now.load(Ordering::Relaxed)
    }
}

impl Drive for Finger {
    fn begin(&self, millis: u32) {
        self.clear();
        self.now.store(millis, Ordering::Relaxed);
    }
    fn inject_press(&self, button: Button) {
        self.pressed.store(button as u8 + 1, Ordering::Relaxed);
    }
    fn inject_release(&self, _button: Button) {}
    fn inject_tap(&self, point: Point) {
        self.at(point);
        self.tapped.store(true, Ordering::Relaxed);
    }
    fn inject_swipe(&self, _direction: SwipeDir) {}
}

impl Canvas for Finger {
    fn screen_size(&self) -> Size {
        FAKE.screen_size()
    }
    fn clear(&self) {
        FAKE.clear()
    }
    fn draw_text(&self, origin: Point, text: &str, font: FontId, style: FontStyle) {
        FAKE.draw_text(origin, text, font, style)
    }
    fn fill_rect(&self, rect: Rect, black: bool) {
        FAKE.fill_rect(rect, black)
    }
    fn stroke_rect(&self, rect: Rect) {
        FAKE.stroke_rect(rect)
    }
    fn draw_line(&self, from: Point, to: Point) {
        FAKE.draw_line(from, to)
    }
    fn fill_rect_dither(&self, rect: Rect, light: bool) {
        FAKE.fill_rect_dither(rect, light)
    }
    fn scrim(&self, rect: Rect) {
        FAKE.scrim(rect)
    }
    fn set_clip(&self, rect: Option<Rect>) {
        FAKE.set_clip(rect)
    }
    fn draw_image(&self, origin: Point, data: &[u8], size: Size) {
        FAKE.draw_image(origin, data, size)
    }
    fn draw_icon(&self, origin: Point, icon: IconRef) {
        FAKE.draw_icon(origin, icon)
    }
    fn icon_size(&self, icon: IconRef) -> i32 {
        FAKE.icon_size(icon)
    }
}

impl TextMetrics for Finger {
    fn font(&self, role: FontRole) -> FontId {
        FAKE.font(role)
    }
    fn text_width(&self, font: FontId, text: &str, style: FontStyle) -> i32 {
        FAKE.text_width(font, text, style)
    }
    fn line_height(&self, font: FontId) -> i32 {
        FAKE.line_height(font)
    }
}

impl Chrome for Finger {
    fn metric(&self, metric: ThemeMetric) -> i32 {
        FAKE.metric(metric)
    }
    fn draw_header(&self, title: Option<&str>, subtitle: Option<&str>) {
        FAKE.draw_header(title, subtitle)
    }
    fn draw_sub_header(&self, rect: Rect, label: &str, right_label: Option<&str>) {
        FAKE.draw_sub_header(rect, label, right_label)
    }
    fn draw_button_hints(&self, back: &Hint, confirm: &Hint, previous: &Hint, next: &Hint) {
        FAKE.draw_button_hints(back, confirm, previous, next)
    }
    fn draw_progress_bar(&self, rect: Rect, current: u32, total: u32) {
        FAKE.draw_progress_bar(rect, current, total)
    }
    fn draw_slider(&self, rect: Rect, value: i32, max: i32, state: ControlState) {
        FAKE.draw_slider(rect, value, max, state)
    }
    fn draw_scroll_indicator(&self, rect: Rect, content: i32, visible: i32, offset: i32) {
        FAKE.draw_scroll_indicator(rect, content, visible, offset)
    }
    fn draw_list<'a>(
        &self,
        rect: Rect,
        rows: usize,
        selected: i32,
        row: &dyn Fn(usize, RowField) -> Option<&'a str>,
    ) {
        FAKE.draw_list(rect, rows, selected, row)
    }
    fn draw_option_popup<'a>(
        &self,
        title: &str,
        options: &dyn Fn(usize) -> Option<&'a str>,
        count: usize,
        selected: i32,
    ) {
        FAKE.draw_option_popup(title, options, count, selected)
    }
    fn option_popup_row_rect<'a>(
        &self,
        title: &str,
        options: &dyn Fn(usize) -> Option<&'a str>,
        count: usize,
        index: usize,
    ) -> Option<Rect> {
        FAKE.option_popup_row_rect(title, options, count, index)
    }
    fn request_update(&self) {
        FAKE.request_update()
    }
}

/// Nothing at all, for the throwaway `Ui` that installs the host.
struct Blank;

impl Screen for Blank {
    type Message = ();
    fn body(&self) -> impl View<()> {
        Text::new("")
    }
    fn update(&mut self, _message: ()) {}
}

static SERIAL: Mutex<()> = Mutex::new(());

/// Installs `FINGER` and holds the lock the rest of the test runs under: the
/// host and the navigator are both process-wide.
fn finger() -> MutexGuard<'static, ()> {
    let guard = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    drop(Ui::new(Blank, &FINGER));
    FINGER.clear();
    testing::reset();
    guard
}

/// Every message a screen was handed, in order.
type Log<M> = Rc<RefCell<Vec<M>>>;

// -- long press ----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Open,
    Menu,
}

/// One label that opens on a tap and offers a menu on a hold.
struct Library {
    log: Log<Row>,
}

impl Screen for Library {
    type Message = Row;

    fn body(&self) -> impl View<Row> {
        // A hold-only region over a tappable label: the tap resolves to the
        // label inside, the hold to the region around it.
        vstack![0;
            Tappable::new(Text::new("Middlemarch").on_tap(Row::Open), Row::Menu)
                .accepting(InputMask::LONG_PRESS),
        ]
    }

    fn update(&mut self, message: Row) {
        self.log.borrow_mut().push(message);
    }
}

/// Well inside the label, which sits at the top-left corner.
const ON_LABEL: Point = Point { x: 4, y: 4 };

fn library() -> (App, Log<Row>) {
    let log = Log::default();
    let mut app = App::new(Library { log: log.clone() });
    app.render();
    (app, log)
}

/// Held past the threshold, the hold fires once, while the finger is still
/// down, and the release that ends it fires nothing.
#[test]
fn a_long_press_fires_once_and_its_release_is_not_a_tap() {
    let _guard = finger();
    let (mut app, log) = library();

    for millis in (1000..=1400).step_by(100) {
        frame(&mut app, millis, held(ON_LABEL));
    }
    assert!(log.borrow().is_empty(), "400 ms is not a long press yet");

    frame(&mut app, 1500, held(ON_LABEL));
    assert_eq!(*log.borrow(), [Row::Menu], "500 ms is");

    frame(&mut app, 1600, held(ON_LABEL));
    frame(&mut app, 1700, lifted(ON_LABEL));
    assert_eq!(
        *log.borrow(),
        [Row::Menu],
        "and neither holding on nor lifting adds anything"
    );

    frame(&mut app, 2000, lifted(ON_LABEL));
    assert_eq!(
        *log.borrow(),
        [Row::Menu, Row::Open],
        "the next quick tap is an ordinary tap"
    );
}

/// A press released under the threshold is a tap, and only a tap.
#[test]
fn a_quick_press_is_still_a_tap() {
    let _guard = finger();
    let (mut app, log) = library();

    frame(&mut app, 1000, held(ON_LABEL));
    frame(&mut app, 1300, held(ON_LABEL));
    frame(&mut app, 1450, lifted(ON_LABEL));

    assert_eq!(*log.borrow(), [Row::Open]);
}

/// `on_long_press` sends its message for a tap and for a hold, and one press
/// sends it once.
#[test]
fn on_long_press_answers_a_hold_as_well_as_a_tap() {
    let _guard = finger();
    let log = Log::default();

    struct Single {
        log: Log<Row>,
    }
    impl Screen for Single {
        type Message = Row;
        fn body(&self) -> impl View<Row> {
            vstack![0; Text::new("Middlemarch").on_long_press(Row::Menu)]
        }
        fn update(&mut self, message: Row) {
            self.log.borrow_mut().push(message);
        }
    }

    let mut app = App::new(Single { log: log.clone() });
    app.render();

    frame(&mut app, 1000, held(ON_LABEL));
    frame(&mut app, 1500, held(ON_LABEL));
    frame(&mut app, 1550, lifted(ON_LABEL));
    assert_eq!(*log.borrow(), [Row::Menu], "the hold, and not its release");

    frame(&mut app, 2000, lifted(ON_LABEL));
    assert_eq!(*log.borrow(), [Row::Menu, Row::Menu], "and a tap");
}

/// A loop blinded by a slow refresh does not credit the gap to the hold.
#[test]
fn a_blind_gap_does_not_count_towards_a_hold() {
    let _guard = finger();
    let (mut app, log) = library();

    frame(&mut app, 1000, held(ON_LABEL));
    frame(&mut app, 1700, held(ON_LABEL));
    assert!(
        log.borrow().is_empty(),
        "the 700 ms nobody saw re-arms the hold"
    );

    frame(&mut app, 2100, held(ON_LABEL));
    assert!(log.borrow().is_empty());
    frame(&mut app, 2200, held(ON_LABEL));
    assert_eq!(
        *log.borrow(),
        [Row::Menu],
        "500 ms of it seen, and it fires"
    );
}

/// A finger that slid off the control before the threshold fires nothing.
#[test]
fn a_finger_that_slid_off_does_not_long_press() {
    let _guard = finger();
    let (mut app, log) = library();
    let away = Point::new(testing::SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1);

    frame(&mut app, 1000, held(ON_LABEL));
    frame(&mut app, 1300, held(away));
    frame(&mut app, 1600, held(away));

    assert!(log.borrow().is_empty());
}

// -- a tap outside a dialog ----------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pick {
    Chose(usize),
    Dismiss,
    Background,
}

const FACES: [&str; 2] = ["Serif", "Sans"];

struct Picking {
    dismissable: bool,
    log: Log<Pick>,
}

impl Screen for Picking {
    type Message = Pick;

    fn body(&self) -> impl View<Pick> {
        let dialog = Modal::picker("Typeface", FACES).on_select(Pick::Chose);
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

    fn on_background_tap(&self, _point: Point) -> Option<Pick> {
        Some(Pick::Background)
    }
}

fn picking(dismissable: bool) -> (App, Log<Pick>) {
    let log = Log::default();
    let mut app = App::new(Picking {
        dismissable,
        log: log.clone(),
    });
    app.render();
    (app, log)
}

/// The bottom-left corner of the panel, which no option row reaches.
const OUTSIDE: Point = Point {
    x: 1,
    y: SCREEN_HEIGHT - 1,
};

/// The middle of the second option.
fn second_option() -> Point {
    let rect = Theme::option_popup_row_rect("Typeface", &|i| FACES.get(i).copied(), 2, 1)
        .expect("the fake lays the dialog out");
    Point::new(rect.x() + rect.width() / 2, rect.y() + rect.height() / 2)
}

/// Outside the options, the dialog's own dismissal is sent, and the screen's
/// background tap is not asked. On an option, the option is chosen.
#[test]
fn a_tap_outside_a_dismissable_dialog_dismisses_it() {
    let _guard = finger();
    let (mut app, log) = picking(true);

    frame(&mut app, 100, lifted(OUTSIDE));
    frame(&mut app, 200, lifted(second_option()));

    assert_eq!(*log.borrow(), [Pick::Dismiss, Pick::Chose(1)]);
}

/// A dialog with no `on_dismiss` leaves the outside tap to the screen, as
/// before.
#[test]
fn without_on_dismiss_an_outside_tap_still_reaches_the_screen() {
    let _guard = finger();
    let (mut app, log) = picking(false);

    frame(&mut app, 100, lifted(OUTSIDE));

    assert_eq!(*log.borrow(), [Pick::Background]);
}

// -- the gestures ------------------------------------------------------------

/// Counts what reaches it, and claims Back only if asked.
struct Page {
    claims_back: bool,
    backs: Rc<Cell<u32>>,
}

impl Screen for Page {
    type Message = ();

    fn body(&self) -> impl View<()> {
        NavigationScreen::new(vstack![0; Text::new("page")])
    }

    fn update(&mut self, _message: ()) {}

    fn on_key(&self, key: Button) -> Option<()> {
        if key == Button::Back {
            self.backs.set(self.backs.get() + 1);
        }
        (self.claims_back && key == Button::Back).then_some(())
    }
}

fn page(claims_back: bool) -> (Page, Rc<Cell<u32>>) {
    let backs = Rc::new(Cell::new(0));
    (
        Page {
            claims_back,
            backs: backs.clone(),
        },
        backs,
    )
}

fn gesture_back(finger: &Finger) {
    finger.back.store(true, Ordering::Relaxed);
}

fn gesture_home(finger: &Finger) {
    finger.home.store(true, Ordering::Relaxed);
}

/// The back gesture is offered to the screen as Back, and unclaimed it
/// finishes the screen.
#[test]
fn the_back_gesture_is_back() {
    let _guard = finger();
    let (root, _) = page(false);
    let (claiming, claimed) = page(true);
    let (plain, offered) = page(false);
    let mut app = App::new(root);
    app.push(claiming);
    app.push(plain);
    app.render();

    frame(&mut app, 100, gesture_back);
    assert_eq!(offered.get(), 1, "offered to on_key");
    assert_eq!(app.depth(), 2, "and, unclaimed, it finished the screen");

    frame(&mut app, 200, gesture_back);
    assert_eq!(claimed.get(), 1);
    assert_eq!(
        app.depth(),
        2,
        "a screen claiming Back keeps the gesture too"
    );
}

/// The back gesture closes a dialog, as the key does.
#[test]
fn the_back_gesture_dismisses_a_dialog() {
    let _guard = finger();
    let (mut app, log) = picking(true);

    frame(&mut app, 100, gesture_back);

    assert_eq!(*log.borrow(), [Pick::Dismiss]);
    assert_eq!(app.depth(), 1);
}

/// `tick` reads the home gesture itself, so a host need not call anything.
#[test]
fn tick_unwinds_on_the_home_gesture() {
    let _guard = finger();
    let mut app = App::new(page(false).0);
    app.push(page(false).0);
    app.push(page(false).0);
    app.render();

    frame(&mut app, 100, gesture_home);

    assert_eq!(app.depth(), 1);
    assert!(app.is_running());
}

/// A drop-down that claims the home gesture by finishing itself.
struct DropDown {
    claims: Rc<Cell<u32>>,
}

impl Screen for DropDown {
    type Message = ();

    fn body(&self) -> impl View<()> {
        Modifiers::<()>::frame(Text::new("Frontlight"), 0, 0)
    }

    fn update(&mut self, _message: ()) {}

    fn is_overlay(&self) -> bool {
        true
    }

    fn handle_home_gesture(&mut self) -> bool {
        self.claims.set(self.claims.get() + 1);
        finish_screen();
        true
    }
}

fn under_a_drop_down() -> (App, Rc<Cell<u32>>) {
    let claims = Rc::new(Cell::new(0));
    let mut app = App::new(page(false).0);
    app.push(page(false).0);
    app.push(DropDown {
        claims: claims.clone(),
    });
    app.render();
    (app, claims)
}

/// A host that still calls `home_gesture` after `tick`, for the gesture `tick`
/// already handled, does not unwind a second time.
#[test]
fn a_host_calling_home_gesture_after_tick_is_not_acted_on_twice() {
    let _guard = finger();
    let (mut app, claims) = under_a_drop_down();

    FINGER.now.store(100, Ordering::Relaxed);
    gesture_home(&FINGER);
    app.tick();
    app.home_gesture();
    FINGER.clear();

    assert_eq!(claims.get(), 1, "the drop-down was offered it once");
    assert_eq!(app.depth(), 2, "and only the drop-down closed");

    // The guard lasts one frame: the next gesture is acted on.
    frame(&mut app, 200, gesture_home);
    assert_eq!(app.depth(), 1);
}

/// And one that calls it before `tick` is not acted on twice either.
#[test]
fn a_host_calling_home_gesture_before_tick_is_not_acted_on_twice() {
    let _guard = finger();
    let (mut app, claims) = under_a_drop_down();

    FINGER.now.store(100, Ordering::Relaxed);
    gesture_home(&FINGER);
    app.home_gesture();
    app.tick();
    FINGER.clear();

    assert_eq!(claims.get(), 1, "the drop-down was offered it once");
    assert_eq!(app.depth(), 2);
}
