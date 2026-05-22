use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum UDP payload size we are willing to read.
pub const MAX_MSG_SIZE: usize = 1024;

/// Return the current wall-clock time in microseconds.
pub fn now_usec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_micros() as u64
}

/// Format a single-line status report for a probe packet.
pub fn format_probe_stats(
    label: &str,
    seq: u32,
    local_ms: u64,
    remote_ms: Option<i64>,
    correction_ms: Option<i64>,
    min_delta: u32,
    synced: bool,
) -> String {
    let remote_str = remote_ms.map(|v| v.to_string()).unwrap_or_else(|| "-".into());
    let corr_str = correction_ms.map(|v| v.to_string()).unwrap_or_else(|| "-".into());
    format!(
        "{label} seq={seq} local_ms={local_ms} remote_ms={remote_str} correction_ms={corr_str} min_delta={min_delta} synced={synced}"
    )
}

/// Format a single-line status report for a received sync packet.
pub fn format_sync_stats(label: &str, min_delta: u32, synced: bool) -> String {
    format!("{label} sync received min_delta={min_delta} synced={synced}")
}
