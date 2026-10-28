//! Lote 21: traps, statues, teleporters, work stations, meals and critters (Terraria).
//!
//! - Traps fire on the rising edge of redstone power (a pressure plate, a lever, a button):
//!   dart traps shoot a poison dart, spear traps stab out of all four sides, flame traps
//!   breathe fire, geysers blast upward and boulder traps let loose a boulder that rolls until
//!   it hits a wall. In 3D a trap aims along the axis that points at you, so a corridor of
//!   them is a real gauntlet and a plate in front of a door is a real ambush.
//! - Statues let out one of their creature each time they get power (at most three about).
//! - Teleporters: stand on one and sneak, and you come out on the next teleporter of the same
//!   channel. The channel is the block under it, so a pair on gold blocks is one line and a
//!   pair on copper another. Power works too.
//! - The extractinator sifts gravel, sand and clay into ore, gems and coins; the keg brews ale
//!   from wheat; the cooking pot cooks raw food. Sneak to do the whole stack at once.
//! - Meals in three tiers make you Well Fed, Plenty Satisfied or Exquisitely Stuffed (the Well
//!   Fed effect at strength 1, 2, 3). Ale makes you tipsy.
//! - Critters live about (fireflies at night, butterflies by day, squirrels in woods,
//!   grasshoppers in grass, snails in the rain), with rare golden ones. Catch them with the
//!   bug net, let them go again, bottle them as lamps or put them on a hook as bait.

use std::collections::HashSet;

use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};
use voxel::physics::Terrain;
use voxel::worldgen::Biome;

use crate::audio::Sfx;
use crate::Game;

const TRAPS: &[&str] = &["dart_trap", "spear_trap", "flame_trap", "geyser_trap", "boulder_trap"];
/// (statue, the creature it lets out)
const STATUES: &[(&str, &str)] = &[
    ("statue_jelly", "jelly_blue"),
    ("statue_bat", "cave_bat"),
    ("statue_skeleton", "skeleton"),
    ("statue_rabbit", "rabbit"),
    ("statue_butterfly", "critter_butterfly"),
];
const STATIONS: &[&str] = &["extractinator", "keg", "cooking_pot"];
/// Meals: (food, tier 1..3).
const MEALS: &[(&str, u8)] = &[
    ("bowl_of_soup", 1),
    ("grilled_squirrel", 1),
    ("fried_egg", 1),
    ("bunny_stew", 2),
    ("seafood_dinner", 2),
    ("pad_thai", 2),
    ("golden_delight", 3),
    ("lobster_tail", 3),
    ("grape_juice", 3),
];
/// Raw food the cooking pot cooks.
const COOK: &[(&str, &str)] = &[
    ("beef", "cooked_beef"),
    ("porkchop", "cooked_porkchop"),
    ("chicken", "cooked_chicken"),
    ("mutton", "cooked_mutton"),
    ("rabbit", "cooked_rabbit"),
    ("cod", "cooked_cod"),
    ("salmon", "cooked_salmon"),
    ("potato", "baked_potato"),
    ("kelp", "dried_kelp"),
];
/// What the extractinator finds: (item, weight, min, max).
const SIFT: &[(&str, u32, u8, u8)] = &[
    ("copper_coin", 300, 3, 30),
    ("silver_coin", 60, 1, 3),
    ("gold_coin", 3, 1, 1),
    ("raw_copper", 120, 1, 4),
    ("raw_iron", 100, 1, 3),
    ("raw_gold", 40, 1, 2),
    ("coal", 120, 1, 3),
    ("flint", 80, 1, 2),
    ("lapis_lazuli", 40, 1, 3),
    ("emerald", 12, 1, 1),
    ("ruby", 10, 1, 1),
    ("sapphire", 10, 1, 1),
    ("topaz", 10, 1, 1),
    ("diamond", 5, 1, 1),
];
/// Critters: (mob, item you catch, gold variant).
const CRITTERS: &[(&str, &str)] = &[
    ("critter_firefly", "critter_firefly"),
    ("critter_butterfly", "critter_butterfly"),
    ("critter_gold_butterfly", "critter_gold_butterfly"),
    ("critter_grasshopper", "critter_grasshopper"),
    ("critter_squirrel", "critter_squirrel"),
    ("critter_gold_squirrel", "critter_gold_squirrel"),
    ("critter_snail", "critter_snail"),
];
const MOST_CRITTERS: usize = 8;

