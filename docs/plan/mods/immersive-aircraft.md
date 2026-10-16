# Immersive Aircraft (by Conczin)

**What it is:** craftable early-tech aircraft. **Biplane** (1 seat, fast, medium difficulty), **airship** (2 seats, slow, easy;
cargo variant), **gyrodyne** (human-powered 2-seater, needs a push to spin up the rotor), **quadrocopter** (bamboo, 4 propellers,
handy for building), **warship** (4 seats, moving turret, 6 weapon mounts). Crafting goes through intermediate parts (engine, sail,
propeller...). Each aircraft has slots for upgrades, fuel, banner, dye, rocket booster and storage, and can be upgraded, tuned and
fitted with weapons/utilities. Multiplayer compatible.

**License:** GNU GPL v3. → Translated code would have to be GPL, so it could only live in a separate GPL pack with source published,
credited to Conczin. **The base game uses the ideas only.** Our own models, textures and sounds.

## How we build it (better than the Java version)
Java vehicles run physics on the client and server separately, with loose correction. On servers this gives rubber-banding, aircraft
that jitter for other players, and stutter when you fly faster than chunks load.
1. **Vehicle = ECS entity** with `RigidBody`, `Seats`, `Inventory` and a data-driven `FlightModel` (mass, lift curve, drag, thrust,
   max climb, turn rates). All 5 aircraft are one system with different data. Modded aircraft are a data pack plus model.
2. **Physics on the fixed 20 TPS server tick** with fixed sub-steps (e.g. 4 × 12.5 ms) so it is deterministic. Collision uses swept
   AABBs against the voxel grid (the same code as player movement), not per-block shape queries.
3. **Client prediction for the pilot**: the pilot's client runs the same physics code (shared crate) on its inputs and reconciles
   with server snapshots: rewind to the acked tick and replay inputs (the same machinery as P7 player prediction). Passengers and
   other players interpolate between snapshots. No rubber-banding for the pilot, smooth motion for everyone else.
4. **Chunk streaming ahead of fast travel**: the chunk loader prioritizes by *predicted position* (position + velocity × lookahead),
   with a cone in the flight direction. LOD terrain covers the far view, so flying out of loaded terrain shows LOD, never holes.
   The server raises a fast entity's generation priority in the job system.
5. **Interest management**: aircraft use a larger network relevance radius and higher update rate than ground mobs (delta-encoded).
6. **Fuel, upgrades, weapons** are item components and modifiers (thrust +x%, fuel use -y%), reusing the item-modifier system. Turret
   aims are networked as one angle pair.
7. **Rendering**: one skinned/instanced model per aircraft type, propellers and rotors as animated bones (no per-frame CPU work),
   banner and dye through the same texture-array path as banners/boats.
8. **Crafting chain** is plain recipe data (engine, hull, sail, propeller parts).

## Where it lands
- **P2/P3**: boat/minecart-style vehicle entity, seats, mounting. **P4** (rails/minecarts): vehicle physics module.
- **P7**: pilot prediction/reconciliation, predicted-position chunk streaming, interest radius. **P8**: aircraft content set.
- **M3/M4**: candidate content-mod port as a separate GPL pack, or our own ideas-only version in the base game.
- Budget: 50 aircraft flying on one server < 1 ms tick; no chunk holes at full biplane speed with 16-chunk view + LOD.
