//! Spells and the world (HP 2026-10-28: "um cajado do gelo na água deve congelar a água").
//! Every spell has an element, and where it lands the element acts on the blocks and mobs there:
//!
//! - Frost freezes the top of water into frosted ice and lava into a dark crust, both thawing
//!   back after a while; it puts fires out, slows mobs and freezes soaked ones solid.
//! - Fire melts ice and snow, boils shallow water away in steam, lights grass, leaves and wood,
//!   sets off TNT and thaws what frost made; it can't light a soaked mob, it dries it.
//! - Water puts fires and burning mobs out, cools lava for good (obsidian, cobblestone), turns
//!   dirt to mud, soaks sponges and mobs; fiery mobs hate it.
//! - Lightning landing in water runs through it to every mob and player swimming close by;
//!   soaked mobs take double and pass it on to soaked mobs near them; it shatters frozen ones.
//! - Nature makes crops, saplings and grass grow; blasts shake leaves down and blow dropped
//!   items away.
//!
//! Blocks a spell changes only for a while are kept in a list (saved in `elements.ron` in the
//! world's folder, by dimension) and put back when their time is up, but only if nobody changed
//! them meanwhile. They are ordinary blocks, so multiplayer sends them like any other change, and
//! should a timer be lost (a guest left mid-freeze) random ticks thaw them anyway.

use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};
use voxel::block::{BlockId, Shape, AIR};
use voxel::physics::Terrain;

use crate::audio::{Material, Sfx};
use crate::Game;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum El {
    Fire,
    Frost,
    Water,
    Shock,
    Nature,
    Arcane,
}

/// The element of a spell school (magic.rs and the spellbooks share the first six numbers).
pub fn of_school(school: u8) -> El {
    match school {
        0 => El::Fire,
        1 => El::Frost,
        2 | 5 => El::Shock,
        4 => El::Nature,
        _ => El::Arcane,
    }
}

/// Mobs made of fire: fire barely hurts them, water and frost hurt more.
pub fn fiery(name: &str) -> bool {
    ["blaze", "magma", "fire", "ember", "lava", "inferno", "flame", "hellhound", "imp"].iter().any(|k| name.contains(k))
}

/// A block a spell changed for a while.
struct Temp {
    dim: u8,
    p: IVec3,
    placed: BlockId,
    back: BlockId,
    t: f32,
}

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    /// Dimension, where, the spell's block, the block it goes back to, seconds left.
    temps: Vec<(u8, [i32; 3], String, String, f32)>,
}

#[derive(Default, Clone, Copy)]
struct Ids {
    water: BlockId,
    lava: BlockId,
    frosted: BlockId,
    crust: BlockId,
    ice: BlockId,
    packed: BlockId,
    snow: BlockId,
    snow_layer: BlockId,
    snowy_grass: BlockId,
    grass: BlockId,
    dirt: BlockId,
    coarse: BlockId,
    mud: BlockId,
    sand: BlockId,
    glass: BlockId,
    tnt: BlockId,
    obsidian: BlockId,
    cobble: BlockId,
    sponge: BlockId,
    wet_sponge: BlockId,
    short_grass: BlockId,
    flowers: [BlockId; 2],
}

#[derive(Default)]
pub struct Elements {
    loaded: bool,
    temps: Vec<Temp>,
    ids: Ids,
    /// Lightning passing from soaked mob to soaked mob: no passing it on again.
    chaining: bool,
    fx_t: f32,
}

impl Elements {
    /// Blocks that thaw on random ticks (copper.rs).
    pub fn ticks(&self, id: BlockId) -> bool {
        id != AIR && (id == self.ids.frosted || id == self.ids.crust)
    }
}

/// How long frost holds water (seconds), before a little randomness.
pub const FREEZE_SECS: f32 = 14.0;

impl Game {
    fn elements_path(&self) -> std::path::PathBuf {
        crate::level_path_hint(&self.base, &self.world_name).join("elements.ron")
    }

    fn el_ready(&mut self) {
        if self.elements.loaded {
            return;
        }
        let g = |n: &str| self.reg.id_or_air(n);
        let ids = Ids {
            water: g("water"),
            lava: g("lava"),
            frosted: g("frosted_ice"),
            crust: g("lava_crust"),
            ice: g("ice"),
            packed: g("packed_ice"),
            snow: g("snow"),
            snow_layer: g("snow_layer"),
            snowy_grass: g("snowy_grass_block"),
            grass: g("grass_block"),
            dirt: g("dirt"),
            coarse: g("coarse_dirt"),
            mud: g("mud"),
            sand: g("sand"),
            glass: g("glass"),
            tnt: g("tnt"),
            obsidian: g("obsidian"),
            cobble: g("cobblestone"),
            sponge: g("sponge"),
            wet_sponge: g("wet_sponge"),
            short_grass: g("short_grass"),
            flowers: [g("poppy"), g("dandelion")],
        };
        let saved: Saved = std::fs::read_to_string(self.elements_path()).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default();
        let temps = saved
            .temps
            .into_iter()
            .filter_map(|(dim, p, placed, back, t)| Some(Temp { dim, p: IVec3::from(p), placed: self.reg.id(&placed)?, back: self.reg.id(&back)?, t }))
            .collect();
        self.elements = Elements { loaded: true, temps, ids, chaining: false, fx_t: 0.0 };
    }

