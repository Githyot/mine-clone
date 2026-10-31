//! Phase 6 state in one place: effects, potions in flight, the stations' working parts,
//! projectiles thrown or shot by the player, statistics and advancements. Saved per world in
//! `progression.ron` next to `survival.ron`, by name, so old worlds and new items get along.

use std::collections::HashMap;
use std::path::Path;

use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};
use voxel::item::{Inventory, ItemRegistry};
use voxel::rng::Rng;

use crate::effects::{Active, Cloud, Thrown};

/// What hurt the player, for the protection enchantments and the effects that block some harm.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DamageKind {
    #[default]
    Attack,
    Projectile,
    Fall,
    Fire,
    Explosion,
    Drown,
    Magic,
    Starve,
}

/// Something that happened, for advancements.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Enchanted,
    Brewed,
    Drank(String),
    Smithed(String),
    Trimmed,
    Killed(String),
    Brushed,
    Shrieked,
    WardenSpawned,
    VaultOpened,
    SpawnerBeaten,
    BeaconOn(u8),
    ConduitOn,
    MaceSmash(f32),
    Recovered,
    Sniffed,
    Repaired,
    Ground,
    /// Anything else, by name ("slept", "traded", "trial_chamber"...).
    Token(String),
}

/// Counters by name (blocks mined, mobs killed, metres walked...), shown on the stats screen.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub counts: HashMap<String, u64>,
}

impl Stats {
    pub fn add(&mut self, key: &str, n: u64) {
        *self.counts.entry(key.to_string()).or_insert(0) += n;
    }

    pub fn get(&self, key: &str) -> u64 {
        self.counts.get(key).copied().unwrap_or(0)
    }
}

/// A brewing stand's working parts (its five slots are a container at the same spot).
#[derive(Clone, Copy, Default, Debug)]
pub struct Brew {
    /// Seconds left of the current brew (0 = idle).
    pub time: f32,
    /// Brews left from the blaze powder put in (20 each).
    pub fuel: u8,
}

/// A beacon's chosen powers (effect ids, 0 = none) and pyramid height last time it looked.
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct BeaconState {
    pub primary: u8,
    pub secondary: u8,
    #[serde(default)]
    pub levels: u8,
}

