use crate::cli::latencydemo::common;
use impatience::net::packets::{AckStartClockPacket, Packet, StatsBatchPacket, StatsEvent, SyncPacket};
use impatience::net::PeerClock;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn run(bind_addr: &str, port: u16, sync_interval_ms: u64) {
    let socket = UdpSocket::bind(format!("{}:{}", bind_addr, port)).expect("server bind failed");
    socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("set_read_timeout failed");

    eprintln!("[server] listening on {}:{}", bind_addr, port);

    let clock = PeerClock::new();
    let client_addr: Arc<Mutex<Option<SocketAddr>>> = Arc::new(Mutex::new(None));
    let clock_started = Arc::new(AtomicBool::new(false));
    let stats_buffer: Arc<Mutex<Vec<StatsEvent>>> = Arc::new(Mutex::new(Vec::new()));

    let socket_send = socket.try_clone().expect("socket clone failed");
    let clock_send = clock.clone();
    let client_addr_send = Arc::clone(&client_addr);
    let clock_started_send = Arc::clone(&clock_started);
    let stats_buffer_send = Arc::clone(&stats_buffer);

    let _stats_handle = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));

            if !clock_started_send.load(Ordering::SeqCst) {
                continue;
            }

            let addr_opt = client_addr_send.lock().unwrap();
            if let Some(addr) = *addr_opt {
                drop(addr_opt);
                let mut events = stats_buffer_send.lock().unwrap();
                if events.is_empty() {
                    continue;
                }
                let batch = StatsBatchPacket {
                    probe_ts24: 0,
                    events: events.clone(),
                };
                events.clear();
                drop(events);

                let mut pkt = batch;
                let now = common::now_usec();
                clock_send.stamp_probe(&mut pkt, now);

                if let Ok(bytes) = Packet::StatsBatch(pkt).to_bytes() {
                    let _ = socket_send.send_to(&bytes, addr);
                }
            }
        }
    });

    let socket_sync = socket.try_clone().expect("socket clone failed");
    let clock_sync = clock.clone();
    let client_addr_sync = Arc::clone(&client_addr);
    let clock_started_sync = Arc::clone(&clock_started);

    let _sync_handle = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(sync_interval_ms));

            if !clock_started_sync.load(Ordering::SeqCst) {
                continue;
            }

            let addr_opt = client_addr_sync.lock().unwrap();
            if let Some(addr) = *addr_opt {
                drop(addr_opt);
                let mut pkt = SyncPacket::default();
                let now = common::now_usec();
                clock_sync.stamp_sync(&mut pkt, now);

                if let Ok(bytes) = Packet::Sync(pkt).to_bytes() {
                    let _ = socket_sync.send_to(&bytes, addr);
                }
            }
        }
    });

    let mut buf = [0u8; common::MAX_MSG_SIZE];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((n, addr)) => {
                *client_addr.lock().unwrap() = Some(addr);

                match Packet::from_bytes(&buf[..n]) {
                    Ok(Packet::StartClock(start_pkt)) => {
                        let now = common::now_usec();
                        eprintln!("[server] received StartClock from {} peer_started_at={}", addr, start_pkt.started_at);

                        clock.start(now);
                        clock_started.store(true, Ordering::SeqCst);
                        clock.set_peer_started_at(start_pkt.started_at);

                        let ack = AckStartClockPacket { started_at: now };
                        if let Ok(bytes) = Packet::AckStartClock(ack).to_bytes() {
                            let _ = socket.send_to(&bytes, addr);
                            eprintln!("[server] sent AckStartClock to {} our_started_at={}", addr, now);
                        }
                    }
                    Ok(Packet::InputEvent(evt)) => {
                        let now_usec = common::now_usec();
                        let _owd = clock.on_probe(&evt, now_usec);

                        let ch = evt.ch as char;
                        print!("{}", ch);
                        let _ = std::io::Write::flush(&mut std::io::stdout());

                        let server_event = StatsEvent {
                            seq: evt.seq,
                            server_print_ms: common::now_ms(),
                        };
                        stats_buffer.lock().unwrap().push(server_event);
                    }
                    Ok(Packet::Sync(sync_pkt)) => {
                        clock.on_sync(&sync_pkt);
                    }
                    Ok(other) => {
                        eprintln!("[server] unexpected packet: {:?}", other);
                    }
                    Err(e) => {
                        eprintln!("[server] packet parse error: {}", e);
                    }
                }
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
            }
            Err(e) => {
                eprintln!("[server] recv error: {}", e);
                break;
            }
        }
    }
}
