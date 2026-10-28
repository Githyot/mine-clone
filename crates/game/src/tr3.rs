//! The Terraria side, third round (plan/terraria-completo.md, "Reauditoria 3"): what a fresh look
//! at Terraria found still missing, made to play well in first person and in 3D. Its data lives
//! in packs/vanilla/tr3/ (written by tools/gen_tr3.py), its state per world in `tr3.ron`.
//!
//! - L17 golf (tr3_golf.rs).
//! - L18 town folk, mini-bosses, event bosses and rare monsters (tr3_folk.rs).
//! - L19 the common monsters of each land (tr3_foes.rs).
//! - L20 tools and weapons (tr3_tools.rs; the guns fly in tr_weapons.rs).

use std::path::Path;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::money::{g, s};
use crate::Game;

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    /// Golf balls lying in the world and the golf score (tr3_golf.rs).
    #[serde(default)]
    balls: Vec<crate::tr3_golf::SavedBall>,
    #[serde(default)]
    golf_points: u32,
    #[serde(default)]
    golf_holes: u32,
    #[serde(default)]
    golf_trophies: u8,
}

#[derive(Default)]
pub struct Tr3 {
    pub(crate) golf: crate::tr3_golf::Golf,
    pub(crate) folk: crate::tr3_folk::Folk,
    pub(crate) foes: crate::tr3_foes::Foes,
    pub(crate) tools: crate::tr3_tools::Tools,
}

/// Shops of this round's town folk: (mob, goods: item, count, price in copper).
pub(crate) const SHOPS: &[(&str, &[(&str, u8, u32)])] = &[
    ("npc_tavernkeep", &[("ale", 1, s(1)), ("ether_crystal", 1, s(25)), ("ballista_rod", 1, g(5)), ("flame_sentry_rod", 1, g(12)), ("explosive_trap_rod", 1, g(15))]),
    ("npc_skeleton_merchant", &[("torch", 16, s(1)), ("bone", 8, s(2)), ("mining_helmet", 1, g(1)), ("magic_mirror", 1, g(4)), ("musket_ball", 50, s(5)), ("lucky_coin", 1, g(20))]),
    ("npc_golfer", &[("golf_ball", 4, s(5)), ("golf_club_putter", 1, s(25)), ("golf_club_iron", 1, s(40)), ("golf_club_wedge", 1, s(40)), ("golf_club_driver", 1, s(60)), ("golf_tee", 4, s(2)), ("golf_cup", 1, s(10)), ("golf_flag", 1, s(10)), ("golf_turf", 16, s(5))]),
];

/// (mob, wish shown by the guide) for tr_town.rs's list.
pub(crate) const NPCS: &[(&str, &str)] = &[
    ("npc_golfer", "o Golfista vem quando você carrega um taco de golfe, ou depois do seu primeiro buraco."),
    ("npc_tavernkeep", "o Taverneiro vem depois que o primeiro chefe cai."),
    ("town_jelly_nerd", "a Geleia Nerd vem depois que a Geleia Rei cai."),
    ("town_jelly_cool", "a Geleia Legal vem quando o mundo entra no modo difícil."),
    ("town_jelly_elder", "a Geleia Anciã vem depois que o Rei dos Ossos cai."),
];

impl Tr3 {
    pub fn load(dir: &Path, items: &voxel::item::ItemRegistry) -> Tr3 {
        let _ = items;
        let s: Saved = std::fs::read_to_string(dir.join("tr3.ron")).ok().and_then(|x| ron::from_str(&x).ok()).unwrap_or_default();
        let mut t = Tr3::default();
        t.golf.load(s.balls, s.golf_points, s.golf_holes, s.golf_trophies);
        t
    }

    pub fn save(&self, dir: &Path) {
        let s = Saved { balls: self.golf.saved(), golf_points: self.golf.points, golf_holes: self.golf.holes, golf_trophies: self.golf.trophies };
        if let Ok(text) = ron::ser::to_string_pretty(&s, Default::default()) {
            let _ = std::fs::create_dir_all(dir);
            if let Err(e) = std::fs::write(dir.join("tr3.ron"), text) {
                log::error!("could not save tr3.ron: {e}");
            }
        }
    }
}

impl Game {
    /// Everything of this round that runs each frame in a world.
    pub(crate) fn update_tr3(&mut self, dt: f32, use_held: bool) {
        self.update_golf(dt, use_held);
        self.update_folk(dt);
        self.update_foes(dt);
        self.update_tools(dt);
    }

    /// A boss fell.
    pub(crate) fn tr3_boss_defeated(&mut self, name: &str) {
        self.folk_boss_defeated(name);
    }

    /// Right click with an item of this round; Some(true) when it did something.
    pub(crate) fn tr3_use(&mut self, name: &str) -> Option<bool> {
        self.golf_use(name).or_else(|| self.tools_use(name))
    }

    /// Who of this round's town folk wants to move in.
    pub(crate) fn tr3_npc_wish(&self, name: &str) -> bool {
        match name {
            "npc_golfer" => self.t5.tr.r3.golf.holes > 0 || self.carries(|n| n.starts_with("golf_club_")),
            "npc_tavernkeep" => !self.boss.defeated.is_empty(),
            "town_jelly_nerd" => self.boss.defeated.iter().any(|b| b == "king_jelly"),
            "town_jelly_cool" => self.t5.hard.on,
            "town_jelly_elder" => self.boss.defeated.iter().any(|b| b == "bone_king"),
            _ => false,
        }
    }

    /// Talking to one of this round's folk before the shop opens. True when the talk is all
    /// there is (no shop).
    pub(crate) fn tr3_talk(&mut self, name: &str, dn: &str) -> bool {
        match name {
            "npc_golfer" => self.golfer_talk(dn),
            "npc_old_man" => return self.old_man_talk(dn),
            n if n.starts_with("town_jelly_") => {
                self.jelly_talk(n, dn);
                return true;
            }
            _ => {}
        }
        false
    }

    /// Give the player an item (on the ground at their feet when the bag is full).
    pub(crate) fn give_or_drop(&mut self, item: &str, n: u8) {
        let Some(id) = self.items.id(item) else { return };
        let st = voxel::item::ItemStack::new(id, n);
        if let Some(rest) = self.surv.inv.add(&self.items, st) {
            let at = self.body.pos + Vec3::Y;
            self.spawn_drop(at, rest, Vec3::Y * 2.0);
        }
    }

    /// Test scenes of this round (MC_TR=golf...).
    pub(crate) fn test_tr3(&mut self, scene: &str) -> bool {
        match scene {
            "golf" | "golf_aim" | "golf_items" | "golf_course" | "golf_cart" | "golf_sim" => {
                self.test_golf(scene);
                true
            }
            "foes" | "foes2" | "crawler" | "medusa" => {
                self.test_foes(scene);
                true
            }
            "folk" | "minis3" | "wyvern" => {
                self.test_folk(scene);
                true
            }
            _ => false,
        }
    }
}
