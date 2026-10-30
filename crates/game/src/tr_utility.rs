//! Lote 9 of the Terraria side (plan/terraria-completo.md): the things that make getting around
//! and crafting easier.
//!
//! - Crafting stations: the Infernal Forge (magmarite gear), the Arcane Anvil (the hard-mode
//!   metals), the Titanic Forge (the celestial gear) and the Alchemy Table (the new potions).
//!   The grid only gives those results with the station within 4 blocks.
//! - Getting around: the Recall, Teleport, Gravity and Wormhole potions, the Magic Mirror (a
//!   recall that is never used up), the Magic Conch (to the nearest beach) and the PDA (every
//!   info item at once, from the backpack, and a recall on right click).
//! - The piggy bank and the safe: the same storage from every piggy bank (and the money
//!   trough), the same from every safe.
//! - The sprayer: pure or vile solution at what you look at, far away.
//! - The Shimmer Lake: items dropped in it turn into others or come apart into what made them;
//!   life and mana crystals and golden apples become permanent upgrades.
//!
//! Teleports to far places hold the player still in the air until the chunks there are in.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::physics::Terrain;
use voxel::worldgen::{Dim, SEA_LEVEL};

use crate::audio::{Material, Sfx};
use crate::rpg::{INFO_COORDS, INFO_DEPTH, INFO_DPS, INFO_ORE, INFO_RADAR, INFO_TIME, INFO_WEATHER};
use crate::survival::{Container, Open};
use crate::Game;

pub const STATION_INFERNAL: u8 = 1;
pub const STATION_ARCANE: u8 = 2;
pub const STATION_TITANIC: u8 = 4;
pub const STATION_ALCHEMY: u8 = 8;

const STATIONS: &[(&str, u8, &str)] = &[
    ("infernal_forge", STATION_INFERNAL, "Forja Infernal"),
    ("arcane_anvil", STATION_ARCANE, "Bigorna Arcana"),
    ("titanic_forge", STATION_TITANIC, "Forja Titânica"),
    ("alchemy_table", STATION_ALCHEMY, "Mesa de Alquimia"),
];

/// The piggy bank and the safe keep their things under these keys (never a real block).
pub(crate) const BANK_X: i32 = i32::MIN + 9;

/// Shimmer swaps (both ways) and one-way changes, before taking a thing apart.
const SHIMMER: &[(&str, &str)] = &[
    ("copper_ingot", "tin_ingot"),
    ("iron_ingot", "lead_ingot"),
    ("silver_ingot", "tungsten_ingot"),
    ("gold_ingot", "platinum_ingot"),
    ("cobalt_ingot", "palladium_ingot"),
    ("mythril_ingot", "orichalcum_ingot"),
    ("adamantite_ingot", "titanium_ingot"),
    ("umbral_ore", "sanguinite_ore"),
    ("soul_of_night", "soul_of_light"),
];
const SHIMMER_ONEWAY: &[(&str, &str)] = &[
    ("life_crystal", "vital_crystal"),
    ("mana_crystal", "arcane_shard"),
    ("golden_apple", "aegis_fruit"),
    ("rotten_flesh", "leather"),
    ("gravel", "flint"),
    ("fallen_star", "glowstone_dust"),
    ("torch", "shimmer_crystal"),
];

/// Permanent upgrades from the Shimmer (bits of `Utility::perma`).
pub const PERMA_VITAL: u32 = 1;
pub const PERMA_ARCANE: u32 = 2;
pub const PERMA_AEGIS: u32 = 4;

#[derive(Default)]
pub struct Utility {
    /// Stations within reach (bits), looked for twice a second.
    pub stations: u8,
    /// Info shown by items carried in the inventory (the PDA and the info accessories).
    pub inv_info: u32,
    scan_t: f32,
    /// A teleport waiting for its chunks: (spot, put on the surface when it loads, seconds left).
    warp: Option<(Vec3, bool, f32)>,
    /// Seconds of the gravity potion left.
    pub gravity_t: f32,
    /// Which town folk the next wormhole goes to.
    worm_k: usize,
    pub perma: u32,
    shimmer_t: f32,
}

