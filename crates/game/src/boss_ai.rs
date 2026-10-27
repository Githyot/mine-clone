//! How the Etapa 5 bosses and flyers move (Terraria's fights, our own creatures). The walkers
//! (World Breaker, Vine Queen) use the Forest Guardian's walk-and-specials in bosses.rs; this
//! file flies the rest, after the mobs' own tick so it has the last word on where they are:
//! - worms (Deep Worm, Steel Serpent) swim through stone and dirt after you and leap out in arcs;
//!   their body follows the head link by link, and a hit anywhere hurts the head;
//! - eyes (demon eyes, the Mechanical Eye) hover around you and dash; the mech eye fires lasers;
//! - skulls (Bone King, Steel Skull) float above you, rain bolts and spin down at you;
//! - the Celestial Herald hovers, blinks next to you and calls down falling stars.
//! Summon items and where they work are here too, and what each boss opens when it falls.

use glam::Vec3;
use voxel::physics::Terrain;
use voxel::worldgen::{Biome, Dim};

use crate::audio::Sfx;
use crate::Game;

#[derive(Default)]
pub struct Fight {
    /// The worm head's (or flyer's) own velocity, kept across frames.
    vel: Vec3,
    /// Seconds into the current move, and which move.
    t: f32,
    mode: u8,
    shot_t: f32,
    /// Seconds before contact can hurt the player again.
    contact_t: f32,
    /// Small flyers (demon eyes) have their own velocities, by mob index.
    eyes: Vec<(usize, Vec3)>,
}

impl Fight {
    pub(crate) fn vel_for_test(&mut self, v: Vec3) {
        self.vel = v;
    }
}

/// (summon item, boss, where it works, what the screen says)
const SUMMONS: &[(&str, &str, &str)] = &[
    ("worm_bait", "deep_worm", "underground"),
    ("cursed_skull", "bone_king", "night"),
    ("breaker_idol", "world_breaker", "nether"),
    ("gear_eye", "mech_eye", "hard_night"),
    ("iron_worm", "mech_worm", "hard_night"),
    ("steel_skull", "mech_skull", "hard_night"),
    ("queen_bulb", "vine_queen", "jungle"),
    ("celestial_sigil", "celestial_herald", "after_queen"),
];

/// Bosses whose whole fight is in this file (bosses.rs leaves them alone).
pub fn own_ai(name: &str) -> bool {
    matches!(name, "deep_worm" | "mech_worm" | "mech_eye" | "bone_king" | "mech_skull" | "celestial_herald")
}

fn is_worm_head(name: &str) -> bool {
    name == "deep_worm" || name == "mech_worm"
}

fn segments(head: &str) -> (&'static str, &'static str, usize) {
    if head == "mech_worm" {
        ("mech_worm_body", "mech_worm_tail", 14)
    } else {
        ("deep_worm_body", "deep_worm_tail", 10)
    }
}

