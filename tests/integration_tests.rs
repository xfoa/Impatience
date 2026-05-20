use impatience::timesync::{Counter16, Counter23, Counter24, TimeSynchroniser, WindowedMinTS24};
use impatience::timesync::synchroniser::{
    TIME_16_ERROR_BOUND, TIME_23_ERROR_BOUND,
};

struct PcgRandom {
    state: u64,
    inc: u64,
}

impl PcgRandom {
    fn seed(y: u64, x: u64) -> Self {
        let mut r = Self { state: 0, inc: (y << 1) | 1 };
        r.next();
        r.state = r.state.wrapping_add(x);
        r.next();
        r
    }

    fn next(&mut self) -> u32 {
        let oldstate = self.state;
        self.state = oldstate.wrapping_mul(6364136223846793005) + self.inc;
        let xorshifted = (((oldstate >> 18) ^ oldstate) >> 27) as u32;
        let rot = (oldstate >> 59) as u32;
        (xorshifted >> rot) | (xorshifted << ((0u32.wrapping_sub(rot)) & 31))
    }
}

fn is_near(x: u64, y: u64, limit: u64) -> Option<u64> {
    let delta = if x > y { x - y } else { y - x };
    if delta <= limit {
        Some(delta)
    } else {
        None
    }
}

#[test]
fn test_windowed_min_ts24() {
    let mut window = WindowedMinTS24::new();

    let window_length_time: u64 = 100;
    let trials = 10;

    for i in 0..trials * window_length_time {
        let timestamp = i;
        let value = Counter24::new(i as u32);
        window.update(value, timestamp, window_length_time);
        let smallest = window.best().to_unsigned();

        let delta = i - smallest as u64;
        if i <= 100 {
            assert!(
                smallest <= 1,
                "error too high during initial step up: i={i} -> {smallest} back by {}",
                i - smallest as u64
            );
        } else {
            let error = 100 - delta;
            assert!(
                error <= 50,
                "error too high during step up: i={i} -> {smallest} back by {}",
                i - smallest as u64
            );
        }
    }

    window.reset(Default::default());
    assert_eq!(window.best().to_unsigned(), 0, "Reset does not produce 0");

    for i in (1..=trials * window_length_time).rev() {
        let timestamp = i;
        let value = Counter24::new(i as u32);
        window.update(value, timestamp, window_length_time);
        let smallest = window.best().to_unsigned();
        assert_eq!(
            smallest,
            i as u32,
            "error too high during step down: i={i} -> {smallest} back by {}",
            i - smallest as u64
        );
    }
}

