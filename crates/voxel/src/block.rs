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
    /// A banner: a cloth hanging from a bar on a pole (variant facing 0-3) or on a wall (4-7).
    Banner,
    /// A head or skull: an 8-pixel cube on the floor.
    Skull,
    /// A hollow log (Distant Lands idea): four walls round a hole, standing up (variant 0) or
    /// lying along x (1) or z (2).
    Hollow,
    /// A fence gate (Macaw's Fences idea): two posts and a swinging leaf; right click opens it.
    FenceGate,
    /// A ladder flat against the wall to one side (variant facing 0-3); climbable.
    Ladder,
    /// A cake: half a block high, losing a slice from one side per bite (variant 0-6 bites).
    Cake,
    /// A bed: two blocks long, a head and a foot part (see `bed`).
    Bed,
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
                | Shape::Banner
                | Shape::Skull
                | Shape::Hollow
                | Shape::FenceGate
                | Shape::Ladder
                | Shape::Cake
                | Shape::Bed
        )
    }
}

/// How a full cube turns when it is placed. Its turns are hidden variants like a stair's, with
/// the face pictures moved round in the registry, so the mesher draws them as plain cubes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Turn {
    #[default]
    None,
    /// A front picture faces the player who placed it (furnace, crafting table): variant 0..3
    /// is the way the player looked (0 north .. 3 west), so the front points back the other way.
    Facing,
    /// Like `Facing`, and two side by side make one double chest: | 4 the left half, | 8 the
    /// right half (as seen from the front).
    Chest,
    /// A log: lies along the axis of the face it was placed on, 0 up, 1 x, 2 z.
    Axis,
}

/// Double chest halves (`Turn::Chest` variant bits).
pub const CHEST_LEFT: u8 = 4;
pub const CHEST_RIGHT: u8 = 8;

/// Face (FACE_DIRS order) the front of a `Facing` block shows on, per variant: the player
/// looked north, so the front faces south (+z), and so on.
pub const FRONT_FACE: [usize; 4] = [4, 1, 5, 0];

/// Way to the left half of a double chest from its right half, per facing: the player who
/// looks at the front has it on the left.
pub fn chest_left_dir(variant: u8) -> [i32; 3] {
    [[-1, 0, 0], [0, 0, -1], [1, 0, 0], [0, 0, 1]][(variant & 3) as usize]
}

