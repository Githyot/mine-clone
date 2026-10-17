//! Finding worlds on the same network: a hosted world shouts its name a couple of times a
//! second on a UDP broadcast port; the multiplayer screen listens and lists what it hears.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const LAN_PORT: u16 = 24455;
const TAG: &str = "MINECLONE1";

/// Announces a hosted world until dropped.
pub struct Beacon {
    stop: Arc<AtomicBool>,
}

impl Beacon {
    pub fn start(world: &str, host: &str, port: u16) -> Beacon {
        let stop = Arc::new(AtomicBool::new(false));
        let s = stop.clone();
        let msg = format!("{TAG}\n{port}\n{world}\n{host}");
        let _ = std::thread::Builder::new().name("lan-beacon".into()).spawn(move || {
            let Ok(sock) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) else { return };
            let _ = sock.set_broadcast(true);
            while !s.load(Ordering::Relaxed) {
                let _ = sock.send_to(msg.as_bytes(), (Ipv4Addr::BROADCAST, LAN_PORT));
                // Same machine (testing, or a second copy of the game) even without broadcast.
                let _ = sock.send_to(msg.as_bytes(), (Ipv4Addr::LOCALHOST, LAN_PORT));
                std::thread::sleep(Duration::from_millis(1500));
            }
        });
        Beacon { stop }
    }
}

impl Drop for Beacon {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LanWorld {
    pub addr: SocketAddr,
    pub world: String,
    pub host: String,
}

/// Listens for beacons until dropped.
pub struct Listener {
    seen: Arc<Mutex<HashMap<SocketAddr, (LanWorld, Instant)>>>,
    stop: Arc<AtomicBool>,
}

impl Listener {
    pub fn start() -> Listener {
        let seen: Arc<Mutex<HashMap<SocketAddr, (LanWorld, Instant)>>> = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let (seen2, s) = (seen.clone(), stop.clone());
        let _ = std::thread::Builder::new().name("lan-listen".into()).spawn(move || {
            let Ok(sock) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, LAN_PORT)) else {
                log::warn!("LAN: port {LAN_PORT} busy, not listening for worlds");
                return;
            };
            let _ = sock.set_read_timeout(Some(Duration::from_millis(300)));
            let mut buf = [0u8; 512];
            while !s.load(Ordering::Relaxed) {
                let Ok((n, from)) = sock.recv_from(&mut buf) else { continue };
                let text = String::from_utf8_lossy(&buf[..n]);
                let mut parts = text.split('\n');
                if parts.next() != Some(TAG) {
                    continue;
                }
                let Some(port) = parts.next().and_then(|p| p.parse::<u16>().ok()) else {
                    continue;
                };
                let world = parts.next().unwrap_or("").chars().take(40).collect();
                let host = parts.next().unwrap_or("").chars().take(24).collect();
                let addr = SocketAddr::new(from.ip(), port);
                seen2.lock().unwrap().insert(addr, (LanWorld { addr, world, host }, Instant::now()));
            }
        });
        Listener { seen, stop }
    }

    /// Worlds heard from in the last few seconds.
    pub fn worlds(&self) -> Vec<LanWorld> {
        let mut m = self.seen.lock().unwrap();
        m.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(5));
        let mut v: Vec<LanWorld> = m.values().map(|(w, _)| w.clone()).collect();
        // The same world heard on loopback and on the LAN address shows once.
        v.sort_by_key(|w| (w.world.clone(), w.host.clone(), !w.addr.ip().is_loopback()));
        v.dedup_by(|a, b| a.world == b.world && a.host == b.host && a.addr.port() == b.addr.port());
        v
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
