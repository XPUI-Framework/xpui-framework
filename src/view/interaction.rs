//! What a widget declares: the regions it owns, and the focus order over them.
//!
//! A widget never hit-tests. It declares the regions it owns and the message
//! each produces, and the runtime resolves one frame of input against that
//! list. What a declaration turns into is [`Trigger`](crate::view::Trigger);
//! see [`crate::screen`] for the resolution itself.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::geometry::Rect;
use crate::view::Trigger;

/// Which kinds of input an interaction accepts.
///
/// This is what stops a finger resting on a button re-firing it every frame:
/// only [`InputMask::DRAG`] interactions are offered held touches, and
/// everything else acts once, on release.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InputMask(u8);

impl InputMask {
    /// A tap: press and release inside the control.
    pub const TAP: InputMask = InputMask(1 << 0);
    /// Reachable by Up/Down, activated by Confirm.
    pub const FOCUS: InputMask = InputMask(1 << 1);
    /// Receives every frame the finger is down, with its position.
    pub const DRAG: InputMask = InputMask(1 << 2);
    /// A press held past the long-press threshold.
    pub const LONG_PRESS: InputMask = InputMask(1 << 3);
    /// This control is moved one step at a time rather than fired.
    ///
    /// Three things follow from it, which is why it is a mask and not a
    /// property of the trigger: Left/Right nudge it while it holds focus,
    /// Confirm declines to fire it, and on a device with no Left/Right pair
    /// Confirm may open it for editing instead. A `Slider` and a `Stepper` both
    /// say yes. It is what lets keys drive a value without the screen wiring
    /// them to one particular control.
    pub const ADJUST: InputMask = InputMask(1 << 4);

    /// What an ordinary control wants: tappable, and reachable by button.
    pub const DEFAULT: InputMask = InputMask(Self::TAP.0 | Self::FOCUS.0);

    /// This mask with `other`'s bits added.
    pub const fn union(self, other: InputMask) -> InputMask {
        InputMask(self.0 | other.0)
    }

    /// This mask with `other`'s bits removed.
    pub const fn without(self, other: InputMask) -> InputMask {
        InputMask(self.0 & !other.0)
    }

    /// Whether every bit of `other` is set here.
    pub const fn contains(self, other: InputMask) -> bool {
        self.0 & other.0 == other.0
    }
}

impl core::ops::BitOr for InputMask {
    type Output = InputMask;

    fn bitor(self, rhs: InputMask) -> InputMask {
        self.union(rhs)
    }
}

/// One interactive region, as declared by the widget that owns it.
pub struct Interaction<M> {
    /// The region, in screen pixels.
    pub rect: Rect,
    /// Which kinds of input it accepts.
    pub mask: InputMask,
    /// What it produces when it fires.
    pub trigger: Trigger<M>,
}

/// Collects a tree's interactions, telling each one whether it has focus as it
/// is declared.
///
/// Focus is an index into the focusable interactions **in tree order**. A widget
/// learns its own state from the return value of
/// [`declare`](Interactions::declare) rather than being told in a later pass,
/// so there is no third walk to keep in step with `render`.
pub struct Interactions<M> {
    items: Vec<Interaction<M>>,
    focus: usize,
    focusable: usize,
    /// Set when a view captured input, carrying the focus index it would like
    /// while it is up. `None` means nothing captured this frame.
    captured: Option<usize>,
    /// While true, declarations are ignored entirely. Set on the second pass of
    /// a frame that captures, so views behind a dialog neither take focus nor
    /// paint themselves focused — clearing them afterwards is too late, since a
    /// widget has already been told it holds focus by then.
    ignoring: bool,
    /// How far the scrolling view has been scrolled, handed down by the
    /// runtime. The view tree is rebuilt every frame, so a scroll view cannot
    /// remember this itself - it reads it here, exactly as a widget reads
    /// whether it holds focus.
    scroll: i32,
    /// The scrolling viewport and the height of what it holds, published by the
    /// scroll view during the walk so the runtime can keep focus visible.
    viewport: Option<(Rect, i32)>,
    /// Whether the control that owns the current focus stop is the one being
    /// walked into. See [`Interactions::parent_focused`].
    parent_focused: bool,
    /// The open edit's working value, when one is open on the control at
    /// `focus`. See [`Interactions::is_editing`].
    ///
    /// **The framework's copy, not the screen's.** While an edit is open the
    /// screen's value does not move, so the control has to paint from this or
    /// paint a number that ignores the keys.
    editing: Option<i32>,
}

