# When Dungeons Arise (by Aureljz and team)

**What it is:** huge, hand-built structures (often hundreds of blocks across) with procedurally varied rooms, mobs and custom loot:
temples, palaces, fortresses, cities, hostile ships at sea, flying airships and planes (some with redstone loot), hidden mines.
Named structures include Coliseum, Keep Kayra, Shiraz Palace, Typhon, Plague Asylum, Monastery, Foundry, Lighthouse, Thornborn Towers,
Heavenly Challenger/Conqueror/Rider (sky structures), Illager Fort/Windmill/Corsair/Galley, Undead Pirate Ship, Mushroom Village/Mines,
Bandit Village/Towers, Jungle Tree House, Aviary, Bathhouse, Wishing Well, and more (~39). Biome-tag placement (works with BOP/BYG),
`/locate` support, data-pack config, server-side only.

**License:** All Rights Reserved. → **Ideas only.** Our own structures; no conversion of their files.

## How we build it (better than the Java version)
1. **Jigsaw structure system** for room/corridor variation, plus support for **very large single templates** split into
   chunk-sized tiles at load time (stored compressed, palette-indexed like our chunk sections).
2. **Streaming placement across chunks:** the layout is solved once per structure start on a worker thread (cached), then each chunk
   pastes only the tiles intersecting it during **parallel worldgen**. No "generate the whole 300-block structure in one chunk" stall
   that Java giants cause; neighbouring chunks never wait on each other.
3. Terrain adaptation as an engine step (terrain smoothing under/around the bounding box) so palaces don't hang over cliffs.
4. **LOD visibility:** big structures appear in our long-distance LOD (P8) as soon as their chunks generate; sky structures
   (airships, Heavenly series) get a coarse LOD mesh from the template itself, so you can see them from far away before arriving.
5. Floating sky structures skip terrain checks; ships use a "water surface" placement mode (data flag).
6. Mobs inside spawn once at generation as persisted entities with a "structure guardian" tag (no per-tick spawn scanning); loot via
   **data-driven loot tables**; `/locate` uses the same grid-hash start lookup (instant, no chunk generation).
7. Large structure starts are budgeted on the job system (priority below player-near chunks) so they never cause frame or tick spikes.

## Where it lands
- **P3** (jigsaw + large-template tiling + streaming placement); sky/sea variants **P3–P5**; LOD visibility **P8**; `/locate` **P7**.
