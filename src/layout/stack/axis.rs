//! Which way a stack lays out, and where children sit across it.

use crate::geometry::{Point, Size};

/// Where children sit across the stacking axis.
///
/// An `HStack` mixing a 32px icon with a line of text looks wrong left-aligned:
/// the text hangs off the top. `Center` is what a row of mismatched controls
/// almost always wants; `Start` is the default.
///
/// ```rust
/// # use xpui::{Alignment, HStack, Icon, IconRef, Spacer, Text, hstack};
/// # xpui::testing::install();
/// # let gap = 8;
/// # let label = Text::new("Frontlight");
/// # let icon = Icon::new(IconRef::new(0)).size(32);
/// # let _: HStack<()> =
/// hstack![gap; label, Spacer::new(), icon].align(Alignment::Center)
/// # ;
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
}

/// The direction a stack lays children out in.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum Axis {
    Vertical,
    Horizontal,
}

impl Axis {
    /// The extent along the stacking direction.
    pub(super) fn main(self, size: Size) -> i32 {
        match self {
            Axis::Vertical => size.height,
            Axis::Horizontal => size.width,
        }
    }

    /// The extent across the stacking direction.
    pub(super) fn cross(self, size: Size) -> i32 {
        match self {
            Axis::Vertical => size.width,
            Axis::Horizontal => size.height,
        }
    }

    pub(super) fn size(self, main: i32, cross: i32) -> Size {
        match self {
            Axis::Vertical => Size::new(cross, main),
            Axis::Horizontal => Size::new(main, cross),
        }
    }

    pub(super) fn advance(self, origin: Point, amount: i32) -> Point {
        match self {
            Axis::Vertical => origin.offset(0, amount),
            Axis::Horizontal => origin.offset(amount, 0),
        }
    }

    /// Moves across the stacking axis, for alignment.
    pub(super) fn advance_cross(self, origin: Point, amount: i32) -> Point {
        match self {
            Axis::Vertical => origin.offset(amount, 0),
            Axis::Horizontal => origin.offset(0, amount),
        }
    }
}
