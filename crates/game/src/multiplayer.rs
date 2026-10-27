//! Multiplayer (roadmap Phase 7): host a world from the game ("Hospedar mundo" / "Abrir para
//! amigos") or join someone else's by join code, address or from the LAN list.
//!
//! The host's game is the authority: it runs the world, the mobs, time and weather, and keeps
//! each guest's things (`saves/<world>/players/<name>.ron`). A guest grows the untouched
//! terrain itself from the seed and only receives what players changed (whole chunks once,
//! then single block changes), like Craft's delta sync. Everything is sent by registry names
//! (blocks, items) or by index after both sides proved they load the same content, so new
//! blocks, items and mobs from later phases and mods travel without touching this file.
//!
//! What syncs: blocks (any change, from any system: players, fluids, redstone, explosions,
//! growth), players (position, pose, swing, held item, armour, name tags), mobs near each
//! guest (monsters hunt guests too; hits and kills by guests count, loot goes to the killer),
//! time of day, weather, containers (chests, furnaces, hoppers), chat with a few commands,
//! damage and blasts. A guest in a dimension the host is not in runs that dimension's world
//! and mobs itself; the host stores its block changes and passes them to others there.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddrV4;
use std::path::PathBuf;
use std::sync::Arc;

use glam::{IVec2, IVec3, Vec3};
use net::lan::{Beacon, LanWorld, Listener};
use net::protocol::{flags, BlockChanges, MobState, PlayerState, RemotePlayer, ToGuest, ToHost, Welcome, PROTOCOL};
use net::{ConnId, Guest, GuestEvent, Host, HostEvent};
use serde::{Deserialize, Serialize};
use voxel::block::BlockId;
use voxel::chunk::{split_xz, Chunk, ChunkPos};
use voxel::item::{Inventory, ItemStack};
use voxel::save::{decode_chunk, encode_chunk, WorldSave};
use voxel::worldgen::{Dim, Generator};
use winit::keyboard::KeyCode;

use crate::audio::Sfx;
use crate::entity::Bones;
use crate::item_render::GpuItem;
use crate::survival::{Container, Furnace, Open};
use crate::ui::{GRAY, WHITE, YELLOW};
use crate::{level_path_hint, player_model, Game, Screen};

/// Host's own player id.
const HOST_ID: u32 = 0;
/// Folder (next to `saves/`) where a guest keeps its copy of someone else's world.
const GUEST_WORLD: &str = "../.multijogador";
const SEND_EVERY: f32 = 0.05;
const CHUNK_EVERY: f32 = 0.25;
/// Mobs farther than this from a guest are not sent to it.
const MOB_RANGE: f32 = 72.0;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct NetConfig {
    pub name: String,
    pub last_addr: String,
    pub recent: Vec<String>,
    pub port: u16,
    /// Who can reach a world we open: only the same network, the internet (the router opens
    /// the port by UPnP) or the internet through a relay.
    pub reach: Reach,
    /// The password typed to join someone's world.
    pub password: String,
    /// The password our own open world asks for. Empty = anyone with the code may come in.
    pub host_password: String,
    /// A relay (`mine-clone --relay`) to also take guests through, when the router won't open
    /// the port. Empty = none.
    pub relay: String,
    /// Players at once in a world we open, ourselves included.
    pub max_players: u8,
    /// Guests play in: 0 = the same mode as the host, 1 = survival, 2 = creative.
    pub guest_mode: u8,
    /// Every guest may use the commands that change things (not only those in ops.txt).
    pub guest_cheats: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reach {
    /// Same network only (the LAN list and the LAN code).
    Lan,
    /// Also the internet: ask the router to open the port (UPnP).
    #[default]
    Internet,
    /// Also the internet, through a relay (no port to open).
    Relay,
}

impl Reach {
    fn label(self) -> &'static str {
        match self {
            Reach::Lan => "Só a mesma rede",
            Reach::Internet => "Internet (roteador)",
            Reach::Relay => "Internet pela ponte",
        }
    }

    fn next(self) -> Reach {
        match self {
            Reach::Lan => Reach::Internet,
            Reach::Internet => Reach::Relay,
            Reach::Relay => Reach::Lan,
        }
    }
}

/// Choices for the most players at once.
const MAX_PLAYERS: [u8; 11] = [2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 32];
const GUEST_MODES: [&str; 3] = ["Igual ao meu", "Sobrevivência", "Criativo"];
const GREEN: [u8; 4] = [85, 255, 85, 255];
const RED: [u8; 4] = [255, 110, 110, 255];

impl Default for NetConfig {
    fn default() -> Self {
        let name = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "Jogador".into());
        NetConfig {
            name: clean_name(&name),
            last_addr: String::new(),
            recent: Vec::new(),
            port: net::DEFAULT_PORT,
            reach: Reach::Internet,
            password: String::new(),
            host_password: String::new(),
            relay: String::new(),
            max_players: 8,
            guest_mode: 0,
            guest_cheats: false,
        }
    }
}

fn clean_name(n: &str) -> String {
    let s: String = n.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').take(16).collect();
    if s.is_empty() {
        "Jogador".into()
    } else {
        s
    }
}

/// Someone else, as we draw them.
pub struct Remote {
    pub name: String,
    pub state: PlayerState,
    pub(crate) shown: Vec3,
    shown_yaw: f32,
    anim: player_model::Animator,
}

impl Remote {
    fn new(name: String, state: PlayerState) -> Remote {
        let p = Vec3::from(state.pos);
        Remote { name, shown: p, shown_yaw: state.yaw, state, anim: player_model::Animator::default() }
    }

    /// How open their mouth is while they talk (voice.rs).
    pub(crate) fn set_talk(&mut self, open: f32) {
        self.anim.talk = open;
    }
}

struct ChatLine {
    text: String,
    color: [u8; 4],
    at: f32,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Field {
    #[default]
    None,
    Name,
    Addr,
    Password,
    /// "Abrir para amigos" screen.
    HostPassword,
    Port,
    Relay,
}

/// Everything multiplayer, kept in one field of `Game`.
#[derive(Default)]
pub struct Net {
    pub cfg: NetConfig,
    mode: Mode,
    pub remotes: HashMap<u32, Remote>,
    chat: Vec<ChatLine>,
    /// Chat input open (Some) and its text.
    typing: Option<String>,
    lan: Option<Listener>,
    field: Field,
    /// Multiplayer screen: what is happening ("Conectando...", an error).
    status: String,
    /// Multiplayer screen: choosing which of our worlds to host.
    picking_world: bool,
    /// Start hosting once the chosen world is loaded.
    pub(crate) host_when_ready: bool,
    loaded: bool,
    /// The key that opened the chat also arrives as text; drop it.
    skip_text: bool,
    /// Test hook MC_NET_BUILD: a guest builds a small pillar once it is in.
    test_build: bool,
    /// Test hook MC_CMD: chat lines to send once playing (";" between them).
    test_cmds: Vec<String>,
    /// The open world's rules (commands.rs), and which world they belong to.
    pub rules: crate::commands::Rules,
    rules_for: String,
    /// Scoreboard and teams (pvp.rs): the host's, or the copy it sent us.
    pub(crate) board: net::Board,
    pub(crate) board_dirty: bool,
    /// The next `damage` comes from another player: not scaled by difficulty.
    pub(crate) pvp_incoming: bool,
    /// Who last hurt each player (lower-case name → attacker, when), for kill messages.
    pub(crate) pvp_last: HashMap<String, (String, f32)>,
    /// Guests already told PvP is off.
    pub(crate) pvp_warned: HashSet<u32>,
    pub(crate) was_dead: bool,
    /// Test hook MC_PVP_HIT: hits made, when the last one was.
    pub(crate) pvp_test: (u32, f32),
    /// The guest whose command is running on the host (empty = our own).
    pub(crate) cmd_by: String,
    /// Dedicated server (`--server`): no player of our own, commands from the console.
    pub(crate) server: bool,
    pub(crate) console: Option<std::sync::mpsc::Receiver<String>>,
    /// The system clipboard (kept open: on Linux what we copy lives as long as it does).
    clip: Option<arboard::Clipboard>,
    /// The port typed on the "Abrir para amigos" screen.
    port_text: String,
    /// "Abrir para amigos" screen: a line saying what just happened ("Copiado!"), and when.
    friends_note: (String, f32),
}

#[derive(Default)]
enum Mode {
    #[default]
    Off,
    Host(Box<HostState>),
    Guest(Box<GuestState>),
}

struct GuestInfo {
    name: String,
    welcomed: bool,
    state: Option<PlayerState>,
    /// Chunks this guest is up to date on: (dim, chunk).
    sent: HashSet<(u8, ChunkPos)>,
    view: i32,
    /// Containers as last sent to (or heard from) this guest.
    containers: HashMap<IVec3, u64>,
    /// Voice chat: the group it listens to (0 = none) and whether its voice is on.
    voice: (u8, bool),
}

/// A dimension the host is not in, kept on disk for guests who are.
struct OffDim {
    save: WorldSave,
    gen: Arc<Generator>,
    chunks: HashMap<ChunkPos, Chunk>,
    dirty: HashSet<ChunkPos>,
}

struct HostState {
    host: Host,
    _beacon: Beacon,
    upnp: Option<crossbeam_channel::Receiver<Result<net::upnp::Mapping, String>>>,
    external: Option<SocketAddrV4>,
    /// What the router said, for people: (sentence, worked).
    upnp_note: (String, Option<bool>),
    /// What the relay said, for people (empty = no relay).
    relay_note: String,
    /// The code through the relay, once it gave us a room.
    relay_code: Option<String>,
    relay_waiting: bool,
    lan_ip: Option<std::net::Ipv4Addr>,
    guests: HashMap<ConnId, GuestInfo>,
    send_t: f32,
    chunk_t: f32,
    sky_t: f32,
    cont_t: f32,
    off: HashMap<u8, OffDim>,
    /// Who last hurt each mob, and when: their kill, their loot.
    credit: HashMap<u32, (ConnId, f32)>,
    /// Guest index for each entry of `Mobs::others`.
    others: Vec<ConnId>,
    bans: Vec<String>,
    /// Lower-case names of guests who may use the commands that change things.
    ops: Vec<String>,
}

struct GuestState {
    guest: Guest,
    joined: bool,
    my_id: u32,
    host_name: String,
    /// Changes for chunks we have not loaded yet.
    pending: HashMap<(u8, ChunkPos), Vec<(IVec3, BlockId)>>,
    send_t: f32,
    save_t: f32,
    cont_t: f32,
    /// The host runs our mobs (it is in our dimension).
    host_mobs: bool,
    /// Mob health as the host last told us: lower now means we hit it.
    mob_health: HashMap<u32, f32>,
    /// Where each mob should be (smoothed towards every frame).
    mob_target: HashMap<u32, (Vec3, f32)>,
    containers: HashMap<IVec3, u64>,
    addr: String,
    /// Our own difficulty, back when we leave (the host's applies while we visit).
    own_difficulty: u8,
    /// The host lets us use the commands that change things.
    op: bool,
}

fn hash_text(s: &str) -> u64 {
    net::content_hash([s])
}

#[derive(Serialize, Deserialize, Default)]
struct SavedContainer {
    furnace: bool,
    slots: Vec<Option<(String, u8, u16)>>,
    burn: f32,
    burn_max: f32,
    cook: f32,
}

/// A guest's own things, kept by the host between visits.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct PlayerSave {
    pos: [f32; 3],
    yaw: f32,
    pitch: f32,
    dim: u8,
    flying: bool,
    health: f32,
    max_health: f32,
    max_mana: f32,
    hunger: f32,
    saturation: f32,
    xp: u32,
    spawn: Option<[i32; 3]>,
    inv: Vec<Option<(String, u8, u16)>>,
    armor: Vec<Option<(String, u8, u16)>>,
    offhand: Option<(String, u8, u16)>,
    hotbar: Vec<String>,
}

impl Net {
    pub fn active(&self) -> bool {
        !matches!(self.mode, Mode::Off)
    }

    pub fn is_guest(&self) -> bool {
        matches!(self.mode, Mode::Guest(_))
    }

    pub fn is_host(&self) -> bool {
        matches!(self.mode, Mode::Host(_))
    }

    fn say(&mut self, text: impl Into<String>, color: [u8; 4], now: f32) {
        let text = text.into();
        log::info!("chat: {text}");
        self.chat.push(ChatLine { text, color, at: now });
        if self.chat.len() > 100 {
            self.chat.remove(0);
        }
    }

    fn config_path(base: &std::path::Path) -> PathBuf {
        base.join("multiplayer.ron")
    }

    fn load_config(&mut self, base: &std::path::Path) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        if let Some(c) = std::fs::read_to_string(Self::config_path(base)).ok().and_then(|t| ron::from_str::<NetConfig>(&t).ok()) {
            self.cfg = c;
        }
        self.cfg.name = clean_name(&self.cfg.name);
    }

    fn save_config(&self, base: &std::path::Path) {
        if let Ok(t) = ron::ser::to_string_pretty(&self.cfg, Default::default()) {
            let _ = std::fs::write(Self::config_path(base), t);
        }
    }
}

fn stack_out(items: &voxel::item::ItemRegistry, s: &Option<ItemStack>) -> Option<(String, u8, u16)> {
    s.map(|s| (items.get(s.item).name.clone(), s.count, s.damage))
}

fn stack_in(items: &voxel::item::ItemRegistry, s: &Option<(String, u8, u16)>) -> Option<ItemStack> {
    let (n, c, d) = s.as_ref()?;
    Some(ItemStack { item: items.id(n)?, count: *c, damage: *d, ench: Default::default(), sockets: 0 })
}

fn chunk_file(dir: &std::path::Path, p: ChunkPos) -> PathBuf {
    dir.join("chunks").join(format!("{}.{}.bin", p.x, p.y))
}

impl Game {
    // ---- shared helpers -------------------------------------------------------------------

    fn content_hash(&self) -> u64 {
        let blocks = self.reg.blocks.iter().map(|b| b.name.as_str());
        // Plugins' items are left out: a guest gets the host's plugins only after joining.
        let items = self.items.items.iter().take(self.plugins.base_items.max(1)).map(|i| i.name.as_str());
        let mobs = self.mobs.reg.defs.iter().map(|d| d.name.as_str());
        net::content_hash(blocks.chain(std::iter::once("|")).chain(items).chain(std::iter::once("|")).chain(mobs))
    }

