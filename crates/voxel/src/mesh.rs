//! Turns chunk sections into vertex data. Only faces next to see-through blocks are emitted,
//! with per-vertex ambient occlusion and smooth lighting like the classic "smooth lighting" option.
//!
//! Vertex layout (12 bytes, three u32):
//! - w0: x 10 bits | y 10 bits | z 10 bits (1/16 block units, relative to the section) | ao 2 bits
//! - w1: u 5 bits | v 5 bits (texture pixels 0..16) | layer 12 bits | face 3 bits | flags 7 bits
//! - w2: sky 4 bits | block light 4 bits | tint r 8 | g 8 | b 8

use crate::block::{BlockId, BlockRegistry, Shape, Transparency, AIR};
use crate::chunk::{local_y, Chunk, MIN_Y, SECTIONS};
use crate::shapes::{CONN_NX, CONN_NZ, CONN_PX, CONN_PZ};

pub const FLAG_WAVE: u32 = 1;
pub const FLAG_LIQUID: u32 = 2;
/// Wave and liquid together (never on a real liquid face): a rooted plant. Its base stays put and
/// only the part above bends in the wind; the ao bits say if it stands on (1) or carries (2)
/// another such plant, so tall stacks bend as one stem.
pub const FLAG_PLANT: u32 = FLAG_WAVE | FLAG_LIQUID;
/// Merged quad (greedy meshing): the shader repeats the texture once per block from the
/// position instead of reading the vertex UVs.
pub const FLAG_TILE: u32 = 4;
/// Connected-texture edges (see `connected_edges`).
pub const FLAG_CTM_U0: u32 = 8;
pub const FLAG_CTM_U1: u32 = 16;
pub const FLAG_CTM_V0: u32 = 32;
pub const FLAG_CTM_V1: u32 = 64;
/// Top bit of the 12-bit layer field (texture arrays stay under 2048 layers): the texture has
/// glowing pixels (Continuity's emissive textures) that the shader draws at full brightness.
pub const LAYER_EMISSIVE: u32 = 2048;

/// Blocks whose textures have glowing parts: gem and precious ores, amethyst, crying obsidian,
/// the lit redstone torch's head.
pub fn is_emissive(name: &str) -> bool {
    const GLOW: [&str; 14] = ["diamond", "emerald", "redstone", "lapis", "gold", "quartz", "ruby", "sapphire", "topaz", "platinum", "ambrosium", "zanite", "gravitite", "resonarium"];
    (name.ends_with("_ore") && GLOW.iter().any(|g| name.contains(g))) || matches!(name, "amethyst_block" | "budding_amethyst" | "crying_obsidian" | "ancient_debris" | "redstone_torch")
}

/// How a block's texture joins its neighbours (Continuity's connected textures).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Connect {
    None,
    /// Every side (glass): no frame lines anywhere in a window.
    Full,
    /// Only sideways on the side faces (bookshelves): a long shelf, top and bottom kept.
    Horizontal,
}

/// A face that looks the same as its neighbours can merge with them: same texture, tint and
/// flags, no ambient occlusion and the same light at every corner.
#[derive(Clone, Copy, PartialEq, Eq)]
struct FaceKey {
    layer: u32,
    tint: [u8; 3],
    sky: u32,
    blk: u32,
    flags: u32,
    /// Liquid top lowered (no liquid above).
    low: bool,
    translucent: bool,
}

/// For each face direction, indexes into [x, y, z]: the normal axis, then the two axes the
/// merge mask spans.
#[inline(always)]
fn face_axes(f: usize) -> (usize, usize, usize) {
    match f {
        0 | 1 => (0, 2, 1),
        2 | 3 => (1, 0, 2),
        _ => (2, 0, 1),
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub w: [u32; 3],
}

#[derive(Default, Clone)]
pub struct SectionMesh {
    /// Opaque and cut-out geometry (4 vertices per quad).
    pub solid: Vec<Vertex>,
    /// Blended geometry (water, ice).
    pub translucent: Vec<Vertex>,
    /// Which faces of the section see each other through non-opaque blocks: bit `a * 6 + b`
    /// (faces in mesh order +x, -x, +y, -y, +z, -z). Used to skip sections hidden in caves.
    pub vis: u64,
}

/// Every face sees every other (an empty section).
pub const VIS_ALL: u64 = (1 << 36) - 1;

impl SectionMesh {
    pub fn is_empty(&self) -> bool {
        self.solid.is_empty() && self.translucent.is_empty()
    }
}

const P: usize = 18; // padded width

/// Blocks and light of a band of a chunk's sections (`lo..hi`, local) plus a one-block border
/// from its neighbours and the sections above and below.
pub struct MeshInput {
    blocks: Vec<BlockId>,
    light: Vec<u8>,
    biomes: [u8; 256],
    pub lo: usize,
    pub hi: usize,
}

#[inline(always)]
fn pi(x: usize, y: usize, z: usize) -> usize {
    x + z * P + y * P * P
}

impl MeshInput {
    /// `chunks` uses the same 3×3 layout as lighting (index 4 = centre). Missing neighbours read as stone-dark air.
    pub fn new(chunks: [Option<&Chunk>; 9]) -> Self {
        Self::band(chunks, 0, SECTIONS)
    }

    pub fn band(chunks: [Option<&Chunk>; 9], lo: usize, hi: usize) -> Self {
        let (lo, hi) = (lo.min(SECTIONS), hi.min(SECTIONS).max(lo));
        let ph = (hi - lo) * 16 + 2;
        // Padded y 0 is the block below the band.
        let first = (lo * 16) as i32 - 1;
        let mut blocks = vec![AIR; P * P * ph];
        let mut light = vec![0xF0u8; P * P * ph];
        for pz in 0..P {
            for px in 0..P {
                let wx = px as i32 - 1;
                let wz = pz as i32 - 1;
                let cx = wx.div_euclid(16);
                let cz = wz.div_euclid(16);
                let (lx, lz) = (wx.rem_euclid(16) as usize, wz.rem_euclid(16) as usize);
                let Some(c) = chunks[((cx + 1) + (cz + 1) * 3) as usize] else { continue };
                let lit = !c.light.is_empty();
                for py in 0..ph {
                    let y = first + py as i32;
                    if y < 0 || y >= (SECTIONS * 16) as i32 {
                        continue;
                    }
                    let y = y as usize;
                    let i = pi(px, py, pz);
                    let sec = &c.sections[y >> 4];
                    if !sec.is_empty() {
                        blocks[i] = sec.get(crate::chunk::section_index(lx, y & 15, lz));
                    }
                    if lit {
                        light[i] = c.light_local(lx, y, lz);
                    }
                }
            }
        }
        let c = chunks[4].expect("centre chunk");
        MeshInput { blocks, light, biomes: c.biomes, lo, hi }
    }

    /// Only sections `lo..=hi` (and a block above and below): enough to mesh just those, for an
    /// edit that has to show in the same frame.
    pub fn new_sections(chunks: [Option<&Chunk>; 9], lo: usize, hi: usize) -> Self {
        Self::band(chunks, lo, hi + 1)
    }

    /// Light for a block just opened up (relative to the centre chunk) before the light workers
    /// get to it: what its neighbours would spread into it, so the faces it uncovers aren't black.
    /// `y` is a world y.
    pub fn guess_light(&mut self, x: i32, y: i32, z: i32) {
        if y < MIN_Y {
            return;
        }
        let py = local_y(y) as i32 - (self.lo * 16) as i32 + 1;
        // Needs a padded block on every side, inside the band.
        if !(0..16).contains(&x) || !(0..16).contains(&z) || py < 1 || py as usize > (self.hi - self.lo) * 16 {
            return;
        }
        let i = pi((x + 1) as usize, py as usize, (z + 1) as usize);
        let (mut sky, mut blk) = (0u8, 0u8);
        for (d, down) in [(1isize, false), (-1, false), (P as isize, false), (-(P as isize), false), ((P * P) as isize, true), (-((P * P) as isize), false)] {
            let l = self.light[(i as isize + d) as usize];
            // Full sunlight keeps going straight down.
            let s = l >> 4;
            sky = sky.max(if down && s == 15 { 15 } else { s.saturating_sub(1) });
            blk = blk.max((l & 15).saturating_sub(1));
        }
        self.light[i] = sky << 4 | blk;
    }

    /// Leave out one block, given relative to the centre chunk (-1..=16 reaches the border).
    /// `y` is a world y.
    pub fn hide(&mut self, x: i32, y: i32, z: i32) {
        let py = local_y(y) as i32 - (self.lo * 16) as i32 + 1;
        if (-1..=16).contains(&x) && (-1..=16).contains(&z) && py >= 0 && (py as usize) < (self.hi - self.lo) * 16 + 2 {
            self.blocks[pi((x + 1) as usize, py as usize, (z + 1) as usize)] = AIR;
        }
    }

    /// `s` is a local section inside the band.
    pub fn section_has_blocks(&self, s: usize) -> bool {
        let y0 = (s - self.lo) * 16 + 1;
        (y0..y0 + 16).any(|y| {
            let row = &self.blocks[pi(0, y, 0)..pi(0, y + 1, 0)];
            row.iter().any(|&b| b != AIR)
        })
    }
}

/// Per-id facts the mesher needs, flattened for speed.
pub struct MeshTables {
    opaque: Vec<bool>,
    shape: Vec<Shape>,
    transp: Vec<Transparency>,
    faces: Vec<[u16; 6]>,
    /// Bit per face: its picture lies a quarter turn round (a log on its side).
    uv_turn: Vec<u8>,
    tint: Vec<u8>, // bit per face: tinted
    cull_same: Vec<bool>,
    wave: Vec<bool>,
    height: Vec<u32>,
    variant: Vec<u8>,
    /// Furniture style and its accent texture layer.
    style: Vec<crate::furniture::Style>,
    accent: Vec<u16>,
    connected: Vec<Connect>,
    /// Layer bits added for blocks with glowing pixels (`LAYER_EMISSIVE` or 0).
    emissive: Vec<u32>,
    /// Fire torches drawn as real torches like the one in hand (`torch_quads`): the texture
    /// layers of the rag head, straps and charred top, then for each block its flame's layer
    /// and colour (None: drawn the plain way, like the redstone torch and candles).
    torch_kit: [u32; 3],
    torch_flame: Vec<Option<(u32, [u8; 3])>>,
    /// Same id for every level of a liquid (and for every shape variant).
    base: Vec<BlockId>,
    /// Liquid surface height in pixels (source 14, falling 16, lower for each step of flow).
    liquid_h: Vec<u32>,
    biome_tints: Vec<[u8; 3]>,
    /// Water drawn with its biome's water colour (swamps).
    water: Vec<bool>,
    /// Blocks standing in water (kelp): water around them shows no face toward them.
    wlog: Vec<bool>,
    water_tints: Vec<[u8; 3]>,
    /// Redstone: 1 wire, 2 something wire runs into from any side, 3 a repeater (joins along
    /// its facing), 0 anything else.
    rs: Vec<u8>,
    /// Texture layers of the redstone torch unlit and lit, for the little torches on
    /// repeaters and comparators (the comparator's front one shows its mode, not its power).
    rs_torch: [u16; 2],
    /// Repeaters and comparators sending power: their torches glow.
    lit_parts: Vec<bool>,
    /// Shapes from resource packs (JSON block models), drawn instead of the block's own.
    /// Behind a lock so a pack change can swap them while the workers keep their tables.
    models: std::sync::RwLock<ModelSet>,
}

#[derive(Default)]
struct ModelSet {
    quads: Vec<Option<std::sync::Arc<[ModelQuad]>>>,
    /// Full blocks whose model leaves gaps: their neighbours' faces toward them are drawn.
    open: Vec<bool>,
}

/// One face of a resource pack model element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelFace {
    /// [u1, v1, u2, v2] in texture pixels (0..16); None = the element's own outline.
    pub uv: Option<[f32; 4]>,
    pub layer: u16,
    /// Hidden when the block on this side (0..6, mesh face order) is opaque.
    pub cull: Option<u8>,
    pub tint: bool,
}

