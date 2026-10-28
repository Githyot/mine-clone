//! Life on the volcanoes (Lote 14; the cones themselves are worldgen/volcano.rs). Made for a
//! world you climb in first person:
//! - fumaroles hiss steam and throw whoever stands on them high into the air (a jump pad up the
//!   cone; landing from it doesn't hurt);
//! - now and then the volcano near you erupts: the ground shakes, a column of smoke rises from
//!   the crater, ash falls and lava bombs fly out in arcs. Each bomb's landing spot glows on the
//!   ground first, so you can see it coming and step aside;
//! - its creatures: the Magma Salamander (lives by the lava, leaves you alone until hit), the
//!   Basalt Golem (curls into a boulder and rolls down the slope at you, then gets up again) and
//!   the Ember Wisp (floats and throws fire);
//! - the Magma Titan, woken in the crater with an Obsidian Idol: it wades through the lava lake
//!   towards you, hurls bombs, slams shockwaves along the rim and calls golems that roll down
//!   into the crater; at half health the lava rises, and when it falls the lava that rose cools
//!   into obsidian.

use glam::{IVec3, Vec3};
use voxel::physics::{Input, Terrain};
use voxel::worldgen::{Biome, Dim, Volcano, ASH_REACH};
use voxel::{BlockId, AIR};

use crate::audio::Sfx;
use crate::Game;

/// Seconds an eruption lasts, and the quiet between two (random within the range).
const ERUPT_SECS: f32 = 24.0;
const QUIET: (f32, f32) = (240.0, 420.0);
/// Gravity of a lava bomb (a little floatier than the player's, so arcs read from below).
const BOMB_G: f32 = 20.0;
/// Upward speed a fumarole gives (about twelve blocks up).
const VENT_PUSH: f32 = 27.0;
/// The creatures of this file.
const OURS: [&str; 4] = ["magma_salamander", "basalt_golem", "basalt_golem_ball", "ember_wisp"];
pub const TITAN: &str = "magma_titan";

/// Bosses this file fights (bosses.rs leaves them alone).
pub fn own_ai(name: &str) -> bool {
    name == TITAN
}

struct Bomb {
    pos: Vec3,
    vel: Vec3,
    dmg: f32,
}

struct Roll {
    id: u32,
    t: f32,
    speed: f32,
    slow: f32,
}

#[derive(Default)]
struct Titan {
    t: f32,
    bomb_t: f32,
    slam_t: f32,
    windup: f32,
    call_t: f32,
    rise_t: f32,
    risen: i32,
    angry: bool,
    contact_t: f32,
    /// The lava it raised, to cool into obsidian when it falls.
    raised: Vec<IVec3>,
}

#[derive(Default)]
pub struct Vulcao {
    near: Option<Volcano>,
    look_t: f32,
    vents: Vec<IVec3>,
    vent_t: f32,
    steam_t: f32,
    /// Seconds left of a steam launch (falls don't hurt meanwhile).
    launched: f32,
    /// How thick the ash in the air is (0..1, read by the mist).
    pub haze: f32,
    erupt: f32,
    next: f32,
    warned: bool,
    bomb_t: f32,
    smoke_t: f32,
    bombs: Vec<Bomb>,
    spawn_t: f32,
    rolls: Vec<Roll>,
    roll_cd: f32,
    titan: Titan,
    fumarole: Option<BlockId>,
    /// Test scene: wake the titan once the crater has loaded.
    titan_test: bool,
}

impl Game {
    fn fumarole_id(&mut self) -> BlockId {
        if self.vulcao.fumarole.is_none() {
            self.vulcao.fumarole = Some(self.reg.id_or_air("fumarole"));
        }
        self.vulcao.fumarole.unwrap_or(AIR)
    }

    pub(crate) fn update_vulcao(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let me = self.body.pos;
        self.vulcao.look_t -= dt;
        if self.vulcao.look_t <= 0.0 {
            self.vulcao.look_t = 1.0;
            self.vulcao.near = if self.dim == Dim::Overworld {
                self.world.gen.volcano_at(me.x.floor() as i32, me.z.floor() as i32).filter(|v| (me.x - v.cx).hypot(me.z - v.cz) < v.radius * ASH_REACH + 40.0)
            } else {
                None
            };
        }
        if self.vulcao.titan_test && self.vulcao.near.is_some_and(|v| self.world.surface_y(v.cx as i32, v.cz as i32).is_some_and(|y| y > 0)) {
            self.vulcao.titan_test = false;
            self.use_obsidian_idol();
        }
        self.vulcao_vents(dt);
        self.vulcao_eruption(dt);
        self.vulcao_bombs(dt);
        self.vulcao_spawns(dt);
        self.vulcao_rolls(dt);
        self.vulcao_titan(dt);
        // Test pictures freeze particles; let an eruption's smoke and bombs move so they show.
        if self.args.screenshot.is_some() && self.vulcao.erupt > 0.0 {
            self.particles.update(&self.world, dt);
        }
        // Ash in the air: thick in an eruption, a light haze over the ash fields.
        let erupting = self.vulcao.erupt > 0.0 && self.vulcao.near.is_some_and(|v| (me.x - v.cx).hypot(me.z - v.cz) < v.radius * 1.8);
        let biome = if self.dim == Dim::Overworld { self.world.gen.base_biome_at(me.x.floor() as i32, me.z.floor() as i32) } else { Biome::Plains };
        let target = if erupting {
            0.8
        } else if matches!(biome, Biome::Volcano | Biome::AshFields) {
            0.22
        } else {
            0.0
        };
        self.vulcao.haze += (target - self.vulcao.haze).clamp(-dt * 0.2, dt * 0.2);
    }

