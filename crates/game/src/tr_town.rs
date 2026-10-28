//! Lote 6 of the Terraria side (plan/terraria-completo.md): the town grows.
//! - Sixteen more town folk, each moving in once their wish is met (npcs.rs does the moving).
//! - Happiness: every town NPC likes one kind of land, loves some neighbours, can't stand
//!   others and hates a crowd. Happy ones sell cheaper, and sell the pylon of where they live.
//! - Pylons: right click one to jump to the next pylon, if both have at least two neighbours.
//! - The peddler: some mornings a travelling seller comes with odd goods and leaves at dusk.
//! - The party: some days (or with a party kit) the town throws a party, with confetti and
//!   lower prices.
//! - Town animals: a licence brings a cat, a dog or a bunny who lives in town.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::worldgen::{Biome, Dim};

use crate::audio::{Material, Sfx};
use crate::money::{g, s};
use crate::Game;

/// (mob, wish shown by the guide)
pub(crate) const NPCS: &[(&str, &str)] = &[
    ("npc_dryad", "a Druida chega depois que você derrota o primeiro chefe."),
    ("npc_demolitionist", "o Demolidor vem quando você carrega bombas, dinamite ou TNT."),
    ("npc_arms_dealer", "a Armeira vem quando você carrega uma arma de fogo ou balas."),
    ("npc_angler", "o Pescador vem quando você carrega um peixe."),
    ("npc_stylist", "a Barbeira vem quando você carrega linha e já tem 3 moradores."),
    ("npc_dye_trader", "a Tintureira vem quando você carrega um corante."),
    ("npc_painter", "o Pintor vem quando a vila tem 6 moradores."),
    ("npc_tailor", "o Alfaiate vem depois do Rei dos Ossos."),
    ("npc_witch_doctor", "o Curandeiro da Selva vem depois da Rainha da Colmeia."),
    ("npc_zoologist", "a Zoóloga vem quando você carrega uma guia (laço)."),
    ("npc_party_girl", "a Festeira vem quando a vila tem 9 moradores."),
    ("npc_pirate", "o Pirata vem no modo difícil, com 7 moradores."),
    ("npc_steampunker", "a Engenheira a Vapor vem depois de um chefe mecânico."),
    ("npc_cyborg", "o Ciborgue vem depois da Rainha das Vinhas."),
    ("npc_princess", "a Princesa vem quando a vila tem 14 moradores."),
    ("npc_mycologist", "o Micólogo vem no modo difícil, quando você carrega um cogumelo."),
    ("npc_winter_elder", "o Velho do Inverno vem em dezembro, ou depois da Rainha do Gelo."),
    ("npc_tax_collector", "o Coletor de Impostos vem no modo difícil, depois que você liberta uma alma atormentada do Submundo com pó de purificação."),
];

