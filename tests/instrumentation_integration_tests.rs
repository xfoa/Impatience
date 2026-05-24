use impatience::clocks::PeerClock;
use impatience::instrumentation::Profiler;
use impatience::net::packets::{PingPacket, SyncPacket};
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

    // A clicks at local time `click_a` (in ms).
    let (click_a, _) = advance(0);
    let span = inst_a.start("click-to-injection", click_a / 1000);

    // Simulate network delay: one more advance of `owd_usec`.
    let (_, injection_b) = advance(owd_usec as u64);

    // B finishes the event at its local time (in ms).
    // Use inst_a (the local clock that started the span) to compute latency.
    let latency = inst_a.finish_remote(&span, injection_b / 1000);

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
