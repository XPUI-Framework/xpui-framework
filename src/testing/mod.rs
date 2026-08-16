//! A deterministic stand-in for a host, so layouts can be tested on a desktop
//! with no backend and no simulator.
//!
//! Enabled by `cfg(test)` here, and by the `testing` feature for crates that
//! want to test their own screens or their own backend.
//!
//! Everything drawn lands in one ordered log ([`ops_log`]). The per-kind
//! accessors below — [`drawn_text`], [`drawn_lists`] and friends — are views
//! onto that same log, so "was a list drawn" and "what did the frame look
//! like" can never disagree.
//!
//! ```rust,ignore
//! testing::install();
//! testing::reset();
//! view.measure(testing::screen());
//! view.render(Point::ORIGIN);
//! assert_eq!(testing::drawn_text().len(), 3);
//! testing::assert_snapshot("my_screen");
//! ```

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use crate::geometry::{Point, Rect, Size};
use crate::host::{
    Canvas, Chrome, Clock, FontId, FontRole, FontStyle, Hint, IconRef, InputSource, Navigator,
    RowField, TextMetrics, ThemeMetric,
};
use crate::screen::Driver;
use crate::{Button, SwipeDir};

mod ops;
mod recorder;
mod snapshot;
#[cfg(not(target_os = "none"))]
mod ui;

pub use ops::{DrawOp, RectKind, RowCells, render};
pub use recorder::Recorder;
pub use snapshot::{assert_snapshot, assert_text_snapshot};
#[cfg(not(target_os = "none"))]
pub use ui::{Drive, Ui};

/// One recorded `draw_text`: position, text, font id and style.
pub type TextDraw = (i32, i32, String, i32, u8);

/// One recorded rectangle: x, y, width, height, and how it was painted.
pub type RectDraw = (i32, i32, i32, i32, RectKind);

/// Screen size the fake reports: a portrait e-reader panel.
pub const SCREEN_WIDTH: i32 = 480;
pub const SCREEN_HEIGHT: i32 = 800;

/// Chrome geometry, in the proportions the real themes use.
pub const TOP_PADDING: i32 = 8;
pub const HEADER_HEIGHT: i32 = 40;
pub const VERTICAL_SPACING: i32 = 12;
pub const BUTTON_HINTS_HEIGHT: i32 = 40;
pub const SIDE_PADDING: i32 = 16;
pub const MIN_TOUCH_SIZE: i32 = 44;
pub const LIST_ROW_HEIGHT: i32 = 40;
pub const LIST_ROW_HEIGHT_WITH_SUBTITLE: i32 = 56;
/// Gap between list rows, as a real theme leaves one.
pub const LIST_ROW_GAP: i32 = 4;
pub const PROGRESS_BAR_HEIGHT: i32 = 6;
/// Slider geometry the fake reports. Taken from a real backend's own slider
/// defaults rather than invented, so a test measuring a touch against the
/// track measures what a device would do.
pub const SLIDER_KNOB_WIDTH: i32 = 14;
pub const SLIDER_KNOB_HEIGHT: i32 = 22;
pub const SLIDER_SIDE_INSET: i32 = 8;
/// A heading's own line, and the theme's small step within a group.
pub const SUB_HEADER_HEIGHT: i32 = 17;
pub const SPACING_SMALL: i32 = 4;
pub const CONTENT_TOP: i32 = TOP_PADDING + HEADER_HEIGHT + VERTICAL_SPACING;
pub const CONTENT_BOTTOM: i32 = SCREEN_HEIGHT - BUTTON_HINTS_HEIGHT;

/// Font ids the fake hands out, and their metrics, indexed by role.
const UI_FONT: i32 = 1003;
const UI_SMALL_FONT: i32 = 1004;
const READER_FONT: i32 = 1001;

fn metrics_of(font: i32) -> (i32, i32) {
    // (line height, per-character advance)
    match font {
        UI_SMALL_FONT => (14, 5),
        READER_FONT => (19, 8),
        _ => (17, 6),
    }
}

/// Line height the fake reports for a font id.
pub fn line_height(font_id: i32) -> i32 {
    metrics_of(font_id).0
}

/// Width the fake reports for `text` in a font id.
pub fn text_width(text: &str, font_id: i32) -> i32 {
    text.chars().count() as i32 * metrics_of(font_id).1
}

/// The screen size, for `measure`.
pub fn screen() -> Size {
    Size::new(SCREEN_WIDTH, SCREEN_HEIGHT)
}

