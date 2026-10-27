//! What the vanilla creatures of lote 2 (`plan/o-que-falta.md`) do beyond walking about. Their
//! models and skins come from tools/gen_lote2_mobs.py. Silverfish creep out of infested stone,
//! spawners in old mines keep making cave spiders, cave spiders and bogged poison, mooshrooms
//! give stew and mushrooms, zombie villagers are cured with a golden apple, allays fetch the item
//! you show them, snow golems are built from snow and a pumpkin and throw snowballs, hoglins rot
//! into zoglins outside the Nether, tadpoles grow into frogs, invokers call vexes, elder
//! guardians tire the miner, the creaking only moves while nobody looks, and lightning in a storm
//! now and then leaves skeleton horses behind.

use std::collections::HashMap;

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::physics::Terrain;
use voxel::AIR;

use crate::audio::{Material, Sfx};
use crate::Game;

/// What the creatures of this file remember between ticks.
#[derive(Default)]
pub struct VMobs {
    t: f32,
    /// Allays and the item each one fetches (by the mob's net id); what it carries now.
    pub allay_wants: HashMap<u32, u16>,
    allay_carry: HashMap<u32, ItemStack>,
    /// Zombie villagers being cured: seconds left.
    curing: HashMap<u32, f32>,
    /// Snow golems' throw timers, invokers' vex timers.
    cd: HashMap<u32, f32>,
}

impl Game {
    fn mob_named(&self, i: usize) -> &str {
        &self.mobs.reg.defs[self.mobs.list[i].def].name
    }

    fn swap_mob(&mut self, i: usize, to: &str) -> bool {
        let Some(d) = self.mobs.reg.id(to) else { return false };
        let m = &mut self.mobs.list[i];
        m.def = d;
        m.health = self.mobs.reg.defs[d].health;
        m.max_health = m.health;
        m.body.speed_override = Some(self.mobs.reg.defs[d].speed);
        true
    }

    pub(crate) fn update_vanilla_mobs(&mut self, dt: f32) {
        self.creaking_stare();
        self.allays(dt);
        self.vm.t += dt;
        for v in self.vm.cd.values_mut() {
            *v -= dt;
        }
        let mut cured = Vec::new();
        for (id, t) in self.vm.curing.iter_mut() {
            *t -= dt;
            if *t <= 0.0 {
                cured.push(*id);
            }
        }
        for id in cured {
            self.vm.curing.remove(&id);
            if let Some(i) = self.mobs.list.iter().position(|m| m.net_id == id && m.alive()) {
                if self.swap_mob(i, "settler_farmer") {
                    let at = self.mobs.list[i].body.pos + Vec3::Y;
                    let glow = self.glow_layer();
                    for k in 0..16 {
                        let a = k as f32 * 0.4;
                        self.particles.glow(at, Vec3::new(a.cos(), 1.5, a.sin()), glow, [1.1, 0.9, 0.3], 0.06, 1.2);
                    }
                    self.toast("O aldeão foi curado", 2.5);
                    self.prog.event(crate::progression::Event::Token("cured_zombie".into()));
                }
            }
        }
        self.snow_golems();
        self.invokers_call_vexes();
        if self.vm.t < 2.0 {
            return;
        }
        self.vm.t = 0.0;
        self.spawners();
        let overworld = self.dim == voxel::worldgen::Dim::Overworld;
        let me = self.body.pos;
        let mut swaps = Vec::new();
        let mut fatigue = false;
        for (i, m) in self.mobs.list.iter_mut().enumerate() {
            if !m.alive() {
                continue;
            }
            let name = self.mobs.reg.defs[m.def].name.as_str();
            match name {
                // Out of the Nether a hoglin turns into a zoglin after a while.
                "hoglin" if overworld && m.age > 15.0 => swaps.push((i, "zoglin")),
                "tadpole" if m.age > 1200.0 => swaps.push((i, "frog")),
                // Vexes only live for a short time.
                "vex" if m.age > 30.0 => {
                    m.health = 0.0;
                }
                "elder_guardian" if m.body.pos.distance(me) < 50.0 => fatigue = true,
                _ => {}
            }
        }
        for (i, to) in swaps {
            self.swap_mob(i, to);
        }
        if fatigue && self.survival && !self.has_effect(crate::effects::MINING_FATIGUE) {
            self.add_effect(crate::effects::MINING_FATIGUE, 2, 300.0, false);
            self.toast("Um guardião ancião te amaldiçoou", 2.5);
        }
    }

    // ---------------------------------------------------------------------------------------
    // Spawners and infested stone.

