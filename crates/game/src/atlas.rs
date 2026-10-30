//! The atlas (the Aged modpack's Antique Atlas idea, HP's request of 2026-10-16): a book of maps
//! that remembers every place it has been carried through. While it is anywhere in the
//! inventory, the land around the player is inked into it a little at a time, and it stays
//! there: the atlas is the player's own record of the world, not a live view like the map.
//!
//! Right click with it in hand opens it over the screen, drawn like a hand-inked chart: parchment,
//! muted watercolour lands, coastlines outlined in ink, little trees for forests, peaks for
//! mountains, and ragged edges where the explored part ends. Home, the last death and the town
//! are marked on their own; N puts a numbered mark where you stand and Delete rubs out the one
//! nearest you. Arrows pan, + and - zoom, Space brings it back to you, Esc or right click closes.
//!
//! Saved per world: the cells in `atlas.bin` (16 x 16 blocks to a tile, 4 x 4 blocks to a cell,
//! one byte each), the marks in `atlas.ron`.

use std::collections::HashMap;
use std::path::PathBuf;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use voxel::worldgen::{Biome, Dim};
use winit::keyboard::KeyCode;

use crate::audio::{Material, Sfx};
use crate::{Game, Screen};

pub const ITEM: &str = "atlas";
/// Blocks per cell, and cells across a tile.
const CELL: i32 = 4;
const TILE: i32 = 4;
/// How far around the player the atlas inks the land (blocks).
const REACH: i32 = 56;
/// Cells looked up per pass, and how often a pass runs.
const PER_PASS: usize = 72;
const PASS_SECS: f32 = 0.2;
const MAX_MARKS: usize = 32;

/// What a cell shows. 0 is still blank parchment; the high bit marks high ground.
mod kind {
    pub const DEEP: u8 = 1;
    pub const WATER: u8 = 2;
    pub const SAND: u8 = 3;
    pub const GRASS: u8 = 4;
    pub const FOREST: u8 = 5;
    pub const JUNGLE: u8 = 6;
    pub const SWAMP: u8 = 7;
    pub const SNOW: u8 = 8;
    pub const STONE: u8 = 9;
    pub const SAVANNA: u8 = 10;
    pub const BADLANDS: u8 = 11;
    pub const ICE: u8 = 12;
    pub const CHERRY: u8 = 13;
    pub const MUSHROOM: u8 = 14;
    pub const TAIGA: u8 = 15;
    pub const HIGH: u8 = 0x80;
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Marks {
    /// (x, z, label) of the player's own marks.
    marks: Vec<(i32, i32, String)>,
}

#[derive(Default)]
pub struct Atlas {
    world: String,
    tiles: HashMap<(i32, i32), [u8; 16]>,
    marks: Marks,
    dirty: bool,
    t: f32,
    pub open: bool,
    /// Centre of the open page (blocks) and pixels per cell.
    view: (f32, f32),
    zoom: f32,
}

fn tile_of(cx: i32, cz: i32) -> ((i32, i32), usize) {
    let (tx, tz) = (cx.div_euclid(TILE), cz.div_euclid(TILE));
    let i = (cz.rem_euclid(TILE) * TILE + cx.rem_euclid(TILE)) as usize;
    ((tx, tz), i)
}

fn hash2(x: i32, z: i32) -> f32 {
    let h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (z as u32).wrapping_mul(0x85EB_CA77);
    let h = (h ^ (h >> 15)).wrapping_mul(0x2C1B_3C6D);
    ((h ^ (h >> 13)) & 0xFFFF) as f32 / 65535.0
}

/// Watercolour wash of a kind (high ground a little paler and greyer).
fn wash(k: u8) -> [u8; 3] {
    let c = match k & 0x7f {
        kind::DEEP => [104, 132, 150],
        kind::WATER => [138, 166, 176],
        kind::SAND => [228, 208, 156],
        kind::GRASS => [176, 186, 120],
        kind::FOREST => [140, 160, 98],
        kind::JUNGLE => [112, 146, 84],
        kind::SWAMP => [134, 142, 104],
        kind::SNOW => [238, 236, 226],
        kind::STONE => [172, 164, 146],
        kind::SAVANNA => [196, 186, 116],
        kind::BADLANDS => [208, 150, 104],
        kind::ICE => [206, 222, 226],
        kind::CHERRY => [214, 180, 182],
        kind::MUSHROOM => [170, 150, 162],
        kind::TAIGA => [130, 150, 110],
        _ => [222, 205, 160],
    };
    if k & kind::HIGH != 0 {
        [((c[0] as u32 + 172) / 2) as u8, ((c[1] as u32 + 166) / 2) as u8, ((c[2] as u32 + 150) / 2) as u8]
    } else {
        c
    }
}

fn wet(k: u8) -> bool {
    matches!(k & 0x7f, kind::DEEP | kind::WATER | kind::ICE)
}

const PAPER: [u8; 4] = [222, 205, 160, 255];
const INK: [u8; 4] = [74, 58, 42, 255];
const RED_INK: [u8; 4] = [150, 40, 30, 255];

impl Game {
    fn atlas_dir(&self) -> PathBuf {
        crate::level_path_hint(&self.base, &self.world_name)
    }

