# LambDynamicLights (by LambdAurora)

**What it is:** client-side dynamic lighting. Light comes from: items held by any entity (if the item's block form emits light, modded
items too), dropped light-emitting items, naturally glowing entities (allay, blaze, fireballs, glow squid, magma cube, spectral arrow),
burning entities, creepers (brighter as they fuse), guardian lasers. Torches give no light underwater. Options for update smoothness,
per-entity toggles, item light overrides via resource packs, a small API for mods and resource packs.

**License:** Custom license (terms not shown on the page). → **Ideas only.** No code read or translated.

## How we build it (better than the Java version)
The Java mod fakes moving light by changing the light values around the source and **re-meshing the nearby chunk sections** every few
frames. That is why it has "update speed" settings and lag spikes with many sources. Our block light is baked into vertices, so we
must not touch it. Instead:
1. **Real GPU dynamic lights**: a per-frame light list (position, radius, color, intensity), uploaded once per frame. Never touches
   chunk light data, never remeshes. Moving a torch costs one buffer write.
2. **Clustered forward+ pass**: a compute shader bins lights into view-space clusters (e.g. 16×9×24) each frame; the opaque, cutout and
   translucent chunk shaders and the entity shader add the cluster's lights on top of baked light. Thousands of lights stay cheap.
3. **Looks like Minecraft light**: falloff matched to block light (level 15 → 0 over 15 blocks), quantized to light levels and
   combined with `max(baked, dynamic)` so it blends with torches and smooth lighting. Option: smooth or "blocky" falloff.
4. **Occlusion**: cheap version uses no shadows (like the mod). Optional high setting: a light only lights faces it can reach via a
   coarse 3D occupancy texture (per nearby section) so light doesn't leak through walls.
5. **Data-driven sources**: item light = block light emission from the registry by default, override by item tag/JSON; entity light is
   a component (`DynamicLight { level, color, flicker }`) set by data (blaze, glow squid) or by state (burning, creeper fuse).
   Underwater rule: per-source "water-sensitive" flag.
6. **Client-only, no network**: client derives lights from already synced held items / entity states. Server logic (mob spawning,
   plant growth) never sees dynamic light, same as the mod.
7. Budget: 256 dynamic lights visible < 0.3 ms GPU at 1440p; light count cap + nearest-first selection in options.

## Where it lands
- **P2**: torches + held items exist → light list + clustered pass (held, dropped, burning entities).
- **P5/P6**: blaze, fireballs, glow squid, creeper fuse, guardian beam, allay as data. **P8**: shader quality options (occlusion mode).
- **M2**: `DynamicLight` component + item light JSON open to mods; spells/projectiles (magic layer) reuse it.