    /// Monster spawners near the player make their mob every so often: cave spiders when
    /// cobwebs hang around them (the old mines), otherwise zombies, skeletons or spiders.
    fn spawners(&mut self) {
        let spawner = self.reg.id_or_air("spawner");
        let web = self.reg.id_or_air("cobweb");
        if spawner == AIR || self.mobs.peaceful {
            return;
        }
        let c = self.body.pos.floor().as_ivec3();
        let mut found = Vec::new();
        for y in -8..=8 {
            for z in -16..=16 {
                for x in -16..=16 {
                    let p = c + IVec3::new(x, y, z);
                    if self.world.block_or_air(p) == spawner {
                        found.push(p);
                    }
                }
            }
        }
        for p in found {
            if self.surv.rng.f32() > 0.2 {
                continue;
            }
            let near = self.mobs.list.iter().filter(|m| m.alive() && m.body.pos.distance(p.as_vec3()) < 9.0).count();
            if near >= 6 {
                continue;
            }
            let webs = (-2..=2).flat_map(|y| (-2..=2).flat_map(move |z| (-2..=2).map(move |x| IVec3::new(x, y, z)))).any(|d| self.world.block_or_air(p + d) == web);
            let kinds = if webs { ["cave_spider"; 3] } else { ["zombie", "skeleton", "spider"] };
            let kind = kinds[(p.x ^ p.z).rem_euclid(3) as usize];
            for _ in 0..1 + self.surv.rng.below(3) {
                let off = IVec3::new(self.surv.rng.below(7) as i32 - 3, 0, self.surv.rng.below(7) as i32 - 3);
                let at = p + off;
                if !self.world.solid(at) && !self.world.solid(at + IVec3::Y) && self.world.solid(at - IVec3::Y) {
                    self.mobs.spawn(kind, at.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
                    let glow = self.glow_layer();
                    self.particles.glow(at.as_vec3() + Vec3::splat(0.5), Vec3::Y, glow, [0.9, 0.4, 0.2], 0.08, 0.8);
                }
            }
        }
    }

    /// A broken infested block lets its silverfish out.
    pub(crate) fn infested_broken(&mut self, p: IVec3, name: &str) {
        if name.starts_with("infested_") && !self.mobs.peaceful {
            self.mobs.spawn("silverfish", p.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
        }
    }

    // ---------------------------------------------------------------------------------------
    // Hits on the player.

    /// A hit from `from` just landed: cave spiders poison; arrows from the bogged too.
    pub(crate) fn mob_hit_effects(&mut self, from: Option<Vec3>) {
        let Some(f) = from else { return };
        if !self.survival {
            return;
        }
        let near = self.mobs.list.iter().enumerate().filter(|(_, m)| m.alive()).map(|(i, m)| (i, m.body.center().distance(f))).min_by(|a, b| a.1.total_cmp(&b.1));
        let melee = near.filter(|&(_, d)| d < 2.5).map(|(i, _)| i);
        use crate::effects::POISON;
        match melee {
            Some(i) if self.mob_named(i) == "cave_spider" => self.add_effect(POISON, 0, 7.0, false),
            None => {
                // An arrow: poisoned if the closest archer is a bogged.
                let me = self.body.pos;
                let archer = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].ranged && m.body.pos.distance(me) < 24.0).min_by(|a, b| a.body.pos.distance(me).total_cmp(&b.body.pos.distance(me)));
                if archer.is_some_and(|m| self.mobs.reg.defs[m.def].name == "bogged") {
                    self.add_effect(POISON, 0, 4.0, false);
                }
            }
            _ => {}
        }
    }

    // ---------------------------------------------------------------------------------------
    // Right click on creatures.

