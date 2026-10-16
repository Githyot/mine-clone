# Falling Tree (by rakambda)

**What it is:** break one log and the whole tree comes down. Modes: **instantaneous** (whole tree breaks at once), **shift down** (the
log you cut slowly shifts the tree down one block per cut), **fall items** (tree falls with an animation, then drops appear) and
**fall blocks** (tree falls to the ground as blocks). Config: what counts as a tree, which tools work, whether leaves break too,
disable while sneaking, breaking speed scales with tree size, tool damage per log, keep tools at 1 durability instead of breaking.

**License:** GNU LGPL v3. → Translated code would stay LGPL, so it could only live in a separate LGPL pack with source published and
credit to rakambda. **The base game uses the ideas only** and we write our own implementation.

## How we build it (better than the Java version)
In Java, each log broken by the mod goes through the normal block-break path: one block update, one light update, one remesh trigger
and one network packet per block. Big jungle or dark-oak trees cause a visible hitch, and leaves then decay one by one over minutes.
1. **Tree detection = bounded flood fill** over 26-neighbours from the cut log, using the dense block-property table (`is_log` and
   `is_leaves` are flags from tags, so modded woods work). Hard cap (e.g. 2,000 logs / 64 blocks of radius), so a log cabin or a
   huge build can't make one cut scan the world.
2. **Natural-leaf flag**: leaves placed by worldgen/saplings carry `persistent=false`, player-placed leaves `persistent=true` (same
   flag leaf decay uses). A tree only counts if it reaches natural leaves, so player-built log walls are never felled.
3. **Batched removal**: all positions go into one `BlockChangeBatch` applied in one server transaction. Lighting gets one batched BFS
   job per section, and each touched section is remeshed once, not once per log.
4. **One network message**: the batch goes out as a single multi-block-change packet per section (palette-indexed), not N packets.
5. **Drops are merged up front**: loot tables roll per block, then identical stacks are combined into a few item entities at the
   stump (or go straight into the inventory, if that option is on), so no pile of 300 item entities appears.
6. **Leaves**: break them in the same batch (option) or schedule them for fast decay through the existing leaf-decay system with a
   short random delay, instead of waiting for random ticks.
7. **Falling animation = one temporary client-side entity**: the server sends "tree fell" (the block list as a compact mesh blob plus
   pivot and direction). The client meshes it once with the normal greedy mesher and rotates it as one rigid body. No N falling-block
   entities, no physics per block. Drops appear when the animation ends.
8. **Shift-down mode** moves the whole tree down by one block per cut, as one batch per cut.
9. Tool damage = logs × config factor, applied once. Sneak disables it. Settings are a per-world data file / game rule.

## Where it lands
- **P2** (tools, drops, loot tables): instant mode, batch + merged drops. **P3** (worldgen trees, leaf decay): natural-leaf flag, modded woods via tags.
- **P8**: falling animation. **M2**: exposed as a `on_block_break` hook example and a `tree` tag other mods extend.
- Budget: felling a 2×2 jungle tree (~600 blocks) < 1 ms server, one packet, no frame hitch on the client.
