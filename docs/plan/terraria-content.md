# Terraria-scale ores and items — our content ladder

HP asked: as many ores and items as Terraria, analyzing Terraria's items and concepts and keeping what fits our game.
Terraria has ~5,400 items and ~25 ores/bars across two "modes"; Minecraft has ~9 ores and 5 tool tiers. This doc defines **our own**
ladder that sits on top of the vanilla inventory. Rule: Terraria's mechanics and structure are ideas we may use; names that are
Terraria's own inventions (and all of its art) are not, so those get our names. Real metals and mythological names are fine.

## 1. Which Terraria concepts fit our game (verdict)

| Terraria concept | Fit? | How we use it |
|---|---|---|
| Long ore ladder (≈ 15 tiers) | **Yes** | 14 tiers below, extending vanilla's 5 (wood → netherite) |
| Alternate ore pairs per world (copper OR tin, etc.) | **Yes** | World seed picks one of each pair → replay value; both always obtainable via trading/other dimension |
| Ores tied to bosses / world state ("hardmode" ores appear after a boss) | **Yes** | After defeating the "World Breaker" boss, a world event seeds new ores into existing chunks (see §3) |
| Bars smelted from ore, then crafted at stations | **Yes** | Same as vanilla ingots; higher tiers need forge/anvil upgrades |
| Gems (7) with gem staffs and gem hooks | **Yes** | Gems for magic (staffs, spellbook upgrades) and grappling hooks |
| Grappling hooks | **Yes** | New movement item class; great with our building/exploration |
| Wings / flight accessories | **Yes, late** | Late-game accessory; vanilla elytra stays as the glide option |
| Armor set bonuses per class | **Yes** | Each tier has 2–4 sets (melee/ranged/magic/summoner) |
| Item modifiers (prefixes) + reforging | **Yes** | Already in `magic-and-rpg.md` |
| Thousands of decorative furniture sets per material | **Yes, data** | Every material gets a furniture set from shape templates (`mods/handcrafted.md`) |
| Potions with many short buffs (≈ 70) | **Partly** | +20 new potions on our brewing system, not 70 |
| 2D-specific things (wiring with wrenches, actuators, platforms) | **No** | Redstone already covers this; one-way platforms maybe as a block |
| Infinite-spawn "Corruption/Crimson/Hallow" spreading biomes | **Yes, optional** | One spreading evil biome + one spreading holy biome after the World Breaker; spread rate is a world option, default slow |
| Coin currency (copper/silver/gold/platinum) | **Partly** | Emeralds stay the currency; town NPCs also accept a coin item to make prices readable |
| Fishing quests, crates, bait power | **Yes** | Folded into fishing (with `mods/aquaculture.md` ideas) |

## 2. Ore and material ladder (14 tiers)

Vanilla tiers are kept; new tiers slot around them. Alternate pairs: the world gets one of the pair as an ore; the other is
available from its other source (trading, boss drop, other dimension).

| Tier | Ore / material (pair) | Where | Unlock | Tools/armor role |
|---|---|---|---|---|
| 1 | wood, stone | everywhere | start | vanilla |
| 2 | **copper / tin** | surface caves | start | early tools, bronze alloy (copper + tin) for both worlds |
| 3 | **iron / lead** | caves | start | vanilla iron; lead = heavier, slower, more durable variant |
| 4 | **silver / tungsten** | deep caves | start | first magic-friendly metal (silver: bonus vs undead) |
| 5 | **gold / platinum** | deep caves | start | vanilla gold; platinum = durable precious tier |
| 6 | diamond + 7 gems (amethyst, topaz, sapphire, emerald, ruby, diamond, amber) | deepslate / desert fossils (amber) | start | gem staffs, hooks, jewelry accessories |
| 7 | **meteorite** | fallen meteors (night event) | after first boss | first space-themed weapons, magic guns |
| 8 | **shadowsteel / bloodstone** (pair, our names) | evil biome chasms | after forest-guardian boss | pre-Nether melee tier |
| 9 | netherite + **magmarite** (our hellstone) | Nether | Nether access | fire weapons, Nether armor |
| 10 | **cobalt / palladium** | everywhere, seeded by World Breaker event | after World Breaker boss | first "hard mode" tier |
| 11 | **mythril / orichalcum** | deeper, seeded same event | after World Breaker boss | mid hard mode |
| 12 | **adamantite / titanium** | deepest, seeded same event | after World Breaker boss | late hard mode |
| 13 | **verdite** (our chlorophyte: grows in jungle mud) + crafted **sporite** and **spectral bars** | jungle depths | after the 3 mechanical-style bosses | class-specific endgame sets |
| 14 | **lumite** (our luminite) | dropped by final boss | final boss | endgame |
| 15 | **Aether/Otherside materials** (from mods) | other dimensions | per mod | side branches, not in main ladder |

