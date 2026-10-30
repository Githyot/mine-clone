//! Redstone that shows itself: lit redstone torches (and the lit little torches on repeaters
//! and comparators) cast a red glow on the blocks around them (`red_lights` in the terrain
//! shader) and give off a few red sparks now and then, and right clicking a repeater or a
//! comparator says what it was set to.

use glam::{IVec3, Vec3};
use voxel::block::{BlockId, Shape, AIR};

use crate::Game;

/// How far around the player glowing redstone is looked for, and how often.
const RANGE: i32 = 16;
const RISE: i32 = 8;
const SCAN_EVERY: f32 = 0.25;
/// Most torches that throw sparks at once (the nearest).
const MOST: usize = 48;
/// How bright the red glow is: a redstone torch (light 7) and a repeater's little torch.
const TORCH_GLOW: f32 = 0.55;
const PART_GLOW: f32 = 0.3;

#[derive(Default)]
pub struct RedstoneFx {
    /// Glowing torch heads near the player: (where, glow 0..1), nearest first.
    spots: Vec<(Vec3, f32)>,
    /// Time to the next look around (set to 0 to look again right away).
    pub(crate) scan_t: f32,
    owed: f32,
    count: u32,
    warmed: bool,
}

/// The glowing torch heads of a block, in block pixels (0..16), with how bright each glows.
pub fn heads(shape: Shape, name: &str, variant: u8) -> Vec<([f32; 3], f32)> {
    match shape {
        Shape::Torch if name == "redstone_torch" => {
            if variant == 0 {
                return vec![([8.0, 10.0, 8.0], TORCH_GLOW)];
            }
            // On a wall: drawn leaning out from a wall to the north, then turned (mesh.rs).
            let mut p = [8.0, 13.0, 6.0];
            for _ in 0..(variant - 1) & 3 {
                p = [16.0 - p[2], p[1], p[0]];
            }
            vec![(p, TORCH_GLOW)]
        }
        Shape::Repeater | Shape::Comparator => {
            let lit = name.ends_with("_on");
            let mut boxes = Vec::new();
            voxel::shapes::boxes(shape, variant, 0, false, &mut boxes);
            boxes
                .iter()
                .enumerate()
                .skip(1)
                .filter(|&(i, _)| if shape == Shape::Comparator && i == 1 { variant & 4 != 0 } else { lit })
                .map(|(_, b)| ([(b[0] + b[3]) as f32 / 2.0, b[4] as f32, (b[2] + b[5]) as f32 / 2.0], PART_GLOW))
                .collect()
        }
        _ => Vec::new(),
    }
}

/// What a repeater or comparator is set to, for the message after a right click.
pub fn setting(shape: Shape, variant: u8) -> Option<String> {
    match shape {
        Shape::Repeater => {
            let ticks = ((variant >> 2) & 3) + 1;
            Some(format!("Repetidor: atraso {ticks} de 4 ({:.1} s)", ticks as f32 * 0.1).replace('.', ","))
        }
        Shape::Comparator => Some(if variant & 4 != 0 { "Comparador: modo subtrair (tocha da frente acesa)" } else { "Comparador: modo comparar (tocha da frente apagada)" }.into()),
        _ => None,
    }
}

impl Game {
    pub(crate) fn update_redstone_fx(&mut self, dt: f32, shot: bool) {
        self.redstone_fx.scan_t -= dt;
        if self.redstone_fx.scan_t <= 0.0 || (shot && !self.redstone_fx.warmed) {
            self.redstone_fx.scan_t = SCAN_EVERY;
            self.scan_redstone_glow();
        }
        if shot {
            if !self.redstone_fx.warmed && !self.redstone_fx.spots.is_empty() {
                self.redstone_fx.warmed = true;
                for _ in 0..60 {
                    self.redstone_sparks(0.03);
                    self.particles.update(&self.world, 0.03);
                }
            }
            return;
        }
        self.redstone_sparks(dt);
    }

