use impatience::clocks::PeerClock;
use impatience::net::{PingPacket, SyncPacket};
use impatience::net::PeerSync;
use impatience::timesync::Counter24;
use std::sync::Arc;
use std::thread;

fn is_near(x: u64, y: u64, limit: u64) -> bool {
    if x > y {
        x - y <= limit
    } else {
        y - x <= limit
    }
}

#[test]
fn peer_clock_concurrent_updates() {
    let clock = Arc::new(PeerClock::new());
    clock.start(0);

    let mut handles = Vec::new();
    for i in 0..4 {
        let c = Arc::clone(&clock);
        handles.push(thread::spawn(move || {
            for t in 0..100u64 {
                let now = t * 10_000;
                let mut probe = PingPacket::default();
                c.stamp_probe(&mut probe, now);
                let recv_time = now + 1_000 + i as u64;
                c.on_probe(&probe, recv_time);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Should not panic; synchronisation state is internally consistent.
    let _ = clock.is_synchronised();
    let _ = clock.correction_ms();
    let _ = clock.min_delta();
}

#[test]
fn peer_clock_concurrent_mixed_ops() {
    let clock = Arc::new(PeerClock::new());
    clock.start(0);
    clock.set_peer_started_at(0);

    let mut handles = Vec::new();

    // Thread that stamps outgoing packets.
    let c = Arc::clone(&clock);
    handles.push(thread::spawn(move || {
        for t in 0..100u64 {
            let now = t * 10_000;
            let mut pkt = SyncPacket::default();
            c.stamp_sync(&mut pkt, now);
        }
    }));

    // Thread that processes incoming probes.
    let c = Arc::clone(&clock);
    handles.push(thread::spawn(move || {
        for t in 0..100u64 {
            let now = t * 10_000;
            let probe = PingPacket {
                probe_ts24: Counter24::new((now / 1000) as u32).to_unsigned(),
                seq: 0,
            };
            c.on_probe(&probe, now + 1_000);
        }
    }));

    // Thread that processes incoming sync packets.
    let c = Arc::clone(&clock);
    handles.push(thread::spawn(move || {
        for t in 0..100u64 {
            let now = t * 10_000;
            let sync = SyncPacket {
                probe_ts24: 0,
                min_delta_ts24: Counter24::new((now / 1000) as u32).to_unsigned(),
            };
            c.on_sync(&sync);
        }
    }));

    for h in handles {
        h.join().unwrap();
    }

    let _ = clock.is_synchronised();
    let _ = clock.correction_ms();
    let _ = clock.remote_ms(true);
}

#[test]
fn peer_clock_clone_shares_state() {
    let a = PeerClock::new();
    a.start(1_000_000);
    let b = a.clone();

    assert_eq!(a.local_ms(), b.local_ms());
}

#[test]
fn peer_clock_full_sync() {
    let clock_delta: u64 = 10_000; // 10 ms
    let owd_usec: u32 = 5_000; // 5 ms

    let mut global_usec: u64 = 0;
    let a = PeerClock::new();
    let b = PeerClock::new();

    a.start(global_usec);
    b.start(global_usec + clock_delta);

    let mut advance = |us: u64| -> (u64, u64) {
        global_usec += us;
        (global_usec, global_usec + clock_delta)
    };

    // Exchange several rounds of probe data.
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

    // Exchange sync data.
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

    let correction_a = a.correction_ms().unwrap();
    let correction_b = b.correction_ms().unwrap();

    assert!(
        is_near(correction_a as u64, 10, 2),
        "correction_a {} not near 10 ms",
        correction_a
    );
    assert!(
        is_near(correction_b.unsigned_abs(), 10, 2),
        "correction_b {} not near -10 ms",
        correction_b
    );
}
