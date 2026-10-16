# Handcrafted (by terrariumearth and team)

**What it is:** 250+ decorative furniture pieces in vanilla, fantasy, steampunk and medieval styles: chairs, tables, benches, desks, and more
(couches, shelves, cupboards, nightstands, lamps, plates/cups, pillows, trophies, etc.), most in every wood type, many sittable or with
tablecloth/cushion colors. Needs Resourceful Lib.

**License:** Custom license (terms on their site, assume no reuse of assets or code). → **Ideas only; our own models and textures.**

## How we build it (better than the Java version)
In Java most furniture pieces are block entities with special renderers, which cost CPU every frame and are a known FPS drain in big builds.
In ours:
1. **Furniture is plain data**: a model file + a shape template, generated for every wood type and color the same way stairs and slabs are
   (see `inventory/blocks.md` shape templates). One definition, many variants.
2. Furniture is **baked into the chunk mesh** like any block (static geometry), so 10,000 chairs cost nothing per frame. Only truly animated
   pieces use entities.
3. **Sitting** is a generic "seat" component any block can have (also used for stairs if the player wants), handled by the riding system.
4. Connected pieces (couches, tables that join) use the same neighbor-connection logic as fences and panes.

## Where it lands
- **P3** (decoration content after colored/wood sets), seat component in **P3**; style packs (fantasy, steampunk, medieval) in **P8**.
