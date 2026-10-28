//! Mobs living together: who is afraid of whom, who answers a call for help, who herds, plays,
//! courts, follows its mother or stops for a chat.
//!
//! A small director looks at the mobs near a player four times a second and hands out
//! intentions; `Mob::tick` (voxel/mob.rs) carries them out and anim.rs poses them:
//! - fear: creepers keep away from cats, skeletons from wolves, foxes from wolves and bears,
//!   town folk from zombies and raiders, prey from a predator that's hunting it;
//! - alarm: a hurt animal makes its herd scatter; a hurt zombie, wolf or piglin brings its
//!   kind into the fight (against the mob that did it, or the player); guards and sentinels
//!   go after a mob that attacks town folk or a baby;
//! - feuds: an arrow or bite from another mob is paid back (see `Mob::grudge`);
//! - herds keep together, friends nuzzle, playful ones (wolves, foxes, cats, babies) chase
//!   each other, animals in love find each other and have a baby, babies follow their parent;
//! - town folk walk up to each other, wave, and talk in turns; now and then they gather in
//!   the middle of their town.
//!
//! Cheap by design: only mobs within `NEAR` blocks of a player, at most `MAX_NEAR` of them,
//! a few distance checks each per pass.

use std::collections::HashMap;

use glam::Vec3;
use voxel::mob::{Act, Kind, Mob, MobDef};

use crate::mobs::{MobOutcome, Mobs};

/// Seconds between passes.
const PASS: f32 = 0.25;
const NEAR: f32 = 64.0;
const MAX_NEAR: usize = 160;
/// Seconds before animals that had a baby can have another.
const BREED_CD: f32 = 300.0;

/// (who, what it keeps away from)
const FEARS: &[(&[&str], &[&str])] = &[
    (&["stalker", "mutant_stalker", "nightwing"], &["cat", "cat_tame"]),
    (&["skeleton", "bogged", "frost_archer", "ash_skeleton", "risen_skeleton", "mutant_skeleton"], &["wolf", "wolf_tame", "spectral_wolf"]),
    (&["fox"], &["wolf", "wolf_tame", "polar_bear", "bear", "grizzly_bear"]),
    (&["piglin"], &["nether_zombie", "zoglin"]),
    (&["rabbit", "chicken", "duck", "squirrel"], &["fox", "cat", "wolf"]),
];

/// Monsters town folk run from.
const MONSTERS: &[&str] = &["zombie", "zombie_villager", "mutant_zombie", "marauder", "brute", "invoker", "ravager", "vex", "mummy", "sunken", "swamp_witch"];

/// Kinds that come to help when one of them is hurt (a zombie horde, a wolf pack...).
const PACKS: &[&str] = &["zombie", "zombie_villager", "nether_zombie", "mummy", "wolf", "wolf_tame", "piglin", "piglin_brute", "bee", "hyena", "lobo_guara", "boar", "crimson_boar", "hoglin", "polar_bear", "spectral_wolf"];

/// Kinds that romp with each other even when grown up.
const PLAYFUL: &[&str] = &["wolf", "wolf_tame", "fox", "cat", "cat_tame", "dolphin", "panda", "goat", "lobo_guara", "quati", "capuchin_monkey", "mico_leao_dourado", "raccoon", "squirrel", "axolotl", "otter", "seal", "penguin", "kangaroo"];

/// How one kind of mob gets on with the others (worked out once per registry).
#[derive(Default, Clone)]
pub(crate) struct Traits {
    /// Town folk: talk, greet, gather.
    folk: bool,
    /// Defends town folk and babies.
    guard: bool,
    herd: bool,
    playful: bool,
    pack: bool,
    /// Kinds it keeps away from, and how far.
    fears: Vec<(u16, f32)>,
    /// Kinds that eat it.
    hunted_by: Vec<u16>,
}

#[derive(Default)]
pub(crate) struct Director {
    clock: f32,
    pub traits: Vec<Traits>,
}

/// What the director decided this pass, applied once it has looked at everyone.
enum Order {
    Frighten(usize, Vec3, f32),
    Aggro(usize),
    Grudge(usize, u32, f32),
    Begin { i: usize, act: Act, at: Vec3, gap: f32, secs: f32, with: u32, first: bool },
    Refresh(usize, Vec3),
    End(usize),
    Birth(usize, usize),
}

