# Alex's Mobs (by sbom_xela, art by Carro3DSculpts)

**What it is:** ~89 new mobs, most of them real-world animals, some fantastical. Every mob has a purpose: unique drops, mechanics or
roles; rare drops often come from an action (shedding, molting) rather than death. Drops craft into weapons, arrows, potions and armor
with special effects. An **Animal Dictionary** book is given on first join. Spawns are configurable per biome / dimension / category.
Content by category (approximate counts):
- **Land mammals (~25)**: grizzly bear, elephant, gorilla, kangaroo, raccoon, bison, moose, tiger, snow leopard, capuchin monkey, rhino...
- **Birds (~10)**: roadrunner, emu, crow, seagull, hummingbird, toucan, blue jay, shoebill, bald eagle (falconry)...
- **Reptiles and amphibians (~10)**: crocodile, komodo dragon, alligator snapping turtle, anaconda, rattlesnake, gecko, caiman, frilled shark...
- **Aquatic (~15)**: orca, cachalot whale, giant squid, mimic octopus, hammerhead shark, lobster, blobfish, flying fish, catfish, seal...
- **Insects and small critters (~10)**: cockroach, fly, mantis shrimp, centipede, leafcutter ant (colonies), cosmic cod, potoo...
- **Nether (~8)**: crimson mosquito, warped toad, soul vulture, straddler, stradpole, laviathan, tusklin, mungus...
- **End and fantastical (~8)**: endergrade, enderiophage, mimicube, bone serpent, skelewag, farseer, guster, murmur...
- **Boss (1)**: the **Void Worm** (multi-segment, End).
- Items: ~100+ (armor pieces like crocodile chestplate, tarantula hawk elytra, rainbow glass, shield of the deep, squid grapple, mob-themed food and potions).

**License:** GNU LGPL v3. → Translated code would remain LGPL: it must live in its own LGPL mod crate/pack with source published, and
credit sbom_xela. Our engine stays unaffected (linking is fine under LGPL). We still draw our own models/textures.

## How we build it (better than the Java version)
Java gives each of ~89 mobs its own entity class, hand-written goals and a Citadel model, all ticking every tick. Ours:
1. **Mobs are data compositions** of shared ECS AI building blocks (wander, flee, herd, stalk-and-pounce, fly/perch, swim/school,
   burrow, tame/follow, steal-item, carry-rider). ~80% of mobs need no code; the rest get a WASM behavior for the special mechanic.
2. **Few rigs, many variants**: ~15 body plans (quadruped, biped, bird, serpent, fish, insect...) with GPU skinning + instancing, so
   a herd of elephants or a school of fish is one draw call. Animation LOD (full near, simplified mid, frozen far).
3. **Multi-segment mobs** (bone serpent, anaconda, centipede, void worm) are one entity with a segment ring buffer following the head
   path, not N linked entities each pathfinding and syncing; client reconstructs segments from the head trail.
4. **Throttled AI** via activation range: far mobs evaluate goals every 10-20 ticks; flying/swimming pathfinding off-thread and cached.
5. **Spawning from biome data**: each biome's spawn list in the data pack; per-mob spawn caps per category so 89 extra mobs don't
   crowd vanilla ones. Configuration = editing a data pack, no separate config system.
6. **Action drops** (shedding, molting, feather drop) are scheduled events per entity, not per-tick random rolls.
7. **Animal Dictionary** is generated from mob registry data (name, habitat from spawn data, drops from loot tables), so it is always
   correct and includes modded mobs.
8. Ant colonies (leafcutter) store their colony as a block entity with a simulated count when unloaded, not real entities.

## Where it lands
- **P3** (all Overworld mobs, taming, fishing): AI building blocks proven on vanilla; first batch of land/aquatic animals.
- **P5**: Nether and End mobs, Void Worm boss. **P6**: potions/armor effects from drops.
- **M3/M4**: candidate for the first big content-mod port (named in `modding.md`); WASM for the special mechanics.
- Budget: 89 mob types loaded, 2,000 active mobs < 3 ms server tick.
