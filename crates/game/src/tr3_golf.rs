//! L17 of the Terraria side (plan/terraria-completo.md, "Reauditoria 3"): golf.
//!
//! - Right click with a golf ball puts it down (on a tee if you look at one); right click near a
//!   ball lying still picks it up again.
//! - Hold right click with a club near the ball: the power swings up and down (Terraria's meter)
//!   while the path the ball will fly is drawn in the air and its landing spot glows on the
//!   ground. Let go to hit, along where you look.
//! - Each club has its own loft: the driver goes far and low, the iron in between, the wedge
//!   high and short, the putter only rolls.
//! - The ball flies with the wind (the wind of wind.rs, told while you aim: with you, against
//!   you, from a side), bounces and rolls by what it lands on: mown golf turf runs far, grass
//!   less, sand stops it dead, ice never stops, slime throws it back up. Water or lava costs a
//!   stroke and the ball goes back to where it last lay.
//! - In a golf cup it drops if it comes in slow enough; too fast it lips out. The strokes are
//!   counted against a par worked out from the length of the hole, for golf points.
//! - The Golfer moves in once you carry a club or sink a ball, sells the gear, and hands out
//!   trophies (and the golf cart's key) as the points grow.
//! - Golf courses lie on the plains (packs/vanilla/tr3/structures.ron): tee box, a bending
//!   fairway, bunkers, the green with its flag, and a shed with gear.