    pub(crate) fn elements_save(&self) {
        if !self.elements.loaded {
            return;
        }
        let s = Saved {
            temps: self.elements.temps.iter().map(|t| (t.dim, t.p.to_array(), self.reg.get(t.placed).name.clone(), self.reg.get(t.back).name.clone(), t.t)).collect(),
        };
        if let Ok(text) = ron::ser::to_string_pretty(&s, Default::default()) {
            if let Err(e) = voxel::save::write_atomic(self.elements_path(), text) {
                log::error!("could not save elements.ron: {e}");
            }
        }
    }

    /// Put `placed` at `p` for `secs` seconds; then what was there comes back.
    pub(crate) fn temp_block(&mut self, p: IVec3, placed: BlockId, secs: f32) -> bool {
        self.el_ready();
        let Some(was) = self.world.block(p) else { return false };
        let dim = self.dim.as_u8();
        if let Some(t) = self.elements.temps.iter_mut().find(|t| t.dim == dim && t.p == p) {
            if t.placed == was {
                // Already a spell's block here: it keeps what it goes back to.
                t.placed = placed;
                t.t = t.t.max(secs);
                if was != placed {
                    self.world.set_block(p, placed);
                }
                return true;
            }
            t.placed = placed;
            t.back = was;
            t.t = secs;
        } else {
            if was == placed {
                return false;
            }
            self.elements.temps.push(Temp { dim, p, placed, back: was, t: secs });
        }
        self.world.set_block(p, placed)
    }

    /// Undo a spell's block at `p` right away. False when there is none.
    fn revert_temp(&mut self, p: IVec3) -> bool {
        let dim = self.dim.as_u8();
        let Some(i) = self.elements.temps.iter().position(|t| t.dim == dim && t.p == p) else { return false };
        let t = self.elements.temps.swap_remove(i);
        if self.world.block_or_air(p) != t.placed {
            return false;
        }
        self.world.set_block(p, t.back);
        true
    }

