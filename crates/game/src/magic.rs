//! Magic (Terraria's idea, our own spells): a mana bar of stars, wands that shoot quick bolts,
//! staffs with strong slow spells and spell books. Spells are glowing bolts made of particles;
//! fire sets mobs alight, frost slows them, nature heals the caster, arcane bolts home in.
//! Mana refills on its own, faster after a moment without casting; mana crystals add stars.

use glam::Vec3;

use voxel::physics::Terrain;

use crate::audio::Sfx;
use crate::ui::UiBatch;
use crate::Game;

#[derive(Clone, Copy, PartialEq)]
enum Shot {
    /// One straight bolt.
    Bolt,
    /// A bolt that turns toward the nearest mob in front.
    Homing,
    /// Several bolts in a fan.
    Fan(u8),
    /// A slow ball that bursts on impact, hurting everything around.
    Burst(f32),
    /// Lightning from the sky onto the point you look at.
    Strike,
    /// A short stream of flame.
    Flame,
}

struct SpellDef {
    item: &'static str,
    /// 0 fire, 1 frost, 2 spark, 3 arcane, 4 nature, 5 storm.
    school: u8,
    cost: f32,
    cooldown: f32,
    damage: f32,
    speed: f32,
    life: f32,
    shot: Shot,
}

const SPELLS: &[SpellDef] = &[
    SpellDef { item: "ruby_wand", school: 0, cost: 3.0, cooldown: 0.4, damage: 4.0, speed: 26.0, life: 1.2, shot: Shot::Bolt },
    SpellDef { item: "sapphire_wand", school: 1, cost: 3.0, cooldown: 0.4, damage: 4.0, speed: 26.0, life: 1.2, shot: Shot::Bolt },
    SpellDef { item: "topaz_wand", school: 2, cost: 2.0, cooldown: 0.22, damage: 3.0, speed: 40.0, life: 0.8, shot: Shot::Bolt },
    SpellDef { item: "amethyst_wand", school: 3, cost: 4.0, cooldown: 0.5, damage: 5.0, speed: 16.0, life: 2.5, shot: Shot::Homing },
    SpellDef { item: "emerald_wand", school: 4, cost: 3.0, cooldown: 0.45, damage: 4.0, speed: 24.0, life: 1.2, shot: Shot::Bolt },
    SpellDef { item: "fire_staff", school: 0, cost: 8.0, cooldown: 1.0, damage: 10.0, speed: 14.0, life: 2.5, shot: Shot::Burst(3.0) },
    SpellDef { item: "frost_staff", school: 1, cost: 8.0, cooldown: 0.9, damage: 4.0, speed: 24.0, life: 1.0, shot: Shot::Fan(5) },
    SpellDef { item: "thunder_tome", school: 5, cost: 10.0, cooldown: 1.2, damage: 12.0, speed: 0.0, life: 0.0, shot: Shot::Strike },
    SpellDef { item: "guardian_staff", school: 4, cost: 6.0, cooldown: 0.6, damage: 6.0, speed: 26.0, life: 1.2, shot: Shot::Fan(3) },
    // Not a held item: the warship's cannon (mods/aircraft.rs) fires these.
    SpellDef { item: "@warship_cannon", school: 0, cost: 0.0, cooldown: 0.0, damage: 8.0, speed: 34.0, life: 1.6, shot: Shot::Burst(2.5) },
    SpellDef { item: "flame_tome", school: 0, cost: 1.0, cooldown: 0.08, damage: 1.5, speed: 12.0, life: 0.35, shot: Shot::Flame },
];

fn color(school: u8) -> [f32; 3] {
    match school {
        0 => [1.6, 0.55, 0.15],
        1 => [0.5, 0.85, 1.6],
        2 => [1.6, 1.3, 0.35],
        3 => [1.1, 0.45, 1.6],
        4 => [0.45, 1.5, 0.5],
        _ => [1.2, 1.2, 1.6],
    }
}

pub struct Bolt {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    spell: usize,
    /// A monster's bolt (school, damage): it hurts the player instead of mobs.
    hostile: Option<(u8, f32)>,
}

/// The caster's mana and spells in flight.
pub struct Magic {
    pub mana: f32,
    pub max_mana: f32,
    /// Seconds since the last cast (mana comes back faster after a moment).
    since_cast: f32,
    cooldown: f32,
    bolts: Vec<Bolt>,
}

impl Magic {
    pub fn new(max_mana: f32) -> Magic {
        let max_mana = max_mana.clamp(20.0, 40.0);
        Magic { mana: max_mana, max_mana, since_cast: 10.0, cooldown: 0.0, bolts: Vec::new() }
    }
}

