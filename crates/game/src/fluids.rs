//! Water and lava that flow. A liquid's level lives in its id (`water:3`): 0 is a source,
//! 1..7 is how far it has flowed, +8 means it is falling. Nothing moves until a block next to
//! it changes, so oceans and lakes cost nothing while left alone.

use std::collections::{HashMap, HashSet};

use glam::IVec3;
use voxel::block::{BlockId, BlockRegistry, Shape, AIR};

use crate::world::World;

const SIDES: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

/// Seconds between steps of flow.
const WATER_DELAY: f64 = 0.25;
const LAVA_DELAY: f64 = 1.5;
/// Most updates handled per frame, so a broken dam can't stall the game.
const BUDGET: usize = 3000;

pub struct Fluids {
    /// Each liquid's variants by level and falling flag: `ids[fluid][variant]`.
    ids: [[BlockId; 16]; 3],
    replaceable: Vec<bool>,
    stone: BlockId,
    cobble: BlockId,
    obsidian: BlockId,
    queue: HashMap<IVec3, f64>,
    pub time: f64,
}

/// What the fluid sim may have changed this frame (for sounds).
#[derive(Default)]
pub struct FluidEvents {
    /// Lava met water here and turned to stone.
    pub hardened: Vec<IVec3>,
}

impl Fluids {
    pub fn new(reg: &BlockRegistry) -> Fluids {
        let mut ids = [[AIR; 16]; 3];
        for (f, name) in [(1, "water"), (2, "lava")] {
            if let Some(base) = reg.id(name) {
                ids[f][0] = base;
                for v in 1..16 {
                    ids[f][v] = reg.id(&format!("{name}:{v}")).unwrap_or(base);
                }
            }
        }
        // Cursed and coral torches burn on under water, like in Terraria.
        let wet_proof = |b: &voxel::block::Block| matches!(reg.blocks[b.base as usize].name.as_str(), "cursed_torch" | "coral_torch");
        let replaceable = reg.blocks.iter().map(|b| (matches!(b.shape, Shape::Cross | Shape::Torch) && !wet_proof(b)) || b.name == "air").collect();
        Fluids {
            ids,
            replaceable,
            stone: reg.id_or_air("stone"),
            cobble: reg.id_or_air("cobblestone"),
            obsidian: reg.id_or_air("obsidian"),
            queue: HashMap::new(),
            time: 0.0,
        }
    }

    fn level(&self, world: &World, id: BlockId) -> Option<(u8, u8)> {
        let f = world.fluid(id);
        (f != 0).then(|| (f, world.reg.get(id).variant))
    }

    fn schedule(&mut self, world: &World, p: IVec3) {
        let f = world.fluid(world.block_or_air(p));
        if f != 0 {
            let due = self.time + if f == 1 { WATER_DELAY } else { LAVA_DELAY };
            self.queue.entry(p).or_insert(due);
        }
    }

    pub fn update(&mut self, world: &mut World, dt: f32) -> FluidEvents {
        self.time += dt as f64;
        let mut ev = FluidEvents::default();
        for p in std::mem::take(&mut world.changed) {
            self.schedule(world, p);
            for d in [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z] {
                self.schedule(world, p + d);
            }
        }
        if self.queue.is_empty() {
            return ev;
        }
        let mut due: Vec<IVec3> = self.queue.iter().filter(|(_, &t)| t <= self.time).map(|(p, _)| *p).collect();
        due.sort_by_key(|p| p.y);
        due.truncate(BUDGET);
        for p in &due {
            self.queue.remove(p);
        }
        for p in due {
            self.step(world, p, &mut ev);
        }
        ev
    }

    fn can_enter(&self, world: &World, p: IVec3) -> bool {
        match world.block(p) {
            Some(id) => self.replaceable[id as usize],
            None => false,
        }
    }

