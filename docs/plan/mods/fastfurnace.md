# FastFurnace (by Shadows_of_Fire)

**What it is:** a pure server performance fix, no gameplay change. Vanilla furnaces, blast furnaces and smokers search the whole smelting
recipe list every tick to find a match for the input item. FastFurnace caches the last matched recipe and checks it first, so a furnace
smelting the same item does not rescan the registry (vanilla has ~70 smelting recipes, big modpacks 700+). Any modded furnace built on the
vanilla furnace class benefits too. Improves TPS, not smelting speed.

**License:** MIT. → Code may be translated with credit, but there is nothing to translate: it is a vanilla performance fix that our
design already includes.

## How we build it (better than the Java version)
1. **Recipe lookup is a hash map** keyed by (recipe type, input item/tag): built once from the recipe graph at load (and after data pack
   reload). Lookup is O(1) even with 10,000 modded recipes; no per-tick list scan, no cache needed.
2. **Furnaces only look up on input change**: the matched recipe ID is stored in the furnace state and re-resolved only when the input
   slot changes (inventory change event), never per tick.
3. **Idle furnaces don't tick**: a furnace with no fuel or no valid input is removed from the active tick list and woken by an inventory
   event, so 1,000 cold furnaces cost zero.
4. **Burning furnaces are batched**: active furnaces live in a dense array processed in one system (progress += 1, fuel -= 1),
   cache-friendly and parallel per region.
5. Same lookup map serves hoppers, crafters, backpack smelting upgrades and modded machines through the recipe API.

## Where it lands
- **P2**: furnace family built this way from day one (recipe index + event-driven ticking).
- Budget: 10,000 active furnaces < 0.3 ms of server tick.
