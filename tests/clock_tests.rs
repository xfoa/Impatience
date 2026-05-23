use impatience::clock::SyncedClock;
use impatience::timesync::{Counter24, TimeSynchroniser};

#[test]
fn test_local_ms() {
    let mut clock = SyncedClock::new();
    clock.start(1_000_000); // 1 second
    assert_eq!(clock.local_ms(2_000_000), 1000);
    assert_eq!(clock.local_ms(1_500_000), 500);
    assert_eq!(clock.local_ms(1_000_000), 0);
}

#[test]
fn test_not_synchronised_initially() {
    let clock = SyncedClock::new();
    assert!(!clock.is_synchronised());
    assert_eq!(clock.correction_ms(), None);
}

#[test]
fn test_get_probe_ts_matches_ts24() {
    let clock = SyncedClock::new();
    let now = 1_234_567_890;
    let ts24 = clock.get_probe_ts(now);
    assert_eq!(
        ts24,
        Counter24::new(TimeSynchroniser::local_time_to_datagram_ts24(now))
    );
}

fn is_near(x: u64, y: u64, limit: u64) -> bool {
    if x > y {
        x - y <= limit
    } else {
        y - x <= limit
    }
}

#[test]
fn test_basic_synchronisation() {
    let clock_delta: u64 = 10_000; // 10 ms
    let owd_usec: u32 = 5_000; // 5 ms

    let mut global_usec: u64 = 0;
    let mut a = SyncedClock::new();
    let mut b = SyncedClock::new();

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

    // Verify local_ms is unaffected by sync
    assert_eq!(a.local_ms(global_usec), global_usec / 1000);
    assert_eq!(b.local_ms(global_usec + clock_delta), global_usec / 1000);
}
