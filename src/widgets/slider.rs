//! A value slider.

use crate::geometry::{Point, Rect, Size};
use crate::host::{ControlState, Theme, ThemeMetric};
use crate::view::{InputMask, Interactions, Trigger, View};
use crate::widgets::readout::Header;
use alloc::string::String;

/// A horizontal slider showing `value` out of `max`.
///
/// Stateless by design: it draws the value it is given and never changes it.
/// The screen owns the value and adjusts it in
/// [`update`](crate::Screen::update).
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
/// into a value for you.
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
    /// What the keys will do to it next, learned during the interactions walk
    /// and read back by `render` — the same way a `List` learns which of its
    /// rows holds focus. A view is rebuilt every frame, so this is never stale
    /// by more than the frame it was measured in.
    state: ControlState,
    /// The unit shown after the number, when this slider draws one at all.
    /// `Some("")` is a bare number; `None` is no readout.
    readout: Option<&'static str>,
    /// The name on the header line, when this slider carries its own.
    title: Option<String>,
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
            state: ControlState::Idle,
            readout: None,
            title: None,
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

    /// Draws the value as a number on a line above the track, at its trailing
    /// edge — beside [`title`](Slider::title) when there is one.
    ///
    /// **This is the only thing that shows the value while an edit is open.**
    /// The framework holds the value then and the screen is not told it, so a
    /// number a screen painted beside the control stands still while the knob
    /// moves; this one is drawn from the working copy.
    ///
    /// `suffix` is appended as given: `"%"`, `"px"`, or `""` for a bare number.
    /// `&'static` because it is copied into a fixed buffer beside the digits.
    ///
    /// ```rust
    /// # use xpui::Slider;
    /// # #[derive(Clone, Copy)]
    /// # enum Msg { Warmth(i32) }
    /// # xpui::testing::install();
    /// # let warmth = 25;
    /// Slider::new(warmth, 100).on_change(Msg::Warmth).readout("%");
    /// ```
    pub fn readout(mut self, suffix: &'static str) -> Self {
        self.readout = Some(suffix);
        self
    }

    /// Names the control on the same line as its number.
    ///
    /// The value on that line has to be the control's, or it stands still
    /// while an edit moves the track — so the line is the control's too.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Drops this slider's focus stop, for a track inside a larger control that
    /// owns the stop itself — and with it, being nudged by key.
    ///
    /// A [`Stepper`](crate::Stepper) is **one** focus stop and three touch
    /// targets: its two glyphs and the track it wraps. Without this the track
    /// would be a second stop and Up/Down would stop twice on one row. Touch
    /// is unaffected: an embedded track still drags and takes a tap.
    pub fn without_focus(mut self) -> Self {
        self.embedded = true;
        self
    }
}

impl<M> Slider<M> {
    fn header(&self) -> Header<'_> {
        Header {
            title: self.title.as_deref(),
            value: self.readout.map(|suffix| (self.value, suffix)),
        }
    }

    /// The track alone: the control's rect less whatever the header took.
    ///
    /// **Used for the touch region as well as the paint.** A drag is converted
    /// to a value against this rect, so a track laid out anywhere but here
    /// would put the knob where the finger was not.
    fn track_bounds(&self, origin: Point) -> Rect {
        let rect = self.bounds(origin);
        let top = self.header().height();
        Rect::new(
            rect.x(),
            rect.y() + top,
            rect.width(),
            (rect.height() - top).max(0),
        )
    }
}

impl<M> View<M> for Slider<M> {
    fn measure(&mut self, available: Size) {
        // One themed row tall, so a slider lines up with list rows beside it —
        // but never shorter than the knob the host will draw.
        let row = Theme::metric(ThemeMetric::ListRowHeight)
            .max(Theme::metric(ThemeMetric::SliderKnobHeight));
        self.measured = Size::new(available.width, row + self.header().height());
    }

    fn size(&self) -> Size {
        self.measured
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<M>) {
        let Some(make) = self.make else { return };

        // DRAG so held frames come here and nowhere else, TAP so a jab on the
        // track jumps to that value; FOCUS and ADJUST unless something larger
        // owns the stop, or no key can reach it.
        //
        // **Two rects, because a focus stop is not a touch target.** The
        // declared rect is what `Runtime` scrolls into view, so a stop covering
        // only the track would leave the name and number clipped above it
        // while the keys move the value. Touch stays on the track: a finger on
        // the name would set a value nobody asked for.
        let trigger = || Trigger::Value {
            make,
            max: self.max,
            value: self.value,
        };
        let focused = !self.embedded
            && out.declare(
                self.bounds(origin),
                InputMask::FOCUS.union(InputMask::ADJUST),
                trigger(),
            );
        out.declare(
            self.track_bounds(origin),
            InputMask::TAP.union(InputMask::DRAG),
            trigger(),
        );

        // An embedded track declares no focus of its own, so it asks the
        // control that owns the stop instead.
        let holds_focus = if self.embedded {
            out.parent_focused()
        } else {
            focused
        };
        // An open edit holds the value; the screen's has not moved and will
        // not until Confirm, so the knob paints from the edit's copy. Safe
        // after declaring: there is one focus, so a control that holds it
        // while an edit is open is the one the edit is on.
        if holds_focus && let Some(value) = out.editing_value() {
            self.value = value;
        }

        self.state = match (holds_focus, out.is_editing()) {
            (true, true) => ControlState::Editing,
            (true, false) => ControlState::Focused,
            (false, _) => ControlState::Idle,
        };
    }

    fn render(&self, origin: Point) {
        if self.measured.is_empty() {
            return;
        }

        // Drawn before the range is checked: a control with nothing to choose
        // between still has a name and a reading, and `measure` reserved the
        // line for them either way.
        self.header().render(self.bounds(origin));
        if self.max <= 0 {
            return;
        }

        // The host draws the track, the fill and the knob. It already owns that
        // geometry for its own screens, and deriving it again here is how the
        // two drift apart.
        Theme::draw_slider(self.track_bounds(origin), self.value, self.max, self.state);
    }
}
