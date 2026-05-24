use crate::clocks::PeerClock;
use crate::instrumentation::event::EventId;
use crate::instrumentation::aggregator::LatencyAggregator;
use crate::instrumentation::reporter::Snapshot;
use crate::instrumentation::span::Span;
use crate::instrumentation::reporter::Reporter;
use std::sync::{Arc, Mutex};

/// The main instrumentation handle.
///
/// Cheaply cloneable (backed by [`Arc`]) so it can be shared between
/// the thread that generates events and the thread that receives remote
/// completions.
#[derive(Clone, Debug)]
pub struct Profiler {
    clock: PeerClock,
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    next_id: u64,
    aggregator: LatencyAggregator,
}

impl Profiler {
    pub fn new(clock: PeerClock) -> Self {
        Self {
            clock,
            inner: Arc::new(Mutex::new(Inner {
                next_id: 1,
                aggregator: LatencyAggregator::default(),
            })),
        }
    }

    pub fn with_capacity(clock: PeerClock, capacity: usize) -> Self {
        Self {
            clock,
            inner: Arc::new(Mutex::new(Inner {
                next_id: 1,
                aggregator: LatencyAggregator::new(capacity),
            })),
        }
    }

    /// Start a new measurement span at the given local time (milliseconds).
    pub fn start(&self, name: &'static str, now_ms: u32) -> Span {
        let mut inner = self.inner.lock().unwrap();
        let id = EventId(inner.next_id);
        inner.next_id = inner.next_id.wrapping_add(1);
        Span::new(id, name, now_ms)
    }

    /// Record a remote finish for a span and insert the computed latency into the aggregator.
    ///
    /// Returns `Some(latency_ms)` on success, or `None` if the clock is not yet synchronised.
    pub fn finish_remote(&self, span: &mut Span, remote_finish_ms: u32) -> Option<u64> {
        let latency = span.finish_remote(remote_finish_ms, &self.clock)?;
        let clamped_latency = latency.max(0) as u64;
        let mut inner = self.inner.lock().unwrap();
        inner.aggregator.insert(clamped_latency);
        Some(clamped_latency)
    }

    /// Record a latency directly (e.g. computed elsewhere) into the aggregator.
    pub fn record_latency(&self, latency_ms: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.aggregator.insert(latency_ms);
    }

    /// Capture a snapshot of the current aggregator state.
    pub fn snapshot(&self) -> Snapshot {
        let inner = self.inner.lock().unwrap();
        Snapshot::from_aggregator(&inner.aggregator)
    }

    /// Report the current snapshot using the given reporter.
    pub fn report<R: Reporter>(&self, reporter: &mut R) {
        reporter.report(&self.snapshot());
    }

    /// Clear all samples.
    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.aggregator.clear();
    }
}