    fn atlas_sync(&mut self) {
        if self.atlas.world == self.world_name {
            return;
        }
        let dir = self.atlas_dir();
        let mut tiles = HashMap::new();
        if let Ok(b) = std::fs::read(dir.join("atlas.bin")) {
            if b.len() >= 4 && &b[..4] == b"ATL1" {
                for e in b[4..].chunks_exact(24) {
                    let x = i32::from_le_bytes([e[0], e[1], e[2], e[3]]);
                    let z = i32::from_le_bytes([e[4], e[5], e[6], e[7]]);
                    let mut t = [0u8; 16];
                    t.copy_from_slice(&e[8..24]);
                    tiles.insert((x, z), t);
                }
            }
        }
        let marks = std::fs::read_to_string(dir.join("atlas.ron")).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default();
        self.atlas = Atlas { world: self.world_name.clone(), tiles, marks, zoom: 4.0, ..Default::default() };
    }

    pub(crate) fn atlas_save(&mut self) {
        if self.atlas.world != self.world_name || self.world_name.is_empty() || !self.atlas.dirty {
            return;
        }
        let dir = self.atlas_dir();
        let mut b = Vec::with_capacity(4 + self.atlas.tiles.len() * 24);
        b.extend_from_slice(b"ATL1");
        for (&(x, z), t) in &self.atlas.tiles {
            b.extend_from_slice(&x.to_le_bytes());
            b.extend_from_slice(&z.to_le_bytes());
            b.extend_from_slice(t);
        }
        let _ = std::fs::create_dir_all(&dir);
        if let Err(e) = voxel::save::write_atomic(dir.join("atlas.bin"), b) {
            log::error!("could not save atlas.bin: {e}");
        }
        if let Ok(t) = ron::ser::to_string_pretty(&self.atlas.marks, Default::default()) {
            let _ = voxel::save::write_atomic(dir.join("atlas.ron"), t);
        }
        self.atlas.dirty = false;
    }

    fn atlas_cell(&self, cx: i32, cz: i32) -> u8 {
        let (t, i) = tile_of(cx, cz);
        self.atlas.tiles.get(&t).map_or(0, |t| t[i])
    }

    fn atlas_kind(&self, x: i32, z: i32) -> u8 {
        let s = self.world.gen.surface(x, z);
        let high = if s.height > 100 { kind::HIGH } else { 0 };
        if let Some(l) = s.liquid {
            if self.reg.get(l).name == "ice" {
                return kind::ICE;
            }
            return if voxel::worldgen::SEA_LEVEL - s.height > 8 { kind::DEEP } else { kind::WATER };
        }
        let top = self.reg.get(s.top).name.as_str();
        let k = match top {
            "sand" | "sandstone" => kind::SAND,
            "red_sand" | "terracotta" => kind::BADLANDS,
            "snow" | "snowy_grass_block" | "powder_snow" => kind::SNOW,
            "packed_ice" | "blue_ice" | "ice" => kind::ICE,
            "stone" | "gravel" | "andesite" | "diorite" | "granite" | "calcite" | "tuff" => kind::STONE,
            "mycelium" => kind::MUSHROOM,
            _ => {
                let forest = s.trees.is_some_and(|(_, k)| k > 0.35);
                match s.biome.base() {
                    Biome::Swamp | Biome::MangroveSwamp => kind::SWAMP,
                    Biome::Jungle => kind::JUNGLE,
                    Biome::Savanna => kind::SAVANNA,
                    Biome::CherryGrove => kind::CHERRY,
                    Biome::Taiga => kind::TAIGA,
                    Biome::Badlands => kind::BADLANDS,
                    Biome::Desert => kind::SAND,
                    Biome::Mountains | Biome::SnowyPeaks if s.height > 110 => kind::STONE,
                    _ if forest => kind::FOREST,
                    _ => kind::GRASS,
                }
            }
        };
        k | high
    }