    // ---------------------------------------------------------------------------- fumaroles

    fn vulcao_vents(&mut self, dt: f32) {
        let fum = self.fumarole_id();
        if fum == AIR {
            return;
        }
        let me = self.body.pos;
        self.vulcao.vent_t -= dt;
        if self.vulcao.vent_t <= 0.0 {
            self.vulcao.vent_t = 1.0;
            self.vulcao.vents.clear();
            if self.vulcao.near.is_some() {
                let c = me.floor().as_ivec3();
                for dz in -16..=16 {
                    for dx in -16..=16 {
                        let (x, z) = (c.x + dx, c.z + dz);
                        if let Some(y) = self.world.surface_y(x, z) {
                            if self.world.block_or_air(IVec3::new(x, y, z)) == fum {
                                self.vulcao.vents.push(IVec3::new(x, y, z));
                            }
                        }
                    }
                }
            }
        }
        // Steam puffing out of every vent in sight; a vent under the player's feet counts even
        // where no volcano is (players build jump pads of them).
        let feet = (me - Vec3::Y * 0.2).floor().as_ivec3();
        let under = (0..3).map(|k| feet - IVec3::Y * k).find(|p| self.world.block_or_air(*p) == fum && (1..=k_free(*p, feet)).all(|j| !self.world.solid(*p + IVec3::Y * j)));
        self.vulcao.steam_t -= dt;
        if self.vulcao.steam_t <= 0.0 {
            self.vulcao.steam_t = 0.12;
            let (layer, _) = self.block_look(self.reg.id_or_air("white_wool"));
            let mut vents = self.vulcao.vents.clone();
            vents.extend(under);
            for v in vents {
                if (v.as_vec3() - me).length() > 24.0 {
                    continue;
                }
                let r = &mut self.surv.rng;
                let at = v.as_vec3() + Vec3::new(0.3 + r.f32() * 0.4, 1.05, 0.3 + r.f32() * 0.4);
                let size = 0.07 + r.f32() * 0.05;
                self.particles.mote(at, layer, [0.86, 0.86, 0.84], size, -2.6, 1.8);
            }
        }
        if let Some(v) = under {
            if self.body.vel.y <= 2.0 && self.screen == crate::Screen::Playing {
                let push = if self.wears("magma_heart") { VENT_PUSH * 1.2 } else { VENT_PUSH };
                self.body.vel.y = push;
                self.body.on_ground = false;
                self.vulcao.launched = 3.0;
                self.sfx(Sfx::Fizz);
                let (layer, _) = self.block_look(self.reg.id_or_air("white_wool"));
                for _ in 0..24 {
                    let r = &mut self.surv.rng;
                    let at = v.as_vec3() + Vec3::new(r.f32(), 1.0 + r.f32() * 2.0, r.f32());
                    self.particles.mote(at, layer, [0.9, 0.9, 0.88], 0.1 + r.f32() * 0.06, -5.0, 1.2);
                }
            }
        }
        // Steam carries you: the landing after a launch doesn't hurt.
        if self.vulcao.launched > 0.0 {
            self.vulcao.launched -= dt;
            self.body.fall_distance = self.body.fall_distance.min(2.0);
            if self.body.on_ground && self.vulcao.launched < 2.6 {
                self.vulcao.launched = 0.0;
            }
        }
        // Mobs standing on a vent go up too.
        for m in self.mobs.list.iter_mut() {
            if !m.alive() || m.still || m.ghost || (m.body.pos - me).length() > 32.0 {
                continue;
            }
            let p = (m.body.pos - Vec3::Y * 0.2).floor().as_ivec3();
            if m.body.on_ground && self.world.block_or_air(p) == fum {
                m.body.vel.y = VENT_PUSH * 0.8;
                m.body.on_ground = false;
            }
        }
    }

    // ---------------------------------------------------------------------------- eruptions

