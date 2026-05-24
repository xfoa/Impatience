use impatience::instrumentation::{Profiler, LatencyAggregator, Snapshot, Span};
use impatience::clocks::PeerClock;

#[test]
fn event_id_monotonic() {
    let clock = PeerClock::new();
    let inst = Profiler::new(clock);
    let a = inst.start("a", 0);
    let b = inst.start("b", 1);
    let c = inst.start("c", 2);
    assert!(a.event_id().0 < b.event_id().0);
    assert!(b.event_id().0 < c.event_id().0);
}

#[test]
fn span_elapsed_local() {
    let span = Span::new(impatience::instrumentation::EventId(1), "test", 10);
    assert_eq!(span.elapsed(20), 10);
    assert_eq!(span.elapsed(10), 0);
    assert_eq!(span.elapsed(5), 0); // saturating
}

#[test]
fn aggregator_basic_percentiles() {
    let mut agg = LatencyAggregator::new(100);
    for i in 1..=100 {
        agg.insert(i as u64);
    }
    assert_eq!(agg.count(), 100);
    assert_eq!(agg.min(), Some(1));
    assert_eq!(agg.max(), Some(100));
    assert_eq!(agg.p50(), Some(51));
    assert_eq!(agg.p95(), Some(95));
    assert_eq!(agg.p99(), Some(99));
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
    let inst = Profiler::new(clock);
    inst.record_latency(1);
    inst.record_latency(2);
    inst.record_latency(3);
    let snap = inst.snapshot();
    assert_eq!(snap.count, 3);
    assert_eq!(snap.min, Some(1));
    assert_eq!(snap.p50, Some(2));
    assert_eq!(snap.max, Some(3));
}

#[test]
fn instrument_finish_remote_unsynced_returns_none() {
    let clock = PeerClock::new();
    let inst = Profiler::new(clock);
    let span = inst.start("click", 0);
    // clock is not synchronised → remote_latency_ms returns None
    assert_eq!(inst.finish_remote(&span, 10), None);
}
