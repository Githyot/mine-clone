//! The world's soundscape: sounds placed in 3D around the player (left/right, fading with
//! distance and muffled behind walls), creatures' recorded voices, and the ambience of each
//! place: caves breathing and dripping, wind on the heights, waves on the shore, lava bubbling,
//! the Nether's moan and the End's hum. Body sounds (landing, jumping, a heartbeat at low health,
//! gasping after a long dive) and interface sounds live here too.
//!
//! Everything except the creatures (recorded, mob_sounds.rs) is synthesised (sfx_extra.rs), and
//! all of it is prepared on a small worker thread so a long ambient sound never costs a frame.

use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

use glam::{IVec3, Vec3};
use rodio::{OutputStreamHandle, Source};
use voxel::mob::Kind;
use voxel::physics::Terrain;
use voxel::worldgen::{Biome, Dim};
use voxel::AIR;

use crate::audio::{Room, Sfx};
use crate::sfx_extra::{voice_of, Call, Fx};
use crate::{Game, Screen};

const RATE: f32 = 44100.0;

/// Everything the immersion stage remembers between frames (sounds and effects.rs).
pub struct Immersion {
    rng: u32,
    prev_ground: bool,
    prev_vel_y: f32,
    prev_pos: Vec3,
    prev_screen: Screen,
    prev_swing: f32,
    prev_head_wet: bool,
    least_air: f32,
    /// Per mob (keyed by its idle phase, which is random per mob): last hurt age, alive, and
    /// walk cycle (for footsteps).
    voices: HashMap<u32, (f32, bool, f32)>,
    cave_t: f32,
    wind_t: f32,
    water_t: f32,
    dim_t: f32,
    heart_t: f32,
    emit_t: f32,
    armor_dist: f32,
    pub(crate) dust_dist: f32,
    pub(crate) breath_t: f32,
    /// Sounds waiting to be played: (seconds left, sound, where, volume).
    pending: Vec<(f32, Sfx, Vec3, f32)>,
    /// Nearest water surface seen lately, and how much of it is around.
    water_near: Option<Vec3>,
    water_amount: f32,
    /// Camera shake strength (decays) and the morning/swamp mist (eases).
    pub(crate) shake: f32,
    pub(crate) mist: f32,
    /// Seconds since the last heartbeat (the vignette pulses with it).
    pub(crate) beat_age: f32,
    pub(crate) prev_health: f32,
    /// The screenshot test hook has run (effects.rs, MC_FX).
    pub(crate) primed: bool,
    /// The lightning bolt on screen: its path (with branches) and seconds left.
    pub(crate) bolt: Vec<Vec<Vec3>>,
    pub(crate) bolt_t: f32,
}

impl Immersion {
    pub fn new() -> Immersion {
        Immersion {
            rng: 0x2545_f491,
            prev_ground: true,
            prev_vel_y: 0.0,
            prev_pos: Vec3::ZERO,
            prev_screen: Screen::Loading,
            prev_swing: 0.0,
            prev_head_wet: false,
            least_air: 10.0,
            voices: HashMap::new(),
            cave_t: 40.0,
            wind_t: 6.0,
            water_t: 3.0,
            dim_t: 4.0,
            heart_t: 0.0,
            emit_t: 0.0,
            armor_dist: 0.0,
            dust_dist: 0.0,
            breath_t: 2.0,
            pending: Vec::new(),
            water_near: None,
            water_amount: 0.0,
            shake: 0.0,
            mist: 0.0,
            beat_age: 10.0,
            prev_health: 0.0,
            primed: false,
            bolt: Vec::new(),
            bolt_t: 0.0,
        }
    }

    pub(crate) fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng as f32 / u32::MAX as f32
    }

    pub(crate) fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.rand()
    }

    /// A random point on a ring around `c` (for far-off animal calls and rumbles).
    fn around(&mut self, c: Vec3, near: f32, far: f32) -> Vec3 {
        let a = self.rand() * std::f32::consts::TAU;
        let r = self.range(near, far);
        c + Vec3::new(a.cos() * r, self.range(-2.0, 4.0), a.sin() * r)
    }
}