pub(crate) fn is_use_block(name: &str) -> bool {
    STATIONS.contains(&name) || name == "fate_altar" || name.ends_with("_monolith")
}

fn kind_of(name: &str) -> Option<u8> {
    if let Some(k) = TRAPS.iter().position(|&t| t == name) {
        return Some(k as u8);
    }
    if let Some(k) = STATUES.iter().position(|s| s.0 == name) {
        return Some(10 + k as u8);
    }
    if let Some(k) = ["desert_fountain", "snow_fountain", "jungle_fountain", "ocean_fountain", "evil_fountain"].iter().position(|&f| f == name) {
        return Some(30 + k as u8);
    }
    if let Some(k) = ["solar_monolith", "vortex_monolith", "nebula_monolith", "stardust_monolith"].iter().position(|&f| f == name) {
        return Some(40 + k as u8);
    }
    match name {
        "teleporter" => Some(20),
        "garden_gnome" => Some(50),
        _ => None,
    }
}

#[derive(Default)]
pub struct Traps {
    /// Traps, statues and teleporters put down: where and which (0.. traps, 10.. statues, 20
    /// teleporter).
    pub(crate) placed: Vec<([i32; 3], u8)>,
    /// The ones that had power at the last look.
    on: HashSet<IVec3>,
    scan_t: f32,
    /// A flame trap breathing: (where, toward, seconds left).
    flames: Vec<(Vec3, Vec3, f32)>,
    tele_cd: f32,
    critter_t: f32,
    net_cd: f32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SavedTraps {
    #[serde(default)]
    pub placed: Vec<([i32; 3], u8)>,
}

impl Traps {
    pub fn load(&mut self, s: SavedTraps) {
        self.placed = s.placed;
    }

    pub fn saved(&self) -> SavedTraps {
        SavedTraps { placed: self.placed.clone() }
    }
}

fn axis_toward(from: Vec3, to: Vec3) -> Vec3 {
    let d = to - from;
    if d.x.abs() > d.z.abs() {
        Vec3::X * d.x.signum()
    } else {
        Vec3::Z * if d.z == 0.0 { 1.0 } else { d.z.signum() }
    }
}

impl Game {
    /// A block went down: remember traps, statues and teleporters.
    pub(crate) fn tr3_placed(&mut self, p: IVec3, name: &str) {
        self.world3_placed(p);
        let Some(k) = kind_of(name) else { return };
        let t = &mut self.t5.tr.r3.traps;
        t.placed.retain(|q| q.0 != p.to_array());
        t.placed.push((p.to_array(), k));
        if self.redstone.powered(&self.world, p) {
            self.t5.tr.r3.traps.on.insert(p);
        }
        if k == 20 {
            let ch = self.channel(p);
            let n = self.t5.tr.r3.traps.placed.iter().filter(|q| q.1 == 20 && self.channel(IVec3::from_array(q.0)) == ch).count();
            self.toast(format!("Teletransportador no canal \"{ch}\" ({n} no canal). Fique em cima e agache para usar."), 3.0);
        }
    }

    fn channel(&self, p: IVec3) -> String {
        self.reg.get(self.world.block_or_air(p - IVec3::Y)).name.clone()
    }

