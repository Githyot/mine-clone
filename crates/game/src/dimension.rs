//! Dimensions: the Overworld and the Nether, nether portals and the trip between them.
//!
//! Each dimension keeps its own chunks, paintings and placed things (chests, corpses, carts,
//! saplings) in its own folder: the world's folder for the Overworld, `nether/` inside it for
//! the Nether. level.ron stays in the world's folder and says where the player is.

use std::sync::Arc;

use glam::{IVec3, Vec3};
use voxel::block::Shape;
use voxel::mesh::MeshTables;
use voxel::raycast::RayHit;
use voxel::save::WorldSave;
use voxel::worldgen::{Biome, Dim, Generator};
use voxel::AIR;

use crate::audio::Sfx;
use crate::world::World;
use crate::{level_path_hint, paintings, Game, Screen};

/// Biggest portal opening, like the classic game.
const MAX_OPENING: i32 = 21;

impl Game {
    /// Load a dimension of the open world. `carry`: the player walks over with what they have,
    /// only the things placed in that dimension come from its save.
    pub(crate) fn load_dim(&mut self, dim: Dim, carry: bool) {
        let root = level_path_hint(&self.base, &self.world_name);
        let dir = if dim.folder().is_empty() { root } else { root.join(dim.folder()) };
        let save = Arc::new(WorldSave::new(dir));
        let gen = Arc::new(Generator::new(self.seed, &self.reg).with_structures(self.structures.clone()).with_dim(dim).with_spread(self.settings.structure_spread()));
        let tints: Vec<[f32; 3]> = (0..Biome::COUNT).map(|b| Biome::from_u8(b as u8).tint()).collect();
        let tables = Arc::new(MeshTables::new(&self.reg, tints));
        // Multiplayer: changes still waiting go out under the old dimension first.
        self.net_flush_log();
        // Dropping the old world stops its worker threads.
        self.world = World::new(self.reg.clone(), gen.clone(), save.clone(), tables);
        if self.net.active() {
            self.world.net_log = Some(Vec::new());
        }
        self.lod.set_gen(gen);
        self.fluids = crate::fluids::Fluids::new(&self.reg);
        self.redstone = crate::redstone::Redstone::new(self.reg.clone());
        self.riding = None;
        self.boating = None;
        self.seat = None;
        self.tnt.clear();
        self.bobber = None;
        self.renderer.clear_chunks();
        self.mobs.list.clear();
        self.mobs.arrows.clear();
        self.nether_end_loaded();
        self.target = None;
        self.mining = None;
        self.paintings = paintings::load(&save.dir, &self.painting_defs);
        let there = crate::survival::Survival::load(&save.dir, &self.items);
        if carry {
            self.surv.containers = there.containers;
            self.surv.corpses = there.corpses;
            self.surv.carts = there.carts;
            self.surv.boats = there.boats;
            self.surv.saplings = there.saplings;
            self.surv.drops.clear();
            self.surv.orbs.clear();
            self.surv.leaf_decay.clear();
        } else {
            self.surv = there;
        }
        self.dim = dim;
    }

    /// Test scene: a 4x5 obsidian frame six blocks ahead, lit from its bottom edge.
    pub(crate) fn test_portal(&mut self) {
        let f = self.forward();
        let at = (self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 6.0).floor().as_ivec3();
        let g = self.world.surface_y(at.x, at.z).unwrap_or(at.y) + 1;
        let obsidian = self.reg.id_or_air("obsidian");
        for x in 0..4 {
            for y in 0..5 {
                let p = IVec3::new(at.x - 1 + x, g + y, at.z);
                let edge = x == 0 || x == 3 || y == 0 || y == 4;
                self.world.set_block(p, if edge { obsidian } else { AIR });
            }
        }
        let hit = RayHit { block: IVec3::new(at.x, g, at.z), normal: IVec3::Y, distance: 3.0 };
        if !self.light_portal(hit) {
            log::error!("test portal did not light");
        }
    }

