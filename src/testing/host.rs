//! The fake host itself: how it is installed, what it paints into the log, and
//! the input and clock it reports.

use alloc::boxed::Box;
use alloc::string::ToString;

use super::metrics::{READER_FONT, UI_FONT, UI_SMALL_FONT, line_height, screen, text_width};
use super::ops::{DrawOp, RectKind};
use super::state::{FINISHES, NOW, PRESENTS, PRESSED, SWIPE, SWIPE_MOVES_SELECTION, push};
use crate::geometry::{Point, Rect, Size};
use crate::host::{
    Canvas, Clock, FontId, FontRole, FontStyle, IconRef, InputSource, Navigator, TextMetrics,
};
use crate::screen::Driver;
use crate::{Button, SwipeDir};

/// The fake host.
pub struct TestHost;

pub(super) static TEST_HOST: TestHost = TestHost;

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