/// Shops of this file's town folk: (mob, goods: item, count, price in copper).
pub(crate) const SHOPS: &[(&str, &[(&str, u8, u32)])] = &[
    ("npc_dryad", &[("purification_powder", 5, s(10)), ("oak_sapling", 4, s(10)), ("jungle_sapling", 2, s(10)), ("wheat_seeds", 8, s(10)), ("pumpkin_seeds", 4, s(10)), ("melon_seeds", 4, s(10)), ("ancient_seed", 1, s(40)), ("bone_meal", 8, s(10))]),
    ("npc_demolitionist", &[("bomb", 4, s(10)), ("grenade", 4, s(10)), ("dynamite", 2, s(20)), ("tnt", 2, s(20)), ("gunpowder", 6, s(10))]),
    ("npc_arms_dealer", &[("musket_ball", 50, s(10)), ("flintlock_pistol", 1, s(60)), ("musket", 1, s(90)), ("boomstick", 1, s(140)), ("rapid_gun", 1, s(220)), ("arrow", 16, s(10))]),
    ("npc_angler", &[("fishing_rod", 1, s(20)), ("cod", 3, s(10)), ("salmon", 2, s(10)), ("bucket", 1, s(20)), ("tropical_fish", 1, s(20)), ("nautilus_shell", 1, s(60))]),
    ("npc_stylist", &[("shears", 1, s(10)), ("name_tag", 1, s(30)), ("poppy", 4, s(10)), ("dandelion", 4, s(10)), ("blue_orchid", 2, s(10)), ("lily_of_the_valley", 2, s(10))]),
    ("npc_dye_trader", &[("red_dye", 4, s(10)), ("yellow_dye", 4, s(10)), ("blue_dye", 4, s(10)), ("green_dye", 4, s(10)), ("purple_dye", 4, s(10)), ("orange_dye", 4, s(10)), ("pink_dye", 4, s(10)), ("black_dye", 4, s(10)), ("white_dye", 4, s(10))]),
    ("npc_painter", &[("lantern", 2, s(10)), ("candle", 4, s(10)), ("white_wool", 8, s(10)), ("red_dye", 4, s(10)), ("glass", 8, s(10)), ("brush", 1, s(20))]),
    ("npc_tailor", &[("white_wool", 8, s(10)), ("string", 8, s(10)), ("leather", 4, s(10)), ("leather_helmet", 1, s(20)), ("leather_chestplate", 1, s(30)), ("leather_boots", 1, s(20)), ("red_carpet", 8, s(10))]),
    ("npc_witch_doctor", &[("jungle_spore", 4, s(10)), ("honey_bottle", 3, s(10)), ("beeswax", 2, s(20)), ("jungle_sapling", 4, s(10)), ("vine", 8, s(10)), ("antidote_charm", 1, s(80))]),
    ("npc_zoologist", &[("cat_license", 1, s(60)), ("dog_license", 1, s(60)), ("bunny_license", 1, s(40)), ("lead", 2, s(10)), ("saddle", 1, s(40)), ("name_tag", 1, s(30)), ("bone", 8, s(10))]),
    ("npc_party_girl", &[("confetti", 10, s(10)), ("party_kit", 1, s(60)), ("firework_rocket", 8, s(10)), ("cake", 1, s(20)), ("note_block", 2, s(10)), ("jukebox", 1, s(60))]),
    ("npc_pirate", &[("spyglass", 1, s(30)), ("compass", 1, s(20)), ("map", 1, s(20)), ("gunpowder", 8, s(10)), ("gold_ingot", 2, s(30)), ("heart_of_the_sea", 1, s(200))]),
    ("npc_steampunker", &[("piston", 4, s(20)), ("observer", 2, s(20)), ("rail", 16, s(20)), ("powered_rail", 6, s(30)), ("minecart", 1, s(30)), ("clock", 1, s(20)), ("copper_ingot", 10, s(10))]),
    ("npc_cyborg", &[("firework_rocket", 16, s(10)), ("tnt", 4, s(20)), ("meteor_shot", 50, s(20)), ("redstone", 16, s(10)), ("rapid_gun", 1, s(180))]),
    ("npc_princess", &[("cake", 1, s(20)), ("golden_apple", 1, s(60)), ("diamond", 1, s(60)), ("gold_block", 1, s(120)), ("music_disc_first_light", 1, s(80))]),
    ("npc_mycologist", &[("glow_mushroom", 4, s(10)), ("brown_mushroom", 6, s(10)), ("red_mushroom", 6, s(10)), ("mushroom_stew", 1, s(10)), ("sporite_ingot", 2, s(60)), ("jungle_spore", 4, s(10))]),
    ("npc_winter_elder", &[("present", 1, s(30)), ("cookie", 8, s(10)), ("snow", 16, s(10)), ("spruce_sapling", 4, s(10)), ("firework_rocket", 8, s(10)), ("candle", 4, s(10))]),
];

