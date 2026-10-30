//! Fire that moves: the fire and soul fire blocks dance in the terrain shader (`fire_sample`),
//! lava swirls slowly, and every fire near the player throws up licks of flame, embers and
//! smoke, like the torch in hand (torch.rs).

use glam::{IVec3, Vec3};

use crate::Game;

/// How far around the player fires are looked for, and how often.
const RANGE: i32 = 16;
const RISE: i32 = 8;
const SCAN_EVERY: f32 = 0.5;
/// Most fires that throw particles at once (the nearest).
const MOST: usize = 40;

#[derive(Clone, Copy)]
enum Flame {
    Fire,
    Soul,
    /// A placed torch: where its flame stands (world), and its colour (None: plain fire).
    Torch(Vec3, Option<[f32; 3]>),
}

#[derive(Clone, Copy)]
struct Spot {
    block: IVec3,
    flame: Flame,
}

/// Where a placed torch's flame stands in its block (torch_quads in the voxel mesher): over the
/// head, or leaning out from a wall for variants 1..4.
fn torch_flame_at(p: IVec3, variant: u8) -> Vec3 {
    let mut q = Vec3::new(8.0, 17.0, 8.0);
    if variant > 0 {
        q.z = q.z - 5.0 + 17.0 * 0.4;
        q.y += 3.0;
        for _ in 0..(variant - 1) & 3 {
            q = Vec3::new(16.0 - q.z, q.y, q.x);
        }
    }
    p.as_vec3() + q / 16.0
}

#[derive(Default)]
pub struct FireFx {
    /// Fires and burning torches near the player.
    spots: Vec<Spot>,
    scan_t: f32,
    owed: f32,
    count: u32,
    /// Screenshots freeze the particles: the fires have been lit up once already.
    warmed: bool,
}

impl Game {
    /// Texture layers + 1 of the animated blocks, for the terrain shader (0 = none).
    pub(crate) fn anim_layers(&self) -> [f32; 4] {
        let layer = |name: &str| {
            let id = self.reg.id_or_air(name);
            if id == voxel::block::AIR { 0.0 } else { self.reg.get(id).faces[0] as f32 + 1.0 }
        };
        [layer("fire"), layer("soul_fire"), layer("lava"), 0.0]
    }

    /// Flames, embers and smoke off the fires around the player.
    pub(crate) fn update_fire_fx(&mut self, dt: f32, shot: bool) {
        self.fire_fx.scan_t -= dt;
        if self.fire_fx.scan_t <= 0.0 || (shot && !self.fire_fx.warmed) {
            self.fire_fx.scan_t = SCAN_EVERY;
            self.scan_fires();
        }
        if shot {
            if !self.fire_fx.warmed && !self.fire_fx.spots.is_empty() {
                self.fire_fx.warmed = true;
                for _ in 0..50 {
                    self.fire_particles(0.02);
                    self.particles.update(&self.world, 0.02);
                }
            }
            return;
        }
        self.fire_particles(dt);
    }

    fn scan_fires(&mut self) {
        let (fire, soul) = (self.reg.id_or_air("fire"), self.reg.id_or_air("soul_fire"));
        let me = self.body.pos.floor().as_ivec3();
        let mut found = Vec::new();
        for y in -RISE..=RISE {
            for z in -RANGE..=RANGE {
                for x in -RANGE..=RANGE {
                    let p = me + IVec3::new(x, y, z);
                    let id = self.world.block_or_air(p);
                    if id == voxel::block::AIR {
                        continue;
                    }
                    let flame = if id == fire {
                        Flame::Fire
                    } else if id == soul {
                        Flame::Soul
                    } else {
                        let b = self.reg.get(id);
                        if b.shape != voxel::block::Shape::Torch {
                            continue;
                        }
                        let Some((_, tint)) = voxel::mesh::torch_flame_look(&self.reg.get(b.base).name) else { continue };
                        let plain = tint == [200, 200, 200];
                        Flame::Torch(torch_flame_at(p, b.variant), (!plain).then(|| tint.map(|c| c as f32 / 200.0 * 1.3)))
                    };
                    found.push(Spot { block: p, flame });
                }
            }
        }
        found.sort_by_key(|s| (s.block - me).length_squared());
        found.truncate(MOST);
        self.fire_fx.spots = found;
    }

    fn fire_particles(&mut self, dt: f32) {
        if self.fire_fx.spots.is_empty() {
            return;
        }
        let glow = self.glow_layer();
        let stone = self.block_look(self.reg.id_or_air("stone")).0;
        // About eight licks of flame a second per fire.
        self.fire_fx.owed += dt * 8.0 * self.fire_fx.spots.len() as f32;
        while self.fire_fx.owed >= 1.0 {
            self.fire_fx.owed -= 1.0;
            self.fire_fx.count = self.fire_fx.count.wrapping_add(1);
            let k = self.fire_fx.count as usize;
            let Spot { block: p, flame } = self.fire_fx.spots[k % self.fire_fx.spots.len()];
            let r = &mut self.surv.rng;
            if let Flame::Torch(top, tint) = flame {
                // Torches burn smaller: every other turn, a little flame off the top.
                if (k / self.fire_fx.spots.len()) % 2 == 1 {
                    continue;
                }
                let at = top + Vec3::new(r.f32() - 0.5, r.f32() * 0.5, r.f32() - 0.5) * 0.12;
                let ember = k % 9 == 0;
                match tint {
                    None => self.particles.flame(at, Vec3::ZERO, glow, 0.045, 0.4, false),
                    Some(c) => self.particles.glow(at, Vec3::new(0.0, 0.9, 0.0), glow, c, 0.04, 0.5),
                }
                if ember {
                    self.particles.spark(at, Vec3::ZERO, glow);
                }
                continue;
            }
            let soul = matches!(flame, Flame::Soul);
            let at = p.as_vec3() + Vec3::new(0.2 + r.f32() * 0.6, 0.15 + r.f32() * 0.45, 0.2 + r.f32() * 0.6);
            if soul {
                self.particles.soul_flame(at, glow, 0.07, 0.5);
            } else {
                self.particles.flame(at, Vec3::ZERO, glow, 0.07, 0.5, false);
            }
            if k % 7 == 0 && !soul {
                self.particles.spark(at + Vec3::Y * 0.3, Vec3::ZERO, glow);
            }
            if k % 5 == 0 {
                let grey = if soul { [0.22, 0.26, 0.3] } else { [0.28, 0.27, 0.26] };
                self.particles.mote(at + Vec3::Y * 0.5, stone, grey, 0.05, -0.8, 1.8);
            }
        }
    }
}
