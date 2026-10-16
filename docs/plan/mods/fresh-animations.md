# Fresh Animations (by FreshLX)

**What it is:** a resource pack that gives vanilla mobs expressive animations: blinking eyes, breathing, head tilts, ears and tails that
react, smoother walking, idle poses, more believable movement. ~85% of mobs covered (villagers, golems, bees, allays, sniffers, goats,
axolotls, piglins, endermen, happy ghasts...). In Java it needs OptiFine CEM, or the EMF + ETF mods, which run small math expressions per
model part every frame.

**License:** custom, assets © FreshLX. No redistribution, no unedited assets in our pack. → **We take the ideas only** and make our own
models and animations. Credit FreshLX as inspiration.

## How we build it (better than the Java version)
Java bolts this on top of a fixed model system with a third-party mod. We make it native:

1. **Bone-based entity models** from day one (each mob = tree of parts with pivots), stored as data files in our own format.
2. **Animation data**: keyframe clips (walk, idle, attack, sleep...) + **procedural layers** written as small expressions
   (e.g. `head.pitch = look_pitch + sin(time*0.1)*2`), with inputs like `limb_swing`, `limb_speed`, `age`, `head_yaw`, `is_hurt`,
   `is_in_water`, random per-entity seed for blink timing.
3. **Compile expressions** at load time to bytecode (or straight to a WGSL compute shader), never parse per frame. CEM/EMF interpret
   them on the CPU every frame; this is the main speedup.
4. **GPU skinning + instancing**: all mobs of one type drawn in one instanced draw call; bone matrices computed in a compute shader.
5. **Animation LOD**: full animation near the player, simplified at medium range, frozen pose / no blinks far away, skipped entirely
   when culled. Update rate scales with distance (60 Hz near, 10 Hz far).
6. Mods (and players) can ship new animation files through the resource-pack layer: no code needed.

## Where it lands
- **P2** (first mobs): bone model format, animation player, GPU skinning, expression compiler.
- **P8** (polish): expressive animations for every mob, player animation set (first/third person).
- Budget: 1,000 animated mobs on screen < 1 ms CPU.
