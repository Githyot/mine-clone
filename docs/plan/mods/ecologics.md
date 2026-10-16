# Ecologics (by SameDifferent and team)

**What it is:** small vanilla-style updates to existing biomes:
- **Beach:** coconut trees, falling coconuts, coconut crabs hiding in coconuts, seashells, a tropical music disc in buried treasure, sandcastles that can hide turtle eggs.
- **Desert:** prickly pears on cacti, desert ruins with pots (pots hold items and can be chiseled).
- **Snowy:** penguins, thin ice that cracks and breaks under you, potion of sliding (penguin feathers), snow bricks and ice bricks.
- **Plains:** walnut trees and squirrels.
- **Lush caves:** azalea wood set, surface moss, stackable moss carpet.
- **Forests:** maple trees tapped with spiles for sap, boiled into maple syrup.

**License:** custom license (terms on their page; no backports without permission). → **Ideas only**, our own code and art.

## How we build it (better than the Java version)
1. New trees (coconut, walnut, maple, azalea) are **wood sets from data** (`../inventory/blocks.md` §4): the full set of planks,
   stairs, doors, signs, boats is generated automatically for each.
2. Biome additions are **features added by biome tag** in data, so they also appear in modded biomes.
3. **Thin ice** = block with a "crack stage" state advanced by an entity-step event (no ticking), breaks at stage 3.
4. **Falling coconuts** reuse the falling-block rule; tapping maples uses a generic "fluid source from block over time" behavior (shared
   with beehives' honey level), scheduled every N ticks instead of a block entity ticking.
5. Mobs (penguin, squirrel, coconut crab) are compositions of shared AI blocks, GPU-skinned and instanced like Naturalist's animals.
6. Sandcastle hiding turtle eggs is a block with a stored block state inside it, no block entity.

## Where it lands
- **P3** (biomes, trees, animals, weather). Maple syrup and potion of sliding in **P6** with brewing.
