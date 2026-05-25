use crate::cli::interface::Count;
use impatience::clocks::{format_probe_stats, format_sync_stats};
use impatience::net::{Packet, PingPacket, SyncPacket};
use impatience::clocks::PeerClock;
use impatience::net::{HandshakeProgress, Initiator, SyncScheduler};
use impatience::time;
use std::io;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const MAX_MSG_SIZE: usize = 1024;

pub fn run(host: &str, port: u16, count: Count, interval_ms: u64, sync_interval_ms: u64) {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("client bind failed");
    socket
        .connect(format!("{}:{}", host, port))
        .expect("client connect failed");
    socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("set_read_timeout failed");

    let clock = PeerClock::new();
    let started_at = time::now_usec();
    clock.start(started_at);

    // --- StartClock handshake ---
    let mut hs = Initiator::new(started_at);
    let pkt = hs.initial_packet();
    eprintln!("[client] sending StartClock with started_at={started_at}");
    let bytes = Packet::StartClock(pkt)
        .to_bytes()
        .expect("serialise StartClock");
    socket.send(&bytes).expect("send StartClock");

    let mut buf = [0u8; MAX_MSG_SIZE];
    let mut acked = false;
    while !acked && !hs.exhausted() {
        match socket.recv(&mut buf) {
            Ok(n) => {
                eprintln!("[client] received {n} bytes");
                match Packet::from_bytes(&buf[..n]) {
                    Ok(pkt) => {
                        match hs.on_receive(&pkt) {
                            HandshakeProgress::Complete { peer_started_at } => {
                                eprintln!("[client] got AckStartClock from {host}:{port} peer_started_at={peer_started_at}");
                                clock.set_peer_started_at(peer_started_at);
                                acked = true;
                            }
                            HandshakeProgress::Pending => {}
                            HandshakeProgress::Ignored => {
                                eprintln!("[client] unexpected packet during handshake: {pkt:?}");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[client] packet parse error: {e}");
                    }
                }
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
                hs.record_retry();
                eprintln!("[client] handshake timeout #{}, retrying StartClock", hs.retries());
                socket.send(&bytes).expect("retry StartClock");
            }
            Err(e) => {
                eprintln!("[client] handshake recv error: {e}");
                break;
            }
        }
    }

    if !acked {
        eprintln!("[client] handshake failed, aborting");
        return;
    }

    eprintln!("[client] handshake complete, starting ping loop");

    let socket_send = socket.try_clone().expect("socket clone failed");
    let clock_send = clock.clone();
    let send_done = Arc::new(AtomicBool::new(false));
    let send_done_flag = Arc::clone(&send_done);

    let send_handle = thread::spawn(move || {
        let mut seq: u32 = 0;
        let mut sync_scheduler = SyncScheduler::new(sync_interval_ms, time::now_usec());

        let max_pings = match count {
            Count::Finite(n) => n,
            Count::Infinite => u64::MAX,
        };

        eprintln!("[client] send thread: max_pings={max_pings}, interval_ms={interval_ms}");

        for _ in 0..max_pings {
            let now = time::now_usec();

            if sync_scheduler.should_send(now) {
                let mut pkt = SyncPacket::default();
                clock_send.stamp_sync(&mut pkt, now);
                if let Ok(bytes) = Packet::Sync(pkt).to_bytes() {
                    let _ = socket_send.send(&bytes);
                }
            }

            let mut pkt = PingPacket::default();
            clock_send.stamp_probe(&mut pkt, now);
            pkt.seq = seq;

            if let Ok(bytes) = Packet::Ping(pkt).to_bytes() {
                let _ = socket_send.send(&bytes);
            }
            seq = seq.wrapping_add(1);

            thread::sleep(Duration::from_millis(interval_ms));
        }

        eprintln!("[client] send thread done");
        send_done_flag.store(true, Ordering::SeqCst);
    });

    let mut send_done_time: Option<u64> = None;

    loop {
        if send_done.load(Ordering::SeqCst) {
            if send_done_time.is_none() {
                send_done_time = Some(time::now_usec());
                eprintln!("[client] send_done detected, starting 2s grace period");
            } else if time::now_usec().saturating_sub(send_done_time.unwrap()) > 2_000_000 {
                eprintln!("[client] grace period expired, exiting");
                break;
            }
        }

        match socket.recv(&mut buf) {
            Ok(n) => {
                match Packet::from_bytes(&buf[..n]) {
                    Ok(Packet::Pong(pong)) => {
                        let now = time::now_usec();
                        let _owd = clock.on_probe(&pong, now);
                        let local_ms = clock.local_ms();
                        let correction = clock.correction_ms();
                        let min_delta = clock.min_delta().to_unsigned();
                        let synced = clock.is_synchronised();
                        let start_delta_ms = clock.start_delta_ms();
                        let remote_ms = clock.remote_ms(true);
                        println!(
                            "{}",
                            format_probe_stats(
                                "pong",
                                pong.ping_seq,
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
                                "client",
                                min_delta,
                                synced,
                                start_delta_ms,
                            )
                        );
                    }
                    Ok(other) => {
                        eprintln!("[client] unexpected packet in main loop: {other:?}");
                    }
                    Err(e) => {
                        eprintln!("[client] packet parse error in main loop: {e}");
                    }
                }
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
                if send_done_time.is_some()
                    && time::now_usec().saturating_sub(send_done_time.unwrap()) > 2_000_000
                {
                    eprintln!("[client] timeout after send_done, exiting");
                    break;
                }
            }
            Err(e) => {
                eprintln!("[client] recv error: {e}");
                break;
            }
        }
    }

    drop(socket);
    send_handle.join().ok();
}
