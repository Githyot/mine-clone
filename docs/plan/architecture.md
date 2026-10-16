# Engine architecture — fast first, mod-friendly always

Goal: a Minecraft-like game in Rust that is dramatically faster than Java Minecraft on the same hardware, and where every piece of content
(including the base game) is loaded the same way mods are.

## 1. Performance targets (we test these in CI from day one)

| Metric | Target (mid-range PC: 6-core CPU, GTX 1660 / RX 6600 class) | Vanilla Java, for comparison |
|---|---|---|
| Frame rate at 32-chunk render distance | 240+ FPS | ~60–120 with Sodium |
| Visible distance with LOD | 128+ chunks (2 km) | 32 chunks max |
| Chunk generation | 1,000+ chunks/s across all cores | ~50–150 chunks/s |
| Chunk meshing | < 0.2 ms per 16³ section | several ms |
| Server tick (20 TPS) with 4 players + 500 entities | < 5 ms (budget is 50 ms) | 15–40 ms |
| Memory for 32-chunk view | < 1.5 GB | 3–6 GB |
| Cold start to in-world | < 3 s | 20–60 s |
| Save of a full world | incremental, never blocks the game thread | autosave stutter |

If a change makes a benchmark slower by > 3%, CI fails unless the PR explains why.

## 2. Tech stack

| Concern | Choice | Why |
|---|---|---|
| Language | Rust (stable), `opt-level=3`, LTO `fat`, `codegen-units=1`, `panic=abort` for release, PGO builds for releases | The video we studied lists compiler optimization as a big win; PGO + LTO typically gives 10–20% for free |
| Window/input | `winit` | cross-platform, standard |
| GPU | `wgpu` on the **Vulkan** backend (Metal on Mac, DX12 as fallback). No Java, no JVM anywhere. If profiling ever shows wgpu overhead, the renderer is isolated so it can be swapped for raw Vulkan via `ash` | native GPU speed, compute shaders, one codebase for every OS |
| ECS | `bevy_ecs` used **standalone** (not the full Bevy engine) | fast archetype ECS, parallel scheduler, we keep control of the loop |
| Parallelism | `rayon` + a custom job system with priorities (chunk gen > meshing > lighting > saving) | uses every core |
| Math | `glam` (SIMD) | fast vectors |
| Noise | own SIMD noise (or port FastNoise2 ideas), optional GPU compute path | worldgen is the #1 CPU cost |
| Audio | `kira` or `rodio` | spatial sound |
| UI | own immediate-mode UI on wgpu (or `egui` for debug/dev tools) | small, fast |
| Networking | custom binary protocol over QUIC (`quinn`) + TCP fallback | encryption + streams built in |
| Saving | own region format: 32×32 chunks per file, paletted sections, `zstd` compression, async I/O | fast load/save |
| Scripting (mods) | WebAssembly via `wasmtime` | sandboxed, any language, near-native speed |
| Profiling | `tracy` integration, built-in F3 graphs, `criterion` benchmarks | measure everything |

## 3. Process layout (same as Minecraft, but parallel)

Singleplayer always runs an **integrated server** on its own threads, talking to the client through an in-memory channel using the same
messages as multiplayer. This keeps one code path for both and makes multiplayer "free" later.

```
Client (render thread + worker pool)        Server (tick thread + worker pool)
  input → prediction                           20 TPS fixed tick
  mesh builder workers                         region-parallel entity/block ticking
  LOD builder workers         <— channel / QUIC —>  worldgen workers
  GPU upload queue                             lighting workers
  audio thread                                 save/IO thread
```

## 4. World data

- **Chunk column** = 16×16 horizontally, split into **sections** of 16×16×16 (Overworld height 384 = 24 sections).
- **Block state** = `u16` global ID (65k states is plenty; Java has ~27k). Each section uses a **palette**: 0 bits (single block, e.g. all air),
  4, 8 bits or direct 16 bits. Most sections end up tiny.
- **Light** stored per section as 4-bit nibbles, skipped for empty/all-solid sections.
- **Block entities** (chests, signs) in a sparse hash map per chunk; **entities** in the ECS, indexed by a spatial grid.
- **Registries** (blocks, items, biomes, entities...) are frozen after load into flat arrays, so lookups are array indexing, never hashing.
- Block properties used in hot loops (opaque, full cube, light emission, collision shape ID) are packed into a dense table indexed by state ID.

## 5. Rendering (where most of the speed comes from)

1. **Face culling** — only faces touching air/transparent blocks are emitted (Craft does this; it removes ~90% of faces).
2. **Binary greedy meshing** — use 64-bit bitmasks per row to find visible faces and merge same-texture faces into big quads (the
   "greedy meshing" from the video). Bitwise meshing runs in ~50–200 µs per section.
3. **Compact vertices** — pack a quad into 8 bytes (position 6 bits ×3, size, face, texture ID, AO, light). GPU expands it with vertex pulling.
4. **GPU-driven drawing** — all sections live in one big buffer; a compute shader does frustum + occlusion culling and writes indirect
   draw commands (multi-draw-indirect). The CPU issues a handful of draw calls per frame regardless of view distance.