    fn my_state(&self) -> PlayerState {
        let pose = self.pose(false);
        let mut f = 0;
        let mut set = |b: bool, bit: u16| {
            if b {
                f |= bit;
            }
        };
        set(pose.on_ground, flags::ON_GROUND);
        set(pose.in_water, flags::IN_WATER);
        set(pose.head_in_water, flags::HEAD_IN_WATER);
        set(pose.flying, flags::FLYING);
        set(pose.sprinting, flags::SPRINTING);
        set(pose.sitting, flags::SITTING);
        set(pose.hurt, flags::HURT);
        set(!self.survival, flags::CREATIVE);
        set(self.screen == Screen::Dead, flags::DEAD);
        set(self.net.server, flags::HIDDEN);
        let name = |s: &Option<ItemStack>| s.map(|s| self.items.get(s.item).name.clone()).unwrap_or_default();
        PlayerState {
            dim: self.dim.as_u8(),
            pos: self.body.pos.to_array(),
            vel: self.body.vel.to_array(),
            yaw: self.yaw,
            pitch: self.pitch,
            flags: f,
            sneaking: pose.sneaking,
            crawling: pose.crawling,
            swing: self.anim.swing,
            held: name(&self.held_stack()),
            armor: std::array::from_fn(|i| name(&self.surv.armor[i])),
            chest: match self.surv.open {
                Open::Chest(p) if self.screen == Screen::Inventory => Some(p.to_array()),
                _ => None,
            },
        }
    }

    fn container_text(&self, c: &Container) -> String {
        let sc = match c {
            Container::Chest(inv) => SavedContainer { slots: inv.slots.iter().map(|s| stack_out(&self.items, s)).collect(), ..Default::default() },
            Container::Furnace(f) => SavedContainer {
                furnace: true,
                slots: f.inv.slots.iter().map(|s| stack_out(&self.items, s)).collect(),
                burn: f.burn,
                burn_max: f.burn_max,
                cook: f.cook,
            },
        };
        ron::to_string(&sc).unwrap_or_default()
    }

    fn container_from(&self, text: &str) -> Option<Container> {
        let sc: SavedContainer = ron::from_str(text).ok()?;
        let mut inv = Inventory::new(sc.slots.len().max(1));
        for (i, s) in sc.slots.iter().enumerate() {
            inv.slots[i] = stack_in(&self.items, s);
        }
        Some(if sc.furnace { Container::Furnace(Furnace { inv, burn: sc.burn, burn_max: sc.burn_max, cook: sc.cook, xp: 0.0 }) } else { Container::Chest(inv) })
    }

    fn apply_container(&mut self, pos: IVec3, data: &Option<String>) {
        match data {
            Some(t) => {
                if let Some(c) = self.container_from(t) {
                    self.surv.containers.insert(pos, c);
                }
            }
            None => {
                self.surv.containers.remove(&pos);
            }
        }
    }

    /// Containers within `r` blocks of `at`, with a hash of their contents.
    fn containers_near(&self, at: Vec3, r: f32) -> Vec<(IVec3, u64, String)> {
        self.surv
            .containers
            .iter()
            .filter(|(p, _)| (p.as_vec3() - at).length() < r)
            .map(|(p, c)| {
                let t = self.container_text(c);
                (*p, hash_text(&t), t)
            })
            .collect()
    }

    /// Put block changes from the network into the world without echoing them back.
    fn apply_remote_blocks(&mut self, changes: &BlockChanges) -> Vec<(IVec3, BlockId)> {
        let log = self.world.net_log.take();
        let mut missed = Vec::new();
        for (p, name) in changes.iter() {
            let p = IVec3::from(p);
            let id = self.reg.id_or_air(name);
            if !self.world.set_block(p, id) {
                missed.push((p, id));
            }
        }
        self.world.net_log = log;
        missed
    }

    fn block_changes(&self, dim: u8, list: &[(IVec3, BlockId)]) -> BlockChanges {
        let mut b = BlockChanges::new(dim);
        for (p, id) in list {
            b.push(p.to_array(), &self.reg.get(*id).name);
        }
        b
    }

    fn player_save(&self) -> String {
        let s = PlayerSave {
            pos: self.body.pos.to_array(),
            yaw: self.yaw,
            pitch: self.pitch,
            dim: self.dim.as_u8(),
            flying: self.body.flying,
            health: self.health,
            max_health: self.max_health,
            max_mana: self.magic.max_mana,
            hunger: self.surv.hunger,
            saturation: self.surv.saturation,
            xp: self.surv.xp,
            spawn: self.surv.spawn.map(|p| p.to_array()),
            inv: self.surv.inv.slots.iter().map(|s| stack_out(&self.items, s)).collect(),
            armor: self.surv.armor.iter().map(|s| stack_out(&self.items, s)).collect(),
            offhand: stack_out(&self.items, &self.surv.offhand),
            hotbar: self.hotbar.iter().map(|&id| self.reg.get(id).name.clone()).collect(),
        };
        ron::to_string(&s).unwrap_or_default()
    }

    fn apply_player_save(&mut self, text: &str) {
        let Ok(s) = ron::from_str::<PlayerSave>(text) else { return };
        if s.dim != self.dim.as_u8() {
            self.load_dim(Dim::from_u8(s.dim), false);
        }
        self.body.pos = Vec3::from(s.pos);
        self.body.flying = s.flying && !self.survival;
        self.yaw = s.yaw;
        self.pitch = s.pitch;
        self.fall_peak = self.body.pos.y;
        self.needs_spawn = false;
        if s.max_health > 0.0 {
            self.max_health = s.max_health;
        }
        self.health = if s.health > 0.0 { s.health.min(self.max_health) } else { self.max_health };
        if s.max_mana > 0.0 {
            self.magic = crate::magic::Magic::new(s.max_mana);
        }
        self.surv.hunger = s.hunger.clamp(0.0, 20.0);
        self.surv.saturation = s.saturation.clamp(0.0, 20.0);
        self.surv.xp = s.xp;
        self.surv.spawn = s.spawn.map(IVec3::from);
        for (i, st) in s.inv.iter().enumerate().take(self.surv.inv.slots.len()) {
            self.surv.inv.slots[i] = stack_in(&self.items, st);
        }
        for (i, st) in s.armor.iter().enumerate().take(4) {
            self.surv.armor[i] = stack_in(&self.items, st);
        }
        self.surv.offhand = stack_in(&self.items, &s.offhand);
        if s.hotbar.len() == 9 {
            for (i, n) in s.hotbar.iter().enumerate() {
                self.hotbar[i] = self.reg.id_or_air(n);
            }
        }
    }

    fn net_say(&mut self, text: impl Into<String>, color: [u8; 4]) {
        let now = self.now;
        self.net.say(text, color, now);
    }

    // ---- per frame ------------------------------------------------------------------------

    /// Called once a frame from `Game::update`, after the world.
    pub(crate) fn net_update(&mut self, dt: f32) {
        if !self.net.loaded {
            self.net_test_hooks();
        }
        if self.net.host_when_ready && self.world_open && self.screen == Screen::Playing {
            self.net.host_when_ready = false;
            self.start_hosting();
        }
        // Test hook MC_FRIENDS=1: open the "Abrir para amigos" screen once playing (and hosting,
        // when MC_HOST is set too).
        static FRIENDS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if self.world_open
            && self.screen == Screen::Playing
            && !self.net.host_when_ready
            && std::env::var("MC_FRIENDS").is_ok()
            && !FRIENDS.swap(true, std::sync::atomic::Ordering::Relaxed)
        {
            self.net_pause_button();
        }
        if self.screen == Screen::Multiplayer {
            if self.net.lan.is_none() {
                self.net.lan = Some(Listener::start());
            }
        } else if !self.net.is_host() {
            self.net.lan = None;
        }
        if self.world_open && self.net.rules_for != self.world_name && !self.net.is_guest() {
            self.net.rules_for = self.world_name.clone();
            self.net.rules = std::fs::read_to_string(self.rules_path()).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default();
            self.net.board = crate::pvp::load_board(&self.base, &self.world_name);
        }
        if self.world_open {
            self.board_tick();
        }
        if !self.net.rules.weather_cycle && !self.net.is_guest() {
            self.weather.state.timer = self.weather.state.timer.max(600.0);
        }
        let joined = !self.net.is_guest() || matches!(&self.net.mode, Mode::Guest(g) if g.joined);
        if !self.net.test_cmds.is_empty() && self.world_open && self.screen == Screen::Playing && joined {
            for c in std::mem::take(&mut self.net.test_cmds) {
                self.send_chat(&c);
            }
        }
        match self.net.mode {
            Mode::Off => {}
            Mode::Host(_) => self.host_update(dt),
            Mode::Guest(_) => self.guest_update(dt),
        }
        // Others drift smoothly to where they were last seen.
        let k = 1.0 - (-dt * 14.0).exp();
        for r in self.net.remotes.values_mut() {
            let target = Vec3::from(r.state.pos);
            if (target - r.shown).length() > 8.0 {
                r.shown = target;
            } else {
                r.shown += (target - r.shown) * k;
            }
            let mut d = r.state.yaw - r.shown_yaw;
            d = (d + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            r.shown_yaw += d * k;
        }
    }

    /// Send block changes not yet sent (the world is about to be swapped for another dimension).
    pub(crate) fn net_flush_log(&mut self) {
        let log = self.world.net_log.as_mut().map(std::mem::take).unwrap_or_default();
        if log.is_empty() {
            return;
        }
        let dim = self.dim.as_u8();
        match &self.net.mode {
            Mode::Host(_) => self.host_send_blocks(dim, &log, None),
            Mode::Guest(g) if g.joined => g.guest.send(&ToHost::Blocks(self.block_changes(dim, &log))),
            _ => {}
        }
    }

    /// Does this machine run the world's simulation here (fluids, redstone, growth, mobs)?
    /// Not when we are a guest in the host's dimension: the host does, and we get the results.
    pub(crate) fn net_simulates(&self) -> bool {
        match &self.net.mode {
            Mode::Guest(g) => !g.host_mobs,
            _ => true,
        }
    }

    /// Send to the host, as a guest that is in.
    pub(crate) fn net_to_host(&self, m: &ToHost) {
        if let Mode::Guest(g) = &self.net.mode {
            if g.joined {
                g.guest.send(m);
            }
        }
    }

    /// Send to one guest, as the host.
    pub(crate) fn net_to_guest(&self, id: u32, m: &ToGuest) {
        if let Mode::Host(h) = &self.net.mode {
            h.host.send(id, m);
        }
    }

    /// Send to every guest, as the host.
    pub(crate) fn net_broadcast(&self, m: &ToGuest) {
        if self.net.is_host() {
            self.host_broadcast(m, None);
        }
    }

    /// A line for everyone (the host says it to its guests too).
    pub(crate) fn net_announce(&mut self, text: String) {
        self.net_broadcast(&ToGuest::Chat { from: String::new(), text: text.clone() });
        self.net_say(text, YELLOW);
    }

    /// A line typed in a dedicated server's console: a command, or chat from "Servidor".
    pub(crate) fn server_line(&mut self, line: &str) {
        self.send_chat(line);
    }

    /// Voice chat: tell the network who hears whom. The host keeps the table (its own place
    /// and each guest's, from where they stand and what they chose); a guest tells the host
    /// what it chose whenever that changes.
    pub(crate) fn voice_routes(&mut self) {
        let vs = &self.settings.voice;
        let on = vs.mode != crate::voice::VoiceMode::Off && self.world_open && !self.net.server;
        let group = if vs.mode == crate::voice::VoiceMode::Group { vs.group } else { 0 };
        match &self.net.mode {
            Mode::Host(h) => {
                let me = net::Listener { dim: self.dim.as_u8(), pos: self.body.eye().to_array(), group, on };
                h.host.set_voice_listener(net::HOST_CONN, (on && self.audio.is_some()).then_some(me));
                for (id, g) in &h.guests {
                    let l = g.state.as_ref().filter(|_| g.welcomed && g.voice.1).map(|s| net::Listener { dim: s.dim, pos: [s.pos[0], s.pos[1] + 1.6, s.pos[2]], group: g.voice.0, on: true });
                    h.host.set_voice_listener(*id, l);
                }
            }
            Mode::Guest(g) if g.joined => {
                if self.voice.told != Some((group, on)) {
                    g.guest.send(&ToHost::VoiceSet { group, on });
                    self.voice.told = Some((group, on));
                }
            }
            _ => self.voice.told = None,
        }
    }

    pub(crate) fn net_guest_joined(&self) -> bool {
        matches!(&self.net.mode, Mode::Guest(g) if g.joined)
    }

    /// Where our voice goes and others' comes from, in the game we are in.
    pub(crate) fn net_voice_link(&self) -> Option<(net::VoiceOut, crossbeam_channel::Receiver<Vec<u8>>)> {
        match &self.net.mode {
            Mode::Host(h) => Some((h.host.voice_out(), h.host.voice_in())),
            Mode::Guest(g) if g.joined => Some((g.guest.voice_out(), g.guest.voice_in())),
            _ => None,
        }
    }

    /// Our own player id in the game we are in.
    pub(crate) fn net_my_id(&self) -> u32 {
        match &self.net.mode {
            Mode::Guest(g) => g.my_id,
            _ => HOST_ID,
        }
    }

    /// May we use the commands that change things? Alone or hosting yes; a guest when op.
    pub(crate) fn net_cheats(&self) -> bool {
        match &self.net.mode {
            Mode::Guest(g) => g.op,
            _ => true,
        }
    }

    /// Send the time and weather to the guests right away (a command changed them).
    pub(crate) fn net_sky_now(&mut self) {
        if self.net.is_host() {
            self.host().sky_t = 1.0;
        }
    }

    /// The name of a player by id (our own included).
    pub(crate) fn net_name_of(&self, id: u32) -> String {
        if id == self.net_my_id() {
            return self.net.cfg.name.clone();
        }
        self.net.remotes.get(&id).map(|r| r.name.clone()).unwrap_or_default()
    }

    /// Where a player stands (our own feet for our id).
    pub(crate) fn net_pos_of(&self, id: u32) -> Option<Vec3> {
        if id == self.net_my_id() {
            return Some(self.body.pos);
        }
        self.net.remotes.get(&id).map(|r| Vec3::from(r.state.pos))
    }

    // ---- hosting --------------------------------------------------------------------------

    pub(crate) fn start_hosting(&mut self) {
        if self.net.active() || !self.world_open {
            return;
        }
        let port = self.net.cfg.port;
        let host = match Host::start(port).or_else(|e| {
            log::warn!("port {port} busy ({e}); trying any port");
            Host::start(0)
        }) {
            Ok(h) => h,
            Err(e) => {
                log::error!("could not host: {e}");
                self.toast("Não deu para abrir o mundo: a rede do computador recusou", 6.0);
                return;
            }
        };
        let port = host.port;
        let beacon = Beacon::start(&self.world_name, &self.net.cfg.name, port);
        let reach = self.net.cfg.reach;
        let upnp = (reach == Reach::Internet).then(|| net::upnp::open(port));
        // The relay: when chosen, or as a second way in when the router may refuse.
        let relay = std::env::var("MC_RELAY").unwrap_or_else(|_| if reach == Reach::Lan { String::new() } else { self.net.cfg.relay.clone() });
        if !relay.trim().is_empty() {
            host.use_relay(&relay);
        }
        let list = |f: &str| -> Vec<String> {
            std::fs::read_to_string(level_path_hint(&self.base, &self.world_name).join(f))
                .map(|t| t.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()).collect())
                .unwrap_or_default()
        };
        let (bans, ops) = (list("bans.txt"), list("ops.txt"));
        self.world.net_log = Some(Vec::new());
        self.net.mode = Mode::Host(Box::new(HostState {
            host,
            _beacon: beacon,
            upnp,
            external: None,
            upnp_note: match reach {
                Reach::Internet => ("pedindo ao roteador para abrir a porta...".into(), None),
                _ => ("não pedida (só a mesma rede ou a ponte)".into(), None),
            },
            relay_note: if relay.trim().is_empty() {
                if reach == Reach::Relay { "escreva o endereço da ponte".into() } else { String::new() }
            } else {
                "conectando...".into()
            },
            relay_code: None,
            relay_waiting: !relay.trim().is_empty(),
            lan_ip: net::upnp::lan_ip(),
            guests: HashMap::new(),
            send_t: 0.0,
            chunk_t: 0.0,
            sky_t: 0.0,
            cont_t: 0.0,
            off: HashMap::new(),
            credit: HashMap::new(),
            others: Vec::new(),
            bans,
            ops,
        }));
        self.net_say("Mundo aberto para amigos", YELLOW);
        self.toast("Mundo aberto para amigos (Esc > Amigos mostra o convite)", 5.0);
    }

