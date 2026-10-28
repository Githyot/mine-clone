//! Lote 16 of the Terraria side (plan/terraria-completo.md): making the fights read and play well
//! in 3D, first person. Terraria shows the whole arena from the side; here the player sees only
//! what is in front of them, so every big attack warns where it will land:
//! - dashes (Great Eye, Mechanical Eye, Tide Duke, Prism Empress, Jelly Queen's dive) stop for a
//!   moment and draw their lane in the air first, so a step to the side dodges them;
//! - things falling from the sky (the Herald's stars, the Empress' lances) glow on the ground
//!   where they will land;
//! - stomps (Jelly King, Scale Idol) send a ring along the ground that a jump clears;
//! - worms dig real tunnels through earth and stone, and the ground above them shakes, so you
//!   can see them coming and fight them in the open caves they leave;
//! - the boss bar points to the boss when it is out of sight.

use glam::{IVec3, Vec3};
use voxel::physics::Terrain;
use voxel::block::BlockId;

use crate::audio::{Material, Sfx};
use crate::Game;

/// Which fight's clock to stop while a boss winds up (each file keeps its own).
#[derive(Clone, Copy)]
pub enum Clock {
    /// tr_bosses.rs
    Tr,
    /// tr_endgame.rs
    End,
    /// boss_ai.rs
    T5,
}

struct Windup {
    mob: usize,
    at: Vec3,
    dir: Vec3,
    len: f32,
    t: f32,
    clock: Clock,
}

struct Mark {
    at: Vec3,
    t: f32,
    tint: [f32; 3],
}

struct Wave {
    at: Vec3,
    r: f32,
    max: f32,
    dmg: f32,
    hit: bool,
}

#[derive(Default)]
pub struct Feel {
    windup: Option<Windup>,
    marks: Vec<Mark>,
    waves: Vec<Wave>,
    rumble_t: f32,
    rumble_sfx_t: f32,
    /// The cells a worm head went through, oldest first, still to be dug out.
    trail: Vec<IVec3>,
    natural: Vec<BlockId>,
    /// Test scene: keep showing the warnings.
    demo: Option<Vec3>,
    demo_t: f32,
}

/// Blocks a worm digs through: the ground as the world made it, never what players build with.
const NATURAL: &[&str] = &[
    "stone", "dirt", "grass_block", "coarse_dirt", "podzol", "mycelium", "rooted_dirt", "mud", "clay", "gravel", "sand", "red_sand", "sandstone", "red_sandstone", "deepslate", "tuff", "granite", "diorite", "andesite", "calcite", "dripstone_block", "netherrack", "soul_sand", "soul_soil", "snow_block", "ash_block",
];

const WAVE_SPEED: f32 = 13.0;

impl Game {
    /// A boss is about to dash along `vel`: it holds still for `secs` and draws its lane first.
    pub(crate) fn dash_windup(&mut self, mob: usize, clock: Clock, vel: Vec3, secs: f32) {
        let at = self.mobs.list[mob].body.pos;
        let speed = vel.length();
        self.t5.tr.feel.windup = Some(Windup { mob, at, dir: vel / speed.max(0.01), len: (speed * 0.6).min(18.0), t: secs, clock });
        self.sfx(Sfx::Thunder(0.15));
    }

    /// Something falls from `height` above the ground at `at` (x, z): mark the spot, then drop it.
    pub(crate) fn sky_strike(&mut self, at: Vec3, height: f32, speed: f32, school: u8, dmg: f32) {
        let ground = self.ground_below(at + Vec3::Y * 3.0, 24);
        let from = Vec3::new(at.x, ground + height, at.z);
        self.enemy_bolt(from, Vec3::NEG_Y * speed, school, dmg);
        let tint = [[1.8, 0.6, 0.3], [0.5, 1.2, 1.8], [1.8, 1.6, 0.6], [1.4, 0.6, 1.8], [0.6, 1.6, 0.6], [1.6, 1.6, 1.6]][school as usize % 6];
        self.t5.tr.feel.marks.push(Mark { at: Vec3::new(at.x, ground + 0.05, at.z), t: height / speed, tint });
    }

