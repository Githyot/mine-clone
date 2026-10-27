//! Windmill sails ("Hélice de Moinho"): the block is the hub, and the sails are a box model
//! ("windmill_sails" in mobs.ron) turning in front of it, drawn by the entity renderer with one
//! matrix per arm. Villages' windmills carry one, and players can craft and place their own.
//!
//! They turn with the wind: gusting a little, faster in rain and much faster in a storm,
//! quicker high up, slow in a sheltered spot and still in a closed room. Finding them costs
//! nothing per frame: the world keeps their places per chunk (`World::spinners`).

use glam::{IVec3, Mat4, Vec3};
use std::f32::consts::{FRAC_PI_2, PI};

use crate::entity::Bones;
use crate::Game;

/// The wind the sails feel, shared by every windmill (each turns at its own rate of it).
#[derive(Default)]
pub struct Sails {
    /// How far the wind has turned a standard windmill so far (radians; f64 so hours of play
    /// don't make the turning stutter).
    pub turn: f64,
}

/// Parts of the model: the hub, then three boxes per arm (spar, cloth, leading board), all
/// drawn pointing up; each arm gets its quarter turn here.
const ARM_PARTS: usize = 3;

/// Which way each facing (north, east, south, west) turns the model's +Z, the side the sails are on.
const FACING_YAW: [f32; 4] = [PI, FRAC_PI_2, 0.0, -FRAC_PI_2];

/// A small number from a block position, to set windmills apart.
fn hash(p: IVec3) -> f32 {
    let h = (p.x as u32).wrapping_mul(0x9E37_79B1) ^ (p.y as u32).wrapping_mul(0x85EB_CA77) ^ (p.z as u32).wrapping_mul(0xC2B2_AE3D);
    (h >> 8) as f32 / (1u32 << 24) as f32
}

impl Game {
    /// The wind's strength now, in radians per second of a standard windmill.
    fn wind(&self) -> f32 {
        let t = self.now;
        // Slow swells and quicker gusts on top.
        let gust = (t * 0.11).sin() * 0.18 + (t * 0.37 + 1.3).sin() * 0.1 + (t * 0.83).sin().max(0.0) * 0.12;
        let storm = if self.weather.state.thunder { 0.9 } else { 0.0 };
        let base = match self.dim {
            voxel::worldgen::Dim::Overworld => 0.55,
            // Hot air rises in the Nether; the End is nearly still.
            voxel::worldgen::Dim::Nether => 0.3,
            _ => 0.12,
        };
        (base + gust + self.weather.rain * (0.45 + storm)).max(0.05)
    }

    pub(crate) fn windmills_update(&mut self, dt: f32) {
        let w = self.wind();
        self.sails.turn += (w * dt) as f64;
    }

    /// Sail models for the entity renderer, for every windmill within `range` of the camera.
    pub(crate) fn windmill_instances(&self, cam_block: IVec3, eye: Vec3, range: f32) -> Vec<(usize, Bones)> {
        let Some(id) = self.mobs.reg.id("windmill_sails") else { return Vec::new() };
        let def = &self.mobs.reg.defs[id];
        let scale = Mat4::from_scale(Vec3::splat(def.scale / 16.0));
        let reach = range + 6.0;
        let mut out = Vec::new();
        for list in self.world.spinners.values() {
            for &p in list {
                let center = p.as_vec3() + Vec3::splat(0.5);
                if (center - eye).length_squared() > reach * reach {
                    continue;
                }
                let facing = (self.reg.get(self.world.block_or_air(p)).variant & 3) as usize;
                let dir = Vec3::new(FACING_YAW[facing].sin(), 0.0, FACING_YAW[facing].cos());
                let front = p + dir.round().as_ivec3();
                let l = self.world.light(front);
                let open = (l >> 4) as f32 / 15.0;
                // Sheltered sails turn slowly; closed in (no sky at all), they stand still.
                // Higher up the wind is stronger.
                let height = (1.0 + (p.y as f32 - 64.0) / 96.0).clamp(0.7, 1.6);
                let k = hash(p);
                let rate = open * open * height * (0.85 + k * 0.3);
                let angle = ((self.sails.turn * rate as f64) % std::f64::consts::TAU) as f32 + k * PI * 2.0;
                let mut b = Bones { light: [open, (l & 15) as f32 / 15.0, 0.0, 0.0], ..Default::default() };
                let base = Mat4::from_translation(center + dir * 0.5 - cam_block.as_vec3()) * Mat4::from_rotation_y(FACING_YAW[facing]);
                for i in 0..def.parts.len().min(b.m.len()) {
                    let arm = if i == 0 { 0 } else { (i - 1) / ARM_PARTS };
                    let turn = Mat4::from_rotation_z(angle + arm as f32 * FRAC_PI_2);
                    b.m[i] = (base * turn * scale).to_cols_array_2d();
                }
                out.push((id, b));
            }
        }
        out
    }
}
