# Deeper and Darker (by KyaniteMods)

**What it is:** expands the deep dark with a new dimension.
- **The Otherside**: a dark dimension reached through a portal in ancient cities, activated with the **Warden's heart**.
- **Biomes (4)**: Deeplands, Echoing Forest (glowing echo trees), Overcast Columns, Blooming Caverns (with sludges).
- **Structure**: Ancient Temple, the Warden's home; breaking its ancient vases can awaken an ancient lurker. Ancient city loot additions.
- **Blocks (100+)**: gloomslate and sculk stone sets (stairs, slabs, walls, bricks, ores), echo wood set, sculk grime, gloomy cactus/sculk plants, crystals.
- **Items**: Warden armor and tools (netherite tier and above), **soul elytra** (elytra upgrade with boost), sculk transmitter
  (links to containers remotely via vibrations), heart of the deep, reinforced echo shards.
- **Mobs (8, one miniboss)**: Otherside creatures such as sculk snappers, shattered, sculk leeches, sculk centipedes, angler fish,
  sludges, shriek worms and the stalker miniboss.

**License:** GNU GPL v3. → Translated code would make the result GPL: only as a separate GPL mod with published source and credit.
We use **ideas only** for anything in the base game, with our own code, models and textures.

## How we build it (better than the Java version)
1. **Dimension = data**: The Otherside is a dimension entry in the data pack (no sky, fixed dark ambient light, fog color, ceiling)
   using the P5 dimension system; biomes, surface rules and features go through the normal worldgen pipeline, generated in parallel.
2. **Portal** reuses the Nether portal machinery (frame shape + activator item + linking), so the Warden's heart is just data.
3. **Echo tree glow** is block light from the lighting engine plus an emissive texture layer in the shader; no entity or tick per tree.
4. **Sculk blocks and vibrations** reuse our P6 deep dark system (vibration events on the spatial grid); sculk transmitter is a
   vibration listener that forwards container access server-side, no chunk scans.
5. **Mobs** are compositions of the shared ECS AI blocks (vibration-hunter = warden's listening goal, burrow, ambush, swim), GPU-skinned
   with animation LOD; the stalker miniboss gets a WASM behavior and a bossbar.
6. **Ancient temple** is a jigsaw structure; vase-awakening is a block-break event hook spawning the lurker.
7. **Soul elytra** reuses elytra physics with a boost-cooldown component; Warden gear is ordinary armor/tool data with a new material tier.
8. Darkness rendering: low ambient light handled by our lightmap; fog and particles from dimension data, no per-frame CPU work.

## Where it lands
- **P5** (dimension system), **P6** (deep dark, warden, ancient city): The Otherside, biomes, blocks, temple, mobs.
- **M3/M4**: good reference "dimension mod" for the mod API (data-only dimension + a few WASM behaviors).
