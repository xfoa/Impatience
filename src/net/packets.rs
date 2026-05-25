use crate::net::traits::{PeerSync, Probe};
use crate::timesync::Counter24;
use rkyv::{Archive, Deserialize, Serialize};

impl Packet {
    /// Serialise the packet to bytes using rkyv.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
            .map(|v| v.into_vec())
            .map_err(|e| format!("serialise error: {}", e))
    }

    /// Deserialise a packet from bytes using rkyv.
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
    /// 24-bit probe timestamp set by the sender.
    pub probe_ts24: u32,
    /// 24-bit minimum delta for clock synchronisation.
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
    /// 24-bit probe timestamp set by the sender.
    pub probe_ts24: u32,
    /// Monotonically increasing sequence number.
    pub seq: u32,
}

impl_probe!(PingPacket);

/// Pong packet echoing the sequence number of the ping it answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct PongPacket {
    /// 24-bit probe timestamp set by the sender.
    pub probe_ts24: u32,
    /// Sequence number of the ping this pong answers.
    pub ping_seq: u32,
}

impl_probe!(PongPacket);

/// StartClock packet instructing the peer to start its clock at the given time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct StartClockPacket {
    /// Local clock start time in microseconds.
    pub started_at: u64,
}

/// AckStartClock packet confirming the peer's clock start time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct AckStartClockPacket {
    /// Server's clock start time in microseconds.
    pub started_at: u64,
}

/// Input event packet sent by the client to the server.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct InputEventPacket {
    /// 24-bit probe timestamp set by the sender.
    pub probe_ts24: u32,
    /// Monotonically increasing sequence number.
    pub seq: u32,
    /// Character code of the input event.
    pub ch: u8,
    /// Artificial delay applied before sending (ms).
    pub delay_ms: u32,
}

impl_probe!(InputEventPacket);

/// A single print event recorded by the server.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Clone, Copy, Debug, PartialEq, Eq))]
pub struct StatsEvent {
    /// Sequence number of the original input event.
    pub seq: u32,
    /// Server's local time when the event was printed (ms).
    pub server_print_ms: u32,
}

/// Stats batch packet sent by the server back to the client.
#[derive(Clone, Debug, Default, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, PartialEq, Eq))]
pub struct StatsBatchPacket {
    /// 24-bit probe timestamp set by the sender.
    pub probe_ts24: u32,
    /// Collected print events to report.
    pub events: Vec<StatsEvent>,
}

impl_probe!(StatsBatchPacket);

/// Unified packet type that can hold any concrete packet.
#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, PartialEq, Eq))]
pub enum Packet {
    /// A sync packet carrying minimum delta for clock synchronisation.
    Sync(SyncPacket),
    /// A ping packet with a sequence number.
    Ping(PingPacket),
    /// A pong packet echoing a ping's sequence number.
    Pong(PongPacket),
    /// A StartClock packet instructing the peer to start its clock.
    StartClock(StartClockPacket),
    /// An AckStartClock packet confirming the peer's clock start time.
    AckStartClock(AckStartClockPacket),
    /// An input event packet sent by the client to the server.
    InputEvent(InputEventPacket),
    /// A stats batch packet containing collected print events.
    StatsBatch(StatsBatchPacket),
}
