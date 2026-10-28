//! Lote 4 of the Terraria side (plan/terraria-completo.md): weapons that fly.
//! - Yoyos: hold the use button and the yoyo spins out to where you aim, biting what it
//!   touches; let go and it comes back.
//! - Boomerangs and flails: thrown out, they turn around and come back to your hand.
//! - Guns: fire musket balls (or meteor shot) from the inventory; the shotgun spreads.
//! - Throwables: shuriken and knives arc and are used up; bombs and dynamite blow up the
//!   ground, grenades only hurt.
//! - The ballista rod sets down a sentry that shoots at monsters for two minutes.
//! - The enchanted sword sends a beam with each swing; the star saber calls stars down.
//! Ore bows are plain bows that hit harder (weapons.rs asks `bow_power`).

use glam::{IVec3, Vec3};
use voxel::physics::Terrain;

use crate::audio::{Material, Sfx};
use crate::effects::{BLEEDING, CHILLED, POISON};
use crate::item_render::GpuItem;
use crate::rpg::{MELEE, MOB_BURN, RANGED, SUMMON};
use crate::Game;

#[derive(Clone, Copy, PartialEq)]
enum Wk {
    /// Reach in blocks.
    Yoyo(f32),
    /// Seconds out before it turns around.
    Boomerang(f32),
    Flail(f32),
    /// Pellets per shot and their spread.
    Gun(u8, f32),
    /// Used up; gravity.
    Throw(f32),
    /// Blast size; breaks blocks; fuse.
    Bomb(f32, bool, f32),
    Sentry,
    /// Swung: a beam (false) or a star from the sky (true).
    Beam(bool),
}

struct WDef {
    item: &'static str,
    kind: Wk,
    dmg: f32,
    cd: f32,
    speed: f32,
    debuff: Option<(u8, f32)>,
}

const fn w(item: &'static str, kind: Wk, dmg: f32, cd: f32, speed: f32, debuff: Option<(u8, f32)>) -> WDef {
    WDef { item, kind, dmg, cd, speed, debuff }
}

