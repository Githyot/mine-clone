# Explorify – Dungeons & Structures (by bebebea_loste)

**What it is:** a data-pack-only structure set of "interesting yet familiar" vanilla-style structures that fit naturally into the
world instead of random houses. From the gallery: supply caches, ruins, mangrove huts, a dark forest set, taverns, watchtowers,
mausoleums, guideposts, farmsteads, desert shrines, plus small dungeons with loot. Configured through the data pack (config screen
with Cristel Lib). Still in development: structures may change between versions.

**License:** All Rights Reserved. → **Ideas only.** We build our own structures in the same spirit; no conversion of their files.

## How we build it (better than the Java version)
1. Pure data on our **jigsaw structure system** (P3): small structures are single-piece or 2–3 piece pools with biome-tag placement,
   loot from **data-driven loot tables**. Good second test of "mods as data packs" next to Towns and Towers.
2. Many small structures = many placement candidates. Our grid-hash structure starts on **parallel worldgen** keep the cost flat;
   small single-template structures skip jigsaw assembly entirely (direct template paste path).
3. Terrain fitting (foundations/legs down to ground, beardifier-style terrain smoothing) is an engine feature, so small structures
   never float or cut into hills, without per-structure code.
4. Versioned structure IDs: when a pack update removes or changes a structure, already-generated ones stay as blocks and
   un-generated starts are dropped cleanly (no world corruption, no backup needed).
5. Config = data-pack layering (spacing, biomes, disable per structure), editable in the in-game pack settings screen.

## Where it lands
- **P3** (our own vanilla-style small structures as data, built with the jigsaw system); pack settings screen with **M1/M5**.
