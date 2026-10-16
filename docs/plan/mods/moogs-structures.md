# Moog's Voyager Structures + Moog's End Structures (by finndog_123 and team)

**What it is:** two structure packs from the same team, both built only from vanilla blocks and entities.
- **Voyager Structures (MVS):** 160+ Overworld/Nether structures (ruins, towers, camps, ships, small settlements...) holding loot chests,
  hostile mobs and villagers. Server-side only; every aspect (spacing, biomes, loot) tweakable via data packs, with a config pack template.
- **End Structures (MES):** a set of new End structures (vanilla style, tough enemies, valuable loot) to fill the empty outer islands.
  Datapack-configurable, works alongside MVS; an optional pack swaps some blocks for Biomes O' Plenty ones.
- Both ship a 3D web viewer of all structures.

**License:** LGPLv3 (both). → Copyleft: anything we convert from their files (structure templates, data packs) must stay LGPLv3,
shipped as a separate replaceable pack with the notice and credit; our engine is unaffected. We don't read their code (there is
little: it's mostly data). Default plan: convert the structure files into an optional `moogs` pack; or design our own if we want it in core.

## How we build it (better than the Java version)
1. Pure data: structure pieces + template pools + placement rules for our **jigsaw structure system** (P3). No code, like Towns and Towers.
2. **Structure converter tool** (`.nbt` → our format, block-ID mapping) imports the 160+ templates in one batch; output stays in
   an LGPL pack folder `packs/moogs/`, separate from `packs/vanilla`.
3. 160 structure types normally hurt Java worldgen (each structure set checked per chunk). Our structure starts use a **grid-hash
   lookup per region** and run on **parallel worldgen** workers, so the cost is per-candidate, not per-structure-type.
4. Spacing/biome/loot overrides are plain data-pack layering (later pack wins), so their "config pack template" becomes a normal pack.
5. End structures need End outer-island terrain first; placement uses biome tags (`#is_end_highlands`) so End biome mods also get them.
6. Loot via our **data-driven loot tables**; mob spawns inside structures via structure spawn overrides (no per-tick scanning).

## Where it lands
- Voyager: **P3** (jigsaw + converter, as a second data-only pack after Towns and Towers). End: **P5** (End dimension). Pack format: **M1**.