const WEAPONS: &[WDef] = &[
    w("wood_yoyo", Wk::Yoyo(6.0), 2.4, 0.0, 22.0, None),
    w("jungle_yoyo", Wk::Yoyo(7.0), 3.4, 0.0, 24.0, Some((POISON, 4.0))),
    w("umbral_yoyo", Wk::Yoyo(8.0), 3.6, 0.0, 26.0, None),
    w("sanguinite_yoyo", Wk::Yoyo(8.0), 3.6, 0.0, 26.0, Some((BLEEDING, 4.0))),
    w("magma_yoyo", Wk::Yoyo(9.0), 5.4, 0.0, 28.0, Some((MOB_BURN, 4.0))),
    w("wood_boomerang", Wk::Boomerang(0.45), 3.0, 0.3, 18.0, None),
    w("enchanted_boomerang", Wk::Boomerang(0.5), 5.0, 0.3, 22.0, None),
    w("ice_boomerang", Wk::Boomerang(0.5), 5.0, 0.3, 22.0, Some((CHILLED, 3.0))),
    w("thorn_chakram", Wk::Boomerang(0.55), 7.0, 0.3, 24.0, Some((POISON, 5.0))),
    w("flame_boomerang", Wk::Boomerang(0.6), 8.0, 0.3, 26.0, Some((MOB_BURN, 4.0))),
    w("iron_flail", Wk::Flail(0.32), 6.0, 0.4, 22.0, None),
    w("umbral_flail", Wk::Flail(0.36), 9.0, 0.4, 24.0, None),
    w("sanguinite_flail", Wk::Flail(0.36), 9.0, 0.4, 24.0, Some((BLEEDING, 4.0))),
    w("bone_flail", Wk::Flail(0.36), 8.0, 0.4, 24.0, None),
    w("sun_flail", Wk::Flail(0.4), 12.0, 0.4, 26.0, Some((MOB_BURN, 5.0))),
    w("flintlock_pistol", Wk::Gun(1, 0.0), 5.0, 0.55, 70.0, None),
    w("musket", Wk::Gun(1, 0.0), 10.0, 0.9, 80.0, None),
    w("boomstick", Wk::Gun(4, 0.12), 4.0, 1.0, 60.0, None),
    w("rapid_gun", Wk::Gun(1, 0.035), 1.9, 0.11, 70.0, None),
    w("shuriken", Wk::Throw(6.0), 2.4, 0.25, 26.0, None),
    w("throwing_knife", Wk::Throw(10.0), 3.0, 0.3, 24.0, None),
    w("javelin", Wk::Throw(7.0), 4.5, 0.45, 26.0, None),
    w("bone_javelin", Wk::Throw(7.0), 5.0, 0.45, 26.0, Some((BLEEDING, 3.0))),
    w("sun_javelin", Wk::Throw(5.0), 21.0, 0.35, 32.0, Some((MOB_BURN, 5.0))),
    w("bomb", Wk::Bomb(3.0, true, 2.5), 10.0, 0.8, 12.0, None),
    w("grenade", Wk::Bomb(2.5, false, 1.6), 6.0, 0.6, 16.0, None),
    w("dynamite", Wk::Bomb(5.0, true, 4.0), 20.0, 1.0, 10.0, None),
    w("ballista_rod", Wk::Sentry, 5.0, 1.0, 40.0, None),
    w("enchanted_sword", Wk::Beam(false), 2.0, 1.0, 30.0, None),
    w("star_saber", Wk::Beam(true), 2.5, 1.2, 40.0, None),
    // Lote 7: the last bosses' and the pillars' weapons.
    w("crystal_yoyo", Wk::Yoyo(11.0), 7.0, 0.0, 30.0, None),
    w("stone_fist", Wk::Flail(0.45), 30.0, 0.4, 28.0, None),
    w("tide_boomerang", Wk::Boomerang(0.6), 30.0, 0.25, 30.0, Some((CHILLED, 4.0))),
    w("bubble_rifle", Wk::Gun(3, 0.08), 5.7, 0.35, 60.0, None),
    w("prism_saber", Wk::Beam(true), 14.0, 0.8, 44.0, None),
    w("solar_eruption", Wk::Flail(0.5), 50.0, 0.35, 32.0, Some((MOB_BURN, 6.0))),
    w("vortex_beater", Wk::Gun(1, 0.03), 4.7, 0.09, 90.0, None),
    w("nebula_blade", Wk::Beam(false), 18.0, 0.62, 40.0, None),
    w("stardust_yoyo", Wk::Yoyo(14.0), 15.5, 0.0, 34.0, None),
    // Lote 14 (second round): the late swords and more sentries.
    w("night_edge", Wk::Beam(false), 3.0, 1.0, 32.0, None),
    w("radiant_blade", Wk::Beam(false), 8.0, 0.7, 34.0, None),
    w("true_night_edge", Wk::Beam(false), 10.0, 0.95, 36.0, Some((POISON, 3.0))),
    w("true_radiant_blade", Wk::Beam(false), 10.0, 1.0, 36.0, None),
    w("terra_blade", Wk::Beam(false), 9.0, 1.2, 44.0, None),
    w("celestial_wrath", Wk::Beam(true), 16.0, 1.0, 50.0, None),
    w("apex_blade", Wk::Beam(true), 16.0, 1.3, 56.0, None),
    w("explosive_trap_rod", Wk::Sentry, 16.0, 2.0, 30.0, Some((MOB_BURN, 3.0))),
    w("lightning_aura_rod", Wk::Sentry, 4.0, 0.35, 60.0, None),
    w("flame_sentry_rod", Wk::Sentry, 7.0, 0.8, 36.0, Some((MOB_BURN, 4.0))),
    // The balance sheet (plan/balanceamento.md): a gun and knives for the stages that had none.
    w("adamantite_rifle", Wk::Gun(1, 0.0), 9.0, 0.36, 80.0, None),
    w("laser_rifle", Wk::Gun(1, 0.0), 12.0, 0.33, 90.0, None),
    w("poison_knife", Wk::Throw(10.0), 3.8, 0.3, 24.0, Some((POISON, 4.0))),
    w("magmarite_knife", Wk::Throw(10.0), 5.8, 0.3, 26.0, Some((MOB_BURN, 3.0))),
    w("mythril_javelin", Wk::Throw(7.0), 13.0, 0.45, 28.0, None),
    w("verdite_knife", Wk::Throw(10.0), 13.0, 0.3, 28.0, None),
];

/// The colour of a sword's beam.
fn beam_tint(item: &str) -> [f32; 3] {
    match item {
        "night_edge" => [1.2, 0.5, 2.0],
        "radiant_blade" | "true_radiant_blade" => [2.2, 1.8, 0.7],
        "true_night_edge" => [0.8, 1.8, 1.0],
        "terra_blade" => [0.6, 2.2, 0.7],
        "celestial_wrath" => [2.0, 0.8, 2.0],
        "apex_blade" => [1.2, 1.8, 2.4],
        "star_saber" | "prism_saber" => [2.0, 1.8, 0.8],
        _ => [0.7, 1.1, 2.2],
    }
}

