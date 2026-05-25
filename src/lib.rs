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
//! ```
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
//! // Simulate receiving a remote completion timestamp.
//! // In a real app this arrives over the network from the peer.
//! let remote_finish_ms = clock.local_ms() + 50;
//!
//! if let Some(latency_ms) = profiler.finish_remote(&mut span, remote_finish_ms) {
//!     assert!(latency_ms >= 50);
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
//! ```
//! use impatience::clocks::PeerClock;
//! use impatience::net::{Initiator, Responder, SyncScheduler};
//! use impatience::net::{Packet, StartClockPacket, SyncPacket};
//! use impatience::time;
//!
//! // Client: create handshake initiator and get the first packet
//! let mut hs = Initiator::new(time::now_usec());
//! let start_pkt = hs.initial_packet();
//! let bytes = Packet::StartClock(start_pkt).to_bytes().unwrap();
//! // ... send `bytes` to the server over UDP ...
//!
//! // Server: respond to the StartClock
//! let start_pkt = StartClockPacket { started_at: 1_000_000 };
//! let (ack, peer_started_at) = Responder::on_start_clock(&start_pkt, 2_000_000);
//! assert_eq!(peer_started_at, 1_000_000);
//!
//! // Client: set up a sync scheduler
//! let clock = PeerClock::new();
//! let mut scheduler = SyncScheduler::new(2000, time::now_usec());
//! // Not yet time to send (just created)
//! assert!(!scheduler.should_send(time::now_usec()));
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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

/// Clock abstractions for local and peer time tracking.
#[cfg(feature = "std")]
pub mod clocks;
/// Latency measurement and aggregation instrumentation.
#[cfg(feature = "std")]
pub mod instrumentation;
/// Network protocol helpers: handshake, scheduling, and packet traits.
#[cfg(feature = "std")]
pub mod net;
/// Wall-clock time utilities.
#[cfg(feature = "std")]
pub mod time;
/// Low-level time synchronisation algorithm and counter types.
pub mod timesync;
