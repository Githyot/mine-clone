//! Visual effects of the immersion stage: dust kicked up by running and landing, sparks when a
//! mob is hit and a puff of smoke when it dies, drops falling from wet ceilings, lava spitting,
//! breath fogging in the cold, lightning bolts you can see, morning and swamp mist, a soft
//! vignette (red and pulsing at low health, blue under water, orange in lava) and a little
//! camera shake for blasts and hard landings. All of it is cheap: a few particles and some
//! translucent rectangles. Turning view bobbing off also turns the shake off.

use glam::Vec3;
use voxel::worldgen::{Biome, Dim};
use voxel::AIR;

use crate::Game;
use crate::Screen;

impl Game {
    pub(crate) fn update_fx(&mut self, dt: f32) {
        // Hits shake the view a little, more for a big one.
        let lost = self.fx.prev_health - self.health;
        if self.survival && lost > 0.5 {
            self.fx.shake = self.fx.shake.max((lost / 8.0).min(0.6));
        }
        self.fx.prev_health = self.health;

        // Dust kicked up while running.
        let flat = (self.body.vel * Vec3::new(1.0, 0.0, 1.0)).length();
        if self.sprinting && self.body.on_ground && !self.body.in_water && !self.body.flying {
            self.fx.dust_dist += flat * dt;
            if self.fx.dust_dist > 1.3 {
                self.fx.dust_dist = 0.0;
                self.kick_dust(3, 0.3);
            }
        }

        // Breath fogging in the cold.
        let eye = self.body.eye();
        let biome = self
            .world
            .gen
            .base_biome_at(eye.x.floor() as i32, eye.z.floor() as i32);
        let cold = self.dim == Dim::Overworld
            && (matches!(biome, Biome::Snowy | Biome::Taiga) || eye.y > 150.0);
        if cold && !self.body.head_in_water {
            self.fx.breath_t -= dt;
            if self.fx.breath_t <= 0.0 {
                self.fx.breath_t = 2.6 + self.fx.rand() * 1.2;
                let f = self.forward();
                let at = eye + f * 0.45 - Vec3::Y * 0.15;
                let (layer, _) = self.block_look(self.reg.id_or_air("white_wool"));
                for _ in 0..5 {
                    let j = Vec3::new(
                        self.fx.rand() - 0.5,
                        self.fx.rand() - 0.5,
                        self.fx.rand() - 0.5,
                    ) * 0.12;
                    self.particles
                        .mote(at + j, layer, [0.95, 0.97, 1.0], 0.035, -0.25, 1.1);
                }
            }
        }

        // A lightning bolt to go with the flash and the thunder.
        if let Some(close) = self.weather.thunder_sound {
            self.lightning_bolt(close, None);
        }
    }

    /// A few bits of the ground under the player's feet thrown up.
    fn kick_dust(&mut self, count: usize, burst: f32) {
        let feet = self.body.pos.floor().as_ivec3();
        let below = self.world.block_or_air(feet - glam::IVec3::Y);
        if below == AIR || self.world.is_liquid(below) {
            return;
        }
        let (layer, tint) = self.block_look(below);
        self.particles.block_bits(feet, layer, tint, count, burst);
    }

    /// Landing after a fall: a ring of dust, bigger the harder the landing.
    pub(crate) fn land_dust(&mut self, k: f32) {
        self.kick_dust(4 + (k * 14.0) as usize, 0.5 + k * 0.6);
    }

    /// Sparks off a hit mob, or a puff of smoke where one died.
    pub(crate) fn mob_hit_fx(&mut self, at: Vec3, height: f32, died: bool) {
        let glow = self.glow_layer();
        if died {
            let (layer, _) = self.block_look(self.reg.id_or_air("white_wool"));
            for _ in 0..12 {
                let j = Vec3::new(
                    self.fx.rand() - 0.5,
                    self.fx.rand() * height - height * 0.6,
                    self.fx.rand() - 0.5,
                );
                self.particles
                    .mote(at + j, layer, [0.8, 0.8, 0.8], 0.09, -0.5, 1.3);
            }
        } else {
            for _ in 0..6 {
                let v = Vec3::new(
                    self.fx.rand() - 0.5,
                    self.fx.rand() * 0.8,
                    self.fx.rand() - 0.5,
                ) * 4.0;
                self.particles
                    .glow(at, v, glow, [1.0, 0.75, 0.4], 0.03, 0.35);
            }
        }
    }