/// How much harder an ore bow hits than the plain bow (None: not one of them).
pub fn bow_power(name: &str) -> Option<f32> {
    match name {
        "copper_bow" => Some(1.08),
        "silver_bow" => Some(1.15),
        "platinum_bow" => Some(1.22),
        "umbral_bow" | "sanguinite_bow" => Some(1.65),
        "magmarite_bow" => Some(2.44),
        "cobalt_bow" => Some(2.75),
        "mythril_bow" => Some(3.1),
        "adamantite_bow" => Some(3.57),
        "radiant_bow" => Some(4.6),
        "verdite_bow" => Some(5.29),
        "lumite_bow" => Some(7.65),
        _ => None,
    }
}

/// Damage per second on one monster and the class it counts for, for the balance sheet
/// (balance.rs): a yoyo bites every 0.3 s, a boomerang or flail lands about one and a half
/// hits a throw, a spread gun puts three quarters of its pellets on one monster, a sentry
/// rod's turret shoots on its own, and a beam sword's beams come on top of its swings.
#[cfg(test)]
pub(crate) fn sustained_dps(name: &str) -> Option<(usize, f32)> {
    let d = WEAPONS.iter().find(|w| w.item == name)?;
    Some(match d.kind {
        Wk::Yoyo(_) => (MELEE, d.dmg / 0.3),
        Wk::Boomerang(t) | Wk::Flail(t) => (MELEE, 1.5 * d.dmg / (2.0 * t + d.cd)),
        Wk::Gun(n, spread) => (RANGED, d.dmg * n as f32 * if spread > 0.05 { 0.75 } else { 1.0 } / d.cd),
        Wk::Throw(_) | Wk::Bomb(..) => (RANGED, d.dmg / d.cd),
        Wk::Sentry => (SUMMON, d.dmg / d.cd),
        Wk::Beam(_) => (MELEE, d.dmg / d.cd),
    })
}

/// Every weapon of this file, for balance.rs to check that each one has a stage.
#[cfg(test)]
pub(crate) fn all_weapons() -> impl Iterator<Item = &'static str> {
    WEAPONS.iter().map(|w| w.item)
}

pub fn is_weapon(name: &str) -> bool {
    WEAPONS.iter().any(|w| w.item == name)
}

#[derive(Clone, Copy, PartialEq)]
enum Fly {
    Yoyo,
    Boomerang,
    Flail,
    Bullet,
    Thrown,
    Bomb,
    Sentry,
    Bolt,
}

struct TrShot {
    fly: Fly,
    def: usize,
    pos: Vec3,
    vel: Vec3,
    age: f32,
    life: f32,
    dmg: f32,
    class: usize,
    pierce: u8,
    /// Mobs hit lately and seconds until each can be hit again.
    hits: Vec<(usize, f32)>,
    back: bool,
    tint: [f32; 3],
    /// Sentries: seconds to the next shot.
    shot_t: f32,
}

#[derive(Default)]
pub struct Weapons {
    shots: Vec<TrShot>,
    cd: f32,
    swing_cd: f32,
    was_held: bool,
}

fn def_of(name: &str) -> Option<usize> {
    WEAPONS.iter().position(|w| w.item == name)
}

/// A sword swung for its beam (terra blade, night's edge...): held by the hilt like a tool.
pub(crate) fn is_blade(name: &str) -> bool {
    def_of(name).is_some_and(|i| matches!(WEAPONS[i].kind, Wk::Beam(_)))
}

impl Game {
    /// Right click with one of these weapons: say we handle it (the firing is in the update).
    pub(crate) fn tr_weapon_use(&mut self, name: &str) -> Option<bool> {
        let i = def_of(name)?;
        if let Wk::Beam(_) = WEAPONS[i].kind {
            return None;
        }
        Some(true)
    }

