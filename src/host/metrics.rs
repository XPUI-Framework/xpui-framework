//! Text measurement.
//!
//! The framework never estimates glyph sizes — an estimate drifts from what is
//! painted and pushes content off the panel. Every width and height comes from
//! the host's own font engine.

/// A font the host has registered, as a number only the host can interpret.
///
/// `0` means "this build does not ship that font".
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FontId(
    /// The host's own number for the font; `0` is none.
    pub i32,
);

impl FontId {
    /// The font a build compiled out, which measures zero.
    pub const UNAVAILABLE: FontId = FontId(0);

    /// Whether this build ships the font.
    pub fn is_available(self) -> bool {
        self.0 != 0
    }
}

/// Weight and slant, as a host-independent value.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum FontStyle {
    /// Upright, regular weight.
    #[default]
    Regular = 0,
    /// Heavier weight.
    Bold = 1,
    /// Slanted.
    Italic = 2,
    /// Both.
    BoldItalic = 3,
}

/// What a piece of text is *for*, rather than which typeface it uses.
///
/// The host decides what each role means. Naming families here would tie the
/// framework to one product's assets, and a role survives those assets being
/// changed.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FontRole {
    /// Interface text: labels, list rows, values.
    Ui,
    /// Smaller interface text, for captions and secondary labels.
    UiSmall,
    /// The face the user chose for reading, including fonts loaded from storage.
    Reader,
}

/// Font lookup and measurement.
pub trait TextMetrics {
    /// The font for a role, or [`FontId::UNAVAILABLE`] when this build does not
    /// ship one.
    ///
    /// **The id must be derived from the face's own bytes**, not from the role
    /// and not from a counter. A consumer keys a glyph cache on it, so handing
    /// out a stable id over changed bytes leaves every such cache serving the
    /// old face with nothing anywhere to notice — the failure is a screen that
    /// paints yesterday's type and a test suite that is entirely green. Hash
    /// the face; two faces that draw the same string differently must not
    /// claim the same id.
    fn font(&self, role: FontRole) -> FontId;

    /// The width `text` paints at in `font` and `style`, in pixels.
    fn text_width(&self, font: FontId, text: &str, style: FontStyle) -> i32;

    /// The height one line of `font` occupies, ascent and descent included.
    fn line_height(&self, font: FontId) -> i32;
}

/// A font plus the style it is drawn in, as widgets reach for it.
///
/// ```rust
/// # use xpui::{Font, Text};
/// # xpui::testing::install();
/// # let value = "72%";
/// Text::new("Battery").font(Font::ui_small());
/// Text::new(value).font(Font::ui().bold());
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Font {
    id: FontId,
    style: FontStyle,
}

impl Font {
    /// A font this build does not ship, which measures zero.
    ///
    /// A missing face therefore takes no room in a layout, and a
    /// [`Text`](crate::Text) in it draws nothing. A widget calling
    /// [`Renderer::draw_text`](crate::host::Renderer::draw_text) itself checks
    /// [`is_available`](Font::is_available) first.
    pub const UNAVAILABLE: Font = Font {
        id: FontId::UNAVAILABLE,
        style: FontStyle::Regular,
    };

    /// The host's font for `role`, in regular style.
    pub fn role(role: FontRole) -> Self {
        Font {
            id: super::current().font(role),
            style: FontStyle::Regular,
        }
    }

    /// Interface text: the default for every widget.
    pub fn ui() -> Self {
        Self::role(FontRole::Ui)
    }

    /// Smaller interface text, for captions and secondary labels.
    pub fn ui_small() -> Self {
        Self::role(FontRole::UiSmall)
    }

    /// The face the user chose for reading.
    pub fn reader() -> Self {
        Self::role(FontRole::Reader)
    }

    /// The same font in bold, replacing any style set before.
    pub fn bold(self) -> Self {
        self.with_style(FontStyle::Bold)
    }

    /// The same font in italic, replacing any style set before.
    pub fn italic(self) -> Self {
        self.with_style(FontStyle::Italic)
    }

    /// The same font in `style`.
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// The host's id for the face.
    pub fn id(self) -> FontId {
        self.id
    }

    /// The weight and slant it is drawn in.
    pub fn style(self) -> FontStyle {
        self.style
    }

    /// Whether this build ships the face.
    pub fn is_available(self) -> bool {
        self.id.is_available()
    }

    /// The height one line occupies, or 0 for a face this build lacks.
    pub fn line_height(self) -> i32 {
        if !self.is_available() {
            return 0;
        }
        super::current().line_height(self.id)
    }

    /// The width `text` paints at, or 0 for a face this build lacks.
    pub fn text_width(self, text: &str) -> i32 {
        if !self.is_available() {
            return 0;
        }
        super::current().text_width(self.id, text, self.style)
    }
}
