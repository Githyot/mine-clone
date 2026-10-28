//! The player's ranged and special weapons (bow, crossbow, trident, mace, spear, wind charges)
//! and everything enchantments change in play: harder hits, fire, knockback, sweeping, loot,
//! faster digging, silk touch and fortune, less wear, mending, protection, thorns, and the
//! boots and leggings that change how you move.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::physics::Terrain;
use voxel::AIR;

use crate::audio::{Material, Sfx};
use crate::effects::{self, PotionDef};
use crate::enchanting::*;
use crate::item_render::GpuItem;
use crate::progression::{DamageKind, Event};
use crate::Game;

#[derive(Clone, Copy)]
pub enum ShotKind {
    Arrow { potion: Option<&'static PotionDef>, pickup: bool },
    Trident { stack: ItemStack, returning: bool },
    WindCharge,
}

pub struct Shot {
    pub pos: Vec3,
    pub vel: Vec3,
    pub kind: ShotKind,
    pub damage: f32,
    pub crit: bool,
    pub punch: u8,
    pub flame: bool,
    /// Mobs it can still pass through (piercing).
    pub pierce: u8,
    pub hit: Vec<usize>,
    pub loyalty: u8,
    pub channeling: bool,
    pub impaling: u8,
    pub age: f32,
    pub stuck: bool,
}

impl Shot {
    fn new(pos: Vec3, vel: Vec3, kind: ShotKind, damage: f32) -> Shot {
        Shot { pos, vel, kind, damage, crit: false, punch: 0, flame: false, pierce: 0, hit: Vec::new(), loyalty: 0, channeling: false, impaling: 0, age: 0.0, stuck: false }
    }
}

/// A fully drawn plain bow's arrow (ore bows multiply it, tr_weapons::bow_power).
const BOW_DAMAGE: f32 = 6.0;
const CROSSBOW_DAMAGE: f32 = 9.0;

/// Damage per second of a bow drawn fully each second, or a crossbow reloaded each 1.25 s, for
/// the balance sheet (balance.rs). A full draw is always critical: on average a quarter more
/// and a point.
#[cfg(test)]
pub(crate) fn bow_dps(name: &str) -> Option<f32> {
    match name {
        "bow" => Some(BOW_DAMAGE * 1.25 + 1.0),
        "crossbow" => Some(CROSSBOW_DAMAGE / 1.35),
        _ => crate::tr_weapons::bow_power(name).map(|k| BOW_DAMAGE * k * 1.25 + 1.0),
    }
}

/// Undead for smite, arthropods for bane of arthropods.
fn arthropod(name: &str) -> bool {
    matches!(name, "spider" | "cave_spider" | "silverfish" | "endermite" | "bee")
}

fn water_mob(name: &str) -> bool {
    matches!(name, "duck" | "guardian" | "elder_guardian" | "drowned" | "squid" | "glow_squid" | "dolphin" | "turtle" | "axolotl" | "cod" | "salmon")
}

impl Game {
    fn held_ench(&self, id: u8) -> u8 {
        level(&self.held_stack(), id)
    }

    /// Highest level of an enchantment on the worn armor.
    pub(crate) fn armor_ench(&self, id: u8) -> u8 {
        self.surv.armor.iter().map(|s| level(s, id)).max().unwrap_or(0)
    }

    pub(crate) fn held_key(&self) -> String {
        self.held_stack().map(|s| self.items.get(s.item).name.clone()).unwrap_or_default()
    }

    // -----------------------------------------------------------------------------------------
    // Melee.

    /// Extra damage for a melee hit on mob `i`: enchantments, effects and the mace's smash.
    pub(crate) fn weapon_melee_bonus(&mut self, i: usize, dmg: f32) -> f32 {
        let name = self.mobs.reg.defs[self.mobs.list[i].def].name.clone();
        let mut d = dmg;
        let sharp = self.held_ench(SHARPNESS);
        if sharp > 0 {
            d += 0.5 * sharp as f32 + 0.5;
        }
        if effects::undead(&name) {
            d += 2.5 * self.held_ench(SMITE) as f32;
        }
        if arthropod(&name) {
            d += 2.5 * self.held_ench(BANE) as f32;
            if self.held_ench(BANE) > 0 {
                self.mobs.list[i].chill = self.mobs.list[i].chill.max(1.0 + 0.5 * self.held_ench(BANE) as f32);
            }
        }
        if self.held_key() == "trident" && (water_mob(&name) || self.mobs.list[i].body.in_water) {
            d += 2.5 * self.held_ench(IMPALING) as f32;
        }
        if self.held_key() == "mace" {
            let fall = (self.prog.mace_peak - self.body.pos.y).max(0.0);
            if fall > 1.5 && !self.body.on_ground {
                let f = fall;
                let smash = 4.0 * f.min(3.0) + 2.0 * (f - 3.0).clamp(0.0, 5.0) + (f - 8.0).max(0.0);
                d += smash + 0.5 * self.held_ench(DENSITY) as f32 * f;
                self.mace_smash(i, f);
            }
        }
        self.melee_effects(d)
    }

