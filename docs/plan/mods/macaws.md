# Macaw's Bridges, Doors, Windows, Fences and Walls, Furniture, Lights and Lamps, Roofs, Trapdoors, Stairs (by sketch_macaw, art by PeachyMacaw)

**What it is:** nine decoration mods, all in vanilla style and in every wood/stone type:
- **Bridges:** rope and wooden bridges, log/rail/stone bridges, bridge stairs, wide bridges with end pieces (right-click with pliers).
- **Doors:** vanilla-style doors in every wood type plus new styles: shoji, metal, jail, modern, barn, glass and others.
- **Windows:** gothic windows, arrow slits, blinds and shutters, mosaic glass and panes, sills, parapets, curtain rods and curtains.
- **Fences and Walls:** new wooden fences and gates, metal fences, double gates, stone walls and stone gates, pillar walls, grass-topped
  walls, wired fences and cheval de frise that hurt entities on contact.
- **Furniture:** wardrobes, drawers, desks, tables, chairs, couches, sinks, in all woods and stripped variants. Pieces stack or merge into
  wider/taller pieces, storage pieces hold items, chairs are sittable.
- **Lights and Lamps:** tiki torches, street lamp posts (16 colors, wood variants), ceiling lights, lamps, paper lamps, wall lanterns,
  light slabs, chandeliers, ceiling fan lights, candle holders. Lamps/posts stack to any height; shift + right-click toggles any light.
- **Roofs:** real roof blocks instead of stairs: roof, steep roof, lower roof, top roof (ridge), in every material; striped awnings and
  rain gutters in 16 colors (gutters fill with water when it rains).
- **Trapdoors:** vanilla-style trapdoors in every wood plus new styles: ranch, bamboo, metal warning, glass lamp trapdoor.
- **Stairs:** 5 new stair designs, railings that attach to any stairs (vanilla or modded), balcony railings (up to 4 per block),
  style changed by right-clicking with shears.

**License:** Doors, Fences and Walls, and Trapdoors are **MIT** → code may be translated with credit (we still draw our own models/textures).
Bridges, Windows, Furniture, Lights and Lamps, Roofs, and Stairs are **All Rights Reserved** → **ideas only**, our own code and
assets. In practice we write all nine ourselves since they are mostly data.

## How we build it (better than the Java version)
1. **Pure data packs, zero code.** Every piece is a model + a shape template (door, fence, wall, pane, stairs, slab) instantiated per
   material, exactly like vanilla variants. The Java mods register thousands of block classes; we generate variants at load time.
2. **Baked into the chunk mesh** like any block: no block entity renderers, so a castle full of curtains and shutters costs nothing per frame.
3. **Connections reuse the fence/pane/wall neighbor logic**: bridges join with bridges, wide bridges get end pieces automatically
   (pliers become an optional manual override, stored as a block state), merged furniture is the same rule as joining couches.
4. **Open/close states** (shutters, blinds, curtains, double gates) are block states toggled by the door/trapdoor behavior; the swing
   animation uses the block animation system for a few ticks, then rebakes into the static mesh.
5. **Damaging fences** use the generic "contact damage" block component (shared with cactus and sweet berry bush), checked in collision,
   not per-tick scans.
6. **Furniture overlaps `handcrafted.md`**: same seat component, same "storage block" component (inventory in the chunk's sparse block
   entity map, only created when something is stored), same connected-piece logic. Macaw's set becomes a style pack next to Handcrafted's.
7. **Lights go through our lighting engine**: each lamp is a block with a `light_level` per on/off state (data), so toggling is a
   block-state change that queues an incremental light update for that section only. Static lamps (posts, chandeliers, light slabs)
   bake into the chunk mesh like any block; only moving parts (ceiling fan blades) use the animation system. Stackable posts reuse the
   wall/pillar connection logic (top/middle/bottom pieces chosen from neighbors).
8. **Roofs** are new shape templates (roof, steep, lower, ridge) with auto corner/hip shapes chosen from neighbors like stairs do;
   every material that has stairs gets roofs for free. Rain gutters fill through the weather system's precipitation tick, not a per-block ticker.
9. **Railings** are an overlay part on the stairs shape template (a block state flag, not a second block), so every stair in the game
   accepts them for free; balcony railings are 4 edge flags in one block, meshed into the chunk like fences.
10. Mosaic glass uses the texture array plus connected textures (see `continuity.md`) so large windows look seamless.

## Where it lands
- **P1**: shape templates (doors, fences, walls, panes) already cover the base; new door/fence/wall variants are data from then on.
- **P3**: roofs, gutters, stair railings and balconies, new trapdoor styles, bridges, windows, lights and lamps (lighting engine from P1), shutters, furniture with seat + storage components, contact-damage fences.
- **M1**: shipped as an example data-only content pack; **P8**: full style pack.
