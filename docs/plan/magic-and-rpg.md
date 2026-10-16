# Magic, bosses, NPC towns and RPG layer (inspired by Terraria)

HP asked to take inspiration from Terraria: magic, mobs, health bars, spellbooks, wands. This is **our own design**: game mechanics and
ideas are free to use, but we never copy Terraria's (or any mod's) names, sprites, sounds or text. Iron's Spells & Spellbooks
(`mods/irons-spells.md`) and Apotheosis (`mods/apotheosis.md`) feed into this same system instead of each bringing its own.

## 1. What we take from Terraria

| Terraria idea | Our version |
|---|---|
| Mana bar (separate from health), mana regen, mana potions, max mana upgrades | **Mana** stat next to hunger; regen faster when not casting; mana crystals (consumable, +20 max, up to 200 base) |
| Visible health on enemies | **Floating health bars** over mobs after they take damage (toggle in options), boss bar at top for bosses, damage numbers (optional) |
| Weapon classes: melee, ranged, magic, summoner | 4 **classes by gear**, not chosen at start: armor sets give bonuses to a class |
| Wands / staffs that shoot projectiles | **Wands** (fast, cheap spells), **staffs** (strong, slow), tied to a spell school |
| Spell tomes / magic books | **Spellbooks** with slots: put spell scrolls in, cast them with the book in hand or offhand |
| Summoning staffs and minions | **Summoner** class: staff summons minions that occupy "minion slots" |
| Accessories (5+ slots: wings, boots, shields, emblems) | **Accessory slots** (start 3, up to 6) with passive effects: double jump, dash, speed, mana regen, fall immunity |
| Bosses summoned with crafted items, progression gating | **Boss ladder**: each boss is summoned with a crafted item and drops the material for the next tier |
| World events (blood moon, invasions, eclipse) | **Events**: blood moon (more/harder mobs), raid-style invasions, meteor fall (new ore) |
| Biome-specific enemies, mini-bosses, rare enemies | Extra mobs per biome + rare variants with better loot (data-driven, like vanilla variants) |
| Prefixes/modifiers on weapons ("Legendary", "Arcane"...) | **Item modifiers** rolled on craft/reforge (+damage, +crit, -mana cost); reforge at a station for coins/emeralds |
| NPC town that moves in when conditions are met | Later: villager specialists that move into player-built houses (merchant, alchemist, wizard) |

## 2. Magic system design

- **Custom spells** (idea from Ars Nouveau, `mods/ars-nouveau.md`): players build spells from glyphs: one form + effects + augments, compiled once when crafted.
- **Schools** (data-driven, mods can add): Fire, Ice, Lightning, Nature, Holy, Shadow, Arcane, Blood.
- **Spell** = data file: school, mana cost, cooldown, cast type (instant, charged, channeled), and an effect script built from shared
  building blocks: projectile, area, beam, self-buff, summon, teleport, shield. Most spells need no code, just data; complex ones use WASM.
- **Casting items**: wand (1 fixed spell), staff (1 strong spell), spellbook (N scroll slots, scroll wheel to switch), scrolls (single-use or slotted).
- **Stats** (stored as entity attributes, same system as armor): max mana, mana regen, spell power per school, cast speed, cooldown reduction.
- **Progression**: magic ores (meteorite, a new deep ore), arcane workbench, inscription table (make scrolls), mana potions via brewing.
- Enchantments and item modifiers can affect spells (e.g. "mana efficiency").

## 3. Performance (why ours will be faster)

1. **Spell projectiles are lightweight ECS entities** processed in batches, with no per-projectile full mob AI. 1,000 projectiles on screen < 0.5 ms.
2. **Particles are GPU-simulated** (compute shader), so big spell effects cost nearly nothing on the CPU.
3. **Health bars**: drawn in one instanced pass for all visible mobs; server sends health only when it changes and only to players nearby.
4. Spell effects are **server-authoritative** but the client predicts the cast animation and projectile spawn, so casting feels instant online.
5. Boss fights use the same region-parallel ticking as everything else; bosses have scripted phases in data.

