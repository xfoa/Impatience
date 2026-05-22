use crate::cli::interface::Count;
use crate::cli::synctest::common;
use impatience::clock::SyncedClock;
use impatience::net::packet::{Packet, PingPacket, SyncPacket};
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

        send_done_flag.store(true, Ordering::SeqCst);
    });

    let mut buf = [0u8; common::MAX_MSG_SIZE];
    let mut send_done_time: Option<u64> = None;

    loop {
        if send_done.load(Ordering::SeqCst) {
            if send_done_time.is_none() {
                send_done_time = Some(common::now_usec());
            } else if common::now_usec().saturating_sub(send_done_time.unwrap()) > 2_000_000 {
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
                            )
                        );
                    }
                    Ok(Packet::Sync(sync_pkt)) => {
                        let mut c = clock.lock().unwrap();
                        c.update_with_sync(sync_pkt.min_delta_ts());
                        let min_delta = c.get_sync_delta().to_unsigned();
                        let synced = c.is_synchronized();
                        println!(
                            "{}",
                            common::format_sync_stats("client", min_delta, synced)
                        );
                    }
                    _ => {}
                }
            }
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
                if send_done_time.is_some()
                    && common::now_usec().saturating_sub(send_done_time.unwrap()) > 2_000_000
                {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    drop(socket);
    send_handle.join().ok();
}
