//! Drawing primitives.

use crate::geometry::{Point, Rect, Size};
use crate::host::metrics::{FontId, FontStyle};

/// An icon the host owns.
///
/// Deliberately opaque numbers: which glyph these select is the host's
/// business. Naming them here would tie the framework to one product's assets.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IconRef {
    /// Which icon, in whatever numbering the host uses.
    pub kind: u16,
    /// A variant of the same icon, where the host offers one (solid vs outline).
    pub variant: u8,
    /// Preferred edge length; the host picks the nearest size it ships.
    pub size: i32,
}

impl IconRef {
    /// Icon `kind` in its first variant, asking for a 32-pixel edge.
    pub fn new(kind: u16) -> Self {
        IconRef {
            kind,
            variant: 0,
            size: 32,
        }
    }
}

/// The framebuffer, as the framework sees it.
pub trait Canvas {
    /// The panel's size in logical pixels, in the current orientation.
    fn screen_size(&self) -> Size;

    /// Clears to background.
    ///
    /// A screen painting over what is already there — an overlay — must not
    /// call this.
    fn clear(&self);

    /// Draws `text` with its **top-left** corner at `origin`.
    ///
    /// Not a baseline. Most drawing libraries take one, so this is the single
    /// obligation a new backend is most likely to get backwards, and getting it backwards
    /// puts every glyph one line too high while everything still compiles and
    /// every stack-depth test still passes. Convert on the way in: add the
    /// face's ascent, so the whole line occupies `[y, y + line_height)` and
    /// the rectangle the framework reserved is the rectangle you fill.
    ///
    /// [`TextMetrics::text_width`](crate::host::TextMetrics::text_width) must agree with what this paints, or a
    /// label reserved to fit will overrun the space it was given.
    fn draw_text(&self, origin: Point, text: &str, font: FontId, style: FontStyle);

    /// Fills `rect`; `black` false means background.
    fn fill_rect(&self, rect: Rect, black: bool);

    /// Outlines `rect` in ink, one pixel wide, inside its bounds.
    fn stroke_rect(&self, rect: Rect);

    /// A one-pixel line in ink from `from` to `to`, both ends included.
    fn draw_line(&self, from: Point, to: Point);

    /// Fills `rect` with a 50% dither, which reads as grey on a 1-bit panel.
    ///
    /// `light` picks which checkerboard parity takes ink, so two adjacent
    /// dithers can differ.
    fn fill_rect_dither(&self, rect: Rect, light: bool);

    /// Darkens `rect` while leaving what is already drawn there legible.
    ///
    /// Unlike [`fill_rect_dither`](Canvas::fill_rect_dither), which clears
    /// before it patterns, this only adds ink — on one checkerboard parity, so
    /// about half of what was behind survives. Use it to push a background
    /// back behind an overlay without repainting it.
    fn scrim(&self, rect: Rect);

    /// Confines drawing to `rect`, or lifts the clip when it is `None`.
    ///
    /// A view taller than the space it was given - a scrolling one - relies on
    /// this to keep its overflow off the chrome around it.
    ///
    /// One clip, not a stack: `Some` replaces whatever was set. Nesting is
    /// [`Renderer`]'s job, which always passes the whole clip in force, already
    /// intersected with every clip outside it.
    fn set_clip(&self, rect: Option<Rect>);

    /// Draws a 1-bpp bitmap of `size` with its top-left corner at `origin`.
    ///
    /// Row-major, MSB first, `(w + 7) / 8` bytes per row, and **bit 0 is ink** —
    /// inverted from the usual convention.
    fn draw_image(&self, origin: Point, data: &[u8], size: Size);

    /// Draws `icon` with its top-left corner at `origin`, at the size
    /// [`icon_size`](Canvas::icon_size) answers.
    fn draw_icon(&self, origin: Point, icon: IconRef);

    /// Edge length the host would actually draw, or 0 if it ships nothing for
    /// this icon.
    fn icon_size(&self, icon: IconRef) -> i32;
}

/// The framebuffer, as widgets reach for it.
///
/// A thin façade over the installed [`Canvas`] so a widget writes
/// `Renderer::fill_rect(..)` rather than plumbing a host reference through
/// every call.
pub struct Renderer;

impl Renderer {
    /// See [`Canvas::screen_size`].
    pub fn screen_size() -> Size {
        super::current().screen_size()
    }

