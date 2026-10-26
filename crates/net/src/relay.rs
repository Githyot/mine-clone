//! The relay ("ponte"): for when the host's router won't open a port. Anyone with a machine
//! that has an open UDP port runs `mine-clone --relay`; the host connects out to it and gets a
//! room number, and its join code then names the relay and the room. Guests connect to the
//! relay, which passes their bytes to the host and back unchanged.
//!
//! On the wire, each side opens one stream to the relay and starts it with a byte: `H` (a
//! host, answered with its room as two bytes) or `J` and the room (a guest). For each guest
//! the relay opens a new stream to the host, starting with the guest's address (16 bytes of
//! IPv6, IPv4 mapped, and the port), and then copies both ways.
//!
//! Each hop is encrypted, but the relay itself sees the game's messages: use one you trust.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::AsyncReadExt;

use crate::transport::{client_endpoint, resolve_relay, runtime, server_config};

pub(crate) const ALPN: &[u8] = b"mine-clone-relay/1";
/// The relay's usual port (one past the game's).
pub const DEFAULT_PORT: u16 = crate::protocol::DEFAULT_PORT + 1;
/// Rooms one relay holds at most.
const MAX_ROOMS: usize = 4096;

/// Connect to a relay.
pub(crate) async fn dial(addr: &str) -> Result<quinn::Connection, String> {
    let a = addr.to_string();
    let target = tokio::task::spawn_blocking(move || resolve_relay(&a)).await.map_err(|e| e.to_string())??;
    let ep = client_endpoint(target, ALPN)?;
    let connecting = ep.connect(target, "mine-clone").map_err(|e| e.to_string())?;
    match tokio::time::timeout(Duration::from_secs(12), connecting).await {
        Ok(Ok(c)) => Ok(c),
        Ok(Err(e)) => Err(format!("a ponte não respondeu: {e}")),
        Err(_) => Err("a ponte não respondeu".to_string()),
    }
}

/// As a host: ask for a room.
pub(crate) async fn register(conn: &quinn::Connection) -> Result<u16, String> {
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    send.write_all(b"H").await.map_err(|e| e.to_string())?;
    let room = recv.read_u16_le().await.map_err(|_| conn.close_reason().map(|r| r.to_string()).unwrap_or_else(|| "a ponte recusou".to_string()))?;
    // The control stream stays open as long as the host does.
    std::mem::forget((send, recv));
    Ok(room)
}

/// As a guest: the stream into a room.
pub(crate) async fn join(conn: &quinn::Connection, room: u16) -> Result<(quinn::SendStream, quinn::RecvStream), String> {
    let (mut send, recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    let mut head = vec![b'J'];
    head.extend_from_slice(&room.to_le_bytes());
    send.write_all(&head).await.map_err(|e| e.to_string())?;
    Ok((send, recv))
}

fn addr_bytes(a: SocketAddr) -> [u8; 18] {
    let ip = match a.ip() {
        IpAddr::V4(v4) => v4.to_ipv6_mapped(),
        IpAddr::V6(v6) => v6,
    };
    let mut b = [0; 18];
    b[..16].copy_from_slice(&ip.octets());
    b[16..].copy_from_slice(&a.port().to_le_bytes());
    b
}

/// As a host: the address a relayed guest came from.
pub(crate) async fn read_addr(recv: &mut quinn::RecvStream) -> Option<SocketAddr> {
    let mut b = [0u8; 18];
    recv.read_exact(&mut b).await.ok()?;
    let ip = Ipv6Addr::from(<[u8; 16]>::try_from(&b[..16]).ok()?);
    let ip = ip.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(ip));
    Some(SocketAddr::new(ip, u16::from_le_bytes([b[16], b[17]])))
}

type Rooms = Arc<Mutex<HashMap<u16, quinn::Connection>>>;

/// Run a relay on `port` until the process ends.
pub fn run(port: u16) -> Result<(), String> {
    let rt = runtime().map_err(|e| e.to_string())?;
    rt.block_on(async move {
        let cfg = server_config(ALPN)?;
        let ep = quinn::Endpoint::server(cfg.clone(), SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port))
            .or_else(|_| quinn::Endpoint::server(cfg, SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), port)))
            .map_err(|e| format!("porta {port}: {e}"))?;
        log::info!("relay listening on UDP port {}", ep.local_addr().map(|a| a.port()).unwrap_or(port));
        let rooms: Rooms = Arc::new(Mutex::new(HashMap::new()));
        while let Some(incoming) = ep.accept().await {
            tokio::spawn(visitor(incoming, rooms.clone()));
        }
        Ok(())
    })
}

