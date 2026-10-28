//! Lote 7 of the Terraria side (plan/terraria-completo.md): the last bosses, our own creatures.
//! - The Jelly Queen (Rainha Geleia), called with a gelatin crystal in the Prism: hops and
//!   throws crystals, then takes wing and rains them, diving at you.
//! - The Scale Idol (Ídolo Escamado), woken at the altar of the Scale Temple under the jungle:
//!   stomps after you, punches up close and shoots sun beams from its eyes; at the end its head
//!   tears loose and flies.
//! - The Tide Duke (Duque das Marés), called with a tide bait by the sea: hovers spitting
//!   bubbles, then charges; angry, it charges more and faster.
//! - The Prism Empress (Imperatriz Prismática), called with a prism lamp in the Prism at night:
//!   rainbow spirals, lances falling from the sky, dashes; by day she is deadly.
//! - The Lunar Prophet (Profeta Lunar): blinks about casting fire, ice and lightning. When he
//!   falls, four pillars rise around you, each shielded until you beat forty of its folk. The
//!   last pillar down calls the Celestial Herald (boss_ai.rs).
//! - Dungeon spirits roam the deep after the Vine Queen and drop ectoplasm.

use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};
use voxel::worldgen::{Biome, Dim};

use crate::audio::Sfx;
use crate::Game;

pub fn own_ai(name: &str) -> bool {
    matches!(name, "jelly_queen" | "scale_idol" | "tide_duke" | "prism_empress" | "lunar_prophet") || name.starts_with("pillar_")
}

/// (pillar kind, its folk, flies?)
const PILLARS: &[(&str, &str, bool)] = &[("solar", "solar_warrior", false), ("vortex", "vortex_hornet", true), ("nebula", "nebula_eye", true), ("stardust", "star_cell", false)];
const SHIELD: u32 = 40;

#[derive(Clone, Serialize, Deserialize)]
pub struct Pillar {
    pub kind: String,
    pub pos: [f32; 3],
    pub shield: u32,
    pub dead: bool,
}

#[derive(Default)]
pub struct Endgame {
    vel: Vec3,
    t: f32,
    mode: u8,
    left: u8,
    shot_t: f32,
    phase2: bool,
    angle: f32,
    told_day: bool,
    /// The four pillars after the Prophet falls (saved with the world).
    pub(crate) pillars: Vec<Pillar>,
    spawn_t: f32,
    herald_in: Option<f32>,
    spirit_t: f32,
}

impl Game {
    /// Summon items of this file (the idol core works at the temple altar instead).
    pub(crate) fn endgame_use(&mut self, item: &str) -> Option<bool> {
        let boss = match item {
            "gelatin_crystal" => "jelly_queen",
            "tide_bait" => "tide_duke",
            "prism_lamp" => "prism_empress",
            "lunar_tablet" => "lunar_prophet",
            "idol_core" => {
                self.toast("Use o núcleo no Altar Escamado, dentro do Templo Escamado (selva, no subsolo)", 4.0);
                return Some(true);
            }
            _ => return None,
        };
        let over = self.dim == Dim::Overworld;
        let prism = self.t5.tr.lands.here == Some(crate::tr_world::Land::Prism);
        let biome = self.world.gen.base_biome_at(self.body.pos.x.floor() as i32, self.body.pos.z.floor() as i32);
        let (ok, why) = match boss {
            "jelly_queen" => (over && self.t5.hard.on && prism, "Só no Prisma, no modo difícil"),
            "tide_duke" => (over && matches!(biome, Biome::Ocean | Biome::Beach), "Só no mar ou na praia"),
            "prism_empress" => (over && prism && self.is_night(), "Só no Prisma, à noite"),
            _ => (over && self.boss.defeated.iter().any(|b| b == "scale_idol") && self.body.pos.y > 30.0, "Só na superfície, depois do Ídolo Escamado"),
        };
        if !ok {
            self.toast(why, 3.0);
            return Some(true);
        }
        Some(self.wake_endgame(boss, true))
    }