    pub(crate) fn update_traps(&mut self, dt: f32) {
        self.flames(dt);
        self.roll_boulders(dt);
        self.teleporters(dt);
        self.critters(dt);
        let t = &mut self.t5.tr.r3.traps;
        t.net_cd = (t.net_cd - dt).max(0.0);
        t.scan_t -= dt;
        if t.scan_t > 0.0 || t.placed.is_empty() {
            return;
        }
        t.scan_t = 0.1;
        let placed = t.placed.clone();
        let mut gone = Vec::new();
        for (p, k) in placed {
            let p = IVec3::from_array(p);
            // Only where the world is loaded, and gone when broken.
            let Some(id) = self.world.block(p) else { continue };
            if kind_of(&self.reg.get(id).name) != Some(k) {
                gone.push(p.to_array());
                continue;
            }
            let on = self.redstone.powered(&self.world, p);
            let was = self.t5.tr.r3.traps.on.contains(&p);
            if on == was {
                continue;
            }
            if on {
                self.t5.tr.r3.traps.on.insert(p);
                match k {
                    0..=4 => self.spring(p, k),
                    10..=19 => self.statue(p, STATUES[(k - 10) as usize].1),
                    40..=49 => self.toggle_monolith(p),
                    30..=39 | 50.. => {}
                    _ => {
                        let feet = self.body.pos.floor().as_ivec3();
                        if feet - IVec3::Y == p || feet == p {
                            self.teleport_from(p);
                        }
                    }
                }
            } else {
                self.t5.tr.r3.traps.on.remove(&p);
            }
        }
        self.t5.tr.r3.traps.placed.retain(|q| !gone.contains(&q.0));
    }

    /// A trap got power.
    fn spring(&mut self, p: IVec3, k: u8) {
        let c = p.as_vec3() + Vec3::splat(0.5);
        let me = self.body.pos + Vec3::Y * 0.9;
        let dir = axis_toward(c, me);
        let glow = self.glow_layer();
        match TRAPS[k as usize] {
            "dart_trap" => {
                let from = c + dir * 0.6;
                self.enemy_bolt_for(from, dir * 26.0 + Vec3::Y * 0.4, 4, 9.0, 1.2);
                self.trap_line(from, dir, 30.0, 12.0, Some(crate::effects::POISON));
                self.sfx_at(Sfx::Click, c, 0.8, 24.0);
            }
            "spear_trap" => {
                // Out of all four sides, three blocks long.
                for d in [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z] {
                    let from = c + d * 0.5;
                    self.trap_line(from, d, 3.0, 20.0, None);
                    let dm = me - from;
                    let along = dm.dot(d);
                    if (0.0..3.0).contains(&along) && (dm - d * along).length() < 0.9 {
                        self.damage(14.0, Some(c));
                    }
                    for i in 0..6 {
                        self.particles.glow(from + d * (i as f32 * 0.5), d * 0.3, glow, [0.8, 0.7, 0.55], 0.05, 0.25);
                    }
                }
                self.sfx_at(Sfx::Piston(true), c, 0.9, 20.0);
            }
            "flame_trap" => {
                self.t5.tr.r3.traps.flames.push((c + dir * 0.6, dir, 1.6));
                self.sfx_at(Sfx::Fizz, c, 0.8, 24.0);
            }
            "geyser_trap" => {
                let top = c + Vec3::Y * 0.6;
                let d = me - top;
                if Vec3::new(d.x, 0.0, d.z).length() < 1.1 && d.y > -0.5 && d.y < 8.0 {
                    self.body.vel.y = self.body.vel.y.max(17.0);
                    self.body.fall_distance = 0.0;
                    self.damage(6.0, Some(top));
                }
                for m in self.mobs.list.iter_mut() {
                    let d = m.body.pos - top;
                    if m.alive() && Vec3::new(d.x, 0.0, d.z).length() < 1.1 && d.y > -0.5 && d.y < 8.0 {
                        m.body.vel.y = 15.0;
                        m.hurt(10.0, top);
                    }
                }
                for i in 0..40 {
                    let a = i as f32 * 2.4;
                    let v = Vec3::new(a.cos() * 0.6, 9.0 + (i % 7) as f32, a.sin() * 0.6);
                    self.particles.glow(top, v, glow, [1.2, 1.2, 1.2], 0.09, 0.8);
                }
                self.sfx_at(Sfx::Splash(0.6), top, 1.0, 24.0);
            }
            _ => {
                let from = c + dir * 1.0 - Vec3::Y * 0.5;
                if !self.world.solid((from + Vec3::Y * 0.5).floor().as_ivec3()) && self.mobs.spawn("rolling_boulder", from) {
                    if let Some(m) = self.mobs.list.last_mut() {
                        m.prey = Some(dir);
                        m.yaw = dir.x.atan2(dir.z);
                    }
                    self.sfx_at(Sfx::Break(crate::audio::Material::Stone), c, 1.0, 30.0);
                }
            }
        }
    }

