//! Turns chunk sections into vertex data. Only faces next to see-through blocks are emitted,
//! with per-vertex ambient occlusion and smooth lighting like the classic "smooth lighting" option.
//!
//! Vertex layout (12 bytes, three u32):
//! - w0: x 10 bits | y 10 bits | z 10 bits (1/16 block units, relative to the section) | ao 2 bits
//! - w1: u 5 bits | v 5 bits (texture pixels 0..16) | layer 12 bits | face 3 bits | flags 7 bits
//! - w2: sky 4 bits | block light 4 bits | tint r 8 | g 8 | b 8

use crate::block::{BlockId, BlockRegistry, Shape, Transparency, AIR};
use crate::chunk::{Chunk, CHUNK_HEIGHT};
use crate::shapes::{CONN_NX, CONN_NZ, CONN_PX, CONN_PZ};

pub const FLAG_WAVE: u32 = 1;
pub const FLAG_LIQUID: u32 = 2;
/// Merged quad (greedy meshing): the shader repeats the texture once per block from the
/// position instead of reading the vertex UVs.
pub const FLAG_TILE: u32 = 4;
/// Connected-texture edges (see `connected_edges`).
pub const FLAG_CTM_U0: u32 = 8;
pub const FLAG_CTM_U1: u32 = 16;
pub const FLAG_CTM_V0: u32 = 32;
pub const FLAG_CTM_V1: u32 = 64;

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
const PH: usize = CHUNK_HEIGHT + 2;

/// Blocks and light of a chunk plus a one-block border from its neighbours.
pub struct MeshInput {
    blocks: Vec<BlockId>,
    light: Vec<u8>,
    biomes: [u8; 256],
}

#[inline(always)]
fn pi(x: usize, y: usize, z: usize) -> usize {
    x + z * P + y * P * P
}

impl MeshInput {
    /// `chunks` uses the same 3×3 layout as lighting (index 4 = centre). Missing neighbours read as stone-dark air.
    pub fn new(chunks: [Option<&Chunk>; 9]) -> Self {
        let mut blocks = vec![AIR; P * P * PH];
        let mut light = vec![0xF0u8; P * P * PH];
        for pz in 0..P {
            for px in 0..P {
                let wx = px as i32 - 1;
                let wz = pz as i32 - 1;
                let cx = wx.div_euclid(16);
                let cz = wz.div_euclid(16);
                let (lx, lz) = (wx.rem_euclid(16) as usize, wz.rem_euclid(16) as usize);
                let Some(c) = chunks[((cx + 1) + (cz + 1) * 3) as usize] else { continue };
                for s in 0..16 {
                    let sec = &c.sections[s];
                    let empty = sec.is_empty();
                    for yy in 0..16 {
                        let y = s * 16 + yy;
                        let i = pi(px, y + 1, pz);
                        if !empty {
                            blocks[i] = sec.get(crate::chunk::section_index(lx, yy, lz));
                        }
                        light[i] = c.light_at(lx, y, lz);
                    }
                }
            }
        }
        let c = chunks[4].expect("centre chunk");
        MeshInput { blocks, light, biomes: c.biomes }
    }