    /// The code friends type to join: the router's address when UPnP worked, else the relay's,
    /// else the LAN one.
    pub(crate) fn net_invite_code(&self) -> Option<String> {
        let Mode::Host(h) = &self.net.mode else { return None };
        if h.external.is_none() && h.relay_code.is_some() {
            return h.relay_code.clone();
        }
        let a = h.external.or_else(|| h.lan_ip.map(|ip| SocketAddrV4::new(ip, h.host.port)))?;
        Some(net::codes::encode(a))
    }

    fn host(&mut self) -> &mut HostState {
        match &mut self.net.mode {
            Mode::Host(h) => h,
            _ => unreachable!("not hosting"),
        }
    }

    fn host_ref(&self) -> &HostState {
        match &self.net.mode {
            Mode::Host(h) => h,
            _ => unreachable!("not hosting"),
        }
    }

    fn host_broadcast(&self, m: &ToGuest, except: Option<ConnId>) {
        let h = self.host_ref();
        let bytes = net::encode(m);
        for (id, g) in &h.guests {
            if g.welcomed && Some(*id) != except {
                h.host.send_raw(*id, bytes.clone());
            }
        }
    }

    fn host_update(&mut self, dt: f32) {
        // Router answer.
        let mut note = None;
        if let Some(rx) = &self.host().upnp {
            if let Ok(r) = rx.try_recv() {
                note = Some(r);
            }
        }
        if let Some(r) = note {
            let h = self.host();
            h.upnp = None;
            match r {
                Ok(m) if m.shared_address() => {
                    // Opened, but on a router that is itself behind another (CGNAT).
                    log::warn!("upnp: router's outside address {} is not public", m.external);
                    net::upnp::close(h.host.port);
                    h.upnp_note = ("sua internet não aceita conexões de fora (CGNAT): use a ponte".into(), Some(false));
                }
                Ok(m) => {
                    h.external = Some(m.external);
                    h.upnp_note = (format!("porta aberta no roteador ({})", m.external), Some(true));
                }
                Err(e) => h.upnp_note = (format!("{e}. Para amigos de fora: abra a porta UDP {} no roteador ou use a ponte", h.host.port), Some(false)),
            }
            let worked = self.host_ref().external.is_some();
            let code = self.net_invite_code().unwrap_or_default();
            self.net_say(if worked { format!("Convite pela internet: {code}") } else { format!("Convite (mesma rede): {code}") }, YELLOW);
        }

        // Relay answer.
        if self.host_ref().relay_waiting {
            if let Some(r) = self.host_ref().host.relay_room() {
                let h = self.host();
                h.relay_waiting = false;
                match r {
                    Ok((addr, room)) => {
                        let code = match addr {
                            std::net::SocketAddr::V4(a) => net::codes::encode_relay(a, room),
                            a => format!("{a}#{room}"),
                        };
                        h.relay_code = Some(code.clone());
                        h.relay_note = "conectada".into();
                        self.net_say(format!("Convite pela ponte: {code}"), YELLOW);
                    }
                    Err(e) => {
                        log::warn!("relay: {e}");
                        h.relay_note = "não respondeu (o endereço está certo? ela está ligada?)".into();
                        self.net_say("A ponte não respondeu", YELLOW);
                    }
                }
            }
        }

        let events = self.host_ref().host.poll();
        for e in events {
            match e {
                HostEvent::Joined(id, addr) => {
                    log::info!("guest {id} connected from {addr}");
                    self.host()
                        .guests
                        .insert(id, GuestInfo { name: String::new(), welcomed: false, state: None, sent: HashSet::new(), view: 8, containers: HashMap::new(), voice: (0, false) });
                }
                HostEvent::Left(id, why) => {
                    if let Some(g) = self.host().guests.remove(&id) {
                        if g.welcomed {
                            self.net.remotes.remove(&id);
                            self.host_broadcast(&ToGuest::PlayerLeft(id), None);
                            let t = format!("{} saiu do jogo", g.name);
                            self.host_broadcast(&ToGuest::Chat { from: String::new(), text: t.clone() }, None);
                            self.net_say(t, YELLOW);
                            log::info!("guest {id} left: {why}");
                        }
                    }
                }
                HostEvent::Message(id, m) => self.host_message(id, m),
            }
        }

        // Block changes made here (by anyone or anything) go to the guests who have that area.
        let log = self.world.net_log.as_mut().map(std::mem::take).unwrap_or_default();
        if !log.is_empty() {
            self.host_send_blocks(self.dim.as_u8(), &log, None);
        }

        // Guests' surroundings stay loaded; monsters know where they are.
        let my_dim = self.dim.as_u8();
        let rd = self.settings.render_distance;
        let mut centers = Vec::new();
        let mut others = Vec::new();
        let mut ids = Vec::new();
        for (id, g) in &self.host_ref().guests {
            let Some(s) = &g.state else { continue };
            if s.dim != my_dim {
                continue;
            }
            let p = Vec3::from(s.pos);
            centers.push((ChunkPos::new((p.x.floor() as i32) >> 4, (p.z.floor() as i32) >> 4), g.view.min(rd)));
            if !s.has(flags::DEAD) {
                others.push((p, s.has(flags::CREATIVE)));
                ids.push(*id);
            }
        }
        self.world.extra_centers = centers;
        self.mobs.others = others;
        self.host().others = ids;

        let h = self.host();
        h.send_t += dt;
        h.chunk_t += dt;
        h.sky_t += dt;
        h.cont_t += dt;
        if h.send_t >= SEND_EVERY {
            h.send_t = 0.0;
            self.host_send_players_and_mobs();
        }
        if self.host_ref().chunk_t >= CHUNK_EVERY {
            self.host().chunk_t = 0.0;
            self.host_send_chunks();
        }
        if self.host_ref().sky_t >= 1.0 {
            self.host().sky_t = 0.0;
            let m = ToGuest::Sky { time: self.time_of_day, raining: self.weather.state.raining, thunder: self.weather.state.thunder };
            self.host_broadcast(&m, None);
        }
        if self.host_ref().cont_t >= 0.5 {
            self.host().cont_t = 0.0;
            self.host_send_containers();
        }
        // Old credit for kills runs out.
        let now = self.now;
        self.host().credit.retain(|_, (_, t)| now - *t < 6.0);
    }

    fn host_message(&mut self, id: ConnId, m: ToHost) {
        let welcomed = self.host_ref().guests.get(&id).is_some_and(|g| g.welcomed);
        match m {
            ToHost::Hello { protocol, name, content, password, view } => {
                if welcomed {
                    return;
                }
                let why = if protocol != PROTOCOL {
                    Some("versão diferente do jogo".to_string())
                } else if content != self.content_hash() {
                    Some("conteúdo diferente (blocos, itens ou mobs): use a mesma versão do jogo".to_string())
                } else if !self.net.cfg.host_password.is_empty() && password.is_empty() {
                    Some("este mundo pede senha: escreva no campo Senha".to_string())
                } else if !self.net.cfg.host_password.is_empty() && password != self.net.cfg.host_password {
                    Some("senha errada".to_string())
                } else if self.host_ref().bans.contains(&clean_name(&name).to_lowercase()) {
                    Some("banido deste mundo".to_string())
                } else if self.net_player_count() >= self.net.cfg.max_players.max(2) as usize {
                    Some(format!("o mundo está cheio ({} de {})", self.net_player_count(), self.net.cfg.max_players))
                } else {
                    None
                };
                if let Some(w) = why {
                    self.host_ref().host.kick(id, &w);
                    return;
                }
                // Names are unique on a server.
                let base = clean_name(&name);
                let mut name = base.clone();
                let mut n = 2;
                while name.eq_ignore_ascii_case(&self.net.cfg.name) || self.host_ref().guests.values().any(|g| g.welcomed && g.name.eq_ignore_ascii_case(&name))
                {
                    name = format!("{base}{n}");
                    n += 1;
                }
                let player =
                    std::fs::read_to_string(level_path_hint(&self.base, &self.world_name).join("players").join(format!("{}.ron", name.to_lowercase()))).ok();
                let w = Welcome {
                    id,
                    host_name: self.net.cfg.name.clone(),
                    world: self.world_name.clone(),
                    seed: self.seed,
                    spread: self.settings.structure_spread(),
                    survival: match self.net.cfg.guest_mode {
                        1 => true,
                        2 => false,
                        _ => self.survival,
                    },
                    difficulty: self.settings.difficulty,
                    time: self.time_of_day,
                    raining: self.weather.state.raining,
                    thunder: self.weather.state.thunder,
                    player,
                    op: self.net.cfg.guest_cheats || self.host_ref().ops.contains(&name.to_lowercase()),
                };
                let files = self.plugins.files();
                let h = self.host();
                if !files.is_empty() {
                    // First, so their items exist before the player's things arrive.
                    h.host.send(id, &ToGuest::Plugins(files));
                }
                h.host.send(id, &ToGuest::Welcome(w));
                if let Some(g) = h.guests.get_mut(&id) {
                    g.name = name.clone();
                    g.welcomed = true;
                    g.view = (view as i32).clamp(2, 16);
                }
                self.net_to_guest(id, &ToGuest::Board(self.net.board.clone()));
                if let Ok(t) = ron::to_string(&self.net.rules) {
                    self.net_to_guest(id, &ToGuest::Rules(t));
                }
                let t = format!("{name} entrou no jogo");
                self.host_broadcast(&ToGuest::Chat { from: String::new(), text: t.clone() }, Some(id));
                self.net_say(t, YELLOW);
                self.sfx(Sfx::Click);
            }
            _ if !welcomed => {}
            ToHost::State(s) => {
                let name = self.host_ref().guests.get(&id).map(|g| g.name.clone()).unwrap_or_default();
                let old_dim = self.host_ref().guests.get(&id).and_then(|g| g.state.as_ref().map(|s| s.dim));
                let was_dead = self.host_ref().guests.get(&id).and_then(|g| g.state.as_ref()).is_some_and(|s| s.has(flags::DEAD));
                if s.has(flags::DEAD) && !was_dead {
                    self.board_death(&name);
                }
                if let Some(g) = self.host().guests.get_mut(&id) {
                    if old_dim.is_some_and(|d| d != s.dim) {
                        // A new dimension: start its chunks afresh.
                        g.sent.retain(|(d, _)| *d != s.dim);
                    }
                    g.state = Some(s.clone());
                }
                match self.net.remotes.get_mut(&id) {
                    Some(r) => r.state = s,
                    None => {
                        self.net.remotes.insert(id, Remote::new(name, s));
                    }
                }
            }
            ToHost::Blocks(b) => {
                if b.dim == self.dim.as_u8() {
                    // Pass on everything the change sets off, except back to its sender.
                    let before = self.world.net_log.as_mut().map(std::mem::take).unwrap_or_default();
                    if !before.is_empty() {
                        self.host_send_blocks(self.dim.as_u8(), &before, None);
                    }
                    let missed = self.apply_remote_blocks(&b);
                    // Not loaded here (rare): straight into the save, which the world reads.
                    for (p, id) in &missed {
                        self.off_apply(b.dim, *p, *id);
                    }
                    if !missed.is_empty() {
                        self.off_flush();
                    }
                    // The sender already has these.
                    let list: Vec<(IVec3, BlockId)> = b.iter().map(|(p, n)| (IVec3::from(p), self.reg.id_or_air(n))).collect();
                    self.host_send_blocks(b.dim, &list, Some(id));
                } else {
                    for (p, n) in b.iter() {
                        let bid = self.reg.id_or_air(n);
                        self.off_apply(b.dim, IVec3::from(p), bid);
                    }
                    let list: Vec<(IVec3, BlockId)> = b.iter().map(|(p, n)| (IVec3::from(p), self.reg.id_or_air(n))).collect();
                    self.host_send_blocks(b.dim, &list, Some(id));
                }
            }
            ToHost::HurtMob { mob, amount, from } => {
                let now = self.now;
                if let Some(m) = self.mobs.list.iter_mut().find(|m| m.net_id == mob) {
                    m.hurt(amount.clamp(0.0, 100.0), Vec3::from(from));
                    self.host().credit.insert(mob, (id, now));
                }
            }
            ToHost::Container { dim, pos, data } => {
                let p = IVec3::from(pos);
                if dim == self.dim.as_u8() {
                    self.apply_container(p, &data);
                }
                let hash = data.as_deref().map(hash_text).unwrap_or(0);
                if let Some(g) = self.host().guests.get_mut(&id) {
                    g.containers.insert(p, hash);
                }
            }
            ToHost::Chat(text) => {
                let name = self.host_ref().guests.get(&id).map(|g| g.name.clone()).unwrap_or_default();
                let text: String = text.chars().take(200).collect();
                if text.starts_with('/') {
                    self.guest_command(id, &text);
                    return;
                }
                self.host_broadcast(&ToGuest::Chat { from: name.clone(), text: text.clone() }, None);
                self.net_say(format!("<{name}> {text}"), WHITE);
            }
            ToHost::HurtPlayer { target, amount, from } => self.pvp_resolve(id, target, amount, Vec3::from(from)),
            ToHost::SavePlayer(data) => {
                let name = self.host_ref().guests.get(&id).map(|g| g.name.to_lowercase()).unwrap_or_default();
                if name.is_empty() || data.len() > 1 << 20 {
                    return;
                }
                let dir = level_path_hint(&self.base, &self.world_name).join("players");
                let _ = std::fs::create_dir_all(&dir);
                if let Err(e) = std::fs::write(dir.join(format!("{name}.ron")), data) {
                    log::error!("could not keep {name}'s things: {e}");
                }
            }
            ToHost::VoiceSet { group, on } => {
                if let Some(g) = self.host().guests.get_mut(&id) {
                    g.voice = (group.min(9), on);
                }
            }
        }
    }

