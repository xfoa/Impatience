use impatience::clocks::PeerClock;
use impatience::instrumentation::Profiler;
use impatience::net::{PingPacket, SyncPacket};
use impatience::net::PeerSync;

fn is_near(x: i64, y: i64, limit: i64) -> bool {
    (x - y).abs() <= limit
}

#[test]
fn profiler_event_to_event_latency() {
    let clock_delta_usec: u64 = 10_000; // 10 ms
    let owd_usec: u32 = 5_000; // 5 ms one-way

    let mut global_usec: u64 = 0;
    let a = PeerClock::new();
    let b = PeerClock::new();

    a.start(global_usec);
    b.start(global_usec + clock_delta_usec);
    a.set_peer_started_at(global_usec + clock_delta_usec);
    b.set_peer_started_at(global_usec);

    let mut advance = |us: u64| -> (u64, u64) {
        global_usec += us;
        (global_usec, global_usec + clock_delta_usec)
    };

    // Synchronise the two clocks first (same pattern as concurrent_tests.rs)
    for _ in 0..10 {
        let (local_a, _local_b) = advance(owd_usec as u64);
        let mut ping_a = PingPacket::default();
        a.stamp_probe(&mut ping_a, local_a);

        let (_local_a, local_b) = advance(owd_usec as u64);
        b.on_probe(&ping_a, local_b);

        let (_local_a, local_b) = advance(owd_usec as u64);
        let mut ping_b = PingPacket::default();
        b.stamp_probe(&mut ping_b, local_b);

        let (local_a, _local_b) = advance(owd_usec as u64);
        a.on_probe(&ping_b, local_a);
    }

    let min_delta_a = a.min_delta();
    let min_delta_b = b.min_delta();

    let mut sync_b = SyncPacket::default();
    sync_b.set_min_delta_ts(min_delta_b);
    a.on_sync(&sync_b);

    let mut sync_a = SyncPacket::default();
    sync_a.set_min_delta_ts(min_delta_a);
    b.on_sync(&sync_a);

    assert!(a.is_synchronised());
    assert!(b.is_synchronised());

    // Now measure an event sent from A to B.
    let inst_a = Profiler::new(a.clone());
    let _inst_b = Profiler::new(b.clone());

    // A starts the event at its local elapsed time.
    // After 10 rounds of probe exchange (40 advances of 5000 usec), global_usec = 200_000.
    // A started at global 0, so A's elapsed time = 200_000 usec = 200 ms.
    let start_a_ms = 200u32;
    let mut span = inst_a.start("click-to-injection", start_a_ms);

    // Simulate network delay: advance by owd_usec (5000 usec).
    // B receives at global 205_000 usec.
    // B's wall clock = 205_000 + 10_000 = 215_000 usec.
    // B started at wall clock 10_000, so B's elapsed time = 205_000 usec = 205 ms.
    let finish_b_ms = 205u32;

    // B finishes the event at its local elapsed time.
    // Use inst_a (the local clock that started the span) to compute latency.
    let latency = inst_a.finish_remote(&mut span, finish_b_ms);

    // The measured latency should be approximately the one-way delay (5 ms).
    assert!(
        latency.is_some(),
        "latency should be Some when clocks are synchronised"
    );
    let latency_ms = latency.unwrap() as i64;
    let owd_ms = owd_usec as i64 / 1000;
    assert!(
        is_near(latency_ms, owd_ms, 1),
        "latency {} not near {} ms",
        latency_ms,
        owd_ms
    );
}

