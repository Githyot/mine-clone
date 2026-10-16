# Immersive UI (by SSKirillSS; art by K0zochka and Ke1o3i, code also by catboybinary)

**What it is:** client-side "juice" animations for the inventory and HUD:
- particle trail behind the item on your cursor when it has non-common rarity;
- wiggle/sway when you move the carried item quickly;
- the item under the cursor scales up;
- smooth sliding animation of the hotbar selector;
- container items matching the carried item float/bob so you can spot where it stacks;
- "and much more" (the page doesn't list the rest).

**License:** All Rights Reserved (the page says it may be used in modpacks). → **Ideas only.** Our own code and art.

## How we build it (better than the Java version)
Java mods do this by injecting into vanilla screen rendering with mixins, drawing extra items one by one in immediate mode each frame.
That costs frame time on big container screens and breaks with other GUI mods. Ours is built into the UI layer:
1. **UI animation is a core feature of our immediate-mode UI**: every widget has a small spring/tween state keyed by a stable widget
   ID (`scale`, `offset`, `rotation`, `glow`). Effects are tweens on that state, not extra draw passes.
2. **Item slots are drawn in one instanced batch** (item icon atlas/texture array + per-instance transform). Hover scaling, bobbing and
   wiggle are only per-instance values, so 100+ slots animated still cost one draw call.
3. **Cursor item wiggle** = a damped spring driven by cursor velocity (computed once per frame).
4. **Matching-item highlight**: when an item is picked up, the menu computes the matching slot list once (on pickup and on
   inventory change), not every frame per slot.
5. **Rarity trail** uses the GPU particle system in screen space with a capped particle count.
6. **Hotbar selector** slides with the same tween (also smooths scroll-wheel jumps).
7. **Data-driven theme**: effect strengths/curves live in a UI theme file in the resource-pack layer; one "reduce motion" switch in
   accessibility options disables all of it.
8. Mods with their own menus get the effects for free because they use the same slot widget.

## Where it lands
- **P1** (hotbar, creative inventory): tween/spring state in the UI library, animated hotbar selector.
- **P2** (inventories/menus): instanced slot batch, hover scale, matching-item float, cursor wiggle.
- **P8** (polish, accessibility): rarity trails, theme file, reduce-motion option.
- Budget: a full double-chest screen with every effect on < 0.2 ms CPU, 1–2 draw calls.
