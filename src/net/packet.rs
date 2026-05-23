use crate::net::traits::{PeerSync, Probe};
use crate::timesync::Counter24;
use rkyv::{Archive, Deserialize, Serialize};

impl Packet {
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
            .map(|v| v.into_vec())
            .map_err(|e| format!("serialise error: {}", e))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut aligned = rkyv::util::AlignedVec::<8>::with_capacity(bytes.len());
        aligned.extend_from_slice(bytes);
        let archived =
            rkyv::access::<ArchivedPacket, rkyv::rancor::Error>(&aligned)
                .map_err(|e| format!("access error: {}", e))?;
        rkyv::deserialize::<Packet, rkyv::rancor::Error>(archived)
            .map_err(|e| format!("deserialise error: {}", e))
    }
}

macro_rules! impl_probe {
    ($ty:ty) => {
        impl Probe for $ty {
            fn remote_send_ts(&self) -> Counter24 {
                Counter24::new(self.probe_ts24)
            }
            fn set_local_ts(&mut self, ts: Counter24) {
                self.probe_ts24 = ts.to_unsigned();
            }
        }
    };
}

/// TimeSync sync packet containing the peer's minimum delta.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct SyncPacket {
    pub probe_ts24: u32,
    pub min_delta_ts24: u32,
}

impl_probe!(SyncPacket);

impl PeerSync for SyncPacket {
    fn min_delta_ts(&self) -> Counter24 {
        Counter24::new(self.min_delta_ts24)
    }

    fn set_min_delta_ts(&mut self, ts: Counter24) {
        self.min_delta_ts24 = ts.to_unsigned();
    }
}

/// Ping packet with a monotonically increasing sequence number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct PingPacket {
    pub probe_ts24: u32,
    pub seq: u32,
}

impl_probe!(PingPacket);

/// Pong packet echoing the sequence number of the ping it answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct PongPacket {
    pub probe_ts24: u32,
    pub ping_seq: u32,
}

impl_probe!(PongPacket);

/// StartClock packet instructing the peer to start its clock at the given time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct StartClockPacket {
    pub started_at: u64,
}

/// AckStartClock packet confirming the peer's clock start time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct AckStartClockPacket {
    pub started_at: u64,
}

/// Unified packet type that can hold any concrete packet.
#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Debug, PartialEq, Eq))]
pub enum Packet {
    Sync(SyncPacket),
    Ping(PingPacket),
    Pong(PongPacket),
    StartClock(StartClockPacket),
    AckStartClock(AckStartClockPacket),
}