/// A box of a resource pack model, in pixels (0..16), with its faces in mesh order
/// (+x, -x, +y, -y, +z, -z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub faces: [Option<ModelFace>; 6],
}

/// A model face ready to mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelQuad {
    pub pos: [[u8; 3]; 4],
    pub uv: [[u8; 2]; 4],
    pub layer: u16,
    pub face: u8,
    /// Face to cull against (6 = never).
    pub cull: u8,
    /// Touches the block's side: lit by the neighbour there.
    pub edge: bool,
    pub tint: bool,
}

/// The quads of a model's elements (positions clamped to the block, as the vertex format
/// holds only 0..16 per block).
pub fn model_quads(elements: &[ModelElement]) -> Vec<ModelQuad> {
    let mut out = Vec::new();
    for e in elements {
        let lo = [0, 1, 2].map(|a| e.from[a].min(e.to[a]).clamp(0.0, 16.0).round() as u32);
        let hi = [0, 1, 2].map(|a| e.from[a].max(e.to[a]).clamp(0.0, 16.0).round() as u32);
        for (f, fi) in FACES.iter().enumerate() {
            let Some(face) = e.faces[f] else { continue };
            // Where the face lies, and the texture outline the element itself covers there.
            let proj = |q: [u32; 3]| -> [f32; 2] {
                let uv = match f {
                    0 => [16 - q[2], 16 - q[1]],
                    1 => [q[2], 16 - q[1]],
                    2 => [q[0], q[2]],
                    3 => [q[0], 16 - q[2]],
                    4 => [q[0], 16 - q[1]],
                    _ => [16 - q[0], 16 - q[1]],
                };
                [uv[0] as f32, uv[1] as f32]
            };
            let corners: Vec<[u32; 3]> = fi.corners.iter().map(|c| [0, 1, 2].map(|a| if c[a] == 1 { hi[a] } else { lo[a] })).collect();
            let p: Vec<[f32; 2]> = corners.iter().map(|&q| proj(q)).collect();
            let (umin, umax) = (p.iter().map(|x| x[0]).fold(16.0, f32::min), p.iter().map(|x| x[0]).fold(0.0, f32::max));
            let (vmin, vmax) = (p.iter().map(|x| x[1]).fold(16.0, f32::min), p.iter().map(|x| x[1]).fold(0.0, f32::max));
            let mut q = ModelQuad { pos: [[0; 3]; 4], uv: [[0; 2]; 4], layer: face.layer, face: f as u8, cull: face.cull.unwrap_or(6), edge: false, tint: face.tint };
            for k in 0..4 {
                q.pos[k] = corners[k].map(|v| v as u8);
                let [mut u, mut v] = p[k];
                if let Some([u1, v1, u2, v2]) = face.uv {
                    let tu = if umax > umin { (u - umin) / (umax - umin) } else { 0.0 };
                    let tv = if vmax > vmin { (v - vmin) / (vmax - vmin) } else { 0.0 };
                    u = u1 + (u2 - u1) * tu;
                    v = v1 + (v2 - v1) * tv;
                }
                q.uv[k] = [u.clamp(0.0, 16.0).round() as u8, v.clamp(0.0, 16.0).round() as u8];
            }
            let axis = f >> 1;
            q.edge = if f & 1 == 0 { hi[axis] == 16 } else { lo[axis] == 0 };
            out.push(q);
        }
    }
    out
}

/// How redstone wire joins the blocks around it (see `MeshTables::rs`).
pub fn redstone_join(reg: &BlockRegistry, id: BlockId) -> u8 {
    let b = reg.get(id);
    match b.shape {
        Shape::Wire => 1,
        Shape::Lever | Shape::Button | Shape::Plate => 2,
        Shape::Repeater | Shape::Comparator => 3,
        _ => {
            let base = &reg.get(b.base).name;
            u8::from(base.starts_with("redstone_torch") || base == "redstone_block" || base == "detector_rail") * 2
        }
    }
}

