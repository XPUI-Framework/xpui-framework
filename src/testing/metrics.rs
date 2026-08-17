//! The numbers the fake answers with: panel size, chrome geometry and font
//! metrics, in the proportions a real theme uses.

use crate::geometry::Size;

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
pub(super) const UI_FONT: i32 = 1003;
pub(super) const UI_SMALL_FONT: i32 = 1004;
pub(super) const READER_FONT: i32 = 1001;

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
