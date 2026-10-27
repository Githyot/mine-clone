//! Voice chat on the wire: small Opus packets sent as QUIC datagrams, apart from the game's
//! stream, so a big chunk never holds a word back and a lost packet is simply skipped (the
//! decoder fills the gap). The host is the hub: guests send it their voice, and it passes each
//! packet on only to the players who should hear it, using the table the game keeps up to date
//! (`Host::set_voice_listener`). Routing runs on the network threads, so voice never waits
//! for a frame.
//!
//! Two lanes: `Group` (like a call: everyone in the same group hears it at full volume,
//! wherever they are) and `Near` (push to talk: only players within `RANGE` blocks in the same
//! dimension get it; the game places it in 3D around them).

use std::sync::Arc;

/// How far a `Near` voice carries, in blocks.
pub const RANGE: f32 = 48.0;
/// Bigger datagrams are not voice; dropped.
pub const MAX_PACKET: usize = 600;
const HEAD: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    Group,
    Near,
}

/// One 20 ms frame of someone's voice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet<'a> {
    /// The speaker's player id (the host fills it in for its guests; the host itself is 0).
    pub from: u32,
    pub seq: u16,
    pub lane: Lane,
    pub group: u8,
    /// One Opus frame.
    pub data: &'a [u8],
}

impl<'a> Packet<'a> {
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(HEAD + self.data.len());
        b.extend_from_slice(&self.from.to_le_bytes());
        b.extend_from_slice(&self.seq.to_le_bytes());
        b.push(match self.lane {
            Lane::Group => 0,
            Lane::Near => 1,
        });
        b.push(self.group);
        b.extend_from_slice(self.data);
        b
    }

    pub fn decode(b: &'a [u8]) -> Option<Packet<'a>> {
        if b.len() <= HEAD || b.len() > MAX_PACKET {
            return None;
        }
        let lane = match b[6] {
            0 => Lane::Group,
            1 => Lane::Near,
            _ => return None,
        };
        Some(Packet { from: u32::from_le_bytes(b[0..4].try_into().ok()?), seq: u16::from_le_bytes([b[4], b[5]]), lane, group: b[7], data: &b[HEAD..] })
    }
}

/// Write the speaker into an encoded packet (the host does this, so guests can't pose as
/// someone else).
pub(crate) fn stamp(b: &mut [u8], from: u32) {
    if b.len() >= 4 {
        b[..4].copy_from_slice(&from.to_le_bytes());
    }
}

/// Where a player is and what they listen to, for routing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Listener {
    pub dim: u8,
    pub pos: [f32; 3],
    /// The group they are in (0 = none: group voices don't reach them).
    pub group: u8,
    /// Voice turned on (off = nothing is sent to them).
    pub on: bool,
}

impl Listener {
    /// Does a packet from a speaker standing at `from` reach this player?
    pub fn hears(&self, from: &Listener, lane: Lane, group: u8) -> bool {
        if !self.on {
            return false;
        }
        match lane {
            Lane::Group => group != 0 && group == self.group,
            Lane::Near => {
                // A little past the range, so a voice fades out rather than cutting off.
                let d2: f32 = (0..3).map(|i| (self.pos[i] - from.pos[i]).powi(2)).sum();
                self.dim == from.dim && d2 <= (RANGE + 4.0) * (RANGE + 4.0)
            }
        }
    }
}

/// Sends one encoded packet towards the other players; cheap to clone, usable from any thread.
#[derive(Clone)]
pub struct VoiceOut(pub(crate) Arc<dyn Fn(&[u8]) + Send + Sync>);

impl VoiceOut {
    pub fn send(&self, packet: &[u8]) {
        (self.0)(packet)
    }

    /// Goes nowhere (playing alone).
    pub fn none() -> VoiceOut {
        VoiceOut(Arc::new(|_| {}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packets_roundtrip() {
        let p = Packet { from: 7, seq: 65000, lane: Lane::Near, group: 3, data: &[1, 2, 3] };
        let mut b = p.encode();
        assert_eq!(Packet::decode(&b), Some(p));
        stamp(&mut b, 9);
        assert_eq!(Packet::decode(&b).unwrap().from, 9);
        assert!(Packet::decode(&b[..HEAD]).is_none());
        assert!(Packet::decode(&vec![0; MAX_PACKET + 1]).is_none());
    }

    #[test]
    fn who_hears_what() {
        let a = Listener { dim: 0, pos: [0.0, 64.0, 0.0], group: 1, on: true };
        let mut b = Listener { dim: 0, pos: [30.0, 64.0, 0.0], group: 1, on: true };
        assert!(b.hears(&a, Lane::Near, 0));
        assert!(b.hears(&a, Lane::Group, 1));
        assert!(!b.hears(&a, Lane::Group, 2));
        b.pos[0] = 500.0;
        assert!(!b.hears(&a, Lane::Near, 0));
        assert!(b.hears(&a, Lane::Group, 1), "groups ignore distance");
        b.pos[0] = 10.0;
        b.dim = 1;
        assert!(!b.hears(&a, Lane::Near, 0));
        b.on = false;
        assert!(!b.hears(&a, Lane::Group, 1));
    }
}
