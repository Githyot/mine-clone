# Mouse Tweaks (by YaLTeR)

**What it is:** client-side inventory controls that make moving items much faster:
- **RMB drag:** right-click drag places one item per slot, and can pass over the same slot again to add more, including slots that already hold items.
- **LMB drag holding an item:** collects matching items into the held stack; with Shift, shift-clicks every matching stack it passes over.
- **LMB drag + Shift, empty hand:** shift-clicks every stack the cursor passes over (fast transfer between chest and inventory).
- **Scroll wheel:** scroll down on a stack to move items one at a time to the other inventory; scroll up to pull items in. Options: search order, scroll direction (inverted / by slot position).
- API for custom screens to opt in or ignore slots.

**License:** BSD. → We may translate code with the copyright notice kept in our credits. Logic is small, so we write it fresh against our menu system.

## How we build it (better than the Java version)
In Java this is a client mod patching the container screen, and every "click" it simulates becomes a separate network packet, so fast
drags on a laggy server desync. In ours:

1. **Built into the base inventory UI**, every container screen gets it automatically (vanilla, mod screens, JEI transfer).
2. **One action message per gesture**: the client sends "drag-distribute these slots" / "move N items from slot A" as a single
   server-validated transaction instead of dozens of fake clicks. Faster, no desync, no dupe bugs.
3. Client predicts the result instantly and the server confirms, so it feels zero-latency in multiplayer.
4. Each tweak is a toggle in Options → Controls, on by default; mods opt slots out via `menu.slot_flags`.

## Where it lands
- **P2** (inventories and menus): all four tweaks + options.
- **P7** (multiplayer): transactional inventory actions with prediction.
