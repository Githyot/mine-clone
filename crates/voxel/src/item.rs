//! Items: every block is also an item, and `items.ron` adds the rest (tools, food, materials).
//! Stacks, inventories and block drops live here so they can be tested without a window.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::block::{BlockId, BlockRegistry, Shape, ToolKind, AIR};
use crate::rng::Rng;

pub type ItemId = u16;

#[derive(Deserialize, Clone, Debug)]
pub struct ToolDef {
    pub kind: ToolKind,
    /// 1 wood, 2 stone, 3 iron, 4 diamond (gold is 1 but very fast).
    pub tier: u8,
    /// Digging speed multiplier on blocks of its kind.
    pub speed: f32,
    /// Melee damage (hand is 1).
    pub damage: f32,
    pub durability: u16,
}

#[derive(Deserialize, Clone, Copy, Debug)]
pub struct FoodDef {
    /// Hunger points restored (a full bar is 20).
    pub hunger: u8,
    pub saturation: f32,
}

/// An item as written in `items.ron`. An entry named like a block adds to that block's item.
#[derive(Deserialize, Clone, Debug)]
pub struct ItemDef {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    /// Texture in `textures/item/`; defaults to the name.
    #[serde(default)]
    pub texture: Option<String>,
    #[serde(default)]
    pub max_stack: Option<u8>,
    #[serde(default)]
    pub tool: Option<ToolDef>,
    #[serde(default)]
    pub food: Option<FoodDef>,
    /// Block placed when used on a block.
    #[serde(default)]
    pub places: Option<String>,
    /// Seconds of furnace fuel.
    #[serde(default)]
    pub fuel: f32,
}

#[derive(Deserialize)]
struct ItemFile {
    items: Vec<ItemDef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// Drawn like the block (a little cube, or a flat sprite for plants).
    Block(BlockId),
    /// A flat sprite: texture layer, counted from the start of the item textures.
    Sprite(u16),
}

#[derive(Clone, Debug)]
pub struct Item {
    pub name: String,
    pub display_name: String,
    pub look: Look,
    pub max_stack: u8,
    pub tool: Option<ToolDef>,
    pub food: Option<FoodDef>,
    pub places: Option<BlockId>,
    pub fuel: f32,
}

pub struct ItemRegistry {
    pub items: Vec<Item>,
    by_name: HashMap<String, ItemId>,
    /// Item of each block id (None for air, liquids, ...).
    block_item: Vec<Option<ItemId>>,
    /// Texture names for `Look::Sprite` layers, in order ("item/<name>").
    pub textures: Vec<String>,
}

impl ItemRegistry {
    pub fn from_ron(text: &str, blocks: &BlockRegistry) -> Result<Self, String> {
        let file: ItemFile = ron::from_str(text).map_err(|e| format!("items.ron: {e}"))?;
        Self::from_defs(file.items, blocks)
    }

    pub fn from_defs(defs: Vec<ItemDef>, blocks: &BlockRegistry) -> Result<Self, String> {
        let mut reg = ItemRegistry { items: Vec::new(), by_name: HashMap::new(), block_item: vec![None; blocks.len()], textures: Vec::new() };
        // Item 0 is unused so a zero id never looks like a real item.
        reg.items.push(Item { name: "air".into(), display_name: String::new(), look: Look::Block(AIR), max_stack: 0, tool: None, food: None, places: None, fuel: 0.0 });
        for (i, b) in blocks.blocks.iter().enumerate() {
            if i as BlockId == AIR || matches!(b.shape, Shape::Liquid | Shape::Invisible) {
                continue;
            }
            let id = reg.items.len() as ItemId;
            reg.items.push(Item {
                name: b.name.clone(),
                display_name: b.display_name.clone(),
                look: Look::Block(i as BlockId),
                max_stack: 64,
                tool: None,
                food: None,
                places: Some(i as BlockId),
                fuel: 0.0,
            });
            reg.by_name.insert(b.name.clone(), id);
            reg.block_item[i] = Some(id);
        }
        let mut tex_index: HashMap<String, u16> = HashMap::new();
        for d in defs {
            let existing = reg.by_name.get(&d.name).copied();
            let places = match &d.places {
                Some(n) => Some(blocks.id(n).ok_or_else(|| format!("item '{}' places unknown block '{n}'", d.name))?),
                None => None,
            };
            if let Some(id) = existing {
                // Extra facts for a block's item.
                let it = &mut reg.items[id as usize];
                if let Some(n) = d.display_name {
                    it.display_name = n;
                }
                if let Some(m) = d.max_stack {
                    it.max_stack = m;
                }
                it.fuel = d.fuel.max(it.fuel);
                it.food = d.food.or(it.food);
                if let Some(t) = d.texture {
                    let layer = *tex_index.entry(t.clone()).or_insert_with(|| {
                        reg.textures.push(format!("item/{t}"));
                        (reg.textures.len() - 1) as u16
                    });
                    it.look = Look::Sprite(layer);
                }
                continue;
            }
            let tex = d.texture.clone().unwrap_or_else(|| d.name.clone());
            let layer = *tex_index.entry(tex.clone()).or_insert_with(|| {
                reg.textures.push(format!("item/{tex}"));
                (reg.textures.len() - 1) as u16
            });
            let max_stack = d.max_stack.unwrap_or(if d.tool.is_some() { 1 } else if d.food.is_some() { 64 } else { 64 });
            let id = reg.items.len() as ItemId;
            reg.by_name.insert(d.name.clone(), id);
            reg.items.push(Item {
                display_name: d.display_name.unwrap_or_else(|| d.name.replace('_', " ")),
                name: d.name,
                look: Look::Sprite(layer),
                max_stack: max_stack.max(1),
                tool: d.tool,
                food: d.food,
                places,
                fuel: d.fuel,
            });
        }
        Ok(reg)
    }