    fn vulcao_eruption(&mut self, dt: f32) {
        let Some(v) = self.vulcao.near else {
            self.vulcao.erupt = 0.0;
            return;
        };
        let me = self.body.pos;
        let dist = (me.x - v.cx).hypot(me.z - v.cz);
        let (cx, cy, cz) = v.crater_top();
        let crater = Vec3::new(cx, cy, cz);
        if self.vulcao.next <= 0.0 {
            // First time near this volcano: it wakes soon, so a visit sees it.
            self.vulcao.next = 45.0 + self.surv.rng.f32() * 60.0;
        }
        if self.vulcao.erupt <= 0.0 {
            // A boss fight keeps it from erupting on top of the titan's own bombs.
            if dist < v.radius * 1.5 && !self.titan_awake() {
                self.vulcao.next -= dt;
            }
            if self.vulcao.next < 5.0 && !self.vulcao.warned && dist < v.radius * 1.5 {
                self.vulcao.warned = true;
                self.toast("O chão treme... o vulcão vai entrar em erupção!", 4.0);
                self.sfx(Sfx::Thunder(0.6));
            }
            if self.vulcao.next < 5.0 {
                self.fx.shake = self.fx.shake.max(0.25);
            }
            if self.vulcao.next <= 0.0 {
                self.start_eruption();
            }
            return;
        }
        self.vulcao.erupt -= dt;
        let left = self.vulcao.erupt;
        if left <= 0.0 {
            self.vulcao.next = QUIET.0 + self.surv.rng.f32() * (QUIET.1 - QUIET.0);
            self.vulcao.warned = false;
            self.toast("A erupção acabou", 3.0);
            return;
        }
        // The ground shakes most in the first seconds, more the closer you are.
        let close = (1.0 - dist / (v.radius * 1.8)).clamp(0.0, 1.0);
        self.fx.shake = self.fx.shake.max(close * if left > ERUPT_SECS - 4.0 { 0.7 } else { 0.3 });
        // A column of smoke and embers over the crater.
        self.vulcao.smoke_t -= dt;
        if self.vulcao.smoke_t <= 0.0 && dist < 260.0 {
            self.vulcao.smoke_t = 0.05;
            let (layer, _) = self.block_look(self.reg.id_or_air("ash_block"));
            let glow = self.glow_layer();
            // Big slow billows, dark at the bottom, paler as they climb (seen from far off).
            for _ in 0..3 {
                let r = &mut self.surv.rng;
                let off = Vec3::new(r.f32() - 0.5, 1.0 + r.f32() * 0.5, r.f32() - 0.5) * v.crater * 0.6;
                let size = 2.5 + r.f32() * 3.0;
                let rise = 5.0 + r.f32() * 5.0;
                let grey = 0.22 + r.f32() * 0.18;
                self.particles.mote(crater + off, layer, [grey, grey * 0.96, grey * 0.94], size, -rise, 9.0);
            }
            let r = &mut self.surv.rng;
            let e = crater + Vec3::new(r.f32() - 0.5, 0.0, r.f32() - 0.5) * v.crater;
            let up = Vec3::new(r.f32() - 0.5, 4.0 + r.f32() * 6.0, r.f32() - 0.5);
            self.particles.glow(e, up, glow, [1.8, 0.7, 0.2], 0.14, 2.0);
        }
        // Ash falling around the player.
        if dist < v.radius * 1.8 {
            let (layer, _) = self.block_look(self.reg.id_or_air("ash_block"));
            // About 50 flakes a second, whatever the frame rate.
            let n = (dt * 50.0 + self.surv.rng.f32()) as usize;
            for _ in 0..n {
                let r = &mut self.surv.rng;
                let at = me + Vec3::new(r.f32() * 24.0 - 12.0, 6.0 + r.f32() * 8.0, r.f32() * 24.0 - 12.0);
                self.particles.mote(at, layer, [0.66, 0.64, 0.62], 0.05, 1.2, 6.0);
            }
        }
        // Bombs: most at the player when on the cone, some anywhere on the slopes.
        self.vulcao.bomb_t -= dt;
        if self.vulcao.bomb_t <= 0.0 && dist < v.radius * 1.4 {
            self.vulcao.bomb_t = 0.7 + self.surv.rng.f32() * 0.6;
            let r = &mut self.surv.rng;
            let aimed = r.chance(0.55);
            let target = if aimed {
                me + Vec3::new(r.f32() * 8.0 - 4.0, 0.0, r.f32() * 8.0 - 4.0)
            } else {
                let a = r.f32() * std::f32::consts::TAU;
                let d = v.crater + r.f32() * v.radius * 0.9;
                Vec3::new(v.cx + a.cos() * d, me.y, v.cz + a.sin() * d)
            };
            let from = crater + Vec3::Y * 2.0;
            self.throw_bomb(from, target, 3.2, 7.0);
        }
    }

    fn start_eruption(&mut self) {
        self.vulcao.erupt = ERUPT_SECS;
        self.vulcao.bomb_t = 2.0;
        self.toast("Erupção! Fuja das marcas no chão", 4.0);
        self.weather.flash = 0.3;
        self.sfx(Sfx::Explode);
        self.sfx(Sfx::Thunder(0.9));
    }

    /// Throw a lava bomb from `from` to land at `target` (x, z; its ground) after `secs`. Its
    /// landing spot glows on the ground until it lands.
    pub(crate) fn throw_bomb(&mut self, from: Vec3, target: Vec3, secs: f32, dmg: f32) {
        let ground = self.ground_top(target);
        let to = Vec3::new(target.x, ground, target.z);
        let vel = (to - from) / secs + Vec3::Y * 0.5 * BOMB_G * secs;
        self.vulcao.bombs.push(Bomb { pos: from, vel, dmg });
        self.ground_mark(to + Vec3::Y * 0.05, secs, [1.9, 0.6, 0.2]);
    }

