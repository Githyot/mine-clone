//! Vanity and dyes (Lote 11, Terraria's idea): a column of "Visual" slots beside the
//! accessories. Armour put in a vanity slot is what the player looks like (the real armour
//! still protects), and a dye in the slot next to it recolours that piece; the fifth dye
//! colours the wings. Four cloth outfits exist only to be worn for looks.

use crate::survival::Slot;
use crate::Game;

/// Dye slots: one per armour piece, then the wings.
pub const DYE_SLOTS: usize = 5;

/// Vanity outfits: material names in `entity::ARMOR_MATERIALS` (cloth, drawn matte).
pub const OUTFITS: [&str; 4] = ["tuxedo", "pirate", "explorer", "royal"];

#[derive(Clone, Copy, PartialEq)]
pub enum Dye {
    Plain([u8; 3]),
    /// Glows in the dark too.
    Bright([u8; 3]),
    /// Runs through every colour.
    Rainbow,
    /// Flickers like a flame, and glows.
    Flame,
}

pub const DYES: &[(&str, Dye)] = &[
    ("red_tint", Dye::Plain([200, 40, 40])),
    ("orange_tint", Dye::Plain([230, 120, 30])),
    ("yellow_tint", Dye::Plain([230, 200, 50])),
    ("lime_tint", Dye::Plain([120, 210, 50])),
    ("green_tint", Dye::Plain([40, 150, 60])),
    ("cyan_tint", Dye::Plain([40, 180, 190])),
    ("blue_tint", Dye::Plain([50, 80, 210])),
    ("purple_tint", Dye::Plain([130, 50, 190])),
    ("pink_tint", Dye::Plain([230, 110, 170])),
    ("black_tint", Dye::Plain([40, 40, 46])),
    ("silver_tint", Dye::Plain([200, 204, 214])),
    ("brown_tint", Dye::Plain([120, 76, 40])),
    ("bright_blue_tint", Dye::Bright([90, 150, 255])),
    ("bright_green_tint", Dye::Bright([90, 255, 120])),
    ("rainbow_tint", Dye::Rainbow),
    ("flame_tint", Dye::Flame),
];

pub fn dye_of(item: &str) -> Option<Dye> {
    DYES.iter().find(|d| d.0 == item).map(|d| d.1)
}

#[derive(Default, Clone)]
pub struct Looks {
    pub vanity: [Option<voxel::item::ItemStack>; 4],
    pub dyes: [Option<voxel::item::ItemStack>; DYE_SLOTS],
}

fn hue(h: f32) -> [f32; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - ((h % 2.0) - 1.0).abs();
    match h as u32 {
        0 => [1.0, x, 0.0],
        1 => [x, 1.0, 0.0],
        2 => [0.0, 1.0, x],
        3 => [0.0, x, 1.0],
        4 => [x, 0.0, 1.0],
        _ => [1.0, 0.0, x],
    }
}

/// The tint a dye puts on the model at time `t` (`Bones::tint`).
pub fn tint(d: Dye, t: f32) -> [f32; 4] {
    let c = |c: [u8; 3]| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0];
    match d {
        Dye::Plain(rgb) => {
            let [r, g, b] = c(rgb);
            [r, g, b, 1.0]
        }
        Dye::Bright(rgb) => {
            let [r, g, b] = c(rgb);
            [r, g, b, 2.0]
        }
        Dye::Rainbow => {
            let [r, g, b] = hue(t * 0.15);
            [r * 0.85 + 0.1, g * 0.85 + 0.1, b * 0.85 + 0.1, 1.0]
        }
        Dye::Flame => {
            let f = ((t * 9.0).sin() * 0.5 + (t * 23.0).sin() * 0.3) * 0.5 + 0.5;
            [1.0, 0.35 + f * 0.35, 0.08 + f * 0.1, 2.0]
        }
    }
}

impl Game {
    /// The tint from dye slot `i`, if dyed.
    fn dye_tint(&self, i: usize) -> Option<[f32; 4]> {
        let st = self.t5.look.dyes.get(i).copied().flatten()?;
        dye_of(&self.items.get(st.item).name).map(|d| tint(d, self.now))
    }