    /// A glowing warning on the ground at `at` for `secs` (where something will land).
    pub(crate) fn ground_mark(&mut self, at: Vec3, secs: f32, tint: [f32; 3]) {
        self.t5.tr.feel.marks.push(Mark { at, t: secs, tint });
    }

    /// A ring runs out along the ground from `at`; it hurts whoever stands on the ground where it
    /// passes. A jump clears it.
    pub(crate) fn shockwave(&mut self, at: Vec3, max: f32, dmg: f32) {
        let y = self.ground_below(at + Vec3::Y, 6);
        self.t5.tr.feel.waves.push(Wave { at: Vec3::new(at.x, y, at.z), r: 0.5, max, dmg, hit: false });
        self.sfx(Sfx::Explode);
    }

    /// The top of the first solid block under `p` (searching `depth` blocks), or `p.y - 1`.
    fn ground_below(&self, p: Vec3, depth: i32) -> f32 {
        let c = p.floor().as_ivec3();
        (0..depth).map(|k| c - IVec3::Y * k).find(|q| self.world.solid(*q)).map(|q| q.y as f32 + 1.0).unwrap_or(p.y - 1.0)
    }

    pub(crate) fn update_tr_feel(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        self.feel_demo(dt);
        self.feel_windup(dt);
        self.feel_marks(dt);
        self.feel_waves(dt);
        self.feel_worms(dt);
    }

