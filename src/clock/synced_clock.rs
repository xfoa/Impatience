use crate::timesync::Counter24;
use crate::timesync::{TimeSynchroniser, TIME_23_LOST_BITS};

/// A higher-level clock abstraction that tracks local elapsed time
/// and maintains a drift estimate with a paired remote clock.
///
/// Internally uses 24-bit truncated timestamps for network sync;
/// externally exposes millisecond-resolution APIs.
#[derive(Clone, Debug)]
pub struct SyncedClock {
    start_usec: u64,
    sync: TimeSynchroniser,
}

impl SyncedClock {
    pub fn new() -> Self {
        Self {
            start_usec: 0,
            sync: TimeSynchroniser::new(),
        }
    }

    /// Record the local wall-clock time at which the clock starts.
    pub fn start(&mut self, now_usec: u64) {
        self.start_usec = now_usec;
        self.sync = TimeSynchroniser::new();
    }

    /// Return the wall-clock time at which the clock was started.
    #[inline]
    pub fn started_at(&self) -> u64 {
        self.start_usec
    }

    /// Update the clock with probe data received on a data datagram.
    ///
    /// `remote_send_ts24` is the 24-bit timestamp from the peer's
    /// [`get_probe_ts`](SyncedClock::get_probe_ts) call.
    /// `local_recv_usec` is the local wall-clock time when the datagram arrived.
    ///
    /// Returns the estimated one-way delay for this datagram in microseconds,
    /// or `0` if the clock is not yet synchronised.
    pub fn update_with_probe(&mut self, remote_send_ts24: Counter24, local_recv_usec: u64) -> u32 {
        self.sync
            .on_authenticated_datagram_timestamp(remote_send_ts24, local_recv_usec)
    }

    /// Update the clock with sync data received on a sync datagram.
    ///
    /// `peer_min_delta_ts24` is the peer's current minimum delta as returned
    /// by [`get_sync_delta`](SyncedClock::get_sync_delta).
    pub fn update_with_sync(&mut self, peer_min_delta_ts24: Counter24) {
        self.sync.on_peer_min_delta_ts24(peer_min_delta_ts24);
    }

    /// Generate the 24-bit timestamp to attach to outgoing data datagrams.
    pub fn get_probe_ts(&self, now_usec: u64) -> Counter24 {
        Counter24::new(TimeSynchroniser::local_time_to_datagram_ts24(now_usec))
    }

    /// Return the number of whole milliseconds since the clock was started.
    pub fn local_ms(&self, now_usec: u64) -> u64 {
        now_usec.saturating_sub(self.start_usec) / 1000
    }

    /// Return the estimated correction in milliseconds to apply to local time
    /// to align with the paired remote clock.
    ///
    /// Returns `None` if the clock is not yet synchronised.
    pub fn correction_ms(&self) -> Option<i64> {
        self.correction_usec().map(|v| v / 1000)
    }

    /// Return the estimated correction in microseconds to apply to local time
    /// to align with the paired remote clock.
    ///
    /// Returns `None` if the clock is not yet synchronised.
    pub fn correction_usec(&self) -> Option<i64> {
        if !self.sync.is_synchronized() {
            return None;
        }

        let ticks = (self.sync.remote_time_delta_usec() >> TIME_23_LOST_BITS) as i32;
        let signed_ticks = if ticks >= (1 << 22) {
            ticks - (1 << 23)
        } else {
            ticks
        };
        Some((signed_ticks as i64) << TIME_23_LOST_BITS)
    }

    /// Returns the current minimum delta (24-bit) for sending to the peer
    /// on sync datagrams.
    #[inline]
    pub fn get_sync_delta(&self) -> Counter24 {
        self.sync.min_delta_ts24()
    }

    /// Returns the minimum one-way delay seen so far (microseconds).
    #[inline]
    pub fn minimum_one_way_delay_usec(&self) -> u32 {
        self.sync.minimum_one_way_delay_usec()
    }

    /// Returns `true` if the clock has received enough data to be
    /// synchronised with its peer.
    #[inline]
    pub fn is_synchronized(&self) -> bool {
        self.sync.is_synchronized()
    }
}

impl Default for SyncedClock {
    fn default() -> Self {
        Self::new()
    }
}
