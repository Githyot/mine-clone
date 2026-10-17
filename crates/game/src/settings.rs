//! Player settings, stored in `settings.ron` next to the game.

use crate::controller::ControllerSettings;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

/// How much work the shader packs may do.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderQuality {
    Fast,
    Fancy,
}

impl ShaderQuality {
    pub fn label(self) -> &'static str {
        match self {
            ShaderQuality::Fast => "Rápido",
            ShaderQuality::Fancy => "Bonito",
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
    /// The bundled pack was turned on once already (so turning it off sticks).
    pub bundled_pack_offered: bool,
    /// Creative: hotbars saved by name (up to 9 rows of 9 items).
    pub saved_hotbars: Vec<Vec<String>>,
    pub show_clouds: bool,
    /// The view sways a little while walking.
    pub view_bobbing: bool,
    /// Items on the ground lie flat and still (like the Item Drops mod) instead of floating.
    pub flat_drops: bool,
    /// Animals and monsters spawn in the world.
    pub mobs: bool,
    /// Textures get a little carved relief near the camera.
    pub relief: bool,
    /// Largest texture size in pixels per block face (16, 32, 64, 128). The game's own art is 64;
    /// lower it on computers with little video memory.
    pub texture_resolution: u32,
    /// 0 peaceful (no monsters, health comes back), 1 easy, 2 normal, 3 hard.
    pub difficulty: u8,
    pub gui_scale: u32,
    pub controller: ControllerSettings,
    /// Shader pack in use: a built-in name or a folder in `shaderpacks/` ("" = off).
    pub shader_pack: String,
    /// Option values chosen for each shader pack: pack → option name → value.
    pub shader_options: BTreeMap<String, BTreeMap<String, f32>>,
    pub shader_quality: ShaderQuality,
    /// Low-detail distant terrain drawn this many blocks past the render distance (0 = off).
    pub lod_distance: i32,
    /// Buildings farther apart in new chunks (Sparse Structures idea).
    pub sparse_structures: bool,
    /// Multiplayer guest: the host's structure spacing, while connected (never saved).
    #[serde(skip)]
    pub spread_override: Option<i32>,
    /// Several cloud layers drifting with the wind (Better Clouds idea); off = one classic layer.
    /// Off by default: on slow (software) renderers it costs a fifth or more of the frame rate
    /// when looking at the sky.
    pub layered_clouds: bool,
    /// Sun and moon shadows (Sildur's idea): 0 off, 1 fast (small map, near), 2 fancy.
    pub shadows: u8,
    /// Water mirrors the world around it (needs a shader pack: it reads the drawn scene).
    pub water_reflections: bool,
    /// Temporal anti-aliasing: smooth edges from jittered frames (needs a shader pack).
    pub taa: bool,
    /// Glowing bits of ore textures stay bright in the dark (Continuity's emissive textures).
    pub emissive_textures: bool,
    /// Inventory and hotbar animations (Immersive UI idea); off = reduce motion.
    pub ui_animations: bool,
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
            difficulty: 2,
            relief: true,
            texture_resolution: crate::textures::BASE_RESOLUTION,
            sound_volume: 0.8,
            music_folder: "music".into(),
            music_delay_min: 600.0,
            music_delay_max: 1200.0,
            resource_packs: Vec::new(),
            bundled_pack_offered: false,
            saved_hotbars: Vec::new(),
            show_clouds: true,
            view_bobbing: true,
            flat_drops: true,
            mobs: true,
            gui_scale: 0,
            controller: ControllerSettings::default(),
            shader_pack: String::new(),
            shader_options: BTreeMap::new(),
            shader_quality: ShaderQuality::Fancy,
            lod_distance: 0,
            sparse_structures: false,
            spread_override: None,
            layered_clouds: false,
            shadows: 0,
            water_reflections: true,
            taa: false,
            emissive_textures: true,
            ui_animations: true,
        }
    }
}

impl Settings {
    /// Percent spacing for buildings.
    pub fn structure_spread(&self) -> i32 {
        if let Some(s) = self.spread_override {
            s
        } else if self.sparse_structures {
            170
        } else {
            100
        }
    }

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

    /// Name of the shader pack in use, for menus.
    pub fn shader_label(&self) -> &str {
        if self.shader_pack.is_empty() {
            "Desligado"
        } else {
            &self.shader_pack
        }
    }

    pub fn save(&self, path: &Path) {
        if let Ok(text) = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(path, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_files_still_load() {
        let s: Settings = ron::from_str("(render_distance: 6, fov: 80.0, mobs: false)").unwrap();
        assert_eq!(s.render_distance, 6);
        assert!(!s.mobs);
        assert!(s.shader_pack.is_empty());
        assert_eq!(s.shader_quality, ShaderQuality::Fancy);
    }
}

/// Names of the shadow settings, as shown in the menu.
pub const SHADOW_LEVELS: [&str; 3] = ["Desligadas", "Rápidas", "Bonitas"];

/// Names of the difficulty levels, as shown in the menu.
pub const DIFFICULTIES: [&str; 4] = ["Pacífico", "Fácil", "Normal", "Difícil"];
