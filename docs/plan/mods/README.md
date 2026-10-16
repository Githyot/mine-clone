# Mods to bring into the game

One file per mod. Each file says: license (decides whether we may translate code or only reuse ideas), what the mod does,
how it maps to our engine, how we make it faster, and which roadmap phase it lands in. Workflow: `../modding.md`.

| Mod | Type | License | Our approach | Phase |
|---|---|---|---|---|
| [Fresh Animations](fresh-animations.md) | Resource pack (mob animations) | Custom, all assets © FreshLX | Ideas only, build our own animation system + our own models | P2 system, P8 content |
| [JEI](jei.md) | Recipe/item viewer | MIT | Built into the base game; code may be referenced with credit | P2 basic, P6 full |
| [Mouse Tweaks](mouse-tweaks.md) | Inventory controls (client) | BSD | Built into the base inventory UI; code may be referenced with credit | P2, P7 |
| [Sophisticated Backpacks](sophisticated-backpacks.md) | Content: backpacks + upgrades | All Rights Reserved | Ideas only; our own code, models and textures | P2 base, P3–P6 upgrades |
| [Essential](essential.md) | Social: host world P2P, friends, chat, screenshots | All Rights Reserved | Ideas only; Host World built into the game | P7, social service after 1.0 |
| [Chat Heads](chat-heads.md) | Chat UI | MPL 2.0 | Built in; chat messages carry a sender ID so no guessing | P7 |
| [Handcrafted](handcrafted.md) | Content: 250+ furniture | Custom | Ideas only; furniture as data baked into chunk meshes | P3, P8 |
| [3D Skin Layers](skin-layers-3d.md) | Player rendering | Custom (tr7zw) | Ideas only; skins voxelized once, GPU instanced, LOD | P2, P7 |
| [Towns and Towers](towns-and-towers.md) | Content: 50+ structures (data) | CC 4.0 (variant to confirm) | First data-only mod; structure converter tool | P3, M1 |
| [What Are They Up To](what-are-they-up-to.md) | Multiplayer body language | All Rights Reserved | Ideas only; tiny activity state in player sync | P7 |
| [Corpse](corpse.md) | Death: corpse holds your items | All Rights Reserved | Ideas only; one entity + stored inventory, no item lag | P2, P6 |
| [Macaw's mods](macaws.md) | Decoration: bridges, doors, windows, fences/walls, furniture, lights, roofs, trapdoors, stairs | Doors, Fences, Trapdoors: MIT; Bridges, Windows, Furniture, Lights, Roofs, Stairs: ARR | Data packs on shape templates (roofs are a new template), baked into chunk mesh; lamps through the lighting engine | P1, P3, M1, P8 |
| [FastFurnace](fastfurnace.md) | Server performance fix | MIT | Built in: recipe index, lookup only when input changes, idle furnaces don't tick | P2 |
| [Clean Swing Through Grass](clean-swing-through-grass.md) | Combat tweak | LGPL-3.0 | Built in: `pass_through_attacks` tag in one combined block+entity raycast | P2, M1 |
| [Continuity](continuity.md) | Connected + emissive textures | LGPL-3.0 | Our own code: neighbor mask from mesher picks texture layer, greedy merge still works; OptiFine CTM pack importer | P1, P3, P8 |
| [Aquaculture 2](aquaculture.md) | Fishing overhaul | ARR | Fish pools as biome data, modular rods as item components, instanced fish rigs | P3, P6, M2 |
| [Alex's Mobs](alexs-mobs.md) | ~89 mobs, 1 boss, 100+ items | LGPL-3.0 | Mobs as data compositions of shared AI blocks + WASM; ~15 GPU-skinned rigs with animation LOD | P3, P5, P6, M3/M4 |
| [Better Combat](better-combat.md) | Combat: animated swings, combos, dual wield | ARR | Attack sets as item data; server-side swept hit detection with lag compensation; world option vs vanilla combat | P2, P7, P8, M2 |
| [Serene Seasons](serene-seasons.md) | Seasons: colors, temperature, crops, weather | ARR | Season = one GPU uniform (zero chunk rebuilds); temperature offset via random ticks; per-season crop/weather tables | P3, P4, P8 |
| [Deeper and Darker](deeperdarker.md) | Dimension: Otherside | GPL-3.0 | Data-defined dimension on normal worldgen; reuses portal/vibration systems; base game takes ideas only | P5, P6, M3/M4 |
| [Moog's Voyager + End Structures](moogs-structures.md) | Structures (data) | LGPL-3.0 | Jigsaw data; converter can import templates into a separate LGPL pack | P3, P5, M1 |
| [Apotheosis](apotheosis.md) | Enchanting, loot rarity, affixes, gems | ARR | Ideas only; affixes/gems as item components + loot functions; feeds `../magic-and-rpg.md` item modifiers | P2, M2, P6–P8 |
| [Iron's Spells 'n Spellbooks](irons-spells.md) | Magic system | ARR | Ideas only; content set built on our own magic system (`../magic-and-rpg.md`) | P6–P8 |
| [YUNG's Better Dungeons](yungs-better-dungeons.md) | Structures + carver | LGPL-3.0 | Own dungeons on jigsaw; optional converted LGPL pack | P3, P5, M1 |
| [Explorify](explorify.md) | Small vanilla-style structures | ARR | Ideas only; own structures as data | P3, M1 |
| [When Dungeons Arise](when-dungeons-arise.md) | Huge structures | ARR | Ideas only; large templates placed in streaming per-chunk pieces, LOD visibility | P3–P5, P7, P8 |
| [Ars Nouveau](ars-nouveau.md) | Magic: craft your own spells from glyphs, mana network, helpers | ARR | Ideas only; folded into our magic system as glyph spell crafting, compiled spells | P6, P8, M2 |
| [CTOV](ctov.md) | Structures: 21+ villages, 11+ outposts | Custom (terms not stated) | Ideas only; data on jigsaw, shared village registry so packs don't stack spawn rates | P3, M1 |
| [The Aether](aether.md) | Sky dimension: islands, 4 tiers, 3 dungeon bosses, moas, accessories | Code LGPL-3.0, assets ARR | Ideas only in base game; dimension as data, generic custom portals and fall-through, fits boss ladder + accessories | P5, P6, P8, M2 |
| [Goblin Traders](goblin-traders.md) | Traveling cave traders | MIT | One traveling-merchant template shared with wandering trader and town NPCs; shared trading UI | P3, P5, P6, M1/M2 |
| [Guard Villagers](guard-villagers.md) | Village defense AI | Custom (ideas only) | Guard = villager profession; one shared threat list per village instead of every guard scanning | P3, P6, M1/M2 |
| [LambDynamicLights](lambdynamiclights.md) | Held/dropped/entity dynamic light | Custom (ideas only) | GPU dynamic lights (clustered forward+), never re-lighting or remeshing chunks | P2, P5/P6, P8, M2 |
| [Mutant Monsters](mutant-monsters.md) | Mini-bosses | AGPL-3.0 | Ideas only in base game; mini-boss rung of the boss ladder, rare variants boosted by blood moon | P2, P6, P8, M2/M3 |
| [Falling Tree](falling-tree.md) | Chop the whole tree at once | LGPL-3.0 | Ideas only in base game; bounded flood fill to natural leaves, one batched removal and one packet | P2, P3, P8, M2 |
| [Immersive Aircraft](immersive-aircraft.md) | Vehicles: planes, airships | GPL-3.0 | Ideas only; data-driven flight model, deterministic physics, pilot prediction, chunk streaming ahead of the plane | P4, P7, P8, M3/M4 |
| [Immersive UI](immersive-ui.md) | Animated inventory UI | ARR | Ideas only; tween/spring animations built into our UI, reduce-motion option | P1, P2, P8 |
| [Snow! Real Magic!](snow-real-magic.md) | Snow on any block | MIT | Snow as a cover layer state (like waterlogging), accumulation on random ticks with seasons | P1, P3, M1 |
| [Naturalist](naturalist.md) | ~47 real-world animals | Custom (ideas only) | Shared AI with predator/prey tags, instanced rigs, ambient critters as client-side flocks | P2, P3, P8, M1/M3 |
| [Presence Footsteps](presence-footsteps.md) | Footstep sounds per material/body part | MIT (Forge port) | Acoustics in the block property table, steps from rig foot bones, capped sound voices with audio LOD | P2, P3/P8 |
| [Particle Rain](particle-rain.md) | Particle weather: angled rain, wind, sandstorms, mist, ripples | MIT | GPU-simulated weather particles with a rain occlusion map; effects as data | P3, P8 |
| [Ecologics](ecologics.md) | Biome updates: coconut/walnut/maple trees, penguins, squirrels, thin ice | Custom (ideas only) | Wood sets and biome features as data; thin ice and syrup without ticking block entities | P3, P6 |