    /// What the player wears as seen from outside: per piece the vanity armour over the real
    /// armour, with its dye; then the wings with theirs. Pairs of (mesh, bones).
    pub(crate) fn look_instances(&self, bones: &crate::entity::Bones) -> Vec<(usize, crate::entity::Bones)> {
        let mut out = Vec::new();
        for piece in 0..4 {
            let shown = self.t5.look.vanity[piece].or(self.surv.armor[piece]);
            let Some(st) = shown else { continue };
            let Some(mesh) = self.entities.armor_mesh(&self.items.get(st.item).name, piece) else { continue };
            let mut b = *bones;
            if let Some(t) = self.dye_tint(piece) {
                b.tint = t;
            }
            out.push((mesh, b));
            // Smithing trims show as a coloured pattern over the piece.
            if let Some(t) = crate::smithing::trim_of(&st).and_then(|(_, m)| self.entities.trim_mesh(m, piece)) {
                out.push((t, b));
            }
        }
        if let Some(k) = self.worn_wings() {
            if let Some(mesh) = self.entities.wing_mesh(k) {
                let mut b = self.wing_bones(bones);
                b.tint = self.dye_tint(4).unwrap_or(if crate::wings::WINGS[k].glow { [1.0, 1.0, 1.0, 3.0] } else { [1.0, 1.0, 1.0, 0.0] });
                out.push((mesh, b));
            }
        }
        out
    }

    /// The "Visual" column left of the accessories: four vanity slots beside four dye slots,
    /// and the wings' dye under them. `x` is the accessory column's left edge.
    pub(crate) fn visual_slots(&mut self, x: f32, y: f32) -> Vec<(Slot, f32, f32)> {
        let s = self.ui.scale;
        let (px, py) = (x - 44.0 * s, y);
        let ph = (5.0 * 18.0 + 16.0) * s;
        self.ui.rect(px - 2.0 * s, py - 10.0 * s, 42.0 * s, ph + 10.0 * s, [198, 198, 198, 255]);
        self.ui.frame(px - 2.0 * s, py - 10.0 * s, 42.0 * s, ph + 10.0 * s, s, [255, 255, 255, 255]);
        self.ui.label(px - 1.0 * s, py - 6.0 * s, "Visual", [64, 64, 64, 255], s);
        let mut out = Vec::new();
        for i in 0..4 {
            out.push((Slot::Vanity(i), px + 1.0 * s, py + (6.0 + i as f32 * 18.0) * s));
            out.push((Slot::Dye(i), px + 20.0 * s, py + (6.0 + i as f32 * 18.0) * s));
        }
        self.ui.label(px - 1.0 * s, py + (6.0 + 4.0 * 18.0 + 5.0) * s, "Asas", [64, 64, 64, 255], s);
        out.push((Slot::Dye(4), px + 20.0 * s, py + (6.0 + 4.0 * 18.0) * s));
        out
    }

    /// May the item under the cursor go into this Visual slot?
    pub(crate) fn visual_fits(&self, slot: Slot, item: &str) -> bool {
        match slot {
            Slot::Vanity(i) => self.items.id(item).and_then(|id| self.items.get(id).armor.as_ref()).is_some_and(|a| a.slot as usize == i),
            Slot::Dye(_) => dye_of(item).is_some(),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyes_tint_and_outfits_are_armour_materials() {
        for (name, d) in DYES {
            let t = tint(*d, 3.7);
            assert!(t[3] >= 1.0 && t[..3].iter().all(|c| (0.0..=1.0).contains(c)), "{name}");
        }
        for o in OUTFITS {
            assert!(crate::entity::ARMOR_MATERIALS.iter().any(|(n, _)| *n == o));
        }
        // The rainbow really changes colour.
        assert_ne!(tint(Dye::Rainbow, 0.0), tint(Dye::Rainbow, 2.0));
    }
}
