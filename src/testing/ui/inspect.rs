//! Reading a painted frame back: what it showed, and where a test may tap.

use alloc::string::String;
use alloc::vec::Vec;

use super::{Drive, Ui};
use crate::geometry::Rect;
use crate::host::{FontId, Host, Theme, ThemeMetric};
use crate::testing::ops::DrawOp;

impl<H: Host + Drive + 'static> Ui<H> {
    /// Every string the last painted frame actually showed, in draw order.
    ///
    /// Clipped-away text is left out: a `ScrollView` draws all of its content
    /// and lets the clip hide what does not fit, so "was drawn" and "can be
    /// seen" are different questions and only the second one is useful here.
    ///
    /// Each string appears once. A theme that paints rows through the
    /// framework's renderer re-enters the recorder, so a row arrives twice —
    /// as a cell of the list op and as the run that drew it — while a backend
    /// drawing in its own language reports it once. Deduping makes a panic
    /// message show the panel rather than the plumbing.
    pub fn visible_text(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for (label, _) in self.painted() {
            if !out.contains(&label) {
                out.push(label);
            }
        }
        out
    }

    /// Every label the frame showed, with where it was shown.
    ///
    /// The single place that walks the draw log, so what a test can *see* and
    /// what it can *tap* can never disagree.
    fn painted(&self) -> Vec<(String, Rect)> {
        Ui::<H>::labels_in(&self.frame)
    }

    /// The walk itself, over any run of draw calls.
    fn labels_in(ops: &[DrawOp]) -> Vec<(String, Rect)> {
        let mut out = Vec::new();
        let mut clip: Option<Rect> = None;

        for op in ops {
            match op {
                DrawOp::Clip(rect) => clip = *rect,
                DrawOp::Text {
                    origin,
                    text,
                    font,
                    style,
                } => {
                    // The font the run was drawn with, not a guess: a hint or a
                    // subtitle uses the small face, and measuring it with the
                    // body face overstates it by half again.
                    let id = FontId(*font);
                    let host = crate::host::current();
                    let height = host.line_height(id).max(1);
                    let width = host.text_width(id, text, *style).max(1);
                    // `draw_text` takes a top-left origin, not a baseline.
                    let rect = Rect::new(origin.x, origin.y, width, height);
                    if let Some(visible) = visible_part(rect, clip) {
                        out.push((text.clone(), visible));
                    }
                }
                DrawOp::Header { title, subtitle } => {
                    out.extend(
                        title
                            .iter()
                            .chain(subtitle.iter())
                            .map(|text| (text.clone(), Rect::new(0, 0, 0, 0))),
                    );
                }
                DrawOp::SubHeader { rect, label, right } => {
                    out.push((label.clone(), *rect));
                    out.extend(right.iter().map(|text| (text.clone(), *rect)));
                }
                DrawOp::List {
                    rect, rows: cells, ..
                } => {
                    // The widget strides by row height *plus* a gap while each
                    // row is only the height tall, so dividing the list
                    // rectangle evenly drifts further with every row.
                    let subtitled = cells.iter().any(|row| row[1].is_some());
                    let height = Theme::metric(if subtitled {
                        ThemeMetric::ListRowHeightWithSubtitle
                    } else {
                        ThemeMetric::ListRowHeight
                    });
                    let pitch = height + Theme::metric(ThemeMetric::ListRowGap);

                    for (index, row) in cells.iter().enumerate() {
                        let bounds = Rect::new(
                            rect.x(),
                            rect.y() + pitch * index as i32,
                            rect.width(),
                            height.max(1),
                        );
                        let Some(visible) = visible_part(bounds, clip) else {
                            continue;
                        };
                        out.extend(row.iter().flatten().map(|cell| (cell.clone(), visible)));
                    }
                }
                DrawOp::OptionPopup { title, options, .. } => {
                    // The theme owns a popup's geometry, so it is asked rather
                    // than guessed. Without this an option is visible and
                    // untappable: a test could open a picker and never choose
                    // anything from it.
                    let lookup = |index: usize| options.get(index).and_then(Option::as_deref);
                    out.push((title.clone(), Rect::new(0, 0, 0, 0)));

                    for (index, option) in options.iter().enumerate() {
                        let Some(text) = option else { continue };
                        let rect =
                            Theme::option_popup_row_rect(title, &lookup, options.len(), index)
                                .unwrap_or(Rect::new(0, 0, 0, 0));
                        out.push((text.clone(), rect));
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Every place `label` is painted, as something tappable.
    ///
    /// A glyph run is the truth: `Canvas::draw_text` takes a **top-left**
    /// origin, so the run's box starts there and is as tall as the font that
    /// drew it. Both matter — an earlier version subtracted a line height on
    /// the belief that the origin was a baseline, which put every rectangle in
    /// the blank space *above* the text it described.
    ///
    /// A list is also recorded as one op covering all its rows. That is used
    /// only when no run matched, because a theme that paints rows through the
    /// framework's own renderer re-enters this recorder and its runs are
    /// exact, while a row synthesised from the list rectangle is not: the
    /// widget strides by row height *plus* a gap, so dividing the rectangle
    /// evenly drifts further with every row.
    ///
    /// Anything clipped away is left out, and a partly visible row reports only
    /// the part you can see — a test must not tap what has been scrolled out of
    /// sight, because a person cannot.
    ///
    /// When a popup is up it takes the whole search. It captures input, so
    /// resolving to a row behind it would tap the scrim and dismiss the dialog
    /// while the test believed it had chosen something.
    pub fn rects_of_text(&self, label: &str) -> Vec<Rect> {
        // Too thin to hit: a sliver left by clipping is not something a person
        // can tap, and its centre is as likely to land in the row next door.
        let floor = Theme::metric(ThemeMetric::MinTouchSize) / 2;

        // A popup captures input, so once one is up nothing behind it can be
        // reached. Seeing and tapping part company here and only here: the
        // content behind a dialog is dimmed rather than hidden, so a person can
        // still read it — they just cannot touch it.
        let topmost = self
            .frame
            .iter()
            .rposition(|op| matches!(op, DrawOp::OptionPopup { .. }));

        let reachable: Vec<(String, Rect)> = match topmost {
            Some(index) => Ui::<H>::labels_in(&self.frame[index..]),
            None => self.painted(),
        };

        let mut out: Vec<Rect> = Vec::new();
        for (text, rect) in reachable {
            if text == label && rect.height() >= floor.max(2) && !out.contains(&rect) {
                out.push(rect);
            }
        }

        // A row and the label drawn inside it are one target, not two. Keep the
        // innermost, which is the rule the runtime resolves a touch by — and
        // tapping the middle of the text lands inside the row that owns it
        // either way, so the choice is safe as well as consistent.
        let nested: Vec<Rect> = out
            .iter()
            .copied()
            .filter(|outer| {
                out.iter()
                    .any(|inner| inner != outer && encloses(*outer, *inner))
            })
            .collect();
        out.retain(|rect| !nested.contains(rect));
        out
    }

    /// Where `label` is, when it is in exactly one place.
    ///
    /// `None` covers both "not on screen" and "on screen more than once", which
    /// is why [`tap_text`](Self::tap_text) reports the two differently.
    pub fn rect_of_text(&self, label: &str) -> Option<Rect> {
        match self.rects_of_text(label).as_slice() {
            [only] => Some(*only),
            _ => None,
        }
    }

    /// Whether the last action repainted, and painted something different.
    ///
    /// Compares draw calls rather than pixels: two frames can put the same
    /// amount of ink on the panel in different places, and an ink count calls
    /// those identical.
    pub fn changed(&self) -> bool {
        self.repainted && !self.previous.is_empty() && self.previous != self.frame
    }

    pub fn depth(&self) -> usize {
        self.app.depth()
    }

    pub fn is_running(&self) -> bool {
        self.app.is_running()
    }

    /// The draw calls of the last painted frame, for assertions this harness
    /// has no word for.
    pub fn frame(&self) -> &[DrawOp] {
        &self.frame
    }

    /// The host underneath, for reading pixels or anything else backend-shaped.
    pub fn host(&self) -> &'static H {
        self.host
    }
}

/// What is left of `rect` inside `clip`, or `None` if the clip removed it all.
fn visible_part(rect: Rect, clip: Option<Rect>) -> Option<Rect> {
    let Some(clip) = clip else { return Some(rect) };

    let left = rect.x().max(clip.x());
    let top = rect.y().max(clip.y());
    let right = (rect.x() + rect.width()).min(clip.x() + clip.width());
    let bottom = (rect.y() + rect.height()).min(clip.y() + clip.height());

    (right > left && bottom > top).then(|| Rect::new(left, top, right - left, bottom - top))
}

/// Whether `outer` completely contains `inner`.
fn encloses(outer: Rect, inner: Rect) -> bool {
    outer.x() <= inner.x()
        && outer.y() <= inner.y()
        && outer.x() + outer.width() >= inner.x() + inner.width()
        && outer.y() + outer.height() >= inner.y() + inner.height()
}
