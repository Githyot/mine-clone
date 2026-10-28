//! Terraria's mounts and pets (Lote 11): items that call a creature to you instead of taming a
//! wild one. Right click with a mount item to climb on (sneak or click again to get off): a
//! giant slime that jumps high and takes the sting out of falls, a bee that flies for a while,
//! a big rabbit, an enchanted deer and a cosmaw that flies for as long as you like. Pet items
//! bring a little follower (a fox kit, a penguin chick...) and light pets a glowing one that
//! lights the way. Summoned creatures are ghosts to the rest of the game: nothing hurts them,
//! they hunt nothing and this file moves them.

use glam::Vec3;
use voxel::physics::{Body, Input, Terrain, WALK_SPEED};

use crate::audio::Sfx;
use crate::Game;

pub struct MountDef {
    pub item: &'static str,
    pub mob: &'static str,
    pub name: &'static str,
    /// Drawn this many times its kind's size.
    pub size: f32,
    /// Ground speed, times walking.
    pub speed: f32,
    /// Jumps this much higher than on foot.
    pub jump: f32,
    pub air_jumps: u8,
    /// Seconds of flight per take-off (0 = none, `f32::INFINITY` = as long as you like).
    pub flight: f32,
    /// Falls don't hurt.
    pub soft: bool,
}

pub const MOUNTS: &[MountDef] = &[
    MountDef { item: "slime_saddle", mob: "ooze", name: "Gosma Montaria", size: 1.6, speed: 1.3, jump: 1.6, air_jumps: 0, flight: 0.0, soft: true },
    MountDef { item: "honeyed_goggles", mob: "bee", name: "Abelha Gigante", size: 3.2, speed: 1.4, jump: 1.0, air_jumps: 0, flight: 5.0, soft: true },
    MountDef { item: "fuzzy_carrot", mob: "rabbit", name: "Coelho Gigante", size: 3.0, speed: 1.7, jump: 1.45, air_jumps: 1, flight: 0.0, soft: false },
    MountDef { item: "blessed_apple", mob: "deer", name: "Cervo Encantado", size: 1.25, speed: 2.4, jump: 1.2, air_jumps: 0, flight: 0.0, soft: false },
    MountDef { item: "cosmic_truffle", mob: "cosmaw", name: "Cosmandíbula", size: 1.1, speed: 1.8, jump: 1.0, air_jumps: 0, flight: f32::INFINITY, soft: true },
    // Lote 22 (tr3_world.rs).
    MountDef { item: "ancient_horn", mob: "mount_basilisk", name: "Basilisco", size: 1.0, speed: 2.7, jump: 1.1, air_jumps: 0, flight: 0.0, soft: false },
    MountDef { item: "witchs_broom", mob: "mount_broom", name: "Vassoura da Bruxa", size: 1.0, speed: 2.0, jump: 1.0, air_jumps: 0, flight: f32::INFINITY, soft: true },
    MountDef { item: "reindeer_bells", mob: "mount_reindeer", name: "Rena", size: 1.0, speed: 2.1, jump: 1.2, air_jumps: 0, flight: f32::INFINITY, soft: true },
    MountDef { item: "drill_containment_unit", mob: "mount_drill", name: "Broca Voadora", size: 1.0, speed: 1.5, jump: 1.0, air_jumps: 0, flight: f32::INFINITY, soft: true },
    MountDef { item: "scaly_truffle", mob: "mount_pigron", name: "Porco-Dragão", size: 1.0, speed: 1.9, jump: 1.0, air_jumps: 1, flight: f32::INFINITY, soft: true },
];

pub struct PetDef {
    pub item: &'static str,
    pub mob: &'static str,
    pub name: &'static str,
    pub size: f32,
    /// Flies beside the shoulder instead of trotting behind.
    pub flies: bool,
    /// A light pet: how bright (0..1) and its colour.
    pub light: f32,
    pub tint: [f32; 3],
}

