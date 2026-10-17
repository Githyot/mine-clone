//! Macaw's Fences idea: barbed-wire fences sting whatever pushes against them, the player and
//! mobs alike. Checked twice a second on the few cells each body touches, not per block.

use glam::{IVec3, Vec3};

use crate::Game;

#[derive(Default)]
pub struct Macaws {
    t: f32,
}

impl Game {
    /// Is a barbed-wire fence touching a body standing at `pos` that is `half` wide?
    fn touches_wire(&self, pos: Vec3, half: f32, height: f32, wire: voxel::BlockId) -> bool {
        let lo = (pos - Vec3::new(half + 0.15, 0.0, half + 0.15)).floor().as_ivec3();
        let hi = (pos + Vec3::new(half + 0.15, height.min(2.0), half + 0.15)).floor().as_ivec3();
        for y in lo.y..=hi.y {
            for z in lo.z..=hi.z {
                for x in lo.x..=hi.x {
                    if self.reg.get(self.world.block_or_air(IVec3::new(x, y, z))).base == wire {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub(crate) fn update_macaws(&mut self, dt: f32) {
        self.mods.macaws.t -= dt;
        if self.mods.macaws.t > 0.0 {
            return;
        }
        self.mods.macaws.t = 0.5;
        let Some(wire) = self.reg.id("wired_fence") else { return };
        if self.touches_wire(self.body.pos, 0.3, 1.8, wire) {
            self.damage(1.0, None);
        }
        let me = self.body.pos;
        let hurt: Vec<usize> = (0..self.mobs.list.len())
            .filter(|&i| {
                let m = &self.mobs.list[i];
                let d = &self.mobs.reg.defs[m.def];
                m.alive() && !d.flies && (m.body.pos - me).length() < 32.0 && self.touches_wire(m.body.pos, d.width * 0.5, d.height, wire)
            })
            .collect();
        for i in hurt {
            let p = self.mobs.list[i].body.pos;
            self.mobs.list[i].hurt(1.0, p - Vec3::Y);
        }
    }
}