    /// Right click on the temple altar.
    pub(crate) fn use_scale_altar(&mut self, at: IVec3) -> bool {
        let held = self.held_stack().map(|s| self.items.get(s.item).name.clone()).unwrap_or_default();
        if held != "idol_core" {
            self.toast("O altar espera um Núcleo do Ídolo", 2.5);
            return true;
        }
        if !self.boss.defeated.iter().any(|b| b == "vine_queen") {
            self.toast("O altar não responde antes da Rainha das Vinhas cair", 3.0);
            return true;
        }
        let _ = at;
        self.wake_endgame("scale_idol", true)
    }

    fn wake_endgame(&mut self, boss: &str, use_item: bool) -> bool {
        if self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].boss) {
            self.toast("Já há um chefe acordado", 3.0);
            return true;
        }
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let mut at = self.body.pos + flat * 12.0;
        at.y = match boss {
            "scale_idol" => self.body.pos.y,
            "jelly_queen" => self.world.surface_y(at.x.floor() as i32, at.z.floor() as i32).map_or(self.body.pos.y, |y| y as f32 + 12.0),
            _ => self.body.pos.y + 6.0,
        };
        if !self.mobs.spawn(boss, at) {
            return false;
        }
        if let Some(m) = self.mobs.list.last_mut() {
            m.aggro = true;
        }
        if use_item && self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        let eg = &mut self.t5.tr.end;
        eg.vel = Vec3::ZERO;
        eg.t = 0.0;
        eg.mode = 0;
        eg.left = 0;
        eg.shot_t = 1.0;
        eg.phase2 = false;
        eg.told_day = false;
        self.boss.called_help = false;
        let text = match boss {
            "jelly_queen" => "A Rainha Geleia desce, brilhando!",
            "scale_idol" => "O Ídolo Escamado desperta no templo!",
            "tide_duke" => "O Duque das Marés salta das ondas!",
            "prism_empress" => "A Imperatriz Prismática chega dançando na luz!",
            _ => "O Profeta Lunar começa seu ritual!",
        };
        self.weather.flash = 0.6;
        self.sfx(Sfx::Thunder(0.6));
        self.toast(text, 4.0);
        true
    }

    pub(crate) fn update_endgame(&mut self, dt: f32) {
        if dt == 0.0 {
            return;
        }
        if let Some(i) = self.mobs.list.iter().position(|m| m.alive() && own_ai(&self.mobs.reg.defs[m.def].name) && !self.mobs.reg.defs[m.def].name.starts_with("pillar_")) {
            let name = self.mobs.reg.defs[self.mobs.list[i].def].name.clone();
            self.t5.tr.end.t += dt;
            self.t5.tr.end.shot_t -= dt;
            match name.as_str() {
                "jelly_queen" => self.fight_jelly_queen(i, dt),
                "scale_idol" => self.fight_idol(i, dt),
                "tide_duke" => self.fight_duke(i, dt),
                "prism_empress" => self.fight_empress(i, dt),
                _ => self.fight_prophet(i, dt),
            }
        }
        self.update_pillars(dt);
        self.dungeon_spirits(dt);
    }

    fn hp_of(&self, i: usize) -> f32 {
        let m = &self.mobs.list[i];
        m.health / m.max_health.max(1.0)
    }

    /// Fly the boss toward `spot` (smoothed), facing the player.
    fn glide(&mut self, i: usize, spot: Vec3, k: f32, dt: f32) {
        let pos = self.mobs.list[i].body.pos;
        let eg = &mut self.t5.tr.end;
        eg.vel += ((spot - pos) * k - eg.vel) * (3.0 * dt).min(1.0);
        self.move_boss(i, dt);
    }

    fn move_boss(&mut self, i: usize, dt: f32) {
        let vel = self.t5.tr.end.vel;
        let me = self.body.eye();
        let m = &mut self.mobs.list[i];
        m.aggro = true;
        m.body.pos += vel * dt;
        m.body.vel = Vec3::ZERO;
        m.body.flying = true;
        let look = me - m.body.pos;
        m.yaw = look.x.atan2(look.z);
    }

    fn fan(&mut self, from: Vec3, n: usize, spread: f32, speed: f32, school: u8, dmg: f32) {
        let to = (self.body.eye() - from).normalize_or_zero();
        let side = to.cross(Vec3::Y).normalize_or_zero();
        for k in 0..n {
            let a = (k as f32 - (n - 1) as f32 / 2.0) * spread;
            self.enemy_bolt(from + to, (to * a.cos() + side * a.sin()).normalize_or_zero() * speed, school, dmg);
        }
        self.sfx(Sfx::Cast(school));
    }

    fn phase_change(&mut self, i: usize, text: &str, tint: [f32; 3]) -> bool {
        if self.t5.tr.end.phase2 || self.hp_of(i) > 0.5 {
            return self.t5.tr.end.phase2;
        }
        self.t5.tr.end.phase2 = true;
        self.t5.tr.end.mode = 0;
        self.t5.tr.end.t = 0.0;
        let at = self.mobs.list[i].body.pos + Vec3::Y * 2.0;
        self.puff(at, 50, 9.0, tint);
        self.toast(text, 3.0);
        self.sfx(Sfx::Thunder(0.5));
        true
    }

    fn fight_jelly_queen(&mut self, i: usize, dt: f32) {
        let me = self.body.pos;
        let attack = self.mobs.reg.defs[self.mobs.list[i].def].attack;
        let p2 = self.phase_change(i, "A Rainha Geleia abre as asas!", [1.6, 1.0, 1.7]);
        let pos = self.mobs.list[i].body.pos;
        if !p2 {
            let on_ground = self.mobs.list[i].body.on_ground;
            let eg = &mut self.t5.tr.end;
            if on_ground && eg.t > 1.1 {
                eg.t = 0.0;
                let dir = Vec3::new(me.x - pos.x, 0.0, me.z - pos.z).normalize_or_zero();
                let m = &mut self.mobs.list[i];
                m.body.vel = dir * 8.0 + Vec3::Y * 12.0;
                m.aggro = true;
                // Crystals fly up as it leaves the ground.
                self.fan(pos + Vec3::Y * 2.5, 3, 0.3, 14.0, 3, attack * 0.6);
                if self.surv.rng.f32() < 0.35 {
                    self.mobs.spawn("jelly_spiked", pos + Vec3::Y * 2.0);
                }
            }
        } else {
            let eg = &mut self.t5.tr.end;
            if eg.mode == 0 {
                let a = eg.t * 0.8;
                let spot = me + Vec3::new(a.cos() * 6.0, 7.0, a.sin() * 6.0);
                let fire = eg.shot_t <= 0.0;
                if fire {
                    eg.shot_t = 1.0;
                }
                let dive = eg.t > 6.0;
                if dive {
                    eg.mode = 1;
                    eg.t = 0.0;
                    eg.vel = (self.body.eye() - pos).normalize_or_zero() * 24.0;
                }
                if !dive {
                    self.glide(i, spot, 2.0, dt);
                }
                if fire {
                    self.fan(pos + Vec3::Y * 1.5, 5, 0.18, 18.0, 3, attack * 0.7);
                }
            } else {
                if eg.t > 0.8 {
                    eg.mode = 0;
                    eg.t = 0.0;
                }
                self.move_boss(i, dt);
            }
        }
        self.tr_contact(pos + Vec3::Y * 1.2, 2.0, attack);
    }

    fn fight_idol(&mut self, i: usize, dt: f32) {
        let me = self.body.eye();
        let attack = self.mobs.reg.defs[self.mobs.list[i].def].attack;
        let p2 = self.phase_change(i, "A cabeça do Ídolo se solta e voa!", [1.8, 0.9, 0.3]);
        let pos = self.mobs.list[i].body.pos;
        let head = pos + Vec3::Y * 3.6;
        let dist = Vec3::new(me.x - pos.x, 0.0, me.z - pos.z).length();
        if p2 {
            let a = self.t5.tr.end.t * 0.6;
            self.glide(i, me + Vec3::new(a.cos() * 8.0, 4.0, a.sin() * 8.0), 1.5, dt);
        } else {
            let on_ground = self.mobs.list[i].body.on_ground;
            if on_ground && self.t5.tr.end.t > 1.6 {
                self.t5.tr.end.t = 0.0;
                let dir = Vec3::new(me.x - pos.x, 0.0, me.z - pos.z).normalize_or_zero();
                let m = &mut self.mobs.list[i];
                m.body.vel = dir * 5.0 + Vec3::Y * 9.0;
                m.aggro = true;
                m.yaw = dir.x.atan2(dir.z);
            }
        }
        if self.t5.tr.end.shot_t <= 0.0 {
            self.t5.tr.end.shot_t = if p2 { 0.6 } else { 1.4 };
            if dist < 6.0 && !p2 {
                // A punch: the ground shakes and the fist lands.
                self.puff(pos + Vec3::Y, 30, 7.0, [1.4, 0.9, 0.5]);
                self.sfx(Sfx::Explode);
                self.t5.tr.fight.reset_contact();
                self.tr_contact(me, 1.0, attack * 1.4);
            } else {
                self.fan(head, 2, 0.08, 30.0, 0, attack * 0.8);
            }
        }
        self.tr_contact(pos + Vec3::Y * 2.0, 2.2, attack);
    }

    fn fight_duke(&mut self, i: usize, dt: f32) {
        let me = self.body.eye();
        let attack = self.mobs.reg.defs[self.mobs.list[i].def].attack;
        let p2 = self.phase_change(i, "O Duque das Marés se enfurece!", [0.4, 1.2, 1.6]);
        let pos = self.mobs.list[i].body.pos + Vec3::Y;
        let dash = if p2 { 32.0 } else { 26.0 };
        let eg = &mut self.t5.tr.end;
        match eg.mode {
            0 => {
                let a = eg.t * 0.7;
                let spot = me + Vec3::new(a.cos() * 10.0, 2.0, a.sin() * 10.0);
                let fire = eg.shot_t <= 0.0;
                if fire {
                    eg.shot_t = 0.5;
                }
                if eg.t > if p2 { 1.6 } else { 2.6 } {
                    eg.mode = 1;
                    eg.t = 0.0;
                    eg.left = if p2 { 4 } else { 3 };
                    eg.vel = (me - pos).normalize_or_zero() * dash;
                    self.sfx(Sfx::Splash(0.8));
                } else {
                    self.glide(i, spot, 2.0, dt);
                }
                if fire {
                    // Slow bubbles.
                    self.fan(pos, 3, 0.35, 8.0, 1, attack * 0.6);
                }
            }
            _ => {
                if eg.t > 0.7 {
                    eg.t = 0.0;
                    eg.left = eg.left.saturating_sub(1);
                    if eg.left == 0 {
                        eg.mode = 0;
                    } else {
                        eg.vel = (me - pos).normalize_or_zero() * dash;
                    }
                } else if eg.t > 0.5 {
                    eg.vel *= 1.0 - (6.0 * dt).min(1.0);
                }
                self.move_boss(i, dt);
            }
        }
        self.tr_contact(pos, 2.0, attack * if p2 { 1.3 } else { 1.0 });
    }

    fn fight_empress(&mut self, i: usize, dt: f32) {
        let me = self.body.eye();
        let day = !self.is_night();
        if day && !self.t5.tr.end.told_day {
            self.t5.tr.end.told_day = true;
            self.toast("A luz do dia deixa a Imperatriz furiosa!", 3.0);
        }
        let attack = self.mobs.reg.defs[self.mobs.list[i].def].attack * if day { 3.0 } else { 1.0 };
        let p2 = self.phase_change(i, "A Imperatriz dança mais rápido!", [1.6, 1.2, 1.8]);
        let pos = self.mobs.list[i].body.pos + Vec3::Y * 1.5;
        let speed = if p2 { 1.4 } else { 1.0 };
        let eg = &mut self.t5.tr.end;
        let mode = eg.mode;
        let t = eg.t;
        if mode < 2 && t > 5.0 / speed {
            eg.mode = (eg.mode + 1) % 3;
            eg.t = 0.0;
            eg.left = 3;
            if eg.mode == 2 {
                eg.vel = (me - pos).normalize_or_zero() * 30.0;
            }
        }
        match mode {
            0 => {
                // The rainbow spiral.
                let a = t * 0.5;
                let spot = me + Vec3::new(a.cos() * 9.0, 4.0, a.sin() * 9.0);
                self.glide(i, spot, 2.0, dt);
                if self.t5.tr.end.shot_t <= 0.0 {
                    self.t5.tr.end.shot_t = 0.09 / speed;
                    self.t5.tr.end.angle += 0.5;
                    let b = self.t5.tr.end.angle;
                    let dir = Vec3::new(b.cos(), -0.15, b.sin());
                    let school = (b * 2.0) as u8 % 6;
                    self.enemy_bolt(pos + dir, dir * 13.0, school, attack * 0.5);
                }
            }
            1 => {
                // Lances fall from the sky around you.
                let spot = me + Vec3::new(0.0, 8.0, 0.0);
                self.glide(i, spot, 1.5, dt);
                if self.t5.tr.end.shot_t <= 0.0 {
                    self.t5.tr.end.shot_t = 0.14 / speed;
                    let off = Vec3::new(self.surv.rng.f32() - 0.5, 0.0, self.surv.rng.f32() - 0.5) * 16.0;
                    self.enemy_bolt(me + off + Vec3::Y * 14.0, Vec3::NEG_Y * 22.0, 2, attack * 0.7);
                }
            }
            _ => {
                let eg = &mut self.t5.tr.end;
                if eg.t > 0.65 {
                    eg.t = 0.0;
                    eg.left = eg.left.saturating_sub(1);
                    if eg.left == 0 {
                        eg.mode = 0;
                    } else {
                        eg.vel = (me - pos).normalize_or_zero() * 30.0;
                    }
                }
                self.move_boss(i, dt);
            }
        }
        if self.surv.rng.f32() < 0.3 {
            let glow = self.glow_layer();
            let c = [[1.8, 0.5, 0.5], [1.8, 1.4, 0.4], [0.5, 1.6, 0.5], [0.5, 1.0, 1.8], [1.4, 0.5, 1.6]][(self.now * 7.0) as usize % 5];
            self.particles.glow(pos + Vec3::Y * 0.5, Vec3::new(0.0, -0.5, 0.0), glow, c, 0.12, 0.8);
        }
        self.tr_contact(pos, 1.4, attack);
    }

    fn fight_prophet(&mut self, i: usize, dt: f32) {
        let me = self.body.eye();
        let attack = self.mobs.reg.defs[self.mobs.list[i].def].attack;
        let pos = self.mobs.list[i].body.pos + Vec3::Y * 1.5;
        let eg = &mut self.t5.tr.end;
        eg.vel = Vec3::ZERO;
        if eg.t > 4.0 {
            eg.t = 0.0;
            eg.mode = (eg.mode + 1) % 3;
            // Blink somewhere around you.
            let a = self.surv.rng.f32() * std::f32::consts::TAU;
            let to = self.body.pos + Vec3::new(a.cos() * 9.0, 2.5, a.sin() * 9.0);
            self.puff(pos, 25, 4.0, [0.5, 0.8, 1.8]);
            self.mobs.list[i].body.pos = to;
            self.puff(to + Vec3::Y, 25, 4.0, [0.5, 0.8, 1.8]);
            self.sfx(Sfx::Cast(3));
        }
        self.move_boss(i, dt);
        if self.t5.tr.end.shot_t <= 0.0 {
            match self.t5.tr.end.mode {
                0 => {
                    self.t5.tr.end.shot_t = 1.2;
                    self.fan(pos, 5, 0.2, 16.0, 0, attack * 0.8);
                }
                1 => {
                    self.t5.tr.end.shot_t = 1.6;
                    self.fan(pos, 7, 0.14, 12.0, 1, attack * 0.6);
                }
                _ => {
                    self.t5.tr.end.shot_t = 1.0;
                    self.fan(pos, 1, 0.0, 9.0, 5, attack * 1.4);
                }
            }
        }
        let _ = me;
    }

    /// A boss of this file fell (tr.rs passes every one): the Prophet raises the pillars.
    pub(crate) fn endgame_boss_defeated(&mut self, name: &str) {
        if name != "lunar_prophet" {
            return;
        }
        let me = self.body.pos;
        self.t5.tr.end.pillars.clear();
        for (k, &(kind, _, _)) in PILLARS.iter().enumerate() {
            let a = (k as f32 + 0.5) / 4.0 * std::f32::consts::TAU;
            let x = me.x + a.cos() * 70.0;
            let z = me.z + a.sin() * 70.0;
            let y = self.world.surface_y(x.floor() as i32, z.floor() as i32).map_or(me.y, |y| y as f32 + 1.0);
            self.t5.tr.end.pillars.push(Pillar { kind: kind.to_string(), pos: [x, y, z], shield: SHIELD, dead: false });
        }
        self.toast("Quatro pilares celestes se ergueram ao seu redor!", 5.0);
        self.weather.flash = 0.8;
        self.sfx(Sfx::Thunder(0.8));
    }

    fn update_pillars(&mut self, dt: f32) {
        if self.t5.tr.end.pillars.is_empty() || self.dim != Dim::Overworld {
            return;
        }
        if let Some(t) = self.t5.tr.end.herald_in.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.t5.tr.end.herald_in = None;
                self.t5.tr.end.pillars.clear();
                let at = self.body.pos + Vec3::Y * 10.0;
                if self.mobs.spawn("celestial_herald", at) {
                    if let Some(m) = self.mobs.list.last_mut() {
                        m.aggro = true;
                    }
                    self.t5.fight = Default::default();
                    self.toast("O Arauto Celeste desce das estrelas!", 5.0);
                    self.weather.flash = 1.0;
                    self.sfx(Sfx::Thunder(1.0));
                }
                return;
            }
        }
        self.t5.tr.end.spawn_t -= dt;
        let spawn = self.t5.tr.end.spawn_t <= 0.0;
        if spawn {
            self.t5.tr.end.spawn_t = 2.0;
        }
        for k in 0..self.t5.tr.end.pillars.len() {
            let p = self.t5.tr.end.pillars[k].clone();
            if p.dead {
                continue;
            }
            let at = Vec3::from(p.pos);
            if at.distance(self.body.pos) > 140.0 {
                continue;
            }
            let name = format!("pillar_{}", p.kind);
            let idx = self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].name == name);
            let idx = match idx {
                Some(i) => i,
                None => {
                    if !self.mobs.spawn(&name, at) {
                        continue;
                    }
                    self.mobs.list.len() - 1
                }
            };
            let m = &mut self.mobs.list[idx];
            m.body.pos = at;
            m.body.vel = Vec3::ZERO;
            m.body.flying = true;
            m.aggro = false;
            if p.shield > 0 {
                m.health = m.max_health;
            }
            // Its folk come while the shield stands and you are near.
            let &(_, folk, flies) = PILLARS.iter().find(|q| q.0 == p.kind).unwrap();
            if spawn && p.shield > 0 && at.distance(self.body.pos) < 80.0 {
                let near = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == folk && m.body.pos.distance(at) < 60.0).count();
                if near < 6 {
                    let a = self.surv.rng.f32() * std::f32::consts::TAU;
                    let s = at + Vec3::new(a.cos() * 8.0, 0.0, a.sin() * 8.0);
                    let y = self.world.surface_y(s.x.floor() as i32, s.z.floor() as i32).map_or(at.y, |y| y as f32 + 1.0) + if flies { 5.0 } else { 0.0 };
                    if self.mobs.spawn(folk, Vec3::new(s.x, y, s.z)) {
                        if let Some(f) = self.mobs.list.last_mut() {
                            f.aggro = true;
                        }
                    }
                }
            }
            if self.surv.rng.f32() < 0.2 {
                let glow = self.glow_layer();
                let tint = match p.kind.as_str() {
                    "solar" => [1.8, 0.8, 0.3],
                    "vortex" => [0.4, 1.6, 1.3],
                    "nebula" => [1.5, 0.5, 1.7],
                    _ => [0.6, 1.2, 1.9],
                };
                let up = Vec3::new(self.surv.rng.f32() - 0.5, 0.0, self.surv.rng.f32() - 0.5) * 3.0;
                self.particles.glow(at + up + Vec3::Y * (self.surv.rng.f32() * 13.0), Vec3::Y * 2.0, glow, tint, 0.18, 1.2);
            }
        }
    }

    /// Deaths: a pillar's folk wear down its shield, a pillar falls.
    pub(crate) fn endgame_deaths(&mut self, deaths: &[(usize, Vec3)]) {
        if self.t5.tr.end.pillars.is_empty() {
            return;
        }
        for &(def, pos) in deaths {
            let name = self.mobs.reg.defs[def].name.clone();
            if let Some(kind) = name.strip_prefix("pillar_") {
                if let Some(p) = self.t5.tr.end.pillars.iter_mut().find(|p| p.kind == kind) {
                    p.dead = true;
                }
                if self.t5.tr.end.pillars.iter().all(|p| p.dead) {
                    self.t5.tr.end.herald_in = Some(20.0);
                    self.toast("O céu escurece... algo enorme se aproxima!", 5.0);
                }
                continue;
            }
            let Some(&(kind, _, _)) = PILLARS.iter().find(|q| q.1 == name) else { continue };
            let mut fell = None;
            if let Some(p) = self.t5.tr.end.pillars.iter_mut().find(|p| p.kind == kind && !p.dead && Vec3::from(p.pos).distance(pos) < 90.0) {
                if p.shield > 0 {
                    p.shield -= 1;
                    if p.shield == 0 {
                        fell = Some(kind);
                    }
                }
            }
            if let Some(k) = fell {
                let dn = match k {
                    "solar" => "Solar",
                    "vortex" => "do Vórtice",
                    "nebula" => "da Nebulosa",
                    _ => "de Pó Estelar",
                };
                self.toast(format!("O escudo do Pilar {dn} caiu! Ataque o pilar."), 4.0);
                self.sfx(Sfx::Thunder(0.4));
            }
        }
    }

    /// How many folk are still needed for each pillar's shield (for the HUD line).
    pub(crate) fn pillar_status(&self) -> Option<String> {
        let ps = &self.t5.tr.end.pillars;
        if ps.is_empty() {
            return None;
        }
        let parts: Vec<String> = ps
            .iter()
            .map(|p| {
                let n = match p.kind.as_str() {
                    "solar" => "Solar",
                    "vortex" => "Vórtice",
                    "nebula" => "Nebulosa",
                    _ => "Pó Estelar",
                };
                if p.dead {
                    format!("{n}: caiu")
                } else if p.shield == 0 {
                    format!("{n}: sem escudo")
                } else {
                    format!("{n}: {}", p.shield)
                }
            })
            .collect();
        Some(parts.join("  "))
    }

    fn dungeon_spirits(&mut self, dt: f32) {
        self.t5.tr.end.spirit_t -= dt;
        if self.t5.tr.end.spirit_t > 0.0 {
            return;
        }
        self.t5.tr.end.spirit_t = 10.0;
        if self.dim != Dim::Overworld || self.body.pos.y > -40.0 || !self.t5.hard.on || self.mobs.peaceful || !self.survival {
            return;
        }
        if !self.boss.defeated.iter().any(|b| b == "vine_queen") || self.surv.rng.f32() > 0.4 {
            return;
        }
        let n = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == "dungeon_spirit").count();
        if n >= 3 {
            return;
        }
        let me = self.body.pos;
        if let Some(at) = self.spot_near(me, 10.0, 22.0, true) {
            if self.mobs.spawn("dungeon_spirit", at + Vec3::Y) {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.aggro = true;
                }
            }
        }
    }

    /// Test scenes: MC_TR=queen2, idol, duke, empress, prophet (each mid-fight) and pillars.
    pub(crate) fn test_tr_endgame(&mut self, scene: &str) -> bool {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let eye = self.body.eye();
        let face = |g: &mut Game| {
            if let Some(m) = g.mobs.list.last_mut() {
                let look = eye - m.body.center();
                m.yaw = look.x.atan2(look.z);
                m.body.flying = true;
            }
        };
        match scene {
            "queen2" | "idol" | "duke" | "empress" | "prophet" => {
                let boss = match scene {
                    "queen2" => "jelly_queen",
                    "idol" => "scale_idol",
                    "duke" => "tide_duke",
                    "empress" => "prism_empress",
                    _ => "lunar_prophet",
                };
                let dist = if scene == "idol" { 11.0 } else { 9.0 };
                let mut at = self.body.pos + flat * dist;
                at.y = if scene == "idol" {
                    self.world.surface_y(at.x.floor() as i32, at.z.floor() as i32).map_or(at.y, |y| y as f32 + 1.0)
                } else {
                    eye.y - 1.0
                };
                self.mobs.spawn(boss, at);
                face(self);
                // A few of its shots in the air.
                let from = at + Vec3::Y * 2.0;
                match scene {
                    "empress" => {
                        for k in 0..18 {
                            let b = k as f32 * 0.35;
                            let dir = Vec3::new(b.cos(), -0.1, b.sin());
                            self.enemy_bolt(from + dir * (1.0 + k as f32 * 0.25), dir, (k % 6) as u8, 1.0);
                        }
                    }
                    "duke" => self.fan(from, 5, 0.3, 0.1, 1, 1.0),
                    "prophet" => self.fan(from, 5, 0.2, 0.1, 0, 1.0),
                    _ => self.fan(from, 5, 0.2, 0.1, 3, 1.0),
                }
                if scene == "queen2" {
                    self.t5.tr.end.phase2 = true;
                }
                true
            }
            "pillars" => {
                for (k, &(kind, folk, flies)) in PILLARS.iter().enumerate() {
                    let side = flat.cross(Vec3::Y);
                    let p = self.body.pos + flat * 26.0 + side * (k as f32 - 1.5) * 12.0;
                    let y = self.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map_or(p.y, |y| y as f32 + 1.0);
                    let at = Vec3::new(p.x, y, p.z);
                    self.t5.tr.end.pillars.push(Pillar { kind: kind.to_string(), pos: at.to_array(), shield: SHIELD, dead: false });
                    self.mobs.spawn(&format!("pillar_{kind}"), at);
                    face(self);
                    let fp = self.body.pos + flat * 9.0 + side * (k as f32 - 1.5) * 2.6;
                    let fy = self.world.surface_y(fp.x.floor() as i32, fp.z.floor() as i32).map_or(fp.y, |y| y as f32 + 1.0) + if flies { 1.5 } else { 0.0 };
                    self.mobs.spawn(folk, Vec3::new(fp.x, fy, fp.z));
                    face(self);
                }
                true
            }
            _ => false,
        }
    }
}