    /// Spells' blocks thaw when their time is up; soaked and frozen mobs show it.
    pub(crate) fn update_elements(&mut self, dt: f32) {
        self.el_ready();
        if dt == 0.0 {
            return;
        }
        let dim = self.dim.as_u8();
        let ids = self.elements.ids;
        let mut due = Vec::new();
        let mut cracking = Vec::new();
        for (i, t) in self.elements.temps.iter_mut().enumerate() {
            if t.dim != dim {
                continue;
            }
            t.t -= dt;
            if t.t <= 0.0 {
                due.push(i);
            } else if t.t < 2.5 {
                cracking.push((t.p, t.placed));
            }
        }
        for &i in due.iter().rev() {
            let (p, placed, back) = {
                let t = &self.elements.temps[i];
                (t.p, t.placed, t.back)
            };
            // Not loaded right now: it thaws once we are back.
            let Some(cur) = self.world.block(p) else { continue };
            self.elements.temps.swap_remove(i);
            if cur == placed {
                self.world.set_block(p, back);
                self.thaw_fx(p, placed);
            }
        }
        let layer = self.glow_layer();
        // About to thaw: frosted ice drips, the crust glows through its cracks.
        for (p, placed) in cracking {
            if self.surv.rng.f32() < dt * 3.0 {
                let at = p.as_vec3() + Vec3::new(self.surv.rng.f32(), 1.02, self.surv.rng.f32());
                if placed == ids.crust {
                    self.particles.spark(at, Vec3::ZERO, layer);
                } else {
                    self.particles.dust(at, Vec3::Y * 0.05, layer, [0.8, 0.9, 1.0], 0.03, 0.6);
                }
            }
        }
        // Soaked mobs drip, frozen ones glitter.
        self.elements.fx_t += dt;
        if self.elements.fx_t < 0.1 {
            return;
        }
        self.elements.fx_t = 0.0;
        let me = self.body.pos;
        let marks: Vec<(Vec3, f32, bool)> = self
            .mobs
            .list
            .iter()
            .filter(|m| m.alive() && (m.frozen > 0.0 || m.wet > 0.0 && !m.body.in_water) && m.body.pos.distance(me) < 32.0)
            .map(|m| {
                let d = &self.mobs.reg.defs[m.def];
                (m.body.pos, d.height * d.scale.max(0.5), m.frozen > 0.0)
            })
            .collect();
        for (pos, h, frozen) in marks {
            let j = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5);
            if frozen {
                self.particles.glow(pos + Vec3::new(j.x, j.y * h, j.z), Vec3::Y * 0.1, layer, [0.7, 0.9, 1.3], 0.05, 0.5);
            } else {
                self.particles.drip(pos + Vec3::new(j.x * 0.5, j.y * h, j.z * 0.5), layer);
            }
        }
    }

    fn thaw_fx(&mut self, p: IVec3, placed: BlockId) {
        let layer = self.glow_layer();
        let top = p.as_vec3() + Vec3::new(0.5, 1.0, 0.5);
        if placed == self.elements.ids.crust {
            for _ in 0..4 {
                self.particles.spark(top, Vec3::ZERO, layer);
            }
        } else {
            let (l, tint) = self.block_look(placed);
            self.particles.block_bits(p, l, tint, 3, 0.4);
        }
    }

    /// Random tick of a spell's block (copper.rs): one whose timer was lost thaws on its own.
    pub(crate) fn elements_tick(&mut self, p: IVec3, id: BlockId, r: u32) {
        let dim = self.dim.as_u8();
        if r % 3 != 0 || self.elements.temps.iter().any(|t| t.dim == dim && t.p == p) || self.prog.frost.iter().any(|f| f.0 == p) {
            return;
        }
        let ids = self.elements.ids;
        self.world.set_block(p, if id == ids.crust { ids.lava } else { ids.water });
        self.thaw_fx(p, id);
    }

    /// The blocks within `r` of `at`, nearest first.
    fn cells_around(&self, at: Vec3, r: f32) -> Vec<(f32, IVec3)> {
        let c = at.floor().as_ivec3();
        let ri = r.ceil() as i32;
        let mut cells = Vec::new();
        for dy in -ri..=ri {
            for dz in -ri..=ri {
                for dx in -ri..=ri {
                    let p = c + IVec3::new(dx, dy, dz);
                    let d = (p.as_vec3() + Vec3::splat(0.5)).distance(at);
                    if d <= r + 0.35 {
                        cells.push((d, p));
                    }
                }
            }
        }
        cells.sort_by(|a, b| a.0.total_cmp(&b.0));
        cells
    }

    /// Does the player's or a mob's body fill this block (freezing it would trap them)? Mobs
    /// in it are frozen solid instead.
    fn body_in(&mut self, p: IVec3) -> bool {
        let inside = |pos: Vec3, w: f32, h: f32| {
            let lo = p.as_vec3();
            pos.x + w > lo.x && pos.x - w < lo.x + 1.0 && pos.z + w > lo.z && pos.z - w < lo.z + 1.0 && pos.y + h > lo.y && pos.y < lo.y + 1.0
        };
        if inside(self.body.pos, 0.3, 1.8) {
            return true;
        }
        let mut any = false;
        for m in self.mobs.list.iter_mut() {
            let d = &self.mobs.reg.defs[m.def];
            if m.alive() && !m.ghost && inside(m.body.pos, d.width * 0.5, d.height) {
                if !d.boss {
                    m.frozen = m.frozen.max(3.0);
                }
                any = true;
            }
        }
        any
    }

    /// The point the player looks at, stopping at liquids too (spells aimed at water).
    pub(crate) fn spell_aim(&self, range: f32) -> Vec3 {
        let eye = self.body.eye();
        let fwd = self.forward();
        let start_wet = self.world.is_liquid(self.world.block_or_air(eye.floor().as_ivec3()));
        let mut t = 0.0;
        while t < range {
            t += 0.25;
            let p = eye + fwd * t;
            let cell = p.floor().as_ivec3();
            if self.mob_at(p).is_some() || self.world.solid(cell) || !start_wet && self.world.is_liquid(self.world.block_or_air(cell)) {
                return p;
            }
        }
        eye + fwd * range
    }

    /// A spell of element `el` lands at `at`, reaching `r` blocks. `power` is about 1 for a
    /// wand's bolt, less for quick little spells, 2 or more for bursts and lightning from the sky.
    pub(crate) fn element_hit(&mut self, at: Vec3, el: El, r: f32, power: f32) {
        if el == El::Arcane {
            return;
        }
        self.el_ready();
        let cells = self.cells_around(at, r.clamp(0.5, 5.0));
        match el {
            El::Fire => self.fire_hit(at, &cells, r, power),
            El::Frost => self.frost_hit(at, &cells, r, power),
            El::Water => self.water_hit(at, &cells, power),
            El::Shock => self.shock_hit(at, &cells, power),
            El::Nature => self.nature_hit(&cells, power),
            El::Arcane => {}
        }
    }

    fn fire_hit(&mut self, at: Vec3, cells: &[(f32, IVec3)], r: f32, power: f32) {
        let ids = self.elements.ids;
        let mut fires = if power >= 2.0 { 5 } else { 1 };
        let chance = (0.35 * power).min(0.75);
        let max_boil = (2.0 * power).ceil() as u32;
        let (mut melted, mut boiled, mut steam) = (0, 0, 0);
        let layer = self.glow_layer();
        for &(d, p) in cells {
            let id = self.world.block_or_air(p);
            if id == AIR {
                continue;
            }
            if id == ids.frosted || id == ids.crust {
                if !self.revert_temp(p) {
                    self.world.set_block(p, if id == ids.crust { ids.lava } else { ids.water });
                }
                melted += 1;
            } else if id == ids.ice {
                self.world.set_block(p, ids.water);
                melted += 1;
            } else if id == ids.packed {
                self.world.set_block(p, ids.ice);
                melted += 1;
            } else if id == ids.snow || id == ids.snow_layer {
                self.world.set_block(p, AIR);
                melted += 1;
            } else if id == ids.snowy_grass && ids.grass != AIR {
                self.world.set_block(p, ids.grass);
                melted += 1;
            } else if self.world.fluid(id) == 1 {
                // Only the surface steams; a puddle with ground right under it boils away.
                if self.world.fluid(self.world.block_or_air(p + IVec3::Y)) == 1 {
                    continue;
                }
                steam += 1;
                let below = self.world.block_or_air(p - IVec3::Y);
                if boiled < max_boil && self.world.fluid(below) != 1 && !self.world.is_wet(id) && self.world.solid(p - IVec3::Y) {
                    self.world.set_block(p, AIR);
                    boiled += 1;
                }
                if steam <= 6 {
                    self.steam_at(p.as_vec3() + Vec3::new(0.5, 0.9, 0.5), layer, 6);
                }
            } else if id == ids.tnt && d < r {
                let fuse = 2.0 + self.surv.rng.f32();
                self.prime_tnt(p, fuse);
            } else if fires > 0 && d < r * 0.9 && (self.extras.is_flammable(id) || id == ids.grass) && self.surv.rng.f32() < chance {
                let spot = if self.reg.get(id).shape == Shape::Cross {
                    self.world.set_block(p, AIR);
                    Some(p)
                } else {
                    [IVec3::Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z].iter().map(|o| p + *o).find(|q| self.world.block_or_air(*q) == AIR)
                };
                if let Some(q) = spot {
                    self.set_fire(q);
                    fires -= 1;
                }
            }
        }
        if melted + boiled + steam > 0 {
            self.sfx_at(Sfx::Fizz, at, 0.8, 24.0);
        }
    }

    fn steam_at(&mut self, at: Vec3, layer: u16, n: usize) {
        for _ in 0..n {
            let j = Vec3::new(self.surv.rng.f32() - 0.5, 0.0, self.surv.rng.f32() - 0.5);
            let v = Vec3::new(j.x * 0.4, 1.2 + self.surv.rng.f32() * 1.2, j.z * 0.4);
            self.particles.dust(at + j * 0.8, v, layer, [0.92, 0.94, 0.97], 0.14 + self.surv.rng.f32() * 0.12, 1.6);
        }
    }

    fn frost_hit(&mut self, at: Vec3, cells: &[(f32, IVec3)], r: f32, power: f32) {
        let ids = self.elements.ids;
        if ids.frosted == AIR {
            return;
        }
        let secs = FREEZE_SECS + 6.0 * (power - 1.0).max(0.0);
        let (mut froze, mut doused) = (0, 0);
        for &(d, p) in cells {
            let id = self.world.block_or_air(p);
            let fluid = self.world.fluid(id);
            if fluid != 0 {
                // The surface freezes over (and whatever the spell itself is in).
                let above = self.world.block_or_air(p + IVec3::Y);
                if self.world.fluid(above) == fluid && d > 0.9 {
                    continue;
                }
                if self.world.is_wet(id) || self.body_in(p) {
                    continue;
                }
                let t = secs + self.surv.rng.f32() * 4.0 - d;
                let placed = if fluid == 1 { ids.frosted } else { ids.crust };
                if placed != AIR && self.temp_block(p, placed, if fluid == 1 { t } else { t * 1.5 }) {
                    froze += 1;
                }
            } else if self.extras.is_fire(id) {
                self.world.set_block(p, AIR);
                doused += 1;
            } else if power >= 2.0 && d < r && ids.snow_layer != AIR && self.reg.get(id).shape == Shape::Cube && self.world.solid(p) && self.world.block_or_air(p + IVec3::Y) == AIR && self.surv.rng.f32() < 0.5 {
                // Strong frost leaves a thin snow behind for a while.
                self.temp_block(p + IVec3::Y, ids.snow_layer, secs * 2.0);
            }
        }
        if froze > 0 {
            let layer = self.glow_layer();
            for _ in 0..(6 + froze * 2).min(40) {
                let j = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() * 0.5, self.surv.rng.f32() - 0.5) * r.max(1.0) * 1.6;
                self.particles.glow(at + j, Vec3::Y * 0.4, layer, [0.75, 0.92, 1.3], 0.05, 0.7);
            }
            self.sfx_at(Sfx::Break(Material::Snow), at, 0.9, 24.0);
        }
        if doused > 0 {
            self.sfx_at(Sfx::Fizz, at, 0.7, 24.0);
        }
    }

    fn water_hit(&mut self, at: Vec3, cells: &[(f32, IVec3)], power: f32) {
        let ids = self.elements.ids;
        let layer = self.glow_layer();
        let mut hiss = 0;
        for &(d, p) in cells {
            let id = self.world.block_or_air(p);
            if id == AIR {
                continue;
            }
            if self.extras.is_fire(id) {
                self.world.set_block(p, AIR);
                hiss += 1;
            } else if self.world.fluid(id) == 2 {
                // Water on lava: a source turns to obsidian, flowing lava to cobblestone.
                self.world.set_block(p, if id == ids.lava { ids.obsidian } else { ids.cobble });
                hiss += 1;
            } else if id == ids.crust {
                // Lava under frost's crust cools for good.
                let back = self.elements.temps.iter().find(|t| t.p == p).map(|t| t.back).unwrap_or(ids.lava);
                self.revert_temp(p);
                self.world.set_block(p, if back == ids.lava { ids.obsidian } else { ids.cobble });
                hiss += 1;
            } else if (id == ids.dirt || id == ids.coarse) && ids.mud != AIR && d < 1.2 + power * 0.3 && !self.world.solid(p + IVec3::Y) {
                self.world.set_block(p, ids.mud);
            } else if id == ids.sponge && ids.wet_sponge != AIR {
                self.world.set_block(p, ids.wet_sponge);
            }
        }
        self.particles.splash(at, layer, 0.3 * power.min(2.0));
        if hiss > 0 {
            self.steam_at(at, layer, 10);
            self.sfx_at(Sfx::Fizz, at, 0.8, 24.0);
        } else {
            self.sfx_at(Sfx::Splash(0.3), at, 0.6, 20.0);
        }
    }

    fn shock_hit(&mut self, at: Vec3, cells: &[(f32, IVec3)], power: f32) {
        let ids = self.elements.ids;
        let layer = self.glow_layer();
        let wet = cells.iter().any(|&(d, p)| d < 1.6 && self.world.fluid(self.world.block_or_air(p)) == 1);
        if wet {
            // The water carries it to everyone swimming close by.
            let reach = 6.0 + 2.0 * power;
            let dmg = 3.0 * power;
            let targets: Vec<(usize, Vec3)> = self
                .mobs
                .list
                .iter()
                .enumerate()
                .filter(|(_, m)| m.alive() && !m.ghost && m.body.in_water && !self.mobs.reg.defs[m.def].town && m.body.center().distance(at) < reach)
                .map(|(k, m)| (k, m.body.center()))
                .collect();
            for (k, c) in targets {
                self.arc(at, c, layer);
                self.elements.chaining = true;
                self.hurt_mob(k, dmg, at, 2);
                self.elements.chaining = false;
            }
            if self.body.in_water && self.body.center().distance(at) < reach {
                self.damage(1.0 + power, Some(at));
            }
            // Sparks dance over the water.
            for _ in 0..(40.0 * power) as usize {
                let a = self.surv.rng.f32() * std::f32::consts::TAU;
                let rr = self.surv.rng.f32().sqrt() * reach;
                let p = at + Vec3::new(a.cos() * rr, 0.0, a.sin() * rr);
                let mut q = p.floor().as_ivec3();
                for _ in 0..3 {
                    if self.world.fluid(self.world.block_or_air(q + IVec3::Y)) == 1 {
                        q.y += 1;
                    }
                }
                if self.world.fluid(self.world.block_or_air(q)) == 1 {
                    self.particles.glow(Vec3::new(p.x, q.y as f32 + 0.95, p.z), Vec3::Y * 0.5, layer, [1.3, 1.3, 1.7], 0.1, 0.5);
                }
            }
            self.sfx_at(Sfx::Fizz, at, 0.6, 24.0);
        }
        if power < 2.0 {
            return;
        }
        // Lightning from the sky: sand fuses to glass, dry grass and wood catch, TNT goes off.
        let mut fires = 2;
        for &(d, p) in cells {
            if d > 1.8 {
                break;
            }
            let id = self.world.block_or_air(p);
            if id == ids.sand && ids.glass != AIR && self.world.block_or_air(p + IVec3::Y) == AIR {
                self.world.set_block(p, ids.glass);
            } else if id == ids.tnt {
                self.prime_tnt(p, 1.5);
            } else if fires > 0 && !wet && !self.weather.state.raining && (self.extras.is_flammable(id) || id == ids.grass) && self.world.block_or_air(p + IVec3::Y) == AIR {
                self.set_fire(p + IVec3::Y);
                fires -= 1;
            }
        }
    }

    fn arc(&mut self, from: Vec3, to: Vec3, layer: u16) {
        let seg = to - from;
        let steps = (seg.length() / 0.3).ceil().max(1.0) as usize;
        for s in 0..=steps {
            let j = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5) * 0.3;
            self.particles.glow(from + seg * (s as f32 / steps as f32) + j, Vec3::ZERO, layer, [1.3, 1.3, 1.8], 0.09, 0.35);
        }
    }

    fn nature_hit(&mut self, cells: &[(f32, IVec3)], power: f32) {
        let ids = self.elements.ids;
        let layer = self.glow_layer();
        let mut grew = Vec::new();
        for &(_, p) in cells {
            let id = self.world.block_or_air(p);
            if id == AIR {
                continue;
            }
            let roll = self.surv.rng.f32();
            if self.farm.is_crop(id) {
                let next = self.farm.next_stage(id);
                if next != AIR && roll < 0.25 + 0.25 * power {
                    self.world.set_block(p, next);
                    grew.push(p);
                }
            } else if voxel::trees::is_sapling(&self.reg.get(id).name) {
                if roll < 0.1 * power {
                    for s in self.surv.saplings.iter_mut().filter(|(q, _)| *q == p) {
                        s.1 = 0.0;
                    }
                    if !self.surv.saplings.iter().any(|(q, _)| *q == p) {
                        self.surv.saplings.push((p, 0.0));
                    }
                    grew.push(p);
                }
            } else if self.world.block_or_air(p + IVec3::Y) != AIR {
                continue;
            } else if (id == ids.dirt || id == ids.coarse) && ids.grass != AIR && roll < 0.5 {
                self.world.set_block(p, ids.grass);
                grew.push(p);
            } else if id == ids.grass && roll < 0.08 * power {
                let b = if roll < 0.02 * power { ids.flowers[(p.x & 1) as usize] } else { ids.short_grass };
                if b != AIR {
                    self.world.set_block(p + IVec3::Y, b);
                    grew.push(p + IVec3::Y);
                }
            }
        }
        for p in grew {
            for k in 0..3 {
                let a = k as f32 * 2.1;
                self.particles.glow(p.as_vec3() + Vec3::new(0.5 + a.cos() * 0.4, 1.05, 0.5 + a.sin() * 0.4), Vec3::Y * 0.6, layer, [0.6, 1.0, 0.4], 0.05, 1.0);
            }
        }
    }

    /// A burst goes off: leaves near it are torn down, leaves further off shed, and dropped
    /// items are blown away from it.
    pub(crate) fn blast_world(&mut self, at: Vec3, r: f32) {
        self.el_ready();
        let cells = self.cells_around(at, r.clamp(1.0, 5.0));
        for &(d, p) in &cells {
            let id = self.world.block_or_air(p);
            if id == AIR || !self.reg.get(id).name.ends_with("_leaves") {
                continue;
            }
            let (layer, tint) = self.block_look(id);
            if d < r * 0.6 && self.surv.rng.f32() < 0.6 {
                self.world.set_block(p, AIR);
                self.particles.block_bits(p, layer, tint, 4, 1.0);
                if self.survival {
                    let drops = self.items.drops(&self.reg, id, None, &mut self.surv.rng);
                    for st in drops {
                        self.pop_drop(p.as_vec3() + Vec3::splat(0.5), st);
                    }
                }
            } else if self.surv.rng.f32() < 0.5 {
                let below = p.as_vec3() + Vec3::new(self.surv.rng.f32(), -0.05, self.surv.rng.f32());
                self.particles.mote(below, layer, tint, 0.06, 0.6, 3.0);
            }
        }
        self.blow_items(at, r * 1.6, 7.0);
    }

    /// Blow dropped items away from `at` (or pull them in, with a negative `strength`).
    pub(crate) fn blow_items(&mut self, at: Vec3, r: f32, strength: f32) {
        for d in self.surv.drops.iter_mut() {
            let to = d.body.pos - at;
            let dist = to.length();
            if dist < r && dist > 0.05 {
                let k = strength * (1.0 - dist / r);
                d.body.vel += to.normalize() * k + Vec3::Y * k.abs() * 0.4;
            }
        }
    }

    /// What an element does to a mob it hits; returns how much harder (or softer) the hit lands.
    pub(crate) fn element_mob(&mut self, k: usize, el: El, dmg: f32) -> f32 {
        let Some(m) = self.mobs.list.get(k) else { return 1.0 };
        let def = &self.mobs.reg.defs[m.def];
        let hot = fiery(&def.name);
        let boss = def.boss;
        let center = m.body.center();
        let layer = self.glow_layer();
        let mut mult = 1.0;
        let mut steam = false;
        let mut shatter = false;
        let mut chain = false;
        let m = &mut self.mobs.list[k];
        let soaked = m.wet > 0.0 || m.body.in_water;
        match el {
            El::Fire => {
                if hot {
                    mult = 0.3;
                }
                if m.frozen > 0.0 {
                    m.frozen = 0.0;
                    mult *= 1.5;
                    steam = true;
                }
                if soaked {
                    // Too wet to burn: the fire dries it a little.
                    m.wet = (m.wet - 4.0).max(0.0);
                    steam = true;
                } else if !hot {
                    m.on_fire = m.on_fire.max(4.0);
                }
            }
            El::Frost => {
                m.on_fire = 0.0;
                m.chill = m.chill.max(3.0);
                if soaked && !boss {
                    m.frozen = m.frozen.max(3.0);
                    m.wet = 0.0;
                }
                if hot {
                    mult = 1.5;
                }
            }
            El::Water => {
                if m.on_fire > 0.0 {
                    steam = true;
                }
                m.wet = m.wet.max(10.0);
                m.on_fire = 0.0;
                if hot {
                    mult = 2.0;
                    steam = true;
                }
            }
            El::Shock => {
                if m.frozen > 0.0 {
                    m.frozen = 0.0;
                    mult = 1.5;
                    shatter = true;
                }
                if soaked {
                    mult *= 2.0;
                    chain = true;
                }
            }
            El::Nature | El::Arcane => {}
        }
        if steam {
            self.steam_at(center, layer, 8);
            self.sfx_at(Sfx::Fizz, center, 0.6, 20.0);
        }
        if shatter {
            for _ in 0..16 {
                let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5) * 4.0;
                self.particles.glow(center, v, layer, [0.75, 0.92, 1.3], 0.07, 0.6);
            }
            self.sfx_at(Sfx::Break(Material::Snow), center, 0.9, 20.0);
        }
        // Lightning jumps on to the soaked mobs near a soaked one (once, not on and on).
        if chain && !self.elements.chaining {
            let next: Vec<(usize, Vec3)> = self
                .mobs
                .list
                .iter()
                .enumerate()
                .filter(|(j, o)| *j != k && o.alive() && !o.ghost && (o.wet > 0.0 || o.body.in_water) && !self.mobs.reg.defs[o.def].town && o.body.center().distance(center) < 6.0)
                .map(|(j, o)| (j, o.body.center()))
                .take(4)
                .collect();
            self.elements.chaining = true;
            for (j, c) in next {
                self.arc(center, c, layer);
                self.hurt_mob(j, dmg * 0.5, center, 2);
            }
            self.elements.chaining = false;
        }
        mult
    }
}

