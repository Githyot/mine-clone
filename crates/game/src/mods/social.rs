//! The social mods, readied for multiplayer (Fase 7 brings the networking): the pieces that
//! don't need a connection are here and work in singleplayer already.
//!
//! - What Are They Up To (idea): the player's activity (typing, a screen open, idle) packed in
//!   two bytes for the player sync message. Idle five minutes and you doze off: "Zz" drifts
//!   up from your head.
//! - Chat Heads (idea): chat lines carry who sent them, so the head next to a line never has
//!   to be guessed; `head_rects` gives the face and hat squares of a skin.
//! - Essential (idea): invite codes like `K7Q-4MZ8-…` that carry an address and port, and a
//!   screenshot gallery count in the photo toast.
//! - Hytale Server Container (idea): the hosted world's settings and lists (`server.ron`):
//!   port, whitelist, operators, bans, password, player limit.

// Most of this is the interface Fase 7's networking plugs into; until then only the activity
// and the gallery are used in game.
#![allow(dead_code)]

use std::net::{Ipv4Addr, SocketAddrV4};

use serde::{Deserialize, Serialize};

use crate::Game;

/// Seconds without input before the player dozes off.
pub const IDLE_SLEEP: f32 = 300.0;

/// Which kind of screen a player has open (never what is in it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenKind {
    None = 0,
    Inventory = 1,
    Container = 2,
    Menu = 3,
}

/// A player's body language, as other players will see it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Activity {
    pub typing: bool,
    /// Keys a second while typing, 0..15 in the packed form.
    pub typing_speed: f32,
    pub screen: ScreenKind,
    /// Seconds without input.
    pub idle: f32,
}

impl Default for Activity {
    fn default() -> Self {
        Activity { typing: false, typing_speed: 0.0, screen: ScreenKind::None, idle: 0.0 }
    }
}

impl Activity {
    pub fn asleep(&self) -> bool {
        self.idle >= IDLE_SLEEP
    }

    /// Two bytes: typing (1 bit), speed (4 bits), screen (2 bits), asleep (1 bit); then idle
    /// minutes. Sent only when it changes.
    pub fn pack(&self) -> [u8; 2] {
        let speed = (self.typing_speed.round() as u8).min(15);
        let a = u8::from(self.typing) | speed << 1 | (self.screen as u8) << 5 | u8::from(self.asleep()) << 7;
        [a, (self.idle / 60.0).min(255.0) as u8]
    }

    pub fn unpack(b: [u8; 2]) -> Activity {
        let screen = match (b[0] >> 5) & 3 {
            1 => ScreenKind::Inventory,
            2 => ScreenKind::Container,
            3 => ScreenKind::Menu,
            _ => ScreenKind::None,
        };
        let idle = if b[0] >> 7 == 1 { (b[1] as f32 * 60.0).max(IDLE_SLEEP) } else { b[1] as f32 * 60.0 };
        Activity { typing: b[0] & 1 == 1, typing_speed: ((b[0] >> 1) & 15) as f32, screen, idle }
    }
}

/// Who a chat line is from: the head drawn beside it comes from this, not from the text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Sender {
    Player { id: u64, name: String },
    Server,
    System,
    Mod(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatLine {
    pub from: Sender,
    pub text: String,
}

/// The face and hat squares (x, y, w, h) of a 64-wide skin, for a chat head.
pub fn head_rects() -> [(u32, u32, u32, u32); 2] {
    [(8, 8, 8, 8), (40, 8, 8, 8)]
}

/// A hosted world's settings, kept as `server.ron` in the world's folder.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
    pub max_players: u8,
    /// Only listed players may join (on by default for a hosted world).
    pub whitelist_on: bool,
    pub password: Option<String>,
    pub whitelist: Vec<String>,
    pub ops: Vec<String>,
    pub bans: Vec<String>,
    /// Ask the router to open the port (UPnP) before falling back to a relay.
    pub upnp: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig { port: 5520, max_players: 8, whitelist_on: true, password: None, whitelist: Vec::new(), ops: Vec::new(), bans: Vec::new(), upnp: true }
    }
}

impl ServerConfig {
    pub fn may_join(&self, name: &str, password: Option<&str>) -> bool {
        let named = |l: &Vec<String>| l.iter().any(|n| n.eq_ignore_ascii_case(name));
        if named(&self.bans) {
            return false;
        }
        if self.whitelist_on && !named(&self.whitelist) && !named(&self.ops) {
            return false;
        }
        self.password.as_deref().is_none_or(|p| Some(p) == password)
    }
}