thread_local! {
    static OPS: RefCell<Vec<DrawOp>> = const { RefCell::new(Vec::new()) };
    static NOW: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
    static SWIPE: core::cell::Cell<SwipeDir> = const { core::cell::Cell::new(SwipeDir::None) };
    static PRESSED: core::cell::Cell<Option<Button>> = const { core::cell::Cell::new(None) };
    static SWIPE_MOVES_SELECTION: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    static FINISHES: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
    static UPDATES: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
    static PRESENTS: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
}

fn push(op: DrawOp) {
    OPS.with(|ops| ops.borrow_mut().push(op));
}

/// Forgets every recorded draw. Call at the start of each test.
pub fn reset() {
    OPS.with(|ops| ops.borrow_mut().clear());
    SWIPE.with(|swipe| swipe.set(SwipeDir::None));
    PRESSED.with(|pressed| pressed.set(None));
    SWIPE_MOVES_SELECTION.with(|flag| flag.set(false));
    FINISHES.with(|count| count.set(0));
    UPDATES.with(|count| count.set(0));
    PRESENTS.with(|count| count.set(0));
}

/// Everything drawn since the last [`reset`], in order.
pub fn ops_log() -> Vec<DrawOp> {
    OPS.with(|ops| ops.borrow().clone())
}

/// Every `draw_text` recorded since the last [`reset`].
pub fn drawn_text() -> Vec<TextDraw> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::Text {
                origin,
                text,
                font,
                style,
            } => Some((origin.x, origin.y, text, font, style as u8)),
            _ => None,
        })
        .collect()
}

/// Every rectangle recorded since the last [`reset`], in draw order.
pub fn drawn_rects() -> Vec<RectDraw> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::Rect { rect, kind, .. } => {
                Some((rect.x(), rect.y(), rect.width(), rect.height(), kind))
            }
            _ => None,
        })
        .collect()
}

/// The title passed to each `draw_header` since the last [`reset`]. `None` is a
/// header asked to draw no title at all, which on a real host paints an empty
/// band - the failure this records exists to catch.
pub fn drawn_headers() -> Vec<Option<String>> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::Header { title, .. } => Some(title),
            _ => None,
        })
        .collect()
}

/// Every list drawn since the last [`reset`], as `(rows, selected)`. `selected`
/// is `-1` when the theme was asked to highlight nothing — which is what a list
/// behind a dialog must report.
pub fn drawn_lists() -> Vec<(usize, i32)> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::List { rows, selected, .. } => Some((rows.len(), selected)),
            _ => None,
        })
        .collect()
}

/// The full cell contents of every list drawn since the last [`reset`].
pub fn drawn_list_rows() -> Vec<Vec<RowCells>> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::List { rows, .. } => Some(rows),
            _ => None,
        })
        .collect()
}

/// Every option dialog drawn since the last [`reset`], as
/// `(title, options, highlighted)`. The highlight is what the arrows move, so
/// this is how a test proves they are alive.
pub fn drawn_popups() -> Vec<(String, usize, i32)> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::OptionPopup {
                title,
                options,
                selected,
            } => Some((title, options.len(), selected)),
            _ => None,
        })
        .collect()
}

/// Which of the four button hints were asked for, per draw: `true` where the
/// slot carries a label, `false` where it was left blank.
pub fn drawn_hints() -> Vec<[bool; 4]> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            // `Hint::None` is the only one that reaches the host as an empty
            // label; `Hint::Standard` arrives as "your own label for this slot".
            DrawOp::Hints(slots) => Some(core::array::from_fn(|index| {
                !matches!(slots[index].as_deref(), Some(""))
            })),
            _ => None,
        })
        .collect()
}

/// Every sub-header (a `Section` title) drawn since the last [`reset`], with
/// the rect it was given. A section title is not focusable, so where it lands
/// is the only evidence that scrolling left it on screen.
pub fn drawn_sub_headers() -> Vec<(Rect, String)> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::SubHeader { rect, label, .. } => Some((rect, label)),
            _ => None,
        })
        .collect()
}

/// Every slider drawn since the last [`reset`], as `(rect, value, max)`. The
/// host owns the knob and track, so this is what a test can hold the widget to.
pub fn drawn_sliders() -> Vec<(Rect, i32, i32)> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::Slider { rect, value, max } => Some((rect, value, max)),
            _ => None,
        })
        .collect()
}