    /// Lava throwing up a few embers as a bubble bursts.
    pub(crate) fn lava_spit(&mut self, at: Vec3) {
        let glow = self.glow_layer();
        for _ in 0..3 {
            let v = Vec3::new(
                self.fx.rand() - 0.5,
                2.5 + self.fx.rand() * 2.0,
                self.fx.rand() - 0.5,
            );
            self.particles
                .glow(at, v, glow, [1.0, 0.5, 0.12], 0.045, 0.9);
        }
    }

    /// Smoke drifting up from a fire.
    pub(crate) fn smoke(&mut self, at: Vec3, count: usize) {
        let (layer, _) = self.block_look(self.reg.id_or_air("stone"));
        for _ in 0..count {
            let j = Vec3::new(self.fx.rand() - 0.5, 0.0, self.fx.rand() - 0.5) * 0.4;
            self.particles
                .mote(at + j, layer, [0.3, 0.3, 0.3], 0.08, -0.6, 2.5);
        }
    }

    /// One drop falling from a ceiling (water, or glowing lava in the Nether).
    pub(crate) fn drip_particle(&mut self, at: Vec3, lava: bool) {
        if lava {
            let glow = self.glow_layer();
            self.particles.glow(
                at,
                Vec3::new(0.0, -5.0, 0.0),
                glow,
                [1.0, 0.45, 0.1],
                0.035,
                0.8,
            );
        } else {
            let (layer, _) = self.block_look(self.reg.id_or_air("water"));
            self.particles
                .mote(at, layer, [0.6, 0.75, 1.0], 0.03, 7.0, 3.0);
        }
    }

    /// Smoke, a flash of fire and a jolt of the camera for an explosion.
    pub(crate) fn explosion_fx(&mut self, pos: Vec3, power: f32) {
        let (layer, _) = self.block_look(self.reg.id_or_air("stone"));
        let glow = self.glow_layer();
        let n = (8.0 + power * 4.0) as usize;
        for _ in 0..n {
            let j = Vec3::new(
                self.fx.rand() - 0.5,
                self.fx.rand() - 0.3,
                self.fx.rand() - 0.5,
            ) * power
                * 1.2;
            self.particles
                .mote(pos + j, layer, [0.35, 0.33, 0.32], 0.16, -0.8, 2.5);
            let v = Vec3::new(self.fx.rand() - 0.5, self.fx.rand(), self.fx.rand() - 0.5) * 6.0;
            self.particles
                .glow(pos, v, glow, [1.0, 0.6, 0.2], 0.06, 0.4);
        }
        let d = (self.body.eye() - pos).length();
        let k = (1.0 - d / (power * 8.0)).clamp(0.0, 1.0);
        self.fx.shake = self.fx.shake.max(k * 1.3);
    }

    /// A jagged line of light from the clouds to the ground, far off or close by.
    fn lightning_bolt(&mut self, close: f32, toward: Option<Vec3>) {
        let eye = self.body.eye();
        let a = match toward {
            Some(d) => d.z.atan2(d.x),
            None => self.fx.rand() * std::f32::consts::TAU,
        };
        let dist = 18.0 + (1.0 - close) * 80.0;
        let base = Vec3::new(eye.x + a.cos() * dist, 0.0, eye.z + a.sin() * dist);
        let ground = self
            .world
            .surface_y(base.x.floor() as i32, base.z.floor() as i32)
            .map(|y| y as f32 + 1.0)
            .unwrap_or(eye.y - 4.0);
        let top = ground + 60.0;
        let mut p = Vec3::new(base.x, top, base.z);
        let mut main = vec![p];
        let mut paths = Vec::new();
        while p.y > ground {
            // Short jags, each a few blocks long.
            p += Vec3::new(
                (self.fx.rand() - 0.5) * 3.0,
                -self.fx.range(1.5, 3.5),
                (self.fx.rand() - 0.5) * 3.0,
            );
            p.y = p.y.max(ground);
            main.push(p);
            if self.fx.rand() < 0.18 && paths.len() < 3 && p.y > ground + 12.0 {
                let dir =
                    Vec3::new(self.fx.rand() - 0.5, 0.0, self.fx.rand() - 0.5).normalize_or_zero();
                let mut b = p;
                let mut branch = vec![b];
                for _ in 0..(3 + (self.fx.rand() * 4.0) as usize) {
                    b += dir * self.fx.range(1.5, 3.0)
                        + Vec3::new(
                            (self.fx.rand() - 0.5) * 2.0,
                            -self.fx.range(1.5, 3.0),
                            (self.fx.rand() - 0.5) * 2.0,
                        );
                    branch.push(b);
                }
                paths.push(branch);
            }
        }
        paths.insert(0, main);
        self.fx.bolt = paths;
        self.fx.bolt_t = 0.35;
        // Sparks where it strikes.
        let glow = self.glow_layer();
        for _ in 0..16 {
            let v = Vec3::new(self.fx.rand() - 0.5, self.fx.rand(), self.fx.rand() - 0.5) * 6.0;
            self.particles.glow(p, v, glow, [1.2, 1.2, 1.5], 0.15, 0.5);
        }
    }

