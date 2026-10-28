//! Graphics presets (Lote 13, speed on weak computers): one button in Opções > Vídeo sets every
//! costly option at once, from Mínimo (old laptops and computers without a graphics card) to
//! Ultra. The options themselves stay where they were; a preset just moves them together, and
//! the button reads "Personalizado" once one is changed by hand.
//!
//! On the very first start (no settings.ron yet) the game picks one for the computer it finds:
//! no graphics card (a software renderer) gets Mínimo, a graphics chip built into the processor
//! gets Leve, a real graphics card gets Equilibrado, and little memory caps it at Leve. If the
//! game still runs slowly for half a minute, it suggests a lighter preset once per session.

use crate::settings::ShaderQuality;
use crate::{Game, Screen};

/// What a preset sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    pub name: &'static str,
    render_distance: i32,
    fancy: bool,
    shadows: u8,
    parallax: u8,
    reflections: bool,
    emissive: bool,
    relief: bool,
    materials: u8,
    clouds: bool,
    layered_clouds: bool,
    ambient_life: u8,
    lod_distance: i32,
    lod_detail: u8,
    drop_glow: bool,
}

pub const PRESETS: [Preset; 5] = [
    Preset { name: "Mínimo", render_distance: 5, fancy: false, shadows: 0, parallax: 0, reflections: false, emissive: false, relief: false, materials: 0, clouds: false, layered_clouds: false, ambient_life: 0, lod_distance: 0, lod_detail: 0, drop_glow: false },
    Preset { name: "Leve", render_distance: 7, fancy: false, shadows: 0, parallax: 0, reflections: false, emissive: true, relief: false, materials: 1, clouds: true, layered_clouds: false, ambient_life: 1, lod_distance: 0, lod_detail: 0, drop_glow: true },
    Preset { name: "Equilibrado", render_distance: 10, fancy: true, shadows: 0, parallax: 1, reflections: true, emissive: true, relief: true, materials: 2, clouds: true, layered_clouds: false, ambient_life: 2, lod_distance: 128, lod_detail: 1, drop_glow: true },
    Preset { name: "Bonito", render_distance: 14, fancy: true, shadows: 1, parallax: 2, reflections: true, emissive: true, relief: true, materials: 2, clouds: true, layered_clouds: true, ambient_life: 3, lod_distance: 256, lod_detail: 1, drop_glow: true },
    Preset { name: "Ultra", render_distance: 20, fancy: true, shadows: 2, parallax: 3, reflections: true, emissive: true, relief: true, materials: 2, clouds: true, layered_clouds: true, ambient_life: 3, lod_distance: 512, lod_detail: 2, drop_glow: true },
];

/// Seconds of slow frames before the game suggests a lighter preset, and what counts as slow.
const SLOW_SECS: f32 = 30.0;
const SLOW_FPS: f32 = 24.0;

/// The preset a computer gets on the first start: from the kind of graphics adapter and how
/// much memory there is (GB).
pub fn for_machine(device: wgpu::DeviceType, ram_gb: f64) -> usize {
    let k = match device {
        wgpu::DeviceType::Cpu => 0,
        wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::VirtualGpu => 1,
        wgpu::DeviceType::DiscreteGpu => 2,
        wgpu::DeviceType::Other => 1,
    };
    if ram_gb > 0.0 && ram_gb < 6.0 {
        k.min(1)
    } else {
        k
    }
}

fn total_ram_gb() -> f64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.total_memory() as f64 / 1e9
}

#[derive(Default)]
pub struct PresetState {
    slow: f32,
    hinted: bool,
}

impl Game {
    /// The preset the options match now, if any.
    pub(crate) fn current_preset(&self) -> Option<usize> {
        let st = &self.settings;
        PRESETS.iter().position(|p| {
            st.render_distance == p.render_distance
                && (st.shader_quality == ShaderQuality::Fancy) == p.fancy
                && st.shadows == p.shadows
                && st.parallax == p.parallax
                && st.water_reflections == p.reflections
                && st.emissive_textures == p.emissive
                && st.relief == p.relief
                && st.materials == p.materials
                && st.show_clouds == p.clouds
                && st.layered_clouds == p.layered_clouds
                && st.ambient_life == p.ambient_life
                && st.lod_distance == p.lod_distance
                && st.lod_detail == p.lod_detail
                && st.drop_glow == p.drop_glow
        })
    }