    fn step(&mut self, world: &mut World, p: IVec3, ev: &mut FluidEvents) {
        let id = world.block_or_air(p);
        let Some((f, v)) = self.level(world, id) else { return };
        let max = if f == 1 { 7 } else { 3 };
        let source = v == 0;

        // Lava touching water hardens: a source into obsidian, flowing lava into cobblestone.
        if f == 2 {
            let wet = [IVec3::Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z].iter().any(|d| world.fluid(world.block_or_air(p + *d)) == 1);
            if wet {
                world.set_block(p, if source { self.obsidian } else { self.cobble });
                ev.hardened.push(p);
                return;
            }
        }

        // A flowing block re-checks what feeds it.
        if !source {
            let above = self.level(world, world.block_or_air(p + IVec3::Y)).filter(|l| l.0 == f).is_some();
            let mut best = u8::MAX;
            let mut sources = 0;
            for d in SIDES {
                if let Some((nf, nv)) = self.level(world, world.block_or_air(p + d)) {
                    if nf == f {
                        let l = if nv & 8 != 0 { 0 } else { nv & 7 };
                        if nv == 0 {
                            sources += 1;
                        }
                        best = best.min(l);
                    }
                }
            }
            let below = world.block_or_air(p - IVec3::Y);
            let new_v = if f == 1 && sources >= 2 && (world.reg.get(below).solid && world.fluid(below) == 0 || world.reg.get(below).base == self.ids[1][0] && world.reg.get(below).variant == 0) {
                Some(0)
            } else if above {
                Some(8)
            } else if best < max {
                Some(best + 1)
            } else {
                None
            };
            match new_v {
                None => {
                    world.set_block(p, AIR);
                    return;
                }
                Some(nv) if nv != v => {
                    // The set wakes this block again, and it spreads on that turn.
                    world.set_block(p, self.ids[f as usize][nv as usize]);
                    return;
                }
                _ => {}
            }
        }

        // Fall first.
        let down = p - IVec3::Y;
        if f == 2 && world.fluid(world.block_or_air(down)) == 1 {
            world.set_block(down, self.stone);
            ev.hardened.push(down);
            return;
        }
        let below = world.block_or_air(down);
        if self.can_enter(world, down) {
            world.set_block(down, self.ids[f as usize][8]);
            if !source {
                return;
            }
        } else if world.fluid(below) == f && !source {
            // Already pouring into more of itself.
            return;
        }

        // Then spread sideways, toward the nearest drop if there is one in reach.
        let next = if v & 8 != 0 { 1 } else { (v & 7) + 1 };
        if next > max {
            return;
        }
        let reach = if f == 1 { 4 } else { 2 };
        let mut dist = [u8::MAX; 4];
        for (k, d) in SIDES.iter().enumerate() {
            let q = p + *d;
            if self.open(world, q, f, next) {
                dist[k] = self.drop_distance(world, q, *d, reach);
            }
        }
        let shortest = *dist.iter().min().unwrap();
        if shortest == u8::MAX {
            return;
        }
        for (k, d) in SIDES.iter().enumerate() {
            let q = p + *d;
            if self.open(world, q, f, next) && (dist[k] == shortest || shortest > reach) {
                world.set_block(q, self.ids[f as usize][next as usize]);
            }
        }
    }

    /// Room for this liquid at `level`: empty, or the same liquid flowing thinner.
    fn open(&self, world: &World, q: IVec3, f: u8, level: u8) -> bool {
        if self.can_enter(world, q) {
            return true;
        }
        match self.level(world, world.block_or_air(q)) {
            Some((nf, nv)) => nf == f && nv & 8 == 0 && nv != 0 && (nv & 7) > level,
            None => false,
        }
    }

    /// Steps from `q` to a hole the liquid could pour into, within `reach`; reach + 1 when none.
    fn drop_distance(&self, world: &World, q: IVec3, came: IVec3, reach: u8) -> u8 {
        let mut seen = HashSet::new();
        let mut front = vec![q];
        seen.insert(q);
        for step in 0..=reach {
            let mut next = Vec::new();
            for c in front {
                if self.can_enter(world, c - IVec3::Y) || world.fluid(world.block_or_air(c - IVec3::Y)) != 0 && world.reg.get(world.block_or_air(c - IVec3::Y)).variant != 0 {
                    return step;
                }
                for d in SIDES {
                    if d == -came && c == q {
                        continue;
                    }
                    let n = c + d;
                    if seen.insert(n) && self.can_enter(world, n) {
                        next.push(n);
                    }
                }
            }
            front = next;
        }
        reach + 1
    }

    /// Ids of a liquid (1 water, 2 lava) at a variant, for buckets and tests.
    pub fn id(&self, fluid: u8, variant: u8) -> BlockId {
        self.ids[fluid as usize][variant as usize]
    }
}
