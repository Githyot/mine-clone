# Inventory: Game systems and mechanics

Reference: Java Edition 26.2. This is everything that is not "a block", "an item" or "a mob". `[P#]` = roadmap phase.

## 1. Player [P1–P2]

- Movement: walk, sprint, sneak, jump, swim, crawl (1-block gaps), climb (ladders, vines, scaffolding), elytra gliding + firework boost, riptide, flying (creative), riding, wind-charge jumps
- Camera: first person, third person front/back, FOV effects
- Health (20), hunger (20), saturation, exhaustion, natural regen; XP orbs, levels, XP cost formulas
- Damage: ~50 damage types (fall, fire, lava, drowning, suffocation, freezing, explosion, magic, wither, void, starvation, lightning, cactus, sweet berry, etc.), armor + toughness formula, enchantment protection, invulnerability frames, knockback
- Combat: attack cooldown (1.9+), critical hits, sweep attack, shield blocking + disabling by axes, mace smash, spear charge
- Inventory: 36 slots + 4 armor + offhand, hotbar, creative inventory tabs, item stacking (1–99), drag/split/shift-click
- Respawn: bed / respawn anchor / world spawn, death screen, keep inventory gamerule, hardcore
- Game modes: survival, creative, adventure, spectator, hardcore. Difficulty: peaceful/easy/normal/hard + regional difficulty

## 2. Blocks interaction [P1–P2]

Mining speed formula (hardness, tool tier, efficiency, haste, underwater, airborne), correct-tool drops, silk touch/fortune, block placement
rules (facing, half, waterlogging, support), block updates (neighbor updates, shape updates), gravity blocks, fluid flow, fire spread and burnout,
explosions (ray-based block resistance), random ticks, scheduled ticks, block entities (chests, signs, etc.).

## 3. Crafting and processing [P2–P6]

- Crafting: 2×2 (inventory), 3×3 table, shaped / shapeless / special recipes (dyeing, map cloning, banner copy, fireworks, suspicious stew, shield decoration, book cloning, repair)
- Recipe book + unlocking, crafter (automated crafting)
- Smelting: furnace, blast furnace (ores, 2× speed), smoker (food, 2×), campfire; fuel values; XP
- Stonecutter, smithing table (netherite upgrade, trims), loom (banners, max 6 layers), cartography table (zoom, clone, lock), grindstone (disenchant, repair), anvil (repair, combine, rename, "too expensive" at 40 levels, prior work penalty)

## 4. Enchanting [P6]

Enchanting table (bookshelf power 0–15, 3 offers, lapis cost, seed), anvil combination, villager librarian books, loot.
Enchantments (43): protection, fire protection, feather falling, blast protection, projectile protection, respiration, aqua affinity, thorns,
depth strider, frost walker, soul speed, swift sneak, curse of binding, sharpness, smite, bane of arthropods, knockback, fire aspect, looting,
sweeping edge, efficiency, silk touch, unbreaking, fortune, power, punch, flame, infinity, luck of the sea, lure, loyalty, impaling, riptide,
channeling, multishot, quick charge, piercing, mending, curse of vanishing, density, breach, wind burst, lunge. Enchantments are data-driven since 1.21.

## 5. Status effects and brewing [P6]

**Effects (39):** speed, slowness, haste, mining fatigue, strength, instant health, instant damage, jump boost, nausea, regeneration, resistance,
fire resistance, water breathing, invisibility, blindness, night vision, hunger, weakness, poison, wither, health boost, absorption, saturation,
glowing, levitation, luck, bad luck, slow falling, conduit power, dolphin's grace, bad omen, hero of the village, darkness, trial omen, raid omen,
wind charged, weaving, oozing, infested.

**Brewing:** brewing stand + blaze powder fuel, water bottle → awkward/mundane/thick, then ingredients, redstone (duration), glowstone (level),
fermented spider eye (corruption), gunpowder (splash), dragon's breath (lingering).
**Potions:** night vision, invisibility, leaping, fire resistance, swiftness, slowness, turtle master, water breathing, healing, harming, poison,
regeneration, strength, weakness, slow falling, wind charging, weaving, oozing, infestation, luck. Tipped arrows from lingering potions.
Beacon (pyramid 1–4 levels, 6 powers), conduit (prismarine frame).

## 6. Villagers, trading and raids [P3]

- Professions (13 + unemployed + nitwit): armorer, butcher, cartographer, cleric, farmer, fisherman, fletcher, leatherworker, librarian, mason, shepherd, toolsmith, weaponsmith — each tied to a workstation block (POI)
- Trade levels (novice → master), restocking twice a day, demand-based prices, gossip/reputation, discounts after curing, hero of the village
- Schedules (work, gather, sleep), beds and bells, breeding (food + beds), farming villagers, iron golem spawning (panic/gossip), zombie siege
- Wandering trader + trader llamas
- Raids: bad omen / raid omen, waves by difficulty, raid captains, ominous banners, rewards