    fn mace_smash(&mut self, i: usize, fall: f32) {
        let at = self.mobs.list[i].body.pos;
        // Everything near the landing is thrown back; the player stops falling.
        for (k, m) in self.mobs.list.iter_mut().enumerate() {
            if k != i && m.alive() && (m.body.pos - at).length() < 3.5 {
                m.body.knockback(m.body.pos - at, 5.0);
            }
        }
        self.body.vel.y = 0.0;
        self.fall_peak = self.body.pos.y;
        self.prog.mace_peak = self.body.pos.y;
        let wind = self.held_ench(WIND_BURST);
        if wind > 0 {
            self.body.vel.y = 8.0 + 4.0 * wind as f32;
            self.prog.mace_peak = self.body.pos.y + 20.0;
        }
        let ground = self.world.block_or_air((at - Vec3::Y * 0.5).floor().as_ivec3());
        if ground != AIR {
            let (layer, tint) = self.block_look(ground);
            self.particles.block_bits(at.floor().as_ivec3(), layer, tint, 24, 1.4);
        }
        self.sfx(Sfx::Explode);
        self.prog.event(Event::MaceSmash(fall));
        self.wear_held(1);
    }

    /// After a melee hit lands: fire, extra knockback, the sweep, the spear's lunge.
    pub(crate) fn after_melee(&mut self, i: usize, dmg: f32) {
        let eye = self.body.eye();
        let fire = self.held_ench(FIRE_ASPECT);
        let kb = self.held_ench(KNOCKBACK);
        let sweep = self.held_ench(SWEEPING);
        let sword = self.held_tool().is_some_and(|t| t.kind == voxel::block::ToolKind::Sword);
        let target = self.mobs.list[i].body.pos;
        {
            let m = &mut self.mobs.list[i];
            if fire > 0 {
                m.on_fire = m.on_fire.max(4.0 * fire as f32);
            }
            if kb > 0 {
                m.body.knockback(m.body.pos - eye, 3.0 * kb as f32);
            }
        }
        // A sword swung standing still catches the mobs next to the one hit.
        if sword && !self.sprinting && self.body.on_ground && self.attack_charge_was_full() {
            let extra = 1.0 + dmg * if sweep > 0 { sweep as f32 / (sweep as f32 + 1.0) } else { 0.0 };
            for (k, m) in self.mobs.list.iter_mut().enumerate() {
                if k != i && m.alive() && (m.body.pos - target).length() < 1.8 && !self.mobs.reg.defs[m.def].town {
                    m.hurt(extra, eye);
                }
            }
            self.prog.sweep = 0.25;
        }
        // The spear's lunge: a quick dash forward with the hit.
        let lunge = self.held_ench(LUNGE);
        if self.held_key().ends_with("_spear") && lunge > 0 && self.surv.hunger > 6.0 {
            let f = self.forward();
            self.body.vel += Vec3::new(f.x, 0.1, f.z).normalize_or_zero() * (4.0 + 2.5 * lunge as f32);
            self.surv.exhaustion += 1.0;
        }
        self.prog.stats.add("damage_dealt", (dmg * 10.0) as u64);
        self.rpg_after_melee(i, dmg);
    }

    fn attack_charge_was_full(&self) -> bool {
        // The charge was used up by the hit just now; a fresh click after a full wait counts.
        self.prog.was_full
    }

    /// Reach for hitting mobs: 3 blocks in survival and 5 in creative, as in Java Edition;
    /// spears reach further.
    pub(crate) fn weapon_reach(&self) -> f32 {
        if self.held_key().ends_with("_spear") {
            4.5
        } else if crate::rpg::is_whip(&self.held_key()) {
            // Whips reach further (rpg.rs).
            5.0
        } else if self.survival {
            3.0
        } else {
            5.0
        }
    }

    // -----------------------------------------------------------------------------------------
    // Digging, drops and wear.

    /// Divides dig time: efficiency (with the right tool), haste, fatigue, under water.
    pub(crate) fn dig_speed_bonus(&self, id: voxel::BlockId) -> f32 {
        let mut k = 1.0;
        if let Some(t) = self.held_tool() {
            let e = self.held_ench(EFFICIENCY) as f32;
            if e > 0.0 && self.reg.get(id).tool == Some(t.kind) {
                k *= (t.speed + e * e + 1.0) / t.speed.max(0.1);
            }
        }
        k *= self.dig_effects();
        k *= self.shears_speed(id);
        if self.body.head_in_water && self.armor_ench(AQUA_AFFINITY) == 0 {
            k /= 5.0;
        }
        k.max(0.0001)
    }

