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

#[derive(Default)]
pub struct FireFx {
    /// Fire blocks near the player: (block, soul fire).
    spots: Vec<(IVec3, bool)>,
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
                    if id != voxel::block::AIR && (id == fire || id == soul) {
                        found.push((p, id == soul));
                    }
                }
            }
        }
        found.sort_by_key(|(p, _)| (*p - me).length_squared());
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
            let (p, soul) = self.fire_fx.spots[k % self.fire_fx.spots.len()];
            let r = &mut self.surv.rng;
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
