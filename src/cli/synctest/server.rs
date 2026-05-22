use crate::cli::synctest::common;
use impatience::clock::SyncedClock;
use impatience::net::packet::{AckStartClockPacket, Packet, PongPacket, SyncPacket};
use impatience::net::traits::{apply_probe, retrieve_probe, PeerSync};
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

    let clock = Arc::new(Mutex::new(SyncedClock::new()));
    let client_addr: Arc<Mutex<Option<SocketAddr>>> = Arc::new(Mutex::new(None));
    let clock_started = Arc::new(AtomicBool::new(false));
    let peer_start: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));

    let socket_send = socket.try_clone().expect("socket clone failed");
    let clock_send = Arc::clone(&clock);
    let client_addr_send = Arc::clone(&client_addr);
    let clock_started_send = Arc::clone(&clock_started);
    let _peer_start_send = Arc::clone(&peer_start);

    let _send_handle = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(sync_interval_ms));

            if !clock_started_send.load(Ordering::SeqCst) {
                continue;
            }

            let addr_opt = client_addr_send.lock().unwrap();
            if let Some(addr) = *addr_opt {
                drop(addr_opt);
                let c = clock_send.lock().unwrap();
                let mut pkt = SyncPacket::default();
                let now = common::now_usec();
                apply_probe(&c, &mut pkt, now);
                pkt.min_delta_ts24 = c.get_sync_delta().to_unsigned();
                drop(c);

                if let Ok(bytes) = Packet::Sync(pkt).to_bytes() {
                    let _ = socket_send.send_to(&bytes, addr);
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
                        let mut c = clock.lock().unwrap();
                        if !clock_started.load(Ordering::SeqCst) {
                            c.start(now);
                            clock_started.store(true, Ordering::SeqCst);
                        }
                        *peer_start.lock().unwrap() = Some(start_pkt.start_usec);
                        drop(c);

                        let ack = AckStartClockPacket {
                            start_usec: now,
                        };
                        if let Ok(bytes) = Packet::AckStartClock(ack).to_bytes() {
                            let _ = socket.send_to(&bytes, addr);
                        }
                    }
                    Ok(Packet::Ping(ping)) => {
                        let now = common::now_usec();

                        let mut c = clock.lock().unwrap();
                        let _owd = retrieve_probe(&mut c, &ping, now);
                        let local_ms = c.local_ms(now);
                        let correction = c.correction_ms();
                        let remote_ms = correction.map(|v| local_ms as i64 + v);
                        let min_delta = c.get_sync_delta().to_unsigned();
                        let synced = c.is_synchronized();
                        let start_ms = c.start_usec() / 1000;
                        let peer_start_ms = peer_start.lock().unwrap().map(|v| v / 1000);

                        let mut pong = PongPacket::default();
                        pong.ping_seq = ping.seq;
                        apply_probe(&c, &mut pong, now);
                        drop(c);

                        if let Ok(bytes) = Packet::Pong(pong).to_bytes() {
                            let _ = socket.send_to(&bytes, addr);
                        }

                        println!(
                            "{}",
                            common::format_probe_stats(
                                "ping",
                                ping.seq,
                                local_ms,
                                remote_ms,
                                correction,
                                min_delta,
                                synced,
                                start_ms,
                                peer_start_ms,
                            )
                        );
                    }
                    Ok(Packet::Sync(sync_pkt)) => {
                        let mut c = clock.lock().unwrap();
                        c.update_with_sync(sync_pkt.min_delta_ts());
                        let min_delta = c.get_sync_delta().to_unsigned();
                        let synced = c.is_synchronized();
                        let peer_start_ms = peer_start.lock().unwrap().map(|v| v / 1000);
                        println!(
                            "{}",
                            common::format_sync_stats(
                                "server",
                                min_delta,
                                synced,
                                c.start_usec() / 1000,
                                peer_start_ms,
                            )
                        );
                    }
                    _ => {}
                }
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
                // Continue waiting
            }
            Err(_) => break,
        }
    }
}