    /// The top of the ground at (x, z): the surface, or in the Nether the first floor below `from`.
    pub(crate) fn ground_y(&self, x: i32, z: i32, from: i32) -> Option<i32> {
        if matches!(self.dim, Dim::Overworld | Dim::Aether) {
            return self.world.surface_y(x, z);
        }
        (1..from.min(voxel::worldgen::NETHER_ROOF)).rev().find(|&y| {
            let p = IVec3::new(x, y, z);
            self.reg.get(self.world.block_or_air(p)).solid && self.world.block_or_air(p + IVec3::Y) == AIR
        })
    }

    /// Right-click with flint and steel on obsidian: light the frame it belongs to.
    pub(crate) fn light_portal(&mut self, hit: RayHit) -> bool {
        if self.reg.get(self.world.block_or_air(hit.block)).name != "obsidian" {
            return false;
        }
        let start = hit.block + hit.normal;
        if self.world.block_or_air(start) != AIR {
            return false;
        }
        let Some(portal) = self.reg.id("nether_portal") else { return false };
        for axis in 0..2u8 {
            if let Some(cells) = self.portal_opening(start, axis) {
                let id = self.reg.variant(portal, axis);
                for c in cells {
                    self.world.set_block(c, id);
                }
                self.sfx(Sfx::Portal);
                return true;
            }
        }
        false
    }