/// Odd goods the peddler may bring (six a visit).
const PEDDLER_POOL: &[(&str, u8, u32)] = &[
    ("lucky_coin", 1, s(120)),
    ("magnet_ring", 1, s(90)),
    ("life_crystal", 1, s(90)),
    ("mana_crystal", 1, s(70)),
    ("totem_of_undying", 1, s(240)),
    ("trident", 1, s(200)),
    ("elytra", 1, s(400)),
    ("enchanted_golden_apple", 1, s(300)),
    ("ender_pearl", 4, s(30)),
    ("blaze_rod", 2, s(40)),
    ("diamond", 2, s(80)),
    ("emerald_block", 1, s(90)),
    ("experience_bottle", 6, s(30)),
    ("cloud_bottle", 1, s(90)),
    ("running_boots", 1, s(80)),
    ("grappling_hook", 1, s(50)),
    ("star_saber", 1, s(180)),
    ("enchanted_sword", 1, s(140)),
    ("fairy_bell", 1, s(100)),
    ("wisp_jar", 1, s(120)),
    ("blessed_apple", 1, s(250)),
    ("jelly_saddle", 1, s(140)),
    ("music_disc_embers", 1, s(100)),
    ("sniffer_egg", 1, s(150)),
    ("heart_of_the_sea", 1, s(180)),
    ("nautilus_shell", 2, s(40)),
];

/// (mob, land it likes, neighbours it loves, neighbours it can't stand)
const MOODS: &[(&str, &str, &[&str], &[&str])] = &[
    ("npc_guide", "forest", &["npc_merchant", "npc_dryad"], &["npc_pirate"]),
    ("npc_merchant", "forest", &["npc_guide", "npc_arms_dealer"], &["npc_tailor"]),
    ("npc_nurse", "forest", &["npc_arms_dealer", "npc_wizard"], &["npc_dryad", "npc_party_girl"]),
    ("npc_wizard", "cave", &["npc_sorceress", "npc_nurse"], &["npc_witch_doctor"]),
    ("npc_smith", "cave", &["npc_demolitionist", "npc_mechanic"], &["npc_stylist"]),
    ("npc_farmer", "forest", &["npc_dryad", "npc_zoologist"], &["npc_demolitionist"]),
    ("npc_guard", "forest", &["npc_arms_dealer", "npc_cyborg"], &["npc_pirate"]),
    ("npc_tinkerer", "cave", &["npc_mechanic", "npc_dye_trader"], &["npc_stylist"]),
    ("npc_mechanic", "snow", &["npc_tinkerer", "npc_cyborg"], &["npc_arms_dealer"]),
    ("npc_sorceress", "forest", &["npc_wizard", "npc_dryad"], &["npc_cyborg"]),
    ("npc_dryad", "jungle", &["npc_witch_doctor", "npc_farmer"], &["npc_angler", "npc_cyborg"]),
    ("npc_demolitionist", "cave", &["npc_tinkerer", "npc_smith"], &["npc_arms_dealer"]),
    ("npc_arms_dealer", "desert", &["npc_nurse", "npc_steampunker"], &["npc_demolitionist"]),
    ("npc_angler", "ocean", &["npc_pirate", "npc_demolitionist"], &["npc_tailor", "npc_dryad"]),
    ("npc_stylist", "ocean", &["npc_dye_trader", "npc_party_girl"], &["npc_tinkerer"]),
    ("npc_dye_trader", "desert", &["npc_painter", "npc_stylist"], &["npc_steampunker"]),
    ("npc_painter", "jungle", &["npc_dryad", "npc_party_girl"], &["npc_cyborg"]),
    ("npc_tailor", "forest", &["npc_dye_trader", "npc_princess"], &["npc_merchant"]),
    ("npc_witch_doctor", "jungle", &["npc_dryad", "npc_guide"], &["npc_nurse"]),
    ("npc_zoologist", "forest", &["npc_witch_doctor", "npc_farmer"], &["npc_angler"]),
    ("npc_party_girl", "forest", &["npc_wizard", "npc_stylist"], &["npc_merchant"]),
    ("npc_pirate", "ocean", &["npc_angler", "npc_tinkerer"], &["npc_guide"]),
    ("npc_steampunker", "desert", &["npc_cyborg", "npc_painter"], &["npc_dryad"]),
    ("npc_cyborg", "snow", &["npc_steampunker", "npc_pirate"], &["npc_zoologist"]),
    ("npc_princess", "forest", &["npc_tailor", "npc_party_girl"], &[]),
    ("npc_mycologist", "cave", &["npc_guide", "npc_witch_doctor"], &["npc_dryad"]),
    ("npc_winter_elder", "snow", &["npc_party_girl", "npc_princess"], &["npc_pirate"]),
    ("npc_tax_collector", "forest", &["npc_merchant", "npc_party_girl"], &["npc_angler", "npc_princess"]),
];

