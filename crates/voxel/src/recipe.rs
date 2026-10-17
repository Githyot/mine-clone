//! Crafting and smelting recipes from `recipes.ron`. Ingredients are item names, or `#tag` for a
//! group listed under `tags` (any plank, any log...).

use serde::Deserialize;
use std::collections::HashMap;

use crate::item::{ItemId, ItemRegistry, ItemStack};

#[derive(Deserialize)]
struct ShapedDef {
    pattern: Vec<String>,
    key: HashMap<char, String>,
    result: String,
    #[serde(default = "one")]
    count: u8,
}

#[derive(Deserialize)]
struct ShapelessDef {
    ingredients: Vec<String>,
    result: String,
    #[serde(default = "one")]
    count: u8,
}

#[derive(Deserialize)]
struct SmeltDef {
    input: String,
    result: String,
    #[serde(default)]
    xp: f32,
}

fn one() -> u8 {
    1
}

#[derive(Deserialize)]
struct RecipeFile {
    #[serde(default)]
    tags: HashMap<String, Vec<String>>,
    #[serde(default)]
    shaped: Vec<ShapedDef>,
    #[serde(default)]
    shapeless: Vec<ShapelessDef>,
    #[serde(default)]
    smelting: Vec<SmeltDef>,
}

/// One ingredient slot: any of these items.
pub type Ingredient = Vec<ItemId>;

pub struct Shaped {
    pub w: usize,
    pub h: usize,
    /// Row-major, None = must be empty.
    pub cells: Vec<Option<Ingredient>>,
    pub result: ItemStack,
}

pub struct Shapeless {
    pub ingredients: Vec<Ingredient>,
    pub result: ItemStack,
}

pub struct Smelt {
    pub input: Ingredient,
    pub result: ItemStack,
    pub xp: f32,
}

pub struct Recipes {
    pub shaped: Vec<Shaped>,
    pub shapeless: Vec<Shapeless>,
    pub smelting: Vec<Smelt>,
}

impl Recipes {
    pub fn from_ron(text: &str, items: &ItemRegistry) -> Result<Recipes, String> {
        Self::from_ron_many(&[text], items)
    }

    /// Several recipe files read as one (the base game, then mod packs); tags merge by name.
    pub fn from_ron_many(texts: &[&str], items: &ItemRegistry) -> Result<Recipes, String> {
        let mut f = RecipeFile { tags: HashMap::new(), shaped: Vec::new(), shapeless: Vec::new(), smelting: Vec::new() };
        for text in texts.iter().filter(|t| !t.trim().is_empty()) {
            let g: RecipeFile = ron::from_str(text).map_err(|e| format!("recipes.ron: {e}"))?;
            for (k, v) in g.tags {
                f.tags.entry(k).or_default().extend(v);
            }
            f.shaped.extend(g.shaped);
            f.shapeless.extend(g.shapeless);
            f.smelting.extend(g.smelting);
        }
        let ing = |name: &str| -> Result<Ingredient, String> {
            if let Some(tag) = name.strip_prefix('#') {
                let list = f.tags.get(tag).ok_or_else(|| format!("unknown tag #{tag}"))?;
                let v: Vec<ItemId> = list.iter().filter_map(|n| items.id(n)).collect();
                if v.is_empty() {
                    return Err(format!("tag #{tag} has no known items"));
                }
                Ok(v)
            } else {
                Ok(vec![items.id(name).ok_or_else(|| format!("unknown item '{name}'"))?])
            }
        };
        let result = |name: &str, count: u8| -> Result<ItemStack, String> {
            Ok(ItemStack::new(items.id(name).ok_or_else(|| format!("unknown result '{name}'"))?, count))
        };
        let mut r = Recipes { shaped: Vec::new(), shapeless: Vec::new(), smelting: Vec::new() };
        for d in &f.shaped {
            let h = d.pattern.len();
            let w = d.pattern.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let mut cells = Vec::with_capacity(w * h);
            for row in &d.pattern {
                let chars: Vec<char> = row.chars().collect();
                for x in 0..w {
                    match chars.get(x) {
                        None | Some(' ') => cells.push(None),
                        Some(c) => cells.push(Some(ing(d.key.get(c).ok_or_else(|| format!("'{c}' not in key for {}", d.result))?)?)),
                    }
                }
            }
            r.shaped.push(Shaped { w, h, cells, result: result(&d.result, d.count)? });
        }
        for d in &f.shapeless {
            let ingredients = d.ingredients.iter().map(|n| ing(n)).collect::<Result<Vec<_>, _>>()?;
            r.shapeless.push(Shapeless { ingredients, result: result(&d.result, d.count)? });
        }
        for d in &f.smelting {
            r.smelting.push(Smelt { input: ing(&d.input)?, result: result(&d.result, 1)?, xp: d.xp });
        }
        Ok(r)
    }

