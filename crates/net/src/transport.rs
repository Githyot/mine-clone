//! QUIC transport (quinn) for one host and its guests.
//!
//! The game thread never waits on the network: `Host` and `Guest` run on a small tokio runtime
//! of their own and hand messages over through channels. Each connection carries one ordered,
//! reliable stream of length-prefixed frames, so a chunk and the block changes after it always
//! arrive in order. QUIC encrypts everything; the host makes a fresh self-signed certificate
//! each time it starts.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver};
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use crate::protocol::{decode, encode, ToGuest, ToHost};

const ALPN: &[u8] = b"mine-clone/1";
const MAX_FRAME: usize = 32 << 20;

pub type ConnId = u32;

pub enum HostEvent {
    Joined(ConnId, SocketAddr),
    Message(ConnId, ToHost),
    Left(ConnId, String),
}

pub enum GuestEvent {
    Connected,
    Message(ToGuest),
    Closed(String),
}

fn runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_name("net").enable_all().build()
}

fn transport() -> Arc<quinn::TransportConfig> {
    let mut t = quinn::TransportConfig::default();
    t.keep_alive_interval(Some(Duration::from_secs(4)));
    t.max_idle_timeout(Some(Duration::from_secs(20).try_into().unwrap()));
    // Chunks are big; let a lot be in flight.
    t.stream_receive_window((8u32 << 20).into());
    t.receive_window((16u32 << 20).into());
    Arc::new(t)
}

async fn read_frame(r: &mut quinn::RecvStream) -> Option<Vec<u8>> {
    let n = r.read_u32_le().await.ok()? as usize;
    if n > MAX_FRAME {
        return None;
    }
    let mut buf = vec![0; n];
    r.read_exact(&mut buf).await.ok()?;
    Some(buf)
}

async fn write_loop(mut w: quinn::SendStream, mut rx: mpsc::UnboundedReceiver<Vec<u8>>) {
    while let Some(frame) = rx.recv().await {
        if w.write_u32_le(frame.len() as u32).await.is_err() || w.write_all(&frame).await.is_err() {
            break;
        }
    }
    let _ = w.finish();
}

/// A hosted world: accepts guests on a UDP port.
pub struct Host {
    pub port: u16,
    events: Receiver<HostEvent>,
    conns: Arc<Mutex<HashMap<ConnId, (mpsc::UnboundedSender<Vec<u8>>, quinn::Connection)>>>,
    endpoint: quinn::Endpoint,
    rt: Option<tokio::runtime::Runtime>,
}

impl Host {
    /// Listen on `port` (0 = any free port), on IPv6 and IPv4 where the system allows both.
    pub fn start(port: u16) -> Result<Host, String> {
        let rt = runtime().map_err(|e| e.to_string())?;
        let cert = rcgen::generate_simple_self_signed(vec!["mine-clone".to_string()]).map_err(|e| e.to_string())?;
        let key = rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
        let mut tls = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| e.to_string())?
            .with_no_client_auth()
            .with_single_cert(vec![cert.cert.der().clone()], key.into())
            .map_err(|e| e.to_string())?;
        tls.alpn_protocols = vec![ALPN.to_vec()];
        let mut cfg = quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(tls).map_err(|e| e.to_string())?));
        cfg.transport_config(transport());
        let endpoint = {
            let _g = rt.enter();
            // Dual-stack first (IPv6 friends can come straight in), plain IPv4 if that fails.
            quinn::Endpoint::server(cfg.clone(), SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port))
                .or_else(|_| quinn::Endpoint::server(cfg, SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)))
                .map_err(|e| format!("porta {port}: {e}"))?
        };
        let port = endpoint.local_addr().map(|a| a.port()).unwrap_or(port);
        let (tx, events) = unbounded();
        let conns: Arc<Mutex<HashMap<ConnId, _>>> = Arc::new(Mutex::new(HashMap::new()));
        let next = Arc::new(AtomicU32::new(1));
        let (ep, conns2) = (endpoint.clone(), conns.clone());
        rt.spawn(async move {
            while let Some(incoming) = ep.accept().await {
                let (tx, conns, next) = (tx.clone(), conns2.clone(), next.clone());
                tokio::spawn(async move {
                    let Ok(conn) = incoming.await else { return };
                    let Ok(Ok((send, mut recv))) = tokio::time::timeout(Duration::from_secs(10), conn.accept_bi()).await else { return };
                    let id = next.fetch_add(1, Ordering::Relaxed);
                    let (wtx, wrx) = mpsc::unbounded_channel();
                    conns.lock().unwrap().insert(id, (wtx, conn.clone()));
                    let _ = tx.send(HostEvent::Joined(id, conn.remote_address()));
                    tokio::spawn(write_loop(send, wrx));
                    let why = loop {
                        match read_frame(&mut recv).await {
                            Some(f) => match decode::<ToHost>(&f) {
                                Some(m) => {
                                    if tx.send(HostEvent::Message(id, m)).is_err() {
                                        break "host stopped".to_string();
                                    }
                                }
                                None => break "mensagem inválida".to_string(),
                            },
                            None => break conn.close_reason().map(|r| r.to_string()).unwrap_or_else(|| "saiu".to_string()),
                        }
                    };
                    conns.lock().unwrap().remove(&id);
                    let _ = tx.send(HostEvent::Left(id, why));
                });
            }
        });
        Ok(Host { port, events, conns, endpoint, rt: Some(rt) })
    }

    pub fn poll(&self) -> Vec<HostEvent> {
        self.events.try_iter().collect()
    }

    pub fn send(&self, to: ConnId, m: &ToGuest) {
        self.send_raw(to, encode(m));
    }

    /// Send already encoded bytes (one encode for many guests).
    pub fn send_raw(&self, to: ConnId, bytes: Vec<u8>) {
        if let Some((tx, _)) = self.conns.lock().unwrap().get(&to) {
            let _ = tx.send(bytes);
        }
    }

    /// Say why, then hang up shortly after (so the reason arrives first).
    pub fn kick(&self, to: ConnId, why: &str) {
        self.send(to, &ToGuest::Bye(why.to_string()));
        if let (Some((_, conn)), Some(rt)) = (self.conns.lock().unwrap().get(&to).cloned(), &self.rt) {
            rt.spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                conn.close(0u32.into(), b"kick");
            });
        }
    }

    /// Round-trip time to a guest, in milliseconds.
    pub fn ping(&self, to: ConnId) -> Option<u32> {
        self.conns.lock().unwrap().get(&to).map(|(_, c)| c.rtt().as_millis() as u32)
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        for (_, (_, c)) in self.conns.lock().unwrap().drain() {
            c.close(0u32.into(), b"host closed");
        }
        self.endpoint.close(0u32.into(), b"host closed");
        if let Some(rt) = self.rt.take() {
            rt.shutdown_timeout(Duration::from_millis(300));
        }
    }
}