    pub(crate) fn update_tr_weapons(&mut self, dt: f32, use_held: bool) {
        let wp = &mut self.t5.tr.wep;
        wp.cd -= dt;
        wp.swing_cd -= dt;
        let name = self.held_key();
        let held = def_of(&name);
        let playing = self.screen == crate::Screen::Playing;
        let pressed = use_held && playing;
        if let Some(i) = held {
            let d = &WEAPONS[i];
            match d.kind {
                Wk::Yoyo(_) => {
                    if pressed && !self.t5.tr.wep.shots.iter().any(|s| s.fly == Fly::Yoyo && !s.back) {
                        let from = self.body.eye() + self.forward() * 0.6;
                        self.add_shot(Fly::Yoyo, i, from, self.forward() * d.speed, 999.0, MELEE);
                        self.sfx(Sfx::Place(Material::Cloth));
                    }
                }
                Wk::Beam(star) => {
                    if self.mouse_held[0] && playing && self.t5.tr.wep.swing_cd <= 0.0 {
                        self.t5.tr.wep.swing_cd = d.cd;
                        self.fire_beam(i, star);
                    }
                }
                _ => {
                    if pressed && self.t5.tr.wep.cd <= 0.0 {
                        self.t5.tr.wep.cd = d.cd;
                        self.fire_weapon(i);
                    }
                }
            }
        }
        // Yoyos come back when the button is let go or the yoyo is put away.
        let yoyo_held = held.is_some_and(|i| matches!(WEAPONS[i].kind, Wk::Yoyo(_)));
        if !pressed || !yoyo_held {
            for s in self.t5.tr.wep.shots.iter_mut().filter(|s| s.fly == Fly::Yoyo) {
                s.back = true;
            }
        }
        self.t5.tr.wep.was_held = pressed;
        self.update_tr_shots(dt);
    }

    fn add_shot(&mut self, fly: Fly, def: usize, pos: Vec3, vel: Vec3, life: f32, class: usize) {
        let d = &WEAPONS[def];
        let tint = match d.debuff {
            Some((MOB_BURN, _)) => [2.0, 0.9, 0.3],
            Some((CHILLED, _)) => [0.6, 1.2, 2.0],
            Some((POISON, _)) => [0.6, 1.8, 0.5],
            Some((BLEEDING, _)) => [1.8, 0.3, 0.3],
            _ => [1.6, 1.4, 1.0],
        };
        self.t5.tr.wep.shots.push(TrShot { fly, def, pos, vel, age: 0.0, life, dmg: d.dmg, class, pierce: 0, hits: Vec::new(), back: false, tint, shot_t: 0.5 });
    }

    /// The first stack of `names` in the inventory: (slot, which name).
    fn find_ammo(&self, names: &[&str]) -> Option<(usize, usize)> {
        for (slot, s) in self.surv.inv.slots.iter().enumerate() {
            let Some(s) = s else { continue };
            let n = &self.items.get(s.item).name;
            if let Some(k) = names.iter().position(|a| a == n) {
                return Some((slot, k));
            }
        }
        None
    }

    fn fire_weapon(&mut self, i: usize) {
        let d = &WEAPONS[i];
        let f = self.forward();
        let eye = self.body.eye();
        let out = self.t5.tr.wep.shots.iter().filter(|s| s.def == i).count();
        match d.kind {
            Wk::Boomerang(_) | Wk::Flail(_) => {
                if out > 0 {
                    return;
                }
                let fly = if matches!(d.kind, Wk::Flail(_)) { Fly::Flail } else { Fly::Boomerang };
                self.add_shot(fly, i, eye + f * 0.6, f * d.speed, 6.0, MELEE);
                self.anim.start_swing();
                self.sfx(Sfx::Place(Material::Cloth));
            }
            Wk::Gun(pellets, spread) => {
                let ammo = if self.survival { self.find_ammo(&["meteor_shot", "musket_ball"]) } else { Some((usize::MAX, 1)) };
                let Some((slot, kind)) = ammo else {
                    self.toast("Sem munição (balas de mosquete)", 1.5);
                    self.t5.tr.wep.cd = 0.6;
                    return;
                };
                let saved = self.t5.rpg.stats.ammo_save > 0.0 && self.surv.rng.f32() < self.t5.rpg.stats.ammo_save;
                if slot != usize::MAX && !saved {
                    self.surv.inv.take(slot, 1);
                }
                for _ in 0..pellets {
                    let r = &mut self.surv.rng;
                    let j = Vec3::new(r.f32() - 0.5, r.f32() - 0.5, r.f32() - 0.5) * 2.0 * spread;
                    let dir = (f + j).normalize_or_zero();
                    self.add_shot(Fly::Bullet, i, eye + dir * 0.5 - Vec3::Y * 0.1, dir * d.speed, 1.2, RANGED);
                    if let Some(s) = self.t5.tr.wep.shots.last_mut() {
                        if kind == 0 {
                            s.pierce = 2;
                            s.dmg += 2.0;
                            s.tint = [2.0, 0.9, 0.3];
                        } else {
                            s.tint = [1.8, 1.6, 1.0];
                        }
                    }
                }
                let glow = self.glow_layer();
                self.particles.glow(eye + f * 0.9 - Vec3::Y * 0.15, f * 2.0, glow, [2.0, 1.4, 0.6], 0.12, 0.08);
                self.sfx(Sfx::Piston(true));
                self.anim.start_swing();
            }
            Wk::Throw(_) | Wk::Bomb(..) => {
                if self.survival {
                    let sel = self.selected;
                    self.surv.inv.take(sel, 1);
                }
                let (fly, class, life) = if let Wk::Bomb(_, _, fuse) = d.kind { (Fly::Bomb, RANGED, fuse) } else { (Fly::Thrown, RANGED, 4.0) };
                let up = if fly == Fly::Bomb { Vec3::Y * 3.0 } else { Vec3::Y * 0.5 };
                self.add_shot(fly, i, eye + f * 0.5, f * d.speed + up + self.body.vel * 0.5, life, class);
                self.anim.start_swing();
                self.sfx(Sfx::Place(Material::Cloth));
            }
            Wk::Sentry => {
                // One sentry of each kind: the old one goes when a new one is set down.
                self.t5.tr.wep.shots.retain(|s| !(s.fly == Fly::Sentry && s.def == i));
                let ahead = self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 2.0;
                let y = self.world.surface_y(ahead.x.floor() as i32, ahead.z.floor() as i32).map_or(ahead.y, |y| (y as f32 + 1.0).min(self.body.pos.y + 3.0));
                self.add_shot(Fly::Sentry, i, Vec3::new(ahead.x, y + 0.6, ahead.z), Vec3::ZERO, 120.0, SUMMON);
                self.anim.start_swing();
                self.sfx(Sfx::Place(Material::Wood));
                let label = format!("{} (2 minutos)", self.items.id(d.item).map(|id| self.items.get(id).display_name.clone()).unwrap_or_default());
                self.toast(label, 1.5);
            }
            _ => {}
        }
    }

