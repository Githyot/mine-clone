# Continuity (by Pepper_Bell and the Continuity Team)

**What it is:** client-side connected textures without OptiFine. Supports resource packs in the OptiFine CTM format (and its own
extended spec): full connected textures, horizontal/vertical/compact connections, random and repeat patterns, overlays. Also supports
OptiFine **emissive textures** (glowing parts of block/item textures) and **custom block layers** (which render layer a block uses).
Ships built-in packs: default connected textures for glass, sandstone, bookshelves, and a glass pane culling fix.

**License:** LGPL-3.0. → Copyleft: translated code would have to stay LGPL, so we **write our own** implementation. The OptiFine
`.properties` format is a public resource pack format, so we may read it for compatibility.

## How we build it (better than the Java version)
In Java, Continuity wraps every block model and checks neighbors while chunks are built, on top of Fabric's rendering API. In ours it is
part of the mesher:
1. **Connection mask computed in the binary greedy mesher.** The mesher already has the 64-bit occupancy rows per section; a same-block
   neighbor mask (8 neighbors per face) comes from the same bitmasks with shifts and ANDs, no per-block model lookups.
2. **Tile selection is a 256-entry lookup table** per CTM method (full, horizontal, vertical, compact, random, repeat) mapping the mask
   to a layer index in the **texture array**. Connected tiles are just more layers; no atlas, no bleeding at tile edges.
3. **Greedy merging still works**: faces merge only when their resolved tile is the same, and "repeat" patterns use the world position in
   the shader, so big glass walls still become a few quads.
4. **Emissive layers** are a second texture layer index in the vertex format, drawn full-bright in the same pass (no extra draw call).
5. **Render layer** (opaque, cutout, translucent) is a block property in data, so "custom block layers" are built in.
6. **Pane culling fix** built in: pane-to-pane faces are culled by the pane shape template.
7. Resource packs: our own RON format first; an importer converts OptiFine/Continuity CTM `.properties` packs at load.

## Where it lands
- **P1**: render layers as block data, pane culling in shape templates.
- **P3**: connected textures in the mesher + default pack (glass, sandstone, bookshelves); emissive layer.
- **P8**: OptiFine CTM pack importer.
