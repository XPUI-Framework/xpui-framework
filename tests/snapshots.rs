//! Whole-screen snapshots, driven through the real runtime.
//!
//! The other test files assert one property each: where a knob sits, which
//! word a toggle shows. These assert the opposite thing — *everything* a frame
//! drew, in order — so a change nobody thought to write an assertion for still
//! shows up, as a diff a reviewer can read.
//!
//! Goldens live in `tests/snapshots/`. To accept an intended change:
//!
//! ```bash
//! UPDATE_SNAPSHOTS=1 cargo test --features testing
//! ```
//!
//! Read the diff before committing it.

use xpui::screen::{Driver, Runtime};
use xpui::testing;
use xpui::{
    Button, Divider, Hint, List, ListRow, Modal, NavigationScreen, OverlayPanel, ProgressBar,
    Screen, Scrim, ScrollView, Section, Slider, Spacer, Stepper, SwipeDir, Text, Toggle, View,
    hstack, vstack,
};

/// Renders one frame of a screen and compares it against its golden.
fn snapshot<S: Screen + 'static>(name: &str, screen: S) {
    testing::install();
    testing::reset();
    let mut runtime = Runtime::new(screen);
    runtime.render();
    testing::assert_snapshot(name);
}

// -- a settings list -------------------------------------------------------

#[derive(Clone, Copy)]
enum SettingsMsg {
    Open(usize),
}

struct Settings {
    hyphenation: bool,
    opened: Option<usize>,
}

impl Settings {
    fn new() -> Self {
        Settings {
            hyphenation: false,
            opened: None,
        }
    }
}

impl Screen for Settings {
    type Message = SettingsMsg;

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![12;
            List::new()
                .push(ListRow::new("Wi-Fi").value("Off").on_tap(SettingsMsg::Open(0)))
                .push(ListRow::new("Storage").subtitle("3.1 GB free").on_tap(SettingsMsg::Open(1)))
                .push(ListRow::new("About").on_tap(SettingsMsg::Open(2))),
            Divider::new(),
            Toggle::new("Hyphenation", self.hyphenation, "On", "Off")
                .on_change(|_| SettingsMsg::Open(3)),
        ])
        .title("Settings")
    }

    fn update(&mut self, message: Self::Message) {
        let SettingsMsg::Open(index) = message;
        self.opened = Some(index);
    }
}

/// The everyday case: a titled page with a list, a rule and a toggle row. If
/// anything about chrome ordering changes — header before content, hints after
/// — this is what notices.
#[test]
fn settings_list() {
    snapshot("settings_list", Settings::new());
}

// -- a brightness panel ----------------------------------------------------

#[derive(Clone, Copy)]
enum LightMsg {
    Set(i32),
    Step(i32),
}

struct Brightness {
    level: i32,
}

impl Screen for Brightness {
    type Message = LightMsg;

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![12;
            hstack![8; Text::new("Brightness"), Spacer::new(), Text::new("60%")],
            Stepper::new(self.level)
                .on_change(LightMsg::Set)
                .on_step(LightMsg::Step),
            Slider::new(self.level, 100).on_change(LightMsg::Set),
            ProgressBar::percent(self.level as u32),
        ])
        .title("Light")
        .hints(Hint::Standard, Hint::text("Save"), Hint::None, Hint::None)
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            LightMsg::Set(level) => self.level = level.clamp(0, 100),
            LightMsg::Step(delta) => self.level = (self.level + delta).clamp(0, 100),
        }
    }
}

/// Every host-painted control on one page — stepper, slider, progress bar —
/// plus a custom hint label and a blanked slot. The hint line in the golden is
/// the only place the `Standard` / `None` / `Text` distinction is visible.
#[test]
fn brightness_panel() {
    snapshot("brightness_panel", Brightness { level: 60 });
}

// -- a dialog over content -------------------------------------------------

struct Picker {
    open: bool,
}

impl Screen for Picker {
    type Message = usize;

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(
            List::new()
                .push(ListRow::new("Font").value("Serif").on_tap(0))
                .push(ListRow::new("Size").value("16").on_tap(1)),
        )
        .title("Reading")
        .overlay_if(
            self.open,
            Modal::picker("Font", ["Serif", "Sans", "Mono"])
                .selected(1)
                .on_select(|index| index)
                .scrim(Scrim::Dim),
        )
    }

    fn update(&mut self, _message: Self::Message) {}
}

/// A dialog captures input, so the list behind it must be told to highlight
/// nothing. The golden shows `list ... selected=-1` under an open dialog and
/// `selected=0` without one — the two snapshots together are the assertion.
#[test]
fn picker_closed() {
    snapshot("picker_closed", Picker { open: false });
}

#[test]
fn picker_open() {
    snapshot("picker_open", Picker { open: true });
}

// -- scrolling -------------------------------------------------------------

struct LongPage;

impl Screen for LongPage {
    type Message = usize;