struct Job {
    sfx: Sfx,
    seed: u32,
    vol: f32,
    pan: f32,
    muffle: f32,
    room: Room,
    handle: OutputStreamHandle,
}

/// The synthesis thread, started on first use.
fn worker() -> &'static Mutex<Sender<Job>> {
    static W: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();
    W.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        std::thread::Builder::new()
            .name("sfx".into())
            .spawn(move || {
                for j in rx {
                    let mono = crate::audio::room_sound(crate::sfx::synth(j.sfx, j.seed), j.room);
                    let stereo = spatial(mono, j.pan, j.muffle);
                    let src =
                        rodio::buffer::SamplesBuffer::new(2, RATE as u32, stereo).amplify(j.vol);
                    let _ = j.handle.play_raw(src);
                }
            })
            .ok();
        Mutex::new(tx)
    })
}

/// Turn a mono sound into stereo: equal-power panning, the far ear a touch later and darker
/// (the head shades it), and an overall low-pass for walls and distance (`muffle` 0..1).
pub(crate) fn spatial(mono: Vec<f32>, pan: f32, muffle: f32) -> Vec<f32> {
    let pan = pan.clamp(-1.0, 1.0);
    let muffle = muffle.clamp(0.0, 1.0);
    let cut = 9000.0 * (450.0f32 / 9000.0).powf(muffle);
    let a = 1.0 - (-std::f32::consts::TAU * cut / RATE).exp();
    let shade =
        1.0 - (-std::f32::consts::TAU * (cut.min(9000.0 - 6500.0 * pan.abs())) / RATE).exp();
    let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
    // Centre stays as loud as before; a hard-panned sound gets a little louder on its side.
    let (gl, gr) = (
        angle.cos() * std::f32::consts::SQRT_2,
        angle.sin() * std::f32::consts::SQRT_2,
    );
    let (gl, gr) = (gl.min(1.25), gr.min(1.25));
    let delay = (pan.abs() * 0.0005 * RATE) as usize;
    let n = mono.len() + delay;
    let mut near = Vec::with_capacity(n);
    let (mut y1, mut y2, mut s1) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..n {
        let x = mono.get(i).copied().unwrap_or(0.0);
        y1 += a * (x - y1);
        y2 += a * (y1 - y2);
        near.push(y2);
    }
    let mut out = vec![0.0f32; n * 2];
    for i in 0..n {
        let far_in = if i >= delay { near[i - delay] } else { 0.0 };
        s1 += shade * (far_in - s1);
        let (l, r) = if pan >= 0.0 {
            (s1 * gl, near[i] * gr)
        } else {
            (near[i] * gl, s1 * gr)
        };
        out[i * 2] = l;
        out[i * 2 + 1] = r;
    }
    out
}

impl Game {
    fn listener_right(&self) -> Vec3 {
        let f = self.forward();
        Vec3::new(f.x, 0.0, f.z).cross(Vec3::Y).normalize_or_zero()
    }

    /// Play a sound at a place in the world: panned to its side, quieter with distance (silent
    /// past `range` blocks), and muffled by the blocks between it and the player.
    pub(crate) fn sfx_at(&mut self, s: Sfx, pos: Vec3, vol: f32, range: f32) {
        if self.audio.is_none() || self.args.screenshot.is_some() {
            return;
        }
        let Some((gain, pan, muffle)) = self.place_sound(pos, range) else { return };
        let vol = vol * gain * self.settings.sound_volume;
        if let Some(c) = crate::subtitles::caption(s) {
            if vol > 0.02 {
                self.subtitle(c, Some(pos));
            }
        }
        self.sfx_raw(s, vol, pan, muffle);
    }

