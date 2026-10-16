# Serene Seasons (by TheAdubbz, with Forstride)

**What it is:** a yearly season cycle.
- **4 seasons × 3 sub-seasons** (early/mid/late), configurable length; a **season sensor** block (redstone output) and **calendar** item.
- **Grass and foliage colors** shift through the year (greens in summer, orange/brown in autumn, pale in winter).
- **Temperature** drops in winter: snow falls and water freezes in biomes that normally have none; it melts in spring.
- **Crops** have fertile seasons (data tags); out of season they grow slowly or not at all. Greenhouses (glass overhead) bypass this.
- **Weather** varies by season (more rain in spring, more thunderstorms in summer, snow in winter).
- Tropical biomes use wet/dry seasons; per-dimension enable; option to keep seasons per world, synced to all players.

**License:** All Rights Reserved. → **Ideas only.** Our own code and assets.

## How we build it (better than the Java version)
In Java, changing foliage color means **re-meshing every loaded chunk** when the season ticks over, and the snow/melt logic scans
chunks. Ours:
1. **Seasons tint on the GPU**: biome grass/foliage/water colors are already per-vertex biome indices into a color lookup. The season
   is a **uniform** (`season_t` 0..1 over the year) that the shader uses to blend between per-biome season color keys (a small
   texture: biome × season). Season change = one uniform update, **zero chunk rebuilds**, and colors drift smoothly day by day.
2. **Season state** is one world resource (day of year), synced to clients as a single number; clients derive everything else.
3. **Temperature** = biome base temperature + season offset, computed in the existing `temperature_at(pos)` function; snow/ice
   placement and melting piggy-back on the **random tick** pass (only sections with exposed tops/water, using per-section counters).
4. **Crop fertility** is biome/season data (crop tag → fertile seasons); the crop's random tick reads a precomputed per-biome
   growth multiplier table updated once per season change.
5. **Greenhouse check** uses the existing sky-light/heightmap: a crop under a transparent block with sky light is "indoors", no scan.
6. **Weather** odds are per-season data in the biome/climate pack; the weather system just reads the active table.
7. Sub-season and day-of-year exposed through event hooks (`on_season_change`) and a `season` predicate for loot tables, spawns and
   worldgen-free features (mods can add seasonal mobs or drops).
8. Leaf-fall particles and snow-covered leaves are client-side shader/particle effects driven by the same uniform.

## Where it lands
- **P3** (weather, snow/ice, farming, biome colors exist): season clock, GPU tint, temperature offset, crop fertility, weather tables.
- **P4**: season sensor redstone block. **P8**: polish (leaf particles, calendar item art).
- Budget: season transition costs < 0.1 ms client and no mesh work.