    /// The held item on a creature of this file. True when something happened.
    pub(crate) fn vanilla_mob_use(&mut self) -> bool {
        let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), 4.0) else { return false };
        if self.target.is_some_and(|h| h.distance < t && self.reg.get(self.world.block_or_air(h.block)).solid) {
            return false;
        }
        if !self.mobs.list[i].alive() {
            return false;
        }
        let name = self.mob_named(i).to_string();
        let held = self.held_key();
        let at = self.mobs.list[i].body.pos + Vec3::Y;
        match (name.as_str(), held.as_str()) {
            ("mooshroom", "bowl") => {
                if let Some(s) = self.items.id("mushroom_stew") {
                    let sel = self.selected;
                    if self.survival && self.surv.inv.slots[sel].is_some_and(|st| st.count > 1) {
                        self.surv.inv.take(sel, 1);
                        if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(s, 1)) {
                            self.throw_stack(left);
                        }
                    } else {
                        self.surv.inv.slots[sel] = Some(ItemStack::new(s, 1));
                    }
                }
                self.sfx(Sfx::Splash(0.15));
            }
            ("mooshroom", "shears") => {
                self.swap_mob(i, "cow");
                if let Some(m) = self.items.id("red_mushroom") {
                    self.pop_drop(at, ItemStack::new(m, 5));
                }
                self.sfx(Sfx::Place(Material::Cloth));
                self.wear_held(1);
            }
            ("zombie_villager", "golden_apple") => {
                let id = self.mobs.list[i].net_id;
                if self.vm.curing.contains_key(&id) {
                    return false;
                }
                self.vm.curing.insert(id, 5.0);
                self.take_one_held_item();
                let glow = self.glow_layer();
                for k in 0..12 {
                    let a = k as f32 * 0.5;
                    self.particles.glow(at + Vec3::new(a.cos() * 0.4, 0.3, a.sin() * 0.4), Vec3::Y * 0.6, glow, [1.0, 0.6, 0.2], 0.06, 2.5);
                }
                self.sfx(Sfx::Fizz);
            }
            ("allay", _) if !held.is_empty() => {
                let Some(st) = self.held_stack() else { return false };
                self.vm.allay_wants.insert(self.mobs.list[i].net_id, st.item);
                let n = self.items.get(st.item).display_name.clone();
                self.toast(format!("O allay vai buscar: {n}"), 2.5);
            }
            ("armor_stand", _) => return false,
            _ => return false,
        }
        self.anim.start_swing();
        true
    }

    fn take_one_held_item(&mut self) {
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
    }

    // ---------------------------------------------------------------------------------------
    // Allays.

    /// An allay flies to the nearest dropped item like the one it was shown, carries it back
    /// and drops it at the player's feet.
    fn allays(&mut self, _dt: f32) {
        if self.vm.allay_wants.is_empty() {
            return;
        }
        let me = self.body.pos;
        let ids: Vec<(u32, u16)> = self.vm.allay_wants.iter().map(|(&a, &b)| (a, b)).collect();
        for (id, want) in ids {
            let Some(i) = self.mobs.list.iter().position(|m| m.net_id == id && m.alive()) else {
                self.vm.allay_wants.remove(&id);
                self.vm.allay_carry.remove(&id);
                continue;
            };
            let pos = self.mobs.list[i].body.pos;
            if let Some(st) = self.vm.allay_carry.get(&id).copied() {
                if pos.distance(me) < 2.5 {
                    self.vm.allay_carry.remove(&id);
                    self.spawn_drop(pos + Vec3::Y * 0.5, st, (me - pos).normalize_or_zero() * 1.5);
                    self.sfx(Sfx::Pickup);
                } else {
                    self.mobs.list[i].go_to(me, 0.5);
                }
                continue;
            }
            let found = self.surv.drops.iter().enumerate().filter(|(_, d)| d.stack.item == want && d.body.pos.distance(pos) < 32.0).min_by(|a, b| a.1.body.pos.distance(pos).total_cmp(&b.1.body.pos.distance(pos)));
            if let Some((k, d)) = found {
                if d.body.pos.distance(pos) < 1.6 {
                    let st = self.surv.drops.remove(k).stack;
                    self.vm.allay_carry.insert(id, st);
                } else {
                    let to = d.body.pos;
                    self.mobs.list[i].go_to(to, 0.2);
                }
            } else if pos.distance(me) > 6.0 {
                self.mobs.list[i].go_to(me, 1.0);
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Snow golems.

    /// A carved pumpkin (or jack o'lantern) set on two blocks of snow comes alive.
    pub(crate) fn try_build_snow_golem(&mut self, p: IVec3, id: voxel::BlockId) {
        let base = self.reg.get(self.reg.get(id).base).name.clone();
        if base != "carved_pumpkin" && base != "jack_o_lantern" {
            return;
        }
        let snow = self.reg.id_or_air("snow_block");
        if snow == AIR || self.world.block_or_air(p - IVec3::Y) != snow || self.world.block_or_air(p - IVec3::Y * 2) != snow {
            return;
        }
        for k in 0..3 {
            self.world.set_block(p - IVec3::Y * k, AIR);
        }
        self.mobs.spawn("snow_golem", (p - IVec3::Y * 2).as_vec3() + Vec3::new(0.5, 0.0, 0.5));
        let (layer, tint) = self.block_look(snow);
        self.particles.block_bits(p - IVec3::Y, layer, tint, 20, 1.2);
        self.sfx(Sfx::Break(Material::Snow));
    }

    /// Snow golems leave snow where they walk, melt in hot places and throw snowballs at
    /// monsters close by.
    fn snow_golems(&mut self) {
        let Some(sb) = self.items.id("snowball") else { return };
        let layer = self.reg.id_or_air("snow_layer");
        let mut throws = Vec::new();
        for (i, m) in self.mobs.list.iter().enumerate() {
            if !m.alive() || self.mobs.reg.defs[m.def].name != "snow_golem" {
                continue;
            }
            let feet = m.body.pos.floor().as_ivec3();
            if layer != AIR && self.world.block_or_air(feet) == AIR && self.world.solid(feet - IVec3::Y) && m.walk_speed > 0.3 {
                throws.push((i, None, Some(feet)));
            }
            if self.vm.cd.get(&m.net_id).is_some_and(|&t| t > 0.0) {
                continue;
            }
            let target = self
                .mobs
                .list
                .iter()
                .filter(|o| o.alive() && self.mobs.reg.defs[o.def].kind == voxel::mob::Kind::Hostile && o.body.pos.distance(m.body.pos) < 10.0)
                .map(|o| o.body.center())
                .min_by(|a, b| a.distance(m.body.pos).total_cmp(&b.distance(m.body.pos)));
            if let Some(t) = target {
                throws.push((i, Some(t), None));
            }
        }
        for (i, target, snow_at) in throws {
            if let Some(p) = snow_at {
                let b = self.world.gen.base_biome_at(p.x, p.z);
                if matches!(b.name(), "Desert" | "Savanna" | "Badlands" | "Jungle") || self.dim == voxel::worldgen::Dim::Nether {
                    // Too hot: it melts.
                    let m = &mut self.mobs.list[i];
                    m.hurt(1.0, m.body.pos);
                } else {
                    self.world.set_block(p, layer);
                }
            }
            if let Some(t) = target {
                let m = &self.mobs.list[i];
                let from = m.body.pos + Vec3::Y * 1.5;
                let d = t - from;
                let flat = Vec3::new(d.x, 0.0, d.z).length();
                let speed = 16.0;
                let tt = (flat / speed).max(0.05);
                let vel = Vec3::new(d.x / tt, d.y / tt + 10.0 * tt, d.z / tt);
                let id = m.net_id;
                self.vm.cd.insert(id, 1.2);
                self.throw_from(sb, from + d.normalize_or_zero() * 0.6, vel);
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Invokers, the creaking, lightning.

    /// An invoker fighting the player calls three vexes now and then.
    fn invokers_call_vexes(&mut self) {
        let me = self.body.pos;
        let mut calls = Vec::new();
        for m in &self.mobs.list {
            if m.alive() && self.mobs.reg.defs[m.def].name == "invoker" && m.body.pos.distance(me) < 16.0 && self.vm.cd.get(&m.net_id).is_none_or(|&t| t <= 0.0) {
                calls.push((m.net_id, m.body.pos));
            }
        }
        for (id, at) in calls {
            self.vm.cd.insert(id, 20.0);
            let vexes = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == "vex").count();
            if vexes >= 6 || !self.survival {
                continue;
            }
            for k in 0..3 {
                let a = k as f32 * 2.1;
                self.mobs.spawn("vex", at + Vec3::new(a.cos(), 2.0, a.sin()));
            }
            self.sfx(Sfx::Cast(3));
        }
    }

    /// The creaking stands still while the player looks at it.
    fn creaking_stare(&mut self) {
        let eye = self.body.eye();
        let look = self.forward();
        for m in self.mobs.list.iter_mut() {
            let def = &self.mobs.reg.defs[m.def];
            if def.name != "creaking" {
                continue;
            }
            let to = (m.body.center() - eye).normalize_or_zero();
            let seen = to.dot(look) > 0.55 && m.body.pos.distance(eye) < 48.0;
            m.body.speed_override = Some(if seen { 0.0 } else { def.speed });
            if seen {
                m.body.vel.x = 0.0;
                m.body.vel.z = 0.0;
            }
        }
    }

    /// Lightning in a storm: one time in twenty it leaves a skeleton trap behind, four
    /// skeleton horses with their riders standing in the rain.
    pub(crate) fn lightning_trap(&mut self) {
        if self.dim != voxel::worldgen::Dim::Overworld || self.surv.rng.f32() > 0.05 || self.mobs.peaceful {
            return;
        }
        let a = self.surv.rng.f32() * std::f32::consts::TAU;
        let d = 16.0 + self.surv.rng.f32() * 16.0;
        let x = (self.body.pos.x + a.cos() * d).floor() as i32;
        let z = (self.body.pos.z + a.sin() * d).floor() as i32;
        let Some(y) = self.world.surface_y(x, z) else { return };
        let at = Vec3::new(x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5);
        for k in 0..4 {
            let o = Vec3::new((k % 2) as f32 * 1.6 - 0.8, 0.0, (k / 2) as f32 * 1.6 - 0.8);
            self.mobs.spawn("skeleton_horse", at + o);
            self.mobs.spawn("skeleton", at + o + Vec3::Y * 0.2);
        }
    }
}