impl MeshTables {
    pub fn new(reg: &BlockRegistry, biome_tints: Vec<[f32; 3]>) -> Self {
        let mut t = MeshTables {
            opaque: vec![],
            shape: vec![],
            transp: vec![],
            faces: vec![],
            uv_turn: vec![],
            tint: vec![],
            cull_same: vec![],
            wave: vec![],
            height: vec![],
            variant: vec![],
            style: vec![],
            accent: vec![],
            connected: vec![],
            emissive: vec![],
            torch_kit: [0; 3],
            torch_flame: vec![],
            base: vec![],
            liquid_h: vec![],
            models: Default::default(),
            biome_tints: biome_tints
                .iter()
                // The lowest bit of red marks foliage (odd), so the shader can turn it with the
                // seasons; untinted faces and water keep it even.
                .map(|c| [(c[0] * 200.0).min(255.0) as u8 | 1, (c[1] * 200.0).min(255.0) as u8, (c[2] * 200.0).min(255.0) as u8])
                .collect(),
            water: vec![],
            wlog: vec![],
            rs: vec![],
            rs_torch: ["redstone_torch_off", "redstone_torch"].map(|n| reg.textures.iter().position(|t| t == n).unwrap_or(0) as u16),
            lit_parts: vec![],
            water_tints: (0..crate::worldgen::Biome::COUNT)
                .map(|i| crate::worldgen::Biome::from_u8(i as u8).water_tint().map(|c| (c * 200.0).min(255.0) as u8 & !1))
                .collect(),
        };
        for (i, b) in reg.blocks.iter().enumerate() {
            t.water.push(b.shape == Shape::Liquid && b.name.starts_with("water"));
            t.wlog.push(b.waterlogged);
            t.rs.push(redstone_join(reg, i as BlockId));
            t.lit_parts.push(matches!(b.shape, Shape::Repeater | Shape::Comparator) && b.name.ends_with("_on"));
            t.opaque.push(reg.is_opaque(i as BlockId));
            t.shape.push(b.shape);
            t.transp.push(b.transparency);
            t.faces.push(b.faces);
            t.uv_turn.push(b.uv_turn);
            let leaves = b.name.ends_with("_leaves");
            let mut tint = 0u8;
            if b.tint {
                for f in 0..6 {
                    // Side textures that carry their own dirt (grass block sides) stay untinted.
                    let name = &reg.textures[b.faces[f] as usize];
                    if !name.ends_with("_side") {
                        tint |= 1 << f;
                    }
                }
            }
            t.tint.push(tint);
            t.cull_same.push(!leaves);
            t.wave.push(leaves || (b.shape == Shape::Cross && b.light == 0));
            t.height.push(b.height as u32);
            t.variant.push(b.variant);
            t.style.push(b.style);
            t.accent.push(b.accent);
            // Glass joins up with its neighbours: no frame lines between panes of a window.
            t.base.push(b.base);
            t.liquid_h.push(if b.shape == Shape::Liquid { crate::shapes::liquid_height(b.variant) } else { 0 });
            t.connected.push(if b.shape == Shape::Pane && (b.name == "glass_pane" || b.name.ends_with("_glass_pane")) {
                Connect::Full
            } else if b.shape != Shape::Cube {
                Connect::None
            } else if b.name == "glass" || b.name.ends_with("_glass") {
                Connect::Full
            } else if b.name == "bookshelf" || b.name.ends_with("_bookshelf") {
                Connect::Horizontal
            } else {
                Connect::None
            });
            t.emissive.push(if is_emissive(&b.name) { LAYER_EMISSIVE } else { 0 });
        }
        let tex = |n: &str| reg.textures.iter().position(|x| x == n).map(|i| i as u32);
        t.torch_flame = vec![None; reg.blocks.len()];
        if let (Some(rag), Some(strap), Some(char), Some(fire)) = (tex("brown_wool"), tex("dark_oak_log"), tex("coal_block"), tex("fire")) {
            t.torch_kit = [rag, strap, char];
            let soul = tex("soul_fire").unwrap_or(fire);
            for (i, b) in reg.blocks.iter().enumerate() {
                if b.shape == Shape::Torch {
                    if let Some((blue, tint)) = torch_flame_look(&reg.blocks[b.base as usize].name) {
                        t.torch_flame[i] = Some((if blue { soul } else { fire }, tint));
                    }
                }
            }
        }
        if t.biome_tints.is_empty() {
            t.biome_tints.push([200, 200, 200]);
        }
        t.set_models(Vec::new());
        t
    }
}

// For each face: the axis it points along, then two tangent axes (u, v) and the four corners
// in counter-clockwise order seen from outside. Corners are given as (du, dv) in {0,1}.
struct FaceInfo {
    normal: [i32; 3],
    /// Corner positions in block space for the 4 vertices.
    corners: [[u32; 3]; 4],
    /// Texture coordinates of the 4 vertices (u, v in 0..16).
    uv: [[u32; 2]; 4],
}

const FACES: [FaceInfo; 6] = [
    // +X
    FaceInfo { normal: [1, 0, 0], corners: [[1, 0, 1], [1, 0, 0], [1, 1, 0], [1, 1, 1]], uv: [[0, 16], [16, 16], [16, 0], [0, 0]] },
    // -X
    FaceInfo { normal: [-1, 0, 0], corners: [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]], uv: [[0, 16], [16, 16], [16, 0], [0, 0]] },
    // +Y
    FaceInfo { normal: [0, 1, 0], corners: [[0, 1, 1], [1, 1, 1], [1, 1, 0], [0, 1, 0]], uv: [[0, 16], [16, 16], [16, 0], [0, 0]] },
    // -Y
    FaceInfo { normal: [0, -1, 0], corners: [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]], uv: [[0, 16], [16, 16], [16, 0], [0, 0]] },
    // +Z
    FaceInfo { normal: [0, 0, 1], corners: [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]], uv: [[0, 16], [16, 16], [16, 0], [0, 0]] },
    // -Z
    FaceInfo { normal: [0, 0, -1], corners: [[1, 0, 0], [0, 0, 0], [0, 1, 0], [1, 1, 0]], uv: [[0, 16], [16, 16], [16, 0], [0, 0]] },
];

/// A placed fire torch like the one in hand (the game's torch.rs): a thick haft with an iron
/// collar, a big rag head bound by two straps, a charred top and a flame of crossed quads that
/// the terrain shader animates like fire. On a wall it leans out from three pixels up, the way
/// the plain torch did. All in the terrain mesh, so a thousand torches cost nothing per frame.
fn torch_quads(haft: u32, kit: [u32; 3], (fire, flame_tint): (u32, [u8; 3]), variant: u8, base: [u32; 3], l: u8, out: &mut Vec<Vertex>) {
    let [rag, strap, char] = kit;
    let (sky, blk) = ((l >> 4) as u32, (l & 15) as u32);
    let white = [200u8, 200, 200];
    let wall = variant > 0;
    // Model pixels → block pixels: lifted and leaning out from a wall to the north, then turned.
    let place = |p: [i32; 3]| -> [u32; 3] {
        let mut q = p;
        if wall {
            q[2] = q[2] - 5 + p[1] * 2 / 5;
            q[1] += 3;
            for _ in 0..(variant - 1) & 3 {
                q = [16 - q[2], q[1], q[0]];
            }
        }
        [base[0] + q[0].max(0) as u32, base[1] + q[1].max(0) as u32, base[2] + q[2].max(0) as u32]
    };
    let mut cube = |lo: [i32; 3], hi: [i32; 3], layer: u32, haft_uv: bool| {
        for (f, fi) in FACES.iter().enumerate() {
            for (k, c) in fi.corners.iter().enumerate() {
                let p = [if c[0] == 1 { hi[0] } else { lo[0] }, if c[1] == 1 { hi[1] } else { lo[1] }, if c[2] == 1 { hi[2] } else { lo[2] }];
                let uv = if haft_uv {
                    // The stick of the torch picture, stretched along the haft.
                    let [u, v] = fi.uv[k];
                    [if u == 0 { 7 } else { 9 }, if f == 2 || f == 3 { if v == 0 { 14 } else { 16 } } else if v == 0 { 6 } else { 16 }]
                } else {
                    let (a, b) = match f {
                        0 | 1 => (p[2], 16 - p[1]),
                        4 | 5 => (p[0], 16 - p[1]),
                        _ => (p[0], p[2]),
                    };
                    [a.clamp(0, 16) as u32, b.clamp(0, 16) as u32]
                };
                out.push(pack(place(p), 3, uv, layer, f as u32, 0, sky, blk, white));
            }
        }
    };
    cube([7, 0, 7], [9, 10, 9], haft, true);
    cube([6, 8, 6], [10, 9, 10], strap, false);
    cube([6, 9, 6], [10, 14, 10], rag, false);
    cube([5, 10, 5], [11, 11, 11], strap, false);
    cube([5, 12, 5], [11, 13, 11], strap, false);
    cube([6, 14, 6], [10, 15, 10], char, false);
    // The flame: two crossed pictures of fire standing on the head, seen from both sides.
    let quads: [[[i32; 3]; 4]; 2] = [
        [[5, 13, 5], [11, 13, 11], [11, 24, 11], [5, 24, 5]],
        [[11, 13, 5], [5, 13, 11], [5, 24, 11], [11, 24, 5]],
    ];
    let uvs = [[0, 16], [16, 16], [16, 0], [0, 0]];
    for q in quads {
        for order in [[0usize, 1, 2, 3], [1, 0, 3, 2]] {
            for &k in &order {
                out.push(pack(place(q[k]), 3, uvs[k], fire, 2, 0, sky, blk, flame_tint));
            }
        }
    }
}