// ---------------------------------------------------------------------------------------------
// Test hook MC_ELEMENTS=frost|fire|shock|water (with MC_PLACE=frames and --screenshot): a pond
// in front of the player, a spell cast into it and the frames after it saved one by one, the
// world running faster after the cast so the thaw fits in a short clip.

impl Game {
    pub(crate) fn test_elements(&mut self, scene: &str) {
        let p = self.body.pos.floor().as_ivec3();
        let f = self.forward();
        let dir = if f.x.abs() > f.z.abs() { IVec3::new(f.x.signum() as i32, 0, 0) } else { IVec3::new(0, 0, f.z.signum() as i32) };
        let side = IVec3::new(-dir.z, 0, dir.x);
        let g = |n: &str| self.reg.id_or_air(n);
        let (grass, dirt, water, lava, stone, log, leaves, tall, snow, tnt) =
            (g("grass_block"), g("dirt"), g("water"), g("lava"), g("stone"), g("oak_log"), g("oak_leaves"), g("short_grass"), g("snow_layer"), g("tnt"));
        let ground = p.y - 1;
        for a in -16..=16 {
            for b in -6..=26 {
                let col = IVec3::new(p.x, ground, p.z) + side * a + dir * b;
                let pond = (4..=16).contains(&b) && (-6..=6).contains(&a) && (a * a + (b - 10) * (b - 10)) < 52;
                let lava_pool = scene != "shock" && (13..=17).contains(&b) && (8..=11).contains(&a);
                // The pond is shallow round its edge (fire boils that part away).
                let shallow = (a * a + (b - 10) * (b - 10)) >= 30;
                for h in -4..14 {
                    let q = col + IVec3::Y * h;
                    let id = if h < -2 {
                        stone
                    } else if h < 0 {
                        if pond && !shallow {
                            water
                        } else if lava_pool && h == -1 {
                            lava
                        } else {
                            dirt
                        }
                    } else if h == 0 {
                        if pond {
                            water
                        } else if lava_pool {
                            lava
                        } else {
                            grass
                        }
                    } else {
                        AIR
                    };
                    self.world.set_block(q, id);
                }
            }
        }
        let at = |a: i32, b: i32, h: i32| IVec3::new(p.x, ground, p.z) + side * a + dir * b + IVec3::Y * h;
        // A tree, dry grass and a TNT block by the pond for fire; snow for fire to melt.
        for h in 1..=4 {
            self.world.set_block(at(-9, 9, h), log);
        }
        for a in -11i32..=-7 {
            for b in 7i32..=11 {
                for h in 4i32..=6 {
                    if (a + 9).abs() + (b - 9).abs() + (h - 5).abs() < 4 && self.world.block_or_air(at(a, b, h)) == AIR {
                        self.world.set_block(at(a, b, h), leaves);
                    }
                }
            }
        }
        for b in 3..=14 {
            if scene == "fire" {
                self.world.set_block(at(-6 - (b % 3), b, 1), tall);
            }
        }
        if scene == "fire" {
            for a in -5..=-3 {
                for b in 17..=19 {
                    self.world.set_block(at(a, b, 1), snow);
                }
            }
            self.world.set_block(at(8, 3, 1), tnt);
        }
        if scene == "water" {
            // Fires burning on the left bank, bare dirt to turn to mud.
            for b in 4..=6 {
                self.world.set_block(at(-7, b, 0), dirt);
                self.set_fire(at(-7, b, 1));
            }
        }
        // Mobs wading in the pond (the night keeps them from burning in the sun).
        self.mobs.list.clear();
        let wade: &[(i32, i32)] = match scene {
            "shock" => &[(-2, 9), (2, 11), (0, 13), (3, 8)],
            "frost" | "water" => &[(-2, 10), (1, 12)],
            _ => &[],
        };
        for &(a, b) in wade {
            let q = at(a, b, 0).as_vec3() + Vec3::new(0.5, 0.1, 0.5);
            if self.mobs.spawn("zombie", q) {
                let m = self.mobs.list.last_mut().unwrap();
                m.yaw = (side.x as f32).atan2(side.z as f32);
                m.aggro = false;
            }
        }
        self.surv.drops.clear();
        self.magic.mana = 999.0;
        self.time_of_day = 1500.0;
        self.weather.state.raining = false;
        self.pitch = -0.32;
        self.body.flying = true;
        self.body.pos.y += 1.5;
    }