/// A focus index no declaration can ever have, meaning "the focus is not in
/// this subtree at all".
///
/// A tree would have to declare `usize::MAX` focusable regions to reach it, and
/// each one costs a `Vec` entry.
const NO_FOCUS: usize = usize::MAX;

impl<M> Interactions<M> {
    /// A collector for a tree whose focused control is at `focus`.
    pub fn new(focus: usize) -> Self {
        Interactions {
            items: Vec::new(),
            focus,
            focusable: 0,
            captured: None,
            scroll: 0,
            viewport: None,
            ignoring: false,
            parent_focused: false,
            editing: None,
        }
    }

    /// A pass that ignores everything until a view calls
    /// [`capture`](Interactions::capture).
    ///
    /// The runtime uses this once it knows the frame contains a capturing view:
    /// a first pass finds out, a second collects only what is reachable.
    pub fn capturing(focus: usize) -> Self {
        Interactions {
            items: Vec::new(),
            focus,
            focusable: 0,
            captured: None,
            scroll: 0,
            viewport: None,
            ignoring: true,
            parent_focused: false,
            editing: None,
        }
    }

    /// Registers an interaction, returning whether it currently holds focus.
    pub fn declare(&mut self, rect: Rect, mask: InputMask, trigger: Trigger<M>) -> bool {
        if self.ignoring {
            return false;
        }

        let focused = mask.contains(InputMask::FOCUS) && {
            let mine = self.focusable;
            self.focusable += 1;
            mine == self.focus
        };

        self.items.push(Interaction {
            rect,
            mask,
            trigger,
        });
        focused
    }

    /// Discards everything declared so far: this view is the only thing
    /// reachable while it is present.
    ///
    /// A dialog drawn over a list shares the list's tree, so without this the
    /// side buttons would walk out of the dialog and into the rows behind it —
    /// invisible on screen, and wrong. Called by a capturing view *before* it
    /// declares its own regions, since the tree is walked in draw order and
    /// everything behind has therefore already been collected.
    ///
    /// `preferred_focus` is where focus should sit while the view is up: a
    /// picker opens on the value already chosen.
    pub fn capture(&mut self, preferred_focus: usize) {
        self.items.clear();
        self.focusable = 0;
        self.captured = Some(preferred_focus);
        self.ignoring = false;
    }

    /// Whether the control being walked into is the one holding focus.
    ///
    /// **For a widget that declares no focus stop of its own.** A `Stepper`
    /// takes one stop for the whole control and embeds a track that takes
    /// none, so the track has nothing to learn from its own declaration and
    /// asks the control that wrapped it instead.
    pub fn parent_focused(&self) -> bool {
        self.parent_focused
    }

    /// Says the subtree about to be walked belongs to a focused control.
    ///
    /// Set around the walk and put back afterwards, so a second control further
    /// down the tree does not inherit the answer.
    pub fn set_parent_focused(&mut self, focused: bool) {
        self.parent_focused = focused;
    }

    /// Whether the control at `focus` is open for editing.
    ///
    /// The runtime owns the edit; a widget cannot know from its own declaration
    /// that the keys have changed meaning. **A widget's question, not a
    /// screen's**: here rather than on `Screen` so a value widget can paint
    /// the working copy, and off the screen's path so the mode stays the
    /// framework's.
    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    /// The open edit's working value, for the control the edit is open on.
    ///
    /// A control that holds focus while this is `Some` **is** that control —
    /// there is one focus — so it paints this rather than the value it was
    /// built with.
    pub fn editing_value(&self) -> Option<i32> {
        self.editing
    }

    /// Says the focused control is open, for the runtime that opened it.
    pub(crate) fn editing(mut self, open: Option<i32>) -> Self {
        self.editing = open;
        self
    }

    /// Starts the walk with a scroll offset the scroll view should apply.
    pub fn scrolled(mut self, offset: i32) -> Self {
        self.scroll = offset;
        self
    }

    /// The offset a scroll view should apply to its content.
    pub fn scroll(&self) -> i32 {
        self.scroll
    }