    /// Silk touch keeps the block itself; fortune multiplies what ores give.
    pub(crate) fn ench_drops(&mut self, id: voxel::BlockId, drops: Vec<ItemStack>) -> Vec<ItemStack> {
        let tool = self.held_tool();
        if self.held_ench(SILK_TOUCH) > 0 && self.reg.get(id).drops.is_some() && self.items.can_harvest(&self.reg, id, tool.as_ref()) {
            if let Some(i) = self.items.of_block(self.reg.get(id).base) {
                return vec![ItemStack::new(i, 1)];
            }
        }
        let fortune = self.held_ench(FORTUNE) as u32;
        if fortune == 0 {
            return drops;
        }
        drops
            .into_iter()
            .map(|mut d| {
                let n = &self.items.get(d.item).name;
                let ore = n.starts_with("raw_") || matches!(n.as_str(), "coal" | "diamond" | "emerald" | "lapis_lazuli" | "redstone" | "quartz" | "ruby" | "sapphire" | "topaz" | "amethyst" | "amethyst_shard" | "gold_nugget" | "glowstone_dust" | "echo_shard" | "flint" | "wheat_seeds" | "melon_slice");
                if ore {
                    let bonus = (self.prog.rng.below(fortune + 2) as i32 - 1).max(0) as u32;
                    d.count = (d.count as u32 * (bonus + 1)).min(64) as u8;
                }
                d
            })
            .collect()
    }

    /// Unbreaking: the chance this use doesn't wear the item.
    pub(crate) fn unbreaking_saves(&mut self, s: &ItemStack, armor: bool) -> bool {
        let u = s.ench.get(UNBREAKING) as f32;
        if u == 0.0 {
            return false;
        }
        let keep = if armor { 0.6 + 0.4 / (u + 1.0) } else { 1.0 / (u + 1.0) };
        self.prog.rng.f32() >= keep
    }

    /// Mending: experience picked up repairs worn mending items first (2 durability per point).
    pub(crate) fn mend_with_xp(&mut self, xp: u32) -> u32 {
        let mut left = xp;
        let slots: Vec<(bool, usize)> = (0..4).map(|i| (true, i)).chain(std::iter::once((false, self.selected))).collect();
        for (armor, i) in slots {
            if left == 0 {
                break;
            }
            let st = if armor { self.surv.armor[i] } else { self.surv.inv.slots[i] };
            let Some(mut st) = st else { continue };
            if st.ench.get(MENDING) == 0 || st.damage == 0 {
                continue;
            }
            let fix = (left * 2).min(st.damage as u32);
            st.damage -= fix as u16;
            left -= fix.div_ceil(2);
            if armor {
                self.surv.armor[i] = Some(st);
            } else {
                self.surv.inv.slots[i] = Some(st);
            }
        }
        left
    }

    /// Hits on the player: protection enchantments, resistance, fire resistance, slow falling and
    /// golden absorption hearts. Thorns hurts the attacker back.
    pub(crate) fn protect(&mut self, amount: f32, from: Option<Vec3>) -> f32 {
        let kind = std::mem::take(&mut self.prog.next_kind);
        let kind = if kind == DamageKind::Attack && from.is_none() { DamageKind::Magic } else { kind };
        if kind == DamageKind::Fire && self.has_effect(effects::FIRE_RESISTANCE) {
            return 0.0;
        }
        if kind == DamageKind::Fall && self.has_effect(effects::SLOW_FALLING) {
            return 0.0;
        }
        let mut epf = 0u32;
        for s in self.surv.armor.iter().flatten() {
            epf += s.ench.get(PROTECTION) as u32;
            epf += 2 * match kind {
                DamageKind::Fire => s.ench.get(FIRE_PROTECTION),
                DamageKind::Explosion => s.ench.get(BLAST_PROTECTION),
                DamageKind::Projectile => s.ench.get(PROJECTILE_PROTECTION),
                _ => 0,
            } as u32;
            if kind == DamageKind::Fall {
                epf += 3 * s.ench.get(FEATHER_FALLING) as u32;
            }
        }
        if matches!(kind, DamageKind::Starve) {
            epf = 0;
        }
        let mut a = amount * (1.0 - epf.min(20) as f32 * 0.04);
        a *= (1.0 - 0.2 * self.effect_level(effects::RESISTANCE) as f32).max(0.0);
        if self.prog.absorb > 0.0 {
            let used = self.prog.absorb.min(a);
            self.prog.absorb -= used;
            a -= used;
        }
        // Thorns: the attacker gets hurt back now and then.
        if let (Some(f), DamageKind::Attack) = (from, kind) {
            let t = self.armor_ench(THORNS) as f32;
            if t > 0.0 && self.prog.rng.f32() < 0.15 * t {
                let dmg = 1.0 + self.prog.rng.below(4) as f32;
                let me = self.body.pos;
                if let Some(m) = self.mobs.list.iter_mut().filter(|m| m.alive()).min_by(|a, b| (a.body.pos - f).length().total_cmp(&(b.body.pos - f).length())) {
                    if (m.body.pos - f).length() < 3.0 {
                        m.hurt(dmg, me);
                    }
                }
            }
        }
        if a > 0.0 {
            self.prog.stats.add("damage_taken", (a * 10.0) as u64);
        }
        a
    }