    fn fire_beam(&mut self, i: usize, star: bool) {
        let d = &WEAPONS[i];
        let f = self.forward();
        if star {
            // A star falls from the sky onto what you look at.
            let target = self.aim_point(24.0);
            let r = &mut self.surv.rng;
            let from = target + Vec3::new(r.f32() * 8.0 - 4.0, 22.0, r.f32() * 8.0 - 4.0);
            self.add_shot(Fly::Bolt, i, from, (target - from).normalize_or_zero() * d.speed, 1.5, MELEE);
            if let Some(s) = self.t5.tr.wep.shots.last_mut() {
                s.tint = beam_tint(d.item);
                s.pierce = 2;
            }
        } else {
            self.add_shot(Fly::Bolt, i, self.body.eye() + f * 0.8, f * d.speed, 0.7, MELEE);
            if let Some(s) = self.t5.tr.wep.shots.last_mut() {
                s.tint = beam_tint(d.item);
                s.pierce = 3;
            }
        }
        self.sfx(Sfx::Cast(1));
    }

    /// Where the player is looking, up to `range` blocks (the first solid block).
    fn aim_point(&self, range: f32) -> Vec3 {
        let eye = self.body.eye();
        let f = self.forward();
        let mut t = 0.0;
        while t < range {
            let p = eye + f * t;
            if self.world.solid(p.floor().as_ivec3()) {
                return eye + f * (t - 0.3).max(0.0);
            }
            t += 0.25;
        }
        eye + f * range
    }

    fn tr_hit(&mut self, k: usize, dmg: f32, class: usize, from: Vec3, debuff: Option<(u8, f32)>) {
        let st = &self.t5.rpg.stats;
        let mut d = dmg * st.dmg[class];
        if self.surv.rng.f32() < st.crit[class] {
            d *= 1.5;
        }
        let m = &mut self.mobs.list[k];
        m.hurt(d, from);
        m.aggro = true;
        let def = &self.mobs.reg.defs[m.def];
        self.last_hit = Some((def.display_name.clone(), m.health.max(0.0), m.max_health, self.now + 3.0));
        if let Some((id, secs)) = debuff {
            self.add_mob_debuff(k, id, secs);
        }
        self.sfx(Sfx::MobHit);
    }

    /// Hostile mobs touching a sphere at `at`.
    fn mobs_touching(&self, at: Vec3, r: f32) -> Vec<usize> {
        (0..self.mobs.list.len())
            .filter(|&k| {
                let m = &self.mobs.list[k];
                if !m.alive() || !self.is_hostile(k) {
                    return false;
                }
                let d = &self.mobs.reg.defs[m.def];
                let c = m.body.center();
                let dx = Vec3::new(at.x - c.x, 0.0, at.z - c.z).length();
                dx < d.width * 0.5 + r && (at.y - c.y).abs() < d.height * 0.5 + r
            })
            .collect()
    }

