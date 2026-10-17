"""YUNG's Cave Biomes: blocks for the new cave features (blue ice and icicles in frozen
caves, moss carpet in lush caves). Placement is in crates/voxel/src/worldgen/caves.rs."""

from .lib import B, R, tex

B("blue_ice", "Gelo Azul", hardness="2.8", tool="Some(Pickaxe)", drops="Some([])", textures=tex(all="blue_ice"))
R(["###", "###", "###"], {"#": "packed_ice"}, "blue_ice")
B("icicle", "Pingente de Gelo", shape="Cross", transparency="Cutout", solid="false", hardness="0.3",
  tool="Some(Pickaxe)", drops="Some([])", textures=tex(all="icicle"))
B("moss_carpet", "Tapete de Musgo", shape="Layer", hardness="0.1", tool="Some(Hoe)", textures=tex(all="moss_block"))
R(["##"], {"#": "moss_block"}, "moss_carpet", 3)
