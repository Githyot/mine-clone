//! Snow! Real Magic! idea, on top of the snowfall in `main.rs` (`settle_snow`):
//!
//! * snow settles on slabs, stairs, fences and walls, which turn into their snowy twins
//!   (`snowy_oak_slab`...) keeping their orientation;
//! * plants get buried under a layer that remembers them (`snowy_poppy`...), and come back when
//!   it melts;
//! * in a snowstorm the layers keep piling up to a full block, not just half;
//! * when the weather warms up the snowy twins and buried plants thaw back;
//! * a thin layer can be built into straight away, a shovel scrapes a layer off, and landing
//!   in deep snow softens a fall.

use glam::IVec3;
use voxel::block::{Shape, ToolKind};
use voxel::item::ItemStack;
use voxel::{BlockId, AIR};

use crate::Game;

#[derive(Default)]
pub struct Snow {
    /// Base block -> its snowy twin, and back (built on first use).
    to_snowy: Vec<BlockId>,
    from_snowy: Vec<BlockId>,
    ready: bool,
    t: f32,
}

impl Game {
    fn snow_tables(&mut self) {
        if self.mods.snow.ready {
            return;
        }
        let n = self.reg.len();
        let mut to = vec![AIR; n];
        let mut from = vec![AIR; n];
        for id in 0..n as BlockId {
            let b = self.reg.get(id);
            if b.variant != 0 || !matches!(b.shape, Shape::Cross | Shape::Slab | Shape::Stairs | Shape::Fence | Shape::Wall) {
                continue;
            }
            if let Some(s) = self.reg.id(&format!("snowy_{}", b.name)) {
                to[id as usize] = s;
                from[s as usize] = id;
            }
        }
        self.mods.snow.to_snowy = to;
        self.mods.snow.from_snowy = from;
        self.mods.snow.ready = true;
    }

    /// The snowy twin of a block (keeping its orientation), if it has one.
    fn snowy_of(&self, id: BlockId) -> Option<BlockId> {
        let b = self.reg.get(id);
        let s = *self.mods.snow.to_snowy.get(b.base as usize)?;
        (s != AIR).then(|| self.reg.variant(s, b.variant))
    }

    fn thawed_of(&self, id: BlockId) -> Option<BlockId> {
        let b = self.reg.get(id);
        let s = *self.mods.snow.from_snowy.get(b.base as usize)?;
        (s != AIR).then(|| if self.reg.get(s).shape == Shape::Cross { s } else { self.reg.variant(s, b.variant) })
    }

    /// Called by the snowfall for each spot it picks (`p` is the top block of the column).
    /// True when it was handled here.
    pub(crate) fn snow_settle_extra(&mut self, p: IVec3) -> bool {
        self.snow_tables();
        let layer = self.reg.id_or_air("snow_layer");
        for q in [p + IVec3::Y, p] {
            let id = self.world.block_or_air(q);
            if id == AIR {
                continue;
            }
            let b = self.reg.get(id);
            // A plant: buried under a layer that remembers it.
            if b.shape == Shape::Cross {
                if let Some(s) = self.snowy_of(id) {
                    self.world.set_block(q, s);
                    return true;
                }
                continue;
            }
            // A slab, stair, fence or wall turns snowy.
            if matches!(b.shape, Shape::Slab | Shape::Stairs | Shape::Fence | Shape::Wall) && self.world.block_or_air(q + IVec3::Y) == AIR {
                if let Some(s) = self.snowy_of(id) {
                    if !(b.shape == Shape::Slab && b.variant == 2) {
                        self.world.set_block(q, s);
                        return true;
                    }
                }
            }
            // In a storm a layer (plain or over a buried plant) grows past half a block.
            if b.shape == Shape::Layer && (b.base == layer || self.thawed_of(id).is_some()) {
                let storm = self.weather.state.thunder;
                let cap = if storm { 7 } else { 3 };
                if b.base != layer && b.variant < cap && self.surv.rng.chance(0.2) {
                    self.world.set_block(q, self.reg.variant(id, b.variant + 1));
                    return true;
                }
                if storm && b.variant >= 3 && b.variant < 7 && self.surv.rng.chance(0.15) {
                    self.world.set_block(q, self.reg.variant(id, b.variant + 1));
                    return true;
                }
                return b.base != layer;
            }
        }
        false
    }

