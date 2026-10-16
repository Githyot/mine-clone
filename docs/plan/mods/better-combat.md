# Better Combat (by daedelus_dev, with KosmX and Malfu)

**What it is:** RPG-style melee combat.
- **Attack animations**: one-handed, two-handed and dual-wield swings, visible in first and third person.
- **Swing-path collision**: weapons hit everything inside their swing arc, no precise cursor aim needed (great in third person).
- **Wind-up** before the hit lands; **combos**: weapons cycle through a sequence of different attacks (slash, stab, uppercut...).
- **Dual wielding** any pair of one-handed weapons (attack key alternates hands); two-handed weapons disable the off-hand.
- **Reworked Sweeping Edge**: multi-target hits deal reduced damage unless enchanted.
- **Hold to attack** (repeat on cooldown), **swing through grass** (zero-hardness blocks are not mined).
- Weapon behavior defined in **JSON attribute files**; unknown modded weapons get a preset by item ID (sword, axe, spear, claymore...).

**License:** All Rights Reserved. → **Ideas only.** Our own code and our own animations.

## How we build it (better than the Java version)
1. **Weapon attributes are item data**: each weapon item in the data pack has an `attack_set` (list of attacks: hitbox shape, angle,
   range, wind-up, active frames, damage multiplier, animation clip). Presets (`sword`, `spear`, `claymore`...) are data templates;
   the base game's swords and axes use the same system, so vanilla and modded weapons are identical citizens.
2. **Server-authoritative hit detection**: the server sweeps the attack shape (OBB/cone) through the entity spatial grid at the active
   frame, with lag compensation (rewind target positions by the client's RTT, capped). Java trusts the client's hit list.
3. Shape tests are cheap SIMD AABB-vs-cone checks over the few entities in nearby grid cells; no raycast per entity.
4. **Animations from our animation system**: attack clips layered on the player rig (upper body only, legs keep walking), GPU skinned,
   with first-person arms driven by the same clip. Client plays the swing instantly (prediction); server confirms hits.
5. **Combo state and dual-wield hand** are small player components; two-handed flag disables the off-hand slot in the menu logic.
6. **Swing through grass**: attack input checks the targeted block's hardness via the block registry (array lookup); grass is skipped.
7. Combat stays switchable: a world option chooses vanilla 1.9 combat or this mode; both share damage/knockback code from P2.
8. Event hooks (`on_attack_start`, `on_attack_hit`) let mods add weapon skills without patching.

## Where it lands
- **P2** (combat exists): attack-set data format, swept hit detection, basic animations.
- **P7**: lag compensation tuned for multiplayer. **P8**: full animation set and combos for every weapon.
- **M2**: attack-set registry and attack hooks for mods.