    /// Draw the bolt as bright lines over the scene (hidden where terrain is in front).
    fn draw_bolt(&mut self, w: f32, h: f32) {
        if self.fx.bolt_t <= 0.0 || self.fx.bolt.is_empty() {
            return;
        }
        let Some((vp, cam_block)) = self.ui_view_proj else {
            return;
        };
        let eye = self.body.eye();
        let fade = (self.fx.bolt_t / 0.35).clamp(0.0, 1.0);
        // It flickers once or twice while it lasts.
        let flick = if (self.fx.bolt_t * 20.0) as i32 % 3 == 1 {
            0.5
        } else {
            1.0
        };
        let paths = std::mem::take(&mut self.fx.bolt);
        for (k, path) in paths.iter().enumerate() {
            let scr: Vec<Option<(f32, f32)>> = path
                .iter()
                .map(|&p| {
                    let to = p - eye;
                    let d = to.length();
                    let world = &self.world;
                    if voxel::raycast::raycast(eye, to / d.max(0.01), d.min(96.0), |c| {
                        voxel::physics::Terrain::solid(world, c)
                    })
                    .is_some()
                    {
                        return None;
                    }
                    let c = vp * (p - cam_block.as_vec3()).extend(1.0);
                    (c.w > 0.1).then(|| ((c.x / c.w * 0.5 + 0.5) * w, (0.5 - c.y / c.w * 0.5) * h))
                })
                .collect();
            let core = if k == 0 { 3.0 } else { 1.8 } * self.ui.scale.max(1.0) * 0.6;
            for seg in scr.windows(2) {
                let (Some(a), Some(b)) = (seg[0], seg[1]) else {
                    continue;
                };
                let alpha = fade * flick;
                self.line(a, b, core * 3.5, [150, 160, 255, (alpha * 60.0) as u8]);
                self.line(a, b, core, [235, 240, 255, (alpha * 255.0) as u8]);
            }
        }
        self.fx.bolt = paths;
    }

