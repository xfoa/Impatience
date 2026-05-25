use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use impatience::instrumentation::{Profiler, histogram_svg, scatter_plot_svg};
use impatience::net::packets::{
    InputEventPacket, Packet, SyncPacket,
};
use impatience::net::{HandshakeProgress, Initiator, PeerClock, SyncScheduler};
use impatience::time;
use rand::Rng;
use std::io;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MAX_MSG_SIZE: usize = 1024;

struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

pub fn run(host: &str, port: u16, sync_interval_ms: u64, max_delay_ms: u32) {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("client bind failed");
    socket
        .connect(format!("{}:{}", host, port))
        .expect("client connect failed");
    socket
        .set_read_timeout(Some(Duration::from_millis(1000)))
        .expect("set_read_timeout failed");

    let clock = PeerClock::new();
    let started_at = time::now_usec();
    clock.start(started_at);

    // --- StartClock handshake ---
    let mut hs = Initiator::new(started_at);
    let pkt = hs.initial_packet();
    eprintln!("[client] sending StartClock with started_at={}", started_at);
    let bytes = Packet::StartClock(pkt).to_bytes().expect("serialise StartClock");
    socket.send(&bytes).expect("send StartClock");

    let mut buf = [0u8; MAX_MSG_SIZE];
    let mut acked = false;
    while !acked && !hs.exhausted() {
        match socket.recv(&mut buf) {
            Ok(n) => {
                match Packet::from_bytes(&buf[..n]) {
                    Ok(pkt) => {
                        match hs.on_receive(&pkt) {
                            HandshakeProgress::Complete { peer_started_at } => {
                                eprintln!(
                                    "[client] got AckStartClock peer_started_at={}",
                                    peer_started_at
                                );
                                clock.set_peer_started_at(peer_started_at);
                                acked = true;
                            }
                            HandshakeProgress::Pending => {}
                            HandshakeProgress::Ignored => {
                                eprintln!("[client] unexpected packet during handshake: {:?}", pkt);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[client] packet parse error: {}", e);
                    }
                }
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
                hs.record_retry();
                eprintln!(
                    "[client] handshake timeout #{}, retrying StartClock",
                    hs.retries()
                );
                socket.send(&bytes).expect("retry StartClock");
            }
            Err(e) => {
                eprintln!("[client] handshake recv error: {}", e);
                break;
            }
        }
    }

    if !acked {
        eprintln!("[client] handshake failed, aborting");
        return;
    }

    eprintln!("[client] handshake complete, starting latency demo. Press Escape to exit.");

    let profiler = Profiler::new(clock.clone());
    let pending_spans: Arc<Mutex<impatience::instrumentation::EventTracker<u32, (u32, char, u32)>>> =
        Arc::new(Mutex::new(impatience::instrumentation::EventTracker::new()));
    let exit_flag = Arc::new(AtomicBool::new(false));
    let next_seq = Arc::new(Mutex::new(0u32));

    if let Err(e) = enable_raw_mode() {
        eprintln!("[client] failed to enable raw mode: {}", e);
        return;
    }
    let _raw_guard = RawModeGuard;

    // --- Input thread ---
    let socket_input = socket.try_clone().expect("socket clone failed");
    let clock_input = clock.clone();
    let profiler_input = profiler.clone();
    let pending_input = Arc::clone(&pending_spans);
    let exit_input = Arc::clone(&exit_flag);
    let next_seq_input = Arc::clone(&next_seq);

    let input_handle = thread::spawn(move || {
        loop {
            if exit_input.load(Ordering::SeqCst) {
                break;
            }

            match crossterm::event::poll(Duration::from_millis(50)) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(e) => {
                    eprint!("[client] event poll error: {}\r\n", e);
                    continue;
                }
            }

            match crossterm::event::read() {
                Ok(CrosstermEvent::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                })) => {
                    exit_input.store(true, Ordering::SeqCst);
                    break;
                }
                Ok(CrosstermEvent::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                })) => {
                    let _ = disable_raw_mode();
                    unsafe { libc::raise(libc::SIGINT) };
                }
                Ok(CrosstermEvent::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    ..
                })) => {
                    let local_ms = clock_input.local_ms();
                    let span = profiler_input.start("input-to-print", local_ms);

                    let mut rng = rand::thread_rng();
                    let delay_ms: u32 = rng.gen_range(0..=max_delay_ms);

                    let mut seq_lock = next_seq_input.lock().unwrap();
                    let seq = *seq_lock;
                    *seq_lock = seq.wrapping_add(1);
                    drop(seq_lock);

                    pending_input
                        .lock()
                        .unwrap()
                        .insert(seq, span, (delay_ms, c, local_ms));

                    print!(
                        "[client] input seq={} ch='{}' local_time={}ms\r\n",
                        seq, c, local_ms
                    );
                    let _ = std::io::Write::flush(&mut std::io::stdout());

                    let socket_send = socket_input.try_clone().expect("socket clone failed");
                    let clock_send = clock_input.clone();
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(delay_ms as u64));
                        let mut pkt = InputEventPacket {
                            probe_ts24: 0,
                            seq,
                            ch: c as u8,
                            delay_ms,
                        };
                        let send_now = time::now_usec();
                        clock_send.stamp_probe(&mut pkt, send_now);

                        if let Ok(bytes) = Packet::InputEvent(pkt).to_bytes() {
                            let _ = socket_send.send(&bytes);
                        }
                    });
                }
                Ok(_) => {}
                Err(e) => {
                    eprint!("[client] event read error: {}\r\n", e);
                }
            }
        }
    });

    // --- Sync sender thread ---
    let socket_sync = socket.try_clone().expect("socket clone failed");
    let clock_sync = clock.clone();
    let exit_sync = Arc::clone(&exit_flag);
    let sync_handle = thread::spawn(move || {
        let mut scheduler = SyncScheduler::new(sync_interval_ms, time::now_usec());
        loop {
            thread::sleep(Duration::from_millis(10));
            if exit_sync.load(Ordering::SeqCst) {
                break;
            }

            let now = time::now_usec();
            if scheduler.should_send(now) {
                let mut pkt = SyncPacket::default();
                clock_sync.stamp_sync(&mut pkt, now);

                if let Ok(bytes) = Packet::Sync(pkt).to_bytes() {
                    let _ = socket_sync.send(&bytes);
                }
            }
        }
    });

    let mut per_event_latencies: Vec<(u32, u32, u64, char)> = Vec::new();

    // --- Main recv loop ---
    let mut exit_time: Option<u64> = None;
    loop {
        if exit_flag.load(Ordering::SeqCst) {
            if exit_time.is_none() {
                let _ = disable_raw_mode();
                exit_time = Some(time::now_ms());
                eprintln!("[client] exit requested, draining for 2s...");
            } else if time::now_ms().saturating_sub(exit_time.unwrap()) > 2000 {
                eprintln!("[client] drain complete");
                break;
            }
        }

        match socket.recv(&mut buf) {
            Ok(n) => {
                match Packet::from_bytes(&buf[..n]) {
                    Ok(Packet::StatsBatch(batch)) => {
                        let now = time::now_usec();
                        let _owd = clock.on_probe(&batch, now);

                        let mut pending = pending_spans.lock().unwrap();
                        for evt in &batch.events {
                            if let Some((mut span, (delay_ms, ch, input_ms) )) = pending.remove(&evt.seq) {
                                if let Some(latency_ms) =
                                    profiler.finish_remote(&mut span, evt.server_print_ms)
                                {
                                    let print_ms = span.finish_local_ms().unwrap_or(0);
                                    per_event_latencies.push((evt.seq, delay_ms, latency_ms, ch));
                                    print!(
                                        "[client] print seq={} ch='{}' input_time={}ms print_time={}ms random_delay={}ms event_latency={}ms\r\n",
                                        evt.seq, ch, input_ms, print_ms, delay_ms, latency_ms
                                    );
                                    let _ = std::io::Write::flush(&mut std::io::stdout());
                                } else {
                                    eprint!(
                                        "[client] clock not synchronised for seq={}\r\n",
                                        evt.seq
                                    );
                                }
                            } else {
                                eprint!(
                                    "[client] unknown seq in stats batch: {}\r\n",
                                    evt.seq
                                );
                            }
                        }
                    }
                    Ok(Packet::Sync(sync_pkt)) => {
                        clock.on_sync(&sync_pkt);
                    }
                    Ok(other) => {
                        eprint!("[client] unexpected packet: {:?}\r\n", other);
                    }
                    Err(e) => {
                        eprint!("[client] packet parse error: {}\r\n", e);
                    }
                }
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
                if exit_time.is_some()
                    && time::now_ms().saturating_sub(exit_time.unwrap()) > 2000
                {
                    eprint!("[client] drain timeout, exiting\r\n");
                    break;
                }
            }
            Err(e) => {
                eprint!("[client] recv error: {}\r\n", e);
                break;
            }
        }
    }

    input_handle.join().ok();
    sync_handle.join().ok();

    // --- Generate reports ---
    let snapshot = profiler.snapshot();
    eprintln!(
        "[client] aggregate: count={} min={:?}ms p50={:?}ms p95={:?}ms p99={:?}ms max={:?}ms",
        snapshot.count, snapshot.min, snapshot.p50, snapshot.p95, snapshot.p99, snapshot.max
    );

    let report = serde_json::json!({
        "total_events": per_event_latencies.len(),
        "min_ms": snapshot.min,
        "p50_ms": snapshot.p50,
        "p95_ms": snapshot.p95,
        "p99_ms": snapshot.p99,
        "max_ms": snapshot.max,
        "events": per_event_latencies.iter().map(|(seq, delay, latency, ch)| {
            serde_json::json!({
                "seq": seq,
                "char": ch.to_string(),
                "delay_ms": delay,
                "latency_ms": latency,
            })
        }).collect::<Vec<_>>(),
    });

    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            if let Err(e) = std::fs::write("latency_stats.json", json) {
                eprintln!("[client] failed to write JSON: {}", e);
            } else {
                eprintln!("[client] wrote latency_stats.json");
            }
        }
        Err(e) => eprintln!("[client] JSON serialize error: {}", e),
    }

    if let Err(e) = generate_html_report(&per_event_latencies, &snapshot) {
        eprintln!("[client] failed to generate HTML report: {}", e);
    } else {
        eprintln!("[client] wrote latency_report.html");
    }
}

