//! The rest of the Terraria side (plan/terraria-completo.md), added lote by lote on top of
//! terraria.rs (Etapa 5), rpg.rs (stats) and deep.rs (the deep world). Its data lives in
//! packs/vanilla/tr/ (written by tools/gen_tr.py), its state per world in `tr.ron`.
//!
//! Lote 1: the early bosses (tr_bosses.rs), the jellies, the jungle hive with its royal larva,
//! and the Great Eye that comes by itself on some nights once you are strong enough.

use std::path::Path;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use voxel::worldgen::Dim;

use crate::Game;

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    /// Nights the Great Eye already came by itself.
    #[serde(default)]
    eye_nights: u32,
    /// Meteor craters (tr_events.rs).
    #[serde(default)]
    craters: Vec<(i32, i32, i32)>,
    /// Chunks the lands were laid over (tr_world.rs), and orbs broken.
    #[serde(default)]
    lands_done: Vec<(i32, i32)>,
    #[serde(default)]
    prism_done: Vec<(i32, i32)>,
    #[serde(default)]
    orbs: u32,
    /// The pet and light pet out (tr_pets.rs).
    #[serde(default)]
    pets: Vec<String>,
    /// Pylons and town animals (tr_town.rs).
    #[serde(default)]
    pylons: Vec<(String, (i32, i32, i32))>,
    #[serde(default)]
    animals: Vec<(String, (f32, f32, f32))>,
}

#[derive(Default)]
pub struct Tr {
    pub(crate) fight: crate::tr_bosses::Fight,
    pub(crate) ev: crate::tr_events::Events,
    pub(crate) lands: crate::tr_world::Lands,
    pub(crate) wep: crate::tr_weapons::Weapons,
    pub(crate) pets: crate::tr_pets::Pets,
    pub(crate) town: crate::tr_town::TownLife,
    eye_nights: u32,
    /// Was it night last frame (to notice dusk).
    night_was: bool,
    /// Seconds until the Great Eye arrives on a night it was announced.
    eye_coming: Option<f32>,
}

impl Tr {
    pub fn load(dir: &Path) -> Tr {
        let s: Saved = std::fs::read_to_string(dir.join("tr.ron")).ok().and_then(|x| ron::from_str(&x).ok()).unwrap_or_default();
        let mut tr = Tr { eye_nights: s.eye_nights, ..Default::default() };
        tr.ev.craters = s.craters.iter().map(|&(x, y, z)| glam::IVec3::new(x, y, z)).collect();
        tr.lands.done = s.lands_done.into_iter().collect();
        tr.lands.prism_done = s.prism_done.into_iter().collect();
        tr.lands.orbs = s.orbs;
        tr.pets.out = s.pets;
        tr.town.pylons = s.pylons.into_iter().map(|(k, (x, y, z))| (k, glam::IVec3::new(x, y, z))).collect();
        tr.town.animals = s.animals.into_iter().map(|(k, (x, y, z))| (k, Vec3::new(x, y, z))).collect();
        tr
    }

    pub fn save(&self, dir: &Path) {
        let s = Saved {
            eye_nights: self.eye_nights,
            craters: self.ev.craters.iter().map(|c| (c.x, c.y, c.z)).collect(),
            lands_done: self.lands.done.iter().copied().collect(),
            prism_done: self.lands.prism_done.iter().copied().collect(),
            orbs: self.lands.orbs,
            pets: self.pets.out.clone(),
            pylons: self.town.pylons.iter().map(|(k, p)| (k.clone(), (p.x, p.y, p.z))).collect(),
            animals: self.town.animals.iter().map(|(k, p)| (k.clone(), (p.x, p.y, p.z))).collect(),
        };
        if let Ok(text) = ron::ser::to_string_pretty(&s, Default::default()) {
            let _ = std::fs::create_dir_all(dir);
            if let Err(e) = std::fs::write(dir.join("tr.ron"), text) {
                log::error!("could not save tr.ron: {e}");
            }
        }
    }
}

impl Game {
    /// Everything of this file that runs each frame in a world.
    pub(crate) fn update_tr(&mut self, dt: f32, use_held: bool) {
        self.update_tr_fight(dt);
        self.great_eye_nights(dt);
        self.update_tr_events(dt);
        self.update_lands(dt);
        self.update_tr_weapons(dt, use_held);
        self.update_tr_pets(dt);
        self.update_tr_town(dt);
    }

