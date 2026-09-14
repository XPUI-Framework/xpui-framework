//! A slider flanked by `-` and `+` steps.

use crate::geometry::{Point, Rect, Size};
use crate::host::{Theme, ThemeMetric};
use crate::layout::{Alignment, HStack, Modifiers};
use crate::view::{InputMask, Interactions, Trigger, View};
use crate::widgets::readout::Header;
use crate::widgets::{Slider, Text};
use alloc::string::String;

/// The row every adjustable setting uses: fine steps at each end, a draggable
/// track between them.
///
/// The glyphs are framed to a row-height square so they sit where the eye
/// expects, and the framework widens anything smaller to the theme's minimum
/// touch target — so a one-character `-` is still comfortably hittable.
///
/// ```rust
/// # use xpui::Stepper;
/// # #[derive(Clone, Copy)]
/// # enum Msg { Brightness(i32), BrightnessStep(i32) }
/// # xpui::testing::install();
/// # let brightness = 40;
/// Stepper::new(brightness)
///     .on_change(Msg::Brightness)    // dragged or tapped on the track
///     .on_step(Msg::BrightnessStep); // -1 or +1 from the end glyphs
/// ```
pub struct Stepper<M> {
    value: i32,
    max: i32,
    change: Option<fn(i32) -> M>,
    step: Option<fn(i32) -> M>,
    /// The unit shown after the number, when this stepper draws one at all.
    readout: Option<&'static str>,
    /// The name on the header line, when this stepper carries its own.
    title: Option<String>,
    row: Option<HStack<M>>,
    measured: Size,
}

impl<M: Clone + 'static> Stepper<M> {
    /// A stepper over 0..=100.
    pub fn new(value: i32) -> Self {
        Stepper::ranged(value, 100)
    }

    /// A stepper over 0..=`max`.
    pub fn ranged(value: i32, max: i32) -> Self {
        Stepper {
            value,
            max,
            change: None,
            step: None,
            readout: None,
            title: None,
            row: None,
            measured: Size::ZERO,
        }
    }

    /// Draws the value as a number on the line above the row, at its trailing
    /// edge — beside [`title`](Stepper::title) when there is one.
    ///
    /// **This is the only thing that shows the value while an edit is open.**
    /// The framework holds the value then and the screen is not told it, so a
    /// number a screen painted beside the control stands still while the track
    /// moves. See [`Slider::readout`](crate::Slider::readout).
    pub fn readout(mut self, suffix: &'static str) -> Self {
        self.readout = Some(suffix);
        self
    }

    /// Names the control on the same line as its number.
    ///
    /// See [`Slider::title`](crate::Slider::title).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sends `make(new_value)` when the track is dragged or tapped.
    ///
    /// It is also what an open edit commits, so a stepper without it can be
    /// nudged but never opened. Given no [`on_step`](Stepper::on_step), it
    /// drives the glyphs and the Left/Right keys as well, with the value one
    /// step either side, held inside the range.
    pub fn on_change(mut self, make: fn(i32) -> M) -> Self {
        self.change = Some(make);
        self
    }

    /// Sends `make(-1)` or `make(+1)` from the end glyphs and the Left/Right
    /// keys.
    ///
    /// Optional: for a screen that wants a relative nudge, whatever a step is
    /// worth to it. Without it, the glyphs and the keys send
    /// [`on_change`](Stepper::on_change) one step from the current value.
    pub fn on_step(mut self, make: fn(i32) -> M) -> Self {
        self.step = Some(make);
        self
    }

    /// Builds the row once, on first measure, so the tree is assembled with
    /// live theme metrics rather than at construction time.
    fn build(&mut self) {
        if self.row.is_some() {
            return;
        }

        let row_height = Theme::metric(ThemeMetric::ListRowHeight);
        let gap = Theme::metric(ThemeMetric::VerticalSpacing);

        let mut slider = Slider::new(self.value, self.max).without_focus();
        if let Some(make) = self.change {
            slider = slider.on_change(make);
        }

        let mut stack = HStack::new(gap);
        if let Some(minus) = self.nudge(-1) {
            // `Text` is a `View<M>` for every `M`, so the message type has to
            // be named before the modifier chain can resolve.
            // Touch-only: the row as a whole is the focus stop, so buttons
            // never land on a bare glyph.
            stack = stack.push(
                Modifiers::<M>::frame(Text::new("-"), row_height, row_height).on_touch(minus),
            );
        }
        stack = stack.push(slider.flexible());
        if let Some(plus) = self.nudge(1) {
            stack = stack
                .push(Modifiers::<M>::frame(Text::new("+"), row_height, row_height).on_touch(plus));
        }

        self.row = Some(stack.align(Alignment::Center));
    }
}

