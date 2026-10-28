//! Wings (Lote 11): every pair flies for its own time per take-off, climbs at its own rate and
//! speeds up flight, then glides once the time runs out (Terraria's idea). They show on the
//! player's back: folded when standing, flapping in flight, spread wide when gliding. Each
//! pair is painted here from its palette and feather style; nothing is copied.

use glam::{Mat4, Vec3};

use crate::Game;

/// How a pair is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Feather,
    /// Bat-like skin between bony fingers.
    Membrane,
    /// Two rounded panes with veins (fairy, bee, butterfly).
    Insect,
    Leaf,
}

pub struct WingDef {
    pub item: &'static str,
    /// Seconds of flight per take-off.
    pub flight: f32,
    /// Fastest climb (blocks per second).
    pub lift: f32,
    /// Flying speed, times walking.
    pub speed: f32,
    pub style: Style,
    /// Dark, middle and light colours.
    pub colors: [[u8; 3]; 3],
    /// Glows in the dark (flame, fairy, frozen and celestial wings).
    pub glow: bool,
}

const fn w(item: &'static str, flight: f32, lift: f32, speed: f32, style: Style, colors: [[u8; 3]; 3], glow: bool) -> WingDef {
    WingDef { item, flight, lift, speed, style, colors, glow }
}

pub const WINGS: &[WingDef] = &[
    w("feather_wings", 1.6, 6.0, 1.1, Style::Feather, [[150, 140, 120], [210, 204, 188], [246, 242, 230]], false),
    w("angel_wings", 3.2, 7.0, 1.25, Style::Feather, [[190, 170, 110], [240, 230, 190], [255, 252, 236]], false),
    w("harpy_wings", 2.2, 6.5, 1.2, Style::Feather, [[60, 84, 120], [96, 136, 176], [170, 206, 228]], false),
    w("demon_wings", 2.6, 7.0, 1.3, Style::Membrane, [[50, 14, 20], [110, 28, 36], [170, 60, 60]], false),
    w("bee_wings", 2.4, 6.5, 1.25, Style::Insect, [[120, 96, 30], [220, 200, 110], [250, 244, 200]], false),
    w("fairy_wings", 2.8, 7.0, 1.35, Style::Insect, [[150, 60, 140], [230, 140, 210], [255, 220, 250]], true),
    w("butterfly_wings", 3.0, 7.0, 1.35, Style::Insect, [[24, 40, 110], [50, 110, 210], [150, 210, 255]], false),
    w("bone_wings", 3.0, 7.5, 1.35, Style::Membrane, [[110, 104, 90], [196, 190, 170], [236, 232, 218]], false),
    w("leaf_wings", 3.0, 7.0, 1.4, Style::Leaf, [[30, 80, 26], [70, 150, 50], [150, 210, 90]], false),
    w("frozen_wings", 3.4, 7.5, 1.4, Style::Feather, [[60, 120, 170], [130, 200, 236], [220, 248, 255]], true),
    w("flame_wings", 3.4, 8.0, 1.45, Style::Feather, [[140, 30, 10], [236, 110, 30], [255, 220, 120]], true),
    w("breaker_wings", 4.0, 8.5, 1.55, Style::Membrane, [[36, 30, 48], [90, 70, 120], [170, 140, 220]], false),
    w("betsy_wing", 3.6, 8.0, 1.5, Style::Membrane, [[60, 14, 6], [150, 50, 18], [220, 120, 50]], false),
    w("festive_wings", 3.6, 8.0, 1.5, Style::Leaf, [[16, 60, 24], [40, 130, 50], [240, 210, 90]], true),
    w("celestial_wings", 4.8, 9.0, 1.7, Style::Feather, [[40, 90, 120], [90, 200, 210], [210, 255, 250]], true),
];

pub fn wing_of(item: &str) -> Option<usize> {
    WINGS.iter().position(|w| w.item == item)
}

/// Body parts the two wings ride on in the player's bones (unused by the model itself).
pub const WING_R: usize = 12;
pub const WING_L: usize = 13;
/// One wing in model pixels: from the shoulder blade out, top down.
pub const WING_SIZE: (f32, f32) = (14.0, 16.0);

