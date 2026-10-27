//! Archaeology (Phase 6): brushing suspicious sand and gravel uncovers pottery sherds and other
//! finds, sherds make decorated pots, and sniffer eggs found in warm ocean ruins hatch into
//! sniffers that dig up ancient seeds (torchflower, pitcher pod).

use glam::{IVec3, Vec3};
use voxel::block::AIR;
use voxel::item::ItemStack;

use crate::audio::{Material, Sfx};
use crate::progression::Event;

/// The 23 pottery sherds, with their names.
pub const SHERDS: [(&str, &str); 23] = [
    ("angler", "Pescador"),
    ("archer", "Arqueiro"),
    ("arms_up", "Braços Erguidos"),
    ("blade", "Lâmina"),
    ("brewer", "Alquimista"),
    ("burn", "Queimadura"),
    ("danger", "Perigo"),
    ("explorer", "Explorador"),
    ("flow", "Fluxo"),
    ("friend", "Amigo"),
    ("guster", "Ventania"),
    ("heart", "Coração"),
    ("heartbreak", "Coração Partido"),
    ("howl", "Uivo"),
    ("miner", "Mineiro"),
    ("mourner", "Enlutado"),
    ("plenty", "Fartura"),
    ("prize", "Prêmio"),
    ("scrape", "Raspagem"),
    ("sheaf", "Feixe"),
    ("shelter", "Abrigo"),
    ("skull", "Caveira"),
    ("snort", "Fungada"),
];

/// What a suspicious block holds, by where it is: desert (wells, pyramids), warm oceans and
/// beaches (ruins, sniffer eggs) or the trail ruins under the grass and taiga.
fn finds(sand: bool, biome: &str) -> &'static [(&'static str, u32)] {
    match (sand, biome) {
        (true, "Desert" | "Badlands") => &[
            ("archer_pottery_sherd", 3),
            ("miner_pottery_sherd", 3),
            ("prize_pottery_sherd", 3),
            ("skull_pottery_sherd", 3),
            ("arms_up_pottery_sherd", 2),
            ("brewer_pottery_sherd", 2),
            ("diamond", 1),
            ("emerald", 2),
            ("tnt", 1),
            ("gunpowder", 2),
        ],
        (true, _) => &[
            ("angler_pottery_sherd", 3),
            ("shelter_pottery_sherd", 3),
            ("snort_pottery_sherd", 3),
            ("sniffer_egg", 2),
            ("iron_axe", 1),
            ("emerald", 2),
            ("wheat", 2),
            ("coal", 2),
            ("gold_nugget", 2),
        ],
        (false, _) => &[
            ("burn_pottery_sherd", 2),
            ("danger_pottery_sherd", 2),
            ("friend_pottery_sherd", 2),
            ("heart_pottery_sherd", 2),
            ("heartbreak_pottery_sherd", 2),
            ("howl_pottery_sherd", 2),
            ("sheaf_pottery_sherd", 2),
            ("blade_pottery_sherd", 2),
            ("explorer_pottery_sherd", 2),
            ("mourner_pottery_sherd", 2),
            ("plenty_pottery_sherd", 2),
            ("wayfinder_armor_trim_smithing_template", 1),
            ("raiser_armor_trim_smithing_template", 1),
            ("shaper_armor_trim_smithing_template", 1),
            ("host_armor_trim_smithing_template", 1),
            ("music_disc_relic", 1),
            ("emerald", 3),
            ("wheat", 2),
            ("wooden_hoe", 1),
            ("clay_ball", 3),
            ("brick", 3),
            ("blue_dye", 1),
            ("lead", 1),
            ("candle", 2),
            ("beetroot_seeds", 2),
        ],
    }
}

/// Seconds of brushing to uncover a find.
const BRUSH_TIME: f32 = 1.6;

