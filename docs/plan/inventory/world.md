# Inventory: World — dimensions, biomes, structures, generation

Reference: Java Edition 26.2. `[P#]` = roadmap phase.

## 1. Dimensions

| Dimension | Height | Notes |
|---|---|---|
| Overworld [P1–P3] | Y −64 to 320 (384 tall) | day/night, weather, sky light |
| Nether [P5] | Y 0 to 256, bedrock roof | 1:8 coordinate scale, no water, ceiling |
| The End [P5] | Y 0 to 256 | main island + outer islands past 1,000 blocks, dragon fight |

World types: default, superflat (customizable), large biomes, amplified, single biome, debug. Seeds (64-bit).

## 2. Biomes (66)

**Overworld surface (49):**
plains, sunflower plains, snowy plains, ice spikes, desert, swamp, mangrove swamp, forest, flower forest, birch forest, old growth birch forest,
dark forest, pale garden, taiga, snowy taiga, old growth pine taiga, old growth spruce taiga, savanna, savanna plateau, windswept savanna,
windswept hills, windswept gravelly hills, windswept forest, jungle, sparse jungle, bamboo jungle, badlands, eroded badlands, wooded badlands,
meadow, cherry grove, grove, snowy slopes, frozen peaks, jagged peaks, stony peaks, river, frozen river, beach, snowy beach, stony shore,
mushroom fields, ocean, deep ocean, warm ocean, lukewarm ocean, deep lukewarm ocean, cold ocean, deep cold ocean, frozen ocean, deep frozen ocean.

**Overworld underground (4):** dripstone caves, lush caves, deep dark, sulfur caves (26.2).

**Nether (5):** nether wastes, soul sand valley, crimson forest, warped forest, basalt deltas.

**End (5):** the end, end highlands, end midlands, small end islands, end barrens. Plus **the void** (superflat).

Each biome = data: temperature, downfall, grass/foliage/water/sky/fog colors, mob spawn lists, features, carvers, music, ambient particles/sounds.

## 3. Generation pipeline (what modern Minecraft does, conceptually)

1. **Climate noise** — 6 parameters (temperature, humidity, continentalness, erosion, weirdness, depth) pick the biome in 3D.
2. **Density function** — 3D noise router decides solid vs air: continents, mountains, rivers, cave "cheese", "spaghetti" and "noodle" caves.
3. **Aquifers** — local water/lava levels underground.
4. **Ore veins** — giant copper (upper) and iron (lower) veins.
5. **Surface rules** — grass/sand/snow/terracotta bands, depending on biome and depth.
6. **Carvers** — classic caves and ravines.
7. **Structures** — starts and pieces, jigsaw assembly.
8. **Features** — ores, trees, flowers, lakes, geodes, dripstone, icebergs, coral, kelp... ordered in 11 decoration steps.
9. **Light + heightmaps**, then spawn initial mobs.

We reimplement this from the publicly documented behavior (Minecraft Wiki's world generation pages), with our own noise code. **We do not need
seed-for-seed parity**, so we can choose faster noise (SIMD, GPU compute) as long as the worlds feel the same.

## 4. Structures [P3, P5, P6]

| Overworld | Nether | End |
|---|---|---|
| villages (plains, desert, savanna, taiga, snowy) | nether fortress | end city (+ end ship) |
| pillager outpost | bastion remnant (housing, stables, treasure, bridge) | obsidian pillars + exit portal |
| woodland mansion | nether fossil | end gateways |
| desert pyramid, jungle pyramid, swamp hut, igloo | ruined portal (nether) | |
| ocean monument, ocean ruins (warm/cold), shipwreck, buried treasure | | |
| mineshaft (+ badlands variant), stronghold, monster room (dungeon) | | |
| ruined portal (all variants), trail ruins, ancient city, trial chambers | | |
| desert well, fossils, amethyst geodes | | |

## 5. Features (decorations)

trees (every species, incl. fancy oak, mega spruce/jungle, dark oak, mangrove, cherry, pale oak, azalea), huge mushrooms, huge fungi, bamboo,
flower patches, grass/fern patches, lava/water lakes, springs, ores (blob placement per height curve), disks (sand, clay, gravel), ice spikes,
icebergs, blue ice, coral reefs, kelp, seagrass, sea pickles, vines, glow lichen, dripstone clusters, lush cave vegetation, sculk patches,
basalt columns/pillars, blackstone blobs, delta features, chorus plants, end islands, fossils, geodes, fallen trees, leaf litter, firefly bushes,
sulfur pools and geysers (26.2).

## 6. Environment simulation [P3]

day/night (24,000 ticks), 8 moon phases, rain/snow/thunder, lightning (fire, charged creepers, skeleton horse trap), snow & ice accumulation,
cauldron filling, random ticks (crop growth, leaf decay, grass spread, ice melt, fire spread, copper oxidation), world border.