pub const PETS: &[PetDef] = &[
    PetDef { item: "fox_bell", mob: "fox", name: "Filhote de Raposa", size: 0.55, flies: false, light: 0.0, tint: [1.0; 3] },
    PetDef { item: "frozen_fish", mob: "penguin", name: "Pinguinzinho", size: 0.6, flies: false, light: 0.0, tint: [1.0; 3] },
    PetDef { item: "golden_acorn", mob: "squirrel", name: "Esquilo Dourado", size: 0.8, flies: false, light: 0.0, tint: [1.0; 3] },
    PetDef { item: "spiny_apple", mob: "hedgehog", name: "Ouricinho", size: 0.8, flies: false, light: 0.0, tint: [1.0; 3] },
    PetDef { item: "axolotl_egg", mob: "axolotl", name: "Axolote de Estimação", size: 0.7, flies: false, light: 0.0, tint: [1.0; 3] },
    PetDef { item: "parrot_cracker", mob: "parrot", name: "Papagaio", size: 0.8, flies: true, light: 0.0, tint: [1.0; 3] },
    // Light pets.
    PetDef { item: "firefly_jar", mob: "firefly", name: "Vaga-Lume Amigo", size: 2.0, flies: true, light: 0.8, tint: [1.2, 1.3, 0.5] },
    PetDef { item: "allay_bell", mob: "allay", name: "Allay Luminoso", size: 0.9, flies: true, light: 0.9, tint: [0.6, 0.9, 1.5] },
    PetDef { item: "wisp_lantern", mob: "firefly", name: "Fogo-Fátuo da Lanterna", size: 2.6, flies: true, light: 1.0, tint: [0.5, 1.0, 1.6] },
];

pub fn mount_of(item: &str) -> Option<usize> {
    MOUNTS.iter().position(|m| m.item == item)
}

pub fn pet_of(item: &str) -> Option<usize> {
    PETS.iter().position(|p| p.item == item)
}

#[derive(Default)]
pub struct Companions {
    /// Riding: which mount, and its creature's net id.
    pub mount: Option<(usize, u32)>,
    flight_t: f32,
    air_jumps: u8,
    jump_was: bool,
    left_ground: bool,
    /// The pet and the light pet called out (kinds in `PETS`; saved), and their creatures.
    pub pet: Option<usize>,
    pub light: Option<usize>,
    pet_id: Option<u32>,
    light_id: Option<u32>,
}

impl Game {
    fn mob_by_id(&self, id: Option<u32>) -> Option<usize> {
        let id = id?;
        self.mobs.list.iter().position(|m| m.net_id == id && m.alive())
    }

    /// Calls a ghost creature: a summoned mount or pet.
    fn call_ghost(&mut self, mob: &str, size: f32, at: Vec3) -> Option<u32> {
        if !self.mobs.spawn(mob, at) {
            return None;
        }
        let def = &self.mobs.reg.defs[self.mobs.list.last()?.def];
        let (w, h) = (def.width * size, def.height * size);
        let m = self.mobs.list.last_mut()?;
        m.ghost = true;
        m.size = size;
        m.body = Body::sized(at, w.min(2.4), h.min(2.6));
        m.yaw = self.yaw + std::f32::consts::PI;
        Some(m.net_id)
    }

