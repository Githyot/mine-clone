//! Double doors. A door goes down hinged on the side away from a door already beside it
//! (facing the same way), so the two meet in the middle and swing apart, and one opened by
//! hand takes the other with it. With no door beside it the hinge goes against the wall, or to
//! the half of the block that was clicked, like vanilla.

use glam::{IVec3, Vec3};
use voxel::block::Shape;
use voxel::shapes::DOOR_HINGE_LEFT;
use voxel::{BlockId, BlockRegistry, AIR};

use crate::Game;

/// North, east, south, west: the four ways a door faces (`shapes::facing_of`).
const DIR4: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(1, 0, 0), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0)];

/// The player's right and left looking the way a door faces.
fn sides(facing: u8) -> (IVec3, IVec3) {
    (DIR4[(facing as usize + 1) & 3], DIR4[(facing as usize + 3) & 3])
}

/// The lower half of a door facing `facing` at `p`, its variant.
fn lower_door(reg: &BlockRegistry, get: &impl Fn(IVec3) -> BlockId, p: IVec3, facing: u8) -> Option<u8> {
    let b = reg.get(get(p));
    (b.shape == Shape::Door && b.variant & 8 == 0 && b.variant & 3 == facing).then_some(b.variant)
}

/// Whether a door going down at `pos` facing `facing` (clicked at `at`) is hinged on the left.
pub fn hinge_left(reg: &BlockRegistry, get: impl Fn(IVec3) -> BlockId, pos: IVec3, facing: u8, at: Vec3) -> bool {
    let (right, left) = sides(facing);
    let (dl, dr) = (lower_door(reg, &get, pos + left, facing).is_some(), lower_door(reg, &get, pos + right, facing).is_some());
    if dl != dr {
        return dr;
    }
    // Against the side with more wall (full blocks beside either half).
    let wall = |d: IVec3| {
        (0..2)
            .filter(|&y| {
                let b = reg.get(get(pos + d + IVec3::Y * y));
                b.solid && b.shape == Shape::Cube
            })
            .count() as i32
    };
    let w = wall(right) - wall(left);
    if w != 0 {
        return w < 0;
    }
    (at - (pos.as_vec3() + Vec3::splat(0.5))).dot(left.as_vec3()) > 0.0
}

/// The other leaf of a double door: a door beside the lower half at `lower` on its free side,
/// facing the same way and hinged on its far side.
pub fn partner(reg: &BlockRegistry, get: impl Fn(IVec3) -> BlockId, lower: IVec3) -> Option<IVec3> {
    let b = reg.get(get(lower));
    if b.shape != Shape::Door || b.variant & 8 != 0 {
        return None;
    }
    let facing = b.variant & 3;
    let (right, left) = sides(facing);
    let left_hinge = b.variant & DOOR_HINGE_LEFT != 0;
    let p = lower + if left_hinge { right } else { left };
    let v = lower_door(reg, &get, p, facing)?;
    ((v & DOOR_HINGE_LEFT != 0) != left_hinge).then_some(p)
}

/// A door that was just put down beside another one hinged toward it: that one's hinge moves
/// to its far side so the two make a pair, unless a door stands there too. Returns its lower
/// half when it changed.
pub fn turn_neighbour(reg: &BlockRegistry, get: impl Fn(IVec3) -> BlockId, lower: IVec3) -> Option<IVec3> {
    let b = reg.get(get(lower));
    let facing = b.variant & 3;
    let (right, left) = sides(facing);
    let left_hinge = b.variant & DOOR_HINGE_LEFT != 0;
    let toward = if left_hinge { right } else { left };
    let p = lower + toward;
    let v = lower_door(reg, &get, p, facing)?;
    let same = (v & DOOR_HINGE_LEFT != 0) == left_hinge;
    (same && lower_door(reg, &get, p + toward, facing).is_none()).then_some(p)
}

