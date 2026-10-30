//! Bees (vanilla idea, our own bees): wild nests hang on trees in open country and bees
//! buzz about near them. Nests fill with honey by themselves, hives you build fill when bees
//! are near. Take the honey with a glass bottle, or break the comb out by hand for honeycomb;
//! without a campfire under it the bees take that badly.

use glam::{IVec3, Vec3};
use voxel::block::BlockId;
use voxel::item::ItemStack;

use crate::audio::{Material, Sfx};
use crate::Game;

impl Game {
    fn bees_near(&self, p: Vec3, r: f32) -> usize {
        self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == "bee" && m.body.pos.distance(p) < r).count()
    }

    /// A random tick on a nest or hive: it fills with honey; a wild nest also sends out bees.
    pub(crate) fn hive_tick(&mut self, p: IVec3, id: BlockId, r: u32) {
        let name = self.reg.get(id).name.clone();
        let c = p.as_vec3() + Vec3::splat(0.5);
        let bees = self.bees_near(c, 16.0);
        if name == "bee_nest" && bees < 2 && r % 3 == 0 {
            let out = c + Vec3::new(0.0, 0.0, 1.2);
            self.mobs.spawn("bee", out);
        }
        let fill = match name.as_str() {
            "bee_nest" => r % 6 == 0,
            "beehive" => bees > 0 && r % 4 == 0,
            _ => false,
        };
        if fill {
            // Facing the same way.
            let full = self.reg.variant(self.reg.id_or_air(&format!("{name}_honey")), self.reg.get(id).variant);
            self.world.set_block(p, full);
        }
    }

    /// Right click on a full nest or hive: honey into a bottle, or the comb out by hand.
    pub(crate) fn hive_use(&mut self, p: IVec3, name: &str) -> bool {
        let Some(empty) = name.strip_suffix("_honey") else { return false };
        let at = p.as_vec3() + Vec3::new(0.5, 1.1, 0.5);
        let bottle = self.items.id("glass_bottle").filter(|&b| self.held_stack().is_some_and(|s| s.item == b));
        if let (Some(_), Some(honey)) = (bottle, self.items.id("honey_bottle")) {
            if self.survival {
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
            }
            self.pop_drop(at, ItemStack::new(honey, 1));
        } else if let Some(comb) = self.items.id("honeycomb") {
            self.pop_drop(at, ItemStack::new(comb, 3));
        }
        let v = self.reg.get(self.world.block_or_air(p)).variant;
        let empty = self.reg.variant(self.reg.id_or_air(empty), v);
        self.world.set_block(p, empty);
        self.anim.start_swing();
        self.sfx(Sfx::Break(Material::Grass));
        // Smoke from a campfire below keeps the bees calm.
        let calm = (1..=5).any(|d| self.reg.get(self.world.block_or_air(p - IVec3::Y * d)).name.contains("campfire"));
        if !calm {
            let c = p.as_vec3();
            for m in self.mobs.list.iter_mut() {
                if m.alive() && self.mobs.reg.defs[m.def].name == "bee" && m.body.pos.distance(c) < 12.0 {
                    m.aggro = true;
                }
            }
        }
        true
    }
}
