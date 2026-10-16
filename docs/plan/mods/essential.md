# Essential (by Spark Universe)

**What it is:** an all-in-one social mod: host your singleplayer world for friends peer-to-peer for free (no server rental), friends list,
direct and group chat, screenshot manager and sharing, world/server invites, plus cosmetics and emotes (its business model).

**License:** All Rights Reserved. → **Ideas only.**

## How we build it (better than the Java version)
Java needs a separate account backend and tunnels traffic through it. We already designed the game so singleplayer runs an integrated
server speaking our multiplayer protocol, so "open to friends" is almost free:

1. **Host World button** in the pause menu: the integrated server starts listening on QUIC; we use UDP hole punching so friends connect
   directly, with a small relay server as fallback when NAT blocks it. No port forwarding.
2. **Invite codes / links** (short code like `K7Q-4MZ`) resolved by a tiny matchmaking service; works without accounts on LAN.
3. **Friends, chat, groups, presence** need a backend: plan it as an optional online service (`services/social`) built after the game works;
   the game never depends on it to start or play.
4. **Screenshot manager** built in: F2 saves, a gallery screen to browse, copy, share.
5. Cosmetics/emotes: out of scope until much later (needs a content pipeline and a business decision).

## Where it lands
- **P7**: Host World, hole punching + relay, invite codes, screenshot gallery.
- **After 1.0**: social backend (friends, chat, groups, presence).
