//! A run of text on one line.

use alloc::string::String;

use crate::geometry::{Point, Size};
use crate::host::{Font, Renderer};
use crate::view::View;

/// Draws a single line of text.
///
/// Holds the string it was built with, unchanged: the framework has no C
/// boundary to satisfy, so a host that needs another representation — a
/// NUL-terminated buffer for a C renderer, say — makes one at its own edge.
/// Formatting the string in `update()` and keeping it, rather than calling
/// `format!` inside `body()`, is what keeps allocation off the render path.
///
/// Size comes from the host's own font metrics, taken while the tree is laid
/// out and kept for [`View::size`]. Estimating it instead drifts from what is
/// painted and pushes content past the bottom of the screen.
///
/// ```rust
/// # use xpui::{Font, Text};
/// # xpui::testing::install();
/// # let (percent, label) = (72, "Battery");
/// Text::new("Battery");
/// Text::new(format!("{percent}%")).bold();
/// Text::new(label).font(Font::ui_small());
/// ```
pub struct Text {
    content: String,
    font: Font,
    measured: Size,
}

impl Text {
    /// Text in the default interface font.
    ///
    /// A string containing an interior NUL renders as empty, since it cannot
    /// be passed to the C++ renderer.
    pub fn new(content: impl Into<String>) -> Self {
        Text {
            content: content.into(),
            font: Font::ui(),
            measured: Size::ZERO,
        }
    }

    /// Draws in `font` instead of the default.
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Draws bold.
    pub fn bold(mut self) -> Self {
        self.font = self.font.bold();
        self
    }

    /// Draws italic.
    pub fn italic(mut self) -> Self {
        self.font = self.font.italic();
        self
    }
}

impl<M> View<M> for Text {
    fn measure(&mut self, _available: Size) {
        self.measured = Size::new(self.font.text_width(&self.content), self.font.line_height());
    }

    fn size(&self) -> Size {
        self.measured
    }

    fn render(&self, origin: Point) {
        Renderer::draw_text(origin, &self.content, self.font.id(), self.font.style());
    }
}
