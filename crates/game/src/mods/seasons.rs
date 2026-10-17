//! Seasons (Serene Seasons idea, our own code): the year has four seasons of three parts
//! each. Grass and leaves turn with the year on the GPU (one number in the frame's uniforms,
//! no chunk is rebuilt): fresh green in spring, deep green in summer, orange and red in
//! autumn, a dull brown-green under the winter snow. Winter brings snow and ice to the
//! temperate biomes and spring melts them; crops have seasons they like (a glass roof makes a
//! greenhouse that ignores them); spring is rainy and summer stormy. A calendar tells the
//! date, and a season sensor powers redstone in the season it is set to.

use glam::{IVec3, Vec3};
use voxel::worldgen::{Biome, Dim};
use voxel::{BlockId, AIR};

use crate::Game;

/// Days in each third of a season (Serene Seasons' default year is 96 days).
pub const DAYS_PER_PART: u32 = 8;

pub const SEASON_NAMES: [&str; 4] = ["Primavera", "Verão", "Outono", "Inverno"];
const PART_NAMES: [&str; 12] = [
    "Início da Primavera", "Meio da Primavera", "Fim da Primavera",
    "Início do Verão", "Meio do Verão", "Fim do Verão",
    "Início do Outono", "Meio do Outono", "Fim do Outono",
    "Início do Inverno", "Meio do Inverno", "Fim do Inverno",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn from_index(i: usize) -> Season {
        [Season::Spring, Season::Summer, Season::Autumn, Season::Winter][i % 4]
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Default)]
pub struct Seasons {
    t: f32,
    leaf_t: f32,
    /// Season sensors placed in this world are found by looking around the player.
    sensor_t: f32,
    /// MC_SEASON: a fixed season part (0..11) for test pictures.
    pub fixed: Option<u32>,
}

/// Where the year is (0..1, 0 = the first day of spring), counting today's hours.
pub fn year_phase(day: u32, time_of_day: f32, fixed: Option<u32>) -> f32 {
    if let Some(p) = fixed {
        return (p as f32 + 0.5) / 12.0;
    }
    let year = (DAYS_PER_PART * 12) as f32;
    // Day 0 starts in the middle of spring so a new world opens on green grass.
    let d = day as f32 + time_of_day / 24000.0 + DAYS_PER_PART as f32;
    (d / year).fract()
}

/// Biomes that have seasons of rain and drought instead of a winter.
pub fn tropical(b: Biome) -> bool {
    matches!(b, Biome::Jungle | Biome::Savanna | Biome::Desert | Biome::Badlands)
}

impl Game {
    pub(crate) fn year(&self) -> f32 {
        year_phase(self.mods.save.day, self.time_of_day, self.mods.seasons.fixed)
    }

    /// The part of the year (0..11: early spring .. late winter).
    pub(crate) fn season_part(&self) -> usize {
        ((self.year() * 12.0) as usize).min(11)
    }

    pub(crate) fn season(&self) -> Season {
        Season::from_index(self.season_part() / 3)
    }

    pub(crate) fn season_name(&self) -> String {
        PART_NAMES[self.season_part()].to_string()
    }

    /// The value the shaders read (Globals.weather.w): 0 with seasons off (other dimensions),
    /// else 1 + the year phase.
    pub(crate) fn season_uniform(&self) -> f32 {
        if self.dim == Dim::Overworld && self.settings_seasons() {
            1.0 + self.year()
        } else {
            0.0
        }
    }

    fn settings_seasons(&self) -> bool {
        std::env::var("MC_NO_SEASONS").is_err()
    }

    /// How cold the season makes the temperate lands: 0 in summer, 1 in deep winter.
    fn season_cold(&self) -> f32 {
        // Per part: early spring .. late winter.
        const COLD: [f32; 12] = [0.5, 0.2, 0.05, 0.0, 0.0, 0.0, 0.05, 0.2, 0.45, 0.75, 1.0, 0.9];
        COLD[self.season_part()]
    }

