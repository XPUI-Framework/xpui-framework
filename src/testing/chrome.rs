//! The chrome the fake reports: the metrics it answers with, and the
//! components it records instead of painting.

use alloc::string::String;

use super::host::TestHost;
use super::metrics::{
    BUTTON_HINTS_HEIGHT, CONTENT_BOTTOM, CONTENT_TOP, HEADER_HEIGHT, LIST_ROW_GAP, LIST_ROW_HEIGHT,
    LIST_ROW_HEIGHT_WITH_SUBTITLE, MIN_TOUCH_SIZE, PROGRESS_BAR_HEIGHT, SCREEN_HEIGHT,
    SCREEN_WIDTH, SIDE_PADDING, SLIDER_KNOB_HEIGHT, SLIDER_KNOB_WIDTH, SLIDER_SIDE_INSET,
    SPACING_SMALL, SUB_HEADER_HEIGHT, TOP_PADDING, VERTICAL_SPACING,
};
use super::ops::DrawOp;
use super::state::{UPDATES, push};
use crate::geometry::Rect;
use crate::host::{Chrome, ControlState, Hint, HintWord, RowField, ThemeMetric};

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
        // A word the host owns is recorded by *which* word, not as a bare
        // dash: `Standard` and `Edit` both leave the label to the host, and a
        // golden that showed them the same could not prove the bar changed.
        let label = |hint: &Hint| match hint.word() {
            HintWord::Standard => hint.label().map(String::from),
            HintWord::Edit => Some(String::from("<edit>")),
            HintWord::Done => Some(String::from("<done>")),
            HintWord::Cancel => Some(String::from("<cancel>")),
        };
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

    fn draw_slider(&self, rect: Rect, value: i32, max: i32, state: ControlState) {
        push(DrawOp::Slider {
            rect,
            value,
            max,
            state,
        });
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
