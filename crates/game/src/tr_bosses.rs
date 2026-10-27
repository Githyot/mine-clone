//! The early bosses Terraria has before its hard mode that we lacked (plan/terraria-completo.md,
//! lote 1), our own creatures:
//! - the Jelly King (Rei Geleia) hops after you in great leaps, shakes the ground where it lands,
//!   sheds jellies as it is hurt and wobbles back to you when you run;
//! - the Great Eye (Grande Olho) hovers and stares, calls servant eyes, dashes through you; at half
//!   health it spins and turns savage, dashing faster and harder;
//! - the Blight Mind (Mente da Chaga) floats behind a ring of creepers that shield it; once they
//!   are dead it hunts you, blinking about and leaving false copies of itself;
//! - the Hive Queen (Rainha da Colmeia) hovers and stings, charges across the arena, calls drones;
//! - the Frost Colossus walks and fights like the Forest Guardian (bosses.rs) and adds lines of
//!   ice spikes and, when angry, a roar that darkens and slows.
//! The small flyers of these fights (servant eyes, drones, loose creepers) are steered here too.

use glam::Vec3;
use voxel::physics::Terrain;
use voxel::worldgen::deep::UBiome;
use voxel::worldgen::{Biome, Dim};

use crate::audio::Sfx;
use crate::effects::{CONFUSED, DARKNESS, POISON, SLOWNESS};
use crate::Game;

/// Bosses fought entirely in this file (bosses.rs leaves them alone).
pub fn own_ai(name: &str) -> bool {
    matches!(name, "king_jelly" | "great_eye" | "blight_mind" | "hive_queen") || crate::tr_endgame::own_ai(name)
}

/// (summon item, boss, where it works)
const SUMMONS: &[(&str, &str, &str)] = &[
    ("jelly_crown", "king_jelly", "surface"),
    ("eye_lure", "great_eye", "night"),
    ("blight_spine", "blight_mind", "underground"),
    ("royal_comb", "hive_queen", "jungle"),
    ("frost_horn", "frost_colossus", "snow"),
];

const SMALL_FLYERS: [&str; 11] = ["vortex_hornet", "nebula_eye", "dungeon_spirit", "harpy", "servant_eye", "hive_drone", "blight_creeper", "meteor_head", "dune_wraith", "reaper", "eclipse_moth"];

#[derive(Default)]
pub struct Fight {
    /// (reset_contact lets the next touch land at once.)
    vel: Vec3,
    t: f32,
    mode: u8,
    /// Moves left in a run of dashes or charges.
    left: u8,
    shot_t: f32,
    spawn_t: f32,
    contact_t: f32,
    /// Health when the boss last shed a jelly / was last shielded.
    mark: f32,
    /// The Great Eye's second phase, the Mind's hunt.
    savage: bool,
    told: bool,
    roar_t: f32,
    spikes_t: f32,
    /// Small flyers' own velocities, by mob index.
    flyers: Vec<(usize, Vec3)>,
}

fn jungle(b: Biome) -> bool {
    matches!(b, Biome::Jungle | Biome::BambooJungle | Biome::SparseJungle | Biome::MataAtlantica)
}

fn snowy(b: Biome) -> bool {
    matches!(b, Biome::Snowy | Biome::SnowyPeaks | Biome::SnowyTaiga | Biome::IceSpikes | Biome::SnowySlopes | Biome::Grove | Biome::FrozenRiver | Biome::SnowyBeach | Biome::FrozenOcean)
}

impl Fight {
    pub(crate) fn reset_contact(&mut self) {
        self.contact_t = 0.0;
    }
}

