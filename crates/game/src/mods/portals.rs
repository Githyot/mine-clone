//! Portals of the mod pack's dimensions: a frame of some block, lit with some item, that takes
//! the player between the Overworld and that dimension at the same x and z (the Aether's
//! glowstone frame lit with water, the Otherside's frame lit with a heart of the deep). The trip
//! itself is the base game's (`travel`), these only find the frame, light it, and build or find
//! the portal on the other side.

use glam::{IVec3, Vec3};
use voxel::block::Shape;
use voxel::worldgen::Dim;
use voxel::AIR;

use crate::audio::Sfx;
use crate::Game;

pub struct PortalDef {
    pub portal: &'static str,
    pub frame: &'static str,
    /// Item that lights it, and what is left in the hand afterwards ("" uses it up).
    pub key: &'static str,
    pub key_left: &'static str,
    pub to: Dim,
    /// Height travellers aim for when nothing is found below them.
    pub arrive_y: i32,
}

pub const PORTALS: [PortalDef; 2] = [
    PortalDef { portal: "aether_portal", frame: "glowstone", key: "water_bucket", key_left: "bucket", to: Dim::Aether, arrive_y: voxel::worldgen::AETHER_ARRIVAL_Y },
    PortalDef { portal: "otherside_portal", frame: "sculk_portal_frame", key: "heart_of_the_deep", key_left: "", to: Dim::Otherside, arrive_y: 60 },
];

pub fn is_mod_portal(name: &str) -> bool {
    PORTALS.iter().any(|p| p.portal == name)
}

/// How the player is arriving in the dimension being loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrive {
    /// Through the portal of `PORTALS[i]`.
    Portal(usize),
    /// Fell out of the bottom of the Aether: lands on the ground below.
    Fall,
}

const MAX_OPENING: i32 = 21;

impl Game {
    /// The dimension code the shaders get in `Globals.weather.z` for the mod dimensions.
    pub(crate) fn mod_dim_uniform(&self) -> f32 {
        match self.dim {
            Dim::Aether => -1.0,
            Dim::Otherside => 1.25,
            _ => 0.0,
        }
    }

    /// Test hook: MC_DIM=aether or otherside starts the game there, at a fresh portal.
    pub(crate) fn mods_dim_test_hook(&mut self) {
        let Ok(want) = std::env::var("MC_DIM") else { return };
        let Some(i) = PORTALS.iter().position(|p| p.portal.starts_with(want.as_str())) else { return };
        if self.dim != Dim::Overworld {
            return;
        }
        let p = self.body.pos;
        self.load_dim(PORTALS[i].to, true);
        self.mods.arriving = Some(Arrive::Portal(i));
        self.arrival = Some(Vec3::new(p.x, PORTALS[i].arrive_y as f32, p.z));
        self.body.pos = self.arrival.unwrap();
        self.needs_spawn = false;
    }

    /// Right click with a portal's key on its frame: light it.
    pub(crate) fn light_mod_portal(&mut self, item: &str) -> bool {
        let Some(hit) = self.target else { return false };
        let frame = self.reg.get(self.world.block_or_air(hit.block)).name.clone();
        let Some(i) = PORTALS.iter().position(|p| p.key == item && p.frame == frame) else { return false };
        let def = &PORTALS[i];
        let start = hit.block + hit.normal;
        if self.world.block_or_air(start) != AIR {
            return false;
        }
        let Some(portal) = self.reg.id(def.portal) else { return false };
        for axis in 0..2u8 {
            if let Some(cells) = self.frame_opening(start, axis, def.frame) {
                let id = self.reg.variant(portal, axis);
                for c in cells {
                    self.world.set_block(c, id);
                }
                self.sfx(Sfx::Portal);
                if self.survival {
                    if def.key_left.is_empty() {
                        let sel = self.selected;
                        self.surv.inv.take(sel, 1);
                    } else {
                        self.swap_held(def.key_left);
                    }
                }
                self.anim.start_swing();
                return true;
            }
        }
        false
    }

    /// The air inside a frame of `frame` blocks around `start`, in the x-y (axis 0) or z-y plane.
    fn frame_opening(&self, start: IVec3, axis: u8, frame: &str) -> Option<Vec<IVec3>> {
        let side = if axis == 0 { IVec3::X } else { IVec3::Z };
        let mut seen = std::collections::HashSet::new();
        let mut todo = vec![start];
        let (mut lo, mut hi) = (start, start);
        while let Some(p) = todo.pop() {
            if !seen.insert(p) {
                continue;
            }
            lo = lo.min(p);
            hi = hi.max(p);
            let span = hi - lo;
            if span.x.max(span.z) >= MAX_OPENING || span.y >= MAX_OPENING {
                return None;
            }
            for d in [side, -side, IVec3::Y, IVec3::NEG_Y] {
                let q = p + d;
                if seen.contains(&q) {
                    continue;
                }
                let id = self.world.block(q)?;
                if id == AIR {
                    todo.push(q);
                } else if self.reg.get(id).name != frame {
                    return None;
                }
            }
        }
        let span = hi - lo;
        if span.x.max(span.z) < 1 || span.y < 2 {
            return None;
        }
        Some(seen.into_iter().collect())
    }

    /// Which mod portal the player stands in.
    fn mod_portal_here(&self) -> Option<usize> {
        let p = self.body.pos;
        [p, p + Vec3::Y * 0.2, p + Vec3::Y].iter().find_map(|q| {
            let b = self.reg.get(self.world.block_or_air(q.floor().as_ivec3()));
            if b.shape != Shape::Portal {
                return None;
            }
            PORTALS.iter().position(|d| d.portal == b.name)
        })
    }

