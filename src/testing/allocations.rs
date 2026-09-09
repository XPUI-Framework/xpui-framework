//! Counting what reaches the heap: what a screen builds in `update` it keeps,
//! and what `body()` builds it pays for on every frame — see
//! [`Text`](crate::Text). This is the part a test can hold.
//!
//! **`cfg(test)`, not the `testing` feature.** A `#[global_allocator]` belongs
//! to a whole binary, and this crate's `testing` feature is a dev-dependency of
//! the backends, the gallery and the tutorial — enabling it must not quietly
//! replace the allocator in every one of their test binaries. This one is
//! compiled only into `xpui`'s own.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;

/// Counts allocations on the calling thread.
///
/// Thread-local rather than one shared count because this binary runs its cases
/// in parallel: a global counter would measure whatever else happened to be
/// building a view tree at the time, and the failure would come and go.
struct Counting;

thread_local! {
    /// `const` initialiser, so first touch cannot itself allocate and recurse
    /// into the allocator doing the asking. `Cell<usize>` has no destructor, so
    /// nothing is registered for TLS teardown either.
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

// Safety: every call is forwarded to `System` with its layout untouched, so
// the contract `System` keeps is the one this keeps.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // `try_with`, because a `Cell` touched during TLS teardown panics — and
        // unwinding out of an allocator is **undefined behaviour**, not a
        // failed test.
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        // Safety: `layout` is forwarded untouched, under the contract the call
        // arrived with; an altered one would be undefined behaviour.
        unsafe { std::alloc::System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Safety: `ptr` and `layout` are the pair `alloc` returned, forwarded
        // unchanged; any other pair is undefined behaviour.
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// How many allocations `body` made on this thread.
pub(crate) fn allocations(body: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.with(Cell::get);
    body();
    ALLOCATIONS.with(Cell::get) - before
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    /// The meter has to move, or "allocated nothing" is true of everything.
    #[test]
    fn the_counter_notices_an_allocation() {
        let counted = allocations(|| assert_eq!(String::from("abc").len(), 3));
        assert!(counted > 0, "a String must register, or the meter is dead");
    }
}
