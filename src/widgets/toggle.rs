//! A boolean setting.

use alloc::string::String;

use crate::geometry::{Point, Size};
use crate::view::{Interactions, View};
use crate::widgets::{List, ListRow};

/// A boolean setting, drawn as a row whose value reads as one of two words.
///
/// The theme draws no switch graphic. It renders through the theme's list, so
/// it is identical whether it stands alone or sits in a [`List`].
///
/// ```rust
/// # use xpui::Toggle;
/// # #[derive(Clone, Copy)]
/// # enum Msg { Hyphenation(bool) }
/// # xpui::testing::install();
/// # let on = false;
/// Toggle::new("Hyphenation", on, "On", "Off")
///     .on_change(Msg::Hyphenation);
/// ```
pub struct Toggle<M> {
    list: List<M>,
    on: bool,
    row: Option<ListRow<M>>,
}

impl<M: Clone> Toggle<M> {
    /// A row labelled `label`, reading `on_label` or `off_label` for `on`.
    pub fn new(
        label: impl Into<String>,
        on: bool,
        on_label: impl Into<String>,
        off_label: impl Into<String>,
    ) -> Self {
        Toggle {
            list: List::new(),
            on,
            row: Some(ListRow::toggle(label, on, on_label, off_label)),
        }
    }

    /// Sends `make(next_state)` when tapped or confirmed.
    ///
    /// The framework flips the value, so the screen never writes
    /// `!self.something`.
    pub fn on_change(mut self, make: fn(bool) -> M) -> Self {
        let next = !self.on;
        if let Some(row) = self.row.take() {
            self.list = List::new().push(row.on_tap(make(next)));
        }
        self
    }

    /// Consumes the builder into the row, for putting several in one [`List`].
    ///
    /// The row carries no message, so give it one with
    /// [`ListRow::on_tap`]. `None` once [`on_change`](Toggle::on_change) has
    /// taken the row.
    pub fn into_row(mut self) -> Option<ListRow<M>> {
        self.row.take()
    }

    /// Materialises the row when no `on_change` was given, so a display-only
    /// toggle still draws.
    fn realise(&mut self) {
        if let Some(row) = self.row.take() {
            self.list = List::new().push(row);
        }
    }
}

impl<M: Clone> View<M> for Toggle<M> {
    fn measure(&mut self, available: Size) {
        self.realise();
        self.list.measure(available);
    }

    fn size(&self) -> Size {
        self.list.size()
    }

    fn render(&self, origin: Point) {
        self.list.render(origin);
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<M>) {
        self.list.interactions(origin, out);
    }
}
