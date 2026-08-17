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

mod accessors;
mod chrome;
mod host;
mod metrics;
mod ops;
mod recorder;
mod snapshot;
mod state;
#[cfg(not(target_os = "none"))]
mod ui;

pub use accessors::{
    RectDraw, TextDraw, clips, drawn_headers, drawn_hints, drawn_indicators, drawn_list_rows,
    drawn_lists, drawn_popups, drawn_progress_bars, drawn_rects, drawn_sliders, drawn_sub_headers,
    drawn_text,
};
pub use host::{TestHost, install, install_forced};
pub use metrics::{
    BUTTON_HINTS_HEIGHT, CONTENT_BOTTOM, CONTENT_TOP, HEADER_HEIGHT, LIST_ROW_GAP, LIST_ROW_HEIGHT,
    LIST_ROW_HEIGHT_WITH_SUBTITLE, MIN_TOUCH_SIZE, PROGRESS_BAR_HEIGHT, SCREEN_HEIGHT,
    SCREEN_WIDTH, SIDE_PADDING, SLIDER_KNOB_HEIGHT, SLIDER_KNOB_WIDTH, SLIDER_SIDE_INSET,
    SPACING_SMALL, SUB_HEADER_HEIGHT, TOP_PADDING, VERTICAL_SPACING, line_height, screen,
    text_width,
};
pub use ops::{DrawOp, RectKind, RowCells, render};
pub use recorder::Recorder;
pub use snapshot::assert_snapshot;
pub use state::{
    finishes, ops_log, presents, press, reset, set_millis, set_swipe, set_swipe_moves_selection,
    updates,
};
#[cfg(not(target_os = "none"))]
pub use ui::{Drive, Ui};