/// Is this item a spell caster?
pub fn is_caster(name: &str) -> bool {
    SPELLS.iter().any(|s| s.item == name)
}

impl Game {
    fn held_item_name(&self) -> Option<&str> {
        self.held_stack().map(|s| self.items.get(s.item).name.as_str())
    }

    pub(crate) fn glow_layer(&self) -> u16 {
        self.block_look(self.reg.id_or_air("white_wool")).0
    }

    /// Right click with a mana crystal: two more stars for good.
    pub(crate) fn use_mana_crystal(&mut self) -> bool {
        if self.magic.max_mana >= 40.0 {
            return false;
        }
        self.magic.max_mana = (self.magic.max_mana + 4.0).min(40.0);
        self.magic.mana = self.magic.max_mana;
        let sel = self.selected;
        self.surv.inv.take(sel, 1);
        self.sfx(Sfx::Cast(3));
        self.anim.start_swing();
        self.toast(format!("Mana máxima: {} estrelas", (self.magic.max_mana / 2.0) as i32), 3.0);
        true
    }

    /// Mana, casting while the use button is held, and the spells in flight.
    pub(crate) fn update_magic(&mut self, dt: f32, use_held: bool) {
        let m = &mut self.magic;
        m.since_cast += dt;
        m.cooldown -= dt;
        let regen = if m.since_cast > 1.2 { 4.0 } else { 0.6 };
        m.mana = (m.mana + regen * dt).min(m.max_mana);
        let casting = use_held && self.screen == crate::Screen::Playing;
        if casting {
            if let Some(i) = self.held_item_name().and_then(|n| SPELLS.iter().position(|s| s.item == n)) {
                self.try_cast(i);
            }
        }
        self.update_bolts(dt);
        // Burning mobs flicker with flames.
        if dt == 0.0 || self.surv.rng.f32() > dt * 12.0 {
            return;
        }
        let layer = self.glow_layer();
        let fire: Vec<Vec3> = self.mobs.list.iter().filter(|m| m.alive() && m.on_fire > 0.0).map(|m| m.body.center()).collect();
        for c in fire {
            self.particles.glow(c, Vec3::Y * 1.5, layer, color(0), 0.09, 0.4);
        }
    }

