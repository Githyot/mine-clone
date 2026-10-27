//! Hard mode (Terraria's second half), here called the World Breaker age: beating the World
//! Breaker shakes the world. New ores grow into the stone of every chunk you visit (one of each
//! pair per world, the other rare), hostile mobs get tougher and hit harder, and the sorceress
//! moves in. Beating the three mechanical bosses lets verdite grow in the jungle.
//! Events: the blood moon (some nights, or a bloody tear) turns the sky red and brings blood
//! zombies and demon eyes; the goblin invasion (a battle standard, or now and then after the
//! Bone King) sends waves until its bar is empty, and the goblin tinkerer moves in afterwards.

use std::collections::HashSet;

use glam::{IVec2, IVec3, Vec3};
use voxel::chunk::CHUNK_SIZE;
use voxel::mob::Kind;
use voxel::rng::Rng;
use voxel::worldgen::{Biome, Dim};

use crate::audio::Sfx;
use crate::Game;

/// The hard mode ore pairs: (first, second, lowest y, highest y, veins per chunk, vein size).
const HARD_ORES: [(&str, &str, i32, i32, u32, u32); 3] = [
    ("cobalt_ore", "palladium_ore", 6, 60, 5, 6),
    ("mythril_ore", "orichalcum_ore", 5, 44, 4, 5),
    ("adamantite_ore", "titanium_ore", 3, 30, 3, 5),
];

/// The same ores further down in the deep world (worldgen/deep.rs), richer the deeper the tier:
/// (from y, to y, veins). Only depths already generated get them.
const DEEP_HARD_ORES: [(i32, i32, u32); 3] = [(-260, -20, 4), (-430, -160, 4), (-610, -320, 4)];

const GOBLINS: [&str; 3] = ["goblin_warrior", "goblin_warrior", "goblin_archer"];

/// A goblin invasion under way.
struct Invasion {
    /// Goblins still to come.
    to_come: u32,
    total: u32,
    spawn_t: f32,
    /// Where it was called: walk too far away and it gives up.
    at: Vec3,
}

#[derive(Default)]
pub struct HardMode {
    pub on: bool,
    /// Chunks that already got their hard mode ores (and verdite).
    pub seeded: HashSet<(i32, i32)>,
    pub verdite: HashSet<(i32, i32)>,
    pub goblins_beaten: bool,
    pub mechanic_freed: bool,
    pub(crate) blood: bool,
    was_night: bool,
    invasion: Option<Invasion>,
    spawn_t: f32,
    scan_t: f32,
}

impl HardMode {
    pub fn blood_moon(&self) -> bool {
        self.blood
    }

    pub(crate) fn invasion_left_for_test(&mut self, n: u32) {
        if let Some(i) = self.invasion.as_mut() {
            i.to_come = n;
        }
    }

    pub fn invasion(&self) -> Option<(u32, u32)> {
        self.invasion.as_ref().map(|i| (i.to_come, i.total))
    }
}