/// How a torch's flame looks, by block name: drawn from the soul fire picture (pale blue)
/// instead of fire, and a colour over it (200 = as painted; red kept even, an odd red marks
/// foliage for the seasons). None for torches without a flame (redstone, candles).
pub fn torch_flame_look(name: &str) -> Option<(bool, [u8; 3])> {
    Some(match name {
        "torch" => (false, [200, 200, 200]),
        "desert_torch" => (false, [230, 190, 150]),
        "jungle_torch" => (false, [120, 250, 70]),
        "cursed_torch" => (false, [90, 255, 40]),
        "ichor_torch" => (false, [254, 230, 40]),
        "demon_torch" => (true, [210, 90, 255]),
        "hallowed_torch" => (true, [254, 150, 230]),
        "ice_torch" => (true, [200, 230, 255]),
        "coral_torch" => (true, [120, 255, 230]),
        "bone_torch" => (true, [254, 250, 240]),
        _ => return None,
    })
}

/// Liquid height at a top corner: the average of the cells of the same liquid that share it,
/// full height when any of them has more of it above.
fn liquid_corner(t: &MeshTables, blocks: &[BlockId], i: usize, cx: u32, cz: u32) -> u32 {
    let me = t.base[blocks[i] as usize];
    let (mut sum, mut cnt) = (0, 0);
    for dz in [cz as isize - 1, cz as isize] {
        for dx in [cx as isize - 1, cx as isize] {
            let j = (i as isize + dx + dz * P as isize) as usize;
            let b = blocks[j] as usize;
            if t.base[b] == me && t.liquid_h[b] > 0 {
                if t.base[blocks[j + P * P] as usize] == me {
                    return 16;
                }
                sum += t.liquid_h[b];
                cnt += 1;
            }
        }
    }
    if cnt == 0 { 14 } else { (sum + cnt / 2) / cnt }
}

/// Connected-texture edge bits: which sides of face `f` of the block at `i` continue into
/// the same block (u low, u high, v low, v high), so the shader can leave out the frame there.
fn connected_edges(blocks: &[BlockId], i: usize, id: BlockId, f: usize) -> u32 {
    let fi = &FACES[f];
    let d = |a: [u32; 3], b: [u32; 3]| [a[0] as i32 - b[0] as i32, a[1] as i32 - b[1] as i32, a[2] as i32 - b[2] as i32];
    let du = d(fi.corners[1], fi.corners[0]);
    let dv = d(fi.corners[0], fi.corners[3]);
    let at = |o: [i32; 3]| (i as isize + o[0] as isize + o[2] as isize * P as isize + o[1] as isize * (P * P) as isize) as usize;
    let n = fi.normal;
    let mut bits = 0;
    for (bit, o) in [(FLAG_CTM_U0, du.map(|x| -x)), (FLAG_CTM_U1, du), (FLAG_CTM_V0, dv.map(|x| -x)), (FLAG_CTM_V1, dv)] {
        // Joined when the neighbour is the same block and its own face here is showing too.
        let front = [o[0] + n[0], o[1] + n[1], o[2] + n[2]];
        if blocks[at(o)] == id && blocks[at(front)] != id {
            bits |= bit;
        }
    }
    bits
}

#[inline(always)]
fn pack(pos: [u32; 3], ao: u32, uv: [u32; 2], layer: u32, face: u32, flags: u32, sky: u32, blk: u32, tint: [u8; 3]) -> Vertex {
    Vertex {
        w: [
            pos[0] | (pos[1] << 10) | (pos[2] << 20) | (ao << 30),
            uv[0] | (uv[1] << 5) | (layer << 10) | (face << 22) | (flags << 25),
            sky | (blk << 4) | ((tint[0] as u32) << 8) | ((tint[1] as u32) << 16) | ((tint[2] as u32) << 24),
        ],
    }
}

