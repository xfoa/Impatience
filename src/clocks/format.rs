/// Format a single-line status report for a probe packet.
pub fn format_probe_stats(
    label: &str,
    seq: u32,
    local_ms: u32,
    remote_ms: Option<i64>,
    correction_ms: Option<i64>,
    min_delta: u32,
    synced: bool,
    start_delta_ms: Option<i64>,
) -> String {
    let remote_str = remote_ms.map(|v| v.to_string()).unwrap_or_else(|| "-".into());
    let corr_str = correction_ms.map(|v| v.to_string()).unwrap_or_else(|| "-".into());
    let delta_str = start_delta_ms
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".into());
    format!(
        "{label} seq={seq} local_ms={local_ms} remote_ms={remote_str} correction_ms={corr_str} min_delta={min_delta} synced={synced} start_delta_ms={delta_str}"
    )
}

/// Format a single-line status report for a received sync packet.
pub fn format_sync_stats(
    label: &str,
    min_delta: u32,
    synced: bool,
    start_delta_ms: Option<i64>,
) -> String {
    let delta_str = start_delta_ms
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".into());
    format!("{label} sync received min_delta={min_delta} synced={synced} start_delta_ms={delta_str}")
}