    /// One frame of the MC_ELEMENTS clip: casts on some frames, then the world steps on.
    pub(crate) fn elements_frame(&mut self, k: i32, scene: &str) {
        let (cast_frames, item): (&[i32], &str) = match scene {
            "frost" => (&[2, 6, 10, 14, 18], "frost_staff"),
            "fire" => (&[2, 10, 18, 26, 34], "fire_staff"),
            "shock" => (&[4, 20], "thunder_tome"),
            "water" => (&[2, 6, 10, 14, 18], "water_bolt"),
            "spikes" => (&[2], "ice_spike_staff"),
            _ => (&[], ""),
        };
        if let Some(i) = cast_frames.iter().position(|&c| c == k) {
            // Sweep across the pond (and on to the lava, the tree, the snow); NaN: at a mob.
            let n = f32::NAN;
            let sweep: [(f32, f32); 5] = match scene {
                "frost" => [(-0.35, -0.45), (-0.12, -0.5), (0.12, -0.5), (n, n), (-0.56, -0.17)],
                "fire" => [(0.0, -0.4), (0.78, 0.05), (0.22, 0.02), (0.6, -0.3), (-0.15, -0.5)],
                "water" => [(-0.56, -0.17), (-0.5, -0.19), (n, n), (0.95, -0.24), (0.9, -0.28)],
                "shock" => [(0.0, -0.3), (n, n), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
                _ => [(0.0, -0.5); 5],
            };
            let (dy, pitch) = sweep[i.min(4)];
            let (yaw0, pitch0) = (self.yaw, self.pitch);
            if dy.is_nan() {
                let eye = self.body.eye();
                if let Some(m) = self.mobs.list.iter().find(|m| m.alive()) {
                    let to = m.body.center() - eye;
                    self.yaw = (-to.x).atan2(-to.z);
                    self.pitch = (to.y / to.length()).asin();
                }
            } else {
                self.yaw = yaw0 + dy;
                self.pitch = pitch;
            }
            self.cast_now(item);
            self.yaw = yaw0;
            self.pitch = pitch0;
        }
        let last = cast_frames.last().copied().unwrap_or(0);
        // Real time while the spell flies, then faster so the ice is seen melting back.
        let dt = if k <= last + 12 { 0.05 } else { 0.5 };
        let steps = if dt > 0.1 { 5 } else { 1 };
        for _ in 0..steps {
            let d = dt / steps as f32;
            self.update_magic(d, false);
            self.update_elements(d);
            self.update_spell_extras(d);
            self.fluids.update(&mut self.world, d);
            let me = self.body.pos;
            let _ = self.mobs.update(&self.world, me, true, 0.0, d);
            self.particles.update(&self.world, d);
        }
        self.update_fire(dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schools_have_elements() {
        assert_eq!(of_school(0), El::Fire);
        assert_eq!(of_school(1), El::Frost);
        assert_eq!(of_school(2), El::Shock);
        assert_eq!(of_school(5), El::Shock);
        assert_eq!(of_school(4), El::Nature);
        assert_eq!(of_school(9), El::Arcane);
        assert!(fiery("blaze") && fiery("magma_cube_small") && !fiery("zombie"));
    }
}
