//! Opening the hosted port on the home router (UPnP IGD), so friends on the internet can come
//! in without anyone touching router settings. Runs on its own thread; many routers have it
//! off, and then the menu says so and shows the manual way.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver};
use igd_next::{search_gateway, PortMappingProtocol, SearchOptions};

pub struct Mapping {
    pub external: SocketAddrV4,
}

impl Mapping {
    /// The router's "public" address is itself a private or shared one (CGNAT, a second
    /// router): the open port leads nowhere from the internet, so the code would not work.
    pub fn shared_address(&self) -> bool {
        not_public(*self.external.ip())
    }
}

/// Private, shared (100.64/10, CGNAT) or otherwise unreachable from the internet.
pub fn not_public(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified() || (o[0] == 100 && (64..128).contains(&o[1]))
}

/// Ask the router to forward UDP `port` to this machine. The answer comes later.
pub fn open(port: u16) -> Receiver<Result<Mapping, String>> {
    let (tx, rx) = bounded(1);
    let _ = std::thread::Builder::new().name("upnp".into()).spawn(move || {
        let _ = tx.send(open_now(port));
    });
    rx
}

fn open_now(port: u16) -> Result<Mapping, String> {
    // The system's own error text ("IO error: ...", in the OS language) goes to the log only;
    // the player gets a sentence that says what it means for them.
    let opts = SearchOptions { timeout: Some(Duration::from_secs(3)), ..Default::default() };
    let gw = search_gateway(opts).map_err(|e| {
        log::warn!("upnp: no gateway: {e}");
        "o roteador não respondeu (UPnP desligado ou não suportado)".to_string()
    })?;
    let local = local_ip_towards(gw.addr.ip()).ok_or("sem endereço na rede local")?;
    let ext_ip = match gw.get_external_ip() {
        Ok(IpAddr::V4(ip)) => ip,
        Ok(IpAddr::V6(_)) => return Err("o roteador só tem endereço IPv6".into()),
        Err(e) => {
            log::warn!("upnp: no external address: {e}");
            return Err("o roteador não informou o endereço da internet".into());
        }
    };
    gw.add_port(PortMappingProtocol::UDP, port, SocketAddr::new(IpAddr::V4(local), port), 0, "Mine Clone")
        .or_else(|_| gw.add_port(PortMappingProtocol::UDP, port, SocketAddr::new(IpAddr::V4(local), port), 7200, "Mine Clone"))
        .map_err(|e| {
            log::warn!("upnp: add_port {port}: {e}");
            "o roteador recusou abrir a porta".to_string()
        })?;
    Ok(Mapping { external: SocketAddrV4::new(ext_ip, port) })
}

/// Take the mapping back down (quietly; the router forgets it anyway after a while).
pub fn close(port: u16) {
    let _ = std::thread::Builder::new().name("upnp".into()).spawn(move || {
        let opts = SearchOptions { timeout: Some(Duration::from_secs(2)), ..Default::default() };
        if let Ok(gw) = search_gateway(opts) {
            let _ = gw.remove_port(PortMappingProtocol::UDP, port);
        }
    });
}

/// Our address on the network that leads to `to` (no packet is sent).
pub fn local_ip_towards(to: IpAddr) -> Option<Ipv4Addr> {
    let s = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    s.connect((to, 9)).ok()?;
    match s.local_addr().ok()?.ip() {
        IpAddr::V4(ip) => Some(ip),
        _ => None,
    }
}

/// This machine's LAN address (for the "same network" line in the menu).
pub fn lan_ip() -> Option<Ipv4Addr> {
    local_ip_towards(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1))).filter(|ip| !ip.is_unspecified())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_addresses() {
        for ip in ["10.0.0.1", "192.168.1.5", "172.20.0.1", "100.64.3.2", "100.127.255.1", "127.0.0.1"] {
            assert!(not_public(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["203.0.113.7", "100.128.0.1", "8.8.8.8", "172.32.0.1"] {
            assert!(!not_public(ip.parse().unwrap()), "{ip}");
        }
    }
}
