//! Items on the ground: their name (and how many) floating above them when close, and a soft
//! glow around them, coloured and stronger when enchanted or rare. Both can be turned off in
//! the options.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;

use crate::item_render::GpuItem;
use crate::survival::Dropped;
use crate::Game;

/// Names show up to this far, fading over the last blocks.
const LABEL_FAR: f32 = 7.0;
/// The item under the crosshair shows its name from further away.
const LOOK_FAR: f32 = 16.0;
const MAX_LABELS: usize = 10;
/// Glows past this are too small to matter.
const GLOW_FAR: f32 = 40.0;
const MAX_GLOWS: usize = 64;
/// The shimmer over an enchanted item (rgb, strength), and its glow's colour.
pub(crate) const ENCH_GLINT: [f32; 4] = [0.62, 0.4, 1.0, 0.7];

impl Game {
    /// Whether a drop lies flat on the ground, and how high over its body it floats.
    pub(crate) fn drop_lift(&self, d: &Dropped) -> (bool, f32) {
        let lying = self.settings.flat_drops && (d.body.on_ground || d.body.vel.length_squared() < 0.01) && !d.body.in_water;
        (lying, if lying { 0.0 } else { (d.age * 2.5).sin() * 0.06 + 0.18 })
    }

    /// The colour of an item's name: loot rarity, then the classic rarity colours, aqua when
    /// enchanted, green with a modifier, white otherwise.
    pub(crate) fn drop_name_color(&self, st: &ItemStack) -> [u8; 4] {
        if let Some(r) = voxel::gems::rarity(st.sockets) {
            return voxel::gems::RARITY_COLORS[r];
        }
        if let Some(c) = crate::juice::rarity(&self.items.get(st.item).name) {
            return [c[0], c[1], c[2], 255];
        }
        if crate::enchanting::is_enchanted(st) {
            return [85, 255, 255, 255];
        }
        if crate::affixes::of(st).is_some() {
            return [140, 235, 140, 255];
        }
        [255, 255, 255, 255]
    }