    fn scan_redstone_glow(&mut self) {
        let me = self.body.pos.floor().as_ivec3();
        let mut cache: Vec<(BlockId, Vec<([f32; 3], f32)>)> = Vec::new();
        let mut found = Vec::new();
        for y in -RISE..=RISE {
            for z in -RANGE..=RANGE {
                for x in -RANGE..=RANGE {
                    let p = me + IVec3::new(x, y, z);
                    let id = self.world.block_or_air(p);
                    if id == AIR {
                        continue;
                    }
                    let b = self.reg.get(id);
                    if !matches!(b.shape, Shape::Torch | Shape::Repeater | Shape::Comparator) {
                        continue;
                    }
                    let hs = match cache.iter().find(|c| c.0 == id) {
                        Some(c) => &c.1,
                        None => {
                            // Turned variants go by their base block's name ("repeater_on").
                            cache.push((id, heads(b.shape, &self.reg.get(b.base).name, b.variant)));
                            &cache.last().unwrap().1
                        }
                    };
                    for &(h, glow) in hs {
                        found.push((p.as_vec3() + Vec3::from(h) / 16.0, glow));
                    }
                }
            }
        }
        let eye = self.body.pos;
        found.sort_by(|a, b| (a.0 - eye).length_squared().total_cmp(&(b.0 - eye).length_squared()));
        found.truncate(MOST);
        self.redstone_fx.spots = found;
    }

    /// The red glow for the terrain shader: the nearest glowing redstone, one light per block.
    pub(crate) fn red_lights(&self, block: IVec3) -> [[f32; 4]; 8] {
        let mut out = [[0.0; 4]; 8];
        let mut used: Vec<IVec3> = Vec::new();
        let mut n = 0;
        for &(p, glow) in &self.redstone_fx.spots {
            let cell = p.floor().as_ivec3();
            if used.contains(&cell) {
                continue;
            }
            used.push(cell);
            let r = p - block.as_vec3();
            out[n] = [r.x, r.y, r.z, glow];
            n += 1;
            if n == out.len() {
                break;
            }
        }
        out
    }

    /// A few red sparks drifting up off each glowing torch head.
    fn redstone_sparks(&mut self, dt: f32) {
        if self.redstone_fx.spots.is_empty() {
            return;
        }
        let glow = self.glow_layer();
        // About one and a half sparks a second per torch.
        self.redstone_fx.owed += dt * 1.6 * self.redstone_fx.spots.len() as f32;
        while self.redstone_fx.owed >= 1.0 {
            self.redstone_fx.owed -= 1.0;
            self.redstone_fx.count = self.redstone_fx.count.wrapping_add(1);
            let k = self.redstone_fx.count as usize;
            let (p, g) = self.redstone_fx.spots[k % self.redstone_fx.spots.len()];
            let size = if g >= TORCH_GLOW { 0.035 } else { 0.025 };
            self.particles.flame(p, Vec3::ZERO, glow, size, 0.9, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torch_heads_and_lit_parts() {
        assert_eq!(heads(Shape::Torch, "redstone_torch", 0).len(), 1);
        assert!(heads(Shape::Torch, "redstone_torch_off", 0).is_empty());
        assert!(heads(Shape::Torch, "torch", 0).is_empty());
        // Unlit repeater: nothing; lit: both little torches.
        assert!(heads(Shape::Repeater, "repeater", 1).is_empty());
        assert_eq!(heads(Shape::Repeater, "repeater_on", 1).len(), 2);
        // Comparator: the front torch glows in subtract mode even with no output.
        assert!(heads(Shape::Comparator, "comparator", 0).is_empty());
        assert_eq!(heads(Shape::Comparator, "comparator", 4).len(), 1);
        assert_eq!(heads(Shape::Comparator, "comparator_on", 4).len(), 3);
        assert_eq!(heads(Shape::Comparator, "comparator_on", 0).len(), 2);
    }

    #[test]
    fn settings_read_out() {
        assert_eq!(setting(Shape::Repeater, 3 << 2).unwrap(), "Repetidor: atraso 4 de 4 (0,4 s)");
        assert!(setting(Shape::Comparator, 4).unwrap().contains("subtrair"));
        assert!(setting(Shape::Torch, 0).is_none());
    }
}