/// Flood-fill the open cells of a section and record which of its faces are connected.
pub fn section_visibility(t: &MeshTables, input: &MeshInput, section: usize) -> u64 {
    let y0 = (section - input.lo) * 16 + 1;
    let mut seen = [false; 4096];
    for (i, s) in seen.iter_mut().enumerate() {
        let (x, y, z) = (i & 15, i >> 8, (i >> 4) & 15);
        *s = t.opaque[input.blocks[pi(x + 1, y0 + y, z + 1)] as usize];
    }
    let mut vis = 0u64;
    let mut stack: Vec<usize> = Vec::new();
    for start in 0..4096 {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut faces = 0u8;
        while let Some(i) = stack.pop() {
            let (x, y, z) = (i & 15, i >> 8, (i >> 4) & 15);
            let mut go = |ok: bool, face: u8, j: isize| {
                if !ok {
                    faces |= 1 << face;
                } else {
                    let j = (i as isize + j) as usize;
                    if !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            };
            go(x < 15, 0, 1);
            go(x > 0, 1, -1);
            go(y < 15, 2, 256);
            go(y > 0, 3, -256);
            go(z < 15, 4, 16);
            go(z > 0, 5, -16);
        }
        for a in 0..6 {
            if faces & (1 << a) != 0 {
                for b in 0..6 {
                    if faces & (1 << b) != 0 {
                        vis |= 1 << (a * 6 + b);
                    }
                }
            }
        }
        if vis == VIS_ALL {
            break;
        }
    }
    vis
}

/// Mesh one 16-block-tall section of the chunk described by `input`.
impl MeshTables {
    /// Swap in the resource packs' block models (by block id). Chunks already meshed keep the
    /// old look until they are meshed again. Beds always keep our own model.
    pub fn set_models(&self, mut list: Vec<(BlockId, Vec<ModelQuad>)>) {
        let mut m = ModelSet { quads: vec![None; self.shape.len()], open: vec![false; self.shape.len()] };
        for (i, &shape) in self.shape.iter().enumerate() {
            if shape == Shape::Bed {
                list.push((i as BlockId, crate::bed::quads(self.variant[i], self.faces[i])));
            }
        }
        for (id, quads) in list {
            let i = id as usize;
            if i < m.quads.len() {
                // Open unless some box fills the whole block.
                m.open[i] = !full_cube(&quads);
                m.quads[i] = Some(quads.into());
            }
        }
        *self.models.write().unwrap() = m;
    }
}

/// Whether the quads close all six sides of the block (each side has a quad covering it all).
fn full_cube(quads: &[ModelQuad]) -> bool {
    (0..6u8).all(|f| {
        quads.iter().any(|q| {
            q.face == f && q.edge && {
                let axis = (f >> 1) as usize;
                (0..3).filter(|&a| a != axis).all(|a| q.pos.iter().any(|p| p[a] == 0) && q.pos.iter().any(|p| p[a] == 16))
            }
        })
    })
}

pub fn mesh_section(t: &MeshTables, input: &MeshInput, section: usize) -> SectionMesh {
    let mut out = SectionMesh { vis: section_visibility(t, input, section), ..Default::default() };
    let model_set = t.models.read().unwrap();
    let models = &model_set.quads;
    let open = &model_set.open;
    // Opaque for drawing: a full block, unless a pack model opened it up.
    let solid = |id: usize| t.opaque[id] && !open.get(id).copied().unwrap_or(false);
    let blocks = &input.blocks;
    let light = &input.light;
    let y0 = (section - input.lo) * 16;
    let white = [200u8, 200, 200];
    // Uniform faces wait here, per direction and slice, to be merged into bigger quads.
    let mut masks: Vec<[Option<FaceKey>; 256]> = vec![[None; 256]; 6 * 16];
    let mut boxes: Vec<crate::shapes::PBox> = Vec::new();
    for ly in 0..16usize {
        let py = y0 + ly + 1;
        for lz in 0..16usize {
            for lx in 0..16usize {
                let (px, pz) = (lx + 1, lz + 1);
                let i = pi(px, py, pz);
                let id = blocks[i];
                if id == AIR {
                    continue;
                }
                let idu = id as usize;
                let shape = t.shape[idu];
                // Tall banners are drawn whole by the game, swaying.
                if shape == Shape::Invisible || shape == Shape::TallBanner {
                    continue;
                }
                let biome = input.biomes[lx + lz * 16] as usize;
                let biome_tint = t.biome_tints[biome.min(t.biome_tints.len() - 1)];
                let wave = if t.wave[idu] { FLAG_WAVE } else { 0 };
                let base = [lx as u32 * 16, ly as u32 * 16, lz as u32 * 16];

                if let Some(Some(quads)) = models.get(idu) {
                    for q in quads.iter() {
                        let ni = |f: usize| {
                            let n = FACES[f].normal;
                            (i as isize + n[0] as isize + n[2] as isize * P as isize + n[1] as isize * (P * P) as isize) as usize
                        };
                        if q.cull < 6 && solid(blocks[ni(q.cull as usize)] as usize) {
                            continue;
                        }
                        // Inside the block: the brightest light around it (a solid block holds none).
                        let l = if q.edge { light[ni(q.face as usize)] } else { (0..6).map(|f| light[ni(f)]).fold(light[i], |a, b| (a & 0xF0).max(b & 0xF0) | (a & 15).max(b & 15)) };
                        let (sky, blk) = ((l >> 4) as u32, (l & 15) as u32);
                        let tint = if q.tint { biome_tint } else { white };
                        for k in 0..4 {
                            let c = q.pos[k];
                            let pos = [base[0] + c[0] as u32, base[1] + c[1] as u32, base[2] + c[2] as u32];
                            out.solid.push(pack(pos, 3, [q.uv[k][0] as u32, q.uv[k][1] as u32], q.layer as u32, q.face as u32, 0, sky, blk, tint));
                        }
                    }
                    continue;
                }

                if shape == Shape::Cross {
                    let l = light[i];
                    let (sky, blk) = ((l >> 4) as u32, (l & 15) as u32);
                    let layer = t.faces[idu][0] as u32;
                    let tint = if t.tint[idu] != 0 { biome_tint } else { white };
                    let quads: [[[u32; 3]; 4]; 2] = [
                        [[1, 0, 1], [15, 0, 15], [15, 16, 15], [1, 16, 1]],
                        [[15, 0, 1], [1, 0, 15], [1, 16, 15], [15, 16, 1]],
                    ];
                    let uvs = [[0, 16], [16, 16], [16, 0], [0, 0]];
                    let (flags, stack) = if wave != 0 {
                        let stem = |j: usize| t.shape[blocks[j] as usize] == Shape::Cross && t.wave[blocks[j] as usize];
                        (FLAG_PLANT, stem(i - P * P) as u32 | (stem(i + P * P) as u32) << 1)
                    } else {
                        (0, 3)
                    };
                    for q in quads {
                        // Front and back so the plant is visible from both sides.
                        for order in [[0usize, 1, 2, 3], [1, 0, 3, 2]] {
                            for &k in &order {
                                let c = q[k];
                                out.solid.push(pack(
                                    [base[0] + c[0], base[1] + c[1], base[2] + c[2]],
                                    stack,
                                    uvs[k],
                                    layer,
                                    2,
                                    flags,
                                    sky,
                                    blk,
                                    tint,
                                ));
                            }
                        }
                    }
                    continue;
                }

                if let Some(flame) = t.torch_flame[idu] {
                    torch_quads(t.faces[idu][0] as u32, t.torch_kit, flame, t.variant[idu], base, light[i], &mut out.solid);
                    continue;
                }
                if shape == Shape::Torch {
                    let l = light[i];
                    let (sky, blk) = ((l >> 4) as u32, (l & 15) as u32);
                    // On a wall: three pixels up, the foot against the wall and the head leaning
                    // four pixels out (drawn for a wall to the north, then turned).
                    let v = t.variant[idu];
                    let wall = v > 0;
                    for (f, fi) in FACES.iter().enumerate() {
                        let layer = t.faces[idu][f] as u32 | t.emissive[idu];
                        for (k, c) in fi.corners.iter().enumerate() {
                            let mut pos = [if c[0] == 1 { 9 } else { 7 }, if c[1] == 1 { 10 } else { 0 }, if c[2] == 1 { 9 } else { 7 }];
                            if wall {
                                pos[1] += 3;
                                pos[2] = pos[2] - 6 + if c[1] == 1 { 4 } else { 0 };
                                for _ in 0..(v - 1) & 3 {
                                    pos = [16 - pos[2], pos[1], pos[0]];
                                }
                            }
                            let pos = [base[0] + pos[0], base[1] + pos[1], base[2] + pos[2]];
                            let [u, v] = fi.uv[k];
                            let u = if u == 0 { 7 } else { 9 };
                            // Sides show the whole stick; the top shows the flame, the bottom the handle end.
                            let v = match f {
                                2 => if v == 0 { 6 } else { 8 },
                                3 => if v == 0 { 14 } else { 16 },
                                _ => if v == 0 { 6 } else { 16 },
                            };
                            out.solid.push(pack(pos, 3, [u, v], layer, f as u32, 0, sky, blk, white));
                        }
                    }
                    continue;
                }

                if shape == Shape::Wire {
                    wire_quads(t, blocks, i, base, light[i], &mut out);
                    continue;
                }
                if shape == Shape::Rail {
                    rail_quad(t, blocks[i], base, light[i], &mut out);
                    continue;
                }

                if shape == Shape::Inset {
                    let tint = |f: usize| if t.tint[idu] & (1 << f) != 0 { biome_tint } else { white };
                    for (f, fi) in FACES.iter().enumerate() {
                        let n = fi.normal;
                        let ni = (i as isize + n[0] as isize + n[2] as isize * P as isize + n[1] as isize * (P * P) as isize) as usize;
                        // Top and bottom are full faces, hidden against the same block or solid ones.
                        if (f == 2 || f == 3) && (blocks[ni] == id || solid(blocks[ni] as usize)) {
                            continue;
                        }
                        let l = if f == 2 || f == 3 { light[ni] } else { light[i].max(light[ni]) };
                        let (sky, blk) = ((l >> 4) as u32, (l & 15) as u32);
                        let layer = t.faces[idu][f] as u32;
                        for (k, c) in fi.corners.iter().enumerate() {
                            let mut q = [c[0] * 16, c[1] * 16, c[2] * 16];
                            // Pull the side faces one pixel in along their normal.
                            if f != 2 && f != 3 {
                                let a = if n[0] != 0 { 0 } else { 2 };
                                q[a] = if c[a] == 1 { 15 } else { 1 };
                            }
                            out.solid.push(pack([base[0] + q[0], base[1] + q[1], base[2] + q[2]], 3, fi.uv[k], layer, f as u32, 0, sky, blk, tint(f)));
                        }
                    }
                    continue;
                }

                if shape == Shape::Pot {
                    // The plant: crossed quads standing in the soil, drawn from the bottom picture.
                    let l = light[i];
                    let (sky, blk) = ((l >> 4) as u32, (l & 15) as u32);
                    let layer = t.faces[idu][3] as u32;
                    let quads: [[[u32; 3]; 4]; 2] = [
                        [[3, 5, 3], [13, 5, 13], [13, 16, 13], [3, 16, 3]],
                        [[13, 5, 3], [3, 5, 13], [3, 16, 13], [13, 16, 3]],
                    ];
                    let uvs = [[0, 16], [16, 16], [16, 0], [0, 0]];
                    for q in quads {
                        for order in [[0usize, 1, 2, 3], [1, 0, 3, 2]] {
                            for &k in &order {
                                let c = q[k];
                                out.solid.push(pack([base[0] + c[0], base[1] + c[1], base[2] + c[2]], 3, uvs[k], layer, 2, 0, sky, blk, white));
                            }
                        }
                    }
                }

                if shape.is_boxes() {
                    let nb = |o: isize| blocks[(i as isize + o) as usize] as usize;
                    let conn_at = |j: isize| {
                        let mut conn = 0;
                        for (bit, o) in [(CONN_PX, 1isize), (CONN_NX, -1), (CONN_PZ, P as isize), (CONN_NZ, -(P as isize))] {
                            let n = nb(j + o);
                            if crate::shapes::joins(t.shape[nb(j)], t.shape[n], solid(n)) {
                                conn |= bit;
                            }
                        }
                        conn
                    };
                    let conn = conn_at(0);
                    // Panes: the sides they join are hidden against the next pane (no end caps
                    // inside a window), and so are top and bottom against a pane above or below
                    // that is shaped the same.
                    let up = (P * P) as isize;
                    let pane = shape == Shape::Pane;
                    let stacked = |o: isize| pane && t.shape[nb(o)] == Shape::Pane && conn_at(o) == conn;
                    let (pane_up, pane_down) = (stacked(up), stacked(-up));
                    let same = |o: isize| blocks[(i as isize + o) as usize] == id;
                    boxes.clear();
                    let accent_from = crate::shapes::boxes_styled(shape, t.style[idu], t.variant[idu], conn, false, &mut boxes);
                    let tint = |f: usize| if t.tint[idu] & (1 << f) != 0 { biome_tint } else { white };
                    let parts = crate::shapes::has_parts(shape);
                    let piston = matches!(shape, Shape::Piston | Shape::PistonHead | Shape::Oriented);
                    let pdir = crate::shapes::dir6_of(t.variant[idu]);
                    let facing = crate::shapes::facing_of(t.variant[idu]);
                    // A door hinged on the left shows its picture mirrored, the handle on the
                    // other side.
                    let mirror = shape == Shape::Door && t.variant[idu] & crate::shapes::DOOR_HINGE_LEFT != 0;
                    for (bi, b) in boxes.iter().enumerate() {
                        let part = parts && bi > 0;
                        let (lo, hi) = ([b[0] as u32, b[1] as u32, b[2] as u32], [b[3] as u32, b[4] as u32, b[5] as u32]);
                        for (f, fi) in FACES.iter().enumerate() {
                            let n = fi.normal;
                            let edge = match f {
                                0 => hi[0] == 16,
                                1 => lo[0] == 0,
                                2 => hi[1] >= 16,
                                3 => lo[1] == 0,
                                4 => hi[2] == 16,
                                _ => lo[2] == 0,
                            };
                            let ni = (i as isize + n[0] as isize + n[2] as isize * P as isize + n[1] as isize * (P * P) as isize) as usize;
                            if edge && solid(blocks[ni] as usize) {
                                continue;
                            }
                            let side_bit = [CONN_PX, CONN_NX, 0, 0, CONN_PZ, CONN_NZ][f];
                            if pane && edge && (conn & side_bit != 0 && t.shape[blocks[ni] as usize] == Shape::Pane || f == 2 && pane_up || f == 3 && pane_down) {
                                continue;
                            }
                            let l = if edge { light[ni] } else { light[i] };
                            let (sky, blk) = ((l >> 4) as u32, (l & 15) as u32);
                            let layer = if bi >= accent_from {
                                t.accent[idu]
                            } else if part && shape == Shape::Comparator && bi == 1 {
                                // Lit in subtract mode, whatever the comparator puts out.
                                let sub = t.variant[idu] & 4 != 0;
                                t.rs_torch[sub as usize] | if sub { LAYER_EMISSIVE as u16 } else { 0 }
                            } else if part {
                                t.faces[idu][4] | if t.lit_parts[idu] { LAYER_EMISSIVE as u16 } else { 0 }
                            } else if piston {
                                // The face it points to shows the piston top, the back its bottom.
                                let df = DIR6_FACE[pdir];
                                t.faces[idu][if f == df { 2 } else if f == df ^ 1 { 3 } else { 0 }]
                            } else if shape == Shape::Pot && f == 3 {
                                t.faces[idu][0]
                            } else {
                                t.faces[idu][f]
                            } as u32;
                            // Glass panes join their frames up with the same pane around them,
                            // like glass blocks: (u low, u high) neighbours for each face.
                            let ctm = if t.connected[idu] == Connect::Full && pane {
                                let (u0, u1) = match f {
                                    0 => (P as isize, -(P as isize)),
                                    1 => (-(P as isize), P as isize),
                                    4 => (-1, 1),
                                    5 => (1, -1),
                                    _ => (0, 0),
                                };
                                let mut bits = 0;
                                if u0 != 0 {
                                    for (bit, o, ok) in [(FLAG_CTM_U0, u0, true), (FLAG_CTM_U1, u1, true), (FLAG_CTM_V0, up, pane_up), (FLAG_CTM_V1, -up, pane_down)] {
                                        if ok && same(o) {
                                            bits |= bit;
                                        }
                                    }
                                }
                                bits
                            } else {
                                0
                            };
                            for c in fi.corners {
                                let q = [if c[0] == 1 { hi[0] } else { lo[0] }, if c[1] == 1 { hi[1] } else { lo[1] }, if c[2] == 1 { hi[2] } else { lo[2] }];
                                let mut uv = match f {
                                    0 => [16 - q[2], 16 - q[1]],
                                    1 => [q[2], 16 - q[1]],
                                    2 => [q[0], q[2]],
                                    3 => [q[0], 16 - q[2]],
                                    4 => [q[0], 16 - q[1]],
                                    _ => [16 - q[0], 16 - q[1]],
                                };
                                if part {
                                    uv = part_uv(f, q, lo, hi);
                                } else if piston && f >> 1 != DIR6_FACE[pdir] >> 1 {
                                    uv = piston_side_uv(f, q, pdir);
                                } else if f == 2 && matches!(shape, Shape::Repeater | Shape::Comparator) {
                                    uv = turn_uv([q[0], q[2]], facing);
                                } else if mirror && f != 2 && f != 3 {
                                    uv[0] = 16 - uv[0];
                                }
                                out.solid.push(pack([base[0] + q[0], base[1] + q[1], base[2] + q[2]], 3, uv, layer, f as u32, ctm, sky, blk, tint(f)));
                            }
                        }
                    }
                    continue;
                }

                let liquid = shape == Shape::Liquid;
                let transp = t.transp[idu];
                // Liquid surface height at each top corner (x, z), averaged with the liquid
                // around it so flowing water slopes smoothly down to its edge.
                let lh = if liquid { [0, 1, 2, 3].map(|k| liquid_corner(t, blocks, i, k & 1, k >> 1)) } else { [16; 4] };
                // Low blocks (slabs, beds): the top sits lower and is always drawn.
                let hgt = t.height[idu];
                for (f, fi) in FACES.iter().enumerate() {
                    let n = fi.normal;
                    let ni = (i as isize + n[0] as isize + n[2] as isize * P as isize + n[1] as isize * (P * P) as isize) as usize;
                    let nid = blocks[ni];
                    let low_top = hgt < 16 && f == 2;
                    if solid(nid as usize) && !low_top {
                        continue;
                    }
                    if nid == id && t.cull_same[idu] && !(hgt < 16 && f != 3) {
                        continue;
                    }
                    // Liquid against liquid: no face in between (any level of the same one, or the
                    // sides against another), only the top shows over a different one.
                    if liquid && (t.shape[nid as usize] == Shape::Liquid && (f != 2 || t.base[nid as usize] == t.base[idu]) || t.water[idu] && t.wlog[nid as usize]) {
                        continue;
                    }
                    // Ice (or glass) on water: neither face between them shows. The translucent
                    // pass doesn't sort faces inside a section, so the ice's dim underside
                    // could be drawn over its top and turn a frozen river dark.
                    let n_liquid = t.shape[nid as usize] == Shape::Liquid;
                    let n_ice = nid != AIR && !n_liquid && t.transp[nid as usize] == Transparency::Translucent && t.shape[nid as usize] == Shape::Cube;
                    if (liquid && n_ice) || (!liquid && n_liquid && transp == Transparency::Translucent && shape == Shape::Cube) {
                        continue;
                    }
                    let layer = t.faces[idu][f] as u32 | t.emissive[idu];
                    let tint = if t.tint[idu] & (1 << f) != 0 {
                        biome_tint
                    } else if t.water[idu] {
                        t.water_tints[biome.min(t.water_tints.len() - 1)]
                    } else {
                        white
                    };
                    let ctm = match t.connected[idu] {
                        Connect::None => 0,
                        Connect::Full => connected_edges(blocks, i, id, f),
                        // Side faces only, and only the left/right edges.
                        Connect::Horizontal if f != 2 && f != 3 => connected_edges(blocks, i, id, f) & (FLAG_CTM_U0 | FLAG_CTM_U1),
                        Connect::Horizontal => 0,
                    };
                    let mut verts = [Vertex::default(); 4];
                    let mut aos = [3u32; 4];
                    let mut lights = [(0u32, 0u32); 4];
                    for (k, c) in fi.corners.iter().enumerate() {
                        // Neighbour cells around this corner, in the plane in front of the face.
                        let (mut sky_sum, mut blk_sum, mut cnt) = (0u32, 0u32, 0u32);
                        let nl = light[ni];
                        sky_sum += (nl >> 4) as u32;
                        blk_sum += (nl & 15) as u32;
                        cnt += 1;
                        let mut ao = 3;
                        if !liquid {
                            // Tangent offsets: for each axis that isn't the normal, step toward the corner.
                            let mut t_off = [[0i32; 3]; 2];
                            let mut ti = 0;
                            for a in 0..3 {
                                if n[a] == 0 {
                                    t_off[ti][a] = if c[a] == 1 { 1 } else { -1 };
                                    ti += 1;
                                }
                            }
                            let off = |o: [i32; 3]| -> usize {
                                (ni as isize + o[0] as isize + o[2] as isize * P as isize + o[1] as isize * (P * P) as isize) as usize
                            };
                            let s1 = off(t_off[0]);
                            let s2 = off(t_off[1]);
                            let cc = off([t_off[0][0] + t_off[1][0], t_off[0][1] + t_off[1][1], t_off[0][2] + t_off[1][2]]);
                            let o1 = solid(blocks[s1] as usize);
                            let o2 = solid(blocks[s2] as usize);
                            let oc = solid(blocks[cc] as usize);
                            ao = if o1 && o2 { 0 } else { 3 - (o1 as u32 + o2 as u32 + oc as u32) };
                            for (idx, o) in [(s1, o1), (s2, o2), (cc, oc && !(o1 && o2))] {
                                if !o && !(o1 && o2 && idx == cc) {
                                    let l = light[idx];
                                    sky_sum += (l >> 4) as u32;
                                    blk_sum += (l & 15) as u32;
                                    cnt += 1;
                                }
                            }
                        }
                        let sky = (sky_sum + cnt / 2) / cnt;
                        let blk = (blk_sum + cnt / 2) / cnt;
                        let mut pos = [base[0] + c[0] * 16, base[1] + c[1] * 16, base[2] + c[2] * 16];
                        if liquid && c[1] == 1 {
                            pos[1] = base[1] + lh[(c[0] + c[2] * 2) as usize];
                        }
                        let mut uv = fi.uv[k];
                        if t.uv_turn[idu] & (1 << f) != 0 {
                            uv = turn_uv(uv, 1);
                        }
                        if liquid && c[1] == 1 && f != 2 && f != 3 {
                            uv[1] = 16 - lh[(c[0] + c[2] * 2) as usize];
                        }
                        if hgt < 16 && c[1] == 1 {
                            pos[1] = base[1] + hgt;
                            // Sides show the bottom part of the texture.
                            if f != 2 && f != 3 {
                                uv[1] = 16 - hgt;
                            }
                        }
                        aos[k] = ao;
                        lights[k] = (sky, blk);
                        verts[k] = pack(pos, ao, uv, layer, f as u32, wave | ctm | if liquid { FLAG_LIQUID } else { 0 }, sky, blk, tint);
                    }
                    // Plain full faces with even light go to the merge masks instead.
                    let uniform = aos.iter().all(|&a| a == 3) && lights.iter().all(|&l| l == lights[0]);
                    let flat = !liquid || (f == 2 && (lh == [14; 4] || lh == [16; 4]));
                    // Merged faces take their picture from the map, so a turned one stays alone.
                    if uniform && hgt == 16 && flat && t.connected[idu] == Connect::None && t.uv_turn[idu] & (1 << f) == 0 {
                        let (na, aa, ba) = face_axes(f);
                        let lp = [lx, ly, lz];
                        masks[f * 16 + lp[na]][lp[aa] + lp[ba] * 16] = Some(FaceKey {
                            layer,
                            tint,
                            sky: lights[0].0,
                            blk: lights[0].1,
                            flags: wave | if liquid { FLAG_LIQUID } else { 0 },
                            low: liquid && lh[0] == 14,
                            translucent: transp == Transparency::Translucent,
                        });
                        continue;
                    }
                    let target = if transp == Transparency::Translucent { &mut out.translucent } else { &mut out.solid };
                    // Flip the quad's diagonal when it makes the AO gradient look right.
                    if aos[0] + aos[2] < aos[1] + aos[3] {
                        target.extend_from_slice(&[verts[1], verts[2], verts[3], verts[0]]);
                    } else {
                        target.extend_from_slice(&verts);
                    }
                }
            }
        }
    }
    greedy(&mut masks, &mut out);
    out
}

/// Merge each mask into as few rectangles as possible (grow along a, then along b).
/// Face index (FACES order) of each of the six piston directions (`shapes::DIRS6`).
const DIR6_FACE: [usize; 6] = [5, 0, 4, 1, 2, 3];

/// Texture coordinates on a top face for a block turned `k` quarter turns from north: the
/// picture's top points the way the block faces.
fn turn_uv(uv: [u32; 2], k: u8) -> [u32; 2] {
    let [mut x, mut z] = uv;
    for _ in 0..k {
        (x, z) = (z, 16 - x);
    }
    [x, z]
}

/// A lever handle or repeater torch: the middle two columns of a torch picture, flame at
/// the far end of its long side.
fn part_uv(f: usize, q: [u32; 3], lo: [u32; 3], hi: [u32; 3]) -> [u32; 2] {
    let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    let long = if size[1] >= size[0] && size[1] >= size[2] { 1 } else if size[0] >= size[2] { 0 } else { 2 };
    let normal = f >> 1;
    let others: Vec<usize> = (0..3).filter(|&a| a != normal).collect();
    if normal == long {
        [7 + (q[others[0]] - lo[others[0]]).min(2), 6 + (q[others[1]] - lo[others[1]]).min(2)]
    } else {
        let across = if others[0] == long { others[1] } else { others[0] };
        [7 + (q[across] - lo[across]).min(2), 6 + (hi[long] - q[long]).min(10)]
    }
}

/// Piston sides: the picture's top (the wooden edge) points the way the piston faces.
fn piston_side_uv(f: usize, q: [u32; 3], dir: usize) -> [u32; 2] {
    let d = crate::shapes::DIRS6[dir];
    let axis = d.iter().position(|&c| c != 0).unwrap_or(1);
    let along = if d[axis] > 0 { q[axis] } else { 16 - q[axis] };
    let normal = f >> 1;
    let across = (0..3).find(|&a| a != axis && a != normal).unwrap_or(0);
    [q[across], 16 - along]
}

/// Redstone dust: one flat quad whose picture (cross, T, corner or line) is turned to match
/// where it joins, plus a strip up the side of each block it climbs. Brighter with power.
fn wire_quads(t: &MeshTables, blocks: &[BlockId], i: usize, base: [u32; 3], l: u8, out: &mut SectionMesh) {
    let idu = blocks[i] as usize;
    let up = (P * P) as isize;
    let nb = |o: isize| blocks[(i as isize + o) as usize] as usize;
    let open_above = !t.opaque[nb(up)];
    // East, west, south, north.
    let dirs = [(CONN_PX, 1isize, 0u8), (CONN_NX, -1, 0), (CONN_PZ, P as isize, 1), (CONN_NZ, -(P as isize), 1)];
    let (mut conn, mut climb) = (0u8, 0u8);
    for (bit, o, axis) in dirs {
        let n = nb(o);
        let joins = match t.rs[n] {
            1 | 2 => true,
            // Repeaters only along their length (facing north/south is axis 1).
            3 => (t.variant[n] & 1 == 0) == (axis == 1),
            _ => false,
        };
        if joins || (!t.opaque[n] && t.rs[nb(o - up)] == 1) {
            conn |= bit;
        } else if open_above && t.rs[nb(o + up)] == 1 {
            conn |= bit;
            climb |= bit;
        }
    }
    let power = t.variant[idu] as u32;
    let k = power as f32 / 15.0;
    let tint = [(70.0 + 185.0 * k) as u8, if power > 0 { (8.0 + 40.0 * k) as u8 } else { 0 }, 0];
    let (sky, blk) = ((l >> 4) as u32, (l & 15) as u32);
    let (e, w, s, n) = (conn & CONN_PX != 0, conn & CONN_NX != 0, conn & CONN_PZ != 0, conn & CONN_NZ != 0);
    let count = conn.count_ones();
    // Textures: side line (north-south), top cross, bottom corner (north + east), front T (no south).
    let (tex, turn) = match count {
        0 | 4 => (2, 0),
        1 => (0, u8::from(e || w)),
        2 if (e && w) || (n && s) => (0, u8::from(e)),
        2 => (3, if n && e { 0 } else if e && s { 1 } else if s && w { 2 } else { 3 }),
        _ => (4, if !s { 0 } else if !w { 1 } else if !n { 2 } else { 3 }),
    };
    let layer = t.faces[idu][tex] as u32;
    let fi = &FACES[2];
    for c in fi.corners.iter() {
        let pos = [base[0] + c[0] * 16, base[1] + 1, base[2] + c[2] * 16];
        let uv = turn_uv([c[0] * 16, c[2] * 16], turn);
        out.solid.push(pack(pos, 3, uv, layer, 2, 0, sky, blk, tint));
    }
    // Up the wall of a block it climbs: one pixel out from it, facing back into this block.
    let line = t.faces[idu][0] as u32;
    for (bit, f, axis, at) in [(CONN_PX, 1usize, 0usize, 15u32), (CONN_NX, 0, 0, 1), (CONN_PZ, 5, 2, 15), (CONN_NZ, 4, 2, 1)] {
        if climb & bit == 0 {
            continue;
        }
        let fi = &FACES[f];
        for (k, c) in fi.corners.iter().enumerate() {
            let mut q = [c[0] * 16, c[1] * 16, c[2] * 16];
            q[axis] = at;
            out.solid.push(pack([base[0] + q[0], base[1] + q[1], base[2] + q[2]], 3, fi.uv[k], line, f as u32, 0, sky, blk, tint));
        }
    }
}

/// A rail: one quad a pixel off the ground (or up the slope), its picture turned to run the
/// way the rail does. Curves and powered rails use the `top` texture.
fn rail_quad(t: &MeshTables, id: BlockId, base: [u32; 3], l: u8, out: &mut SectionMesh) {
    use crate::shapes::{rail_on, rail_shape};
    let idu = id as usize;
    let v = t.variant[idu];
    let s = rail_shape(v);
    let tex = if rail_on(v) || s >= 6 { 2 } else { 0 };
    // The straight picture runs north-south; the curve joins north and east.
    let turn = match s {
        0 | 4 | 5 => 0,
        1..=3 => 1,
        6 => 1,
        7 => 2,
        8 => 3,
        _ => 0,
    };
    let layer = t.faces[idu][tex] as u32;
    let (sky, blk) = ((l >> 4) as u32, (l & 15) as u32);
    let white = [200, 200, 200];
    for c in FACES[2].corners.iter() {
        let lift = match s {
            2 => c[0] * 16,
            3 => (1 - c[0]) * 16,
            4 => (1 - c[2]) * 16,
            5 => c[2] * 16,
            _ => 0,
        };
        let pos = [base[0] + c[0] * 16, base[1] + 1 + lift, base[2] + c[2] * 16];
        let uv = turn_uv([c[0] * 16, c[2] * 16], turn);
        out.solid.push(pack(pos, 3, uv, layer, 2, 0, sky, blk, white));
    }
}

fn greedy(masks: &mut [[Option<FaceKey>; 256]], out: &mut SectionMesh) {
    for f in 0..6 {
        let fi = &FACES[f];
        let (na, aa, ba) = face_axes(f);
        for slice in 0..16 {
            let m = &mut masks[f * 16 + slice];
            for b in 0..16 {
                let mut a = 0;
                while a < 16 {
                    let Some(key) = m[a + b * 16] else {
                        a += 1;
                        continue;
                    };
                    let mut w = 1;
                    while a + w < 16 && m[a + w + b * 16] == Some(key) {
                        w += 1;
                    }
                    let mut h = 1;
                    'grow: while b + h < 16 {
                        for k in 0..w {
                            if m[a + k + (b + h) * 16] != Some(key) {
                                break 'grow;
                            }
                        }
                        h += 1;
                    }
                    for bb in b..b + h {
                        for k in 0..w {
                            m[a + k + bb * 16] = None;
                        }
                    }
                    let target = if key.translucent { &mut out.translucent } else { &mut out.solid };
                    for c in fi.corners.iter() {
                        let mut pos = [0u32; 3];
                        pos[na] = (slice as u32 + c[na]) * 16;
                        pos[aa] = (a as u32 + if c[aa] == 1 { w as u32 } else { 0 }) * 16;
                        pos[ba] = (b as u32 + if c[ba] == 1 { h as u32 } else { 0 }) * 16;
                        if key.low && c[1] == 1 {
                            pos[1] -= 2;
                        }
                        target.push(pack(pos, 3, [0, 0], key.layer, f as u32, key.flags | FLAG_TILE, key.sky, key.blk, key.tint));
                    }
                    a += w;
                }
            }
        }
    }
}

