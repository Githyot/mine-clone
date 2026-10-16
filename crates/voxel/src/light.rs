//! Sky light and block light for one chunk, computed from the chunk and its 8 neighbours.
//! Light travels at most 15 blocks, so a 15-block margin around the chunk is enough to get
//! exactly the same answer as lighting the whole world at once. Each chunk is lit independently,
//! which makes lighting trivially parallel.

use crate::block::BlockRegistry;
use crate::chunk::{column_index, Chunk, CHUNK_HEIGHT, CHUNK_VOLUME};

const MARGIN: usize = 15;
const W: usize = 16 + 2 * MARGIN;
const H: usize = CHUNK_HEIGHT;
const LAYER: usize = W * W;

#[inline(always)]
fn ri(x: usize, y: usize, z: usize) -> usize {
    x + z * W + y * LAYER
}

/// `chunks[(dx + 1) + (dz + 1) * 3]` is the chunk at offset (dx, dz); index 4 is the chunk being lit.
/// Missing neighbours are treated as solid rock, which only matters at the edge of the loaded area.
pub fn compute_light(reg: &BlockRegistry, chunks: [Option<&Chunk>; 9]) -> Vec<u8> {
    let n = W * W * H;
    // 255 = opaque, otherwise extra light lost when entering the cell (beyond the usual 1).
    let mut filter = vec![0u8; n];
    let mut emit: Vec<(usize, u8)> = Vec::new();
    let water = reg.id("water");
    let ice = reg.id("ice");

    // Per-id tables so the fill loop does no hashing.
    let filt_of: Vec<u8> = reg
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| {
            if reg.is_opaque(i as u16) || (b.name == "lava") {
                255
            } else if Some(i as u16) == water || Some(i as u16) == ice || b.name.ends_with("_leaves") {
                1
            } else {
                0
            }
        })
        .collect();

    for rz in 0..W {
        for rx in 0..W {
            let wx = rx as i32 - MARGIN as i32; // relative to the centre chunk origin
            let wz = rz as i32 - MARGIN as i32;
            let cx = wx.div_euclid(16);
            let cz = wz.div_euclid(16);
            let lx = wx.rem_euclid(16) as usize;
            let lz = wz.rem_euclid(16) as usize;
            let chunk = chunks[((cx + 1) + (cz + 1) * 3) as usize];
            match chunk {
                None => {
                    for y in 0..H {
                        filter[ri(rx, y, rz)] = 255;
                    }
                }
                Some(c) => {
                    for s in 0..16 {
                        let sec = &c.sections[s];
                        if sec.is_empty() {
                            continue;
                        }
                        for yy in 0..16 {
                            let y = s * 16 + yy;
                            let id = sec.get(crate::chunk::section_index(lx, yy, lz));
                            if id == 0 {
                                continue;
                            }
                            let i = ri(rx, y, rz);
                            filter[i] = filt_of[id as usize];
                            let l = reg.light_of(id);
                            if l > 0 {
                                emit.push((i, l));
                            }
                        }
                    }
                }
            }
        }
    }

    // ---- Sky light ----
    let mut sky = vec![0u8; n];
    // Lowest y that still sees the sky directly, per column.
    let mut sky_floor = vec![H as u16; W * W];
    for rz in 0..W {
        for rx in 0..W {
            let mut y = H;
            while y > 0 {
                let i = ri(rx, y - 1, rz);
                if filter[i] != 0 {
                    break;
                }
                sky[i] = 15;
                y -= 1;
            }
            sky_floor[rx + rz * W] = y as u16;
        }
    }
    let mut queue: Vec<u32> = Vec::with_capacity(1 << 16);
    for rz in 0..W {
        for rx in 0..W {
            let f = sky_floor[rx + rz * W] as usize;
            // Only cells next to a lower neighbour column can spread light sideways.
            let mut top = f;
            for (dx, dz) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (nx, nz) = (rx as i32 + dx, rz as i32 + dz);
                if nx >= 0 && nz >= 0 && (nx as usize) < W && (nz as usize) < W {
                    top = top.max(sky_floor[nx as usize + nz as usize * W] as usize);
                }
            }
            for y in f..top.min(H) {
                queue.push(ri(rx, y, rz) as u32);
            }
            // The cell right below the sky column (water, leaves) also starts spreading.
            if f < H && f > 0 {
                queue.push(ri(rx, f, rz) as u32);
            }
        }
    }
    flood(&mut sky, &filter, &mut queue);

    // ---- Block light ----
    let mut blk = vec![0u8; n];
    queue.clear();
    for (i, l) in emit {
        blk[i] = l;
        queue.push(i as u32);
    }
    flood(&mut blk, &filter, &mut queue);

    let mut out = vec![0u8; CHUNK_VOLUME];
    for y in 0..H {
        for z in 0..16 {
            for x in 0..16 {
                let i = ri(x + MARGIN, y, z + MARGIN);
                out[column_index(x, y, z)] = (sky[i] << 4) | blk[i];
            }
        }
    }
    out
}

