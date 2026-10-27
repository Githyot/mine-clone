//! Trial chambers (Phase 6): trial spawners that send waves of mobs when a player comes near and
//! give a trial key when the wave is beaten, vaults that open once per player for a key, the
//! breeze with its wind charges, and the ominous versions that Bad Omen turns them into.
//!
//! Also the once-a-second scan of the blocks around the player that the trial, deep dark and
//! archaeology code look at (spawners, vaults, sculk sensors, shriekers, catalysts).

use glam::{IVec3, Vec3};
use voxel::block::AIR;
use voxel::item::ItemStack;
use voxel::physics::Terrain;
use voxel::BlockId;

use crate::audio::Sfx;
use crate::effects::{BAD_OMEN, TRIAL_OMEN};
use crate::progression::Event;

/// A trial spawner's wave.
#[derive(Clone, Debug, Default)]
pub struct Spawner {
    /// 0 waiting for a player, 1 sending its wave, 2 cooling down.
    pub state: u8,
    pub mob: String,
    pub spawned: u8,
    pub total: u8,
    pub next: f32,
    pub cool: f32,
    pub ominous: bool,
    /// Ominous spawners drop an item spawner's gift now and then (arrows, potions).
    pub gift_t: f32,
}

/// A breeze's wind charge in flight (reflected ones fly back and hurt mobs).
#[derive(Clone, Copy, Debug)]
pub struct Charge {
    pub pos: Vec3,
    pub vel: Vec3,
    pub age: f32,
    pub reflected: bool,
}

/// Blocks the scan keeps track of.
pub const WATCHED: [&str; 15] = [
    "sniffer_egg",
    "trial_spawner",
    "trial_spawner_active",
    "trial_spawner_ominous",
    "vault",
    "ominous_vault",
    "sculk_sensor",
    "sculk_sensor_active",
    "calibrated_sculk_sensor",
    "calibrated_sculk_sensor_active",
    "sculk_shrieker",
    "sculk_catalyst",
    "chiseled_tuff",
    "tuff_bricks",
    "reinforced_deepslate",
];

/// What a spawner at `p` spawns: decided by where it is, like each chamber room has its own.
pub fn spawner_mob(p: IVec3) -> &'static str {
    const MOBS: [&str; 7] = ["zombie", "skeleton", "spider", "stalker", "breeze", "skeleton", "spider"];
    let h = (p.x.wrapping_mul(73_856_093) ^ p.y.wrapping_mul(19_349_663) ^ p.z.wrapping_mul(83_492_791)) as u32;
    MOBS[(h % MOBS.len() as u32) as usize]
}

/// Vault rewards: (item, min, max, weight). Ominous vaults have their own, richer list.
const VAULT_LOOT: [(&str, u8, u8, u32); 16] = [
    ("emerald", 2, 4, 5),
    ("arrow", 2, 8, 4),
    ("tipped_arrow_poison", 2, 4, 3),
    ("iron_ingot", 1, 4, 4),
    ("honey_bottle", 1, 2, 2),
    ("ominous_bottle", 1, 1, 2),
    ("wind_charge", 4, 12, 3),
    ("diamond", 1, 2, 2),
    ("shield", 1, 1, 2),
    ("bolt_armor_trim_smithing_template", 1, 1, 1),
    ("flow_banner_pattern", 1, 1, 1),
    ("guster_pottery_sherd", 1, 1, 1),
    ("scrape_pottery_sherd", 1, 1, 1),
    ("golden_apple", 1, 1, 2),
    ("enchanted_book", 1, 1, 3),
    ("crossbow", 1, 1, 2),
];

