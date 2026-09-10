//! A horizontal rule.

use crate::geometry::{Point, Size};
use crate::host::Renderer;
use crate::view::View;

/// A one-pixel line spanning the width it is given, for separating sections.
///
/// ```rust
/// # use xpui::{Divider, Text, VStack, vstack};
/// # xpui::testing::install();
/// # let _: VStack<()> =
/// vstack![8; Text::new("Wi-Fi"), Divider::new(), Text::new("Bluetooth")]
/// # ;
/// ```
pub struct Divider {
    measured: Size,
}

impl Default for Divider {
    fn default() -> Self {
        Divider::new()
    }
}

impl Divider {
    /// A rule across the width it is given.
    pub fn new() -> Self {
        Divider {
            measured: Size::ZERO,
        }
    }
}

impl<M> View<M> for Divider {
    fn measure(&mut self, available: Size) {
        self.measured = Size::new(available.width, 1);
    }

    fn size(&self) -> Size {
        self.measured
    }

    fn render(&self, origin: Point) {
        Renderer::draw_line(origin, origin.offset(self.measured.width - 1, 0));
    }
}
