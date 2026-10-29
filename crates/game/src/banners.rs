//! Tall banners: three blocks high, a long cloth hanging from a bar on a pole, or from a bar
//! against a wall (then the cloth hangs down past the two blocks below). Only the foot is in
//! the block (`Shape::TallBanner`); the terrain leaves it out and the item renderer draws it
//! here, so the cloth can sway.
//!
//! The cloth is a column of strips hinged one under the other. Each strip leans with the wind
//! that blows across the banner's face (wind.rs), and a ripple runs down it, quicker and
//! wider in gusts and storms. A banner out of the sky's reach (indoors) hardly moves. Nothing
//! is kept between frames: the shape is worked out from the clock, the wind and the place.
//!
//! The cloth's picture is twice as tall as it is wide: the block's `top` texture is its upper
//! half, `bottom` its lower half (tools/tex64/banners.py draws them).

use glam::{IVec3, Vec2, Vec3};
use voxel::block::Shape;

use crate::item_render::GpuItem;
use crate::Game;

/// North, east, south, west (`shapes::facing_of`): a banner's cloth faces the other way.
const DIR4: [Vec3; 4] = [Vec3::new(0.0, 0.0, -1.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(-1.0, 0.0, 0.0)];

/// The cloth, in pixels: 20 wide and 40 long, in this many strips.
const WIDTH: f32 = 20.0;
const LENGTH: f32 = 40.0;
const STRIPS: usize = 8;
/// Standing: the pole's height and where the cloth hangs from.
const POLE: f32 = 47.0;
const HANG: f32 = 44.0;
/// On a wall: the bar's height in its block (the cloth hangs below it).
const WALL_HANG: f32 = 14.0;
/// Banners are drawn this far away at most.
const RANGE: f32 = 96.0;

/// A small number from a block position, to set banners apart.
fn hash(p: IVec3) -> f32 {
    let h = (p.x as u32).wrapping_mul(0x9E37_79B1) ^ (p.y as u32).wrapping_mul(0x85EB_CA77) ^ (p.z as u32).wrapping_mul(0xC2B2_AE3D);
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// A vector in the banner's own frame (+z the way its cloth faces) turned by `yaw`, as the item
/// shader turns its boxes.
fn turn(v: Vec3, yaw: f32) -> Vec3 {
    let (s, c) = yaw.sin_cos();
    Vec3::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
}

/// How far down the cloth strip `i` of `STRIPS` leans out from hanging straight (radians,
/// positive toward the way the cloth faces). `lean`: the wind's push across the face, `flutter`:
/// how wide the ripples run, `rate` how quick, `t` the clock, `phase` this banner's own.
pub fn strip_angle(i: usize, lean: f32, flutter: f32, rate: f32, t: f32, phase: f32) -> f32 {
    let k = (i + 1) as f32 / STRIPS as f32;
    // The lower cloth swings further out; ripples start small at the bar and grow down it.
    let wave = (t * rate - i as f32 * 0.85 + phase * 6.3).sin() + 0.35 * (t * rate * 1.7 - i as f32 * 1.4 + phase * 2.0).sin();
    let breath = 0.035 * (t * 0.8 + phase * 6.3).sin();
    lean * k.powf(1.2) + (flutter * wave + breath) * k
}

impl Game {
    /// Boxes for the item renderer: every tall banner near the camera.
    pub(crate) fn banner_instances(&self, cam_block: IVec3, out: &mut Vec<GpuItem>) {
        let eye = self.body.eye();
        let layer = |name: &str, alt: &str| {
            let id = self.reg.id(name).or_else(|| self.reg.id(alt)).unwrap_or(voxel::AIR);
            self.reg.get(id).faces[0] as f32
        };
        let (pole_tex, bar_tex, cap_tex) = (layer("stripped_oak_log", "oak_log"), layer("oak_planks", "oak_log"), layer("gold_block", "oak_planks"));
        for list in self.world.spinners.values() {
            for &p in list {
                let b = self.reg.get(self.world.block_or_air(p));
                if b.shape != Shape::TallBanner || (p.as_vec3() + Vec3::splat(0.5) - eye).length_squared() > RANGE * RANGE {
                    continue;
                }
                let wall = b.variant & 4 != 0;
                let front = -DIR4[(b.variant & 3) as usize];
                let yaw = front.x.atan2(front.z);
                let foot = p.as_vec3() + Vec3::new(0.5, 0.0, 0.5) - cam_block.as_vec3();
                let light = |q: IVec3| {
                    let l = self.world.light(q);
                    ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0)
                };
                let here = light(p);

                // The wood: pole, bar and a cap on top, or just the bar against the wall.
                let mut wood = |bmin: [f32; 3], bmax: [f32; 3], tex: f32| {
                    out.push(GpuItem {
                        pos: [foot.x, foot.y, foot.z, 1.0],
                        info: [yaw, 0.0, here.0, here.1],
                        layers: [tex, tex, tex, 0.0],
                        rot: [0.0; 4],
                        bmin: [bmin[0] / 16.0, bmin[1] / 16.0, bmin[2] / 16.0, 0.0],
                        bmax: [bmax[0] / 16.0, bmax[1] / 16.0, bmax[2] / 16.0, 0.0],
                        piv: [0.5, 0.0, 0.5, 1.0],
                    });
                };
                let hinge = if wall {
                    wood([-2.0, WALL_HANG - 1.0, 0.0], [18.0, WALL_HANG + 1.0, 2.5], bar_tex);
                    Vec3::new(0.0, WALL_HANG, -6.5)
                } else {
                    wood([7.0, 0.0, 6.0], [9.0, POLE, 8.0], pole_tex);
                    wood([-2.0, HANG - 1.0, 6.5], [18.0, HANG + 1.0, 9.5], bar_tex);
                    wood([6.0, POLE, 5.0], [10.0, POLE + 2.0, 9.0], cap_tex);
                    Vec3::new(0.0, HANG, 0.5)
                };

                // The wind across the banner's face, where the sky reaches it.
                let at = p.as_vec3();
                let w = crate::wind::at(self.now, Vec2::new(at.x, at.z), self.weather.rain, self.weather.state.thunder);
                let sky = light(p + IVec3::Y).0.max(here.0);
                let open = sky * sky;
                let push = w.dot(front) * open;
                let strength = w.length() * open;
                let gust = crate::wind::gust(self.now, Vec2::new(at.x, at.z));
                let mut lean = (push * 0.32).clamp(-0.95, 0.95);
                // A wall stops it swinging back; a pole only a little.
                lean = if wall { lean.max(0.0) } else { lean.max(-0.3) };
                let flutter = (0.015 + 0.08 * strength.min(3.0) * (0.5 + gust)) * open.max(0.15);
                let rate = 1.6 + 1.6 * strength.min(3.0);
                let phase = hash(p);

                let seg = LENGTH / STRIPS as f32 / 16.0;
                let mut pivot = foot + turn(hinge / 16.0, yaw);
                for i in 0..STRIPS {
                    let mut a = strip_angle(i, lean, flutter, rate, self.now, phase);
                    if wall {
                        a = a.max(0.0);
                    }
                    let half = i * 2 / STRIPS;
                    let tex = if half == 0 { b.faces[2] } else { b.faces[3] } as f32;
                    let per = STRIPS as f32 / 2.0;
                    let v0 = (i as f32 % per) / per;
                    let mid = pivot + turn(Vec3::new(0.0, -seg * 0.5 * a.cos(), seg * 0.5 * a.sin()), yaw);
                    let l = light((mid + cam_block.as_vec3()).floor().as_ivec3());
                    let l = if l.0 + l.1 > 0.0 { l } else { here };
                    out.push(GpuItem {
                        pos: [pivot.x, pivot.y, pivot.z, 1.0],
                        info: [yaw, 0.0, l.0, l.1],
                        layers: [tex, tex, tex, 0.0],
                        // The shader's pitch swings the bottom back (-z), so lean is its negative.
                        rot: [-a, 0.0, 0.0, 0.0],
                        bmin: [-WIDTH / 32.0, -seg, -0.5 / 16.0, v0],
                        bmax: [WIDTH / 32.0, 0.0, 0.5 / 16.0, v0 + 1.0 / per],
                        piv: [0.0, 0.0, 0.0, 4.0],
                    });
                    pivot += turn(Vec3::new(0.0, -seg * a.cos(), seg * a.sin()), yaw);
                }
            }
        }
    }
}

impl Game {
    /// Test hook MC_BANNERS=calm|storm|side|storm-side: a stone yard with a row of standing banners and three
    /// hung on a wall behind them (from five blocks up), the camera a few blocks in front.
    /// `storm` turns the weather to a thunderstorm so the cloth flies.
    pub(crate) fn build_banner_test(&mut self) {
        let p = self.body.pos.floor().as_ivec3();
        let g = |n: &str| self.reg.id_or_air(n);
        let (stone, bricks) = (g("stone"), g("stone_bricks"));
        let z = p.z - 6;
        for x in -8..=8 {
            for dz in -3..=8 {
                self.world.set_block(IVec3::new(p.x + x, p.y - 1, z + dz), stone);
                for y in 0..8 {
                    self.world.set_block(IVec3::new(p.x + x, p.y + y, z + dz), voxel::AIR);
                }
            }
            for y in 0..7 {
                self.world.set_block(IVec3::new(p.x + x, p.y + y, z - 3), bricks);
            }
        }
        for (k, name) in ["red_banner", "hunt_banner", "blue_banner", "yellow_banner", "white_banner", "green_banner", "black_banner"].iter().enumerate() {
            let id = g(name);
            self.world.set_block(IVec3::new(p.x - 6 + k as i32 * 2, p.y, z), self.reg.variant(id, 0));
        }
        for (k, name) in ["purple_banner", "hunt_banner", "orange_banner"].iter().enumerate() {
            let id = g(name);
            self.world.set_block(IVec3::new(p.x - 4 + k as i32 * 4, p.y + 5, z - 2), self.reg.variant(id, 4));
        }
        let mode = std::env::var("MC_BANNERS").unwrap_or_default();
        if mode.contains("storm") {
            self.weather.state.raining = true;
            self.weather.state.thunder = true;
            self.weather.rain = 1.0;
        }
        // `side`: from the east end of the row (point the view with --look).
        self.body.pos = if mode.contains("side") { Vec3::new(p.x as f32 + 11.5, p.y as f32 + 1.0, z as f32 + 2.5) } else { Vec3::new(p.x as f32 + 0.5, p.y as f32, z as f32 + 8.5) };
        self.body.flying = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloth_hangs_still_without_wind_and_leans_with_it() {
        for i in 0..STRIPS {
            assert!(strip_angle(i, 0.0, 0.0, 2.0, 3.0, 0.2).abs() < 0.05);
        }
        // Leaning, the lower strips go further out than the upper ones.
        let top = strip_angle(0, 0.8, 0.0, 2.0, 0.0, 0.0);
        let bottom = strip_angle(STRIPS - 1, 0.8, 0.0, 2.0, 0.0, 0.0);
        assert!(bottom > top && bottom > 0.7);
    }

    #[test]
    fn the_cloth_is_three_blocks_tall_with_its_pole() {
        assert!(POLE + 2.0 >= 48.0, "a standing banner reaches three blocks up");
        assert!(HANG - LENGTH <= 6.0, "the cloth comes down near the ground");
        // On a wall the cloth hangs down past the two blocks below.
        assert!(WALL_HANG - LENGTH < -16.0);
    }
}