    /// Called first by `portal_step`: true when a mod portal (or falling out of the Aether)
    /// took care of this frame.
    pub(crate) fn mod_portal_step(&mut self, dt: f32) -> bool {
        if self.dim == Dim::Aether && self.body.pos.y < -10.0 {
            let p = self.body.pos;
            self.mods.arriving = Some(Arrive::Fall);
            self.toast("Você caiu do Aether!", 3.0);
            self.travel(Dim::Overworld, Vec3::new(p.x, 120.0, p.z));
            return true;
        }
        let Some(i) = self.mod_portal_here() else { return false };
        if self.portal_t < 0.0 {
            self.portal_t = -0.5;
            return true;
        }
        if self.portal_t == 0.0 {
            self.sfx(Sfx::Portal);
        }
        self.portal_t += dt;
        let need = if self.survival { 4.0 } else { 1.0 };
        if self.portal_t >= need {
            self.portal_t = 0.0;
            let def = &PORTALS[i];
            let to = if self.dim == def.to { Dim::Overworld } else { def.to };
            let p = self.body.pos;
            let y = if to == Dim::Overworld { p.y.clamp(50.0, 200.0) } else { def.arrive_y as f32 };
            self.mods.arriving = Some(Arrive::Portal(i));
            self.travel(to, Vec3::new(p.x, y, p.z));
        }
        true
    }

    /// Called by `finish_travel`: where a traveller through a mod portal (or out of the
    /// Aether) lands.
    pub(crate) fn mod_arrival(&mut self, at: Vec3) -> Option<Vec3> {
        let how = self.mods.arriving.take()?;
        let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
        match how {
            Arrive::Fall => {
                let y = self.world.surface_y(x, z).map(|y| y + 1).unwrap_or(at.y as i32);
                Some(Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5))
            }
            Arrive::Portal(i) => {
                let def = &PORTALS[i];
                Some(self.find_mod_portal(def.portal, at).unwrap_or_else(|| self.build_mod_portal(i, at)))
            }
        }
    }

    fn find_mod_portal(&self, name: &str, at: Vec3) -> Option<Vec3> {
        let c = at.floor().as_ivec3();
        let mut best: Option<(f32, IVec3)> = None;
        for dz in -16..=16 {
            for dx in -16..=16 {
                for y in 1..250 {
                    let p = IVec3::new(c.x + dx, y, c.z + dz);
                    if self.reg.get(self.world.block_or_air(p)).name != name || self.reg.get(self.world.block_or_air(p - IVec3::Y)).name == name {
                        continue;
                    }
                    let d = (p - c).as_vec3().length_squared();
                    if best.is_none_or(|(b, _)| d < b) {
                        best = Some((d, p));
                    }
                }
            }
        }
        best.map(|(_, p)| p.as_vec3() + Vec3::new(0.5, 0.0, 0.5))
    }

    /// Where a new portal stands: the ground under the traveller, a cave floor near them in the
    /// Otherside, or a platform in the air.
    fn mod_portal_ground(&self, def: &PortalDef, at: Vec3) -> IVec3 {
        let c = at.floor().as_ivec3();
        if self.dim != Dim::Otherside {
            return match self.world.surface_y(c.x, c.z) {
                Some(y) if y > 2 => IVec3::new(c.x, y + 1, c.z),
                _ => IVec3::new(c.x, def.arrive_y, c.z),
            };
        }
        let free = |p: IVec3| self.world.block_or_air(p) == AIR;
        let solid = |p: IVec3| self.reg.get(self.world.block_or_air(p)).solid;
        let mut best: Option<(i32, IVec3)> = None;
        for dz in (-12..=12).step_by(2) {
            for dx in (-12..=12).step_by(2) {
                for y in 10..120 {
                    let p = IVec3::new(c.x + dx, y, c.z + dz);
                    if !solid(p - IVec3::Y) || !(0..4).all(|k| free(p + IVec3::Y * k)) {
                        continue;
                    }
                    let score = dx * dx + dz * dz + (y - c.y).pow(2);
                    if best.is_none_or(|(b, _)| score < b) {
                        best = Some((score, p));
                    }
                }
            }
        }
        best.map(|(_, p)| p).unwrap_or(IVec3::new(c.x, def.arrive_y, c.z))
    }

    /// A lit frame with a little floor in front and behind; returns where the feet go.
    fn build_mod_portal(&mut self, i: usize, at: Vec3) -> Vec3 {
        let def = &PORTALS[i];
        let g = self.mod_portal_ground(def, at);
        let frame = self.reg.id_or_air(def.frame);
        let portal = self.reg.id_or_air(def.portal);
        let floor = if self.dim == Dim::Aether { self.reg.id_or_air("holystone") } else { frame };
        let x0 = g.x - 1;
        for dz in -1..=1 {
            for x in x0 - 1..=x0 + 4 {
                let f = IVec3::new(x, g.y - 1, g.z + dz);
                if !self.reg.get(self.world.block_or_air(f)).solid {
                    self.world.set_block(f, floor);
                }
                for y in g.y..g.y + 4 {
                    self.world.set_block(IVec3::new(x, y, g.z + dz), AIR);
                }
            }
        }
        for x in x0..x0 + 4 {
            for y in g.y - 1..g.y + 4 {
                let edge = x == x0 || x == x0 + 3 || y == g.y - 1 || y == g.y + 3;
                self.world.set_block(IVec3::new(x, y, g.z), if edge { frame } else { portal });
            }
        }
        log::info!("built a {} at {g}", def.portal);
        // The feet land on the +z side: face away from the portal, out over the new world.
        self.yaw = std::f32::consts::PI;
        self.pitch = 0.0;
        Vec3::new(x0 as f32 + 2.0, g.y as f32, g.z as f32 + 1.5)
    }
}
