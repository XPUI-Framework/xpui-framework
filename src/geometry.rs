//! Geometry primitives shared by layout and drawing.
//!
//! All coordinates are logical screen pixels in the current orientation — the
//! same space the host draws in. Never assume a size or a shape: the panels
//! this runs on are portrait and landscape, from 296x128 to 800x480, and a
//! screen that hardcodes either is wrong on most of them.

/// A position on screen.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Point {
    /// Pixels from the left edge.
    pub x: i32,
    /// Pixels from the top edge.
    pub y: i32,
}

impl Point {
    /// The top-left corner of the screen.
    pub const ORIGIN: Point = Point { x: 0, y: 0 };

    /// A point at `x`, `y`.
    pub const fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }

    /// This point moved by `dx`/`dy`.
    pub const fn offset(self, dx: i32, dy: i32) -> Self {
        Point::new(self.x + dx, self.y + dy)
    }
}

/// A width and height, clamped at zero by every constructor.
///
/// A layout that over-subtracts therefore cannot produce an inverted
/// rectangle. The fields are public, so a struct literal is not clamped: build
/// a size from computed numbers with [`Size::new`].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Size {
    /// Pixels across.
    pub width: i32,
    /// Pixels down.
    pub height: i32,
}

impl Size {
    /// Nothing at all.
    pub const ZERO: Size = Size {
        width: 0,
        height: 0,
    };

    /// A size of `width` by `height`, each clamped at zero.
    pub fn new(width: i32, height: i32) -> Self {
        Size {
            width: width.max(0),
            height: height.max(0),
        }
    }

    /// This size shrunk by `dw`/`dh`, clamped at zero.
    pub fn shrink(self, dw: i32, dh: i32) -> Self {
        Size::new(self.width - dw, self.height - dh)
    }

    /// Whether either dimension is zero, so nothing could be drawn in it.
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Space inset equally or individually on each edge.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Insets {
    /// Pixels taken from the top edge.
    pub top: i32,
    /// Pixels taken from the right edge.
    pub right: i32,
    /// Pixels taken from the bottom edge.
    pub bottom: i32,
    /// Pixels taken from the left edge.
    pub left: i32,
}

impl Insets {
    /// No inset on any edge.
    pub const ZERO: Insets = Insets {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    /// The same inset on all four edges.
    pub const fn all(value: i32) -> Self {
        Insets {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Independent horizontal and vertical insets.
    pub const fn symmetric(horizontal: i32, vertical: i32) -> Self {
        Insets {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Left and right together: the width an inset rect loses.
    pub const fn horizontal(&self) -> i32 {
        self.left + self.right
    }

    /// Top and bottom together: the height an inset rect loses.
    pub const fn vertical(&self) -> i32 {
        self.top + self.bottom
    }
}

/// A positioned, sized region.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    /// The top-left corner.
    pub origin: Point,
    /// The width and height.
    pub size: Size,
}

impl Rect {
    /// A rect with its top-left corner at `x`, `y`.
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Rect {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    /// The left edge.
    pub const fn x(&self) -> i32 {
        self.origin.x
    }

    /// The top edge.
    pub const fn y(&self) -> i32 {
        self.origin.y
    }

    /// Pixels across.
    pub const fn width(&self) -> i32 {
        self.size.width
    }

    /// Pixels down.
    pub const fn height(&self) -> i32 {
        self.size.height
    }

    /// The first column outside the rect.
    pub const fn right(&self) -> i32 {
        self.origin.x + self.size.width
    }

    /// The first row outside the rect.
    pub const fn bottom(&self) -> i32 {
        self.origin.y + self.size.height
    }

    /// This rect pulled inward on every edge by `insets`.
    pub fn inset(&self, insets: Insets) -> Self {
        Rect {
            origin: self.origin.offset(insets.left, insets.top),
            size: self.size.shrink(insets.horizontal(), insets.vertical()),
        }
    }

    /// Whether `point` is inside; the right and bottom edges are outside.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.y >= self.origin.y
            && point.x < self.right()
            && point.y < self.bottom()
    }

    /// Whether the two overlap at all.
    ///
    /// Touching edges do not count, matching [`contains`](Rect::contains),
    /// which treats the right and bottom edges as outside.
    pub fn intersects(&self, other: Rect) -> bool {
        self.origin.x < other.right()
            && other.origin.x < self.right()
            && self.origin.y < other.bottom()
            && other.origin.y < self.bottom()
    }
}