5. **Cave/occlusion culling** — per-section connectivity graph (which faces can see which, like Sodium/Minecraft's visibility graph) plus
   Hi-Z depth occlusion on the GPU, so underground caves behind walls are never drawn.
6. **Level of detail** — beyond ~16–32 chunks, render simplified 2×, 4×, 8×, 16× downsampled meshes (idea from Distant Horizons and from
   the video). LODs are built from saved data, cached on disk.
7. **Texture array** for block textures (no atlas bleeding, no rebinds). Animated textures updated with one small upload.
8. **Separate passes**: opaque, cutout (leaves, using alpha test, like Craft's discard trick), translucent (water, glass) sorted per section.
9. Mesh rebuilds are **incremental**: a block change remeshes only its section (+ neighbor if on the border), never the whole chunk (Craft
   rebuilds whole 32×32 chunks; we avoid that).

## 6. Simulation (server side)

- **Fixed 20 TPS** to match Minecraft's mechanics (redstone timing, crop growth etc. depend on it). Client renders interpolated at any FPS.
- **Region-parallel ticking**: the world is split into regions; regions that are far apart tick on different cores at the same time
  (idea from Folia). Interactions crossing regions go through a message queue.
- **Lighting** with a BFS flood-fill engine that runs in parallel and batches updates (idea from the Starlight mod — 10–100× faster than vanilla).
- **Redstone** compiled into a graph: wire networks are precomputed and only updated when wires change (ideas from Alternate Current / Redstone
  Multimeter mods). Must still match vanilla behavior for common contraptions.
- **Entities**: ECS systems run in parallel; AI goal evaluation is throttled for far-away mobs (entity activation range idea from Paper);
  pathfinding results are cached and computed off-thread.
- **Hoppers, item merging, collision** use the spatial grid; idle hoppers sleep until notified (Lithium idea).
- **Random ticks** iterate only non-empty sections that contain tickable blocks (tracked in a per-section counter).

## 7. World generation

- Same conceptual pipeline as Minecraft (see `inventory/world.md`), reimplemented from public documentation.
- Every stage is a pure function of (seed, chunk position, neighbors' earlier stages) → trivially parallel.
- Noise sampled on a coarse grid (4×8×4 cells) and trilinearly interpolated, like Minecraft, but with SIMD (8 lanes at a time).
- Optional compute-shader path for density sampling when the GPU is idle.
- Craft saves only player changes and regenerates terrain on load. We store generated chunks instead, because loading is faster than
  regenerating once worldgen is complex, but we keep Craft's **delta sync for multiplayer**: the server sends only changes since the client's cached version.

## 8. Networking

- Server-authoritative; client predicts its own movement and block placing, server confirms or corrects.
- Chunks sent compressed with zstd and a shared dictionary; client caches chunks keyed by (position, version) so revisits cost nothing.
- Entity updates are delta-encoded and prioritized by distance; far entities update at a lower rate.
- Interpolation of other players between last two updates (same as Craft).

## 9. Modding architecture (summary; full design in `modding.md`)

Three layers, all using the same registries the base game uses:
1. **Data packs** — JSON/TOML/RON: blocks, items, recipes, loot, worldgen, mob variants. No code.
2. **WASM mods** — sandboxed scripts calling a stable API (events, block behaviors, entity AI goals, UI). Safe to download.
3. **Native Rust modules** — compiled into the game for maximum speed; used when we port a big mod properly into core.

## 10. What we learned from the references HP shared

| Reference | What we take |
|---|---|
| Video "My game is 262,000 times faster than Minecraft" (channel IGoByLotsOfNames) | The 5 pillars: culling, compiler optimization, LOD, greedy meshing, multithreading. All five are in sections 2, 5 and 6 above and in Phase 0/1 of the roadmap. |
| fogleman/Craft (C, MIT) | Exposed-face-only meshing, frustum culling, background batched saving, delta sync for multiplayer chunks, interpolated remote players, alpha-discard for cutout textures. What we do better: per-section remeshing instead of whole chunk, GPU-driven culling, binary protocol instead of ASCII. |
| stalomeow/MinecraftClone-Unity (C#, MIT) | Data-driven block/biome/item config, block behaviors as scripts with hooks (on place, destroy, click, tick, collision), gravity and fluid "templates", fluid flow limits, async asset loading with stable IDs (GUIDs). This maps directly to our data pack + WASM layers. |
| Leaked Minecraft source | **Not used.** We rely only on public documentation (Minecraft Wiki, official changelogs) and open-source projects, so nothing in our code can be called a copy. |

## 11. Ground rules

- Never copy Mojang code, textures, sounds, music or names of brand content; mechanics and ideas are fine, assets are ours.
- When porting a mod, check its license first (MIT/Apache = fine with attribution; GPL/LGPL = our port would have to be GPL; ARR = ideas only).
- Every system has a benchmark before it has features.