/// Every progress bar drawn since the last [`reset`], as
/// `(rect, current, total)`.
pub fn drawn_progress_bars() -> Vec<(Rect, u32, u32)> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::ProgressBar {
                rect,
                current,
                total,
            } => Some((rect, current, total)),
            _ => None,
        })
        .collect()
}

/// Every scroll indicator asked for since the last [`reset`], as
/// `(content, visible, offset)`.
pub fn drawn_indicators() -> Vec<(i32, i32, i32)> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::ScrollIndicator {
                content,
                visible,
                offset,
                ..
            } => Some((content, visible, offset)),
            _ => None,
        })
        .collect()
}

/// Every clip set or lifted since the last [`reset`], in order. `None` is a
/// lift - a scroll view must leave one behind, or the chrome drawn after it
/// would stay clipped away.
pub fn clips() -> Vec<Option<Rect>> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::Clip(rect) => Some(rect),
            _ => None,
        })
        .collect()
}

/// Reports one swipe to the next frame the runtime reads input, so navigation
/// can be tested without a finger. Cleared by [`reset`].
pub fn set_swipe(direction: SwipeDir) {
    SWIPE.with(|swipe| swipe.set(direction));
}

/// Reports one button press to the next frame the runtime reads input.
/// Cleared by [`reset`], and consumed when read, so it fires exactly once.
pub fn press(button: Button) {
    PRESSED.with(|pressed| pressed.set(Some(button)));
}

/// Chooses which way a swipe moves focus, so both readings can be tested.
/// See [`InputSource::swipe_moves_selection`](crate::host::InputSource::swipe_moves_selection).
pub fn set_swipe_moves_selection(enabled: bool) {
    SWIPE_MOVES_SELECTION.with(|flag| flag.set(enabled));
}

/// Moves the fake clock, so repeat timing is deterministic.
pub fn set_millis(value: u32) {
    NOW.with(|now| now.set(value));
}

/// The fake host.
pub struct TestHost;

static TEST_HOST: TestHost = TestHost;

/// Installs the fake. Idempotent, so every test may call it.
///
/// **Idempotent means it will not take the global back off another host.** A
/// test binary that installs a real backend — to render pixels — and then
/// calls this gets the backend, not the fake, and every `drawn_*` accessor
/// comes back empty while the assertions read as though the screen drew
/// nothing.
///
/// The fix is not a flag: put behaviour tests and pixel tests in *separate
/// files*. Cargo gives each integration test file its own process, which is
/// the only reliable way to keep two global hosts apart. [`install_forced`]
/// exists for the case where one process genuinely must swap back.
pub fn install() {
    if !crate::host::is_installed() {
        // Safety: tests are single-threaded per case and nothing has rendered.
        unsafe { crate::host::install(&TEST_HOST) };
    }
    if !crate::host::is_navigator_installed() {
        // Safety: as above.
        unsafe { crate::host::install_navigator(&TEST_HOST) };
    }
}

/// Installs the fake **over** whatever host is already there.
///
/// For a test binary that must alternate between the fake and a real backend
/// in one process. Prefer separate test files; this is the escape hatch, and
/// it is as unsafe as `install` for the same reason.
///
/// # Safety
/// Same contract as [`crate::host::install`]: before any frame runs, and never
/// concurrently with one.
pub unsafe fn install_forced() {
    // Safety: forwarded to the caller.
    unsafe { crate::host::install(&TEST_HOST) };
    if !crate::host::is_navigator_installed() {
        unsafe { crate::host::install_navigator(&TEST_HOST) };
    }
}

impl Canvas for TestHost {
    fn screen_size(&self) -> Size {
        screen()
    }

    fn clear(&self) {
        push(DrawOp::Clear);
    }

    fn draw_text(&self, origin: Point, text: &str, font: FontId, style: FontStyle) {
        if font.0 == 0 {
            return; // a font this build omitted draws nothing
        }
        push(DrawOp::Text {
            origin,
            text: text.to_string(),
            font: font.0,
            style,
        });
    }

    fn fill_rect(&self, rect: Rect, black: bool) {
        push(DrawOp::Rect {
            rect,
            kind: RectKind::Filled,
            black,
        });
    }

    fn stroke_rect(&self, rect: Rect) {
        push(DrawOp::Rect {
            rect,
            kind: RectKind::Stroked,
            black: true,
        });
    }

    fn draw_line(&self, from: Point, to: Point) {
        push(DrawOp::Line { from, to });
    }