    /// The whole panel, as a rect at the origin.
    pub fn screen_bounds() -> Rect {
        Rect {
            origin: Point::ORIGIN,
            size: Self::screen_size(),
        }
    }

    /// See [`Canvas::clear`].
    pub fn clear() {
        super::current().clear()
    }

    /// See [`Canvas::draw_text`].
    pub fn draw_text(origin: Point, text: &str, font: FontId, style: FontStyle) {
        super::current().draw_text(origin, text, font, style)
    }

    /// Confines drawing to `rect`, inside whatever clip is already set.
    ///
    /// Pair every call with [`Renderer::clear_clip`]. Clips nest: the host is
    /// given `rect` intersected with the clip in force, so a clipping view
    /// inside a scroll view cannot draw past either. A rect that misses the
    /// outer clip entirely confines drawing to nothing.
    ///
    /// Eight deep are remembered, on a fixed stack with no allocation. A clip
    /// past the eighth still applies, intersected with the eighth, and clearing
    /// it restores the eighth.
    pub fn clip(rect: Rect) {
        let depth = clips::depth();
        let effective = match depth.min(CLIP_DEPTH) {
            0 => rect,
            remembered => intersection(rect, clips::get(remembered - 1)),
        };
        if depth < CLIP_DEPTH {
            clips::put(depth, effective);
        }
        clips::set_depth(depth.saturating_add(1));
        super::current().set_clip(Some(effective))
    }

    /// Lifts the innermost clip, restoring the one it was set inside.
    ///
    /// With no clip outside it, the host's clip is lifted entirely. A call with
    /// no clip set lifts the host's clip and is otherwise ignored.
    pub fn clear_clip() {
        let depth = clips::depth().saturating_sub(1);
        clips::set_depth(depth);
        let outer = match depth.min(CLIP_DEPTH) {
            0 => None,
            remembered => Some(clips::get(remembered - 1)),
        };
        super::current().set_clip(outer)
    }

    /// See [`Canvas::fill_rect`].
    pub fn fill_rect(rect: Rect, black: bool) {
        super::current().fill_rect(rect, black)
    }

    /// See [`Canvas::stroke_rect`].
    pub fn stroke_rect(rect: Rect) {
        super::current().stroke_rect(rect)
    }

    /// See [`Canvas::draw_line`].
    pub fn draw_line(from: Point, to: Point) {
        super::current().draw_line(from, to)
    }

    /// See [`Canvas::fill_rect_dither`].
    pub fn fill_rect_dither(rect: Rect, light: bool) {
        super::current().fill_rect_dither(rect, light)
    }

    /// See [`Canvas::scrim`].
    pub fn scrim(rect: Rect) {
        super::current().scrim(rect)
    }

    /// See [`Canvas::draw_image`].
    pub fn draw_image(origin: Point, data: &[u8], size: Size) {
        super::current().draw_image(origin, data, size)
    }

    /// See [`Canvas::draw_icon`].
    pub fn draw_icon(origin: Point, icon: IconRef) {
        super::current().draw_icon(origin, icon)
    }

    /// See [`Canvas::icon_size`].
    pub fn icon_size(icon: IconRef) -> i32 {
        super::current().icon_size(icon)
    }
}

/// How many nested clips [`Renderer`] remembers.
const CLIP_DEPTH: usize = 8;

/// What two rects share; zero-sized where they do not meet.
fn intersection(a: Rect, b: Rect) -> Rect {
    let left = a.x().max(b.x());
    let top = a.y().max(b.y());
    let right = a.right().min(b.right());
    let bottom = a.bottom().min(b.bottom());
    Rect::new(left, top, (right - left).max(0), (bottom - top).max(0))
}

/// The clip stack on a device: one thread, one panel, fixed storage. Load and
/// store only, which both targets have.
#[cfg(not(all(any(test, feature = "testing"), not(target_os = "none"))))]
mod clips {
    use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

    use super::CLIP_DEPTH;
    use crate::geometry::Rect;

    static DEPTH: AtomicUsize = AtomicUsize::new(0);
    /// x, y, width and height of each remembered clip, in nesting order.
    static EDGES: [AtomicI32; CLIP_DEPTH * 4] = [const { AtomicI32::new(0) }; CLIP_DEPTH * 4];

    pub(super) fn depth() -> usize {
        DEPTH.load(Ordering::Relaxed)
    }

