# Aquaculture 2 (by Shadow, with Girafi)

**What it is:** a fishing overhaul.
- **~32 fish species by biome**: freshwater (9: bass, bluegill, trout, carp, catfish, gar, minnow, muskellunge, perch), arid (4),
  arctic ocean (8: cod, halibut, herring, pollock, salmon...), saltwater (3: jellyfish, grouper, tuna), jungle (4: arapaima, piranha...),
  swamp (2: leech, box turtle), mushroom island (2 shroomas). Biomes with no fish list fall back to freshwater fish.
- **Fish have weight**; filleting yields depend on species/weight. Fish can be placed as mounted trophies.
- **Modular rods**: new rods with slots for hook, bait, fishing line and bobber (all customizable/dyeable); vanilla and modded rods still work.
  Hooks change behavior (e.g. heavy, light, double, redstone, note, gold); baits speed up bites.
- **Tackle box** block to apply rod parts and store spares.
- **Loot**: driftwood, tin can, message in a bottle, algae, goldfish, box / lockbox / treasure chest, Neptune's Bounty, rare
  **Neptunium** tools and armor.

**License:** All Rights Reserved (no decompiling, no reuse of artwork). → **Ideas only.** Our own code, models and textures.

## How we build it (better than the Java version)
1. **Fish tables are biome data**: each biome (or biome tag) in the data pack lists its fish pool with weights, weight ranges and
   time/weather conditions. No hard-coded biome checks; modded biomes inherit via tags, not a "default to freshwater" fallback.
2. **Catch roll is one loot-table evaluation** at bite time, using the same loot engine as vanilla fishing (fish / junk / treasure
   splits are just weighted pools). Rod parts are **loot modifiers** (luck, treasure bias, bite-time multiplier), composed once when
   the rod is cast and cached on the bobber entity.
3. **Rod parts as item data components** (hook, bait, line, bobber IDs + bait durability), not NBT inventories inside the item.
   Line/bobber color is a shader tint on the bobber/line mesh, zero extra textures.
4. **Bobber is a lightweight entity** with a fixed-step water check on the fluid grid; bite timer is a scheduled event, not a per-tick
   countdown with random rolls every tick.
5. **Live fish mobs** use the shared ECS aquatic AI building blocks (schooling/boids on the spatial grid, avoid player, flee) and
   instanced GPU-skinned rendering; 30 species are 30 data variants of ~4 body rigs, with animation LOD for schools.
6. **Fish weight** stored as a small component on the item stack (u16), used by fillet recipe and trophy display; leaderboard per world.
7. Hooks with behavior (redstone hook, note hook) are event hooks (`on_bite`, `on_reel`), so WASM mods can add hooks.

## Where it lands
- **P3** (fishing + biomes exist): fish data, biome pools, loot, fillets, live fish mobs.
- **P3–P6**: modular rods, tackle box, hooks; Neptunium gear after smithing/enchanting (P6).
- **M2**: rod-part registry and `on_bite`/`on_reel` hooks exposed to mods.
