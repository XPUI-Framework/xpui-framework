//! Everything the fake host was asked to draw, in the order it was asked.
//!
//! The per-kind accessors in [`super`] answer "was a list drawn, and with how
//! many rows". They cannot answer "what did this screen look like", because
//! each kind is recorded in its own list and the interleaving is lost — a
//! header drawn after the content reads the same as one drawn before it.
//!
//! This log keeps one ordered sequence instead, which is what a snapshot
//! compares. The per-kind accessors are derived from it, so both views of the
//! same frame agree by construction.

use alloc::string::String;
use alloc::vec::Vec;

use crate::geometry::{Point, Rect, Size};
use crate::host::{ControlState, FontStyle, IconRef};

/// How a rectangle was painted.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RectKind {
    Filled,
    Stroked,
    /// Dithered, which reads as grey on a 1-bit panel.
    Dither,
    /// Darkened without erasing, so what was behind stays legible.
    Scrim,
}

impl RectKind {
    fn label(self) -> &'static str {
        match self {
            RectKind::Filled => "filled",
            RectKind::Stroked => "stroked",
            RectKind::Dither => "dither",
            RectKind::Scrim => "scrim",
        }
    }
}

/// The three fields of a list row, as the theme asked for them. `None` is a
/// field the row omitted, which is how a host chooses a one- or two-line row.
pub type RowCells = [Option<String>; 3];

/// One call the framework made on the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DrawOp {
    Clear,
    Text {
        origin: Point,
        text: String,
        font: i32,
        style: FontStyle,
    },
    Rect {
        rect: Rect,
        kind: RectKind,
        /// Ink or background. Only meaningful for [`RectKind::Filled`]; a
        /// dither carries `light` here instead.
        black: bool,
    },
    Line {
        from: Point,
        to: Point,
    },
    Image {
        origin: Point,
        size: Size,
        /// Bytes handed over, so a test can tell an empty bitmap from a real
        /// one without the log holding the pixels.
        bytes: usize,
    },
    Icon {
        origin: Point,
        icon: IconRef,
    },
    /// `None` lifts the clip.
    Clip(Option<Rect>),
    Header {
        title: Option<String>,
        subtitle: Option<String>,
    },
    SubHeader {
        rect: Rect,
        label: String,
        right: Option<String>,
    },
    /// The four slots in meaning order: back, confirm, previous, next. `None`
    /// is the host's own standard label for that slot.
    Hints([Option<String>; 4]),
    ProgressBar {
        rect: Rect,
        current: u32,
        total: u32,
    },
    Slider {
        rect: Rect,
        value: i32,
        max: i32,
        state: ControlState,
    },
    ScrollIndicator {
        rect: Rect,
        content: i32,
        visible: i32,
        offset: i32,
    },
    List {
        rect: Rect,
        selected: i32,
        rows: Vec<RowCells>,
    },
    OptionPopup {
        title: String,
        selected: i32,
        options: Vec<Option<String>>,
    },
}

fn style_label(style: FontStyle) -> &'static str {
    match style {
        FontStyle::Regular => "regular",
        FontStyle::Bold => "bold",
        FontStyle::Italic => "italic",
        FontStyle::BoldItalic => "bold-italic",
    }
}

/// A rect as `(x,y WxH)`, the shape every line in a snapshot uses.
fn rect_text(rect: Rect) -> String {
    alloc::format!(
        "({},{} {}x{})",
        rect.x(),
        rect.y(),
        rect.width(),
        rect.height()
    )
}