    pub(super) fn set_depth(depth: usize) {
        DEPTH.store(depth, Ordering::Relaxed);
    }

    pub(super) fn get(level: usize) -> Rect {
        let edge = |index: usize| EDGES[level * 4 + index].load(Ordering::Relaxed);
        Rect::new(edge(0), edge(1), edge(2), edge(3))
    }

    pub(super) fn put(level: usize, rect: Rect) {
        let values = [rect.x(), rect.y(), rect.width(), rect.height()];
        for (index, value) in values.into_iter().enumerate() {
            EDGES[level * 4 + index].store(value, Ordering::Relaxed);
        }
    }
}

/// **Thread-local under test**, for the reason `value_mode` gives: cases run
/// on parallel threads, and a clip one case set would confine another's frame.
#[cfg(all(any(test, feature = "testing"), not(target_os = "none")))]
mod clips {
    use core::cell::Cell;

    use super::CLIP_DEPTH;
    use crate::geometry::{Point, Rect, Size};

    const NONE: Rect = Rect {
        origin: Point::ORIGIN,
        size: Size::ZERO,
    };

    thread_local! {
        static DEPTH: Cell<usize> = const { Cell::new(0) };
        static STACK: Cell<[Rect; CLIP_DEPTH]> = const { Cell::new([NONE; CLIP_DEPTH]) };
    }

    pub(super) fn depth() -> usize {
        DEPTH.with(Cell::get)
    }

    pub(super) fn set_depth(depth: usize) {
        DEPTH.with(|cell| cell.set(depth));
    }

    pub(super) fn get(level: usize) -> Rect {
        STACK.with(|stack| stack.get()[level])
    }

    pub(super) fn put(level: usize, rect: Rect) {
        STACK.with(|stack| {
            let mut rects = stack.get();
            rects[level] = rect;
            stack.set(rects);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::Renderer;
    use crate::geometry::Rect;
    use crate::testing;

    #[test]
    fn an_inner_clip_stays_inside_the_outer_and_gives_it_back() {
        testing::install();
        testing::reset();
        let outer = Rect::new(0, 60, 480, 700);

        Renderer::clip(outer);
        Renderer::clip(Rect::new(16, 40, 200, 100));
        Renderer::clear_clip();
        Renderer::clear_clip();

        assert_eq!(
            testing::clips(),
            [
                Some(outer),
                Some(Rect::new(16, 60, 200, 80)),
                Some(outer),
                None
            ],
            "clearing the inner clip must restore the outer, not lift both"
        );
    }

    #[test]
    fn a_clip_outside_the_outer_confines_to_nothing() {
        testing::install();
        testing::reset();
        Renderer::clip(Rect::new(0, 0, 100, 100));
        Renderer::clip(Rect::new(200, 200, 50, 50));
        Renderer::clear_clip();
        Renderer::clear_clip();

        let inner = testing::clips()[1].expect("a clip, not a lift");
        assert_eq!((inner.width(), inner.height()), (0, 0));
    }

    #[test]
    fn a_clip_past_the_depth_stays_inside_the_deepest_remembered() {
        testing::install();
        testing::reset();
        let nested = |level: i32| Rect::new(level, level, 100 - 2 * level, 100 - 2 * level);
        for level in 0..10 {
            Renderer::clip(nested(level));
        }
        for _ in 0..10 {
            Renderer::clear_clip();
        }

        let clips = testing::clips();
        assert_eq!(
            &clips[..10],
            (0..10).map(|l| Some(nested(l))).collect::<Vec<_>>()
        );
        assert_eq!(
            clips[10],
            Some(nested(7)),
            "clearing the tenth restores the eighth: the ninth was never remembered"
        );
        assert_eq!(clips[11], Some(nested(7)), "clearing the ninth, the eighth");
        assert_eq!(
            clips[12],
            Some(nested(6)),
            "and below it, the stack is exact"
        );
        assert_eq!(clips[19], None);
    }

    #[test]
    fn an_unmatched_clear_lifts_the_clip_and_is_forgotten() {
        testing::install();
        testing::reset();
        let rect = Rect::new(0, 0, 50, 50);
        Renderer::clear_clip();
        Renderer::clip(rect);
        Renderer::clear_clip();
        assert_eq!(testing::clips(), [None, Some(rect), None]);
    }
}