const PYLONS: &[&str] = &["forest", "desert", "snow", "jungle", "ocean", "cave"];

/// (licence, mob it brings)
const LICENSES: &[(&str, &str)] = &[("cat_license", "cat"), ("dog_license", "wolf"), ("bunny_license", "rabbit")];

const ZONE_NAMES: &[(&str, &str)] = &[("forest", "a floresta"), ("desert", "o deserto"), ("snow", "a neve"), ("jungle", "a selva"), ("ocean", "o mar"), ("cave", "as cavernas")];

#[derive(Default)]
pub struct TownLife {
    /// Registered pylons: (kind, block), saved with the world.
    pub(crate) pylons: Vec<(String, IVec3)>,
    /// Town animals: (mob, home), saved with the world.
    pub(crate) animals: Vec<(String, Vec3)>,
    /// Seconds of party left.
    party: f32,
    confetti_t: f32,
    /// The peddler's goods while he is in town.
    peddler: Vec<(&'static str, u8, u32)>,
    last_time: f32,
}

impl Game {
    fn town_count(&self, name: &str) -> bool {
        self.town.npcs.iter().any(|n| n.name == name)
    }

    fn carries(&self, pred: impl Fn(&str) -> bool) -> bool {
        self.surv.inv.slots.iter().flatten().any(|s| pred(&self.items.get(s.item).name))
    }

    /// Wishes of this file's town folk (npcs.rs asks through hardmode.rs).
    pub(crate) fn tr_npc_wish(&self, name: &str) -> bool {
        let people = self.town.npcs.len();
        let beaten = |b: &str| self.boss.defeated.iter().any(|d| d == b);
        match name {
            "npc_dryad" => !self.boss.defeated.is_empty(),
            "npc_demolitionist" => self.carries(|n| matches!(n, "bomb" | "dynamite" | "tnt" | "grenade")),
            "npc_arms_dealer" => self.carries(|n| matches!(n, "flintlock_pistol" | "musket" | "boomstick" | "rapid_gun" | "musket_ball" | "meteor_shot")),
            "npc_angler" => self.carries(|n| matches!(n, "cod" | "salmon" | "tropical_fish" | "pufferfish")),
            "npc_stylist" => people >= 3 && self.carries(|n| n == "string"),
            "npc_dye_trader" => self.carries(|n| n.ends_with("_dye")),
            "npc_painter" => people >= 6,
            "npc_tailor" => beaten("bone_king"),
            "npc_witch_doctor" => beaten("hive_queen"),
            "npc_zoologist" => self.carries(|n| n == "lead"),
            "npc_party_girl" => people >= 9,
            "npc_pirate" => self.t5.hard.on && people >= 7,
            "npc_steampunker" => ["mech_eye", "mech_worm", "mech_skull"].iter().any(|b| beaten(b)),
            "npc_cyborg" => beaten("vine_queen"),
            "npc_princess" => people >= 14,
            "npc_mycologist" => self.t5.hard.on && self.carries(|n| n.ends_with("mushroom")),
            "npc_tax_collector" => self.t5.hard.on && self.t5.tr.minis.tax_freed,
            "npc_winter_elder" => crate::tr_minis::holiday() == crate::tr_minis::Holiday::Christmas || beaten("ice_queen"),
            _ => false,
        }
    }

