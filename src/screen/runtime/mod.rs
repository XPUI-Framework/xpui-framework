//! The state a screen never sees: which interaction holds focus, how far the
//! content is scrolled, and the tree painted from the two.

mod input;

use super::Screen;
use crate::geometry::Point;
use crate::host::{Button, Renderer, request_update};
use crate::view::{Interactions, View};

/// Per-screen state the runtime owns so screens never see it.
struct Repeat {
    button: Option<Button>,
    /// When the current press started, and when it last fired.
    pressed_at: u32,
    fired_at: u32,
}

/// Drives a [`Screen`]: owns focus, input routing, repeat timing and the
/// first-paint guard.
pub struct Runtime<S: Screen> {
    pub(super) screen: S,
    /// Index into the focusable interactions, in tree order.
    focus: usize,
    /// No touch is routed before the first paint — the tree a touch would be
    /// tested against has not been shown yet. The C++ panel guards the same
    /// way with its `uiReady` flag.
    painted: bool,
    /// Swallows the release that ends a drag, so it cannot also read as a tap.
    dragging: bool,
    /// Where focus sat before a view captured input, so dismissing a dialog
    /// returns to the row that opened it rather than wherever its index landed.
    focus_before_capture: Option<usize>,
    /// How far the screen's scrolling view is scrolled. Owned here because the
    /// view tree is rebuilt every frame and could not remember it, and because
    /// keeping focus visible is the runtime's job — a screen never sees it.
    scroll: i32,
    repeat: Repeat,
}

impl<S: Screen> Runtime<S> {
    pub fn new(screen: S) -> Self {
        Runtime {
            screen,
            focus: 0,
            painted: false,
            dragging: false,
            focus_before_capture: None,
            scroll: 0,
            repeat: Repeat {
                button: None,
                pressed_at: 0,
                fired_at: 0,
            },
        }
    }

    /// Which interaction holds focus. Exposed only for tests that drive the
    /// runtime directly; screens never see focus at all.
    #[cfg(any(test, feature = "testing"))]
    pub fn focused_index(&self) -> usize {
        self.focus
    }

    /// Moves focus in or out of a capturing view. Returns whether it moved.
    ///
    /// Entering capture parks the outer focus and adopts the view's preference —
    /// a picker opens on the value already chosen. Leaving puts the old focus
    /// back, so dismissing a dialog returns to the row that opened it rather
    /// than wherever its index happened to land.
    fn adopt_capture(&mut self, interactions: &Interactions<S::Message>) -> bool {
        match interactions.captured_focus() {
            Some(preferred) if self.focus_before_capture.is_none() => {
                self.focus_before_capture = Some(self.focus);
                self.focus = preferred.min(interactions.focusable_count().saturating_sub(1));
                true
            }
            None => match self.focus_before_capture.take() {
                Some(previous) => {
                    self.focus = previous;
                    true
                }
                None => false,
            },
            _ => false,
        }
    }

    /// Scrolls the minimum needed to bring the focused interaction fully into
    /// view. Returns whether the offset moved.
    ///
    /// Minimum rather than centring: on a panel that repaints whole and takes a
    /// second or more, the less that shifts under the eye the better.
    fn settle_scroll(&mut self, interactions: &Interactions<S::Message>) -> bool {
        let Some((viewport, content_height)) = interactions.viewport() else {
            return false;
        };
        let Some(focused) = interactions.focused_rect(self.focus) else {
            return false;
        };

        let last = interactions.focusable_count().saturating_sub(1);
        let mut next = self.scroll;

        if focused.bottom() > viewport.bottom() {
            next += focused.bottom() - viewport.bottom();
        }
        if focused.origin.y < viewport.origin.y {
            next -= viewport.origin.y - focused.origin.y;
        }

        // Scrolling the bare minimum leaves whatever sits above the first
        // control — a section title, typically — stranded off the top edge,
        // with nothing focusable up there to scroll back to. The ends are
        // special: reaching the first control means the top of the content,
        // and the last means the bottom.
        if self.focus == 0 {
            next = 0;
        } else if self.focus == last {
            next = content_height - viewport.size.height;
        }

        let next = next.clamp(0, (content_height - viewport.size.height).max(0));
        if next == self.scroll {
            return false;
        }
        self.scroll = next;
        true
    }

