use impatience::clocks::{format_probe_stats, format_sync_stats};
use impatience::net::packets::{Packet, PongPacket, SyncPacket};
use impatience::clocks::PeerClock;
use impatience::net::{Responder, SyncScheduler};
use impatience::time;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MAX_MSG_SIZE: usize = 1024;

pub fn run(bind_addr: &str, port: u16, sync_interval_ms: u64) {
    let socket = UdpSocket::bind(format!("{}:{}", bind_addr, port)).expect("server bind failed");
    socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("set_read_timeout failed");

    eprintln!("[server] listening on {bind_addr}:{port}");

    let clock = PeerClock::new();
    let client_addr: Arc<Mutex<Option<SocketAddr>>> = Arc::new(Mutex::new(None));
    let clock_started = Arc::new(AtomicBool::new(false));

    let socket_send = socket.try_clone().expect("socket clone failed");
    let clock_send = clock.clone();
    let client_addr_send = Arc::clone(&client_addr);
    let clock_started_send = Arc::clone(&clock_started);

    let _send_handle = thread::spawn(move || {
        let mut scheduler = SyncScheduler::new(sync_interval_ms, time::now_usec());
        loop {
            thread::sleep(Duration::from_millis(10));

            if !clock_started_send.load(Ordering::SeqCst) {
                continue;
            }

            let now = time::now_usec();
            if !scheduler.should_send(now) {
                continue;
            }

            let addr_opt = client_addr_send.lock().unwrap();
            if let Some(addr) = *addr_opt {
                drop(addr_opt);
                let mut pkt = SyncPacket::default();
                clock_send.stamp_sync(&mut pkt, now);

                if let Ok(bytes) = Packet::Sync(pkt).to_bytes() {
                    let _ = socket_send.send_to(&bytes, addr);
                }
            }
        }
    });

    let mut buf = [0u8; MAX_MSG_SIZE];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((n, addr)) => {
                *client_addr.lock().unwrap() = Some(addr);

                match Packet::from_bytes(&buf[..n]) {
                    Ok(Packet::StartClock(start_pkt)) => {
                        let now = time::now_usec();
                        eprintln!("[server] received StartClock from {addr} peer_started_at={}", start_pkt.started_at);

                        clock.start(now);
                        clock_started.store(true, Ordering::SeqCst);
                        let (ack, peer_started_at) = Responder::on_start_clock(&start_pkt, now);
                        clock.set_peer_started_at(peer_started_at);

                        if let Ok(bytes) = Packet::AckStartClock(ack).to_bytes() {
                            let _ = socket.send_to(&bytes, addr);
                            eprintln!("[server] sent AckStartClock to {addr} our_started_at={now}");
                        }
                    }
                    Ok(Packet::Ping(ping)) => {
                        let now = time::now_usec();

                        if !clock_started.load(Ordering::SeqCst) {
                            clock.start(now);
                            clock_started.store(true, Ordering::SeqCst);
                        }

                        let _owd = clock.on_probe(&ping, now);
                        let local_ms = clock.local_ms();
                        let correction = clock.correction_ms();
                        let min_delta = clock.min_delta().to_unsigned();
                        let synced = clock.is_synchronised();
                        let start_delta_ms = clock.start_delta_ms();
                        let remote_ms = clock.remote_ms(false);

                        let mut pong = PongPacket::default();
                        pong.ping_seq = ping.seq;
                        clock.stamp_probe(&mut pong, now);

                        if let Ok(bytes) = Packet::Pong(pong).to_bytes() {
                            let _ = socket.send_to(&bytes, addr);
                        }

                        println!(
                            "{}",
                            format_probe_stats(
                                "ping",
                                ping.seq,
                                local_ms,
                                remote_ms,
                                correction,
                                min_delta,
                                synced,
                                start_delta_ms,
                            )
                        );
                    }
                    Ok(Packet::Sync(sync_pkt)) => {
                        clock.on_sync(&sync_pkt);
                        let min_delta = clock.min_delta().to_unsigned();
                        let synced = clock.is_synchronised();
                        let start_delta_ms = clock.start_delta_ms();
                        println!(
                            "{}",
                            format_sync_stats(
                                "server",
                                min_delta,
                                synced,
                                start_delta_ms,
                            )
                        );
                    }
                    Ok(other) => {
                        eprintln!("[server] unexpected packet: {other:?}");
                    }
                    Err(e) => {
                        eprintln!("[server] packet parse error: {e}");
                    }
                }
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
                // Continue waiting
            }
            Err(e) => {
                eprintln!("[server] recv error: {e}");
                break;
            }
        }
    }
}
