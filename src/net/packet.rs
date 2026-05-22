use crate::net::probe::TimeSyncProbe;
use crate::timesync::Counter24;
use rkyv::{Archive, Deserialize, Serialize};

macro_rules! impl_probe {
    ($ty:ty) => {
        impl TimeSyncProbe for $ty {
            fn remote_send_ts24(&self) -> Counter24 {
                Counter24::new(self.remote_send_ts24)
            }
            fn set_local_ts24(&mut self, ts: Counter24) {
                self.local_ts24 = ts.to_unsigned();
            }
        }
    };
}

/// TimeSync sync packet containing the peer's minimum delta.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct SyncPacket {
    pub remote_send_ts24: u32,
    pub local_ts24: u32,
    pub min_delta_ts24: u32,
}

impl_probe!(SyncPacket);

/// Ping packet with a monotonically increasing sequence number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct PingPacket {
    pub remote_send_ts24: u32,
    pub local_ts24: u32,
    pub seq: u32,
}

impl_probe!(PingPacket);

/// Pong packet echoing the sequence number of the ping it answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct PongPacket {
    pub remote_send_ts24: u32,
    pub local_ts24: u32,
    pub ping_seq: u32,
}

impl_probe!(PongPacket);

/// Unified packet type that can hold any concrete packet.
#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Debug, PartialEq, Eq))]
pub enum Packet {
    Sync(SyncPacket),
    Ping(PingPacket),
    Pong(PongPacket),
}
