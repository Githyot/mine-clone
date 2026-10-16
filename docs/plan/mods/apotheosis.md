# Apotheosis (by Shadows_of_Fire, art by faellynna)

**What it is:** a big progression overhaul, now split into four modules:
- **Apothic Attributes** (core library): many new attributes (crit chance/damage, armor pierce, life steal, mining speed, XP gain...) and potion effects.
- **Apothic Spawners:** pick up spawners (silk touch) and modify them with items (spawn count, range, delay, ignore light/players, redstone control).
- **Apothic Enchanting:** reworked enchanting table driven by bookshelf stats (Eterna = max level, Quanta = randomness, Arcana = rarity),
  special shelves, levels above vanilla caps, many new enchantments, library block to store enchanted books.
- **Apotheosis (adventure):** affix items with rarities (found in chests/on mobs, or made at a Reforging Table from Rarity Materials);
  Salvaging Table; gems and sockets (Sigils of Socketing/Withdrawal, dimension-specific gems); Augmenting Table to reroll/upgrade single
  affixes; Sigil of Malice (random mutation); 5 World Tiers (Haven → Pinnacle, CTRL+T screen) scaling loot and enemies; Invaders (affix
  mobs that always drop an affix item), Elites, random mob augmentations; Chronicle of Shadows guidebook. Affixes, gems, sockets and
  tier augments are all data-driven.

**License:** All Rights Reserved. → **Ideas only.** Our own code, data, names and textures.

## How we build it (better than the Java version)
1. **Attributes as one ECS component** with a fixed registry index → modifier lists are flat arrays, recomputed only when equipment
   changes (dirty flag), not every hit. All new stats are data-registered attributes, so any mod can add more.
2. **Affixes = data-driven item components** (rarity, affix list, sockets, gem list) on our item data components; affix pools and
   rarity weights live next to our **data-driven loot tables** as a loot function (`apply_affixes`). Works on modded items via item tags.
3. Affix effects run through our **event hooks** (on_attack, on_damage_taken, on_kill, on_block_break); each affix is a small data
   entry that picks an effect type + values. Exotic effects = **WASM** handlers.
4. **Data-driven enchantments** (P6) already compute table offers from bookshelf power; we generalize "power" into the three stats as
   data on shelf blocks, so new shelves are pure data. Level caps are a data field.
5. Spawners: our spawner block entity stores its config as a component; modifiers are recipes on that component. Spawner ticking stays
   in the region-parallel tick, with player-range checks via the spatial index (no per-spawner entity scan).
6. World tiers: a per-player/world difficulty value read by loot and mob-spawn augment rules (data). Elites/invaders = mob spawn
   modifiers; synced to clients only as nameplate + glow.
7. Guidebook through our in-game guide/recipe viewer, not a separate book library.

## Where it lands
- Attributes framework: **P2** (combat). Affixes/gems/tables: **M2** + **P6**. Enchanting rework: **P6**. Spawners: **P6**. World tiers/elites: **P6–P8**.
