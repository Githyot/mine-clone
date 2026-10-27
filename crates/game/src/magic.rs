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
    /// Iron's Spells ideas (Etapa 5): jump to where you look.
    Blink(f32),
    /// Heal yourself.
    Heal,
    /// A ward around you that soaks up damage.
    Ward,
    /// Lightning that jumps from mob to mob.
    Chain(u8),
    /// A burning rock from the sky onto where you look.
    Meteor(f32),
    /// Spirits that circle you and strike nearby monsters.
    Spirits(u8),
    /// A ring of ice spikes flying out all around.
    Ring(u8),
    /// A slow dark orb that pulls monsters in.
    Vortex,
    /// A fan of homing stars.
    Stars(u8),
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
    SpellDef { item: "blink_scroll", school: 3, cost: 6.0, cooldown: 1.0, damage: 0.0, speed: 0.0, life: 0.0, shot: Shot::Blink(14.0) },
    SpellDef { item: "heal_scroll", school: 4, cost: 8.0, cooldown: 2.0, damage: 6.0, speed: 0.0, life: 0.0, shot: Shot::Heal },
    SpellDef { item: "ward_tome", school: 1, cost: 10.0, cooldown: 4.0, damage: 10.0, speed: 0.0, life: 0.0, shot: Shot::Ward },
    SpellDef { item: "chain_tome", school: 2, cost: 7.0, cooldown: 0.8, damage: 6.0, speed: 0.0, life: 0.0, shot: Shot::Chain(4) },
    SpellDef { item: "meteor_staff", school: 0, cost: 14.0, cooldown: 1.6, damage: 18.0, speed: 30.0, life: 3.0, shot: Shot::Meteor(4.5) },
    SpellDef { item: "spirit_staff", school: 3, cost: 12.0, cooldown: 3.0, damage: 4.0, speed: 0.0, life: 20.0, shot: Shot::Spirits(3) },
    SpellDef { item: "ice_spike_staff", school: 1, cost: 10.0, cooldown: 1.2, damage: 7.0, speed: 22.0, life: 0.8, shot: Shot::Ring(14) },
    SpellDef { item: "void_tome", school: 3, cost: 14.0, cooldown: 2.5, damage: 2.0, speed: 6.0, life: 4.0, shot: Shot::Vortex },
    SpellDef { item: "hive_staff", school: 4, cost: 5.0, cooldown: 0.35, damage: 4.0, speed: 16.0, life: 2.5, shot: Shot::Homing },
    SpellDef { item: "meteor_blaster", school: 0, cost: 4.0, cooldown: 0.22, damage: 4.0, speed: 36.0, life: 1.0, shot: Shot::Bolt },
    SpellDef { item: "colossus_staff", school: 1, cost: 9.0, cooldown: 0.8, damage: 6.0, speed: 26.0, life: 1.0, shot: Shot::Fan(5) },
    SpellDef { item: "star_tome", school: 5, cost: 9.0, cooldown: 0.5, damage: 9.0, speed: 22.0, life: 2.5, shot: Shot::Stars(4) },
];

/// What a caster does, for its tooltip.
pub fn describe(item: &str) -> Option<String> {
    let sp = SPELLS.iter().find(|s| s.item == item)?;
    let what = match sp.shot {
        Shot::Blink(r) => format!("Teleporta até {r:.0} blocos"),
        Shot::Heal => format!("Cura {:.0} de vida", sp.damage),
        Shot::Ward => format!("Escudo que absorve {:.0} de dano", sp.damage),
        Shot::Spirits(n) => format!("{n} espíritos lutam ao seu lado"),
        Shot::Vortex => "Um vórtice que puxa os monstros".to_string(),
        _ => format!("Dano: {:.0}", sp.damage),
    };
    Some(format!("{what} · Mana: {:.0}", sp.cost))
}

/// A spirit of the spirit staff: it circles the player and darts at monsters close by.
pub struct Spirit {
    angle: f32,
    life: f32,
    cd: f32,
    damage: f32,
}

/// The void book's orb: drifts slowly, pulls monsters in and hurts the ones at its heart.
pub struct Vortex {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    tick: f32,
    damage: f32,
}

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
    /// Stars the star amulet adds on top (Etapa 5).
    pub extra_max: f32,
    /// Damage multiplier of the spell being cast (modifier, arcane emblem).
    pub power: f32,
    /// Seconds since the last cast (mana comes back faster after a moment).
    since_cast: f32,
    cooldown: f32,
    bolts: Vec<Bolt>,
}