const OMINOUS_LOOT: [(&str, u8, u8, u32); 12] = [
    ("emerald_block", 1, 1, 5),
    ("iron_block", 1, 1, 4),
    ("diamond", 2, 3, 4),
    ("enchanted_golden_apple", 1, 1, 1),
    ("flow_armor_trim_smithing_template", 1, 1, 2),
    ("music_disc_creator", 1, 1, 1),
    ("heavy_core", 1, 1, 1),
    ("enchanted_book", 1, 1, 4),
    ("diamond_axe", 1, 1, 2),
    ("diamond_chestplate", 1, 1, 2),
    ("wind_charge", 8, 12, 3),
    ("ominous_bottle", 1, 2, 2),
];

/// Trial spawner rewards when its wave is beaten.
const SPAWNER_LOOT: [(&str, u8, u8, u32); 7] = [
    ("bread", 1, 3, 3),
    ("cooked_chicken", 1, 3, 3),
    ("arrow", 4, 8, 3),
    ("baked_potato", 1, 3, 2),
    ("tipped_arrow_slowness", 2, 4, 1),
    ("potion_regeneration", 1, 1, 1),
    ("potion_strength", 1, 1, 1),
];

impl crate::Game {
    /// Once a second: where the watched blocks are around the player.
    pub(crate) fn scan_nearby(&mut self, dt: f32) {
        self.prog.scan_t -= dt;
        if self.prog.scan_t > 0.0 {
            return;
        }
        self.prog.scan_t = 1.0;
        let ids: Vec<BlockId> = WATCHED.iter().map(|n| self.reg.id_or_air(n)).collect();
        if ids.iter().all(|&i| i == AIR) {
            return;
        }
        let c = self.body.pos.floor().as_ivec3();
        let mut near = Vec::new();
        for dy in -12..=12 {
            for dz in -16..=16 {
                for dx in -16..=16 {
                    let p = c + IVec3::new(dx, dy, dz);
                    let b = self.world.block_or_air(p);
                    if b != AIR && ids.contains(&b) {
                        near.push((p, b));
                    }
                }
            }
        }
        self.prog.near = near;
    }

    /// Positions of watched blocks called any of `names`.
    pub(crate) fn near_of(&self, names: &[&str]) -> Vec<IVec3> {
        self.prog.near.iter().filter(|(_, b)| names.contains(&self.reg.get(*b).name.as_str())).map(|(p, _)| *p).collect()
    }

    fn roll_loot(&mut self, table: &[(&str, u8, u8, u32)], at: Vec3, rolls: u32) {
        let total: u32 = table.iter().map(|t| t.3).sum();
        for _ in 0..rolls {
            let mut r = self.prog.rng.below(total);
            for &(name, lo, hi, w) in table {
                if r < w {
                    if let Some(id) = self.items.id(name) {
                        let n = lo + self.prog.rng.below((hi - lo + 1) as u32) as u8;
                        let mut st = ItemStack::new(id, n.max(1));
                        if name == "enchanted_book" {
                            st.ench = crate::enchanting::random_any(&self.items, id, &mut self.prog.rng);
                        }
                        self.pop_drop(at, st);
                    }
                    break;
                }
                r -= w;
            }
        }
    }

