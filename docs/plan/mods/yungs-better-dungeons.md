# YUNG's Better Dungeons (by YUNGNICKYOUNG and team)

**What it is:** a full redesign of Minecraft's dungeons:
- Replaces vanilla small dungeons with varied redesigned versions (themed per biome, more loot and spawners).
- Adds three big dungeons: **Catacombs** (huge underground tomb network), **Fortresses of the Undead** (multi-floor undead castles),
  **Spider Caves** (cave networks full of webs and spiders).
- Opt-in small **Nether dungeons**.
- Each dungeon tweakable via config or data pack; data pack to keep vanilla dungeons too; compatible with YUNG's other structure mods
  and Repurposed Structures. Requires YUNG's API.

**License:** LGPLv3. → Copyleft: converted structure files or translated logic must stay LGPLv3 in a separate, replaceable pack/module
with the notice and credit; our engine is unaffected. We don't read their code; we design dungeons ourselves and only import their
structure files (via the converter) if we want an optional `yungs-dungeons` pack.

## How we build it (better than the Java version)
1. Small dungeons: a data-driven **feature** with multiple room templates and biome-tag variants, replacing the hardcoded vanilla one.
2. Big dungeons via our **jigsaw structure system** (P3): rooms, corridors and stair pieces in template pools with depth limits.
   Java mods like this need custom piece code; we add the few missing jigsaw features once (vertical placement in caves, "fit inside
   solid rock" checks, max-count per piece) so these become pure data.
3. Spider Caves are carved, not placed: a data-defined carver + decoration rules (webs, egg sacks) run on **parallel worldgen**.
4. Layout is solved once per structure start on a worker thread and cached per chunk (bounding-box lookup), so huge catacombs
   don't stall chunk generation the way large Java structures can.
5. Loot through **data-driven loot tables**, spawners as block-entity data; all tunables are data-pack fields, no config file.
6. Optional "keep vanilla dungeons" = just another pack toggle.

## Where it lands
- **P3** (dungeons + jigsaw extensions, as our own content); Nether variants **P5**; optional converted pack after **M1** converter.
