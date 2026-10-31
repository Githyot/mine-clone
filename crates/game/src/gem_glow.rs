//! Gem blocks that shine: a ruby block lights the blocks around it red, sapphire blue, topaz
//! amber, and so on (`gem_lights` in the terrain shader). The blocks give off ordinary block
//! light (blocks.ron), so walls stop the glow where they stop that light; this only tints it
//! with the gem's colour. Their faces glow in the dark too (`voxel::mesh::is_emissive`).

use glam::{IVec3, Vec3};
use voxel::block::AIR;

use crate::Game;

/// How far around the player gem blocks are looked for, and how often.
const RANGE: i32 = 16;
const RISE: i32 = 8;
const SCAN_EVERY: f32 = 0.3;
/// Gem lights the shader takes (the nearest).
pub const SLOTS: usize = 12;

/// The colour a gem block shines with, brightest channel 1. Turned variants go by their base
/// block's name.
pub fn gem_color(name: &str) -> Option<[f32; 3]> {
    Some(match name {
        "ruby_block" => [1.0, 0.16, 0.2],
        "sapphire_block" => [0.22, 0.48, 1.0],
        "topaz_block" => [1.0, 0.66, 0.12],
        "emerald_block" => [0.2, 1.0, 0.42],
        "diamond_block" => [0.55, 0.95, 1.0],
        "lapis_block" => [0.24, 0.32, 1.0],
        "zanite_block" => [0.6, 0.42, 1.0],
        "amethyst_block" | "budding_amethyst" | "amethyst_cluster" | "amethyst_gem_leaves" => [0.74, 0.4, 1.0],
        _ => return None,
    })
}

#[derive(Default)]
pub struct GemGlow {
    /// Gem blocks near the player: (centre, colour), nearest first.
    spots: Vec<(Vec3, [f32; 3])>,
    scan_t: f32,
}

impl Game {
    pub(crate) fn update_gem_glow(&mut self, dt: f32) {
        self.gem_glow.scan_t -= dt;
        if self.gem_glow.scan_t > 0.0 {
            return;
        }
        self.gem_glow.scan_t = SCAN_EVERY;
        // Which block ids are gems, looked up once per scan instead of once per block.
        let colors: Vec<Option<[f32; 3]>> = self.reg.blocks.iter().map(|b| gem_color(&self.reg.get(b.base).name)).collect();
        if colors.iter().all(Option::is_none) {
            return;
        }
        let me = self.body.pos.floor().as_ivec3();
        let mut found = Vec::new();
        for y in -RISE..=RISE {
            for z in -RANGE..=RANGE {
                for x in -RANGE..=RANGE {
                    let p = me + IVec3::new(x, y, z);
                    let id = self.world.block_or_air(p);
                    if id == AIR {
                        continue;
                    }
                    if let Some(Some(c)) = colors.get(id as usize) {
                        found.push((p.as_vec3() + Vec3::splat(0.5), *c));
                    }
                }
            }
        }
        let eye = self.body.pos;
        found.sort_by(|a, b| (a.0 - eye).length_squared().total_cmp(&(b.0 - eye).length_squared()));
        found.truncate(SLOTS);
        self.gem_glow.spots = found;
    }

    /// The gem glow for the terrain shader: positions relative to the camera's block with the
    /// block's light level (0..1) in w, and the colours.
    pub(crate) fn gem_lights(&self, block: IVec3) -> ([[f32; 4]; SLOTS], [[f32; 4]; SLOTS]) {
        let mut at = [[0.0; 4]; SLOTS];
        let mut col = [[0.0; 4]; SLOTS];
        for (i, &(p, c)) in self.gem_glow.spots.iter().enumerate() {
            let r = p - block.as_vec3();
            let level = self.reg.light_of(self.world.block_or_air(p.floor().as_ivec3())).max(5) as f32 / 15.0;
            at[i] = [r.x, r.y, r.z, level];
            col[i] = [c[0], c[1], c[2], 1.0];
        }
        (at, col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gem_block_has_a_colour_and_light() {
        let reg = crate::game_data(crate::embedded::BLOCKS_RON, crate::embedded::ITEMS_RON, crate::embedded::RECIPES_RON).unwrap().0;
        for name in ["ruby_block", "sapphire_block", "topaz_block", "emerald_block", "diamond_block", "lapis_block", "zanite_block", "amethyst_block"] {
            let c = gem_color(name).unwrap();
            assert!((c.iter().cloned().fold(0.0, f32::max) - 1.0).abs() < 1e-6, "{name}: brightest channel 1");
            let id = reg.id(name).unwrap_or_else(|| panic!("{name} missing"));
            assert!(reg.light_of(id) >= 7, "{name} should give off light");
        }
        assert!(gem_color("stone").is_none());
        assert!(gem_color("ruby_ore").is_none());
    }
}
