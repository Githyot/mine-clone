use std::time::Instant;
use voxel::chunk::{ChunkPos, BASE_SECTION};
use voxel::worldgen::Generator;
use voxel::BlockRegistry;
fn main() {
    let reg = BlockRegistry::from_ron_many(&[include_str!("../../../packs/vanilla/blocks.ron"), include_str!("../../../packs/vanilla/fase6/blocks.ron"), include_str!("../../../packs/vanilla/deep/blocks.ron")]).unwrap();
    let g = Generator::new(12345, &reg);
    for lo in [BASE_SECTION, BASE_SECTION - 6, BASE_SECTION - 8, 0] {
        let mut best = f64::MAX;
        for _ in 0..3 {
            let t = Instant::now();
            for z in -8..8 { for x in -8..8 { std::hint::black_box(g.generate_band(ChunkPos::new(x, z), lo)); } }
            best = best.min(t.elapsed().as_secs_f64() * 1000.0 / 256.0);
        }
        println!("deep from section {lo}: {best:.2} ms/chunk");
    }
}
