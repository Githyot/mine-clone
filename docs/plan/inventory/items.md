# Inventory: Items

Reference: Java Edition 26.2. Every block also has a block-item (not repeated here). Java has ~1,400 items in total.
Items are built from **data components** (durability, food, enchantments, tool rules, equippable, etc.), so in our engine an item
is mostly a data file that combines components. `[P#]` = roadmap phase.

## 1. Tools and weapons [P2, P6]

| Item | Tiers |
|---|---|
| Pickaxe, axe, shovel, hoe, sword | wood, stone, copper (1.21.9), iron, gold, diamond, netherite |
| Spear (1.21.11, charge/lunge attack) | wood, stone, copper, iron, gold, diamond, netherite |
| Ranged | bow, crossbow (charged with arrow or firework), trident, snowball, egg, wind charge, ender pearl |
| Special melee | mace (with heavy core + breeze rod), trident |
| Defense | shield (with banner patterns) |
| Utility | fishing rod, carrot on a stick, warped fungus on a stick, flint and steel, fire charge, shears, brush, spyglass, compass, recovery compass, clock, lead, name tag (craftable since 26.1), saddle, harness ×16 colors (happy ghast), bundle ×17, goat horn ×8 sounds |

## 2. Armor [P2, P6]

- Helmet, chestplate, leggings, boots × leather (dyeable), chainmail, copper, iron, gold, diamond, netherite
- Turtle shell, elytra, carved pumpkin (wearable), mob heads
- Animal armor: wolf armor (dyeable), horse armor (leather, copper, iron, gold, diamond, netherite), nautilus armor (1.21.11)
- Armor trims: 18 smithing templates (sentry, dune, coast, wild, ward, eye, vex, tide, snout, rib, spire, wayfinder, shaper, silence, raiser, host, flow, bolt) × 11 trim materials (iron, copper, gold, lapis, emerald, diamond, netherite, redstone, amethyst, quartz, resin)
- Netherite upgrade smithing template

## 3. Raw materials [P2]

stick, coal, charcoal, diamond, emerald, lapis lazuli, nether quartz, amethyst shard, echo shard, raw iron/copper/gold, iron/copper/gold ingot,
iron/copper/gold nugget, netherite scrap, netherite ingot, redstone dust, glowstone dust, string, feather, flint, leather, rabbit hide, rabbit foot,
bone, bone meal, gunpowder, slime ball, magma cream, blaze rod, blaze powder, breeze rod, ghast tear, ender pearl, eye of ender,
phantom membrane, turtle scute, armadillo scute, shulker shell, dragon's breath, nether star, heavy core, prismarine shard, prismarine crystals,
nautilus shell, heart of the sea, sugar, paper, book, brick, nether brick, clay ball, honeycomb, ink sac, glow ink sac, resin clump, resin brick,
popped chorus fruit, fermented spider eye, glistering melon slice, disc fragment, trial key, ominous trial key, ominous bottle (I–V).

## 4. Dyes [P3]

16 dyes: white, light gray, gray, black, brown, red, orange, yellow, lime, green, cyan, light blue, blue, purple, magenta, pink.

## 5. Seeds and plantables [P3]

wheat seeds, beetroot seeds, melon seeds, pumpkin seeds, torchflower seeds, pitcher pod, cocoa beans, nether wart, sweet berries, glow berries, golden dandelion (stops baby mob growth, 26.1).

## 6. Food [P2–P3]

apple, golden apple, enchanted golden apple, bread, cookie, cake, pumpkin pie, melon slice, carrot, golden carrot, potato, baked potato,
poisonous potato, beetroot, beetroot soup, mushroom stew, rabbit stew, suspicious stew (per flower effect), raw/cooked beef, porkchop, chicken,
mutton, rabbit, cod, salmon; tropical fish, pufferfish, rotten flesh, spider eye, dried kelp, sweet berries, glow berries, chorus fruit,
honey bottle, milk bucket. Each has nutrition + saturation + optional effects.

## 7. Containers and buckets [P2–P3]

bucket, water bucket, lava bucket, milk bucket, powder snow bucket, mob buckets (cod, salmon, pufferfish, tropical fish (2,700 patterns), axolotl, tadpole, sulfur cube), glass bottle, water bottle, honey bottle, bowl, bundle.

## 8. Potions and arrows [P6]

Potion, splash potion, lingering potion, tipped arrow for each potion type; arrow, spectral arrow; experience bottle. Full potion list in `systems.md`.

## 9. Books, maps and paper [P6]

book, book and quill, written book, enchanted book (every enchantment/level), empty map, filled map (scales 0–4, locked), explorer maps
(woodland, ocean, trial, village variants), banner patterns (flower, creeper, skull, thing, globe, snout, flow, guster, field masoned, bordure indented),
firework rocket (flight 1–3), firework star (shapes, colors, fade, trail, twinkle).

## 10. Music discs [P8]

13, cat, blocks, chirp, far, mall, mellohi, stal, strad, ward, 11, wait, otherside, 5, pigstep, relic, Precipice, Creator, Creator (Music Box),
Tears, Lava Chicken, Bounce (26.2). We must make our **own** music; only the mechanic is copied.

## 11. Archaeology [P6]

23 pottery sherds: angler, archer, arms up, blade, brewer, burn, danger, explorer, flow, friend, guster, heart, heartbreak, howl, miner, mourner,
plenty, prize, scrape, sheaf, shelter, skull, snort. Brush, decorated pot.

## 12. Vehicles [P3–P4]

boat + chest boat × 9 woods, bamboo raft + chest raft, minecart, minecart with chest, furnace, hopper, TNT, command block.

## 13. Special / creative [P7]

spawn egg for every mob, debug stick, knowledge book, command block minecart, structure block, jigsaw, light, barrier, armor stand, item frame,
glow item frame, painting (~50 variants, data-driven), end crystal, totem of undying, totem.