    /// Hurts the first mob along a line out of a trap.
    fn trap_line(&mut self, from: Vec3, dir: Vec3, reach: f32, dmg: f32, debuff: Option<u8>) {
        let mut best: Option<(usize, f32)> = None;
        for (k, m) in self.mobs.list.iter().enumerate() {
            if !m.alive() || m.ghost {
                continue;
            }
            let d = m.body.center() - from;
            let along = d.dot(dir);
            if along > 0.0 && along < reach && (d - dir * along).length() < (m.body.half_width * 2.0).max(0.5) && best.is_none_or(|b| along < b.1) {
                best = Some((k, along));
            }
        }
        let Some((k, along)) = best else { return };
        if !(1..(along * 2.0) as i32).all(|i| !self.world.solid((from + dir * (i as f32 * 0.5)).floor().as_ivec3())) {
            return;
        }
        let m = &mut self.mobs.list[k];
        m.hurt(dmg, from);
        if let Some(e) = debuff {
            m.debuffs.push((e, 6.0));
        }
    }

    /// Flame traps breathe fire for a moment.
    fn flames(&mut self, dt: f32) {
        if self.t5.tr.r3.traps.flames.is_empty() {
            return;
        }
        let glow = self.glow_layer();
        let mut fl = std::mem::take(&mut self.t5.tr.r3.traps.flames);
        for f in fl.iter_mut() {
            f.2 -= dt;
            let (at, dir) = (f.0, f.1);
            for i in 0..3 {
                let side = Vec3::new(dir.z, 0.0, -dir.x) * ((self.now * 37.0 + i as f32).sin() * 0.8);
                self.particles.glow(at, dir * 9.0 + side + Vec3::Y * 0.6, glow, [2.2, 1.0, 0.25], 0.12, 0.6);
            }
            if (self.now / 0.12).fract() < dt / 0.12 {
                self.enemy_bolt_for(at, dir * 10.0, 0, 3.0, 0.55);
                self.trap_line(at, dir, 5.5, 4.0, None);
                for m in self.mobs.list.iter_mut() {
                    let d = m.body.center() - at;
                    let along = d.dot(dir);
                    if m.alive() && along > 0.0 && along < 5.5 && (d - dir * along).length() < 1.0 {
                        m.on_fire = m.on_fire.max(4.0);
                    }
                }
            }
        }
        fl.retain(|f| f.2 > 0.0);
        self.t5.tr.r3.traps.flames.extend(fl);
    }

