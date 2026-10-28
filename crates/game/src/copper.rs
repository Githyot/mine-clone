//! The copper family: blocks that slowly turn green in the open (four stages), wax that keeps
//! them as they are, and an axe that scrapes wax or a stage of rust back off.
//!
//! Every stage is its own block, named the classic way: `copper_block`, `exposed_copper`,
//! `weathered_copper`, `oxidized_copper`, and `exposed_cut_copper_stairs` and so on for the
//! other shapes, each with a `waxed_` twin.

use glam::IVec3;
use voxel::block::{BlockId, BlockRegistry, AIR};

use crate::Game;

const STAGES: [&str; 4] = ["", "exposed_", "weathered_", "oxidized_"];

/// A copper block's place in the family: (waxed, stage 0..3, shape) where shape is `copper`
/// for the plain block or a name like `cut_copper_slab`.
fn split(name: &str) -> Option<(bool, usize, &str)> {
    let (waxed, rest) = match name.strip_prefix("waxed_") {
        Some(r) => (true, r),
        None => (false, name),
    };
    if rest == "copper_block" {
        return Some((waxed, 0, "copper"));
    }
    for (k, s) in STAGES.iter().enumerate().skip(1) {
        if let Some(shape) = rest.strip_prefix(s) {
            return shape.contains("copper").then_some((waxed, k, shape));
        }
    }
    (rest.contains("copper") && rest != "copper" && !rest.contains("ore") && !rest.contains("ingot")).then_some((waxed, 0, rest))
}

fn join(waxed: bool, stage: usize, shape: &str) -> String {
    let w = if waxed { "waxed_" } else { "" };
    if shape == "copper" && stage == 0 {
        format!("{w}copper_block")
    } else {
        format!("{w}{}{shape}", STAGES[stage])
    }
}

/// Per-block steps of the copper family, flattened for the random ticks.
pub struct Copper {
    /// The next stage of rust (AIR when it can't rust any more, or isn't copper).
    next: Vec<BlockId>,
    /// One stage less rust.
    prev: Vec<BlockId>,
    /// Waxed twin, and the unwaxed one for waxed blocks.
    waxed: Vec<BlockId>,
    unwaxed: Vec<BlockId>,
    t: f32,
    seed: u64,
}

impl Copper {
    pub fn new(reg: &BlockRegistry) -> Copper {
        let n = reg.len();
        let mut c = Copper { next: vec![AIR; n], prev: vec![AIR; n], waxed: vec![AIR; n], unwaxed: vec![AIR; n], t: 0.0, seed: 0x9e37_79b9_7f4a_7c15 };
        for (i, b) in reg.blocks.iter().enumerate() {
            if b.variant != 0 {
                continue;
            }
            let Some((w, stage, shape)) = split(&b.name) else { continue };
            let find = |name: String| reg.id(&name).unwrap_or(AIR);
            if !w && stage < 3 {
                c.next[i] = find(join(false, stage + 1, shape));
            }
            if stage > 0 {
                c.prev[i] = find(join(w, stage - 1, shape));
            }
            if w {
                c.unwaxed[i] = find(join(false, stage, shape));
            } else {
                c.waxed[i] = find(join(true, stage, shape));
            }
        }
        c
    }

    fn rand(&mut self) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }
}

impl Game {
    /// The same orientation (stairs, doors) of another block of the family.
    fn copper_swap(&mut self, p: IVec3, to: BlockId) {
        let id = self.world.block_or_air(p);
        let b = self.reg.get(id);
        let new = self.reg.variant(to, b.variant);
        self.world.set_block(p, new);
        // A door is two blocks.
        if b.shape == voxel::block::Shape::Door {
            let other = if b.variant & 8 != 0 { p - IVec3::Y } else { p + IVec3::Y };
            let o = self.world.block_or_air(other);
            if self.reg.get(o).base == b.base {
                let v = self.reg.get(o).variant;
                self.world.set_block(other, self.reg.variant(to, v));
            }
        }
    }

