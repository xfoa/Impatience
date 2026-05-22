use crate::cli::interface::Count;
use crate::cli::synctest::common;
use impatience::clock::SyncedClock;
use impatience::net::packet::{Packet, PingPacket, StartClockPacket, SyncPacket};
use impatience::net::traits::{apply_probe, retrieve_probe, PeerSync};
use std::io;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn run(host: &str, port: u16, count: Count, interval_ms: u64, sync_interval_ms: u64) {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("client bind failed");
    socket
        .connect(format!("{}:{}", host, port))
        .expect("client connect failed");
    socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("set_read_timeout failed");

    let clock = Arc::new(Mutex::new(SyncedClock::new()));
    let start_usec = common::now_usec();
    clock.lock().unwrap().start(start_usec);

    // --- StartClock handshake ---
    eprintln!("[client] sending StartClock with start_usec={start_usec}");
    let pkt = StartClockPacket { start_usec };
    let bytes = Packet::StartClock(pkt)
        .to_bytes()
        .expect("serialize StartClock");
    socket.send(&bytes).expect("send StartClock");

    let mut buf = [0u8; common::MAX_MSG_SIZE];
    let mut acked = false;
    let peer_start: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));
    let mut handshake_retries = 0;
    while !acked && handshake_retries < 50 {
        match socket.recv(&mut buf) {
            Ok(n) => {
                eprintln!("[client] received {n} bytes");
                match Packet::from_bytes(&buf[..n]) {
                    Ok(Packet::AckStartClock(ack)) => {
                        eprintln!("[client] got AckStartClock peer_start_usec={}", ack.start_usec);
                        *peer_start.lock().unwrap() = Some(ack.start_usec);
                        acked = true;
                    }
                    Ok(other) => {
                        eprintln!("[client] unexpected packet during handshake: {other:?}");
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
                handshake_retries += 1;
                eprintln!("[client] handshake timeout #{handshake_retries}, retrying StartClock");
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
    let clock_send = Arc::clone(&clock);
    let send_done = Arc::new(AtomicBool::new(false));
    let send_done_flag = Arc::clone(&send_done);

    let send_handle = thread::spawn(move || {
        let mut seq: u32 = 0;
        let mut next_sync = start_usec.saturating_add(sync_interval_ms * 1000);

        let max_pings = match count {
            Count::Finite(n) => n,
            Count::Infinite => u64::MAX,
        };

        eprintln!("[client] send thread: max_pings={max_pings}, interval_ms={interval_ms}");

        for _ in 0..max_pings {
            let now = common::now_usec();

            if now >= next_sync {
                let mut pkt = SyncPacket::default();
                let c = clock_send.lock().unwrap();
                apply_probe(&c, &mut pkt, now);
                pkt.min_delta_ts24 = c.get_sync_delta().to_unsigned();
                drop(c);

                if let Ok(bytes) = Packet::Sync(pkt).to_bytes() {
                    let _ = socket_send.send(&bytes);
                }
                next_sync = now.saturating_add(sync_interval_ms * 1000);
            }

            let mut pkt = PingPacket::default();
            {
                let c = clock_send.lock().unwrap();
                apply_probe(&c, &mut pkt, now);
            }
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
                send_done_time = Some(common::now_usec());
                eprintln!("[client] send_done detected, starting 2s grace period");
            } else if common::now_usec().saturating_sub(send_done_time.unwrap()) > 2_000_000 {
                eprintln!("[client] grace period expired, exiting");
                break;
            }
        }

        match socket.recv(&mut buf) {
            Ok(n) => {
                match Packet::from_bytes(&buf[..n]) {
                    Ok(Packet::Pong(pong)) => {
                        let now = common::now_usec();
                        let mut c = clock.lock().unwrap();
                        let _owd = retrieve_probe(&mut c, &pong, now);
                        let local_ms = c.local_ms(now);
                        let correction = c.correction_ms();
                        let remote_ms = correction.map(|v| local_ms as i64 + v);
                        let min_delta = c.get_sync_delta().to_unsigned();
                        let synced = c.is_synchronized();
                        let peer_start_ms = peer_start.lock().unwrap().map(|v| v / 1000);
                        println!(
                            "{}",
                            common::format_probe_stats(
                                "pong",
                                pong.ping_seq,
                                local_ms,
                                remote_ms,
                                correction,
                                min_delta,
                                synced,
                                c.start_usec() / 1000,
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
                                "client",
                                min_delta,
                                synced,
                                c.start_usec() / 1000,
                                peer_start_ms,
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
                    && common::now_usec().saturating_sub(send_done_time.unwrap()) > 2_000_000
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
