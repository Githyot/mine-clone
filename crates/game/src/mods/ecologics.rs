//! Ecologics idea: thin ice cracks under your feet. Standing or walking on it cracks it a stage
//! at a time (sneaking softly takes longer), and the third crack drops you into the water.
//! Only the block under the player is looked at, once a frame: no ticking blocks.

use glam::{IVec3, Vec3};

use crate::audio::{Material, Sfx};
use crate::Game;

#[derive(Default)]
pub struct Ecologics {
    /// Seconds spent on the cracking block, and which block it is.
    on_ice: f32,
    at: Option<IVec3>,
}

/// Seconds on thin ice before it cracks one more stage.
const CRACK_TIME: f32 = 0.7;

impl Game {
    pub(crate) fn update_ecologics(&mut self, dt: f32) {
        if !self.body.on_ground || self.body.flying {
            self.mods.ecologics.on_ice = 0.0;
            return;
        }
        let below = (self.body.pos - Vec3::new(0.0, 0.1, 0.0)).floor().as_ivec3();
        let id = self.world.block_or_air(below);
        let name = self.reg.get(id).name.as_str();
        let stage = match name {
            "thin_ice" => 0,
            "thin_ice_1" => 1,
            "thin_ice_2" => 2,
            _ => {
                self.mods.ecologics.on_ice = 0.0;
                self.mods.ecologics.at = None;
                return;
            }
        };
        if self.mods.ecologics.at != Some(below) {
            self.mods.ecologics.at = Some(below);
            self.mods.ecologics.on_ice = 0.0;
        }
        let soft = if self.crouch > 0.5 { 0.35 } else { 1.0 };
        self.mods.ecologics.on_ice += dt * soft;
        if self.mods.ecologics.on_ice < CRACK_TIME {
            return;
        }
        self.mods.ecologics.on_ice = 0.0;
        let (layer, tint) = self.block_look(id);
        self.particles.block_bits(below, layer, tint, 6, 0.5);
        let next = match stage {
            0 => self.reg.id("thin_ice_1"),
            1 => self.reg.id("thin_ice_2"),
            _ => self.reg.id("water"),
        };
        if let Some(n) = next {
            self.world.set_block(below, n);
        }
        self.sfx(if stage == 2 { Sfx::Break(Material::Glass) } else { Sfx::Hit(Material::Glass) });
        if stage == 2 {
            self.toast("O gelo fino quebrou!", 2.0);
        }
    }
}