    fn update_tr_shots(&mut self, dt: f32) {
        if self.t5.tr.wep.shots.is_empty() {
            return;
        }
        let mut shots = std::mem::take(&mut self.t5.tr.wep.shots);
        let hand = self.body.eye() - Vec3::Y * 0.4 + self.forward() * 0.3;
        let glow = self.glow_layer();
        let mut blasts: Vec<(Vec3, usize)> = Vec::new();
        let mut keep = Vec::with_capacity(shots.len());
        for mut s in shots.drain(..) {
            s.age += dt;
            for h in s.hits.iter_mut() {
                h.1 -= dt;
            }
            s.hits.retain(|h| h.1 > 0.0);
            let d = &WEAPONS[s.def];
            let mut alive = s.age < s.life;
            match s.fly {
                Fly::Yoyo => {
                    let Wk::Yoyo(reach) = d.kind else { continue };
                    let target = if s.back { hand } else { self.aim_point(reach) };
                    let to = target - s.pos;
                    s.vel = to.normalize_or_zero() * d.speed.min(to.length() / dt.max(1e-3));
                    s.pos += s.vel * dt;
                    if s.back && to.length() < 1.0 {
                        alive = false;
                    }
                    // The string, a few faint dots from the hand.
                    for k in 1..6 {
                        let p = hand.lerp(s.pos, k as f32 / 6.0);
                        self.particles.glow(p, Vec3::ZERO, glow, [0.5, 0.5, 0.5], 0.02, 0.03);
                    }
                }
                Fly::Boomerang | Fly::Flail => {
                    let out = match d.kind {
                        Wk::Boomerang(t) | Wk::Flail(t) => t,
                        _ => 0.5,
                    };
                    if s.age > out {
                        s.back = true;
                    }
                    let next = s.pos + s.vel * dt;
                    if !s.back && self.world.solid(next.floor().as_ivec3()) {
                        s.back = true;
                        self.sfx(Sfx::Hit(Material::Wood));
                    }
                    if s.back {
                        let to = hand - s.pos;
                        s.vel = to.normalize_or_zero() * d.speed * 1.2;
                        if to.length() < 1.2 {
                            alive = false;
                        }
                        s.pos += s.vel * dt;
                    } else {
                        s.pos = next;
                    }
                    if s.fly == Fly::Flail {
                        for k in 1..8 {
                            let p = hand.lerp(s.pos, k as f32 / 8.0);
                            self.particles.glow(p, Vec3::ZERO, glow, [0.45, 0.45, 0.5], 0.03, 0.03);
                        }
                    }
                }
                Fly::Bullet | Fly::Bolt => {
                    // Fast: move in short steps so nothing is skipped.
                    let steps = ((s.vel.length() * dt) / 0.5).ceil().max(1.0) as usize;
                    let step = s.vel * dt / steps as f32;
                    for _ in 0..steps {
                        s.pos += step;
                        if self.world.solid(s.pos.floor().as_ivec3()) {
                            alive = false;
                            self.particles.glow(s.pos - step, -step.normalize_or_zero() * 2.0, glow, s.tint, 0.06, 0.2);
                            break;
                        }
                        if !alive {
                            break;
                        }
                        for k in self.mobs_touching(s.pos, 0.15) {
                            if s.hits.iter().any(|h| h.0 == k) {
                                continue;
                            }
                            self.tr_hit(k, s.dmg, s.class, s.pos - step * 4.0, d.debuff);
                            s.hits.push((k, 10.0));
                            if s.pierce == 0 {
                                alive = false;
                                break;
                            }
                            s.pierce -= 1;
                        }
                        if !alive {
                            break;
                        }
                    }
                    let size = if s.fly == Fly::Bolt { 0.16 } else { 0.05 };
                    self.particles.glow(s.pos, Vec3::ZERO, glow, s.tint, size, 0.06);
                    if s.fly == Fly::Bolt {
                        self.particles.glow(s.pos - s.vel * 0.02, Vec3::ZERO, glow, s.tint, size * 0.7, 0.15);
                    }
                }
                Fly::Thrown => {
                    let Wk::Throw(g) = d.kind else { continue };
                    s.vel.y -= g * dt;
                    s.pos += s.vel * dt;
                    if self.world.solid(s.pos.floor().as_ivec3()) {
                        alive = false;
                    }
                    if let Some(&k) = self.mobs_touching(s.pos, 0.25).first() {
                        self.tr_hit(k, s.dmg, s.class, s.pos - s.vel * 0.05, d.debuff);
                        alive = false;
                    }
                }
                Fly::Bomb => {
                    s.vel.y -= 18.0 * dt;
                    let next = s.pos + s.vel * dt;
                    if self.world.solid(next.floor().as_ivec3()) {
                        // Bounce a little, then settle.
                        if self.world.solid(IVec3::new(next.x.floor() as i32, s.pos.y.floor() as i32, next.z.floor() as i32)) {
                            s.vel.x *= -0.3;
                            s.vel.z *= -0.3;
                        }
                        s.vel.y *= -0.3;
                        s.vel *= 0.7;
                    } else {
                        s.pos = next;
                    }
                    if (s.age * 8.0) as i32 % 2 == 0 {
                        self.particles.glow(s.pos + Vec3::Y * 0.35, Vec3::Y * 0.5, glow, [2.0, 1.2, 0.4], 0.05, 0.2);
                    }
                    if !alive {
                        blasts.push((s.pos, s.def));
                    }
                }
                Fly::Sentry => {
                    s.shot_t -= dt;
                    if s.shot_t <= 0.0 {
                        s.shot_t = d.cd;
                        let eye = s.pos + Vec3::Y * 0.6;
                        if let Some(k) = self.nearest_hostile(eye, 22.0) {
                            let to = self.mobs.list[k].body.center() - eye;
                            let dmg = d.dmg;
                            keep.push(TrShot { fly: Fly::Bolt, def: s.def, pos: eye, vel: to.normalize_or_zero() * d.speed, age: 0.0, life: 1.0, dmg, class: SUMMON, pierce: 0, hits: Vec::new(), back: false, tint: if d.item == "lightning_aura_rod" { [0.8, 1.4, 2.4] } else { s.tint }, shot_t: 0.0 });
                            self.sfx(Sfx::Place(Material::Cloth));
                        }
                    }
                }
            }
            // Melee throwables hit everything they pass, each mob twice a second at most.
            if matches!(s.fly, Fly::Yoyo | Fly::Boomerang | Fly::Flail) {
                let r = if s.fly == Fly::Flail { 0.5 } else { 0.35 };
                for k in self.mobs_touching(s.pos, r) {
                    if s.hits.iter().any(|h| h.0 == k) {
                        continue;
                    }
                    self.tr_hit(k, s.dmg, s.class, s.pos - s.vel.normalize_or_zero(), d.debuff);
                    s.hits.push((k, if s.fly == Fly::Yoyo { 0.3 } else { 0.5 }));
                    if s.fly == Fly::Flail {
                        s.back = true;
                    }
                }
                if d.item == "enchanted_boomerang" || d.debuff.is_some() {
                    self.particles.glow(s.pos, Vec3::ZERO, glow, s.tint, 0.08, 0.15);
                }
            }
            if alive {
                keep.push(s);
            }
        }
        // Whatever was added during the update (none today) goes after the ones kept.
        keep.append(&mut shots);
        self.t5.tr.wep.shots = keep;
        for (at, def) in blasts {
            self.tr_blast(at, def);
        }
    }

