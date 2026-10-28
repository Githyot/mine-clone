//! Better Combat idea: weapons swing through an arc and hit everything inside it, and each
//! weapon cycles through a combo of different attacks (a slash, a backhand, a heavy finisher).
//! Two-handed weapons (greatswords, spears, halberds) hit harder and reach further but swing
//! slower and leave no hand free for the off-hand item.
//!
//! Only melee weapons take this path; picks, shovels and the bare hand keep the classic hit.
//! The swing still goes through grass and flowers (the caller measures the block in the way).

use glam::Vec3;
use voxel::block::ToolKind;

use crate::audio::Sfx;
use crate::Game;

/// How one attack of a combo moves and hits.
#[derive(Clone, Copy, Debug)]
struct Attack {
    /// Animation (see `player_model::Animator::swing_style`): 0 chop, 1 slash, 2 thrust, 3 uppercut.
    style: u8,
    /// Damage multiplier on the weapon's damage.
    damage: f32,
    /// Half-angle of the arc in degrees (a thrust is narrow, a slash wide).
    arc: f32,
    /// Knockback push.
    push: f32,
}

/// A weapon class: reach, swing time and its combo.
#[derive(Clone, Copy, Debug)]
pub struct Weapon {
    reach: f32,
    /// Seconds between full-strength swings.
    pub cooldown: f32,
    /// Share of the damage dealt to the other mobs in the arc (the primary takes it all).
    sweep: f32,
    pub two_handed: bool,
    combo: [Attack; 3],
}

const fn at(style: u8, damage: f32, arc: f32, push: f32) -> Attack {
    Attack { style, damage, arc, push }
}

const SWORD: Weapon = Weapon { reach: 3.2, cooldown: 0.625, sweep: 0.5, two_handed: false, combo: [at(1, 1.0, 50.0, 1.0), at(1, 1.0, 50.0, 1.0), at(2, 1.3, 14.0, 2.0)] };
const AXE: Weapon = Weapon { reach: 3.0, cooldown: 1.0, sweep: 0.35, two_handed: false, combo: [at(0, 1.0, 28.0, 1.5), at(0, 1.0, 28.0, 1.5), at(3, 1.35, 30.0, 3.0)] };
const GREATSWORD: Weapon = Weapon { reach: 3.8, cooldown: 1.2, sweep: 0.8, two_handed: true, combo: [at(1, 1.0, 80.0, 2.0), at(1, 1.0, 80.0, 2.0), at(0, 1.6, 40.0, 4.0)] };
const SPEAR: Weapon = Weapon { reach: 4.8, cooldown: 0.85, sweep: 0.8, two_handed: true, combo: [at(2, 1.0, 12.0, 1.5), at(2, 1.0, 12.0, 1.5), at(2, 1.45, 16.0, 3.5)] };
const HALBERD: Weapon = Weapon { reach: 4.3, cooldown: 1.35, sweep: 0.7, two_handed: true, combo: [at(0, 1.0, 45.0, 2.5), at(1, 1.1, 65.0, 2.5), at(2, 1.5, 15.0, 4.5)] };

/// After this long without a swing the combo starts over.
const COMBO_WINDOW: f32 = 0.9;

#[derive(Default)]
pub struct Combat {
    /// Which attack of the combo comes next (0..3).
    step: u8,
    /// Game time of the last swing.
    last: f32,
    /// The off-hand item was put away for a two-handed weapon (one message per weapon).
    warned: bool,
}

/// The weapon class of an item, from its name and tool kind.
pub fn weapon_of(name: &str, kind: Option<ToolKind>) -> Option<Weapon> {
    if name.ends_with("_greatsword") {
        Some(GREATSWORD)
    } else if name.ends_with("_spear") {
        Some(SPEAR)
    } else if name.ends_with("_halberd") {
        Some(HALBERD)
    } else {
        match kind {
            Some(ToolKind::Sword) => Some(SWORD),
            Some(ToolKind::Axe) => Some(AXE),
            _ => None,
        }
    }
}

impl Game {
    pub(crate) fn held_weapon(&self) -> Option<Weapon> {
        let st = self.held_stack()?;
        let it = self.items.get(st.item);
        weapon_of(&it.name, it.tool.as_ref().map(|t| t.kind))
    }

    /// The swing time of the weapon in hand, when it is a combat weapon.
    pub(crate) fn weapon_cooldown(&self) -> Option<f32> {
        self.held_weapon().map(|w| w.cooldown)
    }