/// What puts each animal in love (Minecraft's breeding foods, and a grazer's default).
pub(crate) fn breed_food(d: &MobDef) -> &'static [&'static str] {
    match d.name.as_str() {
        "cow" | "mooshroom" | "sheep" | "sheep_sheared" | "goat" | "ox" => &["wheat"],
        "pig" | "boar" => &["carrot", "potato", "beetroot"],
        "chicken" | "duck" => &["wheat_seeds", "beetroot_seeds", "melon_seeds", "pumpkin_seeds"],
        "rabbit" => &["carrot", "dandelion", "golden_carrot"],
        "horse" | "donkey" | "mule" | "zebra" => &["golden_carrot", "golden_apple", "apple"],
        "llama" | "camel" => &["hay_block", "cactus"],
        "wolf_tame" => &["beef", "porkchop", "mutton", "chicken", "rabbit", "cooked_beef", "cooked_porkchop", "cooked_mutton", "cooked_chicken"],
        // (A wild cat's fish tames it instead, pets.rs.)
        "cat_tame" => &["cod", "salmon"],
        "fox" => &["sweet_berries"],
        "panda" => &["bamboo"],
        "turtle" => &["seagrass"],
        "frog" => &["slime_ball"],
        "armadillo" => &["spider_eye"],
        "hoglin" => &["crimson_fungus"],
        "strider" => &["warped_fungus"],
        "bee" => &["poppy", "dandelion"],
        _ if d.kind == Kind::Passive && d.spawn == voxel::mob::SpawnRule::Grass && d.prey.is_empty() && !d.town && !d.flies && d.group.1 >= 2 => &["wheat", "apple"],
        _ => &[],
    }
}

fn folk_name(n: &str) -> bool {
    n.starts_with("npc_") || n.starts_with("settler_") || n == "goblin_trader"
}

pub(crate) fn traits(defs: &[MobDef]) -> Vec<Traits> {
    let ids = |names: &[&str]| -> Vec<u16> { names.iter().filter_map(|n| defs.iter().position(|d| d.name == *n)).map(|i| i as u16).collect() };
    defs.iter()
        .map(|d| {
            let n = d.name.as_str();
            let folk = folk_name(n);
            let guard = (folk && d.attack > 0.0) || n == "sentinel";
            let mut fears: Vec<(u16, f32)> = Vec::new();
            for (who, of) in FEARS {
                if who.contains(&n) {
                    fears.extend(ids(of).into_iter().map(|k| (k, 7.0)));
                }
            }
            if folk && !guard {
                fears.extend(ids(MONSTERS).into_iter().map(|k| (k, 9.0)));
            }
            let hunted_by = if d.kind == Kind::Passive && !folk && !guard {
                defs.iter().enumerate().filter(|(_, o)| o.prey.iter().any(|p| p == n) && !folk_name(&o.name) && o.name != "sentinel").map(|(k, _)| k as u16).collect()
            } else {
                Vec::new()
            };
            Traits {
                folk,
                guard,
                herd: d.kind != Kind::Hostile && !folk && !guard && d.group.1 >= 3 && !d.flies && !d.aquatic && !d.town,
                playful: PLAYFUL.contains(&n),
                pack: PACKS.contains(&n),
                fears,
                hunted_by,
            }
        })
        .collect()
}

/// A number in 0..1 from a roll and a salt, so one roll per mob serves several choices.
fn sub(r: f32, salt: u32) -> f32 {
    ((r * 7919.0 + salt as f32 * 0.618_034) * (1.0 + salt as f32 * 0.37)).fract()
}

impl Mobs {
    /// Run the social director every `PASS` seconds (players are `player` and `self.others`).
    pub(crate) fn social_pass(&mut self, player: Vec3, day: bool, dt: f32, out: &mut MobOutcome) {
        if dt <= 0.0 {
            return;
        }
        self.social.clock -= dt;
        if self.social.clock > 0.0 {
            return;
        }
        self.social.clock = PASS;
        if self.social.traits.len() != self.reg.defs.len() {
            self.social.traits = traits(&self.reg.defs);
        }
        let players: Vec<Vec3> = std::iter::once(player).chain(self.others.iter().map(|o| o.0)).collect();
        let near: Vec<usize> = (0..self.list.len())
            .filter(|&i| {
                let m = &self.list[i];
                m.alive() && !m.ghost && !m.ridden && players.iter().any(|p| (m.body.pos - *p).length_squared() < NEAR * NEAR)
            })
            .take(MAX_NEAR)
            .collect();
        if near.is_empty() {
            return;
        }
        let rolls: Vec<f32> = near.iter().map(|_| self.rand_f32()).collect();
        let orders = self.decide(&near, &rolls, &players, day, out);
        self.apply(orders, out);
    }