/// The station a crafted result needs, if any.
fn station_for(name: &str) -> Option<u8> {
    let gear = |rest: &str| !["ingot", "ore", "block", "fragment", "tablet", "nugget"].iter().any(|s| rest == *s);
    let (pre, rest) = name.split_once('_')?;
    match pre {
        "magmarite" if gear(rest) => Some(STATION_INFERNAL),
        "mythril" | "orichalcum" | "adamantite" | "titanium" | "radiant" | "verdite" | "spectral" | "sporite" | "beetle" | "turtle" if gear(rest) && rest != "shell" && rest != "scute" && rest != "husk" => Some(STATION_ARCANE),
        "lumite" | "solar" | "vortex" | "nebula" | "stardust" if gear(rest) => Some(STATION_TITANIC),
        _ => match name {
            "recall_potion" | "teleport_potion" | "gravity_potion" | "wormhole_potion" | "pure_solution" | "vile_solution" => Some(STATION_ALCHEMY),
            "sun_javelin" => Some(STATION_TITANIC),
            _ => None,
        },
    }
}

fn station_name(bit: u8) -> &'static str {
    STATIONS.iter().find(|s| s.1 == bit).map(|s| s.2).unwrap_or("")
}

impl Game {
    // -----------------------------------------------------------------------------------------
    // Stations.

    /// May the grid give this result here (the station it needs is near)?
    pub(crate) fn station_ok(&self, item: voxel::item::ItemId) -> bool {
        if !self.survival {
            return true;
        }
        station_for(&self.items.get(item).name).is_none_or(|b| self.t5.tr.util.stations & b != 0)
    }

    /// " + Bigorna Arcana" for the recipe book, or nothing.
    pub(crate) fn station_label(&self, item: voxel::item::ItemId) -> String {
        station_for(&self.items.get(item).name).map(|b| format!(" + {}", station_name(b))).unwrap_or_default()
    }

    fn scan_stations(&mut self) {
        let ids: Vec<(u16, u8)> = STATIONS.iter().map(|s| (self.reg.id_or_air(s.0), s.1)).filter(|s| s.0 != voxel::AIR).collect();
        let me = self.body.pos.floor().as_ivec3();
        let mut got = 0;
        for dy in -2..=3 {
            for dz in -4..=4 {
                for dx in -4..=4 {
                    // Any facing of a station counts (the arcane anvil turns).
                    let id = self.reg.get(self.world.block_or_air(me + IVec3::new(dx, dy, dz))).base;
                    if let Some(&(_, b)) = ids.iter().find(|s| s.0 == id) {
                        got |= b;
                    }
                }
            }
        }
        self.t5.tr.util.stations = got;
        // Info items carried anywhere in the inventory count too.
        let mut info = 0;
        for st in self.surv.inv.slots.iter().flatten() {
            let name = &self.items.get(st.item).name;
            if name == "pda" {
                info |= INFO_DEPTH | INFO_COORDS | INFO_TIME | INFO_WEATHER | INFO_RADAR | INFO_DPS | INFO_ORE;
            } else if let Some(a) = crate::rpg::accessory(name) {
                for m in a.mods {
                    if let crate::rpg::M::Info(b) = m {
                        info |= b;
                    }
                }
            }
        }
        self.t5.tr.util.inv_info = info;
    }

    // -----------------------------------------------------------------------------------------
    // Items.