impl Magic {
    /// Test pictures: no bolts in front of the camera.
    pub(crate) fn clear_bolts_for_test(&mut self) {
        self.bolts.clear();
    }

    pub fn new(max_mana: f32) -> Magic {
        let max_mana = max_mana.clamp(20.0, 40.0);
        Magic { mana: max_mana, max_mana, extra_max: 0.0, power: 1.0, since_cast: 10.0, cooldown: 0.0, bolts: Vec::new() }
    }
}

/// Is this item a spell caster?
pub fn is_caster(name: &str) -> bool {
    SPELLS.iter().any(|s| s.item == name) || crate::mods::spellbooks::is_book(name)
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
        m.mana = (m.mana + regen * dt).min(m.max_mana + m.extra_max);
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
        if self.has_effect(crate::effects::SILENCED) {
            self.magic.cooldown = 0.5;
            self.toast("Silenciado: não dá para lançar magias", 1.0);
            return;
        }
        let (power, mana_k, speed_k) = self.caster_mods();
        let mut cost = sp.cost * mana_k * self.t5.rpg.stats.mana_cost;
        if sp.item == "meteor_blaster" && self.t5.rpg.stats.set.is_some_and(|s| crate::rpg::SETS[s].prefix == "meteor") {
            cost = 0.0;
        }
        let free = !self.survival;
        // The mana blossom drinks a potion when mana runs out (rpg.rs).
        if !free && self.magic.mana < cost && self.wears("mana_blossom") {
            self.quick_mana();
        }
        if !free && self.magic.mana < cost {
            return;
        }
        if !free {
            self.magic.mana -= cost;
        }
        self.magic.power = power;
        self.magic.cooldown = sp.cooldown / speed_k;
        if self.cast_extra(i) {
            self.magic.since_cast = 0.0;
            self.sfx(Sfx::Cast(sp.school));
            self.anim.start_swing();
            return;
        }
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
                if matches!(sp.shot, Shot::Burst(_) | Shot::Meteor(_)) {
                    hits.push((b.pos, b.spell, None));
                }
                return false;
            }
            if matches!(sp.shot, Shot::Homing | Shot::Stars(_)) {
                if let Some(t) = self.nearest_mob(b.pos, b.vel.normalize_or_zero(), 14.0) {
                    let want = (t - b.pos).normalize_or_zero() * sp.speed;
                    b.vel = b.vel.lerp(want, (dt * 5.0).min(1.0));
                }
            }
            if let Shot::Burst(_) = sp.shot {
                b.vel.y -= 4.0 * dt;
            }
            if let Shot::Meteor(_) = sp.shot {
                // Burns as it falls.
                b.vel.y -= 10.0 * dt;
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
                Shot::Meteor(_) => (0.3, 5),
                Shot::Stars(_) => (0.11, 2),
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
    pub(crate) fn mob_at(&self, p: Vec3) -> Option<usize> {
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
    pub(crate) fn nearest_mob(&self, from: Vec3, dir: Vec3, range: f32) -> Option<Vec3> {
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

    pub(crate) fn hurt_mob(&mut self, i: usize, dmg: f32, from: Vec3, school: u8) {
        let mut dmg = dmg * self.magic.power;
        // Magic critical hits (rpg.rs).
        if self.surv.rng.f32() < self.t5.rpg.stats.crit[crate::rpg::MAGIC] {
            dmg *= 1.5;
        }
        self.orichalcum_petals(i, dmg);
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
        if let Shot::Burst(r) | Shot::Meteor(r) = sp.shot {
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
        let holding = holding || self.held_item_name() == Some("glyph_book");
        if !self.survival || (!holding && self.magic.mana >= self.magic.max_mana + self.magic.extra_max - 0.01) {
            return;
        }
        let s = self.ui.scale;
        let stars = ((self.magic.max_mana + self.magic.extra_max) / 2.0).ceil() as i32;
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

// ---------------------------------------------------------------------------------------------
// Etapa 5: the spells of the Iron's Spells idea that are not plain bolts.

impl Game {
    /// Where the player looks, up to `range` blocks: the first mob or block in the way.
    pub(crate) fn look_point(&self, range: f32) -> Vec3 {
        let eye = self.body.eye();
        let fwd = self.forward();
        let mut p = eye;
        let mut t = 0.0;
        while t < range {
            p += fwd * 0.25;
            t += 0.25;
            if self.mob_at(p).is_some() || self.world.solid(p.floor().as_ivec3()) {
                return p - fwd * 0.25;
            }
        }
        p
    }

    /// Casts the spell if it is one of the new kinds; false leaves it to the bolt code.
    fn cast_extra(&mut self, i: usize) -> bool {
        let sp = &SPELLS[i];
        let eye = self.body.eye();
        let fwd = self.forward();
        let layer = self.glow_layer();
        let col = color(sp.school);
        match sp.shot {
            Shot::Blink(range) => {
                let mut to = self.look_point(range);
                // Land on your feet in open air.
                let mut k = 0;
                while k < 8 && (self.world.solid(to.floor().as_ivec3()) || self.world.solid((to + Vec3::Y).floor().as_ivec3())) {
                    to += Vec3::Y;
                    k += 1;
                }
                for _ in 0..20 {
                    let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5) * 3.0;
                    self.particles.glow(self.body.pos + Vec3::Y, v, layer, col, 0.1, 0.5);
                    self.particles.glow(to + Vec3::Y, v, layer, col, 0.1, 0.5);
                }
                self.body.pos = Vec3::new(to.x, to.y.floor(), to.z);
                self.body.vel = Vec3::ZERO;
                self.fall_peak = self.body.pos.y;
            }
            Shot::Heal => {
                self.health = (self.health + sp.damage * self.magic.power).min(self.max_health);
                for _ in 0..16 {
                    let v = Vec3::new(self.surv.rng.f32() - 0.5, 1.0, self.surv.rng.f32() - 0.5) * 2.0;
                    self.particles.glow(self.body.pos + Vec3::Y * 0.5, v, layer, col, 0.09, 0.8);
                }
            }
            Shot::Ward => {
                self.t5.ward = sp.damage * self.magic.power;
                self.t5.ward_t = 12.0;
            }
            Shot::Chain(n) => {
                let mut from = eye;
                let mut dir = fwd;
                let mut hit: Vec<usize> = Vec::new();
                for _ in 0..n {
                    let Some(k) = self
                        .mobs
                        .list
                        .iter()
                        .enumerate()
                        .filter(|(k, m)| m.alive() && !hit.contains(k) && !self.mobs.reg.defs[m.def].town)
                        .map(|(k, m)| (k, m.body.center()))
                        .filter(|(_, c)| {
                            let to = *c - from;
                            to.length() < if hit.is_empty() { 16.0 } else { 7.0 } && (hit.len() > 0 || to.normalize_or_zero().dot(dir) > 0.8)
                        })
                        .min_by(|a, b| a.1.distance(from).total_cmp(&b.1.distance(from)))
                        .map(|(k, _)| k)
                    else {
                        break;
                    };
                    let to = self.mobs.list[k].body.center();
                    let seg = to - from;
                    let steps = (seg.length() / 0.3).ceil().max(1.0) as usize;
                    for s in 0..=steps {
                        let jitter = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5) * 0.25;
                        self.particles.glow(from + seg * (s as f32 / steps as f32) + jitter, Vec3::ZERO, layer, col, 0.07, 0.25);
                    }
                    self.hurt_mob(k, sp.damage, from, sp.school);
                    hit.push(k);
                    dir = seg.normalize_or_zero();
                    from = to;
                }
                if hit.is_empty() {
                    return false;
                }
                self.sfx(Sfx::SpellHit(2));
            }
            Shot::Meteor(_) => {
                let at = self.look_point(48.0);
                let flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
                let from = at - flat * 8.0 + Vec3::Y * 26.0;
                let vel = (at - from).normalize_or_zero() * sp.speed;
                self.magic.bolts.push(Bolt { pos: from, vel, life: sp.life, spell: i, hostile: None });
            }
            Shot::Spirits(n) => {
                self.t5.spirits.clear();
                for k in 0..n {
                    self.t5.spirits.push(Spirit { angle: k as f32 / n as f32 * std::f32::consts::TAU, life: sp.life, cd: 0.5 + k as f32 * 0.3, damage: sp.damage * self.magic.power });
                }
            }
            Shot::Ring(n) => {
                let from = self.body.pos + Vec3::Y * 1.0;
                for k in 0..n {
                    let a = k as f32 / n as f32 * std::f32::consts::TAU;
                    let dir = Vec3::new(a.cos(), 0.0, a.sin());
                    self.magic.bolts.push(Bolt { pos: from + dir * 0.6, vel: dir * sp.speed, life: sp.life, spell: i, hostile: None });
                }
            }
            Shot::Vortex => {
                self.t5.vortexes.push(Vortex { pos: eye + fwd, vel: fwd * sp.speed, life: sp.life, tick: 0.0, damage: sp.damage * self.magic.power });
            }
            Shot::Stars(n) => {
                let right = fwd.cross(Vec3::Y).normalize_or_zero();
                for k in 0..n {
                    let a = (k as f32 - (n - 1) as f32 / 2.0) * 0.25;
                    let dir = (fwd * a.cos() + right * a.sin() + Vec3::Y * 0.1).normalize();
                    self.magic.bolts.push(Bolt { pos: eye + fwd * 0.6, vel: dir * sp.speed, life: sp.life, spell: i, hostile: None });
                }
            }
            _ => return false,
        }
        true
    }

    /// Spirits, vortexes and the ward, each frame.
    pub(crate) fn update_spell_extras(&mut self, dt: f32) {
        if dt == 0.0 {
            return;
        }
        let layer = self.glow_layer();
        if self.t5.ward > 0.0 {
            self.t5.ward_t -= dt;
            if self.t5.ward_t <= 0.0 {
                self.t5.ward = 0.0;
            } else if self.surv.rng.f32() < dt * 10.0 {
                let a = self.surv.rng.f32() * std::f32::consts::TAU;
                let p = self.body.pos + Vec3::new(a.cos() * 0.7, 0.2 + self.surv.rng.f32() * 1.6, a.sin() * 0.7);
                self.particles.glow(p, Vec3::ZERO, layer, [0.6, 0.85, 1.5], 0.06, 0.4);
            }
        }
        // Spirits: circle, then dart at the closest monster in reach.
        let me = self.body.pos + Vec3::Y * 1.4;
        let mut spirits = std::mem::take(&mut self.t5.spirits);
        for s in spirits.iter_mut() {
            s.life -= dt;
            s.angle += dt * 2.2;
            s.cd -= dt;
            let at = me + Vec3::new(s.angle.cos() * 1.6, (s.angle * 2.0).sin() * 0.3, s.angle.sin() * 1.6);
            self.particles.glow(at, Vec3::ZERO, layer, color(3), 0.12, 0.08);
            if s.cd <= 0.0 {
                s.cd = 0.9;
                let target = self
                    .mobs
                    .list
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| m.alive() && self.mobs.reg.defs[m.def].kind == voxel::mob::Kind::Hostile)
                    .map(|(k, m)| (k, m.body.center()))
                    .filter(|(_, c)| c.distance(at) < 8.0)
                    .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)));
                if let Some((k, c)) = target {
                    let seg = c - at;
                    for j in 0..8 {
                        self.particles.glow(at + seg * (j as f32 / 8.0), Vec3::ZERO, layer, color(3), 0.06, 0.2);
                    }
                    let power = self.magic.power;
                    self.magic.power = 1.0;
                    self.hurt_mob(k, s.damage, at, 3);
                    self.magic.power = power;
                }
            }
        }
        spirits.retain(|s| s.life > 0.0);
        self.t5.spirits = spirits;
        // Vortexes.
        let mut vs = std::mem::take(&mut self.t5.vortexes);
        for v in vs.iter_mut() {
            v.life -= dt;
            v.vel *= 1.0 - (dt * 0.6).min(1.0);
            let next = v.pos + v.vel * dt;
            if !self.world.solid(next.floor().as_ivec3()) {
                v.pos = next;
            }
            for _ in 0..3 {
                let a = self.surv.rng.f32() * std::f32::consts::TAU;
                let r = 0.4 + self.surv.rng.f32() * 1.6;
                let p = v.pos + Vec3::new(a.cos() * r, (self.surv.rng.f32() - 0.5) * 0.8, a.sin() * r);
                let swirl = Vec3::new(-a.sin(), 0.0, a.cos()) * 3.0 + (v.pos - p) * 1.5;
                self.particles.glow(p, swirl, layer, [0.5, 0.2, 0.9], 0.1, 0.3);
            }
            v.tick -= dt;
            let hit = v.tick <= 0.0;
            if hit {
                v.tick = 0.4;
            }
            for k in 0..self.mobs.list.len() {
                let m = &self.mobs.list[k];
                if !m.alive() || self.mobs.reg.defs[m.def].kind != voxel::mob::Kind::Hostile || self.mobs.reg.defs[m.def].boss {
                    continue;
                }
                let to = v.pos - m.body.center();
                let d = to.length();
                if d < 7.0 {
                    let m = &mut self.mobs.list[k];
                    m.body.vel += to.normalize_or_zero() * dt * 30.0;
                    if hit && d < 2.2 {
                        let power = self.magic.power;
                        self.magic.power = 1.0;
                        self.hurt_mob(k, v.damage, v.pos, 3);
                        self.magic.power = power;
                    }
                }
            }
        }
        vs.retain(|v| v.life > 0.0);
        self.t5.vortexes = vs;
    }
}