    pub(crate) fn apply_preset(&mut self, k: usize) {
        let p = PRESETS[k.min(PRESETS.len() - 1)];
        let st = &mut self.settings;
        let quality = if p.fancy { ShaderQuality::Fancy } else { ShaderQuality::Fast };
        let recompile = st.shader_quality != quality;
        st.render_distance = p.render_distance;
        st.shader_quality = quality;
        st.shadows = p.shadows;
        st.parallax = p.parallax;
        st.water_reflections = p.reflections;
        st.emissive_textures = p.emissive;
        st.relief = p.relief;
        st.materials = p.materials;
        st.show_clouds = p.clouds;
        st.layered_clouds = p.layered_clouds;
        st.ambient_life = p.ambient_life;
        st.lod_distance = p.lod_distance;
        st.lod_detail = p.lod_detail;
        st.drop_glow = p.drop_glow;
        if recompile {
            self.apply_shader();
        }
        self.settings.save(&self.base.join("settings.ron"));
    }

    /// The options button: the next preset (from "Personalizado", the one for this computer).
    pub(crate) fn next_preset(&mut self) {
        let k = match self.current_preset() {
            Some(k) => (k + 1) % PRESETS.len(),
            None => self.machine_preset(),
        };
        self.apply_preset(k);
    }

    fn machine_preset(&self) -> usize {
        let device = self.renderer.get().map(|r| r.adapter_info.device_type).unwrap_or(wgpu::DeviceType::Other);
        for_machine(device, total_ram_gb())
    }

    /// The first start: a preset for this computer, said once on the title screen.
    pub(crate) fn first_run_preset(&mut self) {
        if self.renderer.get().is_none() {
            return;
        }
        let k = self.machine_preset();
        let name = self.renderer.get().map(|r| r.adapter_info.name.clone()).unwrap_or_default();
        log::info!("first start: graphics preset {} for {name}", PRESETS[k].name);
        self.apply_preset(k);
        self.toast(format!("Gráficos: {} (escolhido para este computador). Mude em Opções > Vídeo", PRESETS[k].name), 8.0);
    }

    /// Slow for half a minute in a row while playing: suggest a lighter preset, once.
    pub(crate) fn update_presets(&mut self, dt: f32) {
        if self.presets.hinted || self.screen != Screen::Playing || self.args.bench.is_some() || self.args.screenshot.is_some() {
            return;
        }
        if self.fps > 0.0 && self.fps < SLOW_FPS {
            self.presets.slow += dt;
        } else {
            self.presets.slow = (self.presets.slow - dt * 2.0).max(0.0);
        }
        if self.presets.slow < SLOW_SECS {
            return;
        }
        self.presets.hinted = true;
        let lighter = match self.current_preset() {
            Some(0) => return,
            Some(k) => k - 1,
            None => self.machine_preset().min(1),
        };
        self.toast(format!("O jogo está lento: experimente Gráficos: {} em Opções > Vídeo", PRESETS[lighter].name), 8.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_climb_in_cost() {
        for w in PRESETS.windows(2) {
            assert!(w[1].render_distance > w[0].render_distance, "{}", w[1].name);
            assert!(w[1].lod_distance >= w[0].lod_distance);
            assert!(w[1].shadows >= w[0].shadows && w[1].parallax >= w[0].parallax);
        }
    }

    #[test]
    fn machines_get_a_fitting_preset() {
        assert_eq!(for_machine(wgpu::DeviceType::Cpu, 16.0), 0);
        assert_eq!(for_machine(wgpu::DeviceType::IntegratedGpu, 16.0), 1);
        assert_eq!(for_machine(wgpu::DeviceType::DiscreteGpu, 16.0), 2);
        assert_eq!(for_machine(wgpu::DeviceType::DiscreteGpu, 4.0), 1);
    }
}