    pub(crate) fn atlas_carried(&self) -> bool {
        let Some(id) = self.items.id(ITEM) else { return false };
        self.surv.inv.slots.iter().flatten().any(|s| s.item == id) || self.surv.offhand.is_some_and(|s| s.item == id)
    }

    pub(crate) fn update_atlas(&mut self, dt: f32) {
        if !self.world_open || self.screen == Screen::Loading || self.world_name.is_empty() {
            return;
        }
        self.atlas_sync();
        if self.atlas.open && (self.screen != Screen::Playing || !self.atlas_carried()) {
            self.atlas.open = false;
        }
        self.atlas.t -= dt;
        if self.atlas.t > 0.0 || self.dim != Dim::Overworld || !self.atlas_carried() {
            return;
        }
        self.atlas.t = PASS_SECS;
        // Ink the blank cells nearest the player first, a handful per pass.
        let (px, pz) = ((self.body.pos.x.floor() as i32).div_euclid(CELL), (self.body.pos.z.floor() as i32).div_euclid(CELL));
        let r = REACH / CELL;
        let mut todo = Vec::new();
        'rings: for ring in 0..=r {
            for dz in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring || dx * dx + dz * dz > r * r {
                        continue;
                    }
                    let (cx, cz) = (px + dx, pz + dz);
                    if self.atlas_cell(cx, cz) == 0 {
                        todo.push((cx, cz));
                        if todo.len() >= PER_PASS {
                            break 'rings;
                        }
                    }
                }
            }
        }
        for (cx, cz) in todo {
            let k = self.atlas_kind(cx * CELL + CELL / 2, cz * CELL + CELL / 2);
            let (t, i) = tile_of(cx, cz);
            self.atlas.tiles.entry(t).or_insert([0; 16])[i] = k.max(1);
            self.atlas.dirty = true;
        }
    }

    /// Right click with the atlas: open it (or close it again).
    pub(crate) fn atlas_use(&mut self) -> bool {
        if self.held_name().as_deref() != Some(ITEM) {
            return false;
        }
        self.atlas_sync();
        self.atlas.open = !self.atlas.open;
        if self.atlas.open {
            self.atlas.view = (self.body.pos.x, self.body.pos.z);
            self.sfx(Sfx::Place(Material::Cloth));
            if self.dim != Dim::Overworld {
                self.toast("As páginas do atlas ficam em branco fora do Mundo Superior", 2.5);
            }
        }
        true
    }

    pub(crate) fn atlas_key(&mut self, code: KeyCode) -> bool {
        if !self.atlas.open || self.screen != Screen::Playing {
            return false;
        }
        let step = 24.0 * CELL as f32 / self.atlas.zoom;
        match code {
            KeyCode::Escape => self.atlas.open = false,
            KeyCode::ArrowLeft => self.atlas.view.0 -= step,
            KeyCode::ArrowRight => self.atlas.view.0 += step,
            KeyCode::ArrowUp => self.atlas.view.1 -= step,
            KeyCode::ArrowDown => self.atlas.view.1 += step,
            KeyCode::Equal | KeyCode::NumpadAdd => self.atlas.zoom = (self.atlas.zoom + 1.0).min(8.0),
            KeyCode::Minus | KeyCode::NumpadSubtract => self.atlas.zoom = (self.atlas.zoom - 1.0).max(1.0),
            KeyCode::Space => self.atlas.view = (self.body.pos.x, self.body.pos.z),
            KeyCode::KeyN => {
                if self.dim != Dim::Overworld {
                    return true;
                }
                if self.atlas.marks.marks.len() >= MAX_MARKS {
                    self.toast("O atlas já tem marcas demais: apague uma com Delete", 2.0);
                    return true;
                }
                let n = (1..).find(|n| !self.atlas.marks.marks.iter().any(|m| m.2 == n.to_string())).unwrap_or(1);
                self.atlas.marks.marks.push((self.body.pos.x.floor() as i32, self.body.pos.z.floor() as i32, n.to_string()));
                self.atlas.dirty = true;
                self.toast(format!("Marca {n} desenhada no atlas"), 1.5);
                self.sfx(Sfx::Place(Material::Cloth));
            }
            KeyCode::Delete | KeyCode::Backspace => {
                let me = Vec3::new(self.body.pos.x, 0.0, self.body.pos.z);
                let near = self.atlas.marks.marks.iter().enumerate().map(|(i, m)| (i, Vec3::new(m.0 as f32, 0.0, m.1 as f32).distance(me))).filter(|m| m.1 < 12.0).min_by(|a, b| a.1.total_cmp(&b.1));
                match near {
                    Some((i, _)) => {
                        let m = self.atlas.marks.marks.remove(i);
                        self.atlas.dirty = true;
                        self.toast(format!("Marca {} apagada", m.2), 1.5);
                    }
                    None => self.toast("Nenhuma marca perto de você para apagar", 1.5),
                }
            }
            // Movement and the rest go on while the atlas is open.
            _ => return false,
        }
        true
    }

    /// The open atlas, over everything else on the HUD.
    pub(crate) fn draw_atlas(&mut self, w: f32, h: f32) {
        if !self.atlas.open || self.atlas.world != self.world_name {
            return;
        }
        let s = self.ui.scale;
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 110]);
        let pw = (w - 24.0 * s).min(520.0 * s);
        let ph = (h - 40.0 * s).min(320.0 * s);
        let (px, py) = ((w - pw) / 2.0, (h - ph) / 2.0 - 6.0 * s);
        // Parchment, darker toward the edges, with a few age spots.
        self.ui.rect(px - 3.0 * s, py - 3.0 * s, pw + 6.0 * s, ph + 6.0 * s, [120, 84, 52, 255]);
        self.ui.rect(px, py, pw, ph, PAPER);
        for k in 0..6 {
            let e = k as f32 * 2.0 * s;
            let a = 26 - k * 4;
            self.ui.frame(px + e, py + e, pw - 2.0 * e, ph - 2.0 * e, 2.0 * s, [120, 90, 50, a as u8]);
        }
        for k in 0..40 {
            let (fx, fz) = (hash2(k, 7), hash2(3, k));
            let r = (1.0 + hash2(k, k) * 3.0) * s;
            self.ui.rect(px + fx * (pw - r), py + fz * (ph - r), r, r, [190, 165, 115, 60]);
        }
        let mx0 = px + 8.0 * s;
        let my0 = py + 18.0 * s;
        let mw = pw - 16.0 * s;
        let mh = ph - 34.0 * s;
        let z = (self.atlas.zoom * s * 0.5).max(1.0).round();
        let cols = (mw / z) as i32;
        let rows = (mh / z) as i32;
        let (vx, vz) = self.atlas.view;
        let c0x = (vx / CELL as f32).floor() as i32 - cols / 2;
        let c0z = (vz / CELL as f32).floor() as i32 - rows / 2;
        if self.dim == Dim::Overworld {
            for j in 0..rows {
                for i in 0..cols {
                    let (cx, cz) = (c0x + i, c0z + j);
                    let k = self.atlas_cell(cx, cz);
                    if k == 0 {
                        continue;
                    }
                    // The border of what has been seen is ragged, like a wash that ran out.
                    let open = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().filter(|(dx, dz)| self.atlas_cell(cx + dx, cz + dz) == 0).count();
                    if open > 0 && hash2(cx, cz) < 0.35 {
                        continue;
                    }
                    let mut c = wash(k);
                    // Grain of the brush.
                    let g = (hash2(cx * 3, cz * 5) * 14.0) as i32 - 7;
                    for v in c.iter_mut() {
                        *v = (*v as i32 + g).clamp(0, 255) as u8;
                    }
                    let a = if open > 0 { 150 } else { 230 };
                    let (x, y) = (mx0 + i as f32 * z, my0 + j as f32 * z);
                    self.ui.rect(x, y, z, z, [c[0], c[1], c[2], a]);
                    // Coastlines in ink.
                    if !wet(k) {
                        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                            let n = self.atlas_cell(cx + dx, cz + dz);
                            if n != 0 && wet(n) {
                                let t = (z * 0.25).max(1.0);
                                let (lx, ly, lw, lh) = match (dx, dz) {
                                    (1, _) => (x + z - t, y, t, z),
                                    (-1, _) => (x, y, t, z),
                                    (_, 1) => (x, y + z - t, z, t),
                                    _ => (x, y, z, t),
                                };
                                self.ui.rect(lx, ly, lw, lh, [70, 86, 96, 220]);
                            }
                        }
                    }
                }
            }
            // Little drawings over the wash: trees, peaks, waves, reeds.
            if z >= 3.0 {
                let every = if z >= 5.0 { 2 } else { 3 };
                for j in (0..rows).step_by(every) {
                    for i in (0..cols).step_by(every) {
                        let (cx, cz) = (c0x + i, c0z + j);
                        let k = self.atlas_cell(cx, cz);
                        if k == 0 || hash2(cx * 7, cz * 11) < 0.45 {
                            continue;
                        }
                        let (x, y) = (mx0 + i as f32 * z, my0 + j as f32 * z);
                        let u = (z * 0.5).max(1.0);
                        if k & kind::HIGH != 0 || k & 0x7f == kind::STONE {
                            // A peak: two strokes meeting at the top.
                            for d in 0..3 {
                                let d = d as f32 * u;
                                self.ui.rect(x + d, y + 2.0 * u - d, u, u, INK);
                                self.ui.rect(x + 4.0 * u - d, y + 2.0 * u - d, u, u, INK);
                            }
                            continue;
                        }
                        match k & 0x7f {
                            kind::FOREST | kind::JUNGLE | kind::TAIGA | kind::CHERRY => {
                                let crown = match k & 0x7f {
                                    kind::JUNGLE => [52, 92, 44, 255],
                                    kind::TAIGA => [46, 74, 52, 255],
                                    kind::CHERRY => [176, 104, 120, 255],
                                    _ => [64, 98, 50, 255],
                                };
                                self.ui.rect(x + u, y + 2.0 * u, u, u, INK);
                                self.ui.rect(x, y, 3.0 * u, 2.0 * u, crown);
                            }
                            kind::WATER | kind::DEEP => {
                                self.ui.rect(x, y + u, u, u, [88, 112, 128, 200]);
                                self.ui.rect(x + u, y, u, u, [88, 112, 128, 200]);
                                self.ui.rect(x + 2.0 * u, y + u, u, u, [88, 112, 128, 200]);
                            }
                            kind::SWAMP => {
                                self.ui.rect(x, y, u, 2.0 * u, [70, 84, 52, 255]);
                                self.ui.rect(x + 2.0 * u, y + u, u, u, [70, 84, 52, 255]);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        // Where a world point lands on the page, if it is on it.
        let to_page = |x: f32, zz: f32| -> Option<(f32, f32)> {
            let sx = mx0 + (x / CELL as f32 - c0x as f32) * z;
            let sy = my0 + (zz / CELL as f32 - c0z as f32) * z;
            (sx >= mx0 && sy >= my0 && sx < mx0 + mw && sy < my0 + mh).then_some((sx, sy))
        };
        if self.dim == Dim::Overworld {
            // Home: a little house in red ink.
            let home = self.home_spot();
            if let Some((x, y)) = to_page(home.x, home.z) {
                self.ui.rect(x - 3.0 * s, y - 1.0 * s, 6.0 * s, 4.0 * s, RED_INK);
                self.ui.rect(x - 2.0 * s, y - 3.0 * s, 4.0 * s, 2.0 * s, RED_INK);
                self.ui.rect(x - 1.0 * s, y - 4.0 * s, 2.0 * s, 1.0 * s, RED_INK);
                self.ui.text(x + 5.0 * s, y - 4.0 * s, "Casa", INK);
            }
            // The town.
            if let Some(&(_, at)) = self.town.houses.first() {
                if let Some((x, y)) = to_page(at.x, at.z) {
                    for d in [-3.0f32, 1.0] {
                        self.ui.rect(x + d * s, y - 2.0 * s, 2.0 * s, 4.0 * s, INK);
                    }
                    self.ui.text(x + 5.0 * s, y - 4.0 * s, "Vila", INK);
                }
            }
            // The last death: a cross.
            if let Some((p, d)) = self.prog.death {
                if d == 0 {
                    if let Some((x, y)) = to_page(p.x, p.z) {
                        for k in -2..=2 {
                            let k = k as f32 * s;
                            self.ui.rect(x + k - 0.5 * s, y + k - 0.5 * s, s, s, RED_INK);
                            self.ui.rect(x + k - 0.5 * s, y - k - 0.5 * s, s, s, RED_INK);
                        }
                    }
                }
            }
            for (mx, mz, label) in self.atlas.marks.marks.clone() {
                if let Some((x, y)) = to_page(mx as f32 + 0.5, mz as f32 + 0.5) {
                    self.ui.rect(x - 2.0 * s, y - 2.0 * s, 4.0 * s, 4.0 * s, RED_INK);
                    self.ui.text(x + 4.0 * s, y - 4.0 * s, &label, RED_INK);
                }
            }
            // You: an arrow along where you look.
            if let Some((x, y)) = to_page(self.body.pos.x, self.body.pos.z) {
                let f = self.forward();
                let d = glam::Vec2::new(f.x, f.z).normalize_or_zero();
                for k in 0..5 {
                    let t = (k as f32 - 1.5) * 1.5 * s;
                    self.ui.rect(x + d.x * t - s, y + d.y * t - s, 2.0 * s, 2.0 * s, [30, 20, 12, 255]);
                }
                let tip = (x + d.x * 5.0 * s, y + d.y * 5.0 * s);
                let side = glam::Vec2::new(-d.y, d.x);
                for k in 1..3 {
                    let k = k as f32 * 1.5 * s;
                    for sg in [-1.0f32, 1.0] {
                        self.ui.rect(tip.0 - d.x * k + side.x * k * sg - s, tip.1 - d.y * k + side.y * k * sg - s, 2.0 * s, 2.0 * s, [30, 20, 12, 255]);
                    }
                }
            }
        }
        // Compass rose, title, scale and keys.
        let (rx, ry) = (px + pw - 22.0 * s, py + 26.0 * s);
        self.ui.rect(rx - 0.5 * s, ry - 8.0 * s, s, 16.0 * s, INK);
        self.ui.rect(rx - 8.0 * s, ry - 0.5 * s, 16.0 * s, s, INK);
        self.ui.rect(rx - 1.5 * s, ry - 8.0 * s, 3.0 * s, 4.0 * s, RED_INK);
        self.ui.text_centered(rx, ry - 17.0 * s, "N", INK);
        self.ui.text_centered(w / 2.0, py + 5.0 * s, "Atlas", INK);
        let blocks = (40.0 * s / z * CELL as f32).round();
        self.ui.rect(px + 10.0 * s, py + ph - 14.0 * s, 40.0 * s, s, INK);
        self.ui.text(px + 54.0 * s, py + ph - 18.0 * s, &format!("{blocks} blocos"), INK);
        let help = "Setas mover  +/- zoom  N marca  Del apaga  Esc fecha";
        let hw = self.ui.text_width(help, s);
        self.ui.rect(w / 2.0 - hw / 2.0 - 3.0 * s, py + ph + 4.0 * s, hw + 6.0 * s, 10.0 * s, [0, 0, 0, 150]);
        self.ui.text_centered(w / 2.0, py + ph + 5.0 * s, help, [235, 225, 200, 255]);
    }

    /// Test hook (MC_ATLAS=1): an atlas in hand, the land around inked, the page open.
    pub(crate) fn test_atlas(&mut self) {
        let Some(id) = self.items.id(ITEM) else { return };
        self.surv.inv.slots[self.selected] = Some(voxel::item::ItemStack::new(id, 1));
        self.atlas_sync();
        // As if the player had wandered a while around the spawn.
        let p = self.body.pos;
        let (px, pz) = ((p.x as i32).div_euclid(CELL), (p.z as i32).div_euclid(CELL));
        for cz in pz - 40..pz + 30 {
            for cx in px - 70..px + 60 {
                let (dx, dz) = ((cx - px) as f32, (cz - pz) as f32);
                if (dx * 0.03).sin() * 25.0 + 35.0 < dz.abs() + hash2(cx, cz) * 4.0 {
                    continue;
                }
                let k = self.atlas_kind(cx * CELL + CELL / 2, cz * CELL + CELL / 2);
                let (t, i) = tile_of(cx, cz);
                self.atlas.tiles.entry(t).or_insert([0; 16])[i] = k.max(1);
            }
        }
        self.atlas.marks.marks = vec![(p.x as i32 + 30, p.z as i32 - 20, "1".into())];
        self.atlas.open = true;
        self.atlas.view = (p.x, p.z);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_map_into_tiles_both_sides_of_zero() {
        assert_eq!(tile_of(0, 0), ((0, 0), 0));
        assert_eq!(tile_of(3, 3), ((0, 0), 15));
        assert_eq!(tile_of(-1, -1), ((-1, -1), 15));
        assert_eq!(tile_of(-4, 5), ((-1, 1), 4));
    }

    #[test]
    fn high_ground_keeps_its_kind() {
        assert!(wet(kind::DEEP) && wet(kind::WATER) && !wet(kind::GRASS | kind::HIGH));
        assert_ne!(wash(kind::GRASS), wash(kind::GRASS | kind::HIGH));
    }
}