    fn decide(&self, near: &[usize], rolls: &[f32], players: &[Vec3], day: bool, out: &mut MobOutcome) -> Vec<Order> {
        let list = &self.list;
        let defs = &self.reg.defs;
        let tr = &self.social.traits;
        let ids: HashMap<u32, usize> = near.iter().map(|&i| (list[i].net_id, i)).collect();
        let mut orders = Vec::new();
        // The nearest other mob in reach that passes `f`.
        let nearest = |i: usize, r: f32, f: &dyn Fn(&Mob) -> bool| -> Option<(usize, f32)> {
            let p = list[i].body.pos;
            near.iter()
                .filter(|&&j| j != i)
                .map(|&j| (j, list[j].body.pos.distance(p)))
                .filter(|&(j, d)| d < r && f(&list[j]))
                .min_by(|a, b| a.1.total_cmp(&b.1))
        };
        for (k, &i) in near.iter().enumerate() {
            let m = &list[i];
            let d = &defs[m.def];
            let t = &tr[m.def];
            let r = rolls[k];
            let pos = m.body.pos;

            // Acts under way: follow the partner as it moves; a baby is born.
            if m.social.act != Act::None && m.social.with != 0 {
                match ids.get(&m.social.with).copied().filter(|&j| list[j].alive()) {
                    None => orders.push(Order::End(i)),
                    Some(j) => {
                        let q = list[j].body.pos;
                        let at = if m.social.act == Act::Play && !m.social.first {
                            let away = (pos - q).normalize_or(Vec3::new(m.yaw.sin(), 0.0, m.yaw.cos()));
                            pos + away * 4.0
                        } else {
                            q
                        };
                        orders.push(Order::Refresh(i, at));
                        let o = &list[j];
                        if m.social.act == Act::Mate && o.social.act == Act::Mate && o.social.with == m.net_id && i < j && pos.distance(q) < d.width + 0.9 && m.social.age > 2.0 {
                            orders.push(Order::Birth(i, j));
                        }
                    }
                }
            }

            // Fear.
            if m.scare < 0.6 {
                let threat = nearest(i, 12.0, &|o: &Mob| {
                    let od = o.def as u16;
                    let dist = o.body.pos.distance(pos);
                    if let Some(&(_, range)) = t.fears.iter().find(|f| f.0 == od) {
                        return dist < range && !o.baby();
                    }
                    if t.hunted_by.contains(&od) {
                        let after_me = o.prey.is_some_and(|p| p.distance(pos) < 1.5);
                        return after_me || (o.hungry(&defs[o.def]) && dist < 8.0) || dist < 4.0;
                    }
                    false
                });
                if let Some((j, _)) = threat {
                    orders.push(Order::Frighten(i, list[j].body.pos, 2.0));
                }
            }

            // Love: hearts, and the other one in love goes to it.
            if m.love > 0.0 && rolls[k] < 0.4 {
                out.hearts.push(pos + Vec3::Y * d.height * m.size);
            }

            // A call for help: just hurt.
            if m.hurt_age < PASS + 0.05 {
                let by = m.hurt_by;
                for &j in near {
                    if j == i || list[j].body.pos.distance(pos) > 16.0 {
                        continue;
                    }
                    let o = &list[j];
                    let (od, ot) = (&defs[o.def], &tr[o.def]);
                    let kin = o.def == m.def && o.body.pos.distance(pos) < 12.0;
                    if kin && d.kind == Kind::Passive && !t.guard && !t.pack {
                        // The herd scatters.
                        orders.push(Order::Frighten(j, pos, 3.0));
                    } else if kin && t.pack && !o.baby() && o.scare <= 0.0 {
                        if by != 0 && by != o.net_id {
                            orders.push(Order::Grudge(j, by, 15.0));
                        } else if by == 0 && od.kind != Kind::Passive {
                            orders.push(Order::Aggro(j));
                        }
                    } else if ot.guard && by != 0 && (t.folk || m.baby()) && od.attack > 0.0 {
                        orders.push(Order::Grudge(j, by, 20.0));
                    }
                }
            }

            if !m.idle_now() || m.leashed {
                continue;
            }
            // Pets stay with their player rather than wandering off with friends.
            if d.name.ends_with("_tame") && players.iter().all(|p| p.distance(pos) > 6.0) {
                continue;
            }

            // Babies keep near a parent (or any grown-up of their kind), and play.
            if m.baby() {
                let parent = ids.get(&m.parent).copied().filter(|&j| list[j].alive()).or_else(|| nearest(i, 16.0, &|o: &Mob| o.def == m.def && !o.baby()).map(|x| x.0));
                if r < 0.04 {
                    if let Some((j, _)) = nearest(i, 8.0, &|o: &Mob| o.def == m.def && o.baby() && o.idle_now()) {
                        play(&mut orders, list, i, j);
                        continue;
                    }
                }
                if let Some(j) = parent {
                    if list[j].body.pos.distance(pos) > 3.0 {
                        orders.push(Order::Begin { i, act: Act::Follow, at: list[j].body.pos, gap: 2.0, secs: 4.0, with: list[j].net_id, first: false });
                    }
                }
                continue;
            }

            // In love: find the other one.
            if m.love > 0.0 && m.breed_cd <= 0.0 {
                let mate = nearest(i, 10.0, &|o: &Mob| o.def == m.def && o.love > 0.0 && o.breed_cd <= 0.0 && !o.baby() && (o.idle_now() || o.social.with == m.net_id));
                if let Some((j, _)) = mate {
                    let gap = d.width * 0.6 + 0.2;
                    orders.push(Order::Begin { i, act: Act::Mate, at: list[j].body.pos, gap, secs: 10.0, with: list[j].net_id, first: true });
                    orders.push(Order::Begin { i: j, act: Act::Mate, at: pos, gap, secs: 10.0, with: m.net_id, first: false });
                }
                continue;
            }

            if t.folk {
                if r < 0.025 {
                    // Walk up to a neighbour and have a chat.
                    if let Some((j, _)) = nearest(i, 10.0, &|o: &Mob| folk_name(&defs[o.def].name) && o.idle_now()) {
                        let secs = 9.0 + sub(r, 1) * 6.0;
                        orders.push(Order::Begin { i, act: Act::Chat, at: list[j].body.pos, gap: 1.7, secs, with: list[j].net_id, first: true });
                        orders.push(Order::Begin { i: j, act: Act::Chat, at: pos, gap: 1.7, secs, with: m.net_id, first: false });
                        out.chatter.push((m.def, pos));
                    }
                } else if day && r > 0.996 {
                    // Meet in the middle of town.
                    let homes: Vec<Vec3> = near.iter().filter_map(|&j| list[j].home.filter(|_| tr[list[j].def].folk)).filter(|h| h.distance(pos) < 40.0).collect();
                    if homes.len() >= 2 {
                        let c = homes.iter().copied().sum::<Vec3>() / homes.len() as f32;
                        let a = sub(r, 2) * std::f32::consts::TAU;
                        orders.push(Order::Begin { i, act: Act::Follow, at: c + Vec3::new(a.cos(), 0.0, a.sin()) * 2.0, gap: 1.0, secs: 20.0, with: 0, first: false });
                    }
                }
                continue;
            }

            if d.kind == Kind::Hostile || t.guard {
                continue;
            }
            if r < 0.012 {
                // A friend close by: nose to nose for a moment.
                if let Some((j, _)) = nearest(i, 3.5 + d.width, &|o: &Mob| o.def == m.def && o.idle_now() && !o.baby()) {
                    let gap = d.width * 0.55 + 0.25;
                    orders.push(Order::Begin { i, act: Act::Nuzzle, at: list[j].body.pos, gap, secs: 3.5, with: list[j].net_id, first: true });
                    orders.push(Order::Begin { i: j, act: Act::Nuzzle, at: pos, gap, secs: 3.5, with: m.net_id, first: false });
                    out.chatter.push((m.def, pos));
                }
            } else if t.playful && r > 0.99 {
                if let Some((j, _)) = nearest(i, 8.0, &|o: &Mob| o.def == m.def && o.idle_now()) {
                    play(&mut orders, list, i, j);
                }
            } else if t.herd && (0.3..0.38).contains(&r) {
                // Strayed from the herd: drift back toward it.
                let mates: Vec<Vec3> = near.iter().filter(|&&j| j != i && list[j].def == m.def).map(|&j| list[j].body.pos).filter(|p| p.distance(pos) < 20.0).collect();
                if !mates.is_empty() && mates.iter().all(|p| p.distance(pos) > 6.0) {
                    let c = mates.iter().copied().sum::<Vec3>() / mates.len() as f32;
                    orders.push(Order::Begin { i, act: Act::Follow, at: c, gap: 3.0, secs: 5.0, with: 0, first: false });
                }
            }
        }
        orders
    }