    /// Right click with an item of this file; true when it did something.
    pub(crate) fn tr_use(&mut self, name: &str) -> Option<bool> {
        if let Some(r) = self.tr_weapon_use(name) {
            return Some(r);
        }
        if let Some(r) = self.tr_pet_use(name) {
            return Some(r);
        }
        if let Some(r) = self.tr_town_use(name) {
            return Some(r);
        }
        match name {
            "solar_tablet" => return Some(self.use_solar_tablet()),
            "purification_powder" => return Some(self.use_powder(true)),
            "vile_powder" => return Some(self.use_powder(false)),
            _ => {}
        }
        self.tr_summon(name)
    }

    /// After a block was broken: the royal larva wakes the Hive Queen.
    pub(crate) fn tr_block_broken(&mut self, p: glam::IVec3, name: &str) {
        match name {
            "ruin_orb" | "blight_heart" => return self.orb_broken(p, name == "ruin_orb"),
            "ruin_altar" | "blight_altar" => return self.altar_broken(p, name == "ruin_altar"),
            n if n.starts_with("pylon_") => return self.tr_pylon_broken(p),
            _ => {}
        }
        if name == "hive_larva" && self.mobs.list.iter().all(|m| !(m.alive() && self.mobs.reg.defs[m.def].boss)) {
            let at = p.as_vec3() + Vec3::new(0.5, 2.0, 0.5);
            if self.spawn_tr_boss("hive_queen", at) {
                self.toast("A Rainha da Colmeia despertou!", 4.0);
                self.sfx(crate::audio::Sfx::Thunder(0.4));
            }
        }
    }

    /// What a fallen boss of this file opens.
    pub(crate) fn tr_boss_defeated(&mut self, name: &str) {
        self.tr_event_boss_down();
        if name == "great_eye" {
            // The eye leaves some of the world's evil ore behind (and its seeds, in Terraria).
            let ore = if crate::tr_world::world_evil(self.seed) == crate::tr_world::Land::Ruin { "umbral_ore" } else { "sanguinite_ore" };
            if let Some(id) = self.items.id(ore) {
                let at = self.body.pos + Vec3::Y;
                self.spawn_drop(at, voxel::item::ItemStack::new(id, 30), Vec3::Y);
            }
        }
        let text = match name {
            "king_jelly" => "As geleias se acalmam... por enquanto.",
            "great_eye" => "Uma sensação estranha passa por você.",
            "blight_mind" => "A Chaga perde a força por um tempo.",
            "hive_queen" => "A colmeia silencia.",
            "frost_colossus" => "A neve se aquieta.",
            _ => return,
        };
        self.toast(text, 4.0);
    }

