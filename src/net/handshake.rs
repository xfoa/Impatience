use crate::net::packets::{AckStartClockPacket, Packet, StartClockPacket};

/// Result of feeding a received packet into the handshake state machine.
pub enum HandshakeProgress {
    /// The handshake is not yet complete; continue waiting for packets.
    Pending,
    /// The handshake completed successfully.
    Complete {
        /// The peer's clock start time in microseconds.
        peer_started_at: u64,
    },
    /// The received packet was not relevant to the handshake.
    Ignored,
}

/// Initiator-side (client) handshake state machine.
///
/// Manages retry tracking for the StartClock exchange.
/// The caller is responsible for socket I/O and retry timing.
///
/// # Example
///
/// ```
/// use impatience::net::{Initiator, HandshakeProgress};
/// use impatience::net::packets::{Packet, AckStartClockPacket};
///
/// let mut hs = Initiator::new(1_000_000);
/// let pkt = hs.initial_packet();
/// assert_eq!(pkt.started_at, 1_000_000);
///
/// // Simulate receiving the ack
/// let ack = Packet::AckStartClock(AckStartClockPacket { started_at: 2_000_000 });
/// match hs.on_receive(&ack) {
///     HandshakeProgress::Complete { peer_started_at } => {
///         assert_eq!(peer_started_at, 2_000_000);
///     }
///     _ => panic!("expected Complete"),
/// }
/// ```
pub struct Initiator {
    started_at: u64,
    retries: u32,
    max_retries: u32,
}

impl Initiator {
    /// Create a new initiator with the default max retries (50).
    pub fn new(started_at: u64) -> Self {
        Self {
            started_at,
            retries: 0,
            max_retries: 50,
        }
    }

    /// Create a new initiator with a custom max retry count.
    pub fn with_max_retries(started_at: u64, max_retries: u32) -> Self {
        Self {
            started_at,
            retries: 0,
            max_retries,
        }
    }

    /// Get the StartClock packet to send (or resend on retry).
    pub fn initial_packet(&self) -> StartClockPacket {
        StartClockPacket {
            started_at: self.started_at,
        }
    }

    /// Number of retries attempted so far.
    pub fn retries(&self) -> u32 {
        self.retries
    }

    /// Returns `true` if the maximum retry count has been reached.
    pub fn exhausted(&self) -> bool {
        self.retries >= self.max_retries
    }

    /// Record that a timeout occurred and a retry will be attempted.
    pub fn record_retry(&mut self) {
        self.retries = self.retries.saturating_add(1);
    }

    /// Feed a received packet into the handshake state machine.
    pub fn on_receive(&mut self, packet: &Packet) -> HandshakeProgress {
        match packet {
            Packet::AckStartClock(ack) => HandshakeProgress::Complete {
                peer_started_at: ack.started_at,
            },
            _ => HandshakeProgress::Ignored,
        }
    }
}

/// Responder-side (server) handshake handler.
///
/// Stateless: processes a single StartClock packet and returns the ack.
///
/// # Example
///
/// ```
/// use impatience::net::Responder;
/// use impatience::net::packets::StartClockPacket;
///
/// let pkt = StartClockPacket { started_at: 1_000_000 };
/// let (ack, peer_started_at) = Responder::on_start_clock(&pkt, 2_000_000);
/// assert_eq!(ack.started_at, 2_000_000);
/// assert_eq!(peer_started_at, 1_000_000);
/// ```
pub struct Responder;

impl Responder {
    /// Process a received StartClock packet and produce the ack to send back.
    ///
    /// Returns the ack packet and the peer's start time.
    pub fn on_start_clock(
        packet: &StartClockPacket,
        local_now_usec: u64,
    ) -> (AckStartClockPacket, u64) {
        let ack = AckStartClockPacket {
            started_at: local_now_usec,
        };
        (ack, packet.started_at)
    }
}