    fn body(&self) -> impl View<Self::Message> {
        // Deliberately taller than the 700px content band. A page that fits
        // reports offset=0 forever and would pass every assertion below
        // without a scroll ever happening.
        NavigationScreen::new(ScrollView::new(vstack![8;
            Section::new("Display", List::new()
                .push(ListRow::new("Brightness").value("60%").on_tap(0))
                .push(ListRow::new("Warmth").value("30%").on_tap(1))
                .push(ListRow::new("Frontlight").value("On").on_tap(2))
                .push(ListRow::new("Invert").value("Off").on_tap(3))),
            Section::new("Reading", List::new()
                .push(ListRow::new("Font").value("Serif").on_tap(4))
                .push(ListRow::new("Margins").value("Medium").on_tap(5))
                .push(ListRow::new("Line spacing").value("1.4").on_tap(6))
                .push(ListRow::new("Hyphenation").value("On").on_tap(7))),
            Section::new("System", List::new()
                .push(ListRow::new("Language").value("English").on_tap(8))
                .push(ListRow::new("Time zone").value("CEST").on_tap(9))
                .push(ListRow::new("Sleep after").value("15 min").on_tap(10))
                .push(ListRow::new("Storage").value("3.1 GB free").on_tap(11))),
            Section::new("About", List::new()
                .push(ListRow::new("Firmware").value("1.4.2").on_tap(12))
                .push(ListRow::new("Serial").value("X4-0001").on_tap(13))
                .push(ListRow::new("Licences").on_tap(14))),
        ]))
        .title("All settings")
    }

    fn update(&mut self, _message: Self::Message) {}
}

/// Content taller than the band it sits in. The golden pins the clip going on
/// before the content and coming off after it — miss the lift and the button
/// hints get clipped away, which is invisible until you look at a device.
#[test]
fn scrolling_sections() {
    snapshot("scrolling_sections", LongPage);

    let (content, visible, offset) = testing::drawn_indicators()[0];
    assert!(
        content > visible,
        "this page must overflow or it tests nothing: content {content}, visible {visible}"
    );
    assert_eq!(offset, 0, "and it opens at the top");

    let clips = testing::clips();
    assert_eq!(
        clips,
        vec![clips[0], None],
        "the clip must be lifted, or the hints drawn after it are clipped away"
    );
}

/// The same page after walking focus to the bottom. Scrolling is the runtime's
/// job, so the only evidence it happened is that the content moved and the
/// indicator's offset changed.
#[test]
fn scrolling_sections_at_the_bottom() {
    testing::install();
    testing::reset();
    let mut runtime = Runtime::new(LongPage);
    runtime.render();
    let opened_at = testing::drawn_indicators()[0].2;

    // Wrap backwards to the last control: one press, no arithmetic about how
    // many focusable rows the page happens to have.
    testing::press(Button::Up);
    runtime.loop_();

    testing::reset();
    runtime.render();

    let (content, visible, offset) = testing::drawn_indicators()[0];
    assert_eq!(opened_at, 0, "it opened at the top");
    assert_eq!(
        offset,
        content - visible,
        "and wrapping to the last control scrolls all the way down"
    );

    testing::assert_snapshot("scrolling_sections_at_the_bottom");
}

// -- an overlay panel ------------------------------------------------------

struct Panel;

impl Screen for Panel {
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        OverlayPanel::new(vstack![8;
            Text::new("Aa").bold(),
            Slider::new(40, 100),
        ])
        .on_scrim_tap(())
        .scrim(Scrim::Dim)
    }

    fn update(&mut self, _message: Self::Message) {}

    fn is_overlay(&self) -> bool {
        true
    }
}

/// An overlay paints over what is already on the panel, so it must NOT clear.
/// The golden's first line is the assertion: a `clear` here would mean the
/// screen underneath was wiped.
#[test]
fn overlay_panel_does_not_clear() {
    snapshot("overlay_panel", Panel);

    let cleared = testing::ops_log()
        .iter()
        .any(|op| matches!(op, testing::DrawOp::Clear));
    assert!(!cleared, "an overlay must leave the screen beneath it");
}

// -- focus moves the highlight ---------------------------------------------

/// Focus is the runtime's, and a list highlights whichever row holds it with
/// no screen code at all. Two snapshots one keypress apart are the proof.
#[test]
fn focus_moves_the_highlight() {
    testing::install();
    testing::reset();
    let mut runtime = Runtime::new(Settings::new());
    runtime.render();
    assert_eq!(
        testing::drawn_lists()
            .first()
            .map(|(_, selected)| *selected),
        Some(0),
        "focus opens on the first row"
    );

    testing::press(Button::Down);
    runtime.loop_();
    testing::reset();
    runtime.render();
    testing::assert_snapshot("settings_list_second_row_focused");
}

/// A vertical swipe walks the same focus a button does, so a touch panel and a
/// button one navigate identically. Snapshotting the result is how that stays
/// true.
#[test]
fn a_swipe_walks_the_same_focus() {
    testing::install();
    testing::reset();
    let mut runtime = Runtime::new(Settings::new());
    runtime.render();

    testing::set_swipe(SwipeDir::Up);
    runtime.loop_();
    testing::reset();
    runtime.render();

    let selected = testing::drawn_lists().first().map(|(_, s)| *s);
    assert_eq!(
        selected,
        Some(1),
        "swiping up drags the content, which walks focus down one row"
    );
}
