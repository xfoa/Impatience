# Impatience

A Rust library and CLI toolkit for measuring event-to-event latency across networked peers with automatic clock synchronisation.

## Features

- I/O-agnostic core library with no networking or threading dependencies
- Four abstraction levels: high-level instrumentation, network protocol helpers, clock primitives, and the underlying synchronisation algorithm
- UDP-based clock synchronisation using a windowed-minimum one-way delay algorithm with 24-bit truncated timestamps
- Zero-copy packet serialisation via [`rkyv`](https://crates.io/crates/rkyv)
- Handshake state machine (`Initiator` / `Responder`) for non-blocking clock establishment
- CLI example programs: `synctest` (clock sync debugging) and `latency-demo` (end-to-end latency measurement)

## Installation

### Library

```toml
[dependencies]
impatience = "1.0"
```

### CLI

```bash
cargo install impatience
```

## Quick Start

```rust
use impatience::clocks::PeerClock;
use impatience::instrumentation::Profiler;
use impatience::time;

let clock = PeerClock::new();
clock.start(time::now_usec());
let profiler = Profiler::new(clock.clone());

let mut span = profiler.start("input-to-print", clock.local_ms());
let remote_finish_ms = clock.local_ms() + 50;

if let Some(latency_ms) = profiler.finish_remote(&mut span, remote_finish_ms) {
    println!("Latency: {} ms", latency_ms);
}
```

## CLI Usage

### `synctest`

A clock synchronisation test over UDP. The server listens for handshake and sync packets; the client initiates the handshake and sends periodic pings.

```bash
# Server
impatience synctest --server 0.0.0.0 --port 7340

# Client
impatience synctest --client 192.168.1.5 --port 7340 \
    --count 100 --interval 400 --sync-interval 2000
```

### `latency-demo`

An event-to-event latency demo. The client captures keyboard input, sends it to the server after a random delay, and receives a completion timestamp. Results are printed to the terminal and saved as an HTML report.

```bash
# Server
impatience latency-demo --server 0.0.0.0 --port 7341

# Client
impatience latency-demo --client 192.168.1.5 --port 7341 \
    --sync-interval 2000 --max-delay 100
```

## Library API Levels

### Level 1: Instrumentation

[`Profiler`](src/instrumentation/profiler.rs) and [`PeerClock`](src/clocks/peer_clock.rs) provide the highest-level API for latency measurement. `Profiler::start` creates a [`Span`](src/instrumentation/span.rs); `Profiler::finish_remote` records the latency when a remote completion timestamp arrives. [`LatencyAggregator`](src/instrumentation/aggregator.rs) and [`Snapshot`](src/instrumentation/reporter.rs) provide percentile statistics and console reporting.

### Level 2: Network Protocol

[`net::Initiator`](src/net/handshake.rs) and [`net::Responder`](src/net/handshake.rs) implement the two-way handshake that establishes peer start times. [`SyncScheduler`](src/net/sync_scheduler.rs) tracks when to emit periodic sync heartbeats. The packet types ([`Packet`](src/net/packets.rs), [`StartClockPacket`](src/net/packets.rs), [`SyncPacket`](src/net/packets.rs), etc.) are archived with `rkyv` and can be serialised via `Packet::to_bytes` and `Packet::from_bytes`.

```rust
use impatience::net::{Initiator, Responder, SyncScheduler, Packet, StartClockPacket};
use impatience::time;

let mut hs = Initiator::new(time::now_usec());
let start_pkt = hs.initial_packet();
let bytes = Packet::StartClock(start_pkt).to_bytes().unwrap();

let start = StartClockPacket { started_at: 1_000_000 };
let (ack, peer_started_at) = Responder::on_start_clock(&start, 2_000_000);
```

### Level 3: Clock Primitives

[`SyncedClock`](src/clocks/synced_clock.rs) exposes raw probe and sync update methods plus correction values. It is single-threaded and does not track peer start times; callers must manage thread safety and peer-start tracking themselves.

### Level 4: Algorithm Core

[`TimeSynchroniser`](src/timesync/synchroniser.rs) and [`Counter24`](src/timesync/counter.rs) expose the underlying windowed-minimum one-way delay algorithm and rollover-safe fixed-bit-width counter arithmetic. This layer is suitable for porting the algorithm to other languages or experimenting with custom windowing strategies.

## Interoperability

### Wire Format

Built-in packet types use `rkyv` for serialisation. The exact byte layout depends on the `rkyv` archived struct format. Non-Rust peers have two options:

1. Link an `rkyv`-compatible deserializer.
2. Parse the archived bytes directly, matching the field layout of the Rust structs.

The packet variants are:

| Packet | Payload |
|--------|---------|
| `StartClock` | `started_at: u64` |
| `AckStartClock` | `started_at: u64`, `peer_started_at: u64` |
| `Sync` | `timestamp: u32` (24-bit truncated), `min_delta_ts24: u32` (24-bit) |
| `Ping` | `timestamp: u32` |
| `Pong` | `timestamp: u32` |
| `InputEvent` | `event_id: u64`, `local_ms: u32` |
| `StatsBatch` | `events: Vec<StatsEvent>` |

### Clock Synchronisation Protocol

After the initial `StartClock` / `AckStartClock` handshake, peers periodically exchange `SyncPacket` values. Each packet contains:

- `timestamp`: Sender local time, truncated to 24 bits.
- `min_delta_ts24`: Sender's current minimum one-way delay estimate, also 24-bit truncated.

The receiver expands the truncated timestamp to a full 64-bit value with `Counter24::expand_from_truncated`, then feeds it into `TimeSynchroniser::on_authenticated_datagram_timestamp`. The small timestamp size limits per-packet overhead while rollover-safe counter arithmetic handles 24-bit wraparound.

### Custom Packet Types

Custom wire formats (JSON, Protobuf, Cap'n Proto, etc.) are supported by implementing the [`Probe`](src/net/traits.rs) and [`PeerSync`](src/net/traits.rs) traits. `PeerClock` and `SyncedClock` accept any type implementing these traits, so the built-in `rkyv` packet types are optional.

### Thread Safety

`PeerClock` uses `Arc<Mutex<Inner>>` internally and is `Clone + Send + Sync`. The lower-level types (`TimeSynchroniser`, `SyncedClock`, `WindowedMinTS24`) are single-threaded; concurrent access requires external synchronisation.

## Architecture

```
impatience
|-- instrumentation   # Profiler, Span, LatencyAggregator, Reporter, SVG
|-- clocks            # PeerClock, SyncedClock, format helpers
|-- net               # Handshake, SyncScheduler, packet traits, packet types
|-- timesync          # Counter types, TimeSynchroniser, WindowedMinTS24
|-- time              # Wall-clock time utilities
|-- cli               # Binary: synctest and latency-demo commands
```

## Requirements

- Rust 1.95 or later

## License

GPL-3.0, see [LICENSE.md](LICENSE.md) for details.