fn generate_html_report(
    events: &[(u32, u32, u64, char)],
    snapshot: &impatience::instrumentation::Snapshot,
) -> Result<(), std::io::Error> {
    if events.is_empty() {
        std::fs::write(
            "latency_report.html",
            "<!DOCTYPE html><html><body><h1>No events recorded</h1></body></html>",
        )?;
        return Ok(());
    }

    let mut html = String::new();
    html.push_str(r##"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Latency Report</title>
<style>
* { box-sizing: border-box; }
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    background: #f5f7fa;
    color: #2d3748;
    margin: 0;
    padding: 40px 20px;
    line-height: 1.6;
}
.container {
    max-width: 960px;
    margin: 0 auto;
}
h1 {
    font-size: 2rem;
    font-weight: 600;
    margin: 0 0 8px 0;
    color: #1a202c;
}
.subtitle {
    color: #718096;
    font-size: 0.95rem;
    margin: 0 0 24px 0;
}
.cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 16px;
    margin-bottom: 32px;
}
.card {
    background: #fff;
    border-radius: 10px;
    padding: 16px 20px;
    box-shadow: 0 1px 3px rgba(0,0,0,0.08);
    border: 1px solid #e2e8f0;
}
.card-label {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #a0aec0;
    margin: 0 0 4px 0;
}
.card-value {
    font-size: 1.4rem;
    font-weight: 700;
    color: #2d3748;
    margin: 0;
}
h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 32px 0 12px 0;
    color: #1a202c;
}
svg {
    background: #fff;
    border-radius: 10px;
    box-shadow: 0 1px 3px rgba(0,0,0,0.08);
    border: 1px solid #e2e8f0;
    display: block;
    width: 100%;
    height: auto;
    padding: 16px;
}
</style>
</head>
<body>
<div class="container">
"##);
    html.push_str(&format!(
        "<h1>Latency Report</h1>\n<p class=\"subtitle\">Total events: {}</p>\n",
        events.len()
    ));

    html.push_str("<div class=\"cards\">\n");
    let stat = |label: &str, value: Option<u64>| {
        let v = value.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string());
        format!(
            "<div class=\"card\"><p class=\"card-label\">{}</p><p class=\"card-value\">{} ms</p></div>\n",
            label, v
        )
    };
    html.push_str(&stat("Min", snapshot.min));
    html.push_str(&stat("P50", snapshot.p50));
    html.push_str(&stat("P95", snapshot.p95));
    html.push_str(&stat("P99", snapshot.p99));
    html.push_str(&stat("Max", snapshot.max));
    html.push_str("</div>\n");

    let plot_data: Vec<(usize, u64)> = events
        .iter()
        .enumerate()
        .map(|(i, (_, _, latency, _))| (i, *latency))
        .collect();

    html.push_str("<h2>Latency Over Time</h2>\n");
    html.push_str(&scatter_plot_svg(&plot_data));

    html.push_str("<h2>Latency Histogram</h2>\n");
    html.push_str(&histogram_svg(&plot_data, 20));

    html.push_str("</div>\n</body>\n</html>\n");
    std::fs::write("latency_report.html", html)
}
