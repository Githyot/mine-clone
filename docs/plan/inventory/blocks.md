# Inventory: Blocks

Reference: Minecraft Java Edition 26.2 "Chaos Cubed" (June 2026). Newer drops get appended at the bottom.

Blocks are grouped into **families** on purpose: in our engine most blocks are generated from a family template
(one definition + a list of materials), not hand-written one by one. `[P#]` = roadmap phase that implements it
(see `../roadmap.md`). Counting all shape/color/oxidation variants, Java has ~1,100 block types and ~27,000 block states.

---

## 1. Shape templates (generate variants automatically) [P1]

Most building materials come in these shapes. Implement the shape once, apply to every material that has it.

| Template | Notes |
|---|---|
| Full block | default |
| Stairs | 4 facings × top/bottom × 5 shapes (straight, inner/outer L/R), waterloggable |
| Slab | top / bottom / double, waterloggable |
| Wall | connects up to 4 sides, tall/low sides, post, waterloggable |
| Fence / fence gate | wood + nether brick fence; gate open/closed, in-wall |
| Pane / bars | glass panes, iron bars, copper bars, connects 4 sides |
| Pillar (axis) | logs, basalt, quartz pillar, purpur pillar, bone block, hay, chains |
| Carpet | wool carpets, moss carpets |
| Door / trapdoor | 2-tall door with hinge; trapdoor top/bottom/open |
| Button / pressure plate | wall/floor/ceiling buttons; plates |
| Sign / hanging sign | standing + wall variants, editable text both sides, glow ink, wax |

Materials that get stairs + slab (+ wall where noted): stone, cobblestone (wall), mossy cobblestone (wall), smooth stone (slab only),
stone bricks (wall), mossy stone bricks (wall), granite (wall), polished granite, diorite (wall), polished diorite, andesite (wall),
polished andesite, cobbled deepslate (wall), polished deepslate (wall), deepslate bricks (wall), deepslate tiles (wall), tuff (wall),
polished tuff (wall), tuff bricks (wall), sandstone (wall), smooth sandstone, cut sandstone (slab), red sandstone (wall), smooth red sandstone,
cut red sandstone (slab), bricks (wall), mud bricks (wall), resin bricks (wall), prismarine (wall), prismarine bricks, dark prismarine,
nether bricks (wall), red nether bricks (wall), blackstone (wall), polished blackstone (wall), polished blackstone bricks (wall),
end stone bricks (wall), purpur, quartz, smooth quartz, cut copper ×8 (see copper), every wood planks type, bamboo mosaic,
sulfur / polished sulfur / sulfur bricks (wall), cinnabar / polished cinnabar / cinnabar bricks (wall).

---

## 2. Stone and terrain [P1–P3]

- **Stone family:** stone, cobblestone, mossy cobblestone, smooth stone, stone bricks, mossy / cracked / chiseled stone bricks
- **Igneous:** granite, diorite, andesite (+ polished of each)
- **Deepslate:** deepslate, cobbled, polished, bricks, cracked bricks, tiles, cracked tiles, chiseled, reinforced deepslate
- **Tuff:** tuff, polished tuff, tuff bricks, chiseled tuff, chiseled tuff bricks
- **Cave:** calcite, dripstone block, pointed dripstone (5 thicknesses, up/down), amethyst block, budding amethyst, small/medium/large amethyst bud, amethyst cluster, smooth basalt
- **Sulfur Caves (26.2):** sulfur, polished sulfur, sulfur bricks, chiseled sulfur, cinnabar, polished cinnabar, cinnabar bricks, chiseled cinnabar, potent sulfur, sulfur spikes
- **Bedrock**, barrier-like world floor
- **Infested** (silverfish): stone, cobblestone, stone bricks ×4, deepslate
- **Soil:** dirt, coarse dirt, rooted dirt, grass block, podzol, mycelium, dirt path, farmland (moisture 0–7), mud, packed mud, mud bricks, muddy mangrove roots, clay
- **Sand & gravel (gravity):** sand, red sand, gravel, suspicious sand, suspicious gravel (archaeology, 4 dusted stages)
- **Sandstone:** sandstone, chiseled, cut, smooth; red sandstone ×4
- **Moss:** moss block, moss carpet, pale moss block, pale moss carpet, pale hanging moss
- **Ice & snow:** ice, packed ice, blue ice, frosted ice, snow layer (1–8), snow block, powder snow
- **Fluids:** water, lava (source + 8 flow levels, falling), bubble column (up/down)
- **Misc natural:** obsidian, crying obsidian, magma block, glowstone, cobweb, sponge, wet sponge, bone block

