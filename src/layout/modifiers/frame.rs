//! Gives a view a fixed size, centring it in the space that makes.

use crate::geometry::{Point, Size};
use crate::view::{Interactions, View};

/// Gives a view a fixed size, centring it in the space that makes.
///
/// A "−" glyph is a few pixels wide, but the control it stands for is a
/// row-height square. Framing it keeps the glyph where the eye expects it, and
/// gives [`Tappable`](crate::Tappable) a sensible rect to grow from.
///
/// ```rust
/// # use xpui::{Icon, IconRef, Modifiers, Text};
/// # /// What a backend publishes; the framework only ever sees the number.
/// # #[derive(Copy, Clone)]
/// # enum Glyph { Sun }
/// # impl From<Glyph> for IconRef {
/// #     fn from(glyph: Glyph) -> IconRef { IconRef::new(glyph as u16) }
/// # }
/// # xpui::testing::install();
/// # let row_height = 44;
/// Modifiers::<()>::frame(Text::new("-"), row_height, row_height); // a square tap zone
/// Modifiers::<()>::frame(Icon::new(Glyph::Sun), 0, row_height);   // 0 keeps that axis natural
/// ```
///
/// `Text` and `Icon` are views for *every* message type, so a frame built from
/// a bare one has to name which is meant, inside a stack as much as outside
/// one: `Frame` does not mention the message type, so nothing infers it.
pub struct Frame<V> {
    child: V,
    width: Option<i32>,
    height: Option<i32>,
    measured: Size,
}

impl<V> Frame<V> {
    /// `child` with neither dimension fixed yet.
    pub fn new(child: V) -> Self {
        Frame {
            child,
            width: None,
            height: None,
            measured: Size::ZERO,
        }
    }

    /// Fixes the width, or leaves it natural when `width` is zero or less.
    pub fn width(mut self, width: i32) -> Self {
        self.width = (width > 0).then_some(width);
        self
    }

    /// Fixes the height, or leaves it natural when `height` is zero or less.
    pub fn height(mut self, height: i32) -> Self {
        self.height = (height > 0).then_some(height);
        self
    }

    /// Where the child sits once centred. Shared by `render` and
    /// `interactions`, so the two cannot drift.
    fn child_origin(&self, origin: Point, child: Size) -> Point {
        origin.offset(
            (self.measured.width - child.width).max(0) / 2,
            (self.measured.height - child.height).max(0) / 2,
        )
    }
}

impl<V: View<M>, M> View<M> for Frame<V> {
    fn measure(&mut self, available: Size) {
        // A fixed axis is what the child is offered, so a flexible child fills
        // the frame rather than overflowing it.
        self.child.measure(Size::new(
            self.width.unwrap_or(available.width),
            self.height.unwrap_or(available.height),
        ));

        let child = self.child.size();
        self.measured = Size::new(
            self.width.unwrap_or(child.width).max(child.width),
            self.height.unwrap_or(child.height).max(child.height),
        );
    }

    fn size(&self) -> Size {
        self.measured
    }

    fn render(&self, origin: Point) {
        self.child
            .render(self.child_origin(origin, self.child.size()));
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<M>) {
        let at = self.child_origin(origin, self.child.size());
        self.child.interactions(at, out);
    }

    /// A frame states a size deliberately, so it counts even around a spacer.
    fn contributes_cross_size(&self) -> bool {
        true
    }
}