    fn apply(&mut self, orders: Vec<Order>, out: &mut MobOutcome) {
        for o in orders {
            match o {
                Order::Frighten(i, from, secs) => self.list[i].frighten(from, secs),
                Order::Aggro(i) => self.list[i].aggro = true,
                Order::Grudge(i, foe, secs) => {
                    let m = &mut self.list[i];
                    if m.grudge <= 0.0 || m.foe == foe {
                        m.grudge = m.grudge.max(secs);
                        m.foe = foe;
                        m.social.act = Act::None;
                    }
                }
                Order::Begin { i, act, at, gap, secs, with, first } => {
                    let m = &mut self.list[i];
                    m.begin(act, at, gap, secs, with);
                    m.social.first = first;
                }
                Order::Refresh(i, at) => self.list[i].social.at = at,
                Order::End(i) => self.list[i].social.act = Act::None,
                Order::Birth(i, j) => {
                    let (a, b) = (self.list[i].body.pos, self.list[j].body.pos);
                    let at = (a + b) * 0.5;
                    let def = self.list[i].def;
                    let parent = self.list[i].net_id;
                    for k in [i, j] {
                        let m = &mut self.list[k];
                        m.love = 0.0;
                        m.breed_cd = BREED_CD;
                        m.social.act = Act::None;
                    }
                    let s = self.next_rand();
                    let mut baby = Mob::new(def, &self.reg.defs[def], at, s);
                    baby.make_baby(&self.reg.defs[def], parent);
                    baby.yaw = self.list[i].yaw;
                    self.push_mob(baby);
                    out.births.push((def, at));
                    let h = self.reg.defs[def].height;
                    for n in 0..6 {
                        out.hearts.push(at + Vec3::new((n as f32 * 1.7).cos() * 0.4, h * 0.6 + n as f32 * 0.08, (n as f32 * 1.7).sin() * 0.4));
                    }
                }
            }
        }
    }
}