    fn tr_blast(&mut self, at: Vec3, def: usize) {
        let d = &WEAPONS[def];
        let Wk::Bomb(power, breaks, _) = d.kind else { return };
        if breaks {
            self.explode(at, power);
            return;
        }
        let r = power * 1.6;
        let hit: Vec<(usize, f32)> = (0..self.mobs.list.len())
            .filter(|&k| self.mobs.list[k].alive() && self.is_hostile(k))
            .map(|k| (k, self.mobs.list[k].body.center().distance(at)))
            .filter(|&(_, dist)| dist < r)
            .collect();
        for (k, dist) in hit {
            self.tr_hit(k, d.dmg * (1.0 - dist / r * 0.6), RANGED, at, None);
        }
        let glow = self.glow_layer();
        for _ in 0..40 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.3, self.surv.rng.f32() - 0.5) * 10.0;
            self.particles.glow(at, v, glow, [2.0, 1.1, 0.4], 0.14, 0.5);
        }
        self.sfx(Sfx::Explode);
    }

    /// The flying weapons as items for the renderer.
    pub(crate) fn tr_weapon_instances(&self, cam_block: IVec3) -> Vec<GpuItem> {
        let shots = &self.t5.tr.wep.shots;
        if shots.is_empty() {
            return Vec::new();
        }
        let c = cam_block.as_vec3();
        let mut out = Vec::new();
        for s in shots {
            // Sentries have their own 3D models (tr_sentry_models).
            if matches!(s.fly, Fly::Bullet | Fly::Bolt) || (s.fly == Fly::Sentry && crate::summons::rod_model(WEAPONS[s.def].item).is_some()) {
                continue;
            }
            let Some(id) = self.items.id(WEAPONS[s.def].item) else { continue };
            let voxel::item::Look::Sprite(l) = self.items.get(id).look else { continue };
            let layer = (self.item_base + l) as f32;
            let l = self.world.light(s.pos.floor().as_ivec3());
            let (sky, blk) = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let p = s.pos - c;
            let (size, yaw, pitch, roll) = match s.fly {
                Fly::Yoyo | Fly::Boomerang | Fly::Thrown => (0.5, 0.0, std::f32::consts::FRAC_PI_2, s.age * 25.0),
                Fly::Flail => (0.6, s.age * 12.0, 0.0, 0.0),
                Fly::Sentry => (1.2, 0.0, 0.0, 0.0),
                _ => (0.45, s.age * 6.0, 0.0, 0.0),
            };
            // Extruded like a held item, so a spinning boomerang has its thickness.
            out.push(GpuItem { pos: [p.x, p.y, p.z, size], info: [yaw, 2.0, sky, blk], layers: [layer, layer, layer, 0.0], rot: [pitch, roll, 0.0, 0.0], ..bytemuck::Zeroable::zeroed() });
        }
        out
    }

    /// The sentries the rods set down, as 3D models (summons.rs) turned to the nearest monster.
    pub(crate) fn tr_sentry_models(&self, cam_block: IVec3) -> Vec<(usize, crate::entity::Bones)> {
        let c = cam_block.as_vec3();
        let mut out = Vec::new();
        for s in self.t5.tr.wep.shots.iter().filter(|s| s.fly == Fly::Sentry) {
            let d = &WEAPONS[s.def];
            let Some(model) = crate::summons::rod_model(d.item) else { continue };
            let Some(mesh) = self.entities.summon_mesh(model) else { continue };
            // Its foot: the shot sits a little over the ground.
            let foot = s.pos - Vec3::Y * 0.6;
            let eye = s.pos;
            let aim = self.nearest_hostile(eye, 22.0).map(|k| self.mobs.list[k].body.center() - eye).unwrap_or(Vec3::ZERO);
            let l = self.world.light((foot + Vec3::Y * 0.5).floor().as_ivec3());
            let pose = crate::summons::SentryPose {
                pos: foot - c,
                aim,
                t: self.now + foot.x * 0.37,
                fire: if aim == Vec3::ZERO { 0.0 } else { (1.0 - (d.cd - s.shot_t) / 0.6).clamp(0.0, 1.0) },
                light: [(l >> 4) as f32 / 15.0, ((l & 15) as f32 / 15.0).max(0.4), 0.0, 0.0],
            };
            out.push((mesh, crate::summons::sentry_bones(model, &pose)));
        }
        out
    }
}