/// Pillar blocks besides logs and stems that lie along an axis.
fn is_pillar(name: &str) -> bool {
    name.ends_with("_log") || name.ends_with("_stem") || name.ends_with("_pillar") || matches!(name, "hay_block" | "bone_block" | "basalt" | "polished_basalt")
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
    /// Stands in water (kelp): drawn and swum through as water, leaves water when broken.
    #[serde(default)]
    pub waterlogged: bool,
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
    pub waterlogged: bool,
    /// Orientation (stairs, doors...); 0 for the base block.
    pub variant: u8,
    /// The base block of a variant (itself for the base): what it drops and shows as.
    pub base: BlockId,
    /// How a full cube turns when placed; its turned variants keep the base's `name`.
    pub turn: Turn,
    /// Faces (bits in FACE_DIRS order) whose picture lies a quarter turn round (a log on its side).
    pub uv_turn: u8,
    /// The name in the registry and in saved worlds: `name`, or "chest:5" for a turned block
    /// whose `name` stays "chest" so everything that asks for a chest still finds it.
    pub key: String,
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
        Self::from_ron_many(&[text])
    }

    /// Several block files read as one list, in order (the base game, then mod packs).
    pub fn from_ron_many(texts: &[&str]) -> Result<Self, String> {
        let mut defs = Vec::new();
        for text in texts.iter().filter(|t| !t.trim().is_empty()) {
            let file: BlockFile = ron::from_str(text).map_err(|e| format!("blocks.ron: {e}"))?;
            defs.extend(file.blocks);
        }
        Self::from_defs(defs)
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
            waterlogged: false,
        }];
        all.extend(defs.into_iter().filter(|d| d.name != "air"));
        for d in all {
            if reg.by_name.contains_key(&d.name) {
                // Two data files naming the same block (stages merged together): the first wins.
                continue;
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
            let turn = if d.shape != Shape::Cube || height != 16 {
                Turn::None
            } else if front != side {
                if d.name == "chest" { Turn::Chest } else { Turn::Facing }
            } else if top != side && is_pillar(&d.name) {
                Turn::Axis
            } else {
                Turn::None
            };
            if turn == Turn::Chest {
                // The pictures of a double chest's halves (see tools/gen_double_chest.py).
                for t in ["front_left", "front_right", "side_left", "side_right", "top_w", "top_e", "top_n", "top_s"] {
                    tex(&format!("{}_{t}", d.name), &mut reg.textures);
                }
            }
            let id = reg.blocks.len() as BlockId;
            let display = d.display_name.clone().unwrap_or_else(|| title_case(&d.name));
            reg.by_name.insert(d.name.clone(), id);
            reg.opaque.push(opaque);
            reg.light.push(d.light.min(15));
            reg.blocks.push(Block {
                name: d.name.clone(),
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
                waterlogged: d.waterlogged,
                variant: 0,
                base: id,
                turn,
                uv_turn: 0,
                key: d.name,
            });
        }
        // Orientations get hidden ids after every base block, so base ids don't move.
        let bases = reg.blocks.len();
        reg.first_variant = vec![AIR; bases];
        for base in 0..bases {
            let b = reg.blocks[base].clone();
            let n = variant_count(&b);
            if n > 1 {
                reg.first_variant[base] = reg.blocks.len() as BlockId;
            }
            for v in 1..n {
                let id = reg.blocks.len() as BlockId;
                let name = format!("{}:{}", b.name, v);
                let mut vb = b.clone();
                vb.key = name.clone();
                if b.turn == Turn::None {
                    vb.name = name.clone();
                }
                vb.variant = v;
                vb.in_inventory = false;
                // A double slab is a full cube; the upper half of a door uses the top texture.
                vb.opaque = (b.shape == Shape::Slab && v == 2 || b.shape == Shape::Layer && v == 7) && b.transparency == Transparency::Opaque;
                if b.shape == Shape::Door {
                    vb.faces = [if v & 8 != 0 { b.faces[2] } else { b.faces[0] }; 6];
                }
                if b.turn != Turn::None {
                    vb.opaque = b.opaque;
                    let named = |t: &str, reg: &BlockRegistry| reg.textures.iter().position(|n| *n == format!("{}_{t}", b.name)).unwrap_or(0) as u16;
                    (vb.faces, vb.uv_turn) = turned_faces(&b, v, |t| named(t, &reg));
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
        if v == 0 || v >= variant_count(&self.blocks[base as usize]) {
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

/// How many ids a block needs for its orientations.
pub fn variant_count(b: &Block) -> u8 {
    match b.turn {
        Turn::Facing => 4,
        Turn::Chest => 12,
        Turn::Axis => 3,
        Turn::None => crate::shapes::variant_count(b.shape),
    }
}

/// The face pictures of variant `v` of a turning cube `b`, and which of them lie a quarter
/// turn round. `named` finds the pictures of a double chest's halves.
fn turned_faces(b: &Block, v: u8, named: impl Fn(&str) -> u16) -> ([u16; 6], u8) {
    // Base faces: +x side, -x side, top, bottom, +z front, -z side.
    let [side, _, top, bottom, front, _] = b.faces;
    match b.turn {
        Turn::Axis => match v {
            // Along x: the ends east and west, the bark round the other four turned to run along x.
            1 => ([top, bottom, side, side, side, side], 0b11_1100),
            // Along z: the ends south and north; on top and bottom the grain already runs along z.
            2 => ([side, side, side, side, top, bottom], 0b00_0011),
            _ => (b.faces, 0),
        },
        _ => {
            let f = (v & 3) as usize;
            let mut faces = [side, side, top, bottom, side, side];
            let back = FRONT_FACE[f] ^ 1;
            faces[FRONT_FACE[f]] = front;
            let half = v & (CHEST_LEFT | CHEST_RIGHT);
            if half != 0 {
                let left = half == CHEST_LEFT;
                faces[FRONT_FACE[f]] = named(if left { "front_left" } else { "front_right" });
                // From behind, the left half is on the right.
                faces[back] = named(if left { "side_right" } else { "side_left" });
                // The lid picture is laid out on the map: split west/east or north/south.
                let d = chest_left_dir(v);
                let low = (d[0] + d[2] < 0) == left;
                let (lo, hi) = if d[0] != 0 { ("top_w", "top_e") } else { ("top_n", "top_s") };
                faces[2] = named(if low { lo } else { hi });
                // Seen from below the picture's north is at the bottom.
                faces[3] = named(if low == (d[0] != 0) { lo } else { hi });
            }
            (faces, 0)
        }
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