Totals: ~22 new ores/materials, ~20 new bars/alloys, 7 gems. Each tier gets: bar/gem, block of it, pickaxe-equivalent (mining power),
axe, sword, bow, plus 1–4 armor sets with set bonus, and a furniture set. Mining power numbers gate which tier can mine the next ore.

## 3. The "World Breaker" shift (our hard mode)

Defeating a mid-game boss (placeholder name "World Breaker", after the Wither) changes the world once:
- New ores (tiers 10–12) are **seeded into already-generated chunks** by a background job (only underground sections, deterministic per
  chunk), so players don't need a new world. Chunks not yet generated simply generate with them.
- Two spreading biomes appear as diagonal stripes; spread happens on random ticks with a per-world rate option, and can be stopped with
  a purifying item.
- Stronger mob variants spawn, mechanical-style bosses become summonable.
- Performance: the seeding job is throttled (e.g. 50 chunks/s) and runs off the main thread; saving marks chunks as "seeded v2".

## 4. Item categories to add (target counts)

| Category | Terraria has | Our target | Notes |
|---|---|---|---|
| Ores, bars, gems | ~45 | ~50 | §2 |
| Melee weapons (swords, spears, yoyos-style, flails) | ~300 | ~120 | built on Better Combat-style attack sets (`mods/better-combat.md`) |
| Ranged (bows, guns-style launchers, thrown) | ~200 | ~80 | bows/crossbows + new launcher class; ammo types |
| Magic (wands, staffs, tomes) | ~150 | ~80 | `magic-and-rpg.md` |
| Summoner (staffs, whips) | ~60 | ~30 | minion slots |
| Tools (pickaxes, drills, axes, hammers, hooks) | ~150 | ~80 | hooks are new |
| Armor sets | ~100 sets | ~45 sets | 2–4 per tier, set bonuses |
| Accessories | ~300 | ~120 | movement (double jump, dash, wings), defensive, class boosters, info (depth meter, compass, DPS meter) |
| Potions / buffs | ~70 | ~20 new + vanilla | brewing system |
| Boss summons + treasure bags | ~40 | ~25 | boss ladder |
| Furniture/decoration | ~1,500 | generated per material | data templates, cost ~0 code |
| Consumables (life/mana crystals, food, fishing) | ~200 | ~80 | life crystals raise max health in steps (+2 hearts), up to 20 extra hearts |
| Mounts and pets | ~100 | ~30 | riding system |

So roughly **+1,000 hand-designed items** on top of vanilla's ~1,400, plus generated furniture variants. That's comparable to
Terraria in depth while staying buildable.

## 5. Performance notes

- Items are **data** (components), so 2,500 items cost only registry space; no per-item class/code.
- Item icons rendered once into an atlas at load (cached on disk), so inventories with thousands of item types stay instant.
- Ore seeding (§3) and spreading biomes run as throttled background jobs.
- Projectiles from new weapons use the lightweight projectile ECS path (`magic-and-rpg.md` §3).

## 6. Where it lands

| Phase | What |
|---|---|
| P2 | Tiers 1–6 (copper/tin through gems), alloys, life crystals, grappling hook |
| P3 | Meteorite event, evil biome tier 8, fishing crates/quests |
| P5 | Magmarite (Nether) tier |
| P6 | World Breaker shift, tiers 10–12, armor set bonuses, accessories, item modifiers |
| P8 | Tiers 13–14, wings, full weapon/accessory catalog, mounts/pets |