impl Game {
    /// Test scene (MC_TR=weapons): a yoyo out, a boomerang and a flail in the air, bullets,
    /// a bomb about to go off and a sentry, with a few monsters to aim at.
    pub(crate) fn test_tr_weapons(&mut self) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let eye = self.body.eye();
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        for (k, n) in ["zombie", "skeleton", "zombie"].iter().enumerate() {
            let p = self.body.pos + flat * 11.0 + side * (k as f32 - 1.0) * 3.0;
            self.mobs.spawn(n, Vec3::new(p.x, ground(self, p), p.z));
        }
        let shots: [(&str, Fly, Vec3, Vec3, f32); 6] = [
            ("magma_yoyo", Fly::Yoyo, eye + flat * 4.0 - side * 1.5, Vec3::ZERO, 999.0),
            ("enchanted_boomerang", Fly::Boomerang, eye + flat * 6.0 + side * 2.0 + Vec3::Y * 0.5, flat * 20.0, 99.0),
            ("sun_flail", Fly::Flail, eye + flat * 3.5 + side * 1.2 - Vec3::Y * 0.3, flat * 20.0, 99.0),
            ("bomb", Fly::Bomb, eye + flat * 5.0 - Vec3::Y * 1.2 - side * 3.0, Vec3::ZERO, 99.0),
            ("ballista_rod", Fly::Sentry, eye + flat * 4.0 + side * 3.5 - Vec3::Y * 1.0, Vec3::ZERO, 99.0),
            ("flintlock_pistol", Fly::Bullet, eye + flat * 2.0, flat * 0.01, 99.0),
        ];
        for (item, fly, pos, vel, life) in shots {
            let Some(i) = def_of(item) else { continue };
            self.add_shot(fly, i, pos, vel, life, MELEE);
        }
        if let Some(id) = self.items.id("magma_yoyo") {
            self.surv.inv.slots[self.selected] = Some(voxel::item::ItemStack::new(id, 1));
        }
    }
}