impl<M: Clone + 'static> Stepper<M> {
    /// What a glyph `delta` steps away sends: the nudge when there is one,
    /// otherwise the value that far along, held inside the range.
    fn nudge(&self, delta: i32) -> Option<M> {
        match (self.step, self.change) {
            (Some(step), _) => Some(step(delta)),
            (None, Some(change)) => Some(change(
                self.value.saturating_add(delta).clamp(0, self.max.max(0)),
            )),
            (None, None) => None,
        }
    }

    /// The one focus stop's trigger: relative when the screen asked for
    /// nudges, absolute otherwise, and `None` when nothing could change it.
    fn trigger(&self) -> Option<Trigger<M>> {
        match (self.step, self.change) {
            (Some(step), set) => Some(Trigger::Step {
                make: step,
                // The track's own setter, so an open edit can commit an
                // absolute value whatever a step is worth to the screen.
                set,
                max: self.max,
                value: self.value,
            }),
            // An absolute trigger is nudged by resolving `value ± 1` against
            // its range, and an edit commits through it, so the keys need
            // nothing more.
            (None, Some(make)) => Some(Trigger::Value {
                make,
                max: self.max,
                value: self.value,
            }),
            (None, None) => None,
        }
    }

    fn header(&self) -> Header<'_> {
        Header {
            title: self.title.as_deref(),
            value: self.readout.map(|suffix| (self.value, suffix)),
        }
    }

    /// The glyph row, below whatever the header took.
    fn row_bounds(&self, origin: Point) -> Rect {
        let top = self.header().height();
        Rect::new(
            origin.x,
            origin.y + top,
            self.measured.width,
            (self.measured.height - top).max(0),
        )
    }
}

impl<M: Clone + 'static> View<M> for Stepper<M> {
    fn measure(&mut self, available: Size) {
        self.build();
        let row = match &mut self.row {
            Some(row) => {
                row.measure(available);
                row.size()
            }
            None => Size::ZERO,
        };
        // Kept rather than recomputed on every `size()`: the header's height
        // is fixed once the control is built, and `row_bounds` reads the same
        // number `render` does.
        self.measured = Size::new(row.width, row.height + self.header().height());
    }

    fn size(&self) -> Size {
        self.measured
    }

    fn render(&self, origin: Point) {
        if self.measured.is_empty() {
            return;
        }
        self.header().render(self.bounds(origin));
        if let Some(row) = &self.row {
            row.render(self.row_bounds(origin).origin);
        }
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<M>) {
        // One focus stop for the whole control, adjusted by Left/Right. Without
        // this a stepper would be three stops - minus, track, plus - and Up/Down
        // would walk through glyphs instead of between settings.
        //
        // **Declared before the row**, so the track inside can be told whether
        // the keys are on it: an embedded slider takes no focus of its own and
        // has nothing else to learn it from. The order is free — the glyphs are
        // touch-only and this declaration takes no tap, so nothing else moves.
        let bounds = self.bounds(origin);
        let focused = self.trigger().is_some_and(|trigger| {
            out.declare(bounds, InputMask::FOCUS.union(InputMask::ADJUST), trigger)
        });

        // **The working value, before the row is walked.** The embedded track
        // reads it for itself, but the header's number is drawn by this control
        // and would otherwise show what the screen holds while the track shows
        // what the keys have moved it to — the two halves of one control
        // disagreeing, which is the whole fault this readout exists to fix.
        if focused && let Some(value) = out.editing_value() {
            self.value = value;
        }

        let row_origin = self.row_bounds(origin).origin;
        if let Some(row) = &mut self.row {
            let outer = out.parent_focused();
            out.set_parent_focused(focused);
            row.interactions(row_origin, out);
            out.set_parent_focused(outer);
        }
    }
}