/// The anvil's waiting rename text (typed while the anvil is open).
#[derive(Default)]
pub struct Prog {
    pub effects: Vec<Active>,
    /// Golden hearts from absorption, used up before health.
    pub absorb: f32,
    pub fx_tick: f32,
    pub fx_ticks: u32,
    pub swirl_t: f32,
    pub drink: f32,
    pub drink_sound: f32,
    pub thrown: Vec<Thrown>,
    pub clouds: Vec<Cloud>,
    /// Set just before `damage` to say what kind of harm it is (reset after).
    pub next_kind: DamageKind,
    pub stats: Stats,
    /// Advancement keys earned, and the ones not yet shown in a toast.
    pub earned: Vec<String>,
    pub pending_toasts: Vec<String>,
    pub events: Vec<Event>,
    /// Enchanting table: the seed that decides its offers (changes after each enchanting).
    pub ench_seed: u32,
    /// The items in an open table, anvil, grindstone or smithing table (given back on close).
    pub bench: Inventory,
    pub brews: HashMap<IVec3, Brew>,
    pub beacons: HashMap<IVec3, BeaconState>,
    pub beacon_t: f32,
    pub conduit_t: f32,
    /// Player arrows, tridents, wind charges in flight.
    pub shots: Vec<crate::weapons::Shot>,
    /// Bow or crossbow being drawn: seconds held; crossbow loaded.
    pub draw: f32,
    pub loaded: bool,
    pub was_using: bool,
    /// Riptide spin left, and mace fall start.
    pub spin: f32,
    pub mace_peak: f32,
    /// Frosted ice made by frost walker: where and how long it lasts.
    pub frost: Vec<(IVec3, f32)>,
    /// Where the player died last (recovery compass, locator bar).
    pub death: Option<(Vec3, u8)>,
    /// Waypoints on the locator bar.
    pub show_advancements: bool,
    pub adv_tab: usize,
    pub adv_scroll: usize,
    pub stats_open: bool,
    pub rng: Rng,
    /// Deep dark: the warning level the shriekers have built up (0..4) and its cool-down.
    pub warning: u8,
    pub warning_t: f32,
    pub shriek_cd: f32,
    pub heartbeat_t: f32,
    /// Vibrations waiting to reach sculk sensors: (source, kind, seconds left).
    pub vibrations: Vec<(Vec3, u8, f32)>,
    pub sensor_on: Vec<(IVec3, f32)>,
    /// Trial spawners: per block, the wave state.
    pub trials: HashMap<IVec3, crate::trials::Spawner>,
    /// Vaults already opened (by position) in this world.
    pub vaults_opened: Vec<IVec3>,
    /// Brushing: block, seconds brushed.
    pub brushing: Option<(IVec3, f32)>,
    /// Sniffers' digs and eggs hatching: position, seconds left.
    pub eggs: Vec<(IVec3, f32)>,
    pub sniff_t: f32,
    pub catalyst_bloom: Vec<(Vec3, u32)>,
    pub stat_walk: f32,
    pub play_t: f32,
    /// Sweeping arc to draw: seconds left.
    pub sweep: f32,
    pub wind_t: f32,
    /// The swing that just landed had a full charge (sweeping).
    pub was_full: bool,
    pub prev_ground: bool,
    pub last_pos: Vec3,
    pub adv_t: f32,
    pub toast_t: f32,
    /// Watched blocks around the player (trials.rs scans them once a second).
    pub near: Vec<(IVec3, voxel::BlockId)>,
    pub scan_t: f32,
    /// Breeze wind charges in flight.
    pub charges: Vec<crate::trials::Charge>,
    /// Height a wind charge launched the player from (0 = not flying from one).
    pub wind_from: f32,
    /// Deep dark: steps walked since the last vibration, last position, mob vibration timer,
    /// the vibration kind each sensor heard last, the warden's timers.
    pub dd_last: Vec3,
    pub dd_walk: f32,
    pub prev_ground_dd: bool,
    pub mob_vib_t: f32,
    pub sensor_kind: HashMap<IVec3, u8>,
    pub boom_t: f32,
    pub dark_t: f32,
    pub warden_away: f32,
}

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    #[serde(default)]
    effects: Vec<Active>,
    #[serde(default)]
    absorb: f32,
    #[serde(default)]
    stats: HashMap<String, u64>,
    #[serde(default)]
    earned: Vec<String>,
    #[serde(default)]
    ench_seed: u32,
    #[serde(default)]
    beacons: Vec<([i32; 3], BeaconState)>,
    #[serde(default)]
    brews: Vec<([i32; 3], f32, u8)>,
    #[serde(default)]
    death: Option<([f32; 3], u8)>,
    #[serde(default)]
    vaults_opened: Vec<[i32; 3]>,
    #[serde(default)]
    frost: Vec<([i32; 3], f32)>,
    #[serde(default)]
    eggs: Vec<([i32; 3], f32)>,
    #[serde(default)]
    warning: u8,
}

impl Prog {
    pub fn new() -> Prog {
        Prog { ench_seed: 0x51ed, bench: Inventory::new(3), rng: Rng::new(0x6a5e), ..Default::default() }
    }

    pub fn event(&mut self, e: Event) {
        self.events.push(e);
    }

    pub fn save(&self, dir: &Path, _items: &ItemRegistry) {
        let s = Saved {
            effects: self.effects.iter().filter(|e| !e.ambient).copied().collect(),
            absorb: self.absorb,
            stats: self.stats.counts.clone(),
            earned: self.earned.clone(),
            ench_seed: self.ench_seed,
            beacons: self.beacons.iter().map(|(p, b)| (p.to_array(), *b)).collect(),
            brews: self.brews.iter().map(|(p, b)| (p.to_array(), b.time, b.fuel)).collect(),
            death: self.death.map(|(p, d)| (p.to_array(), d)),
            vaults_opened: self.vaults_opened.iter().map(|p| p.to_array()).collect(),
            frost: self.frost.iter().map(|(p, t)| (p.to_array(), *t)).collect(),
            eggs: self.eggs.iter().map(|(p, t)| (p.to_array(), *t)).collect(),
            warning: self.warning,
        };
        if let Ok(text) = ron::ser::to_string_pretty(&s, Default::default()) {
            let _ = std::fs::create_dir_all(dir);
            if let Err(e) = voxel::save::write_atomic(dir.join("progression.ron"), text) {
                log::error!("could not save progression.ron: {e}");
            }
        }
    }