/// The skin of one pair, at `k` texels per model pixel: both big faces of the wing box at
/// texture origin (0, 0) carry the picture; the thin edges take the dark colour.
pub fn paint(def: &WingDef, k: u32) -> Vec<u8> {
    let (sw, sh) = (crate::entity::SKIN_W * k, crate::entity::SKIN_H * k);
    let mut px = vec![0u8; (sw * sh * 4) as usize];
    let (ww, wh) = (WING_SIZE.0 as u32 * k, WING_SIZE.1 as u32 * k);
    let d = k;
    let mut put = |x: u32, y: u32, c: [u8; 3]| {
        let i = ((y * sw + x) * 4) as usize;
        px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
    };
    for j in 0..wh {
        for i in 0..ww {
            // u from the shoulder (0) to the tip (1), v from the top (0) down.
            let u = (i as f32 + 0.5) / ww as f32;
            let v = (j as f32 + 0.5) / wh as f32;
            let Some(c) = texel(def, u, v) else { continue };
            // Front face, and the back face mirrored so the picture lines up through the wing.
            put(d + i, d + j, c);
            put(2 * d + ww + (ww - 1 - i), d + j, c);
        }
    }
    // Edges and ends.
    for y in 0..d + wh {
        for x in 0..2 * d + 2 * ww {
            let i = ((y * sw + x) * 4) as usize;
            if px[i + 3] == 0 && (y < d || x < d || (x >= d + ww && x < 2 * d + ww)) {
                let c = def.colors[0];
                px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
    }
    px
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t) as u8)
}

