# Snow! Real Magic! (by Snownee)

**What it is:** realistic snow.
- Snow can sit **in the same block** as fences, walls, tall grass and plants; **snow-covered** slabs, stairs, walls, fences, gates
  (also generated naturally).
- Snow layers **fall like sand**; falling snow that touches water becomes ice.
- Layers **build up** during snowfall/snowstorms; can be set to never melt, melt in warm biomes, or melt naturally over time.
- Thinner collision box when walking on snow; **landing on snow reduces fall damage** by layer count.
- Blocks can be placed straight into non-full snow (it is replaceable).
- Shovel right-click or sneak-use with empty hand scrapes one layer off as a snowball; empty-hand right-click toggles a fancy overlay.
- Optional blizzard joke feature (game rule). Most of it is configurable; data packs customize block tags and snowy models.

**License:** MIT. → **Code may be translated with credit** to Snownee (keep the MIT notice). We still prefer our own design (below)
and our own textures/models.

## How we build it (better than the Java version)
Java can't store two blocks in one position, so the mod replaces the fence/plant with a block entity or a special block wrapping the
original. That means more block entities, model baking for every combination, and compatibility patches for other mods.
1. **"Snowlogged" is a generic layer, like waterlogging**: block states have an optional `cover` property (`none`, `snow_1..8`) on
   any block whose shape tag allows it. It is a palette entry, not a block entity, so it costs 0 extra bytes for unchanged sections.
2. **Meshing**: the mesher draws the base block, then the snow layer quad set from the cover value (greedy-merged with neighbors).
   No pre-baked "snowy fence" models; mods' blocks get snow for free via the `snow_coverable` tag.
3. **Accumulation via the random-tick pass**: during snowfall, only sections with a sky-exposed top (heightmap) and temperature
   below 0.15 (biome + season offset from Serene Seasons) roll for +1 layer, with a max height rule. Melting runs on the same pass
   (light level ≥ 12 or warm biome/season). No chunk scans.
4. **Snow gravity** uses the shared falling-block system (sand), with "touches water → ice" as data in the falling-block rule.
5. **Physics**: layer height is already the collision shape; the fall-damage reduction is a per-block `fall_damage_multiplier`
   property (as hay bales and beds use).
6. **Replaceable when not full** and shovel scraping are plain block-behavior data (replaceable predicate, use-item rule with loot).
7. **Worldgen**: the "freeze top layer" feature step writes `cover=snow_1` on coverable blocks directly.
8. Fancy overlay toggle = a client render option, not a per-block state.

## Where it lands
- **P1** (shape templates, waterlogging-ready state model): reserve the `cover` property design next to `waterlogged`.
- **P3** (weather, snow/ice, worldgen features): accumulation, melting, snowy features, falling snow, fall-damage property.
- **M1**: tags `snow_coverable` + data-driven rules so mods opt in. Budget: snowfall over 32-chunk view < 0.2 ms extra tick.