    /// Where a sound at `pos` sits for our ears: its gain (distance and walls), pan (-1 left
    /// .. 1 right) and muffling (0..1). None past `range`. Voices use it too (voice.rs).
    pub(crate) fn place_sound(&self, pos: Vec3, range: f32) -> Option<(f32, f32, f32)> {
        let eye = self.body.eye();
        let to = pos - eye;
        let d = to.length();
        if d > range {
            return None;
        }
        let gain = (1.0 - d / range).powf(1.3);
        let right = self.listener_right();
        let pan = if d > 0.01 {
            to.normalize().dot(right) * (d / 2.5).min(1.0)
        } else {
            0.0
        };
        // Count the solid blocks on the way: each muffles and softens the sound.
        let mut walls = 0;
        let mut last = eye.floor().as_ivec3();
        let steps = (d / 0.5) as i32;
        for k in 1..steps {
            let p = (eye + to * (k as f32 / steps as f32)).floor().as_ivec3();
            if p != last
                && (pos - p.as_vec3() - Vec3::splat(0.5)).length() > 0.9
                && self.world.solid(p)
            {
                walls += 1;
                if walls >= 4 {
                    break;
                }
            }
            last = p;
        }
        let occ = (walls as f32 / 3.0).min(1.0);
        let muffle = occ * 0.85 + (d / range) * 0.3;
        Some((gain * (1.0 - 0.5 * occ), pan, muffle))
    }

    /// Play a sound with no place (in the head, or all around), but still in stereo and
    /// on the worker thread.
    pub(crate) fn sfx_here(&mut self, s: Sfx, vol: f32) {
        if self.args.screenshot.is_some() {
            return;
        }
        let pan = self.fx.range(-0.15, 0.15);
        self.sfx_raw(s, vol * self.settings.sound_volume, pan, 0.0);
    }

    fn sfx_raw(&mut self, s: Sfx, vol: f32, pan: f32, muffle: f32) {
        if vol <= 0.002 {
            return;
        }
        let seed = (self.fx.rand() * 1e9) as u32;
        let Some(a) = self.audio.as_ref() else { return };
        let job = Job {
            sfx: s,
            seed,
            vol,
            pan,
            muffle,
            room: a.room,
            handle: a.handle.clone(),
        };
        if let Ok(tx) = worker().lock() {
            let _ = tx.send(job);
        }
    }

    /// Called every frame while a world is open and being played.
    pub(crate) fn update_immersion(&mut self, dt: f32) {
        if dt <= 0.0 {
            self.prime_effects();
            return;
        }
        self.fx.shake = (self.fx.shake - dt * 2.5).max(0.0);
        self.fx.bolt_t -= dt;
        self.fx.beat_age += dt;
        self.update_mist(dt);
        self.body_sounds(dt);
        self.screen_sounds();
        self.mob_voices(dt);
        self.scan_emitters(dt);
        self.ambience(dt);
        self.update_fx(dt);
        // Delayed sounds (a drop landing after its fall).
        let mut due = Vec::new();
        self.fx.pending.retain_mut(|p| {
            p.0 -= dt;
            if p.0 <= 0.0 {
                due.push((p.1, p.2, p.3));
                false
            } else {
                true
            }
        });
        for (s, at, vol) in due {
            self.sfx_at(s, at, vol, 20.0);
        }
    }

