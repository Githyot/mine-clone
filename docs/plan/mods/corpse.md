# Corpse (by henkelmax)

**What it is:** when you die, a corpse with your skin appears holding all your items (unlimited), instead of items scattering and despawning.
Right-click to get items back, items return to their original slots. Corpse can't fall into the void or burn in lava; turns into a
skeleton after 1 hour (cosmetic). Press **U** for your death history (coordinates, dimension; recover items in creative). Operator
command `/deathhistory <player>`. Options: owner-only access, despawn times, face-down pose.

**License:** All Rights Reserved. → **Ideas only.**

## How we build it (better than the Java version)
1. **Corpse = one entity** with a reference to a stored inventory record (same server-side storage as backpacks), not hundreds of item
   entities. Dying in a mob farm or with a full inventory creates zero item lag.
2. "Return to original slots" is free: we save the slot layout at death.
3. Void/lava safety: corpse spawns at the last safe position (we already track last ground position for fall damage).
4. Death history stored per player in the world save; **U** screen with a compass/locator-bar waypoint to your corpse (ties into the
   recovery compass mechanic).
5. A world option chooses between vanilla behavior (scatter items) and corpses, default corpses.

## Where it lands
- **P2** (death/respawn exists): corpse entity + item storage; **P6**: death history screen + waypoint.
