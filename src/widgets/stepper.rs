//! A slider flanked by `-` and `+` steps.

use crate::geometry::{Point, Size};
use crate::host::{Theme, ThemeMetric};
use crate::layout::{Alignment, HStack, Modifiers};
use crate::view::{InputMask, Interactions, Trigger, View};
use crate::widgets::{Slider, Text};

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
    row: Option<HStack<M>>,
}

impl<M: Clone + 'static> Stepper<M> {
    /// A stepper over 0..=100.
    pub fn new(value: i32) -> Self {
        Stepper::ranged(value, 100)
    }

    pub fn ranged(value: i32, max: i32) -> Self {
        Stepper {
            value,
            max,
            change: None,
            step: None,
            row: None,
        }
    }

    /// Sends `make(new_value)` when the track is dragged or tapped.
    pub fn on_change(mut self, make: fn(i32) -> M) -> Self {
        self.change = Some(make);
        self
    }

    /// Sends `make(-1)` or `make(+1)` from the end glyphs.
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
        if let Some(step) = self.step {
            // `Text` is a `View<M>` for every `M`, so the message type has to
            // be named before the modifier chain can resolve.
            // Touch-only: the row as a whole is the focus stop, so buttons
            // never land on a bare glyph.
            stack = stack.push(
                Modifiers::<M>::frame(Text::new("-"), row_height, row_height).on_touch(step(-1)),
            );
        }
        stack = stack.push(slider.flexible());
        if let Some(step) = self.step {
            stack = stack.push(
                Modifiers::<M>::frame(Text::new("+"), row_height, row_height).on_touch(step(1)),
            );
        }

        self.row = Some(stack.align(Alignment::Center));
    }
}

impl<M: Clone + 'static> View<M> for Stepper<M> {
    fn measure(&mut self, available: Size) {
        self.build();
        if let Some(row) = &mut self.row {
            row.measure(available);
        }
    }

    fn size(&self) -> Size {
        self.row.as_ref().map_or(Size::ZERO, |row| row.size())
    }

    fn render(&self, origin: Point) {
        if let Some(row) = &self.row {
            row.render(origin);
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
        let focused = self.step.is_some_and(|step| {
            out.declare(
                self.bounds(origin),
                InputMask::FOCUS.union(InputMask::ADJUST),
                Trigger::Step {
                    make: step,
                    // The track's own setter, so an open edit can commit an
                    // absolute value whatever a step is worth to the screen.
                    set: self.change,
                    max: self.max,
                    value: self.value,
                },
            )
        });

        if let Some(row) = &mut self.row {
            let outer = out.parent_focused();
            out.set_parent_focused(focused);
            row.interactions(origin, out);
            out.set_parent_focused(outer);
        }
    }
}