    /// The land kind around a spot, for pylons and happiness.
    fn zone_at(&self, p: Vec3) -> &'static str {
        if p.y < 40.0 {
            return "cave";
        }
        match self.world.gen.base_biome_at(p.x.floor() as i32, p.z.floor() as i32) {
            Biome::Desert | Biome::Badlands | Biome::Savanna => "desert",
            Biome::Snowy | Biome::Taiga | Biome::SnowyPeaks | Biome::FrozenOcean => "snow",
            Biome::Jungle | Biome::Swamp | Biome::MangroveSwamp => "jungle",
            Biome::Ocean | Biome::Beach => "ocean",
            _ => "forest",
        }
    }

    /// Price factor of a town NPC (0.75 happy .. 1.5 miserable) and a line on how they feel.
    pub(crate) fn npc_mood(&self, name: &str) -> (f32, String) {
        let Some(&(_, likes, loves, hates)) = MOODS.iter().find(|m| m.0 == name) else { return (1.0, String::new()) };
        let Some(me) = self.town.npcs.iter().find(|n| n.name == name) else { return (1.0, String::new()) };
        let home = Vec3::from(me.home);
        let mut f = 1.0;
        let mut why = Vec::new();
        if self.zone_at(home) == likes {
            f -= 0.08;
            let z = ZONE_NAMES.iter().find(|z| z.0 == likes).map_or("", |z| z.1);
            why.push(format!("adoro morar perto de {z}"));
        }
        let near: Vec<&str> = self.town.npcs.iter().filter(|n| n.name != name && Vec3::from(n.home).distance(home) < 25.0).map(|n| n.name.as_str()).collect();
        let dn = |n: &str| self.mobs.reg.id(n).map(|i| self.mobs.reg.defs[i].display_name.clone()).unwrap_or_default();
        for n in &near {
            if loves.contains(n) {
                f -= 0.06;
                why.push(format!("gosto de ter {} por perto", dn(n)));
            }
            if hates.contains(n) {
                f += 0.08;
                why.push(format!("não suporto {}", dn(n)));
            }
        }
        if near.len() > 3 {
            f += 0.05 * (near.len() - 3) as f32;
            why.push("tem gente demais por aqui".to_string());
        } else if near.len() <= 2 {
            f -= 0.05;
        }
        if self.t5.tr.town.party > 0.0 {
            f -= 0.1;
            why.push("que festa boa!".to_string());
        }
        let f = f.clamp(0.75, 1.5);
        let mood = if f <= 0.85 {
            "Estou muito feliz"
        } else if f <= 0.95 {
            "Estou contente"
        } else if f < 1.1 {
            "Estou bem"
        } else {
            "Não estou feliz"
        };
        let line = if why.is_empty() { format!("{mood}.") } else { format!("{mood}: {}.", why.join(", ")) };
        (f, line)
    }