/// Our side of a connection to someone else's world.
pub struct Guest {
    events: Receiver<GuestEvent>,
    out: mpsc::UnboundedSender<Vec<u8>>,
    conn: Arc<Mutex<Option<quinn::Connection>>>,
    rt: Option<tokio::runtime::Runtime>,
}

impl Guest {
    /// Start connecting to `addr` ("host:port", "host", "[v6]:port" or a join code). Returns at
    /// once; `Connected` or `Closed` arrives through `poll`.
    pub fn connect(addr: &str) -> Guest {
        let (tx, events) = unbounded();
        let (out, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let conn_slot: Arc<Mutex<Option<quinn::Connection>>> = Arc::new(Mutex::new(None));
        let rt = match runtime() {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(GuestEvent::Closed(e.to_string()));
                return Guest { events, out, conn: conn_slot, rt: None };
            }
        };
        let addr = addr.trim().to_string();
        let slot = conn_slot.clone();
        rt.spawn(async move {
            let fail = |e: String| {
                let _ = tx.send(GuestEvent::Closed(e));
            };
            let target = match tokio::task::spawn_blocking(move || resolve(&addr)).await {
                Ok(Ok(a)) => a,
                Ok(Err(e)) => return fail(e),
                Err(e) => return fail(e.to_string()),
            };
            let bind = if target.is_ipv6() { SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0) } else { SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0) };
            let mut ep = match quinn::Endpoint::client(bind) {
                Ok(e) => e,
                Err(e) => return fail(e.to_string()),
            };
            let mut tls = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])
                .expect("tls 1.3")
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(AnyCert(rustls::crypto::ring::default_provider())))
                .with_no_client_auth();
            tls.alpn_protocols = vec![ALPN.to_vec()];
            let mut cfg = quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(tls).expect("quic tls")));
            cfg.transport_config(transport());
            ep.set_default_client_config(cfg);
            let connecting = match ep.connect(target, "mine-clone") {
                Ok(c) => c,
                Err(e) => return fail(e.to_string()),
            };
            let conn = match tokio::time::timeout(Duration::from_secs(12), connecting).await {
                Ok(Ok(c)) => c,
                Ok(Err(e)) => return fail(format!("não conectou: {e}")),
                Err(_) => return fail("ninguém respondeu nesse endereço".to_string()),
            };
            let (send, mut recv) = match conn.open_bi().await {
                Ok(s) => s,
                Err(e) => return fail(e.to_string()),
            };
            *slot.lock().unwrap() = Some(conn.clone());
            let (wtx, wrx) = mpsc::unbounded_channel();
            tokio::spawn(write_loop(send, wrx));
            tokio::spawn(async move {
                while let Some(f) = out_rx.recv().await {
                    if wtx.send(f).is_err() {
                        break;
                    }
                }
            });
            let _ = tx.send(GuestEvent::Connected);
            let why = loop {
                match read_frame(&mut recv).await {
                    Some(f) => match decode::<ToGuest>(&f) {
                        Some(m) => {
                            if tx.send(GuestEvent::Message(m)).is_err() {
                                break String::new();
                            }
                        }
                        None => break "mensagem inválida do anfitrião".to_string(),
                    },
                    None => break conn.close_reason().map(|r| r.to_string()).unwrap_or_else(|| "conexão encerrada".to_string()),
                }
            };
            let _ = tx.send(GuestEvent::Closed(why));
            ep.wait_idle().await;
        });
        Guest { events, out, conn: conn_slot, rt: Some(rt) }
    }

    pub fn poll(&self) -> Vec<GuestEvent> {
        self.events.try_iter().collect()
    }

    pub fn send(&self, m: &ToHost) {
        let _ = self.out.send(encode(m));
    }

    pub fn ping(&self) -> Option<u32> {
        self.conn.lock().unwrap().as_ref().map(|c| c.rtt().as_millis() as u32)
    }
}

