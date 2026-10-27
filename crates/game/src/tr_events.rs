//! Lote 2 of the Terraria side (plan/terraria-completo.md): the world's events.
//! - Meteors: some nights, once a boss has fallen, a meteor streaks down and leaves a crater
//!   lined with meteorite. Burning heads float around the craters.
//! - Falling stars: on clear nights stars drop from the sky and land as fallen stars.
//! - Jelly rain: some mornings jellies fall from the sky; kill enough and the Jelly King comes.
//! - Sandstorms: now and then the wind rises over the deserts, full of grit and sand wraiths.
//! - Lantern night: after a boss falls (or by luck) lanterns rise all night and no monsters
//!   spawn on the surface.
//! - Solar eclipse (World Breaker age, or a solar tablet): the day goes dark and reapers, shade
//!   brutes and the eclipse moth come out.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::physics::Terrain;
use voxel::worldgen::{Biome, Dim};

use crate::audio::Sfx;
use crate::Game;

const JELLY_RAIN: [&str; 7] = ["jelly_green", "jelly_green", "jelly_blue", "jelly_blue", "jelly_red", "jelly_green", "jelly_pink"];
const ECLIPSE_MOBS: [&str; 3] = ["reaper", "shade_brute", "eclipse_moth"];
/// Craters remembered (for their floating heads); the oldest is forgotten first.
const MAX_CRATERS: usize = 8;

struct Falling {
    from: Vec3,
    to: Vec3,
    t: f32,
}

struct JellyRain {
    killed: u32,
    need: u32,
    spawn_t: f32,
    left: f32,
}

#[derive(Default)]
pub struct Events {
    pub(crate) craters: Vec<IVec3>,
    night_was: bool,
    pub(crate) meteor_in: Option<f32>,
    falling: Option<Falling>,
    /// Stars in the sky on their way down: (position, velocity).
    stars: Vec<(Vec3, Vec3)>,
    star_t: f32,
    rain: Option<JellyRain>,
    /// Seconds left of the sandstorm (0: none), its wind, how strong it is where you stand.
    storm: f32,
    storm_check: f32,
    wind: Vec3,
    storm_k: f32,
    storm_force: bool,
    /// Tonight is a lantern night, and the lanterns rising: (position, seconds left).
    lantern_night: bool,
    lanterns: Vec<(Vec3, f32)>,
    /// A boss fell since the last dusk (a lantern night follows).
    boss_recent: bool,
    pub(crate) eclipse: bool,
    spawn_t: f32,
    head_t: f32,
}

impl Game {
    pub(crate) fn update_tr_events(&mut self, dt: f32) {
        let night = self.is_night();
        let over = self.dim == Dim::Overworld;
        let dusk = night && !self.t5.tr.ev.night_was;
        let dawn = !night && self.t5.tr.ev.night_was;
        self.t5.tr.ev.night_was = night;
        if dusk {
            self.events_at_dusk(over);
        }
        if dawn {
            self.events_at_dawn(over);
        }
        self.mobs.calm = self.t5.tr.ev.lantern_night && over;
        self.update_meteors(dt, over);
        self.update_stars(dt, night && over);
        self.update_jelly_rain(dt, over);
        self.update_sandstorm(dt, over);
        self.update_lanterns(dt, night && over);
        self.update_eclipse(dt, over);
    }

    fn events_at_dusk(&mut self, over: bool) {
        let ev = &mut self.t5.tr.ev;
        if ev.eclipse {
            ev.eclipse = false;
            self.toast("O eclipse terminou", 3.0);
        }
        if self.t5.tr.ev.rain.take().is_some() {
            self.toast("A chuva de geleias parou", 3.0);
        }
        if !over || !self.survival {
            return;
        }
        let recent = std::mem::take(&mut self.t5.tr.ev.boss_recent);
        if !self.t5.hard.blood && (recent || self.surv.rng.f32() < 0.07) {
            self.t5.tr.ev.lantern_night = true;
            self.toast("Lanternas sobem ao céu... esta noite será tranquila.", 5.0);
        }
        // Meteors come once the world has seen a boss fall.
        if !self.boss.defeated.is_empty() && self.surv.rng.f32() < 0.14 {
            self.t5.tr.ev.meteor_in = Some(60.0 + self.surv.rng.f32() * 120.0);
        }
    }

