//! Visitors that come and go: a goblin trader who turns up in caves for a few minutes (ideas
//! from Goblin Traders) and the mutant zombie, a rare night mini-boss that leaps and slams the
//! ground (ideas from Mutant Monsters).

use glam::{IVec3, Vec3};

use crate::audio::Sfx;
use crate::Game;

#[derive(Default)]
pub struct Wanderers {
    check_t: f32,
    /// Seconds the goblin has left before he packs up.
    goblin_left: f32,
    /// Seconds until the mutant can leap again.
    slam_cd: f32,
    /// Seconds until the mutant skeleton's next volley.
    volley_cd: f32,
    /// A leap in the air: lands with a slam.
    leaping: bool,
    seed: u64,
}

impl Wanderers {
    fn rand(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.seed >> 40) as f32 / (1u64 << 24) as f32
    }
}

impl Game {
    fn mob_index(&self, name: &str) -> Option<usize> {
        let def = self.mobs.reg.id(name)?;
        self.mobs.list.iter().position(|m| m.def == def && m.alive())
    }

    /// A free spot (floor under, two air cells) around `center`, dark (`cave`) or under the sky.
    pub(crate) fn spot_near(&mut self, center: Vec3, min: f32, max: f32, cave: bool) -> Option<Vec3> {
        for _ in 0..40 {
            let a = self.wander.rand() * std::f32::consts::TAU;
            let r = min + (max - min) * self.wander.rand();
            let x = (center.x + a.cos() * r).floor() as i32;
            let z = (center.z + a.sin() * r).floor() as i32;
            let y0 = center.y.floor() as i32;
            for dy in -6..=6 {
                let p = IVec3::new(x, y0 + dy, z);
                let floor = self.world.block_or_air(p - IVec3::Y);
                let free = |q: IVec3| self.world.block(q).is_some_and(|b| !self.reg.get(b).solid && !self.world.is_liquid(b));
                if self.reg.is_opaque(floor) && free(p) && free(p + IVec3::Y) {
                    let sky = self.world.light(p + IVec3::Y) >> 4;
                    if (cave && sky == 0) || (!cave && sky >= 8) {
                        return Some(p.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
                    }
                }
            }
        }
        None
    }

    pub(crate) fn update_wanderers(&mut self, dt: f32, daylight: f32) {
        if dt == 0.0 {
            return;
        }
        self.update_goblin(dt);
        self.update_mutant(dt);
        self.update_mutant_skeleton(dt);
        self.wander.check_t -= dt;
        if self.wander.check_t > 0.0 {
            return;
        }
        self.wander.check_t = 30.0;
        let me = self.body.pos;
        // Deep in a cave: now and then a goblin trader sets up nearby.
        let in_cave = self.world.light((me + Vec3::Y).floor().as_ivec3()) >> 4 == 0 && me.y < 60.0;
        if in_cave && self.mob_index("goblin_trader").is_none() && self.wander.rand() < 0.15 {
            if let Some(at) = self.spot_near(me, 6.0, 14.0, true) {
                if self.mobs.spawn("goblin_trader", at) {
                    self.wander.goblin_left = 300.0;
                    self.toast("Um mercador goblin apareceu por perto!", 4.0);
                }
            }
        }
        // On a dark night in survival, rarely, a mutant zombie or skeleton comes looking.
        // (The mutant stalker and void walker come from the mod pack, when it has them.)
        const MUTANTS: [&str; 4] = ["mutant_zombie", "mutant_skeleton", "mutant_stalker", "mutant_void_walker"];
        let none = MUTANTS.iter().all(|n| self.mob_index(n).is_none());
        if self.survival && daylight < 0.2 && none && self.wander.rand() < 0.04 {
            let which = MUTANTS[((self.wander.rand() * 4.0) as usize).min(3)];
            if let Some(at) = self.spot_near(me, 20.0, 30.0, false) {
                if self.mobs.spawn(which, at) {
                    self.toast("Algo muito grande ronda por aqui...", 4.0);
                }
            }
        }
    }

    fn update_goblin(&mut self, dt: f32) {
        let Some(i) = self.mob_index("goblin_trader") else { return };
        self.wander.goblin_left -= dt;
        let far = self.mobs.list[i].body.pos.distance(self.body.pos) > 80.0;
        if self.wander.goblin_left <= 0.0 || far {
            self.mobs.list.remove(i);
        }
    }

    fn update_mutant(&mut self, dt: f32) {
        let Some(i) = self.mob_index("mutant_zombie") else {
            self.wander.leaping = false;
            return;
        };
        self.wander.slam_cd -= dt;
        let me = self.body.pos;
        let hunted = self.hunted();
        let m = &mut self.mobs.list[i];
        let d = me - m.body.pos;
        let flat = Vec3::new(d.x, 0.0, d.z);
        if self.wander.leaping {
            if m.body.on_ground {
                // The slam: a ring of dust and a blow to everyone close.
                self.wander.leaping = false;
                let at = m.body.pos;
                let ground = (at - Vec3::Y * 0.5).floor().as_ivec3();
                let floor = self.world.block_or_air(ground);
                let (layer, tint) = self.block_look(floor);
                for k in 0..8 {
                    let a = k as f32 / 8.0 * std::f32::consts::TAU;
                    let p = (at + Vec3::new(a.cos(), -0.5, a.sin()) * 2.0).floor().as_ivec3();
                    self.particles.block_bits(p, layer, tint, 4, 1.2);
                }
                self.sfx(Sfx::Explode);
                self.pads.rumble(0.8, 0.8, 0.3);
                if (me - at).length() < 5.0 && self.body.on_ground {
                    self.damage(7.0, Some(at));
                    self.body.vel.y = 7.0;
                }
            }
            return;
        }
        if self.wander.slam_cd <= 0.0 && m.body.on_ground && flat.length() > 3.0 && flat.length() < 10.0 && m.aggro && hunted {
            // Leap toward the player.
            let v = flat.normalize() * flat.length().min(8.0) * 1.1;
            m.body.vel = Vec3::new(v.x, 11.0, v.z);
            m.body.on_ground = false;
            self.wander.leaping = true;
            self.wander.slam_cd = 6.0;
        }
    }

    /// The mutant skeleton: besides its normal shots, every few seconds it looses a fan of five
    /// arrows at the player.
    fn update_mutant_skeleton(&mut self, dt: f32) {
        let Some(i) = self.mob_index("mutant_skeleton") else { return };
        self.wander.volley_cd -= dt;
        let m = &self.mobs.list[i];
        let from = m.body.pos + Vec3::Y * 2.9;
        let target = self.body.pos + Vec3::Y * 0.9;
        let d = target - from;
        let flat = Vec3::new(d.x, 0.0, d.z);
        if self.wander.volley_cd > 0.0 || !m.aggro || !self.hunted() || flat.length() > 20.0 || flat.length() < 3.0 {
            return;
        }
        self.wander.volley_cd = 7.0;
        let speed = 18.0;
        let t = (flat.length() / speed).max(0.05);
        let side = Vec3::new(-flat.z, 0.0, flat.x).normalize_or_zero();
        for k in -2..=2 {
            let aim = target + side * k as f32 * 1.4;
            let d = aim - from;
            let vel = Vec3::new(d.x / t, d.y / t + 0.5 * crate::mobs::ARROW_GRAVITY * t, d.z / t);
            self.mobs.arrows.push(crate::mobs::Arrow { pos: from, vel, age: 0.0, stuck: false, damage: 4.0, shooter: 0 });
        }
        self.sfx(Sfx::Place(crate::audio::Material::Cloth));
    }
}
