//! The rest of the pale garden (Lote 9): eyeblossoms open at night and close by day, and the
//! creaking heart. A heart sits in some pale oak trunks; at night it calls a creaking out near
//! it, and the creakings around it cannot be hurt while it beats. Break the heart and they
//! crumble. It gives resin, which smelts into resin bricks.

use glam::{IVec3, Vec3};
use voxel::{BlockId, BlockRegistry, AIR};

use crate::Game;

pub(crate) struct Pale {
    open: BlockId,
    closed: BlockId,
    heart: BlockId,
}

impl Pale {
    pub fn new(reg: &BlockRegistry) -> Pale {
        Pale { open: reg.id_or_air("open_eyeblossom"), closed: reg.id_or_air("closed_eyeblossom"), heart: reg.id_or_air("creaking_heart") }
    }

    pub fn ticks(&self, id: BlockId) -> bool {
        id != AIR && (id == self.open || id == self.closed || id == self.heart)
    }
}

impl Game {
    pub(crate) fn pale_tick(&mut self, p: IVec3, id: BlockId, r: u32) {
        let night = self.is_night();
        if id == self.pale.open && !night && self.pale.closed != AIR {
            self.world.set_block(p, self.pale.closed);
        } else if id == self.pale.closed && night && self.pale.open != AIR {
            self.world.set_block(p, self.pale.open);
        } else if id == self.pale.heart && night && r % 2 == 0 {
            self.heart_call(p, r);
        }
    }

    /// At night a heart with no creaking near it calls one out of the dark close by.
    fn heart_call(&mut self, heart: IVec3, r: u32) {
        let Some(def) = self.mobs.reg.id("creaking") else { return };
        let c = heart.as_vec3();
        if self.mobs.list.iter().any(|m| m.def == def && m.alive() && m.body.pos.distance(c) < 32.0) {
            return;
        }
        let a = (r >> 8) as f32 * 0.37;
        let d = 4.0 + ((r >> 16) % 8) as f32;
        let mut at = IVec3::new(heart.x + (a.cos() * d) as i32, heart.y + 6, heart.z + (a.sin() * d) as i32);
        for _ in 0..16 {
            let below = self.world.block_or_air(at - IVec3::Y);
            if self.reg.get(below).solid && self.world.block_or_air(at) == AIR && self.world.block_or_air(at + IVec3::Y) == AIR {
                self.mobs.spawn("creaking", at.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
                return;
            }
            at.y -= 1;
        }
    }

    /// A heart broken: the creakings near it crumble.
    pub(crate) fn pale_broken(&mut self, p: IVec3, was: BlockId) {
        if was == AIR || was != self.pale.heart {
            return;
        }
        let Some(def) = self.mobs.reg.id("creaking") else { return };
        let c = p.as_vec3();
        let glow = self.glow_layer();
        for m in self.mobs.list.iter_mut() {
            if m.def == def && m.alive() && m.body.pos.distance(c) < 32.0 {
                m.health = 0.0;
                for k in 0..12 {
                    let a = k as f32 * 0.52;
                    self.particles.glow(m.body.pos + Vec3::new(a.cos() * 0.4, 0.3 + k as f32 * 0.2, a.sin() * 0.4), Vec3::Y * 0.4, glow, [0.95, 0.55, 0.15], 0.06, 1.0);
                }
            }
        }
    }
}

impl Game {
    /// Test hook MC_PALE: the new pale garden and resin blocks in a row in front.
    pub(crate) fn test_pale(&mut self) {
        let f = self.forward();
        let c = (self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 6.0).floor().as_ivec3();
        for dz in -1..4 {
            for dx in -7..8 {
                for y in -1..4 {
                    let id = if y == -1 { self.reg.id_or_air("grass_block") } else { AIR };
                    self.world.set_block(c + IVec3::new(dx, y, dz), id);
                }
            }
        }
        let row = [
            "pale_oak_door:2", "pale_oak_trapdoor", "pale_oak_sign:4", "pale_oak_button:2", "pale_oak_pressure_plate", "open_eyeblossom", "closed_eyeblossom",
            "creaking_heart", "resin_block", "resin_bricks", "resin_brick_slab", "resin_brick_stairs:2", "resin_brick_wall", "chiseled_resin_bricks", "pale_oak_planks",
        ];
        for (k, n) in row.iter().enumerate() {
            let at = c + IVec3::new(k as i32 - 7, 0, 1);
            self.world.set_block(at, self.reg.id_or_air(n));
            if n.starts_with("pale_oak_door") {
                self.world.set_block(at + IVec3::Y, self.reg.id_or_air("pale_oak_door:10"));
            }
        }
    }
}
