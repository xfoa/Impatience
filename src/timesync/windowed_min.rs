use crate::timesync::counter::Counter24;

#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub value: Counter24,
    pub timestamp: u64,
}

impl Sample {
    #[inline]
    pub const fn new(value: Counter24, timestamp: u64) -> Self {
        Self { value, timestamp }
    }

    #[inline]
    pub const fn timeout_expired(self, now: u64, timeout: u64) -> bool {
        now.wrapping_sub(self.timestamp) > timeout
    }
}

/// Sliding-window minimum over 24-bit timestamp deltas.
///
/// Keeps three sorted samples. The best (smallest circular) value is at
/// index 0. Samples are expired based on an external wall-clock timeout.
#[derive(Clone, Debug)]
pub struct WindowedMinTS24 {
    samples: [Sample; 3],
}

impl WindowedMinTS24 {
    pub const SAMPLE_COUNT: usize = 3;

    pub fn new() -> Self {
        Self {
            samples: [Sample::default(); 3],
        }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.samples[0].value.to_unsigned() != 0
    }

    #[inline]
    pub fn best(&self) -> Counter24 {
        self.samples[0].value
    }

    #[inline]
    pub fn reset(&mut self, sample: Sample) {
        self.samples[0] = sample;
        self.samples[1] = sample;
        self.samples[2] = sample;
    }

    pub fn update(&mut self, value: Counter24, timestamp: u64, window_length_time: u64) {
        let sample = Sample::new(value, timestamp);

        if !self.is_valid()
            || value.circ_le(self.samples[0].value)
            || self.samples[2].timeout_expired(sample.timestamp, window_length_time)
        {
            self.reset(sample);
            return;
        }

        if value.circ_le(self.samples[1].value) {
            self.samples[2] = sample;
            self.samples[1] = sample;
        } else if value.circ_le(self.samples[2].value) {
            self.samples[2] = sample;
        }

        if self.samples[0].timeout_expired(sample.timestamp, window_length_time) {
            if self.samples[1].timeout_expired(sample.timestamp, window_length_time) {
                self.samples[0] = self.samples[2];
                self.samples[1] = sample;
            } else {
                self.samples[0] = self.samples[1];
                self.samples[1] = self.samples[2];
            }
            self.samples[2] = sample;
            return;
        }

        // Quarter of window has gone by without a better value - use second-best
        if self.samples[1].value == self.samples[0].value
            && self.samples[1].timeout_expired(sample.timestamp, window_length_time / 4)
        {
            self.samples[2] = sample;
            self.samples[1] = sample;
            return;
        }

        // Half the window has gone by without a better value - use third-best
        if self.samples[2].value == self.samples[1].value
            && self.samples[2].timeout_expired(sample.timestamp, window_length_time / 2)
        {
            self.samples[2] = sample;
        }
    }
}

impl Default for WindowedMinTS24 {
    fn default() -> Self {
        Self::new()
    }
}