async fn visitor(incoming: quinn::Incoming, rooms: Rooms) {
    let Ok(conn) = incoming.await else { return };
    let Ok(Ok((mut send, mut recv))) = tokio::time::timeout(Duration::from_secs(10), conn.accept_bi()).await else { return };
    let Ok(kind) = recv.read_u8().await else { return };
    match kind {
        b'H' => {
            let room = {
                let mut r = rooms.lock().unwrap();
                if r.len() >= MAX_ROOMS {
                    conn.close(1u32.into(), b"a ponte esta cheia");
                    return;
                }
                let mut room = 0;
                while room == 0 || r.contains_key(&room) {
                    room = (rand_u32() % 32767 + 1) as u16;
                }
                r.insert(room, conn.clone());
                room
            };
            log::info!("room {room}: host {}", conn.remote_address());
            if send.write_all(&room.to_le_bytes()).await.is_err() {
                rooms.lock().unwrap().remove(&room);
                return;
            }
            let why = conn.closed().await;
            rooms.lock().unwrap().remove(&room);
            log::info!("room {room} closed ({why})");
        }
        b'J' => {
            let Ok(room) = recv.read_u16_le().await else { return };
            let host = rooms.lock().unwrap().get(&room).cloned();
            let Some(host) = host else {
                conn.close(2u32.into(), "sala não encontrada na ponte".as_bytes());
                return;
            };
            let Ok((mut hsend, mut hrecv)) = host.open_bi().await else {
                conn.close(3u32.into(), "o anfitrião saiu da ponte".as_bytes());
                return;
            };
            if hsend.write_all(&addr_bytes(conn.remote_address())).await.is_err() {
                return;
            }
            log::info!("room {room}: guest {}", conn.remote_address());
            // Guest to host; when the guest stops, the host sees its stream end.
            tokio::spawn(async move {
                let _ = tokio::io::copy(&mut recv, &mut hsend).await;
                let _ = hsend.finish();
            });
            // Host to guest; when the host ends the stream (left, or kicked them), hang up.
            let _ = tokio::io::copy(&mut hrecv, &mut send).await;
            let _ = send.finish();
            tokio::time::sleep(Duration::from_millis(500)).await;
            conn.close(0u32.into(), b"fim");
        }
        _ => conn.close(4u32.into(), b"?"),
    }
}

/// Good enough for picking room numbers.
fn rand_u32() -> u32 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    h.finish() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ToGuest, ToHost};
    use crate::transport::{GuestEvent, HostEvent};
    use crate::{Guest, Host};
    use std::time::Instant;

    fn wait<T>(mut f: impl FnMut() -> Option<T>) -> T {
        let t = Instant::now();
        loop {
            if let Some(v) = f() {
                return v;
            }
            assert!(t.elapsed() < Duration::from_secs(20), "timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn through_the_relay() {
        let port = 24600 + (std::process::id() % 200) as u16;
        std::thread::spawn(move || run(port).unwrap());
        std::thread::sleep(Duration::from_millis(300));
        let host = Host::start(0).expect("host");
        host.use_relay(&format!("127.0.0.1:{port}"));
        let (relay, room) = wait(|| host.relay_room()).expect("room");
        let SocketAddr::V4(r4) = relay else { panic!("v4") };
        let code = crate::codes::encode_relay(r4, room);
        let guest = Guest::connect(&code);
        wait(|| guest.poll().into_iter().find(|e| matches!(e, GuestEvent::Connected)).map(|_| ()));
        guest.send(&ToHost::Chat("pela ponte".into()));
        let (id, said) = wait(|| {
            host.poll().into_iter().find_map(|e| match e {
                HostEvent::Message(id, ToHost::Chat(c)) => Some((id, c)),
                _ => None,
            })
        });
        assert_eq!(said, "pela ponte");
        host.send(id, &ToGuest::PlayerLeft(3));
        wait(|| guest.poll().into_iter().find(|e| matches!(e, GuestEvent::Message(ToGuest::PlayerLeft(3)))).map(|_| ()));
        // A kick ends only that guest; the room stays.
        host.kick(id, "tchau");
        wait(|| guest.poll().into_iter().find(|e| matches!(e, GuestEvent::Closed(_))).map(|_| ()));
        let again = Guest::connect(&format!("127.0.0.1:{port}#{room}"));
        wait(|| again.poll().into_iter().find(|e| matches!(e, GuestEvent::Connected)).map(|_| ()));
        // A room nobody holds says so.
        let lost = Guest::connect(&format!("127.0.0.1:{port}#{}", room % 32767 + 1));
        lost.send(&ToHost::Chat("oi".into()));
        let why = wait(|| {
            lost.poll().into_iter().find_map(|e| match e {
                GuestEvent::Closed(w) => Some(w),
                _ => None,
            })
        });
        assert!(why.contains("sala"), "{why}");
    }
}
