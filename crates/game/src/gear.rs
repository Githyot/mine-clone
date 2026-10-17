//! Area tools (Silent Gear idea): a hammer breaks the 3x3 of stone around the block you mine,
//! across the face you hit; an excavator does the same for dirt, sand and gravel; a sickle
//! reaps every plant in a 5x5 around. Crouching works one block at a time.

use glam::IVec3;
use voxel::block::{Shape, ToolKind};
use voxel::{BlockId, AIR};

use crate::Game;

impl Game {
    pub(crate) fn area_break(&mut self, center: IVec3, normal: IVec3, id: BlockId) {
        if self.crouch > 0.5 {
            return;
        }
        let Some(name) = self.held_stack().map(|s| self.items.get(s.item).name.clone()) else { return };
        let mut spots: Vec<IVec3> = Vec::new();
        if name.ends_with("_hammer") || name.ends_with("_excavator") {
            let want = if name.ends_with("_hammer") { ToolKind::Pickaxe } else { ToolKind::Shovel };
            let hard = self.reg.get(id).hardness;
            for a in -1..=1 {
                for b in -1..=1 {
                    if a == 0 && b == 0 {
                        continue;
                    }
                    let off = match (normal.x != 0, normal.y != 0) {
                        (true, _) => IVec3::new(0, a, b),
                        (_, true) => IVec3::new(a, 0, b),
                        _ => IVec3::new(a, b, 0),
                    };
                    let p = center + off;
                    let bid = self.world.block_or_air(p);
                    let bd = self.reg.get(bid);
                    // Only blocks this kind of tool is for, no harder than the one mined.
                    if bid != AIR && bd.tool == Some(want) && bd.hardness >= 0.0 && bd.hardness <= hard + 1.5 {
                        spots.push(p);
                    }
                }
            }
        } else if name.ends_with("_sickle") && self.reg.get(id).shape == Shape::Cross {
            for dz in -2..=2 {
                for dx in -2..=2 {
                    for dy in -1..=1 {
                        let p = center + IVec3::new(dx, dy, dz);
                        if p != center && self.reg.get(self.world.block_or_air(p)).shape == Shape::Cross {
                            spots.push(p);
                        }
                    }
                }
            }
        }
        for p in spots {
            let bid = self.world.block_or_air(p);
            if self.held_stack().is_none() || !self.world.break_block(p) {
                continue;
            }
            if self.survival {
                self.surv_block_broken(p, bid);
            }
            let (layer, tint) = self.block_look(bid);
            self.particles.block_bits(p, layer, tint, 6, 0.8);
        }
    }

    /// Right-click with an instrument (Immersive Melodies idea): play one of its tunes, and
    /// little notes float up while it lasts.
    pub(crate) fn play_instrument(&mut self, name: &str) -> bool {
        let Some(i) = ["lute", "flute", "cello"].iter().position(|n| *n == name) else { return false };
        if self.melody_cd > 0.0 {
            return true;
        }
        let tune = (self.surv.rng.f32() * 4.0) as u8 % 4;
        self.sfx(crate::audio::Sfx::Melody(i as u8, tune));
        self.melody_cd = 6.0;
        self.anim.start_swing();
        let glow = self.glow_layer();
        let at = self.body.eye() + self.forward() * 0.6;
        for k in 0..6 {
            let v = glam::Vec3::new((k as f32 * 1.7).cos() * 0.4, 0.8 + k as f32 * 0.1, (k as f32 * 1.7).sin() * 0.4);
            self.particles.glow(at, v, glow, [0.5, 1.0, 0.6], 0.06, 2.0);
        }
        true
    }
}
