//! Furniture you can use: sitting on chairs (ideas from Handcrafted and Macaw's Furniture).

use glam::{IVec3, Vec3};
use voxel::block::Shape;

use crate::Game;

impl Game {
    /// Right click on a chair: sit down on it.
    pub(crate) fn sit_use(&mut self) -> bool {
        if self.crouch > 0.5 || self.riding.is_some() || self.seat.is_some() {
            return false;
        }
        let Some(hit) = self.target else { return false };
        let id = self.world.block_or_air(hit.block);
        if !matches!(self.reg.get(id).shape, Shape::Chair | Shape::Bench) {
            return false;
        }
        // No room to sit with a block right over the seat.
        let above = self.world.block_or_air(hit.block + IVec3::Y);
        if self.reg.get(above).solid {
            return false;
        }
        self.seat = Some(hit.block);
        self.body.vel = Vec3::ZERO;
        self.toast("Agache (Shift) para levantar", 2.5);
        true
    }

    /// Where the player is while seated: hips on the seat. True while sitting.
    pub(crate) fn seat_step(&mut self, sneak: bool) -> bool {
        let Some(p) = self.seat else { return false };
        let id = self.world.block_or_air(p);
        if !matches!(self.reg.get(id).shape, Shape::Chair | Shape::Bench) || sneak {
            self.seat = None;
            self.body.pos = p.as_vec3() + Vec3::new(0.5, 0.63, 0.5);
            self.body.vel = Vec3::ZERO;
            return false;
        }
        self.body.pos = p.as_vec3() + Vec3::new(0.5, -0.12, 0.5);
        self.body.vel = Vec3::ZERO;
        self.body.on_ground = true;
        self.body.fall_distance = 0.0;
        true
    }
}