/// Can the player give it a pat: an animal (with a head), not a monster, town folk, a cart or
/// a saddled mount (right click rides those).
fn pettable(d: &MobDef, m: &Mob) -> bool {
    d.kind != Kind::Hostile && !d.boss && !folk_name(&d.name) && d.name != "sentinel" && !m.saddled && !m.ghost && d.parts.iter().any(|p| p.anim == voxel::mob::Anim::Head) && !d.name.starts_with("wagon") && d.name != "armor_stand"
}

impl crate::Game {
    /// Right click on an animal: its food puts it in love (or helps a baby grow), an empty
    /// hand pets it.
    pub(crate) fn animal_use(&mut self) -> bool {
        let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), 4.0) else { return false };
        if self.target.is_some_and(|h| h.distance < t && self.reg.get(self.world.block_or_air(h.block)).solid) {
            return false;
        }
        let m = &self.mobs.list[i];
        if !m.alive() || m.ghost {
            return false;
        }
        let def = &self.mobs.reg.defs[m.def];
        let held = self.held_key();
        let glow = self.glow_layer();
        let top = m.body.pos + Vec3::Y * (def.height * def.scale.max(0.5) * m.size);
        if !held.is_empty() && breed_food(def).contains(&held.as_str()) {
            let m = &mut self.mobs.list[i];
            if m.baby() {
                m.grow = (m.grow - voxel::mob::GROW_TIME * 0.1).max(0.01);
            } else if m.love > 0.0 || m.breed_cd > 0.0 {
                return false;
            } else {
                m.love = 30.0;
                m.social.act = Act::None;
            }
            if self.survival {
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
            }
            for k in 0..7 {
                let a = k as f32 * 0.9;
                self.particles.glow(top + Vec3::new(a.cos() * 0.35, 0.1, a.sin() * 0.35), Vec3::Y * 0.7, glow, [1.0, 0.32, 0.42], 0.065, 1.2);
            }
            self.sfx_at(crate::audio::Sfx::Eat, top, 0.7, 12.0);
            self.anim.start_swing();
            return true;
        }
        if !held.is_empty() || !pettable(def, m) || self.crouch > 0.5 {
            return false;
        }
        let me = self.body.pos;
        let name = def.name.clone();
        let m = &mut self.mobs.list[i];
        m.begin(Act::Petted, me, 99.0, 2.6, 0);
        m.social.age = 0.0;
        m.sleeping = false;
        for k in 0..4 {
            let a = k as f32 * 1.6;
            self.particles.glow(top + Vec3::new(a.cos() * 0.3, 0.15, a.sin() * 0.3), Vec3::Y * 0.6, glow, [1.0, 0.32, 0.42], 0.06, 1.1);
        }
        if let Some(fx) = crate::mob_sounds::pick(&name, crate::sfx_extra::Call::Idle, 1.0, self.surv.rng.f32(), self.surv.rng.f32()) {
            self.sfx_at(crate::audio::Sfx::Fx(fx), top, 0.6, 16.0);
        }
        self.anim.start_swing();
        true
    }

    /// What the mobs' social life made this tick: hearts, births, voices.
    pub(crate) fn social_effects(&mut self, out: &MobOutcome) {
        let glow = self.glow_layer();
        for &h in &out.hearts {
            let j = Vec3::new(self.surv.rng.f32() - 0.5, 0.0, self.surv.rng.f32() - 0.5) * 0.5;
            self.particles.glow(h + j, Vec3::Y * 0.6, glow, [1.0, 0.32, 0.42], 0.06, 1.1);
        }
        for &(def, at) in &out.births {
            self.prog.stats.add("breeds", 1);
            self.prog.event(crate::progression::Event::Token("bred".into()));
            if self.survival && at.distance(self.body.pos) < 24.0 {
                let n = 1 + (self.surv.rng.f32() * 6.0) as u32;
                self.spawn_orbs(at + Vec3::Y * 0.5, n);
            }
            let name = self.mobs.reg.defs[def].name.clone();
            if let Some(fx) = crate::mob_sounds::pick(&name, crate::sfx_extra::Call::Idle, 0.7, self.surv.rng.f32(), 0.9) {
                self.sfx_at(crate::audio::Sfx::Fx(fx), at, 0.7, 20.0);
            }
        }
        for &(def, at) in &out.chatter {
            if at.distance(self.body.pos) > 20.0 {
                continue;
            }
            let name = self.mobs.reg.defs[def].name.clone();
            if let Some(fx) = crate::mob_sounds::pick(&name, crate::sfx_extra::Call::Idle, 1.0, self.surv.rng.f32(), self.surv.rng.f32()) {
                self.sfx_at(crate::audio::Sfx::Fx(fx), at, 0.45, 16.0);
            }
        }
    }
}