    fn fill_rect_dither(&self, rect: Rect, light: bool) {
        push(DrawOp::Rect {
            rect,
            kind: RectKind::Dither,
            black: light,
        });
    }

    fn set_clip(&self, rect: Option<Rect>) {
        push(DrawOp::Clip(rect));
    }

    fn scrim(&self, rect: Rect) {
        push(DrawOp::Rect {
            rect,
            kind: RectKind::Scrim,
            black: true,
        });
    }

    fn draw_image(&self, origin: Point, data: &[u8], size: Size) {
        push(DrawOp::Image {
            origin,
            size,
            bytes: data.len(),
        });
    }

    fn draw_icon(&self, origin: Point, icon: IconRef) {
        push(DrawOp::Icon { origin, icon });
    }

    /// Reports the requested size, so icon layout is predictable.
    fn icon_size(&self, icon: IconRef) -> i32 {
        icon.size
    }
}

impl TextMetrics for TestHost {
    fn font(&self, role: FontRole) -> FontId {
        FontId(match role {
            FontRole::Ui => UI_FONT,
            FontRole::UiSmall => UI_SMALL_FONT,
            FontRole::Reader => READER_FONT,
        })
    }

    fn text_width(&self, font: FontId, text: &str, _style: FontStyle) -> i32 {
        if font.0 == 0 {
            return 0;
        }
        text_width(text, font.0)
    }

    fn line_height(&self, font: FontId) -> i32 {
        if font.0 == 0 {
            return 0;
        }
        line_height(font.0)
    }
}

impl Chrome for TestHost {
    fn metric(&self, metric: ThemeMetric) -> i32 {
        match metric {
            ThemeMetric::TopPadding => TOP_PADDING,
            ThemeMetric::HeaderHeight => HEADER_HEIGHT,
            ThemeMetric::VerticalSpacing => VERTICAL_SPACING,
            ThemeMetric::ButtonHintsHeight => BUTTON_HINTS_HEIGHT,
            ThemeMetric::ContentSidePadding => SIDE_PADDING,
            ThemeMetric::ContentTop => CONTENT_TOP,
            ThemeMetric::ContentBottom => CONTENT_BOTTOM,
            ThemeMetric::ListRowHeight => LIST_ROW_HEIGHT,
            ThemeMetric::ListRowHeightWithSubtitle => LIST_ROW_HEIGHT_WITH_SUBTITLE,
            ThemeMetric::ListRowGap => LIST_ROW_GAP,
            ThemeMetric::ProgressBarHeight => PROGRESS_BAR_HEIGHT,
            ThemeMetric::MinTouchSize => MIN_TOUCH_SIZE,
            ThemeMetric::SliderKnobWidth => SLIDER_KNOB_WIDTH,
            ThemeMetric::SliderKnobHeight => SLIDER_KNOB_HEIGHT,
            ThemeMetric::SliderSideInset => SLIDER_SIDE_INSET,
            ThemeMetric::SubHeaderHeight => SUB_HEADER_HEIGHT,
            ThemeMetric::SpacingSmall => SPACING_SMALL,
        }
    }

    fn draw_header(&self, title: Option<&str>, subtitle: Option<&str>) {
        push(DrawOp::Header {
            title: title.map(String::from),
            subtitle: subtitle.map(String::from),
        });
    }

    fn draw_sub_header(&self, rect: Rect, label: &str, right: Option<&str>) {
        push(DrawOp::SubHeader {
            rect,
            label: String::from(label),
            right: right.map(String::from),
        });
    }

    fn draw_button_hints(&self, back: &Hint, confirm: &Hint, prev: &Hint, next: &Hint) {
        let label = |hint: &Hint| hint.label().map(String::from);
        push(DrawOp::Hints([
            label(back),
            label(confirm),
            label(prev),
            label(next),
        ]));
    }

    fn draw_progress_bar(&self, rect: Rect, current: u32, total: u32) {
        push(DrawOp::ProgressBar {
            rect,
            current,
            total,
        });
    }

    fn draw_scroll_indicator(&self, rect: Rect, content: i32, visible: i32, offset: i32) {
        push(DrawOp::ScrollIndicator {
            rect,
            content,
            visible,
            offset,
        });
    }

    fn draw_slider(&self, rect: Rect, value: i32, max: i32) {
        push(DrawOp::Slider { rect, value, max });
    }