/// Index buffer for `quads` quads (0,1,2, 0,2,3 pattern), shared by every chunk.
pub fn quad_indices(quads: usize) -> Vec<u32> {
    let mut v = Vec::with_capacity(quads * 6);
    for q in 0..quads as u32 {
        let b = q * 4;
        v.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::ChunkPos;

    #[test]
    fn single_block_has_six_faces() {
        let reg = BlockRegistry::from_ron(r#"(blocks: [(name: "stone")])"#).unwrap();
        let t = MeshTables::new(&reg, vec![]);
        const B: usize = crate::chunk::BASE_SECTION;
        let mut c = Chunk::new(ChunkPos::new(0, 0));
        c.set(5, 5, 5, 1);
        let input = MeshInput::new([None, None, None, None, Some(&c), None, None, None, None]);
        let m = mesh_section(&t, &input, B);
        assert_eq!(m.solid.len(), 24);
        assert!(input.section_has_blocks(B));
        assert!(!input.section_has_blocks(B + 3));
        assert_eq!(section_visibility(&t, &input, B), VIS_ALL);
        // Two touching blocks hide two faces, and the evenly lit faces left merge into a 2x1 box.
        c.set(6, 5, 5, 1);
        let input = MeshInput::new([None, None, None, None, Some(&c), None, None, None, None]);
        let m = mesh_section(&t, &input, B);
        assert_eq!(m.solid.len(), 24);
        assert!(m.solid.iter().all(|v| (v.w[1] >> 25) & FLAG_TILE != 0));
    }
}
