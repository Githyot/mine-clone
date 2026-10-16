# Towns and Towers (by Biban_Auriu, Kubek, Cristelknight)

**What it is:** structure pack with 50+ structures: 24 villages (8 new types: jungle, badlands, forest, swamp, sunflower plains, sparse jungle,
snowy slopes, old growth taiga, meadow, mediterranean, swedish, alpha islands, beach, viking, rustic, tudor, flower forest, birch forest, grove,
mushroom), 28 pillager outposts (towers with 1 allay prisoner, forts with 2, camps), 3 ships (villager fleet, pillager galleon, flotsam), a
wandering trader camp, and a secret structure. Compatible with worldgen mods (Terralith, BOP, BYG) to place structures in their biomes.

**License:** Creative Commons 4.0 (exact variant, e.g. BY or BY-NC, to confirm on their license page). → If BY: we may convert their structure
files into our format with credit. If BY-NC: only in a non-commercial release. Until confirmed: ideas only.

## How we build it (better than the Java version)
1. This is pure data, which is exactly what our **jigsaw structure system** (P3) is for: structure pieces + template pools + biome lists.
   No code needed, so it's the ideal first test of "mods as data packs".
2. **Structure converter tool**: reads Minecraft structure files (`.nbt`) and maps block IDs to ours. Useful for any structure mod whose license allows it.
3. Structure placement uses our parallel worldgen; structure start checks use a cheap grid hash so 50 extra structure types add no
   noticeable generation cost (Java checks every structure set per chunk, which slows generation with big packs).
4. Biome tags (e.g. `#is_jungle`) instead of fixed biome lists, so structures automatically appear in biomes added by other mods.

## Where it lands
- **P3** (villages + jigsaw) as the first data-only mod; converter tool with **M1**.