    /// The Great Eye comes by itself now and then at dusk, once the player has 15 hearts and
    /// hasn't beaten it yet (Terraria's idea). It is announced first.
    fn great_eye_nights(&mut self, dt: f32) {
        let night = self.is_night();
        let dusk = night && !self.t5.tr.night_was;
        self.t5.tr.night_was = night;
        if let Some(t) = self.t5.tr.eye_coming.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.t5.tr.eye_coming = None;
                let free = self.mobs.list.iter().all(|m| !(m.alive() && self.mobs.reg.defs[m.def].boss));
                if night && free && self.dim == Dim::Overworld && self.body.pos.y > 30.0 {
                    let a = self.surv.rng.f32() * std::f32::consts::TAU;
                    let at = self.body.pos + Vec3::new(a.cos() * 16.0, 10.0, a.sin() * 16.0);
                    if self.spawn_tr_boss("great_eye", at) {
                        self.toast("O Grande Olho chegou!", 4.0);
                        self.t5.tr.eye_nights += 1;
                    }
                }
            }
            return;
        }
        if !dusk || !self.survival || self.dim != Dim::Overworld || self.max_health < 30.0 {
            return;
        }
        if self.boss.defeated.iter().any(|b| b == "great_eye") || self.t5.tr.eye_nights >= 3 {
            return;
        }
        if self.surv.rng.f32() < 0.33 {
            self.toast("Você sente que está sendo observado...", 5.0);
            self.t5.tr.eye_coming = Some(25.0);
        }
    }

    /// Test scenes for screenshots (MC_TR=name): each new boss mid-fight, the jellies, the items.
    pub(crate) fn test_tr(&mut self, scene: &str) {
        if !self.t5.loaded {
            self.t5_load();
        }
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        let face = |g: &mut Game, k: usize| {
            let eye = g.body.eye();
            let m = &mut g.mobs.list[k];
            let look = eye - m.body.center();
            m.yaw = look.x.atan2(look.z);
            m.head_pitch = (-look.y / look.length().max(0.1)).asin().clamp(-1.0, 1.0);
        };
        match scene {
            "king_jelly" | "great_eye" | "blight_mind" | "hive_queen" | "frost_colossus" => {
                let dist = if scene == "frost_colossus" { 13.0 } else { 10.0 };
                let mut at = self.body.pos + flat * dist;
                at.y = match scene {
                    "king_jelly" | "frost_colossus" => ground(self, at),
                    _ => self.body.pos.y + 2.5,
                };
                let first = self.mobs.list.len();
                self.spawn_tr_boss(scene, at);
                for _ in 0..20 {
                    self.update_tr_fight(0.03);
                }
                // Back in place for the picture, facing the player.
                let n = self.mobs.list.len();
                if let Some(m) = self.mobs.list.get_mut(first) {
                    m.body.pos = at;
                    m.body.vel = Vec3::ZERO;
                }
                for k in first..n {
                    face(self, k);
                }
                if scene == "great_eye" {
                    for (j, k) in [(-1.0, 0), (1.0, 1), (0.0, 2)] {
                        self.mobs.spawn("servant_eye", at + side * j * 3.0 + Vec3::Y * (1.5 + k as f32 * 0.5) - flat * 2.0);
                        let last = self.mobs.list.len() - 1;
                        face(self, last);
                    }
                }
                if scene == "hive_queen" {
                    for j in [-1.0, 1.0] {
                        self.mobs.spawn("hive_drone", at + side * j * 3.5 - flat * 3.0);
                        let last = self.mobs.list.len() - 1;
                        face(self, last);
                    }
                }
                self.magic.clear_bolts_for_test();
            }
            "jellies" => {
                let names = ["jelly_green", "jelly_blue", "jelly_red", "jelly_ice", "jelly_lava", "jelly_pink", "jelly_spiked"];
                for (k, n) in names.iter().enumerate() {
                    let at = self.body.pos + flat * (5.0 + (k % 2) as f32 * 1.5) + side * (k as f32 - 3.0) * 1.5;
                    let y = ground(self, at);
                    self.mobs.spawn(n, Vec3::new(at.x, y, at.z));
                    let last = self.mobs.list.len() - 1;
                    face(self, last);
                }
            }
            "items" => {
                self.survival = true;
                let names = [
                    "gel", "lens", "jelly_crown", "eye_lure", "blight_spine", "royal_comb", "frost_horn", "blight_tissue", "beeswax", "frost_shard", "hive_staff", "colossus_staff",
                    "jelly_staff", "hive_blade", "jelly_hook", "royal_gel", "watcher_shield", "mind_ward", "hive_pack", "fur_cloak", "hive_block", "hive_larva",
                ];
                for (k, n) in names.iter().enumerate() {
                    if let Some(id) = self.items.id(n) {
                        self.surv.inv.slots[k] = Some(voxel::item::ItemStack::new(id, 1));
                    }
                }
                self.args.show_inventory = true;
            }
            "items2" => {
                self.survival = true;
                let names = [
                    "meteorite_ore", "meteorite_ingot", "meteor_helmet", "meteor_chestplate", "meteor_leggings", "meteor_boots", "meteor_blaster", "fallen_star", "solar_tablet", "dark_shard",
                    "moth_dust", "eclipse_scythe", "sandstorm_bottle", "gel",
                ];
                for (k, n) in names.iter().enumerate() {
                    if let Some(id) = self.items.id(n) {
                        self.surv.inv.slots[k] = Some(voxel::item::ItemStack::new(id, 1));
                    }
                }
                self.args.show_inventory = true;
            }
            "meteor_set" => {
                // The meteor armour worn, seen in third person from the front.
                self.survival = true;
                for (slot, n) in ["meteor_helmet", "meteor_chestplate", "meteor_leggings", "meteor_boots"].iter().enumerate() {
                    if let Some(id) = self.items.id(n) {
                        self.surv.armor[slot] = Some(voxel::item::ItemStack::new(id, 1));
                    }
                }
                if let Some(id) = self.items.id("meteor_blaster") {
                    self.surv.inv.slots[0] = Some(voxel::item::ItemStack::new(id, 1));
                }
            }
            "ruin" => self.test_land(crate::tr_world::Land::Ruin),
            "blight" => self.test_land(crate::tr_world::Land::Blight),
            "prism" => {
                self.t5.hard.on = true;
                self.test_land(crate::tr_world::Land::Prism);
                for (k, n) in ["prism_pixie", "prism_unicorn", "prism_pixie"].iter().enumerate() {
                    let p = self.body.pos + flat * (8.0 + k as f32) + side * (k as f32 - 1.0) * 3.0;
                    let y = ground(self, p) + if n.ends_with("pixie") { 2.0 } else { 0.0 };
                    self.mobs.spawn(n, Vec3::new(p.x, y, p.z));
                    let last = self.mobs.list.len() - 1;
                    face(self, last);
                }
            }
            "land_mobs" => {
                for (k, n) in ["soul_eater", "blight_flyer", "blight_brute", "prism_pixie", "prism_unicorn"].iter().enumerate() {
                    let p = self.body.pos + flat * (6.0 + (k % 2) as f32 * 2.0) + side * (k as f32 - 2.0) * 2.5;
                    let y = ground(self, p) + if matches!(*n, "soul_eater" | "blight_flyer" | "prism_pixie") { 1.5 } else { 0.0 };
                    self.mobs.spawn(n, Vec3::new(p.x, y, p.z));
                    let last = self.mobs.list.len() - 1;
                    face(self, last);
                }
            }
            "sky_island" | "pyramid" => {
                let name = if scene == "sky_island" { "ilha_do_ceu" } else { "piramide" };
                let (x, z) = (self.body.pos.x as i32, self.body.pos.z as i32);
                if let Some((o, size)) = self.world.gen.find_structure(name, x, z, 40) {
                    let c = o.as_vec3() + size.as_vec3() * 0.5;
                    let (back, up) = if scene == "sky_island" { (26.0, 6.0) } else { (30.0, 10.0) };
                    let y = if scene == "sky_island" { c.y.max(150.0) + up } else { self.world.gen.height_at(c.x as i32, c.z as i32) as f32 + up };
                    self.body.pos = Vec3::new(c.x, y, c.z + back);
                    self.yaw = std::f32::consts::PI;
                    self.body.flying = true;
                    log::info!("{name} at {o} size {size}");
                } else {
                    log::warn!("no {name} found");
                }
            }
            "items3" => {
                self.survival = true;
                let names = [
                    "umbral_ingot", "sanguinite_ingot", "purification_powder", "vile_powder", "star_band", "ruin_thorn", "blight_rod", "pixie_dust", "unicorn_horn",
                    "sturdy_fossil", "jungle_spore", "sky_blade", "umbral_pickaxe", "sanguinite_pickaxe", "umbral_sword", "sanguinite_sword", "umbral_helmet", "sanguinite_chestplate",
                    "hive_helmet", "jungle_chestplate", "fossil_helmet", "necro_chestplate", "ninja_helmet", "ruin_orb", "blight_heart", "ruin_altar", "sky_brick", "cloud_block",
                    "umbral_ore", "sanguinite_ore", "ruin_grass", "blight_grass", "prism_grass", "prism_stone",
                ];
                for (k, n) in names.iter().enumerate() {
                    if let Some(id) = self.items.id(n) {
                        self.surv.inv.slots[k] = Some(voxel::item::ItemStack::new(id, 1));
                    }
                }
                self.args.show_inventory = true;
            }
            "weapons" => self.test_tr_weapons(),
            "pets" | "mounts" | "harpies" | "wings" => self.test_tr_pets(scene),
            "town" | "townshop" => self.test_tr_town(scene),
            s if self.test_tr_event(s) => {}
            _ => log::warn!("unknown MC_TR scene {scene}"),
        }
    }
}