    fn sparkle(&mut self, at: Vec3, tint: [f32; 3]) {
        let glow = self.glow_layer();
        for _ in 0..18 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5) * 3.0;
            self.particles.glow(at, v, glow, tint, 0.09, 0.6);
        }
    }

    /// Right click with a mount or pet item. None when the item is neither.
    pub(crate) fn companion_use(&mut self, name: &str) -> Option<bool> {
        if let Some(k) = mount_of(name) {
            if self.t5.comp.mount.is_some_and(|(j, _)| j == k) {
                self.dismount_companion();
                return Some(true);
            }
            if self.riding.is_some() || self.seat.is_some() || self.mobs.list.iter().any(|m| m.ridden) || self.mods.mount.ridden.is_some() {
                return Some(false);
            }
            self.dismount_companion();
            let def = &MOUNTS[k];
            let at = self.body.pos;
            let Some(id) = self.call_ghost(def.mob, def.size, at) else {
                self.toast("Essa montaria não está disponível aqui", 2.0);
                return Some(true);
            };
            self.t5.comp.mount = Some((k, id));
            self.t5.comp.flight_t = def.flight.min(1e6);
            self.t5.comp.air_jumps = def.air_jumps;
            self.sparkle(at + Vec3::Y * 0.6, [1.2, 1.2, 1.3]);
            self.sfx(Sfx::Pickup);
            self.toast(format!("{} (agache para descer)", def.name), 2.5);
            return Some(true);
        }
        if let Some(k) = pet_of(name) {
            let light = PETS[k].light > 0.0;
            let slot = if light { &mut self.t5.comp.light } else { &mut self.t5.comp.pet };
            let was = *slot;
            *slot = if was == Some(k) { None } else { Some(k) };
            let now = *slot;
            // The old creature goes; the new one comes in update_companions.
            let old = if light { self.t5.comp.light_id.take() } else { self.t5.comp.pet_id.take() };
            if let Some(i) = self.mob_by_id(old) {
                let p = self.mobs.list[i].body.pos;
                self.mobs.list.remove(i);
                self.sparkle(p, [1.0, 1.0, 1.0]);
            }
            let msg = match now {
                Some(k) => format!("{} veio com você", PETS[k].name),
                None => format!("{} foi descansar", PETS[k].name),
            };
            self.toast(msg, 2.0);
            self.sfx(Sfx::Pickup);
            return Some(true);
        }
        None
    }

    pub(crate) fn dismount_companion(&mut self) {
        let Some((_, id)) = self.t5.comp.mount.take() else { return };
        if let Some(i) = self.mob_by_id(Some(id)) {
            let m = self.mobs.list.remove(i);
            let side = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
            self.body.pos = m.body.pos + side * (m.body.half_width + 0.6) + Vec3::Y * 0.1;
            self.body.vel = Vec3::ZERO;
            self.fall_peak = self.body.pos.y;
            self.sparkle(m.body.pos + Vec3::Y * 0.5, [1.2, 1.2, 1.3]);
        }
    }

    /// Riding a summoned mount: it runs, jumps and flies where the player steers. True while
    /// riding (the player's own step is skipped).
    pub(crate) fn companion_mount_step(&mut self, wish: Vec3, jump: bool, sneak: bool, dt: f32) -> bool {
        let Some((k, id)) = self.t5.comp.mount else { return false };
        let Some(i) = self.mob_by_id(Some(id)) else {
            self.t5.comp.mount = None;
            return false;
        };
        if sneak || self.screen == crate::Screen::Dead {
            self.dismount_companion();
            return false;
        }
        let def = &MOUNTS[k];
        let pressed = jump && !self.t5.comp.jump_was;
        self.t5.comp.jump_was = jump;
        let f = self.forward();
        let water = self.mobs.list[i].body.in_water;
        let ground = self.mobs.list[i].body.on_ground;
        let c = &mut self.t5.comp;
        if ground || water {
            c.air_jumps = def.air_jumps;
            c.flight_t = def.flight.min(1e6);
            c.left_ground = false;
        }
        let m = &mut self.mobs.list[i];
        m.yaw = f.x.atan2(f.z);
        m.body.speed_override = Some(WALK_SPEED * def.speed);
        let mut fly = false;
        if !ground && !water {
            if pressed && c.air_jumps > 0 {
                c.air_jumps -= 1;
                m.body.vel.y = 9.0 * def.jump;
            } else if jump && def.flight > 0.0 {
                if c.flight_t > 0.0 {
                    c.flight_t -= dt;
                    m.body.vel.y = (m.body.vel.y + 40.0 * dt).min(6.5);
                    fly = true;
                } else if m.body.vel.y < -2.5 {
                    // Out of flight: glide down.
                    m.body.vel.y = -2.5;
                }
            } else if def.flight.is_infinite() {
                // A tireless flier only sinks slowly when nothing is held.
                m.body.vel.y = m.body.vel.y.max(-3.0);
            }
        }
        let before = m.body.pos;
        let take_off = jump && ground;
        m.body.step(&self.world, &Input { wish, jump: take_off, sneak: false, sprint: false, crawl: false }, dt);
        // The jump that just left the ground goes higher than on foot.
        if take_off && !m.body.on_ground && !c.left_ground {
            m.body.vel.y *= def.jump;
            c.left_ground = true;
        }
        if def.soft || fly {
            m.body.fall_distance = 0.0;
        }
        let speed = ((m.body.pos - before) * Vec3::new(1.0, 0.0, 1.0)).length() / dt.max(1e-4);
        m.walk_speed += (speed - m.walk_speed) * (dt * 10.0).min(1.0);
        m.walk += m.walk_speed * dt * 2.2 / def.size.sqrt();
        m.health = m.max_health;
        let seat = m.body.height * 0.8;
        let (pos, vel, hard_landing) = (m.body.pos, m.body.vel, m.body.fall_distance);
        if hard_landing > 0.0 {
            m.body.fall_distance = 0.0;
        }
        self.body.pos = pos + Vec3::Y * seat;
        self.body.vel = vel;
        self.body.on_ground = true;
        self.body.fall_distance = 0.0;
        self.fall_peak = if def.soft || fly { self.body.pos.y } else { self.fall_peak.max(self.body.pos.y) };
        if fly && self.surv.rng.f32() < dt * 12.0 {
            let glow = self.glow_layer();
            self.particles.glow(pos + Vec3::Y * 0.3, Vec3::new(0.0, -1.5, 0.0), glow, [1.3, 1.2, 0.6], 0.07, 0.4);
        }
        true
    }

    /// Flight left on the mount, as a fraction (for the HUD bar); None when not flying one.
    pub(crate) fn mount_flight_left(&self) -> Option<f32> {
        let (k, _) = self.t5.comp.mount?;
        let def = &MOUNTS[k];
        (def.flight > 0.0 && def.flight.is_finite()).then(|| (self.t5.comp.flight_t / def.flight).clamp(0.0, 1.0))
    }

    /// Pets come when called and stay at heel; the mount goes if the world changes under it.
    pub(crate) fn update_companions(&mut self, dt: f32) {
        if dt == 0.0 {
            return;
        }
        if self.screen == crate::Screen::Dead || !self.mobs.enabled {
            self.t5.comp.pet_id = None;
            self.t5.comp.light_id = None;
            self.t5.comp.mount = None;
            return;
        }
        if let Some((_, id)) = self.t5.comp.mount {
            if self.mob_by_id(Some(id)).is_none() {
                self.t5.comp.mount = None;
            }
        }
        let me = self.body.pos;
        for light in [false, true] {
            let (kind, id) = if light { (self.t5.comp.light, self.t5.comp.light_id) } else { (self.t5.comp.pet, self.t5.comp.pet_id) };
            let Some(k) = kind else { continue };
            let def = &PETS[k];
            let i = match self.mob_by_id(id) {
                Some(i) => i,
                None => {
                    let side = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
                    let at = me + side * 1.2 + Vec3::Y * if def.flies { 1.8 } else { 0.3 };
                    let new = self.call_ghost(def.mob, def.size, at);
                    if light {
                        self.t5.comp.light_id = new;
                    } else {
                        self.t5.comp.pet_id = new;
                    }
                    if new.is_some() {
                        self.sparkle(at, def.tint);
                    }
                    continue;
                }
            };
            let t = self.now;
            let m = &mut self.mobs.list[i];
            m.health = m.max_health;
            if def.flies {
                // Beside the shoulder, bobbing; the light pet on the other side.
                let s = if light { -1.0 } else { 1.0 };
                let side = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin()) * s;
                let back = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
                let goal = me + side * 0.9 + back * 0.6 + Vec3::Y * (1.9 + (t * 2.1 + m.phase).sin() * 0.15);
                let to = goal - m.body.pos;
                if to.length() > 24.0 {
                    m.body.pos = goal;
                }
                let v = to * (dt * 4.0).min(1.0) / dt;
                m.body.pos += v * dt;
                m.body.vel = v;
                let flat = Vec3::new(v.x, 0.0, v.z);
                m.yaw = if flat.length() > 0.8 { flat.x.atan2(flat.z) } else { self.yaw + std::f32::consts::PI };
                m.walk_speed = flat.length();
                m.walk += dt * 6.0;
            } else {
                let to = me - m.body.pos;
                let flat = Vec3::new(to.x, 0.0, to.z);
                if flat.length() > 20.0 || to.y.abs() > 8.0 {
                    let back = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
                    m.body.pos = me + back * 1.2 + Vec3::Y * 0.3;
                    m.body.vel = Vec3::ZERO;
                }
                let wish = if flat.length() > 2.2 { flat.normalize() } else { Vec3::ZERO };
                let run = flat.length() > 6.0;
                let blocked = m.body.on_ground && wish != Vec3::ZERO && self.world.solid((m.body.pos + wish * 0.6 + Vec3::Y * 0.2).floor().as_ivec3());
                m.body.speed_override = Some(WALK_SPEED * if run { 1.6 } else { 1.0 });
                let before = m.body.pos;
                m.body.step(&self.world, &Input { wish, jump: blocked, sneak: false, sprint: run, crawl: false }, dt);
                if wish != Vec3::ZERO {
                    m.yaw = wish.x.atan2(wish.z);
                }
                let speed = ((m.body.pos - before) * Vec3::new(1.0, 0.0, 1.0)).length() / dt.max(1e-4);
                m.walk_speed += (speed - m.walk_speed) * (dt * 10.0).min(1.0);
                m.walk += m.walk_speed * dt * 2.2 / def.size.max(0.3);
            }
        }
    }

    /// Light pets light the way like a held torch (`dynamic_lights` picks them up).
    pub(crate) fn companion_lights(&self) -> Vec<(Vec3, f32)> {
        let Some(k) = self.t5.comp.light else { return Vec::new() };
        match self.mob_by_id(self.t5.comp.light_id) {
            Some(i) => vec![(self.mobs.list[i].body.pos + Vec3::Y * 0.2, PETS[k].light)],
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounts_and_pets_are_creatures_of_the_game() {
        let reg = crate::mobs::registry_for_tests();
        for m in MOUNTS {
            assert!(reg.id(m.mob).is_some(), "mount {} needs mob {}", m.item, m.mob);
        }
        for p in PETS {
            assert!(reg.id(p.mob).is_some(), "pet {} needs mob {}", p.item, p.mob);
        }
        assert_eq!(PETS.iter().filter(|p| p.light > 0.0).count(), 3);
    }
}

impl Game {
    /// Test scenes (MC_LOTE11=name, with --survival): wings, a mount, pets, vanity and dyes,
    /// sentries at work.
    pub(crate) fn test_lote11(&mut self, scene: &str) {
        use voxel::item::ItemStack;
        if !self.t5.loaded {
            self.t5_load();
        }
        self.survival = true;
        let id = |g: &Game, n: &str| g.items.id(n).map(|i| ItemStack::new(i, 1));
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        let wings = std::env::var("MC_WINGS").unwrap_or_else(|_| "celestial_wings".into());
        match scene {
            "wings" | "glide" | "folded" => {
                self.t5.acc[0] = id(self, &wings);
                for (k, n) in ["tuxedo_helmet", "tuxedo_chestplate", "tuxedo_leggings", "tuxedo_boots"].iter().enumerate() {
                    self.surv.armor[k] = id(self, n);
                }
                if scene != "folded" {
                    self.body.pos.y += 3.0;
                    self.body.on_ground = false;
                    self.t5.moves.flying = scene == "wings";
                    self.t5.moves.wing_t = if scene == "wings" { 2.0 } else { 0.0 };
                }
            }
            "mount" => {
                let item = std::env::var("MC_MOUNT").unwrap_or_else(|_| "honeyed_goggles".into());
                self.companion_use(&item);
                if let Some(i) = self.mob_by_id(self.t5.comp.mount.map(|m| m.1)) {
                    let m = &self.mobs.list[i];
                    self.body.pos = m.body.pos + Vec3::Y * m.body.height * 0.8;
                }
            }
            "pets" => {
                self.companion_use("fox_bell");
                self.companion_use("firefly_jar");
                for _ in 0..3 {
                    self.update_companions(0.05);
                }
                // In front of the camera for the picture.
                let side = flat.cross(Vec3::Y);
                for (k, id) in [self.t5.comp.pet_id, self.t5.comp.light_id].into_iter().enumerate() {
                    if let Some(i) = self.mob_by_id(id) {
                        let at = self.body.pos + flat * 3.0 + side * (k as f32 * 1.6 - 0.8);
                        let y = if k == 0 { ground(self, at) } else { self.body.eye().y };
                        let m = &mut self.mobs.list[i];
                        m.body.pos = Vec3::new(at.x, y, at.z);
                        m.yaw = (-flat).x.atan2(-flat.z);
                    }
                }
            }
            "vanity" | "vanity_inv" => {
                for (k, n) in ["iron_helmet", "iron_chestplate", "iron_leggings", "iron_boots"].iter().enumerate() {
                    self.surv.armor[k] = id(self, n);
                }
                for (k, n) in ["pirate_helmet", "royal_chestplate", "pirate_leggings", "explorer_boots"].iter().enumerate() {
                    self.t5.look.vanity[k] = id(self, n);
                }
                for (k, n) in ["red_tint", "", "black_tint", "", "rainbow_tint"].iter().enumerate() {
                    self.t5.look.dyes[k] = id(self, n);
                }
                self.t5.acc[0] = id(self, &wings);
                let items = ["red_tint", "blue_tint", "rainbow_tint", "flame_tint", "bright_green_tint", "tuxedo_chestplate", "royal_helmet", "fairy_wings", "slime_saddle", "fox_bell", "firefly_jar", "ballista_cane"];
                for (k, n) in items.iter().enumerate() {
                    self.surv.inv.slots[k] = id(self, n);
                }
                if scene == "vanity_inv" {
                    self.args.show_inventory = true;
                }
            }
            "sentries" => {
                self.magic.mana = 200.0;
                self.add_effect(crate::effects::SUMMONING, 0, 300.0, false);
                self.t5.rpg.stats = self.compute_stats();
                self.target = None;
                let side = flat.cross(Vec3::Y);
                // MC_SENTRIES=all: every kind (and a fourth slot for them).
                let all = std::env::var("MC_SENTRIES").is_ok();
                let staffs: &[&str] = if all { &["ember_tower_staff", "frost_crystal_staff", "storm_totem_staff", "ballista_cane"] } else { &["ember_tower_staff", "frost_crystal_staff"] };
                for (k, s) in staffs.iter().enumerate() {
                    if all {
                        self.t5.sentries.push(crate::sentries::Sentry::test(crate::sentries::sentry_of(s).unwrap_or(0)));
                    } else {
                        self.sentry_use(s);
                    }
                    let at = self.body.pos + flat * if all { 6.0 } else { 4.0 } + side * (k as f32 * 2.4 - (staffs.len() - 1) as f32 * 1.2);
                    let y = ground(self, at);
                    if let Some(st) = self.t5.sentries.last_mut() {
                        st.pos = Vec3::new(at.x.floor() + 0.5, y, at.z.floor() + 0.5);
                    }
                }
                let at = self.body.pos + flat * 10.0;
                for n in ["zombie", "skeleton", "zombie"] {
                    let y = ground(self, at);
                    self.mobs.spawn(n, Vec3::new(at.x, y, at.z));
                }
                if all {
                    // More than the slots allow: no updates (they'd send the extra home); aim
                    // them by hand, each caught just after a shot.
                    let aim = Vec3::new(at.x, ground(self, at) + 1.0, at.z);
                    for st in self.t5.sentries.iter_mut() {
                        st.test_aim(aim, 0.15);
                    }
                } else {
                    for _ in 0..30 {
                        self.update_sentries(0.05);
                    }
                }
            }
            _ => log::warn!("MC_LOTE11: unknown scene {scene}"),
        }
        self.t5.rpg.stats = self.compute_stats();
    }
}
