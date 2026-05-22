use crate::clock::SyncedClock;
use crate::timesync::Counter24;

/// Trait for types representing a time-sync probe packet.
///
/// Probe packets carry a 24-bit remote-send timestamp that the receiver
/// passes to [`SyncedClock::update_with_probe`](crate::clock::SyncedClock::update_with_probe),
/// and a local timestamp field that the sender fills via [`apply_probe`].
pub trait Probe {
    fn remote_send_ts24(&self) -> Counter24;
    fn set_local_ts24(&mut self, ts: Counter24);
}

/// Stamp `header` with the local probe timestamp derived from `now_usec`.
///
/// Works with any [`Probe`] implementor.
pub fn apply_probe<D: Probe>(
    clock: &SyncedClock,
    header: &mut D,
    now_usec: u64,
) {
    header.set_local_ts24(clock.get_probe_ts(now_usec));
}

/// Consume a remote probe timestamp from `header` and update the clock.
///
/// Works with any [`Probe`] implementor.
pub fn retrieve_probe<D: Probe>(
    clock: &mut SyncedClock,
    header: &D,
    local_recv_usec: u64,
) -> u32 {
    clock.update_with_probe(header.remote_send_ts24(), local_recv_usec)
}

/// Trait for types representing a time-synchronisation packet.
///
/// Sync packets carry the sender's current minimum delta, which the
/// receiver passes to [`SyncedClock::update_with_sync`](crate::clock::SyncedClock::update_with_sync).
/// Unlike probes, sync packets are self-contained, so no apply/retrieve
/// helper functions are provided.
pub trait PeerSync {
    fn min_delta_ts24(&self) -> Counter24;
}
