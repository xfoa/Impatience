use crate::clocks::PeerClock;
use crate::instrumentation::event::EventId;

/// A measurement span from a local start event to a (potential) remote finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    event_id: EventId,
    name: &'static str,
    start_local_ms: u64,
}

impl Span {
    pub fn new(event_id: EventId, name: &'static str, start_local_ms: u64) -> Self {
        Self {
            event_id,
            name,
            start_local_ms,
        }
    }

    #[inline]
    pub fn event_id(&self) -> EventId {
        self.event_id
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Local-only elapsed time in milliseconds.
    #[inline]
    pub fn elapsed(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.start_local_ms)
    }

    /// Compute cross-host latency given the remote finish time and a synchronised clock.
    ///
    /// The remote finish time should be the remote peer's local wall-clock timestamp
    /// in milliseconds (e.g. when the injection occurred).  The clock correction
    /// translates the local start time into the remote time base so the subtraction
    /// yields end-to-end latency.
    ///
    /// Returns `None` if the clock is not yet synchronised.
    #[inline]
    pub fn remote_latency_ms(&self, remote_finish_ms: u64, clock: &PeerClock) -> Option<i64> {
        let correction = clock.correction_ms()?;
        Some(remote_finish_ms as i64 - correction - self.start_local_ms as i64)
    }
}