## 3. Ores and storage blocks [P2]

- **Overworld ores (stone + deepslate variant each):** coal, iron, copper, gold, redstone (lit/unlit), lapis, diamond, emerald
- **Nether ores:** nether gold ore, nether quartz ore, ancient debris
- **Raw blocks:** raw iron, raw copper, raw gold
- **Storage:** coal, iron, gold, diamond, emerald, lapis, redstone, netherite, quartz (+chiseled, pillar, bricks, smooth), resin block, resin bricks, chiseled resin bricks, slime block, honey block, honeycomb block, hay bale, dried kelp block

## 4. Wood sets [P1–P3]

**Species (12):** oak, spruce, birch, jungle, acacia, dark oak, mangrove, cherry, pale oak, bamboo, crimson (fungus), warped (fungus).

Per species: log/stem, wood/hyphae, stripped log, stripped wood, planks, stairs, slab, fence, fence gate, door, trapdoor,
pressure plate, button, sign, hanging sign, shelf (1.21.9), sapling/fungus/propagule, leaves (persistent + distance 1–7, waterloggable), boat + chest boat (raft for bamboo).
Specials: bamboo block, stripped bamboo block, bamboo mosaic; mangrove roots; crimson/warped nylium, wart blocks, roots, weeping/twisting vines, shroomlight; azalea and flowering azalea (bush + leaves); creaking heart (pale oak, active/dormant/uprooted).

## 5. Plants, crops and flowers [P3]

- **Grass-like:** short grass, tall grass, fern, large fern, dead bush, bush, firefly bush, short dry grass, tall dry grass, leaf litter, nether sprouts, hanging roots
- **Small flowers:** dandelion, golden dandelion (26.1), poppy, blue orchid, allium, azure bluet, red/orange/white/pink tulip, oxeye daisy, cornflower, lily of the valley, wither rose, torchflower, open eyeblossom, closed eyeblossom, pink petals (1–4), wildflowers (1–4), cactus flower
- **Tall flowers:** sunflower, lilac, rose bush, peony, pitcher plant
- **Crops (age states):** wheat, carrots, potatoes, beetroots, melon stem + attached, pumpkin stem + attached, cocoa, sweet berry bush, nether wart, torchflower crop, pitcher crop, cave vines (glow berries), chorus plant + flower
- **Produce blocks:** melon, pumpkin, carved pumpkin, jack o'lantern
- **Vertical growers:** cactus, sugar cane, bamboo (+ shoot), vines, glow lichen, kelp, twisting/weeping vines, pointed dripstone
- **Water:** lily pad, seagrass, tall seagrass, sea pickle (1–4), kelp
- **Coral (5 types: tube, brain, bubble, fire, horn):** block, coral, fan, wall fan, each with a dead variant (40 blocks)
- **Mushrooms:** brown, red, brown/red mushroom block, mushroom stem (6-face texture states)
- **Lush caves:** spore blossom, big dripleaf (tilt states), small dripleaf, moss, azalea
- **Flower pot** (+ potted version of every sapling/flower/fungus/cactus/bamboo)

## 6. Sculk and Deep Dark [P6]

sculk, sculk vein, sculk catalyst, sculk sensor, calibrated sculk sensor, sculk shrieker, reinforced deepslate.

## 7. Animal-related blocks [P3]

turtle egg (1–4, hatch stages), sniffer egg, frogspawn, ochre / verdant / pearlescent froglight, bee nest, beehive (honey 0–5), dried ghast (hydration stages), copper golem statue (poses).

## 8. Nether blocks [P5]

