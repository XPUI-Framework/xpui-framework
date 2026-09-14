//! Time, for anything the framework paces itself with.

/// The host's monotonic clock.
pub trait Clock {
    /// Milliseconds since boot.
    ///
    /// Key auto-repeat is timed against it. The caller only takes wrapping
    /// differences, so a host may return a plain counter that wraps.
    fn millis(&self) -> u32;
}

/// Milliseconds since boot.
pub fn millis() -> u32 {
    super::current().millis()
}