    fn try_cast(&mut self, i: usize) {
        let sp = &SPELLS[i];
        if self.magic.cooldown > 0.0 {
            return;
        }
        let free = !self.survival;
        if !free && self.magic.mana < sp.cost {
            return;
        }
        if !free {
            self.magic.mana -= sp.cost;
        }
        self.magic.cooldown = sp.cooldown;
        self.magic.since_cast = 0.0;
        let eye = self.body.eye();
        let fwd = self.forward();
        let right = fwd.cross(Vec3::Y).normalize_or_zero();
        let from = eye + fwd * 0.6 + right * 0.25 - Vec3::Y * 0.15;
        match sp.shot {
            Shot::Strike => self.strike(i, eye, fwd),
            Shot::Fan(n) => {
                for k in 0..n {
                    let a = (k as f32 - (n - 1) as f32 / 2.0) * 0.12;
                    let dir = (fwd * a.cos() + right * a.sin()).normalize();
                    self.magic.bolts.push(Bolt { pos: from, vel: dir * sp.speed, life: sp.life, spell: i, hostile: None });
                }
            }
            Shot::Flame => {
                let jitter = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5) * 0.15;
                self.magic.bolts.push(Bolt { pos: from, vel: (fwd + jitter).normalize() * sp.speed, life: sp.life, spell: i, hostile: None });
            }
            _ => self.magic.bolts.push(Bolt { pos: from, vel: fwd * sp.speed, life: sp.life, spell: i, hostile: None }),
        }
        // The flame book hisses on its own rhythm, not on every puff.
        if sp.shot != Shot::Flame || self.magic.since_cast == 0.0 && self.surv.rng.f32() < 0.25 {
            self.sfx(Sfx::Cast(sp.school));
        }
        self.anim.start_swing();
    }

    /// A shot from a mounted gun (the warship's cannon): a burst of fire that hurts mobs.
    pub(crate) fn cannon_shot(&mut self, from: Vec3, dir: Vec3) {
        let Some(i) = SPELLS.iter().position(|s| s.item == "@warship_cannon") else { return };
        let sp = &SPELLS[i];
        self.magic.bolts.push(Bolt { pos: from, vel: dir.normalize_or_zero() * sp.speed, life: sp.life, spell: i, hostile: None });
        self.sfx(Sfx::Explode);
    }

    /// A monster's bolt (bosses): flies straight and hurts the player.
    pub(crate) fn enemy_bolt(&mut self, from: Vec3, vel: Vec3, school: u8, damage: f32) {
        self.magic.bolts.push(Bolt { pos: from, vel, life: 3.0, spell: 0, hostile: Some((school, damage)) });
    }

    fn update_bolts(&mut self, dt: f32) {
        if self.magic.bolts.is_empty() || dt == 0.0 {
            return;
        }
        let layer = self.glow_layer();
        let mut bolts = std::mem::take(&mut self.magic.bolts);
        let mut hits: Vec<(Vec3, usize, Option<usize>)> = Vec::new();
        let mut player_hits: Vec<(Vec3, u8, f32)> = Vec::new();
        let me = self.body.pos;
        bolts.retain_mut(|b| {
            let sp = &SPELLS[b.spell];
            b.life -= dt;
            if let Some((school, dmg)) = b.hostile {
                if b.life <= 0.0 {
                    return false;
                }
                let step = b.vel * dt;
                let n = (step.length() / 0.25).ceil().max(1.0) as usize;
                for _ in 0..n {
                    b.pos += step / n as f32;
                    let d = b.pos - me;
                    if d.x.abs() < 0.45 && d.z.abs() < 0.45 && d.y > -0.1 && d.y < 1.9 {
                        player_hits.push((b.pos, school, dmg));
                        return false;
                    }
                    if self.world.solid(b.pos.floor().as_ivec3()) {
                        return false;
                    }
                }
                return true;
            }
            if b.life <= 0.0 {
                if matches!(sp.shot, Shot::Burst(_)) {
                    hits.push((b.pos, b.spell, None));
                }
                return false;
            }
            if sp.shot == Shot::Homing {
                if let Some(t) = self.nearest_mob(b.pos, b.vel.normalize_or_zero(), 14.0) {
                    let want = (t - b.pos).normalize_or_zero() * sp.speed;
                    b.vel = b.vel.lerp(want, (dt * 5.0).min(1.0));
                }
            }
            if let Shot::Burst(_) = sp.shot {
                b.vel.y -= 4.0 * dt;
            }
            let step = b.vel * dt;
            let n = (step.length() / 0.25).ceil().max(1.0) as usize;
            for _ in 0..n {
                b.pos += step / n as f32;
                if let Some(mi) = self.mob_at(b.pos) {
                    hits.push((b.pos, b.spell, Some(mi)));
                    return false;
                }
                if self.world.solid(b.pos.floor().as_ivec3()) {
                    hits.push((b.pos - step / n as f32, b.spell, None));
                    return false;
                }
            }
            true
        });
        // Trails.
        for b in &bolts {
            let sp = &SPELLS[b.spell];
            if let Some((school, _)) = b.hostile {
                self.particles.glow(b.pos, -b.vel * 0.05, layer, color(school), 0.1, 0.35);
                continue;
            }
            let (size, count) = match sp.shot {
                Shot::Burst(_) => (0.16, 3),
                Shot::Flame => (0.12, 1),
                _ => (0.08, 2),
            };
            for _ in 0..count {
                self.particles.glow(b.pos, -b.vel * 0.05, layer, color(sp.school), size, 0.35);
            }
        }
        self.magic.bolts = bolts;
        for (at, spell, mob) in hits {
            self.spell_hit(at, spell, mob);
        }
        for (at, school, dmg) in player_hits {
            self.damage(dmg, Some(at));
            self.sfx(Sfx::SpellHit(school));
        }
    }

    /// The alive mob whose body contains `p`.
    fn mob_at(&self, p: Vec3) -> Option<usize> {
        self.mobs.list.iter().position(|m| {
            if !m.alive() {
                return false;
            }
            let d = &self.mobs.reg.defs[m.def];
            let b = m.body.pos;
            let r = d.width * 0.5 + 0.15;
            (p.x - b.x).abs() < r && (p.z - b.z).abs() < r && p.y > b.y - 0.1 && p.y < b.y + d.height + 0.1
        })
    }

    /// Centre of the closest alive mob within `range` and roughly ahead of `dir`.
    fn nearest_mob(&self, from: Vec3, dir: Vec3, range: f32) -> Option<Vec3> {
        self.mobs
            .list
            .iter()
            .filter(|m| m.alive())
            .map(|m| m.body.center())
            .filter(|c| {
                let to = *c - from;
                to.length() < range && to.normalize_or_zero().dot(dir) > 0.3
            })
            .min_by(|a, b| a.distance(from).total_cmp(&b.distance(from)))
    }

    fn hurt_mob(&mut self, i: usize, dmg: f32, from: Vec3, school: u8) {
        let m = &mut self.mobs.list[i];
        m.hurt(dmg, from);
        match school {
            0 => m.on_fire = m.on_fire.max(4.0),
            1 => m.chill = m.chill.max(3.0),
            _ => {}
        }
        let def = &self.mobs.reg.defs[m.def];
        self.last_hit = Some((def.display_name.clone(), m.health.max(0.0), m.max_health, self.now + 3.0));
        if school == 4 && self.survival {
            // Nature magic gives a little life back.
            self.health = (self.health + 1.0).min(self.max_health);
        }
    }

    fn spell_hit(&mut self, at: Vec3, spell: usize, mob: Option<usize>) {
        let sp = &SPELLS[spell];
        let layer = self.glow_layer();
        let eye = self.body.eye();
        if let Shot::Burst(r) = sp.shot {
            let around: Vec<(usize, f32)> = self
                .mobs
                .list
                .iter()
                .enumerate()
                .filter(|(_, m)| m.alive())
                .map(|(k, m)| (k, m.body.center().distance(at)))
                .filter(|(_, d)| *d < r)
                .collect();
            for (k, d) in around {
                self.hurt_mob(k, sp.damage * (1.0 - d / r * 0.6), at, sp.school);
            }
            for _ in 0..40 {
                let dir = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.3, self.surv.rng.f32() - 0.5).normalize_or_zero();
                self.particles.glow(at, dir * 6.0, layer, color(sp.school), 0.14, 0.6);
            }
            self.sfx(Sfx::Explode);
            return;
        }
        if let Some(k) = mob {
            self.hurt_mob(k, sp.damage, eye, sp.school);
            self.sfx(Sfx::MobHit);
        }
        for _ in 0..8 {
            let dir = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5);
            self.particles.glow(at, dir * 3.0, layer, color(sp.school), 0.07, 0.35);
        }
        if sp.shot != Shot::Flame {
            self.sfx(Sfx::SpellHit(sp.school));
        }
    }

    /// Lightning onto the block or mob in sight (up to 32 blocks away).
    fn strike(&mut self, spell: usize, eye: Vec3, fwd: Vec3) {
        let sp = &SPELLS[spell];
        let mut at = eye + fwd * 32.0;
        let mut p = eye;
        for _ in 0..128 {
            p += fwd * 0.25;
            if self.mob_at(p).is_some() || self.world.solid(p.floor().as_ivec3()) {
                at = p;
                break;
            }
        }
        let around: Vec<usize> = self.mobs.list.iter().enumerate().filter(|(_, m)| m.alive() && m.body.center().distance(at) < 2.5).map(|(k, _)| k).collect();
        for k in around {
            self.hurt_mob(k, sp.damage, at + Vec3::Y * 8.0, sp.school);
        }
        let layer = self.glow_layer();
        let mut y = 0.0;
        let mut x = at;
        while y < 14.0 {
            // A jagged column of light down from the sky.
            x += Vec3::new(self.surv.rng.f32() - 0.5, 0.0, self.surv.rng.f32() - 0.5) * 0.35;
            self.particles.glow(x + Vec3::Y * y, Vec3::ZERO, layer, color(5), 0.13, 0.3);
            y += 0.3;
        }
        self.weather.flash = 1.0;
        self.sfx(Sfx::Thunder(1.0));
    }

    /// Mana stars down the right edge of the screen while a caster is in hand or mana is low.
    pub(crate) fn draw_mana(&mut self, w: f32) {
        let holding = self.held_item_name().is_some_and(is_caster);
        if !self.survival || (!holding && self.magic.mana >= self.magic.max_mana - 0.01) {
            return;
        }
        let s = self.ui.scale;
        let stars = (self.magic.max_mana / 2.0).ceil() as i32;
        for i in 0..stars {
            let col = i / 10;
            let row = i % 10;
            let x = w - (12.0 + col as f32 * 10.0) * s;
            let y = (6.0 + row as f32 * 9.0) * s;
            let v = self.magic.mana - i as f32 * 2.0;
            draw_star(&mut self.ui, x, y, s, v.clamp(0.0, 2.0) / 2.0);
        }
    }
}

fn draw_star(ui: &mut UiBatch, x: f32, y: f32, s: f32, fill: f32) {
    const ROWS: [&str; 7] = ["...#...", "..###..", "#######", ".#####.", "..###..", ".##.##.", "##...##"];
    for (r, row) in ROWS.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            if ch != '#' {
                continue;
            }
            // Mana drains from the top of each star down.
            let full = (7 - r) as f32 / 7.0 <= fill + 0.01;
            let col = if full { [70, 120, 255, 255] } else { [20, 24, 60, 220] };
            ui.rect(x + c as f32 * s, y + r as f32 * s, s, s, col);
        }
    }
}
