//! The End: eyes of ender set into a ring of portal frames, the arrival platform, the Void
//! Guardian that heals from the crystals on the pillars, and the way home it leaves behind.
//! Ender pearls throw you where you look; chorus fruit drops you somewhere close by.

use glam::{IVec3, Vec3};
use voxel::raycast::RayHit;
use voxel::worldgen::{end_pillars, Dim, END_EXIT_Y, END_PLATFORM};
use voxel::AIR;

use crate::audio::Sfx;
use crate::Game;

pub const GUARDIAN: &str = "void_guardian";

/// The twelve frame places around a 3x3 opening centred on `c`.
fn ring(c: IVec3) -> impl Iterator<Item = IVec3> {
    (-1..=1).flat_map(move |k| {
        [IVec3::new(k, 0, -2), IVec3::new(k, 0, 2), IVec3::new(-2, 0, k), IVec3::new(2, 0, k)].into_iter().map(move |d| c + d)
    })
}

impl Game {
    /// Where an Overworld traveller lands in the End.
    pub(crate) fn end_arrival(&self) -> Vec3 {
        let (x, y, z) = END_PLATFORM;
        Vec3::new(x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5)
    }

    /// Home after the End: the bed, or the world's spawn.
    pub(crate) fn home_spot(&self) -> Vec3 {
        match self.surv.spawn {
            Some(b) => b.as_vec3() + Vec3::new(0.5, 0.0, 0.5),
            None => Vec3::new(self.spawn_xz.0 as f32 + 0.5, 100.0, self.spawn_xz.1 as f32 + 0.5),
        }
    }

