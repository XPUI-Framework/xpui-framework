//! A screen inside the navigation stack.

use alloc::boxed::Box;
use alloc::string::String;

use crate::geometry::{Point, Size};
use crate::host::ValueMode;
use crate::host::{Hint, ScreenChrome, Theme};
use crate::view::{Interactions, View};

/// The root view for a screen pushed onto the activity stack.
///
/// The backend draws the title band and the button hints; the content is laid
/// out in the region between them. Using this rather than painting a whole
/// screen by hand is what keeps a screen looking like every other screen on
/// the device, and following the user's theme and button remapping.
///
/// By default the header shows the screen's own title and **all four** hints
/// are `Hint::Standard` — the host resolves each to its own label for that
/// slot, so every key a device has is named.
///
/// ```rust
/// # use xpui::{Hint, NavigationScreen, Text, vstack};
/// # xpui::testing::install();
/// # let (name, value) = ("Free space", "182 KB");
/// # let _: NavigationScreen<()> =
/// NavigationScreen::new(vstack![20; Text::new(name), Text::new(value)])
///     .title("Storage")                       // else the screen's own title
///     .hints(Hint::Standard, Hint::text("Save"), Hint::None, Hint::None)
/// # ;
/// ```
pub struct NavigationScreen<M> {
    content: Box<dyn View<M>>,
    /// `None` defers to the activity's localized title.
    title: Option<String>,
    /// Drawn over the content, outside any scrolling it does.
    overlay: Option<Box<dyn View<M>>>,
    hints: [Hint; 4],
    measured: Size,
}

impl<M: 'static> NavigationScreen<M> {
    pub fn new(content: impl View<M> + 'static) -> Self {
        NavigationScreen {
            content: Box::new(content),
            title: None,
            hints: [
                Hint::Standard,
                Hint::Standard,
                Hint::Standard,
                Hint::Standard,
            ],
            overlay: None,
            measured: Size::ZERO,
        }
    }

    /// Overrides the header title.
    ///
    /// Prefer the activity's own title, which is already translated; this is
    /// for titles computed at run time, such as a file name.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// A view drawn over the content: a dialog, typically.
    ///
    /// It sits outside the content, so a scrolling view cannot clip it and it
    /// does not scroll away. Declared last, so a dialog that captures input
    /// discards everything the content declared.
    pub fn overlay(mut self, overlay: impl View<M> + 'static) -> Self {
        self.overlay = Some(Box::new(overlay));
        self
    }

    /// The overlay, only when `when` holds. Saves a screen an `if` in `body`.
    pub fn overlay_if(self, when: bool, overlay: impl View<M> + 'static) -> Self {
        if when { self.overlay(overlay) } else { self }
    }

    /// Sets the four hints, given by meaning rather than by screen position.
    pub fn hints(mut self, back: Hint, confirm: Hint, previous: Hint, next: Hint) -> Self {
        self.hints = [back, confirm, previous, next];
        self
    }
}

impl<M> View<M> for NavigationScreen<M> {
    fn measure(&mut self, available: Size) {
        self.content.measure(Theme::content_area().size);
        if let Some(overlay) = self.overlay.as_mut() {
            // The whole screen: a dialog centres itself on the panel, not on
            // the content band.
            overlay.measure(available);
        }
        self.measured = available;
    }

    fn size(&self) -> Size {
        self.measured
    }

    fn render(&self, origin: Point) {
        match &self.title {
            Some(title) => ScreenChrome::draw_header(title),
            None => ScreenChrome::draw_screen_header(),
        }

        let content = Theme::content_area();
        self.content.render(origin.offset(content.x(), content.y()));

        // The screen said what these four keys do. While a value control has
        // the focus, two of them do something else, and the screen has no way
        // to know — so the runtime says so here rather than being asked.
        let (back, confirm) = match crate::host::value_mode() {
            ValueMode::None => (&self.hints[0], &self.hints[1]),
            ValueMode::Openable => (&self.hints[0], &Hint::Edit),
            ValueMode::Open => (&Hint::Cancel, &Hint::Done),
        };
        ScreenChrome::draw_button_hints(back, confirm, &self.hints[2], &self.hints[3]);

        // Last, so it covers the content and the hints alike.
        if let Some(overlay) = &self.overlay {
            overlay.render(origin);
        }
    }

    fn interactions(&mut self, origin: Point, out: &mut Interactions<M>) {
        // Same inset render uses; the chrome itself is not interactive.
        let content = Theme::content_area();
        let at = origin.offset(content.x(), content.y());
        self.content.interactions(at, out);

        // After the content: a capturing overlay discards what came before it.
        if let Some(overlay) = self.overlay.as_mut() {
            overlay.interactions(origin, out);
        }
    }
}