    fn body_sounds(&mut self, dt: f32) {
        let ground = self.body.on_ground;
        let vy = self.body.vel.y;
        let flying = self.body.flying;
        let wet = self.body.in_water;
        let feet = self.body.pos;
        if !flying && !wet && self.seat.is_none() && self.riding.is_none() {
            if ground && !self.fx.prev_ground && self.fx.prev_vel_y < -7.0 {
                // Landing: a thump that grows with the fall, and the ground's own sound.
                let k = ((-self.fx.prev_vel_y - 7.0) / 15.0).clamp(0.0, 1.0);
                self.sfx_here(Sfx::Fx(Fx::Land(k)), 0.5 + 0.4 * k);
                let below = self
                    .world
                    .block_or_air((feet - Vec3::Y * 0.2).floor().as_ivec3());
                if below != AIR {
                    let m = crate::audio::Material::of(&self.reg.get(below).name);
                    self.sfx_here(Sfx::Step(m), 0.6 + 0.6 * k);
                }
                self.land_dust(k);
                self.fx.shake = self.fx.shake.max(k * 0.5);
            } else if !ground && self.fx.prev_ground && vy > 4.0 {
                self.sfx_here(Sfx::Fx(Fx::Jump), 0.25);
            }
        }
        // Armour shifting as you walk.
        let moved = ((feet - self.fx.prev_pos) * Vec3::new(1.0, 0.0, 1.0)).length();
        if ground && !flying && moved < 2.0 {
            self.fx.armor_dist += moved;
            if self.fx.armor_dist > 1.7 {
                self.fx.armor_dist = 0.0;
                if let Some(metal) = self.worn_armor() {
                    self.sfx_here(
                        Sfx::Fx(Fx::Armor(metal)),
                        if self.sprinting { 0.3 } else { 0.18 },
                    );
                }
            }
        }
        // A swing through the air.
        let sw = self.anim.swing;
        if sw > 0.0 && (self.fx.prev_swing <= 0.0 || sw < self.fx.prev_swing) {
            self.sfx_here(Sfx::Fx(Fx::Swing), 0.3);
        }
        self.fx.prev_swing = sw;
        // Breath: remember how low the air got, and gasp coming up from a long dive.
        let head_wet = self.body.head_in_water;
        if head_wet {
            self.fx.least_air = self.fx.least_air.min(self.air);
        } else if self.fx.prev_head_wet {
            if self.survival && self.fx.least_air < 4.0 {
                self.sfx_here(Sfx::Fx(Fx::Gasp), 0.55);
            }
            self.fx.least_air = 10.0;
        }
        self.fx.prev_head_wet = head_wet;
        // A heartbeat when health runs low, faster the lower it is.
        let low = self.survival
            && self.screen != Screen::Dead
            && self.health > 0.0
            && self.health <= self.max_health * 0.3;
        if low {
            self.fx.heart_t -= dt;
            if self.fx.heart_t <= 0.0 {
                let k = self.health / (self.max_health * 0.3);
                self.fx.heart_t = 0.65 + 0.6 * k;
                self.fx.beat_age = 0.0;
                self.sfx_here(Sfx::Fx(Fx::Heartbeat), 0.75);
            }
        } else {
            self.fx.heart_t = 0.0;
        }
        self.fx.prev_ground = ground;
        self.fx.prev_vel_y = vy;
        self.fx.prev_pos = feet;
    }

    /// Some(true) when metal armour is worn, Some(false) for leather only, None for nothing.
    fn worn_armor(&self) -> Option<bool> {
        let mut any = None;
        for s in self.surv.armor.iter().flatten() {
            let n = &self.items.get(s.item).name;
            let metal = ["iron", "gold", "diamond", "chain", "netherite", "copper"]
                .iter()
                .any(|m| n.contains(m));
            any = Some(any.unwrap_or(false) || metal);
        }
        any
    }

    fn screen_sounds(&mut self) {
        let (was, now) = (self.fx.prev_screen, self.screen);
        if was != now {
            if now == Screen::Inventory {
                self.sfx_here(Sfx::Fx(Fx::InvOpen), 0.45);
            } else if was == Screen::Inventory {
                self.sfx_here(Sfx::Fx(Fx::InvClose), 0.4);
            }
        }
        self.fx.prev_screen = now;
    }

