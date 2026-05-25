use crate::clocks::SyncedClock;
use crate::net::traits::{apply_peer_sync, apply_probe, retrieve_peer_sync, retrieve_probe, PeerSync, Probe};
use crate::timesync::Counter24;
use std::sync::{Arc, Mutex};

macro_rules! safe_cast {
    ($type:ty, $var:expr) => {
        <$type>::try_from($var).expect("value out of range for u to i cast")
    };
}

/// A peer-aware clock abstraction that wraps [`SyncedClock`] and adds
/// remote-start tracking, generic probe/sync packet handling, and
/// remote-time estimation maths.
///
/// This is the next level of abstraction above [`SyncedClock`]: given
/// packets that implement [`Probe`] and [`PeerSync`], [`PeerClock`]
/// handles stamping outgoing packets, updating state from incoming
/// packets, and computing remote-side timestamps.
///
/// [`PeerClock`] is cheaply cloneable (backed by [`Arc`]) so it can be
/// shared between send and receive threads.
#[derive(Clone, Debug)]
pub struct PeerClock {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    clock: SyncedClock,
    peer_started_at: Option<u64>,
}

impl PeerClock {
    /// Create a new, unstarted peer clock.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                clock: SyncedClock::new(),
                peer_started_at: None,
            })),
        }
    }

    /// Start (or restart) the local clock at `now_usec`.
    pub fn start(&self, now_usec: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.clock.start(now_usec);
        inner.peer_started_at = None;
    }

    /// Record the peer's start time.
    pub fn set_peer_started_at(&self, ts: u64) {
        self.inner.lock().unwrap().peer_started_at = Some(ts);
    }

    /// Return the peer's start time, if known.
    pub fn peer_started_at(&self) -> Option<u64> {
        self.inner.lock().unwrap().peer_started_at
    }

    /// Return the local wall-clock time at which the clock was started.
    pub fn started_at(&self) -> u64 {
        self.inner.lock().unwrap().clock.started_at()
    }

    /// Stamp an outgoing probe packet with the local probe timestamp.
    pub fn stamp_probe(&self, packet: &mut impl Probe, now_usec: u64) {
        let inner = self.inner.lock().unwrap();
        apply_probe(&inner.clock, packet, now_usec);
    }

    /// Update the clock from an incoming probe packet.
    ///
    /// Returns the estimated one-way delay for this datagram in
    /// microseconds, or `0` if not yet synchronised.
    pub fn on_probe(&self, packet: &impl Probe, now_usec: u64) -> u32 {
        let mut inner = self.inner.lock().unwrap();
        retrieve_probe(&mut inner.clock, packet, now_usec)
    }

    /// Update the clock from an incoming sync packet.
    pub fn on_sync(&self, packet: &impl PeerSync) {
        let mut inner = self.inner.lock().unwrap();
        retrieve_peer_sync(&mut inner.clock, packet);
    }

    /// Stamp an outgoing sync packet with both probe and sync-delta fields.
    pub fn stamp_sync(&self, packet: &mut (impl Probe + PeerSync), now_usec: u64) {
        let inner = self.inner.lock().unwrap();
        apply_probe(&inner.clock, packet, now_usec);
        apply_peer_sync(&inner.clock, packet);
    }

    /// Local milliseconds elapsed since the clock was started.
    pub fn local_ms(&self) -> u32 {
        self.inner.lock().unwrap().clock.local_ms()
    }

    /// Estimated correction to local time in milliseconds.
    pub fn correction_ms(&self) -> Option<i64> {
        self.inner.lock().unwrap().clock.correction_ms()
    }

    /// Estimated correction to local time in microseconds.
    pub fn correction_usec(&self) -> Option<i64> {
        self.inner.lock().unwrap().clock.correction_usec()
    }

    /// Current minimum delta for sending to the peer on sync packets.
    pub fn min_delta(&self) -> Counter24 {
        self.inner.lock().unwrap().clock.get_sync_delta()
    }

    /// `true` if the clock is synchronised with its peer.
    pub fn is_synchronised(&self) -> bool {
        self.inner.lock().unwrap().clock.is_synchronised()
    }

    /// Minimum one-way delay seen so far (microseconds).
    pub fn minimum_one_way_delay_usec(&self) -> u32 {
        self.inner.lock().unwrap().clock.minimum_one_way_delay_usec()
    }

    /// Difference between local and peer start times in milliseconds.
    pub fn start_delta_ms(&self) -> Option<i64> {
        let inner = self.inner.lock().unwrap();
        let peer = inner.peer_started_at?;
        Some((inner.clock.started_at() as i64 - peer as i64) / 1000)
    }

    fn remote_to_local_inner(inner: &Inner, remote_time_ms: u32, owd_sign: i64) -> Option<i64> {
        let peer_started_at = inner.peer_started_at?;
        let correction_usec = inner.clock.correction_usec().unwrap_or(0);
        let min_owd_usec = inner.clock.minimum_one_way_delay_usec();
        let start_delta_usec: i64 = safe_cast!(i64, inner.clock.started_at()) - safe_cast!(i64, peer_started_at);
        let remote_usec: i64 = remote_time_ms as i64 * 1000 + start_delta_usec + correction_usec + owd_sign * min_owd_usec as i64;
        Some((remote_usec + if remote_usec >= 0 { 500 } else { -500 }) / 1000)
    }

    fn remote_elapsed_to_local_inner(inner: &Inner, remote_time_ms: u32) -> Option<i64> {
        let peer_started_at = inner.peer_started_at?;
        let correction_usec = inner.clock.correction_usec().unwrap_or(0);
        let start_delta_usec: i64 = safe_cast!(i64, inner.clock.started_at()) - safe_cast!(i64, peer_started_at);
        let remote_usec: i64 = remote_time_ms as i64 * 1000 - start_delta_usec - correction_usec;
        Some((remote_usec + if remote_usec >= 0 { 500 } else { -500 }) / 1000)
    }

    /// Convert a remote elapsed time (milliseconds since peer started) to the
    /// equivalent local elapsed time.
    ///
    /// This is used for latency measurement of past events: given a remote
    /// timestamp recorded at the peer, compute the equivalent local elapsed
    /// time when that event occurred. This only accounts for the start time
    /// offset between peers and does not apply one-way delay adjustment.
    pub fn remote_elapsed_to_local(&self, remote_time_ms: u32) -> Option<i64> {
        let inner = self.inner.lock().unwrap();
        Self::remote_elapsed_to_local_inner(&inner, remote_time_ms)
    }

    /// Estimate the remote peer's current time in milliseconds.
    ///
    /// `initiator` controls how the minimum one-way delay is applied:
    /// * `true` for the initiating side (client)
    /// * `false` for the responding side (server)
    pub fn remote_ms(&self, initiator: bool) -> Option<i64> {
        let inner = self.inner.lock().unwrap();
        let local_ms = inner.clock.local_ms();
        let owd_sign: i64 = if initiator { -1 } else { 1 };
        let remote_ms = Self::remote_to_local_inner(&inner, local_ms, owd_sign)?;
        Some((remote_ms + if remote_ms >= 0 { 500 } else { -500 }) / 1000)
    }
}

impl Default for PeerClock {
    fn default() -> Self {
        Self::new()
    }
}

