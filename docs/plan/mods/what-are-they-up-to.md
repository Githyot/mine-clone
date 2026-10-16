# What Are They Up To — Watut (by Corosus)

**What it is:** multiplayer body-language: shows a typing animation above players who are typing (faster typing = more chaotic), a player
with a screen open moves their hand as if tapping a floating screen, players idle 5+ minutes visibly fall asleep with "Zz" in the tab list.
Privacy: commands being typed are hidden, other players' inventory contents are never shown. Needs client + server, and CoroUtil.

**License:** All Rights Reserved (modpack use allowed). → **Ideas only.**

## How we build it (better than the Java version)
1. **Player activity state** is one tiny field in our player sync message: `{typing, typing_speed, screen_open(kind), idle}`, a couple of bytes,
   only sent when it changes. No separate mod channel.
2. Animations are just extra layers in the animation system (Fresh Animations work): `typing`, `using_screen`, `sleeping_idle`.
3. Privacy by design: the server never forwards what is typed or what's in the screen, only the state.
4. Each indicator toggleable per player in Options (show mine / show others).

## Where it lands
- **P7** (multiplayer), using the **P2** animation system.
