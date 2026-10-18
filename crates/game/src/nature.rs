//! Hard survival (Lote 8 of plan/o-que-falta.md; ideas from Tough As Nails and the Aged
//! modpack, no code or art from them): body temperature, thirst and nutrition. Each is a world
//! rule, off by default: `/regra temperatura sim`, `/regra sede sim`, `/regra nutricao sim`.
//!
//! - Temperature: the body drifts toward what the surroundings give (biome, season, height,
//!   night, rain, water, fire and ice nearby, clothes, running). Too cold slows and then hurts;
//!   too hot dries the player out and then hurts. Leather keeps warm; a fire resistance potion
//!   takes the heat away.
//! - Thirst: a second bar of drops over the hunger bar, drained by time, running and heat.
//!   Water bottles, milk, honey, soups and juicy fruit fill it; with an empty hand, a sip from
//!   any water (it may not agree with you). Empty, it hurts like hunger does.
//! - Nutrition: five food groups (fruit, vegetables, protein, grains, sweets and milk) slowly
//!   empty and fill as they are eaten. Eating all of them now and then mends the body a little;
//!   eating only one or two leaves it weak. `/nutricao` shows them.

use std::path::Path;

use glam::IVec3;
use serde::{Deserialize, Serialize};
use voxel::worldgen::{Biome, Dim};

use crate::effects::{FIRE_RESISTANCE, NAUSEA, SLOWNESS, WEAKNESS};
use crate::Game;

pub const GROUPS: [&str; 5] = ["Frutas", "Verduras", "Proteínas", "Grãos", "Doces e leite"];
/// Seconds for a full food group to empty.
const GROUP_LASTS: f32 = 2400.0;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub(crate) struct Nature {
    /// Body temperature, -10 (freezing) .. 10 (burning); 0 is comfortable.
    pub temp: f32,
    /// Where the surroundings pull it.
    pub target: f32,
    /// 0..20, like hunger.
    pub thirst: f32,
    thirst_ex: f32,
    /// Food groups, 0..1 each.
    pub food: [f32; 5],
    #[serde(skip)]
    env_t: f32,
    #[serde(skip)]
    hurt_t: f32,
    #[serde(skip)]
    mend_t: f32,
    #[serde(skip)]
    effect_t: f32,
}

impl Default for Nature {
    fn default() -> Self {
        Nature { temp: 0.0, target: 0.0, thirst: 20.0, thirst_ex: 0.0, food: [0.5; 5], env_t: 0.0, hurt_t: 0.0, mend_t: 0.0, effect_t: 0.0 }
    }
}

pub(crate) fn load(dir: &Path) -> Nature {
    std::fs::read_to_string(dir.join("nature.ron")).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default()
}

pub(crate) fn save(dir: &Path, n: &Nature) {
    if let Ok(t) = ron::ser::to_string_pretty(n, Default::default()) {
        let _ = std::fs::write(dir.join("nature.ron"), t);
    }
}

/// How warm a biome is, -10..10 (0 = mild).
fn biome_heat(b: Biome) -> f32 {
    match b {
        Biome::Snowy | Biome::SnowyPeaks | Biome::FrozenOcean => -6.0,
        Biome::Taiga => -3.0,
        Biome::Mountains => -2.0,
        Biome::Ocean | Biome::River => -1.0,
        Biome::Plains | Biome::Forest | Biome::BirchForest | Biome::DarkForest | Biome::CherryGrove | Biome::Meadow | Biome::MushroomFields => 0.0,
        Biome::Beach | Biome::Swamp | Biome::MangroveSwamp => 1.0,
        Biome::Jungle => 3.0,
        Biome::Savanna => 4.0,
        Biome::Badlands => 5.0,
        Biome::Desert => 6.0,
        _ => 0.0,
    }
}

