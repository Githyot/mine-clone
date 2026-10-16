# Inventory: Mobs and entities

Reference: Java Edition 26.2. ~85 living mobs + ~45 non-living entities. `[P#]` = roadmap phase.
"Variants" are data-driven in Java now (cat, wolf, frog, pig, cow, chicken, painting...), and they will be data files in ours too.

## 1. Passive [P2 basic farm animals, P3 rest]

| Mob | Key behavior to implement |
|---|---|
| Pig (3 climate variants) | breed (carrot/potato/beetroot), saddle + carrot on a stick, lightning → zombified piglin |
| Cow (3 variants), Mooshroom (red/brown) | milk, shear mooshroom, suspicious stew |
| Chicken (3 variants) | lays eggs, slow fall, chicken jockey |
| Sheep (16 colors + jeb_) | shear, dye, eat grass to regrow wool |
| Rabbit (7 types, Killer Bunny) | eats carrots crops |
| Horse (7 colors × 5 markings), Donkey, Mule | taming, saddles, armor, chests, breeding stat inheritance |
| Skeleton horse, Zombie horse | trap spawn, undead mounts |
| Camel | 2 riders, dash, sits |
| Llama, Trader llama | caravans, carpets, spit |
| Cat (11 variants), Ocelot | tame, scare creepers/phantoms, gifts |
| Parrot (5) | imitate mobs, dance to jukebox, shoulder riding |
| Fox (red, snow) | sleeps, steals items, pounces |
| Panda (7 personalities) | genetics, sneezes, rolls |
| Polar bear | protects cubs |
| Turtle | home beach, lays eggs, scute drop |
| Axolotl (5 colors) | plays dead, hunts, regen effect |
| Frog (3 variants), Tadpole | eat slimes/magma cubes → froglights, grow by biome |
| Allay | collects items, follows note blocks |
| Sniffer | digs ancient seeds |
| Armadillo | rolls up, drops scute |
| Bat | ambient, hangs |
| Squid, Glow squid | ink |
| Cod, Salmon (3 sizes), Tropical fish (2,700), Pufferfish (inflates) | schools, buckets |
| Strider | walks on lava, saddle + warped fungus on a stick |
| Villager (7 biome types × 15 professions), Wandering trader | see systems.md (trading, gossip, schedules) |
| Snow golem, Iron golem, Copper golem (1.21.9, sorts items between copper chests) | built from blocks |
| Happy ghast (1.21.6), Ghastling | grown from dried ghast, flying mount for 4 with harness |
| Nautilus (1.21.11) | underwater mount |
| Sulfur cube (26.2) | absorbs blocks, changes movement by block, splits on death |

## 2. Neutral [P3, P5]

bee, dolphin, enderman, fox (to some), goat (rams, horns), iron golem, llama, panda (aggressive), piglin, polar bear, spider & cave spider (by light), wolf (9 variants, tame, armor), zombified piglin.

## 3. Hostile [P2 core 6, P3–P6 rest]

| Group | Mobs |
|---|---|
| Core (first to implement) | zombie, skeleton, creeper, spider, enderman, slime |
| Zombie family | zombie, baby zombie, husk, drowned (trident), zombie villager (curable), chicken jockey, zombie horse rider |
| Skeleton family | skeleton, stray, bogged, parched (1.21.11), wither skeleton, skeleton horseman |
| Mounted (1.21.11) | camel husk, zombie nautilus |
| Cave | cave spider, silverfish, warden (vibration + smell, darkness, sonic boom), creaking (pale garden, tied to creaking heart) |
| Night/sky | phantom (spawns after insomnia) |
| Water | guardian, elder guardian (mining fatigue) |
| Illagers (raids) | pillager, vindicator, evoker (fangs, vexes, totem), ravager, witch, vex, illusioner (unused) |
| Trial chambers | breeze (wind charges), bogged, any trial spawner mob |
| Nether | blaze, ghast, magma cube, hoglin, zoglin, piglin brute, wither skeleton |
| End | endermite, shulker (levitation bullets), enderman |
| Bosses | Ender Dragon (fight phases, crystals, respawn), Wither (summoning, armor phase, block breaking) |
| Unused | giant |

## 4. Non-living entities [P2–P6]

- **Core:** player, dropped item, experience orb, falling block, primed TNT, lightning bolt
- **Projectiles:** arrow, spectral arrow, trident, snowball, egg, ender pearl, eye of ender, experience bottle, splash/lingering potion, fireball, small fireball, dragon fireball, wither skull, shulker bullet, llama spit, wind charge, breeze wind charge, firework rocket, fishing bobber
- **Vehicles:** boats, chest boats, rafts, all minecarts
- **Decorations:** armor stand, item frame, glow item frame, painting, leash knot, end crystal
- **Effects:** area effect cloud, evoker fangs, ominous item spawner
- **Technical:** marker, interaction, block display, item display, text display

## 5. AI building blocks we need (shared by all mobs) [P2]

Implementing mobs = composing these. Build them as reusable systems, not per-mob code:
pathfinding (ground, water, flying, climbing, lava), target selection, melee attack, ranged attack, flee/avoid, wander, follow parent/owner/leader,
breed, tempt with item, sit, eat block, look at player, panic, swim/float, door opening, sleep schedule, shared memory "brains"
(villager, piglin, warden, allay, axolotl, frog, sniffer, camel, armadillo, breeze), sensors (nearby items, players, hostiles, vibrations).