netherrack, nether bricks, red nether bricks, cracked/chiseled nether bricks, nether brick fence, basalt, polished basalt, blackstone,
polished blackstone, polished blackstone bricks, cracked polished blackstone bricks, chiseled polished blackstone, gilded blackstone,
soul sand, soul soil, soul fire, fire, nether wart block, warped wart block, shroomlight, magma, glowstone, crying obsidian,
respawn anchor (0–4), nether portal (axis), lodestone.

## 9. End blocks [P5]

end stone, end stone bricks, purpur block, purpur pillar, end rod, chorus plant/flower, dragon egg, end portal, end portal frame (eye/no eye), end gateway, dragon head.

## 10. Colored sets — 16 dye colors [P3]

white, light gray, gray, black, brown, red, orange, yellow, lime, green, cyan, light blue, blue, purple, magenta, pink.

Per color: wool, carpet, terracotta, glazed terracotta, concrete, concrete powder (gravity, hardens in water), stained glass,
stained glass pane, bed, banner (+ wall banner), candle (1–4, lit), candle cake, shulker box. Plus uncolored: terracotta, candle, shulker box,
glass, glass pane, tinted glass. ≈ 220 blocks.

## 11. Copper family [P4]

4 oxidation stages (normal, exposed, weathered, oxidized) × waxed/unwaxed = 8 variants each:
copper block, cut copper (+stairs, slab), chiseled copper, copper grate, copper door, copper trapdoor, copper bulb (lit/powered),
copper chest (1.21.9), copper golem statue, copper bars, copper chain, copper lantern, lightning rod.
Also copper torch + wall torch, raw copper block, copper ore. Copper under a note block = trumpet (26.1).

## 12. Light sources [P2]

torch, wall torch, soul torch, copper torch, redstone torch, lantern, soul lantern, copper lantern, campfire, soul campfire,
glowstone, sea lantern, shroomlight, froglights, jack o'lantern, end rod, candles, beacon, conduit, redstone lamp, light block (invisible, level 0–15), magma, crying obsidian, glow lichen, sea pickle, cave vines with berries.

## 13. Workstations and storage [P2–P6]

crafting table, crafter, furnace, blast furnace, smoker, campfire (cooking), chest, trapped chest, ender chest, barrel, shulker boxes, copper chests,
hopper, dropper, dispenser, anvil (normal/chipped/damaged), grindstone, smithing table, stonecutter, loom, cartography table, fletching table,
enchanting table, bookshelf, chiseled bookshelf (6 slots), shelf, lectern, brewing stand, cauldron (empty/water/lava/powder snow, 3 levels),
composter (0–8), beacon, conduit, respawn anchor, lodestone, bed, bell, jukebox, note block (16 instruments × 25 pitches), decorated pot,
vault, ominous vault, trial spawner, monster spawner, heavy core, target, scaffolding, ladder, chain, iron bars, cake (7 bites), TNT, mob heads/skulls
(skeleton, wither skeleton, zombie, creeper, piglin, dragon, player; floor + wall).

## 14. Redstone components [P4]

redstone wire (power 0–15, connections), redstone torch, redstone block, repeater (delay 1–4, locked), comparator (compare/subtract),
lever, buttons (wood/stone/blackstone), pressure plates (wood, stone, polished blackstone, light/heavy weighted), tripwire hook + tripwire,
daylight detector (inverted), observer, piston, sticky piston (+ piston head, moving piston), slime block, honey block, rail, powered rail,
detector rail, activator rail, redstone lamp, target, sculk sensors, crafter, copper bulb, hopper, dropper, dispenser, lightning rod, TNT,
note block, all doors/trapdoors/gates.

## 15. Technical / creative-only [P7]

air, cave air, void air, structure block, structure void, jigsaw, command block (impulse / chain / repeating), barrier, light, moving piston,
test block, test instance block, end gateway, nether portal, end portal.

---

## Newer additions log
- 26.1 Tiny Takeover (Mar 2026): golden dandelion, copper trumpet note block, new stonecutter recipes.
- 26.2 Chaos Cubed (Jun 2026): sulfur & cinnabar sets, potent sulfur, sulfur spikes, geysers.
- 26.3 is out (seen in mod version lists, Oct 2026); its content still needs to be added here.
