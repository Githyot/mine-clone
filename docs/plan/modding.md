# Modding plan — how mods get into our game

HP will bring mods (Fabric/Forge/NeoForge Java mods or ideas) and we reformulate them in Rust. This doc says how.

## 1. The rule that makes this work

**The base game is itself a mod.** Every block, item, mob, recipe and biome in the vanilla inventory is registered through the same API
a mod uses. If the API can express vanilla, it can express most mods. This is how Minecraft's data packs and Forge's registries work, and
how MinecraftClone-Unity's config + Lua design works.

```
crates/
  engine/        rendering, ECS, chunks, networking, IO (no gameplay)
  api/           stable mod API: registries, events, behaviors, components
  vanilla/       the base game, written ONLY against api/  ← proves the API
  mods/<name>/   native Rust mods, same shape as vanilla/
  wasm-host/     loads .wasm mods, exposes api/ to them
packs/
  vanilla/       data: blocks.ron, items.ron, recipes/, loot/, worldgen/, lang/
```

## 2. Three ways to add a mod

| Layer | Use for | Speed | Safety |
|---|---|---|---|
| **Data pack** (RON/JSON/TOML) | new blocks, items, recipes, loot, biomes, structures, mob variants, textures | n/a (data) | totally safe |
| **WASM mod** | new behaviors, events, simple AI, commands, UI screens | ~1.2–2× native | sandboxed, can be downloaded from strangers |
| **Native Rust crate** | big systems: tech mods, new dimensions, renderers, worldgen | native | trusted code only, compiled with the game |

Rule of thumb: start a port as data + WASM; promote to native only when profiling says it matters.

## 3. Porting workflow for a Java mod

1. **License check** — MIT/Apache/BSD: we can port code with attribution. LGPL/GPL: porting the code makes our file GPL; prefer reimplementing from the mod's documented behavior. All-rights-reserved: ideas only, no code reading for translation.
2. **Inventory the mod** like we did for vanilla: blocks, items, entities, systems, config, worldgen, network packets.
3. **Map each piece** to a layer (table above). Mixins that patch vanilla internals become either an **event hook** in our API or an engine option.
4. **Write the data files**, then the behaviors, then tests (a headless server test world that checks the behavior).
5. **Benchmark** the world with and without the mod; mods must not cost more than their budget.

## 4. API surface (first version, grows with the roadmap)

- Registries: blocks, block states/properties, items, data components, entities, biomes, features, structures, recipes types, menus, sounds, particles, commands
- Block behavior hooks: on_place, on_break, on_use, on_neighbor_update, on_random_tick, on_scheduled_tick, on_entity_inside, get_redstone_power, get_collision_shape
- Item hooks: on_use, on_use_on_block, on_hit_entity, on_inventory_tick, tooltip
- Entity: components + AI goals/behaviors registered by ID
- Events: player join/leave, chunk load/generate, tick start/end, damage, death, crafting, explosion, chat, command
- Client: models, textures, shaders, HUD layers, screens, keybinds
- Networking: custom typed messages, synced automatically
- Shared block components found while planning mods: seat, storage block, contact damage (cactus, berry bush), emissive texture layer, `pass_through_attacks` tag, connected-texture rules

## 5. Performance mods are different

Mods like Sodium, Lithium, Starlight, FerriteCore, C2ME, Entity Culling, Distant Horizons, Alternate Current, Krypton are not "content".
We don't port them as mods; their **ideas are already built into the engine** (see `architecture.md` section 5–6). If HP brings one,
we compare it against our engine and adopt any idea we're missing.

## 6. Popular content mods worth planning for (examples, to check later)

Create (mechanical contraptions), Biomes O' Plenty / Terralith (worldgen), Farmer's Delight (food), Twilight Forest (dimension),
Applied Energistics / Refined Storage (storage networks), Mekanism / Thermal (tech), Alex's Mobs (mobs), Waystones, JEI (recipe viewer).
Each one tests a different part of the API, so they are good targets for the first ports.
