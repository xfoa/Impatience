# Impatience

A Rust library for measuring event-to-event latency across networked peers with automatic clock synchronisation.

The library is I/O-agnostic: it has zero dependencies on `std::net` or threading. Bring your own UDP socket and event loop. Two example CLI programs (`sync-test` and `latency-demo`) are included to demonstrate real-world usage.

## Features

- **Four abstraction levels** from high-level instrumentation down to the raw synchronisation algorithm
- **UDP-based clock sync** using a windowed-minimum one-way-delay estimator with 24-bit truncated timestamps
- **Zero-copy serialisation** of packet types via [`rkyv`](https://crates.io/crates/rkyv)
- **Non-blocking handshake** (`Initiator` / `Responder`) for establishing peer clocks
- **Optional features**: `serde` (enabled by default), `uncertainty` (for confidence-interval calculations)

## Installation

```toml
[dependencies]
impatience = "1.0"
```

Enable the `uncertainty` feature for statistical confidence intervals:

```toml
[dependencies]
impatience = { version = "1.0", features = ["uncertainty"] }
```

The CLI tools can be installed with:

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

For a complete networked example with handshake and sync scheduling, see the crate-level documentation (`cargo doc --open`).

## CLI Tools

`impatience` ships with two example programs.

### `sync-test`

Debug clock synchronisation between two machines over UDP.

```bash
# Server
impatience sync-test --server 0.0.0.0 --port 7340

# Client
impatience sync-test --client 192.168.1.5 --port 7340 \
    --count 100 --interval 400 --sync-interval 2000
```

### `latency-demo`

Measure end-to-end event latency. The client sends keyboard input to the server after a random delay; the server echoes a completion timestamp. Results are printed to the terminal and saved as an HTML report.

```bash
# Server
impatience latency-demo --server 0.0.0.0 --port 7341

# Client
impatience latency-demo --client 192.168.1.5 --port 7341 \
    --sync-interval 2000 --max-delay 100
```

## Library Overview

| Module | Purpose |
|--------|---------|
| `instrumentation` | [`Profiler`](src/instrumentation/profiler.rs), [`Span`](src/instrumentation/span.rs), [`LatencyAggregator`](src/instrumentation/aggregator.rs) |
| `clocks` | [`PeerClock`](src/clocks/peer_clock.rs) (thread-safe), [`SyncedClock`](src/clocks/synced_clock.rs) (raw), formatting helpers |
| `net` | Handshake ([`Initiator`](src/net/handshake.rs) / [`Responder`](src/net/handshake.rs)), [`SyncScheduler`](src/net/sync_scheduler.rs), packet types |
| `timesync` | [`TimeSynchroniser`](src/timesync/synchroniser.rs) algorithm and rollover-safe [`Counter24`](src/timesync/counter.rs) |
| `time` | Wall-clock time utilities |

### Level 1: Instrumentation

Use [`Profiler`](src/instrumentation/profiler.rs) and [`PeerClock`](src/clocks/peer_clock.rs) for application-level latency tracking. [`Profiler::start`](src/instrumentation/profiler.rs) creates a [`Span`](src/instrumentation/span.rs); [`Profiler::finish_remote`](src/instrumentation/profiler.rs) records the latency when a remote completion timestamp arrives. [`LatencyAggregator`](src/instrumentation/aggregator.rs) and [`Snapshot`](src/instrumentation/reporter.rs) provide percentile statistics and console reporting.

### Level 2: Network Protocol

Use [`net::Initiator`](src/net/handshake.rs) and [`net::Responder`](src/net/handshake.rs) for the two-way handshake that establishes peer start times. [`SyncScheduler`](src/net/sync_scheduler.rs) tracks when to emit periodic sync heartbeats. Packet types ([`Packet`](src/net/packets.rs), [`StartClockPacket`](src/net/packets.rs), [`SyncPacket`](src/net/packets.rs), etc.) are archived with `rkyv` and serialised via [`Packet::to_bytes`](src/net/packets.rs) and [`Packet::from_bytes`](src/net/packets.rs).

### Level 3: Clock Primitives

Use [`SyncedClock`](src/clocks/synced_clock.rs) when you need raw probe and sync update methods plus correction values without the thread-safe [`PeerClock`](src/clocks/peer_clock.rs) wrapper. It is single-threaded and does not track peer start times; callers must manage thread safety and peer-start tracking themselves.

### Level 4: Algorithm Core

Use [`TimeSynchroniser`](src/timesync/synchroniser.rs) and [`Counter24`](src/timesync/counter.rs) to study or extend the windowed-minimum one-way delay algorithm and rollover-safe fixed-bit-width counter arithmetic. This layer is suitable for porting the algorithm to other languages or experimenting with custom windowing strategies.

## Interoperability

### Wire Format

Built-in packet types are serialised with [`rkyv`](https://crates.io/crates/rkyv). Non-Rust peers must either link an `rkyv` deserializer or parse the archived bytes directly.

Custom formats (JSON, Protobuf, etc.) are supported by implementing the [`Probe`](src/net/traits.rs) and [`PeerSync`](src/net/traits.rs) traits. `PeerClock` and `SyncedClock` work with any type that implements these traits, so the built-in packet types are optional.

### Protocol

Clock synchronisation runs over UDP in two phases:

1. **Handshake**: Client sends `StartClock`, server replies with `AckStartClock`.
2. **Periodic sync**: Both peers exchange `SyncPacket` containing a 24-bit truncated local timestamp and a minimum one-way-delay estimate. The receiver expands the truncated timestamp back to 64 bits using rollover-safe [`Counter24`](src/timesync/counter.rs) arithmetic.

### Thread Safety

[`PeerClock`](src/clocks/peer_clock.rs) is `Clone + Send + Sync` (backed by `Arc<Mutex<_>>`). The lower-level types (`TimeSynchroniser`, `SyncedClock`, `WindowedMinTS24`) are single-threaded.

## Requirements

- Rust 1.95 or later

## License

GPL-3.0-only. See [LICENSE.md](LICENSE.md) for details.