    /// The air inside an obsidian frame around `start`, in the x-y plane (axis 0) or z-y plane (1).
    fn portal_opening(&self, start: IVec3, axis: u8) -> Option<Vec<IVec3>> {
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
                } else if self.reg.get(id).name != "obsidian" {
                    return None;
                }
            }
        }
        let span = hi - lo;
        // At least two wide and three tall.
        if span.x.max(span.z) < 1 || span.y < 2 {
            return None;
        }
        Some(seen.into_iter().collect())
    }

    /// Standing in a portal: Some(true) for an end portal, Some(false) for a nether portal.
    fn portal_here(&self) -> Option<bool> {
        let p = self.body.pos;
        [p, p + Vec3::Y * 0.2, p + Vec3::Y].iter().find_map(|q| {
            let b = self.reg.get(self.world.block_or_air(q.floor().as_ivec3()));
            (b.shape == Shape::Portal && !crate::mods::portals::is_mod_portal(&b.name)).then(|| b.name.starts_with("end_"))
        })
    }

    /// Standing in a portal for a few seconds takes the player to the other dimension.
    pub(crate) fn portal_step(&mut self, dt: f32) {
        if self.mod_portal_step(dt) {
            return;
        }
        if self.dim == Dim::End && self.body.pos.y < -24.0 {
            // Fell off into the void.
            if self.survival {
                self.damage(4.0 * dt * 10.0, None);
            } else {
                let at = self.end_arrival();
                self.body.pos = at;
                self.body.vel = Vec3::ZERO;
                self.fall_peak = at.y;
            }
            return;
        }
        let Some(end) = self.portal_here() else {
            self.portal_t = if self.portal_t < 0.0 { (self.portal_t + dt).min(0.0) } else { 0.0 };
            return;
        };
        if self.portal_t < 0.0 {
            // Just came through: step out before going back.
            self.portal_t = -0.5;
            return;
        }
        if self.portal_t == 0.0 {
            self.sfx(Sfx::Portal);
        }
        self.portal_t += dt;
        if end && self.portal_t >= 0.25 {
            // End portals take you at once: to the End, or from it back home.
            self.portal_t = 0.0;
            if self.dim == Dim::End {
                let home = self.home_spot();
                self.arrive_plain = true;
                self.travel(Dim::Overworld, home);
            } else {
                let at = self.end_arrival();
                self.travel(Dim::End, at);
            }
            return;
        }
        let need = if self.survival { 4.0 } else { 1.0 };
        if !end && self.portal_t >= need {
            self.portal_t = 0.0;
            let to = if self.dim == Dim::Overworld { Dim::Nether } else { Dim::Overworld };
            let p = self.body.pos;
            let k = if to == Dim::Nether { 1.0 / 8.0 } else { 8.0 };
            let y = if to == Dim::Nether { p.y.clamp(36.0, 110.0) } else { p.y.clamp(50.0, 200.0) };
            self.travel(to, Vec3::new(p.x * k, y, p.z * k));
        }
    }

    /// How far into a portal trip the player is (0..1), for the purple haze on screen.
    pub(crate) fn portal_haze(&self) -> f32 {
        let need = if self.survival { 4.0 } else { 1.0 };
        (self.portal_t / need).clamp(0.0, 1.0)
    }

    /// Save, load the other dimension and land near `at` (in its portal, or a new one).
    pub(crate) fn travel(&mut self, to: Dim, at: Vec3) {
        self.save_all();
        self.load_dim(to, true);
        self.arrival = Some(at);
        self.body.pos = at;
        self.body.vel = Vec3::ZERO;
        self.needs_spawn = false;
        self.loaded_frames = 0;
        self.set_screen(Screen::Loading);
    }

    /// Died in the Nether: back to the Overworld, then respawn there.
    pub(crate) fn travel_home_after_death(&mut self) {
        self.save_all();
        self.load_dim(Dim::Overworld, true);
        let (sx, sz) = self.spawn_xz;
        self.body.pos = Vec3::new(sx as f32 + 0.5, 100.0, sz as f32 + 0.5);
        if let Some(b) = self.surv.spawn {
            self.body.pos = b.as_vec3();
        }
        self.respawn_home = true;
        self.loaded_frames = 0;
        self.set_screen(Screen::Loading);
    }

    /// The chunks around the landing spot are in: find or build the portal and step out of it.
    pub(crate) fn finish_travel(&mut self) {
        if self.respawn_home {
            self.respawn_home = false;
            self.respawn();
            return;
        }
        let Some(at) = self.arrival.take() else { return };
        let spot = if std::mem::take(&mut self.ne.gateway_landing) {
            self.finish_gateway(at)
        } else if let Some(s) = self.mod_arrival(at) {
            s
        } else if self.dim == Dim::End {
            self.arrive_in_end()
        } else if std::mem::take(&mut self.arrive_plain) {
            if let Some(b) = self.surv.spawn {
                b.as_vec3() + Vec3::new(0.5, 0.0, 0.5)
            } else {
            let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
            let y = self.world.surface_y(x, z).map(|y| y + 1).unwrap_or(at.y as i32);
            Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5)
            }
        } else {
            self.find_portal(at).unwrap_or_else(|| self.build_portal(at))
        };
        self.body.pos = spot;
        self.body.vel = Vec3::ZERO;
        self.unstick();
        let spot = self.body.pos;
        self.fall_peak = spot.y;
        self.portal_t = -0.5;
        self.set_screen(Screen::Playing);
        match self.dim {
            Dim::Nether => self.toast("Nether", 3.0),
            Dim::End => self.toast("O End", 3.0),
            _ => {}
        }
        if self.args.mobs && (std::env::var("MC_NETHER").is_ok() || std::env::var("MC_END").is_ok()) {
            // Test hook: the Nether's creatures lined up in front of the portal.
            self.spawn_demo_mobs();
        }
        if let Ok(what) = std::env::var("MC_NE") {
            // Test hook: the Nether and End's fights, right after arriving.
            self.nether_end_test(&what);
        }
    }

    /// Standing inside solid blocks (a closed Nether cave, a chunk that came in late): move to
    /// the nearest spot with two free blocks, above solid ground if there is one close by.
    pub(crate) fn unstick(&mut self) {
        let solid = |g: &Game, p: IVec3| {
            let id = g.world.block_or_air(p);
            g.reg.get(id).solid
        };
        let fits = |g: &Game, p: IVec3| !solid(g, p) && !solid(g, p + IVec3::Y) && g.world.block(p).is_some();
        let here = self.body.pos.floor().as_ivec3();
        let (lo, hi) = self.body.aabb();
        let blocked = (lo.x.floor() as i32..=hi.x.floor() as i32).any(|x| {
            (lo.z.floor() as i32..=hi.z.floor() as i32).any(|z| (lo.y.floor() as i32..=(hi.y - 0.01).floor() as i32).any(|y| solid(self, IVec3::new(x, y, z))))
        });
        if !blocked {
            return;
        }
        let lava = |g: &Game, p: IVec3| g.world.is_liquid(g.world.block_or_air(p)) && g.reg.get(g.world.block_or_air(p)).name.contains("lava");
        let mut best: Option<(i32, IVec3)> = None;
        for r in 0..=12i32 {
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dz.abs()) != r {
                        continue;
                    }
                    for dy in -16..=16i32 {
                        let p = here + IVec3::new(dx, dy, dz);
                        if !fits(self, p) || lava(self, p) {
                            continue;
                        }
                        // Ground under the feet is worth a lot: no falling into the lava sea.
                        let score = dx * dx + dz * dz + dy * dy + if solid(self, p - IVec3::Y) { 0 } else { 400 };
                        if best.is_none_or(|(b, _)| score < b) {
                            best = Some((score, p));
                        }
                    }
                }
            }
            if best.is_some_and(|(b, _)| b < (r + 1) * (r + 1)) {
                break;
            }
        }
        if let Some((_, p)) = best {
            log::info!("unstuck from {here} to {p}");
            self.body.pos = p.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
            self.body.vel = Vec3::ZERO;
            self.fall_peak = self.body.pos.y;
        }
    }

    /// The nearest portal block (its lowest one) within 16 blocks: the player's feet go there.
    fn find_portal(&self, at: Vec3) -> Option<Vec3> {
        let c = at.floor().as_ivec3();
        let mut best: Option<(f32, IVec3)> = None;
        let top = if self.dim == Dim::Nether { 126 } else { 250 };
        for dz in -16..=16 {
            for dx in -16..=16 {
                for y in 1..top {
                    let p = IVec3::new(c.x + dx, y, c.z + dz);
                    if self.reg.get(self.world.block_or_air(p)).name != "nether_portal" {
                        continue;
                    }
                    if self.reg.get(self.world.block_or_air(p - IVec3::Y)).name == "nether_portal" {
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

    /// Room for a portal: solid ground with four free blocks above, nearest to `at`.
    fn portal_ground(&self, at: Vec3) -> IVec3 {
        let c = at.floor().as_ivec3();
        let free = |p: IVec3| {
            let id = self.world.block_or_air(p);
            id == AIR || !self.reg.get(id).solid && !self.world.is_liquid(id)
        };
        let solid = |p: IVec3| self.reg.get(self.world.block_or_air(p)).solid;
        if self.dim == Dim::Overworld {
            let y = self.world.surface_y(c.x, c.z).unwrap_or(c.y) + 1;
            return IVec3::new(c.x, y, c.z);
        }
        let mut best: Option<(i32, IVec3)> = None;
        for dz in (-12..=12).step_by(2) {
            for dx in (-12..=12).step_by(2) {
                for y in (voxel::worldgen::NETHER_LAVA + 2)..(voxel::worldgen::NETHER_ROOF - 6) {
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
        // Nowhere to stand: hang a platform in the air.
        best.map(|(_, p)| p).unwrap_or(IVec3::new(c.x, c.y.clamp(48, 100), c.z))
    }

    /// An obsidian frame with a lit portal, with a little floor in front and behind.
    fn build_portal(&mut self, at: Vec3) -> Vec3 {
        let g = self.portal_ground(at);
        let obsidian = self.reg.id_or_air("obsidian");
        let portal = self.reg.id_or_air("nether_portal");
        // Frame along x: x0..x0+3 wide, g-1..g+3 tall.
        let x0 = g.x - 1;
        for dz in -1..=1 {
            for x in x0 - 1..=x0 + 4 {
                let floor = IVec3::new(x, g.y - 1, g.z + dz);
                if !self.reg.get(self.world.block_or_air(floor)).solid {
                    self.world.set_block(floor, obsidian);
                }
                for y in g.y..g.y + 4 {
                    let p = IVec3::new(x, y, g.z + dz);
                    self.world.set_block(p, AIR);
                }
            }
        }
        for x in x0..x0 + 4 {
            for y in g.y - 1..g.y + 4 {
                let p = IVec3::new(x, y, g.z);
                let edge = x == x0 || x == x0 + 3 || y == g.y - 1 || y == g.y + 3;
                self.world.set_block(p, if edge { obsidian } else { portal });
            }
        }
        Vec3::new(x0 as f32 + 2.0, g.y as f32, g.z as f32 + 0.5)
    }
}