fn chunk_seed(seed: u64, cp: IVec2, salt: u64) -> u64 {
    seed ^ (cp.x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (cp.y as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ salt
}

impl Game {
    fn mechs_beaten(&self) -> bool {
        ["mech_eye", "mech_worm", "mech_skull"].iter().all(|b| self.boss.defeated.iter().any(|d| d == b))
    }

    pub(crate) fn update_hardmode(&mut self, dt: f32) {
        if dt == 0.0 {
            return;
        }
        let overworld = self.dim == Dim::Overworld;
        // Day and night: a new night may rise red.
        let night = self.is_night();
        if night && !self.t5.hard.was_night && overworld {
            let chance = if self.t5.hard.on { 0.16 } else if !self.boss.defeated.is_empty() { 0.11 } else { 0.0 };
            if self.surv.rng.f32() < chance {
                self.start_blood_moon();
            }
        }
        if !night && self.t5.hard.blood {
            self.t5.hard.blood = false;
            self.toast("A lua de sangue se pôs", 3.0);
        }
        self.t5.hard.was_night = night;
        if overworld {
            self.update_night_spawns(dt, night);
            self.update_invasion(dt);
        }
        // Monsters of the World Breaker age are tougher.
        if self.t5.hard.on {
            for m in self.mobs.list.iter_mut() {
                let d = &self.mobs.reg.defs[m.def];
                if d.kind == Kind::Hostile && !d.boss && m.max_health == d.health {
                    m.max_health = d.health * 1.6;
                    m.health *= 1.6;
                }
            }
        }
        self.t5.hard.scan_t -= dt;
        if self.t5.hard.scan_t <= 0.0 && overworld && (self.t5.hard.on || self.mechs_beaten()) {
            self.t5.hard.scan_t = 0.25;
            self.seed_hard_ores();
        }
    }

    fn start_blood_moon(&mut self) {
        self.t5.hard.blood = true;
        self.weather.flash = 0.4;
        self.sfx(Sfx::Thunder(0.5));
        self.toast("A lua de sangue está nascendo...", 4.0);
    }

    /// Right click with a bloody tear: tonight the moon rises red.
    pub(crate) fn call_blood_moon(&mut self) -> bool {
        if !self.is_night() {
            self.toast("A lágrima só chama a lua à noite", 3.0);
            return true;
        }
        if self.t5.hard.blood {
            self.toast("A lua já está vermelha", 2.5);
            return true;
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.start_blood_moon();
        true
    }

    /// Right click with a goblin battle standard: the goblins come.
    pub(crate) fn call_goblins(&mut self) -> bool {
        if self.t5.hard.invasion.is_some() {
            self.toast("Os goblins já estão vindo!", 2.5);
            return true;
        }
        if self.dim != Dim::Overworld {
            self.toast("Os goblins só marcham na superfície", 2.5);
            return true;
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.start_invasion();
        true
    }

    pub(crate) fn start_invasion(&mut self) {
        let total = if self.t5.hard.on { 60 } else { 32 };
        self.t5.hard.invasion = Some(Invasion { to_come: total, total, spawn_t: 3.0, at: self.body.pos });
        self.sfx(Sfx::Thunder(0.4));
        self.toast("Um exército goblin se aproxima!", 4.0);
    }

    /// A monster of `name` somewhere 18 to 30 blocks away on the ground.
    pub(crate) fn spawn_around(&mut self, name: &str, flying: bool) -> bool {
        for _ in 0..6 {
            let a = self.surv.rng.f32() * std::f32::consts::TAU;
            let r = 18.0 + self.surv.rng.f32() * 12.0;
            let x = (self.body.pos.x + a.cos() * r).floor() as i32;
            let z = (self.body.pos.z + a.sin() * r).floor() as i32;
            let Some(y) = self.world.surface_y(x, z) else { continue };
            let y = if flying { (y + 6).max(self.body.pos.y as i32 + 4) } else { y + 1 };
            let at = Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5);
            if self.mobs.spawn(name, at) {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.aggro = true;
                }
                return true;
            }
        }
        false
    }

    fn count_alive(&self, names: &[&str]) -> usize {
        self.mobs.list.iter().filter(|m| m.alive() && names.contains(&self.mobs.reg.defs[m.def].name.as_str())).count()
    }

    /// Demon eyes on any night, and the blood moon's crowd.
    fn update_night_spawns(&mut self, dt: f32, night: bool) {
        if !night || self.mobs.peaceful || !self.mobs.enabled {
            return;
        }
        self.t5.hard.spawn_t -= dt;
        if self.t5.hard.spawn_t > 0.0 {
            return;
        }
        let blood = self.t5.hard.blood;
        self.t5.hard.spawn_t = if blood { 3.0 } else { 20.0 };
        let eyes = self.count_alive(&["demon_eye"]);
        if eyes < if blood { 4 } else { 2 } && self.surv.rng.f32() < 0.6 {
            self.spawn_around("demon_eye", true);
        }
        if blood && self.count_alive(&["blood_zombie"]) < 8 {
            self.spawn_around("blood_zombie", false);
            if self.surv.rng.f32() < 0.4 {
                self.spawn_around("zombie", false);
            }
        }
    }

    fn update_invasion(&mut self, dt: f32) {
        let Some(inv) = self.t5.hard.invasion.as_mut() else { return };
        if inv.at.distance(self.body.pos) > 200.0 {
            self.t5.hard.invasion = None;
            self.toast("Os goblins desistiram da invasão", 3.0);
            return;
        }
        inv.spawn_t -= dt;
        let ready = inv.spawn_t <= 0.0 && inv.to_come > 0;
        if ready {
            inv.spawn_t = 2.0;
        }
        let names = ["goblin_warrior", "goblin_archer", "goblin_sorcerer"];
        let alive = self.count_alive(&names);
        if ready && alive < 10 {
            let to_come = self.t5.hard.invasion.as_ref().map(|i| i.to_come).unwrap_or(0);
            let name = if to_come % 7 == 0 { "goblin_sorcerer" } else { GOBLINS[(to_come % 3) as usize] };
            if self.spawn_around(name, false) {
                if let Some(i) = self.t5.hard.invasion.as_mut() {
                    i.to_come -= 1;
                }
            }
        }
        let done = self.t5.hard.invasion.as_ref().is_some_and(|i| i.to_come == 0) && alive == 0;
        if done {
            self.t5.hard.invasion = None;
            self.t5.hard.goblins_beaten = true;
            self.sfx(Sfx::Xp);
            self.toast("A invasão goblin foi derrotada!", 5.0);
        }
    }

    /// Kills of the invasion so far, as a fraction (for its bar).
    fn invasion_progress(&self) -> Option<f32> {
        let (to_come, total) = self.t5.hard.invasion()?;
        let alive = self.count_alive(&["goblin_warrior", "goblin_archer", "goblin_sorcerer"]) as u32;
        let killed = total.saturating_sub(to_come + alive);
        Some(killed as f32 / total as f32)
    }

    /// The event bar at the top, under the boss bar if there is one.
    pub(crate) fn draw_event_bar(&mut self, w: f32) {
        let Some(frac) = self.invasion_progress() else { return };
        let s = self.ui.scale;
        let boss_up = self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].boss && m.body.pos.distance(self.body.pos) < 64.0);
        let y = if boss_up { 38.0 * s } else { 14.0 * s };
        let bw = 140.0 * s;
        let x = (w - bw) / 2.0;
        let t = "Invasão Goblin";
        let tw = self.ui.text_width(t, s);
        self.ui.text((w - tw) / 2.0, y - 10.0 * s, t, [230, 200, 140, 255]);
        self.ui.rect(x - s, y - s, bw + 2.0 * s, 6.0 * s, [0, 0, 0, 200]);
        self.ui.rect(x, y, bw, 4.0 * s, [50, 40, 30, 255]);
        self.ui.rect(x, y, bw * frac, 4.0 * s, [200, 140, 60, 255]);
    }

    /// Hard mode ores in the stone of loaded chunks near the player that don't have them yet,
    /// and verdite in jungle chunks once the mechanical bosses are beaten. A few chunks a call.
    fn seed_hard_ores(&mut self) {
        let center = IVec2::new((self.body.pos.x.floor() as i32).div_euclid(CHUNK_SIZE as i32), (self.body.pos.z.floor() as i32).div_euclid(CHUNK_SIZE as i32));
        let verdite_on = self.mechs_beaten();
        let mut done = 0;
        for r in 0..=5i32 {
            for dx in -r..=r {
                for dz in -r..=r {
                    if dx.abs() != r && dz.abs() != r {
                        continue;
                    }
                    let cp = center + IVec2::new(dx, dz);
                    let key = (cp.x, cp.y);
                    if self.world.chunk(cp).is_none() {
                        continue;
                    }
                    // Only chunks with all their neighbours loaded, so veins never fall off.
                    if (0..9).any(|i| self.world.chunk(cp + IVec2::new(i % 3 - 1, i / 3 - 1)).is_none()) {
                        continue;
                    }
                    if self.t5.hard.on && !self.t5.hard.seeded.contains(&key) {
                        self.t5.hard.seeded.insert(key);
                        self.place_hard_ores(cp);
                        done += 1;
                    }
                    if verdite_on && !self.t5.hard.verdite.contains(&key) {
                        self.t5.hard.verdite.insert(key);
                        let mid = cp * CHUNK_SIZE as i32 + IVec2::splat(8);
                        if self.world.gen.base_biome_at(mid.x, mid.y) == Biome::Jungle {
                            self.place_veins(cp, "verdite_ore", 8, 50, 4, 6, 0x7E2D);
                            done += 1;
                        }
                    }
                    if done >= 2 {
                        return;
                    }
                }
            }
        }
    }

    fn place_hard_ores(&mut self, cp: IVec2) {
        for (i, &(a, b, lo, hi, veins, size)) in HARD_ORES.iter().enumerate() {
            let (common, rare) = if voxel::worldgen::common_metal(self.seed, 4 + i) { (a, b) } else { (b, a) };
            self.place_veins(cp, common, lo, hi, veins, size, 0x4A11 + i as u64);
            self.place_veins(cp, rare, lo, hi, (veins / 3).max(1), size.saturating_sub(2).max(3), 0x4B22 + i as u64);
            let (dlo, dhi, dv) = DEEP_HARD_ORES[i];
            self.place_veins(cp, common, dlo, dhi, dv, size + 1, 0x4C33 + i as u64);
            self.place_veins(cp, rare, dlo, dhi, dv / 2, size, 0x4D44 + i as u64);
        }
    }

    /// Veins of `ore` replacing stone (and deepslate) in chunk `cp`, the same for every world
    /// with this seed.
    fn place_veins(&mut self, cp: IVec2, ore: &str, lo: i32, hi: i32, veins: u32, size: u32, salt: u64) {
        let ore = self.reg.id_or_air(ore);
        if ore == voxel::AIR {
            return;
        }
        let hosts = [self.reg.id_or_air("stone"), self.reg.id_or_air("deepslate"), self.reg.id_or_air("andesite"), self.reg.id_or_air("tuff")];
        let mut rng = Rng::new(chunk_seed(self.seed, cp, salt));
        let base = IVec3::new(cp.x * CHUNK_SIZE as i32, 0, cp.y * CHUNK_SIZE as i32);
        for _ in 0..veins {
            let mut p = base + IVec3::new(rng.range(1, 15), rng.range(lo, hi), rng.range(1, 15));
            for _ in 0..size {
                let id = self.world.block_or_air(p);
                if id != voxel::AIR && hosts.contains(&id) {
                    self.world.set_block(p, ore);
                }
                p += [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z][rng.below(6) as usize];
            }
        }
    }

    /// The world enters hard mode (the World Breaker fell).
    pub(crate) fn enter_hard_mode(&mut self) {
        if self.t5.hard.on {
            return;
        }
        self.t5.hard.on = true;
        self.weather.flash = 1.0;
        self.sfx(Sfx::Thunder(1.0));
        self.toast("O mundo foi abalado! Novos minérios brotam na pedra e os monstros ficam mais fortes.", 7.0);
    }

    /// Wishes of the Terraria side's town folk.
    pub(crate) fn t5_npc_wish(&self, name: &str) -> bool {
        match name {
            "npc_tinkerer" => self.t5.hard.goblins_beaten,
            "npc_mechanic" => self.t5.hard.mechanic_freed,
            "npc_sorceress" => self.t5.hard.on,
            _ => false,
        }
    }
}

