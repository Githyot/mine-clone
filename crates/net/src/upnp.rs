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

/// Ask the router to forward UDP `port` to this machine. The answer comes later.
pub fn open(port: u16) -> Receiver<Result<Mapping, String>> {
    let (tx, rx) = bounded(1);
    let _ = std::thread::Builder::new().name("upnp".into()).spawn(move || {
        let _ = tx.send(open_now(port));
    });
    rx
}

fn open_now(port: u16) -> Result<Mapping, String> {
    let opts = SearchOptions { timeout: Some(Duration::from_secs(3)), ..Default::default() };
    let gw = search_gateway(opts).map_err(|e| format!("roteador sem UPnP ({e})"))?;
    let local = local_ip_towards(gw.addr.ip()).ok_or("sem endereço local")?;
    let ext_ip = match gw.get_external_ip().map_err(|e| e.to_string())? {
        IpAddr::V4(ip) => ip,
        IpAddr::V6(_) => return Err("o roteador só deu IPv6".into()),
    };
    gw.add_port(PortMappingProtocol::UDP, port, SocketAddr::new(IpAddr::V4(local), port), 0, "Mine Clone")
        .or_else(|_| gw.add_port(PortMappingProtocol::UDP, port, SocketAddr::new(IpAddr::V4(local), port), 7200, "Mine Clone"))
        .map_err(|e| format!("o roteador recusou abrir a porta ({e})"))?;
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