impl Game {
    /// Which way a door placed at `pos` facing `facing` gets its hinge (its variant bit).
    pub(crate) fn door_hinge(&self, pos: IVec3, facing: u8, at: Vec3) -> u8 {
        if hinge_left(&self.reg, |p| self.world.block_or_air(p), pos, facing, at) {
            DOOR_HINGE_LEFT
        } else {
            0
        }
    }

    /// After a door went down at `lower`: a door beside it hinged toward it turns round so the
    /// two make a double door.
    pub(crate) fn pair_doors(&mut self, lower: IVec3) {
        if let Some(p) = turn_neighbour(&self.reg, |p| self.world.block_or_air(p), lower) {
            self.set_door(p, |v| v ^ DOOR_HINGE_LEFT);
        }
    }

    /// The other leaf of a double door with an open state changed by hand follows it.
    pub(crate) fn door_follow(&mut self, any_half: IVec3) {
        let b = self.reg.get(self.world.block_or_air(any_half));
        let lower = if b.variant & 8 != 0 { any_half - IVec3::Y } else { any_half };
        let open = b.variant & 4;
        let Some(p) = partner(&self.reg, |p| self.world.block_or_air(p), lower) else { return };
        let pb = self.reg.get(self.world.block_or_air(p));
        // Iron doors only move for redstone.
        if pb.variant & 4 != open && !self.reg.get(pb.base).name.starts_with("iron_") {
            self.set_door(p, |v| v ^ 4);
        }
    }

    /// Test hook MC_DOORS=x: a plank wall ahead with two double doors put down the way a player
    /// would (the first hinged on its right, as every door used to go), the right pair opened
    /// by hand; the camera stands x blocks to the side (-3 faces the closed pair, 2 the open one).
    pub(crate) fn build_door_test(&mut self) {
        let p = self.body.pos.floor().as_ivec3();
        let g = |n: &str| self.reg.id_or_air(n);
        let (planks, floor, door) = (g("oak_planks"), g("stone"), g("oak_door"));
        let z = p.z - 4;
        for x in -6..=6 {
            for dz in -6..=1 {
                self.world.set_block(IVec3::new(p.x + x, p.y - 1, z + dz), floor);
                for y in 0..4 {
                    self.world.set_block(IVec3::new(p.x + x, p.y + y, z + dz), AIR);
                }
            }
            for y in 0..3 {
                self.world.set_block(IVec3::new(p.x + x, p.y + y, z), planks);
            }
        }
        for (a, open) in [(-4, false), (1, true)] {
            for (k, x) in [a, a + 1].into_iter().enumerate() {
                let pos = IVec3::new(p.x + x, p.y, z);
                // Clicked on the right half of the floor each time.
                let at = pos.as_vec3() + Vec3::new(0.8, 0.0, 0.5);
                let v = if k == 0 { 0 } else { self.door_hinge(pos, 0, at) };
                self.world.set_block(pos, self.reg.variant(door, v));
                self.world.set_block(pos + IVec3::Y, self.reg.variant(door, v | 8));
                self.pair_doors(pos);
            }
            if open {
                let pos = IVec3::new(p.x + a, p.y, z);
                self.set_door(pos, |v| v ^ 4);
                self.door_follow(pos + IVec3::Y);
            }
        }
        self.world.swings.clear();
        let side = std::env::var("MC_DOORS").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
        self.body.pos = Vec3::new(p.x as f32 + 0.5 + side, p.y as f32, z as f32 + 4.5);
        self.body.flying = true;
    }

    /// Change both halves of the door whose lower half is at `lower`.
    fn set_door(&mut self, lower: IVec3, change: impl Fn(u8) -> u8) {
        for p in [lower, lower + IVec3::Y] {
            let id = self.world.block_or_air(p);
            let b = self.reg.get(id);
            if id != AIR && b.shape == Shape::Door {
                let new = self.reg.variant(b.base, change(b.variant));
                self.world.set_block(p, new);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn reg() -> BlockRegistry {
        BlockRegistry::from_ron(include_str!("../../../packs/vanilla/blocks.ron")).unwrap()
    }

    struct W<'a> {
        reg: &'a BlockRegistry,
        m: HashMap<IVec3, BlockId>,
    }

    impl W<'_> {
        fn door(&mut self, p: IVec3, v: u8) {
            let base = self.reg.id("oak_door").unwrap();
            self.m.insert(p, self.reg.variant(base, v));
            self.m.insert(p + IVec3::Y, self.reg.variant(base, v | 8));
        }
        fn get(&self) -> impl Fn(IVec3) -> BlockId + '_ {
            |p| self.m.get(&p).copied().unwrap_or(AIR)
        }
        fn variant(&self, p: IVec3) -> u8 {
            self.reg.get(self.get()(p)).variant
        }
    }

