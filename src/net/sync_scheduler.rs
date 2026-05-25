pub struct SyncScheduler {
    interval_usec: u64,
    next_send_usec: u64,
}

impl SyncScheduler {
    pub fn new(interval_ms: u64, now_usec: u64) -> Self {
        let interval_usec = interval_ms * 1000;
        Self {
            interval_usec,
            next_send_usec: now_usec.saturating_add(interval_usec),
        }
    }

    pub fn should_send(&mut self, now_usec: u64) -> bool {
        if now_usec >= self.next_send_usec {
            self.next_send_usec = now_usec.saturating_add(self.interval_usec);
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self, now_usec: u64) {
        self.next_send_usec = now_usec.saturating_add(self.interval_usec);
    }
}
