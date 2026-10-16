# 3D Skin Layers (by tr7zw)

**What it is:** turns the flat outer layer of player skins (jacket, sleeves, pants, hat) into real 3D voxel geometry, with transparency, also
on player head items. Falls back to flat 2D beyond 12 blocks to keep FPS. Client-only.

**License:** tr7zw Protective License (custom: no redistribution outside modpacks). → **Ideas only.**

## How we build it (better than the Java version)
The Java mod rebuilds a mesh on the CPU and renders it through the vanilla entity pipeline. In ours:
1. When a skin loads, **voxelize the outer layer once** into a small mesh (each opaque/translucent pixel → a thin cube; inner faces culled,
   greedy-merged like terrain). Cached per skin hash.
2. Drawn with **GPU skinning and instancing** (same system as Fresh Animations-style mob models), so 100 players cost about the same as 1.
3. **LOD**: 3D within ~16 blocks, flat layer from 16–64, skin-colored impostor beyond. Distance set in Options.
4. Corner zigzag artifacts the Java mod has are avoided by generating corner pieces at mesh build time.
5. Option to make 3D layers the default for everyone, because it's built in, not a mod.

## Where it lands
- **P7** (players/skins in multiplayer); base voxelizer can come in **P2** with the player model.
