//! A dialog offering a list of choices.

use alloc::string::String;
use alloc::vec::Vec;

use crate::geometry::{Point, Rect, Size};
use crate::host::{Renderer, Theme};
use crate::view::{InputMask, Interactions, Scrim, Trigger, View};

/// A centred dialog offering a list of options, drawn by the host's theme so
/// it matches the host's own dialogs.
///
/// It **captures input**: while one is in the tree, nothing behind it can be
/// reached, focus starts on the option already chosen, and the side buttons
/// walk its options rather than the list underneath.
///
/// Renders over whatever is already in the framebuffer without clearing, so a
/// screen draws its content first and puts the dialog last.
///
/// The screen owns whether it is open — hold that in your own state and include
/// the dialog in `body()` while it is true. Back never finishes the screen
/// under an open dialog: it sends [`on_dismiss`](Modal::on_dismiss), or
/// nothing.
///
/// ```rust
/// # use xpui::Modal;
/// # #[derive(Clone, Copy)]
/// # enum Msg { Chose(usize), Dismiss }
/// # xpui::testing::install();
/// # let current = 1;
/// Modal::picker("Refresh Frequency", ["1 page", "5 pages", "10 pages"])
///     .selected(current)
///     .on_select(Msg::Chose)
///     .on_dismiss(Msg::Dismiss);
/// ```
pub struct Modal<M> {
    title: String,
    options: Vec<String>,
    selected: usize,
    scrim: Scrim,
    /// Set by [`on_select`](Modal::on_select); without one the dialog still
    /// captures and draws, it simply reports nothing.
    make: Option<fn(usize) -> M>,
    /// Set by [`on_dismiss`](Modal::on_dismiss): what Back and a tap outside
    /// the options send.
    dismiss: Option<M>,
    /// Which option held focus at the last interactions walk, so `render` can
    /// highlight it. Without this the dialog would keep painting whichever
    /// value was passed to `selected`, and the arrows would appear dead.
    focused_option: Option<usize>,
    measured: Size,
}

impl<M: Clone> Modal<M> {
    /// A dialog titled `title` offering `options`, the first selected.
    pub fn new<S: Into<String>>(
        title: impl Into<String>,
        options: impl IntoIterator<Item = S>,
    ) -> Self {
        Modal {
            title: title.into(),
            options: options.into_iter().map(Into::into).collect(),
            selected: 0,
            scrim: Scrim::None,
            make: None,
            dismiss: None,
            focused_option: None,
            measured: Size::ZERO,
        }
    }

    /// A dialog for choosing one value from several, the same dialog as
    /// [`new`](Modal::new) under a name that reads better at a settings row.
    pub fn picker<S: Into<String>>(
        title: impl Into<String>,
        options: impl IntoIterator<Item = S>,
    ) -> Self {
        Self::new(title, options)
    }

    /// A yes/no question, opening with focus on the first of `choices`, so the
    /// confirming one goes first.
    pub fn confirm<S: Into<String>>(
        title: impl Into<String>,
        choices: impl IntoIterator<Item = S>,
    ) -> Self {
        Self::new(title, choices)
    }

    /// The option to highlight, and where focus opens.
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Sends `make(index)` when an option is chosen, by touch or by Confirm.
    pub fn on_select(mut self, make: fn(usize) -> M) -> Self {
        self.make = Some(make);
        self
    }

    /// Sends `message` when the dialog is dismissed without a choice: by a Back
    /// the screen does not claim, or by a tap on none of its options.
    ///
    /// Without it, Back does nothing while the dialog is open, and a tap
    /// outside the options goes to `Screen::on_background_tap`. Either way the
    /// screen is never finished from under an open dialog. Like a choice, the
    /// message does not close the dialog; `update` does.
    pub fn on_dismiss(mut self, message: M) -> Self {
        self.dismiss = Some(message);
        self
    }

    /// Sets how the whole panel behind the dialog is painted, left as it was
    /// ([`Scrim::None`]) unless this is called.
    pub fn scrim(mut self, scrim: Scrim) -> Self {
        self.scrim = scrim;
        self
    }

    /// How many options it offers.
    pub fn len(&self) -> usize {
        self.options.len()
    }

    /// Whether it offers no options.
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    /// The option the theme should highlight: whichever holds focus, falling
    /// back to the value the screen said was current.
    fn highlighted(&self) -> i32 {
        let index = self.focused_option.unwrap_or(self.selected);
        i32::try_from(index).unwrap_or(-1)
    }

    /// Screen rect of one option row.
    ///
    /// Asked for rather than derived: the backend already did this arithmetic
    /// to paint the dialog, and computing it a second time here is how the
    /// painted rows and the touchable ones drift apart.
    fn row_rect(&self, index: usize) -> Option<Rect> {
        Theme::option_popup_row_rect(
            &self.title,
            &|i| self.options.get(i).map(String::as_str),
            self.options.len(),
            index,
        )
    }
}

impl<M: Clone> View<M> for Modal<M> {
    fn measure(&mut self, available: Size) {
        // The theme centres the dialog and sizes it to its content, so the
        // widget occupies the whole area as far as layout is concerned.
        self.measured = available;
    }

    fn size(&self) -> Size {
        self.measured
    }

    fn render(&self, _origin: Point) {
        if self.options.is_empty() {
            return;
        }

        if self.scrim == Scrim::Dim {
            Renderer::scrim(Renderer::screen_bounds());
        }

        Theme::draw_option_popup(
            &self.title,
            &|i| self.options.get(i).map(String::as_str),
            self.options.len(),
            self.highlighted(),
        );
    }

    fn interactions(&mut self, _origin: Point, out: &mut Interactions<M>) {
        self.focused_option = None;
        if self.options.is_empty() {
            return;
        }

        // Everything behind the dialog becomes unreachable, and focus opens on
        // the value already chosen. This happens even with no `on_select`: a
        // dialog with no outcome must still not let input through to the list.
        out.capture(self.selected);
        if let Some(message) = &self.dismiss {
            out.dismiss_with(message.clone());
        }

        let Some(make) = self.make else {
            return;
        };

        for index in 0..self.options.len() {
            if let Some(rect) = self.row_rect(index)
                && out.declare(rect, InputMask::DEFAULT, Trigger::Message(make(index)))
            {
                self.focused_option = Some(index);
            }
        }
    }
}
