//! The numbers the fake answers with: panel size, chrome geometry and font
//! metrics, in the proportions a real theme uses.

use crate::geometry::Size;

/// The width of the panel the fake reports: a portrait e-reader.
pub const SCREEN_WIDTH: i32 = 480;
/// The height of the panel the fake reports.
pub const SCREEN_HEIGHT: i32 = 800;

/// Gap above the header band.
pub const TOP_PADDING: i32 = 8;
/// Height of the header band.
pub const HEADER_HEIGHT: i32 = 40;
/// The gap between stacked elements.
pub const VERTICAL_SPACING: i32 = 12;
/// The band reserved at the bottom for the button hints.
pub const BUTTON_HINTS_HEIGHT: i32 = 40;
/// Space between the panel's side edges and the content.
pub const SIDE_PADDING: i32 = 16;
/// The smallest comfortably tappable dimension.
pub const MIN_TOUCH_SIZE: i32 = 44;
/// Height of a one-line list row.
pub const LIST_ROW_HEIGHT: i32 = 40;
/// Height of a list row carrying a subtitle.
pub const LIST_ROW_HEIGHT_WITH_SUBTITLE: i32 = 56;
/// Gap between list rows, as a real theme leaves one.
pub const LIST_ROW_GAP: i32 = 4;
/// Height of the progress bar.
pub const PROGRESS_BAR_HEIGHT: i32 = 6;
// Slider geometry, taken from a real backend's own defaults rather than
// invented, so a test measuring a touch against the track measures what a
// device would do.
/// The slider knob's width.
pub const SLIDER_KNOB_WIDTH: i32 = 14;
/// The slider knob's height.
pub const SLIDER_KNOB_HEIGHT: i32 = 22;
/// The padding a slider's track is inset by at each end.
pub const SLIDER_SIDE_INSET: i32 = 8;
/// A heading's own line.
pub const SUB_HEADER_HEIGHT: i32 = 17;
/// The theme's small step, within a group.
pub const SPACING_SMALL: i32 = 4;
/// First y below the header that content may use.
pub const CONTENT_TOP: i32 = TOP_PADDING + HEADER_HEIGHT + VERTICAL_SPACING;
/// First y occupied by the button hints.
pub const CONTENT_BOTTOM: i32 = SCREEN_HEIGHT - BUTTON_HINTS_HEIGHT;

// The font ids the fake hands out; `metrics_of` matches on them.
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
