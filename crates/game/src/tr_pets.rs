//! Lote 5 of the Terraria side (plan/terraria-completo.md): things that carry you or keep you
//! company.
//! - Wings of many kinds (accessories.rs flies them; this file knows how long and in what
//!   colour, and draws them on the player's back in third person).
//! - Item mounts: a saddle, goggles or an apple calls a jelly that bounces, a bee that flies
//!   or a unicorn that runs monsters down. Get off (Shift) and it goes away.
//! - Pets and light pets: use the item to call one, use it again to send it home. Light pets
//!   light the way (survival.rs asks `light_of`).
//! - Souls drop in the World Breaker age; harpies roam the high sky, where you fall slower.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::physics::GRAVITY;
use voxel::worldgen::Dim;
use winit::keyboard::KeyCode;

use crate::audio::Sfx;
use crate::item_render::GpuItem;
use crate::Game;

/// (item, seconds of flight, particle tint)
const WINGS: &[(&str, f32, [f32; 3])] = &[
    ("empress_wings", 3.6, [1.6, 1.2, 1.8]),
    ("tide_wings", 3.2, [0.5, 1.4, 1.5]),
    ("crystal_wings", 3.0, [1.5, 1.0, 1.7]),
    ("frost_wings", 2.6, [0.8, 1.3, 2.0]),
];

/// (item, pet mob, light pet?)
const PETS: &[(&str, &str, bool)] = &[
    ("eye_spring", "pet_eye", false),
    ("honey_jar", "pet_bee", false),
    ("jelly_bubble", "pet_jelly", false),
    ("fairy_bell", "pet_fairy", true),
    ("shadow_orb_lamp", "pet_orb", true),
    ("blight_lamp", "pet_heart", true),
    ("wisp_jar", "pet_wisp", true),
];

/// (item, mount mob)
const MOUNTS: &[(&str, &str)] = &[
    ("jelly_saddle", "mount_jelly"),
    ("honey_goggles", "mount_bee"),
    ("blessed_apple", "mount_unicorn"),
    // Lote 14 (second round).
    ("fluffy_carrot", "mount_bunny"),
    ("turtle_saddle", "mount_turtle"),
    ("star_key", "mount_ufo"),
    // Third round (tr3.rs).
    ("golf_cart_key", "mount_golf_cart"),
];

/// How bright a light pet is (None: not one).
pub fn light_of(mob: &str) -> Option<f32> {
    match mob {
        "pet_wisp" => Some(1.0),
        "pet_orb" | "pet_heart" => Some(0.8),
        "pet_fairy" => Some(0.7),
        _ => None,
    }
}

fn is_pet_mob(name: &str) -> bool {
    PETS.iter().any(|p| p.1 == name)
}

fn is_mount_mob(name: &str) -> bool {
    MOUNTS.iter().any(|m| m.1 == name)
}

impl Pets {
    /// Test scenes: keep pets and mounts where they were put.
    pub(crate) fn test_keep(&mut self) {
        self.test = true;
    }
}

#[derive(Default)]
pub struct Pets {
    /// The pet mobs out (at most one pet and one light pet), saved with the world.
    pub(crate) out: Vec<String>,
    /// Seconds of flight left on the bee.
    bee_t: f32,
    /// Mobs the unicorn ran into lately.
    trampled: Vec<(usize, f32)>,
    harpy_t: f32,
    /// Test scenes keep their pets where they put them.
    test: bool,
}

impl Game {
    /// The worn wings of this file: (item, flight seconds, tint).
    pub(crate) fn tr_wing(&self) -> Option<(&'static str, f32, [f32; 3])> {
        WINGS.iter().find(|w| self.wears(w.0)).copied()
    }