    pub(crate) fn update_seasons(&mut self, dt: f32) {
        if !self.settings_seasons() {
            self.weather.season_cold = 0.0;
            return;
        }
        let part = self.season_part();
        self.weather.season_cold = if self.dim == Dim::Overworld { self.season_cold() } else { 0.0 };
        // Rain: wet spring, stormy summer, misty autumn, snowy winter.
        const RAIN: [f32; 4] = [1.7, 0.8, 1.3, 1.1];
        const STORM: [f32; 4] = [0.8, 2.0, 1.0, 0.3];
        self.weather.rain_odds = RAIN[part / 3];
        self.weather.storm_odds = STORM[part / 3];
        if dt <= 0.0 || self.dim != Dim::Overworld {
            return;
        }
        self.freeze_and_thaw(dt);
        self.autumn_leaves(dt);
        self.mods.seasons.sensor_t -= dt;
        if self.mods.seasons.sensor_t <= 0.0 {
            self.mods.seasons.sensor_t = 1.0;
            self.update_season_sensors();
        }
    }

    /// Winter freezes still water and lets snow lie in the temperate biomes; spring melts it.
    fn freeze_and_thaw(&mut self, dt: f32) {
        self.mods.seasons.t += dt * 40.0;
        let n = self.mods.seasons.t as i32;
        self.mods.seasons.t -= n as f32;
        let me = self.body.pos.floor().as_ivec3();
        let (ice, water, snow) = (self.reg.id_or_air("ice"), self.reg.id_or_air("water"), self.reg.id_or_air("snow_layer"));
        for _ in 0..n.min(40) {
            let x = me.x + self.surv.rng.range(-40, 41);
            let z = me.z + self.surv.rng.range(-40, 41);
            let biome = self.world.gen.biome_at(x, z);
            if matches!(biome, Biome::Snowy | Biome::Taiga) || tropical(biome) {
                continue;
            }
            let Some(y) = self.world.surface_y(x, z) else { continue };
            let p = IVec3::new(x, y, z);
            let id = self.world.block_or_air(p);
            let cold = self.weather.cold(biome);
            if cold {
                // Still water under the open sky freezes over.
                if id == water && (self.world.light(p + IVec3::Y) >> 4) >= 14 && self.surv.rng.chance(0.3) {
                    self.world.set_block(p, ice);
                }
            } else {
                let base = self.reg.get(id).base;
                if base == snow || self.reg.get(self.world.block_or_air(p + IVec3::Y)).base == snow {
                    let at = if base == snow { p } else { p + IVec3::Y };
                    self.world.set_block(at, AIR);
                } else if id == ice && y <= voxel::worldgen::SEA_LEVEL && self.surv.rng.chance(0.25) {
                    self.world.set_block(p, water);
                }
            }
        }
    }

    /// In autumn the trees shed orange and red leaves.
    fn autumn_leaves(&mut self, dt: f32) {
        let part = self.season_part();
        if !(6..=8).contains(&part) && part != 9 {
            return;
        }
        let rate = if part == 9 { 20.0 } else { 60.0 };
        self.mods.seasons.leaf_t += dt * rate;
        let n = self.mods.seasons.leaf_t as i32;
        self.mods.seasons.leaf_t -= n as f32;
        let me = self.body.pos.floor().as_ivec3();
        const AUTUMN: [[f32; 3]; 4] = [[0.95, 0.55, 0.15], [0.85, 0.3, 0.12], [0.95, 0.75, 0.2], [0.6, 0.4, 0.2]];
        for _ in 0..n.min(30) {
            let r = &mut self.surv.rng;
            let p = me + IVec3::new(r.range(-12, 13), r.range(-2, 12), r.range(-12, 13));
            let jit = Vec3::new(r.f32(), 0.0, r.f32());
            let tint = AUTUMN[r.below(4) as usize];
            let id = self.world.block_or_air(p);
            if !self.reg.get(id).name.ends_with("_leaves") || self.world.block_or_air(p - IVec3::Y) != AIR {
                continue;
            }
            if tropical(self.world.gen.biome_at(p.x, p.z)) {
                continue;
            }
            let (layer, _) = self.block_look(id);
            self.particles.mote(p.as_vec3() + Vec3::new(jit.x, -0.05, jit.z), layer, tint, 0.06, 0.7, 7.0);
        }
    }