    /// A melee swing with a combat weapon: hits every mob in its arc, nearer than the first
    /// solid block (`block_dist`). False when the held item is not a weapon or nothing is in
    /// reach, so the classic hit runs instead.
    pub(crate) fn combat_swing(&mut self, block_dist: f32) -> bool {
        let Some(w) = self.held_weapon() else { return false };
        let step = if self.now - self.mods.combat.last <= w.cooldown + COMBO_WINDOW { self.mods.combat.step } else { 0 };
        let a = w.combo[step as usize % 3];
        let eye = self.body.eye();
        let fwd = self.forward();
        let cos_arc = a.arc.to_radians().cos();
        // Every living mob whose body is inside the arc, nearest first.
        let mut hits: Vec<(usize, f32)> = Vec::new();
        for (i, m) in self.mobs.list.iter().enumerate() {
            if !m.alive() {
                continue;
            }
            let def = &self.mobs.reg.defs[m.def];
            let center = m.body.pos + Vec3::Y * (def.height * 0.5).min(1.2);
            let to = center - eye;
            let d = to.length();
            let r = def.width * 0.5 * def.scale;
            if d - r > w.reach || d - r > block_dist {
                continue;
            }
            // Measure the angle to the nearest point of the body, so big mobs are easy to hit.
            let along = to.dot(fwd);
            let side = (to - fwd * along).length();
            let angle_ok = along > 0.0 && (along / d.max(0.01) >= cos_arc || side <= r + 0.35);
            if angle_ok {
                hits.push((i, d));
            }
        }
        if hits.is_empty() {
            return false;
        }
        hits.sort_by(|a, b| a.1.total_cmp(&b.1));
        // Town folk are only hit when aimed at directly.
        let primary = hits[0].0;
        hits.retain(|&(i, _)| i == primary || !self.mobs.reg.defs[self.mobs.list[i].def].town);
        let charge = if self.survival { self.attack_charge() } else { 1.0 };
        let base = if self.survival { self.surv_attack_damage() } else { 4.0 + 3.0 * w.two_handed as u8 as f32 };
        let crit = self.body.vel.y < -0.5 && !self.body.on_ground && !self.body.in_water && charge > 0.9;
        let dmg = base * a.damage * if crit { 1.5 } else { 1.0 };
        for (k, &(i, _)) in hits.iter().enumerate().take(6) {
            let share = if k == 0 { 1.0 } else { w.sweep };
            let m = &mut self.mobs.list[i];
            m.hurt(dmg * share, eye);
            let dir = m.body.pos - eye;
            m.body.knockback(dir, a.push * charge.max(0.3));
            if k == 0 {
                let def = &self.mobs.reg.defs[m.def];
                self.last_hit = Some((def.display_name.clone(), m.health.max(0.0), m.max_health, self.now + 3.0));
            }
        }
        // A sweep of glints along the arc (a few for a thrust, a fan for a slash).
        let glow = self.glow_layer();
        let right = fwd.cross(Vec3::Y).normalize_or_zero();
        let n = if a.arc > 30.0 { 7 } else { 3 };
        for j in 0..n {
            let t = j as f32 / (n - 1).max(1) as f32 * 2.0 - 1.0;
            let ang = t * a.arc.to_radians() * 0.8;
            let dir = (fwd * ang.cos() + right * ang.sin()).normalize_or_zero();
            let p = eye + dir * (w.reach * 0.6) - Vec3::Y * 0.25;
            self.particles.glow(p, dir * 1.5, glow, [1.0, 1.0, 0.95], 0.05, 0.25);
        }
        self.attack_cd = if self.survival { 0.1 } else { 0.45 };
        self.break_cd = 0.3;
        self.mods.combat.last = self.now;
        self.mods.combat.step = (step + 1) % 3;
        self.anim.start_styled_swing(a.style);
        self.sfx(Sfx::MobHit);
        if step == 2 {
            // The finisher lands heavier.
            self.sfx(Sfx::Hit(crate::audio::Material::Metal));
        }
        self.pads.rumble(0.3 + 0.2 * w.two_handed as u8 as f32, 0.6, 0.1);
        true
    }

    /// Two-handed weapons need both hands: the off-hand item goes back into the bag.
    pub(crate) fn update_combat(&mut self) {
        let two = self.held_weapon().is_some_and(|w| w.two_handed);
        if !two {
            self.mods.combat.warned = false;
            return;
        }
        // A shield stays: it is slung on the back until the hands are free (shield.rs).
        if let Some(st) = self.surv.offhand.filter(|s| self.items.get(s.item).name != "shield") {
            let left = self.surv.inv.add(&self.items, st);
            self.surv.offhand = left;
            if !self.mods.combat.warned {
                self.mods.combat.warned = true;
                self.toast(if left.is_none() { "Arma de duas mãos: o item da outra mão foi para a mochila" } else { "Arma de duas mãos: libere a outra mão" }, 3.0);
            }
        }
    }

    /// The combo step shown on the F3 screen.
    pub(crate) fn combat_line(&self) -> Option<String> {
        let w = self.held_weapon()?;
        let step = if self.now - self.mods.combat.last <= w.cooldown + COMBO_WINDOW { self.mods.combat.step } else { 0 };
        Some(format!("Combate: golpe {} de 3{}", step + 1, if w.two_handed { " (duas mãos)" } else { "" }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_classes_from_names() {
        assert!(weapon_of("iron_greatsword", Some(ToolKind::Sword)).unwrap().two_handed);
        assert!(weapon_of("iron_spear", Some(ToolKind::Sword)).unwrap().reach > 4.0);
        assert!(!weapon_of("iron_sword", Some(ToolKind::Sword)).unwrap().two_handed);
        assert!(weapon_of("iron_pickaxe", Some(ToolKind::Pickaxe)).is_none());
        // Every combo ends with a heavier blow.
        for w in [SWORD, AXE, GREATSWORD, SPEAR, HALBERD] {
            assert!(w.combo[2].damage > w.combo[0].damage);
        }
    }
}