impl Game {
    /// Right click with one of the Etapa 5 summon items: checks where it works, then wakes the
    /// boss like the others (bosses.rs). Worms get their body right away.
    pub(crate) fn t5_summon(&mut self, item: &str) -> Option<bool> {
        let &(_, boss, rule) = SUMMONS.iter().find(|s| s.0 == item)?;
        let ok = match rule {
            "underground" => self.body.pos.y < 50.0 && self.dim == Dim::Overworld,
            "night" => self.is_night() && self.dim == Dim::Overworld,
            "nether" => self.dim == Dim::Nether,
            "hard_night" => self.t5.hard.on && self.is_night() && self.dim == Dim::Overworld,
            "jungle" => self.t5.hard.on && self.dim == Dim::Overworld && self.world.gen.base_biome_at(self.body.pos.x as i32, self.body.pos.z as i32) == Biome::Jungle,
            _ => self.boss.defeated.iter().any(|b| b == "vine_queen"),
        };
        if !ok {
            let why = match rule {
                "underground" => "Isso só funciona no subsolo (abaixo da altura 50)",
                "night" => "Isso só desperta à noite",
                "nether" => "Isso só funciona no Nether",
                "hard_night" => "Só à noite, depois que o Quebra-Mundos cair",
                "jungle" => "Só na selva, depois que o Quebra-Mundos cair",
                _ => "Só depois de derrotar a Rainha das Vinhas",
            };
            self.toast(why, 3.0);
            return Some(true);
        }
        if self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].boss) {
            self.toast("Já há um chefe acordado", 3.0);
            return Some(true);
        }
        let text = match boss {
            "deep_worm" => "O Verme das Profundezas está cavando até você!",
            "bone_king" => "O Rei dos Ossos se ergue!",
            "world_breaker" => "O Quebra-Mundos despertou!",
            "mech_eye" => "O Olho Mecânico está observando...",
            "mech_worm" => "A Serpente de Aço range sob a terra!",
            "mech_skull" => "O Crânio de Aço gira no céu!",
            "vine_queen" => "A Rainha das Vinhas desabrocha!",
            _ => "O Arauto Celeste desce das estrelas!",
        };
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let mut at = self.body.pos + flat * 14.0;
        let flies = matches!(boss, "mech_eye" | "mech_skull" | "bone_king" | "celestial_herald");
        if flies {
            at.y = self.body.pos.y + 8.0;
        } else if is_worm_head(boss) {
            at.y = self.body.pos.y - 6.0;
        } else {
            let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
            at.y = self.world.surface_y(x, z).map(|y| y as f32 + 1.0).unwrap_or(at.y);
        }
        if !self.mobs.spawn(boss, at) {
            return Some(false);
        }
        if is_worm_head(boss) {
            self.spawn_worm_body(boss, at);
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.boss.called_help = false;
        self.t5.fight = Fight { vel: Vec3::Y * 4.0, ..Default::default() };
        self.weather.flash = 0.6;
        self.sfx(Sfx::Thunder(0.6));
        self.toast(text, 4.0);
        Some(true)
    }

    /// The links behind a worm head that was just spawned (they come right after it in the list).
    pub(crate) fn spawn_worm_body(&mut self, head: &str, at: Vec3) {
        let (body, tail, n) = segments(head);
        for k in 0..n {
            let name = if k + 1 == n { tail } else { body };
            self.mobs.spawn(name, at - Vec3::Y * (k as f32 + 1.0) * 1.2);
            if let Some(m) = self.mobs.list.last_mut() {
                m.aggro = true;
            }
        }
    }

    pub(crate) fn update_fight(&mut self, dt: f32) {
        self.t5.fight.contact_t -= dt;
        // Worms, flyers and the herald, one boss at a time.
        let boss = self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].boss);
        if let Some(i) = boss {
            let name = self.mobs.reg.defs[self.mobs.list[i].def].name.clone();
            match name.as_str() {
                "deep_worm" | "mech_worm" => self.fly_worm(i, &name, dt),
                "mech_eye" => self.fly_eye(i, dt, true),
                "bone_king" | "mech_skull" => self.fly_skull(i, &name, dt),
                "celestial_herald" => self.fly_herald(i, dt),
                _ => {}
            }
        }
        self.clean_worm_bodies();
        self.fly_demon_eyes(dt);
    }

    /// Hurt the player when a flying or burrowing body touches them.
    fn contact(&mut self, pos: Vec3, radius: f32, damage: f32) {
        if self.t5.fight.contact_t > 0.0 || !self.survival {
            return;
        }
        let me = self.body.pos + Vec3::Y * 0.9;
        if me.distance(pos) < radius + 0.6 {
            self.t5.fight.contact_t = 0.7;
            self.damage(damage, Some(pos));
        }
    }

    fn fly_worm(&mut self, i: usize, name: &str, dt: f32) {
        if dt == 0.0 {
            return;
        }
        let mech = name == "mech_worm";
        let target = self.body.pos + Vec3::Y * 0.8;
        let (head_pos, angry) = {
            let m = &self.mobs.list[i];
            (m.body.pos, m.health < m.max_health * 0.5)
        };
        let inside = self.world.solid(head_pos.floor().as_ivec3()) || self.world.solid((head_pos + Vec3::Y).floor().as_ivec3());
        let speed = if mech { 17.0 } else { 12.0 } * if angry { 1.2 } else { 1.0 };
        let f = &mut self.t5.fight;
        if inside || mech {
            // Through the ground (the steel serpent also through the air): steer at the player.
            let want = (target - head_pos).normalize_or_zero() * speed;
            let turn = if mech { 2.2 } else { 1.6 };
            f.vel += (want - f.vel) * (turn * dt).min(1.0);
        } else {
            // Out in the air: it arcs and falls back down.
            f.vel.y -= 22.0 * dt;
        }
        if f.vel.length() > speed * 1.4 {
            f.vel = f.vel.normalize() * speed * 1.4;
        }
        let vel = f.vel;
        let new = head_pos + vel * dt;
        let link = if mech { 1.5 } else { 1.25 };
        {
            let m = &mut self.mobs.list[i];
            m.aggro = true;
            m.body.pos = new;
            m.body.vel = Vec3::ZERO;
            m.body.flying = true;
            m.yaw = vel.x.atan2(vel.z);
            m.head_pitch = (-vel.y / vel.length().max(0.1)).asin().clamp(-1.4, 1.4);
        }
        // Breaking out of the ground throws up dirt.
        if inside != (self.world.solid(new.floor().as_ivec3())) {
            let id = self.world.block_or_air(head_pos.floor().as_ivec3()).max(self.world.block_or_air(new.floor().as_ivec3()));
            let (layer, tint) = self.block_look(id);
            self.particles.block_bits(new.floor().as_ivec3(), layer, tint, 6, 1.2);
        }
        let attack = self.mobs.reg.defs[self.mobs.list[i].def].attack;
        self.contact(new + Vec3::Y * 0.6, 1.0, attack);
        // The links follow, each a fixed distance behind the one before.
        let mut prev = new;
        let mut k = i + 1;
        while k < self.mobs.list.len() {
            let dn = &self.mobs.reg.defs[self.mobs.list[k].def].name;
            if !(dn.starts_with(name) && dn != name) {
                break;
            }
            let tail = dn.ends_with("_tail");
            let seg_attack = self.mobs.reg.defs[self.mobs.list[k].def].attack;
            let m = &mut self.mobs.list[k];
            let d = prev - m.body.pos;
            let len = d.length();
            if len > link {
                m.body.pos = prev - d / len * link;
            }
            m.body.vel = Vec3::ZERO;
            m.body.flying = true;
            m.aggro = true;
            m.yaw = d.x.atan2(d.z);
            m.head_pitch = (-d.y / len.max(0.1)).asin().clamp(-1.4, 1.4);
            // A hit on any link hurts the head.
            let lost = m.max_health - m.health;
            if lost > 0.0 {
                m.health = m.max_health;
                let head = &mut self.mobs.list[i];
                head.health -= lost;
                head.hurt_age = 0.0;
            }
            prev = self.mobs.list[k].body.pos;
            self.contact(prev + Vec3::Y * 0.6, 0.9, seg_attack);
            // The steel serpent's links fire lasers now and then.
            if mech && !tail && k % 3 == 0 && self.surv.rng.f32() < dt * if angry { 0.35 } else { 0.18 } {
                let from = prev + Vec3::Y * 0.7;
                let dir = (self.body.eye() - from).normalize_or_zero();
                self.enemy_bolt(from + dir, dir * 20.0, 3, 4.0);
            }
            k += 1;
        }
        if self.mobs.list[i].health <= 0.0 {
            self.mobs.list[i].health = 0.0;
        }
    }

    /// Links of dead worms go with them (no drops of their own).
    fn clean_worm_bodies(&mut self) {
        let mut head_alive = false;
        for k in 0..self.mobs.list.len() {
            let name = self.mobs.reg.defs[self.mobs.list[k].def].name.as_str();
            if is_worm_head(name) {
                head_alive = self.mobs.list[k].alive();
            } else if name.starts_with("deep_worm_") || name.starts_with("mech_worm_") {
                let m = &mut self.mobs.list[k];
                if !head_alive && m.alive() {
                    m.health = 0.0;
                    m.counted = true;
                    m.body.flying = false;
                }
            } else {
                // Anything between heads breaks the chain.
                head_alive = false;
            }
        }
    }

    /// Hover near the player, now and then dash through; the mechanical eye fires lasers.
    fn fly_eye(&mut self, i: usize, dt: f32, mech: bool) {
        if dt == 0.0 {
            return;
        }
        let me = self.body.eye();
        let (pos, angry) = {
            let m = &self.mobs.list[i];
            (m.body.pos + Vec3::Y * 0.8, m.health < m.max_health * 0.5)
        };
        let f = &mut self.t5.fight;
        f.t += dt;
        let dash_every = if angry { 2.2 } else { 3.5 };
        if f.mode == 0 {
            // Hover: circle above and to the side.
            let a = f.t * 0.8;
            let spot = me + Vec3::new(a.cos() * 7.0, 4.0, a.sin() * 7.0);
            let want = (spot - pos) * 2.0;
            f.vel += (want - f.vel) * (3.0 * dt).min(1.0);
            if f.t > dash_every {
                f.t = 0.0;
                f.mode = 1;
                f.vel = (me - pos).normalize_or_zero() * if angry { 26.0 } else { 20.0 };
            }
        } else if f.t > 0.9 {
            f.t = 0.0;
            f.mode = 0;
        }
        let vel = f.vel;
        f.shot_t -= dt;
        let shoot = mech && angry && f.mode == 0 && f.shot_t <= 0.0;
        if shoot {
            f.shot_t = 0.9;
        }
        let m = &mut self.mobs.list[i];
        m.aggro = true;
        m.body.pos += vel * dt;
        m.body.vel = Vec3::ZERO;
        m.body.flying = true;
        let look = me - pos;
        m.yaw = look.x.atan2(look.z);
        m.head_pitch = (-look.y / look.length().max(0.1)).asin().clamp(-1.2, 1.2);
        let attack = self.mobs.reg.defs[m.def].attack;
        if shoot {
            let dir = look.normalize_or_zero();
            self.enemy_bolt(pos + dir, dir * 22.0, 3, 5.0);
            self.sfx(Sfx::Cast(3));
        }
        self.contact(pos, 0.9, attack);
    }

    /// Float above the player raining bolts; every few seconds spin down at them.
    fn fly_skull(&mut self, i: usize, name: &str, dt: f32) {
        if dt == 0.0 {
            return;
        }
        let mech = name == "mech_skull";
        let me = self.body.eye();
        let (pos, angry) = {
            let m = &self.mobs.list[i];
            (m.body.pos + Vec3::Y * 1.0, m.health < m.max_health * 0.5)
        };
        let f = &mut self.t5.fight;
        f.t += dt;
        f.shot_t -= dt;
        let spinning = f.mode == 1;
        if !spinning {
            let spot = me + Vec3::Y * 6.0;
            let want = (spot - pos) * 1.6;
            f.vel += (want - f.vel) * (2.5 * dt).min(1.0);
            if f.t > if angry { 5.0 } else { 7.0 } {
                f.t = 0.0;
                f.mode = 1;
            }
        } else {
            let want = (me - pos).normalize_or_zero() * if mech { 11.0 } else { 8.0 };
            f.vel += (want - f.vel) * (4.0 * dt).min(1.0);
            if f.t > 3.0 {
                f.t = 0.0;
                f.mode = 0;
            }
        }
        let vel = f.vel;
        let shoot = !spinning && f.shot_t <= 0.0;
        if shoot {
            f.shot_t = if angry { 1.2 } else { 1.8 };
        }
        let m = &mut self.mobs.list[i];
        m.aggro = true;
        m.body.pos += vel * dt;
        m.body.vel = Vec3::ZERO;
        m.body.flying = true;
        let look = me - pos;
        m.yaw = if spinning { m.yaw + dt * 12.0 } else { look.x.atan2(look.z) };
        m.head_pitch = (-look.y / look.length().max(0.1)).asin().clamp(-1.0, 1.0);
        let attack = self.mobs.reg.defs[m.def].attack;
        if shoot {
            let n = if angry { 3 } else { 1 };
            let dir = look.normalize_or_zero();
            let side = dir.cross(Vec3::Y).normalize_or_zero();
            for k in 0..n {
                let a = (k as f32 - (n - 1) as f32 / 2.0) * 0.2;
                let d = (dir * a.cos() + side * a.sin()).normalize_or_zero();
                // Bone bolts are cold; the steel skull's burn.
                self.enemy_bolt(pos + d, d * 16.0, if mech { 3 } else { 1 }, if mech { 6.0 } else { 4.0 });
            }
            self.sfx(Sfx::Cast(if mech { 3 } else { 1 }));
        }
        self.contact(pos, 1.1, attack * if spinning { 1.5 } else { 1.0 });
    }

    /// Hover high, blink next to the player, call down falling stars.
    fn fly_herald(&mut self, i: usize, dt: f32) {
        if dt == 0.0 {
            return;
        }
        let me = self.body.eye();
        let (pos, angry) = {
            let m = &self.mobs.list[i];
            (m.body.pos + Vec3::Y * 2.5, m.health < m.max_health * 0.5)
        };
        let f = &mut self.t5.fight;
        f.t += dt;
        f.shot_t -= dt;
        let spot = me + Vec3::new((f.t * 0.6).sin() * 8.0, 7.0, (f.t * 0.6).cos() * 8.0);
        let want = (spot - pos) * 1.5;
        f.vel += (want - f.vel) * (2.0 * dt).min(1.0);
        let blink = f.t > if angry { 4.0 } else { 6.0 };
        if blink {
            f.t = 0.0;
        }
        let stars = f.shot_t <= 0.0;
        if stars {
            f.shot_t = if angry { 1.4 } else { 2.2 };
        }
        let vel = f.vel;
        let glow = self.glow_layer();
        if blink {
            let a = self.surv.rng.f32() * std::f32::consts::TAU;
            let to = self.body.pos + Vec3::new(a.cos() * 6.0, 4.0, a.sin() * 6.0);
            for _ in 0..30 {
                let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5) * 6.0;
                self.particles.glow(pos, v, glow, [1.4, 1.3, 0.6], 0.12, 0.6);
            }
            self.mobs.list[i].body.pos = to;
            self.sfx(Sfx::Cast(2));
        }
        let m = &mut self.mobs.list[i];
        m.aggro = true;
        m.body.pos += vel * dt;
        m.body.vel = Vec3::ZERO;
        m.body.flying = true;
        let look = me - pos;
        m.yaw = look.x.atan2(look.z);
        let attack = self.mobs.reg.defs[m.def].attack;
        if stars {
            for k in 0..if angry { 7 } else { 4 } {
                let a = k as f32 * 2.4 + self.surv.rng.f32();
                let r = 1.0 + self.surv.rng.f32() * 4.0;
                let from = self.body.pos + Vec3::new(a.cos() * r, 14.0, a.sin() * r);
                self.enemy_bolt(from, Vec3::new(0.0, -16.0, 0.0), 2, 6.0);
            }
            self.sfx(Sfx::Cast(2));
        }
        self.contact(pos, 1.2, attack);
    }

    /// Demon eyes: weave towards the player, bounce off when they hit.
    fn fly_demon_eyes(&mut self, dt: f32) {
        if dt == 0.0 {
            return;
        }
        let Some(def) = self.mobs.reg.id("demon_eye") else { return };
        let me = self.body.eye() - Vec3::Y * 0.3;
        let mut eyes = std::mem::take(&mut self.t5.fight.eyes);
        eyes.retain(|&(k, _)| k < self.mobs.list.len() && self.mobs.list[k].def == def && self.mobs.list[k].alive());
        for k in 0..self.mobs.list.len() {
            if self.mobs.list[k].def == def && self.mobs.list[k].alive() && !eyes.iter().any(|e| e.0 == k) {
                eyes.push((k, Vec3::ZERO));
            }
        }
        let t = self.time_of_day;
        for (k, vel) in eyes.iter_mut() {
            let pos = self.mobs.list[*k].body.pos + Vec3::Y * 0.35;
            let to = me - pos;
            let wobble = Vec3::new(0.0, ((t * 0.05) + *k as f32).sin() * 2.0, 0.0);
            let want = to.normalize_or_zero() * 6.0 + wobble;
            *vel += (want - *vel) * (1.5 * dt).min(1.0);
            let m = &mut self.mobs.list[*k];
            m.aggro = true;
            let next = m.body.pos + *vel * dt;
            if !self.world.solid((next + Vec3::Y * 0.35).floor().as_ivec3()) {
                m.body.pos = next;
            } else {
                *vel = -*vel * 0.5;
            }
            m.body.vel = Vec3::ZERO;
            m.body.flying = true;
            m.yaw = to.x.atan2(to.z);
            m.head_pitch = (-to.y / to.length().max(0.1)).asin().clamp(-1.2, 1.2);
            if to.length() < 1.0 && self.t5.fight.contact_t <= 0.0 && self.survival {
                self.t5.fight.contact_t = 0.7;
                *vel = -to.normalize_or_zero() * 8.0;
                self.damage(3.0, Some(pos));
            }
        }
        self.t5.fight.eyes = eyes;
    }

    /// What a fallen Etapa 5 boss opens.
    pub(crate) fn t5_boss_defeated(&mut self, name: &str) {
        match name {
            "world_breaker" => self.enter_hard_mode(),
            "bone_king" => {
                if !self.t5.hard.mechanic_freed {
                    self.t5.hard.mechanic_freed = true;
                    self.toast("A Mecânica estava presa no castelo dos ossos: construa uma casa para ela!", 6.0);
                }
            }
            "mech_eye" | "mech_worm" | "mech_skull" => {
                let all = ["mech_eye", "mech_worm", "mech_skull"].iter().all(|b| self.boss.defeated.iter().any(|d| d == b));
                if all {
                    self.toast("A selva treme: a verdita brotou nas pedras da selva!", 6.0);
                }
            }
            _ => {}
        }
    }
}
