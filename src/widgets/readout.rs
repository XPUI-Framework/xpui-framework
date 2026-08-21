//! A value control's own name and number, and the line they sit on.
//!
//! Two jobs, and they belong together: the number is why the line exists. A
//! screen cannot supply that number while an edit is open — it is not told the
//! working value, and must not be — and it could not build one per frame
//! anyway, since `body()` runs on every paint and `format!` is off that path.
//! So the control formats it, into a buffer on the stack, and draws it on a
//! line it owns.

use crate::geometry::{Point, Rect};
use crate::host::{Font, Renderer, Theme, ThemeMetric};

/// The number a value control draws on the line above its track.
///
/// Built during `render`, used immediately, and dropped: it exists to turn an
/// `i32` into a `&str` that [`Renderer::draw_text`](crate::host::Renderer)
/// can take, with no heap between the two.
pub(crate) struct Readout {
    buf: [u8; Readout::CAPACITY],
    len: usize,
}

impl Readout {
    /// Eleven bytes for the widest `i32` — `-2147483648` — and five for a
    /// suffix, which covers `%`, `px`, `min` and anything else worth putting
    /// after a number on a panel this size.
    ///
    /// A suffix that does not fit loses whole characters from its end: a number
    /// missing its unit still reads, and a control that silently drew nothing
    /// would not. Whole characters because a `°` cut in half is not UTF-8, and
    /// [`as_str`](Readout::as_str) would refuse the lot.
    const CAPACITY: usize = 16;

    pub(crate) fn new(value: i32, suffix: &str) -> Self {
        let mut out = Readout {
            buf: [0; Readout::CAPACITY],
            len: 0,
        };

        // `unsigned_abs` rather than negating: `-i32::MIN` overflows, and this
        // is reached with whatever a screen last put in its model.
        // Ten digits is the widest `u32` this can hold: `i32::MIN.unsigned_abs()`
        // is 2147483648. Written out at least once, so zero renders as `0`
        // rather than as nothing.
        let mut magnitude = value.unsigned_abs();
        let mut digits = [0u8; 10];
        let mut count = 0;
        loop {
            digits[count] = b'0' + (magnitude % 10) as u8;
            magnitude /= 10;
            count += 1;
            if magnitude == 0 {
                break;
            }
        }

        if value < 0 {
            out.push(b'-');
        }
        while count > 0 {
            count -= 1;
            out.push(digits[count]);
        }
        // Whole characters only — see `CAPACITY`.
        for character in suffix.chars() {
            if out.len + character.len_utf8() > Readout::CAPACITY {
                break;
            }
            let mut encoded = [0u8; 4];
            for byte in character.encode_utf8(&mut encoded).as_bytes() {
                out.push(*byte);
            }
        }
        out
    }