    /// The glow of an item: colour, strength and radius. Every item has a faint warm one;
    /// enchanted and rare ones glow more, in their colour.
    fn drop_glow(&self, st: &ItemStack) -> ([f32; 3], f32, f32) {
        let f = |c: [u8; 4]| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0];
        if crate::enchanting::is_enchanted(st) {
            let g = ENCH_GLINT;
            return ([g[0], g[1], g[2]], 0.3, 0.6);
        }
        if let Some(r) = voxel::gems::rarity(st.sockets).filter(|&r| r > 0) {
            return (f(voxel::gems::RARITY_COLORS[r]), 0.16 + r as f32 * 0.03, 0.55);
        }
        if let Some(c) = crate::juice::rarity(&self.items.get(st.item).name) {
            return (f([c[0], c[1], c[2], 255]), 0.22, 0.55);
        }
        if crate::affixes::of(st).is_some() {
            return ([0.55, 0.92, 0.6], 0.16, 0.5);
        }
        ([1.0, 0.9, 0.72], 0.07, 0.42)
    }

    /// Glows for the item renderer, nearest first.
    pub(crate) fn drop_halos(&self, cam_block: IVec3) -> Vec<GpuItem> {
        if !self.settings.drop_glow {
            return Vec::new();
        }
        let c = cam_block.as_vec3();
        let eye = self.body.eye();
        let mut found: Vec<(f32, GpuItem)> = Vec::new();
        for d in &self.surv.drops {
            let dist = d.body.pos.distance(eye);
            if dist > GLOW_FAR {
                continue;
            }
            let (col, k, mut r) = self.drop_glow(&d.stack);
            if d.stack.count > 1 {
                r *= 1.15;
            }
            // A slow breath, each item out of step with the others.
            let k = k * (1.0 + 0.12 * (d.age * 2.0 + d.spin_rest * 3.0).sin());
            let (_, lift) = self.drop_lift(d);
            let at = d.body.pos + Vec3::Y * (lift + 0.15);
            let sky = (self.world.light((d.body.pos + Vec3::Y * 0.2).floor().as_ivec3()) >> 4) as f32 / 15.0;
            let p = at - c;
            found.push((dist, GpuItem { pos: [p.x, p.y, p.z, r], info: [0.0, 0.0, sky, 0.0], layers: [col[0], col[1], col[2], k], ..bytemuck::Zeroable::zeroed() }));
        }
        if found.len() > MAX_GLOWS {
            found.sort_by(|a, b| a.0.total_cmp(&b.0));
            found.truncate(MAX_GLOWS);
        }
        found.into_iter().map(|f| f.1).collect()
    }

    /// Names over the items lying nearby, and over the one under the crosshair from further.
    pub(crate) fn draw_drop_labels(&mut self, w: f32, h: f32) {
        if !self.settings.drop_labels || self.surv.drops.is_empty() {
            return;
        }
        let Some((vp, cam_block)) = self.ui_view_proj else { return };
        let eye = self.body.eye();
        let fwd = self.forward();
        // The item the crosshair is on: the closest to the line of sight, within half a block.
        let mut looked: Option<(usize, f32)> = None;
        let mut near: Vec<(f32, usize)> = Vec::new();
        for (i, d) in self.surv.drops.iter().enumerate() {
            let to = d.body.pos + Vec3::Y * 0.2 - eye;
            let dist = to.length();
            if dist > LOOK_FAR {
                continue;
            }
            let along = to.dot(fwd);
            if along > 0.0 {
                let off = (to - fwd * along).length();
                if off < 0.5 && looked.is_none_or(|(_, o)| off < o) {
                    looked = Some((i, off));
                }
            }
            if dist < LABEL_FAR {
                near.push((dist, i));
            }
        }
        let looked = looked.map(|l| l.0);
        if let Some(i) = looked {
            if !near.iter().any(|n| n.1 == i) {
                near.push(((self.surv.drops[i].body.pos - eye).length(), i));
            }
        }
        if near.is_empty() {
            return;
        }
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        // The looked-at one first, so it always gets its place.
        if let Some(k) = looked.and_then(|i| near.iter().position(|n| n.1 == i)) {
            let n = near.remove(k);
            near.insert(0, n);
        }
        let s = self.ui.scale;
        let mut placed: Vec<[f32; 4]> = Vec::new();
        for &(dist, i) in near.iter() {
            if placed.len() >= MAX_LABELS {
                break;
            }
            let d = &self.surv.drops[i];
            // Not through walls or the ground.
            if !self.in_sight(&[d.body.pos + Vec3::Y * 0.3]) {
                continue;
            }
            let (lying, lift) = self.drop_lift(d);
            let top = d.body.pos + Vec3::Y * (lift + if lying { 0.55 } else { 0.5 });
            let c = vp * (top - cam_block.as_vec3()).extend(1.0);
            if c.w < 0.1 {
                continue;
            }
            let (nx, ny) = (c.x / c.w, c.y / c.w);
            if nx.abs() > 1.05 || ny.abs() > 1.05 {
                continue;
            }
            let fade = if Some(i) == looked { 1.0 } else { 1.0 - ((dist - (LABEL_FAR - 2.5)) / 2.5).clamp(0.0, 1.0) };
            if fade <= 0.02 {
                continue;
            }
            let (x, y) = ((nx * 0.5 + 0.5) * w, (0.5 - ny * 0.5) * h);
            // Smaller with distance, in whole pixel steps so the letters stay crisp.
            let sc = (s * (4.0 / dist.max(0.5)).clamp(0.5, 1.0)).round().max(1.0);
            let name = self.item_label(&d.stack);
            let label = if d.stack.count > 1 { format!("{name} x{}", d.stack.count) } else { name };
            let tw = self.ui.text_width(&label, sc);
            let (bw, bh) = (tw + 4.0 * sc, 10.0 * sc);
            // A label that would cover a nearer one moves up a row, or is left out.
            let overlaps = |by: f32, placed: &[[f32; 4]]| placed.iter().any(|r| x - bw / 2.0 < r[0] + r[2] && r[0] < x + bw / 2.0 && by < r[1] + r[3] && r[1] < by + bh);
            let Some(by) = (0..3).map(|k| y - bh - k as f32 * (bh + sc)).find(|&by| !overlaps(by, &placed)) else { continue };
            placed.push([x - bw / 2.0, by, bw, bh]);
            let a = |v: f32| (v * fade).round() as u8;
            let mut col = self.drop_name_color(&d.stack);
            col[3] = a(255.0);
            self.ui.rect(x - bw / 2.0, by, bw, bh, [0, 0, 0, a(110.0)]);
            self.ui.text_scaled(x - tw / 2.0, by + sc, &label, col, sc);
        }
    }

    /// Test hook MC_DROPS: a spread of items on the ground in front, plain, stacked, rare,
    /// enchanted and with a loot rarity, to see their names and glows.
    pub(crate) fn test_drops(&mut self) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-flat.z, 0.0, flat.x);
        let make = |name: &str, count: u8, ench: &[(u8, u8)], sockets: u32| {
            let mut st = ItemStack::new(self.items.id(name)?, count);
            for &(id, l) in ench {
                st.ench.set(id, l);
            }
            st.sockets = sockets;
            Some(st)
        };
        use crate::enchanting::{SHARPNESS, UNBREAKING};
        let list = [
            (make("cobblestone", 12, &[], 0), -1.6, 2.4),
            (make("apple", 1, &[], 0), 0.0, 2.2),
            (make("diamond_sword", 1, &[(SHARPNESS, 3), (UNBREAKING, 2)], 0), 1.3, 2.6),
            (make("diamond", 3, &[], 0), -0.9, 4.0),
            (make("iron_chestplate", 1, &[], voxel::gems::ranked(3)), 0.8, 4.3),
            (make("golden_apple", 1, &[], 0), 2.4, 4.0),
            (make("iron_pickaxe", 1, &[(UNBREAKING, 1)], 0), -2.6, 5.2),
            (make("oak_planks", 64, &[], 0), 0.2, 6.0),
        ];
        for (st, x, z) in list {
            let Some(st) = st else { continue };
            let at = self.body.pos + side * x + flat * z;
            let y = self.world.surface_y(at.x.floor() as i32, at.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(at.y);
            self.spawn_drop(Vec3::new(at.x, y + 0.05, at.z), st, Vec3::ZERO);
        }
        for d in self.surv.drops.iter_mut() {
            d.age = 1.0;
        }
    }
}
