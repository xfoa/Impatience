use impatience::concurrent::{AtomicClock, AtomicSynchroniser};
use impatience::timesync::{Counter24, TimeSynchroniser};
use std::sync::atomic::{AtomicU32, Ordering};
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
fn atomic_synchroniser_concurrent_updates() {
    let sync = Arc::new(AtomicSynchroniser::new());
    let counter = Arc::new(AtomicU32::new(0));

    let mut handles = Vec::new();
    for i in 0..4 {
        let s = Arc::clone(&sync);
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                let now = c.fetch_add(10_000, Ordering::SeqCst) as u64;
                let ts = Counter24::new(TimeSynchroniser::local_time_to_datagram_ts24(now));
                s.on_authenticated_datagram_timestamp(ts, now + 1_000 + i as u64);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert!(sync.min_delta_ts24().to_unsigned() != 0 || !sync.is_synchronised());
}

#[test]
fn atomic_synchroniser_clone_shares_state() {
    let a = AtomicSynchroniser::new();
    let b = a.clone();

    let now = 1_000_000u64;
    let ts = Counter24::new(TimeSynchroniser::local_time_to_datagram_ts24(now));
    a.on_authenticated_datagram_timestamp(ts, now + 5_000);

    assert_eq!(a.min_delta_ts24(), b.min_delta_ts24());
}

#[test]
fn atomic_clock_concurrent_updates() {
    let clock = Arc::new(AtomicClock::new());
    clock.start(0);

    let mut handles = Vec::new();
    for i in 0..4 {
        let c = Arc::clone(&clock);
        handles.push(thread::spawn(move || {
            for t in 0..100u64 {
                let now = t * 10_000;
                let ts = c.get_probe_ts(now);
                c.update_with_probe(ts, now + 1_000 + i as u64);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Should not panic; synchronisation state is internally consistent.
    let _ = clock.is_synchronised();
    let _ = clock.correction_ms();
    let _ = clock.get_sync_delta();
}

#[test]
fn atomic_clock_clone_shares_state() {
    let a = AtomicClock::new();
    a.start(1_000_000);
    let b = a.clone();

    assert_eq!(a.local_ms(2_000_000), b.local_ms(2_000_000));
    assert_eq!(a.local_ms(2_000_000), 1000);
}

#[test]
fn atomic_clock_full_sync_across_threads() {
    let clock_delta: u64 = 10_000; // 10 ms
    let owd_usec: u32 = 5_000; // 5 ms

    let mut global_usec: u64 = 0;
    let a = Arc::new(AtomicClock::new());
    let b = Arc::new(AtomicClock::new());

    a.start(global_usec);
    b.start(global_usec + clock_delta);

    let mut advance = |us: u64| -> (u64, u64) {
        global_usec += us;
        (global_usec, global_usec + clock_delta)
    };

    // Exchange several rounds of probe data
    for _ in 0..10 {
        let (local_a, _local_b) = advance(owd_usec as u64);
        let ts_a = a.get_probe_ts(local_a);

        let (_local_a, local_b) = advance(owd_usec as u64);
        b.update_with_probe(ts_a, local_b);

        let (_local_a, local_b) = advance(owd_usec as u64);
        let ts_b = b.get_probe_ts(local_b);

        let (local_a, _local_b) = advance(owd_usec as u64);
        a.update_with_probe(ts_b, local_a);
    }

    // Exchange sync data
    let min_delta_a = a.get_sync_delta();
    let min_delta_b = b.get_sync_delta();

    a.update_with_sync(min_delta_b);
    b.update_with_sync(min_delta_a);

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
