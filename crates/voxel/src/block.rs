//! Data-driven block registry. Every block (including the base game's) comes from a RON data file,
//! so mods add blocks the same way the base game does.

use serde::Deserialize;
use std::collections::HashMap;

pub type BlockId = u16;
pub const AIR: BlockId = 0;

/// Face order used everywhere: +X, -X, +Y, -Y, +Z, -Z.
pub const FACE_DIRS: [[i32; 3]; 6] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Shape {
    #[default]
    Cube,
    /// Two crossed quads (flowers, grass).
    Cross,
    /// A small 3D stick (2×10×2 pixels) cut from the middle of its texture.
    Torch,
    /// Water and lava: cube with a lowered top, drawn in the translucent pass.
    Liquid,
    /// Nothing to draw.
    Invisible,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Transparency {
    #[default]
    Opaque,
    /// Pixels are either fully visible or fully transparent (leaves, glass, plants).
    Cutout,
    /// Blended (water, ice).
    Translucent,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct TextureSpec {
    pub all: Option<String>,
    pub top: Option<String>,
    pub bottom: Option<String>,
    pub side: Option<String>,
    pub front: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_hardness() -> f32 {
    1.0
}

/// A block as written in the data file.
#[derive(Deserialize, Clone, Debug)]
pub struct BlockDef {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub shape: Shape,
    #[serde(default)]
    pub transparency: Transparency,
    #[serde(default)]
    pub textures: TextureSpec,
    /// Light emitted, 0..15.
    #[serde(default)]
    pub light: u8,
    /// Whether entities collide with it.
    #[serde(default = "default_true")]
    pub solid: bool,
    /// Multiply by the biome color (grass, leaves).
    #[serde(default)]
    pub tint: bool,
    #[serde(default = "default_hardness")]
    pub hardness: f32,
    /// Falls when unsupported (sand, gravel).
    #[serde(default)]
    pub gravity: bool,
    /// Shown in the creative inventory.
    #[serde(default = "default_true")]
    pub in_inventory: bool,
}

#[derive(Deserialize)]
pub struct BlockFile {
    pub blocks: Vec<BlockDef>,
}

/// A registered block with everything resolved for fast lookups in hot loops.
#[derive(Clone, Debug)]
pub struct Block {
    pub name: String,
    pub display_name: String,
    pub shape: Shape,
    pub transparency: Transparency,
    /// Texture layer per face, in FACE_DIRS order.
    pub faces: [u16; 6],
    pub light: u8,
    pub solid: bool,
    pub tint: bool,
    pub hardness: f32,
    pub gravity: bool,
    pub in_inventory: bool,
    /// Fully hides neighbor faces and stops light.
    pub opaque: bool,
}

pub struct BlockRegistry {
    pub blocks: Vec<Block>,
    by_name: HashMap<String, BlockId>,
    /// Texture names in layer order; the renderer loads these from the resource packs.
    pub textures: Vec<String>,
    // Dense per-id property tables for the hot loops (meshing, lighting).
    opaque: Vec<bool>,
    light: Vec<u8>,
}

impl BlockRegistry {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let file: BlockFile = ron::from_str(text).map_err(|e| format!("blocks.ron: {e}"))?;
        Self::from_defs(file.blocks)
    }

    pub fn from_defs(defs: Vec<BlockDef>) -> Result<Self, String> {
        let mut reg = BlockRegistry {
            blocks: Vec::new(),
            by_name: HashMap::new(),
            textures: Vec::new(),
            opaque: Vec::new(),
            light: Vec::new(),
        };
        let mut tex_index: HashMap<String, u16> = HashMap::new();
        let mut tex = |name: &str, reg_tex: &mut Vec<String>| -> u16 {
            if let Some(&i) = tex_index.get(name) {
                return i;
            }
            let i = reg_tex.len() as u16;
            reg_tex.push(name.to_string());
            tex_index.insert(name.to_string(), i);
            i
        };
        // Air is always id 0.
        let mut all = vec![BlockDef {
            name: "air".into(),
            display_name: Some("Air".into()),
            shape: Shape::Invisible,
            transparency: Transparency::Translucent,
            textures: TextureSpec::default(),
            light: 0,
            solid: false,
            tint: false,
            hardness: 0.0,
            gravity: false,
            in_inventory: false,
        }];
        all.extend(defs.into_iter().filter(|d| d.name != "air"));
        for d in all {
            if reg.by_name.contains_key(&d.name) {
                return Err(format!("duplicate block name '{}'", d.name));
            }
            let t = &d.textures;
            let fallback = t.all.clone().or_else(|| t.side.clone()).unwrap_or_else(|| d.name.clone());
            let side = t.side.clone().unwrap_or_else(|| fallback.clone());
            let top = t.top.clone().or_else(|| t.all.clone()).unwrap_or_else(|| side.clone());
            let bottom = t.bottom.clone().or_else(|| t.all.clone()).unwrap_or_else(|| top.clone());
            let front = t.front.clone().unwrap_or_else(|| side.clone());
            let faces = if d.shape == Shape::Invisible {
                [0; 6]
            } else {
                [
                    tex(&side, &mut reg.textures),
                    tex(&side, &mut reg.textures),
                    tex(&top, &mut reg.textures),
                    tex(&bottom, &mut reg.textures),
                    tex(&front, &mut reg.textures),
                    tex(&side, &mut reg.textures),
                ]
            };
            let opaque = d.shape == Shape::Cube && d.transparency == Transparency::Opaque;
            let id = reg.blocks.len() as BlockId;
            let display = d.display_name.clone().unwrap_or_else(|| title_case(&d.name));
            reg.by_name.insert(d.name.clone(), id);
            reg.opaque.push(opaque);
            reg.light.push(d.light.min(15));
            reg.blocks.push(Block {
                name: d.name,
                display_name: display,
                shape: d.shape,
                transparency: d.transparency,
                faces,
                light: d.light.min(15),
                solid: d.solid,
                tint: d.tint,
                hardness: d.hardness,
                gravity: d.gravity,
                in_inventory: d.in_inventory,
                opaque,
            });
        }
        if reg.blocks.len() > u16::MAX as usize {
            return Err("too many blocks".into());
        }
        Ok(reg)
    }

    #[inline(always)]
    pub fn get(&self, id: BlockId) -> &Block {
        &self.blocks[id as usize]
    }

    #[inline(always)]
    pub fn is_opaque(&self, id: BlockId) -> bool {
        self.opaque[id as usize]
    }

    #[inline(always)]
    pub fn light_of(&self, id: BlockId) -> u8 {
        self.light[id as usize]
    }

    pub fn id(&self, name: &str) -> Option<BlockId> {
        self.by_name.get(name).copied()
    }

    /// Like `id` but falls back to air, for world generation code that names base-game blocks.
    pub fn id_or_air(&self, name: &str) -> BlockId {
        self.id(name).unwrap_or(AIR)
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

fn title_case(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_resolves_textures() {
        let reg = BlockRegistry::from_ron(
            r#"(blocks: [
                (name: "stone"),
                (name: "grass_block", tint: true, textures: (top: Some("grass_top"), side: Some("grass_side"), bottom: Some("dirt"))),
                (name: "glass", transparency: Cutout),
            ])"#,
        )
        .unwrap();
        assert_eq!(reg.id("air"), Some(0));
        let g = reg.get(reg.id("grass_block").unwrap());
        assert_eq!(reg.textures[g.faces[2] as usize], "grass_top");
        assert_eq!(reg.textures[g.faces[3] as usize], "dirt");
        assert_eq!(reg.textures[g.faces[0] as usize], "grass_side");
        assert!(reg.is_opaque(reg.id("stone").unwrap()));
        assert!(!reg.is_opaque(reg.id("glass").unwrap()));
        assert_eq!(g.display_name, "Grass Block");
    }
}
