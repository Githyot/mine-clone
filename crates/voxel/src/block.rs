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
    /// Half block: bottom, top or both (see `shapes`).
    Slab,
    Stairs,
    Fence,
    Wall,
    /// Thin glass or bars.
    Pane,
    /// Two blocks tall; right click opens it.
    Door,
    Trapdoor,
    /// A cube whose sides sit one pixel in, each showing its whole picture, so spikes drawn
    /// at the edges of the texture stick out without leaving see-through gaps (the cactus).
    Inset,
    /// Thin layers that pile up, 1 to 8 (snow).
    Layer,
    /// A sloped roof (Macaw's Roofs idea): eight thin steps rising to the side it faces.
    Roof,
    /// Redstone dust lying on a block; the variant is its power, 0 to 15.
    Wire,
    /// On the floor or a wall; flips on and off.
    Lever,
    /// On the floor or a wall; pressed for a moment.
    Button,
    /// Pressure plate, pressed while something stands on it.
    Plate,
    /// A redstone repeater: a thin slab with two little torches.
    Repeater,
    /// Piston base, facing any of six ways, pushed out or not.
    Piston,
    /// The arm of a pushed-out piston.
    PistonHead,
    /// A redstone comparator: like a repeater, with two torches at the back.
    Comparator,
    /// A full block with a front facing any of six ways (observer, dropper, dispenser).
    Oriented,
    /// A hopper: a wide bowl over a spout pointing down or to a side.
    Hopper,
    /// A rail: flat on the ground, straight, curved or sloping up a block (see `shapes::RAIL_ENDS`).
    Rail,
    /// A chair you can sit on (Handcrafted and Macaw's Furniture idea): faces one of four ways,
    /// with its back on the far side.
    Chair,
    /// A table: a top on legs; tables side by side share one top and lose the legs between them.
    Table,
    /// A lantern standing on the ground (variant 0) or hanging from a chain (1).
    Lantern,
    /// A thin chain hanging straight down.
    Chain,
    /// A rope bridge (Macaw's Bridges idea): a plank deck with rope rails, running north-south
    /// (variant 0) or east-west (1).
    Bridge,
    /// The glowing sheet inside a lit portal frame: across x (variant 0) or across z (1).
    Portal,
    /// A small pot (Botany Pots idea) with a plant standing in it, crossed like a flower: the
    /// pot takes the side and top pictures, the plant the bottom one.
    Pot,
    /// A bench to sit on (Another Furniture idea): a long seat on two legs, no back.
    Bench,
    /// A shelf board on a wall, with two brackets under it.
    Shelf,
    /// A sign: a board on a post (variant facing 0-3) or on a wall (4-7).
    Sign,
    /// A head or skull: an 8-pixel cube on the floor.
    Skull,
}

impl Shape {
    /// Drawn and collided as boxes from `shapes::boxes`.
    pub fn is_boxes(self) -> bool {
        matches!(
            self,
            Shape::Slab
                | Shape::Stairs
                | Shape::Fence
                | Shape::Wall
                | Shape::Pane
                | Shape::Door
                | Shape::Trapdoor
                | Shape::Layer
                | Shape::Roof
                | Shape::Wire
                | Shape::Lever
                | Shape::Button
                | Shape::Plate
                | Shape::Repeater
                | Shape::Piston
                | Shape::PistonHead
                | Shape::Comparator
                | Shape::Oriented
                | Shape::Hopper
                | Shape::Rail
                | Shape::Chair
                | Shape::Table
                | Shape::Lantern
                | Shape::Chain
                | Shape::Bridge
                | Shape::Portal
                | Shape::Pot
                | Shape::Bench
                | Shape::Shelf
                | Shape::Sign
                | Shape::Skull
        )
    }
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
    /// Height in pixels for low cube blocks (slabs 8, beds 9); 16 is a full block.
    #[serde(default)]
    pub height: Option<u8>,
    /// The tool that digs it fast.
    #[serde(default)]
    pub tool: Option<ToolKind>,
    /// Lowest tool tier that gets drops (0 = hand is fine; 1 wood, 2 stone, 3 iron, 4 diamond).
    #[serde(default)]
    pub tier: u8,
    /// What it drops; missing means "itself", an empty list means nothing.
    #[serde(default)]
    pub drops: Option<Vec<DropDef>>,
    /// Experience dropped when mined (min, max).
    #[serde(default)]
    pub xp: (u8, u8),
}

