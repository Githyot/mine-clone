# Clean Swing Through Grass (by Raycoms)

**What it is:** lets you hit mobs standing in tall grass, flowers, ferns and other non-solid plants. In vanilla your attack ray stops at
the grass block, so you break the grass instead of hitting the mob behind it. With the mod the attack passes through blocks that have no
collision and hits the entity. Works client and server side; no configuration.

**License:** LGPL-3.0. → Copyleft: translated code would have to stay LGPL, so we **write our own** (the idea is a one-line rule anyway).

## How we build it (better than the Java version)
1. **Built into the base game, no mod needed.** Our crosshair raycast has two passes: blocks and entities. When the hit block is tagged
   `pass_through_attacks` (data tag: grass, ferns, flowers, saplings, crops, snow layer 1, cobwebs optional) and an entity lies further
   along the ray within reach, the entity wins.
2. **One raycast, not two**: the ray walks voxels with DDA and tests entities from the spatial grid along the same walk, so the
   "through grass" check costs nothing extra (Java mods redo the pick on attack).
3. **Server-authoritative**: the server validates the attack with the same rule (same tag, same reach), so it works in multiplayer
   without a client mod and cannot be abused to hit through solid blocks.
4. Mining still works: holding attack on grass with no entity behind it breaks the grass as usual. A gameplay option can turn the
   behavior off for vanilla purists.
5. Tag is data-driven, so mods add their own plants to it.

## Where it lands
- **P2** (combat and block interaction): part of the base raycast/attack system.
- **M1**: `pass_through_attacks` tag exposed to data packs.
