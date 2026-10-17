//! The Otherside (Deeper and Darker's idea): a closed world of dark gloomslate under a bedrock
//! roof, reached through a portal of sculk frames. Four regions: the Deeplands (bare grey
//! plains crusted with sculk grime), the Echoing Forest (pale soil under drooping echo trees
//! hung with glowing bulbs), the Overcast Columns (huge pillars from floor to roof) and the
//! Blooming Caverns (glowing moss and flowers). Still black pools lie in the lowest hollows.
//! Our own take on the idea, from public descriptions.

use super::{Biome, Generator};
use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos};
use crate::modtrees::{self, ModTree, Shape};
use crate::rng::Rng;

/// Bedrock roof; nothing above.
pub const OTHERSIDE_ROOF: i32 = 127;
/// Open space at or below this height fills with water.
const POOL: i32 = 22;
const H: usize = OTHERSIDE_ROOF as usize + 1;

#[derive(Clone)]
pub(super) struct OthersidePalette {
    bedrock: BlockId,
    gloomslate: BlockId,
    sculk_stone: BlockId,
    grime: BlockId,
    echo_soil: BlockId,
    moss: BlockId,
    ore: BlockId,
    water: BlockId,
    tendrils: BlockId,
    flower: BlockId,
    bulb: BlockId,
    echo: ModTree,
    amber: BlockId,
}

impl OthersidePalette {
    pub(super) fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        OthersidePalette {
            bedrock: g("bedrock"),
            gloomslate: g("gloomslate"),
            sculk_stone: g("sculk_stone"),
            grime: g("sculk_grime"),
            echo_soil: g("echo_soil"),
            moss: g("blooming_moss"),
            ore: g("resonarium_ore"),
            water: g("water"),
            tendrils: g("sculk_tendrils"),
            flower: g("gloom_bloom"),
            bulb: g("echo_bulb"),
            echo: ModTree { log: g("echo_log"), leaves: g("echo_leaves"), shape: Shape::Weeping, fruit: g("echo_bulb") },
            amber: g("crystallized_amber"),
        }
    }
}

impl Generator {
    pub(super) fn otherside_biome(&self, wx: i32, wz: i32) -> Biome {
        let (x, z) = (wx as f32, wz as f32);
        let a = self.variant.fbm2(x / 240.0 - 900.0, z / 240.0, 3);
        let b = self.humidity.fbm2(x / 280.0 + 600.0, z / 280.0, 2);
        if a > 0.18 {
            Biome::OvercastColumns
        } else if b > 0.12 {
            Biome::EchoingForest
        } else if b < -0.16 {
            Biome::BloomingCaverns
        } else {
            Biome::Deeplands
        }
    }

    /// Floor, roof and how readily columns rise, for one column of the world.
    fn otherside_shape(&self, wx: i32, wz: i32) -> (f32, f32, f32) {
        let (x, z) = (wx as f32, wz as f32);
        let floor = 30.0 + self.continent.fbm2(x / 70.0 - 300.0, z / 70.0, 3) * 18.0;
        let roof = 104.0 + self.mountain.fbm2(x / 50.0, z / 50.0 + 200.0, 3) * 14.0;
        let col_edge = if self.otherside_biome(wx, wz) == Biome::OvercastColumns { 0.5 } else { 0.74 };
        (floor, roof, col_edge)
    }

    /// Rock or open space: one huge cavern between a rolling floor and a hanging roof, with
    /// columns in the Overcast Columns.
    pub(crate) fn otherside_solid(&self, wx: i32, y: i32, wz: i32) -> bool {
        self.otherside_solid_in(self.otherside_shape(wx, wz), wx, y, wz)
    }

    fn otherside_solid_in(&self, (floor, roof, col_edge): (f32, f32, f32), wx: i32, y: i32, wz: i32) -> bool {
        if y <= 0 || y >= OTHERSIDE_ROOF - 2 {
            return true;
        }
        let (x, yf, z) = (wx as f32, y as f32, wz as f32);
        if yf < floor || yf > roof {
            return true;
        }
        // Columns: round, a little twisted.
        if self.cave_b.noise2(x / 22.0 + (yf / 40.0).sin() * 0.4, z / 22.0) > col_edge {
            return true;
        }
        // Lumps and shelves in the open space.
        self.cheese.noise3(x / 26.0, yf / 18.0, z / 26.0) > 0.58
    }

