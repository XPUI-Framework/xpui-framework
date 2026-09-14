//! A host that writes down what was drawn on its way through to a real one.
//!
//! The fake [`TestHost`](super::TestHost) records every draw call, which is what
//! makes the snapshot tests possible — but its geometry is fixed at 480x800 with
//! hardcoded metrics, so it cannot stand in for a 296x128 panel.
//!
//! A UI test needs both: the real backend's layout, because that is what decides
//! where a control ends up, *and* a record of what was drawn, because that is
//! how a test finds the control it wants to tap. This wraps one to get the other.
//!
//! ```rust
//! # use xpui::host::Host;
//! # use xpui::testing::Recorder;
//! /// `backend` is the real host, already sized for the panel under test.
//! fn install_recording<H: Host + 'static>(backend: &'static H) {
//!     let recorded = Recorder::wrap(backend);
//!     // Safety: before the first frame, and never concurrently with one.
//!     unsafe { xpui::host::install(recorded) };
//! }
//! ```
//!
//! Everything is forwarded. Nothing is answered from the recording, so a screen
//! measures, lays out and paints exactly as it would without this in the way.
//! A repaint request is counted by [`updates`](super::updates) on its way
//! through, as the fake counts its own.

use alloc::boxed::Box;
use alloc::string::ToString;

use alloc::string::String;
use alloc::vec::Vec;

use super::ops::{DrawOp, RectKind, RowCells};
use super::state::{UPDATES, push};
use crate::geometry::{Point, Rect, Size};
use crate::host::{
    Button, Canvas, Chrome, Clock, ControlState, FontId, FontRole, FontStyle, Hint, HintWord, Host,
    IconRef, InputSource, RowField, SwipeDir, TextMetrics, ThemeMetric,
};

/// Wraps a host, recording every draw call before passing it on.
pub struct Recorder<H: Host + 'static> {
    inner: &'static H,
}

impl<H: Host + 'static> Recorder<H> {
    /// Wraps `inner` and leaks the wrapper, because a host must be `'static`.
    ///
    /// Leaked rather than borrowed for the same reason the backends are: the
    /// installed host outlives every screen that could draw through it, and a
    /// test process is going to exit anyway.
    pub fn wrap(inner: &'static H) -> &'static Recorder<H> {
        Box::leak(Box::new(Recorder { inner }))
    }

    /// The host underneath, for anything this wrapper does not model — reading
    /// the framebuffer, say.
    pub fn inner(&self) -> &'static H {
        self.inner
    }
}

impl<H: Host + 'static> Canvas for Recorder<H> {
    fn screen_size(&self) -> Size {
        self.inner.screen_size()
    }

    fn clear(&self) {
        push(DrawOp::Clear);
        self.inner.clear();
    }

    fn draw_text(&self, origin: Point, text: &str, font: FontId, style: FontStyle) {
        // A font this build omitted draws nothing, and recording it would put
        // text in the log that never reached the panel — which is exactly the
        // sort of thing a UI test would then try to tap.
        if font.0 != 0 {
            push(DrawOp::Text {
                origin,
                text: text.to_string(),
                font: font.0,
                style,
            });
        }
        self.inner.draw_text(origin, text, font, style);
    }

    fn fill_rect(&self, rect: Rect, black: bool) {
        push(DrawOp::Rect {
            rect,
            kind: RectKind::Filled,
            black,
        });
        self.inner.fill_rect(rect, black);
    }

    fn stroke_rect(&self, rect: Rect) {
        push(DrawOp::Rect {
            rect,
            kind: RectKind::Stroked,
            black: true,
        });
        self.inner.stroke_rect(rect);
    }

    fn draw_line(&self, from: Point, to: Point) {
        push(DrawOp::Line { from, to });
        self.inner.draw_line(from, to);
    }

    fn fill_rect_dither(&self, rect: Rect, light: bool) {
        push(DrawOp::Rect {
            rect,
            kind: RectKind::Dither,
            // `black` carries `light` for a dither, as the fake records it.
            black: light,
        });
        self.inner.fill_rect_dither(rect, light);
    }

    fn scrim(&self, rect: Rect) {
        push(DrawOp::Rect {
            rect,
            kind: RectKind::Scrim,
            black: true,
        });
        self.inner.scrim(rect);
    }

    fn set_clip(&self, rect: Option<Rect>) {
        push(DrawOp::Clip(rect));
        self.inner.set_clip(rect);
    }

    fn draw_image(&self, origin: Point, data: &[u8], size: Size) {
        push(DrawOp::Image {
            origin,
            size,
            bytes: data.len(),
        });
        self.inner.draw_image(origin, data, size);
    }

    fn draw_icon(&self, origin: Point, icon: IconRef) {
        push(DrawOp::Icon { origin, icon });
        self.inner.draw_icon(origin, icon);
    }

    fn icon_size(&self, icon: IconRef) -> i32 {
        self.inner.icon_size(icon)
    }
}