/// Which food groups an item feeds.
pub(crate) fn groups_of(item: &str) -> &'static [usize] {
    match item {
        "apple" | "golden_apple" | "enchanted_golden_apple" | "melon_slice" | "sweet_berries" | "glow_berries" | "chorus_fruit" => &[0],
        "carrot" | "golden_carrot" | "potato" | "baked_potato" | "poisonous_potato" | "beetroot" | "beetroot_soup" | "dried_kelp" | "mushroom_stew" | "suspicious_stew" => &[1],
        "rabbit_stew" => &[1, 2],
        "bread" | "wheat" => &[3],
        "cookie" => &[3, 4],
        "cake" | "pumpkin_pie" => &[3, 4],
        "honey_bottle" | "milk_bucket" | "sugar" => &[4],
        n if n.starts_with("cooked_") || n.contains("beef") || n.contains("porkchop") || n.contains("mutton") || n.contains("chicken") || n.contains("rabbit") || n.contains("cod") || n.contains("salmon") || n == "tropical_fish" || n == "pufferfish" || n == "rotten_flesh" || n == "spider_eye" || n == "egg" => &[2],
        _ => &[],
    }
}

/// How much a drink or food quenches thirst (in half drops).
fn quench(item: &str) -> f32 {
    match item {
        "potion_water" => 7.0,
        "milk_bucket" => 5.0,
        "honey_bottle" => 3.0,
        "beetroot_soup" | "mushroom_stew" | "rabbit_stew" | "suspicious_stew" => 3.0,
        "melon_slice" => 2.0,
        "apple" | "sweet_berries" | "glow_berries" | "chorus_fruit" => 1.0,
        n if n.starts_with("potion_") => 3.0,
        _ => 0.0,
    }
}

impl Game {
    fn rule_temp(&self) -> bool {
        self.net.rules.temperature
    }

    fn rule_thirst(&self) -> bool {
        self.net.rules.thirst
    }

    fn rule_food(&self) -> bool {
        self.net.rules.nutrition
    }

    /// The thirst bar is on screen.
    pub(crate) fn thirst_shown(&self) -> bool {
        self.rule_thirst() && self.survival
    }

    /// What the surroundings make the body tend to.
    fn env_heat(&self) -> f32 {
        let p = self.body.pos.floor().as_ivec3();
        let mut t = match self.dim {
            Dim::Nether => 8.0,
            Dim::End => -3.0,
            _ => {
                let b = self.world.gen.base_biome_at(p.x, p.z);
                let mut t = biome_heat(b);
                // Winter cools the lands that have one; it barely touches the hot ones.
                t -= self.weather.season_cold * if t >= 3.0 { 1.0 } else { 4.0 };
                if p.y > 110 {
                    t -= ((p.y - 110) as f32 / 25.0).min(4.0);
                }
                let sky = (self.world.light(p) >> 4) > 8;
                if sky {
                    if self.is_night() {
                        t -= 1.5;
                    }
                    if self.weather.state.raining {
                        t -= 1.5;
                    }
                } else if p.y < 50 {
                    // Deep caves keep an even chill whatever is above.
                    t = t * 0.3 - 0.5;
                }
                t
            }
        };
        if self.body.in_water {
            t -= 3.0;
        }
        // Fire and ice nearby.
        let mut warm = 0.0f32;
        let mut chill = 0.0f32;
        for dy in -2..=2 {
            for dz in -3..=3 {
                for dx in -3..=3 {
                    let id = self.world.block_or_air(p + IVec3::new(dx, dy, dz));
                    if id == voxel::block::AIR {
                        continue;
                    }
                    let n = self.reg.get(self.reg.get(id).base).name.as_str();
                    if n == "lava" || n == "fire" || n.contains("campfire") || n == "magma_block" || n.ends_with("furnace_lit") || n == "lit_furnace" {
                        warm += 1.5;
                    } else if n.contains("torch") && !n.contains("redstone") || n.contains("lantern") {
                        warm += 0.5;
                    } else if n.contains("ice") || n == "snow_block" || n == "powder_snow" {
                        chill += 0.5;
                    }
                }
            }
        }
        t += warm.min(6.0) - chill.min(3.0);
        // Clothes: leather keeps warm; heavy armour cooks in the heat.
        for s in self.surv.armor.iter().flatten() {
            let n = self.items.get(s.item).name.as_str();
            if n.starts_with("leather_") {
                t += 0.75;
            } else if t > 3.0 && (n.starts_with("iron_") || n.starts_with("diamond_") || n.starts_with("netherite_") || n.starts_with("golden_")) {
                t += 0.5;
            }
        }
        if self.sprinting {
            t += 1.0;
        }
        if self.has_effect(FIRE_RESISTANCE) {
            t = t.min(2.0);
        }
        t.clamp(-10.0, 10.0)
    }