    /// The saved state, or a fresh one. Effects come back as they were (health boost is part of
    /// the saved maximum health already).
    pub fn load(dir: &Path) -> Prog {
        let mut p = Prog::new();
        let Some(s) = std::fs::read_to_string(dir.join("progression.ron")).ok().and_then(|t| ron::from_str::<Saved>(&t).ok()) else {
            return p;
        };
        p.effects = s.effects;
        p.absorb = s.absorb;
        p.stats.counts = s.stats;
        p.earned = s.earned;
        p.ench_seed = if s.ench_seed == 0 { 0x51ed } else { s.ench_seed };
        p.beacons = s.beacons.into_iter().map(|(a, b)| (IVec3::from(a), b)).collect();
        p.brews = s.brews.into_iter().map(|(a, t, f)| (IVec3::from(a), Brew { time: t, fuel: f })).collect();
        p.death = s.death.map(|(a, d)| (Vec3::from(a), d));
        p.vaults_opened = s.vaults_opened.into_iter().map(IVec3::from).collect();
        p.frost = s.frost.into_iter().map(|(a, t)| (IVec3::from(a), t)).collect();
        p.eggs = s.eggs.into_iter().map(|(a, t)| (IVec3::from(a), t)).collect();
        p.warning = s.warning;
        p
    }
}

/// Seconds on a clock that keeps running, for shimmering icons drawn without the game at hand.
pub fn clock_now() -> f32 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_secs_f32()
}

impl crate::Game {
    /// Everything Phase 6 does each frame.
    pub(crate) fn update_fase6(&mut self, dt: f32, use_held: bool) {
        self.prog.play_t += dt;
        if self.prog.play_t >= 1.0 {
            self.prog.play_t -= 1.0;
            self.prog.stats.add("play_seconds", 1);
        }
        self.update_effects(dt);
        self.update_drinking(dt, use_held);
        self.update_weapons(dt, use_held);
        self.update_stations(dt);
        self.scan_nearby(dt);
        self.update_trials(dt);
        self.wind_jump_check();
        self.update_deepdark(dt);
        self.update_archaeology(dt, use_held);
        self.update_advancements(dt);
        self.update_extras(dt, use_held);
    }

    /// Mobs that died this tick: statistics, advancements, extra drops (spider eyes...).
    pub(crate) fn fase6_mob_deaths(&mut self, deaths: &[(usize, Vec3)]) {
        for &(def, pos) in deaths {
            let name = self.mobs.reg.defs[def].name.clone();
            self.extras_mob_drop(&name, pos);
            self.prog.stats.add("mobs_killed", 1);
            self.prog.stats.add(&format!("killed:{name}"), 1);
            self.prog.event(Event::Killed(name.clone()));
            let xp = self.mobs.reg.defs[def].xp;
            self.catalyst_feed(pos, xp);
            if !self.survival {
                continue;
            }
            let extra: &[(&str, f32, u8)] = match name.as_str() {
                "spider" => &[("spider_eye", 0.33, 1)],
                "rabbit" => &[("rabbit_foot", 0.1, 1)],
                "swamp_witch" => &[("glowstone_dust", 0.3, 2), ("sugar", 0.3, 2), ("spider_eye", 0.2, 1), ("glass_bottle", 0.3, 2), ("redstone", 0.2, 2), ("slime_ball", 0.3, 2)],
                "fire_imp" => &[("magma_cream", 0.3, 1)],
                "ember_wraith" => &[("ghast_tear", 0.15, 1)],
                "void_walker" | "void_walker_soul" | "void_walker_crimson" | "void_walker_savanna" | "void_walker_swamp" | "void_walker_snowy" | "void_walker_desert" => &[("phantom_membrane", 0.2, 1)],
                "warden" => &[("sculk_catalyst", 1.0, 1)],
                "breeze" => &[("breeze_rod", 1.0, 2)],
                "chicken" => &[("feather", 0.0, 1)],
                "zombie" | "skeleton" => &[("glowstone_dust", 0.0, 1)],
                _ => &[],
            };
            for &(item, chance, max) in extra {
                if self.prog.rng.f32() < chance {
                    if let Some(id) = self.items.id(item) {
                        let n = 1 + self.prog.rng.below(max as u32) as u8;
                        self.pop_drop(pos + Vec3::Y * 0.5, voxel::item::ItemStack::new(id, u16::from(n)));
                    }
                }
            }
        }
    }