    /// Mobs talk now and then, cry out when hurt and when they die.
    fn mob_voices(&mut self, dt: f32) {
        let eye = self.body.eye();
        let mut calls: Vec<(Fx, Vec3, f32)> = Vec::new();
        let mut captions: Vec<(String, Vec3)> = Vec::new();
        let mut hurt_fx: Vec<(Vec3, f32, bool)> = Vec::new();
        let mut steps: Vec<(Vec3, f32, bool)> = Vec::new();
        use crate::sfx_extra::Voice as V;
        let mut seen = std::collections::HashSet::new();
        let day = {
            let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
            (a.sin() * 2.2 + 0.45) > 0.3
        };
        for i in 0..self.mobs.list.len() {
            let m = &self.mobs.list[i];
            let def = &self.mobs.reg.defs[m.def];
            let key = m.phase.to_bits();
            seen.insert(key);
            let at = m.body.pos + Vec3::Y * def.height * 0.8;
            let (hurt_age, alive, sleeping) = (m.hurt_age, m.alive(), m.sleeping);
            let (walk, walk_speed, grounded, feet) = (m.walk, m.walk_speed, m.body.on_ground, m.body.pos);
            let (height, width) = (def.height, def.width);
            let Some(voice) = voice_of(&def.name, def.kind == Kind::Hostile) else {
                continue;
            };
            // Bigger creatures sound deeper.
            let depth = (height / 1.4).clamp(0.5, 2.6).sqrt();
            let prev = self.fx.voices.insert(key, (hurt_age, alive, walk));
            let near = (at - eye).length() < 28.0;
            // Footsteps (Presence Footsteps idea): one per leg landing. Four-legged animals
            // patter paw by paw, lighter each; people and monsters step twice per stride.
            let four_legs = width >= height * 0.7 && !matches!(voice, V::Murmur | V::Groan | V::Rattle | V::Wail | V::Cackle);
            if let Some(&(_, _, pw)) = prev.as_ref() {
                let per = std::f32::consts::PI / if four_legs { 2.0 } else { 1.0 };
                if alive && grounded && walk_speed > 0.4 && (walk / per).floor() != (pw / per).floor() && (feet - eye).length() < 16.0 && steps.len() < 6 {
                    let size = (height * width.max(0.3)).sqrt();
                    let vol = (0.12 + size * 0.15).min(0.5) * if four_legs { 0.7 } else { 1.0 };
                    steps.push((feet, vol, size > 1.6));
                }
            }
            match prev {
                Some((ph, pa, _)) => {
                    if pa && !alive {
                        if near {
                            if let Some(fx) = crate::mob_sounds::pick(&def.name, Call::Death, depth, self.fx.rand(), self.fx.rand()) {
                                calls.push((fx, at, 0.9));
                                captions.push((format!("{} morre", def.display_name), at));
                            }
                        }
                        hurt_fx.push((at, height, true));
                    } else if alive && hurt_age < ph {
                        if near {
                            if let Some(fx) = crate::mob_sounds::pick(&def.name, Call::Hurt, depth, self.fx.rand(), self.fx.rand()) {
                                calls.push((fx, at, 0.85));
                                captions.push((format!("{} sofre dano", def.display_name), at));
                            }
                        }
                        hurt_fx.push((at, height, false));
                    }
                }
                None => continue,
            }
            if alive && !sleeping && near {
                // Songbirds and doves keep quiet at night.
                if voice == crate::sfx_extra::Voice::Coo && !day {
                    continue;
                }
                let chance = dt / voice.chattiness();
                if self.fx.rand() < chance {
                    if let Some(fx) = crate::mob_sounds::pick(&def.name, Call::Idle, depth, self.fx.rand(), self.fx.rand()) {
                        calls.push((fx, at, 0.6));
                        captions.push((def.display_name.clone(), at));
                    }
                }
            }
        }
        self.fx.voices.retain(|k, _| seen.contains(k));
        for (c, at) in captions {
            self.subtitle(&c, Some(at));
        }
        for (fx, at, vol) in calls {
            self.sfx_at(Sfx::Fx(fx), at, vol, 24.0);
        }
        for (feet, vol, heavy) in steps {
            let below = self.world.block_or_air((feet - Vec3::Y * 0.2).floor().as_ivec3());
            if below == AIR {
                continue;
            }
            let m = crate::audio::Material::of(&self.reg.get(below).name);
            self.sfx_at(Sfx::Step(m), feet, vol, 16.0);
            if heavy {
                // Big animals land with weight.
                self.sfx_at(Sfx::Fx(Fx::Land(0.1)), feet, vol * 0.8, 20.0);
            }
        }
        for (at, height, died) in hurt_fx {
            self.mob_hit_fx(at, height, died);
        }
    }

