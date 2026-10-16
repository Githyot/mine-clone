# Sophisticated Backpacks (by P3pp3rF1y)

**What it is:** wearable or placeable backpacks in 6 tiers (leather, copper, iron, gold, diamond, netherite), each with more storage slots
and more **upgrade slots**. Upgrades:
- Collection: pickup, magnet, restock (pull from nearby containers), refill (keep hotbar items topped up)
- Item control: filter, void, compacting (9 nuggets → ingot etc.), deposit (push into nearby containers)
- Processing: smelting / smoking / blasting (manual and auto), crafting, stonecutter, anvil, smithing
- Capacity: stack upgrades / downgrades (more or fewer items per slot)
- Fluids and XP: tank, pump, experience pump (energy/battery only if we ever add an energy system)
- Convenience: feeding (auto-eat), alchemy (auto potions), tool swapper (picks the right tool for the block), jukebox
- Special: inception (backpacks inside backpacks, one level), everlasting (can't be destroyed), infinity (creative)
- Place with sneak + right-click, works with hoppers; two-color dyeing (main + accent); open key **B**; mobs sometimes spawn wearing loot backpacks.

**License:** All Rights Reserved. → **Ideas only.** We don't read or translate its code and we draw our own models and textures.

## How we build it (better than the Java version)
The Java mod stores the whole backpack contents inside the item's data, which is copied and synced to clients whenever the item moves.
Big backpacks (especially nested ones) make huge packets, cause lag, and are a classic source of duplication bugs and "too much data" kicks.
In ours:

1. **Contents live in a server-side storage table keyed by a backpack ID**; the item only carries that ID, tier, colors and upgrade list.
   Moving, dropping or wearing it costs a few bytes. Contents sync to a client only while the backpack screen is open.
2. **Upgrades are behaviors, not per-tick loops**: each upgrade subscribes to the event it needs (item pickup event for pickup/filter/void,
   hunger change for feeding, block-start-breaking for tool swapper). Magnet uses the entity spatial grid and runs every 4 ticks, not every tick.
3. **Processing upgrades** (smelting etc.) reuse the same furnace/crafting systems as the base game and only tick while they have work.
4. **Compacting** uses the recipe graph from the JEI work (precomputed 2×2 / 3×3 compression pairs), so no recipe search at runtime.
5. Duplication-safe by design: one storage record per backpack ID, moves are server transactions (same system as Mouse Tweaks).
6. Fits the mod API as a reference "big content mod": new item type, new menu, upgrade registry other mods can extend, placeable block form.

## Where it lands
- **P2**: basic backpack (tiers, storage, wear/hold/place, key B, dyeing).
- **P3–P6**: upgrades as their base systems exist (smelting after furnaces, alchemy after brewing, tank after fluids).
- **M2**: upgrade registry exposed to mods.
- Budget: 100 players each with a netherite backpack full of upgrades < 0.5 ms of server tick.