    fn feel_windup(&mut self, dt: f32) {
        let Some(w) = self.t5.tr.feel.windup.as_mut() else { return };
        w.t -= dt;
        let (mob, at, dir, len, done, clock) = (w.mob, w.at, w.dir, w.len, w.t <= 0.0, w.clock);
        if done {
            self.t5.tr.feel.windup = None;
        }
        let Some(m) = self.mobs.list.get_mut(mob).filter(|m| m.alive()) else {
            self.t5.tr.feel.windup = None;
            return;
        };
        if done {
            return;
        }
        // Hold still (the fight's clock stops too, so the dash keeps its full length).
        m.body.pos = at;
        m.body.vel = Vec3::ZERO;
        match clock {
            Clock::Tr => self.t5.tr.fight.hold(dt),
            Clock::End => self.t5.tr.end.hold(dt),
            Clock::T5 => self.t5.fight.hold(dt),
        }
        let glow = self.glow_layer();
        let from = at + Vec3::Y;
        for _ in 0..14 {
            let s = self.surv.rng.f32() * len;
            self.particles.glow(from + dir * s, Vec3::ZERO, glow, [2.0, 0.4, 0.3], 0.16, 0.18);
        }
        // It flares up: aimed straight at you, the lane is only a dot, so the flare is the warning.
        for _ in 0..3 {
            let jitter = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5);
            self.particles.glow(from + jitter * 2.5, jitter * 4.0, glow, [2.0, 0.5, 0.3], 0.3, 0.3);
        }
    }

    fn feel_marks(&mut self, dt: f32) {
        if self.t5.tr.feel.marks.is_empty() {
            return;
        }
        let glow = self.glow_layer();
        let now = self.now;
        let mut marks = std::mem::take(&mut self.t5.tr.feel.marks);
        marks.retain_mut(|m| {
            m.t -= dt;
            // A ring that closes in as the hit comes.
            let r = 0.5 + m.t.clamp(0.0, 1.0) * 1.2;
            for k in 0..6 {
                let a = now * 3.0 + k as f32 * 1.047;
                self.particles.glow(m.at + Vec3::new(a.cos() * r, 0.35, a.sin() * r), Vec3::Y * 0.4, glow, m.tint, 0.22, 0.12);
            }
            m.t > 0.0
        });
        self.t5.tr.feel.marks = marks;
    }

    fn feel_waves(&mut self, dt: f32) {
        if self.t5.tr.feel.waves.is_empty() {
            return;
        }
        let glow = self.glow_layer();
        let me = self.body.pos;
        let grounded = self.body.on_ground;
        let mut hurt = None;
        let mut waves = std::mem::take(&mut self.t5.tr.feel.waves);
        waves.retain_mut(|w| {
            w.r += WAVE_SPEED * dt;
            let n = (w.r * 4.0) as usize + 6;
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU + w.r;
                self.particles.glow(w.at + Vec3::new(a.cos() * w.r, 0.4, a.sin() * w.r), Vec3::Y * 1.5, glow, [1.8, 1.3, 0.7], 0.3, 0.2);
            }
            let flat = Vec3::new(me.x - w.at.x, 0.0, me.z - w.at.z).length();
            if !w.hit && grounded && (flat - w.r).abs() < 0.9 && (me.y - w.at.y).abs() < 1.6 {
                w.hit = true;
                hurt = Some((w.dmg, w.at));
            }
            w.r < w.max
        });
        self.t5.tr.feel.waves = waves;
        if let Some((dmg, at)) = hurt {
            if self.hunted() {
                self.damage(dmg, Some(at));
                self.body.vel.y = self.body.vel.y.max(5.0);
            }
        }
    }

    /// Worm heads dig tunnels behind them, and the ground over them shakes.
    fn feel_worms(&mut self, dt: f32) {
        let head = self.mobs.list.iter().find(|m| m.alive() && matches!(self.mobs.reg.defs[m.def].name.as_str(), "deep_worm" | "mech_worm")).map(|m| m.body.pos);
        let Some(head) = head else {
            self.t5.tr.feel.trail.clear();
            return;
        };
        if self.t5.tr.feel.natural.is_empty() {
            self.t5.tr.feel.natural = NATURAL.iter().filter_map(|n| self.reg.id(n)).collect();
        }
        let cell = head.floor().as_ivec3();
        let keep = [cell, cell + IVec3::Y];
        let f = &mut self.t5.tr.feel;
        if f.trail.last() != Some(&cell) {
            f.trail.push(cell);
        }
        // Dig out the cells the head has left (not the ones it is in: it swims while inside).
        let mut dig = Vec::new();
        while f.trail.first().is_some_and(|c| (c.as_vec3() - cell.as_vec3()).length() > 1.8) {
            dig.push(f.trail.remove(0));
        }
        let mut dug = 0;
        let mut look = None;
        for c in dig {
            for x in -1..=1i32 {
                for y in -1..=1i32 {
                    for z in -1..=1i32 {
                        if x.abs() + y.abs() + z.abs() > 2 {
                            continue;
                        }
                        let q = c + IVec3::new(x, y, z);
                        if keep.contains(&q) {
                            continue;
                        }
                        let id = self.world.block_or_air(q);
                        if self.t5.tr.feel.natural.contains(&id) && self.world.set_block(q, voxel::block::AIR) {
                            dug += 1;
                            look.get_or_insert((q, id));
                        }
                    }
                }
            }
        }
        if let Some((q, id)) = look {
            let (layer, tint) = self.block_look(id);
            self.particles.block_bits(q, layer, tint, 4, 1.0);
            if dug > 6 && self.surv.rng.f32() < 0.3 {
                self.sfx(Sfx::Break(Material::Stone));
            }
        }
        // The ground above shakes while it is under you.
        let f = &mut self.t5.tr.feel;
        f.rumble_t -= dt;
        f.rumble_sfx_t -= dt;
        let me = self.body.pos;
        let near = Vec3::new(head.x - me.x, 0.0, head.z - me.z).length() < 28.0;
        if !near || !self.world.solid(cell) || f.rumble_t > 0.0 {
            return;
        }
        f.rumble_t = 0.1;
        let sfx = f.rumble_sfx_t <= 0.0;
        if sfx {
            f.rumble_sfx_t = 1.1;
        }
        if let Some(top) = (1..14).map(|k| cell + IVec3::Y * k).find(|q| !self.world.solid(*q)) {
            let below = self.world.block_or_air(top - IVec3::Y);
            let (layer, tint) = self.block_look(below);
            self.particles.block_bits(top - IVec3::Y, layer, tint, 3, 0.8);
            if sfx {
                self.sfx(Sfx::Thunder(0.2));
            }
        }
    }

    fn feel_demo(&mut self, dt: f32) {
        let Some(at) = self.t5.tr.feel.demo else { return };
        self.t5.tr.feel.demo_t -= dt;
        if self.t5.tr.feel.demo_t > 0.0 {
            return;
        }
        self.t5.tr.feel.demo_t = 0.45;
        self.shockwave(at, 7.0, 0.0);
        let side = self.forward().cross(Vec3::Y).normalize_or_zero();
        for k in 0..3 {
            let p = at + side * (k as f32 * 2.5 - 9.0) + Vec3::new(self.surv.rng.f32(), 0.0, self.surv.rng.f32()) * 2.0;
            self.sky_strike(p, 14.0, 16.0, 2, 0.0);
        }
    }

    /// Test scenes: the warnings around a still boss (feel3d), a worm digging up to you (worm3d).
    pub(crate) fn test_tr_feel(&mut self, scene: &str) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        if scene == "worm3d" {
            let at = self.body.pos + flat * 9.0 + side * 3.0 - Vec3::Y * 5.0;
            if self.mobs.spawn("deep_worm", at) {
                self.spawn_worm_body("deep_worm", at);
            }
            self.t5.fight.vel_for_test(Vec3::new(-side.x, 0.3, -side.z) * 10.0);
            self.feel_frames(30, true);
            return;
        }
        self.t5.tr.feel.demo = Some(self.body.pos + flat * 7.0);
        let at = self.body.pos + flat * 14.0 + side * 5.0 + Vec3::Y * 6.0;
        if self.mobs.spawn("great_eye", at) {
            let i = self.mobs.list.len() - 1;
            let vel = (self.body.eye() - at).normalize_or_zero() * 20.0;
            self.dash_windup(i, Clock::Tr, vel, 999.0);
        }
        self.feel_frames(14, false);
    }

    /// Runs the warnings (and the worm) for a few frames before the screenshot.
    fn feel_frames(&mut self, n: usize, fight: bool) {
        for _ in 0..n {
            if fight {
                self.update_fight(0.03);
            }
            self.update_tr_feel(0.03);
            self.particles.update(&self.world, 0.03);
        }
    }

    /// Arrows beside the boss bar towards a boss that is out of sight, and how far it is.
    pub(crate) fn boss_pointer(&mut self, pos: Vec3, w: f32, y: f32) {
        let eye = self.body.eye();
        let rel = pos - eye;
        let dist = rel.length();
        if dist < 0.5 {
            return;
        }
        let f = self.forward();
        let flat_f = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let flat_r = Vec3::new(rel.x, 0.0, rel.z).normalize_or_zero();
        let side = flat_f.cross(Vec3::Y).dot(flat_r);
        let ahead = flat_f.dot(flat_r);
        let up = (rel.y / dist).asin() - f.y.clamp(-1.0, 1.0).asin();
        let mut text = String::new();
        if up > 0.6 {
            text.push('^');
        } else if up < -0.6 {
            text.push('v');
        }
        let s = self.ui.scale;
        let bw = 182.0 * s;
        let label = format!("{text} {:.0} m", dist);
        let col = [255, 210, 120, 255];
        if ahead < 0.7 {
            let arrow = if ahead < -0.3 { 3 } else { 2 };
            if side > 0.0 {
                let t = format!("{}{label}", ">".repeat(arrow));
                self.ui.text((w + bw) / 2.0 + 6.0 * s, y - s, &t, col);
            } else {
                let t = format!("{label}{}", "<".repeat(arrow));
                let tw = self.ui.text_width(&t, s);
                self.ui.text((w - bw) / 2.0 - 6.0 * s - tw, y - s, &t, col);
            }
        } else if !text.is_empty() {
            self.ui.text((w + bw) / 2.0 + 6.0 * s, y - s, &label, col);
        }
    }
}