    /// Thawing: around the player, snowy blocks and buried plants come back when it is warm.
    pub(crate) fn update_snow(&mut self, dt: f32) {
        self.mods.snow.t += dt * 30.0;
        let n = self.mods.snow.t as i32;
        self.mods.snow.t -= n as f32;
        if n == 0 {
            return;
        }
        self.snow_tables();
        let me = self.body.pos.floor().as_ivec3();
        for _ in 0..n.min(30) {
            let x = me.x + self.surv.rng.range(-32, 33);
            let z = me.z + self.surv.rng.range(-32, 33);
            let biome = self.world.gen.base_biome_at(x, z);
            if self.weather.cold(biome) {
                continue;
            }
            let Some(y) = self.world.surface_y(x, z) else { continue };
            for q in [IVec3::new(x, y, z), IVec3::new(x, y + 1, z)] {
                let id = self.world.block_or_air(q);
                let Some(back) = self.thawed_of(id) else { continue };
                let b = self.reg.get(id);
                if b.shape == Shape::Layer && b.variant > 0 {
                    self.world.set_block(q, self.reg.variant(id, b.variant - 1));
                } else {
                    self.world.set_block(q, back);
                }
            }
        }
    }

    /// A thin snow layer (one deep) gives way to a block placed into it.
    pub(crate) fn snow_replaceable(&self, id: BlockId) -> bool {
        let b = self.reg.get(id);
        b.shape == Shape::Layer && b.variant == 0 && (b.name == "snow_layer" || b.name.starts_with("snowy_"))
    }

    /// Right click with a shovel on snow: scrape one layer off (and keep it).
    pub(crate) fn snow_use(&mut self) -> bool {
        let Some(t) = self.held_tool() else { return false };
        if t.kind != ToolKind::Shovel {
            return false;
        }
        let Some(hit) = self.target else { return false };
        self.snow_tables();
        let id = self.world.block_or_air(hit.block);
        let b = self.reg.get(id);
        let plain = b.name == "snow_layer" || b.base == self.reg.id_or_air("snow_layer");
        let new = if b.shape == Shape::Layer && (plain || self.thawed_of(id).is_some()) {
            if b.variant > 0 {
                self.reg.variant(id, b.variant - 1)
            } else if plain {
                AIR
            } else {
                self.thawed_of(id).unwrap_or(AIR)
            }
        } else if let Some(back) = self.thawed_of(id) {
            back
        } else {
            return false;
        };
        self.world.set_block(hit.block, new);
        if self.survival {
            if let Some(i) = self.items.id("snow_layer") {
                if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(i, 1)) {
                    self.pop_drop(hit.block.as_vec3() + glam::Vec3::splat(0.5), left);
                }
            }
            self.wear_held(1);
        }
        let (layer, tint) = self.block_look(id);
        self.particles.block_bits(hit.block, layer, tint, 5, 0.4);
        self.sfx(crate::audio::Sfx::Break(crate::audio::Material::Snow));
        self.anim.start_swing();
        true
    }

    /// How much of a fall's damage is left after landing on snow (deep snow catches you).
    pub(crate) fn snow_fall_factor(&self) -> f32 {
        let feet = self.body.pos.floor().as_ivec3();
        for q in [feet, feet - IVec3::Y] {
            let b = self.reg.get(self.world.block_or_air(q));
            if b.shape == Shape::Layer && (b.name.starts_with("snow_layer") || b.name.starts_with("snowy_")) {
                return 1.0 - (b.variant as f32 + 1.0) * 0.1;
            }
            if b.name == "snow" || b.name == "snow_block" {
                return 0.2;
            }
        }
        1.0
    }
}
