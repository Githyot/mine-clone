//! Botany pots (Botany Pots idea): a seed, root or sapling set into a pot keeps growing on its
//! own, and every so often its harvest drops into the hopper or chest under the pot.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::{BlockId, BlockRegistry, AIR};

use crate::audio::{Material, Sfx};
use crate::survival::Container;
use crate::Game;

#[derive(Clone, Copy)]
enum Yield {
    /// The drops of this ripe crop block.
    Crop(BlockId),
    /// A tree: its log, now and then a sapling (and apples on oaks).
    Tree(&'static str),
}

pub struct Pots {
    empty: BlockId,
    /// What each block id yields, when it's a planted pot.
    yields: Vec<Option<Yield>>,
    /// Planted pot for each block a held item would place (crop first stage or sapling).
    planted: Vec<BlockId>,
}

const TREES: [&str; 7] = ["oak", "birch", "spruce", "jungle", "acacia", "dark_oak", "cherry"];

impl Pots {
    pub fn new(reg: &BlockRegistry) -> Pots {
        let n = reg.len();
        let mut p = Pots { empty: reg.id_or_air("botany_pot"), yields: vec![None; n], planted: vec![AIR; n] };
        for (kind, stages) in [("wheat", 8), ("carrots", 4), ("potatoes", 4), ("beetroots", 4)] {
            let (Some(pot), Some(seed), Some(ripe)) = (reg.id(&format!("botany_pot_{kind}")), reg.id(&format!("{kind}_stage0")), reg.id(&format!("{kind}_stage{}", stages - 1))) else { continue };
            p.yields[pot as usize] = Some(Yield::Crop(ripe));
            p.planted[seed as usize] = pot;
        }
        for t in TREES {
            let (Some(pot), Some(sapling)) = (reg.id(&format!("botany_pot_{t}")), reg.id(&format!("{t}_sapling"))) else { continue };
            p.yields[pot as usize] = Some(Yield::Tree(t));
            p.planted[sapling as usize] = pot;
        }
        p
    }

    pub fn is_planted(&self, id: BlockId) -> bool {
        self.yields.get(id as usize).is_some_and(|y| y.is_some())
    }
}

impl Game {
    /// Right-click an empty pot with something that grows: plant it.
    pub(crate) fn pot_plant(&mut self, places: Option<BlockId>) -> bool {
        let (Some(hit), Some(b)) = (self.target, places) else { return false };
        if self.pots.empty == AIR || self.world.block_or_air(hit.block) != self.pots.empty {
            return false;
        }
        let pot = self.pots.planted.get(b as usize).copied().unwrap_or(AIR);
        if pot == AIR {
            return false;
        }
        self.world.set_block(hit.block, pot);
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.anim.start_swing();
        self.sfx(Sfx::Place(Material::Grass));
        if self.world.block_or_air(hit.block - IVec3::Y) == AIR || !self.surv.containers.contains_key(&(hit.block - IVec3::Y)) {
            self.toast("Ponha um funil ou baú embaixo do vaso para receber a colheita", 4.0);
        }
        true
    }

    /// A random tick on a planted pot: sometimes a harvest, into the container below.
    pub(crate) fn pot_tick(&mut self, p: IVec3, id: BlockId, r: u32) {
        let Some(y) = self.pots.yields[id as usize] else { return };
        if r % 3 != 0 || !matches!(self.surv.containers.get(&(p - IVec3::Y)), Some(Container::Chest(_))) {
            return;
        }
        let rng = &mut self.surv.rng;
        let drops: Vec<ItemStack> = match y {
            Yield::Crop(ripe) => self.items.drops(&self.reg, ripe, None, rng),
            Yield::Tree(t) => {
                let mut v = Vec::new();
                let it = |n: String| self.items.id(&n);
                if let Some(log) = it(format!("{t}_log")) {
                    v.push(ItemStack::new(log, 1 + (rng.f32() < 0.4) as u8));
                }
                if rng.f32() < 0.1 {
                    if let Some(s) = it(format!("{t}_sapling")) {
                        v.push(ItemStack::new(s, 1));
                    }
                }
                if matches!(t, "oak" | "dark_oak") && rng.f32() < 0.12 {
                    if let Some(a) = it("apple".into()) {
                        v.push(ItemStack::new(a, 1));
                    }
                }
                if rng.f32() < 0.2 {
                    if let Some(s) = it("stick".into()) {
                        v.push(ItemStack::new(s, 1 + (rng.f32() < 0.5) as u8));
                    }
                }
                v
            }
        };
        let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&(p - IVec3::Y)) else { return };
        let full = drops.into_iter().filter_map(|d| inv.add(&self.items, d)).count() > 0;
        if !full && self.body.pos.distance(p.as_vec3()) < 24.0 {
            let glow = self.glow_layer();
            self.particles.glow(p.as_vec3() + Vec3::new(0.5, 0.6, 0.5), Vec3::Y * 0.8, glow, [0.6, 0.95, 0.4], 0.06, 0.8);
        }
    }

    /// Test scene: a row of planted pots on hoppers over a chest.
    pub(crate) fn test_pots(&mut self) {
        let f = self.forward();
        let c = (self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 4.0).floor().as_ivec3();
        let hopper = self.reg.id_or_air("hopper");
        let names = ["botany_pot_wheat", "botany_pot_carrots", "botany_pot_oak", "botany_pot_cherry", "botany_pot", "botany_pot_beetroots"];
        for (k, n) in names.iter().enumerate() {
            let p = c + IVec3::new(k as i32 - 2, 1, 0);
            for dy in -1..4 {
                self.world.set_block(p + IVec3::Y * dy, AIR);
            }
            self.world.set_block(p, hopper);
            self.surv.containers.insert(p, Container::Chest(voxel::item::Inventory::new(5)));
            self.world.set_block(p + IVec3::Y, self.reg.id_or_air(n));
        }
    }
}