fn run_two_rounds(clock_delta: u64, owd_usec: u32) -> bool {
    let mut sync_a = TimeSynchroniser::new();
    let mut sync_b = TimeSynchroniser::new();

    assert!(!sync_a.is_synchronized());
    assert!(!sync_b.is_synchronized());

    let mut global_usec: u64 = 0;

    let mut advance = |us: u64| {
        global_usec += us;
        (global_usec, global_usec + clock_delta)
    };

    let (local_usec_a, _local_usec_b) = advance(owd_usec as u64);
    let ts_a = TimeSynchroniser::local_time_to_datagram_ts24(local_usec_a);

    let (_local_usec_a, local_usec_b) = advance(owd_usec as u64);
    let owd_a_to_b = sync_b.on_authenticated_datagram_timestamp(
        Counter24::new(ts_a),
        local_usec_b,
    );
    assert_eq!(owd_a_to_b, 0);
    assert!(!sync_b.is_synchronized());

    let (_local_usec_a, local_usec_b) = advance(owd_usec as u64);
    let ts_b = TimeSynchroniser::local_time_to_datagram_ts24(local_usec_b);

    let (local_usec_a, _local_usec_b) = advance(owd_usec as u64);
    let owd_b_to_a = sync_a.on_authenticated_datagram_timestamp(
        Counter24::new(ts_b),
        local_usec_a,
    );
    assert_eq!(owd_b_to_a, 0);
    assert!(!sync_a.is_synchronized());

    let (local_usec_a, _local_usec_b) = advance(owd_usec as u64);
    let ts_a = TimeSynchroniser::local_time_to_datagram_ts24(local_usec_a);
    let min_delta_ts24_a = sync_a.min_delta_ts24();

    let (_local_usec_a, local_usec_b) = advance(owd_usec as u64);
    let owd_a_to_b = sync_b.on_authenticated_datagram_timestamp(
        Counter24::new(ts_a),
        local_usec_b,
    );
    assert_eq!(owd_a_to_b, 0);
    assert!(!sync_b.is_synchronized());

    sync_b.on_peer_min_delta_ts24(min_delta_ts24_a);
    assert!(sync_b.is_synchronized());

    let (_local_usec_a, local_usec_b) = advance(owd_usec as u64);
    let ts_b = TimeSynchroniser::local_time_to_datagram_ts24(local_usec_b);
    let min_delta_ts24_b = sync_b.min_delta_ts24();

    let (local_usec_a, _local_usec_b) = advance(owd_usec as u64);
    let owd_b_to_a = sync_a.on_authenticated_datagram_timestamp(
        Counter24::new(ts_b),
        local_usec_a,
    );
    assert_eq!(owd_b_to_a, 0);
    assert!(!sync_a.is_synchronized());

    sync_a.on_peer_min_delta_ts24(min_delta_ts24_b);
    assert!(sync_a.is_synchronized());

    let min_owd_a = sync_a.minimum_one_way_delay_usec();
    let min_owd_b = sync_b.minimum_one_way_delay_usec();

    assert!(
        is_near(min_owd_a as u64, owd_usec as u64, TIME_23_ERROR_BOUND as u64).is_some(),
        "min_owd_a {} not near {owd_usec}",
        min_owd_a
    );
    assert!(
        is_near(min_owd_b as u64, owd_usec as u64, TIME_23_ERROR_BOUND as u64).is_some(),
        "min_owd_b {} not near {owd_usec}",
        min_owd_b
    );

    global_usec += owd_usec as u64;
    let local_usec_a = global_usec;
    let local_usec_b = global_usec + clock_delta;

    let expected_time_a = local_usec_a;
    let expected_time_b = local_usec_b;

    let remote_time_a_16 = sync_a.to_remote_time_16(local_usec_a).unwrap();
    let remote_time_b_16 = sync_b.to_remote_time_16(local_usec_b).unwrap();

    global_usec += owd_usec as u64;
    let local_usec_a = global_usec;
    let local_usec_b = global_usec + clock_delta;

    let recovered_time_a =
        TimeSynchroniser::from_local_time_16(local_usec_a, Counter16::new(remote_time_b_16));
    let recovered_time_b =
        TimeSynchroniser::from_local_time_16(local_usec_b, Counter16::new(remote_time_a_16));

    assert!(
        is_near(expected_time_a, recovered_time_a, TIME_16_ERROR_BOUND as u64).is_some(),
        "16-bit time sync failed: expected={expected_time_a} recovered={recovered_time_a}"
    );
    assert!(
        is_near(expected_time_b, recovered_time_b, TIME_16_ERROR_BOUND as u64).is_some(),
        "16-bit time sync failed: expected={expected_time_b} recovered={recovered_time_b}"
    );

    global_usec += owd_usec as u64;
    let local_usec_a = global_usec;
    let local_usec_b = global_usec + clock_delta;

    let expected_time_a = local_usec_a;
    let expected_time_b = local_usec_b;

    let remote_time_a_23 = sync_a.to_remote_time_23(local_usec_a).unwrap();
    let remote_time_b_23 = sync_b.to_remote_time_23(local_usec_b).unwrap();

    global_usec += owd_usec as u64;
    let local_usec_a = global_usec;
    let local_usec_b = global_usec + clock_delta;

    let recovered_time_a =
        TimeSynchroniser::from_local_time_23(local_usec_a, Counter23::new(remote_time_b_23));
    let recovered_time_b =
        TimeSynchroniser::from_local_time_23(local_usec_b, Counter23::new(remote_time_a_23));

    assert!(
        is_near(expected_time_a, recovered_time_a, (TIME_23_ERROR_BOUND * 2) as u64).is_some(),
        "23-bit time sync failed: expected={expected_time_a} recovered={recovered_time_a}"
    );
    assert!(
        is_near(expected_time_b, recovered_time_b, (TIME_23_ERROR_BOUND * 2) as u64).is_some(),
        "23-bit time sync failed: expected={expected_time_b} recovered={recovered_time_b}"
    );

    true
}

#[test]
fn test_two_rounds() {
    const TRIALS: u32 = 1_000_000;
    let mut prng = PcgRandom::seed(1000, 0);

    for i in 0..TRIALS {
        let clock_delta = prng.next() as u64;
        let owd_usec = (prng.next() % 200_000) + 2_000; // 2..202 ms

        assert!(
            run_two_rounds(clock_delta, owd_usec),
            "Failed for i={i} clock_delta={clock_delta} owd_usec={owd_usec}"
        );
    }
}

use std::cell::RefCell;
use std::rc::Rc;

struct TestPeer {
    time_sync: TimeSynchroniser,
    global_clock: Rc<RefCell<u64>>,
    clock_delta: u64,
    smoothed_owd_usec: u32,
}

impl TestPeer {
    fn new(global_clock: Rc<RefCell<u64>>, clock_delta: u64) -> Self {
        Self {
            time_sync: TimeSynchroniser::new(),
            global_clock,
            clock_delta,
            smoothed_owd_usec: 0,
        }
    }

    fn get_usec(&self) -> u64 {
        *self.global_clock.borrow() + self.clock_delta
    }

    fn update_owd_estimate(&mut self, owd_usec: u32) {
        if self.smoothed_owd_usec == 0 {
            self.smoothed_owd_usec = owd_usec;
        } else {
            self.smoothed_owd_usec = (self.smoothed_owd_usec * 7 + owd_usec) / 8;
        }
    }

    fn get_data_timestamp(&mut self) -> u32 {
        TimeSynchroniser::local_time_to_datagram_ts24(self.get_usec())
    }

