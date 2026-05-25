//! A library for instrumentation of event-to-event latency over a network.
//!
//! # Abstraction Levels
//!
//! This crate provides four layers of abstraction, from high-level convenience
//! to low-level control.  Most users will only need the top two layers.
//!
//! ## Level 1: Measure latency (instrumentation + clocks)
//!
//! If you want to measure the time between an event on one machine and its
//! completion on another, use [`Profiler`](instrumentation::Profiler) with a
//! [`PeerClock`](clocks::PeerClock):
//!
//! ```ignore
//! use impatience::clocks::PeerClock;
//! use impatience::instrumentation::Profiler;
//! use impatience::time;
//!
//! let clock = PeerClock::new();
//! clock.start(time::now_usec());
//! let profiler = Profiler::new(clock.clone());
//!
//! // Start a span when the event begins locally.
//! let mut span = profiler.start("input-to-print", clock.local_ms());
//!
//! // ... send event to peer, receive completion timestamp ...
//!
//! // Finish the span with the remote completion time.
//! if let Some(latency_ms) = profiler.finish_remote(&mut span, remote_finish_ms) {
//!     println!("latency: {} ms", latency_ms);
//! }
//! ```
//!
//! ## Level 2: Build a networked app (net + clocks)
//!
//! If you are building your own UDP-based application with clock synchronisation,
//! use [`Initiator`](net::Initiator) / [`Responder`](net::Responder) for the
//! handshake, [`SyncScheduler`](net::SyncScheduler) for periodic sync, and
//! implement [`Probe`](net::Probe) / [`PeerSync`](net::PeerSync) for your
//! packet types:
//!
//! ```ignore
//! use impatience::clocks::PeerClock;
//! use impatience::net::{Initiator, Responder, SyncScheduler, Probe, PeerSync};
//! use impatience::time;
//!
//! // Client handshake
//! let mut hs = Initiator::new(time::now_usec());
//! socket.send(&hs.initial_packet());
//!
//! // Server handshake
//! let (ack, peer_started_at) = Responder::on_start_clock(&start_pkt, time::now_usec());
//! clock.set_peer_started_at(peer_started_at);
//!
//! // Periodic sync
//! let mut scheduler = SyncScheduler::new(2000, time::now_usec());
//! if scheduler.should_send(time::now_usec()) {
//!     let mut pkt = MySyncPacket::default();
//!     clock.stamp_sync(&mut pkt, time::now_usec());
//!     socket.send(&pkt);
//! }
//! ```
//!
//! ## Level 3: Fine-grained clock control (clocks)
//!
//! If you need direct access to the clock synchronisation state without the
//! [`PeerClock`](clocks::PeerClock) wrapper, use [`SyncedClock`](clocks::SyncedClock).
//! It exposes raw probe/sync update methods and correction values, but requires
//! you to manage thread safety and peer-start tracking yourself.
//!
//! ## Level 4: Understand the algorithm (timesync)
//!
//! If you want to study or extend the clock synchronisation algorithm itself,
//! use [`TimeSynchroniser`](timesync::TimeSynchroniser) and
//! [`Counter24`](timesync::Counter24) directly.

pub mod clocks;
pub mod instrumentation;
pub mod net;
pub mod time;
pub mod timesync;
