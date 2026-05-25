use crate::net::packets::{AckStartClockPacket, Packet, StartClockPacket};

pub enum HandshakeProgress {
    Pending,
    Complete { peer_started_at: u64 },
    Ignored,
}

pub struct Initiator {
    started_at: u64,
    retries: u32,
    max_retries: u32,
}

impl Initiator {
    pub fn new(started_at: u64) -> Self {
        Self {
            started_at,
            retries: 0,
            max_retries: 50,
        }
    }

    pub fn with_max_retries(started_at: u64, max_retries: u32) -> Self {
        Self {
            started_at,
            retries: 0,
            max_retries,
        }
    }

    pub fn initial_packet(&self) -> StartClockPacket {
        StartClockPacket {
            started_at: self.started_at,
        }
    }

    pub fn retries(&self) -> u32 {
        self.retries
    }

    pub fn exhausted(&self) -> bool {
        self.retries >= self.max_retries
    }

    pub fn record_retry(&mut self) {
        self.retries = self.retries.saturating_add(1);
    }

    pub fn on_receive(&mut self, packet: &Packet) -> HandshakeProgress {
        match packet {
            Packet::AckStartClock(ack) => HandshakeProgress::Complete {
                peer_started_at: ack.started_at,
            },
            _ => HandshakeProgress::Ignored,
        }
    }
}

pub struct Responder;

impl Responder {
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