    /// The End's chunks are in: lay the obsidian platform, wake the guardian if it still lives,
    /// open the way home if it doesn't.
    pub(crate) fn arrive_in_end(&mut self) -> Vec3 {
        let (px, py, pz) = END_PLATFORM;
        let obsidian = self.reg.id_or_air("obsidian");
        for dz in -2..=2 {
            for dx in -2..=2 {
                self.world.set_block(IVec3::new(px + dx, py, pz + dz), obsidian);
                for k in 1..4 {
                    self.world.set_block(IVec3::new(px + dx, py + k, pz + dz), AIR);
                }
            }
        }
        if self.boss.defeated.iter().any(|d| d == GUARDIAN) {
            self.open_end_exit();
        } else if !self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].name == GUARDIAN) {
            let y = self.world.surface_y(12, 12).unwrap_or(END_EXIT_Y) + 1;
            self.mobs.spawn(GUARDIAN, Vec3::new(12.5, y as f32, 12.5));
            self.boss.called_help = false;
            self.toast("O Guardião do Vazio espera no centro da ilha", 4.0);
        }
        self.end_arrival()
    }

    /// The bedrock fountain in the middle fills with portal: the way home.
    pub(crate) fn open_end_exit(&mut self) {
        let Some(portal) = self.reg.id("end_portal") else { return };
        let id = self.reg.variant(portal, 2);
        for dz in -2i32..=2 {
            for dx in -2i32..=2 {
                let d2 = dx * dx + dz * dz;
                if d2 > 0 && d2 <= 8 {
                    self.world.set_block(IVec3::new(dx, END_EXIT_Y, dz), id);
                }
            }
        }
    }

    /// Crystals still standing on the pillars (positions), nearest first.
    pub(crate) fn live_crystals(&self) -> Vec<IVec3> {
        let crystal = self.reg.id_or_air("end_crystal");
        end_pillars(self.seed)
            .iter()
            .map(|p| IVec3::from(p.crystal()))
            .filter(|&c| self.world.block_or_air(c) == crystal)
            .collect()
    }

    /// Right-click a portal frame with an eye of ender: set it in; a full ring opens the portal.
    pub(crate) fn use_ender_eye(&mut self, hit: RayHit) -> bool {
        let block = self.world.block_or_air(hit.block);
        if self.reg.get(block).name != "end_portal_frame" {
            return false;
        }
        let filled = self.reg.id_or_air("end_portal_frame_filled");
        self.world.set_block(hit.block, filled);
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.sfx(Sfx::Place(crate::audio::Material::Glass));
        self.anim.start_swing();
        // Is this frame part of a finished ring? Try every opening it could border.
        for dz in -3..=3 {
            for dx in -3..=3 {
                let c = hit.block + IVec3::new(dx, 0, dz);
                let full = ring(c).all(|p| self.world.block_or_air(p) == filled);
                let open = (-1..=1).all(|a| (-1..=1).all(|b| self.world.block_or_air(c + IVec3::new(a, 0, b)) == AIR));
                if full && open {
                    let portal = self.reg.variant(self.reg.id_or_air("end_portal"), 2);
                    for a in -1..=1 {
                        for b in -1..=1 {
                            self.world.set_block(c + IVec3::new(a, 0, b), portal);
                        }
                    }
                    self.sfx(Sfx::Portal);
                    self.toast("O portal do End se abriu", 4.0);
                    log::info!("end portal opened at {c}");
                    return true;
                }
            }
        }
        true
    }

    /// Right-click with an ender pearl: off it flies, and you land where it lands.
    pub(crate) fn throw_ender_pearl(&mut self) -> bool {
        let eye = self.body.eye();
        let f = self.forward();
        let reg = &self.reg;
        let world = &self.world;
        let hit = voxel::raycast::raycast(eye, f, 40.0, |p| world.block(p).is_some_and(|b| b != AIR && reg.get(b).solid));
        let Some(hit) = hit else { return false };
        let to = hit.block + hit.normal;
        let free = |p: IVec3| !self.reg.get(self.world.block_or_air(p)).solid;
        // Land on top of what it hit, or on the floor below the side it hit.
        let mut at = to;
        while at.y > 1 && free(at - IVec3::Y) && (to.y - at.y) < 6 {
            at.y -= 1;
        }
        if !free(at) || !free(at + IVec3::Y) {
            return false;
        }
        self.teleport(at.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
            self.damage(2.5, None);
        }
        self.anim.start_swing();
        true
    }

    /// Chorus fruit: a jump to a random safe spot a few blocks away.
    pub(crate) fn chorus_hop(&mut self) {
        let here = self.body.pos.floor().as_ivec3();
        for _ in 0..16 {
            let dx = (self.surv.rng.f32() * 16.0 - 8.0) as i32;
            let dz = (self.surv.rng.f32() * 16.0 - 8.0) as i32;
            for dy in (-6..=6).rev() {
                let p = here + IVec3::new(dx, dy, dz);
                let solid = |q: IVec3| self.reg.get(self.world.block_or_air(q)).solid;
                if solid(p - IVec3::Y) && !solid(p) && !solid(p + IVec3::Y) && !self.world.is_liquid(self.world.block_or_air(p)) {
                    self.teleport(p.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
                    return;
                }
            }
        }
    }

    fn teleport(&mut self, to: Vec3) {
        let from = self.body.pos;
        let layer = self.glow_layer();
        for k in 0..16 {
            let a = k as f32 * 0.8;
            let v = Vec3::new(a.cos(), 1.0 + (k % 4) as f32 * 0.4, a.sin()) * 0.8;
            self.particles.glow(from + Vec3::Y, v, layer, [0.6, 0.3, 0.9], 0.15, 0.8);
            self.particles.glow(to + Vec3::Y, v, layer, [0.6, 0.3, 0.9], 0.15, 0.8);
        }
        self.body.pos = to;
        self.body.vel = Vec3::ZERO;
        self.fall_peak = to.y;
        self.sfx(Sfx::Portal);
    }

    /// The guardian's extras: crystals heal it (a beam of sparks shows which one), and it calls
    /// void walkers instead of the dead.
    pub(crate) fn update_void_guardian(&mut self, i: usize, dt: f32) {
        if self.dim != Dim::End {
            return;
        }
        let crystals = self.live_crystals();
        let m = &mut self.mobs.list[i];
        if crystals.is_empty() || m.health >= m.max_health {
            return;
        }
        m.health = (m.health + dt * 1.2 * crystals.len().min(4) as f32).min(m.max_health);
        let head = m.body.pos + Vec3::Y * 4.0;
        if self.surv.rng.f32() < dt * 6.0 {
            let c = crystals[(self.surv.rng.f32() * crystals.len() as f32) as usize % crystals.len()];
            let from = c.as_vec3() + Vec3::splat(0.5);
            let layer = self.glow_layer();
            self.particles.glow(from, (head - from) * 0.9, layer, [0.95, 0.6, 1.0], 0.2, 1.1);
        }
    }

    /// Void walkers blink about (Enderman Overhaul idea): an angry one far off jumps to a spot
    /// beside the player, and one that was just hit may vanish to somewhere close by.
    pub(crate) fn update_walkers(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        self.walker_t -= dt;
        if self.walker_t > 0.0 {
            return;
        }
        self.walker_t = 0.5;
        let me = self.body.pos;
        let glow = self.glow_layer();
        for i in 0..self.mobs.list.len() {
            let m = &self.mobs.list[i];
            if !m.alive() || !self.mobs.reg.defs[m.def].name.starts_with("void_walker") {
                continue;
            }
            let (pos, aggro, hit) = (m.body.pos, m.aggro, m.hurt_age < 0.5);
            let far = pos.distance(me) > 7.0;
            let roll = self.surv.rng.f32();
            let to = if aggro && far && roll < 0.25 {
                me
            } else if hit && roll < 0.35 {
                pos
            } else {
                continue;
            };
            for _ in 0..8 {
                let a = self.surv.rng.f32() * std::f32::consts::TAU;
                let r = 2.5 + self.surv.rng.f32() * 4.0;
                let c = (to + Vec3::new(a.cos() * r, 0.0, a.sin() * r)).floor().as_ivec3();
                let solid = |q: IVec3| self.reg.get(self.world.block_or_air(q)).solid;
                let Some(y) = (-4..=4).rev().map(|dy| c.y + dy).find(|&y| {
                    let p = IVec3::new(c.x, y, c.z);
                    solid(p - IVec3::Y) && !solid(p) && !solid(p + IVec3::Y) && !solid(p + IVec3::Y * 2)
                }) else {
                    continue;
                };
                let dest = Vec3::new(c.x as f32 + 0.5, y as f32, c.z as f32 + 0.5);
                for k in 0..10 {
                    let v = Vec3::new((k as f32).cos(), 1.0, (k as f32).sin()) * 0.7;
                    self.particles.glow(pos + Vec3::Y * 1.5, v, glow, [0.6, 0.3, 0.9], 0.12, 0.7);
                    self.particles.glow(dest + Vec3::Y * 1.5, v, glow, [0.6, 0.3, 0.9], 0.12, 0.7);
                }
                let m = &mut self.mobs.list[i];
                m.body.pos = dest;
                m.body.vel = Vec3::ZERO;
                if dest.distance(me) < 20.0 {
                    self.sfx(Sfx::Portal);
                }
                break;
            }
        }
    }

    /// Test scene: a ring of frames in front, eleven eyes in, the last one about to go.
    pub(crate) fn test_end_ring(&mut self) {
        let f = self.forward();
        let c = (self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 6.0).floor().as_ivec3();
        let y = self.world.surface_y(c.x, c.z).unwrap_or(c.y) + 1;
        let c = IVec3::new(c.x, y, c.z);
        let (frame, filled) = (self.reg.id_or_air("end_portal_frame"), self.reg.id_or_air("end_portal_frame_filled"));
        for a in -2..=2 {
            for b in -2..=2 {
                for k in 0..3 {
                    self.world.set_block(c + IVec3::new(a, k, b), AIR);
                }
            }
        }
        let places: Vec<IVec3> = ring(c).collect();
        for (k, p) in places.iter().enumerate() {
            self.world.set_block(*p, if k == 0 { frame } else { filled });
        }
        self.use_ender_eye(RayHit { block: places[0], normal: IVec3::Y, distance: 3.0 });
    }
}