/// Letters that can't be mistaken for one another.
const CODE: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// An invite code for an address and port: 48 bits in ten letters, grouped by dashes.
pub fn invite_code(addr: SocketAddrV4) -> String {
    let mut v = (u32::from(*addr.ip()) as u64) << 16 | addr.port() as u64;
    // Scramble so neighbouring addresses don't look alike (and the code isn't a readable IP).
    v ^= 0x5A3C_96E1_7B2D;
    let mut out = String::new();
    for k in 0..10 {
        if k == 3 || k == 7 {
            out.push('-');
        }
        out.push(CODE[(v & 31) as usize] as char);
        v >>= 5;
    }
    out
}

pub fn read_invite(code: &str) -> Option<SocketAddrV4> {
    let mut v: u64 = 0;
    let letters: Vec<u8> = code.bytes().filter(|c| *c != b'-' && *c != b' ').map(|c| c.to_ascii_uppercase()).collect();
    if letters.len() != 10 {
        return None;
    }
    for (k, c) in letters.iter().enumerate() {
        let d = CODE.iter().position(|x| x == c)? as u64;
        v |= d << (5 * k);
    }
    v ^= 0x5A3C_96E1_7B2D;
    if v >> 48 != 0 {
        return None;
    }
    Some(SocketAddrV4::new(Ipv4Addr::from((v >> 16) as u32), (v & 0xFFFF) as u16))
}

#[derive(Default)]
pub struct Social {
    pub me: Activity,
    last_look: (f32, f32),
    zz_t: f32,
}

impl Game {
    fn server_file(&self) -> std::path::PathBuf {
        crate::level_path_hint(&self.base, &self.world_name).join("server.ron")
    }

    /// The open world's hosting settings (defaults when it was never hosted).
    pub(crate) fn server_config(&self) -> ServerConfig {
        std::fs::read_to_string(self.server_file()).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default()
    }

    pub(crate) fn save_server_config(&self, c: &ServerConfig) {
        if let Ok(text) = ron::ser::to_string_pretty(c, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(self.server_file(), text);
        }
    }

    /// How many photos the gallery holds (for the screenshot toast).
    pub(crate) fn gallery_count(&self) -> usize {
        std::fs::read_dir(self.base.join("screenshots")).map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().extension().is_some_and(|x| x == "png")).count()).unwrap_or(0)
    }

    /// Keep the player's own activity up to date, and show them dozing when idle.
    pub(crate) fn update_social(&mut self, dt: f32) {
        let look = (self.yaw, self.pitch);
        let busy = !self.keys.is_empty() || self.mouse_held.iter().any(|m| *m) || !self.pad_held.is_empty() || look != self.mods.social.last_look;
        let s = &mut self.mods.social;
        s.last_look = look;
        s.me.idle = if busy { 0.0 } else { s.me.idle + dt };
        s.me.screen = match self.screen {
            crate::Screen::Playing => ScreenKind::None,
            crate::Screen::Inventory => match self.surv.open {
                crate::survival::Open::Chest(_) | crate::survival::Open::Furnace(_) | crate::survival::Open::Shop => ScreenKind::Container,
                _ => ScreenKind::Inventory,
            },
            _ => ScreenKind::Menu,
        };
        if s.me.asleep() {
            s.zz_t -= dt;
            if s.zz_t <= 0.0 {
                s.zz_t = 1.2;
                let at = self.body.eye() + glam::Vec3::Y * 0.6;
                let layer = self.glow_layer();
                self.particles.glow(at, glam::Vec3::new(0.2, 0.6, 0.0), layer, [0.9, 0.9, 1.0], 0.08, 2.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_codes_round_trip() {
        for (ip, port) in [([192, 168, 0, 12], 5520), ([8, 8, 4, 4], 1), ([255, 255, 255, 255], 65535), ([0, 0, 0, 0], 0)] {
            let a = SocketAddrV4::new(Ipv4Addr::from(ip), port);
            let code = invite_code(a);
            assert_eq!(code.len(), 12, "{code}");
            assert_eq!(read_invite(&code), Some(a));
            assert_eq!(read_invite(&code.to_lowercase()), Some(a));
        }
        assert_eq!(read_invite("not-a-code"), None);
    }

    #[test]
    fn activity_packs_in_two_bytes() {
        let a = Activity { typing: true, typing_speed: 9.0, screen: ScreenKind::Container, idle: 420.0 };
        let b = Activity::unpack(a.pack());
        assert!(b.typing && b.typing_speed == 9.0 && b.screen == ScreenKind::Container && b.asleep());
        assert_eq!(Activity::unpack(Activity::default().pack()), Activity::default());
    }

    #[test]
    fn whitelist_bans_and_password() {
        let mut c = ServerConfig { whitelist: vec!["Ana".into()], ..Default::default() };
        assert!(c.may_join("ana", None));
        assert!(!c.may_join("Bia", None));
        c.password = Some("x".into());
        assert!(!c.may_join("Ana", None) && c.may_join("Ana", Some("x")));
        c.bans.push("ANA".into());
        assert!(!c.may_join("Ana", Some("x")));
    }
}
