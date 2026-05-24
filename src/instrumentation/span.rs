use crate::clocks::PeerClock;
use crate::instrumentation::event::EventId;

/// A measurement span from a local start event to a (potential) remote finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    event_id: EventId,
    name: &'static str,
    start_local_ms: u32,
    finish_local_ms: Option<u32>
}

impl Span {
    pub fn new(event_id: EventId, name: &'static str, start_local_ms: u32) -> Self {
        Self {
            event_id,
            name,
            start_local_ms,
            finish_local_ms: None
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

    /// `None` if no finish time has been recorded yet.
    pub fn finish_local_ms(&self) -> Option<u32> {
        self.finish_local_ms
    }

    /// Local-only elapsed time in milliseconds.
    /// 
    /// Stops once finish time has been recorded.
    #[inline]
    pub fn elapsed_ms(&self, clock: &PeerClock) -> u32 {
        match self.finish_local_ms {
            Some(finish_local_ms) => finish_local_ms,
            None => clock.local_ms() - self.start_local_ms
        }
    }

    pub fn duration_ms(&self, clock: &PeerClock) -> u32 {
        self.elapsed_ms(clock) - self.start_local_ms
    }

    /// Compute and record cross-host latency given the remote finish time and a synchronised clock.
    ///
    /// Both `start_local_ms` and `remote_finish_ms` must be elapsed milliseconds
    /// since the respective peer clock started.  The PeerClock uses clock correction
    /// and start-delta to translate between the two elapsed-time domains so the subtraction
    /// yields end-to-end latency.
    ///
    /// Returns `None` if the clock is not yet synchronised or the peer start time
    /// is unknown.
    #[inline]
    pub fn finish_remote(&mut self, finish_remote_ms: u32, clock: &PeerClock) -> Option<u32> {
        let finish_local_ms = clock.remote_to_local(finish_remote_ms, true).map(|v| {v.max(0) as u32 })?;
        self.finish_local_ms = Some(finish_local_ms);
        Some(self.duration_ms(clock))
    }
}