fn quoted(text: &str) -> String {
    // Escaping keeps one op on one line even if a screen draws a newline, and
    // keeps a quote in the content from ending the field early.
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn optional(text: &Option<String>) -> String {
    match text {
        Some(text) => quoted(text),
        None => String::from("-"),
    }
}

impl DrawOp {
    /// One line of a snapshot.
    ///
    /// Deliberately flat and readable rather than serialised: the point of a
    /// golden file is that a reviewer can see what changed in the diff without
    /// running anything.
    pub fn to_line(&self) -> String {
        match self {
            DrawOp::Clear => String::from("clear"),
            DrawOp::Text {
                origin,
                text,
                font,
                style,
            } => alloc::format!(
                "text        ({},{}) {} font={} {}",
                origin.x,
                origin.y,
                quoted(text),
                font,
                style_label(*style)
            ),
            DrawOp::Rect { rect, kind, black } => {
                let mut line = alloc::format!("rect        {} {}", rect_text(*rect), kind.label());
                if matches!(kind, RectKind::Filled) && !black {
                    line.push_str(" background");
                }
                if matches!(kind, RectKind::Dither) && *black {
                    line.push_str(" light");
                }
                line
            }
            DrawOp::Line { from, to } => {
                alloc::format!("line        ({},{}) -> ({},{})", from.x, from.y, to.x, to.y)
            }
            DrawOp::Image {
                origin,
                size,
                bytes,
            } => alloc::format!(
                "image       ({},{}) {}x{} bytes={}",
                origin.x,
                origin.y,
                size.width,
                size.height,
                bytes
            ),
            DrawOp::Icon { origin, icon } => alloc::format!(
                "icon        ({},{}) kind={} variant={} size={}",
                origin.x,
                origin.y,
                icon.kind,
                icon.variant,
                icon.size
            ),
            DrawOp::Clip(Some(rect)) => alloc::format!("clip        {}", rect_text(*rect)),
            DrawOp::Clip(None) => String::from("clip        none"),
            DrawOp::Header { title, subtitle } => alloc::format!(
                "header      title={} subtitle={}",
                optional(title),
                optional(subtitle)
            ),
            DrawOp::SubHeader { rect, label, right } => alloc::format!(
                "sub-header  {} {} right={}",
                rect_text(*rect),
                quoted(label),
                optional(right)
            ),
            DrawOp::Hints(slots) => alloc::format!(
                "hints       back={} confirm={} previous={} next={}",
                optional(&slots[0]),
                optional(&slots[1]),
                optional(&slots[2]),
                optional(&slots[3])
            ),
            DrawOp::ProgressBar {
                rect,
                current,
                total,
            } => alloc::format!("progress    {} {}/{}", rect_text(*rect), current, total),
            DrawOp::Slider {
                rect,
                value,
                max,
                state,
            } => {
                let mode = match state {
                    ControlState::Idle => "",
                    ControlState::Focused => " focused",
                    ControlState::Editing => " editing",
                };
                alloc::format!("slider      {} {}/{}{}", rect_text(*rect), value, max, mode)
            }
            DrawOp::ScrollIndicator {
                rect,
                content,
                visible,
                offset,
            } => alloc::format!(
                "scrollbar   {} content={} visible={} offset={}",
                rect_text(*rect),
                content,
                visible,
                offset
            ),
            DrawOp::List {
                rect,
                selected,
                rows,
            } => {
                let mut line = alloc::format!(
                    "list        {} rows={} selected={}",
                    rect_text(*rect),
                    rows.len(),
                    selected
                );
                for (index, cells) in rows.iter().enumerate() {
                    line.push_str(&alloc::format!(
                        "\n  row {index}  title={} subtitle={} value={}",
                        optional(&cells[0]),
                        optional(&cells[1]),
                        optional(&cells[2])
                    ));
                }
                line
            }
            DrawOp::OptionPopup {
                title,
                selected,
                options,
            } => {
                let mut line = alloc::format!(
                    "popup       {} options={} selected={}",
                    quoted(title),
                    options.len(),
                    selected
                );
                for (index, option) in options.iter().enumerate() {
                    line.push_str(&alloc::format!("\n  option {index}  {}", optional(option)));
                }
                line
            }
        }
    }
}

/// Every op as text, one per line, which is exactly a golden file's body.
pub fn render(ops: &[DrawOp]) -> String {
    let mut out = String::new();
    for op in ops {
        out.push_str(&op.to_line());
        out.push('\n');
    }
    out
}