    /// Crops grow best in their own seasons. Out of season they grow slowly, and not at all
    /// in the cold, unless a glass roof makes a greenhouse over them. False skips this tick.
    pub(crate) fn season_lets_grow(&self, p: IVec3, crop: BlockId, r: u32) -> bool {
        if self.dim != Dim::Overworld || !self.settings_seasons() || tropical(self.world.gen.biome_at(p.x, p.z)) {
            return true;
        }
        let name = &self.reg.get(crop).name;
        // Fertile seasons per crop: spring, summer, autumn, winter.
        let fertile: [bool; 4] = if name.starts_with("wheat") {
            [true, true, true, false]
        } else if name.starts_with("carrots") {
            [true, false, true, true]
        } else if name.starts_with("potatoes") {
            [true, true, true, false]
        } else if name.starts_with("beetroots") {
            [false, true, true, true]
        } else {
            return true;
        };
        if self.greenhouse(p) {
            return true;
        }
        let s = self.season().index();
        if fertile[s] {
            return true;
        }
        // Frost stops growth; a mild off-season only slows it.
        if self.season_cold() > 0.6 {
            return false;
        }
        r % 4 == 0
    }

    /// Glass somewhere in the 12 blocks above, with nothing solid and opaque in between.
    fn greenhouse(&self, p: IVec3) -> bool {
        for k in 1..=12 {
            let id = self.world.block_or_air(p + IVec3::Y * k);
            if id == AIR {
                continue;
            }
            let b = self.reg.get(id);
            return b.name.contains("glass");
        }
        false
    }

    /// Season sensors close to the player: on (a power source) while it is their season.
    fn update_season_sensors(&mut self) {
        let s = self.season().index();
        let me = self.body.pos.floor().as_ivec3();
        let sensors: Vec<(IVec3, BlockId)> = self
            .mods
            .save
            .sensors
            .iter()
            .filter(|p| (IVec3::from(**p) - me).abs().max_element() < 96)
            .filter_map(|p| self.world.block(IVec3::from(*p)).map(|id| (IVec3::from(*p), id)))
            .collect();
        let mut gone = Vec::new();
        for (p, id) in sensors {
            let name = self.reg.get(id).name.clone();
            let Some(rest) = name.strip_prefix("season_sensor_") else {
                gone.push(p.to_array());
                continue;
            };
            let (which, on) = match rest.strip_suffix("_on") {
                Some(w) => (w, true),
                None => (rest, false),
            };
            let Some(k) = SENSOR_SEASONS.iter().position(|&n| n == which) else { continue };
            let want_on = k == s;
            if want_on != on {
                let next = if want_on { format!("season_sensor_{which}_on") } else { format!("season_sensor_{which}") };
                if let Some(nid) = self.reg.id(&next) {
                    self.world.set_block(p, nid);
                }
            }
        }
        self.mods.save.sensors.retain(|p| !gone.contains(p));
    }