impl Drop for Guest {
    fn drop(&mut self) {
        if let Some(c) = self.conn.lock().unwrap().take() {
            c.close(0u32.into(), b"bye");
        }
        if let Some(rt) = self.rt.take() {
            rt.shutdown_timeout(Duration::from_millis(300));
        }
    }
}

/// "1.2.3.4:5", "name:5", "name" (default port), "[::1]:5" or a join code.
pub fn resolve(text: &str) -> Result<SocketAddr, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("digite um endereço ou código".to_string());
    }
    if let Some(a) = crate::codes::decode(t) {
        return Ok(a);
    }
    if let Ok(a) = t.parse::<SocketAddr>() {
        return Ok(a);
    }
    if let Ok(ip) = t.trim_matches(|c| c == '[' || c == ']').parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, crate::protocol::DEFAULT_PORT));
    }
    let with_port = if t.rsplit_once(':').is_some_and(|(_, p)| p.parse::<u16>().is_ok()) { t.to_string() } else { format!("{t}:{}", crate::protocol::DEFAULT_PORT) };
    with_port
        .to_socket_addrs()
        .map_err(|e| format!("endereço desconhecido: {e}"))?
        .next()
        .ok_or_else(|| "endereço desconhecido".to_string())
}

/// Hosts make a new self-signed certificate each run, so there is no authority to check
/// against; the connection is still encrypted. (Who may join is the host's call: password and
/// whitelist.)
#[derive(Debug)]
struct AnyCert(rustls::crypto::CryptoProvider);

impl rustls::client::danger::ServerCertVerifier for AnyCert {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::PlayerState;
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
    fn host_and_guest_talk() {
        let host = Host::start(0).expect("host");
        let guest = Guest::connect(&format!("127.0.0.1:{}", host.port));
        wait(|| guest.poll().into_iter().find(|e| matches!(e, GuestEvent::Connected)).map(|_| ()));
        guest.send(&ToHost::Chat("oi".into()));
        let big = vec![7u8; 3 << 20];
        guest.send(&ToHost::State(PlayerState { held: "x".into(), ..Default::default() }));
        let mut id = None;
        let mut got = Vec::new();
        wait(|| {
            for e in host.poll() {
                match e {
                    HostEvent::Joined(i, _) => id = Some(i),
                    HostEvent::Message(_, m) => got.push(m),
                    HostEvent::Left(..) => panic!("left"),
                }
            }
            (got.len() == 2).then_some(())
        });
        assert_eq!(got[0], ToHost::Chat("oi".into()));
        let id = id.unwrap();
        host.send(id, &ToGuest::Chunk { dim: 0, pos: [1, 2], data: big.clone() });
        host.send(id, &ToGuest::PlayerLeft(9));
        let mut msgs = Vec::new();
        wait(|| {
            for e in guest.poll() {
                if let GuestEvent::Message(m) = e {
                    msgs.push(m);
                }
            }
            (msgs.len() == 2).then_some(())
        });
        assert_eq!(msgs[0], ToGuest::Chunk { dim: 0, pos: [1, 2], data: big });
        assert_eq!(msgs[1], ToGuest::PlayerLeft(9));
        host.kick(id, "tchau");
        wait(|| guest.poll().into_iter().find_map(|e| match e {
            GuestEvent::Message(ToGuest::Bye(w)) => Some(w),
            _ => None,
        }));
        drop(guest);
        wait(|| host.poll().into_iter().find(|e| matches!(e, HostEvent::Left(..))).map(|_| ()));
    }

    #[test]
    fn nobody_home() {
        let g = Guest::connect("127.0.0.1:1");
        wait(|| g.poll().into_iter().find_map(|e| match e {
            GuestEvent::Closed(w) => Some(w),
            _ => None,
        }));
    }

    #[test]
    fn addresses() {
        assert_eq!(resolve("1.2.3.4:5").unwrap(), "1.2.3.4:5".parse().unwrap());
        assert_eq!(resolve("1.2.3.4").unwrap().port(), crate::protocol::DEFAULT_PORT);
        assert_eq!(resolve("localhost:7").unwrap().port(), 7);
        assert!(resolve("").is_err());
    }
}