    #[inline]
    pub fn get(&self, id: ItemId) -> &Item {
        &self.items[id as usize]
    }

    pub fn id(&self, name: &str) -> Option<ItemId> {
        self.by_name.get(name).copied()
    }

    pub fn of_block(&self, b: BlockId) -> Option<ItemId> {
        self.block_item.get(b as usize).copied().flatten()
    }

    /// What breaking `block` with `tool` gives. Nothing when the block needs a better tool.
    pub fn drops(&self, blocks: &BlockRegistry, block: BlockId, tool: Option<&ToolDef>, rng: &mut Rng) -> Vec<ItemStack> {
        let b = blocks.get(block);
        if !self.can_harvest(blocks, block, tool) {
            return Vec::new();
        }
        let mut out = Vec::new();
        match &b.drops {
            None => {
                if let Some(i) = self.of_block(block) {
                    out.push(ItemStack::new(i, 1));
                }
            }
            Some(list) => {
                for d in list {
                    if rng.f32() >= d.chance {
                        continue;
                    }
                    let Some(i) = self.id(&d.item) else { continue };
                    let n = if d.max > d.min { d.min + (rng.f32() * (d.max - d.min + 1) as f32) as u8 } else { d.min };
                    let n = n.min(d.max.max(d.min));
                    if n > 0 {
                        out.push(ItemStack::new(i, n));
                    }
                }
            }
        }
        out
    }

    pub fn can_harvest(&self, blocks: &BlockRegistry, block: BlockId, tool: Option<&ToolDef>) -> bool {
        let b = blocks.get(block);
        if b.tier == 0 {
            return true;
        }
        match (b.tool, tool) {
            (Some(k), Some(t)) => t.kind == k && t.tier >= b.tier,
            _ => false,
        }
    }

    /// Seconds to dig `block` holding `tool` (classic formula: hardness × 1.5, ×5 without the
    /// right tool for blocks that need one, divided by the tool's speed).
    pub fn dig_time(&self, blocks: &BlockRegistry, block: BlockId, tool: Option<&ToolDef>) -> f32 {
        let b = blocks.get(block);
        if b.hardness < 0.0 {
            return f32::INFINITY;
        }
        if b.hardness == 0.0 {
            return 0.05;
        }
        let mut speed = 1.0;
        if let (Some(k), Some(t)) = (b.tool, tool) {
            if t.kind == k {
                speed = t.speed;
            }
        }
        // Swords cut leaves and webs quickly.
        if let Some(t) = tool {
            if t.kind == ToolKind::Sword && b.name.ends_with("_leaves") {
                speed = 1.5;
            }
        }
        let factor = if self.can_harvest(blocks, block, tool) { 1.5 } else { 5.0 };
        (b.hardness * factor / speed).max(0.05)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: ItemId,
    pub count: u8,
    /// Wear on tools.
    #[serde(default)]
    pub damage: u16,
}

impl ItemStack {
    pub fn new(item: ItemId, count: u8) -> ItemStack {
        ItemStack { item, count, damage: 0 }
    }