    /// The top of the ground at a spot (the first solid block down from a little above it).
    fn ground_top(&self, p: Vec3) -> f32 {
        let c = IVec3::new(p.x.floor() as i32, p.y.floor() as i32 + 12, p.z.floor() as i32);
        (0..48).map(|k| c - IVec3::Y * k).find(|q| self.world.solid(*q) || self.world.fluid(self.world.block_or_air(*q)) == 2).map(|q| q.y as f32 + 1.0).unwrap_or(p.y)
    }

    fn vulcao_bombs(&mut self, dt: f32) {
        if self.vulcao.bombs.is_empty() {
            return;
        }
        let glow = self.glow_layer();
        let mut bombs = std::mem::take(&mut self.vulcao.bombs);
        let mut landed = Vec::new();
        bombs.retain_mut(|b| {
            b.vel.y -= BOMB_G * dt;
            let next = b.pos + b.vel * dt;
            let cell = next.floor().as_ivec3();
            let hit = Terrain::solid(&self.world, cell) || (b.vel.y < 0.0 && self.world.fluid(self.world.block_or_air(cell)) == 2 && next.y < b.pos.y);
            b.pos = next;
            if hit || b.pos.y < -64.0 {
                landed.push((b.pos, b.dmg));
                return false;
            }
            true
        });
        for b in &bombs {
            let r = &mut self.surv.rng;
            let back = -b.vel.normalize_or_zero() * 0.3;
            self.particles.glow(b.pos + back, Vec3::new(r.f32() - 0.5, r.f32(), r.f32() - 0.5), glow, [1.9, 0.7, 0.2], 0.3, 0.6);
            self.particles.glow(b.pos, Vec3::ZERO, glow, [2.0, 1.0, 0.35], 0.7, 0.06);
        }
        self.vulcao.bombs = bombs;
        for (at, dmg) in landed {
            self.bomb_lands(at, dmg);
        }
    }