    fn line(&mut self, a: (f32, f32), b: (f32, f32), width: f32, col: [u8; 4]) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let (nx, ny) = (-dy / len * width * 0.5, dx / len * width * 0.5);
        self.ui.quad(
            [
                [a.0 + nx, a.1 + ny],
                [b.0 + nx, b.1 + ny],
                [b.0 - nx, b.1 - ny],
                [a.0 - nx, a.1 - ny],
            ],
            [[0.0; 2]; 4],
            col,
            -2,
        );
    }

    /// Test hook for screenshots (which hold time still): MC_FX=1 fills the air with this
    /// stage's effects at once: full mist, a lightning bolt, breath, sparks and smoke on mobs.
    pub(crate) fn prime_effects(&mut self) {
        if self.fx.primed || self.args.screenshot.is_none() || std::env::var("MC_FX").is_err() {
            return;
        }
        self.fx.primed = true;
        if std::env::var("MC_FX").is_ok_and(|v| v == "low") {
            // Low health: the red vignette at the top of its pulse.
            self.health = 4.0;
            self.fx.prev_health = 4.0;
            self.fx.beat_age = 0.0;
        }
        for _ in 0..40 {
            self.update_mist(1.0);
        }
        let f = self.forward();
        self.lightning_bolt(0.8, Some(f));
        self.fx.breath_t = 0.0;
        self.update_effects(0.05);
        let eye = self.body.eye();
        let near: Vec<(Vec3, f32)> = self
            .mobs
            .list
            .iter()
            .filter(|m| (m.body.pos - eye).length() < 16.0)
            .map(|m| {
                let h = self.mobs.reg.defs[m.def].height;
                (m.body.pos + Vec3::Y * h * 0.8, h)
            })
            .collect();
        for (i, (at, h)) in near.into_iter().enumerate() {
            self.mob_hit_fx(at, h, i % 2 == 1);
        }
        self.land_dust(1.0);
        for _ in 0..4 {
            self.particles.update(&self.world, 0.05);
        }
    }

    /// Where the camera is pushed by shaking (add to the eye position).
    pub(crate) fn shake_offset(&self) -> Vec3 {
        if !self.settings.view_bobbing || self.settings.reduce_motion || self.fx.shake <= 0.0 {
            return Vec3::ZERO;
        }
        let t = self.now;
        let amp = self.fx.shake * self.fx.shake * 0.12;
        Vec3::new(
            (t * 37.1).sin(),
            (t * 41.7 + 1.0).sin(),
            (t * 29.3 + 2.0).sin(),
        ) * amp
    }

    /// Ease the mist in and out: swamps, valleys at dawn, and rain.
    pub(crate) fn update_mist(&mut self, dt: f32) {
        let eye = self.body.eye();
        let mut target = 0.0f32;
        if self.dim == Dim::Overworld && !self.body.head_in_water {
            let head = eye.floor().as_ivec3();
            let sky = self.world.light(head) >> 4;
            let biome = self.world.gen.base_biome_at(head.x, head.z);
            let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
            let dawn = a.cos() > 0.0 && a.sin() > -0.2 && a.sin() < 0.3;
            if sky > 0 {
                if biome == Biome::Swamp {
                    target = 0.55;
                }
                let valley = matches!(
                    biome,
                    Biome::Plains
                        | Biome::Forest
                        | Biome::BirchForest
                        | Biome::Taiga
                        | Biome::DarkForest
                        | Biome::CherryGrove
                        | Biome::Swamp
                );
                if dawn && valley && eye.y < 85.0 {
                    target = target.max(0.45);
                }
                target = target.max(self.weather.rain * 0.3);
            }
        }
        let step = dt * 0.25;
        self.fx.mist += (target - self.fx.mist).clamp(-step, step);
    }

    /// Pull the fog in for mist (Overworld above water only).
    pub(crate) fn mist_fog(&self, start: f32, end: f32) -> (f32, f32) {
        let m = self.fx.mist;
        if m <= 0.0 {
            return (start, end);
        }
        let s = start * (1.0 - 0.8 * m);
        let near_end = (end * 0.35).max(s + 24.0);
        (s, end + (near_end - end) * m)
    }

    /// The vignette: soft dark corners always, red and pulsing at low health, blue under water,
    /// orange in lava.
    pub(crate) fn draw_immersion_overlay(&mut self, w: f32, h: f32) {
        if !self.world_open
            || !matches!(
                self.screen,
                Screen::Playing | Screen::Inventory | Screen::Dead
            )
        {
            return;
        }
        let eye = self.body.eye().floor().as_ivec3();
        let l = self.world.light(eye);
        let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
        let daylight = (a.sin() * 2.2 + 0.45).clamp(0.0, 1.0);
        let bright = (((l >> 4) as f32 * daylight).max((l & 15) as f32) / 15.0).clamp(0.0, 1.0);
        let mut layers: Vec<([u8; 3], f32)> = vec![([0, 0, 0], 0.22 + 0.18 * (1.0 - bright))];
        if self.body.head_in_water {
            layers.push(([5, 20, 60], 0.35));
        }
        let feet = self.body.pos.floor().as_ivec3();
        let in_lava = self.world.fluid(self.world.block_or_air(feet)) == 2
            || self
                .world
                .fluid(self.world.block_or_air(feet + glam::IVec3::Y))
                == 2;
        if in_lava {
            let flicker = 0.85 + 0.15 * (self.now * 17.0).sin();
            layers.push(([255, 110, 20], 0.55 * flicker));
        }
        if self.survival && self.health > 0.0 && self.health <= self.max_health * 0.3 {
            let pulse = (-self.fx.beat_age * 4.0).exp();
            layers.push(([150, 0, 0], 0.25 + 0.3 * pulse));
        }
        self.draw_bolt(w, h);
        const RINGS: usize = 28;
        let t = (w.min(h) * 0.016).max(1.0);
        for (c, strength) in layers {
            for i in 0..RINGS {
                let k = 1.0 - i as f32 / RINGS as f32;
                // Thin rings that don't overlap, fading towards the middle.
                let alpha = (strength * k.powf(2.5) * 255.0).clamp(0.0, 255.0) as u8;
                if alpha == 0 {
                    continue;
                }
                let col = [c[0], c[1], c[2], alpha];
                let inset = i as f32 * t;
                let (iw, ih) = (w - 2.0 * inset, h - 2.0 * inset);
                if iw <= 2.0 * t || ih <= 2.0 * t {
                    break;
                }
                self.ui.rect(inset, inset, iw, t, col);
                self.ui.rect(inset, h - inset - t, iw, t, col);
                self.ui.rect(inset, inset + t, t, ih - 2.0 * t, col);
                self.ui.rect(w - inset - t, inset + t, t, ih - 2.0 * t, col);
            }
        }
    }
}
