//! Player settings, stored in `settings.ron` next to the game.

use crate::audio::SoundCat;
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
    /// The master volume: music, voices and every sound group below follow it.
    pub sound_volume: f32,
    /// Volume of each group of sounds (Áudio options).
    pub volumes: Volumes,
    /// 0 dark (moody) .. 0.5 normal .. 1 bright: how much the shadows and caves are lifted.
    pub brightness: f32,
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
    /// Sun and moon shadows (Sildur's idea): 0 off, 1 fast (small map, near), 2 fancy. Mobs and
    /// the player cast them too, and so do blocks in the light of a torch held in the hand.
    pub shadows: u8,
    /// Rippling water that mirrors the world around it (it reads the drawn scene).
    pub water_reflections: bool,
    /// Temporal anti-aliasing: smooth edges from jittered frames (needs a shader pack).
    pub taa: bool,
    /// Glowing bits of ore textures stay bright in the dark (Continuity's emissive textures).
    pub emissive_textures: bool,
    /// Block depth (parallax occlusion mapping, as in shader packs): 0 off, then the steps of
    /// `PARALLAX_LEVELS`. Needs the "Bonito" quality.
    pub parallax: u8,
    /// Inventory and hotbar animations (Immersive UI idea); off = reduce motion.
    pub ui_animations: bool,
    /// "pt" (Portuguese, the game's own) or "en" (English, through lang.rs).
    pub language: String,
    /// Captions for the sounds around: what made it and from which side.
    pub subtitles: bool,
    /// Calm camera and interface: no view bobbing, shaking, sprint zoom or bouncing icons.
    pub reduce_motion: bool,
    /// Keyboard keys moved from their defaults: action → key (`keybinds.rs`).
    pub key_binds: BTreeMap<String, String>,
    /// Voice chat (voice.rs).
    pub voice: crate::voice::VoiceSettings,
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
            volumes: Volumes::default(),
            brightness: 0.5,
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
            parallax: 2,
            ui_animations: true,
            language: "pt".into(),
            subtitles: false,
            reduce_motion: false,
            key_binds: BTreeMap::new(),
            voice: Default::default(),
        }
    }
}

/// Volume of each group of sounds, 0..1.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Volumes {
    pub records: f32,
    pub weather: f32,
    pub blocks: f32,
    pub creatures: f32,
    pub players: f32,
    pub ambient: f32,
    pub interface: f32,
}

impl Default for Volumes {
    fn default() -> Self {
        Volumes { records: 1.0, weather: 1.0, blocks: 1.0, creatures: 1.0, players: 1.0, ambient: 1.0, interface: 1.0 }
    }
}

impl Volumes {
    pub fn of(&self, c: SoundCat) -> f32 {
        *self.clone().of_mut(c)
    }

    pub fn of_mut(&mut self, c: SoundCat) -> &mut f32 {
        let v = self;
        match c {
            SoundCat::Records => &mut v.records,
            SoundCat::Weather => &mut v.weather,
            SoundCat::Blocks => &mut v.blocks,
            SoundCat::Creatures => &mut v.creatures,
            SoundCat::Players => &mut v.players,
            SoundCat::Ambient => &mut v.ambient,
            SoundCat::Interface => &mut v.interface,
        }
    }
}

/// The light lift the shaders get from the brightness setting (0.3 is the classic look).
pub fn brightness_lift(b: f32) -> f32 {
    let b = b.clamp(0.0, 1.0);
    if b < 0.5 {
        b * 0.6
    } else {
        0.3 + (b - 0.5) * 1.4
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
/// Block depth choices: menu name and how deep the darkest texels sink, in blocks.
pub const PARALLAX_LEVELS: [(&str, f32); 4] = [("Não", 0.0), ("Rasa", 0.06), ("Média", 0.1), ("Funda", 0.16)];

pub const SHADOW_LEVELS: [&str; 3] = ["Desligadas", "Rápidas", "Bonitas"];

/// Names of the difficulty levels, as shown in the menu.
pub const DIFFICULTIES: [&str; 4] = ["Pacífico", "Fácil", "Normal", "Difícil"];
