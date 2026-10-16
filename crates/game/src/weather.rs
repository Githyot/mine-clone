//! Weather: clear spells, rain (snow in cold biomes, nothing in deserts) and thunderstorms.
//! The sky greys and darkens while it rains; lightning flashes and thunder rolls in storms.
//! Rain is drawn per block column around the camera, stopping on the first roof above.

use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};
use voxel::rng::Rng;
use voxel::worldgen::Biome;

use crate::world::World;

/// Columns around the camera that get rain.
const RADIUS: i32 = 10;

/// What is saved with the world.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct WeatherSave {
    pub raining: bool,
    pub thunder: bool,
    /// Seconds until the weather changes.
    pub timer: f32,
}

/// One column of falling rain or snow for the renderer.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RainColumn {
    /// x, z of the column's corner and the bottom and top heights, relative to the camera's block.
    pub col: [f32; 4],
    /// x: 1 for snow, y: sky light at the bottom (0..1), z, w: absolute x and z (pattern seed).
    pub info: [f32; 4],
}

pub struct Weather {
    pub state: WeatherSave,
    /// How much it is raining right now (0..1); eases in and out.
    pub rain: f32,
    /// Lightning flash brightness (0..1), fading fast.
    pub flash: f32,
    bolt_in: f32,
    rain_sound_in: f32,
    cols_in: f32,
    pub columns: Vec<RainColumn>,
    /// Snow is falling where the player stands (for sound and rain-or-snow choices).
    pub snowing_here: bool,
    /// Something heard this frame: (thunder closeness 0..1), rain loop.
    pub thunder_sound: Option<f32>,
    pub rain_sound: bool,
    rng: Rng,
}

impl Weather {
    pub fn new(save: WeatherSave, seed: u64) -> Weather {
        let mut w = Weather {
            state: save,
            rain: if save.raining { 1.0 } else { 0.0 },
            flash: 0.0,
            bolt_in: 8.0,
            rain_sound_in: 0.0,
            cols_in: 0.0,
            columns: Vec::new(),
            snowing_here: false,
            thunder_sound: None,
            rain_sound: false,
            rng: Rng::new(seed ^ 0x5eed_7a11),
        };
        if w.state.timer <= 0.0 {
            w.state.timer = w.clear_spell();
        }
        w
    }

    /// 10 to 25 minutes of clear sky.
    fn clear_spell(&mut self) -> f32 {
        600.0 + self.rng.f32() * 900.0
    }

    /// Start or stop rain by hand (commands and test screenshots).
    pub fn set(&mut self, raining: bool, thunder: bool) {
        self.state.raining = raining;
        self.state.thunder = raining && thunder;
        self.state.timer = if raining { 300.0 + self.rng.f32() * 300.0 } else { self.clear_spell() };
    }

    pub fn update(&mut self, world: &World, eye: Vec3, dt: f32, instant: bool) {
        self.thunder_sound = None;
        self.rain_sound = false;
        self.state.timer -= dt;
        if self.state.timer <= 0.0 {
            if self.state.raining {
                self.set(false, false);
            } else {
                // A third of rains are storms.
                let storm = self.rng.f32() < 0.33;
                self.set(true, storm);
            }
        }
        let target = if self.state.raining { 1.0 } else { 0.0 };
        if instant {
            self.rain = target;
        } else {
            let step = dt / 12.0;
            self.rain += (target - self.rain).clamp(-step, step);
        }
        self.flash = (self.flash - dt * 2.5).max(0.0);

        let here = world.gen.biome_at(eye.x.floor() as i32, eye.z.floor() as i32);
        self.snowing_here = cold(here);
        if self.rain <= 0.0 {
            self.columns.clear();
            return;
        }

        // Lightning: a flash now, its thunder a moment later the farther it struck.
        if self.state.thunder && self.rain > 0.8 {
            self.bolt_in -= dt;
            if self.bolt_in <= 0.0 {
                self.bolt_in = 6.0 + self.rng.f32() * 18.0;
                let close = self.rng.f32();
                self.flash = 0.5 + close * 0.5;
                self.thunder_sound = Some(close);
            }
        }
        // The rain loop, but not in deserts or snow.
        self.rain_sound_in -= dt;
        if self.rain_sound_in <= 0.0 {
            self.rain_sound_in = 1.95;
            let dry = matches!(here, Biome::Desert | Biome::Badlands) || self.snowing_here;
            let roofed = world.surface_y(eye.x.floor() as i32, eye.z.floor() as i32).is_some_and(|y| y as f32 > eye.y + 1.0);
            self.rain_sound = !dry && self.rain > 0.3;
            if roofed {
                // Under a roof it still patters, just less often.
                self.rain_sound &= self.rng.f32() < 0.5;
            }
        }

        // Rebuild the columns a few times a second (the camera moves less than a block).
        self.cols_in -= dt;
        if self.cols_in > 0.0 && !instant {
            return;
        }
        self.cols_in = 0.1;
        self.columns.clear();
        let cb = eye.floor().as_ivec3();
        let top = eye.y + 14.0;
        let bottom = eye.y - 10.0;
        for dz in -RADIUS..=RADIUS {
            for dx in -RADIUS..=RADIUS {
                if dx * dx + dz * dz > RADIUS * RADIUS {
                    continue;
                }
                let (x, z) = (cb.x + dx, cb.z + dz);
                let biome = world.gen.biome_at(x, z);
                if matches!(biome, Biome::Desert | Biome::Badlands) {
                    continue;
                }
                let Some(surface) = world.surface_y(x, z) else { continue };
                let y0 = (surface + 1) as f32;
                let y0 = y0.max(bottom);
                if y0 >= top {
                    continue;
                }
                let l = world.light(IVec3::new(x, y0 as i32, z));
                self.columns.push(RainColumn {
                    col: [dx as f32, dz as f32, y0 - cb.y as f32, top - cb.y as f32],
                    info: [if cold(biome) { 1.0 } else { 0.0 }, (l >> 4) as f32 / 15.0, x as f32, z as f32],
                });
            }
        }
    }

    /// How much darker and greyer the day is.
    pub fn gloom(&self) -> f32 {
        self.rain * if self.state.thunder { 0.55 } else { 0.38 }
    }
}

fn cold(b: Biome) -> bool {
    matches!(b, Biome::Snowy | Biome::Taiga)
}