    fn get_sync(&mut self) -> (u32, u32) {
        let local_usec = self.get_usec();
        let ts = TimeSynchroniser::local_time_to_datagram_ts24(local_usec);
        let min_delta = self.time_sync.min_delta_ts24().to_unsigned();
        (ts, min_delta)
    }

    fn on_data(&mut self, timestamp: u32) {
        let local_recv_usec = self.get_usec();
        let owd_usec = self
            .time_sync
            .on_authenticated_datagram_timestamp(Counter24::new(timestamp), local_recv_usec);
        self.update_owd_estimate(owd_usec);
    }

    fn on_sync(&mut self, timestamp: u32, min_delta_ts24: u32) {
        let local_recv_usec = self.get_usec();
        let owd_usec = self
            .time_sync
            .on_authenticated_datagram_timestamp(Counter24::new(timestamp), local_recv_usec);
        self.update_owd_estimate(owd_usec);
        self.time_sync.on_peer_min_delta_ts24(Counter24::new(min_delta_ts24));
    }

    fn get_remote_timestamp_23(&mut self) -> u32 {
        self.time_sync.to_remote_time_23(self.get_usec()).unwrap_or(0)
    }

    fn convert_to_local_23(&mut self, timestamp23: u32) -> u64 {
        let local_usec = self.get_usec();
        TimeSynchroniser::from_local_time_23(local_usec, Counter23::new(timestamp23))
    }
}

fn run_simple(clock_delta_a: u64, clock_delta_b: u64, owd_usec: u32) -> bool {
    let mut prng = PcgRandom::seed(clock_delta_a, 0);
    let global_clock = Rc::new(RefCell::new(0u64));

    let mut a = TestPeer::new(Rc::clone(&global_clock), clock_delta_a);
    let mut b = TestPeer::new(Rc::clone(&global_clock), clock_delta_b);

    const ROUNDS: u32 = 100;
    for i in 0..ROUNDS {
        if i % 10 == 9 {
            let (ts, min_delta) = a.get_sync();
            *global_clock.borrow_mut() += owd_usec as u64 + (prng.next() % (owd_usec / 10)) as u64;
            b.on_sync(ts, min_delta);
        } else {
            let ts = a.get_data_timestamp();
            *global_clock.borrow_mut() += owd_usec as u64 + (prng.next() % (owd_usec / 10)) as u64;
            b.on_data(ts);
        }

        if i % 10 == 9 {
            let (ts, min_delta) = b.get_sync();
            *global_clock.borrow_mut() += owd_usec as u64 + (prng.next() % (owd_usec / 10)) as u64;
            a.on_sync(ts, min_delta);
        } else {
            let ts = b.get_data_timestamp();
            *global_clock.borrow_mut() += owd_usec as u64 + (prng.next() % (owd_usec / 10)) as u64;
            a.on_data(ts);
        }
    }

    let error_bound = owd_usec / 10;
    assert!(
        is_near(a.smoothed_owd_usec as u64, owd_usec as u64, error_bound as u64).is_some(),
        "OWD estimate out of range A: {} vs {}",
        a.smoothed_owd_usec,
        owd_usec
    );
    assert!(
        is_near(b.smoothed_owd_usec as u64, owd_usec as u64, error_bound as u64).is_some(),
        "OWD estimate out of range B: {} vs {}",
        b.smoothed_owd_usec,
        owd_usec
    );

    let a0 = a.get_usec();
    let b0 = b.get_usec();

    let timestamp_from_a = a.get_remote_timestamp_23();
    let timestamp_from_b = b.get_remote_timestamp_23();

    *global_clock.borrow_mut() += owd_usec as u64 + (prng.next() % (owd_usec / 10)) as u64;

    let timestamp_at_a = a.convert_to_local_23(timestamp_from_b);
    let timestamp_at_b = b.convert_to_local_23(timestamp_from_a);

    let min_owd_a = a.time_sync.minimum_one_way_delay_usec();
    let min_owd_b = b.time_sync.minimum_one_way_delay_usec();

    let error_bound_a = TIME_23_ERROR_BOUND * 2 + min_owd_b.saturating_sub(owd_usec);
    let error_bound_b = TIME_23_ERROR_BOUND * 2 + min_owd_a.saturating_sub(owd_usec);

    assert!(
        is_near(timestamp_at_a, a0, error_bound_a as u64).is_some(),
        "Time sync does not work A: timestamp_at_a={timestamp_at_a} a0={a0}"
    );
    assert!(
        is_near(timestamp_at_b, b0, error_bound_b as u64).is_some(),
        "Time sync does not work B: timestamp_at_b={timestamp_at_b} b0={b0}"
    );

    true
}

#[test]
fn test_simple_usage() {
    const TRIALS: u32 = 10_000;
    let mut prng = PcgRandom::seed(1000, 0);

    for i in 0..TRIALS {
        let clock_delta_a = prng.next() as u64;
        let clock_delta_b = prng.next() as u64;
        let owd_usec = (prng.next() % 200_000) + 2_000; // 2..202 ms

        assert!(
            run_simple(clock_delta_a, clock_delta_b, owd_usec),
            "Failed for i={i}"
        );
    }
}