impl crate::Game {
    /// Holding use with a brush on suspicious sand or gravel.
    pub(crate) fn update_brushing(&mut self, dt: f32, use_held: bool) {
        let holding = self.held_key() == "brush";
        let target = self.target.map(|t| t.block);
        let (Some(b), true, true) = (target, holding, use_held) else {
            self.prog.brushing = None;
            return;
        };
        let name = self.reg.get(self.world.block_or_air(b)).name.clone();
        let sand = match name.as_str() {
            "suspicious_sand" => true,
            "suspicious_gravel" => false,
            _ => {
                self.prog.brushing = None;
                return;
            }
        };
        let t = match self.prog.brushing {
            Some((p, t)) if p == b => t + dt,
            _ => 0.0,
        };
        self.prog.brushing = Some((b, t));
        // Dust flies, the brush scratches softly.
        if ((t - dt) * 4.0) as i32 != (t * 4.0) as i32 {
            self.anim.start_swing();
            self.sfx(Sfx::Hit(if sand { Material::Sand } else { Material::Gravel }));
            let (layer, tint) = self.block_look(self.world.block_or_air(b));
            self.particles.block_bits(b, layer, tint, 4, 0.4);
        }
        if t < BRUSH_TIME {
            return;
        }
        self.prog.brushing = None;
        let biome = self.world.gen.base_biome_at(b.x, b.z).name().to_string();
        let table = finds(sand, &biome);
        let total: u32 = table.iter().map(|f| f.1).sum();
        let mut r = self.prog.rng.below(total);
        let mut found = None;
        for &(item, w) in table {
            if r < w {
                found = Some(item);
                break;
            }
            r -= w;
        }
        let plain = self.reg.id_or_air(if sand { "sand" } else { "gravel" });
        self.world.set_block(b, plain);
        if let Some(id) = found.and_then(|n| self.items.id(n)) {
            let face = self.target.map(|t| t.normal).unwrap_or(IVec3::Y);
            self.pop_drop(b.as_vec3() + Vec3::splat(0.5) + face.as_vec3() * 0.6, ItemStack::new(id, 1));
        }
        self.prog.event(Event::Brushed);
        self.prog.stats.add("brushed", 1);
        // The brush wears a little.
        if self.survival {
            self.wear_held(1);
        }
    }

    /// Sniffer eggs near the player hatch (faster on moss), and sniffers dig up ancient seeds.
    pub(crate) fn update_archaeology(&mut self, dt: f32, use_held: bool) {
        self.update_brushing(dt, use_held);
        if dt <= 0.0 {
            return;
        }
        let egg = self.reg.id_or_air("sniffer_egg");
        if egg != AIR {
            for p in self.near_of(&["sniffer_egg"]) {
                if !self.prog.eggs.iter().any(|(q, _)| *q == p) {
                    let moss = self.reg.get(self.world.block_or_air(p - IVec3::Y)).name == "moss_block";
                    self.prog.eggs.push((p, if moss { 600.0 } else { 1200.0 }));
                }
            }
            let mut i = 0;
            while i < self.prog.eggs.len() {
                let (p, t) = self.prog.eggs[i];
                if self.world.block(p).is_none() {
                    i += 1;
                    continue;
                }
                if self.world.block_or_air(p) != egg {
                    self.prog.eggs.swap_remove(i);
                    continue;
                }
                let t = t - dt;
                if t <= 0.0 {
                    self.world.set_block(p, AIR);
                    self.mobs.spawn("sniffer", p.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
                    self.prog.event(Event::Token("hatched".into()));
                    self.sfx(Sfx::Break(Material::Glass));
                    self.prog.eggs.swap_remove(i);
                    continue;
                }
                self.prog.eggs[i].1 = t;
                i += 1;
            }
        }
        // Sniffers: every so often one sniffs out a seed in grass, dirt or moss under it.
        self.prog.sniff_t -= dt;
        if self.prog.sniff_t > 0.0 {
            return;
        }
        self.prog.sniff_t = 40.0 + self.prog.rng.f32() * 30.0;
        let Some(si) = self.mobs.reg.id("sniffer") else { return };
        let me = self.body.pos;
        let spots: Vec<Vec3> = self.mobs.list.iter().filter(|m| m.def == si && m.alive() && (m.body.pos - me).length() < 32.0).map(|m| m.body.pos).collect();
        for at in spots {
            let under = (at - Vec3::Y * 0.2).floor().as_ivec3();
            let ground = self.reg.get(self.world.block_or_air(under)).name.clone();
            if !matches!(ground.as_str(), "grass_block" | "dirt" | "moss_block" | "rocky_dirt" | "podzol" | "mud") {
                continue;
            }
            let seed = if self.prog.rng.f32() < 0.5 { "torchflower_seeds" } else { "pitcher_pod" };
            if let Some(id) = self.items.id(seed) {
                self.pop_drop(at + Vec3::Y * 0.3, ItemStack::new(id, 1));
                let (layer, tint) = self.block_look(self.world.block_or_air(under));
                self.particles.block_bits(under, layer, tint, 8, 0.5);
                self.sfx(Sfx::Hit(Material::Gravel));
                self.prog.event(Event::Sniffed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sherd_can_be_found_or_comes_from_trials() {
        let all: Vec<&str> = [finds(true, "Desert"), finds(true, "Ocean"), finds(false, "Plains")].iter().flat_map(|t| t.iter().map(|f| f.0)).collect();
        for (s, _) in SHERDS {
            let item = format!("{s}_pottery_sherd");
            // Flow, guster and scrape come from trial chamber vaults and pots.
            if matches!(s, "flow" | "guster" | "scrape") {
                continue;
            }
            assert!(all.contains(&item.as_str()), "{item} can't be found");
        }
    }
}