    /// Chat commands a guest may use.
    fn guest_command(&mut self, id: ConnId, text: &str) {
        let name = self.host_ref().guests.get(&id).map(|g| g.name.to_lowercase()).unwrap_or_default();
        let op = self.net.cfg.guest_cheats || self.host_ref().ops.contains(&name);
        let cmd = text.split_whitespace().next().unwrap_or("").to_lowercase();
        let reply = match cmd.as_str() {
            "/lista" | "/list" => self.player_list_line(),
            // A guest set the command of a command block (they are edited where the world runs).
            crate::cmdblock::SET_CMD if op => self.cmdblock_set_line(text),
            _ if crate::commands::scope(&cmd) == crate::commands::Scope::World => {
                self.net.cmd_by = self.host_ref().guests.get(&id).map(|g| g.name.clone()).unwrap_or_default();
                let r = self.run_command(text, op);
                self.net.cmd_by.clear();
                if op {
                    // Everyone sees the new sky right away, and who changed it.
                    self.host().sky_t = 1.0;
                    let t = format!("{name}: {r}");
                    self.host_broadcast(&ToGuest::Chat { from: String::new(), text: t.clone() }, Some(id));
                    self.net_say(t, YELLOW);
                }
                r
            }
            _ => crate::commands::HELP.to_string(),
        };
        self.host_ref().host.send(id, &ToGuest::Chat { from: String::new(), text: reply });
    }

    fn player_list_line(&self) -> String {
        // A dedicated server's own player is nobody.
        let mut names: Vec<String> = self.net.remotes.values().filter(|r| !r.state.has(flags::HIDDEN)).map(|r| r.name.clone()).collect();
        if !self.net.server {
            names.push(self.net.cfg.name.clone());
        }
        names.sort();
        names.dedup();
        format!("{} jogando: {}", names.len(), names.join(", "))
    }

    fn host_send_blocks(&self, dim: u8, list: &[(IVec3, BlockId)], except: Option<ConnId>) {
        let h = self.host_ref();
        for (gid, g) in &h.guests {
            if !g.welcomed || Some(*gid) == except {
                continue;
            }
            let Some(s) = &g.state else { continue };
            if s.dim != dim {
                continue;
            }
            // Only for chunks this guest is up to date on; other chunks come whole later.
            let mine: Vec<(IVec3, BlockId)> = list.iter().copied().filter(|(p, _)| g.sent.contains(&(dim, split_xz(p.x, p.z).0))).collect();
            if !mine.is_empty() {
                h.host.send(*gid, &ToGuest::Blocks(self.block_changes(dim, &mine)));
            }
        }
        // Chunks a guest is not up to date on come whole when it gets near them.
    }

    fn host_send_players_and_mobs(&mut self) {
        let me = RemotePlayer { id: HOST_ID, name: self.net.cfg.name.clone(), state: self.my_state() };
        let all: Vec<RemotePlayer> = std::iter::once(me)
            .chain(
                self.host_ref()
                    .guests
                    .iter()
                    .filter(|(_, g)| g.welcomed)
                    .filter_map(|(id, g)| g.state.clone().map(|s| RemotePlayer { id: *id, name: g.name.clone(), state: s })),
            )
            .collect();
        let my_dim = self.dim.as_u8();
        let h = self.host_ref();
        for (id, g) in &h.guests {
            if !g.welcomed {
                continue;
            }
            let list: Vec<RemotePlayer> = all.iter().filter(|p| p.id != *id).cloned().collect();
            h.host.send(*id, &ToGuest::Players(list));
            let Some(s) = &g.state else { continue };
            if s.dim != my_dim {
                continue;
            }
            let at = Vec3::from(s.pos);
            let mobs: Vec<MobState> = self
                .mobs
                .list
                .iter()
                .filter(|m| (m.body.pos - at).length() < MOB_RANGE)
                .map(|m| MobState {
                    id: m.net_id,
                    def: m.def as u16,
                    pos: m.body.pos.to_array(),
                    vel: m.body.vel.to_array(),
                    yaw: m.yaw,
                    head_yaw: m.head_yaw,
                    head_pitch: m.head_pitch,
                    health: m.health,
                    max_health: m.max_health,
                    hurt_age: m.hurt_age,
                    walk: m.walk,
                    walk_speed: m.walk_speed,
                    dead_age: m.dead_age,
                    phase: m.phase,
                    sleeping: m.sleeping,
                    aggro: m.aggro,
                    on_fire: m.on_fire,
                })
                .collect();
            h.host.send(*id, &ToGuest::Mobs { dim: my_dim, list: mobs });
        }
    }

