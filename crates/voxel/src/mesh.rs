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
    biome_tints: Vec<[u8; 3]>,
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
            biome_tints: biome_tints
                .iter()
                .map(|c| [(c[0] * 200.0).min(255.0) as u8, (c[1] * 200.0).min(255.0) as u8, (c[2] * 200.0).min(255.0) as u8])
                .collect(),
        };
        for (i, b) in reg.blocks.iter().enumerate() {
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
                let biome_tint = t.biome_tints[(input.biomes[lx + lz * 16] as usize).min(t.biome_tints.len() - 1)];
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
                    for (f, fi) in FACES.iter().enumerate() {
                        let layer = t.faces[idu][f] as u32;
                        for (k, c) in fi.corners.iter().enumerate() {
                            let pos = [base[0] + if c[0] == 1 { 9 } else { 7 }, base[1] + if c[1] == 1 { 10 } else { 0 }, base[2] + if c[2] == 1 { 9 } else { 7 }];
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
                    for b in &boxes {
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
                            let layer = t.faces[idu][f] as u32;
                            for c in fi.corners {
                                let q = [if c[0] == 1 { hi[0] } else { lo[0] }, if c[1] == 1 { hi[1] } else { lo[1] }, if c[2] == 1 { hi[2] } else { lo[2] }];
                                let uv = match f {
                                    0 => [16 - q[2], 16 - q[1]],
                                    1 => [q[2], 16 - q[1]],
                                    2 => [q[0], q[2]],
                                    3 => [q[0], 16 - q[2]],
                                    4 => [q[0], 16 - q[1]],
                                    _ => [16 - q[0], 16 - q[1]],
                                };
                                out.solid.push(pack([base[0] + q[0], base[1] + q[1], base[2] + q[2]], 3, uv, layer, f as u32, 0, sky, blk, tint(f)));
                            }
                        }
                    }
                    continue;
                }

                let liquid = shape == Shape::Liquid;
                let transp = t.transp[idu];
                let above_same = blocks[i + P * P] == id;
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
                    // Water next to any non-air block that isn't see-through to it: skip faces against solid cubes only.
                    if liquid && f != 2 && t.shape[nid as usize] == Shape::Liquid {
                        continue;
                    }
                    let layer = t.faces[idu][f] as u32;
                    let tint = if t.tint[idu] & (1 << f) != 0 { biome_tint } else { white };
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
                        if liquid && c[1] == 1 && !above_same {
                            pos[1] -= 2;
                        }
                        let mut uv = fi.uv[k];
                        if hgt < 16 && c[1] == 1 {
                            pos[1] = base[1] + hgt;
                            // Sides show the bottom part of the texture.
                            if f != 2 && f != 3 {
                                uv[1] = 16 - hgt;
                            }
                        }
                        aos[k] = ao;
                        lights[k] = (sky, blk);
                        verts[k] = pack(pos, ao, uv, layer, f as u32, wave | if liquid { FLAG_LIQUID } else { 0 }, sky, blk, tint);
                    }
                    // Plain full faces with even light go to the merge masks instead.
                    let uniform = aos.iter().all(|&a| a == 3) && lights.iter().all(|&l| l == lights[0]);
                    if uniform && hgt == 16 && (!liquid || f == 2) {
                        let (na, aa, ba) = face_axes(f);
                        let lp = [lx, ly, lz];
                        masks[f * 16 + lp[na]][lp[aa] + lp[ba] * 16] = Some(FaceKey {
                            layer,
                            tint,
                            sky: lights[0].0,
                            blk: lights[0].1,
                            flags: wave | if liquid { FLAG_LIQUID } else { 0 },
                            low: liquid && !above_same,
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