    /// What a town NPC sells now, priced by their mood (None: not a seller).
    pub(crate) fn npc_offers(&self, name: &str) -> Option<(Vec<(&'static str, u8, u32)>, String)> {
        let mut offers: Vec<(&'static str, u8, u32)> = if name == "npc_peddler" {
            self.t5.tr.town.peddler.clone()
        } else {
            crate::npcs::SHOPS.iter().chain(crate::hardmode::SHOPS).chain(SHOPS).find(|s| s.0 == name)?.1.to_vec()
        };
        let (f, line) = self.npc_mood(name);
        // Happy folk sell the pylon of the land they live in.
        if f <= 0.9 {
            if let Some(home) = self.town.npcs.iter().find(|n| n.name == name).map(|n| Vec3::from(n.home)) {
                let zone = self.zone_at(home);
                let pylon = PYLONS.iter().position(|p| *p == zone).map(|k| ["pylon_forest", "pylon_desert", "pylon_snow", "pylon_jungle", "pylon_ocean", "pylon_cave"][k]);
                if let Some(p) = pylon {
                    offers.push((p, 1, g(1)));
                }
            }
        }
        for o in offers.iter_mut() {
            o.2 = ((o.2 as f32 * f).round() as u32).max(1);
        }
        Some((offers, line))
    }

    pub(crate) fn update_tr_town(&mut self, dt: f32) {
        if self.dim != Dim::Overworld || dt == 0.0 {
            return;
        }
        let t = self.time_of_day;
        let new_day = t < self.t5.tr.town.last_time && self.t5.tr.town.last_time - t > 12000.0;
        self.t5.tr.town.last_time = t;
        if new_day {
            self.town_morning();
        }
        // The peddler packs up at dusk.
        if t > 12000.0 && !self.t5.tr.town.peddler.is_empty() {
            self.t5.tr.town.peddler.clear();
            let def = self.mobs.reg.id("npc_peddler");
            let before = self.mobs.list.len();
            self.mobs.list.retain(|m| Some(m.def) != def);
            if self.mobs.list.len() < before {
                self.toast("O Mascate arrumou as coisas e foi embora", 3.0);
            }
        }
        if self.t5.tr.town.party > 0.0 {
            let life = &mut self.t5.tr.town;
            life.party -= dt;
            life.confetti_t -= dt;
            let (over, burst) = (life.party <= 0.0, life.confetti_t <= 0.0);
            if over {
                self.toast("A festa acabou", 2.5);
            }
            if burst {
                self.t5.tr.town.confetti_t = 0.25;
                let at: Vec<Vec3> = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].town && m.body.pos.distance(self.body.pos) < 48.0).map(|m| m.body.pos).collect();
                for p in at {
                    self.confetti(p + Vec3::Y * 2.2, 4, 1.5);
                }
            }
        }
        self.keep_town_animals();
    }

    fn town_morning(&mut self) {
        self.angler_new_day();
        let people = self.town.npcs.len();
        // The peddler comes some mornings, to one of the houses.
        if people >= 2 && self.t5.tr.town.peddler.is_empty() && self.surv.rng.f32() < 0.4 {
            let home = Vec3::from(self.town.npcs[(self.surv.rng.f32() * people as f32) as usize % people].home);
            self.peddler_arrives(home);
        }
        // A party some days, when the Festeira lives here.
        if self.town_count("npc_party_girl") && self.surv.rng.f32() < 0.25 {
            self.start_party(600.0);
        }
    }

    fn peddler_arrives(&mut self, at: Vec3) {
        let mut pool: Vec<(&'static str, u8, u32)> = PEDDLER_POOL.iter().copied().filter(|o| self.items.id(o.0).is_some()).collect();
        let mut stock = Vec::new();
        while stock.len() < 6 && !pool.is_empty() {
            let k = (self.surv.rng.f32() * pool.len() as f32) as usize % pool.len();
            stock.push(pool.swap_remove(k));
        }
        self.t5.tr.town.peddler = stock;
        if self.mobs.spawn("npc_peddler", at + Vec3::X) {
            if let Some(m) = self.mobs.list.last_mut() {
                m.home = Some(at);
                m.fed = 0.0;
            }
        }
        self.toast("O Mascate chegou à vila! Ele vai embora ao anoitecer.", 4.0);
    }

    fn start_party(&mut self, secs: f32) {
        self.t5.tr.town.party = secs;
        self.toast("Festa na vila! Os moradores estão felizes e os preços caíram.", 4.0);
        self.sfx(Sfx::Xp);
    }

    fn confetti(&mut self, at: Vec3, n: usize, spread: f32) {
        const COLOURS: [[f32; 3]; 5] = [[1.6, 0.4, 0.7], [1.6, 1.4, 0.3], [0.4, 1.1, 1.7], [0.6, 1.5, 0.5], [1.5, 0.8, 0.3]];
        let layer = self.glow_layer();
        for _ in 0..n {
            let r = &mut self.surv.rng;
            let v = Vec3::new(r.f32() - 0.5, r.f32() * 0.8 + 0.4, r.f32() - 0.5) * spread;
            let c = COLOURS[(r.f32() * 5.0) as usize % 5];
            self.particles.dust(at, v, layer, c, 0.07, 2.0);
        }
    }

    /// Licences, confetti and the party kit.
    pub(crate) fn tr_town_use(&mut self, name: &str) -> Option<bool> {
        match name {
            "confetti" => {
                let at = self.body.eye() + self.forward() * 1.5;
                self.confetti(at, 40, 3.0);
                self.sfx(Sfx::Pickup);
                self.take_held_one();
                Some(true)
            }
            "party_kit" => {
                self.start_party(300.0);
                self.take_held_one();
                Some(true)
            }
            _ => {
                let &(_, mob) = LICENSES.iter().find(|l| l.0 == name)?;
                // The animal lives at the nearest house (or where you stand).
                let me = self.body.pos;
                let home = self.town.npcs.iter().map(|n| Vec3::from(n.home)).min_by(|a, b| a.distance(me).total_cmp(&b.distance(me))).filter(|h| h.distance(me) < 64.0).unwrap_or(me);
                self.t5.tr.town.animals.push((mob.to_string(), home));
                self.toast("Um bichinho se mudou para a vila!", 3.0);
                self.sfx(Sfx::Place(Material::Cloth));
                self.take_held_one();
                Some(true)
            }
        }
    }

    fn take_held_one(&mut self) {
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
    }

    fn keep_town_animals(&mut self) {
        for k in 0..self.t5.tr.town.animals.len() {
            let (mob, home) = self.t5.tr.town.animals[k].clone();
            if home.distance(self.body.pos) > 80.0 {
                continue;
            }
            let near = self.mobs.list.iter().any(|m| m.alive() && m.home == Some(home) && self.mobs.reg.defs[m.def].name == mob);
            if !near && self.mobs.spawn(&mob, home + Vec3::new(0.5, 0.0, 0.5)) {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.home = Some(home);
                }
            }
        }
    }

    /// A pylon went down: it joins the network (one of each kind).
    pub(crate) fn tr_after_place(&mut self, p: IVec3) {
        let name = self.reg.get(self.world.block_or_air(p)).name.clone();
        let Some(kind) = name.strip_prefix("pylon_") else { return };
        let pylons = &mut self.t5.tr.town.pylons;
        pylons.retain(|(k, _)| k != kind);
        pylons.push((kind.to_string(), p));
        if self.zone_at(p.as_vec3()) != kind {
            self.toast("Este pilone só funciona no lugar certo (veja o nome dele)", 3.5);
        } else {
            self.toast("Pilone ligado à rede", 2.0);
        }
    }

    pub(crate) fn tr_pylon_broken(&mut self, p: IVec3) {
        self.t5.tr.town.pylons.retain(|(_, q)| *q != p);
    }

    fn pylon_ok(&self, kind: &str, p: IVec3) -> bool {
        let c = p.as_vec3();
        self.zone_at(c) == kind && self.town.npcs.iter().filter(|n| Vec3::from(n.home).distance(c) < 40.0).count() >= 2
    }

    /// Right click on a pylon: off to the next one in the network.
    pub(crate) fn use_pylon(&mut self, p: IVec3, kind: &str) -> bool {
        if !self.t5.tr.town.pylons.iter().any(|(_, q)| *q == p) {
            self.tr_after_place(p);
        }
        if !self.pylon_ok(kind, p) {
            self.toast("Um pilone precisa de 2 moradores por perto e do lugar certo", 3.0);
            return true;
        }
        let list = self.t5.tr.town.pylons.clone();
        let at = list.iter().position(|(_, q)| *q == p).unwrap_or(0);
        let next = (1..list.len()).map(|k| &list[(at + k) % list.len()]).find(|(k, q)| self.pylon_ok(k, *q));
        let Some((_, to)) = next else {
            self.toast("Não há outro pilone ativo para onde ir", 3.0);
            return true;
        };
        let glow = self.glow_layer();
        for _ in 0..30 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() * 2.0, self.surv.rng.f32() - 0.5) * 2.0;
            self.particles.glow(self.body.pos + Vec3::Y, v, glow, [1.0, 1.4, 1.8], 0.1, 0.8);
        }
        self.body.pos = to.as_vec3() + Vec3::new(0.5, 1.0, 0.5);
        self.body.vel = Vec3::ZERO;
        self.body.fall_distance = 0.0;
        self.sfx(Sfx::Respawn);
        true
    }

    /// Test scenes: MC_TR=town (the new town folk in a row, at a party, by a pylon) and
    /// townshop (the arms dealer's shop with the mood line).
    pub(crate) fn test_tr_town(&mut self, scene: &str) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let eye = self.body.eye();
        let names: Vec<&str> = NPCS.iter().map(|n| n.0).chain(["npc_peddler"]).collect();
        let n = names.len();
        for (k, name) in names.iter().enumerate() {
            let row = (k / 8) as f32;
            let col = (k % 8) as f32 - 3.5;
            let p = self.body.pos + flat * (6.0 + row * 2.5) + side * col * 1.3;
            let y = self.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map_or(p.y, |y| y as f32 + 1.0);
            let home = Vec3::new(p.x, y, p.z);
            if k < n - 1 {
                self.town.npcs.push(crate::npcs::NpcSave { name: name.to_string(), home: home.to_array() });
            }
            self.mobs.spawn(name, home);
            if let Some(m) = self.mobs.list.last_mut() {
                let look = eye - m.body.center();
                m.yaw = look.x.atan2(look.z);
            }
        }
        if scene == "town" {
            self.t5.tr.town.party = 60.0;
            let pp = (self.body.pos + flat * 3.0 + side * 2.0).floor().as_ivec3();
            let y = self.world.surface_y(pp.x, pp.z).unwrap_or(pp.y - 1) + 1;
            let id = self.reg.id_or_air("pylon_forest");
            self.world.set_block(IVec3::new(pp.x, y, pp.z), id);
            for _ in 0..30 {
                let p = self.body.pos + flat * 6.0 + Vec3::Y * 3.0 + side * (self.surv.rng.f32() - 0.5) * 9.0;
                self.confetti(p, 8, 1.2);
            }
        }
        if scene == "townshop" {
            self.t5.tr.town.peddler = PEDDLER_POOL[..6].to_vec();
            if let Some(e) = self.items.id("emerald") {
                self.surv.inv.add(&self.items, ItemStack::new(e, 30));
            }
            let who = std::env::var("MC_SHOP").unwrap_or_else(|_| "npc_arms_dealer".into());
            if let Some(m) = self.mobs.list.iter_mut().find(|m| self.mobs.reg.defs[m.def].name == who) {
                m.body.pos = self.body.eye() + flat * 2.0 - Vec3::Y * 1.0;
            }
            self.talk_to_npc();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn town_goods_exist() {
        // Every name in every data file of the base pack.
        fn read(dir: &std::path::Path, out: &mut String) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    read(&p, out);
                } else if p.extension().is_some_and(|x| x == "ron") {
                    out.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
                }
            }
        }
        let mut items = String::new();
        read(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../packs/vanilla")), &mut items);
        let mut missing = Vec::new();
        for (_, goods) in super::SHOPS {
            for g in goods.iter() {
                if !items.contains(&format!("\"{}\"", g.0)) {
                    missing.push(g.0);
                }
            }
        }
        for g in super::PEDDLER_POOL {
            if !items.contains(&format!("\"{}\"", g.0)) {
                missing.push(g.0);
            }
        }
        assert!(missing.is_empty(), "{missing:?}");
    }
}