    /// The result for a square crafting grid (`n` × `n`, row-major).
    pub fn craft(&self, grid: &[Option<ItemStack>], n: usize) -> Option<ItemStack> {
        let filled: Vec<(usize, usize)> = (0..n * n).filter(|&i| grid[i].is_some()).map(|i| (i % n, i / n)).collect();
        if filled.is_empty() {
            return None;
        }
        let x0 = filled.iter().map(|p| p.0).min().unwrap();
        let y0 = filled.iter().map(|p| p.1).min().unwrap();
        let x1 = filled.iter().map(|p| p.0).max().unwrap();
        let y1 = filled.iter().map(|p| p.1).max().unwrap();
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        let at = |x: usize, y: usize| grid[(y0 + y) * n + x0 + x].map(|s| s.item);
        for r in &self.shaped {
            if r.w != w || r.h != h {
                continue;
            }
            for mirror in [false, true] {
                let ok = (0..h).all(|y| {
                    (0..w).all(|x| {
                        let rx = if mirror { w - 1 - x } else { x };
                        match (&r.cells[y * w + rx], at(x, y)) {
                            (None, None) => true,
                            (Some(ing), Some(it)) => ing.contains(&it),
                            _ => false,
                        }
                    })
                });
                if ok {
                    return Some(r.result);
                }
            }
        }
        let mut have: Vec<ItemId> = grid.iter().take(n * n).flatten().map(|s| s.item).collect();
        'recipes: for r in &self.shapeless {
            if r.ingredients.len() != have.len() {
                continue;
            }
            let mut left = have.clone();
            for ing in &r.ingredients {
                match left.iter().position(|it| ing.contains(it)) {
                    Some(i) => {
                        left.swap_remove(i);
                    }
                    None => continue 'recipes,
                }
            }
            return Some(r.result);
        }
        have.clear();
        None
    }

    pub fn smelt(&self, item: ItemId) -> Option<&Smelt> {
        self.smelting.iter().find(|s| s.input.contains(&item))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockRegistry;

    /// The real game files load together, and armor can be made.
    #[test]
    fn vanilla_pack_loads() {
        let blocks = BlockRegistry::from_ron(include_str!("../../../packs/vanilla/blocks.ron")).unwrap();
        let items = ItemRegistry::from_ron(include_str!("../../../packs/vanilla/items.ron"), &blocks).unwrap();
        let r = Recipes::from_ron(include_str!("../../../packs/vanilla/recipes.ron"), &items).unwrap();
        let ingot = Some(ItemStack::new(items.id("iron_ingot").unwrap(), 1));
        let grid = [ingot, ingot, ingot, ingot, None, ingot, None, None, None];
        let out = r.craft(&grid, 3).unwrap();
        assert_eq!(items.get(out.item).name, "iron_helmet");
        assert_eq!(items.get(out.item).armor.unwrap().slot, 0);
        assert!(r.smelt(items.id("raw_iron").unwrap()).is_some());
    }

    #[test]
    fn planks_sticks_and_a_mirrored_axe() {
        let blocks = BlockRegistry::from_ron(r#"(blocks: [(name: "oak_log"), (name: "oak_planks"), (name: "birch_planks")])"#).unwrap();
        let items = ItemRegistry::from_ron(r#"(items: [(name: "stick"), (name: "wooden_axe")])"#, &blocks).unwrap();
        let rec = Recipes::from_ron(
            r###"(
                tags: {"planks": ["oak_planks", "birch_planks"]},
                shaped: [
                    (pattern: ["#", "#"], key: {'#': "#planks"}, result: "stick", count: 4),
                    (pattern: ["##", "#/", " /"], key: {'#': "#planks", '/': "stick"}, result: "wooden_axe"),
                ],
                shapeless: [(ingredients: ["oak_log"], result: "oak_planks", count: 4)],
            )"###,
            &items,
        )
        .unwrap();
        let s = |n: &str| Some(ItemStack::new(items.id(n).unwrap(), 1));
        // A log anywhere in the 2x2 grid gives 4 planks.
        let grid = [None, None, None, s("oak_log")];
        let r = rec.craft(&grid, 2).unwrap();
        assert_eq!((items.get(r.item).name.as_str(), r.count), ("oak_planks", 4));
        // Two planks stacked, mixing kinds, in the right column of a 3x3 grid.
        let grid = [None, None, s("oak_planks"), None, None, s("birch_planks"), None, None, None];
        assert_eq!(items.get(rec.craft(&grid, 3).unwrap().item).name, "stick");
        // The axe, mirrored.
        let grid = [s("oak_planks"), s("oak_planks"), None, s("stick"), s("oak_planks"), None, s("stick"), None, None];
        assert_eq!(items.get(rec.craft(&grid, 3).unwrap().item).name, "wooden_axe");
        assert!(rec.craft(&[s("stick"), None, None, None], 2).is_none());
    }
}