/// Test hook MC_SOCIAL_RUN: screenshots keep the mobs moving at a fixed 15 steps a second
/// (with MC_PLACE=frames that makes a frame-by-frame film).
pub(crate) fn film_dt() -> f32 {
    if std::env::var("MC_SOCIAL_RUN").is_ok() {
        1.0 / 15.0
    } else {
        0.0
    }
}

impl crate::Game {
    /// Test hook MC_SOCIAL=vila|fazenda|caca|brincar: a little scene of mobs getting on (or
    /// not) in front of the camera, already MC_SOCIAL_T seconds (default 3) under way.
    pub(crate) fn test_social(&mut self, scene: &str) {
        let f = self.forward();
        let fwd = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-fwd.z, 0.0, fwd.x);
        let me = self.body.pos;
        let put = |g: &mut crate::Game, name: &str, ahead: f32, across: f32| -> Option<usize> {
            let p = me + fwd * ahead + side * across;
            let (x, z) = (p.x.floor() as i32, p.z.floor() as i32);
            let y = g.ground_y(x, z, me.y as i32 + 4).unwrap_or(p.y as i32) + 1;
            let at = Vec3::new(p.x, y as f32, p.z);
            if !g.mobs.spawn(name, at) {
                return None;
            }
            let m = g.mobs.list.last_mut().unwrap();
            m.yaw = (me - at).x.atan2((me - at).z);
            Some(g.mobs.list.len() - 1)
        };
        let pair = |g: &mut crate::Game, a: Option<usize>, b: Option<usize>, act: Act, gap: f32, secs: f32| {
            let (Some(a), Some(b)) = (a, b) else { return };
            let (pa, pb) = (g.mobs.list[a].body.pos, g.mobs.list[b].body.pos);
            let (ia, ib) = (g.mobs.list[a].net_id, g.mobs.list[b].net_id);
            g.mobs.list[a].begin(act, pb, gap, secs, ib);
            g.mobs.list[a].social.first = true;
            g.mobs.list[b].begin(act, pa, gap, secs, ia);
            if act == Act::Play {
                g.mobs.list[b].social.at = pb + (pb - pa).normalize_or(Vec3::X) * 4.0;
            }
        };
        match scene {
            "vila" => {
                let a = put(self, "npc_guide", 6.0, -3.2);
                let b = put(self, "npc_merchant", 7.5, -1.0);
                pair(self, a, b, Act::Chat, 1.7, 60.0);
                let c = put(self, "npc_nurse", 6.5, 1.6);
                let d = put(self, "npc_farmer", 8.0, 3.4);
                pair(self, c, d, Act::Chat, 1.7, 60.0);
                put(self, "npc_guard", 10.5, 0.2);
            }
            "fazenda" => {
                let a = put(self, "cow", 6.5, -3.0);
                let b = put(self, "cow", 7.5, -1.6);
                pair(self, a, b, Act::Nuzzle, 1.0, 60.0);
                if let Some(m) = put(self, "cow", 9.0, 2.5) {
                    let id = self.mobs.list[m].net_id;
                    for (ahead, across) in [(7.0, 4.2), (6.3, 2.6)] {
                        if let Some(c) = put(self, "cow", ahead, across) {
                            let d = self.mobs.list[c].def;
                            let def = self.mobs.reg.defs[d].clone();
                            self.mobs.list[c].make_baby(&def, id);
                        }
                    }
                }
                for across in [0.2, 1.4] {
                    if let Some(s) = put(self, "sheep", 5.0, across) {
                        self.mobs.list[s].love = 60.0;
                    }
                }
                if let Some(p) = put(self, "pig", 4.2, -1.2) {
                    let me = self.body.pos;
                    self.mobs.list[p].begin(Act::Petted, me, 99.0, 60.0, 0);
                }
            }
            "caca" => {
                if let Some(w) = put(self, "wolf", 10.0, -3.5) {
                    self.mobs.list[w].fed = 0.0;
                }
                put(self, "sheep", 7.0, -2.0);
                put(self, "sheep", 7.5, -0.8);
                put(self, "cat", 6.0, 2.5);
                put(self, "stalker", 7.5, 3.0);
                if let Some(s) = put(self, "skeleton", 10.0, 1.0) {
                    self.mobs.list[s].aggro = false;
                }
                put(self, "wolf_tame", 11.5, 2.0);
            }
            _ => {
                let a = put(self, "wolf", 7.0, -2.0);
                let b = put(self, "wolf", 8.0, -0.5);
                pair(self, a, b, Act::Play, 0.8, 60.0);
                for across in [1.5, 2.6] {
                    if let Some(p) = put(self, "pig", 5.5, across) {
                        let d = self.mobs.list[p].def;
                        let def = self.mobs.reg.defs[d].clone();
                        self.mobs.list[p].make_baby(&def, 0);
                    }
                }
                let p = self.mobs.list.len();
                if p >= 2 {
                    pair(self, Some(p - 2), Some(p - 1), Act::Play, 0.8, 60.0);
                }
            }
        }
        let secs = std::env::var("MC_SOCIAL_T").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(3.0);
        let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
        let daylight = (a.sin() * 2.2 + 0.45).clamp(0.0, 1.0);
        let steps = (secs / 0.05) as usize;
        for _ in 0..steps {
            let out = self.mobs.update(&self.world, self.body.pos, true, daylight, 0.05);
            self.social_effects(&out);
            self.mobs.animate(self.body.pos, 0.05);
            self.particles.update(&self.world, 0.05);
        }
    }
}