    /// Boulders roll straight on till a wall stops them, crushing what they meet.
    fn roll_boulders(&mut self, dt: f32) {
        let me = self.body.pos;
        let mut hits_me = false;
        let mut crush = Vec::new();
        for (k, m) in self.mobs.list.iter_mut().enumerate() {
            if !m.alive() || self.mobs.reg.defs[m.def].name != "rolling_boulder" {
                continue;
            }
            let dir = m.prey.unwrap_or(Vec3::X);
            let ahead = (m.body.pos + dir * 0.9 + Vec3::Y * 0.4).floor().as_ivec3();
            if self.world.solid(ahead) && self.world.solid(ahead + IVec3::Y) || m.age > 14.0 {
                m.counted = true;
                m.health = 0.0;
                continue;
            }
            m.body.vel.x = dir.x * 9.0;
            m.body.vel.z = dir.z * 9.0;
            m.head_pitch -= dt * 9.0;
            m.walk += dt * 9.0;
            let c = m.body.pos;
            if (me - c).length() < 1.3 {
                hits_me = true;
            }
            crush.push((k, c));
        }
        if hits_me && self.t5.tr.r3.traps.net_cd <= 0.0 {
            self.damage(24.0, Some(me - Vec3::X));
            self.t5.tr.r3.traps.net_cd = 0.8;
        }
        for (k, c) in crush {
            for (j, m) in self.mobs.list.iter_mut().enumerate() {
                if j != k && m.alive() && !m.ghost && (m.body.pos - c).length() < 1.3 {
                    m.hurt(40.0 * dt * 4.0, c);
                }
            }
        }
    }