    /// Same item, so the two can sit in one stack.
    pub fn stacks_with(&self, o: &ItemStack) -> bool {
        self.item == o.item && self.damage == 0 && o.damage == 0
    }
}

/// A row of slots: the player's 36, a chest's 27, a furnace's 3.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn new(n: usize) -> Inventory {
        Inventory { slots: vec![None; n] }
    }

    /// Put a stack in, filling matching stacks first, then empty slots in `order`.
    /// Returns what didn't fit.
    pub fn add_in(&mut self, reg: &ItemRegistry, mut stack: ItemStack, order: impl Iterator<Item = usize> + Clone) -> Option<ItemStack> {
        let max = reg.get(stack.item).max_stack.max(1);
        for i in order.clone() {
            if let Some(s) = &mut self.slots[i] {
                if s.stacks_with(&stack) && s.count < max {
                    let n = (max - s.count).min(stack.count);
                    s.count += n;
                    stack.count -= n;
                    if stack.count == 0 {
                        return None;
                    }
                }
            }
        }
        for i in order {
            if self.slots[i].is_none() {
                let n = stack.count.min(max);
                self.slots[i] = Some(ItemStack { count: n, ..stack });
                stack.count -= n;
                if stack.count == 0 {
                    return None;
                }
            }
        }
        Some(stack)
    }

    pub fn add(&mut self, reg: &ItemRegistry, stack: ItemStack) -> Option<ItemStack> {
        let n = self.slots.len();
        self.add_in(reg, stack, 0..n)
    }

    /// Take `n` from a slot, emptying it when it runs out.
    pub fn take(&mut self, i: usize, n: u8) -> Option<ItemStack> {
        let s = self.slots[i].as_mut()?;
        let k = n.min(s.count);
        s.count -= k;
        let out = ItemStack { count: k, ..*s };
        if s.count == 0 {
            self.slots[i] = None;
        }
        (k > 0).then_some(out)
    }

    pub fn count_of(&self, item: ItemId) -> u32 {
        self.slots.iter().flatten().filter(|s| s.item == item).map(|s| s.count as u32).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regs() -> (BlockRegistry, ItemRegistry) {
        let blocks = BlockRegistry::from_ron(
            r#"(blocks: [
                (name: "stone", hardness: 1.5, tool: Some(Pickaxe), tier: 1, drops: Some([(item: "cobblestone")])),
                (name: "cobblestone", hardness: 2.0, tool: Some(Pickaxe), tier: 1),
                (name: "dirt", hardness: 0.5, tool: Some(Shovel)),
            ])"#,
        )
        .unwrap();
        let items = ItemRegistry::from_ron(
            r#"(items: [
                (name: "wooden_pickaxe", tool: Some((kind: Pickaxe, tier: 1, speed: 2.0, damage: 2.0, durability: 59))),
                (name: "stick"),
            ])"#,
            &blocks,
        )
        .unwrap();
        (blocks, items)
    }

    #[test]
    fn stone_needs_a_pickaxe() {
        let (blocks, items) = regs();
        let stone = blocks.id("stone").unwrap();
        let mut rng = Rng::new(1);
        assert!(items.drops(&blocks, stone, None, &mut rng).is_empty());
        let pick = items.get(items.id("wooden_pickaxe").unwrap()).tool.clone().unwrap();
        let d = items.drops(&blocks, stone, Some(&pick), &mut rng);
        assert_eq!(d.len(), 1);
        assert_eq!(items.get(d[0].item).name, "cobblestone");
        // The pickaxe digs faster than the bare hand.
        assert!(items.dig_time(&blocks, stone, Some(&pick)) < items.dig_time(&blocks, stone, None) / 4.0);
        let dirt = blocks.id("dirt").unwrap();
        assert_eq!(items.get(items.drops(&blocks, dirt, None, &mut rng)[0].item).name, "dirt");
    }

    #[test]
    fn stacks_fill_then_spill() {
        let (_, items) = regs();
        let stick = items.id("stick").unwrap();
        let mut inv = Inventory::new(2);
        assert!(inv.add(&items, ItemStack::new(stick, 60)).is_none());
        assert!(inv.add(&items, ItemStack::new(stick, 10)).is_none());
        assert_eq!(inv.slots[0].unwrap().count, 64);
        assert_eq!(inv.slots[1].unwrap().count, 6);
        let left = inv.add(&items, ItemStack::new(stick, 100)).unwrap();
        assert_eq!(left.count, 42);
        assert_eq!(inv.count_of(stick), 128);
    }
}