    fn events_at_dawn(&mut self, over: bool) {
        self.t5.tr.ev.lantern_night = false;
        if !over || !self.survival {
            return;
        }
        let roll = self.surv.rng.f32();
        if self.t5.hard.on && roll < 0.05 {
            self.start_eclipse();
        } else if roll > 0.94 {
            self.start_jelly_rain();
        }
    }

    pub(crate) fn tr_event_boss_down(&mut self) {
        self.t5.tr.ev.boss_recent = true;
    }

    /// Mobs killed this frame: jellies count for the jelly rain.
    pub(crate) fn tr_deaths(&mut self, deaths: &[(usize, Vec3)]) {
        self.tr_soul_drops(deaths);
        self.endgame_deaths(deaths);
        self.invasion_deaths(deaths);
        let Some(rain) = self.t5.tr.ev.rain.as_mut() else { return };
        for &(def, _) in deaths {
            if self.mobs.reg.defs[def].name.starts_with("jelly_") {
                rain.killed += 1;
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Meteors

    fn update_meteors(&mut self, dt: f32, over: bool) {
        if let Some(t) = self.t5.tr.ev.meteor_in.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.t5.tr.ev.meteor_in = None;
                if over {
                    self.launch_meteor();
                }
            }
        }
        if let Some(f) = self.t5.tr.ev.falling.as_mut() {
            f.t += dt / 3.0;
            let (from, to, t) = (f.from, f.to, f.t);
            let at = from.lerp(to, t.min(1.0));
            let glow = self.glow_layer();
            for k in 0..6 {
                let back = at + (from - to).normalize_or_zero() * k as f32 * 0.8;
                self.particles.glow(back, Vec3::ZERO, glow, [2.2, 1.0, 0.3], 0.45 - k as f32 * 0.05, 0.5);
            }
            if t >= 1.0 {
                self.t5.tr.ev.falling = None;
                self.meteor_impact(to);
            }
        }
        // Burning heads float around the craters you are near.
        self.t5.tr.ev.head_t -= dt;
        if self.t5.tr.ev.head_t > 0.0 || !over || self.mobs.peaceful {
            return;
        }
        self.t5.tr.ev.head_t = 6.0;
        let me = self.body.pos;
        let Some(c) = self.t5.tr.ev.craters.iter().copied().find(|c| c.as_vec3().distance(me) < 40.0) else { return };
        if self.count_tr(&["meteor_head"]) < 3 {
            let a = self.surv.rng.f32() * std::f32::consts::TAU;
            let at = c.as_vec3() + Vec3::new(a.cos() * 5.0, 6.0, a.sin() * 5.0);
            if self.mobs.spawn("meteor_head", at) {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.aggro = true;
                }
            }
        }
    }

    /// A meteor streaks down somewhere 40 to 70 blocks away.
    fn launch_meteor(&mut self) {
        for _ in 0..8 {
            let a = self.surv.rng.f32() * std::f32::consts::TAU;
            let r = 40.0 + self.surv.rng.f32() * 30.0;
            let x = (self.body.pos.x + a.cos() * r).floor() as i32;
            let z = (self.body.pos.z + a.sin() * r).floor() as i32;
            let Some(y) = self.world.surface_y(x, z) else { continue };
            let to = Vec3::new(x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5);
            let from = to + Vec3::new(-a.cos() * 50.0, 90.0, -a.sin() * 50.0);
            self.t5.tr.ev.falling = Some(Falling { from, to, t: 0.0 });
            self.sfx(Sfx::Thunder(0.3));
            return;
        }
    }

    fn meteor_impact(&mut self, at: Vec3) {
        let c = at.floor().as_ivec3();
        self.make_crater(c);
        self.weather.flash = 0.6;
        let near = (1.0 - at.distance(self.body.pos) / 90.0).clamp(0.2, 1.0);
        self.sfx(Sfx::Thunder(near));
        self.sfx(Sfx::Explode);
        let d = at - self.body.pos;
        let dir = if d.x.abs() > d.z.abs() {
            if d.x > 0.0 { "leste" } else { "oeste" }
        } else if d.z > 0.0 {
            "sul"
        } else {
            "norte"
        };
        self.toast(&format!("Um meteoro caiu ao {dir}!"), 5.0);
    }

