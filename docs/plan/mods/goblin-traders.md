# Goblin Traders (by MrCrayfish)

**What it is:** small goblin merchants that spawn rarely and randomly, like the wandering trader but in other places. **Goblin Trader**
lives underground in Overworld caves, **Vein Goblin Trader** lives in the Nether. Each type has its own exclusive, good-value trades,
changeable with a data pack. Goblins have attitude: they dislike being hit (fight back or run off). Feed one its favourite food and it
plays a happy animation. (Details beyond the CurseForge page, e.g. exact trade lists, despawn timers and stock refresh, are not taken
from the mod; our numbers are our own design.)

**License:** MIT. → **Code may be translated with credit** (MIT notice in our credits file). In practice it is small and we design it
on our own systems; we still draw our own models, textures and sounds and pick our own names.

## How we build it (better than the Java version)
1. **One "traveling merchant" template, many merchants**: the vanilla wandering trader, the town NPC traveling merchant
   (`magic-and-rpg.md` §5) and goblins are all instances of the same data-driven merchant entity. A goblin is a data file: model,
   spawn rule, trade table, favourite food, temper. Mods add new merchants with zero code.
2. **Shared trading UI and trade tables**: same trade screen, same offer format (cost items, result, max uses, restock rule, price
   modifiers) as villagers and town NPCs. Trades are loot-table style rolls, so packs can add or weight offers.
3. **Cheap spawning**: not a per-tick spawn attempt per player. A merchant "director" runs once every few in-game minutes per player
   region, picks a cave pocket from the already-known cave-air sections (cave culling graph), and spawns one goblin there. Hard cap of
   one goblin per type per ~N players nearby.
4. **Despawn and anti-farming**: goblin carries a lifetime timer (ECS component, decremented in the batched timer system). It leaves
   (vanishing animation) when the timer ends or it is hit too often; stock is per goblin, so no infinite restock exploit.
5. **Temper as reusable AI blocks**: "flee when hit", "retaliate once", "approach player holding favourite food", "eat + emote" are
   shared AI goals any mob can use. AI is throttled when no player is within 32 blocks, so idle goblins cost nearly nothing.
6. **Favourite food = item tag**, so packs and mods can change it; feeding grants a small discount (ties into NPC happiness pricing).
7. Network: offers are sent only when the trade screen opens; idle goblins sync like any mob (interest management).

## Where it lands
- **P3**: merchant template built together with the wandering trader; Overworld goblin as a sample data-pack merchant.
- **P5**: Nether goblin (needs Nether caves). **P6**: same template powers the town traveling merchant + shared trading UI.
- **M1/M2**: merchant + trade registries open to data packs; AI goals open to mods.