    pub(crate) fn fase6_block_broken(&mut self, pos: IVec3, _id: voxel::BlockId) {
        self.prog.stats.add("blocks_mined", 1);
        self.vibrate(pos.as_vec3() + Vec3::splat(0.5), crate::deepdark::BLOCK);
    }

    pub(crate) fn fase6_died(&mut self) {
        let dim = self.dim.as_u8();
        self.prog.death = Some((self.body.pos, dim));
        self.prog.stats.add("deaths", 1);
        self.clear_effects();
        self.prog.absorb = 0.0;
    }

    /// Right click on a block (before the older blocks get theirs). True when handled.
    pub(crate) fn fase6_block_use(&mut self, name: &str, pos: IVec3) -> bool {
        self.station_use(name, pos) || self.vault_use(name, pos)
    }

    /// Right click with an item in hand. True when handled.
    pub(crate) fn fase6_item_use(&mut self, name: &str) -> bool {
        if crate::effects::potion_of(name).is_some() {
            return self.use_potion(name);
        }
        if matches!(name, "milk_bucket" | "honey_bottle" | "ominous_bottle") {
            return true;
        }
        if name == "glass_bottle" {
            // Scoop water: the first liquid block along the look, within reach.
            let eye = self.body.eye();
            let f = self.forward();
            for k in 1..10 {
                let q = (eye + f * (k as f32 * 0.5)).floor().as_ivec3();
                let b = self.world.block_or_air(q);
                if self.world.fluid(b) == 1 {
                    if let Some(id) = self.items.id("potion_water") {
                        if self.survival {
                            let sel = self.selected;
                            self.surv.inv.take(sel, 1);
                            if let Some(rest) = self.surv.inv.add(&self.items, voxel::item::ItemStack::new(id, 1)) {
                                self.pop_drop(self.body.pos + Vec3::Y, rest);
                            }
                        }
                        self.sfx(crate::audio::Sfx::Splash(0.2));
                    }
                    return true;
                }
                if b != voxel::block::AIR {
                    break;
                }
            }
            return false;
        }
        if name == "experience_bottle" {
            let f = self.forward();
            let at = self.body.eye() + f * 2.0;
            let n = 3 + self.prog.rng.below(9);
            self.spawn_orbs(at, n);
            if self.survival {
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
            }
            self.sfx(crate::audio::Sfx::Break(crate::audio::Material::Glass));
            return true;
        }
        self.weapon_use(name)
    }

    /// Food with effects: golden apples, spider eyes, pufferfish, rotten flesh.
    pub(crate) fn fase6_ate(&mut self, name: &str) {
        use crate::effects::*;
        self.extras_ate(name);
        self.tr3_ate(name);
        self.prog.stats.add("foods_eaten_total", 1);
        let key = format!("ate:{name}");
        if self.prog.stats.get(&key) == 0 {
            self.prog.stats.add("foods_eaten", 1);
        }
        self.prog.stats.add(&key, 1);
        match name {
            "golden_apple" => {
                self.add_effect(REGENERATION, 1, 5.0, false);
                self.add_effect(ABSORPTION, 0, 120.0, false);
            }
            "enchanted_golden_apple" => {
                self.add_effect(REGENERATION, 1, 20.0, false);
                self.add_effect(ABSORPTION, 3, 120.0, false);
                self.add_effect(RESISTANCE, 0, 300.0, false);
                self.add_effect(FIRE_RESISTANCE, 0, 300.0, false);
            }
            "spider_eye" => self.add_effect(POISON, 0, 5.0, false),
            "pufferfish" => {
                self.add_effect(POISON, 1, 60.0, false);
                self.add_effect(HUNGER, 2, 15.0, false);
                self.add_effect(NAUSEA, 0, 15.0, false);
            }
            "rotten_flesh" if self.prog.rng.f32() < 0.8 => self.add_effect(HUNGER, 0, 30.0, false),
            "golden_carrot" | "glow_berries" => {}
            _ => {}
        }
    }