    pub fn section_has_blocks(&self, s: usize) -> bool {
        let y0 = s * 16 + 1;
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
    tint: Vec<u8>, // bit per face: tinted
    cull_same: Vec<bool>,
    wave: Vec<bool>,
    height: Vec<u32>,
    variant: Vec<u8>,
    connected: Vec<bool>,
    /// Same id for every level of a liquid (and for every shape variant).
    base: Vec<BlockId>,
    /// Liquid surface height in pixels (source 14, falling 16, lower for each step of flow).
    liquid_h: Vec<u32>,
    biome_tints: Vec<[u8; 3]>,
    /// Water drawn with its biome's water colour (swamps).
    water: Vec<bool>,
    water_tints: Vec<[u8; 3]>,
    /// Redstone: 1 wire, 2 something wire runs into from any side, 3 a repeater (joins along
    /// its facing), 0 anything else.
    rs: Vec<u8>,
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
            tint: vec![],
            cull_same: vec![],
            wave: vec![],
            height: vec![],
            variant: vec![],
            connected: vec![],
            base: vec![],
            liquid_h: vec![],
            biome_tints: biome_tints
                .iter()
                // The lowest bit of red marks foliage (odd), so the shader can turn it with the
                // seasons; untinted faces and water keep it even.
                .map(|c| [(c[0] * 200.0).min(255.0) as u8 | 1, (c[1] * 200.0).min(255.0) as u8, (c[2] * 200.0).min(255.0) as u8])
                .collect(),
            water: vec![],
            rs: vec![],
            water_tints: (0..crate::worldgen::Biome::COUNT)
                .map(|i| crate::worldgen::Biome::from_u8(i as u8).water_tint().map(|c| (c * 200.0).min(255.0) as u8 & !1))
                .collect(),
        };
        for (i, b) in reg.blocks.iter().enumerate() {
            t.water.push(b.shape == Shape::Liquid && b.name.starts_with("water"));
            t.rs.push(redstone_join(reg, i as BlockId));
            t.opaque.push(reg.is_opaque(i as BlockId));
            t.shape.push(b.shape);
            t.transp.push(b.transparency);
            t.faces.push(b.faces);
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
            // Glass joins up with its neighbours: no frame lines between panes of a window.
            t.base.push(b.base);
            t.liquid_h.push(if b.shape == Shape::Liquid { crate::shapes::liquid_height(b.variant) } else { 0 });
            t.connected.push(b.shape == Shape::Cube && (b.name == "glass" || b.name.ends_with("_glass")));
        }
        if t.biome_tints.is_empty() {
            t.biome_tints.push([200, 200, 200]);
        }
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
    let y0 = section * 16 + 1;
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
pub fn mesh_section(t: &MeshTables, input: &MeshInput, section: usize) -> SectionMesh {
    let mut out = SectionMesh { vis: section_visibility(t, input, section), ..Default::default() };
    let blocks = &input.blocks;
    let light = &input.light;
    let y0 = section * 16;
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
                if shape == Shape::Invisible {
                    continue;
                }
                let biome = input.biomes[lx + lz * 16] as usize;
                let biome_tint = t.biome_tints[biome.min(t.biome_tints.len() - 1)];
                let wave = if t.wave[idu] { FLAG_WAVE } else { 0 };
                let base = [lx as u32 * 16, ly as u32 * 16, lz as u32 * 16];

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
                    for q in quads {
                        // Front and back so the plant is visible from both sides.
                        for order in [[0usize, 1, 2, 3], [1, 0, 3, 2]] {
                            for &k in &order {
                                let c = q[k];
                                out.solid.push(pack(
                                    [base[0] + c[0], base[1] + c[1], base[2] + c[2]],
                                    3,
                                    uvs[k],
                                    layer,
                                    2,
                                    wave,
                                    sky,
                                    blk,
                                    tint,
                                ));
                            }
                        }
                    }
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
                        let layer = t.faces[idu][f] as u32;
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
                        if (f == 2 || f == 3) && (blocks[ni] == id || t.opaque[blocks[ni] as usize]) {
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
                    let mut conn = 0;
                    for (bit, o) in [(CONN_PX, 1isize), (CONN_NX, -1), (CONN_PZ, P as isize), (CONN_NZ, -(P as isize))] {
                        let n = nb(o);
                        if crate::shapes::joins(shape, t.shape[n], t.opaque[n]) {
                            conn |= bit;
                        }
                    }
                    boxes.clear();
                    crate::shapes::boxes(shape, t.variant[idu], conn, false, &mut boxes);
                    let tint = |f: usize| if t.tint[idu] & (1 << f) != 0 { biome_tint } else { white };
                    let parts = crate::shapes::has_parts(shape);
                    let piston = matches!(shape, Shape::Piston | Shape::PistonHead | Shape::Oriented);
                    let pdir = crate::shapes::dir6_of(t.variant[idu]);
                    let facing = crate::shapes::facing_of(t.variant[idu]);
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
                            if edge && t.opaque[blocks[ni] as usize] {
                                continue;
                            }
                            let l = if edge { light[ni] } else { light[i] };
                            let (sky, blk) = ((l >> 4) as u32, (l & 15) as u32);
                            let layer = if part {
                                t.faces[idu][4]
                            } else if piston {
                                // The face it points to shows the piston top, the back its bottom.
                                let df = DIR6_FACE[pdir];
                                t.faces[idu][if f == df { 2 } else if f == df ^ 1 { 3 } else { 0 }]
                            } else if shape == Shape::Pot && f == 3 {
                                t.faces[idu][0]
                            } else {
                                t.faces[idu][f]
                            } as u32;
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
                                }
                                out.solid.push(pack([base[0] + q[0], base[1] + q[1], base[2] + q[2]], 3, uv, layer, f as u32, 0, sky, blk, tint(f)));
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
                    if t.opaque[nid as usize] && !low_top {
                        continue;
                    }
                    if nid == id && t.cull_same[idu] && !(hgt < 16 && f != 3) {
                        continue;
                    }
                    // Liquid against liquid: no face in between (any level of the same one, or the
                    // sides against another), only the top shows over a different one.
                    if liquid && t.shape[nid as usize] == Shape::Liquid && (f != 2 || t.base[nid as usize] == t.base[idu]) {
                        continue;
                    }
                    let layer = t.faces[idu][f] as u32;
                    let tint = if t.tint[idu] & (1 << f) != 0 {
                        biome_tint
                    } else if t.water[idu] {
                        t.water_tints[biome.min(t.water_tints.len() - 1)]
                    } else {
                        white
                    };
                    let ctm = if t.connected[idu] { connected_edges(blocks, i, id, f) } else { 0 };
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
                            let o1 = t.opaque[blocks[s1] as usize];
                            let o2 = t.opaque[blocks[s2] as usize];
                            let oc = t.opaque[blocks[cc] as usize];
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
                    if uniform && hgt == 16 && flat && !t.connected[idu] {
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
        let mut c = Chunk::new(ChunkPos::new(0, 0));
        c.set(5, 5, 5, 1);
        let input = MeshInput::new([None, None, None, None, Some(&c), None, None, None, None]);
        let m = mesh_section(&t, &input, 0);
        assert_eq!(m.solid.len(), 24);
        assert!(input.section_has_blocks(0));
        assert!(!input.section_has_blocks(3));
        assert_eq!(section_visibility(&t, &input, 0), VIS_ALL);
        // Two touching blocks hide two faces, and the evenly lit faces left merge into a 2x1 box.
        c.set(6, 5, 5, 1);
        let input = MeshInput::new([None, None, None, None, Some(&c), None, None, None, None]);
        let m = mesh_section(&t, &input, 0);
        assert_eq!(m.solid.len(), 24);
        assert!(m.solid.iter().all(|v| (v.w[1] >> 25) & FLAG_TILE != 0));
    }
}