/// Shops of the Terraria side's town folk: (mob name, goods: item, count, price in emeralds).
pub const SHOPS: &[(&str, &[(&str, u8, u8)])] = &[
    ("npc_tinkerer", &[("running_boots", 1, 8), ("cloud_bottle", 1, 9), ("shiny_balloon", 1, 8), ("climbing_claws", 1, 6), ("lucky_horseshoe", 1, 10), ("reforge_table", 1, 6), ("grappling_hook", 1, 5), ("goblin_battle_standard", 1, 6), ("claw_glove", 1, 9), ("frog_charm", 1, 8), ("creature_radar", 1, 7), ("battle_tally", 1, 6), ("ore_finder", 1, 8), ("weather_vane", 1, 5)]),
    ("npc_mechanic", &[("redstone", 16, 1), ("repeater", 2, 1), ("piston", 2, 2), ("grappling_hook", 1, 4), ("rail", 16, 2), ("cursed_skull", 1, 8)]),
    ("npc_sorceress", &[("parchment", 8, 1), ("glyph_book", 1, 6), ("mana_crystal", 1, 6), ("void_tome", 1, 18), ("ice_spike_staff", 1, 14), ("star_tome", 1, 22), ("bloody_tear", 1, 4), ("greater_mana_potion", 2, 4), ("arcane_cuffs", 1, 14), ("imp_staff", 1, 16)]),
];
