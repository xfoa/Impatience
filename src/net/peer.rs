use crate::clock::SyncedClock;
use crate::net::traits::{apply_probe, retrieve_probe, PeerSync, PeerSyncMut, Probe};
use crate::timesync::Counter24;
use std::sync::{Arc, Mutex};

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
        inner.clock.update_with_sync(packet.min_delta_ts());
    }

    /// Stamp an outgoing sync packet with both probe and sync-delta fields.
    pub fn stamp_sync(&self, packet: &mut (impl Probe + PeerSyncMut), now_usec: u64) {
        let inner = self.inner.lock().unwrap();
        apply_probe(&inner.clock, packet, now_usec);
        packet.set_min_delta_ts(inner.clock.get_sync_delta());
    }

    /// Local milliseconds elapsed since the clock was started.
    pub fn local_ms(&self, now_usec: u64) -> u64 {
        self.inner.lock().unwrap().clock.local_ms(now_usec)
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
    pub fn is_synchronized(&self) -> bool {
        self.inner.lock().unwrap().clock.is_synchronized()
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

    /// Estimate the remote peer's time in milliseconds.
    ///
    /// `owd_sign` controls how the minimum one-way delay is applied:
    /// * `-1` for the initiating side (client)
    /// * `+1` for the responding side (server)
    pub fn remote_ms(&self, now_usec: u64, owd_sign: i64) -> Option<i64> {
        let inner = self.inner.lock().unwrap();
        let peer = inner.peer_started_at?;
        let correction_usec = inner.clock.correction_usec().unwrap_or(0);
        let min_owd_usec = inner.clock.minimum_one_way_delay_usec() as i64;
        let local_usec = now_usec.saturating_sub(inner.clock.started_at());
        let start_delta_usec = inner.clock.started_at() as i64 - peer as i64;
        let remote_usec = local_usec as i64 + start_delta_usec + correction_usec + owd_sign * min_owd_usec;
        Some((remote_usec + if remote_usec >= 0 { 500 } else { -500 }) / 1000)
    }
}

impl Default for PeerClock {
    fn default() -> Self {
        Self::new()
    }
}
