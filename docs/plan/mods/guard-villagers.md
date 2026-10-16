# Guard Villagers (by almightytallestred; art hadez43, docs flowerfugue)

**What it is:** villages spawn **guards** (groups of six, iron sword or crossbow) who defend against threats, including hostile players.
Recruit more by sneak + right-click on a nitwit/unemployed villager with a sword or crossbow. Guards gossip with villagers; with high
reputation or Hero of the Village you can open their inventory, give armor, swap weapons, put a shield or food/potions in the offhand.
Shielded guards block; guards eat/drink when low. Villagers share spare food with guards; smiths repair guard and golem gear (to half
durability, 3×/day); clerics heal guards, villagers and heroes (3×/day). Patrol button sets a patrol area; with Hero of the Village a
follow button (or ringing a bell) makes guards follow you. Extras (each toggleable): big-head baby villagers, witches attack villagers,
illagers kill animals in raids, villagers/illagers fear polar bears, golems float. Equipment via loot-table data pack.

**License:** Custom license (terms not shown on the page). → **Ideas only.** No code read or translated; own models, textures, names.

## How we build it (better than the Java version)
The page itself says disabling patrols reduces lag: in the Java mod every guard runs its own target scan and pathing each tick.
1. **Guard is a villager profession, not a new mob type**: same villager entity + profession registry (data file: allowed weapons,
   recruit item tag, equipment loot table, AI goal list). Mods can add "archer", "knight", etc. as data.
2. **Group targeting**: one **village threat list** per village (ECS resource), updated every 10 ticks from the entity spatial grid.
   Guards pick targets from it with a cheap assignment (spread guards over threats, ranged guards prefer far targets) instead of N
   guards each scanning. Hostile players enter the list through the existing gossip/reputation values.
3. **Shared AI blocks**: melee, ranged, shield-block, eat-when-low, follow-leader, patrol-area, flee are the same goals other mobs
   use; guards just combine them in data. Patrol routes are computed once and cached (pathfinding cache, off-thread).
4. **Village "services" as scheduled jobs**: smith repair and cleric heal are a daily schedule slot with a counter, not per-tick checks.
   Food sharing runs on the villager gossip/meet tick that already exists.
5. **Activation range**: guards far from players fall to low-rate AI like all mobs; raids raise the village to full rate.
6. **Commands to guards** (patrol, follow, hold) are one small server message to the whole group, using the same bell/area query.
7. Inventory UI = standard entity inventory menu (same as horses/backpacks), gated by reputation.
8. Extras become separate game rules / data toggles, off or on per world.

## Where it lands
- **P3**: villagers + raids + golems arrive; guard profession, village threat list, recruit, patrol/follow.
- **P6**: Hero of the Village (raids done) gates follow/inventory; healing via potions after brewing.
- **M1/M2**: profession + AI goal registries open to mods. Budget: a raid on a 60-villager town < 0.3 ms server tick.
