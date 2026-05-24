use crate::instrumentation::aggregator::LatencyAggregator;

/// A point-in-time snapshot of an aggregator's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Snapshot {
    pub count: usize,
    pub min: Option<u64>,
    pub max: Option<u64>,
    pub p50: Option<u64>,
    pub p95: Option<u64>,
    pub p99: Option<u64>,
}

impl Snapshot {
    pub fn from_aggregator(agg: &LatencyAggregator) -> Self {
        Self {
            count: agg.count(),
            min: agg.min(),
            max: agg.max(),
            p50: agg.p50(),
            p95: agg.p95(),
            p99: agg.p99(),
        }
    }
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            count: 0,
            min: None,
            max: None,
            p50: None,
            p95: None,
            p99: None,
        }
    }
}

#[cfg(feature = "uncertainty")]
impl Snapshot {
    /// Return a confidence interval for the given percentile estimate.
    ///
    /// Uses a Normal approximation with standard deviation derived from the
    /// sample inter-quartile range.  Treat the result as indicative only.
    pub fn percentile_ci(&self, p: f64, confidence: f64) -> Option<(u64, u64)> {
        let val = self.percentile_value(p)? as f64;
        let q1 = self.p50? as f64;
        let q3 = self.p95? as f64;
        let iqr = q3 - q1;
        let std_dev = iqr / 1.35;
        if std_dev <= 0.0 || confidence <= 0.0 || confidence >= 1.0 {
            return Some((val as u64, val as u64));
        }
        let dist = statrs::distribution::Normal::new(val, std_dev).ok()?;
        let alpha = 1.0 - confidence;
        let lo = statrs::distribution::ContinuousCDF::inverse_cdf(&dist, alpha / 2.0);
        let hi = statrs::distribution::ContinuousCDF::inverse_cdf(&dist, 1.0 - alpha / 2.0);
        Some((lo.max(0.0) as u64, hi.max(0.0) as u64))
    }

    fn percentile_value(&self, p: f64) -> Option<u64> {
        match (p * 100.0).round() as u8 {
            50 => self.p50,
            95 => self.p95,
            99 => self.p99,
            _ => None,
        }
    }
}

/// Trait for pluggable metric output.
pub trait Reporter {
    fn report(&mut self, snapshot: &Snapshot);
}

/// A simple reporter that prints a human-readable line to stderr.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConsoleReporter;

impl Reporter for ConsoleReporter {
    fn report(&mut self, snapshot: &Snapshot) {
        if snapshot.count == 0 {
            eprintln!("[instrument] no samples yet");
            return;
        }
        let fmt = |v: Option<u64>| match v {
            Some(ms) => format!("{} ms", ms),
            None => "n/a".to_string(),
        };
        eprintln!(
            "[instrument] count={} min={} p50={} p95={} p99={} max={}",
            snapshot.count,
            fmt(snapshot.min),
            fmt(snapshot.p50),
            fmt(snapshot.p95),
            fmt(snapshot.p99),
            fmt(snapshot.max),
        );
    }
}
