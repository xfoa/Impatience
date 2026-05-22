use crate::clock::SyncedClock;
use crate::timesync::Counter24;

pub trait TimeSyncProbe {
    fn remote_send_ts24(&self) -> Counter24;
    fn set_local_ts24(&mut self, ts: Counter24);
}

/// Stamp `header` with the local probe timestamp derived from `now_usec`.
///
/// Works with any [`TimeSyncProbe`] implementor, including [`SyncPacket`].
pub fn apply_probe<D: TimeSyncProbe>(
    clock: &SyncedClock,
    header: &mut D,
    now_usec: u64,
) {
    header.set_local_ts24(clock.get_probe_ts(now_usec));
}

/// Consume a remote probe timestamp from `header` and update the clock.
///
/// Works with any [`TimeSyncProbe`] implementor, including [`SyncPacket`].
pub fn retrieve_probe<D: TimeSyncProbe>(
    clock: &mut SyncedClock,
    header: &D,
    local_recv_usec: u64,
) -> u32 {
    clock.update_with_probe(header.remote_send_ts24(), local_recv_usec)
}