    fn push(&mut self, byte: u8) {
        if self.len < Readout::CAPACITY {
            self.buf[self.len] = byte;
            self.len += 1;
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        // The digits are ASCII and the suffix is copied a whole character at a
        // time, so this cannot fail. Checked rather than asserted anyway: the
        // cost is a length comparison per frame, and the alternative is an
        // `unsafe` in a widget for nothing.
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

/// The font a readout is drawn in: the interface font, so a control's number
/// matches the value in a list row above or below it.
pub(crate) fn font() -> Font {
    Font::ui()
}

/// Draws `value` right-aligned against `rect`'s trailing edge.
///
/// Right-aligned because the number's own width changes with the value — `9%`
/// is narrower than `10%` — and a left-aligned one would move its last digit
/// every time it crossed a power of ten.
///
/// **Nothing has to be reserved for it.** The number sits on a line of its own
/// against the trailing edge, so its width is not taken out of anything. A
/// readout beside the track would have to be measured at the widest value in
/// the range instead, or the track would change length as the value crossed a
/// power of ten and an e-ink panel would repaint it.
pub(crate) fn draw(rect: Rect, value: i32, suffix: &str) {
    let font = font();
    let text = Readout::new(value, suffix);
    let x = rect.x() + rect.width() - font.text_width(text.as_str());
    let y = rect.y() + (rect.height() - font.line_height()) / 2;
    Renderer::draw_text(Point::new(x, y), text.as_str(), font.id(), font.style());
}

/// The line above a value control carrying its name and its number.
///
/// **Above the track rather than beside it.** A number at the trailing end
/// takes width from the track: around 40 columns, against the 288 a control
/// gets on a 296-wide panel and the 232 left between a stepper's two glyphs —
/// a seventh to a sixth of the room a value has to express itself in. It also
/// separates the number from the name, leaving a list of settings reading as a
/// column of anonymous tracks. This is the arrangement a settings screen writes
/// by hand. Putting it *in* the control is what makes the number live while an
/// edit is open, which a screen cannot do: it is not told the working value.
pub(crate) struct Header<'a> {
    pub(crate) title: Option<&'a str>,
    /// The value and its unit, when the control shows one.
    pub(crate) value: Option<(i32, &'static str)>,
}

impl Header<'_> {
    pub(crate) fn is_empty(&self) -> bool {
        self.title.is_none() && self.value.is_none()
    }

    /// What the header costs above the control, gap included, or zero.
    pub(crate) fn height(&self) -> i32 {
        if self.is_empty() {
            return 0;
        }
        font().line_height() + Theme::metric(ThemeMetric::SpacingSmall)
    }

    /// Draws into the top of `rect`: the name at the leading edge, the number
    /// at the trailing one.
    pub(crate) fn render(&self, rect: Rect) {
        if self.is_empty() {
            return;
        }
        let font = font();
        let line = Rect::new(rect.x(), rect.y(), rect.width(), font.line_height());
        if let Some(title) = self.title {
            Renderer::draw_text(
                Point::new(line.x(), line.y()),
                title,
                font.id(),
                font.style(),
            );
        }
        if let Some((value, suffix)) = self.value {
            draw(line, value, suffix);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::allocations::allocations;

    /// **The reason this type exists.** `body()` runs on every paint and on
    /// every frame carrying input, so a `format!` here would allocate several
    /// times a second while a key is held — on a device whose whole heap is a
    /// few tens of kilobytes — and drag `core::fmt` in with it.
    ///
    /// Scope is the formatting, not `render`: what a backend's `draw_text` does
    /// with the `&str` is the backend's business, and the recording double used
    /// by the rest of the suite allocates deliberately, to record.
    #[test]
    fn formatting_a_number_does_not_allocate() {
        // Every value in the range, so a carry into a new digit is covered: one
        // that reached for the heap only when the number grew a digit would
        // otherwise pass on whatever it was built with.
        let counted = allocations(|| {
            for value in 0..=100 {
                assert!(!Readout::new(value, "%").as_str().is_empty());
            }
            assert!(!Readout::new(i32::MIN, "px").as_str().is_empty());
        });
        assert_eq!(counted, 0, "a control's own number must not reach the heap");
    }

    #[test]
    fn it_renders_the_ends_of_the_range_and_a_carry() {
        for (value, suffix, expected) in [
            (0, "%", "0%"),
            (5, "%", "5%"),
            (9, "%", "9%"),
            (10, "%", "10%"),
            (99, "%", "99%"),
            (100, "%", "100%"),
            (7, "", "7"),
            (-1, "%", "-1%"),
        ] {
            assert_eq!(Readout::new(value, suffix).as_str(), expected);
        }
    }

    /// The two values that break a naive implementation: `0` falls out of a
    /// `while magnitude > 0` loop having written nothing, and `-i32::MIN`
    /// overflows a negation.
    #[test]
    fn it_survives_zero_and_the_smallest_integer() {
        assert_eq!(Readout::new(0, "").as_str(), "0");
        assert_eq!(Readout::new(i32::MIN, "").as_str(), "-2147483648");
        assert_eq!(Readout::new(i32::MAX, "").as_str(), "2147483647");
    }

    /// A suffix that would not fit loses its own tail, never the number.
    ///
    /// `-2147483648` is eleven characters, leaving five of the sixteen bytes,
    /// so `percent` arrives as `perce`. What matters is which end is cut: a number truncated to
    /// `-214748364` reads as a different number, and nothing on the panel would
    /// say so.
    #[test]
    fn a_number_keeps_its_digits_when_the_suffix_is_too_long() {
        let readout = Readout::new(i32::MIN, "percent");
        assert_eq!(readout.as_str(), "-2147483648perce");
    }

    /// And a multi-byte suffix is cut between characters, not through one.
    ///
    /// `°` is two bytes, and the widest number leaves five, so two whole
    /// degrees fit and the third does not — cut by byte instead and the buffer ends with half a
    /// character, `as_str` refuses the lot, and the control draws **nothing**:
    /// the number disappears because its unit did not fit.
    #[test]
    fn a_suffix_is_cut_between_characters() {
        assert_eq!(Readout::new(i32::MIN, "°°°").as_str(), "-2147483648°°");
        assert_eq!(Readout::new(0, "°C").as_str(), "0°C");
    }
}