    // -----------------------------------------------------------------------------------------
    // Moving: speed effects, swift sneak, soul speed, depth strider, frost walker, levitation...

    pub(crate) fn pre_step(&mut self, sneaking: bool) {
        let mut m = self.move_multiplier();
        let ss = self.armor_ench(SWIFT_SNEAK) as f32;
        if sneaking && ss > 0.0 {
            m *= (0.3 + 0.15 * ss) / 0.3;
        }
        let soul = self.armor_ench(SOUL_SPEED) as f32;
        if soul > 0.0 {
            let below = self.world.block_or_air((self.body.pos - Vec3::Y * 0.2).floor().as_ivec3());
            let n = &self.reg.get(below).name;
            if n == "soul_sand" || n == "soul_soil" {
                m *= 1.3 + 0.105 * soul;
            }
        }
        self.body.speed_override = if (m - 1.0).abs() > 0.001 { Some(voxel::physics::WALK_SPEED * m) } else { None };
        self.prog.prev_ground = self.body.on_ground;
        if !self.body.on_ground {
            self.prog.mace_peak = self.prog.mace_peak.max(self.body.pos.y);
        } else {
            self.prog.mace_peak = self.body.pos.y;
        }
    }

    pub(crate) fn post_step(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let jb = self.effect_level(effects::JUMP_BOOST) as f32;
        if jb > 0.0 && self.prog.prev_ground && !self.body.on_ground && self.body.vel.y > 8.0 {
            self.body.vel.y += 1.8 * jb;
        }
        let lev = self.effect_level(effects::LEVITATION) as f32;
        if lev > 0.0 {
            let target = 0.9 * lev;
            self.body.vel.y += (target - self.body.vel.y) * (dt * 4.0).min(1.0);
            self.fall_peak = self.body.pos.y;
            self.prog.wind_from = self.body.pos.y.max(0.01);
        }
        if self.has_effect(effects::SLOW_FALLING) && self.body.vel.y < -1.6 {
            self.body.vel.y = -1.6;
            self.fall_peak = self.body.pos.y;
        }
        // In water: depth strider and dolphin's grace push the swim along.
        if self.body.in_water && !self.body.flying {
            let ds = self.armor_ench(DEPTH_STRIDER) as f32;
            let dg = self.has_effect(effects::DOLPHINS_GRACE);
            let k = 1.0 + ds / 3.0 * 0.8 + if dg { 1.2 } else { 0.0 };
            if k > 1.0 {
                let h = Vec3::new(self.body.vel.x, 0.0, self.body.vel.z);
                let cap = 2.2 * k;
                let boosted = h * (1.0 + (k - 1.0) * (dt * 6.0).min(1.0));
                let v = if boosted.length() > cap { boosted.normalize() * cap } else { boosted };
                self.body.vel.x = v.x;
                self.body.vel.z = v.z;
            }
        }
        // Frost walker: still water under the feet freezes for a while.
        let fw = self.armor_ench(FROST_WALKER) as i32;
        if fw > 0 && self.body.on_ground && !self.body.in_water {
            let water = self.reg.id_or_air("water");
            let ice = self.reg.id_or_air("frosted_ice");
            if ice != AIR {
                let c = (self.body.pos - Vec3::Y * 0.5).floor().as_ivec3();
                let r = 2 + fw;
                for dz in -r..=r {
                    for dx in -r..=r {
                        if dx * dx + dz * dz > r * r {
                            continue;
                        }
                        let p = c + IVec3::new(dx, 0, dz);
                        if self.world.block_or_air(p) == water && self.world.block_or_air(p + IVec3::Y) == AIR {
                            self.world.set_block(p, ice);
                            let t = 6.0 + self.prog.rng.f32() * 6.0;
                            self.prog.frost.push((p, t));
                        }
                    }
                }
            }
        }
        if self.prog.spin > 0.0 {
            self.prog.spin -= dt;
            self.fall_peak = self.body.pos.y;
        }
        // Statistics: distance on foot, by swimming, flying; jumps.
        let moved = (self.body.pos - self.prog.last_pos).length().min(4.0);
        self.prog.last_pos = self.body.pos;
        self.prog.stat_walk += moved;
        if self.prog.stat_walk >= 1.0 {
            let n = self.prog.stat_walk as u64;
            self.prog.stat_walk -= n as f32;
            let key = if self.body.in_water { "cm_swum" } else if self.body.flying { "cm_flown" } else if self.riding.is_some() { "cm_by_cart" } else { "cm_walked" };
            self.prog.stats.add(key, n * 100);
        }
        if self.prog.prev_ground && !self.body.on_ground && self.body.vel.y > 8.0 {
            self.prog.stats.add("jumps", 1);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Bows, crossbows, tridents, wind charges.

    /// Right click with a ranged weapon or a wind charge. True when it is one (the draw happens
    /// while the button is held, in `update_weapons`).
    pub(crate) fn weapon_use(&mut self, name: &str) -> bool {
        if crate::tr_weapons::bow_power(name).is_some() {
            return true;
        }
        match name {
            "bow" | "trident" => true,
            "crossbow" => {
                if self.prog.loaded && !self.prog.was_using {
                    self.fire_crossbow();
                }
                true
            }
            "wind_charge" => {
                if self.prog.wind_t > 0.0 {
                    return true;
                }
                self.prog.wind_t = 0.5;
                let f = self.forward();
                let from = self.body.eye() + f * 0.5;
                self.prog.shots.push(Shot::new(from, f * 24.0, ShotKind::WindCharge, 1.0));
                if self.survival {
                    let sel = self.selected;
                    self.surv.inv.take(sel, 1);
                }
                self.anim.start_swing();
                self.sfx(Sfx::Cast(5));
                true
            }
            _ => false,
        }
    }

    /// An arrow from the inventory: tipped arrows first if one is in hand's reach... the first
    /// arrow stack found, like vanilla (offhand, then hotbar, then the rest).
    fn find_arrow(&self) -> Option<(usize, Option<&'static PotionDef>)> {
        let off = self.surv.offhand.map(|s| self.items.get(s.item).name.clone());
        if let Some(n) = off {
            if n == "arrow" || n == "spectral_arrow" {
                return Some((usize::MAX, None));
            }
            if let Some((effects::Form::Arrow, p)) = effects::potion_of(&n) {
                return Some((usize::MAX, Some(p)));
            }
        }
        for (i, s) in self.surv.inv.slots.iter().enumerate() {
            let Some(s) = s else { continue };
            let n = &self.items.get(s.item).name;
            if n == "arrow" || n == "spectral_arrow" {
                return Some((i, None));
            }
            if let Some((effects::Form::Arrow, p)) = effects::potion_of(n) {
                return Some((i, Some(p)));
            }
        }
        None
    }

    fn use_arrow(&mut self, slot: usize) {
        if !self.survival {
            return;
        }
        if slot == usize::MAX {
            if let Some(s) = &mut self.surv.offhand {
                s.count -= 1;
                if s.count == 0 {
                    self.surv.offhand = None;
                }
            }
        } else {
            self.surv.inv.take(slot, 1);
        }
    }

    pub(crate) fn update_weapons(&mut self, dt: f32, use_held: bool) {
        self.prog.wind_t -= dt;
        self.prog.sweep = (self.prog.sweep - dt).max(0.0);
        let mut name = self.held_key();
        // Ore bows (tr_weapons.rs) draw like the plain bow.
        if crate::tr_weapons::bow_power(&name).is_some() {
            name = "bow".into();
        }
        let playing = self.screen == crate::Screen::Playing;
        let holding = use_held && playing;
        match name.as_str() {
            "bow" => {
                if holding {
                    if self.find_arrow().is_some() || !self.survival || self.held_ench(INFINITY) > 0 {
                        self.prog.draw += dt;
                    }
                } else if self.prog.was_using && self.prog.draw > 0.1 {
                    self.fire_bow();
                }
            }
            "crossbow" => {
                if holding && !self.prog.loaded {
                    if self.find_arrow().is_some() || !self.survival {
                        self.prog.draw += dt;
                        let need = (1.25 - 0.25 * self.held_ench(QUICK_CHARGE) as f32).max(0.1);
                        if self.prog.draw >= need {
                            self.prog.loaded = true;
                            self.prog.draw = 0.0;
                            if let Some((slot, _)) = self.find_arrow() {
                                self.use_arrow(slot);
                            }
                            self.sfx(Sfx::Piston(false));
                        }
                    }
                } else if !holding {
                    self.prog.draw = 0.0;
                }
            }
            "trident" => {
                if holding {
                    self.prog.draw += dt;
                } else if self.prog.was_using && self.prog.draw > 0.5 {
                    self.throw_trident();
                }
            }
            _ => self.prog.draw = 0.0,
        }
        if !holding && name != "crossbow" {
            self.prog.draw = if self.prog.was_using { 0.0 } else { self.prog.draw.min(0.0) };
        }
        self.prog.was_using = holding;
        self.update_weapon_shots(dt);
    }

    /// How far the bow is drawn (0..1), for the arm pose and the zoom.
    pub(crate) fn bow_pull(&self) -> f32 {
        let n = self.held_key();
        if n == "bow" || crate::tr_weapons::bow_power(&n).is_some() {
            (self.prog.draw / 1.0).min(1.0)
        } else if n == "crossbow" {
            if self.prog.loaded {
                1.0
            } else {
                (self.prog.draw / 1.25).min(1.0)
            }
        } else {
            0.0
        }
    }

    fn fire_bow(&mut self) {
        let pull = (self.prog.draw / 1.0).min(1.0);
        self.prog.draw = 0.0;
        let pull = (pull * pull + pull * 2.0) / 3.0;
        if pull < 0.1 {
            return;
        }
        let infinity = self.held_ench(INFINITY) > 0;
        let arrow = self.find_arrow();
        if self.survival && arrow.is_none() && !infinity {
            return;
        }
        let potion = arrow.and_then(|a| a.1);
        let mut pickup = self.survival;
        if let Some((slot, p)) = arrow {
            // The hunter's quiver sometimes saves the arrow (rpg.rs).
            let saved = self.t5.rpg.stats.ammo_save > 0.0 && self.surv.rng.f32() < self.t5.rpg.stats.ammo_save;
            if !(infinity && p.is_none()) && !saved {
                self.use_arrow(slot);
            } else {
                pickup = false;
            }
        }
        let f = self.forward();
        let mut s = Shot::new(self.body.eye() + f * 0.3, f * 60.0 * pull, ShotKind::Arrow { potion, pickup }, 0.0);
        let power = self.held_ench(POWER) as f32;
        let ranged = self.t5.rpg.stats.dmg[crate::rpg::RANGED];
        let ore = crate::tr_weapons::bow_power(&self.held_key()).unwrap_or(1.0);
        s.damage = (BOW_DAMAGE * pull) * if power > 0.0 { 1.0 + 0.25 * (power + 1.0) } else { 1.0 } * ranged * ore;
        s.crit = pull >= 1.0 || pull > 0.6 && self.surv.rng.f32() < self.t5.rpg.stats.crit[crate::rpg::RANGED];
        s.punch = self.held_ench(PUNCH);
        s.flame = self.held_ench(FLAME) > 0;
        self.prog.shots.push(s);
        self.wear_held(1);
        self.sfx(Sfx::Place(Material::Cloth));
        self.prog.stats.add("arrows_shot", 1);
    }

    fn fire_crossbow(&mut self) {
        self.prog.loaded = false;
        let f = self.forward();
        let multi = self.held_ench(MULTISHOT) > 0;
        let pierce = self.held_ench(PIERCING);
        let angles: &[f32] = if multi { &[-0.17, 0.0, 0.17] } else { &[0.0] };
        for &a in angles {
            let dir = Vec3::new(f.x * a.cos() - f.z * a.sin(), f.y, f.x * a.sin() + f.z * a.cos()).normalize_or_zero();
            let mut s = Shot::new(self.body.eye() + dir * 0.3, dir * 63.0, ShotKind::Arrow { potion: None, pickup: a == 0.0 && self.survival }, CROSSBOW_DAMAGE * self.t5.rpg.stats.dmg[crate::rpg::RANGED]);
            s.pierce = pierce;
            s.crit = self.surv.rng.f32() < self.t5.rpg.stats.crit[crate::rpg::RANGED];
            self.prog.shots.push(s);
        }
        self.wear_held(if multi { 3 } else { 1 });
        self.sfx(Sfx::Piston(true));
    }

    fn throw_trident(&mut self) {
        self.prog.draw = 0.0;
        let Some(st) = self.held_stack() else { return };
        let riptide = st.ench.get(RIPTIDE);
        let f = self.forward();
        if riptide > 0 {
            // Riptide: only in water or rain; the player is flung instead.
            if self.body.in_water || self.weather.rain > 0.3 {
                self.body.vel = f * (10.0 + 6.0 * riptide as f32);
                self.prog.spin = 1.0;
                self.sfx(Sfx::Splash(0.6));
                self.wear_held(1);
            }
            return;
        }
        let mut s = Shot::new(self.body.eye() + f * 0.4, f * 50.0, ShotKind::Trident { stack: st, returning: false }, 8.0);
        s.loyalty = st.ench.get(LOYALTY);
        s.channeling = st.ench.get(CHANNELING) > 0;
        s.impaling = st.ench.get(IMPALING);
        self.prog.shots.push(s);
        if self.survival {
            let sel = self.selected;
            self.surv.inv.slots[sel] = None;
        }
        self.sfx(Sfx::Cast(0));
    }

    fn update_weapon_shots(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let eye = self.body.eye();
        let mut i = 0;
        while i < self.prog.shots.len() {
            let mut remove = false;
            {
                let s = &mut self.prog.shots[i];
                s.age += dt;
                if s.age > 60.0 {
                    remove = true;
                }
            }
            // A loyal trident flies home.
            if let ShotKind::Trident { returning: true, stack } = self.prog.shots[i].kind {
                let s = &mut self.prog.shots[i];
                let to = eye - s.pos;
                s.stuck = false;
                s.vel = to.normalize_or_zero() * (20.0 + 6.0 * s.loyalty as f32);
                s.pos += s.vel * dt;
                if to.length() < 1.2 {
                    self.give_back_stack(stack);
                    remove = true;
                }
            } else if !self.prog.shots[i].stuck {
                let (pos, vel, kind) = {
                    let s = &self.prog.shots[i];
                    (s.pos, s.vel, s.kind)
                };
                let g = if matches!(kind, ShotKind::WindCharge) { 0.0 } else { 20.0 };
                let mut v = vel;
                v.y -= g * dt;
                let next = pos + v * dt;
                // Mobs on the way.
                let hit = self.mobs.pick(pos, v.normalize_or_zero(), (next - pos).length() + 0.3).filter(|(k, _)| !self.prog.shots[i].hit.contains(k));
                if let Some((k, _)) = hit {
                    remove = self.shot_hits_mob(i, k);
                } else if Terrain::solid(&self.world, next.floor().as_ivec3()) {
                    match kind {
                        ShotKind::WindCharge => {
                            self.wind_burst(pos, 1.0);
                            remove = true;
                        }
                        ShotKind::Trident { stack, .. } => {
                            let s = &mut self.prog.shots[i];
                            s.stuck = true;
                            if s.loyalty > 0 {
                                s.kind = ShotKind::Trident { stack, returning: true };
                            }
                        }
                        _ => {
                            let s = &mut self.prog.shots[i];
                            s.stuck = true;
                            s.vel = v * 0.001;
                            self.sfx(Sfx::Hit(Material::Wood));
                        }
                    }
                } else {
                    let s = &mut self.prog.shots[i];
                    s.pos = next;
                    s.vel = v;
                    if s.flame && (s.age * 10.0) as i32 % 2 == 0 {
                        let glow = self.glow_layer();
                        self.particles.glow(next, Vec3::ZERO, glow, [1.4, 0.6, 0.2], 0.05, 0.3);
                    }
                }
            }
            // Picking arrows and tridents back up.
            let s = &self.prog.shots[i];
            if s.stuck && (s.pos - (self.body.pos + Vec3::Y)).length() < 1.4 && !remove {
                match s.kind {
                    ShotKind::Arrow { pickup: true, potion } => {
                        let name = match potion {
                            Some(p) => format!("tipped_arrow_{}", p.key),
                            None => "arrow".to_string(),
                        };
                        if let Some(id) = self.items.id(&name) {
                            if self.surv.inv.add(&self.items, ItemStack::new(id, 1)).is_none() {
                                self.sfx(Sfx::Pickup);
                                remove = true;
                            }
                        }
                    }
                    ShotKind::Trident { stack, .. } => {
                        self.give_back_stack(stack);
                        remove = true;
                    }
                    ShotKind::Arrow { pickup: false, .. } if s.age > 1.0 => remove = true,
                    _ => {}
                }
            }
            if remove {
                self.prog.shots.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn give_back_stack(&mut self, stack: ItemStack) {
        if !self.survival {
            return;
        }
        let sel = self.selected;
        if self.surv.inv.slots[sel].is_none() {
            self.surv.inv.slots[sel] = Some(stack);
        } else if let Some(left) = self.surv.inv.add(&self.items, stack) {
            self.throw_stack(left);
        }
        self.sfx(Sfx::Pickup);
    }

    /// A shot reaches mob `k`. True when the shot is used up.
    fn shot_hits_mob(&mut self, i: usize, k: usize) -> bool {
        let eye = self.body.eye();
        let (kind, mut dmg, crit, punch, flame, pierce, channel, imp, vel) = {
            let s = &self.prog.shots[i];
            (s.kind, s.damage, s.crit, s.punch, s.flame, s.pierce, s.channeling, s.impaling, s.vel)
        };
        if crit {
            dmg += self.prog.rng.below((dmg / 2.0 + 2.0) as u32) as f32;
        }
        let name = self.mobs.reg.defs[self.mobs.list[k].def].name.clone();
        if let ShotKind::WindCharge = kind {
            let at = self.mobs.list[k].body.pos;
            self.wind_burst(at, 1.0);
            return true;
        }
        if matches!(kind, ShotKind::Trident { .. }) && (water_mob(&name) || self.mobs.list[k].body.in_water) {
            dmg += 2.5 * imp as f32;
        }
        self.orichalcum_petals(k, dmg);
        let m = &mut self.mobs.list[k];
        m.hurt(dmg, eye);
        if punch > 0 {
            m.body.knockback(vel, 3.0 * punch as f32);
        }
        if flame {
            m.on_fire = m.on_fire.max(5.0);
        }
        let at = m.body.pos;
        self.last_hit = Some((self.mobs.reg.defs[self.mobs.list[k].def].display_name.clone(), self.mobs.list[k].health.max(0.0), self.mobs.list[k].max_health, self.now + 3.0));
        self.sfx(Sfx::MobHit);
        if let ShotKind::Arrow { potion: Some(p), .. } = kind {
            // Tipped arrows give an eighth of the potion.
            let eff: Vec<(u8, u8, f32)> = p.effects.iter().map(|&(e, a, t)| (e, a, t / 8.0)).collect();
            self.potion_on_mob(k, &name, &eff, 1.0, eye);
        }
        if channel && self.weather.state.thunder {
            self.weather.flash = 1.0;
            self.sfx(Sfx::Thunder(1.0));
            let m = &mut self.mobs.list[k];
            m.hurt(5.0, at + Vec3::Y * 5.0);
            m.on_fire = m.on_fire.max(8.0);
        }
        self.prog.shots[i].hit.push(k);
        match kind {
            ShotKind::Trident { stack, .. } => {
                let s = &mut self.prog.shots[i];
                s.vel = -s.vel * 0.1;
                s.stuck = true;
                s.pos = at + Vec3::Y;
                if s.loyalty > 0 {
                    s.kind = ShotKind::Trident { stack, returning: true };
                }
                false
            }
            _ => {
                if pierce > 0 {
                    self.prog.shots[i].pierce -= 1;
                    false
                } else {
                    true
                }
            }
        }
    }

    /// A wind charge bursts: everything near is thrown away from it (the player upward too).
    pub(crate) fn wind_burst(&mut self, at: Vec3, power: f32) {
        self.sfx(Sfx::Cast(5));
        let glow = self.glow_layer();
        for k in 0..16 {
            let a = k as f32 * 0.39;
            self.particles.glow(at, Vec3::new(a.cos() * 4.0, 1.0, a.sin() * 4.0), glow, [0.85, 0.9, 1.2], 0.12, 0.5);
        }
        for m in self.mobs.list.iter_mut() {
            let d = m.body.pos - at;
            if m.alive() && d.length() < 3.0 {
                m.body.knockback(d, 6.0 * power);
                m.body.vel.y += 4.0 * power;
            }
        }
        let d = self.body.pos + Vec3::Y * 0.5 - at;
        if d.length() < 3.0 {
            let away = d.normalize_or_zero();
            self.body.vel += away * 8.0 * power + Vec3::Y * 9.0 * power;
            // A wind charge jump doesn't count the height it started from as a fall.
            self.fall_peak = self.body.pos.y;
        }
        // Light blocks get pushed: doors, trapdoors, levers flip (the game's toggle code).
    }

    /// The player's arrows, tridents and wind charges for drawing.
    pub(crate) fn shot_instances(&self, cam_block: IVec3) -> Vec<GpuItem> {
        let sprite = |name: &str| {
            self.items.id(name).and_then(|i| match self.items.get(i).look {
                voxel::item::Look::Sprite(l) => Some((self.item_base + l) as f32),
                _ => None,
            })
        };
        let c = cam_block.as_vec3();
        let mut out = Vec::new();
        for s in &self.prog.shots {
            let (layer, size) = match s.kind {
                ShotKind::Arrow { potion: Some(p), .. } => (sprite(&format!("tipped_arrow_{}", p.key)).or(sprite("arrow")), 0.5),
                ShotKind::Arrow { .. } => (sprite("arrow"), 0.5),
                ShotKind::Trident { .. } => (sprite("trident"), 1.1),
                ShotKind::WindCharge => (sprite("wind_charge"), 0.45),
            };
            let Some(layer) = layer else { continue };
            let d = s.vel.normalize_or_zero();
            let flat = (d.x * d.x + d.z * d.z).sqrt();
            let yaw = (-d.z).atan2(d.x);
            let roll = -std::f32::consts::FRAC_PI_4 + d.y.atan2(flat);
            let l = self.world.light(s.pos.floor().as_ivec3());
            let (sky, blk) = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let p = s.pos - c;
            if matches!(s.kind, ShotKind::WindCharge) {
                out.push(GpuItem { pos: [p.x, p.y, p.z, size], info: [s.age * 8.0, 0.0, sky, blk], layers: [layer, layer, layer, 0.0], rot: [0.0; 4], ..bytemuck::Zeroable::zeroed() });
                continue;
            }
            for pitch in [0.0, std::f32::consts::FRAC_PI_2] {
                out.push(GpuItem { pos: [p.x, p.y, p.z, size], info: [yaw, 1.0, sky, blk], layers: [layer, layer, layer, 0.0], rot: [pitch, roll, 0.0, 0.0], ..bytemuck::Zeroable::zeroed() });
            }
        }
        out
    }
}