impl<H: Host + 'static> TextMetrics for Recorder<H> {
    fn font(&self, role: FontRole) -> FontId {
        self.inner.font(role)
    }

    fn text_width(&self, font: FontId, text: &str, style: FontStyle) -> i32 {
        self.inner.text_width(font, text, style)
    }

    fn line_height(&self, font: FontId) -> i32 {
        self.inner.line_height(font)
    }
}

impl<H: Host + 'static> Chrome for Recorder<H> {
    fn metric(&self, metric: ThemeMetric) -> i32 {
        self.inner.metric(metric)
    }

    fn draw_header(&self, title: Option<&str>, subtitle: Option<&str>) {
        push(DrawOp::Header {
            title: title.map(ToString::to_string),
            subtitle: subtitle.map(ToString::to_string),
        });
        self.inner.draw_header(title, subtitle);
    }

    fn draw_sub_header(&self, rect: Rect, label: &str, right_label: Option<&str>) {
        push(DrawOp::SubHeader {
            rect,
            label: label.to_string(),
            right: right_label.map(ToString::to_string),
        });
        self.inner.draw_sub_header(rect, label, right_label);
    }

    fn draw_button_hints(&self, back: &Hint, confirm: &Hint, previous: &Hint, next: &Hint) {
        // Recorded by which word the slot asked for, so a golden can tell the
        // host's usual label from the one a mode wants.
        let label = |hint: &Hint| match hint.word() {
            HintWord::Standard => hint.label().map(String::from),
            HintWord::Edit => Some(String::from("<edit>")),
            HintWord::Done => Some(String::from("<done>")),
            HintWord::Cancel => Some(String::from("<cancel>")),
        };
        push(DrawOp::Hints([
            label(back),
            label(confirm),
            label(previous),
            label(next),
        ]));
        self.inner.draw_button_hints(back, confirm, previous, next);
    }

    fn draw_progress_bar(&self, rect: Rect, current: u32, total: u32) {
        push(DrawOp::ProgressBar {
            rect,
            current,
            total,
        });
        self.inner.draw_progress_bar(rect, current, total);
    }

    fn draw_slider(&self, rect: Rect, value: i32, max: i32, state: ControlState) {
        push(DrawOp::Slider {
            rect,
            value,
            max,
            state,
        });
        self.inner.draw_slider(rect, value, max, state);
    }

    fn draw_scroll_indicator(&self, rect: Rect, content: i32, visible: i32, offset: i32) {
        push(DrawOp::ScrollIndicator {
            rect,
            content,
            visible,
            offset,
        });
        self.inner
            .draw_scroll_indicator(rect, content, visible, offset);
    }

    fn draw_list<'a>(
        &self,
        rect: Rect,
        rows: usize,
        selected: i32,
        row: &dyn Fn(usize, RowField) -> Option<&'a str>,
    ) {
        // Every cell is read now rather than storing the callback: the strings
        // it borrows live only as long as the widget that built them.
        const FIELDS: [RowField; 3] = [RowField::Title, RowField::Subtitle, RowField::Value];
        let cells: Vec<RowCells> = (0..rows)
            .map(|index| core::array::from_fn(|field| row(index, FIELDS[field]).map(String::from)))
            .collect();
        push(DrawOp::List {
            rect,
            selected,
            rows: cells,
        });
        self.inner.draw_list(rect, rows, selected, row);
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
            options: (0..count)
                .map(|index| options(index).map(String::from))
                .collect(),
        });
        self.inner
            .draw_option_popup(title, options, count, selected);
    }

    fn option_popup_row_rect<'a>(
        &self,
        title: &str,
        options: &dyn Fn(usize) -> Option<&'a str>,
        count: usize,
        index: usize,
    ) -> Option<Rect> {
        self.inner
            .option_popup_row_rect(title, options, count, index)
    }

    fn request_update(&self) {
        UPDATES.with(|count| count.set(count.get() + 1));
        self.inner.request_update();
    }
}

impl<H: Host + 'static> InputSource for Recorder<H> {
    fn was_pressed(&self, button: Button) -> bool {
        self.inner.was_pressed(button)
    }

    fn is_pressed(&self, button: Button) -> bool {
        self.inner.is_pressed(button)
    }

    fn was_released(&self, button: Button) -> bool {
        self.inner.was_released(button)
    }

    fn has_touch(&self) -> bool {
        self.inner.has_touch()
    }

    fn tap(&self) -> Option<Point> {
        self.inner.tap()
    }

    fn touch_held(&self) -> Option<Point> {
        self.inner.touch_held()
    }

    fn touch_released(&self) -> bool {
        self.inner.touch_released()
    }

    fn swipe(&self) -> SwipeDir {
        self.inner.swipe()
    }

    fn was_back_gesture(&self) -> bool {
        self.inner.was_back_gesture()
    }

    fn was_home_gesture(&self) -> bool {
        self.inner.was_home_gesture()
    }

    fn has_left_right_keys(&self) -> bool {
        self.inner.has_left_right_keys()
    }

    fn swipe_moves_selection(&self) -> bool {
        self.inner.swipe_moves_selection()
    }
}

impl<H: Host + 'static> Clock for Recorder<H> {
    fn millis(&self) -> u32 {
        self.inner.millis()
    }
}
