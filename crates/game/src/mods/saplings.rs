//! Saplings of the mod pack's woods (skyroot, golden oak, echo, ipê, araucária, buriti,
//! jabuticaba, and any wood a pack adds the same way) grow with `voxel::modtrees`. The tree is
//! found from the names alone: `<wood>_sapling` grows `<wood>_leaves` on `<wood>_log`, or on
//! the log of a shorter name (`ipe_pink_sapling` grows on `ipe_log`), in the shape listed
//! below (a round crown for a wood not listed) with `<wood>_cluster` as its fruit, if any.

use glam::IVec3;
use voxel::modtrees::{self, ModTree, Shape};
use voxel::rng::Rng;
use voxel::{BlockId, BlockRegistry, AIR};

use crate::survival::WorldTrees;
use crate::Game;

/// Tree shapes of the known mod woods (by the name before `_sapling`, or its first part).
const SHAPES: &[(&str, Shape)] = &[
    ("golden_oak", Shape::Spread),
    ("echo", Shape::Weeping),
    ("ipe", Shape::Spread),
    ("araucaria", Shape::Umbrella),
    ("buriti", Shape::Palm),
    ("jabuticaba", Shape::TrunkFruit),
    ("coconut", Shape::Palm),
    ("maple", Shape::Spread),
];

/// Fruit that is not `<wood>_cluster`.
const FRUIT: &[(&str, &str)] = &[("echo", "echo_bulb")];

/// The tree a mod sapling grows into, if its wood exists.
pub fn mod_tree(reg: &BlockRegistry, sapling: &str) -> Option<ModTree> {
    let wood = sapling.strip_suffix("_sapling")?;
    let leaves = reg.id(&format!("{wood}_leaves"))?;
    // The log: the wood's own, or that of the name with its last parts cut off.
    let mut stem = wood;
    let log = loop {
        if let Some(id) = reg.id(&format!("{stem}_log")) {
            break id;
        }
        stem = &stem[..stem.rfind('_')?];
    };
    let shape = SHAPES.iter().find(|(n, _)| *n == wood || *n == stem).map(|s| s.1).unwrap_or(Shape::Round);
    let fruit = FRUIT.iter().find(|(n, _)| *n == stem).map(|f| f.1.to_string()).unwrap_or(format!("{stem}_cluster"));
    let fruit: BlockId = reg.id(&fruit).unwrap_or(AIR);
    Some(ModTree { log, leaves, shape, fruit })
}

impl Game {
    /// Grow the mod sapling at `p` (block `id`, called `name`); nothing happens for a block that
    /// is no mod sapling. Waits when there is no room above it.
    pub(crate) fn grow_mod_sapling(&mut self, p: IVec3, id: BlockId, name: &str) {
        let Some(tree) = mod_tree(&self.reg, name) else { return };
        let h = tree.shape.min_height();
        if (1..=h).any(|y| !matches!(self.world.block(p + IVec3::Y * y), Some(AIR))) {
            self.surv.saplings.push((p, 30.0));
            return;
        }
        self.world.set_block(p, AIR);
        let mut rng = Rng::new(self.surv.rng.next_u64());
        let ground = p - IVec3::Y;
        modtrees::grow(tree, &mut WorldTrees(&mut self.world), &mut rng, ground.x, ground.y, ground.z);
        if self.world.block_or_air(p) == AIR {
            // Nothing took its place: give the sapling back.
            self.world.set_block(p, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mod_sapling_has_a_tree() {
        let reg = BlockRegistry::from_ron_many(&[include_str!("../../../../packs/vanilla/blocks.ron"), include_str!("../../../../packs/vanilla/mods/blocks.ron")]).unwrap();
        let mut n = 0;
        for b in &reg.blocks {
            if b.name.ends_with("_sapling") && voxel::trees::TreeKind::from_sapling(&b.name).is_none() {
                let t = mod_tree(&reg, &b.name).unwrap_or_else(|| panic!("{} grows nothing", b.name));
                assert!(t.log != AIR && t.leaves != AIR);
                n += 1;
            }
        }
        assert!(n >= 9, "only {n} mod saplings");
        let ipe = mod_tree(&reg, "ipe_pink_sapling").unwrap();
        assert_eq!((ipe.log, ipe.shape), (reg.id("ipe_log").unwrap(), Shape::Spread));
        assert_eq!(mod_tree(&reg, "jabuticaba_sapling").unwrap().fruit, reg.id("jabuticaba_cluster").unwrap());
        assert_eq!(mod_tree(&reg, "echo_sapling").unwrap().shape, Shape::Weeping);
    }
}