    pub(super) fn generate_otherside(&self, pos: ChunkPos) -> Chunk {
        let p = &self.otherside;
        let mut chunk = Chunk::new(pos);
        let (bx, bz) = (pos.x * 16, pos.y * 16);
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 0x07E5);
        let mut solid = vec![false; 16 * 16 * H];
        let at = |x: usize, y: usize, z: usize| (y * 16 + z) * 16 + x;
        for z in 0..16 {
            for x in 0..16 {
                let shape = self.otherside_shape(bx + x as i32, bz + z as i32);
                for y in 0..H {
                    solid[at(x, y, z)] = self.otherside_solid_in(shape, bx + x as i32, y as i32, bz + z as i32);
                }
            }
        }
        let mut floors: Vec<(usize, usize, i32)> = Vec::new();
        for z in 0..16usize {
            for x in 0..16usize {
                let (wx, wz) = (bx + x as i32, bz + z as i32);
                let biome = self.otherside_biome(wx, wz);
                for y in 0..H {
                    let yi = y as i32;
                    let open_above = y + 1 < H && !solid[at(x, y + 1, z)];
                    let id = if solid[at(x, y, z)] {
                        let bedrock = yi == 0 || yi == OTHERSIDE_ROOF || (yi < 3 && rng.chance(0.5)) || (yi > OTHERSIDE_ROOF - 3 && rng.chance(0.5));
                        if bedrock {
                            p.bedrock
                        } else if open_above && yi > POOL {
                            floors.push((x, z, yi));
                            match biome {
                                Biome::EchoingForest => p.echo_soil,
                                Biome::BloomingCaverns => p.moss,
                                Biome::Deeplands if rng.chance(0.55) => p.grime,
                                Biome::OvercastColumns => p.sculk_stone,
                                _ => p.gloomslate,
                            }
                        } else if biome == Biome::OvercastColumns && yi > 40 {
                            if rng.chance(0.02) {
                                p.amber
                            } else {
                                p.sculk_stone
                            }
                        } else if rng.chance(0.006) && yi < 70 {
                            p.ore
                        } else {
                            p.gloomslate
                        }
                    } else if yi <= POOL {
                        p.water
                    } else {
                        AIR
                    };
                    if id != AIR {
                        chunk.set(x, y, z, id);
                    }
                }
                // Hanging tendrils and bulbs under the roof.
                for y in (60..OTHERSIDE_ROOF as usize - 3).rev() {
                    if solid[at(x, y + 1, z)] && !solid[at(x, y, z)] {
                        if rng.chance(0.06) {
                            let len = 1 + rng.below(4) as usize;
                            for k in 0..len {
                                if y >= k && !solid[at(x, y - k, z)] {
                                    chunk.set(x, y - k, z, if k + 1 == len && rng.chance(0.4) { p.bulb } else { p.tendrils });
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
        // Plants on the floors.
        for &(x, z, y) in &floors {
            let above = chunk.get(x, y as usize + 1, z);
            if above != AIR {
                continue;
            }
            let biome = self.otherside_biome(bx + x as i32, bz + z as i32);
            let r = rng.f32();
            let id = match biome {
                Biome::BloomingCaverns if r < 0.25 => p.flower,
                Biome::Deeplands | Biome::EchoingForest if r < 0.06 => p.tendrils,
                _ => AIR,
            };
            if id != AIR {
                chunk.set(x, y as usize + 1, z, id);
            }
        }
        // Echo trees, inside the chunk like the Overworld's.
        if self.otherside_biome(bx + 8, bz + 8) == Biome::EchoingForest {
            for _ in 0..6 {
                if floors.is_empty() {
                    break;
                }
                let (x, z, y) = floors[rng.below(floors.len() as u32) as usize];
                let reach = p.echo.shape.reach();
                if !(reach..16 - reach).contains(&(x as i32)) || !(reach..16 - reach).contains(&(z as i32)) {
                    continue;
                }
                if chunk.get(x, y as usize, z) != p.echo_soil || (1..10).any(|k| chunk.get(x, (y + k) as usize, z) != AIR) {
                    continue;
                }
                let mut space = super::ChunkSpace(&mut chunk);
                modtrees::grow(p.echo, &mut space, &mut rng, x as i32, y, z as i32);
            }
        }
        self.place_structures(&mut chunk);
        chunk
    }
}