/// Tool families. Blocks name the one that digs them fast; items say which one they are.
#[derive(Deserialize, serde::Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolKind {
    Pickaxe,
    Axe,
    Shovel,
    Sword,
    Hoe,
}

/// One possible drop: `count` from min to max, with a chance.
#[derive(Deserialize, Clone, Debug)]
pub struct DropDef {
    pub item: String,
    #[serde(default = "one")]
    pub min: u8,
    #[serde(default = "one")]
    pub max: u8,
    #[serde(default = "one_f")]
    pub chance: f32,
}

fn one() -> u8 {
    1
}
fn one_f() -> f32 {
    1.0
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
    /// Pixels from the bottom (16 = full block).
    pub height: u8,
    pub tool: Option<ToolKind>,
    pub tier: u8,
    pub drops: Option<Vec<DropDef>>,
    pub xp: (u8, u8),
    /// Orientation (stairs, doors...); 0 for the base block.
    pub variant: u8,
    /// The base block of a variant (itself for the base): what it drops and shows as.
    pub base: BlockId,
}

pub struct BlockRegistry {
    pub blocks: Vec<Block>,
    by_name: HashMap<String, BlockId>,
    /// Texture names in layer order; the renderer loads these from the resource packs.
    pub textures: Vec<String>,
    // Dense per-id property tables for the hot loops (meshing, lighting).
    opaque: Vec<bool>,
    light: Vec<u8>,
    /// Id of variant 1 of each base block (variants follow it in order), 0 when it has none.
    first_variant: Vec<BlockId>,
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
            first_variant: Vec::new(),
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
            height: None,
            tool: None,
            tier: 0,
            drops: Some(Vec::new()),
            xp: (0, 0),
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
            let height = d.height.unwrap_or(16).clamp(1, 16);
            let opaque = d.shape == Shape::Cube && d.transparency == Transparency::Opaque && height == 16;
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
                height,
                tool: d.tool,
                tier: d.tier,
                drops: d.drops,
                xp: d.xp,
                variant: 0,
                base: id,
            });
        }
        // Orientations get hidden ids after every base block, so base ids don't move.
        let bases = reg.blocks.len();
        reg.first_variant = vec![AIR; bases];
        for base in 0..bases {
            let b = reg.blocks[base].clone();
            let n = crate::shapes::variant_count(b.shape);
            if n > 1 {
                reg.first_variant[base] = reg.blocks.len() as BlockId;
            }
            for v in 1..n {
                let id = reg.blocks.len() as BlockId;
                let name = format!("{}:{}", b.name, v);
                let mut vb = b.clone();
                vb.name = name.clone();
                vb.variant = v;
                vb.in_inventory = false;
                // A double slab is a full cube; the upper half of a door uses the top texture.
                vb.opaque = (b.shape == Shape::Slab && v == 2 || b.shape == Shape::Layer && v == 7) && b.transparency == Transparency::Opaque;
                if b.shape == Shape::Door {
                    vb.faces = [if v & 8 != 0 { b.faces[2] } else { b.faces[0] }; 6];
                }
                reg.by_name.insert(name, id);
                reg.opaque.push(vb.opaque);
                reg.light.push(vb.light);
                reg.blocks.push(vb);
            }
            if b.shape == Shape::Door {
                reg.blocks[base].faces = [b.faces[0]; 6];
            }
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

    /// Variant `v` of a block (any variant of it may be passed); the base itself for 0 or when
    /// the shape has no such variant.
    pub fn variant(&self, id: BlockId, v: u8) -> BlockId {
        let base = self.blocks[id as usize].base;
        if v == 0 || v >= crate::shapes::variant_count(self.blocks[base as usize].shape) {
            return base;
        }
        self.first_variant[base as usize] + v as BlockId - 1
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