    fn draw_list<'a>(
        &self,
        rect: Rect,
        rows: usize,
        selected: i32,
        row: &dyn Fn(usize, RowField) -> Option<&'a str>,
    ) {
        // Ask for every cell now rather than storing the callback: the strings
        // it borrows live only as long as the widget that built them, and a
        // snapshot is read long after the frame has gone.
        const FIELDS: [RowField; 3] = [RowField::Title, RowField::Subtitle, RowField::Value];
        let cells = (0..rows)
            .map(|index| core::array::from_fn(|field| row(index, FIELDS[field]).map(String::from)))
            .collect();
        push(DrawOp::List {
            rect,
            selected,
            rows: cells,
        });
    }

    fn draw_option_popup<'a>(
        &self,
        title: &str,
        options: &dyn Fn(usize) -> Option<&'a str>,
        count: usize,
        selected: i32,
    ) {
        push(DrawOp::OptionPopup {
            title: String::from(title),
            selected,
            options: (0..count).map(|i| options(i).map(String::from)).collect(),
        });
    }

    /// Rows stacked from the middle of the screen, so hit-testing a modal is
    /// predictable without reproducing the real dialog's geometry.
    fn option_popup_row_rect<'a>(
        &self,
        _title: &str,
        _options: &dyn Fn(usize) -> Option<&'a str>,
        count: usize,
        index: usize,
    ) -> Option<Rect> {
        if index >= count {
            return None;
        }
        let top = SCREEN_HEIGHT / 4;
        Some(Rect::new(
            SIDE_PADDING,
            top + index as i32 * LIST_ROW_HEIGHT,
            SCREEN_WIDTH - SIDE_PADDING * 2,
            LIST_ROW_HEIGHT,
        ))
    }

    fn request_update(&self) {
        UPDATES.with(|count| count.set(count.get() + 1));
    }
}

/// The fake navigator: records what a screen asked for rather than doing it,
/// so a test can assert that Back actually reached the navigation.
impl Navigator for TestHost {
    fn screen_title(&self) -> &'static str {
        "Test"
    }

    fn finish(&self) {
        FINISHES.with(|count| count.set(count.get() + 1));
    }

    fn present(&self, screen: Box<dyn Driver>) -> Option<Box<dyn Driver>> {
        PRESENTS.with(|count| count.set(count.get() + 1));
        // Nothing here owns a stack, so the screen goes back to the caller —
        // the same answer a host whose navigation lives elsewhere gives.
        Some(screen)
    }
}

/// How many times a screen asked to be finished since the last [`reset`].
pub fn finishes() -> u32 {
    FINISHES.with(|count| count.get())
}

/// How many repaints were requested since the last [`reset`].
pub fn updates() -> u32 {
    UPDATES.with(|count| count.get())
}

/// How many screens were offered to the navigator since the last [`reset`].
pub fn presents() -> u32 {
    PRESENTS.with(|count| count.get())
}

/// No input by default. A test that needs a touch drives the view directly.
impl InputSource for TestHost {
    fn was_pressed(&self, button: Button) -> bool {
        // Consumed on read, so an injected press fires for exactly one frame —
        // the same edge behaviour a real input manager has.
        PRESSED.with(|pressed| {
            let matched = pressed.get() == Some(button);
            if matched {
                pressed.set(None);
            }
            matched
        })
    }

    fn is_pressed(&self, _button: Button) -> bool {
        false
    }

    fn was_released(&self, _button: Button) -> bool {
        false
    }

    fn has_touch(&self) -> bool {
        false
    }

    fn tap(&self) -> Option<Point> {
        None
    }

    fn touch_held(&self) -> Option<Point> {
        None
    }

    fn touch_released(&self) -> bool {
        false
    }

    fn swipe(&self) -> SwipeDir {
        // Consumed on read: a swipe is an edge event a real input manager
        // reports for one frame, and leaving it set makes it fire again on the
        // next.
        SWIPE.with(|swipe| {
            let direction = swipe.get();
            swipe.set(SwipeDir::None);
            direction
        })
    }

    fn was_back_gesture(&self) -> bool {
        false
    }

    fn swipe_moves_selection(&self) -> bool {
        SWIPE_MOVES_SELECTION.with(|flag| flag.get())
    }

    fn was_home_gesture(&self) -> bool {
        false
    }
}

impl Clock for TestHost {
    fn millis(&self) -> u32 {
        NOW.with(|now| now.get())
    }
}
