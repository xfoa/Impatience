use crate::clock::SyncedClock;
use crate::timesync::Counter24;
use std::sync::{Arc, Mutex};

/// Thread-safe, shareable wrapper around [`SyncedClock`].
///
/// All mutable operations are serialized via an internal [`Mutex`],
/// and the wrapper is [`Clone`] so it can be shared cheaply across
/// threads without an outer [`Arc`].
#[derive(Clone, Debug)]
pub struct AtomicClock {
    inner: Arc<Mutex<SyncedClock>>,
}

impl AtomicClock {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SyncedClock::new())),
        }
    }

    /// See [`SyncedClock::start`].
    pub fn start(&self, now_usec: u64) {
        self.inner.lock().unwrap().start(now_usec);
    }

    /// See [`SyncedClock::update_with_probe`].
    pub fn update_with_probe(&self, remote_send_ts24: Counter24, local_recv_usec: u64) -> u32 {
        self.inner
            .lock()
            .unwrap()
            .update_with_probe(remote_send_ts24, local_recv_usec)
    }

    /// See [`SyncedClock::update_with_sync`].
    pub fn update_with_sync(&self, peer_min_delta_ts24: Counter24) {
        self.inner.lock().unwrap().update_with_sync(peer_min_delta_ts24);
    }

    /// See [`SyncedClock::get_probe_ts`].
    pub fn get_probe_ts(&self, now_usec: u64) -> Counter24 {
        self.inner.lock().unwrap().get_probe_ts(now_usec)
    }

    /// See [`SyncedClock::local_ms`].
    pub fn local_ms(&self, now_usec: u64) -> u64 {
        self.inner.lock().unwrap().local_ms(now_usec)
    }

    /// See [`SyncedClock::correction_ms`].
    pub fn correction_ms(&self) -> Option<i64> {
        self.inner.lock().unwrap().correction_ms()
    }

    /// See [`SyncedClock::get_sync_delta`].
    pub fn get_sync_delta(&self) -> Counter24 {
        self.inner.lock().unwrap().get_sync_delta()
    }

    /// See [`SyncedClock::is_synchronised`].
    pub fn is_synchronised(&self) -> bool {
        self.inner.lock().unwrap().is_synchronised()
    }
}

impl Default for AtomicClock {
    fn default() -> Self {
        Self::new()
    }
}