    /// Collects, then settles focus against whatever captured input this frame,
    /// so every caller — Confirm, Up/Down, taps — reads the same table.
    fn collect_settled(&mut self) -> Interactions<S::Message> {
        let interactions = self.collect();
        if self.adopt_capture(&interactions) {
            return self.collect();
        }
        // Not while something has captured input. `self.focus` then indexes the
        // dialog's rows, not the screen's, so settling against it reads "option
        // 0" as "the top of the content" and drags the list out from under the
        // dialog. Content behind an overlay is frozen until the overlay goes.
        if interactions.captured_focus().is_none() && self.settle_scroll(&interactions) {
            return self.collect();
        }
        interactions
    }

    /// Builds, measures and collects the tree's interactions.
    ///
    /// One pass is enough for routing: [`Interactions::capture`] discards
    /// whatever was declared before it, so the table already holds only what a
    /// dialog left reachable. Painting is the case that needs more — see
    /// [`render`](super::Driver::render).
    fn collect(&self) -> Interactions<S::Message> {
        let mut view = self.screen.body();
        view.measure(Renderer::screen_size());

        let mut out = Interactions::new(self.focus).scrolled(self.scroll);
        view.interactions(Point::ORIGIN, &mut out);
        out
    }

    /// Applies a message and schedules the repaint the screen would otherwise
    /// have to remember.
    fn dispatch(&mut self, message: S::Message) {
        self.screen.update(message);
        request_update();
    }

    /// Moves focus, wrapping at both ends. Returns whether anything moved.
    fn move_focus(&mut self, delta: isize, count: usize) -> bool {
        if count == 0 {
            return false;
        }
        let next = (self.focus as isize + delta).rem_euclid(count as isize) as usize;
        if next == self.focus {
            return false;
        }
        self.focus = next;
        true
    }

    /// Scrolls when moving the focus could not.
    ///
    /// Scrolling is normally a side effect of keeping the focused control
    /// visible, which works until a screen has nothing focusable on it — a page
    /// of text, say. On a device with a touchscreen you would swipe; on one
    /// with only buttons there is no other way to reach the rest, so the
    /// buttons have to do it.
    ///
    /// Half a viewport per press: a whole one loses the line you were reading,
    /// and a single line takes forever on a panel that refreshes in a second.
    fn scroll_by(&mut self, delta: isize, interactions: &Interactions<S::Message>) -> bool {
        let Some((viewport, content_height)) = interactions.viewport() else {
            return false;
        };

        let furthest = (content_height - viewport.size.height).max(0);
        if furthest == 0 {
            return false;
        }

        let step = (viewport.size.height / 2).max(1);
        let next = (self.scroll + delta as i32 * step).clamp(0, furthest);
        if next == self.scroll {
            return false;
        }
        self.scroll = next;
        true
    }

    pub(super) fn render(&mut self) {
        if !self.screen.is_overlay() {
            Renderer::clear();
        }

        // Settle focus before painting: a dialog appearing moves focus into it,
        // and the frame it opens on must highlight the right option.
        let capturing = self.collect_settled().captured_focus().is_some();

        let mut view = self.screen.body();
        view.measure(Renderer::screen_size());
        // Interactions run before painting so each widget learns whether it
        // holds focus and can draw itself selected. Behind a dialog it must
        // learn the opposite: a widget is told it holds focus *as it declares*,
        // so a list behind would record a focused row and keep painting the
        // highlight. Clearing the table afterwards is too late — this pass
        // ignores those declarations outright.
        let mut out = if capturing {
            Interactions::capturing(self.focus)
        } else {
            Interactions::new(self.focus)
        }
        .scrolled(self.scroll);
        view.interactions(Point::ORIGIN, &mut out);
        view.render(Point::ORIGIN);

        self.painted = true;
    }
}