    /// Right click on copper with a honeycomb (wax it) or an axe (scrape off wax or a stage
    /// of rust). True when it did something.
    pub(crate) fn copper_use(&mut self, item: &str) -> bool {
        let Some(hit) = self.target else { return false };
        let id = self.world.block_or_air(hit.block);
        let base = self.reg.get(id).base as usize;
        let axe = self.held_tool().is_some_and(|t| t.kind == voxel::block::ToolKind::Axe);
        let to = if item == "honeycomb" {
            self.copper.waxed[base]
        } else if axe {
            if self.copper.unwaxed[base] != AIR {
                self.copper.unwaxed[base]
            } else {
                self.copper.prev[base]
            }
        } else {
            AIR
        };
        if to == AIR {
            return false;
        }
        self.copper_swap(hit.block, to);
        let (layer, tint) = self.block_look(to);
        self.particles.block_bits(hit.block + hit.normal, layer, tint, 6, 0.5);
        self.anim.start_swing();
        if self.survival {
            let sel = self.selected;
            if item == "honeycomb" {
                self.surv.inv.take(sel, 1);
            } else {
                self.wear_held(1);
            }
        }
        self.sfx(crate::audio::Sfx::Hit(crate::audio::Material::Stone));
        true
    }

    /// Random ticks near the player, the way plants and copper change in the classic game:
    /// three random blocks of every 16-block cube within four chunks, twenty times a second.
    /// Unwaxed copper takes its next stage of rust on about one tick in twenty, so a stage
    /// takes some twenty minutes.
    pub(crate) fn update_random_ticks(&mut self, dt: f32) {
        self.copper.t += dt;
        let me = self.body.pos.floor().as_ivec3();
        let (cx, cz) = (me.x.div_euclid(16), me.z.div_euclid(16));
        let y0 = (me.y - 48).max(0) & !15;
        let y1 = (me.y + 48).min(255);
        let mut steps = 0;
        while self.copper.t >= 0.05 && steps < 4 {
            self.copper.t -= 0.05;
            steps += 1;
            for dx in -4..=4 {
                for dz in -4..=4 {
                    let mut y = y0;
                    while y <= y1 {
                        for _ in 0..3 {
                            let r = self.copper.rand();
                            let p = IVec3::new((cx + dx) * 16 + (r & 15) as i32, y + ((r >> 4) & 15) as i32, (cz + dz) * 16 + ((r >> 8) & 15) as i32);
                            let Some(id) = self.world.block(p) else { continue };
                            let base = self.reg.get(id).base as usize;
                            let next = self.copper.next[base];
                            if next != AIR && (r >> 16) % 20 == 0 {
                                self.copper_swap(p, next);
                            }
                            if self.farm.is_crop(id) {
                                self.crop_tick(p, id, (r >> 24) as u32);
                            } else if id == self.farm.cane {
                                self.cane_tick(p, (r >> 24) as u32);
                            } else if self.pots.is_planted(id) {
                                self.pot_tick(p, id, (r >> 24) as u32);
                            } else if id != AIR && self.farm.hives.contains(&id) {
                                self.hive_tick(p, id, (r >> 24) as u32);
                            } else if self.extras.ticks(id) {
                                self.extras_tick(p, id, (r >> 24) as u32);
                            } else if self.pale.ticks(id) {
                                self.pale_tick(p, id, (r >> 24) as u32);
                            } else if self.elements.ticks(id) {
                                self.elements_tick(p, id, (r >> 24) as u32);
                            }
                        }
                        y += 16;
                    }
                }
            }
        }
        self.copper.t = self.copper.t.min(0.2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for n in ["copper_block", "exposed_copper", "waxed_oxidized_cut_copper_stairs", "weathered_copper_bulb_lit", "cut_copper", "waxed_copper_block", "copper_door"] {
            let (w, s, shape) = split(n).unwrap();
            assert_eq!(join(w, s, shape), n);
        }
        assert_eq!(split("copper_ore"), None);
        assert_eq!(split("copper_ingot"), None);
        assert_eq!(join(false, 1, "copper"), "exposed_copper");
    }
}