    fn host_send_chunks(&mut self) {
        let my_dim = self.dim.as_u8();
        let ids: Vec<ConnId> = self.host_ref().guests.keys().copied().collect();
        for id in ids {
            let Some(g) = self.host_ref().guests.get(&id) else { continue };
            let Some(s) = g.state.clone() else { continue };
            if !g.welcomed {
                continue;
            }
            let view: i32 = g.view;
            let c = ChunkPos::new((s.pos[0].floor() as i32) >> 4, (s.pos[2].floor() as i32) >> 4);
            let mut budget = 24;
            let mut out = Vec::new();
            let mut checked = Vec::new();
            'scan: for r in 0..=view {
                for dz in -r..=r {
                    for dx in -r..=r {
                        if dx.abs().max(dz.abs()) != r || dx * dx + dz * dz > (view + 1) * (view + 1) {
                            continue;
                        }
                        let p = c + IVec2::new(dx, dz);
                        if self.host_ref().guests[&id].sent.contains(&(s.dim, p)) {
                            continue;
                        }
                        let data = if s.dim == my_dim { self.host_chunk_bytes(p) } else { self.off_chunk_bytes(s.dim, p) };
                        checked.push(p);
                        if let Some(d) = data {
                            out.push(ToGuest::Chunk { dim: s.dim, pos: [p.x, p.y], data: d });
                            budget -= 1;
                            if budget == 0 {
                                break 'scan;
                            }
                        }
                        if checked.len() > 400 {
                            break 'scan;
                        }
                    }
                }
            }
            let h = self.host();
            for m in &out {
                h.host.send(id, m);
            }
            if let Some(g) = h.guests.get_mut(&id) {
                for p in checked {
                    g.sent.insert((s.dim, p));
                }
                // Forget chunks far behind: if they change meanwhile, they come whole again.
                let far = (view + 4) * (view + 4);
                g.sent.retain(|(d, p)| {
                    *d != s.dim || {
                        let q = *p - c;
                        q.x * q.x + q.y * q.y <= far
                    }
                });
            }
        }
    }

    /// A chunk of the host's dimension as it is now, if players ever changed it.
    fn host_chunk_bytes(&self, p: ChunkPos) -> Option<Vec<u8>> {
        if let Some(c) = self.world.chunk(p) {
            if c.modified || self.world.save.has_chunk(p) {
                return Some(encode_chunk(&self.reg, c));
            }
            return None;
        }
        std::fs::read(chunk_file(&self.world.save.dir, p)).ok()
    }

    fn off_dim(&mut self, dim: u8) -> &mut OffDim {
        let root = level_path_hint(&self.base, &self.world_name);
        let d = Dim::from_u8(dim);
        let dir = if d.folder().is_empty() { root } else { root.join(d.folder()) };
        let (seed, reg, structures, spread) = (self.seed, self.reg.clone(), self.structures.clone(), self.settings.structure_spread());
        self.host().off.entry(dim).or_insert_with(|| OffDim {
            save: WorldSave::new(dir),
            gen: Arc::new(Generator::new(seed, &reg).with_structures(structures).with_dim(d).with_spread(spread)),
            chunks: HashMap::new(),
            dirty: HashSet::new(),
        })
    }

    fn off_chunk_bytes(&mut self, dim: u8, p: ChunkPos) -> Option<Vec<u8>> {
        let reg = self.reg.clone();
        let o = self.off_dim(dim);
        if let Some(c) = o.chunks.get(&p) {
            return c.modified.then(|| encode_chunk(&reg, c));
        }
        std::fs::read(chunk_file(&o.save.dir, p)).ok()
    }

    /// A block change in a dimension (or an area) the host has not loaded.
    fn off_apply(&mut self, dim: u8, p: IVec3, id: BlockId) {
        if p.y < 0 || p.y >= voxel::chunk::CHUNK_HEIGHT as i32 {
            return;
        }
        let reg = self.reg.clone();
        let (cp, x, z) = split_xz(p.x, p.z);
        let o = self.off_dim(dim);
        let c = o.chunks.entry(cp).or_insert_with(|| o.save.load_chunk(&reg, cp).unwrap_or_else(|| o.gen.generate(cp)));
        c.set(x, p.y as usize, z, id);
        c.modified = true;
        o.dirty.insert(cp);
    }

    fn off_flush(&mut self) {
        let reg = self.reg.clone();
        if let Mode::Host(h) = &mut self.net.mode {
            for o in h.off.values_mut() {
                for p in o.dirty.drain() {
                    if let Some(c) = o.chunks.get(&p) {
                        let _ = o.save.save_chunk(&reg, c);
                    }
                }
            }
            h.off.clear();
        }
    }

    /// Write the dimensions kept for guests (before the host's own world loads one of them),
    /// and hand a guest's things to the host.
    pub(crate) fn net_save(&mut self) {
        self.off_flush();
        if let Mode::Guest(g) = &self.net.mode {
            if g.joined {
                g.guest.send(&ToHost::SavePlayer(self.player_save()));
            }
        }
    }

    fn host_send_containers(&mut self) {
        let my_dim = self.dim.as_u8();
        let ids: Vec<ConnId> = self.host_ref().guests.keys().copied().collect();
        for id in ids {
            let Some(s) = self.host_ref().guests.get(&id).and_then(|g| g.state.clone()) else {
                continue;
            };
            if s.dim != my_dim {
                continue;
            }
            let near = self.containers_near(Vec3::from(s.pos), 48.0);
            let h = self.host();
            let Some(g) = h.guests.get_mut(&id) else { continue };
            let mut seen = HashSet::new();
            for (p, hash, text) in near {
                seen.insert(p);
                if g.containers.get(&p) != Some(&hash) {
                    g.containers.insert(p, hash);
                    h.host.send(id, &ToGuest::Container { dim: my_dim, pos: p.to_array(), data: Some(text) });
                }
            }
            // Broken containers.
            let gone: Vec<IVec3> = g
                .containers
                .iter()
                .filter(|(p, h)| **h != 0 && !seen.contains(*p) && (p.as_vec3() - Vec3::from(s.pos)).length() < 40.0)
                .map(|(p, _)| *p)
                .collect();
            for p in gone {
                g.containers.insert(p, 0);
                h.host.send(id, &ToGuest::Container { dim: my_dim, pos: p.to_array(), data: None });
            }
        }
    }

    /// Blasts reach guests too (the blocks they break travel like any change).
    pub(crate) fn net_explosion(&mut self, pos: Vec3, power: f32) {
        if !self.net.is_host() {
            return;
        }
        let my_dim = self.dim.as_u8();
        let h = self.host_ref();
        for (id, g) in &h.guests {
            if g.state.as_ref().is_some_and(|s| s.dim == my_dim && (Vec3::from(s.pos) - pos).length() < 64.0) {
                h.host.send(*id, &ToGuest::Explosion { pos: pos.to_array(), power });
            }
        }
    }

    /// Monster hits on guests (from `MobOutcome::remote_hits`).
    pub(crate) fn net_remote_hits(&mut self, hits: &[(usize, f32, Vec3)]) {
        if !self.net.is_host() {
            return;
        }
        let amount_k = [0.0, 0.5, 1.0, 1.5][self.settings.difficulty.min(3) as usize];
        let h = self.host_ref();
        for &(k, dmg, from) in hits {
            if let Some(id) = h.others.get(k) {
                h.host.send(*id, &ToGuest::Hurt { amount: dmg * amount_k, from: Some(from.to_array()) });
            }
        }
    }

    /// Mob deaths: the ones a guest finished off are theirs (their loot); the rest stay ours.
    pub(crate) fn net_kill_credit(&mut self, deaths: &[(usize, Vec3)]) -> Vec<(usize, Vec3)> {
        if !self.net.is_host() || deaths.is_empty() {
            return deaths.to_vec();
        }
        let mut mine = Vec::new();
        for &(def, pos) in deaths {
            let who = self
                .mobs
                .list
                .iter()
                .find(|m| m.def == def && m.body.pos == pos && !m.alive())
                .and_then(|m| self.host_ref().credit.get(&m.net_id))
                .map(|(id, _)| *id);
            match who.filter(|id| self.host_ref().guests.contains_key(id)) {
                Some(id) => self.host_ref().host.send(id, &ToGuest::Killed { mob: def as u16, pos: pos.to_array() }),
                None => mine.push((def, pos)),
            }
        }
        mine
    }

    // ---- joining --------------------------------------------------------------------------

    pub(crate) fn join(&mut self, addr: &str) {
        self.net_stop();
        let addr = addr.trim().to_string();
        if addr.is_empty() {
            self.net.status = "Digite o código ou o endereço do mundo.".into();
            return;
        }
        self.net.cfg.last_addr = addr.clone();
        self.net.cfg.recent.retain(|a| *a != addr);
        self.net.cfg.recent.insert(0, addr.clone());
        self.net.cfg.recent.truncate(5);
        self.net.save_config(&self.base);
        self.net.status = format!("Conectando a {addr}...");
        self.net.mode = Mode::Guest(Box::new(GuestState {
            guest: Guest::connect(&addr),
            joined: false,
            my_id: 0,
            host_name: String::new(),
            pending: HashMap::new(),
            send_t: 0.0,
            save_t: 0.0,
            cont_t: 0.0,
            host_mobs: false,
            mob_health: HashMap::new(),
            mob_target: HashMap::new(),
            containers: HashMap::new(),
            addr,
            own_difficulty: self.settings.difficulty,
            op: false,
        }));
    }

    fn guest(&mut self) -> &mut GuestState {
        match &mut self.net.mode {
            Mode::Guest(g) => g,
            _ => unreachable!("not a guest"),
        }
    }

    fn guest_ref(&self) -> &GuestState {
        match &self.net.mode {
            Mode::Guest(g) => g,
            _ => unreachable!("not a guest"),
        }
    }

    fn guest_update(&mut self, dt: f32) {
        let events = self.guest_ref().guest.poll();
        for e in events {
            match e {
                GuestEvent::Connected => {
                    let hello = ToHost::Hello {
                        protocol: PROTOCOL,
                        name: self.net.cfg.name.clone(),
                        content: self.content_hash(),
                        password: self.net.cfg.password.clone(),
                        view: self.settings.render_distance.clamp(2, 16) as u8,
                    };
                    self.guest_ref().guest.send(&hello);
                    self.net.status = "Conectado, entrando no mundo...".into();
                }
                GuestEvent::Closed(why) => {
                    self.guest_left(&why);
                    return;
                }
                GuestEvent::Message(m) => {
                    if let ToGuest::Bye(why) = &m {
                        let why = why.clone();
                        self.guest_left(&why);
                        return;
                    }
                    self.guest_message(m);
                }
            }
            if !self.net.is_guest() {
                return;
            }
        }
        if !self.guest_ref().joined || !self.world_open {
            return;
        }
        if self.net.test_build && self.screen == Screen::Playing {
            // Test hook: a gold pillar three blocks ahead, and a hello in the chat.
            self.net.test_build = false;
            let f = self.forward();
            let at = (self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 3.0).floor().as_ivec3();
            let gold = self.reg.id_or_air("gold_block");
            for y in 0..4 {
                self.world.set_block(at + IVec3::Y * y, gold);
            }
            self.send_chat("Oi! Construí uma coluna de ouro.");
        }
        // Our block changes go to the host. (In the host's dimension only the player's own
        // edits happen here; elsewhere this machine runs that world and sends all of it.)
        let my_dim = self.dim.as_u8();
        let log = self.world.net_log.as_mut().map(std::mem::take).unwrap_or_default();
        if !log.is_empty() {
            let b = self.block_changes(my_dim, &log);
            self.guest_ref().guest.send(&ToHost::Blocks(b));
        }
        // Changes that arrived before their chunk did.
        let ready: Vec<(u8, ChunkPos)> = self.guest_ref().pending.keys().copied().filter(|(d, p)| *d == my_dim && self.world.chunk(*p).is_some()).collect();
        for k in ready {
            let list = self.guest().pending.remove(&k).unwrap_or_default();
            let b = self.block_changes(k.0, &list);
            self.apply_remote_blocks(&b);
        }
        // Our hits on the host's mobs.
        if self.guest_ref().host_mobs {
            let me = self.body.center();
            let mut hits = Vec::new();
            for m in &self.mobs.list {
                if let Some(&h) = self.guest_ref().mob_health.get(&m.net_id) {
                    if m.health < h - 0.01 {
                        hits.push((m.net_id, h - m.health));
                    }
                }
            }
            for (id, amount) in hits {
                let now = self.mobs.list.iter().find(|m| m.net_id == id).map(|m| m.health).unwrap_or(0.0);
                self.guest().mob_health.insert(id, now);
                self.guest_ref().guest.send(&ToHost::HurtMob { mob: id, amount, from: me.to_array() });
            }
            // Smooth mobs towards where the host has them.
            let k = 1.0 - (-dt * 12.0).exp();
            let Mode::Guest(g) = &self.net.mode else { return };
            for m in self.mobs.list.iter_mut() {
                if let Some(&(p, yaw)) = g.mob_target.get(&m.net_id) {
                    if (p - m.body.pos).length() > 6.0 {
                        m.body.pos = p;
                    } else {
                        m.body.pos += (p - m.body.pos) * k;
                    }
                    let mut d = yaw - m.yaw;
                    d = (d + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    m.yaw += d * k;
                    m.hurt_age += dt;
                    if !m.alive() {
                        m.dead_age += dt;
                    }
                }
            }
        }
        let g = self.guest();
        g.send_t += dt;
        g.save_t += dt;
        g.cont_t += dt;
        if g.send_t >= SEND_EVERY {
            g.send_t = 0.0;
            let s = self.my_state();
            self.guest_ref().guest.send(&ToHost::State(s));
        }
        if self.guest_ref().save_t >= 20.0 {
            self.guest().save_t = 0.0;
            let data = self.player_save();
            self.guest_ref().guest.send(&ToHost::SavePlayer(data));
        }
        if self.guest_ref().cont_t >= 0.5 {
            self.guest().cont_t = 0.0;
            // Only the container we have open: the host runs the rest (furnaces burning,
            // hoppers moving things) and tells us.
            let open = match self.surv.open {
                Open::Chest(p) | Open::Furnace(p) if self.screen == Screen::Inventory => Some(p),
                _ => None,
            };
            let near: Vec<(IVec3, u64, String)> = self.containers_near(self.body.pos, 12.0).into_iter().filter(|(p, _, _)| Some(*p) == open).collect();
            let mut send = Vec::new();
            let g = self.guest();
            for (p, h, t) in near {
                if g.containers.get(&p) != Some(&h) {
                    g.containers.insert(p, h);
                    send.push(ToHost::Container { dim: my_dim, pos: p.to_array(), data: Some(t) });
                }
            }
            for m in send {
                self.guest_ref().guest.send(&m);
            }
        }
    }

    fn guest_message(&mut self, m: ToGuest) {
        match m {
            ToGuest::Welcome(w) => self.guest_open(w),
            ToGuest::Plugins(files) => {
                self.plugins_from_host(&files);
            }
            ToGuest::Bye(_) => {}
            ToGuest::Chunk { dim, pos, data } => {
                if dim != self.dim.as_u8() || !self.world_open {
                    return;
                }
                let p = ChunkPos::new(pos[0], pos[1]);
                if let Some(c) = decode_chunk(&self.reg, p, &data) {
                    self.world.replace_chunk(c);
                    self.guest().pending.remove(&(dim, p));
                }
            }
            ToGuest::Blocks(b) => {
                if b.dim != self.dim.as_u8() {
                    return;
                }
                let missed = self.apply_remote_blocks(&b);
                for (p, id) in missed {
                    let cp = split_xz(p.x, p.z).0;
                    self.guest().pending.entry((b.dim, cp)).or_default().push((p, id));
                }
            }
            ToGuest::Players(list) => {
                let mut seen = HashSet::new();
                let my_dim = self.dim.as_u8();
                let mut host_here = false;
                for p in list {
                    seen.insert(p.id);
                    if p.id == HOST_ID {
                        host_here = p.state.dim == my_dim;
                        self.guest().host_name = p.name.clone();
                    }
                    match self.net.remotes.get_mut(&p.id) {
                        Some(r) => {
                            r.state = p.state;
                            r.name = p.name;
                        }
                        None => {
                            self.net.remotes.insert(p.id, Remote::new(p.name, p.state));
                        }
                    }
                }
                self.net.remotes.retain(|id, _| seen.contains(id));
                let was = self.guest_ref().host_mobs;
                if was != host_here {
                    self.guest().host_mobs = host_here;
                    if host_here {
                        // The host takes over the mobs here; ours go.
                        self.mobs.list.clear();
                        self.mobs.arrows.clear();
                    } else {
                        // We run this dimension now; the host's mobs stay as they were.
                        self.guest().mob_target.clear();
                    }
                }
            }
            ToGuest::PlayerLeft(id) => {
                self.net.remotes.remove(&id);
            }
            ToGuest::Mobs { dim, list } => {
                if dim != self.dim.as_u8() || !self.guest_ref().host_mobs {
                    return;
                }
                let ids: HashSet<u32> = list.iter().map(|m| m.id).collect();
                self.mobs.list.retain(|m| ids.contains(&m.net_id));
                for s in list {
                    let def = s.def as usize;
                    if def >= self.mobs.reg.defs.len() {
                        continue;
                    }
                    let pos = Vec3::from(s.pos);
                    let i = match self.mobs.list.iter().position(|m| m.net_id == s.id) {
                        Some(i) => i,
                        None => {
                            let mut m = voxel::mob::Mob::new(def, &self.mobs.reg.defs[def], pos, s.id as u64);
                            m.net_id = s.id;
                            m.yaw = s.yaw;
                            self.mobs.list.push(m);
                            self.mobs.list.len() - 1
                        }
                    };
                    let m = &mut self.mobs.list[i];
                    m.body.vel = Vec3::from(s.vel);
                    m.head_yaw = s.head_yaw;
                    m.head_pitch = s.head_pitch;
                    m.health = s.health;
                    m.max_health = s.max_health;
                    m.hurt_age = s.hurt_age;
                    m.walk = s.walk;
                    m.walk_speed = s.walk_speed;
                    m.dead_age = s.dead_age;
                    m.phase = s.phase;
                    m.sleeping = s.sleeping;
                    m.aggro = s.aggro;
                    m.on_fire = s.on_fire;
                    let g = self.guest();
                    g.mob_health.insert(s.id, s.health);
                    g.mob_target.insert(s.id, (pos, s.yaw));
                }
                let g = self.guest();
                g.mob_health.retain(|id, _| ids.contains(id));
                g.mob_target.retain(|id, _| ids.contains(id));
            }
            ToGuest::Sky { time, raining, thunder } => {
                self.time_of_day = time;
                self.weather.state.raining = raining;
                self.weather.state.thunder = thunder;
                // The host decides when it changes.
                self.weather.state.timer = 600.0;
            }
            ToGuest::Hurt { amount, from } => self.damage(amount, from.map(Vec3::from)),
            ToGuest::Explosion { pos, power } => {
                let pos = Vec3::from(pos);
                let reach = power * 2.0;
                let d = (self.body.center() - pos).length();
                if d < reach {
                    let k = 1.0 - d / reach;
                    self.damage((k * power * 7.0).round(), Some(pos));
                    self.body.knockback(self.body.center() - pos, k * 12.0);
                }
                if d < 64.0 {
                    self.sfx(Sfx::Explode);
                }
            }
            ToGuest::Killed { mob, pos } => {
                if self.survival {
                    self.surv_mob_deaths(&[(mob as usize, Vec3::from(pos))]);
                }
            }
            ToGuest::Container { dim, pos, data } => {
                if dim != self.dim.as_u8() {
                    return;
                }
                let p = IVec3::from(pos);
                self.apply_container(p, &data);
                let h = data.as_deref().map(hash_text).unwrap_or(0);
                self.guest().containers.insert(p, h);
            }
            ToGuest::PlayerHit { amount, from, by } => self.pvp_hurt_me(amount, Vec3::from(from), &by),
            ToGuest::Board(b) => self.net.board = b,
            ToGuest::Rules(t) => {
                if let Ok(r) = ron::from_str(&t) {
                    self.net.rules = r;
                }
            }
            ToGuest::RunAs(line) => {
                let r = self.run_command(&line, true);
                if !r.is_empty() {
                    self.net_say(r, YELLOW);
                }
            }
            ToGuest::Op(op) => {
                self.guest().op = op;
                let t = if op { "Agora pode usar comandos (/ajuda)" } else { "Comandos desligados pelo anfitrião" };
                self.net_say(t, YELLOW);
            }
            ToGuest::Chat { from, text } => {
                if from.is_empty() {
                    self.net_say(text, YELLOW);
                } else {
                    self.net_say(format!("<{from}> {text}"), WHITE);
                }
            }
        }
    }

    /// The host said welcome: load its world (our copy grows from the same seed).
    fn guest_open(&mut self, w: Welcome) {
        let dir = level_path_hint(&self.base, GUEST_WORLD);
        let _ = std::fs::create_dir_all(self.base.join("saves"));
        // Whatever we kept from an earlier visit may be stale: the host sends what changed.
        let _ = std::fs::remove_dir_all(&dir);
        self.settings.spread_override = Some(w.spread);
        self.settings.difficulty = w.difficulty;
        self.mobs.peaceful = w.difficulty == 0;
        self.open_world(GUEST_WORLD, Some(w.seed), Some(w.survival));
        self.time_of_day = w.time;
        self.weather.state.raining = w.raining;
        self.weather.state.thunder = w.thunder;
        match &w.player {
            Some(p) => self.apply_player_save(p),
            None => {
                // First visit: beside the spawn (not inside whoever stands on it), facing it.
                let k = 3 + (w.id % 4) as i32;
                self.spawn_xz.0 += k;
                self.yaw = std::f32::consts::FRAC_PI_2;
            }
        }
        self.world.net_log = Some(Vec::new());
        self.world_open = true;
        self.title_world = GUEST_WORLD.to_string();
        self.loaded_frames = 0;
        self.set_screen(Screen::Loading);
        let g = self.guest();
        g.joined = true;
        g.op = w.op;
        g.my_id = w.id;
        g.host_name = w.host_name.clone();
        self.net.status.clear();
        self.net_say(format!("Bem-vindo ao mundo \"{}\" de {}", w.world, w.host_name), YELLOW);
        self.toast(format!("Mundo de {}", w.host_name), 4.0);
    }

    fn guest_left(&mut self, why: &str) {
        let joined = matches!(&self.net.mode, Mode::Guest(g) if g.joined);
        let addr = matches!(&self.net.mode, Mode::Guest(_)).then(|| self.guest_ref().addr.clone()).unwrap_or_default();
        if let Mode::Guest(g) = &self.net.mode {
            self.settings.difficulty = g.own_difficulty;
            self.mobs.peaceful = g.own_difficulty == 0;
        }
        self.net.mode = Mode::Off;
        self.net.remotes.clear();
        self.settings.spread_override = None;
        self.plugins_leave_host();
        self.world.net_log = None;
        let why = if why.is_empty() { "a conexão caiu".to_string() } else { why.to_string() };
        if joined && self.world_open {
            self.quit_to_title();
        }
        self.net.status = if joined { format!("Desconectado de {addr}: {why}") } else { format!("Não deu para entrar em {addr}: {why}") };
        log::info!("multiplayer: {}", self.net.status);
        self.set_screen(Screen::Multiplayer);
    }

    /// Some(may use commands) on a guest, None otherwise.
    pub(crate) fn net_guest_op(&self) -> Option<bool> {
        match &self.net.mode {
            Mode::Guest(g) => Some(g.op),
            _ => None,
        }
    }

    /// Leave or close multiplayer (quitting to the title, closing the game).
    pub(crate) fn net_stop(&mut self) {
        match std::mem::take(&mut self.net.mode) {
            Mode::Off => return,
            Mode::Host(h) => {
                let bye = net::encode(&ToGuest::Bye("o anfitrião fechou o mundo".into()));
                for id in h.guests.keys() {
                    h.host.send_raw(*id, bye.clone());
                }
                if h.external.is_some() {
                    net::upnp::close(h.host.port);
                }
                // Keep what guests changed in other dimensions.
                self.net.mode = Mode::Host(h);
                self.net_save();
                if let Mode::Host(h) = std::mem::take(&mut self.net.mode) {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    drop(h);
                }
                self.mobs.others.clear();
                self.world.extra_centers.clear();
            }
            Mode::Guest(g) => {
                self.settings.difficulty = g.own_difficulty;
                self.mobs.peaceful = g.own_difficulty == 0;
                if g.joined && self.world_open {
                    g.guest.send(&ToHost::SavePlayer(self.player_save()));
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                self.settings.spread_override = None;
                self.plugins_leave_host();
            }
        }
        self.world.net_log = None;
        self.net.remotes.clear();
    }

    // ---- drawing --------------------------------------------------------------------------

    /// Other players' figures (with armour) and what they hold.
    pub(crate) fn net_instances(&mut self, block: IVec3) -> (Vec<(usize, Bones)>, Vec<GpuItem>) {
        let mut models = Vec::new();
        let mut held = Vec::new();
        let Some(pd) = self.mobs.reg.id("player") else { return (models, held) };
        if self.net.remotes.is_empty() || !self.world_open {
            return (models, held);
        }
        let my_dim = self.dim.as_u8();
        let dt = self.frame_times.last().copied().unwrap_or(0.016).min(0.1);
        let ids: Vec<u32> = self.net.remotes.keys().copied().collect();
        for id in ids {
            let (pose, swing, armor, item) = {
                let r = &self.net.remotes[&id];
                let s = &r.state;
                if s.dim != my_dim || s.has(flags::HIDDEN) || (r.shown - self.body.pos).length() > 160.0 {
                    continue;
                }
                let pose = player_model::Pose {
                    pos: r.shown,
                    vel: Vec3::from(s.vel),
                    look_yaw: r.shown_yaw,
                    look_pitch: s.pitch,
                    on_ground: s.has(flags::ON_GROUND),
                    in_water: s.has(flags::IN_WATER),
                    head_in_water: s.has(flags::HEAD_IN_WATER),
                    flying: s.has(flags::FLYING),
                    sneaking: s.sneaking,
                    crawling: if s.has(flags::DEAD) { 1.0 } else { s.crawling },
                    sprinting: s.has(flags::SPRINTING),
                    burning: s.has(flags::BURNING),
                    hurt: s.has(flags::HURT),
                    sitting: s.has(flags::SITTING),
                    holding: !s.held.is_empty(),
                    torch: crate::torch::index(&s.held).is_some(),
                    ..Default::default()
                };
                (pose, s.swing, s.armor.clone(), s.held.clone())
            };
            let l = self.world.light((pose.pos + Vec3::Y).floor().as_ivec3());
            let (sky, blk) = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let def = &self.mobs.reg.defs[pd];
            let r = self.net.remotes.get_mut(&id).unwrap();
            r.anim.update(&pose, dt);
            r.anim.swing = swing;
            let mut bones = r.anim.bones(def, &pose, block, sky, blk);
            // An instrument in their hands: held with both, in 3D.
            if let Some(i) = crate::instruments::index(&item) {
                if let (Some(ib), Some(mesh)) = (crate::instruments::pose(def, &mut bones, i, None), self.entities.instrument_mesh(i)) {
                    models.push((mesh, ib));
                }
            }
            for (piece, name) in armor.iter().enumerate() {
                if !name.is_empty() {
                    if let Some(m) = self.entities.armor_mesh(name, piece) {
                        models.push((m, bones));
                    }
                }
            }
            models.push((pd, bones));
            // A torch is a 3D model with its flame (torch.rs).
            if let Some((t, mesh)) = crate::torch::index(&item).and_then(|t| Some((t, self.entities.torch_mesh(t)?))) {
                let pv = def.parts[2].pivot;
                let (tb, _) = crate::torch::hold(t, bones.m[2], Vec3::new(pv.0 as f32, pv.1 as f32, pv.2 as f32), 1.0, 0.62, self.now, [sky, 1.0, 0.0, 0.0]);
                models.push((mesh, tb));
            }
            // What they hold, put in our hand for a moment to reuse the same drawing.
            if let Some(it) = self.items.id(&item) {
                let pv = def.parts[2].pivot;
                let sel = self.selected;
                let mine = self.surv.inv.slots[sel];
                self.surv.inv.slots[sel] = Some(ItemStack::new(it, 1));
                held.extend(self.held_item_third_person(bones.m[2], Vec3::new(pv.0 as f32, pv.1 as f32, pv.2 as f32)));
                self.surv.inv.slots[sel] = mine;
            }
        }
        (models, held)
    }

    /// Name tags, the chat and the player list (Tab).
    pub(crate) fn draw_net_overlay(&mut self, w: f32, h: f32) {
        let s = self.ui.scale;
        if self.world_open && self.screen != Screen::Loading {
            if let Some((vp, cam_block)) = self.ui_view_proj {
                let eye = self.body.eye();
                let my_dim = self.dim.as_u8();
                let tags: Vec<(Vec3, String, bool, [u8; 4])> = self
                    .net
                    .remotes
                    .values()
                    .filter(|r| r.state.dim == my_dim && !r.state.has(flags::HIDDEN))
                    .map(|r| (r.shown + Vec3::Y * (2.1 - r.state.sneaking * 0.3), r.name.clone(), r.state.sneaking > 0.5, self.team_color(&r.name)))
                    .collect();
                for (top, name, sneaking, color) in tags {
                    let dist = (top - eye).length();
                    if dist > 64.0 || (sneaking && dist > 8.0) {
                        continue;
                    }
                    let c = vp * (top - cam_block.as_vec3()).extend(1.0);
                    if c.w < 0.1 {
                        continue;
                    }
                    let (nx, ny) = (c.x / c.w, c.y / c.w);
                    if nx.abs() > 1.1 || ny.abs() > 1.1 {
                        continue;
                    }
                    let (x, y) = ((nx * 0.5 + 0.5) * w, (0.5 - ny * 0.5) * h);
                    let tw = self.ui.text_width(&name, s);
                    self.ui.rect(x - tw / 2.0 - 2.0 * s, y - s, tw + 4.0 * s, 10.0 * s, [0, 0, 0, 100]);
                    self.ui.text(x - tw / 2.0, y, &name, if sneaking { GRAY } else { color });
                }
            }
        }
        // Chat: recent lines fade out; all of them while typing.
        if self.world_open || self.net.typing.is_some() {
            let open = self.net.typing.is_some();
            let line_h = 10.0 * s;
            let mut y = h - 40.0 * s - line_h;
            if let Some(t) = &self.net.typing {
                let shown = format!("{t}{}", if (self.now * 2.0) as i32 % 2 == 0 { "_" } else { "" });
                self.ui.rect(2.0 * s, h - 14.0 * s, w - 4.0 * s, 12.0 * s, [0, 0, 0, 150]);
                self.ui.text(4.0 * s, h - 12.0 * s, &shown, WHITE);
            }
            let now = self.now;
            let lines: Vec<(String, [u8; 4], f32)> =
                self.net.chat.iter().rev().take(if open { 20 } else { 8 }).map(|l| (l.text.clone(), l.color, now - l.at)).collect();
            for (text, color, age) in lines {
                if !open && age > 10.0 {
                    continue;
                }
                let a = if open { 1.0 } else { (10.0 - age).clamp(0.0, 1.0) };
                let tw = self.ui.text_width(&text, s);
                self.ui.rect(2.0 * s, y - s, tw + 4.0 * s, line_h, [0, 0, 0, (110.0 * a) as u8]);
                self.ui.text(4.0 * s, y, &text, [color[0], color[1], color[2], (255.0 * a) as u8]);
                y -= line_h;
            }
        }
        // Tab: who is here.
        if self.world_open && self.net.active() && self.screen == Screen::Playing && self.keys.contains(&KeyCode::Tab) {
            let mut rows = vec![self.player_list_line()];
            if let Some(code) = self.net_invite_code() {
                rows.push(format!("Convite: {code}"));
            }
            if let Mode::Guest(g) = &self.net.mode {
                rows.push(format!("Ping: {} ms", g.guest.ping().unwrap_or(0)));
            }
            let bw = rows.iter().map(|r| self.ui.text_width(r, s)).fold(0.0, f32::max) + 10.0 * s;
            self.ui.rect(w / 2.0 - bw / 2.0, 8.0 * s, bw, rows.len() as f32 * 10.0 * s + 6.0 * s, [0, 0, 0, 150]);
            for (i, r) in rows.iter().enumerate() {
                self.ui.text_centered(w / 2.0, 11.0 * s + i as f32 * 10.0 * s, r, WHITE);
            }
        }
    }

    // ---- input ----------------------------------------------------------------------------

    /// Keys for the chat and the multiplayer screen's text fields. True = used up.
    pub(crate) fn net_key(&mut self, code: KeyCode, pressed: bool) -> bool {
        if !pressed {
            return false;
        }
        if let Some(t) = self.net.typing.as_mut() {
            match code {
                KeyCode::Escape => {
                    self.net.typing = None;
                    self.set_grab(true);
                }
                KeyCode::Backspace => {
                    t.pop();
                }
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let text = self.net.typing.take().unwrap_or_default();
                    self.set_grab(true);
                    self.send_chat(text.trim());
                }
                _ => {}
            }
            self.keys.clear();
            return true;
        }
        if self.screen == Screen::Playing
            && self.world_open
            && matches!(code, KeyCode::KeyT | KeyCode::Enter | KeyCode::Slash)
            && !self.keys.contains(&KeyCode::F3)
        {
            self.net.typing = Some(if code == KeyCode::Slash { "/".into() } else { String::new() });
            self.keys.clear();
            self.set_grab(false);
            // The key's own letter must not land in the box.
            self.net.field = Field::None;
            self.net.skip_text = true;
            return true;
        }
        let ctrl = self.keys.contains(&KeyCode::ControlLeft) || self.keys.contains(&KeyCode::ControlRight);
        let friends = self.screen == Screen::Friends;
        if (self.screen == Screen::Multiplayer || friends) && self.net.field != Field::None {
            match code {
                KeyCode::KeyV if ctrl => {
                    self.paste_into_field();
                }
                KeyCode::Backspace => {
                    self.field_text(self.net.field).0.pop();
                }
                KeyCode::Tab => {
                    self.net.field = match self.net.field {
                        Field::Name => Field::Addr,
                        Field::Addr => Field::Password,
                        Field::Password => Field::Name,
                        Field::HostPassword if !self.net.is_host() => Field::Port,
                        Field::Port if self.net.cfg.reach != Reach::Lan => Field::Relay,
                        _ => Field::HostPassword,
                    }
                }
                KeyCode::Enter | KeyCode::NumpadEnter if !friends => {
                    let a = self.net.cfg.last_addr.clone();
                    self.net.field = Field::None;
                    self.join(&a);
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Escape => {
                    self.net.field = Field::None;
                    self.net.save_config(&self.base);
                }
                _ => {}
            }
            return true;
        }
        false
    }

    /// Typed text for the chat or a field. True = used up.
    pub(crate) fn net_text(&mut self, text: &str) -> bool {
        if std::mem::take(&mut self.net.skip_text) {
            return self.net.typing.is_some();
        }
        if let Some(t) = self.net.typing.as_mut() {
            for c in text.chars() {
                if !c.is_control() && t.chars().count() < 200 {
                    t.push(c);
                }
            }
            return true;
        }
        if matches!(self.screen, Screen::Multiplayer | Screen::Friends) && self.net.field != Field::None {
            let digits = self.net.field == Field::Port;
            let (f, max) = self.field_text(self.net.field);
            for c in text.chars() {
                if !c.is_control() && f.chars().count() < max && (!digits || c.is_ascii_digit()) {
                    f.push(c);
                }
            }
            return true;
        }
        false
    }

    fn send_chat(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if text.starts_with('/') {
            if self.local_command(text) {
                return;
            }
            let cmd = text.split_whitespace().next().unwrap_or("").to_lowercase();
            let world = crate::commands::scope(&cmd) == crate::commands::Scope::World;
            let (guest, cheats) = match &self.net.mode {
                Mode::Guest(g) => (true, g.op),
                _ => (false, true),
            };
            if guest && world {
                // The world's clock and sky live on the host.
                if let Mode::Guest(g) = &self.net.mode {
                    g.guest.send(&ToHost::Chat(text.to_string()));
                }
                return;
            }
            // Alone or hosting, a command may name another player or use @p, @a...
            let r = if cheats && !guest { self.run_command_at(text, self.body.pos) } else { self.run_command(text, cheats) };
            if !r.is_empty() {
                self.net_say(r, YELLOW);
            }
            if world && self.net.is_host() {
                self.host().sky_t = 1.0;
            }
            return;
        }
        match &self.net.mode {
            Mode::Guest(g) => g.guest.send(&ToHost::Chat(text.to_string())),
            Mode::Host(_) => {
                let name = self.net.cfg.name.clone();
                self.host_broadcast(&ToGuest::Chat { from: name.clone(), text: text.to_string() }, None);
                self.net_say(format!("<{name}> {text}"), WHITE);
            }
            Mode::Off => {
                let name = self.net.cfg.name.clone();
                self.net_say(format!("<{name}> {text}"), WHITE);
            }
        }
    }

    /// Commands handled on this machine. True = handled.
    fn local_command(&mut self, text: &str) -> bool {
        let mut parts = text.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let arg = parts.next().unwrap_or("").to_string();
        match cmd {
            // "/tp Ana": go to Ana. "/tp Ana 0 80 0" and "/tp @p ..." are for run_command_at.
            "/tp" if !arg.is_empty() && arg.parse::<f32>().is_err() && !arg.starts_with('@') && parts.next().is_none() => {
                let target = self.net.remotes.values().find(|r| r.name.eq_ignore_ascii_case(&arg)).map(|r| (r.state.dim, Vec3::from(r.state.pos)));
                match target {
                    Some((d, p)) if d == self.dim.as_u8() => {
                        self.body.pos = p;
                        self.body.vel = Vec3::ZERO;
                        self.fall_peak = p.y;
                        self.net_say(format!("Teleportado até {arg}"), YELLOW);
                    }
                    Some(_) => self.net_say(format!("{arg} está em outra dimensão"), YELLOW),
                    None => self.net_say(format!("Ninguém chamado \"{arg}\""), YELLOW),
                }
                true
            }
            // "/ponte 203.0.113.5" takes guests through that relay from the next time the
            // world opens; "/ponte nada" stops.
            "/ponte" | "/relay" => {
                if arg.is_empty() {
                    let now = if self.net.cfg.relay.is_empty() { "nenhuma".to_string() } else { self.net.cfg.relay.clone() };
                    self.net_say(format!("Ponte: {now} (use /ponte <endereço> ou /ponte nada)"), YELLOW);
                    return true;
                }
                self.net.cfg.relay = if matches!(arg.as_str(), "nada" | "off" | "none") { String::new() } else { arg.clone() };
                self.net.save_config(&self.base);
                if self.net.cfg.relay.is_empty() {
                    self.net_say("Ponte desligada".to_string(), YELLOW);
                } else {
                    if let Mode::Host(h) = &mut self.net.mode {
                        h.host.use_relay(&arg);
                        h.relay_waiting = true;
                    }
                    self.net_say(format!("Ponte: {arg}"), YELLOW);
                }
                true
            }
            "/lista" | "/list" => {
                let l = self.player_list_line();
                self.net_say(l, YELLOW);
                true
            }
            "/kick" | "/ban" | "/desbanir" if self.net.is_host() => {
                let who = self.host_ref().guests.iter().find(|(_, g)| g.welcomed && g.name.eq_ignore_ascii_case(&arg)).map(|(id, _)| *id);
                let file = level_path_hint(&self.base, &self.world_name).join("bans.txt");
                if cmd == "/desbanir" {
                    let low = arg.to_lowercase();
                    self.host().bans.retain(|b| *b != low);
                    let _ = std::fs::write(&file, self.host_ref().bans.join("\n"));
                    self.net_say(format!("{arg} pode voltar"), YELLOW);
                    return true;
                }
                if cmd == "/ban" {
                    self.host().bans.push(arg.to_lowercase());
                    let _ = std::fs::write(&file, self.host_ref().bans.join("\n"));
                }
                match who {
                    Some(id) => {
                        self.host_ref().host.kick(id, if cmd == "/ban" { "banido pelo anfitrião" } else { "expulso pelo anfitrião" });
                        self.net_say(format!("{arg} saiu ({})", if cmd == "/ban" { "banido" } else { "expulso" }), YELLOW);
                    }
                    None if cmd == "/ban" => self.net_say(format!("{arg} não poderá entrar"), YELLOW),
                    None => self.net_say(format!("Ninguém chamado \"{arg}\""), YELLOW),
                }
                true
            }
            "/op" | "/deop" if self.net.is_host() => {
                let low = arg.to_lowercase();
                if low.is_empty() {
                    self.net_say("Use: /op <nome>", YELLOW);
                    return true;
                }
                let give = cmd == "/op";
                let h = self.host();
                h.ops.retain(|o| *o != low);
                if give {
                    h.ops.push(low.clone());
                }
                let who = h.guests.iter().find(|(_, g)| g.welcomed && g.name.eq_ignore_ascii_case(&low)).map(|(id, _)| *id);
                if let Some(id) = who {
                    h.host.send(id, &ToGuest::Op(give));
                }
                let file = level_path_hint(&self.base, &self.world_name).join("ops.txt");
                let _ = std::fs::write(&file, self.host_ref().ops.join("\n"));
                self.net_say(format!("{arg} {} usar comandos", if give { "agora pode" } else { "não pode mais" }), YELLOW);
                true
            }
            "/ajuda" | "/help" => {
                self.net_say(crate::commands::HELP, YELLOW);
                if self.net.is_host() {
                    self.net_say("Anfitrião: /op <nome>, /deop <nome>, /kick <nome>, /ban <nome>, /desbanir <nome>", YELLOW);
                }
                true
            }
            _ => false,
        }
    }

    // ---- menus ----------------------------------------------------------------------------

    /// Pause menu button text.
    pub(crate) fn net_pause_label(&self) -> String {
        match &self.net.mode {
            Mode::Off => "Abrir para amigos...".into(),
            Mode::Host(_) => format!("Amigos: {} de {}...", self.net_player_count(), self.net.cfg.max_players),
            Mode::Guest(g) => format!("Mundo de {} ({} ms)", g.host_name, g.guest.ping().unwrap_or(0)),
        }
    }

    pub(crate) fn net_pause_button(&mut self) {
        match &self.net.mode {
            Mode::Guest(_) => {
                let l = self.player_list_line();
                self.net_say(l, YELLOW);
            }
            _ if self.world_name == GUEST_WORLD => {}
            _ => {
                self.net.load_config(&self.base.clone());
                self.net.port_text = self.net.cfg.port.to_string();
                self.net.field = Field::None;
                self.set_screen(Screen::Friends);
            }
        }
    }

    /// Players in the world we host, ourselves included.
    fn net_player_count(&self) -> usize {
        match &self.net.mode {
            Mode::Host(h) => h.guests.values().filter(|g| g.welcomed).count() + usize::from(!self.net.server),
            _ => 1,
        }
    }

    /// Put text on the system clipboard. False when there is none (a bare server, no display).
    fn clip_set(&mut self, text: &str) -> bool {
        if self.net.clip.is_none() {
            self.net.clip = arboard::Clipboard::new().map_err(|e| log::warn!("clipboard: {e}")).ok();
        }
        self.net.clip.as_mut().is_some_and(|c| c.set_text(text.to_string()).map_err(|e| log::warn!("clipboard: {e}")).is_ok())
    }

    /// The clipboard's text, made fit for a one-line field (first line, no control characters).
    fn clip_get(&mut self) -> Option<String> {
        if self.net.clip.is_none() {
            self.net.clip = arboard::Clipboard::new().map_err(|e| log::warn!("clipboard: {e}")).ok();
        }
        let t = self.net.clip.as_mut()?.get_text().ok()?;
        let line = t.lines().map(str::trim).find(|l| !l.is_empty())?;
        // "Convite: K3F9Q-2ZB7M" pasted whole from a chat message still works.
        let line = line.rsplit_once(": ").map_or(line, |(_, code)| code).trim();
        Some(line.chars().filter(|c| !c.is_control()).collect())
    }

    /// Paste into the focused field. True = there was a field and something to paste.
    fn paste_into_field(&mut self) -> bool {
        if self.net.field == Field::None {
            return false;
        }
        let Some(t) = self.clip_get() else {
            self.toast("Nada para colar", 2.0);
            return true;
        };
        let (f, max) = self.field_text(self.net.field);
        f.clear();
        f.extend(t.chars().take(max));
        true
    }

    /// The text behind a field and its longest length.
    fn field_text(&mut self, field: Field) -> (&mut String, usize) {
        let c = &mut self.net.cfg;
        match field {
            Field::Name => (&mut c.name, 16),
            Field::Addr => (&mut c.last_addr, 64),
            Field::Password => (&mut c.password, 32),
            Field::HostPassword => (&mut c.host_password, 32),
            Field::Relay => (&mut c.relay, 64),
            Field::Port => (&mut self.net.port_text, 5),
            Field::None => unreachable!("no field"),
        }
    }

    fn friends_note(&mut self, text: impl Into<String>) {
        self.net.friends_note = (text.into(), self.now);
    }

    /// "Abrir para amigos": how the world opens (before and while open), the invite to copy,
    /// how people can reach it, and who is in (with a button to send them away).
    pub(crate) fn draw_friends_menu(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        // A busy screen: one interface size smaller when the window is short.
        let full = self.ui.scale;
        self.ui.scale = (h / 300.0).floor().clamp(1.0, full);
        self.draw_friends_inner(w, h, mouse);
        self.ui.scale = full;
    }

    fn draw_friends_inner(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        let hosting = self.net.is_host();
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 190]);
        let pw = (440.0 * s).min(w - 16.0 * s);
        let x0 = (w - pw) / 2.0;
        let gap = 10.0 * s;
        let cw = (pw - gap) / 2.0;
        let (lx, rx) = (x0, x0 + cw + gap);
        let bh = 18.0 * s;
        let top = 8.0 * s;
        self.ui.text_centered(w / 2.0, top, if hosting { "Mundo aberto para amigos" } else { "Abrir para amigos" }, WHITE);
        if self.clicked {
            self.net.field = Field::None;
        }
        let clicked = self.clicked;
        let click = |hit: bool| hit && clicked;

        // ---- left: settings ----
        let mut y = top + 18.0 * s;
        self.ui.text(lx, y, "Configurações", YELLOW);
        y += 12.0 * s;
        let cfg = self.net.cfg.clone();
        let rows = [
            format!("Máximo de jogadores: {}", cfg.max_players),
            format!("Modo dos amigos: {}", GUEST_MODES[cfg.guest_mode.min(2) as usize]),
            format!("Amigos usam comandos: {}", if cfg.guest_cheats { "Sim" } else { "Não" }),
            format!("PvP: {}", if self.net.rules.pvp { "Sim" } else { "Não" }),
            format!("Alcance: {}", cfg.reach.label()),
        ];
        let mut pressed = None;
        for (i, l) in rows.iter().enumerate() {
            // How far the world reaches is chosen before it opens.
            let locked = hosting && i == 4;
            let hit = self.ui.button(lx, y, cw, bh, l, mouse);
            if locked {
                self.ui.rect(lx, y, cw, bh, [0, 0, 0, 110]);
            } else if click(hit) {
                pressed = Some(i);
            }
            y += bh + 3.0 * s;
        }
        let back = self.keys.contains(&KeyCode::ShiftLeft) || self.keys.contains(&KeyCode::ShiftRight);
        match pressed {
            Some(0) => {
                let i = MAX_PLAYERS.iter().position(|&m| m >= cfg.max_players).unwrap_or(0);
                let n = MAX_PLAYERS.len();
                self.net.cfg.max_players = MAX_PLAYERS[if back { (i + n - 1) % n } else { (i + 1) % n }];
            }
            Some(1) => self.net.cfg.guest_mode = (cfg.guest_mode + if back { 2 } else { 1 }) % 3,
            Some(2) => {
                self.net.cfg.guest_cheats = !cfg.guest_cheats;
                // Guests already in get (or lose) the commands right away; ops.txt keeps its own.
                if let Mode::Host(hs) = &self.net.mode {
                    for (id, g) in hs.guests.iter().filter(|(_, g)| g.welcomed) {
                        let op = self.net.cfg.guest_cheats || hs.ops.contains(&g.name.to_lowercase());
                        hs.host.send(*id, &ToGuest::Op(op));
                    }
                }
            }
            Some(3) => {
                self.net.rules.pvp = !self.net.rules.pvp;
                self.save_rules();
                let t = format!("PvP {}", if self.net.rules.pvp { "ligado" } else { "desligado" });
                if hosting {
                    self.net_announce(t);
                }
            }
            Some(_) => self.net.cfg.reach = cfg.reach.next(),
            None => {}
        }
        if pressed.is_some() {
            self.sfx(Sfx::Click);
            self.net.save_config(&self.base);
        }
        y += 9.0 * s;
        let pass = self.net.cfg.host_password.clone();
        self.text_field(lx, y, cw, "Senha (vazio = sem senha)", &pass, Field::HostPassword, true, mouse);
        y += 28.0 * s;
        if hosting {
            let port = self.host_ref().host.port;
            self.ui.text(lx, y - 9.0 * s, "Porta (UDP)", GRAY);
            self.ui.text(lx + 4.0 * s, y + 4.0 * s, &port.to_string(), WHITE);
        } else {
            let pt = self.net.port_text.clone();
            self.text_field(lx, y, cw * 0.4, "Porta (UDP)", &pt, Field::Port, false, mouse);
        }
        if self.net.cfg.reach != Reach::Lan {
            let relay = self.net.cfg.relay.clone();
            let label = if self.net.cfg.reach == Reach::Relay { "Endereço da ponte" } else { "Ponte (opcional)" };
            let fx = lx + cw * 0.4 + 6.0 * s;
            if hosting {
                self.ui.text(fx, y - 9.0 * s, label, GRAY);
                self.ui.text(fx + 4.0 * s, y + 4.0 * s, if relay.is_empty() { "nenhuma" } else { &relay }, WHITE);
            } else {
                self.text_field(fx, y, cw - cw * 0.4 - 6.0 * s, label, &relay, Field::Relay, false, mouse);
            }
        }
        y += 22.0 * s;
        let hint = match (hosting, self.net.cfg.reach) {
            (true, _) => "Senha, jogadores, modo e comandos valem para quem entrar agora.",
            (false, Reach::Lan) => "Só quem está no mesmo Wi-Fi ou cabo consegue entrar.",
            (false, Reach::Internet) => "O jogo pede ao roteador para abrir a porta (UPnP). Se ele recusar, amigos da mesma rede ainda entram.",
            (false, Reach::Relay) => "A ponte é um computador com porta aberta rodando mine-clone --relay.",
        };
        for line in wrap_text(&self.ui, hint, cw, s) {
            self.ui.text(lx, y, &line, GRAY);
            y += 10.0 * s;
        }

        // ---- right: connection, invite, players ----
        let mut y = top + 18.0 * s;
        self.ui.text(rx, y, "Convite", YELLOW);
        y += 12.0 * s;
        if !hosting {
            for line in wrap_text(&self.ui, "Abra o mundo para gerar o código do convite. O amigo cola o código em Multijogador > Entrar.", cw, s) {
                self.ui.text(rx, y, &line, GRAY);
                y += 10.0 * s;
            }
        } else {
            let code = self.net_invite_code();
            let hs = self.host_ref();
            let lan_code = hs.lan_ip.map(|ip| net::codes::encode(SocketAddrV4::new(ip, hs.host.port)));
            let lan_addr = hs.lan_ip.map(|ip| format!("{ip}:{}", hs.host.port)).unwrap_or_else(|| format!("porta {}", hs.host.port));
            let via = if hs.external.is_some() {
                "funciona pela internet"
            } else if hs.relay_code.is_some() {
                "pela ponte"
            } else {
                "só na mesma rede"
            };
            let (upnp_note, upnp_ok) = hs.upnp_note.clone();
            let relay_note = hs.relay_note.clone();
            let waiting = hs.upnp.is_some() || hs.relay_waiting;
            // The big code and its copy button.
            let code_txt = code.clone().unwrap_or_else(|| "...".into());
            let bs = (s * 1.5).floor().max(s);
            self.ui.rect(rx, y, cw, 24.0 * s, [0, 0, 0, 160]);
            self.ui.text_scaled(rx + 5.0 * s, y + 12.0 * s - 4.0 * bs, &code_txt, WHITE, bs);
            y += 26.0 * s;
            for line in wrap_text(&self.ui, &format!("({via}{})", if waiting { ", verificando..." } else { "" }), cw, s) {
                self.ui.text(rx, y, &line, GRAY);
                y += 10.0 * s;
            }
            y += 1.0 * s;
            let half = (cw - 4.0 * s) / 2.0;
            let mut copy = None;
            if click(self.ui.button(rx, y, half, bh, "Copiar código", mouse)) {
                copy = code.clone();
            }
            if click(self.ui.button(rx + half + 4.0 * s, y, half, bh, "Copiar da rede", mouse)) {
                copy = lan_code.clone();
            }
            if let Some(c) = copy {
                self.sfx(Sfx::Click);
                let ok = self.clip_set(&c);
                self.friends_note(if ok { format!("Copiado: {c}") } else { "Não deu para copiar: anote o código".into() });
            }
            y += bh + 6.0 * s;
            self.ui.text(rx, y, "Conexão", YELLOW);
            y += 12.0 * s;
            self.ui.text(rx, y, &format!("Mesma rede: {lan_addr}"), WHITE);
            y += 10.0 * s;
            let color = match upnp_ok {
                Some(true) => GREEN,
                Some(false) => RED,
                None => GRAY,
            };
            for line in wrap_text(&self.ui, &format!("Internet: {upnp_note}"), cw, s) {
                self.ui.text(rx, y, &line, color);
                y += 10.0 * s;
            }
            if !relay_note.is_empty() {
                for line in wrap_text(&self.ui, &format!("Ponte: {relay_note}"), cw, s) {
                    self.ui.text(rx, y, &line, GRAY);
                    y += 10.0 * s;
                }
            }
            y += 4.0 * s;
            // Who is in.
            let n = self.net_player_count();
            self.ui.text(rx, y, &format!("Jogadores ({n} de {})", self.net.cfg.max_players), YELLOW);
            y += 12.0 * s;
            if !self.net.server {
                // (No "ê": the pack's font lacks it.)
                self.ui.text(rx + 2.0 * s, y, &format!("{} (anfitrião)", self.net.cfg.name), WHITE);
                y += 11.0 * s;
            }
            let mut list: Vec<(ConnId, String, u32)> = {
                let hs = self.host_ref();
                hs.guests.iter().filter(|(_, g)| g.welcomed).map(|(id, g)| (*id, g.name.clone(), hs.host.ping(*id).unwrap_or(0))).collect()
            };
            list.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
            let kw = self.ui.text_width("Expulsar", s) + 8.0 * s;
            let bottom = h - 30.0 * s;
            let mut kick = None;
            for (id, name, ping) in &list {
                if y + 16.0 * s > bottom {
                    self.ui.text(rx + 2.0 * s, y, "...", GRAY);
                    break;
                }
                self.ui.text(rx + 2.0 * s, y + 4.0 * s, &format!("{name}  {ping} ms"), WHITE);
                if click(self.ui.button(rx + cw - kw * 2.0 - 3.0 * s, y, kw, 15.0 * s, "Expulsar", mouse)) {
                    kick = Some((*id, name.clone(), false));
                }
                if click(self.ui.button(rx + cw - kw, y, kw, 15.0 * s, "Banir", mouse)) {
                    kick = Some((*id, name.clone(), true));
                }
                y += 17.0 * s;
            }
            if list.is_empty() {
                self.ui.text(rx + 2.0 * s, y, "Ninguém entrou ainda", GRAY);
            }
            if let Some((id, name, ban)) = kick {
                self.sfx(Sfx::Click);
                if ban {
                    self.host().bans.push(name.to_lowercase());
                    let file = level_path_hint(&self.base, &self.world_name).join("bans.txt");
                    let _ = std::fs::write(&file, self.host_ref().bans.join("\n"));
                }
                self.host_ref().host.kick(id, if ban { "banido pelo anfitrião" } else { "expulso pelo anfitrião" });
                self.net_say(format!("{name} saiu ({})", if ban { "banido" } else { "expulso" }), YELLOW);
            }
        }

        // ---- bottom: what just happened, open/close, back ----
        let (note, at) = self.net.friends_note.clone();
        if !note.is_empty() && self.now - at < 4.0 {
            self.ui.text_centered(w / 2.0, h - 42.0 * s, &note, GREEN);
        }
        let by = h - 26.0 * s;
        let bw = (pw - 6.0 * s) / 2.0;
        let main = if hosting { "Fechar para amigos" } else { "Abrir mundo" };
        if click(self.ui.button(x0, by, bw, 20.0 * s, main, mouse)) {
            self.sfx(Sfx::Click);
            if hosting {
                self.net_stop();
                self.net_say("Mundo fechado para amigos", YELLOW);
            } else {
                self.net.cfg.port = self.net.port_text.trim().parse().ok().filter(|&p: &u16| p >= 1024).unwrap_or(net::DEFAULT_PORT);
                self.net.port_text = self.net.cfg.port.to_string();
                self.net.save_config(&self.base);
                self.start_hosting();
            }
        }
        if click(self.ui.button(x0 + bw + 6.0 * s, by, bw, 20.0 * s, "Voltar", mouse)) {
            self.sfx(Sfx::Click);
            self.net.save_config(&self.base);
            self.set_screen(Screen::Paused);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn text_field(&mut self, x: f32, y: f32, w: f32, label: &str, value: &str, field: Field, hidden: bool, mouse: (f32, f32)) {
        let s = self.ui.scale;
        let hh = 16.0 * s;
        self.ui.text(x, y - 9.0 * s, label, GRAY);
        let focus = self.net.field == field;
        self.ui.rect(x, y, w, hh, if focus { [230, 230, 230, 255] } else { [140, 140, 140, 255] });
        self.ui.rect(x + s, y + s, w - 2.0 * s, hh - 2.0 * s, [0, 0, 0, 255]);
        let shown = if hidden { "*".repeat(value.chars().count()) } else { value.to_string() };
        let caret = if focus && (self.now * 2.0) as i32 % 2 == 0 { "_" } else { "" };
        self.ui.text(x + 4.0 * s, y + 4.0 * s, &format!("{shown}{caret}"), WHITE);
        if self.clicked && mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + hh {
            self.net.field = field;
        }
    }

    pub(crate) fn draw_multiplayer_menu(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        if !self.net.loaded {
            self.net.load_config(&self.base.clone());
        }
        let bw = (300.0 * s).min(w - 20.0 * s);
        let x = (w - bw) / 2.0;
        let bh = 20.0 * s;
        let mut y = 10.0 * s;
        // A dark panel so the grey labels read over a bright sky.
        self.ui.rect(x - 8.0 * s, 4.0 * s, bw + 16.0 * s, h - 8.0 * s, [0, 0, 0, 120]);
        self.ui.text_centered(w / 2.0, y, "Multijogador", WHITE);
        y += 22.0 * s;
        if self.clicked {
            self.net.field = Field::None;
        }
        if self.net.picking_world {
            self.ui.text_centered(w / 2.0, y, "Qual mundo abrir para os amigos?", GRAY);
            y += 14.0 * s;
            let worlds: Vec<String> = self.worlds.iter().map(|w| w.name.clone()).filter(|n| !n.starts_with('.')).take(8).collect();
            if worlds.is_empty() {
                self.ui.text_centered(w / 2.0, y + 4.0 * s, "Nenhum mundo ainda: crie um em Um jogador.", GRAY);
                y += 20.0 * s;
            }
            let mut pick = None;
            for name in worlds {
                if self.ui.button(x, y, bw, bh, &name, mouse) && self.clicked {
                    pick = Some(name);
                }
                y += bh + 3.0 * s;
            }
            if self.ui.button(x, y + 6.0 * s, bw, bh, "Voltar", mouse) && self.clicked {
                self.sfx(Sfx::Click);
                self.net.picking_world = false;
            }
            if let Some(name) = pick {
                self.sfx(Sfx::Click);
                self.net.picking_world = false;
                self.net.save_config(&self.base);
                self.net.host_when_ready = true;
                self.play_world(&name);
            }
            return;
        }
        let name = self.net.cfg.name.clone();
        let addr = self.net.cfg.last_addr.clone();
        let pass = self.net.cfg.password.clone();
        let half = (bw - 6.0 * s) / 2.0;
        y += 10.0 * s;
        self.text_field(x, y, half, "Seu nome", &name, Field::Name, false, mouse);
        self.text_field(x + half + 6.0 * s, y, half, "Senha (opcional)", &pass, Field::Password, true, mouse);
        y += 30.0 * s;
        let jw = 60.0 * s;
        self.text_field(x, y, bw - jw * 2.0 - 10.0 * s, "Código do convite ou endereço (IP:porta)", &addr, Field::Addr, false, mouse);
        if self.ui.button(x + bw - jw * 2.0 - 4.0 * s, y - 2.0 * s, jw, bh, "Colar", mouse) && self.clicked {
            self.sfx(Sfx::Click);
            match self.clip_get() {
                Some(t) => {
                    self.net.cfg.last_addr = t.chars().take(64).collect();
                    self.net.status = "Código colado: clique em Entrar".into();
                }
                None => self.net.status = "Nada para colar (copie o código do convite primeiro)".into(),
            }
        }
        let addr = self.net.cfg.last_addr.clone();
        let connecting = matches!(&self.net.mode, Mode::Guest(g) if !g.joined);
        if self.ui.button(x + bw - jw, y - 2.0 * s, jw, bh, if connecting { "Cancelar" } else { "Entrar" }, mouse) && self.clicked {
            self.sfx(Sfx::Click);
            if connecting {
                self.net_stop();
                self.net.status = "Cancelado.".into();
            } else {
                self.join(&addr);
            }
        }
        y += 26.0 * s;
        if !self.net.status.is_empty() {
            let st = self.net.status.clone();
            self.ui.text_centered(w / 2.0, y, &st, YELLOW);
        }
        y += 14.0 * s;
        // Worlds heard on the LAN, then recent addresses.
        let lan: Vec<LanWorld> = self.net.lan.as_ref().map(|l| l.worlds()).unwrap_or_default();
        self.ui.text(x, y, "Na sua rede:", GRAY);
        y += 11.0 * s;
        let mut go = None;
        if lan.is_empty() {
            self.ui.text(x + 6.0 * s, y, "(procurando mundos abertos...)", GRAY);
            y += 12.0 * s;
        }
        for l in lan.iter().take(4) {
            let label = format!("{} - {} ({})", l.world, l.host, l.addr);
            if self.ui.button(x, y, bw, bh, &label, mouse) && self.clicked {
                go = Some(l.addr.to_string());
            }
            y += bh + 3.0 * s;
        }
        let recent: Vec<String> = self.net.cfg.recent.iter().take(3).cloned().collect();
        if !recent.is_empty() {
            y += 4.0 * s;
            self.ui.text(x, y, "Recentes:", GRAY);
            y += 11.0 * s;
            for r in recent {
                if self.ui.button(x, y, bw, bh, &r, mouse) && self.clicked {
                    go = Some(r);
                }
                y += bh + 3.0 * s;
            }
        }
        if let Some(a) = go {
            self.sfx(Sfx::Click);
            self.net.cfg.last_addr = a.clone();
            self.join(&a);
        }
        y = y.max(h - 52.0 * s);
        let upnp = format!(
            "Alcance: {}",
            match self.net.cfg.reach {
                Reach::Lan => "Rede local",
                Reach::Internet => "Internet",
                Reach::Relay => "Ponte",
            }
        );
        if self.ui.button(x, y, half, bh, "Hospedar mundo...", mouse) && self.clicked {
            self.sfx(Sfx::Click);
            self.worlds = crate::menus::list_worlds(&self.base);
            self.net.picking_world = true;
        }
        if self.ui.button(x + half + 6.0 * s, y, half, bh, &upnp, mouse) && self.clicked {
            self.sfx(Sfx::Click);
            self.net.cfg.reach = self.net.cfg.reach.next();
            self.net.save_config(&self.base);
        }
        y += bh + 4.0 * s;
        if self.ui.button(x, y, bw, bh, "Voltar", mouse) && self.clicked {
            self.sfx(Sfx::Click);
            self.net.save_config(&self.base);
            self.set_screen(Screen::Title);
        }
    }

    /// Test hooks: MC_HOST=1 hosts the world being opened, MC_JOIN=address joins at start
    /// (MC_NAME sets the name), MC_MP_MENU=1 opens the multiplayer screen, MC_NET_BUILD=1 has a guest
    /// build a pillar once in.
    pub(crate) fn net_test_hooks(&mut self) {
        self.net.load_config(&self.base.clone());
        if let Ok(n) = std::env::var("MC_NAME") {
            self.net.cfg.name = clean_name(&n);
        }
        if std::env::var("MC_HOST").is_ok() {
            self.net.host_when_ready = true;
        }
        self.net.test_build = std::env::var("MC_NET_BUILD").is_ok();
        self.net.test_cmds = std::env::var("MC_CMD").map(|c| c.split(';').map(|l| l.trim().to_string()).collect()).unwrap_or_default();
        if std::env::var("MC_MP_MENU").is_ok() {
            // Test hook: open the multiplayer screen (with --title).
            self.set_screen(Screen::Multiplayer);
        }
        if let Ok(a) = std::env::var("MC_JOIN") {
            self.join(&a);
        }
    }
}

/// Break text into lines no wider than `max`.
fn wrap_text(ui: &crate::ui::UiBatch, text: &str, max: f32, s: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if ui.text_width(&next, s) > max && !line.is_empty() {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