impl Game {
    fn tr_boss(&self) -> Option<usize> {
        self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].boss)
    }

    /// Right click with a summon item of this file.
    pub(crate) fn tr_summon(&mut self, item: &str) -> Option<bool> {
        let &(_, boss, rule) = SUMMONS.iter().find(|s| s.0 == item)?;
        let over = self.dim == Dim::Overworld;
        let (x, z) = (self.body.pos.x.floor() as i32, self.body.pos.z.floor() as i32);
        let biome = self.world.gen.base_biome_at(x, z);
        let ok = over
            && match rule {
                "surface" => self.body.pos.y > 30.0,
                "night" => self.is_night() && self.body.pos.y > 30.0,
                "underground" => self.body.pos.y < 0.0,
                "jungle" => jungle(biome) || self.ubiome() == UBiome::Jungle,
                _ => snowy(biome) || self.ubiome() == UBiome::Ice,
            };
        if !ok {
            let why = match rule {
                "surface" => "Isso só funciona na superfície",
                "night" => "Isso só desperta à noite, na superfície",
                "underground" => "Isso só funciona nas profundezas (abaixo da altura 0)",
                "jungle" => "Isso só funciona na selva",
                _ => "Isso só funciona na neve",
            };
            self.toast(why, 3.0);
            return Some(true);
        }
        if self.tr_boss().is_some() {
            self.toast("Já há um chefe acordado", 3.0);
            return Some(true);
        }
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let mut at = self.body.pos + flat * 12.0;
        let ground = self.world.surface_y(at.x.floor() as i32, at.z.floor() as i32).map(|y| y as f32 + 1.0);
        match boss {
            // Falls out of the sky.
            "king_jelly" => at.y = ground.unwrap_or(self.body.pos.y) + 14.0,
            "frost_colossus" => at.y = if self.body.pos.y > 0.0 { ground.unwrap_or(self.body.pos.y) } else { self.body.pos.y },
            _ => at.y = self.body.pos.y + 7.0,
        }
        if !self.spawn_tr_boss(boss, at) {
            return Some(false);
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        let text = match boss {
            "king_jelly" => "O Rei Geleia caiu do céu!",
            "great_eye" => "O Grande Olho está te observando...",
            "blight_mind" => "A Mente da Chaga desperta!",
            "hive_queen" => "A Rainha da Colmeia despertou!",
            _ => "O Colosso do Gelo vem pela neve!",
        };
        self.weather.flash = 0.5;
        self.sfx(Sfx::Thunder(0.5));
        self.toast(text, 4.0);
        Some(true)
    }

    /// Wake a boss of this file at `at` (and the Mind's creepers).
    pub(crate) fn spawn_tr_boss(&mut self, boss: &str, at: Vec3) -> bool {
        if !self.mobs.spawn(boss, at) {
            return false;
        }
        if let Some(m) = self.mobs.list.last_mut() {
            m.aggro = true;
        }
        if boss == "blight_mind" {
            for k in 0..12 {
                let a = k as f32 / 12.0 * std::f32::consts::TAU;
                self.mobs.spawn("blight_creeper", at + Vec3::new(a.cos() * 4.0, 1.0, a.sin() * 4.0));
            }
        }
        self.boss.called_help = false;
        let hp = self.mobs.list.iter().rev().find(|m| self.mobs.reg.defs[m.def].name == boss).map(|m| m.max_health).unwrap_or(0.0);
        self.t5.tr.fight = Fight { mark: hp, t: 0.0, spikes_t: 4.0, roar_t: 8.0, ..Default::default() };
        true
    }

    pub(crate) fn update_tr_fight(&mut self, dt: f32) {
        if dt == 0.0 {
            return;
        }
        self.t5.tr.fight.contact_t -= dt;
        if let Some(i) = self.tr_boss() {
            let name = self.mobs.reg.defs[self.mobs.list[i].def].name.clone();
            match name.as_str() {
                "king_jelly" => self.fight_king(i, dt),
                "great_eye" => self.fight_eye(i, dt),
                "blight_mind" => self.fight_mind(i, dt),
                "hive_queen" => self.fight_queen(i, dt),
                "frost_colossus" => self.fight_colossus(i, dt),
                _ => {}
            }
        }
        self.fly_small(dt);
    }

    /// Hurt the player when a boss's body touches them.
    pub(crate) fn tr_contact(&mut self, pos: Vec3, radius: f32, damage: f32) {
        if self.t5.tr.fight.contact_t > 0.0 || !self.survival {
            return;
        }
        let me = self.body.pos + Vec3::Y * 0.9;
        if me.distance(pos) < radius + 0.6 {
            self.t5.tr.fight.contact_t = 0.7;
            self.damage(damage, Some(pos));
            self.rpg_mob_hit(Some(pos));
        }
    }

    pub(crate) fn puff(&mut self, at: Vec3, n: usize, spread: f32, tint: [f32; 3]) {
        let glow = self.glow_layer();
        for _ in 0..n {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.3, self.surv.rng.f32() - 0.5) * spread;
            self.particles.glow(at, v, glow, tint, 0.14, 0.6);
        }
    }

    // ---------------------------------------------------------------------------------------
    // The Jelly King

    fn fight_king(&mut self, i: usize, dt: f32) {
        let me = self.body.pos;
        let (pos, on_ground, hp, max) = {
            let m = &self.mobs.list[i];
            (m.body.pos, m.body.on_ground, m.health, m.max_health)
        };
        let angry = hp < max * 0.5;
        let flat = Vec3::new(me.x - pos.x, 0.0, me.z - pos.z);
        let dist = flat.length();
        let f = &mut self.t5.tr.fight;
        f.t += dt;
        let mut jump = None;
        let mut landed = false;
        if on_ground {
            if f.mode == 1 && f.t > 0.2 {
                landed = true;
                f.mode = 0;
                f.t = 0.0;
            } else if f.t > if angry { 0.9 } else { 1.4 } {
                f.left = f.left.wrapping_add(1);
                let big = f.left % 4 == 0;
                let dir = flat.normalize_or_zero();
                jump = Some(dir * if big { 5.0 } else { 7.5 } + Vec3::Y * if big { 15.0 } else { 10.0 });
                f.mode = 1;
                f.t = 0.0;
            }
        }
        // Shed a jelly for every 7% of its health lost.
        let shed = f.mark - hp > max * 0.07;
        if shed {
            f.mark = hp;
        }
        let m = &mut self.mobs.list[i];
        m.aggro = true;
        m.yaw = flat.x.atan2(flat.z);
        if let Some(v) = jump {
            m.body.vel = v;
        } else if on_ground {
            m.body.vel.x *= 0.5;
            m.body.vel.z *= 0.5;
        }
        let attack = self.mobs.reg.defs[m.def].attack;
        if landed {
            let ground = self.reg.id_or_air("slime_block");
            let (layer, tint) = self.block_look(ground);
            for k in 0..10 {
                let a = k as f32 / 10.0 * std::f32::consts::TAU;
                self.particles.block_bits((pos + Vec3::new(a.cos() * 2.0, -0.5, a.sin() * 2.0)).floor().as_ivec3(), layer, tint, 2, 1.2);
            }
            self.sfx(Sfx::Explode);
            if dist < 4.0 && (me.y - pos.y).abs() < 2.0 {
                self.t5.tr.fight.contact_t = 0.0;
                self.tr_contact(me + Vec3::Y * 0.9, 1.0, attack * 1.3);
            }
        }
        if shed {
            let kind = if angry && self.surv.rng.f32() < 0.5 { "jelly_spiked" } else { "jelly_blue" };
            self.mobs.spawn(kind, pos + Vec3::Y * 1.5);
            if let Some(j) = self.mobs.list.last_mut() {
                j.aggro = true;
                j.body.vel = Vec3::new(self.surv.rng.f32() - 0.5, 1.0, self.surv.rng.f32() - 0.5) * 6.0;
            }
        }
        // Too far away: it wobbles, sinks and comes up beside you.
        if dist > 30.0 || (me.y - pos.y).abs() > 14.0 {
            self.puff(pos + Vec3::Y, 30, 6.0, [0.5, 0.8, 1.6]);
            let a = self.surv.rng.f32() * std::f32::consts::TAU;
            let to = me + Vec3::new(a.cos() * 6.0, 0.0, a.sin() * 6.0);
            let y = self.world.surface_y(to.x.floor() as i32, to.z.floor() as i32).map(|y| y as f32 + 1.0).filter(|y| (y - me.y).abs() < 8.0).unwrap_or(me.y);
            let m = &mut self.mobs.list[i];
            m.body.pos = Vec3::new(to.x, y, to.z);
            m.body.vel = Vec3::ZERO;
            self.puff(Vec3::new(to.x, y + 1.0, to.z), 30, 6.0, [0.5, 0.8, 1.6]);
        }
        self.tr_contact(pos + Vec3::Y * 1.3, 1.8, attack);
    }

    // ---------------------------------------------------------------------------------------
    // The Great Eye

    fn fight_eye(&mut self, i: usize, dt: f32) {
        let me = self.body.eye();
        let (pos, hp, max) = {
            let m = &self.mobs.list[i];
            (m.body.pos + Vec3::Y * 1.1, m.health, m.max_health)
        };
        let savage_now = hp < max * 0.5;
        let servants = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == "servant_eye").count();
        let f = &mut self.t5.tr.fight;
        f.t += dt;
        f.spawn_t -= dt;
        let mut call = false;
        let mut turned = false;
        let mut spin = 0.0;
        if savage_now && !f.savage {
            // The change: it spins in place, then comes back savage.
            f.savage = true;
            f.mode = 3;
            f.t = 0.0;
            turned = true;
        }
        match f.mode {
            0 => {
                let a = f.t * 0.9;
                let spot = me + Vec3::new(a.cos() * 7.0, 5.0, a.sin() * 7.0);
                let want = (spot - pos) * 2.2;
                f.vel += (want - f.vel) * (3.0 * dt).min(1.0);
                if !f.savage && f.spawn_t <= 0.0 && servants < 6 {
                    f.spawn_t = 1.3;
                    call = true;
                }
                if f.t > if f.savage { 2.5 } else { 4.5 } {
                    f.mode = 1;
                    f.t = 0.0;
                    f.left = if f.savage { 5 } else { 3 };
                    f.vel = (me - pos).normalize_or_zero() * if f.savage { 28.0 } else { 20.0 };
                }
            }
            1 => {
                // A dash, then a short brake before the next.
                if f.t > 0.75 {
                    f.left = f.left.saturating_sub(1);
                    f.t = 0.0;
                    if f.left == 0 {
                        f.mode = 0;
                    } else {
                        f.vel = (me - pos).normalize_or_zero() * if f.savage { 28.0 } else { 20.0 };
                    }
                } else if f.t > 0.5 {
                    f.vel *= 1.0 - (6.0 * dt).min(1.0);
                }
            }
            _ => {
                f.vel *= 1.0 - (3.0 * dt).min(1.0);
                spin = 14.0;
                if f.t > 1.6 {
                    f.mode = 0;
                    f.t = 0.0;
                }
            }
        }
        let vel = f.vel;
        let savage = f.savage;
        let m = &mut self.mobs.list[i];
        m.aggro = true;
        m.body.pos += vel * dt;
        m.body.vel = Vec3::ZERO;
        m.body.flying = true;
        let look = if self.t5.tr.fight.mode == 1 { vel } else { me - pos };
        m.yaw = if spin > 0.0 { m.yaw + spin * dt } else { look.x.atan2(look.z) };
        m.head_pitch = (-look.y / look.length().max(0.1)).asin().clamp(-1.2, 1.2);
        let attack = self.mobs.reg.defs[m.def].attack;
        if turned {
            self.toast("O Grande Olho se abre em fúria!", 3.0);
            self.puff(pos, 40, 8.0, [1.4, 0.3, 0.3]);
            self.sfx(Sfx::Thunder(0.4));
        }
        if call {
            self.mobs.spawn("servant_eye", pos + Vec3::new(0.0, -0.8, 0.0));
            if let Some(s) = self.mobs.list.last_mut() {
                s.aggro = true;
            }
        }
        self.tr_contact(pos, 1.3, attack * if savage { 1.5 } else { 1.0 });
    }

    // ---------------------------------------------------------------------------------------
    // The Blight Mind

    fn fight_mind(&mut self, i: usize, dt: f32) {
        let me = self.body.eye();
        let pos = self.mobs.list[i].body.pos + Vec3::Y * 1.2;
        let creepers: Vec<usize> = (0..self.mobs.list.len()).filter(|&k| self.mobs.list[k].alive() && self.mobs.reg.defs[self.mobs.list[k].def].name == "blight_creeper").collect();
        let shielded = !creepers.is_empty();
        let hp = self.mobs.list[i].health;
        let f = &mut self.t5.tr.fight;
        f.t += dt;
        let mut hit_shield = false;
        let mut freed = false;
        if shielded {
            // The creepers take every blow meant for it.
            if hp < f.mark {
                hit_shield = !f.told;
                f.told = true;
                self.mobs.list[i].health = f.mark;
            }
            let spot = me + Vec3::new((f.t * 0.4).sin() * 6.0, 4.0, (f.t * 0.4).cos() * 6.0);
            f.vel += ((spot - pos) * 0.8 - f.vel) * (1.5 * dt).min(1.0);
        } else {
            f.mark = hp;
            if !f.savage {
                f.savage = true;
                f.t = 0.0;
                freed = true;
            }
            let want = (me - pos).normalize_or_zero() * 7.0;
            f.vel += (want - f.vel) * (2.0 * dt).min(1.0);
        }
        let blink = !shielded && f.t > 3.0;
        if blink {
            f.t = 0.0;
        }
        let t = f.t;
        let vel = f.vel;
        if freed {
            self.toast("Sem os rastejantes, a Mente vem atrás de você!", 3.0);
        }
        if hit_shield {
            self.toast("A Mente está protegida: derrote os rastejantes primeiro!", 3.0);
        }
        if blink {
            // Jumps somewhere around you and leaves false copies behind.
            self.puff(pos, 25, 5.0, [1.3, 0.3, 0.4]);
            let a = self.surv.rng.f32() * std::f32::consts::TAU;
            for k in 1..4 {
                let b = a + k as f32 * 1.57;
                self.puff(me + Vec3::new(b.cos() * 7.0, 1.0, b.sin() * 7.0), 18, 2.0, [1.1, 0.3, 0.4]);
            }
            self.mobs.list[i].body.pos = me + Vec3::new(a.cos() * 7.0, 0.0, a.sin() * 7.0);
            self.sfx(Sfx::Cast(3));
        }
        let m = &mut self.mobs.list[i];
        m.aggro = true;
        m.body.pos += vel * dt;
        m.body.vel = Vec3::ZERO;
        m.body.flying = true;
        let look = me - pos;
        m.yaw = look.x.atan2(look.z);
        let attack = self.mobs.reg.defs[m.def].attack;
        let n = creepers.len().max(1) as f32;
        let center = self.mobs.list[i].body.pos + Vec3::Y * 0.8;
        for (j, &k) in creepers.iter().enumerate() {
            let c = &mut self.mobs.list[k];
            c.aggro = true;
            c.body.flying = true;
            c.body.vel = Vec3::ZERO;
            // Every few seconds one of them darts at you; the rest circle the Mind.
            let turn = (t * 0.5 + j as f32 * 0.37) % 4.0;
            let to = if turn < 0.7 && j % 3 == 0 {
                me
            } else {
                let a = t * 1.2 + j as f32 / n * std::f32::consts::TAU;
                center + Vec3::new(a.cos() * 3.5, (a * 2.0).sin() * 1.2, a.sin() * 3.5)
            };
            let d = to - c.body.pos;
            let step = d.normalize_or_zero() * (d.length() * 4.0).min(14.0) * dt;
            c.body.pos += step;
            c.yaw = (me - c.body.pos).x.atan2((me - c.body.pos).z);
            let cp = c.body.pos + Vec3::Y * 0.35;
            let ca = self.mobs.reg.defs[c.def].attack;
            self.tr_contact(cp, 0.6, ca);
        }
        self.tr_contact(pos, 1.2, attack);
    }

    // ---------------------------------------------------------------------------------------
    // The Hive Queen

    fn fight_queen(&mut self, i: usize, dt: f32) {
        let me = self.body.eye();
        let (pos, hp, max) = {
            let m = &self.mobs.list[i];
            (m.body.pos + Vec3::Y * 1.0, m.health, m.max_health)
        };
        let angry = hp < max * 0.5;
        let drones = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == "hive_drone").count();
        let f = &mut self.t5.tr.fight;
        f.t += dt;
        f.shot_t -= dt;
        f.spawn_t -= dt;
        let mut sting = false;
        let mut call = false;
        match f.mode {
            0 => {
                // Hover to one side and sting.
                let side = if (f.left % 2) == 0 { 1.0 } else { -1.0 };
                let spot = me + Vec3::new(side * 6.0, 3.0, 2.0);
                f.vel += ((spot - pos) * 2.0 - f.vel) * (2.5 * dt).min(1.0);
                if f.shot_t <= 0.0 {
                    f.shot_t = if angry { 0.4 } else { 0.65 };
                    sting = true;
                }
                if f.t > 4.0 {
                    f.mode = 1;
                    f.t = 0.0;
                    f.left = if angry { 4 } else { 3 };
                    let d = Vec3::new(me.x - pos.x, (me.y - pos.y) * 0.5, me.z - pos.z).normalize_or_zero();
                    f.vel = d * if angry { 24.0 } else { 19.0 };
                }
            }
            1 => {
                // Charges straight through, turning between them.
                if f.t > 1.1 {
                    f.left = f.left.saturating_sub(1);
                    f.t = 0.0;
                    if f.left == 0 {
                        f.mode = 2;
                    } else {
                        let d = Vec3::new(me.x - pos.x, (me.y - pos.y) * 0.5, me.z - pos.z).normalize_or_zero();
                        f.vel = d * if angry { 24.0 } else { 19.0 };
                    }
                } else if f.t > 0.75 {
                    f.vel *= 1.0 - (5.0 * dt).min(1.0);
                }
            }
            _ => {
                // Rise up and call the drones.
                let spot = me + Vec3::Y * 7.0;
                f.vel += ((spot - pos) * 1.5 - f.vel) * (2.0 * dt).min(1.0);
                if f.spawn_t <= 0.0 && drones < 8 {
                    f.spawn_t = 0.5;
                    call = true;
                }
                if f.t > 2.2 {
                    f.mode = 0;
                    f.t = 0.0;
                    f.left = f.left.wrapping_add(1);
                }
            }
        }
        let vel = f.vel;
        let charging = f.mode == 1;
        let m = &mut self.mobs.list[i];
        m.aggro = true;
        m.body.pos += vel * dt;
        m.body.vel = Vec3::ZERO;
        m.body.flying = true;
        let look = if charging { vel } else { me - pos };
        m.yaw = look.x.atan2(look.z);
        m.head_pitch = (-look.y / look.length().max(0.1)).asin().clamp(-0.8, 0.8);
        let attack = self.mobs.reg.defs[m.def].attack;
        if sting {
            let dir = (me - pos).normalize_or_zero();
            let side = dir.cross(Vec3::Y).normalize_or_zero();
            for k in 0..if angry { 3 } else { 1 } {
                let a = (k as f32 - if angry { 1.0 } else { 0.0 }) * 0.15;
                let d = (dir * a.cos() + side * a.sin()).normalize_or_zero();
                self.enemy_bolt(pos + d * 1.5, d * 18.0, 4, 3.0);
            }
        }
        if call {
            self.mobs.spawn("hive_drone", pos - Vec3::Y * 0.5);
            if let Some(d) = self.mobs.list.last_mut() {
                d.aggro = true;
            }
        }
        self.tr_contact(pos, 1.5, attack * if charging { 1.4 } else { 1.0 });
    }

    // ---------------------------------------------------------------------------------------
    // The Frost Colossus (walks in bosses.rs)

    fn fight_colossus(&mut self, i: usize, dt: f32) {
        let me = self.body.pos;
        let (pos, hp, max) = {
            let m = &self.mobs.list[i];
            (m.body.pos, m.health, m.max_health)
        };
        let angry = hp < max * 0.5;
        let f = &mut self.t5.tr.fight;
        f.spikes_t -= dt;
        f.roar_t -= dt;
        let spikes = f.spikes_t <= 0.0;
        if spikes {
            f.spikes_t = if angry { 4.0 } else { 6.0 };
        }
        let roar = angry && f.roar_t <= 0.0;
        if roar {
            f.roar_t = 12.0;
        }
        if spikes {
            // A line of ice spikes bursting up from the ground towards you.
            let flat = Vec3::new(me.x - pos.x, 0.0, me.z - pos.z);
            let len = flat.length().min(18.0);
            let dir = flat.normalize_or_zero();
            let glow = self.glow_layer();
            let mut hit = false;
            let n = (len / 1.2) as usize + 2;
            for k in 1..n {
                let p = pos + dir * k as f32 * 1.2;
                let y = self.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).filter(|y| (y - pos.y).abs() < 4.0).unwrap_or(pos.y);
                let at = Vec3::new(p.x, y, p.z);
                for h in 0..6 {
                    self.particles.glow(at + Vec3::Y * (h as f32 * 0.3), Vec3::new(0.0, 5.0, 0.0), glow, [0.7, 1.1, 1.6], 0.2, 0.5);
                }
                if (Vec3::new(me.x, 0.0, me.z) - Vec3::new(at.x, 0.0, at.z)).length() < 1.1 && (me.y - y).abs() < 1.5 {
                    hit = true;
                }
            }
            self.sfx(Sfx::Cast(1));
            if hit {
                self.t5.tr.fight.contact_t = 0.0;
                self.tr_contact(me + Vec3::Y * 0.9, 1.0, if angry { 8.0 } else { 6.0 });
                self.body.vel.y += 7.0;
            }
        }
        if roar && me.distance(pos) < 24.0 {
            self.toast("O Colosso ruge!", 2.0);
            self.sfx(Sfx::Thunder(0.3));
            self.add_effect(DARKNESS, 0, 5.0, false);
            self.add_effect(SLOWNESS, 0, 4.0, false);
        }
    }

    // ---------------------------------------------------------------------------------------
    // Servant eyes, drones and creepers on their own

    fn fly_small(&mut self, dt: f32) {
        let ids: Vec<usize> = SMALL_FLYERS.iter().filter_map(|n| self.mobs.reg.id(n)).collect();
        if ids.is_empty() {
            return;
        }
        let mind = self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].name == "blight_mind");
        let creeper = self.mobs.reg.id("blight_creeper");
        let me = self.body.eye() - Vec3::Y * 0.3;
        let mut flyers = std::mem::take(&mut self.t5.tr.fight.flyers);
        let wanted = |g: &Game, k: usize| {
            let m = &g.mobs.list[k];
            m.alive() && ids.contains(&m.def) && !(mind && Some(m.def) == creeper)
        };
        flyers.retain(|&(k, _)| k < self.mobs.list.len() && wanted(self, k));
        for k in 0..self.mobs.list.len() {
            if wanted(self, k) && !flyers.iter().any(|e| e.0 == k) {
                flyers.push((k, Vec3::ZERO));
            }
        }
        let t = self.now;
        for (k, vel) in flyers.iter_mut() {
            let pos = self.mobs.list[*k].body.pos + Vec3::Y * 0.3;
            let to = me - pos;
            let wobble = Vec3::new(((t * 1.7) + *k as f32).cos(), ((t * 2.3) + *k as f32).sin() * 1.5, ((t * 1.3) + *k as f32).sin());
            let want = to.normalize_or_zero() * 7.0 + wobble * 2.0;
            *vel += (want - *vel) * (1.8 * dt).min(1.0);
            let m = &mut self.mobs.list[*k];
            m.aggro = true;
            let next = m.body.pos + *vel * dt;
            if !self.world.solid((next + Vec3::Y * 0.3).floor().as_ivec3()) {
                m.body.pos = next;
            } else {
                *vel = -*vel * 0.5;
            }
            m.body.vel = Vec3::ZERO;
            m.body.flying = true;
            m.yaw = to.x.atan2(to.z);
            m.head_pitch = (-to.y / to.length().max(0.1)).asin().clamp(-1.2, 1.2);
            let attack = self.mobs.reg.defs[m.def].attack;
            if to.length() < 0.9 && self.t5.tr.fight.contact_t <= 0.0 && self.survival {
                self.t5.tr.fight.contact_t = 0.6;
                *vel = -to.normalize_or_zero() * 8.0;
                self.damage(attack, Some(pos));
                self.rpg_mob_hit(Some(pos));
            }
        }
        self.t5.tr.fight.flyers = flyers;
    }

    // ---------------------------------------------------------------------------------------
    // Accessories from these bosses

    /// After the player was hurt by something: the Mind's ward confuses, the hive pack stings.
    pub(crate) fn tr_after_hurt(&mut self, _amount: f32, from: Vec3) {
        if self.wears("mind_ward") {
            let me = self.body.pos;
            for k in 0..self.mobs.list.len() {
                if self.is_hostile(k) && self.mobs.list[k].body.pos.distance(me) < 7.0 && !self.mobs.reg.defs[self.mobs.list[k].def].boss {
                    self.add_mob_debuff(k, CONFUSED, 3.0);
                    let away = (self.mobs.list[k].body.pos - me).normalize_or_zero();
                    self.mobs.list[k].body.vel += away * 5.0 + Vec3::Y * 3.0;
                }
            }
            self.puff(me + Vec3::Y, 16, 5.0, [1.3, 0.4, 0.6]);
        }
        if self.wears("hive_pack") {
            let glow = self.glow_layer();
            let me = self.body.pos + Vec3::Y;
            let mut hit = Vec::new();
            for _ in 0..3 {
                let Some(k) = self.nearest_hostile(from, 8.0).filter(|k| !hit.contains(k)).or_else(|| (0..self.mobs.list.len()).find(|&k| self.is_hostile(k) && !hit.contains(&k) && self.mobs.list[k].body.pos.distance(from) < 8.0)) else { break };
                hit.push(k);
                let c = self.mobs.list[k].body.center();
                for j in 0..10 {
                    let at = me + (c - me) * (j as f32 / 10.0);
                    self.particles.glow(at, Vec3::ZERO, glow, [1.5, 1.2, 0.3], 0.07, 0.4);
                }
                let dmg = 4.0 * self.t5.rpg.stats.dmg[crate::rpg::SUMMON];
                self.mobs.list[k].hurt(dmg, me);
                self.add_mob_debuff(k, POISON, 4.0);
            }
        }
    }

    /// The royal gel: jellies don't hurt you.
    pub(crate) fn tr_ignores_hit(&self, from: Option<Vec3>) -> bool {
        let Some(f) = from else { return false };
        if !self.wears("royal_gel") {
            return false;
        }
        let near = self.mobs.list.iter().filter(|m| m.alive()).map(|m| (m, m.body.center().distance(f))).min_by(|a, b| a.1.total_cmp(&b.1));
        near.is_some_and(|(m, d)| d < 3.5 && self.mobs.reg.defs[m.def].name.starts_with("jelly_"))
    }
}
