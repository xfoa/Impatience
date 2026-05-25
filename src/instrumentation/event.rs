use std::fmt;

/// Opaque identifier for an instrumentation event.
///
/// Monotonically increasing within a single [`Profiler`](crate::instrumentation::Profiler)
/// instance.  Collision-safe across peers because each peer mints its own IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u64);

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An instrumentation event.
///
/// Carries a stable ID, a human-readable name, and the local wall-clock time
/// at which it occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct Event {
    pub id: EventId,
    pub name: &'static str,
    pub local_ts: u64,
}