    /// Right click on a vault with a key. True when handled.
    pub(crate) fn vault_use(&mut self, name: &str, pos: IVec3) -> bool {
        let ominous = match name {
            "vault" => false,
            "ominous_vault" => true,
            _ => return false,
        };
        let key = if ominous { "ominous_trial_key" } else { "trial_key" };
        if self.prog.vaults_opened.contains(&pos) {
            self.toast("Este cofre já foi aberto por você", 2.0);
            self.sfx(Sfx::Click);
            return true;
        }
        if self.held_key() != key {
            self.toast(if ominous { "Precisa de uma chave dos desafios agourenta" } else { "Precisa de uma chave dos desafios" }, 2.0);
            return true;
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.prog.vaults_opened.push(pos);
        let at = pos.as_vec3() + Vec3::new(0.5, 1.1, 0.5);
        let rolls = 1 + self.prog.rng.below(if ominous { 3 } else { 2 });
        if ominous {
            self.roll_loot(&OMINOUS_LOOT, at, rolls);
            self.prog.event(Event::Token("ominous_vault".into()));
        } else {
            self.roll_loot(&VAULT_LOOT, at, rolls);
        }
        self.prog.event(Event::VaultOpened);
        self.prog.stats.add("vaults", 1);
        self.sfx(Sfx::Xp);
        let glow = self.glow_layer();
        for k in 0..12 {
            let a = k as f32 * 0.52;
            self.particles.glow(at, Vec3::new(a.cos() * 1.5, 2.0, a.sin() * 1.5), glow, if ominous { [0.5, 0.8, 1.4] } else { [1.4, 1.0, 0.4] }, 0.08, 0.8);
        }
        true
    }

    /// Spawners, vaults' glow, breezes and their charges, and Bad Omen turning into Trial Omen.
    pub(crate) fn update_trials(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let me = self.body.pos;
        let spawners = self.near_of(&["trial_spawner", "trial_spawner_active", "trial_spawner_ominous"]);
        // Bad Omen near a spawner becomes Trial Omen, 15 minutes for each level.
        if !spawners.is_empty() && self.has_effect(BAD_OMEN) && self.survival {
            let lvl = self.effect_level(BAD_OMEN).max(1);
            self.prog.effects.retain(|e| e.id != BAD_OMEN);
            self.add_effect(TRIAL_OMEN, 0, 900.0 * lvl as f32, false);
            self.sfx(Sfx::Cast(3));
        }
        let omen = self.has_effect(TRIAL_OMEN);
        for p in spawners {
            let mut s = self.prog.trials.remove(&p).unwrap_or_else(|| Spawner { mob: spawner_mob(p).to_string(), ..Default::default() });
            let dist = (p.as_vec3() + Vec3::splat(0.5) - me).length();
            match s.state {
                0 => {
                    if dist < 14.0 && self.survival && !self.mobs.peaceful {
                        s.state = 1;
                        s.ominous = omen;
                        s.total = if omen { 10 } else { 6 };
                        s.spawned = 0;
                        s.next = 1.0;
                        self.prog.event(Event::Token("trial_chamber".into()));
                        self.set_spawner_look(p, if omen { "trial_spawner_ominous" } else { "trial_spawner_active" });
                    }
                }
                1 => {
                    let alive = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == s.mob && (m.body.pos - p.as_vec3()).length() < 24.0).count();
                    s.next -= dt;
                    let max_alive = if s.ominous { 3 } else { 2 };
                    if s.spawned < s.total && alive < max_alive && s.next <= 0.0 {
                        if let Some(at) = self.spawn_spot(p) {
                            if self.mobs.spawn(&s.mob, at) {
                                if let Some(m) = self.mobs.list.last_mut() {
                                    m.aggro = true;
                                }
                                s.spawned += 1;
                                let glow = self.glow_layer();
                                let col = if s.ominous { [0.4, 0.7, 1.4] } else { [1.4, 0.7, 0.2] };
                                for k in 0..10 {
                                    let a = k as f32 * 0.63;
                                    self.particles.glow(at + Vec3::Y * 0.5, Vec3::new(a.cos(), 1.5, a.sin()), glow, col, 0.08, 0.6);
                                }
                                self.sfx(Sfx::Fizz);
                            }
                        }
                        s.next = if s.ominous { 1.5 } else { 2.0 };
                    }
                    if s.ominous {
                        s.gift_t -= dt;
                        if s.gift_t <= 0.0 && dist < 20.0 {
                            s.gift_t = 8.0;
                            self.ominous_gift(me);
                        }
                    }
                    if s.spawned >= s.total && alive == 0 {
                        // Beaten: keys and rewards pop out, then it rests for half an hour.
                        let at = p.as_vec3() + Vec3::new(0.5, 1.2, 0.5);
                        let key = if s.ominous { "ominous_trial_key" } else { "trial_key" };
                        if let Some(id) = self.items.id(key) {
                            self.pop_drop(at, ItemStack::new(id, 1));
                        }
                        self.roll_loot(&SPAWNER_LOOT, at, if s.ominous { 3 } else { 2 });
                        self.spawn_orbs(at, 15);
                        self.prog.event(Event::SpawnerBeaten);
                        self.sfx(Sfx::Xp);
                        s.state = 2;
                        s.cool = 1800.0;
                        self.set_spawner_look(p, "trial_spawner");
                    } else if dist > 47.0 {
                        // The player left: it waits for them again.
                        s.state = 0;
                        self.set_spawner_look(p, "trial_spawner");
                    }
                }
                _ => {
                    s.cool -= dt;
                    if s.cool <= 0.0 {
                        s.state = 0;
                    }
                }
            }
            self.prog.trials.insert(p, s);
        }
        // Spawners far away keep counting their cool-down.
        for (p, s) in self.prog.trials.iter_mut() {
            if s.state == 2 && (p.as_vec3() - me).length() > 40.0 {
                s.cool -= dt;
                if s.cool <= 0.0 {
                    s.state = 0;
                }
            }
        }
        self.update_breezes(dt);
        self.update_charges(dt);
    }