fn hash(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(374_761_393) ^ b.wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// One texel of a wing, or None where the outline leaves it open.
fn texel(def: &WingDef, u: f32, v: f32) -> Option<[u8; 3]> {
    let [dark, mid, light] = def.colors;
    match def.style {
        Style::Feather => {
            // Covert feathers near the top, then long flight feathers that get longer
            // toward the tip; the bottom edge is scalloped by each feather's end.
            let n = 6.0;
            let f = (u * n).floor();
            let fu = u * n - f;
            let len = 0.55 + 0.45 * (f / (n - 1.0)).powf(0.8);
            let end = len - (fu - 0.5).powi(2) * 0.5;
            if v > end || v < 0.05 * (1.0 - u) {
                return None;
            }
            let shaft = (fu - 0.5).abs() < 0.07 && v > 0.3;
            let edge = fu < 0.08;
            let top = v < 0.3;
            let shade = 1.0 - v * 0.5 + hash((u * 64.0) as u32, (v * 64.0) as u32) * 0.08;
            Some(if shaft {
                mix(mid, dark, 0.4)
            } else if edge {
                mix(mid, dark, 0.6)
            } else if top {
                mix(light, mid, 0.4 + (v * 12.0).sin().abs() * 0.3)
            } else {
                mix(mid, light, shade)
            })
        }
        Style::Membrane => {
            // Three bony fingers from the shoulder, skin hanging between them in arcs.
            let fingers = [0.05f32, 0.45, 0.8];
            let mut bone = v < 0.08 + 0.04 * u;
            let mut low = 1.0f32;
            for (k, a) in fingers.iter().enumerate() {
                // Each finger runs from the shoulder (0, 0.05) to (1, a) along the wing.
                let line = 0.05 + (a - 0.05) * u;
                if (v - line).abs() < 0.035 && u > 0.08 {
                    bone = true;
                }
                if k == fingers.len() - 1 {
                    low = line;
                }
            }
            // The hem between finger tips sags in scallops.
            let hem = low.max(0.5 + 0.45 * u) + ((u * 3.0 * std::f32::consts::PI).sin().abs() - 1.0) * 0.12;
            if v > hem.min(0.98) {
                return None;
            }
            Some(if bone {
                light
            } else {
                mix(dark, mid, 0.5 + hash((u * 40.0) as u32, (v * 40.0) as u32) * 0.2 - v * 0.3)
            })
        }
        Style::Insect => {
            // A big fore pane and a smaller hind pane, each an ellipse with veins.
            let fore = ((u - 0.55) / 0.48).powi(2) + ((v - 0.32) / 0.3).powi(2);
            let hind = ((u - 0.38) / 0.36).powi(2) + ((v - 0.72) / 0.26).powi(2);
            let (pane, cu, cv) = if fore <= 1.0 { (fore, 0.0, 0.1) } else if hind <= 1.0 { (hind, 0.0, 0.55) } else { return None };
            let ang = (v - cv).atan2(u - cu);
            let vein = (ang * 9.0).sin().abs() < 0.12 || pane > 0.86;
            let spot = def.item == "butterfly_wings" && ((u - 0.75).powi(2) + (v - 0.25).powi(2)) < 0.006;
            Some(if vein {
                dark
            } else if spot {
                light
            } else {
                mix(light, mid, pane * 0.9)
            })
        }
        Style::Leaf => {
            // Three overlapping leaves, each pointed with a midrib.
            for (k, (cu, cv, a)) in [(0.5f32, 0.25f32, 0.15f32), (0.55, 0.55, 0.35), (0.45, 0.8, 0.55)].iter().enumerate() {
                let du = u - cu;
                let dv = v - cv - du * a;
                let half = 0.17 * (1.0 - (du / 0.5).powi(2)).max(0.0);
                if dv.abs() <= half {
                    let rib = dv.abs() < 0.02;
                    let vein = ((du * 30.0) - dv.abs() * 40.0).sin().abs() < 0.15;
                    let base = if k % 2 == 0 { mid } else { mix(mid, dark, 0.3) };
                    return Some(if rib { light } else if vein { mix(base, light, 0.4) } else { mix(base, dark, dv.abs() / half.max(1e-3) * 0.4) });
                }
            }
            None
        }
    }
}

impl Game {
    /// The pair of wings worn (first one in the accessory slots).
    pub(crate) fn worn_wings(&self) -> Option<usize> {
        let n = self.acc_slots();
        self.t5.acc[..n].iter().flatten().find_map(|s| wing_of(&self.items.get(s.item).name))
    }

    /// The wings over the player's back: `body` is the body bone, `t` the time. Folded on the
    /// ground, flapping while climbing, held wide while gliding.
    pub(crate) fn wing_bones(&self, bones: &crate::entity::Bones) -> crate::entity::Bones {
        let mut b = *bones;
        let body = Mat4::from_cols_array_2d(&bones.m[1]);
        let flying = self.t5.moves.flying && self.t5.moves.wing_t > 0.0;
        let air = !self.body.on_ground && !self.body.in_water && self.seat.is_none() && self.riding.is_none() && self.t5.comp.mount.is_none();
        let (sweep, flap) = if flying {
            (0.1, 0.55 + (self.now * 14.0).sin() * 0.5)
        } else if air {
            // Gliding: spread, a slow ripple.
            (0.05, 0.85 + (self.now * 3.0).sin() * 0.05)
        } else {
            // Folded along the back, breathing a little.
            (1.25, -0.35 + (self.now * 1.4).sin() * 0.03)
        };
        for (part, side) in [(WING_R, 1.0f32), (WING_L, -1.0)] {
            let m = body
                * Mat4::from_translation(Vec3::new(side * 1.0, 22.0, -2.2))
                * Mat4::from_scale(Vec3::new(side, 1.0, 1.0))
                * Mat4::from_rotation_y(sweep)
                * Mat4::from_rotation_z(flap);
            b.m[part] = m.to_cols_array_2d();
        }
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pair_is_an_accessory_and_paints() {
        for w in WINGS {
            assert!(crate::rpg::accessory(w.item).is_some(), "{} must be an accessory", w.item);
            let px = paint(w, 2);
            let filled = px.chunks(4).filter(|p| p[3] == 255).count();
            assert!(filled > 400, "{} paints {} texels", w.item, filled);
        }
        // Better wings fly longer.
        assert!(WINGS.iter().all(|w| w.flight >= 1.6 && w.lift <= 9.0));
    }
}

#[cfg(test)]
mod lote11_tests {
    /// Every item the Lote 11 tables name is in the game's data (tools/gen_lote11.py).
    #[test]
    fn lote11_items_exist() {
        let (_, items, recipes) = crate::game_data(crate::embedded::BLOCKS_RON, crate::embedded::ITEMS_RON, crate::embedded::RECIPES_RON).unwrap();
        let mut names: Vec<String> = super::WINGS.iter().map(|w| w.item.to_string()).collect();
        names.extend(crate::companions::MOUNTS.iter().map(|m| m.item.to_string()));
        names.extend(crate::companions::PETS.iter().map(|p| p.item.to_string()));
        names.extend(crate::vanity::DYES.iter().map(|d| d.0.to_string()));
        names.extend(crate::sentries::SENTRIES.iter().flat_map(|s| [s.staff.to_string(), s.sprite.to_string()]));
        for o in crate::vanity::OUTFITS {
            for p in ["helmet", "chestplate", "leggings", "boots"] {
                names.push(format!("{o}_{p}"));
            }
        }
        for n in &names {
            let id = items.id(n).unwrap_or_else(|| panic!("missing item {n}"));
            // Everything but the turret pictures and the boss-only wings can be made.
            let made = recipes.shaped.iter().any(|r| r.result.item == id) || recipes.shapeless.iter().any(|r| r.result.item == id);
            let dropped = ["breaker_wings", "feather_wings", "angel_wings", "betsy_wing", "festive_wings", "ancient_horn", "witchs_broom", "reindeer_bells", "scaly_truffle"].contains(&n.as_str()) || n.starts_with("sentry_");
            assert!(made || dropped, "{n} has no recipe");
        }
    }
}