    pub(crate) fn nature_update(&mut self, dt: f32) {
        if !self.survival || self.screen == crate::Screen::Dead || dt <= 0.0 {
            return;
        }
        let (temp, thirst, food) = (self.rule_temp(), self.rule_thirst(), self.rule_food());
        if !(temp || thirst || food) {
            return;
        }
        self.nature.env_t -= dt;
        if temp && self.nature.env_t <= 0.0 {
            self.nature.env_t = 1.0;
            self.nature.target = self.env_heat();
        }
        if temp {
            let rate = if self.body.in_water { 0.25 } else { 0.06 };
            let n = &mut self.nature;
            n.temp += (n.target - n.temp).clamp(-rate * dt * 4.0, rate * dt * 4.0) * 0.25;
            n.temp = n.temp.clamp(-10.0, 10.0);
        } else {
            self.nature.temp = 0.0;
        }
        if thirst {
            let moved = if self.sprinting { 0.02 } else { 0.0 };
            let heat = if temp && self.nature.temp > 5.0 { 2.0 } else { 1.0 };
            self.nature.thirst_ex += dt * (1.0 / 60.0) * heat + moved * dt * 4.0;
            while self.nature.thirst_ex >= 1.0 {
                self.nature.thirst_ex -= 1.0;
                self.nature.thirst = (self.nature.thirst - 1.0).max(0.0);
            }
            if self.nature.thirst <= 6.0 && self.sprinting {
                // Too parched to run.
                self.sprinting = false;
            }
        }
        if food {
            for g in self.nature.food.iter_mut() {
                *g = (*g - dt / GROUP_LASTS).max(0.0);
            }
        }
        // Harm and help, every few seconds.
        self.nature.hurt_t += dt;
        if self.nature.hurt_t >= 4.0 {
            self.nature.hurt_t = 0.0;
            let t = self.nature.temp;
            if temp && t <= -9.0 {
                self.toast("Hipotermia: procure calor (fogo, roupas de couro)", 2.5);
                self.damage(1.0, None);
            } else if temp && t >= 9.0 {
                self.toast("Insolação: procure sombra, água ou o frio", 2.5);
                self.damage(1.0, None);
            }
            if thirst && self.nature.thirst <= 0.0 && self.health > 1.0 {
                self.toast("Com sede: beba água", 2.5);
                self.damage(1.0, None);
            }
        }
        self.nature.effect_t -= dt;
        if self.nature.effect_t <= 0.0 {
            self.nature.effect_t = 5.0;
            if temp && self.nature.temp <= -6.0 {
                self.add_effect(SLOWNESS, 0, 6.0, true);
            }
            if food {
                let fed = self.nature.food.iter().filter(|&&g| g >= 0.3).count();
                if fed <= 1 {
                    self.add_effect(WEAKNESS, 0, 6.0, true);
                }
            }
        }
        if food && self.nature.food.iter().all(|&g| g >= 0.3) && self.health < self.max_health {
            // A varied diet mends a little on top of the usual healing.
            self.nature.mend_t += dt;
            if self.nature.mend_t >= 20.0 {
                self.nature.mend_t = 0.0;
                self.health = (self.health + 1.0).min(self.max_health);
            }
        }
    }

    /// Something was eaten or drunk (after it was used up).
    pub(crate) fn nature_consumed(&mut self, item: &str) {
        if self.rule_thirst() {
            self.nature.thirst = (self.nature.thirst + quench(item)).min(20.0);
        }
        if self.rule_food() {
            for &g in groups_of(item) {
                self.nature.food[g] = (self.nature.food[g] + 0.25).min(1.0);
            }
        }
    }

