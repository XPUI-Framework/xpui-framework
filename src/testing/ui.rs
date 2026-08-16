//! Driving a screen the way a person does.
//!
//! ```rust,ignore
//! let mut ui = Ui::new(Menu::new(), backend);
//! ui.tap_text("Lists");
//! assert!(ui.visible_text().iter().any(|line| line == "Rows, subtitles, values"));
//! ```
//!
//! Everything goes through [`App`] and the installed host, because that is
//! where the interesting failures are. A harness that drove the runtime
//! directly would have reported the arrow keys working for as long as they were
//! broken: the focus index moves correctly even when nothing reaches the panel.

use alloc::string::String;
use alloc::vec::Vec;
use std::sync::{Mutex, MutexGuard};

use super::ops::DrawOp;
use super::{Recorder, ops_log, reset};
use crate::app::App;
use crate::geometry::{Point, Rect};
use crate::host::{Button, FontId, Host, SwipeDir, Theme, ThemeMetric};
use crate::screen::Screen;

/// A host that a test can feed input to.
///
/// [`InputSource`](crate::host::InputSource) only reads; something has to write.
/// A backend already has these — this names them so the harness can reach them
/// without knowing which backend it is driving.
pub trait Drive {
    /// Starts a frame, clearing the previous frame's edges. Without this a
    /// press stays "just pressed" forever and every frame acts on it again.
    fn begin(&self, millis: u32);
    fn inject_press(&self, button: Button);
    fn inject_release(&self, button: Button);
    fn inject_tap(&self, point: Point);
    fn inject_swipe(&self, direction: SwipeDir);
}

/// Held for as long as a `Ui` exists, because it installs the process-wide
/// host. Two at once would race on the backend's `RefCell` and one of them
/// would panic "already borrowed" — which is a confusing way to learn about a
/// rule, so the type enforces it instead of documenting it.
static SERIAL: Mutex<()> = Mutex::new(());

/// A screen under test, with the app and host that drive it.
pub struct Ui<H: Host + Drive + 'static> {
    app: App,
    host: &'static H,
    millis: u32,
    frame: Vec<DrawOp>,
    previous: Vec<DrawOp>,
    /// Whether the *last* action repainted. Without this, an action that drew
    /// nothing kept reporting the previous action's answer.
    repainted: bool,
    /// Dropped last, releasing the lock when the test ends.
    _guard: MutexGuard<'static, ()>,
}

impl<H: Host + Drive + 'static> Ui<H> {
    /// Installs `host` behind a recorder, starts `screen`, and paints once.
    ///
    /// The first paint is not optional: the runtime ignores taps until it has
    /// painted, so without it the first tap of every test is swallowed and the
    /// screen looks broken for a reason that is not its fault.
    ///
    /// Only one `Ui` runs at a time. The rest of the test harness blocks here
    /// rather than racing, so tests in one file need no lock of their own.
    pub fn new<S: Screen + 'static>(screen: S, host: &'static H) -> Ui<H> {
        // A panicking test poisons the lock. Recover rather than cascading:
        // the next test installs its own host and starts from a clean log, so
        // there is no state left to be poisoned by.
        let guard = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let recorder = Recorder::wrap(host);
        // Safety: `guard` is held for this `Ui`'s whole life, so nothing else
        // is drawing through the installed host while this one replaces it.
        unsafe { crate::host::install(recorder) };

        reset();
        host.begin(0);
        let mut app = App::new(screen);
        app.render();

        Ui {
            app,
            host,
            millis: 0,
            frame: ops_log(),
            previous: Vec::new(),
            repainted: true,
            _guard: guard,
        }
    }

    // -- acting ------------------------------------------------------------

    /// Taps the middle of whatever control shows `label`.
    ///
    /// # Panics
    /// If nothing shows that text, listing what is on screen. A silent miss
    /// leaves a test passing because nothing happened.
    pub fn tap_text(&mut self, label: &str) -> &mut Self {
        let found = self.rects_of_text(label);
        let rect = match found.as_slice() {
            [only] => *only,
            [] => panic!(
                "nothing on screen shows {label:?}.\nVisible text: {:#?}",
                self.visible_text()
            ),
            many => panic!(
                "{label:?} is on screen {} times, at {many:?}. Say which one \
                 with `tap_nth_text`, because picking for you would make this \
                 test pass or fail depending on draw order.",
                many.len()
            ),
        };
        self.tap_at(Point::new(
            rect.x() + rect.width() / 2,
            rect.y() + rect.height() / 2,
        ))
    }

    /// Taps the `index`th place `label` appears, in draw order.
    ///
    /// # Panics
    /// If there are fewer than `index + 1` of them.
    pub fn tap_nth_text(&mut self, label: &str, index: usize) -> &mut Self {
        let found = self.rects_of_text(label);
        let rect = *found.get(index).unwrap_or_else(|| {
            panic!(
                "{label:?} appears {} times; asked for number {index}",
                found.len()
            )
        });
        self.tap_at(Point::new(
            rect.x() + rect.width() / 2,
            rect.y() + rect.height() / 2,
        ))
    }

    /// Taps a panel coordinate.
    pub fn tap_at(&mut self, point: Point) -> &mut Self {
        self.advance(|host| host.inject_tap(point))
    }

    pub fn press(&mut self, button: Button) -> &mut Self {
        self.advance(|host| {
            host.inject_press(button);
            host.inject_release(button);
        })
    }

    pub fn swipe(&mut self, direction: SwipeDir) -> &mut Self {
        self.advance(|host| host.inject_swipe(direction))
    }

    /// One frame: deliver the input, let the screen react, repaint if asked.
    fn advance(&mut self, feed: impl FnOnce(&H)) -> &mut Self {
        self.millis += 16;
        self.host.begin(self.millis);
        feed(self.host);

        reset();
        self.app.tick();
        self.repainted = self.app.render_if_dirty();
        if self.repainted {
            self.previous = core::mem::take(&mut self.frame);
            self.frame = ops_log();
        }
        self
    }

    // -- observing ---------------------------------------------------------

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
        let from = self
            .frame
            .iter()
            .rposition(|op| matches!(op, DrawOp::OptionPopup { .. }))
            .unwrap_or(0);

        let mut out = Vec::new();
        let mut clip: Option<Rect> = None;

        for op in &self.frame[from..] {
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
                    out.push((title.clone(), Rect::new(0, 0, 0, 0)));
                    out.extend(
                        options
                            .iter()
                            .flatten()
                            .map(|text| (text.clone(), Rect::new(0, 0, 0, 0))),
                    );
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

        let mut out: Vec<Rect> = Vec::new();
        for (text, rect) in self.painted() {
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
