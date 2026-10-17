//! The mod content pack: the mods HP asked for, rebuilt as our own code and art from the ideas
//! in `plan/mods/`. Each mod lives in its own file and hooks into the game through the few
//! calls below (update, use, keys, HUD, saving), so the base game files stay small.
//!
//! The pack's data (blocks, items, recipes, mobs, structures) is in `packs/vanilla/mods/`,
//! read after the base game's files.

pub mod aether;
pub mod combat;
pub mod dwellers;
pub mod ecologics;
pub mod macaws;
pub mod mounts;
pub mod portals;
pub mod seasons;

use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

use crate::{level_path_hint, Game};

/// What the mods remember about a world, saved in `mods.ron` next to `level.ron`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModSave {
    /// Whole days since the world was made (seasons count from it).
    pub day: u32,
    /// Season sensors placed in this world.
    pub sensors: Vec<[i32; 3]>,
    /// Buildings whose dwellers were already spawned: origin and dimension.
    pub dwellers: Vec<[i32; 4]>,
}

#[derive(Default)]
pub struct Mods {
    pub save: ModSave,
    /// Time of day last frame, to count days as midnight passes.
    last_time: f32,
    pub seasons: seasons::Seasons,
    dwell_t: f32,
    pub aether: aether::Aether,
    pub mount: mounts::Mount,
    /// How the player arrives in the dimension being loaded, when a mod portal sent them.
    pub arriving: Option<portals::Arrive>,
    pub combat: combat::Combat,
    pub macaws: macaws::Macaws,
    pub ecologics: ecologics::Ecologics,
}

impl Game {
    fn mods_file(&self) -> std::path::PathBuf {
        level_path_hint(&self.base, &self.world_name).join("mods.ron")
    }

    /// Read the open world's mod state (a fresh one for a new world).
    pub(crate) fn load_mods(&mut self) {
        let save = std::fs::read_to_string(self.mods_file()).ok().and_then(|t| ron::from_str::<ModSave>(&t).ok()).unwrap_or_default();
        self.mods = Mods { save, last_time: self.time_of_day, ..Default::default() };
    }

    pub(crate) fn save_mods(&self) {
        if !self.world_open {
            return;
        }
        if let Ok(text) = ron::ser::to_string_pretty(&self.mods.save, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(self.mods_file(), text);
        }
    }

    /// Every frame while a world is open and not paused.
    pub(crate) fn update_mods(&mut self, dt: f32) {
        if self.time_of_day + 1000.0 < self.mods.last_time {
            self.mods.save.day += 1;
        }
        self.mods.last_time = self.time_of_day;
        self.update_seasons(dt);
        self.update_aether(dt);
        if dt > 0.0 {
            self.update_dwellers(dt);
            self.update_mount(dt);
            self.update_combat();
            self.update_macaws(dt);
            self.update_ecologics(dt);
        }
    }

    /// Test hooks run once the world around the player has loaded (MC_* variables).
    pub(crate) fn mods_test_hooks(&mut self) {
        self.seasons_test_hook();
    }

    /// Right click: a mod block or item that does something. True when it was used.
    pub(crate) fn mods_use(&mut self) -> bool {
        self.aether_use() || self.seasons_use()
    }

    /// A key went down. True when a mod used it.
    pub(crate) fn mods_key(&mut self, _code: KeyCode) -> bool {
        false
    }

    /// Lines for the F3 screen.
    pub(crate) fn mods_info_lines(&self) -> Vec<String> {
        let mut v = vec![self.season_line()];
        v.extend(self.combat_line());
        v
    }

    /// HUD bits drawn over the game view.
    pub(crate) fn draw_mods_hud(&mut self, w: f32, h: f32) {
        self.draw_season_hud(w, h);
    }
}
