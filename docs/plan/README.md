# Minecraft-like game in Rust — master plan

The game is 100% Rust with a Vulkan renderer; no Java is used. "Java Edition" below only names which Minecraft version we copy the *content list* from.

Goal: everything Minecraft Java Edition has (reference version 26.2, June 2026), rebuilt in Rust, much faster, and built so mods can be added by reformulating them in Rust.

| File | What's in it |
|---|---|
| [RESUMO.md](RESUMO.md) | **Resumo de tudo para avaliação (português)** |
| [inventory/blocks.md](inventory/blocks.md) | All blocks, grouped into families (~1,100 block types) |
| [inventory/items.md](inventory/items.md) | Tools, armor, materials, food, potions, discs, sherds... |
| [inventory/mobs.md](inventory/mobs.md) | All ~85 mobs + non-living entities + shared AI building blocks |
| [inventory/world.md](inventory/world.md) | Dimensions, 66 biomes, generation pipeline, structures, features |
| [inventory/systems.md](inventory/systems.md) | Player, combat, crafting, enchanting, brewing, villagers, redstone, commands, data packs, multiplayer, UI |
| [architecture.md](architecture.md) | Rust tech stack, performance targets, rendering/simulation design, lessons from the references |
| [roadmap.md](roadmap.md) | Phases 0–8 plus the mod track |
| [modding.md](modding.md) | How mods are added: data packs, WASM, native Rust; porting workflow and license rules |
| [mods/](mods/README.md) | One plan file per mod HP sends (license, ideas, how we build it faster, phase) |
| [magic-and-rpg.md](magic-and-rpg.md) | Magic, mana, health bars, spellbooks, wands, boss ladder, NPCs in player houses, events (Terraria-inspired) |
| [terraria-content.md](terraria-content.md) | Terraria-scale ore ladder (14 tiers, ~22 new ores, 7 gems), item categories (~1,000 new items), "World Breaker" hard-mode shift |
| [game-feel.md](game-feel.md) | Animation and game-feel polish (from the "Fresh Animation Mods" video): emotes, eating, juicy drops, HUD and UI tweens |
| [music.md](music.md) | Music that plays like classic Minecraft, 7 original tracks already made, option to use your own music folder |
| [textures.md](textures.md) | Classic 16×16 look (our own art), texture packs like Minecraft, importer for existing Minecraft packs |

Sources: Minecraft Wiki and official changelogs (public behavior), open-source projects fogleman/Craft and stalomeow/MinecraftClone-Unity
(both MIT), and the video "My game is 262,000 times faster than Minecraft". No proprietary Minecraft code is used.