    /// A bowl blown out of the ground, its walls turned to meteorite.
    pub(crate) fn make_crater(&mut self, c: IVec3) {
        let ore = self.reg.id_or_air("meteorite_ore");
        if ore == voxel::AIR {
            return;
        }
        const R: i32 = 7;
        for dx in -R..=R {
            for dy in -R..=3 {
                for dz in -R..=R {
                    let p = c + IVec3::new(dx, dy, dz);
                    let d = ((dx * dx + dz * dz) as f32 + (dy * dy) as f32 * 1.7).sqrt();
                    let id = self.world.block_or_air(p);
                    if d < 5.5 {
                        if id != voxel::AIR {
                            self.world.set_block(p, voxel::AIR);
                        }
                    } else if d < 7.0 && dy <= 0 && id != voxel::AIR && !self.world.liquid(p) {
                        self.world.set_block(p, ore);
                    }
                }
            }
        }
        // A heap of meteorite in the middle of the bowl.
        let floor = (0..8).map(|k| c - IVec3::Y * k).find(|p| self.world.block_or_air(*p - IVec3::Y) != voxel::AIR).unwrap_or(c);
        for p in [floor, floor + IVec3::X, floor - IVec3::Z, floor + IVec3::Y] {
            self.world.set_block(p, ore);
        }
        let ev = &mut self.t5.tr.ev;
        ev.craters.push(c);
        if ev.craters.len() > MAX_CRATERS {
            ev.craters.remove(0);
        }
        let glow = self.glow_layer();
        for _ in 0..60 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5) * 12.0;
            self.particles.glow(c.as_vec3() + Vec3::Y, v, glow, [2.0, 0.9, 0.3], 0.2, 1.4);
        }
    }

    // ---------------------------------------------------------------------------------------
    // Falling stars

    fn update_stars(&mut self, dt: f32, open_night: bool) {
        if open_night && self.body.pos.y > 30.0 {
            self.t5.tr.ev.star_t -= dt;
            if self.t5.tr.ev.star_t <= 0.0 {
                self.t5.tr.ev.star_t = 8.0 + self.surv.rng.f32() * 14.0;
                if self.t5.tr.ev.stars.len() < 4 {
                    let r = &mut self.surv.rng;
                    let off = Vec3::new(r.f32() * 60.0 - 30.0, 50.0, r.f32() * 60.0 - 30.0);
                    let drift = Vec3::new(r.f32() - 0.5, 0.0, r.f32() - 0.5) * 10.0;
                    self.t5.tr.ev.stars.push((self.body.pos + off, drift - Vec3::Y * 20.0));
                }
            }
        }
        if self.t5.tr.ev.stars.is_empty() {
            return;
        }
        let glow = self.glow_layer();
        let star = self.items.id("fallen_star");
        let mut stars = std::mem::take(&mut self.t5.tr.ev.stars);
        stars.retain_mut(|(pos, vel)| {
            *pos += *vel * dt;
            self.particles.glow(*pos, Vec3::ZERO, glow, [2.0, 1.8, 0.9], 0.22, 0.25);
            self.particles.glow(*pos - *vel * 0.04, Vec3::ZERO, glow, [1.2, 1.0, 0.5], 0.14, 0.4);
            let p = pos.floor().as_ivec3();
            let Some(top) = self.world.surface_y(p.x, p.z) else { return false };
            if p.y > top && !self.world.solid(p) {
                return true;
            }
            let land = Vec3::new(pos.x, top as f32 + 1.2, pos.z);
            if let Some(id) = star {
                self.spawn_drop(land, ItemStack::new(id, 1), Vec3::ZERO);
            }
            for _ in 0..14 {
                let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5) * 4.0;
                self.particles.glow(land, v, glow, [2.0, 1.8, 0.9], 0.1, 0.8);
            }
            false
        });
        self.t5.tr.ev.stars = stars;
    }

    // ---------------------------------------------------------------------------------------
    // Jelly rain

    pub(crate) fn start_jelly_rain(&mut self) {
        let beaten = self.boss.defeated.iter().any(|b| b == "king_jelly");
        let need = if beaten { 30 } else { 60 };
        self.t5.tr.ev.rain = Some(JellyRain { killed: 0, need, spawn_t: 2.0, left: 240.0 });
        self.toast("Está chovendo geleias!", 4.0);
    }

    fn update_jelly_rain(&mut self, dt: f32, over: bool) {
        let Some(rain) = self.t5.tr.ev.rain.as_mut() else { return };
        rain.left -= dt;
        rain.spawn_t -= dt;
        let (killed, need, left) = (rain.killed, rain.need, rain.left);
        let spawn = rain.spawn_t <= 0.0;
        if spawn {
            rain.spawn_t = 1.2;
        }
        if killed >= need {
            self.t5.tr.ev.rain = None;
            let f = self.forward();
            let ahead = self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 14.0;
            let y = self.world.surface_y(ahead.x.floor() as i32, ahead.z.floor() as i32).map_or(ahead.y, |y| y as f32 + 8.0);
            if self.spawn_tr_boss("king_jelly", Vec3::new(ahead.x, y, ahead.z)) {
                self.toast("O Rei Geleia caiu do céu!", 4.0);
                self.sfx(Sfx::Thunder(0.5));
            }
            return;
        }
        if left <= 0.0 || !over {
            self.t5.tr.ev.rain = None;
            self.toast("A chuva de geleias parou", 3.0);
            return;
        }
        if spawn && !self.mobs.peaceful && self.count_tr(&JELLY_RAIN) < 14 {
            let name = JELLY_RAIN[(self.surv.rng.f32() * JELLY_RAIN.len() as f32) as usize % JELLY_RAIN.len()];
            self.spawn_around(name, true);
        }
    }

    // ---------------------------------------------------------------------------------------
    // Sandstorm

    fn update_sandstorm(&mut self, dt: f32, over: bool) {
        let ev = &mut self.t5.tr.ev;
        ev.storm = (ev.storm - dt).max(0.0);
        ev.storm_check -= dt;
        if ev.storm_check <= 0.0 {
            ev.storm_check = 120.0;
            if ev.storm <= 0.0 && self.surv.rng.f32() < 0.22 {
                ev.storm = 60.0 + self.surv.rng.f32() * 100.0;
                let a = self.surv.rng.f32() * std::f32::consts::TAU;
                ev.wind = Vec3::new(a.cos(), 0.0, a.sin()) * 7.0;
            }
        }
        let p = self.body.pos.floor().as_ivec3();
        let desert = matches!(self.world.gen.biome_at(p.x, p.z), Biome::Desert | Biome::Badlands);
        let open = self.world.surface_y(p.x, p.z).map_or(true, |y| y <= p.y + 1);
        let ev = &mut self.t5.tr.ev;
        let want = if ev.storm > 0.0 && over && open && (desert || ev.storm_force) { 1.0 } else { 0.0 };
        ev.storm_k += (want - ev.storm_k) * (dt * 0.6).min(1.0);
        let k = ev.storm_k;
        if k < 0.05 {
            return;
        }
        if ev.wind == Vec3::ZERO {
            ev.wind = Vec3::new(1.0, 0.0, 0.3).normalize() * 7.0;
        }
        let wind = ev.wind;
        // Grit flying past, from upwind.
        let sand = self.reg.id_or_air("sand");
        let (layer, _) = self.block_look(sand);
        let n = (k * 22.0) as usize;
        for _ in 0..n {
            let r = &mut self.surv.rng;
            let off = Vec3::new(r.f32() * 24.0 - 12.0, r.f32() * 8.0 - 1.5, r.f32() * 24.0 - 12.0) - wind.normalize() * 6.0;
            let v = wind * (1.6 + r.f32()) + Vec3::new(0.0, r.f32() - 0.6, 0.0);
            self.particles.dust(self.body.pos + off, v, layer, [0.95, 0.82, 0.58], 0.035, 1.3);
        }
        // The wind leans on you out in the open.
        if open && !self.body.flying {
            self.body.vel += wind * 0.25 * k * dt;
        }
        self.t5.tr.ev.spawn_t -= dt;
        if self.t5.tr.ev.spawn_t <= 0.0 && !self.mobs.peaceful {
            self.t5.tr.ev.spawn_t = 12.0;
            if self.count_tr(&["dune_wraith"]) < 2 && self.surv.rng.f32() < 0.6 {
                self.spawn_around("dune_wraith", true);
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Lantern night

    fn update_lanterns(&mut self, dt: f32, open_night: bool) {
        let ev = &mut self.t5.tr.ev;
        if ev.lantern_night && open_night && ev.lanterns.len() < 48 && self.surv.rng.f32() < dt * 6.0 {
            let r = &mut self.surv.rng;
            let a = r.f32() * std::f32::consts::TAU;
            let d = 6.0 + r.f32() * 30.0;
            let x = self.body.pos.x + a.cos() * d;
            let z = self.body.pos.z + a.sin() * d;
            let y = self.world.surface_y(x.floor() as i32, z.floor() as i32).map_or(self.body.pos.y, |y| y as f32 + 1.5);
            ev.lanterns.push((Vec3::new(x, y, z), 30.0 + r.f32() * 20.0));
        }
        if ev.lanterns.is_empty() {
            return;
        }
        let glow = self.glow_layer();
        let t = self.now;
        let mut lanterns = std::mem::take(&mut self.t5.tr.ev.lanterns);
        lanterns.retain_mut(|(pos, left)| {
            *left -= dt;
            let sway = Vec3::new((t * 0.7 + pos.y * 0.3).sin() * 0.25, 1.1, (t * 0.5 + pos.x).cos() * 0.2);
            *pos += sway * dt;
            let fade = (*left / 6.0).min(1.0);
            self.particles.glow(*pos, Vec3::ZERO, glow, [1.8 * fade, 0.85 * fade, 0.3 * fade], 0.32, 0.08);
            self.particles.glow(*pos + Vec3::Y * 0.25, Vec3::ZERO, glow, [0.9 * fade, 0.35 * fade, 0.1 * fade], 0.5, 0.08);
            *left > 0.0
        });
        self.t5.tr.ev.lanterns = lanterns;
    }

    // ---------------------------------------------------------------------------------------
    // Solar eclipse

    pub(crate) fn start_eclipse(&mut self) {
        self.t5.tr.ev.eclipse = true;
        self.weather.flash = 0.3;
        self.sfx(Sfx::Thunder(0.6));
        self.toast("Um eclipse solar começou!", 5.0);
    }

    /// Right click with a solar tablet: the sun goes dark (by day).
    pub(crate) fn use_solar_tablet(&mut self) -> bool {
        if self.is_night() || self.dim != Dim::Overworld {
            self.toast("A tábua só escurece o sol de dia, na superfície", 3.0);
            return true;
        }
        if self.t5.tr.ev.eclipse {
            self.toast("O sol já está escuro", 2.5);
            return true;
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.start_eclipse();
        true
    }

    fn update_eclipse(&mut self, dt: f32, over: bool) {
        if !self.t5.tr.ev.eclipse || !over || self.mobs.peaceful {
            return;
        }
        self.t5.tr.ev.spawn_t -= dt;
        if self.t5.tr.ev.spawn_t > 0.0 {
            return;
        }
        self.t5.tr.ev.spawn_t = 2.5;
        if self.count_tr(&ECLIPSE_MOBS) >= 8 {
            return;
        }
        let roll = self.surv.rng.f32();
        if roll < 0.05 && self.count_tr(&["eclipse_moth"]) == 0 {
            if self.spawn_around("eclipse_moth", true) {
                self.toast("Uma Mariposa do Eclipse se aproxima!", 3.0);
            }
        } else if roll < 0.5 {
            self.spawn_around("reaper", true);
        } else {
            self.spawn_around("shade_brute", false);
        }
    }

    /// The light mobs see: dark all day during an eclipse, so the undead don't burn.
    pub(crate) fn tr_daylight(&self, d: f32) -> f32 {
        if self.t5.tr.ev.eclipse && self.dim == Dim::Overworld {
            d.min(0.08)
        } else {
            d
        }
    }

    /// The sky of the events: the eclipse darkens it, a sandstorm turns it ochre.
    pub(crate) fn tr_sky(&self, top: &mut [f32; 3], horizon: &mut [f32; 3], daylight: &mut f32) {
        if self.dim != Dim::Overworld {
            return;
        }
        self.land_sky(top, horizon);
        let ev = &self.t5.tr.ev;
        let mix = |c: &mut [f32; 3], to: [f32; 3], k: f32| {
            for i in 0..3 {
                c[i] += (to[i] - c[i]) * k;
            }
        };
        if ev.eclipse {
            mix(top, [0.03, 0.02, 0.025], 0.9);
            mix(horizon, [0.32, 0.12, 0.04], 0.85);
            *daylight = daylight.min(0.3);
        }
        if ev.storm_k > 0.02 {
            let k = ev.storm_k * 0.75;
            mix(top, [0.72, 0.58, 0.36], k);
            mix(horizon, [0.86, 0.7, 0.45], k);
            *daylight *= 1.0 - 0.25 * ev.storm_k;
        }
    }

    /// The events' HUD: the sandstorm's haze and the jelly rain's bar.
    pub(crate) fn draw_tr_hud(&mut self, w: f32, h: f32) {
        let k = self.t5.tr.ev.storm_k;
        if k > 0.02 {
            self.ui.rect(0.0, 0.0, w, h, [200, 160, 100, (k * 80.0) as u8]);
        }
        if let Some((title, k)) = self.invasion_hud() {
            let s = self.ui.scale;
            let bw = 160.0 * s;
            let (x, y) = ((w - bw) / 2.0, 44.0 * s);
            let tw = self.ui.text_width(&title, s);
            self.ui.text((w - tw) / 2.0, y - 11.0 * s, &title, [240, 210, 160, 255]);
            self.ui.rect(x - s, y - s, bw + 2.0 * s, 6.0 * s, [0, 0, 0, 200]);
            self.ui.rect(x, y, bw, 4.0 * s, [60, 40, 30, 255]);
            self.ui.rect(x, y, bw * k.min(1.0), 4.0 * s, [230, 150, 60, 255]);
        }
        if let Some(line) = self.pillar_status() {
            let s = self.ui.scale;
            let t = format!("Pilares: {line}");
            let tw = self.ui.text_width(&t, s);
            self.ui.text((w - tw) / 2.0, 58.0 * s, &t, [200, 210, 255, 255]);
        }
        let Some((killed, need)) = self.t5.tr.ev.rain.as_ref().map(|r| (r.killed, r.need)) else { return };
        let s = self.ui.scale;
        let boss_up = self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].boss && m.body.pos.distance(self.body.pos) < 64.0);
        let mut y = if boss_up { 38.0 * s } else { 14.0 * s };
        if self.t5.hard.invasion().is_some() {
            y += 24.0 * s;
        }
        let bw = 140.0 * s;
        let x = (w - bw) / 2.0;
        let t = "Chuva de Geleias";
        let tw = self.ui.text_width(t, s);
        self.ui.text((w - tw) / 2.0, y - 10.0 * s, t, [150, 200, 240, 255]);
        self.ui.rect(x - s, y - s, bw + 2.0 * s, 6.0 * s, [0, 0, 0, 200]);
        self.ui.rect(x, y, bw, 4.0 * s, [30, 40, 60, 255]);
        self.ui.rect(x, y, bw * (killed as f32 / need as f32).min(1.0), 4.0 * s, [80, 150, 230, 255]);
    }

    fn count_tr(&self, names: &[&str]) -> usize {
        self.mobs.list.iter().filter(|m| m.alive() && names.contains(&self.mobs.reg.defs[m.def].name.as_str())).count()
    }

    /// Test scenes of the events (MC_TR=meteor, stars, jelly_rain, sandstorm, lanterns, eclipse).
    pub(crate) fn test_tr_event(&mut self, scene: &str) -> bool {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        match scene {
            "meteor" => {
                self.time_of_day = 18000.0;
                self.t5.tr.ev.night_was = true;
                let at = self.body.pos + flat * 11.0;
                let c = Vec3::new(at.x, ground(self, at) - 1.0, at.z).floor().as_ivec3();
                self.make_crater(c);
                for k in 0..3 {
                    let p = c.as_vec3() + side * (k as f32 - 1.0) * 3.0 + Vec3::Y * (4.0 + k as f32);
                    self.mobs.spawn("meteor_head", p);
                }
                // One more coming down far behind it.
                let to = c.as_vec3() + flat * 50.0 + side * 20.0;
                self.t5.tr.ev.falling = Some(Falling { from: to + Vec3::new(-30.0, 70.0, 0.0), to, t: 0.0 });
            }
            "stars" => {
                self.time_of_day = 18000.0;
                self.t5.tr.ev.night_was = true;
                for k in 0..4 {
                    let p = self.body.pos + flat * (14.0 + k as f32 * 5.0) + side * (k as f32 * 4.0 - 6.0) + Vec3::Y * (14.0 + k as f32 * 6.0);
                    self.t5.tr.ev.stars.push((p, Vec3::new(1.5, -2.0, 0.5)));
                }
                if let Some(id) = self.items.id("fallen_star") {
                    for k in 0..3 {
                        let p = self.body.pos + flat * 4.0 + side * (k as f32 - 1.0) * 1.5;
                        self.spawn_drop(Vec3::new(p.x, ground(self, p) + 0.3, p.z), ItemStack::new(id, 1), Vec3::ZERO);
                    }
                }
            }
            "jelly_rain" => {
                self.time_of_day = 6000.0;
                self.start_jelly_rain();
                if let Some(r) = self.t5.tr.ev.rain.as_mut() {
                    r.killed = 23;
                }
                for k in 0..6 {
                    let p = self.body.pos + flat * (6.0 + (k % 3) as f32 * 3.0) + side * (k as f32 - 2.5) * 2.0;
                    let y = ground(self, p) + if k % 2 == 0 { 0.0 } else { 5.0 + k as f32 };
                    self.mobs.spawn(JELLY_RAIN[k % JELLY_RAIN.len()], Vec3::new(p.x, y, p.z));
                }
            }
            "sandstorm" => {
                self.time_of_day = 6000.0;
                let ev = &mut self.t5.tr.ev;
                ev.storm = 200.0;
                ev.storm_force = true;
                ev.storm_k = 1.0;
                ev.wind = side * 7.0;
                for k in 0..2 {
                    let p = self.body.pos + flat * (8.0 + k as f32 * 3.0) + side * (k as f32 * 4.0 - 2.0);
                    self.mobs.spawn("dune_wraith", Vec3::new(p.x, ground(self, p) + 0.5, p.z));
                }
            }
            "lanterns" => {
                self.time_of_day = 17000.0;
                self.t5.tr.ev.night_was = true;
                self.t5.tr.ev.lantern_night = true;
                for k in 0..44 {
                    let (a, b, c) = (self.surv.rng.f32(), self.surv.rng.f32(), self.surv.rng.f32());
                    let p = self.body.pos + flat * (6.0 + a * 30.0) + side * (b * 40.0 - 20.0);
                    let y = ground(self, p) + 1.0 + c * (4.0 + k as f32 * 0.6);
                    self.t5.tr.ev.lanterns.push((Vec3::new(p.x, y, p.z), 40.0));
                }
            }
            "eclipse" => {
                self.time_of_day = 6000.0;
                self.t5.tr.ev.eclipse = true;
                let names = ["reaper", "shade_brute", "eclipse_moth", "shade_brute", "reaper"];
                for (k, n) in names.iter().enumerate() {
                    let d = if *n == "eclipse_moth" { 13.0 } else { 7.0 + (k % 2) as f32 * 2.0 };
                    let p = self.body.pos + flat * d + side * (k as f32 - 2.0) * 3.0;
                    let y = ground(self, p) + if *n == "shade_brute" { 0.0 } else { 2.5 };
                    self.mobs.spawn(n, Vec3::new(p.x, y, p.z));
                }
            }
            _ => return false,
        }
        let me = self.body.eye();
        for m in self.mobs.list.iter_mut() {
            let look = me - m.body.center();
            m.yaw = look.x.atan2(look.z);
        }
        true
    }
}
