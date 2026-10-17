//! What host and guests say to each other.
//!
//! Everything is named by the game's own registries (block, item and mob *names*, or indices
//! once both sides proved they load the same content), so new blocks, items and mobs added by
//! later phases or mods travel without touching this file.

use serde::{Deserialize, Serialize};

/// Bumped whenever a message changes shape. Host and guest must match.
pub const PROTOCOL: u32 = 1;

/// Default UDP port for hosted worlds.
pub const DEFAULT_PORT: u16 = 24454;

/// Guest → host.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ToHost {
    /// First message on a connection.
    Hello {
        protocol: u32,
        name: String,
        /// Hash of the block, item and mob registries (see `content_hash`). Must match the host's.
        content: u64,
        password: String,
        /// Chunks around the guest the host keeps in sync.
        view: u8,
    },
    /// Where the guest is and how it looks, about 20 times a second.
    State(PlayerState),
    /// Blocks the guest changed (placing, breaking, or the world running in a dimension the
    /// host is not in).
    Blocks(BlockChanges),
    /// The guest hurt a mob the host runs.
    HurtMob {
        mob: u32,
        amount: f32,
        from: [f32; 3],
    },
    /// A chest, furnace or other container the guest changed. `data` is the game's own text
    /// form (None = removed).
    Container {
        dim: u8,
        pos: [i32; 3],
        data: Option<String>,
    },
    Chat(String),
    /// The guest's own things (inventory, health, where it stands) for the host to keep.
    SavePlayer(String),
}

/// Host → guest.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ToGuest {
    Welcome(Welcome),
    /// Refused or kicked; the text says why.
    Bye(String),
    /// A whole chunk the players changed since it was generated (`voxel::save::encode_chunk`).
    /// Untouched chunks are never sent: the guest generates them from the seed.
    Chunk {
        dim: u8,
        pos: [i32; 2],
        data: Vec<u8>,
    },
    Blocks(BlockChanges),
    /// Everyone else who is connected (the host included).
    Players(Vec<RemotePlayer>),
    PlayerLeft(u32),
    /// The mobs near the guest, run by the host.
    Mobs {
        dim: u8,
        list: Vec<MobState>,
    },
    /// Time of day and weather.
    Sky {
        time: f32,
        raining: bool,
        thunder: bool,
    },
    /// The guest was hurt (by a mob the host runs, or a blast).
    Hurt {
        amount: f32,
        from: Option<[f32; 3]>,
    },
    /// A blast went off near the guest.
    Explosion {
        pos: [f32; 3],
        power: f32,
    },
    /// The guest landed the killing blow: its loot is the guest's.
    Killed {
        mob: u16,
        pos: [f32; 3],
    },
    Container {
        dim: u8,
        pos: [i32; 3],
        data: Option<String>,
    },
    Chat {
        from: String,
        text: String,
    },
    /// Made (or no longer) an operator.
    Op(bool),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Welcome {
    pub id: u32,
    pub host_name: String,
    pub world: String,
    pub seed: u64,
    /// Generator options the guest needs to grow the same terrain.
    pub spread: i32,
    pub survival: bool,
    pub difficulty: u8,
    pub time: f32,
    pub raining: bool,
    pub thunder: bool,
    /// What the host kept of this player last time (None = first visit).
    pub player: Option<String>,
    /// May use the commands that change things (/op).
    pub op: bool,
}

/// Block changes in one dimension. Names travel once per message in `names`, changes point
/// into it.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct BlockChanges {
    pub dim: u8,
    pub names: Vec<String>,
    pub changes: Vec<([i32; 3], u16)>,
}

impl BlockChanges {
    pub fn new(dim: u8) -> Self {
        BlockChanges { dim, ..Default::default() }
    }

    pub fn push(&mut self, pos: [i32; 3], name: &str) {
        let i = match self.names.iter().position(|n| n == name) {
            Some(i) => i,
            None => {
                self.names.push(name.to_string());
                self.names.len() - 1
            }
        };
        self.changes.push((pos, i as u16));
    }

    pub fn iter(&self) -> impl Iterator<Item = ([i32; 3], &str)> {
        self.changes.iter().filter_map(|(p, i)| self.names.get(*i as usize).map(|n| (*p, n.as_str())))
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

pub mod flags {
    pub const ON_GROUND: u16 = 1;
    pub const IN_WATER: u16 = 2;
    pub const HEAD_IN_WATER: u16 = 4;
    pub const FLYING: u16 = 8;
    pub const SPRINTING: u16 = 16;
    pub const SITTING: u16 = 32;
    pub const HURT: u16 = 64;
    pub const BURNING: u16 = 128;
    pub const CREATIVE: u16 = 256;
    pub const DEAD: u16 = 512;
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PlayerState {
    pub dim: u8,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub flags: u16,
    pub sneaking: f32,
    pub crawling: f32,
    /// Attack swing 0..1.
    pub swing: f32,
    /// Item names: in hand, then head, chest, legs, feet ("" = nothing).
    pub held: String,
    pub armor: [String; 4],
}

impl PlayerState {
    pub fn has(&self, f: u16) -> bool {
        self.flags & f != 0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct RemotePlayer {
    pub id: u32,
    pub name: String,
    pub state: PlayerState,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct MobState {
    pub id: u32,
    /// Index in the mob registry (both sides proved they have the same one).
    pub def: u16,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub yaw: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub health: f32,
    pub max_health: f32,
    pub hurt_age: f32,
    pub walk: f32,
    pub walk_speed: f32,
    pub dead_age: f32,
    pub phase: f32,
    pub sleeping: bool,
    pub aggro: bool,
    pub on_fire: f32,
}

/// FNV-1a over a list of names: same content on both sides gives the same number.
pub fn content_hash<'a>(names: impl IntoIterator<Item = &'a str>) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for n in names {
        for b in n.bytes().chain(std::iter::once(0)) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

pub fn encode<T: Serialize>(m: &T) -> Vec<u8> {
    postcard::to_stdvec(m).unwrap_or_default()
}

pub fn decode<'a, T: Deserialize<'a>>(b: &'a [u8]) -> Option<T> {
    postcard::from_bytes(b).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_roundtrip() {
        let mut b = BlockChanges::new(1);
        b.push([1, 2, 3], "stone");
        b.push([4, 5, 6], "dirt");
        b.push([7, 8, 9], "stone");
        assert_eq!(b.names.len(), 2);
        let m = ToGuest::Blocks(b.clone());
        assert_eq!(decode::<ToGuest>(&encode(&m)), Some(m));
        let got: Vec<_> = b.iter().collect();
        assert_eq!(got[2], ([7, 8, 9], "stone"));
        let s = ToHost::State(PlayerState { pos: [1.0, 2.0, 3.0], held: "torch".into(), ..Default::default() });
        assert_eq!(decode::<ToHost>(&encode(&s)), Some(s));
    }

    #[test]
    fn content_hash_sees_order_and_names() {
        assert_eq!(content_hash(["a", "b"]), content_hash(["a", "b"]));
        assert_ne!(content_hash(["a", "b"]), content_hash(["b", "a"]));
        assert_ne!(content_hash(["ab"]), content_hash(["a", "b"]));
    }
}