## 4. Bosses (Terraria-style progression)

Terraria's core loop: explore → craft a summon item → fight a boss → its drops unlock the next tier of gear and the next boss.
We keep vanilla's Ender Dragon and Wither and add a ladder around them.

- **Summoning**: each boss has a crafted summon item used in the right place/time (e.g. at night, in a biome, at an altar). Some bosses
  also appear naturally when conditions are met (first time you reach a depth, after an event).
- **Progression gating**: boss drops are the materials for the next armor/weapon tier and the next summon item. Defeating some bosses
  changes the world (unlocks a new ore, makes an event possible, moves in a new NPC).
- **Example ladder (names are placeholders, ours to design)**: forest guardian (early) → cave worm (pre-iron) → skeleton king (dungeon)
  → [vanilla Wither / Ender Dragon] → "hard mode" style world shift → 3 mechanical-style bosses → jungle/plant boss → final boss.
- **Boss design rules**: multiple phases, telegraphed attacks, arena awareness, scaling with player count, a boss bar with phase marks,
  a treasure bag per player (everyone gets loot, no fighting over drops), a trophy block and a relic.
- **Data-driven**: phases, attack patterns, music, drops and summon conditions are data; complex attacks use the shared spell/attack
  building blocks or WASM. Mods add bosses through the boss registry.
- **Performance**: boss attacks spawn lightweight ECS projectiles (section 3); big bosses (worms, multi-part) are one entity with
  segment transforms, not N entities chained together.

## 5. NPCs moving into player-built houses

Terraria's town: build a valid house, and NPCs move in when their condition is met (e.g. a merchant after you hold enough money).

- **Valid house** (checked by a flood-fill when the player asks or when blocks change near a claimed house): enclosed room of at least
  60 blocks of air and at most ~750, with walls/roof, a door, a light source, a table/flat surface and a chair/seat. One NPC per house.
- **Housing menu**: shows valid/invalid rooms with the reason ("missing light source"), lets the player assign NPCs to houses.
- **NPC roster** (each with move-in condition): merchant (has money), nurse (heals for a fee; after max health increase), alchemist
  (after brewing), wizard / magic shop (after first spell), blacksmith/reforger (after first boss), guide (from the start), explorer
  (sells maps), tailor, gunsmith-style ranged vendor, traveling merchant (visits on random days).
- **Happiness**: prices improve if the NPC likes its biome and neighbors; crowded towns raise prices. Happy NPCs sell extra items.
- **Respawn**: an NPC who dies returns after a day if its house still exists.
- **Relation to villagers**: villagers stay as Minecraft's own villages. Town NPCs are a separate system for **player towns**,
  so a player base becomes a town over time. Both share the trading UI.
- **Performance**: house validation is event-driven (only when nearby blocks change), runs as a background job, caches room shape;
  NPC AI uses the shared AI blocks with a daily schedule and sleeps in its house at night.

## 6. Where it lands in the roadmap

| Phase | What |
|---|---|
| P2 | Mob health bars, damage numbers option, attributes system (needed by mana) |
| P6 | Mana, wands, staffs, spellbooks, scrolls, 8 schools, accessory slots, item modifiers, magic ores |
| P6 | First boss ladder (3 bosses), boss bar with phases, treasure bags, blood moon event |
| P6 | Housing check, housing menu, first 4 town NPCs (guide, merchant, nurse, wizard), shared trading UI |
| P8 | Full boss ladder incl. hard-mode world shift, all town NPCs + happiness, summoner class, more events, Iron's Spells content set ported onto this system |
| M2/M3 | Spell, school, boss, event and town-NPC registries open to mods |

## 7. Open choices (defaults picked)

- Magic is **on by default** but can be turned off per world ("vanilla-like" world option). Default: on.
- Health bars default: shown only after a mob is hit, hidden after 5 s.