    /// Look at a few random blocks around the player for things that make sound: lava,
    /// fire, water surfaces, ceilings that drip.
    fn scan_emitters(&mut self, dt: f32) {
        self.fx.emit_t += dt;
        if self.fx.emit_t < 0.2 {
            return;
        }
        self.fx.emit_t = 0.0;
        let me = self.body.pos.floor().as_ivec3();
        let mut water = 0;
        let mut nearest: Option<(f32, Vec3)> = None;
        let underground = self.dim == Dim::Overworld
            && (self.world.light(self.body.eye().floor().as_ivec3()) >> 4) == 0;
        for _ in 0..40 {
            let off = IVec3::new(
                (self.fx.rand() * 25.0) as i32 - 12,
                (self.fx.rand() * 13.0) as i32 - 6,
                (self.fx.rand() * 25.0) as i32 - 12,
            );
            let p = me + off;
            let id = self.world.block_or_air(p);
            if id == AIR {
                continue;
            }
            let above = self.world.block_or_air(p + IVec3::Y);
            let below = self.world.block_or_air(p - IVec3::Y);
            let fluid = self.world.fluid(id);
            let c = p.as_vec3() + Vec3::splat(0.5);
            if fluid == 2 && above == AIR {
                if self.fx.rand() < 0.12 {
                    self.sfx_at(Sfx::Fx(Fx::LavaPop), c + Vec3::Y * 0.5, 0.55, 16.0);
                    self.lava_spit(c + Vec3::Y * 0.5);
                }
            } else if fluid == 1 && above == AIR {
                water += 1;
                let d = (c - self.body.pos).length();
                if nearest.is_none_or(|(nd, _)| d < nd) {
                    nearest = Some((d, c + Vec3::Y * 0.5));
                }
            } else if fluid == 0 && self.world.solid(p) && below == AIR {
                // A ceiling: water seeping from above drips; in caves some stone drips anyway.
                let wet = self.world.fluid(above) == 1;
                let lava = self.world.fluid(above) == 2;
                let cave_drip = underground && self.fx.rand() < 0.04;
                if (wet || lava || cave_drip) && self.fx.rand() < 0.5 {
                    self.ceiling_drip(p, lava);
                }
            }
            let name = &self.reg.get(id).name;
            if (name == "fire" || name.contains("campfire")) && self.fx.rand() < 0.3 {
                self.sfx_at(Sfx::Fx(Fx::FireCrackle), c, 0.5, 14.0);
                self.smoke(c + Vec3::Y * 0.4, 2);
            }
        }
        self.fx.water_amount = self.fx.water_amount * 0.8 + water as f32 / 40.0 * 0.2;
        if nearest.is_some() {
            self.fx.water_near = nearest.map(|n| n.1);
        } else if self.fx.water_amount < 0.01 {
            self.fx.water_near = None;
        }
    }

    /// A drop falls from the ceiling at `p`; its sound plays when it lands.
    fn ceiling_drip(&mut self, p: IVec3, lava: bool) {
        let mut fall = 1;
        while fall < 24 && self.world.block_or_air(p - IVec3::Y * (fall + 1)) == AIR {
            fall += 1;
        }
        if fall >= 24 {
            return;
        }
        let at = p.as_vec3() + Vec3::new(self.fx.range(0.2, 0.8), -0.05, self.fx.range(0.2, 0.8));
        self.drip_particle(at, lava);
        let t = (2.0 * fall as f32 / 16.0).sqrt();
        let land = at - Vec3::Y * fall as f32;
        let sound = if lava {
            Sfx::Fx(Fx::LavaPop)
        } else {
            Sfx::Fx(Fx::Drip)
        };
        self.fx
            .pending
            .push((t, sound, land, if lava { 0.3 } else { 0.4 }));
    }

