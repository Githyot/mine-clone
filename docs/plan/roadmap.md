# Roadmap — phases from empty repo to full Minecraft

Each phase ends with something **playable** and with **benchmarks green**. Phases can overlap; the order is what depends on what.
`[P#]` tags in the inventory files point here. Rough size is in "work units" (1 unit ≈ one focused PR-sized chunk of work), not dates.

---

## Phase 0 — Engine foundation (~25 units)
Nothing to play yet, but everything after this is faster to build.
- Cargo workspace: `engine`, `api`, `vanilla`, `wasm-host`, `server`, `client`, `bench`
- Release profile: LTO, codegen-units=1, PGO script; CI on Windows/Linux/macOS
- winit window, wgpu renderer, camera, texture array loader
- Chunk + section storage with palettes, block state registry, data-driven block loading from `packs/vanilla`
- Binary greedy mesher + compact vertex format + GPU-driven indirect drawing + frustum culling
- Job system (rayon + priorities), tracy profiling, F3 debug overlay
- Integrated server split (client ↔ server channel) from the very first frame
- Benchmarks: meshing µs/section, FPS at fixed camera path, memory per chunk

**Exit:** fly around a flat/noise world at 32+ chunks with 240+ FPS.

## Phase 1 — Creative sandbox (~30 units)
- Basic noise terrain (heightmap + simple caves), trees, water
- Player physics: AABB collision, walk/sprint/sneak/jump/fly, swim
- Place / break blocks, block picking, hotbar, creative inventory
- All shape templates (stairs, slabs, walls, fences, panes, doors...) driven by data
- Lighting engine (sky + block, smooth light, AO), day/night
- Save/load (region files, zstd, async), world list screen
- Cave/occlusion culling, first LOD level
- Wood sets, stone sets, colored sets (data only)

**Exit:** a creative mode you can build in; worlds save and load instantly.

## Phase 2 — Survival core (~45 units)
- Health, hunger, XP, damage types, death/respawn
- Mining speed + tools + drops + loot tables
- Crafting (2×2, 3×3, recipe book), furnace family, fuel
- Item entities, chests, inventories and menus
- Ores and storage blocks, torches and lights
- Mob framework: ECS components, pathfinding, AI goals, spawning rules, despawning
- First mobs: pig, cow, sheep, chicken + zombie, skeleton, creeper, spider, enderman, slime
- Combat (cooldown, crits, knockback, shields), bow/arrows, armor
- Beds, sleep, spawn point; sounds + particles basics; classic-style music engine (`music.md`)
- Extended ore tiers 1–6 (copper/tin … gems), alloys, life crystals, grappling hook, mob health bars (`terraria-content.md`, `magic-and-rpg.md`)

**Exit:** a real survival night-one experience.

## Phase 3 — Living Overworld (~70 units)
- Full worldgen pipeline: climate noise, density router, aquifers, ore veins, surface rules, carvers, 11 feature steps
- All 53 Overworld biomes, all Overworld features
- Structures: jigsaw system, villages (5), outposts, temples, shipwrecks, ruins, mineshafts, strongholds, dungeons, monuments, mansions, igloos, geodes
- Fluids fully (flow, waterlogging, bubble columns), weather, lightning, snow/ice
- Farming, plants, crops, bone meal, composting, bees
- All passive/neutral Overworld mobs, breeding, taming, horses, fishing
- Villagers: professions, trades, schedules, gossip, golems; wandering trader; raids
- Colored blocks, dyes, banners, maps

**Exit:** world feels like Minecraft; exploration loop works.

## Phase 4 — Redstone and automation (~30 units)
- Power model, wire graph, repeaters, comparators, torches, observers
- Pistons (sticky, slime/honey), dispensers/droppers/hoppers, crafter
- Rails and minecarts, TNT
- Copper family (oxidation, waxing), copper bulbs/golem/chests
- Test suite: a library of classic contraptions (clocks, T flip-flops, item sorters, piston doors, farms) that must behave like vanilla

**Exit:** standard redstone builds from tutorials work.

## Phase 5 — Nether and End (~35 units)
- Portals and linking, dimension system (separate worlds, coordinate scaling)
- Nether biomes, fortress, bastions, all Nether blocks and mobs, piglin bartering
- End: strongholds → end portal, dragon fight + respawn, outer islands, end cities, shulkers, elytra
- Wither boss

**Exit:** you can "beat the game".

## Phase 6 — Progression depth (~40 units)
- Enchanting (table, anvil, grindstone, books), all 43 enchantments
- Brewing, all effects and potions, beacons, conduits
- Smithing + netherite + armor trims
- Archaeology, sniffers, trail ruins
- Trial chambers (trial spawners, vaults, breeze, mace)
- Deep dark (sculk, vibrations, warden, ancient city)
- Advancements, statistics, recovery compass, locator bar
- Magic (mana, wands, staffs, spellbooks, glyph spells), accessories, item modifiers, first boss ladder, town NPCs in player houses
- World Breaker hard-mode shift with ore tiers 10–12 (`terraria-content.md`)

**Exit:** feature parity with Java gameplay.

## Phase 7 — Multiplayer and server scale (~30 units)
- QUIC protocol, login, dedicated server binary, server list
- Prediction/reconciliation, delta chunk sync, entity interest management
- Region-parallel ticking at scale (target: 100 players on one 8-core server at 20 TPS)
- Commands (~85), permissions, game rules, scoreboards, teams, bossbars
- Creative/technical blocks (command blocks, structure blocks, jigsaw)

**Exit:** public server ready.

## Phase 8 — Content completion and polish (~40 units)
- Every remaining block/item/mob from the inventory checklists (newest drops: copper age, happy ghast, mounts, sulfur caves, golden dandelion)
- Our own art, sound effects and music
- Full options menus, accessibility (subtitles, narrator, remapping), localization
- Resource pack loading (models, textures, sounds, shaders)
- Long-distance LOD to 128+ chunks, shader quality options

**Exit:** 1.0 release.

---

## Mod track (runs alongside)
| Step | When | What |
|---|---|---|
| M1 | Phase 0–1 | Registries + data packs; vanilla loads from `packs/vanilla` |
| M2 | after Phase 2 | Behavior hooks + events in `api/`; vanilla blocks use them |
| M3 | after Phase 3 | WASM host (wasmtime), first sample WASM mod |
| M4 | after Phase 4 | First real mod port brought by HP (porting workflow in `modding.md`) |
| M5 | Phase 7 | Server-side mod sync, mod list on join, mod packaging format |

## How we track it
- One repository (to be created) with these phase docs, and one GitHub issue per inventory section (e.g. "Wood sets", "Villagers").
- Inventory sections get marked done (with the PR link) as they land.
- Next step: create the repo and start Phase 0.