    /// Published by a scroll view: the visible band, and how tall its content
    /// is. `None` means nothing on this screen scrolls.
    pub fn set_viewport(&mut self, viewport: Rect, content_height: i32) {
        self.viewport = Some((viewport, content_height));
    }

    /// The scrolling viewport and its content height, if a scroll view
    /// published one.
    pub fn viewport(&self) -> Option<(Rect, i32)> {
        self.viewport
    }

    /// How many interactions have been declared so far. A container uses this
    /// to find the ones its own child added.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Withdraws `mask` from every interaction declared since `from` that falls
    /// outside `visible`. A scrolled-away control keeps its focus stop, so it
    /// can still be reached, but stops accepting touches aimed at whatever now
    /// occupies that part of the screen.
    pub fn restrict_outside(&mut self, from: usize, visible: Rect, mask: InputMask) {
        for item in self.items.iter_mut().skip(from) {
            if !item.rect.intersects(visible) {
                item.mask = item.mask.without(mask);
            }
        }
    }

    /// The focus index a capturing view asked for, or `None` if none captured.
    pub fn captured_focus(&self) -> Option<usize> {
        self.captured
    }

    /// Rect of the focusable interaction at `focus`, in tree order. The
    /// runtime scrolls to bring this into view.
    pub fn focused_rect(&self, focus: usize) -> Option<Rect> {
        self.items
            .iter()
            .filter(|item| item.mask.contains(InputMask::FOCUS))
            .nth(focus)
            .map(|item| item.rect)
    }

    /// How many interactions can hold focus. The runtime wraps its cursor on
    /// this.
    pub fn focusable_count(&self) -> usize {
        self.focusable
    }

    /// Everything declared so far, in tree order.
    pub fn items(&self) -> &[Interaction<M>] {
        &self.items
    }

    /// Whether nothing has been declared.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// A collector for a sub-component, positioned so its focus numbering
    /// continues this one's. A mapped component's controls therefore sit in the
    /// parent's focus order exactly where they appear in the tree.
    ///
    /// **Everything a widget can ask crosses with it.** A sub-component is a
    /// tree like any other: a `Slider` inside one has to learn that it holds
    /// focus, that the control wrapping it does, and that an edit is open on
    /// it, or it paints the screen's value while the keys move a copy it
    /// cannot see.
    pub(crate) fn child<N>(&self) -> Interactions<N> {
        // `saturating_sub` would answer `0` when the focus is *behind* this
        // subtree, telling the first thing inside it that it holds a focus that
        // is somewhere above. `NO_FOCUS` is the honest answer: nothing here.
        let focus = self.focus.checked_sub(self.focusable).unwrap_or(NO_FOCUS);
        Interactions {
            items: Vec::new(),
            focus,
            focusable: 0,
            captured: None,
            scroll: self.scroll,
            viewport: None,
            ignoring: self.ignoring,
            parent_focused: self.parent_focused,
            editing: self.editing,
        }
    }

    /// Folds a sub-component's interactions in, translating its messages.
    pub(crate) fn absorb<N: 'static>(&mut self, inner: Interactions<N>, convert: fn(N) -> M)
    where
        M: 'static,
    {
        for item in inner.items {
            let trigger = match item.trigger {
                Trigger::Message(message) => Trigger::Message(convert(message)),
                Trigger::Value { make, max, value } => Trigger::MappedValue {
                    make: Box::new(move |v| convert(make(v))),
                    max,
                    value,
                },
                Trigger::MappedValue { make, max, value } => Trigger::MappedValue {
                    make: Box::new(move |v| convert(make(v))),
                    max,
                    value,
                },
                Trigger::Step {
                    make,
                    set,
                    max,
                    value,
                } => Trigger::MappedStep {
                    make: Box::new(move |delta| convert(make(delta))),
                    set: set.map(|set| Box::new(move |v| convert(set(v))) as Box<dyn Fn(i32) -> M>),
                    max,
                    value,
                },
                Trigger::MappedStep {
                    make,
                    set,
                    max,
                    value,
                } => Trigger::MappedStep {
                    make: Box::new(move |delta| convert(make(delta))),
                    set: set.map(|set| Box::new(move |v| convert(set(v))) as Box<dyn Fn(i32) -> M>),
                    max,
                    value,
                },
            };
            self.items.push(Interaction {
                rect: item.rect,
                mask: item.mask,
                trigger,
            });
        }
        self.focusable += inner.focusable;
    }
}
