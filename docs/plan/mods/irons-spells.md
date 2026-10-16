# Iron's Spells 'n Spellbooks (by Iron431 and team)

> Note: this content is built on our shared magic system, see `../magic-and-rpg.md`.

**What it is:** a complete magic system:
- **Mana** bar and mana regen, with attributes for max mana, spell power per school, cooldown reduction, resistances.
- **Schools** (fire, ice, lightning, holy, ender, blood, evocation, nature, eldritch) and **100+ upgradable spells** (projectiles,
  beams, AoE, summons, buffs, teleports) with levels and rarities.
- **Spellbooks** with spell slots (equipped in a curio slot), **scrolls** (single-use spells), scroll forge / inscription table to
  craft and move spells, arcane anvil to imbue spells into weapons and upgrade armor.
- **12 wizard armor sets**, magic weapons, ink/ingredients, artifacts.
- **2 spellcasting bosses**, **9 spellcasting mobs/allies/merchants**, **11 generated structures** (towers, dungeons...).
- Per-spell configuration; compat with Apotheosis, Better Combat, JEI and others.

**License:** All Rights Reserved (the page also calls the mod open-source, but the label says ARR). → **Ideas only.** Our own code,
spells, models, textures, sounds and particles.

## How we build it (better than the Java version)
1. **Native Rust module** (`mods/magic/`): spell casting is hot (beams, AoE, many projectiles in fights), so the core runs native;
   individual spells are **data** (school, cost, cooldown, cast time, level scaling) plus an effect graph built from reusable
   primitives (projectile, ray, area, summon, apply effect, move entity). New spells from data packs; exotic ones via **WASM**.
2. Mana = an ECS component + attributes (shares the attribute framework from the Apotheosis plan), regen in a parallel system.
3. Spell projectiles are ECS entities with a lightweight physics component (no full mob AI); AoE queries use the spatial index.
4. **Server-authoritative casting with client prediction**: client plays the cast animation/particles at once, server validates
   mana/cooldown and resolves hits; only spell ID + target is sent over QUIC, not effect state.
5. **GPU particles** (compute-shader emitters, data-defined) for spell visuals, so a big fight costs GPU time, not CPU ticks.
6. Spellbook/scrolls = item data components (spell list, levels); inscription/imbuing = recipes on those components.
7. Bosses/mobs use our AI goal system with a "cast spell" goal reading the same spell data. Structures via **jigsaw** + loot tables.

## Where it lands
- Needs P2 combat + attributes, P6 particles/effects depth. Core module + first spells: after **M3**. Mobs, bosses, structures: **P6–P8**.
