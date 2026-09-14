//! Folding a mapped sub-component's declarations into its parent's.

use alloc::boxed::Box;
use alloc::vec::Vec;

use super::{Interaction, Interactions, NO_FOCUS};
use crate::view::Trigger;

impl<M> Interactions<M> {
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
            dismissal: None,
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