/// Breadth-first spread: each step loses 1 light (plus the cell's filter).
fn flood(light: &mut [u8], filter: &[u8], queue: &mut Vec<u32>) {
    let mut head = 0;
    while head < queue.len() {
        let i = queue[head] as usize;
        head += 1;
        let l = light[i];
        if l <= 1 {
            continue;
        }
        let x = i % W;
        let z = (i / W) % W;
        let y = i / LAYER;
        let mut try_n = |j: usize| {
            let f = filter[j];
            if f == 255 {
                return;
            }
            let nl = l.saturating_sub(1 + f);
            if nl > light[j] {
                light[j] = nl;
                queue.push(j as u32);
            }
        };
        if x + 1 < W {
            try_n(i + 1);
        }
        if x > 0 {
            try_n(i - 1);
        }
        if z + 1 < W {
            try_n(i + W);
        }
        if z > 0 {
            try_n(i - W);
        }
        if y + 1 < H {
            try_n(i + LAYER);
        }
        if y > 0 {
            try_n(i - LAYER);
        }
        // Reclaim memory in very long floods.
        if head > 1 << 20 && head * 2 > queue.len() {
            queue.drain(..head);
            head = 0;
        }
    }
    queue.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::ChunkPos;

    fn reg() -> BlockRegistry {
        BlockRegistry::from_ron(r#"(blocks: [(name: "stone"), (name: "glowstone", light: 15)])"#).unwrap()
    }

    #[test]
    fn open_sky_is_bright_and_caves_are_dark() {
        let r = reg();
        let stone = r.id("stone").unwrap();
        let mut cs: Vec<Chunk> = (0..9).map(|i| Chunk::new(ChunkPos::new(i % 3 - 1, i / 3 - 1))).collect();
        for c in cs.iter_mut() {
            for y in 0..64 {
                for z in 0..16 {
                    for x in 0..16 {
                        c.set(x, y, z, stone);
                    }
                }
            }
        }
        // Carve a sealed room in the centre chunk and light it with glowstone.
        for y in 20..24 {
            for z in 4..10 {
                for x in 4..10 {
                    cs[4].set(x, y, z, 0);
                }
            }
        }
        cs[4].set(5, 20, 5, r.id("glowstone").unwrap());
        let refs: [Option<&Chunk>; 9] = std::array::from_fn(|i| Some(&cs[i]));
        let l = compute_light(&r, refs);
        assert_eq!(l[column_index(3, 100, 3)] >> 4, 15);
        assert_eq!(l[column_index(8, 22, 8)] >> 4, 0);
        assert_eq!(l[column_index(5, 21, 5)] & 15, 14);
        assert!(l[column_index(9, 23, 9)] & 15 > 0);
    }
}
