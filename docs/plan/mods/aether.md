# The Aether (by The Aether Team)

**What it is:** a whole new dimension in the sky: floating islands over clouds, with its own survival progression ("hostile paradise").
- **Portal:** glowstone frame, lit by pouring water in it. Falling off the islands drops you back into the Overworld.
- **Materials (4 tiers):** skyroot (wood), holystone (stone), zanite (gem), gravitite (floating ore). Each tier's tools have special
  abilities (e.g. zanite gets stronger as it wears down, gravitite launches targets upward).
- **Mobs:** sky versions of farm animals, flying sheep/pigs, the **moa** (rideable bird with multi-jump, tiers by color), plus hostile
  "mythical" mobs.
- **Dungeons + bosses (3):** bronze dungeon (slider, a stone block boss), silver dungeon / valkyrie temple (valkyrie queen),
  gold dungeon (sun spirit, fire boss). Dungeon keys and loot gate the next tier.
- **Accessories** with their own equipment screen (rings, pendants, gloves, capes) found in dungeons.
- **Utility blocks:** altar (enchant/repair with ambrosium), freezer (freeze items with icestone), incubator (hatch moa eggs),
  sun altar (control the day cycle after the final boss). The Aether is in **eternal day** until the sun spirit is defeated.
- Book of Lore describing every block and item.

**License:** code LGPL v3.0, assets All Rights Reserved (custom license on CurseForge). → Code could only be reused inside a separate
LGPL pack with source and credit; art can't be reused at all. **Base game takes ideas only**, our own art and names.

## How we build it (better than the Java version)
1. **A sky dimension as data**: dimension type (no bedrock, void below, fall-through to the Overworld) + noise settings for floating
   islands. Our worldgen pipeline handles "island" density functions the same way it handles the End's islands, in parallel.
2. **Falling back to the Overworld** is a generic dimension rule ("below Y → teleport to dimension X at same x/z"), reusable by mods.
3. **Clouds as real blocks** use a translucent-pass, greedy-meshed texture, and **cloud LOD** for the sky view far away, so the
   endless cloud sea is cheap to draw.
4. **Accessories** use the accessory slots we already designed in `../magic-and-rpg.md`; the Aether's dungeons fit the **boss ladder**
   (bronze → silver → gold boss), and the sun altar fits "boss defeat changes the world".
5. **Moa** = mount built on the riding system with a multi-jump component (shared with double-jump accessories).
6. Tool abilities are item behaviors (event hooks: on_hit, on_mine) in data + WASM, no per-tick checks.
7. Portal = generic custom-portal registry (frame block + igniter item + target dimension), so Deeper Darker and others use the same code.

## Where it lands
- **P5** (dimension system, portals) for the dimension, islands and materials; **P6** for dungeons, bosses and accessories;
  **P8** for full mob roster and lore book. Custom portal + fall-through rules in **M2**.
