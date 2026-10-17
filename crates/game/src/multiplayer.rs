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
    /// Ask the router to open the port (UPnP) when hosting.
    pub upnp: bool,
    /// Empty = anyone with the code may come in.
    pub password: String,
}

impl Default for NetConfig {
    fn default() -> Self {
        let name = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "Jogador".into());
        NetConfig { name: clean_name(&name), last_addr: String::new(), recent: Vec::new(), port: net::DEFAULT_PORT, upnp: true, password: String::new() }
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
    shown: Vec3,
    shown_yaw: f32,
    anim: player_model::Animator,
}

impl Remote {
    fn new(name: String, state: PlayerState) -> Remote {
        let p = Vec3::from(state.pos);
        Remote { name, shown: p, shown_yaw: state.yaw, state, anim: player_model::Animator::default() }
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
    host_when_ready: bool,
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
    upnp_note: String,
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
    Some(ItemStack { item: items.id(n)?, count: *c, damage: *d })
}

fn chunk_file(dir: &std::path::Path, p: ChunkPos) -> PathBuf {
    dir.join("chunks").join(format!("{}.{}.bin", p.x, p.y))
}

impl Game {
    // ---- shared helpers -------------------------------------------------------------------

    fn content_hash(&self) -> u64 {
        let blocks = self.reg.blocks.iter().map(|b| b.name.as_str());
        let items = self.items.items.iter().map(|i| i.name.as_str());
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
                self.toast(format!("Não deu para hospedar: {e}"), 6.0);
                return;
            }
        };
        let port = host.port;
        let beacon = Beacon::start(&self.world_name, &self.net.cfg.name, port);
        let upnp = self.net.cfg.upnp.then(|| net::upnp::open(port));
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
            upnp_note: if self.net.cfg.upnp { "abrindo a porta no roteador...".into() } else { "UPnP desligado".into() },
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
        let lan = self.net_lan_line();
        self.net_say(format!("Mundo aberto para amigos. Na mesma rede: {lan}"), YELLOW);
        self.toast("Mundo aberto para amigos (Esc mostra o convite)", 5.0);
    }

    fn net_lan_line(&self) -> String {
        let Mode::Host(h) = &self.net.mode else { return String::new() };
        match h.lan_ip {
            Some(ip) => format!("{ip}:{}", h.host.port),
            None => format!("porta {}", h.host.port),
        }
    }

    /// The code friends type to join: the router's address when UPnP worked, else the LAN one.
    pub(crate) fn net_invite_code(&self) -> Option<String> {
        let Mode::Host(h) = &self.net.mode else { return None };
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
                Ok(m) => {
                    h.external = Some(m.external);
                    h.upnp_note = format!("pela internet: {}", m.external);
                }
                Err(e) => h.upnp_note = format!("{e}; abra a porta UDP {} no roteador para jogar pela internet", h.host.port),
            }
            let line = self.host_ref().upnp_note.clone();
            self.net_say(format!("Convite: {} ({line})", self.net_invite_code().unwrap_or_default()), YELLOW);
        }

        let events = self.host_ref().host.poll();
        for e in events {
            match e {
                HostEvent::Joined(id, addr) => {
                    log::info!("guest {id} connected from {addr}");
                    self.host()
                        .guests
                        .insert(id, GuestInfo { name: String::new(), welcomed: false, state: None, sent: HashSet::new(), view: 8, containers: HashMap::new() });
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
                } else if !self.net.cfg.password.is_empty() && password != self.net.cfg.password {
                    Some("senha errada".to_string())
                } else if self.host_ref().bans.contains(&clean_name(&name).to_lowercase()) {
                    Some("banido deste mundo".to_string())
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
                    survival: self.survival,
                    difficulty: self.settings.difficulty,
                    time: self.time_of_day,
                    raining: self.weather.state.raining,
                    thunder: self.weather.state.thunder,
                    player,
                    op: self.host_ref().ops.contains(&name.to_lowercase()),
                };
                let h = self.host();
                h.host.send(id, &ToGuest::Welcome(w));
                if let Some(g) = h.guests.get_mut(&id) {
                    g.name = name.clone();
                    g.welcomed = true;
                    g.view = (view as i32).clamp(2, 16);
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
        }
    }

    /// Chat commands a guest may use.
    fn guest_command(&mut self, id: ConnId, text: &str) {
        let name = self.host_ref().guests.get(&id).map(|g| g.name.to_lowercase()).unwrap_or_default();
        let op = self.host_ref().ops.contains(&name);
        let cmd = text.split_whitespace().next().unwrap_or("").to_lowercase();
        let reply = match cmd.as_str() {
            "/lista" | "/list" => self.player_list_line(),
            _ if crate::commands::scope(&cmd) == crate::commands::Scope::World => {
                let r = self.run_command(text, op);
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
        let mut names = vec![match &self.net.mode {
            Mode::Guest(g) => g.host_name.clone(),
            _ => self.net.cfg.name.clone(),
        }];
        names.extend(self.net.remotes.values().map(|r| r.name.clone()));
        if self.net.is_guest() {
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
        self.world.net_log = None;
        let why = if why.is_empty() { "a conexão caiu".to_string() } else { why.to_string() };
        if joined && self.world_open {
            self.quit_to_title();
        }
        self.net.status = if joined { format!("Desconectado de {addr}: {why}") } else { format!("Não deu para entrar em {addr}: {why}") };
        self.set_screen(Screen::Multiplayer);
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
                if s.dim != my_dim || (r.shown - self.body.pos).length() > 160.0 {
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
                };
                (pose, s.swing, s.armor.clone(), s.held.clone())
            };
            let l = self.world.light((pose.pos + Vec3::Y).floor().as_ivec3());
            let (sky, blk) = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let def = &self.mobs.reg.defs[pd];
            let r = self.net.remotes.get_mut(&id).unwrap();
            r.anim.update(&pose, dt);
            r.anim.swing = swing;
            let bones = r.anim.bones(def, &pose, block, sky, blk);
            for (piece, name) in armor.iter().enumerate() {
                if !name.is_empty() {
                    if let Some(m) = self.entities.armor_mesh(name, piece) {
                        models.push((m, bones));
                    }
                }
            }
            models.push((pd, bones));
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
                let tags: Vec<(Vec3, String, bool)> = self
                    .net
                    .remotes
                    .values()
                    .filter(|r| r.state.dim == my_dim)
                    .map(|r| (r.shown + Vec3::Y * (2.1 - r.state.sneaking * 0.3), r.name.clone(), r.state.sneaking > 0.5))
                    .collect();
                for (top, name, sneaking) in tags {
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
                    self.ui.text(x - tw / 2.0, y, &name, if sneaking { GRAY } else { WHITE });
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
        if self.screen == Screen::Multiplayer && self.net.field != Field::None {
            let f = match self.net.field {
                Field::Name => &mut self.net.cfg.name,
                Field::Addr => &mut self.net.cfg.last_addr,
                Field::Password => &mut self.net.cfg.password,
                Field::None => unreachable!(),
            };
            match code {
                KeyCode::Backspace => {
                    f.pop();
                }
                KeyCode::Tab => {
                    self.net.field = match self.net.field {
                        Field::Name => Field::Addr,
                        Field::Addr => Field::Password,
                        _ => Field::Name,
                    }
                }
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let a = self.net.cfg.last_addr.clone();
                    self.net.field = Field::None;
                    self.join(&a);
                }
                KeyCode::Escape => self.net.field = Field::None,
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
        if self.screen == Screen::Multiplayer && self.net.field != Field::None {
            let (f, max) = match self.net.field {
                Field::Name => (&mut self.net.cfg.name, 16),
                Field::Addr => (&mut self.net.cfg.last_addr, 64),
                Field::Password => (&mut self.net.cfg.password, 32),
                Field::None => unreachable!(),
            };
            for c in text.chars() {
                if !c.is_control() && f.chars().count() < max {
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
            let r = self.run_command(text, cheats);
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
            "/tp" if !arg.is_empty() && arg.parse::<f32>().is_err() => {
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
            Mode::Off => "Abrir para amigos".into(),
            Mode::Host(h) => match self.net_invite_code() {
                Some(c) => format!("Convite: {c} ({} aqui)", h.guests.values().filter(|g| g.welcomed).count()),
                None => "Aberto para amigos".into(),
            },
            Mode::Guest(g) => format!("Mundo de {} ({} ms)", g.host_name, g.guest.ping().unwrap_or(0)),
        }
    }

    pub(crate) fn net_pause_button(&mut self) {
        match &self.net.mode {
            Mode::Off => {
                if self.world_name == GUEST_WORLD {
                    return;
                }
                self.start_hosting();
            }
            Mode::Host(h) => {
                let lines = vec![
                    format!("Código: {}", self.net_invite_code().unwrap_or_default()),
                    format!("Mesma rede: {}", self.net_lan_line()),
                    h.upnp_note.clone(),
                ];
                for l in lines {
                    self.net_say(l, YELLOW);
                }
                self.toast("O convite está no chat (T)", 4.0);
            }
            Mode::Guest(_) => {
                let l = self.player_list_line();
                self.net_say(l, YELLOW);
            }
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
        self.text_field(x, y, bw - jw - 6.0 * s, "Código do convite ou endereço (IP:porta)", &addr, Field::Addr, false, mouse);
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
        let upnp = format!("Abrir no roteador: {}", if self.net.cfg.upnp { "Sim" } else { "Não" });
        if self.ui.button(x, y, half, bh, "Hospedar mundo...", mouse) && self.clicked {
            self.sfx(Sfx::Click);
            self.worlds = crate::menus::list_worlds(&self.base);
            self.net.picking_world = true;
        }
        if self.ui.button(x + half + 6.0 * s, y, half, bh, &upnp, mouse) && self.clicked {
            self.sfx(Sfx::Click);
            self.net.cfg.upnp = !self.net.cfg.upnp;
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