/// `i` chases `j`, who runs off laughing (both run; the chaser has `first`).
fn play(orders: &mut Vec<Order>, list: &[Mob], i: usize, j: usize) {
    let (p, q) = (list[i].body.pos, list[j].body.pos);
    orders.push(Order::Begin { i, act: Act::Play, at: q, gap: 0.8, secs: 5.0, with: list[j].net_id, first: true });
    orders.push(Order::Begin { i: j, act: Act::Play, at: q + (q - p).normalize_or(Vec3::X) * 4.0, gap: 0.8, secs: 5.0, with: list[i].net_id, first: false });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mobs() -> Mobs {
        Mobs::new(crate::mobs::registry_for_tests(), voxel::AIR, 7)
    }

    fn put(ms: &mut Mobs, name: &str, at: Vec3) -> usize {
        assert!(ms.spawn(name, at), "{name}");
        ms.list.len() - 1
    }

    fn passes(ms: &mut Mobs, n: usize) {
        let mut out = MobOutcome::default();
        for _ in 0..n {
            ms.social_pass(Vec3::ZERO, true, PASS, &mut out);
        }
    }

    #[test]
    fn creepers_fear_cats_and_skeletons_fear_wolves() {
        let mut ms = mobs();
        let c = put(&mut ms, "stalker", Vec3::new(0.0, 10.0, 0.0));
        put(&mut ms, "cat", Vec3::new(3.0, 10.0, 0.0));
        let s = put(&mut ms, "skeleton", Vec3::new(20.0, 10.0, 0.0));
        put(&mut ms, "wolf", Vec3::new(24.0, 10.0, 0.0));
        let z = put(&mut ms, "zombie", Vec3::new(-20.0, 10.0, 0.0));
        passes(&mut ms, 1);
        assert!(ms.list[c].scare > 0.0);
        assert!(ms.list[s].scare > 0.0);
        assert_eq!(ms.list[z].scare, 0.0);
    }

    #[test]
    fn town_folk_chat_in_pairs() {
        let mut ms = mobs();
        let a = put(&mut ms, "npc_guide", Vec3::new(0.0, 10.0, 0.0));
        let b = put(&mut ms, "npc_merchant", Vec3::new(4.0, 10.0, 0.0));
        passes(&mut ms, 400);
        // Somewhere in 100 seconds they must have talked; check the pairing logic directly too.
        let mut out = MobOutcome::default();
        ms.list[a].social.act = Act::None;
        ms.list[b].social.act = Act::None;
        let orders = ms.decide(&[a, b], &[0.01, 0.5], &[Vec3::ZERO], true, &mut out);
        ms.apply(orders, &mut out);
        assert_eq!(ms.list[a].social.act, Act::Chat);
        assert_eq!(ms.list[b].social.act, Act::Chat);
        assert_eq!(ms.list[a].social.with, ms.list[b].net_id);
        assert!(ms.list[a].social.first && !ms.list[b].social.first);
        // They take turns: never both speaking.
        for step in 0..60 {
            let age = step as f32 * 0.2;
            ms.list[a].social.age = age;
            ms.list[b].social.age = age;
            assert!(!(ms.list[a].social.speaking() && ms.list[b].social.speaking()));
        }
    }

    #[test]
    fn two_in_love_have_a_baby_that_follows() {
        let mut ms = mobs();
        let a = put(&mut ms, "cow", Vec3::new(0.0, 10.0, 0.0));
        let b = put(&mut ms, "cow", Vec3::new(1.2, 10.0, 0.0));
        for i in [a, b] {
            ms.list[i].love = 30.0;
        }
        let mut out = MobOutcome::default();
        ms.social_pass(Vec3::ZERO, true, PASS, &mut out);
        assert_eq!(ms.list[a].social.act, Act::Mate);
        for i in [a, b] {
            ms.list[i].social.age = 3.0;
        }
        ms.social.clock = 0.0;
        ms.social_pass(Vec3::ZERO, true, PASS, &mut out);
        assert_eq!(out.births.len(), 1);
        let baby = ms.list.last().unwrap();
        assert!(baby.baby() && baby.size < 0.6);
        assert_eq!(baby.parent, ms.list[a].net_id);
        assert!(ms.list[a].breed_cd > 0.0 && ms.list[a].love == 0.0);
        // Out of reach of its mother, the calf goes after her.
        let n = ms.list.len() - 1;
        ms.list[n].body.pos = Vec3::new(8.0, 10.0, 0.0);
        ms.social.clock = 0.0;
        ms.social_pass(Vec3::ZERO, true, PASS, &mut out);
        assert_eq!(ms.list[n].social.act, Act::Follow);
    }

    #[test]
    fn a_hurt_zombie_calls_the_horde_and_guards_defend_folk() {
        let mut ms = mobs();
        let z = put(&mut ms, "zombie", Vec3::new(0.0, 10.0, 0.0));
        let z2 = put(&mut ms, "zombie", Vec3::new(5.0, 10.0, 0.0));
        ms.list[z].hurt(1.0, Vec3::new(-2.0, 10.0, 0.0));
        ms.list[z2].aggro = false;
        passes(&mut ms, 1);
        assert!(ms.list[z2].aggro, "horde");

        let mut ms = mobs();
        let f = put(&mut ms, "npc_guide", Vec3::new(0.0, 10.0, 0.0));
        let g = put(&mut ms, "npc_guard", Vec3::new(6.0, 10.0, 0.0));
        let z = put(&mut ms, "zombie", Vec3::new(-1.0, 10.0, 0.0));
        let zid = ms.list[z].net_id;
        ms.list[f].hurt(2.0, Vec3::new(-1.0, 10.0, 0.0));
        ms.list[f].hurt_by = zid;
        passes(&mut ms, 1);
        assert_eq!(ms.list[g].foe, zid);
        assert!(ms.list[g].grudge > 0.0);
        // The guide runs from the zombie.
        assert!(ms.list[f].scare > 0.0);
    }

    #[test]
    fn foods_for_the_farm_animals() {
        let reg = crate::mobs::registry_for_tests();
        let food = |n: &str| breed_food(&reg.defs[reg.id(n).unwrap()]);
        assert_eq!(food("cow"), &["wheat"]);
        assert!(food("chicken").contains(&"wheat_seeds"));
        assert!(food("zombie").is_empty());
        assert!(food("npc_guide").is_empty());
    }
}
