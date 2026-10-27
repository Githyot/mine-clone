//! Join codes: an IPv4 address and port written as ten easy letters ("K3F9Q-2ZB7M"), so a
//! friend can read it out over a call. Crockford base32: no I, L, O or U to mix up.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pub fn encode(addr: SocketAddrV4) -> String {
    let mut v: u64 = 0;
    for b in addr.ip().octets() {
        v = v << 8 | b as u64;
    }
    v = v << 16 | addr.port() as u64;
    let mut s = String::with_capacity(11);
    for i in (0..10).rev() {
        s.push(ALPHABET[((v >> (i * 5)) & 31) as usize] as char);
        if i == 5 {
            s.push('-');
        }
    }
    s
}

pub fn decode(code: &str) -> Option<SocketAddr> {
    let clean: Vec<u8> = code.bytes().filter(|c| !matches!(c, b'-' | b' ')).map(|c| c.to_ascii_uppercase()).collect();
    if clean.len() != 10 {
        return None;
    }
    let mut v: u64 = 0;
    for c in clean {
        let c = match c {
            b'I' | b'L' => b'1',
            b'O' => b'0',
            c => c,
        };
        let d = ALPHABET.iter().position(|&a| a == c)? as u64;
        v = v << 5 | d;
    }
    let port = (v & 0xFFFF) as u16;
    let ip = Ipv4Addr::from(((v >> 16) & 0xFFFF_FFFF) as u32);
    if port == 0 {
        return None;
    }
    Some(SocketAddr::V4(SocketAddrV4::new(ip, port)))
}

/// A code through a relay: the relay's address, then the room ("K3F9Q-2ZB7M-4XR").
pub fn encode_relay(relay: SocketAddrV4, room: u16) -> String {
    let mut s = encode(relay);
    s.push('-');
    for i in (0..3).rev() {
        s.push(ALPHABET[((room >> (i * 5)) & 31) as usize] as char);
    }
    s
}

pub fn decode_relay(code: &str) -> Option<(SocketAddr, u16)> {
    let clean: String = code.chars().filter(|c| !matches!(c, '-' | ' ')).collect();
    if clean.len() != 13 || !clean.is_ascii() {
        return None;
    }
    let addr = decode(&clean[..10])?;
    let mut room = 0u16;
    for c in clean[10..].bytes() {
        let c = match c.to_ascii_uppercase() {
            b'I' | b'L' => b'1',
            b'O' => b'0',
            c => c,
        };
        room = room << 5 | ALPHABET.iter().position(|&a| a == c)? as u16;
    }
    (room != 0).then_some((addr, room))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let a: SocketAddrV4 = "203.0.113.77:24454".parse().unwrap();
        let c = encode(a);
        assert_eq!(c.len(), 11);
        assert_eq!(decode(&c), Some(SocketAddr::V4(a)));
        assert_eq!(decode(&c.to_lowercase().replace('-', " ")), Some(SocketAddr::V4(a)));
        assert_eq!(decode("1.2.3.4:5"), None);
        assert_eq!(decode("localhost"), None);
        let r = encode_relay(a, 12345);
        assert_eq!(r.len(), 15);
        assert_eq!(decode_relay(&r), Some((SocketAddr::V4(a), 12345)));
        assert_eq!(decode(&r), None);
        assert_eq!(decode_relay(&c), None);
    }
}