    /// Right click: a calendar tells the date; a season sensor is placed (and remembered) or
    /// turned to the next season.
    pub(crate) fn seasons_use(&mut self) -> bool {
        if let Some(hit) = self.target {
            let name = self.reg.get(self.world.block_or_air(hit.block)).name.clone();
            if let Some(rest) = name.strip_prefix("season_sensor_") {
                if self.crouch <= 0.5 {
                    let which = rest.trim_end_matches("_on");
                    let k = SENSOR_SEASONS.iter().position(|&n| n == which).unwrap_or(0);
                    let next = SENSOR_SEASONS[(k + 1) % 4];
                    if let Some(id) = self.reg.id(&format!("season_sensor_{next}")) {
                        self.world.set_block(hit.block, id);
                    }
                    self.toast(format!("Sensor de estação: {}", SEASON_NAMES[(k + 1) % 4]), 2.0);
                    self.mods.seasons.sensor_t = 0.0;
                    return true;
                }
            }
        }
        let Some(st) = self.held_stack() else { return false };
        let item = self.items.get(st.item).name.clone();
        if item == "calendar" {
            let text = format!("{}: dia {} do ano", self.season_name(), self.mods.save.day % (DAYS_PER_PART * 12) + 1);
            self.toast(text, 4.0);
            self.anim.start_swing();
            return true;
        }
        if item == "season_sensor_spring" {
            let Some(id) = self.reg.id("season_sensor_spring") else { return false };
            if let Some(pos) = self.place_block_as(id) {
                if self.survival {
                    let sel = self.selected;
                    self.surv.inv.take(sel, 1);
                }
                self.mods.save.sensors.push(pos.to_array());
                self.mods.seasons.sensor_t = 0.0;
                return true;
            }
        }
        false
    }

    pub(crate) fn season_line(&self) -> String {
        if self.season_uniform() == 0.0 {
            return "Estação: -".into();
        }
        format!("Estação: {} (dia {} do ano, frio {:.0}%)", self.season_name(), self.mods.save.day % (DAYS_PER_PART * 12) + 1, self.weather.season_cold * 100.0)
    }

    /// Holding a calendar: the season in a small card at the top left.
    pub(crate) fn draw_season_hud(&mut self, _w: f32, _h: f32) {
        let Some(st) = self.held_stack() else { return };
        if self.items.get(st.item).name != "calendar" || self.season_uniform() == 0.0 {
            return;
        }
        let s = self.ui.scale;
        let text = self.season_name();
        let day = format!("Dia {} de {}", self.mods.save.day % (DAYS_PER_PART * 12) + 1, DAYS_PER_PART * 12);
        let tw = self.ui.text_width(&text, s).max(self.ui.text_width(&day, s));
        const COLORS: [[u8; 4]; 4] = [[120, 200, 110, 255], [240, 210, 80, 255], [220, 120, 50, 255], [170, 200, 230, 255]];
        let c = COLORS[self.season().index()];
        self.ui.rect(4.0 * s, 4.0 * s, tw + 10.0 * s, 24.0 * s, [0, 0, 0, 140]);
        self.ui.rect(4.0 * s, 4.0 * s, 2.0 * s, 24.0 * s, c);
        self.ui.text(9.0 * s, 6.0 * s, &text, c);
        self.ui.text(9.0 * s, 16.0 * s, &day, [220, 220, 220, 255]);
    }

    /// MC_SEASON=0..11 holds the year at one part (test pictures); MC_DAY=n sets the day.
    pub(crate) fn seasons_test_hook(&mut self) {
        if let Some(p) = std::env::var("MC_SEASON").ok().and_then(|v| v.parse::<u32>().ok()) {
            self.mods.seasons.fixed = Some(p.min(11));
            if p >= 9 {
                // Let winter show: a light snowfall already lying.
                self.weather.season_cold = 1.0;
                for _ in 0..400 {
                    self.settle_snow(0.5);
                }
            }
        }
        if let Some(d) = std::env::var("MC_DAY").ok().and_then(|v| v.parse::<u32>().ok()) {
            self.mods.save.day = d;
        }
    }
}

/// Season sensor block names, in season order.
const SENSOR_SEASONS: [&str; 4] = ["spring", "summer", "autumn", "winter"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn year_wraps_and_starts_in_spring() {
        let p = year_phase(0, 0.0, None);
        assert!(p > 0.0 && p < 0.25, "{p}");
        let y = DAYS_PER_PART * 12;
        assert!((year_phase(y, 0.0, None) - p).abs() < 1e-4);
        assert_eq!(((year_phase(0, 0.0, Some(10)) * 12.0) as usize), 10);
    }
}