    /// A statue got power: one of its creature, if there aren't three about already.
    fn statue(&mut self, p: IVec3, mob: &str) {
        let c = p.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
        let near = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == mob && m.body.pos.distance(c) < 24.0).count();
        if near >= 3 {
            return;
        }
        for d in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z, IVec3::Y] {
            let q = p + d;
            if !self.world.solid(q) && !self.world.solid(q + IVec3::Y) && self.mobs.spawn(mob, q.as_vec3() + Vec3::new(0.5, 0.0, 0.5)) {
                let glow = self.glow_layer();
                for i in 0..12 {
                    let a = i as f32 * 0.52;
                    self.particles.glow(c + Vec3::Y * 0.8, Vec3::new(a.cos(), 1.0, a.sin()) * 1.5, glow, [1.0, 1.0, 1.4], 0.06, 0.5);
                }
                return;
            }
        }
    }

    /// Sneaking on a teleporter takes you to the next one of its channel.
    fn teleporters(&mut self, dt: f32) {
        let t = &mut self.t5.tr.r3.traps;
        t.tele_cd = (t.tele_cd - dt).max(0.0);
        if t.tele_cd > 0.0 || self.crouch < 0.5 || !self.body.on_ground {
            return;
        }
        let under = (self.body.pos - Vec3::Y * 0.3).floor().as_ivec3();
        if self.reg.get(self.world.block_or_air(under)).name == "teleporter" {
            self.teleport_from(under);
        }
    }

    fn teleport_from(&mut self, p: IVec3) {
        self.t5.tr.r3.traps.tele_cd = 1.5;
        let ch = self.channel(p);
        let line: Vec<IVec3> = self.t5.tr.r3.traps.placed.iter().filter(|q| q.1 == 20).map(|q| IVec3::from_array(q.0)).filter(|&q| self.channel(q) == ch).collect();
        let Some(i) = line.iter().position(|&q| q == p) else {
            self.t5.tr.r3.traps.placed.push((p.to_array(), 20));
            return;
        };
        if line.len() < 2 {
            self.toast(format!("Nenhum outro teletransportador no canal \"{ch}\""), 2.0);
            return;
        }
        let to = line[(i + 1) % line.len()];
        let glow = self.glow_layer();
        let from = self.body.pos;
        let dest = to.as_vec3() + Vec3::new(0.5, 1.0, 0.5);
        for i in 0..30 {
            let a = i as f32 * 0.7;
            let v = Vec3::new(a.cos() * 0.8, 2.0 + (i % 5) as f32 * 0.6, a.sin() * 0.8);
            self.particles.glow(from, v, glow, [0.6, 1.4, 2.2], 0.07, 0.6);
            self.particles.glow(dest, v, glow, [0.6, 1.4, 2.2], 0.07, 0.6);
        }
        self.body.pos = dest;
        self.body.vel = Vec3::ZERO;
        self.body.fall_distance = 0.0;
        self.sfx(Sfx::Portal);
    }

    /// Right click on a station.
    pub(crate) fn tr3_use_block(&mut self, name: &str, at: IVec3) -> bool {
        if self.world3_use_block(name, at) {
            return true;
        }
        let held = self.held_key();
        let all = self.crouch > 0.5;
        let sel = self.selected;
        let have = self.held_stack().map(|s| s.count).unwrap_or(0);
        match name {
            "extractinator" => {
                if !matches!(held.as_str(), "gravel" | "sand" | "red_sand" | "clay" | "clay_ball" | "soul_sand") {
                    self.toast("O extrator peneira cascalho, areia e argila: segure um deles", 2.0);
                    return true;
                }
                let n = if all { have } else { 1 };
                self.surv.inv.take(sel, n);
                let total: u32 = SIFT.iter().map(|s| s.1).sum();
                for _ in 0..n {
                    // Half the time there is nothing in it.
                    if self.surv.rng.f32() < 0.45 {
                        continue;
                    }
                    let mut r = (self.surv.rng.f32() * total as f32) as u32;
                    let Some(&(item, _, lo, hi)) = SIFT.iter().find(|s| {
                        if r < s.1 {
                            return true;
                        }
                        r -= s.1;
                        false
                    }) else {
                        continue;
                    };
                    let k = lo + ((hi - lo + 1) as f32 * self.surv.rng.f32()) as u8;
                    self.give_or_drop(item, k.min(hi));
                }
                self.sfx(Sfx::Hit(crate::audio::Material::Sand));
                self.anim.start_swing();
                true
            }
            "keg" => {
                if held != "wheat" || have < 3 {
                    self.toast("O barril faz cerveja com trigo: 3 trigos por caneca", 2.0);
                    return true;
                }
                let n = if all { have / 3 } else { 1 };
                self.surv.inv.take(sel, n * 3);
                self.give_or_drop("ale", n);
                self.sfx(Sfx::Bubbles);
                true
            }
            _ => {
                let Some(&(_, done)) = COOK.iter().find(|c| c.0 == held) else {
                    self.toast("O caldeirão cozinha carne, peixe, batata e alga crua", 2.0);
                    return true;
                };
                let n = if all { have } else { 1 };
                self.surv.inv.take(sel, n);
                self.give_or_drop(done, n);
                self.sfx(Sfx::Fizz);
                true
            }
        }
    }

    /// After eating: meals and ale.
    pub(crate) fn tr3_ate(&mut self, name: &str) {
        use crate::effects::*;
        if let Some(&(_, tier)) = MEALS.iter().find(|m| m.0 == name) {
            let secs = [0.0, 600.0, 720.0, 900.0][tier as usize] * self.tr3_meal_factor();
            self.add_effect(WELL_FED, tier - 1, secs, false);
            let title = ["", "Bem Alimentado", "Bem Satisfeito", "Saciado ao Extremo"][tier as usize];
            self.toast(title, 2.0);
            if name == "grape_juice" {
                self.add_effect(LUCK, 0, 600.0, false);
            }
        } else if name == "ale" {
            // Tipsy: hits harder, sways a little.
            self.add_effect(STRENGTH, 0, 240.0, false);
            self.add_effect(NAUSEA, 0, 5.0, false);
        }
    }

    /// The bug net, and letting a caught critter go.
    pub(crate) fn traps_use(&mut self, name: &str) -> Option<bool> {
        if name == "bug_net" {
            return Some(self.swing_net());
        }
        let &(mob, _) = CRITTERS.iter().find(|c| c.1 == name)?;
        let at = self.body.eye() + self.forward() * 1.2 - Vec3::Y * 0.6;
        if self.world.solid(at.floor().as_ivec3()) {
            return Some(false);
        }
        if self.mobs.spawn(mob, at) {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
            self.anim.start_swing();
        }
        Some(true)
    }

    fn swing_net(&mut self) -> bool {
        if self.t5.tr.r3.traps.net_cd > 0.0 {
            return true;
        }
        self.t5.tr.r3.traps.net_cd = 0.35;
        self.anim.start_swing();
        let eye = self.body.eye();
        let look = self.forward();
        let mut best: Option<(usize, f32)> = None;
        for (k, m) in self.mobs.list.iter().enumerate() {
            if !m.alive() {
                continue;
            }
            let name = self.mobs.reg.defs[m.def].name.as_str();
            if !CRITTERS.iter().any(|c| c.0 == name) {
                continue;
            }
            let d = m.body.center() - eye;
            let dist = d.length();
            // A wide sweep: anything within 3.5 blocks and 40 degrees of where you look.
            if dist < 3.5 && d.normalize_or_zero().dot(look) > 0.75 && best.is_none_or(|b| dist < b.1) {
                best = Some((k, dist));
            }
        }
        self.sfx(Sfx::Swim);
        let Some((k, _)) = best else { return true };
        let name = self.mobs.reg.defs[self.mobs.list[k].def].name.clone();
        let item = CRITTERS.iter().find(|c| c.0 == name).map(|c| c.1).unwrap_or("critter_butterfly");
        self.mobs.list.remove(k);
        self.give_or_drop(item, 1);
        self.sfx(Sfx::Pickup);
        if name.contains("gold") {
            self.toast("Um bicho dourado! Vale muito para o comerciante.", 3.0);
        }
        true
    }

    /// Critters come and go near the player.
    fn critters(&mut self, dt: f32) {
        let t = &mut self.t5.tr.r3.traps;
        t.critter_t -= dt;
        if t.critter_t > 0.0 || !self.mobs.enabled {
            return;
        }
        t.critter_t = 7.0;
        let names: Vec<&str> = CRITTERS.iter().map(|c| c.0).collect();
        let me = self.body.pos;
        // Wander off when far away.
        for m in self.mobs.list.iter_mut() {
            if m.alive() && names.contains(&self.mobs.reg.defs[m.def].name.as_str()) && m.body.pos.distance(me) > 64.0 && m.name.is_none() {
                m.counted = true;
                m.health = 0.0;
            }
        }
        if self.count_alive(&names) >= MOST_CRITTERS || self.dim != voxel::worldgen::Dim::Overworld || me.y < 40.0 {
            return;
        }
        let biome = self.world.gen.base_biome_at(me.x.floor() as i32, me.z.floor() as i32);
        use Biome::*;
        let woods = matches!(biome, Forest | BirchForest | DarkForest | FlowerForest | OldGrowthTaiga | Taiga | CherryGrove | GoldenGrove | MataAtlantica | Araucaria | PaleGarden);
        let grass = matches!(biome, Plains | SunflowerPlains | Meadow | Savanna | Cerrado | FlowerForest | CherryGrove);
        let wet = matches!(biome, Swamp | MangroveSwamp | Jungle | BambooJungle | SparseJungle | Pantanal);
        let night = self.is_night();
        let rain = self.weather.rain > 0.4;
        let mut pick: Vec<(&str, bool)> = Vec::new();
        if night && (woods || grass || wet) && !rain {
            pick.extend([("critter_firefly", true); 3]);
        }
        if !night && (woods || grass) && !rain {
            pick.push(("critter_butterfly", true));
        }
        if !night && woods {
            pick.push(("critter_squirrel", false));
        }
        if !night && grass {
            pick.push(("critter_grasshopper", false));
        }
        if rain || wet {
            pick.push(("critter_snail", false));
        }
        if pick.is_empty() {
            return;
        }
        let (mut mob, flies) = pick[(self.surv.rng.f32() * pick.len() as f32) as usize % pick.len()];
        // One in a hundred and fifty is golden.
        if self.surv.rng.f32() < 1.0 / 150.0 {
            mob = match mob {
                "critter_butterfly" => "critter_gold_butterfly",
                "critter_squirrel" => "critter_gold_squirrel",
                m => m,
            };
        }
        let a = self.surv.rng.f32() * std::f32::consts::TAU;
        let r = 10.0 + self.surv.rng.f32() * 14.0;
        let (x, z) = ((me.x + a.cos() * r).floor() as i32, (me.z + a.sin() * r).floor() as i32);
        let Some(y) = self.world.surface_y(x, z) else { return };
        let at = Vec3::new(x as f32 + 0.5, y as f32 + if flies { 2.0 } else { 1.0 }, z as f32 + 0.5);
        if self.world.is_liquid(self.world.block_or_air(IVec3::new(x, y, z))) {
            return;
        }
        self.mobs.spawn(mob, at);
        if mob == "critter_firefly" {
            for _ in 0..3 {
                let off = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5) * 4.0;
                self.mobs.spawn(mob, at + off);
            }
        }
    }

    /// Fireflies blink.
    pub(crate) fn critter_glow(&mut self) {
        let glow = self.glow_layer();
        let now = self.now;
        let me = self.body.pos;
        let mut at = Vec::new();
        for (k, m) in self.mobs.list.iter().enumerate() {
            if m.alive() && self.mobs.reg.defs[m.def].name == "critter_firefly" && m.body.pos.distance(me) < 48.0 && (now * 0.7 + k as f32 * 0.37).fract() < 0.45 {
                at.push(m.body.center());
            }
        }
        for c in at {
            self.particles.glow(c, Vec3::ZERO, glow, [1.6, 2.2, 0.5], 0.09, 0.08);
        }
    }

    /// Test scenes: MC_TR=traps (traps, statues and stations in rows ahead) and critters.
    pub(crate) fn test_traps(&mut self, scene: &str) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let spot = |g: &Game, d: f32, s: f32| {
            let p = g.body.pos + flat * d + side * s;
            IVec3::new(p.x.floor() as i32, g.body.pos.y.floor() as i32, p.z.floor() as i32)
        };
        let set = |g: &mut Game, p: IVec3, n: &str| {
            if let Some(id) = g.reg.id(n) {
                g.world.set_block(p - IVec3::Y, g.reg.id_or_air("stone"));
                for up in 1..4 {
                    g.world.set_block(p + IVec3::Y * up, voxel::AIR);
                }
                g.world.set_block(p, id);
                g.tr3_placed(p, n);
            }
        };
        match scene {
            "traps" => {
                for (i, t) in TRAPS.iter().enumerate() {
                    let p = spot(self, 7.0, -6.0 + i as f32 * 3.0);
                    set(self, p, t);
                }
                for (i, s) in STATUES.iter().enumerate() {
                    let p = spot(self, 10.0, -6.0 + i as f32 * 3.0);
                    set(self, p, s.0);
                }
                for (i, s) in ["extractinator", "keg", "cooking_pot", "teleporter", "firefly_jar", "butterfly_jar"].iter().enumerate() {
                    let p = spot(self, 4.0, -5.0 + i as f32 * 2.0);
                    set(self, p, s);
                }
                // Fire the traps at once, as if a plate was stepped on.
                let placed = self.t5.tr.r3.traps.placed.clone();
                for (p, k) in placed {
                    if k < 10 {
                        self.spring(IVec3::from_array(p), k);
                    }
                }
            }
            _ => {
                for (i, c) in CRITTERS.iter().enumerate() {
                    let p = spot(self, 4.0, -3.0 + i as f32 * 1.0).as_vec3() + Vec3::new(0.5, if c.0.contains("fly") { 1.0 } else { 0.0 }, 0.5);
                    self.mobs.spawn(c.0, p);
                    if let Some(m) = self.mobs.list.last_mut() {
                        m.still = true;
                        m.yaw = (-flat.x).atan2(-flat.z);
                    }
                }
                let p = spot(self, 7.0, 3.0).as_vec3() + Vec3::new(0.5, 0.0, 0.5);
                self.mobs.spawn("rolling_boulder", p);
                if let Some(m) = self.mobs.list.last_mut() {
                    m.still = true;
                }
            }
        }
    }
}
