use crate::clocks::SyncedClock;
use crate::timesync::Counter24;

/// Trait for types representing a time-sync probe packet.
///
/// Probe packets carry a 24-bit remote-send timestamp that the receiver
/// passes to [`SyncedClock::update_with_probe`](crate::clock::SyncedClock::update_with_probe),
/// and a local timestamp field that the sender fills via [`apply_probe`].
pub trait Probe {
    fn remote_send_ts(&self) -> Counter24;
    fn set_local_ts(&mut self, ts: Counter24);
}

/// Stamp `header` with the local probe timestamp derived from `now_usec`.
///
/// Works with any [`Probe`] implementor.
pub fn apply_probe(
    clock: &SyncedClock,
    header: &mut impl Probe,
    now_usec: u64,
) {
    header.set_local_ts(clock.get_probe_ts(now_usec));
}

/// Consume a remote probe timestamp from `header` and update the clock.
///
/// Works with any [`Probe`] implementor.
pub fn retrieve_probe(
    clock: &mut SyncedClock,
    header: &impl Probe,
    local_recv_usec: u64,
) -> u32 {
    clock.update_with_probe(header.remote_send_ts(), local_recv_usec)
}

/// Trait for types representing a time-synchronisation packet.
///
/// Sync packets carry the sender's current minimum delta, which the
/// receiver passes to [`SyncedClock::update_with_sync`](crate::clock::SyncedClock::update_with_sync).
pub trait PeerSync {
    fn min_delta_ts(&self) -> Counter24;
    fn set_min_delta_ts(&mut self, ts: Counter24);
}

/// Stamp `header` with the current sync delta derived from `clock`.
///
/// Works with any [`PeerSync`] implementor.
pub fn apply_peer_sync(clock: &SyncedClock, header: &mut impl PeerSync) {
    header.set_min_delta_ts(clock.get_sync_delta());
}

/// Consume a peer sync delta from `header` and update the clock.
///
/// Works with any [`PeerSync`] implementor.
pub fn retrieve_peer_sync(clock: &mut SyncedClock, header: &impl PeerSync) {
    clock.update_with_sync(header.min_delta_ts());
}