    /// Tooltip lines Phase 6 adds: a potion's effects.
    pub(crate) fn fase6_details(&self, st: &voxel::item::ItemStack) -> Vec<(String, [u8; 4])> {
        let mut out = Vec::new();
        let name = &self.items.get(st.item).name;
        if let Some((form, p)) = crate::effects::potion_of(name) {
            if p.effects.is_empty() {
                out.push(("Sem efeitos".to_string(), [170, 170, 170, 255]));
            }
            for &(e, a, t) in p.effects {
                let (n, _, bad) = crate::effects::info(e);
                let t = match form {
                    crate::effects::Form::Arrow => t / 8.0,
                    crate::effects::Form::Lingering => t / 4.0,
                    _ => t,
                };
                let lvl = if a > 0 { format!(" {}", crate::enchanting::roman(a + 1)) } else { String::new() };
                let time = if t > 0.0 { format!(" ({})", crate::effects::clock(t)) } else { String::new() };
                out.push((format!("{n}{lvl}{time}"), if bad { [255, 85, 85, 255] } else { [85, 85, 255, 255] }));
            }
        }
        out
    }

    /// 0..1: how much night vision lights the dark (it flickers out in its last seconds).
    pub(crate) fn night_vision(&self) -> f32 {
        let nv = self.prog.effects.iter().find(|e| e.id == crate::effects::NIGHT_VISION).map(|e| e.time);
        let cp = self.has_effect(crate::effects::CONDUIT_POWER) && self.body.head_in_water;
        match nv {
            Some(t) if t < 10.0 => 0.7 * (0.5 + 0.5 * (t * 3.0).sin().abs()),
            Some(_) => 0.7,
            None if cp => 0.6,
            None => 0.0,
        }
    }

    /// Blindness and darkness pull the fog in close.
    pub(crate) fn effect_fog(&self, color: [f32; 3], start: f32, end: f32) -> ([f32; 3], f32, f32) {
        if self.has_effect(crate::effects::BLINDNESS) {
            return ([0.0, 0.0, 0.0], 0.0, 6.0);
        }
        if self.has_effect(crate::effects::DARKNESS) {
            let k = 0.5 + 0.5 * (self.now * 1.6).sin().abs();
            return ([0.0, 0.0, 0.0], 2.0, 10.0 + 14.0 * (1.0 - k));
        }
        (color, start, end)
    }

    /// Phase 6 parts of the HUD: absorption hearts, effect icons, the bow's pull, the locator
    /// bar and advancement toasts.
    pub(crate) fn draw_fase6_hud(&mut self, w: f32, h: f32, hx: f32, hy: f32, cell: f32) {
        let s = self.ui.scale;
        if self.survival && self.prog.absorb > 0.0 {
            let rows = ((self.max_health / 2.0).ceil() as i32 + 9) / 10;
            let y = hy - 11.0 * s - rows as f32 * 10.0 * s;
            let n = (self.prog.absorb / 2.0).ceil() as i32;
            for i in 0..n.min(10) {
                let x = hx + i as f32 * 8.0 * s;
                self.ui.rect(x + s, y + s, 7.0 * s, 6.0 * s, [230, 180, 30, 255]);
                self.ui.rect(x + 2.0 * s, y + 2.0 * s, 2.0 * s, 2.0 * s, [255, 240, 140, 255]);
            }
        }
        let pull = self.bow_pull();
        if pull > 0.0 && self.screen == crate::Screen::Playing {
            let (cx, cy) = (w / 2.0, h / 2.0);
            let bw = 16.0 * s;
            self.ui.rect(cx - bw / 2.0, cy + 12.0 * s, bw, 2.0 * s, [40, 40, 40, 200]);
            self.ui.rect(cx - bw / 2.0, cy + 12.0 * s, bw * pull, 2.0 * s, [255, 220, 120, 230]);
        }
        let _ = cell;
        self.draw_effects_hud(w);
        self.draw_locator_bar(w, hx, hy);
        self.draw_advancement_toast(w);
        let _ = h;
    }
}
