//! Structure dwellers (When Dungeons Arise and YUNG's idea): a structure's data can list mobs
//! that live inside it. They are spawned once, the first time the player comes near, on free
//! floor inside the building, and the world remembers it so they never come back after being
//! beaten. No per-tick spawn scanning: a few chunks around the player are checked every two
//! seconds.

use glam::{IVec3, Vec3};
use voxel::chunk::ChunkPos;
use voxel::AIR;

use crate::Game;

/// Chunks around the player whose buildings are checked.
const RADIUS: i32 = 3;

impl Game {
    pub(crate) fn update_dwellers(&mut self, dt: f32) {
        self.mods.dwell_t -= dt;
        if self.mods.dwell_t > 0.0 || !self.mobs.enabled {
            return;
        }
        self.mods.dwell_t = 2.0;
        let me = self.body.pos.floor().as_ivec3();
        let (cx, cz) = (me.x.div_euclid(16), me.z.div_euclid(16));
        let gen = self.world.gen.clone();
        let structs = gen.structures();
        let mut todo = Vec::new();
        for dz in -RADIUS..=RADIUS {
            for dx in -RADIUS..=RADIUS {
                let pos = ChunkPos::new(cx + dx, cz + dz);
                // Only once that chunk is really there (the floor must exist to stand on).
                if !self.world.is_meshed(pos) {
                    continue;
                }
                for p in gen.placements_near(pos) {
                    let s = &structs.list[p.index];
                    if s.def.mobs.is_empty() {
                        continue;
                    }
                    let key = [p.origin.x, p.origin.y, p.origin.z, self.dim.as_u8() as i32];
                    if self.mods.save.dwellers.contains(&key) || todo.iter().any(|(k, _, _, _)| *k == key) {
                        continue;
                    }
                    let f = s.turned_footprint(p.turns);
                    let size = IVec3::new(f.x, s.size().y, f.y);
                    todo.push((key, p.origin, size, s.def.mobs.clone()));
                }
            }
        }
        for (key, origin, size, mobs) in todo {
            // Wait until the whole footprint is loaded.
            let corners = [origin, origin + IVec3::new(size.x - 1, 0, size.z - 1)];
            if corners.iter().any(|c| !self.world.is_meshed(ChunkPos::new(c.x.div_euclid(16), c.z.div_euclid(16)))) {
                continue;
            }
            self.mods.save.dwellers.push(key);
            let spots = self.floor_spots(origin, size);
            if spots.is_empty() {
                continue;
            }
            let mut k = 0usize;
            for (name, n) in mobs {
                for _ in 0..n {
                    let at = spots[(k * 7919 + self.surv.rng.below(spots.len() as u32) as usize) % spots.len()];
                    k += 1;
                    self.mobs.spawn(&name, at);
                }
            }
        }
    }

    /// Air cells with two free blocks above solid floor inside a building's box.
    fn floor_spots(&self, origin: IVec3, size: IVec3) -> Vec<Vec3> {
        let mut out = Vec::new();
        for y in 0..size.y {
            for z in (0..size.z).step_by(2) {
                for x in (0..size.x).step_by(2) {
                    let p = origin + IVec3::new(x, y, z);
                    let free = self.world.block_or_air(p) == AIR && self.world.block_or_air(p + IVec3::Y) == AIR;
                    if free && self.reg.get(self.world.block_or_air(p - IVec3::Y)).solid {
                        out.push(p.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
                    }
                }
            }
        }
        out
    }
}