    fn set_spawner_look(&mut self, p: IVec3, name: &str) {
        let id = self.reg.id_or_air(name);
        if id != AIR && self.world.block_or_air(p) != id {
            self.world.set_block(p, id);
        }
    }

    /// Ominous spawners drop things on the player now and then: a lingering potion or arrows.
    fn ominous_gift(&mut self, me: Vec3) {
        let at = me + Vec3::Y * 4.0;
        let pick = self.prog.rng.below(4);
        let key = match pick {
            0 => "poison",
            1 => "slowness",
            2 => "weakness",
            _ => "",
        };
        if let Some(p) = crate::effects::potion(key) {
            self.prog.thrown.push(crate::effects::Thrown { pos: at, vel: Vec3::new(0.0, -2.0, 0.0), potion: p, lingering: true });
        } else {
            for k in 0..3 {
                let d = Vec3::new((k as f32 - 1.0) * 0.6, -12.0, 0.3);
                self.mobs.arrows.push(crate::mobs::Arrow { pos: at + Vec3::X * (k as f32 - 1.0), vel: d, age: 0.0, stuck: false, damage: 3.0 });
            }
        }
        self.sfx(Sfx::Cast(3));
    }

    /// A free spot next to a spawner for a mob: air for two blocks over solid ground.
    fn spawn_spot(&mut self, p: IVec3) -> Option<Vec3> {
        for _ in 0..12 {
            let dx = self.prog.rng.range(-4, 5);
            let dz = self.prog.rng.range(-4, 5);
            for dy in [0, 1, -1, 2] {
                let q = p + IVec3::new(dx, dy, dz);
                let air = |q: IVec3| !Terrain::solid(&self.world, q);
                if air(q) && air(q + IVec3::Y) && !air(q - IVec3::Y) {
                    return Some(q.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
                }
            }
        }
        None
    }

    /// Breezes hop about and fire wind charges at the player.
    fn update_breezes(&mut self, dt: f32) {
        let Some(bi) = self.mobs.reg.id("breeze") else { return };
        let eye = self.body.eye();
        let me = self.body.pos;
        let creative = !self.survival || self.has_effect(crate::effects::INVISIBILITY);
        let mut shots = Vec::new();
        for m in self.mobs.list.iter_mut() {
            if m.def != bi || !m.alive() {
                continue;
            }
            let d = me - m.body.pos;
            let dist = d.length();
            if dist > 24.0 {
                continue;
            }
            // Hops: high, sideways, often.
            if m.body.on_ground && self.prog.rng.f32() < dt * 0.6 {
                let side = Vec3::new(-d.z, 0.0, d.x).normalize_or_zero() * if self.prog.rng.f32() < 0.5 { 1.0 } else { -1.0 };
                m.body.vel += side * 4.0 + Vec3::Y * 9.0;
            }
            if !creative && dist > 3.0 && self.prog.rng.f32() < dt / 2.2 {
                let from = m.body.pos + Vec3::Y * 1.2;
                let to = eye - Vec3::Y * 0.3 - from;
                shots.push(Charge { pos: from, vel: to.normalize_or_zero() * 11.0, age: 0.0, reflected: false });
            }
        }
        if !shots.is_empty() {
            self.sfx(Sfx::Cast(5));
        }
        self.prog.charges.extend(shots);
    }

    fn update_charges(&mut self, dt: f32) {
        if self.prog.charges.is_empty() {
            return;
        }
        let eye = self.body.eye();
        let fwd = self.forward();
        let swinging = self.anim.swing > 0.0 && self.anim.swing < 0.35;
        let glow = self.glow_layer();
        let mut i = 0;
        while i < self.prog.charges.len() {
            let mut c = self.prog.charges[i];
            c.age += dt;
            let next = c.pos + c.vel * dt;
            let mut gone = c.age > 6.0;
            // A swing hits it back where it came from.
            if !c.reflected && swinging && (c.pos - eye).length() < 3.0 && (c.pos - eye).normalize_or_zero().dot(fwd) > 0.6 {
                c.reflected = true;
                c.vel = fwd * 14.0;
                c.age = 0.0;
                self.sfx(Sfx::MobHit);
            }
            if !gone && !c.reflected && (next - (self.body.pos + Vec3::Y * 0.9)).length() < 0.9 {
                self.prog.next_kind = crate::progression::DamageKind::Projectile;
                self.damage(1.0, Some(c.pos));
                self.wind_burst(next, 1.0);
                gone = true;
            }
            if !gone && c.reflected {
                if let Some((k, _)) = self.mobs.pick(c.pos, c.vel.normalize_or_zero(), (next - c.pos).length() + 0.4) {
                    let breeze = self.mobs.reg.defs[self.mobs.list[k].def].name == "breeze";
                    let died = self.mobs.list[k].hurt(if breeze { 6.0 } else { 3.0 }, c.pos);
                    if died && breeze {
                        self.prog.event(Event::Token("blowback".into()));
                    }
                    self.wind_burst(next, 1.0);
                    gone = true;
                }
            }
            if !gone && Terrain::solid(&self.world, next.floor().as_ivec3()) {
                self.wind_burst(c.pos, 0.8);
                gone = true;
            }
            if gone {
                self.prog.charges.swap_remove(i);
                continue;
            }
            c.pos = next;
            self.particles.glow(c.pos, Vec3::ZERO, glow, [0.8, 0.9, 1.3], 0.18, 0.15);
            self.prog.charges[i] = c;
            i += 1;
        }
    }

    /// Wind charge jumps: rising 8 blocks in one counts for "who needs rockets".
    pub(crate) fn wind_jump_check(&mut self) {
        if self.body.vel.y > 0.0 && self.body.pos.y - self.prog.wind_from > 8.0 && self.prog.wind_from > 0.0 {
            self.prog.event(Event::Token("wind_jump".into()));
            self.prog.wind_from = 0.0;
        }
        if self.body.on_ground {
            self.prog.wind_from = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawner_mob_is_stable() {
        let p = IVec3::new(10, 20, -30);
        assert_eq!(spawner_mob(p), spawner_mob(p));
        let kinds: std::collections::HashSet<_> = (0..200).map(|i| spawner_mob(IVec3::new(i * 7, 30, i * 3))).collect();
        assert!(kinds.len() >= 4);
    }

    #[test]
    fn loot_weights_are_positive() {
        for t in VAULT_LOOT.iter().chain(OMINOUS_LOOT.iter()).chain(SPAWNER_LOOT.iter()) {
            assert!(t.3 > 0 && t.1 <= t.2);
        }
    }
}