    /// The sounds of the place: caves, wind, water, the Nether and the End.
    fn ambience(&mut self, dt: f32) {
        let eye = self.body.eye();
        let head = eye.floor().as_ivec3();
        let sky = (self.world.light(head) >> 4) as f32;
        let biome = self.world.gen.base_biome_at(head.x, head.z);
        match self.dim {
            Dim::Nether => {
                self.fx.dim_t -= dt;
                if self.fx.dim_t <= 0.0 {
                    self.fx.dim_t = self.fx.range(7.0, 16.0);
                    let s = if biome == Biome::SoulSandValley {
                        Fx::SoulWind
                    } else {
                        Fx::NetherDrone
                    };
                    self.sfx_here(Sfx::Fx(s), 0.5);
                }
                return;
            }
            Dim::End => {
                self.fx.dim_t -= dt;
                if self.fx.dim_t <= 0.0 {
                    self.fx.dim_t = self.fx.range(9.0, 18.0);
                    self.sfx_here(Sfx::Fx(Fx::EndHum), 0.45);
                    if self.fx.rand() < 0.4 {
                        self.sfx_here(Sfx::Fx(Fx::Wind(0.3)), 0.25);
                    }
                }
                return;
            }
            _ => {}
        }
        // Deep under ground: a rare low swell or a far-off rockfall, like the classic cave sounds.
        let deep = sky == 0.0 && eye.y < 60.0 && !self.body.head_in_water;
        if deep {
            self.fx.cave_t -= dt;
            if self.fx.cave_t <= 0.0 {
                self.fx.cave_t = self.fx.range(70.0, 200.0);
                if self.fx.rand() < 0.55 {
                    self.sfx_here(Sfx::Fx(Fx::CaveDrone), 0.55);
                } else {
                    let at = self.fx.around(eye, 10.0, 20.0);
                    self.sfx_at(Sfx::Fx(Fx::CaveRumble), at, 0.8, 30.0);
                }
            }
        }
        // Wind out in the open: stronger high up, in storms and on bare, cold land.
        if sky >= 12.0 && !self.body.head_in_water {
            self.fx.wind_t -= dt;
            if self.fx.wind_t <= 0.0 {
                self.fx.wind_t = self.fx.range(4.0, 9.0);
                let height = ((eye.y - 72.0) / 60.0).clamp(0.0, 1.0);
                let open = match biome {
                    Biome::Mountains
                    | Biome::Snowy
                    | Biome::Desert
                    | Biome::Badlands
                    | Biome::Savanna
                    | Biome::Beach
                    | Biome::Ocean => 0.25,
                    _ => 0.0,
                };
                let storm = self.weather.rain
                    * if self.weather.state.thunder {
                        0.6
                    } else {
                        0.35
                    };
                let s = (height + open + storm).min(1.0);
                let forest = matches!(
                    biome,
                    Biome::Forest
                        | Biome::BirchForest
                        | Biome::DarkForest
                        | Biome::Jungle
                        | Biome::Taiga
                        | Biome::CherryGrove
                );
                if forest && self.fx.rand() < 0.5 {
                    self.sfx_here(Sfx::Fx(Fx::Canopy), 0.25 + s * 0.3);
                } else if s > 0.12 {
                    self.sfx_here(Sfx::Fx(Fx::Wind(s)), 0.2 + s * 0.4);
                }
            }
        }
        // Water nearby: waves on a sea shore, small laps on lakes and rivers.
        if let Some(w) = self.fx.water_near {
            self.fx.water_t -= dt;
            if self.fx.water_t <= 0.0 && !self.body.head_in_water {
                let sea =
                    matches!(biome, Biome::Ocean | Biome::Beach) && self.fx.water_amount > 0.08;
                if sea {
                    self.fx.water_t = self.fx.range(4.5, 8.0);
                    self.sfx_at(Sfx::Fx(Fx::Waves), w, 0.7, 40.0);
                } else {
                    self.fx.water_t = self.fx.range(5.0, 10.0);
                    self.sfx_at(Sfx::Fx(Fx::Lapping), w, 0.45, 16.0);
                }
            }
        }
    }
}
