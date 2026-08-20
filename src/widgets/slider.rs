//! A value slider.

use crate::geometry::{Point, Size};
use crate::host::{Theme, ThemeMetric};
use crate::view::{InputMask, Interactions, Trigger, View};

/// A horizontal slider showing `value` out of `max`.
///
/// Stateless by design: it draws the value it is given and never changes it.
/// The screen owns the value and adjusts it in
/// [`update`](crate::Screen::update), which is how the firmware's own interval
/// and frontlight screens work.
///
/// ```rust
/// # use xpui::Slider;
/// # xpui::testing::install();
/// # const MIN: i32 = 5;
/// # const MAX: i32 = 60;
/// # let minutes = 15;
/// # let _: Slider<()> =
/// Slider::new(minutes - MIN, MAX - MIN)
/// # ;
/// ```
///
/// Give it [`on_change`](Slider::on_change) and the framework converts a touch
/// into a value for you, applying the same rounding the C++ screens use.
///
/// **It is also a focus stop**: Up and Down reach it, and Left and Right move
/// it one step against its own bounds, so a board with no touchscreen can still
/// change it. [`without_focus`](Slider::without_focus) gives that up, for a
/// track inside a larger control that owns the stop itself.
pub struct Slider<M> {
    value: i32,
    max: i32,
    /// Built when a drag or tap lands on the track; `None` leaves the slider
    /// display-only.
    make: Option<fn(i32) -> M>,
    /// Set when this slider is the track of a larger control, which then owns
    /// the focus stop. See [`Slider::without_focus`].
    embedded: bool,
    measured: Size,
}

impl<M> Slider<M> {
    /// A slider at `value` of `max`. A `max` of zero renders empty rather than
    /// dividing by zero.
    pub fn new(value: i32, max: i32) -> Self {
        Slider {
            value,
            max,
            make: None,
            embedded: false,
            measured: Size::ZERO,
        }
    }

    /// A slider at `percent` of the way along.
    pub fn percent(percent: i32) -> Self {
        Slider::new(percent.clamp(0, 100), 100)
    }

    /// Reports drags and taps by building a message from the new value.
    ///
    /// The framework converts the touch position, so the screen never sees
    /// geometry:
    ///
    /// ```rust
    /// # use xpui::Slider;
    /// # #[derive(Clone, Copy)]
    /// # enum Msg { Brightness(i32) }
    /// # xpui::testing::install();
    /// # let brightness = 40;
    /// Slider::new(brightness, 100).on_change(Msg::Brightness);
    /// ```
    pub fn on_change(mut self, make: fn(i32) -> M) -> Self {
        self.make = Some(make);
        self
    }

    /// Drops this slider's focus stop, for a track inside a larger control that
    /// owns the stop itself.
    ///
    /// Keys are what a focus stop is for, so this drops the ability to be
    /// nudged with it.
    ///
    /// A [`Stepper`](crate::Stepper) is deliberately **one** focus stop and
    /// three touch targets: its two glyphs and the track it wraps. Without this
    /// the track would be a second stop inside it and Up/Down would stop twice
    /// on one row — which is the thing that made a stepper a single stop in the
    /// first place.
    ///
    /// Touch is unaffected: an embedded track still drags and still takes a
    /// tap, because it is still a place a finger can land.
    pub fn without_focus(mut self) -> Self {
        self.embedded = true;
        self
    }
}

impl<M> View<M> for Slider<M> {
    fn measure(&mut self, available: Size) {
        // One themed row tall, so a slider lines up with list rows beside it —
        // but never shorter than the knob the host will draw.
        let row = Theme::metric(ThemeMetric::ListRowHeight)
            .max(Theme::metric(ThemeMetric::SliderKnobHeight));
        self.measured = Size::new(available.width, row);
    }

    fn size(&self) -> Size {
        self.measured
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<M>) {
        let Some(make) = self.make else { return };

        // DRAG so the framework feeds held frames here and nowhere else, TAP so
        // a jab on the track jumps to that value.
        //
        // FOCUS and ADJUST unless something larger owns the stop. Without them
        // a slider is unreachable by any key, which on a device with no
        // touchscreen leaves it visible and impossible to move.
        let mut mask = InputMask::TAP.union(InputMask::DRAG);
        if !self.embedded {
            mask = mask.union(InputMask::FOCUS).union(InputMask::ADJUST);
        }

        out.declare(
            self.bounds(origin),
            mask,
            Trigger::Value {
                make,
                max: self.max,
                value: self.value,
            },
        );
    }

    fn render(&self, origin: Point) {
        if self.measured.is_empty() || self.max <= 0 {
            return;
        }

        // The host draws the track, the fill and the knob. It already owns that
        // geometry for its own screens, and deriving it again here is how the
        // two drift apart.
        Theme::draw_slider(self.bounds(origin), self.value, self.max);
    }
}
