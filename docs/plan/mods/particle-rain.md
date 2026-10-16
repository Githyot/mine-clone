# Particle Rain (by PigCart)

**What it is:** replaces vanilla's flat rain/snow sheets with real particle weather: rain falls at an angle pushed by wind (stronger in
storms), snowflakes rotate as they fall, sandstorms with dust and tumbling shrubs in dry biomes (tinted by the ground block), rain tinted by
biome water color, ripples where rain hits water, streaks running down walls, mist rising after rain in wet/spooky biomes, fog. In-game
editor to create custom weather effects per block or biome. (Puddles moved to a separate mod.)

**License:** MIT. → Code may be translated with credit; we write it on our own particle system.

## How we build it (better than the Java version)
In Java every raindrop is a CPU particle object updated and sorted on the main thread, which is why heavy weather costs FPS.
1. **All weather particles simulate on the GPU** (compute shader, the same GPU particle system as spells in `../magic-and-rpg.md`).
   Hundreds of thousands of drops cost about 0.2 ms; the CPU only uploads wind, camera position and a rain-occlusion height map.
2. **Rain occlusion map**: a small top-down height texture around the player (from chunk heightmaps we already keep) tells the
   shader where drops stop, so rain never falls through roofs and ripples/splashes spawn exactly on the surface.
3. **Wind** is one global vector + noise field, shared with leaves swaying, falling leaf litter, smoke and flags.
4. **Ripples and wall streaks** are decals in the deferred/forward pass using the same occlusion map, no extra particles.
5. **Sandstorms/mist/fog** are volumetric fog parameters per biome (data), blended with Serene Seasons' season uniform (`serene-seasons.md`).
6. Effects are data files (biome + block → effect), so the in-game editor just writes our data format.
7. Quality slider: particle count and distance scale down automatically if the frame budget is exceeded.

## Where it lands
- **P3** (weather): GPU rain/snow, occlusion map, wind, biome tint. **P8**: sandstorms, mist, ripples/streaks, editor.
