# Mutant Monsters (by Fuzs; based on Mutant Creatures by Shcott21 and Mutant Beasts by Chumbanotz, textures tdstress)

**What it is:** giant mutant versions of vanilla mobs, plus their unique drops:
- **Mutant Zombie**: big brute with ground-smash attacks → **Hulk Hammer** (ground slam, knockback).
- **Mutant Skeleton**: ranged marksman → bones for **Mutant Skeleton Armor**.
- **Mutant Creeper**: chain explosions → **Creeper Minion Egg** (hatch tamed explosive minions, control with a **Minion Tracker**;
  breaking the egg gives a **Creeper Shard**, a weapon that explodes on terrain).
- **Mutant Enderman**: teleports, cut off its hand → **Endersoul Hand** (teleport, move blocks).
- **Mutant Snow Golem**: an ally that throws ice chunks. **Spider Pig**: pig fed a fermented spider eye + **Chemical X**.
- **Chemical X**: liquid that mutates mobs into mutants. The page mentions "and more" without naming them; spawn rules are not
  described there (from memory: mutants spawn rarely at night like normal hostiles, configurable; treat as unverified).

**License:** GNU AGPL v3. → **Copyleft: only inside a separate optional pack** (that pack would then be AGPL); the **base game uses
ideas only**. We don't read its code; our mutants get our own names, models, textures and sounds.

## How we build it (better than the Java version)
1. **Mini-boss tier in the boss ladder** (`magic-and-rpg.md` §4): mutants are the "rare variant / mini-boss" rung between normal mobs and
   summoned bosses. Data file per mutant: base mob, scale, health, phases, attack set, drops; boss-style health bar (no boss bar).
2. **Rare variants, not new spawn loops**: the normal spawner rolls "mutant chance" on a regular spawn (biome, depth, moon phase, blood
   moon event, local difficulty). Zero extra spawn cost; packs tune it.
3. **Attacks from shared building blocks**: smash = area shockwave, ice chunk = projectile, chain explosion = scheduled explosion
   batch, teleport = spell teleport block. The same blocks bosses and spells use; complex ones in WASM.
4. **Explosions batched**: chain blasts are queued and resolved in one parallel pass per tick (one lighting/remesh batch per region,
   not per blast) so a Mutant Creeper can't freeze the server.
5. **Thrown blocks / grabbed blocks** (Endersoul Hand, enderman) are lightweight projectile entities carrying a block state, with the
   world change applied as a server transaction on landing.
6. **Big bodies, cheap rendering**: GPU-skinned instanced rigs (`fresh-animations.md`); multi-part hitboxes as one entity with parts.
7. **Chemical X = mutation registry**: `base mob + catalyst → mutant`, data-driven, so mods add mutations; minions reuse the tame +
   minion-slot system of the summoner class.
8. Trophy drops feed gear (Hulk-Hammer-style weapon, bone armor set) into the class/accessory system as one tier of the ladder.

## Where it lands
- **P2**: mob framework supports scale + multi-part hitbox. **P6**: mutation registry, first 3 mutants (zombie, skeleton, creeper)
  as mini-bosses with health bar and treasure drops, blood moon raises mutant chance.
- **P8**: enderman, snow golem ally, minions, Chemical X content. **M2/M3**: mutation + attack registries open to mods.
