use std::collections::VecDeque;

/// Fixed-capacity circular buffer that maintains latency samples in
/// sorted order so percentile queries are cheap.
///
/// When the capacity is exceeded, the oldest sample is evicted and the
/// new sample is inserted in sorted position.
#[derive(Clone, Debug)]
pub struct LatencyAggregator {
    capacity: usize,
    sorted: Vec<u64>,
    ring: VecDeque<u64>,
}

impl LatencyAggregator {
    /// Create a new aggregator with the given sample capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            sorted: Vec::with_capacity(capacity),
            ring: VecDeque::with_capacity(capacity),
        }
    }

    /// Insert a new latency sample in milliseconds.
    ///
    /// Amortised O(N) because inserting into a sorted vector requires shifting
    /// elements to make room.
    pub fn insert(&mut self, latency_ms: u64) {
        if self.ring.len() == self.capacity {
            if let Some(old) = self.ring.pop_front() {
                if let Ok(idx) = self.sorted.binary_search(&old) {
                    self.sorted.remove(idx);
                }
            }
        }

        let pos = self.sorted.binary_search(&latency_ms).unwrap_or_else(|e| e);
        self.sorted.insert(pos, latency_ms);
        self.ring.push_back(latency_ms);
    }

    #[inline]
    /// Number of samples currently stored.
    pub fn count(&self) -> usize {
        self.sorted.len()
    }

    #[inline]
    /// Returns `true` if no samples have been recorded.
    pub fn is_empty(&self) -> bool {
        self.sorted.is_empty()
    }

    /// Minimum observed latency in milliseconds.
    #[inline]
    pub fn min(&self) -> Option<u64> {
        self.sorted.first().copied()
    }

    /// Maximum observed latency in milliseconds.
    #[inline]
    pub fn max(&self) -> Option<u64> {
        self.sorted.last().copied()
    }

    /// Return the percentile value in milliseconds.
    ///
    /// `p` is in the range `[0.0, 1.0]`.
    /// Uses linear interpolation between adjacent samples.
    pub fn percentile(&self, p: f64) -> Option<u64> {
        let n = self.sorted.len();
        if n == 0 {
            return None;
        }
        if p <= 0.0 {
            return self.min();
        }
        if p >= 1.0 {
            return self.max();
        }

        let idx = p * (n as f64 - 1.0);
        let lower = idx.floor() as usize;
        let upper = idx.ceil() as usize;
        let frac = idx - lower as f64;

        let lower_val = self.sorted[lower] as f64;
        let upper_val = self.sorted[upper] as f64;
        let interpolated = lower_val + frac * (upper_val - lower_val);

        Some(interpolated.round() as u64)
    }

    /// Convenience: p50 in milliseconds.
    #[inline]
    pub fn p50(&self) -> Option<u64> {
        self.percentile(0.50)
    }

    /// Convenience: p95 in milliseconds.
    #[inline]
    pub fn p95(&self) -> Option<u64> {
        self.percentile(0.95)
    }

    /// Convenience: p99 in milliseconds.
    #[inline]
    pub fn p99(&self) -> Option<u64> {
        self.percentile(0.99)
    }

    /// Reset all samples.
    pub fn clear(&mut self) {
        self.sorted.clear();
        self.ring.clear();
    }

    /// Iterate over samples in insertion order (oldest first).
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &u64> {
        self.ring.iter()
    }
}

impl Default for LatencyAggregator {
    fn default() -> Self {
        Self::new(10_000)
    }
}