    fn bomb_lands(&mut self, at: Vec3, dmg: f32) {
        let glow = self.glow_layer();
        for _ in 0..18 {
            let r = &mut self.surv.rng;
            let v = Vec3::new(r.f32() - 0.5, r.f32() * 1.2, r.f32() - 0.5) * 9.0;
            self.particles.glow(at + Vec3::Y * 0.5, v, glow, [1.9, 0.75, 0.25], 0.16, 0.8);
        }
        let me = self.body.pos + Vec3::Y * 0.9;
        let d = me.distance(at);
        if d < 12.0 {
            self.sfx(Sfx::Explode);
            self.fx.shake = self.fx.shake.max(0.5 * (1.0 - d / 12.0));
        }
        if d < 3.2 {
            let cloak = if self.wears("salamander_cloak") { 0.5 } else { 1.0 };
            self.damage(dmg * (1.0 - d / 4.5) * cloak, Some(at));
            if !self.wears("magma_heart") {
                let push = (me - at).normalize_or_zero() * 7.0 + Vec3::Y * 5.0;
                self.body.vel += push;
            }
        }
        for i in 0..self.mobs.list.len() {
            let m = &self.mobs.list[i];
            if !m.alive() || m.ghost || OURS.contains(&self.mobs.reg.defs[m.def].name.as_str()) || own_ai(&self.mobs.reg.defs[m.def].name) {
                continue;
            }
            if m.body.center().distance(at) < 3.0 {
                self.hurt_mob(i, dmg * 0.8, at, 0);
            }
        }
        // Where it lands on the volcano, the ground scorches: a fire, or fresh magma rock.
        let cell = at.floor().as_ivec3();
        let biome = self.world.gen.base_biome_at(cell.x, cell.z);
        if matches!(biome, Biome::Volcano | Biome::AshFields) && self.surv.rng.chance(0.35) {
            let top = (0..4).map(|k| cell + IVec3::Y * (1 - k)).find(|q| self.world.solid(*q));
            if let Some(q) = top {
                let natural = ["ash_block", "ash_grass", "basalt", "blackstone", "tuff", "coarse_dirt", "gravel"];
                let here = self.world.block_or_air(q);
                if self.surv.rng.chance(0.5) && natural.contains(&self.reg.get(here).name.as_str()) {
                    let magma = self.reg.id_or_air("magma_block");
                    if magma != AIR {
                        self.world.set_block(q, magma);
                    }
                } else if self.world.block_or_air(q + IVec3::Y) == AIR {
                    let fire = self.reg.id_or_air("fire");
                    if fire != AIR {
                        self.world.set_block(q + IVec3::Y, fire);
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------------------- creatures

    fn vulcao_spawns(&mut self, dt: f32) {
        self.vulcao.spawn_t -= dt;
        let Some(v) = self.vulcao.near else { return };
        if self.vulcao.spawn_t > 0.0 || !self.mobs.enabled || self.mobs.peaceful || !self.survival_or_mobs() {
            return;
        }
        self.vulcao.spawn_t = 4.0;
        let me = self.body.pos;
        let ours = self.mobs.list.iter().filter(|m| m.alive() && OURS.contains(&self.mobs.reg.defs[m.def].name.as_str()) && (m.body.pos - me).length() < 64.0).count();
        let cap = if self.vulcao.erupt > 0.0 { 7 } else { 4 };
        if ours >= cap {
            return;
        }
        let night = self.is_night();
        for _ in 0..6 {
            let r = &mut self.surv.rng;
            let a = r.f32() * std::f32::consts::TAU;
            let d = 18.0 + r.f32() * 22.0;
            let roll = r.f32();
            let (x, z) = ((me.x + a.cos() * d).floor() as i32, (me.z + a.sin() * d).floor() as i32);
            let biome = self.world.gen.base_biome_at(x, z);
            if !matches!(biome, Biome::Volcano | Biome::AshFields) {
                continue;
            }
            let Some(y) = self.world.surface_y(x, z) else { continue };
            let top = self.world.block_or_air(IVec3::new(x, y, z));
            let lava = self.world.fluid(top) == 2;
            let at = Vec3::new(x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5);
            let pick = if lava {
                "magma_salamander"
            } else if (night || self.vulcao.erupt > 0.0) && roll < 0.45 {
                "ember_wisp"
            } else if biome == Biome::Volcano && roll < 0.8 {
                "basalt_golem"
            } else if roll < 0.9 {
                "magma_salamander"
            } else {
                continue;
            };
            if !lava && self.world.solid(IVec3::new(x, y + 1, z)) {
                continue;
            }
            let at = if pick == "ember_wisp" { at + Vec3::Y * 4.0 } else { at };
            if v.cx.is_finite() && self.mobs.spawn(pick, at) {
                return;
            }
        }
    }

    fn survival_or_mobs(&self) -> bool {
        self.mobs.spawning
    }

    /// Basalt golems curl up and roll down at you.
    fn vulcao_rolls(&mut self, dt: f32) {
        let me = self.body.pos;
        self.vulcao.roll_cd -= dt;
        if self.vulcao.roll_cd <= 0.0 {
            self.vulcao.roll_cd = 1.5;
            let golem = self.mobs.reg.id("basalt_golem");
            let start = self.mobs.list.iter().position(|m| {
                Some(m.def) == golem && m.alive() && !m.still && {
                    let d = m.body.pos - me;
                    let flat = Vec3::new(d.x, 0.0, d.z).length();
                    (5.0..26.0).contains(&flat) && (d.y > 1.5 || flat < 16.0) && m.body.on_ground
                }
            });
            if let Some(i) = start {
                if self.surv.rng.chance(0.6) {
                    if let Some(j) = self.swap_kind(i, "basalt_golem_ball") {
                        let id = self.mobs.list[j].net_id;
                        self.mobs.list[j].still = true;
                        self.vulcao.rolls.push(Roll { id, t: 0.0, speed: 2.0, slow: 0.0 });
                        self.sfx(Sfx::Break(crate::audio::Material::Stone));
                    }
                }
            }
        }
        let mut rolls = std::mem::take(&mut self.vulcao.rolls);
        let mut done = Vec::new();
        for r in rolls.iter_mut() {
            let Some(i) = self.mobs.list.iter().position(|m| m.net_id == r.id && m.alive()) else {
                r.t = 99.0;
                continue;
            };
            r.t += dt;
            let m = &mut self.mobs.list[i];
            let to = me - m.body.pos;
            let dir = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();
            let before = m.body.pos;
            // Faster and faster, faster still going downhill.
            r.speed = (r.speed + dt * 7.0).min(12.0);
            m.body.speed_override = Some(r.speed);
            let input = Input { wish: dir, jump: false, sneak: false, sprint: false, crawl: false };
            m.body.step(&self.world, &input, dt);
            let moved = m.body.pos - before;
            if moved.y < -0.01 {
                r.speed = (r.speed + dt * 10.0).min(15.0);
            }
            let flat = Vec3::new(moved.x, 0.0, moved.z).length();
            m.roll += flat / 0.75;
            m.yaw = dir.x.atan2(dir.z);
            r.slow = if flat < r.speed * dt * 0.3 { r.slow + dt } else { 0.0 };
            let hit = (m.body.pos + Vec3::Y * 0.6).distance(me + Vec3::Y * 0.9) < 1.6;
            let speed = r.speed;
            if hit {
                let at = m.body.pos;
                self.damage(5.0 + speed * 0.4, Some(at));
                self.body.vel += dir * 6.0 + Vec3::Y * 4.0;
                r.t = 99.0;
            }
            if r.slow > 0.5 || r.t > 7.0 {
                done.push(r.id);
            }
            if (self.surv.rng.f32()) < 0.5 {
                let (layer, tint) = self.block_look(self.reg.id_or_air("basalt"));
                let p = self.mobs.list[i].body.pos.floor().as_ivec3() - IVec3::Y;
                self.particles.block_bits(p, layer, tint, 1, 0.6);
            }
        }
        rolls.retain(|r| r.t < 7.0 && r.slow <= 0.5 && !done.contains(&r.id));
        self.vulcao.rolls = rolls;
        // Stopped: it uncurls and stands up again.
        for id in done {
            if let Some(i) = self.mobs.list.iter().position(|m| m.net_id == id && m.alive()) {
                if let Some(j) = self.swap_kind(i, "basalt_golem") {
                    self.mobs.list[j].aggro = true;
                }
            }
        }
    }

    /// Replace a mob with another kind in the same place, keeping its share of health (the
    /// old one goes without dropping anything).
    fn swap_kind(&mut self, i: usize, into: &str) -> Option<usize> {
        let (pos, yaw, frac, aggro) = {
            let m = &self.mobs.list[i];
            (m.body.pos, m.yaw, m.health / m.max_health.max(1.0), m.aggro)
        };
        if !self.mobs.spawn(into, pos) {
            return None;
        }
        let j = self.mobs.list.len() - 1;
        let n = &mut self.mobs.list[j];
        n.health = (n.max_health * frac).max(1.0);
        n.yaw = yaw;
        n.aggro = aggro;
        let old = &mut self.mobs.list[i];
        old.counted = true;
        old.health = 0.0;
        old.dead_age = 10.0;
        Some(j)
    }

    // ---------------------------------------------------------------------------- the titan

    fn titan_index(&self) -> Option<usize> {
        self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].name == TITAN)
    }

    fn titan_awake(&self) -> bool {
        self.titan_index().is_some()
    }

    /// Right click with something of this file in hand.
    pub(crate) fn vulcao_use(&mut self) -> bool {
        match self.held_stack().map(|s| self.items.get(s.item).name.as_str()) {
            Some("obsidian_idol") => self.use_obsidian_idol(),
            _ => false,
        }
    }

    /// Right click with the Obsidian Idol: in a crater, it wakes the titan.
    pub(crate) fn use_obsidian_idol(&mut self) -> bool {
        let me = self.body.pos;
        let Some(v) = self.vulcao.near.filter(|v| (me.x - v.cx).hypot(me.z - v.cz) < v.crater + 14.0 && me.y > v.lava as f32 - 4.0) else {
            self.toast("O ídolo só desperta algo na cratera de um vulcão", 3.0);
            return true;
        };
        if self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].boss) {
            self.toast("Já há um chefe acordado", 3.0);
            return true;
        }
        let at = Vec3::new(v.cx, v.lava as f32 - 9.0, v.cz);
        if !self.mobs.spawn(TITAN, at) {
            return true;
        }
        if let Some(m) = self.mobs.list.last_mut() {
            m.still = true;
            m.aggro = true;
        }
        self.vulcao.titan = Titan { bomb_t: 5.0, slam_t: 8.0, call_t: 14.0, rise_t: 3.0, ..Default::default() };
        self.vulcao.erupt = 0.0;
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.weather.flash = 0.6;
        self.fx.shake = self.fx.shake.max(0.8);
        self.sfx(Sfx::Explode);
        self.sfx(Sfx::Thunder(0.8));
        self.toast("O Titã de Magma desperta na cratera!", 4.0);
        true
    }

    fn vulcao_titan(&mut self, dt: f32) {
        let Some(i) = self.titan_index() else { return };
        let me = self.body.pos;
        let Some(v) = self.world.gen.volcano_at(self.mobs.list[i].body.pos.x as i32, self.mobs.list[i].body.pos.z as i32) else { return };
        let tp = self.mobs.list[i].body.pos;
        // Gone too far: it sinks back into its lava.
        if (tp - me).length() > 140.0 {
            let m = &mut self.mobs.list[i];
            m.counted = true;
            m.health = 0.0;
            m.dead_age = 10.0;
            return;
        }
        let t = &mut self.vulcao.titan;
        t.t += dt;
        t.contact_t -= dt;
        let (hp, max) = (self.mobs.list[i].health, self.mobs.list[i].max_health);
        if !t.angry && hp < max * 0.5 {
            t.angry = true;
            t.rise_t = 1.0;
            self.toast("O Titã ruge: a lava está subindo!", 4.0);
            self.sfx(Sfx::Thunder(0.9));
            self.fx.shake = self.fx.shake.max(0.8);
        }
        let angry = self.vulcao.titan.angry;
        let lava = v.lava as f32 + self.vulcao.titan.risen as f32;
        // Rising out of the lake, then wading towards the player, staying in the lava.
        let to = Vec3::new(me.x - v.cx, 0.0, me.z - v.cz);
        let reach = v.crater * 0.72;
        let goal = Vec3::new(v.cx, 0.0, v.cz) + to.normalize_or_zero() * to.length().min(reach);
        let rising = self.vulcao.titan.t < 3.5;
        let feet = if rising { v.lava as f32 - 9.0 + self.vulcao.titan.t / 3.5 * 7.5 } else { lava - 1.5 + (self.vulcao.titan.t * 1.3).sin() * 0.3 };
        {
            let m = &mut self.mobs.list[i];
            let flat = Vec3::new(goal.x - m.body.pos.x, 0.0, goal.z - m.body.pos.z);
            let speed: f32 = if angry { 3.2 } else { 2.2 };
            let step = if rising { Vec3::ZERO } else { flat.normalize_or_zero() * speed.min(flat.length() / dt.max(0.001)) * dt };
            m.body.pos = Vec3::new(m.body.pos.x + step.x, feet, m.body.pos.z + step.z);
            m.body.vel = Vec3::ZERO;
            let look = me - m.body.pos;
            m.yaw = look.x.atan2(look.z);
            if step.length() > 0.001 {
                m.walk += step.length() * 0.6;
                m.walk_speed = speed;
            } else {
                m.walk_speed = 0.0;
            }
        }
        if rising {
            let glow = self.glow_layer();
            let r = &mut self.surv.rng;
            let p = tp + Vec3::new(r.f32() * 6.0 - 3.0, 4.0, r.f32() * 6.0 - 3.0);
            self.particles.glow(p, Vec3::new(0.0, 5.0, 0.0), glow, [1.9, 0.7, 0.2], 0.2, 1.0);
            return;
        }
        // Test pictures want it standing in its lake, not the player knocked about by it.
        if self.args.screenshot.is_some() {
            return;
        }
        let hand = tp + Vec3::Y * 9.5;
        let flat_d = Vec3::new(me.x - tp.x, 0.0, me.z - tp.z).length();
        // Bombs, in a fan around the player.
        let t = &mut self.vulcao.titan;
        t.bomb_t -= dt;
        if t.bomb_t <= 0.0 {
            t.bomb_t = if angry { 2.4 } else { 3.4 };
            let n = if angry { 5 } else { 3 };
            self.mobs.list[i].attack_age = 0.0;
            for k in 0..n {
                let r = &mut self.surv.rng;
                let off = if k == 0 { Vec3::ZERO } else { Vec3::new(r.f32() * 10.0 - 5.0, 0.0, r.f32() * 10.0 - 5.0) };
                let secs = 1.6 + r.f32() * 0.8;
                self.throw_bomb(hand, me + off, secs, 9.0);
            }
            self.sfx(Sfx::Cast(0));
        }
        // A slam: it raises its fists, then a ring runs out along the ground from where they
        // strike (jump it).
        let t = &mut self.vulcao.titan;
        t.slam_t -= dt;
        if t.windup > 0.0 {
            t.windup -= dt;
            if t.windup <= 0.0 {
                let dir = Vec3::new(me.x - tp.x, 0.0, me.z - tp.z).normalize_or_zero();
                let at = tp + dir * 4.0;
                let at = Vec3::new(at.x, me.y.max(lava + 1.0), at.z);
                self.shockwave(at, 18.0, 9.0);
                self.fx.shake = self.fx.shake.max(0.7);
                let glow = self.glow_layer();
                for _ in 0..30 {
                    let r = &mut self.surv.rng;
                    self.particles.glow(at, Vec3::new(r.f32() - 0.5, r.f32(), r.f32() - 0.5) * 10.0, glow, [1.9, 0.7, 0.2], 0.2, 0.7);
                }
            }
        } else if t.slam_t <= 0.0 && flat_d < 16.0 {
            t.slam_t = if angry { 5.0 } else { 7.0 };
            t.windup = 0.9;
            self.mobs.list[i].attack_age = 0.0;
            self.sfx(Sfx::Thunder(0.4));
        }
        // Golems from the rim, rolling down into the crater.
        let t = &mut self.vulcao.titan;
        t.call_t -= dt;
        if t.call_t <= 0.0 {
            t.call_t = if angry { 14.0 } else { 20.0 };
            for k in 0..2 {
                let a = self.surv.rng.f32() * std::f32::consts::TAU + k as f32 * 3.0;
                let d = v.crater + 3.0;
                let (x, z) = (v.cx + a.cos() * d, v.cz + a.sin() * d);
                let y = self.world.surface_y(x.floor() as i32, z.floor() as i32).unwrap_or(v.rim) as f32 + 1.0;
                self.mobs.spawn("basalt_golem", Vec3::new(x, y, z));
            }
            self.toast("Golens de basalto descem a cratera!", 2.5);
        }
        // The lava rises, a block at a time.
        if angry && self.vulcao.titan.risen < 4 {
            self.vulcao.titan.rise_t -= dt;
            if self.vulcao.titan.rise_t <= 0.0 {
                self.vulcao.titan.rise_t = 6.0;
                self.raise_lava(&v);
            }
        }
        // Touching it burns.
        let near = (me + Vec3::Y * 0.9).distance(tp + Vec3::Y * 4.0) < 4.5;
        if near && self.vulcao.titan.contact_t <= 0.0 {
            self.vulcao.titan.contact_t = 0.8;
            self.damage(10.0, Some(tp));
        }
    }

    fn raise_lava(&mut self, v: &Volcano) {
        let lava = self.reg.id_or_air("lava");
        if lava == AIR {
            return;
        }
        let y = v.lava + self.vulcao.titan.risen + 1;
        let r = v.crater.ceil() as i32 + 2;
        let mut n = 0;
        for dz in -r..=r {
            for dx in -r..=r {
                if ((dx * dx + dz * dz) as f32).sqrt() > v.crater + 1.0 {
                    continue;
                }
                let p = IVec3::new(v.cx.floor() as i32 + dx, y, v.cz.floor() as i32 + dz);
                let below = self.world.block_or_air(p - IVec3::Y);
                if self.world.block_or_air(p) == AIR && self.world.fluid(below) == 2 {
                    self.world.set_block(p, lava);
                    self.vulcao.titan.raised.push(p);
                    n += 1;
                }
            }
        }
        self.vulcao.titan.risen += 1;
        if n > 0 {
            self.sfx(Sfx::Fizz);
            self.fx.shake = self.fx.shake.max(0.4);
        }
    }

    /// Called when any boss falls.
    pub(crate) fn vulcao_boss_defeated(&mut self, name: &str) {
        if name != TITAN {
            return;
        }
        let obsidian = self.reg.id_or_air("obsidian");
        let raised = std::mem::take(&mut self.vulcao.titan.raised);
        if obsidian != AIR {
            for p in raised {
                if self.world.fluid(self.world.block_or_air(p)) == 2 {
                    self.world.set_block(p, obsidian);
                }
            }
        }
        self.vulcao.next = 1200.0;
        self.vulcao.erupt = 0.0;
        self.toast("O vulcão adormece. A lava que subiu esfriou em obsidiana.", 5.0);
    }

    /// A melee hit with the Volcanic Hammer sets the monster alight.
    pub(crate) fn vulcao_melee_hit(&mut self, i: usize) {
        if self.held_stack().is_some_and(|s| self.items.get(s.item).name == "volcanic_hammer") {
            if let Some(m) = self.mobs.list.get_mut(i) {
                m.on_fire = m.on_fire.max(4.0);
            }
        }
    }

    // ---------------------------------------------------------------------------- tests

    /// Test scenes (MC_VULCAO): `crater` stands on the rim, `slope` halfway up, `tube` inside a
    /// lava tube looking up it, `erupt` on the slope as it erupts, `titan` on the rim with the titan awake,
    /// `golem` a golem rolling at you, `vent` on a fumarole.
    pub(crate) fn test_vulcao(&mut self, scene: &str) {
        let me = self.body.pos;
        let mut found = None;
        'search: for r in 0..12 {
            for (i, j) in [(0, 0), (1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                let x = me.x as i32 + (i * r) * 1700;
                let z = me.z as i32 + (j * r) * 1700;
                if let Some(v) = self.world.gen.volcano_at(x, z) {
                    found = Some(v);
                    break 'search;
                }
            }
        }
        let Some(v) = found else {
            log::warn!("vulcao: no volcano found");
            return;
        };
        log::info!("vulcao: volcano at {:.0},{:.0} rim {} lava {} radius {:.0}", v.cx, v.cz, v.rim, v.lava, v.radius);
        let gen = &self.world.gen;
        let spot = |d: f32, a: f32| -> Vec3 {
            let (x, z) = (v.cx + a.cos() * d, v.cz + a.sin() * d);
            Vec3::new(x, gen.height_at(x as i32, z as i32) as f32 + 1.0, z)
        };
        let (pos, look) = match scene {
            "crater" => (spot(v.crater + 2.5, 0.0), Vec3::new(v.cx, v.lava as f32, v.cz)),
            // Hovering just over the rim, looking down at the lake.
            "titan" => (Vec3::new(v.cx + v.crater + 6.0, v.rim as f32 + 7.0, v.cz), Vec3::new(v.cx + v.crater * 0.45, v.lava as f32 + 2.0, v.cz)),
            // Off the slope a little, so its own bumps don't hide the golem just up it.
            "golem" => (spot(v.radius * 0.8 + 6.0, 0.3) + Vec3::Y * 2.0, spot(v.radius * 0.8 - 2.0, 0.3) + Vec3::Y),
            "tube" => {
                let a = v.tubes[0];
                let (mouth, end) = v.tube_ends();
                let d = mouth - 14.0;
                let at = |d: f32| Vec3::new(v.cx + a.cos() * d, v.tube_y(d) - 2.2, v.cz + a.sin() * d);
                (at(d), at(end + 10.0) + Vec3::Y * 1.6)
            }
            "far" => (spot(v.radius * 1.9, 0.6) + Vec3::Y * 30.0, Vec3::new(v.cx, v.rim as f32 - 20.0, v.cz)),
            _ => (spot(v.radius * 0.55, 0.3), Vec3::new(v.cx, v.rim as f32, v.cz)),
        };
        self.body.pos = pos;
        self.body.vel = Vec3::ZERO;
        self.body.flying = matches!(scene, "far" | "titan" | "golem");
        let d = look - (pos + Vec3::Y * 1.6);
        self.yaw = (-d.x).atan2(-d.z);
        self.pitch = (d.y / d.length().max(0.1)).asin();
        self.vulcao.near = Some(v);
        self.vulcao.look_t = 1.0;
        match scene {
            "erupt" | "far" => {
                self.vulcao.next = 0.01;
                self.vulcao.warned = true;
            }
            "titan" => {
                self.vulcao.next = 999.0;
                self.vulcao.titan_test = true;
            }
            "golem" => {
                // Just up the slope from the player, on the real ground.
                let up = spot(v.radius * 0.8 - 2.0, 0.3);
                self.mobs.spawn("basalt_golem", up);
            }
            _ => self.vulcao.next = 999.0,
        }
    }
}

/// How many blocks over a vent at `p` must be open for it to push something at `feet`.
fn k_free(p: IVec3, feet: IVec3) -> i32 {
    (feet.y - p.y).max(0)
}
