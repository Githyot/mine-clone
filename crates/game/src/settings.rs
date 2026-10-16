//! Player settings, stored in `settings.ron` next to the game.

use crate::controller::ControllerSettings;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicMode {
    /// The game's own tracks.
    Classic,
    /// Only the songs in your music folder.
    Custom,
    /// Both mixed together.
    Both,
    Off,
}

impl MusicMode {
    pub fn next(self) -> Self {
        match self {
            MusicMode::Classic => MusicMode::Custom,
            MusicMode::Custom => MusicMode::Both,
            MusicMode::Both => MusicMode::Off,
            MusicMode::Off => MusicMode::Classic,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            MusicMode::Classic => "Clássica",
            MusicMode::Custom => "Minha pasta",
            MusicMode::Both => "Ambas",
            MusicMode::Off => "Desligada",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub render_distance: i32,
    pub fov: f32,
    pub mouse_sensitivity: f32,
    pub invert_mouse: bool,
    pub vsync: bool,
    pub music_mode: MusicMode,
    pub music_volume: f32,
    pub sound_volume: f32,
    /// Your own music. Put files directly here, or in sub-folders named
    /// overworld, underwater, creative, nether, end or title.
    pub music_folder: String,
    /// Silence between songs, like the classic game (seconds).
    pub music_delay_min: f32,
    pub music_delay_max: f32,
    /// Enabled resource packs from the `resourcepacks` folder, highest priority first.
    pub resource_packs: Vec<String>,
    pub show_clouds: bool,
    pub gui_scale: u32,
    pub controller: ControllerSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            render_distance: 10,
            fov: 70.0,
            mouse_sensitivity: 0.12,
            invert_mouse: false,
            vsync: true,
            music_mode: MusicMode::Classic,
            music_volume: 0.6,
            sound_volume: 0.8,
            music_folder: "music".into(),
            music_delay_min: 600.0,
            music_delay_max: 1200.0,
            resource_packs: Vec::new(),
            show_clouds: true,
            gui_scale: 0,
            controller: ControllerSettings::default(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        match std::fs::read_to_string(path) {
            Ok(text) => ron::from_str(&text).unwrap_or_else(|e| {
                log::warn!("settings.ron could not be read ({e}), using defaults");
                Settings::default()
            }),
            Err(_) => {
                let s = Settings::default();
                s.save(path);
                s
            }
        }
    }

    pub fn save(&self, path: &Path) {
        if let Ok(text) = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(path, text);
        }
    }
}
