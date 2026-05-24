use impatience::instrument::{Instrument, LatencyAggregator, Snapshot, Span};
use impatience::clocks::PeerClock;

#[test]
fn event_id_monotonic() {
    let clock = PeerClock::new();
    let inst = Instrument::new(clock);
    let a = inst.start("a", 0);
    let b = inst.start("b", 1);
    let c = inst.start("c", 2);
    assert!(a.event_id().0 < b.event_id().0);
    assert!(b.event_id().0 < c.event_id().0);
}

#[test]
fn span_elapsed_local() {
    let span = Span::new(impatience::instrument::EventId(1), "test", 1000);
    assert_eq!(span.elapsed(2000), 1000);
    assert_eq!(span.elapsed(1000), 0);
    assert_eq!(span.elapsed(500), 0); // saturating
}

#[test]
fn aggregator_basic_percentiles() {
    let mut agg = LatencyAggregator::new(100);
    for i in 1..=100 {
        agg.insert(i as u64 * 100);
    }
    assert_eq!(agg.count(), 100);
    assert_eq!(agg.min(), Some(100));
    assert_eq!(agg.max(), Some(10_000));
    assert_eq!(agg.p50(), Some(5_050));
    assert_eq!(agg.p95(), Some(9_505));
    assert_eq!(agg.p99(), Some(9_901));
}

#[test]
fn aggregator_capacity_eviction() {
    let mut agg = LatencyAggregator::new(5);
    agg.insert(10);
    agg.insert(20);
    agg.insert(30);
    agg.insert(40);
    agg.insert(50);
    assert_eq!(agg.count(), 5);
    agg.insert(60);
    assert_eq!(agg.count(), 5);
    assert_eq!(agg.min(), Some(20));
    assert_eq!(agg.max(), Some(60));
}

#[test]
fn snapshot_from_empty_aggregator() {
    let agg = LatencyAggregator::new(10);
    let snap = Snapshot::from_aggregator(&agg);
    assert_eq!(snap.count, 0);
    assert_eq!(snap.min, None);
    assert_eq!(snap.p50, None);
}

#[test]
fn instrument_record_and_snapshot() {
    let clock = PeerClock::new();
    let inst = Instrument::new(clock);
    inst.record_latency(1_000);
    inst.record_latency(2_000);
    inst.record_latency(3_000);
    let snap = inst.snapshot();
    assert_eq!(snap.count, 3);
    assert_eq!(snap.min, Some(1_000));
    assert_eq!(snap.p50, Some(2_000));
    assert_eq!(snap.max, Some(3_000));
}

#[test]
fn instrument_finish_remote_unsynced_returns_none() {
    let clock = PeerClock::new();
    let inst = Instrument::new(clock);
    let span = inst.start("click", 0);
    // clock is not synchronised → remote_latency_us returns None
    assert_eq!(inst.finish_remote(&span, 10_000), None);
}
