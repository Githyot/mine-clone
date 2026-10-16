//! Quick timing of the world pipeline: `cargo run --release -p voxel --example bench`.
//! Also writes a top-down map of the generated area to `target/map.ppm`.
use std::time::Instant;
use voxel::chunk::{Chunk, ChunkPos};
use voxel::light::compute_light;
use voxel::mesh::{mesh_section, MeshInput, MeshTables};
use voxel::worldgen::{Biome, Generator};
use voxel::BlockRegistry;

fn main() {
    let reg = BlockRegistry::from_ron(include_str!("../../../packs/vanilla/blocks.ron")).unwrap();
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(12345u64);
    let g = Generator::new(seed, &reg);
    let r = 16i32;
    let n = (2 * r + 1) as usize;
    let t = Instant::now();
    let mut chunks: Vec<Chunk> = Vec::new();
    for z in -r..=r {
        for x in -r..=r {
            chunks.push(g.generate(ChunkPos::new(x, z)));
        }
    }
    let gen = t.elapsed();
    let at = |x: i32, z: i32| -> Option<usize> {
        if x.abs() <= r && z.abs() <= r { Some(((x + r) + (z + r) * (2 * r + 1)) as usize) } else { None }
    };
    let t = Instant::now();
    let mut lit = 0;
    let mut lights = Vec::new();
    for z in -4..=4 {
        for x in -4..=4 {
            let nb: [Option<&Chunk>; 9] = std::array::from_fn(|i| at(x + i as i32 % 3 - 1, z + i as i32 / 3 - 1).map(|k| &chunks[k]));
            lights.push((at(x, z).unwrap(), compute_light(&reg, nb)));
            lit += 1;
        }
    }
    let light_t = t.elapsed();
    for (k, l) in lights {
        chunks[k].light = l;
    }
    let tables = MeshTables::new(&reg, (0..Biome::COUNT).map(|b| Biome::from_u8(b as u8).tint()).collect());
    let t = Instant::now();
    let mut verts = 0;
    for z in -3..=3 {
        for x in -3..=3 {
            let nb: [Option<&Chunk>; 9] = std::array::from_fn(|i| at(x + i as i32 % 3 - 1, z + i as i32 / 3 - 1).map(|k| &chunks[k]));
            let input = MeshInput::new(nb);
            for s in 0..16 {
                if input.section_has_blocks(s) {
                    let m = mesh_section(&tables, &input, s);
                    verts += m.solid.len() + m.translucent.len();
                }
            }
        }
    }
    let mesh_t = t.elapsed();
    println!("generate: {} chunks in {:?} ({:.2} ms/chunk)", chunks.len(), gen, gen.as_secs_f64() * 1000.0 / chunks.len() as f64);
    println!("light:    {} chunks in {:?} ({:.2} ms/chunk)", lit, light_t, light_t.as_secs_f64() * 1000.0 / lit as f64);
    println!("mesh:     49 chunks in {:?} ({:.2} ms/chunk), {} vertices", mesh_t, mesh_t.as_secs_f64() * 1000.0 / 49.0, verts);

    // Top-down map.
    let w = n * 16;
    let mut img = format!("P6 {w} {w} 255\n").into_bytes();
    let colors: Vec<[u8; 3]> = reg.blocks.iter().map(|b| match b.name.as_str() {
        "water" => [40, 80, 200], "sand" => [220, 210, 150], "grass_block" => [80, 160, 60], "snowy_grass_block" | "snow" => [240, 240, 250],
        "stone" => [128, 128, 128], "oak_leaves" | "birch_leaves" => [30, 110, 30], "spruce_leaves" => [30, 80, 50], "ice" => [160, 200, 255],
        "gravel" => [110, 100, 100], "clay" => [150, 160, 180], "cactus" => [20, 140, 20], _ => [200, 100, 200],
    }).collect();
    for wz in 0..w {
        for wx in 0..w {
            let c = &chunks[(wx / 16) + (wz / 16) * n];
            let y = c.top_y(wx % 16, wz % 16).max(0);
            let id = c.get(wx % 16, y as usize, wz % 16);
            let col = colors[id as usize];
            let shade = 0.6 + (y as f32 / 255.0) * 0.8;
            img.extend(col.iter().map(|&v| (v as f32 * shade).min(255.0) as u8));
        }
    }
    std::fs::write("target/map.ppm", img).unwrap();
}