use glam::{IVec3, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use voxel::physics::Terrain;

use crate::audio::{Material, Sfx};
use crate::item_render::GpuItem;
use crate::Game;

const R: f32 = 0.08;
const GRAVITY: f32 = 22.0;

/// (item, loft in degrees, top speed in blocks a second, name)
const CLUBS: &[(&str, f32, f32)] = &[("golf_club_driver", 16.0, 38.0), ("golf_club_iron", 28.0, 28.0), ("golf_club_wedge", 52.0, 19.0), ("golf_club_putter", 0.0, 16.0)];

/// Golf points needed for each trophy the Golfer hands out.
const TROPHIES: &[(u32, &str, &str)] = &[(500, "golf_trophy_bronze", "de Bronze"), (1500, "golf_trophy_silver", "de Prata"), (3000, "golf_trophy_gold", "de Ouro")];

pub struct Ball {
    pos: Vec3,
    vel: Vec3,
    rest: bool,
    strokes: u32,
    /// Where it was first put down (the tee), for the par.
    start: Vec3,
    /// Where it last lay still (it goes back there out of the water).
    last: Vec3,
    tap_t: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedBall {
    pos: (f32, f32, f32),
    strokes: u32,
    start: (f32, f32, f32),
}

#[derive(Default)]
pub struct Golf {
    balls: Vec<Ball>,
    pub points: u32,
    pub holes: u32,
    /// Trophies already handed out (how many of TROPHIES).
    pub trophies: u8,
    /// Seconds the swing has been charging, and which ball.
    charge: Option<(f32, usize)>,
    preview_t: f32,
    hint_t: f32,
}

impl Golf {
    pub fn load(&mut self, balls: Vec<SavedBall>, points: u32, holes: u32, trophies: u8) {
        self.points = points;
        self.holes = holes;
        self.trophies = trophies;
        self.balls = balls
            .into_iter()
            .map(|b| {
                let p = Vec3::new(b.pos.0, b.pos.1, b.pos.2);
                Ball { pos: p, vel: Vec3::ZERO, rest: true, strokes: b.strokes, start: Vec3::new(b.start.0, b.start.1, b.start.2), last: p, tap_t: 0.0 }
            })
            .collect();
    }

    pub fn saved(&self) -> Vec<SavedBall> {
        self.balls.iter().map(|b| SavedBall { pos: (b.last.x, b.last.y, b.last.z), strokes: b.strokes, start: (b.start.x, b.start.y, b.start.z) }).collect()
    }
}

/// How a surface takes the ball: (bounce kept, rolling friction in blocks a second per second).
fn surface(name: &str) -> (f32, f32) {
    if name.starts_with("golf_turf") || name == "golf_cup" {
        (0.32, 3.0)
    } else if name.contains("sand") || name == "gravel" {
        (0.05, 14.0)
    } else if name.contains("ice") {
        (0.4, 0.15)
    } else if name.contains("slime") || name.contains("jelly") {
        (0.92, 2.0)
    } else if matches!(name, "grass_block" | "dirt" | "podzol" | "moss_block" | "mud" | "coarse_dirt" | "rooted_dirt") {
        (0.25, 3.2)
    } else if name.contains("wool") || name.contains("carpet") || name == "hay_block" {
        (0.1, 6.0)
    } else {
        (0.5, 2.4)
    }
}

/// 0 -> 1 -> 0 over two seconds and so on: the power meter.
fn meter(t: f32) -> f32 {
    let x = (t / 1.2) % 2.0;
    1.0 - (x - 1.0).abs()
}

impl Game {
    fn club(&self) -> Option<(f32, f32)> {
        let n = self.held_key();
        CLUBS.iter().find(|c| c.0 == n).map(|c| (c.1, c.2))
    }

    /// The still ball nearest the player's feet that a swing would hit.
    fn ball_at_feet(&self) -> Option<usize> {
        let feet = self.body.pos;
        self.t5
            .tr
            .r3
            .golf
            .balls
            .iter()
            .enumerate()
            .filter(|(_, b)| b.rest && Vec2::new(b.pos.x - feet.x, b.pos.z - feet.z).length() < 3.0 && (b.pos.y - feet.y).abs() < 2.0)
            .min_by(|a, b| a.1.pos.distance(feet).total_cmp(&b.1.pos.distance(feet)))
            .map(|(k, _)| k)
    }

    /// The ball the player looks at (within 4 blocks of the eye).
    fn ball_looked_at(&self) -> Option<usize> {
        let eye = self.body.eye();
        let f = self.forward();
        self.t5.tr.r3.golf.balls.iter().enumerate().filter(|(_, b)| b.rest).find_map(|(k, b)| {
            let to = b.pos - eye;
            let along = to.dot(f);
            (along > 0.0 && along < 4.5 && (to - f * along).length() < 0.45).then_some(k)
        })
    }

    pub(crate) fn golf_use(&mut self, name: &str) -> Option<bool> {
        if CLUBS.iter().any(|c| c.0 == name) {
            // The swing is in the update, while the button is held.
            return Some(true);
        }
        if name != "golf_ball" {
            return None;
        }
        if let Some(k) = self.ball_looked_at() {
            self.t5.tr.r3.golf.balls.remove(k);
            if self.t5.tr.r3.golf.charge.is_some_and(|c| c.1 == k) {
                self.t5.tr.r3.golf.charge = None;
            }
            self.give_or_drop("golf_ball", 1);
            self.sfx(Sfx::Pickup);
            return Some(true);
        }
        let Some((cell, normal)) = self.looked_block() else { return Some(true) };
        let block = self.reg.get(self.world.block_or_air(cell)).name.clone();
        let pos = if block == "golf_tee" {
            cell.as_vec3() + Vec3::new(0.5, 0.42, 0.5)
        } else {
            let top = if normal == IVec3::Y { cell + IVec3::Y } else { cell + IVec3::Y };
            if self.world.solid(top) {
                return Some(true);
            }
            top.as_vec3() + Vec3::new(0.5, R + 0.01, 0.5)
        };
        if pos.distance(self.body.pos) > 6.0 {
            return Some(true);
        }
        self.t5.tr.r3.golf.balls.push(Ball { pos, vel: Vec3::ZERO, rest: true, strokes: 0, start: pos, last: pos, tap_t: 0.0 });
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.sfx(Sfx::Place(Material::Stone));
        Some(true)
    }

    pub(crate) fn update_golf(&mut self, dt: f32, use_held: bool) {
        let g = &mut self.t5.tr.r3.golf;
        g.hint_t -= dt;
        g.preview_t -= dt;
        if let Some((angle, top)) = self.club().filter(|_| self.screen == crate::Screen::Playing) {
            match self.t5.tr.r3.golf.charge {
                None if use_held => {
                    if let Some(k) = self.ball_at_feet() {
                        self.t5.tr.r3.golf.charge = Some((0.0, k));
                    } else if self.t5.tr.r3.golf.hint_t <= 0.0 {
                        self.t5.tr.r3.golf.hint_t = 4.0;
                        self.toast("Ponha uma bola de golfe no chão e chegue perto dela", 2.5);
                    }
                }
                Some((t, k)) if use_held => {
                    let t = t + dt;
                    self.t5.tr.r3.golf.charge = Some((t, k));
                    self.aim_golf(k, angle, top, meter(t));
                }
                Some((t, k)) => {
                    self.t5.tr.r3.golf.charge = None;
                    self.swing_golf(k, angle, top, meter(t));
                }
                None => {}
            }
        } else {
            self.t5.tr.r3.golf.charge = None;
        }
        if dt > 0.0 {
            let n = (dt / (1.0 / 120.0)).ceil().clamp(1.0, 8.0) as usize;
            for k in (0..self.t5.tr.r3.golf.balls.len()).rev() {
                for _ in 0..n {
                    if self.step_ball(k, dt / n as f32) {
                        break;
                    }
                }
            }
        }
    }

    /// The launch velocity of a swing with this club at `power` (0..1).
    fn golf_launch(&self, angle: f32, top: f32, power: f32) -> Vec3 {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let a = angle.to_radians();
        let v = top * (0.1 + 0.9 * power);
        flat * a.cos() * v + Vec3::Y * a.sin() * v
    }

    fn golf_wind(&self, at: Vec3) -> Vec3 {
        let rain = if self.weather.state.raining { 1.0 } else { 0.0 };
        crate::wind::at(self.now, Vec2::new(at.x, at.z), rain, self.weather.state.thunder) * 0.45
    }

    /// While aiming: the meter, the wind, the flight path in the air and where it lands.
    fn aim_golf(&mut self, k: usize, angle: f32, top: f32, power: f32) {
        let Some(b) = self.t5.tr.r3.golf.balls.get(k) else { return };
        let start = b.pos;
        let wind = self.golf_wind(start);
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let along = wind.dot(flat);
        let across = wind.dot(Vec3::new(-flat.z, 0.0, flat.x));
        let wtext = if wind.length() < 0.15 {
            "sem vento".to_string()
        } else if along.abs() > across.abs() {
            format!("vento {} {:.1}", if along > 0.0 { "a favor" } else { "contra" }, wind.length() / 0.45)
        } else {
            format!("vento da {} {:.1}", if across > 0.0 { "esquerda" } else { "direita" }, wind.length() / 0.45)
        };
        let bars = (power * 10.0).round() as usize;
        self.toast(format!("Força {}{} {:.0}%  ·  {wtext}", "|".repeat(bars), ".".repeat(10 - bars), power * 100.0), 0.2);
        if self.t5.tr.r3.golf.preview_t > 0.0 {
            return;
        }
        self.t5.tr.r3.golf.preview_t = 0.08;
        // Fly it ahead without bounces: dots along the way, a glow where it comes down.
        let mut p = start;
        let mut v = self.golf_launch(angle, top, power);
        let glow = self.glow_layer();
        let h = 1.0 / 30.0;
        for step in 0..150 {
            if angle > 0.0 {
                v.y -= GRAVITY * h;
                v += wind * h;
            }
            let np = p + v * h;
            if self.world.solid(np.floor().as_ivec3()) || (angle == 0.0 && step > 0 && (np - start).length() > v.length() * v.length() / (2.0 * 1.1)) {
                self.ground_mark(Vec3::new(p.x, p.y.floor() + 0.05, p.z), 0.12, [1.6, 1.6, 1.6]);
                break;
            }
            if angle == 0.0 {
                v *= 1.0 - (1.1 / v.length().max(0.5)) * h;
            }
            p = np;
            if step % 3 == 0 {
                let fade = 1.0 - step as f32 / 150.0;
                self.particles.glow(p, Vec3::ZERO, glow, [1.4 * fade + 0.4, 1.4 * fade + 0.4, 1.6], 0.035, 0.1);
            }
        }
    }

    fn swing_golf(&mut self, k: usize, angle: f32, top: f32, power: f32) {
        let vel = self.golf_launch(angle, top, power);
        let Some(b) = self.t5.tr.r3.golf.balls.get_mut(k) else { return };
        if !b.rest {
            return;
        }
        b.vel = vel;
        b.rest = false;
        b.strokes += 1;
        b.last = b.pos;
        let strokes = b.strokes;
        self.anim.start_swing();
        self.sfx(Sfx::Fx(crate::sfx_extra::Fx::Swing));
        self.sfx(Sfx::Hit(Material::Wood));
        if strokes > 1 {
            self.toast(format!("Tacada {strokes}"), 1.2);
        }
    }

    /// One small step of a ball's flight; true when the ball is gone (holed, lost).
    fn step_ball(&mut self, k: usize, h: f32) -> bool {
        let wind = self.golf_wind(self.t5.tr.r3.golf.balls[k].pos);
        let (rest, mut pos, mut vel) = {
            let b = &self.t5.tr.r3.golf.balls[k];
            (b.rest, b.pos, b.vel)
        };
        if rest {
            // Still balls stay unless what holds them up goes away.
            let under = (pos - Vec3::Y * (R + 0.05)).floor().as_ivec3();
            if self.world.solid(under) || self.world.block(under).is_none() || self.reg.get(self.world.block_or_air(under)).name == "golf_tee" {
                return false;
            }
            self.t5.tr.r3.golf.balls[k].rest = false;
        }
        let cell = pos.floor().as_ivec3();
        if self.world.block(cell).is_none() {
            // Off in unloaded land: wait there.
            let b = &mut self.t5.tr.r3.golf.balls[k];
            b.rest = true;
            b.vel = Vec3::ZERO;
            return true;
        }
        // Into water or lava: a stroke lost, back to where it lay.
        let here = self.world.block_or_air(cell);
        let fluid = self.world.fluid(here);
        if fluid > 0 || pos.y < -800.0 {
            let b = &mut self.t5.tr.r3.golf.balls[k];
            b.pos = b.last;
            b.vel = Vec3::ZERO;
            b.rest = true;
            b.strokes += 1;
            if fluid == 2 {
                self.sfx(Sfx::Fizz);
                self.toast("Na lava! +1 tacada, a bola volta", 2.5);
            } else {
                let water = self.reg.get(self.reg.id_or_air("water")).faces[2];
                self.particles.splash(pos, water, 0.4);
                self.sfx(Sfx::Splash(0.3));
                self.toast("Na água! +1 tacada, a bola volta", 2.5);
            }
            return false;
        }
        vel.y -= GRAVITY * h;
        let support = (pos - Vec3::Y * (R + 0.02)).floor().as_ivec3();
        let grounded = self.world.solid(support);
        if !grounded {
            vel += wind * h;
            vel *= 1.0 - 0.08 * h;
        }
        let mut tap = None;
        for axis in [0usize, 2, 1] {
            let mut np = pos;
            np[axis] += vel[axis] * h;
            let mut probe = np;
            probe[axis] += R * vel[axis].signum();
            let c = probe.floor().as_ivec3();
            if !self.world.solid(c) {
                pos = np;
                continue;
            }
            let name = self.reg.get(self.world.block_or_air(c)).name.as_str();
            let (bounce, _) = surface(name);
            if axis == 1 && vel.y < 0.0 {
                if -vel.y > 2.2 {
                    tap = Some(-vel.y);
                    vel.y = -vel.y * bounce;
                    // The spin bites as it lands: soft ground takes more of the run out.
                    let keep = 0.55 + 0.4 * bounce;
                    vel.x *= keep;
                    vel.z *= keep;
                } else {
                    vel.y = 0.0;
                }
                pos.y = c.y as f32 + 1.0 + R;
            } else {
                if vel[axis].abs() > 2.0 {
                    tap = Some(vel[axis].abs());
                }
                vel[axis] = -vel[axis] * 0.45;
            }
        }
        // Rolling: the ground under it brakes it.
        let support = (pos - Vec3::Y * (R + 0.02)).floor().as_ivec3();
        let mut holed = false;
        if self.world.solid(support) && vel.y.abs() < 0.5 {
            let ground = self.reg.get(self.world.block_or_air(support)).name.clone();
            let (_, fric) = surface(&ground);
            let flat = Vec2::new(vel.x, vel.z);
            let speed = flat.length();
            // Grass drags harder the faster it rolls; ice hardly at all.
            let drag = if ground.contains("ice") { 0.05 } else { 0.6 };
            let left = (speed - (fric + drag * speed) * h).max(0.0);
            let scale = if speed > 0.0 { left / speed } else { 0.0 };
            vel.x *= scale;
            vel.z *= scale;
            if ground == "golf_cup" {
                let c = support.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
                if Vec2::new(pos.x - c.x, pos.z - c.z).length() < 0.34 {
                    if speed < 5.0 {
                        holed = true;
                    } else {
                        // Too fast: it rattles over the rim and hops on.
                        vel.y = 2.4;
                        tap = Some(3.0);
                    }
                }
            }
            if left < 0.06 && !holed {
                let b = &mut self.t5.tr.r3.golf.balls[k];
                b.pos = pos;
                b.vel = Vec3::ZERO;
                b.rest = true;
                b.last = pos;
                return false;
            }
        }
        {
            let b = &mut self.t5.tr.r3.golf.balls[k];
            b.pos = pos;
            b.vel = vel;
            b.tap_t -= h;
        }
        if let Some(s) = tap {
            if self.t5.tr.r3.golf.balls[k].tap_t <= 0.0 {
                self.t5.tr.r3.golf.balls[k].tap_t = 0.12;
                self.sfx_at(Sfx::Hit(Material::Stone), pos, (s / 20.0).clamp(0.15, 0.7), 24.0);
            }
        }
        if holed {
            self.ball_holed(k);
            return true;
        }
        false
    }

    fn ball_holed(&mut self, k: usize) {
        let b = self.t5.tr.r3.golf.balls.remove(k);
        if self.t5.tr.r3.golf.charge.is_some_and(|c| c.1 >= k) {
            self.t5.tr.r3.golf.charge = None;
        }
        let dist = Vec2::new(b.pos.x - b.start.x, b.pos.z - b.start.z).length();
        let par = if dist < 24.0 {
            3
        } else if dist < 48.0 {
            4
        } else {
            5
        };
        let strokes = b.strokes.max(1);
        let diff = strokes as i32 - par;
        let call = match (strokes, diff) {
            (1, _) => "Hole-in-one!".to_string(),
            (_, -3) => "Albatroz!".to_string(),
            (_, -2) => "Eagle!".to_string(),
            (_, -1) => "Birdie!".to_string(),
            (_, 0) => "Par.".to_string(),
            (_, 1) => "Bogey.".to_string(),
            (_, 2) => "Bogey duplo.".to_string(),
            (_, d) => format!("+{d}."),
        };
        let mut pts = (par - strokes as i32 + 3).max(0) as u32 * 50;
        if strokes == 1 {
            pts += 300;
        }
        let g = &mut self.t5.tr.r3.golf;
        g.points += pts;
        g.holes += 1;
        self.toast(format!("Buraco em {strokes} tacada{} (par {par}): {call} +{pts} pontos de golfe", if strokes == 1 { "" } else { "s" }), 5.0);
        self.sfx(Sfx::Xp);
        let glow = self.glow_layer();
        let at = b.pos + Vec3::Y * 0.3;
        let colours = [[2.0, 0.4, 0.4], [0.4, 1.8, 0.5], [0.5, 0.7, 2.0], [2.0, 1.8, 0.4], [1.8, 0.5, 1.8]];
        let n = if strokes == 1 { 120 } else { 40 };
        for i in 0..n {
            let r = &mut self.surv.rng;
            let v = Vec3::new(r.f32() - 0.5, 1.0 + r.f32(), r.f32() - 0.5) * if strokes == 1 { 7.0 } else { 4.0 };
            self.particles.glow(at, v, glow, colours[i % colours.len()], 0.07, 1.4);
        }
        if strokes == 1 {
            self.sfx(Sfx::Thunder(0.15));
        }
        self.give_or_drop("golf_ball", 1);
    }

    /// The Golfer: tells the score and hands out the trophies (and the cart's key).
    pub(crate) fn golfer_talk(&mut self, dn: &str) {
        let g = &self.t5.tr.r3.golf;
        let (points, holes, had) = (g.points, g.holes, g.trophies as usize);
        let earned = TROPHIES.iter().filter(|t| points >= t.0).count();
        if earned > had {
            for t in &TROPHIES[had..earned] {
                self.give_or_drop(t.1, 1);
            }
            if had < 2 && earned >= 2 {
                self.give_or_drop("golf_cart_key", 1);
            }
            self.t5.tr.r3.golf.trophies = earned as u8;
            let last = TROPHIES[earned - 1].2;
            self.toast(format!("{dn}: Que jogo! Leve o Troféu de Golfe {last}.{}", if had < 2 && earned >= 2 { " E a chave do meu carrinho!" } else { "" }), 5.0);
            self.sfx(Sfx::Xp);
            return;
        }
        let next = TROPHIES.iter().find(|t| points < t.0).map(|t| format!(" Faltam {} para o troféu {}.", t.0 - points, t.2)).unwrap_or_default();
        self.toast(format!("{dn}: {points} pontos de golfe em {holes} buraco{}.{next}", if holes == 1 { "" } else { "s" }), 5.0);
    }

    /// The balls as little round pictures facing the eye, for the renderer.
    pub(crate) fn golf_instances(&self, cam_block: IVec3) -> Vec<GpuItem> {
        let g = &self.t5.tr.r3.golf;
        if g.balls.is_empty() {
            return Vec::new();
        }
        let Some(id) = self.items.id("golf_ball") else { return Vec::new() };
        let voxel::item::Look::Sprite(l) = self.items.get(id).look else { return Vec::new() };
        let layer = (self.item_base + l) as f32;
        let c = cam_block.as_vec3();
        let eye = self.body.eye();
        g.balls
            .iter()
            .filter(|b| b.pos.distance(eye) < 96.0)
            .map(|b| {
                let li = self.world.light(b.pos.floor().as_ivec3());
                let (sky, blk) = ((li >> 4) as f32 / 15.0, (li & 15) as f32 / 15.0);
                let to = eye - b.pos;
                let p = b.pos - c;
                GpuItem { pos: [p.x, p.y, p.z, R * 2.6], info: [to.x.atan2(to.z), 1.0, sky, blk], layers: [layer, layer, layer, 0.0], ..bytemuck::Zeroable::zeroed() }
            })
            .collect()
    }

    /// Test scenes: MC_TR=golf (a green, the ball on the tee, the putter aimed), golf_aim (the
    /// driver's flight drawn), golf_items, golf_course (the course from above), golf_cart.
    pub(crate) fn test_golf(&mut self, scene: &str) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let base = self.body.pos.floor().as_ivec3();
        let y0 = self.world.surface_y(base.x, base.z).unwrap_or(base.y - 1);
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        match scene {
            "golf_items" => {
                self.survival = true;
                let names = ["golf_club_driver", "golf_club_iron", "golf_club_wedge", "golf_club_putter", "golf_ball", "golf_tee", "golf_cup", "golf_flag", "golf_turf", "golf_cart_key", "golf_trophy_bronze", "golf_trophy_silver", "golf_trophy_gold"];
                for (k, n) in names.iter().enumerate() {
                    if let Some(id) = self.items.id(n) {
                        self.surv.inv.slots[k] = Some(voxel::item::ItemStack::new(id, 1));
                    }
                }
                self.args.show_inventory = true;
            }
            "golf" | "golf_aim" | "golf_sim_strip" => {
                // A strip of turf ahead with a bunker, the cup and flag, the ball on a tee.
                let turf = self.reg.id_or_air("golf_turf");
                let sand = self.reg.id_or_air("sand");
                // The simulation gets a long, wide fairway so no flight ends in a pond.
                let (wide, long) = if scene == "golf_sim_strip" { (14, 170) } else { (4, 40) };
                for a in -wide..=wide {
                    for d in -2..=long {
                        let p = (self.body.pos + flat * d as f32 + side * a as f32).floor().as_ivec3();
                        let p = IVec3::new(p.x, y0, p.z);
                        let bunker = (d as f32 - 22.0).hypot(a as f32 * 1.4 - 2.0) < 2.6;
                        self.world.set_block(p, if bunker { sand } else { turf });
                        for up in 1..6 {
                            self.world.set_block(p + IVec3::Y * up, voxel::AIR);
                        }
                    }
                }
                let cup = (self.body.pos + flat * 34.0).floor().as_ivec3();
                let cup = IVec3::new(cup.x, y0, cup.z);
                self.world.set_block(cup, self.reg.id_or_air("golf_cup"));
                self.world.set_block(cup + IVec3::Y, self.reg.id_or_air("golf_flag"));
                let tee = (self.body.pos + flat * 1.2).floor().as_ivec3();
                let tee = IVec3::new(tee.x, y0 + 1, tee.z);
                self.world.set_block(tee, self.reg.id_or_air("golf_tee"));
                let pos = tee.as_vec3() + Vec3::new(0.5, 0.42, 0.5);
                self.t5.tr.r3.golf.balls.push(Ball { pos, vel: Vec3::ZERO, rest: true, strokes: 0, start: pos, last: pos, tap_t: 0.0 });
                let club = if scene == "golf" { "golf_club_iron" } else { "golf_club_driver" };
                if let Some(id) = self.items.id(club) {
                    self.surv.inv.slots[self.selected] = Some(voxel::item::ItemStack::new(id, 1));
                }
                let p = self.body.pos + flat * 5.0 - side * 3.0;
                self.mobs.spawn("npc_golfer", Vec3::new(p.x, ground(self, p), p.z));
                if let Some(m) = self.mobs.list.last_mut() {
                    let look = self.body.eye() - m.body.center();
                    m.yaw = look.x.atan2(look.z);
                }
                let (angle, top) = CLUBS.iter().find(|c| c.0 == club).map(|c| (c.1, c.2)).unwrap();
                self.pitch = -0.3;
                self.aim_golf(0, angle, top, if scene == "golf" { 0.55 } else { 0.8 });
            }
            "golf_sim" => {
                // Flights logged: each club at a few powers from the tee, then putts at a cup.
                self.test_golf("golf_sim_strip");
                let start = self.t5.tr.r3.golf.balls[0].pos;
                for &(club, angle, top) in CLUBS {
                    for power in [0.3, 0.6, 1.0] {
                        let b = &mut self.t5.tr.r3.golf.balls[0];
                        b.pos = start;
                        b.rest = true;
                        b.strokes = 0;
                        self.swing_golf(0, angle, top, power);
                        let (mut t, mut far, mut top) = (0.0, 0.0f32, 0.0f32);
                        while t < 20.0 && !self.t5.tr.r3.golf.balls[0].rest {
                            self.update_golf(1.0 / 60.0, false);
                            t += 1.0 / 60.0;
                            let b = &self.t5.tr.r3.golf.balls[0];
                            far = far.max(Vec2::new(b.pos.x - start.x, b.pos.z - start.z).length());
                            top = top.max(b.pos.y - start.y);
                        }
                        let b = &self.t5.tr.r3.golf.balls[0];
                        log::info!("golf_sim {club} power {power}: rests after {t:.1}s, {:.1} blocks away at y {:.2} (went {far:.1} out, {top:.1} up)", Vec2::new(b.pos.x - start.x, b.pos.z - start.z).length(), b.pos.y);
                    }
                }
                // Putting at the cup from 6 blocks: find a power that sinks it.
                let f = self.forward();
                let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
                let cup = (self.body.pos + flat * 34.0).floor();
                for power in [0.3, 0.4, 0.5, 0.55, 0.6, 0.65, 0.7, 0.8, 1.0] {
                    if self.t5.tr.r3.golf.balls.is_empty() {
                        let p = Vec3::ZERO;
                        self.t5.tr.r3.golf.balls.push(Ball { pos: p, vel: Vec3::ZERO, rest: true, strokes: 0, start: p, last: p, tap_t: 0.0 });
                    }
                    let b = &mut self.t5.tr.r3.golf.balls[0];
                    b.pos = Vec3::new(cup.x + 0.5, start.y - 0.34 + R, cup.z + 0.5) - flat * 6.0;
                    b.rest = true;
                    b.strokes = 0;
                    self.body.pos = b.pos - flat * 1.0;
                    self.swing_golf(0, 0.0, 16.0, power);
                    let mut t = 0.0;
                    while t < 20.0 && self.t5.tr.r3.golf.balls.first().is_some_and(|b| !b.rest) {
                        self.update_golf(1.0 / 60.0, false);
                        t += 1.0 / 60.0;
                    }
                    let left = self.t5.tr.r3.golf.balls.first().map(|b| Vec2::new(b.pos.x - cup.x - 0.5, b.pos.z - cup.z - 0.5).length());
                    log::info!("golf_sim putt power {power}: {}", left.map(|d| format!("stops {d:.2} from the cup")).unwrap_or("holed!".into()));
                }
            }
            "golf_cart" => {
                for (k, n) in ["mount_golf_cart", "npc_golfer"].iter().enumerate() {
                    let p = self.body.pos + flat * 5.0 + side * (k as f32 * 3.0 - 1.5);
                    self.mobs.spawn(n, Vec3::new(p.x, ground(self, p), p.z));
                    if let Some(m) = self.mobs.list.last_mut() {
                        m.yaw = 0.8;
                    }
                }
                self.t5.tr.pets.test_keep();
            }
            "golf_course" => {
                let (x, z) = (self.body.pos.x as i32, self.body.pos.z as i32);
                if let Some((o, size)) = self.world.gen.find_structure("campo_de_golfe", x, z, 60) {
                    let c = o.as_vec3() + size.as_vec3() * 0.5;
                    let y = self.world.gen.height_at(c.x as i32, c.z as i32) as f32;
                    self.body.pos = Vec3::new(o.x as f32 - 10.0, y + 22.0, o.z as f32 - 6.0);
                    let to = c - self.body.pos;
                    self.yaw = (-to.x).atan2(-to.z);
                    self.pitch = -0.6;
                    self.body.flying = true;
                    log::info!("campo_de_golfe at {o} size {size}");
                } else {
                    log::warn!("no campo_de_golfe found");
                }
            }
            _ => {}
        }
    }
}