    pub(crate) fn tr_utility_use(&mut self, name: &str) -> Option<bool> {
        let used_up = match name {
            "recall_potion" => self.warp_home(),
            "magic_mirror" | "pda" => return Some(self.warp_home()),
            "teleport_potion" => self.warp_random(),
            "magic_conch" => return Some(self.warp_beach()),
            "wormhole_potion" => self.warp_wormhole(),
            "gravity_potion" => {
                self.t5.tr.util.gravity_t = 180.0;
                self.toast("Gravidade leve por 3 minutos: segure Espaço no ar para flutuar", 3.0);
                self.sfx(Sfx::Eat);
                true
            }
            "money_trough" => return Some(self.open_bank(0)),
            "sprayer" => return Some(self.use_sprayer()),
            "vital_crystal" | "arcane_shard" | "aegis_fruit" => {
                let (bit, text) = match name {
                    "vital_crystal" => (PERMA_VITAL, "Cristal Vital: a vida volta mais depressa para sempre"),
                    "arcane_shard" => (PERMA_ARCANE, "Lasca Arcana: mais mana e mana que volta mais depressa, para sempre"),
                    _ => (PERMA_AEGIS, "Fruta da Égide: +4 de defesa para sempre"),
                };
                if self.t5.tr.util.perma & bit != 0 {
                    self.toast("Você já usou um destes", 2.0);
                    return Some(true);
                }
                self.t5.tr.util.perma |= bit;
                self.toast(text, 4.0);
                self.sfx(Sfx::Xp);
                true
            }
            _ => return None,
        };
        if used_up && self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        Some(true)
    }

