use std::time::{SystemTime, UNIX_EPOCH};

/// Current wall-clock time in microseconds since Unix epoch.
///
/// # Example
///
/// ```
/// let t = impatience::time::now_usec();
/// assert!(t > 0);
/// ```
pub fn now_usec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_micros() as u64
}

/// Current wall-clock time in milliseconds since Unix epoch.
///
/// # Example
///
/// ```
/// let t = impatience::time::now_ms();
/// assert!(t > 0);
/// ```
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_millis() as u64
}