## 7. Farming and animals [P3]

Crop growth (random ticks, hydration, light, neighbor crops), bone meal, composting, sugar cane/cactus/bamboo growth, tree growth from saplings,
breeding with love mode + cooldowns, baby mobs and growth (golden dandelion pauses it), taming (wolves, cats, parrots, horses, llamas),
leads (physics since 1.21.6), fishing (loot tables: fish/junk/treasure, open water), beehives (pollination, honey, smoke with campfire), turtle eggs, sniffer seeds, frog → froglight.

## 8. Redstone and automation [P4]

- Power levels 0–15, strong vs weak power, signal decay, quasi-connectivity (pistons/dispensers), update order and BUD behavior
- Game ticks (20/s) vs redstone ticks (10/s), scheduled tick priorities, repeater delays, comparator measure/subtract, container fill level reading
- Pistons (push limit 12, slime/honey sticking, immovable blocks, 0-tick behaviors we may intentionally simplify)
- Item transport: hoppers (cooldown 8 ticks), droppers, dispensers (place/use items), hopper minecarts, crafter, copper golem sorting
- Rails: powered, detector, activator; minecart physics (new experimental physics in 1.21.2+)
- Sculk sensors: vibration frequencies 1–15, calibrated sensor, game events system
- Observers, daylight sensors, target blocks, lightning rods, bells, lecterns, note blocks, jukeboxes, copper bulbs

## 9. Exploration and progression [P5–P6]

Nether portals (frame sizes, linking search radius, 1:8), end portal + eyes of ender + strongholds, Ender Dragon fight and respawn, end gateways,
elytra, Wither summoning, maps (exploration maps, banner markers, locked maps), compass, lodestone, recovery compass, locator bar (1.21.6),
archaeology (brushing suspicious sand/gravel, sherds, sniffer eggs), trial chambers (trial spawners, vaults, ominous trials, keys), ancient cities
(warden summoning via shriekers), advancements (~125 across 5 tabs), statistics.

## 10. Environment [P3]

Lighting (sky 0–15 + block 0–15, smooth lighting, ambient occlusion), day/night, weather, seasons-free climate tints, fog per biome/dimension,
sounds and ambient music, particles (~110 types), clouds, stars, sun/moon, end sky, nether ambient.

## 11. Commands and admin [P7]

~85 commands: advancement, attribute, ban/pardon, bossbar, clear, clone, damage, data, datapack, debug, defaultgamemode, difficulty, effect,
enchant, execute, experience, fill, fillbiome, forceload, function, gamemode, gamerule, give, help, item, jfr, kick, kill, list, locate, loot, me,
msg, op/deop, particle, perf, place, playsound, random, recipe, reload, return, ride, rotate, save-all/on/off, say, schedule, scoreboard, seed,
setblock, setworldspawn, spawnpoint, spectate, spreadplayers, stop, stopsound, summon, tag, team, teammsg, teleport, tellraw, test, tick,
time, title, transfer, trigger, waypoint, weather, whitelist, worldborder. Target selectors (@p @a @r @e @s @n) + NBT/component paths.
Game rules (~55), scoreboards, teams, bossbars.

## 12. Data and content packs (very important for our mod system) [P1 → P8]

Java is already ~70% data-driven. **Data packs:** recipes, loot tables, advancements, tags, functions, predicates, item modifiers, damage types,
enchantments, dimension types, worldgen (biomes, noise, features, structures, template pools, processors), chat types, trim patterns/materials,
banner patterns, painting/wolf/cat/frog/pig/cow/chicken variants, jukebox songs, instruments, trial spawner configs.
**Resource packs:** blockstates, block/item models, item model definitions, textures, sounds, fonts, languages, shaders, atlases, equipment layers.
We copy this idea: our base game itself is loaded as a "pack", so mods use the same path.

## 13. Multiplayer [P7]

Dedicated server, LAN, integrated server for singleplayer, login/encryption, chat (signed messages optional for us), tab list, skins & capes,
whitelist, ops, bans, server list ping, resource pack push, transfer between servers, view distance vs simulation distance.

## 14. UI and settings [P1–P8]

Title screen, world list, world creation (game mode, difficulty, cheats, seed, world type, data packs, game rules), pause menu, options
(video, sound, controls, chat, accessibility, language, skin), HUD (hotbar, health, hunger, armor, air, XP, boss bars, locator bar, titles,
action bar, subtitles), F3 debug screen, screenshots, narrator, recipe book, advancements screen, statistics screen.
