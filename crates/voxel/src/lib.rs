//! World simulation core: block registry, chunk storage, world generation,
//! lighting, meshing, raycasting, physics and saving. Pure CPU code so it can be
//! tested and benchmarked without a window or GPU.

pub mod bed;
pub mod block;
pub mod chunk;
pub mod flora;
pub mod furniture;
pub mod gems;
pub mod item;
pub mod light;
pub mod mesh;
pub mod mob;
pub mod modtrees;
pub mod noise;
pub mod physics;
pub mod props;
pub mod raycast;
pub mod recipe;
pub mod rng;
pub mod save;
pub mod shapes;
pub mod structures;
pub mod trees;
pub mod worldgen;

pub use block::{BlockId, BlockRegistry, AIR};
pub use chunk::{Chunk, ChunkPos, CHUNK_HEIGHT, CHUNK_SIZE};
