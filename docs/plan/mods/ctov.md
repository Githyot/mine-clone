# ChoiceTheorem's Overhauled Village — CTOV (by ChoiceTheorem)

**What it is:** structure pack with 21–23 new village variants and 11–14 pillager outpost variants, each matching its biome's terrain
and theme (the page gives both counts). Newer versions add its own structures alongside vanilla villages instead of replacing them.
Uses vanilla biome tags, so villages also appear in biomes from Terralith, BOP, etc. Mostly vanilla loot tables plus some custom ones
(food, armor). Needs Lithostitched. Works with Towns and Towers, YUNG's and other structure mods.

**License:** custom license, terms not stated on the page. → **Ideas only** until the terms are read; our own builds.

## How we build it (better than the Java version)
1. Same path as `towns-and-towers.md`: **pure data on our jigsaw system** (pieces, template pools, biome tags), no code.
2. Villages from CTOV, Towns and Towers and vanilla share one **village registry** with weights per biome, so stacking several
   village packs never makes villages spawn 3× as often or overlap (a common problem when combining these mods in Java).
3. **Terrain fitting**: village pieces adapt to slopes using our structure terrain-adaptation step (beardifier-like) computed in the
   parallel worldgen jobs, so hillside villages don't float or bury paths.
4. Custom villager professions or guards come through the villager registry (P3) rather than separate compat packs.

## Where it lands
- **P3** (villages + jigsaw), with **M1** data packs.