    #[test]
    fn variants_round_trip() {
        let reg = reg();
        let base = reg.id("oak_door").unwrap();
        for v in 0..32u8 {
            assert_eq!(reg.get(reg.variant(base, v)).variant, v);
        }
        assert_eq!(reg.get(reg.id("oak_door:26").unwrap()).variant, 26);
    }

    #[test]
    fn second_door_mirrors_the_first() {
        let reg = reg();
        let mut w = W { reg: &reg, m: HashMap::new() };
        // Facing north: right is +x. A door hinged on the left at x = 0.
        let a = IVec3::ZERO;
        w.door(a, DOOR_HINGE_LEFT);
        let b = IVec3::X;
        // Clicked on its right half, but the door on its left wins: hinge right.
        assert!(!hinge_left(&reg, w.get(), b, 0, Vec3::new(1.9, 0.5, 0.5)));
        w.door(b, 0);
        assert_eq!(partner(&reg, w.get(), a), Some(b));
        assert_eq!(partner(&reg, w.get(), b), Some(a));
        assert_eq!(turn_neighbour(&reg, w.get(), b), None);
        // And the other way round: a door on the right gives a left hinge.
        assert!(hinge_left(&reg, w.get(), IVec3::new(-1, 0, 0), 0, Vec3::new(-0.1, 0.5, 0.5)));
    }

    #[test]
    fn neighbour_hinged_toward_turns_round() {
        let reg = reg();
        let mut w = W { reg: &reg, m: HashMap::new() };
        // Both hinged on the right (how every door used to go down): the left one turns.
        w.door(IVec3::ZERO, 0);
        w.door(IVec3::X, 0);
        assert_eq!(turn_neighbour(&reg, w.get(), IVec3::X), Some(IVec3::ZERO));
        assert_eq!(partner(&reg, w.get(), IVec3::X), None);
        w.door(IVec3::ZERO, DOOR_HINGE_LEFT);
        assert_eq!(partner(&reg, w.get(), IVec3::X), Some(IVec3::ZERO));
        assert_eq!(w.variant(IVec3::Y) & DOOR_HINGE_LEFT, DOOR_HINGE_LEFT);
    }

    #[test]
    fn hinge_follows_wall_then_click() {
        let reg = reg();
        let mut w = W { reg: &reg, m: HashMap::new() };
        let stone = reg.id("stone").unwrap();
        // Facing east: right is +z, left is -z. Clicked on the left half.
        let at = Vec3::new(0.5, 0.5, 0.2);
        assert!(hinge_left(&reg, w.get(), IVec3::ZERO, 1, at));
        assert!(!hinge_left(&reg, w.get(), IVec3::ZERO, 1, Vec3::new(0.5, 0.5, 0.8)));
        // A wall on the right pulls the hinge to it.
        w.m.insert(IVec3::Z, stone);
        w.m.insert(IVec3::Z + IVec3::Y, stone);
        assert!(!hinge_left(&reg, w.get(), IVec3::ZERO, 1, at));
    }

    #[test]
    fn different_facings_do_not_pair() {
        let reg = reg();
        let mut w = W { reg: &reg, m: HashMap::new() };
        w.door(IVec3::ZERO, 1 | DOOR_HINGE_LEFT);
        w.door(IVec3::X, 0);
        assert_eq!(partner(&reg, w.get(), IVec3::X), None);
    }
}