    /// Right click with an empty hand on water nearby: a sip.
    pub(crate) fn nature_sip(&mut self) -> bool {
        if !self.thirst_shown() || self.held_stack().is_some() || self.nature.thirst >= 20.0 {
            return false;
        }
        let eye = self.body.eye();
        let f = self.forward();
        let mut hit = false;
        for i in 1..=10 {
            let p = (eye + f * (i as f32 * 0.25)).floor().as_ivec3();
            let id = self.world.block_or_air(p);
            if self.world.is_liquid(id) && self.reg.get(self.reg.get(id).base).name.contains("water") {
                hit = true;
                break;
            }
            if id != voxel::block::AIR && !self.world.is_liquid(id) {
                break;
            }
        }
        if !hit {
            return false;
        }
        self.nature.thirst = (self.nature.thirst + 2.0).min(20.0);
        self.sfx(crate::audio::Sfx::Bubbles);
        self.anim.start_swing();
        // Water from the ground may not agree with you.
        if self.fx.rand() < 0.25 {
            self.add_effect(NAUSEA, 0, 6.0, false);
        }
        true
    }

    pub(crate) fn nutrition_line(&self) -> String {
        if !self.rule_food() {
            return "Nutrição desligada (/regra nutricao sim)".into();
        }
        let parts: Vec<String> = GROUPS.iter().zip(self.nature.food.iter()).map(|(n, v)| format!("{n} {:.0}%", v * 100.0)).collect();
        parts.join(", ")
    }

    /// The thirst drops over the hunger bar and the thermometer by the hotbar.
    pub(crate) fn draw_nature_hud(&mut self, hx: f32, hy: f32, cell: f32) {
        if !self.survival {
            return;
        }
        let s = self.ui.scale;
        if self.rule_thirst() {
            let y = hy - 21.0 * s;
            for i in 0..10 {
                let x = hx + cell * 9.0 - (i + 1) as f32 * 8.0 * s;
                let v = self.nature.thirst - i as f32 * 2.0;
                draw_drop(&mut self.ui, x, y, s, 1.0, true);
                if v > 0.0 {
                    draw_drop(&mut self.ui, x, y, s, if v >= 2.0 { 1.0 } else { 0.5 }, false);
                }
            }
        }
        if self.rule_temp() {
            // A small thermometer left of the hotbar: blue cold, green fine, red hot.
            let t = self.nature.temp;
            let (x, y) = (hx - 10.0 * s, hy + 2.0 * s);
            let bh = 16.0 * s;
            self.ui.rect(x - s, y - s, 5.0 * s, bh + 2.0 * s, [20, 20, 20, 200]);
            let k = (t + 10.0) / 20.0;
            let col = if t <= -6.0 {
                [90, 150, 255, 255]
            } else if t >= 6.0 {
                [255, 90, 60, 255]
            } else if t.abs() >= 3.0 {
                [240, 220, 90, 255]
            } else {
                [110, 210, 110, 255]
            };
            let fill = (k * bh).clamp(s, bh);
            self.ui.rect(x, y + bh - fill, 3.0 * s, fill, col);
            self.ui.rect(x - 2.0 * s, y + bh / 2.0, 7.0 * s, (s * 0.5).max(1.0), [255, 255, 255, 160]);
        }
    }
}

/// A water drop, 7×7 "pixels": `fill` < 1 draws only its right half.
fn draw_drop(ui: &mut crate::ui::UiBatch, x: f32, y: f32, s: f32, fill: f32, empty: bool) {
    const ROWS: [&str; 7] = ["...W...", "..WWW..", ".WWWWW.", ".WWHWW.", "WWWHWWW", ".WWWWW.", "..WWW.."];
    for (r, row) in ROWS.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            if fill < 1.0 && c < 3 {
                continue;
            }
            let col = match ch {
                'W' if empty => [20, 30, 50, 255],
                'H' if empty => [20, 30, 50, 255],
                'W' => [60, 120, 230, 255],
                'H' => [190, 220, 255, 255],
                _ => continue,
            };
            ui.rect(x + c as f32 * s, y + r as f32 * s, s, s, col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foods_have_groups_and_drinks_quench() {
        assert_eq!(groups_of("cooked_beef"), &[2]);
        assert_eq!(groups_of("apple"), &[0]);
        assert_eq!(groups_of("bread"), &[3]);
        assert!(groups_of("stone").is_empty());
        assert!(quench("potion_water") > quench("apple"));
        assert_eq!(quench("bread"), 0.0);
        assert!(biome_heat(Biome::Desert) > 0.0 && biome_heat(Biome::Snowy) < 0.0);
    }
}
