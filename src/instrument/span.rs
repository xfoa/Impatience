use crate::clocks::PeerClock;
use crate::instrument::event::EventId;

/// A measurement span from a local start event to a (potential) remote finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    event_id: EventId,
    name: &'static str,
    start_local_usec: u64,
}

impl Span {
    pub fn new(event_id: EventId, name: &'static str, start_local_usec: u64) -> Self {
        Self {
            event_id,
            name,
            start_local_usec,
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

    /// Local-only elapsed time in microseconds.
    #[inline]
    pub fn elapsed(&self, now_usec: u64) -> u64 {
        now_usec.saturating_sub(self.start_local_usec)
    }

    /// Compute cross-host latency given the remote finish time and a synchronised clock.
    ///
    /// The remote finish time should be the remote peer's local wall-clock timestamp
    /// (e.g. when the injection occurred).  The clock correction translates the local
    /// start time into the remote time base so the subtraction yields end-to-end latency.
    ///
    /// Returns `None` if the clock is not yet synchronised.
    #[inline]
    pub fn remote_latency_us(&self, remote_finish_usec: u64, clock: &PeerClock) -> Option<i64> {
        let correction = clock.correction_usec()?;
        Some(remote_finish_usec as i64 + correction - self.start_local_usec as i64)
    }
}