    fn warp_glow(&mut self) {
        let glow = self.glow_layer();
        for _ in 0..30 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() * 2.0, self.surv.rng.f32() - 0.5) * 2.0;
            self.particles.glow(self.body.pos + Vec3::Y, v, glow, [1.2, 1.0, 1.8], 0.1, 0.8);
        }
        self.sfx(Sfx::Respawn);
    }

    /// Teleport, waiting in place for the chunks there if they aren't in yet.
    fn warp_to(&mut self, spot: Vec3, surface: bool) {
        self.warp_glow();
        self.t5.tr.util.warp = Some((spot, surface, 20.0));
        self.hold_warp();
    }

    fn hold_warp(&mut self) {
        let Some((spot, surface, left)) = self.t5.tr.util.warp else { return };
        let mut at = spot;
        let col = IVec3::new(spot.x.floor() as i32, 64, spot.z.floor() as i32);
        let loaded = self.world.block(col).is_some();
        if loaded && surface {
            if let Some(y) = self.world.surface_y(col.x, col.z) {
                at.y = y as f32 + 1.0;
            }
        }
        self.body.pos = at;
        self.body.vel = Vec3::ZERO;
        self.body.fall_distance = 0.0;
        self.fall_peak = at.y;
        if loaded || left <= 0.0 {
            self.t5.tr.util.warp = None;
            self.unstick();
        }
    }

    fn warp_home(&mut self) -> bool {
        if self.dim != Dim::Overworld {
            self.toast("Só funciona no mundo normal", 2.0);
            return false;
        }
        let (spot, surface) = match self.surv.spawn {
            Some(b) => (b.as_vec3() + Vec3::new(0.5, 0.6, 0.5), false),
            None => (Vec3::new(self.spawn_xz.0 as f32 + 0.5, 100.0, self.spawn_xz.1 as f32 + 0.5), true),
        };
        self.warp_to(spot, surface);
        true
    }

    fn warp_random(&mut self) -> bool {
        if self.dim != Dim::Overworld {
            self.toast("Só funciona no mundo normal", 2.0);
            return false;
        }
        for _ in 0..24 {
            let a = self.surv.rng.f32() * std::f32::consts::TAU;
            let r = 150.0 + self.surv.rng.f32() * 450.0;
            let (x, z) = ((self.body.pos.x + a.cos() * r) as i32, (self.body.pos.z + a.sin() * r) as i32);
            let h = self.world.gen.height_at(x, z);
            if h > SEA_LEVEL {
                self.warp_to(Vec3::new(x as f32 + 0.5, h as f32 + 1.0, z as f32 + 0.5), true);
                return true;
            }
        }
        self.toast("A poção não achou terra firme", 2.0);
        false
    }

    /// The nearest sea, then back towards the player to its shore.
    fn warp_beach(&mut self) -> bool {
        if self.dim != Dim::Overworld {
            self.toast("Só funciona no mundo normal", 2.0);
            return true;
        }
        let me = self.body.pos;
        let mut best = None;
        'out: for ring in 1..40 {
            let r = ring as f32 * 64.0;
            for k in 0..24 {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                let (x, z) = (me.x + a.cos() * r, me.z + a.sin() * r);
                if self.world.gen.height_at(x as i32, z as i32) < SEA_LEVEL - 6 {
                    best = Some((a, r));
                    break 'out;
                }
            }
        }
        let Some((a, r)) = best else {
            self.toast("A concha não ouve o mar daqui", 2.0);
            return true;
        };
        let mut d = r;
        while d > 0.0 {
            let (x, z) = ((me.x + a.cos() * d) as i32, (me.z + a.sin() * d) as i32);
            let h = self.world.gen.height_at(x, z);
            if h >= SEA_LEVEL {
                self.toast("O som do mar te leva até a praia", 2.5);
                self.warp_to(Vec3::new(x as f32 + 0.5, h as f32 + 1.0, z as f32 + 0.5), true);
                return true;
            }
            d -= 4.0;
        }
        true
    }

    fn warp_wormhole(&mut self) -> bool {
        let homes: Vec<(String, Vec3)> = self.town.npcs.iter().map(|n| (n.name.clone(), Vec3::from(n.home))).collect();
        if homes.is_empty() || self.dim != Dim::Overworld {
            self.toast("Ninguém mora na sua cidade ainda", 2.0);
            return false;
        }
        let k = self.t5.tr.util.worm_k % homes.len();
        self.t5.tr.util.worm_k += 1;
        let (_, at) = &homes[k];
        self.warp_to(*at + Vec3::new(0.0, 0.2, 0.0), false);
        true
    }

    // -----------------------------------------------------------------------------------------
    // Piggy bank and safe.

    /// 0 = piggy bank, 1 = safe.
    pub(crate) fn open_bank(&mut self, which: i32) -> bool {
        let p = IVec3::new(BANK_X, 0, which);
        self.surv.containers.entry(p).or_insert_with(|| Container::Chest(voxel::item::Inventory::new(36)));
        self.surv.open = Open::Chest(p);
        self.set_screen(crate::Screen::Inventory);
        self.sfx(Sfx::Place(if which == 0 { Material::Glass } else { Material::Metal }));
        true
    }

    // -----------------------------------------------------------------------------------------
    // The sprayer.

    fn use_sprayer(&mut self) -> bool {
        let sol = ["pure_solution", "vile_solution"].iter().find_map(|n| {
            let id = self.items.id(n)?;
            let slot = self.surv.inv.slots.iter().position(|s| s.is_some_and(|s| s.item == id))?;
            Some((slot, *n == "pure_solution"))
        });
        let Some((slot, pure)) = sol.or(if self.survival { None } else { Some((usize::MAX, true)) }) else {
            self.toast("O aspersor precisa de Solução Pura ou Vil na mochila", 2.5);
            return true;
        };
        let eye = self.body.eye();
        let f = self.forward();
        let mut hit = None;
        for k in 2..64 {
            let p = eye + f * (k as f32 * 0.5);
            if self.world.solid(p.floor().as_ivec3()) {
                hit = Some(p);
                break;
            }
        }
        let Some(at) = hit else { return true };
        let n = self.convert_area(at.floor().as_ivec3(), 3, pure);
        let glow = self.glow_layer();
        let tint = if pure { [0.8, 1.8, 0.8] } else { [1.2, 0.5, 1.8] };
        for k in 0..30 {
            let t = k as f32 / 30.0;
            let p = eye + (at - eye) * t;
            self.particles.glow(p, Vec3::Y * 0.3, glow, tint, 0.07, 0.6);
        }
        if n > 0 && slot != usize::MAX {
            self.surv.inv.take(slot, 1);
        }
        self.sfx(Sfx::Splash(0.3));
        true
    }

    // -----------------------------------------------------------------------------------------
    // The Shimmer Lake.

    /// What a stack turns into in the shimmer: (new stacks, how many of the old one it used).
    fn shimmer_result(&self, st: ItemStack) -> Option<(Vec<ItemStack>, u8)> {
        let name = &self.items.get(st.item).name;
        let swap = SHIMMER.iter().find_map(|(a, b)| if a == name { Some(*b) } else if b == name { Some(*a) } else { None });
        let to = swap.or_else(|| SHIMMER_ONEWAY.iter().find(|(a, _)| a == name).map(|(_, b)| *b));
        if let Some(to) = to {
            let id = self.items.id(to)?;
            return Some((vec![ItemStack::new(id, st.count)], st.count));
        }
        // Taken apart into what made it.
        let mut parts: Option<(Vec<voxel::item::ItemId>, u8)> = None;
        for r in &self.recipes.shaped {
            if r.result.item == st.item {
                parts = Some((r.cells.iter().flatten().filter_map(|i| i.first().copied()).collect(), r.result.count));
                break;
            }
        }
        if parts.is_none() {
            for r in &self.recipes.shapeless {
                if r.result.item == st.item {
                    parts = Some((r.ingredients.iter().filter_map(|i| i.first().copied()).collect(), r.result.count));
                    break;
                }
            }
        }
        let (ings, per) = parts?;
        let per = per.max(1);
        let times = st.count / per;
        if times == 0 || ings.is_empty() {
            return None;
        }
        let mut out: Vec<ItemStack> = Vec::new();
        for id in ings {
            match out.iter_mut().find(|s| s.item == id) {
                Some(s) => s.count = s.count.saturating_add(times),
                None => out.push(ItemStack::new(id, times)),
            }
        }
        Some((out, times * per))
    }

    fn update_shimmer(&mut self) {
        let shimmer = self.reg.id_or_air("shimmer");
        if shimmer == voxel::AIR {
            return;
        }
        let mut found = Vec::new();
        for (i, d) in self.surv.drops.iter().enumerate() {
            if d.age > 0.6 && d.age < 60.0 && self.world.block_or_air(d.body.pos.floor().as_ivec3()) == shimmer {
                found.push(i);
            }
        }
        for &i in found.iter().rev() {
            let d = &self.surv.drops[i];
            let (st, pos) = (d.stack, d.body.pos);
            let Some((out, used)) = self.shimmer_result(st) else {
                // Nothing to become: float back up and out.
                self.surv.drops[i].age = 60.5;
                self.surv.drops[i].body.vel = Vec3::Y * 6.0;
                continue;
            };
            if used >= st.count {
                self.surv.drops.remove(i);
            } else {
                self.surv.drops[i].stack.count -= used;
                self.surv.drops[i].age = 60.5;
            }
            for s in out {
                let max = self.items.get(s.item).max_stack.max(1);
                let mut left = s.count;
                while left > 0 {
                    let n = left.min(max);
                    left -= n;
                    let v = Vec3::new(self.surv.rng.f32() - 0.5, 4.0, self.surv.rng.f32() - 0.5);
                    self.spawn_drop(pos + Vec3::Y * 0.5, ItemStack { count: n, ..s }, v);
                    if let Some(d) = self.surv.drops.last_mut() {
                        d.age = 60.5;
                    }
                }
            }
            let glow = self.glow_layer();
            for _ in 0..16 {
                let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() * 2.0, self.surv.rng.f32() - 0.5) * 1.5;
                self.particles.glow(pos, v, glow, [1.6, 1.2, 2.0], 0.08, 0.9);
            }
            self.sfx(Sfx::Fizz);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Each frame.

    pub(crate) fn update_tr_utility(&mut self, dt: f32) {
        self.hold_warp();
        let u = &mut self.t5.tr.util;
        u.scan_t -= dt;
        u.shimmer_t -= dt;
        let scan = u.scan_t <= 0.0;
        let shim = u.shimmer_t <= 0.0;
        if scan {
            self.t5.tr.util.scan_t = 0.5;
            self.scan_stations();
        }
        if shim {
            self.t5.tr.util.shimmer_t = 0.25;
            self.update_shimmer();
        }
        if self.t5.tr.util.gravity_t > 0.0 && dt > 0.0 {
            self.t5.tr.util.gravity_t -= dt;
            let space = self.screen == crate::Screen::Playing && self.keys.contains(&winit::keyboard::KeyCode::Space);
            if !self.body.on_ground && !self.body.in_water {
                if space {
                    self.body.vel.y = self.body.vel.y.max(4.5);
                } else {
                    self.body.vel.y = self.body.vel.y.max(-4.0);
                }
                self.body.fall_distance = 0.0;
                self.fall_peak = self.body.pos.y;
            }
        }
    }

    /// Test scenes: MC_TR=stations (the four stations and the piggy bank and safe in a row),
    /// shimmer (a pool of shimmer with things floating in it).
    pub(crate) fn test_tr_utility(&mut self, scene: &str) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-flat.z, 0.0, flat.x);
        let base = self.body.pos + flat * 5.0;
        let names: &[&str] = if scene == "stations" {
            &["infernal_forge", "arcane_anvil", "titanic_forge", "alchemy_table", "piggy_bank", "safe", "tombstone"]
        } else {
            &[]
        };
        for (k, n) in names.iter().enumerate() {
            let p = (base + side * ((k as f32 - names.len() as f32 / 2.0) * 1.5)).floor().as_ivec3();
            let id = self.reg.id_or_air(n);
            self.world.set_block(p, id);
            if *n == "tombstone" {
                self.world.set_block(p + IVec3::Y, voxel::AIR);
            }
        }
        if scene == "shimmer" {
            let shimmer = self.reg.id_or_air("shimmer");
            let crystal = self.reg.id_or_air("shimmer_crystal");
            let rim = self.reg.id_or_air("amethyst_block");
            let c = (self.body.pos + flat * 6.0).floor().as_ivec3();
            for dz in -4i32..=4 {
                for dx in -4i32..=4 {
                    let d2 = dx * dx + dz * dz;
                    let p = c + IVec3::new(dx, -1, dz);
                    if d2 <= 9 {
                        self.world.set_block(p, shimmer);
                    } else if d2 <= 16 {
                        self.world.set_block(p, rim);
                    }
                }
            }
            for (dx, dz) in [(-4, -4), (4, -4), (-4, 4), (4, 4)] {
                self.world.set_block(c + IVec3::new(dx, 0, dz), crystal);
            }
            for (k, n) in ["life_crystal", "gold_ingot", "iron_pickaxe", "torch"].iter().enumerate() {
                if let Some(id) = self.items.id(n) {
                    let at = c.as_vec3() + Vec3::new(-1.5 + k as f32, 0.2, 0.5);
                    self.spawn_drop(at, ItemStack::new(id, 1), Vec3::ZERO);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stations_gate_the_right_things() {
        assert_eq!(station_for("magmarite_sword"), Some(STATION_INFERNAL));
        assert_eq!(station_for("magmarite_ingot"), None);
        assert_eq!(station_for("mythril_chestplate"), Some(STATION_ARCANE));
        assert_eq!(station_for("turtle_shell"), None);
        assert_eq!(station_for("solar_helmet"), Some(STATION_TITANIC));
        assert_eq!(station_for("solar_fragment"), None);
        assert_eq!(station_for("recall_potion"), Some(STATION_ALCHEMY));
        assert_eq!(station_for("iron_sword"), None);
        // The stations themselves never need a station.
        for s in STATIONS {
            assert_eq!(station_for(s.0), None, "{}", s.0);
        }
    }

    #[test]
    fn shimmer_items_exist() {
        fn read(dir: &std::path::Path, out: &mut String) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    read(&p, out);
                } else if p.extension().is_some_and(|x| x == "ron") {
                    out.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
                }
            }
        }
        let mut items = String::new();
        read(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../packs/vanilla")), &mut items);
        let missing: Vec<_> = SHIMMER.iter().chain(SHIMMER_ONEWAY).flat_map(|(a, b)| [*a, *b]).filter(|n| !items.contains(&format!("name: \"{n}\""))).collect();
        assert!(missing.is_empty(), "{missing:?}");
    }
}

/// Where the piggy bank's (0) or the safe's (1) contents are kept.
pub(crate) fn bank_pos(which: i32) -> IVec3 {
    IVec3::new(BANK_X, 0, which)
}
