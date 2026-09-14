//! Views onto the draw log, one per kind of thing a screen can draw.
//!
//! Every one of them reads [`ops_log`](super::ops_log), so "was a list drawn"
//! and "what did the frame look like" can never disagree.

use alloc::string::String;
use alloc::vec::Vec;

use super::ops::{DrawOp, RectKind, RowCells};
use super::state::ops_log;
use crate::geometry::Rect;

/// One recorded `draw_text`: position, text, font id and style.
pub type TextDraw = (i32, i32, String, i32, u8);

/// One recorded rectangle: x, y, width, height, and how it was painted.
pub type RectDraw = (i32, i32, i32, i32, RectKind);

/// Every `draw_text` recorded since the last [`reset`](super::reset).
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

/// Every rectangle recorded since the last [`reset`](super::reset), in draw
/// order.
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

/// The title passed to each `draw_header` since the last
/// [`reset`](super::reset).
///
/// `None` is a header asked to draw no title at all, which on a real host
/// paints an empty band — the failure this records exists to catch.
pub fn drawn_headers() -> Vec<Option<String>> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::Header { title, .. } => Some(title),
            _ => None,
        })
        .collect()
}

/// Every list drawn since the last [`reset`](super::reset), as
/// `(rows, selected)`.
///
/// `selected` is `-1` when the theme was asked to highlight
/// nothing — which is what a list behind a dialog must report.
pub fn drawn_lists() -> Vec<(usize, i32)> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::List { rows, selected, .. } => Some((rows.len(), selected)),
            _ => None,
        })
        .collect()
}

/// The full cell contents of every list drawn since the last
/// [`reset`](super::reset).
pub fn drawn_list_rows() -> Vec<Vec<RowCells>> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::List { rows, .. } => Some(rows),
            _ => None,
        })
        .collect()
}

/// Every option dialog drawn since the last [`reset`](super::reset), as
/// `(title, options, highlighted)`.
///
/// The highlight is what the arrows move, so
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

/// Every sub-header (a `Section` title) drawn since the last
/// [`reset`](super::reset), with the rect it was given.
///
/// A section title is not focusable, so where it lands is the only evidence
/// that scrolling left it on screen.
pub fn drawn_sub_headers() -> Vec<(Rect, String)> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::SubHeader { rect, label, .. } => Some((rect, label)),
            _ => None,
        })
        .collect()
}

/// Every slider drawn since the last [`reset`](super::reset), as
/// `(rect, value, max)`.
///
/// The host owns the knob and track, so this is what a
/// test can hold the widget to.
pub fn drawn_sliders() -> Vec<(Rect, i32, i32)> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::Slider {
                rect, value, max, ..
            } => Some((rect, value, max)),
            _ => None,
        })
        .collect()
}

/// Every progress bar drawn since the last [`reset`](super::reset), as
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

/// Every scroll indicator asked for since the last [`reset`](super::reset), as
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

/// Every clip set or lifted since the last [`reset`](super::reset), in order.
///
/// `None` is a lift: a scroll view must leave one behind, or the chrome drawn
/// after it would stay clipped away.
pub fn clips() -> Vec<Option<Rect>> {
    ops_log()
        .into_iter()
        .filter_map(|op| match op {
            DrawOp::Clip(rect) => Some(rect),
            _ => None,
        })
        .collect()
}
