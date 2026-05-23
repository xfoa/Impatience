use crate::timesync::{Counter16, Counter23, Counter24, TimeSynchroniser};
use std::sync::{Arc, Mutex};

/// Thread-safe, shareable wrapper around [`TimeSynchroniser`].
///
/// All mutable operations are serialized via an internal [`Mutex`],
/// and the wrapper is [`Clone`] so it can be shared cheaply across
/// threads without an outer [`Arc`].
#[derive(Clone, Debug)]
pub struct AtomicSynchroniser {
    inner: Arc<Mutex<TimeSynchroniser>>,
}

impl AtomicSynchroniser {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TimeSynchroniser::new())),
        }
    }

    /// See [`TimeSynchroniser::on_peer_min_delta_ts24`].
    pub fn on_peer_min_delta_ts24(&self, min_delta_ts24: Counter24) {
        self.inner.lock().unwrap().on_peer_min_delta_ts24(min_delta_ts24);
    }

    /// See [`TimeSynchroniser::on_authenticated_datagram_timestamp`].
    pub fn on_authenticated_datagram_timestamp(
        &self,
        remote_send_ts24: Counter24,
        local_recv_usec: u64,
    ) -> u32 {
        self.inner
            .lock()
            .unwrap()
            .on_authenticated_datagram_timestamp(remote_send_ts24, local_recv_usec)
    }

    /// See [`TimeSynchroniser::min_delta_ts24`].
    pub fn min_delta_ts24(&self) -> Counter24 {
        self.inner.lock().unwrap().min_delta_ts24()
    }

    /// See [`TimeSynchroniser::is_synchronised`].
    pub fn is_synchronised(&self) -> bool {
        self.inner.lock().unwrap().is_synchronised()
    }

    /// See [`TimeSynchroniser::minimum_one_way_delay_usec`].
    pub fn minimum_one_way_delay_usec(&self) -> u32 {
        self.inner.lock().unwrap().minimum_one_way_delay_usec()
    }

    /// See [`TimeSynchroniser::remote_time_delta_usec`].
    pub fn remote_time_delta_usec(&self) -> u32 {
        self.inner.lock().unwrap().remote_time_delta_usec()
    }

    /// See [`TimeSynchroniser::to_remote_time_16`].
    pub fn to_remote_time_16(&self, local_usec: u64) -> Option<u16> {
        self.inner.lock().unwrap().to_remote_time_16(local_usec)
    }

    /// See [`TimeSynchroniser::to_remote_time_23`].
    pub fn to_remote_time_23(&self, local_usec: u64) -> Option<u32> {
        self.inner.lock().unwrap().to_remote_time_23(local_usec)
    }

    /// See [`TimeSynchroniser::local_time_to_datagram_ts24`].
    #[inline]
    pub fn local_time_to_datagram_ts24(local_usec: u64) -> u32 {
        TimeSynchroniser::local_time_to_datagram_ts24(local_usec)
    }

    /// See [`TimeSynchroniser::from_local_time_16`].
    #[inline]
    pub fn from_local_time_16(local_usec: u64, timestamp16: Counter16) -> u64 {
        TimeSynchroniser::from_local_time_16(local_usec, timestamp16)
    }

    /// See [`TimeSynchroniser::from_local_time_23`].
    #[inline]
    pub fn from_local_time_23(local_usec: u64, timestamp23: Counter23) -> u64 {
        TimeSynchroniser::from_local_time_23(local_usec, timestamp23)
    }
}

impl Default for AtomicSynchroniser {
    fn default() -> Self {
        Self::new()
    }
}
