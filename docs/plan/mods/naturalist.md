# Naturalist (by crispytwig / Starfish Studios; art by noonyeyzz)

**What it is:** real-world animals with believable behavior and drops. The page says **47 animals (66 variants)** (the older About
text says 24). Mobs interact with each other and their surroundings (predators hunt prey, prey flees), some are tameable. Full list
is on Starfish Studios' wiki; from memory it includes bears (sleep, eat berries/honey), deer, boars, snails, butterflies (pollinate),
fireflies (glow at night), snakes (coral, rattlesnake), songbirds (robin, cardinal, blue jay, canary, sparrow), ducks, alligators,
tortoises, lizards, dragonflies, catfish/bass, and savanna animals (lion, elephant, zebra, giraffe, rhino, hippo, vulture).

**License:** custom license (the text is linked from the page, not shown on it). → **Ideas only.** Our own models, textures and code.

## How we build it (better than the Java version)
Like Alex's Mobs, each Java animal is its own class with its own goals and a GeckoLib model animated on the CPU every frame.
1. **Shared AI blocks** (same library as `alexs-mobs.md`): wander, graze, flee-from-tag, hunt-tag, sleep-at-time, herd, perch/fly
   (birds), flutter (butterflies), pollinate, burrow, bask, swim. Each animal is a data file combining blocks + parameters (flee
   list, prey list, active hours). Predator/prey uses entity **tags**, so modded animals join the food chain.
2. **Perception from the spatial grid**: "nearest predator within 12 blocks" is one grid query cached per 10 ticks, shared by all
   AI blocks that need it.
3. **Few GPU-skinned rigs**: quadruped, bird, snake, insect, fish, tortoise. Bones are computed in a compute shader and mobs are
   instanced per type: a flock of 30 songbirds is one draw.
4. **Animation LOD** (`fresh-animations.md`): full clips near, reduced bone set at mid range, pose-only far, nothing when culled.
   Butterflies and fireflies far away become GPU particles (impostors).
5. **Ambient critters** (butterflies, fireflies, dragonflies) are a lightweight entity class: client-side flocking with no
   pathfinding, server only tracks counts and spawns. Firefly glow is emissive bloom, not dynamic block light.
6. **Activation range** throttles AI of far animals; sleeping animals tick AI every 40 ticks.
7. Spawns from biome data; variants (coat colors, sizes) are data-driven variant tables, like vanilla wolf/cat variants.

## Where it lands
- **P2** (mob framework): AI blocks, rigs, skinning. **P3** (all Overworld mobs, taming): Naturalist-style animal set and food chain.
- **P8**: ambient critter impostors and polish. **M1/M3**: animals as data packs, WASM only for unique mechanics.
- Budget: 300 ambient critters + 200 animals near the player < 1 ms server, < 0.5 ms client CPU.
