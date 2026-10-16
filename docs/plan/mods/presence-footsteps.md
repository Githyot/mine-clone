# Presence Footsteps (original by Hurricaaane (Ha3), Fabric by Sollace; Forge port by Paint_Ninja)

**What it is:** client-side footstep overhaul. The Forge page is just "an unofficial 1:1 port of Presence Footsteps from Fabric",
with configs compatible with the original. From memory: footstep sounds chosen per **material** (wood, stone, gravel, metal,
carpet, leaves, snow...) with separate layers for walking, running, landing and jumping; **brush/rustle** sounds when moving through
plants; **armor and equipment clank** on top of the surface sound; **per-body-part** stepping: bipeds step with 2 feet, quadrupeds
(horses, wolves) with 4 in a gait pattern, winged mobs flap; works for players and mobs; volume settings per source.

**License:** MIT (Forge port page). → **Code may be translated with credit** (Sollace, Hurricaaane, Paint_Ninja). We still write
our own engine-side design and make **our own sound assets**.

## How we build it (better than the Java version)
In Java the mod plays sounds through the vanilla sound system, which has a hard channel limit; in crowds footsteps cut out other
sounds, and each step looks up the block's acoustic by name from a big string map.
1. **Acoustics data table**: each block (or tag) maps to an acoustic ID (`wood`, `gravel`, `metal`...). At registry freeze it is
   baked into the dense block-property table, so a step lookup is one array read. Acoustics are layered (base + overlay, e.g.
   `stone` + `snow` cover from `snow-real-magic.md`; carpet over wood).
2. **Per-body-part stepping from the rig**: the animation system (`fresh-animations.md`) emits foot-contact events from bones
   tagged `foot`. Bipeds, quadrupeds, insects and modded rigs all get correct gaits for free, matched to the animation.
3. **Events, not polling**: steps are generated client-side from animation contacts + ground block; the server sends nothing.
4. **Voice limits and priority**: the audio thread has a fixed voice pool (e.g. 64). Footsteps are a low-priority category, max N
   voices, oldest/quietest stolen first; the local player and nearby entities are always first. Footsteps never block important sounds.
5. **Audio LOD**: within 8 blocks full layered sound; 8–24 blocks base layer only, rate-limited; beyond 24 blocks none. Culled
   entities don't step. Many identical steps in one frame merge into one voice with higher gain.
6. **Variation without many files**: random pitch/volume per step and a small sample pool per acoustic.
7. **Equipment clank** = an overlay acoustic from item data (`armor_material.sound`).
8. Resource packs can override acoustics and sounds; mods add acoustics by tagging their blocks.

## Where it lands
- **P2** (sounds basics, first mobs): acoustic table, voice pool with categories and priorities, step events.
- **P3/P8**: per-body-part events from rigs, plant brush, armor overlay, audio LOD, our own sound set.
- Budget: 200 walking mobs nearby < 0.1 ms client CPU, never more than 16 footstep voices.