    pub(crate) fn tr_pet_use(&mut self, name: &str) -> Option<bool> {
        if let Some(&(_, mob, light)) = PETS.iter().find(|p| p.0 == name) {
            let out = &mut self.t5.tr.pets.out;
            if out.iter().any(|o| o == mob) {
                out.retain(|o| o != mob);
                self.toast("O mascote voltou para casa", 2.0);
            } else {
                out.retain(|o| PETS.iter().find(|p| p.1 == o).is_some_and(|p| p.2 != light));
                out.push(mob.to_string());
                self.toast("Um mascote te acompanha", 2.0);
            }
            self.sfx(Sfx::Place(crate::audio::Material::Cloth));
            return Some(true);
        }
        if let Some(&(_, mob)) = MOUNTS.iter().find(|m| m.0 == name) {
            if self.mobs.list.iter().any(|m| m.ridden) || self.riding.is_some() {
                return Some(true);
            }
            let at = self.body.pos;
            if self.mobs.spawn(mob, at) {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.ridden = true;
                    m.saddled = true;
                    m.yaw = self.yaw;
                }
                self.t5.tr.pets.bee_t = 4.0;
                self.toast("Agache (Shift) para descer", 2.0);
                let glow = self.glow_layer();
                for _ in 0..16 {
                    let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5) * 3.0;
                    self.particles.glow(at + Vec3::Y * 0.5, v, glow, [1.3, 1.3, 1.4], 0.12, 0.6);
                }
            }
            return Some(true);
        }
        None
    }

    pub(crate) fn update_tr_pets(&mut self, dt: f32) {
        self.update_mounts(dt);
        self.update_pet_mobs(dt);
        if self.dim != Dim::Overworld {
            return;
        }
        // The high sky: you fall slower above 200, and harpies fly there.
        if self.body.pos.y > 200.0 && !self.body.on_ground && !self.body.flying {
            self.body.vel.y += GRAVITY * 0.45 * dt;
        }
        let p = &mut self.t5.tr.pets;
        p.harpy_t -= dt;
        if p.harpy_t <= 0.0 {
            p.harpy_t = 9.0;
            let me = self.body.pos.floor().as_ivec3();
            let open = self.world.surface_y(me.x, me.z).map_or(true, |y| y <= me.y + 1);
            let n = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == "harpy").count();
            if self.body.pos.y > 120.0 && open && n < 3 && self.mobs.monster_room() && self.survival && self.surv.rng.f32() < 0.6 {
                self.spawn_around("harpy", true);
            }
        }
    }

    fn update_mounts(&mut self, dt: f32) {
        // A mount nobody rides goes home.
        let reg = &self.mobs.reg;
        if !self.t5.tr.pets.test {
            self.mobs.list.retain(|m| !(is_mount_mob(&reg.defs[m.def].name) && !m.ridden));
        }
        let Some(i) = self.mobs.list.iter().position(|m| m.ridden) else { return };
        let name = self.mobs.reg.defs[self.mobs.list[i].def].name.clone();
        let space = self.keys.contains(&KeyCode::Space);
        match name.as_str() {
            "mount_jelly" => {
                let m = &mut self.mobs.list[i];
                // Big bounces, and landing on a monster hurts it.
                if m.body.on_ground && space {
                    m.body.vel.y = 13.0;
                }
                if m.body.vel.y < -4.0 {
                    let at = m.body.pos;
                    let under = (0..self.mobs.list.len()).find(|&k| {
                        k != i && self.mobs.list[k].alive() && self.is_hostile(k) && {
                            let c = self.mobs.list[k].body.pos;
                            Vec3::new(c.x - at.x, 0.0, c.z - at.z).length() < 1.4 && at.y - c.y > 0.3 && at.y - c.y < 2.5
                        }
                    });
                    if let Some(k) = under {
                        self.mobs.list[k].hurt(8.0 * self.t5.rpg.stats.dmg[crate::rpg::SUMMON], at);
                        self.mobs.list[i].body.vel.y = 10.0;
                        self.sfx(Sfx::MobHit);
                    }
                }
            }
            "mount_bee" => {
                let m = &mut self.mobs.list[i];
                if m.body.on_ground {
                    self.t5.tr.pets.bee_t = 4.0;
                }
                if space && self.t5.tr.pets.bee_t > 0.0 {
                    self.t5.tr.pets.bee_t -= dt;
                    m.body.vel.y = (m.body.vel.y + 40.0 * dt).min(6.0);
                } else if m.body.vel.y < -2.5 {
                    m.body.vel.y = -2.5;
                }
                m.body.fall_distance = 0.0;
            }
            "mount_bunny" => {
                // Hops high and far.
                let m = &mut self.mobs.list[i];
                if m.body.on_ground && space {
                    m.body.vel.y = 11.0;
                }
                m.body.fall_distance = 0.0;
            }
            "mount_turtle" => {
                // Slow on land, quick in the water, and it never sinks.
                let f = self.forward();
                let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
                let ahead = self.keys.contains(&KeyCode::KeyW);
                let m = &mut self.mobs.list[i];
                if m.body.in_water {
                    if ahead {
                        m.body.vel.x = flat.x * 7.0;
                        m.body.vel.z = flat.z * 7.0;
                    }
                    m.body.vel.y = m.body.vel.y.max(if space { 3.0 } else { 0.0 });
                }
            }
            "mount_ufo" => {
                // Flies as long as you like: Space up, otherwise it slowly settles.
                let glow = self.glow_layer();
                let m = &mut self.mobs.list[i];
                m.body.vel.y = if space { 7.0 } else { m.body.vel.y.max(-1.5) };
                m.body.fall_distance = 0.0;
                let at = m.body.pos;
                if (self.now * 10.0) as i32 % 2 == 0 {
                    self.particles.glow(at, -Vec3::Y * 2.0, glow, [0.6, 1.8, 1.2], 0.1, 0.3);
                }
            }
            "mount_unicorn" => {
                let p = &mut self.t5.tr.pets;
                for t in p.trampled.iter_mut() {
                    t.1 -= dt;
                }
                p.trampled.retain(|t| t.1 > 0.0);
                let m = &self.mobs.list[i];
                let speed = Vec3::new(m.body.vel.x, 0.0, m.body.vel.z).length();
                let at = m.body.pos;
                if speed > 5.0 {
                    let hits: Vec<usize> = (0..self.mobs.list.len())
                        .filter(|&k| k != i && self.mobs.list[k].alive() && self.is_hostile(k) && self.mobs.list[k].body.pos.distance(at) < 1.8)
                        .filter(|k| !self.t5.tr.pets.trampled.iter().any(|t| t.0 == *k))
                        .collect();
                    for k in hits {
                        self.mobs.list[k].hurt(9.0 * self.t5.rpg.stats.dmg[crate::rpg::SUMMON], at);
                        self.t5.tr.pets.trampled.push((k, 0.6));
                        self.sfx(Sfx::MobHit);
                    }
                }
            }
            _ => {}
        }
    }

    fn update_pet_mobs(&mut self, dt: f32) {
        if self.t5.tr.pets.test {
            return;
        }
        let out = self.t5.tr.pets.out.clone();
        // Strays and doubles go; missing ones come.
        let reg = &self.mobs.reg;
        let mut seen: Vec<String> = Vec::new();
        self.mobs.list.retain(|m| {
            let n = &reg.defs[m.def].name;
            if !is_pet_mob(n) {
                return true;
            }
            if out.contains(n) && !seen.contains(n) && m.alive() {
                seen.push(n.clone());
                return true;
            }
            false
        });
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let me = self.body.pos;
        for name in &out {
            if !seen.contains(name) {
                self.mobs.spawn(name, me + Vec3::Y * 1.5 - flat);
            }
            let light = PETS.iter().any(|p| p.1 == name && p.2);
            let want = if light { me + side * 0.9 + Vec3::Y * 2.1 + flat * 0.3 } else { me - side * 1.0 - flat * 1.2 + Vec3::Y * 0.9 };
            let bob = (self.now * 2.0 + if light { 1.0 } else { 0.0 }).sin() * 0.15;
            let Some(m) = self.mobs.list.iter_mut().find(|m| &self.mobs.reg.defs[m.def].name == name) else { continue };
            let to = want + Vec3::Y * bob - m.body.pos;
            if to.length() > 20.0 {
                m.body.pos = want;
            } else {
                m.body.pos += to * (dt * 4.0).min(1.0);
            }
            m.body.vel = Vec3::ZERO;
            m.body.flying = true;
            m.health = m.max_health;
            m.yaw = f.x.atan2(f.z);
        }
    }

    /// Souls in the World Breaker age: of night in the evil lands and caves, of light in the
    /// Prism, of flight from harpies.
    pub(crate) fn tr_soul_drops(&mut self, deaths: &[(usize, Vec3)]) {
        if !self.t5.hard.on {
            return;
        }
        let here = self.t5.tr.lands.here;
        for &(def, pos) in deaths {
            let d = &self.mobs.reg.defs[def];
            if d.kind != voxel::mob::Kind::Hostile || d.boss {
                continue;
            }
            let name = d.name.as_str();
            let roll = self.surv.rng.f32();
            let soul = match name {
                "harpy" if roll < 0.3 => Some("soul_of_flight"),
                "prism_pixie" | "prism_unicorn" if roll < 0.2 => Some("soul_of_light"),
                "soul_eater" | "blight_flyer" | "blight_brute" if roll < 0.2 => Some("soul_of_night"),
                _ if pos.y < 20.0 && roll < 0.12 => match here {
                    Some(crate::tr_world::Land::Prism) => Some("soul_of_light"),
                    Some(_) => Some("soul_of_night"),
                    None if roll < 0.04 => Some("soul_of_night"),
                    None => None,
                },
                _ => None,
            };
            if let Some(id) = soul.and_then(|s| self.items.id(s)) {
                self.spawn_drop(pos + Vec3::Y * 0.5, ItemStack::new(id, 1), Vec3::Y);
            }
        }
    }

    /// The worn wings on the player's back, seen in third person.
    pub(crate) fn tr_wing_instances(&self, cam_block: IVec3) -> Vec<GpuItem> {
        if self.camera_mode == 0 || self.screen == crate::Screen::Dead {
            return Vec::new();
        }
        // Pairs painted in wings.rs draw on the player model instead (vanity.rs).
        if self.worn_wings().is_some() {
            return Vec::new();
        }
        let Some((name, _, _)) = self.tr_wing() else { return Vec::new() };
        let Some(id) = self.items.id(name) else { return Vec::new() };
        let voxel::item::Look::Sprite(l) = self.items.get(id).look else { return Vec::new() };
        let layer = (self.item_base + l) as f32;
        let f = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let at = self.body.pos - f * 0.3 + Vec3::Y * 1.25;
        let light = self.world.light(at.floor().as_ivec3());
        let (sky, blk) = ((light >> 4) as f32 / 15.0, (light & 15) as f32 / 15.0);
        let flap = if self.t5.moves.flying { (self.now * 14.0).sin() * 0.35 } else { 0.15 };
        let p = at - cam_block.as_vec3();
        vec![GpuItem { pos: [p.x, p.y, p.z, 1.7], info: [self.yaw, 1.0, sky, blk], layers: [layer, layer, layer, 0.0], rot: [flap, 0.0, 0.0, 0.0], ..bytemuck::Zeroable::zeroed() }]
    }

    /// Test scenes: MC_TR=pets (every pet), mounts, harpies, wings (third person, flying).
    /// Pets and mounts drawn with a summons.rs model in place of their own plain one.
    pub(crate) fn pet_models(&self, cam_block: IVec3, max_dist: f32) -> Vec<(usize, crate::entity::Bones)> {
        let c = cam_block.as_vec3();
        let mut out = Vec::new();
        for m in &self.mobs.list {
            if (m.body.pos - c).length() > max_dist || !m.alive() {
                continue;
            }
            let def = &self.mobs.reg.defs[m.def];
            let Some(model) = crate::summons::pet_model(&def.name) else { continue };
            let Some(mesh) = self.entities.summon_mesh(model) else { continue };
            let l = self.world.light((m.body.pos + Vec3::Y * 0.5).floor().as_ivec3());
            let speed = Vec3::new(m.body.vel.x, 0.0, m.body.vel.z).length();
            let light = [(l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0, 0.0, 0.0];
            out.push((mesh, crate::summons::pet_bones(model, m.body.pos - c, m.yaw, m.age + m.phase * 10.0, speed, light)));
        }
        out
    }

    pub(crate) fn test_tr_pets(&mut self, scene: &str) {
        self.t5.tr.pets.test = true;
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let eye = self.body.eye();
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        let names: &[&str] = match scene {
            "pets" => &["pet_eye", "pet_bee", "pet_jelly", "pet_fairy", "pet_orb", "pet_heart", "pet_wisp"],
            "mounts" => &["mount_jelly", "mount_bee", "mount_unicorn", "mount_bunny", "mount_turtle", "mount_ufo"],
            "harpies" => &["harpy", "harpy", "harpy"],
            _ => &[],
        };
        let n = names.len() as f32;
        for (k, name) in names.iter().enumerate() {
            let spread = if scene == "pets" { 1.3 } else { 3.2 };
            let dist = if scene == "pets" { 4.0 } else { 8.0 };
            let p = self.body.pos + flat * dist + side * (k as f32 - (n - 1.0) / 2.0) * spread;
            let y = if scene == "mounts" { ground(self, p) } else { eye.y + 0.2 + (k % 2) as f32 * 0.6 };
            self.mobs.spawn(name, Vec3::new(p.x, y, p.z));
            if let Some(m) = self.mobs.list.last_mut() {
                let look = eye - m.body.center();
                m.yaw = look.x.atan2(look.z);
                m.body.flying = scene != "mounts";
            }
        }
        if scene == "wings" {
            if let Some(id) = self.items.id("flame_wings") {
                self.t5.acc[0] = Some(ItemStack::new(id, 1));
            }
            self.t5.moves.flying = true;
            self.body.pos.y += 3.0;
            self.body.flying = true;
        }
    }
}
