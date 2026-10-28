//! Things for finding your way: a compass that points home (the bed, or the world's spawn),
//! a clock that tells the hour and the day, a spyglass that zooms in while the use button is
//! held, and a map that draws the land around you while it is in your hand.

use glam::Vec3;
use voxel::worldgen::{Biome, Dim};

use crate::Game;

/// Map cells across, and blocks per cell.
const MAP_CELLS: i32 = 48;
const MAP_STEP: i32 = 3;

#[derive(Default)]
pub struct MapCache {
    /// Colours of the cells, row by row, and where they were drawn around.
    cells: Vec<[u8; 4]>,
    center: (i32, i32),
    t: f32,
}

impl Game {
    /// The spyglass: true while it is held up.
    pub(crate) fn spyglass_up(&self) -> bool {
        (self.rdown && self.held_name().as_deref() == Some("spyglass")) || self.scope_up()
    }

    fn map_colour(&self, x: i32, z: i32) -> [u8; 4] {
        let s = self.world.gen.surface(x, z);
        if s.liquid.is_some() {
            let deep = (voxel::worldgen::SEA_LEVEL - s.height).clamp(0, 20) as u8;
            if self.reg.get(s.liquid.unwrap()).name == "ice" {
                return [170, 200, 230, 255];
            }
            return [40, 70u8.saturating_sub(deep), 160u8.saturating_sub(deep * 3), 255];
        }
        let top = self.reg.get(s.top).name.as_str();
        let mut c = match top {
            "sand" | "sandstone" => [214, 200, 140],
            "red_sand" | "terracotta" => [180, 100, 50],
            "snow" | "snowy_grass_block" => [235, 240, 245],
            "packed_ice" => [170, 200, 230],
            "stone" | "gravel" => [125, 125, 125],
            "mycelium" => [130, 110, 130],
            "clay" => [150, 155, 170],
            "mud" => [70, 62, 58],
            _ => match s.biome.base() {
                Biome::Swamp => [80, 100, 50],
                Biome::Savanna => [150, 160, 70],
                Biome::Jungle => [50, 140, 40],
                Biome::CherryGrove => [200, 160, 180],
                Biome::MangroveSwamp => [70, 95, 45],
                _ => [90, 150, 60],
            },
        };
        if s.trees.is_some_and(|(_, k)| k > 0.5) {
            c = [c[0] * 3 / 4, c[1] * 3 / 4, c[2] * 3 / 4];
        }
        // Shade by height so hills read.
        let k = (s.height - 64).clamp(-20, 60) as f32 / 100.0;
        let f = |v: u8| ((v as f32) * (1.0 + k)).clamp(0.0, 255.0) as u8;
        [f(c[0]), f(c[1]), f(c[2]), 255]
    }

    pub(crate) fn update_map(&mut self, dt: f32) {
        if self.held_name().as_deref() != Some("map") || self.dim != Dim::Overworld {
            return;
        }
        self.map.t -= dt;
        let me = (self.body.pos.x.floor() as i32, self.body.pos.z.floor() as i32);
        let moved = (me.0 - self.map.center.0).abs().max((me.1 - self.map.center.1).abs()) >= MAP_STEP * 2;
        if self.map.t > 0.0 && !moved && !self.map.cells.is_empty() {
            return;
        }
        self.map.t = 2.0;
        self.map.center = me;
        let half = MAP_CELLS / 2;
        let mut cells = Vec::with_capacity((MAP_CELLS * MAP_CELLS) as usize);
        for j in 0..MAP_CELLS {
            for i in 0..MAP_CELLS {
                cells.push(self.map_colour(me.0 + (i - half) * MAP_STEP, me.1 + (j - half) * MAP_STEP));
            }
        }
        self.map.cells = cells;
    }

    /// What the held compass, clock or map shows, drawn over the HUD.
    pub(crate) fn draw_explore_hud(&mut self, w: f32, _h: f32) {
        let Some(name) = self.held_name() else { return };
        let s = self.ui.scale;
        match name.as_str() {
            "compass" => {
                let home = self.home_spot();
                let to = home - self.body.pos;
                let f = self.forward();
                let rel = to.z.atan2(to.x) - f.z.atan2(f.x);
                let (cx, cy, r) = (w / 2.0, 30.0 * s, 12.0 * s);
                self.ui.rect(cx - r - 2.0 * s, cy - r - 2.0 * s, 2.0 * r + 4.0 * s, 2.0 * r + 4.0 * s, [30, 30, 34, 200]);
                // The needle: red toward home, grey behind.
                for k in 0..10 {
                    let t = k as f32 / 10.0 * r;
                    let (dx, dy) = (rel.sin() * t, -rel.cos() * t);
                    self.ui.rect(cx + dx - s, cy + dy - s, 2.0 * s, 2.0 * s, [220, 40, 40, 255]);
                    self.ui.rect(cx - dx * 0.6 - s, cy - dy * 0.6 - s, 2.0 * s, 2.0 * s, [170, 170, 175, 255]);
                }
                if self.dim != Dim::Overworld {
                    self.ui.text_centered(cx, cy + r + 4.0 * s, "A agulha gira sem rumo", [220, 220, 220, 255]);
                } else {
                    let d = Vec3::new(to.x, 0.0, to.z).length();
                    self.ui.text_centered(cx, cy + r + 4.0 * s, &format!("Casa a {d:.0} blocos"), [235, 235, 235, 255]);
                }
            }
            "clock" => {
                let t = self.time_of_day.rem_euclid(24000.0);
                // Tick 0 is six in the morning.
                let minutes = ((t / 1000.0 + 6.0) % 24.0 * 60.0) as u32;
                let part = match minutes / 60 {
                    5..=11 => "manhã",
                    12..=17 => "tarde",
                    18..=20 => "anoitecer",
                    _ => "noite",
                };
                let text = if self.dim == Dim::Overworld { format!("{:02}:{:02}, {part}", minutes / 60, minutes % 60) } else { "Os ponteiros giram sem parar".to_string() };
                self.ui.text_centered(w / 2.0, 22.0 * s, &text, [240, 230, 180, 255]);
            }
            "map" if self.dim == Dim::Overworld && !self.map.cells.is_empty() => {
                let px = 2.0 * s;
                let size = MAP_CELLS as f32 * px;
                let (x0, y0) = (w - size - 8.0 * s, 8.0 * s);
                self.ui.rect(x0 - 3.0 * s, y0 - 3.0 * s, size + 6.0 * s, size + 6.0 * s, [196, 176, 130, 255]);
                for j in 0..MAP_CELLS {
                    for i in 0..MAP_CELLS {
                        let c = self.map.cells[(j * MAP_CELLS + i) as usize];
                        self.ui.rect(x0 + i as f32 * px, y0 + j as f32 * px, px, px, c);
                    }
                }
                // You: a dot where you stand (north is up).
                let (mx, my) = (x0 + size / 2.0, y0 + size / 2.0);
                let f = self.forward();
                self.ui.rect(mx - 2.0 * s, my - 2.0 * s, 4.0 * s, 4.0 * s, [255, 255, 255, 255]);
                self.ui.rect(mx + f.x * 5.0 * s - s, my + f.z * 5.0 * s - s, 2.0 * s, 2.0 * s, [220, 40, 40, 255]);
            }
            _ => {}
        }
    }
}
