/// Tracks when the next periodic sync packet should be sent.
///
/// This is a pure timer, it does not send packets or spawn threads.
/// The caller integrates it into their event loop.
///
/// # Example
///
/// ```
/// use impatience::net::SyncScheduler;
///
/// let mut scheduler = SyncScheduler::new(2000, 1_000_000);
/// // Not yet time to send
/// assert!(!scheduler.should_send(1_500_000));
/// // 2 seconds have passed, should send now
/// assert!(scheduler.should_send(3_000_000));
/// // Just sent, should not send again immediately
/// assert!(!scheduler.should_send(3_000_001));
/// ```
pub struct SyncScheduler {
    interval_usec: u64,
    next_send_usec: u64,
}

impl SyncScheduler {
    /// Create a new scheduler that fires every `interval_ms` milliseconds.
    pub fn new(interval_ms: u64, now_usec: u64) -> Self {
        let interval_usec = interval_ms * 1000;
        Self {
            interval_usec,
            next_send_usec: now_usec.saturating_add(interval_usec),
        }
    }

    /// Returns `true` if a sync packet should be sent now.
    pub fn should_send(&mut self, now_usec: u64) -> bool {
        if now_usec >= self.next_send_usec {
            self.next_send_usec = now_usec.saturating_add(self.interval_usec);
            true
        } else {
            false
        }
    }

    /// Reset the scheduler to fire at `now_usec + interval`.
    pub fn reset(&mut self, now_usec: u64) {
        self.next_send_usec = now_usec.saturating_add(self.interval_usec);
    }
}
