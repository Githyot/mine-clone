//! Redstone. Levers, buttons, pressure plates, redstone torches and blocks of redstone give
//! power; dust carries it, one level weaker per block (15 down to 0); lamps, doors,
//! trapdoors, pistons and TNT react to it. Repeaters pass it on after a delay, and torches turn
//! off when the block they stand on is powered, which makes inverters and clocks.
//!
//! Like the classic game, a block can be powered two ways: strongly (a torch under it, a
//! lever on it, a repeater pointing into it), which also feeds dust next to it, or weakly
//! (dust running onto it), which only reaches lamps, pistons and the like beside it.
//!
//! Work only happens around blocks that changed: each edit wakes the redstone within two
//! blocks of it, dust networks are recomputed as a whole, and torches, repeaters, buttons and
//! lamps going dark wait on timers counted in redstone ticks (a tenth of a second).

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::Arc;

use glam::IVec3;
use voxel::block::{BlockId, BlockRegistry, Shape, AIR};
use voxel::shapes::{dir6_of, DIRS6};

/// One redstone tick.
pub const RT: f64 = 0.1;

/// The blocks redstone reads and writes. `get` is None where nothing is loaded.
pub trait Grid {
    fn get(&self, p: IVec3) -> Option<BlockId>;
    fn set(&mut self, p: IVec3, id: BlockId) -> bool;
    /// Remove a block the way breaking does (both halves of a door).
    fn remove(&mut self, p: IVec3) {
        self.set(p, AIR);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    None,
    Wire,
    /// Lit or not.
    Torch(bool),
    Block,
    Lever,
    /// Wooden ones stay down longer.
    Button(bool),
    /// Wooden ones also feel items.
    Plate(bool),
    Lamp(bool),
    Repeater(bool),
    /// Sticky or not.
    Piston(bool),
    Head,
    Tnt,
    Door,
    Trapdoor,
}

/// What a piston does with a block in its way.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Push {
    /// Nothing there worth keeping (air, water): the line ends here.
    Free,
    /// Pops off (plants, dust, torches, doors): the line ends here.
    Break,
    /// Moves along.
    Move,
    /// Holds the piston back (bedrock, obsidian, chests, a piston that is out).
    Stop,
}

/// Things the game should show or do after an update.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A block broke off (lost its support or was in a piston's way): drop it.
    Popped(IVec3, BlockId),
    /// TNT lit by power.
    Prime(IVec3),
    /// A lever, button or plate clicked.
    Click(IVec3),
    /// A piston pushed out (true) or pulled back.
    Piston(IVec3, bool),
    /// Blocks starting at `from` moved one step along `dir` (to shove whoever stands there).
    Moved { from: IVec3, dir: IVec3, count: i32 },
    /// A torch switched too fast and burnt out for a while.
    Burnout(IVec3),
}

const SIDES: [IVec3; 6] = [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z];
/// North, east, south, west, the order of facings in block variants.
const H4: [IVec3; 4] = [IVec3::NEG_Z, IVec3::X, IVec3::Z, IVec3::NEG_X];

fn h4(v: u8) -> IVec3 {
    H4[(v & 3) as usize]
}

fn dir6(v: u8) -> IVec3 {
    IVec3::from_array(DIRS6[dir6_of(v)])
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Timer {
    /// Due time in microseconds, negated so the heap pops the earliest first.
    due: std::cmp::Reverse<i64>,
    p: [i32; 3],
}

pub struct Redstone {
    reg: Arc<BlockRegistry>,
    kind: Vec<Kind>,
    /// Full blocks: dust, torches and levers can sit on them and they carry power.
    opaque: Vec<bool>,
    support: Vec<bool>,
    push: Vec<Push>,
    /// How dust joins each block (see `voxel::mesh::redstone_join`).
    join: Vec<u8>,
    wire: BlockId,
    torch: (BlockId, BlockId),
    lamp: (BlockId, BlockId),
    repeater: (BlockId, BlockId),
    head: BlockId,
    time: f64,
    queue: Vec<IVec3>,
    queued: HashSet<IVec3>,
    timers: BinaryHeap<Timer>,
    timed: HashSet<IVec3>,
    /// Doors remember whether they were powered, so they only move when the power changes
    /// and can still be opened by hand.
    door_power: HashMap<IVec3, bool>,
    /// Torch switches in the last second, and torches burnt out until a time.
    flips: HashMap<IVec3, (f64, u32)>,
    burnt: HashMap<IVec3, f64>,
    /// Plates something stands on this frame.
    occupied: HashSet<IVec3>,
    pub events: Vec<Event>,
}

impl Redstone {
    pub fn new(reg: Arc<BlockRegistry>) -> Redstone {
        let id = |n: &str| reg.id(n).unwrap_or(AIR);
        let kind = reg
            .blocks
            .iter()
            .map(|b| {
                let base = reg.get(b.base).name.as_str();
                match b.shape {
                    Shape::Wire => Kind::Wire,
                    Shape::Lever => Kind::Lever,
                    Shape::Button => Kind::Button(!base.starts_with("stone")),
                    Shape::Plate => Kind::Plate(!base.starts_with("stone")),
                    Shape::Repeater => Kind::Repeater(base.ends_with("_on")),
                    Shape::Piston => Kind::Piston(base.starts_with("sticky")),
                    Shape::PistonHead => Kind::Head,
                    Shape::Door => Kind::Door,
                    Shape::Trapdoor => Kind::Trapdoor,
                    _ => match base {
                        "redstone_torch" => Kind::Torch(true),
                        "redstone_torch_off" => Kind::Torch(false),
                        "redstone_block" => Kind::Block,
                        "redstone_lamp" => Kind::Lamp(false),
                        "redstone_lamp_on" => Kind::Lamp(true),
                        "tnt" => Kind::Tnt,
                        _ => Kind::None,
                    },
                }
            })
            .collect::<Vec<_>>();
        let opaque: Vec<bool> = (0..reg.len()).map(|i| reg.is_opaque(i as BlockId)).collect();
        let support = reg.blocks.iter().enumerate().map(|(i, b)| opaque[i] || (b.solid && matches!(b.shape, Shape::Cube | Shape::Inset))).collect();
        let push = reg
            .blocks
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let base = reg.get(b.base).name.as_str();
                if i as BlockId == AIR || b.shape == Shape::Liquid {
                    Push::Free
                } else if b.hardness < 0.0 || ["obsidian", "bedrock", "chest", "furnace"].contains(&base) || base.ends_with("_bed") || kind[i] == Kind::Head || (b.shape == Shape::Piston && b.variant & 8 != 0) {
                    Push::Stop
                } else if !b.solid || matches!(b.shape, Shape::Cross | Shape::Torch | Shape::Layer | Shape::Door | Shape::Repeater) || kind[i] != Kind::None && !matches!(kind[i], Kind::Block | Kind::Lamp(_) | Kind::Tnt | Kind::Piston(_)) {
                    Push::Break
                } else {
                    Push::Move
                }
            })
            .collect();
        let join = (0..reg.len()).map(|i| voxel::mesh::redstone_join(&reg, i as BlockId)).collect();
        Redstone {
            kind,
            opaque,
            support,
            push,
            join,
            wire: id("redstone_wire"),
            torch: (id("redstone_torch_off"), id("redstone_torch")),
            lamp: (id("redstone_lamp"), id("redstone_lamp_on")),
            repeater: (id("repeater"), id("repeater_on")),
            head: id("piston_head"),
            reg,
            time: 0.0,
            queue: Vec::new(),
            queued: HashSet::new(),
            timers: BinaryHeap::new(),
            timed: HashSet::new(),
            door_power: HashMap::new(),
            flips: HashMap::new(),
            burnt: HashMap::new(),
            occupied: HashSet::new(),
            events: Vec::new(),
        }
    }

    pub fn kind(&self, id: BlockId) -> Kind {
        self.kind[id as usize]
    }

    fn at<G: Grid>(&self, g: &G, p: IVec3) -> BlockId {
        g.get(p).unwrap_or(AIR)
    }

    fn v(&self, id: BlockId) -> u8 {
        self.reg.get(id).variant
    }

    /// Wake everything that may depend on the block at `p`: itself, its neighbours and theirs
    /// (power reaches through one block).
    pub fn touch(&mut self, p: IVec3) {
        for a in std::iter::once(IVec3::ZERO).chain(SIDES) {
            for b in std::iter::once(IVec3::ZERO).chain(SIDES) {
                if a + b == IVec3::ZERO && a != IVec3::ZERO {
                    continue;
                }
                let q = p + a + b;
                if self.queued.insert(q) {
                    self.queue.push(q);
                }
            }
        }
    }

    fn schedule(&mut self, p: IVec3, delay: f64) {
        if self.timed.insert(p) {
            let due = ((self.time + delay) * 1e6) as i64;
            self.timers.push(Timer { due: std::cmp::Reverse(due), p: p.to_array() });
        }
    }

    fn put<G: Grid>(&mut self, g: &mut G, p: IVec3, id: BlockId) {
        if g.set(p, id) {
            self.touch(p);
        }
    }

    fn pop<G: Grid>(&mut self, g: &mut G, p: IVec3) {
        let id = self.at(g, p);
        g.remove(p);
        self.events.push(Event::Popped(p, id));
        self.touch(p);
    }

    // ---------------------------------------------------------------------------------------
    // Power.

    /// Power the source at `q` (not dust) gives to its neighbour at `q + d`.
    fn source_power<G: Grid>(&self, g: &G, q: IVec3, d: IVec3) -> u8 {
        let id = self.at(g, q);
        let v = self.v(id);
        let on = match self.kind(id) {
            Kind::Block => true,
            // Not into the block it stands on.
            Kind::Torch(true) => d != IVec3::NEG_Y,
            Kind::Lever | Kind::Button(_) => v & 4 != 0,
            Kind::Plate(_) => v & 1 != 0,
            Kind::Repeater(true) => d == h4(v),
            _ => false,
        };
        if on {
            15
        } else {
            0
        }
    }

    /// Whether the source at `q` strongly powers the block at `q + d`.
    fn strong_into<G: Grid>(&self, g: &G, q: IVec3, d: IVec3) -> bool {
        let id = self.at(g, q);
        let v = self.v(id);
        match self.kind(id) {
            Kind::Torch(true) => d == IVec3::Y,
            Kind::Lever | Kind::Button(_) => v & 4 != 0 && d == attach(v),
            Kind::Plate(_) => v & 1 != 0 && d == IVec3::NEG_Y,
            Kind::Repeater(true) => d == h4(v),
            _ => false,
        }
    }

    /// The sides dust at `p` points to: east, west, south, north bits (a lone dot or a single
    /// arm points both ways along its line, a cross every way).
    fn wire_dirs<G: Grid>(&self, g: &G, p: IVec3) -> u8 {
        let open_above = !self.opaque[self.at(g, p + IVec3::Y) as usize];
        let mut c = 0u8;
        for (k, d) in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z].into_iter().enumerate() {
            let n = self.at(g, p + d);
            let joins = match self.join[n as usize] {
                1 | 2 => true,
                3 => (self.v(n) & 1 == 0) == (k >= 2),
                _ => false,
            };
            if joins || (!self.opaque[n as usize] && self.kind(self.at(g, p + d - IVec3::Y)) == Kind::Wire) || (open_above && self.kind(self.at(g, p + d + IVec3::Y)) == Kind::Wire) {
                c |= 1 << k;
            }
        }
        match c.count_ones() {
            0 => 15,
            1 => c | if c & 3 != 0 { 3 } else { 12 },
            _ => c,
        }
    }

    /// Power dust at `q` gives to `q + d`: down into its block and along where it points.
    fn wire_into<G: Grid>(&self, g: &G, q: IVec3, d: IVec3) -> u8 {
        let id = self.at(g, q);
        if self.kind(id) != Kind::Wire || d == IVec3::Y {
            return 0;
        }
        let power = self.v(id);
        if d == IVec3::NEG_Y || power == 0 {
            return power;
        }
        let bit = match (d.x, d.z) {
            (1, _) => 1,
            (-1, _) => 2,
            (_, 1) => 4,
            _ => 8,
        };
        if self.wire_dirs(g, q) & bit != 0 {
            power
        } else {
            0
        }
    }

    fn strongly_powered<G: Grid>(&self, g: &G, p: IVec3) -> bool {
        SIDES.iter().any(|&d| self.strong_into(g, p + d, -d))
    }

    /// The power a full block holds: 15 when strongly powered, else what dust gives it.
    fn block_power<G: Grid>(&self, g: &G, p: IVec3) -> u8 {
        if !self.opaque[self.at(g, p) as usize] {
            return 0;
        }
        if self.strongly_powered(g, p) {
            return 15;
        }
        SIDES.iter().map(|&d| self.wire_into(g, p + d, -d)).max().unwrap_or(0)
    }

    /// What reaches `p` from its neighbour on side `d`.
    fn power_from<G: Grid>(&self, g: &G, p: IVec3, d: IVec3) -> u8 {
        let q = p + d;
        self.wire_into(g, q, -d).max(self.source_power(g, q, -d)).max(self.block_power(g, q))
    }

    /// The strongest power reaching a lamp, piston or door at `p`, leaving out side `skip`.
    fn input<G: Grid>(&self, g: &G, p: IVec3, skip: IVec3) -> u8 {
        SIDES.iter().filter(|&&d| d != skip).map(|&d| self.power_from(g, p, d)).max().unwrap_or(0)
    }

    /// Power entering dust at `p` from anything but other dust.
    fn wire_input<G: Grid>(&self, g: &G, p: IVec3) -> u8 {
        let mut best = 0;
        for d in SIDES {
            let q = p + d;
            best = best.max(self.source_power(g, q, -d));
            if self.opaque[self.at(g, q) as usize] && self.strongly_powered(g, q) {
                best = 15;
            }
        }
        best
    }

    /// Dust joined to the dust at `p`, on the same level or a step up or down.
    fn wire_links<G: Grid>(&self, g: &G, p: IVec3, out: &mut Vec<IVec3>) {
        let open_above = !self.opaque[self.at(g, p + IVec3::Y) as usize];
        for d in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
            let n = p + d;
            let id = self.at(g, n);
            if self.kind(id) == Kind::Wire {
                out.push(n);
                continue;
            }
            if !self.opaque[id as usize] && self.kind(self.at(g, n - IVec3::Y)) == Kind::Wire {
                out.push(n - IVec3::Y);
            }
            if open_above && self.kind(self.at(g, n + IVec3::Y)) == Kind::Wire {
                out.push(n + IVec3::Y);
            }
        }
    }

    fn supported<G: Grid>(&self, g: &G, p: IVec3, side: IVec3) -> bool {
        self.support[self.at(g, p + side) as usize]
    }

    // ---------------------------------------------------------------------------------------
    // Updates.

    /// Run timers due by now and settle everything woken since the last call. `changed` are
    /// blocks edited by anything else; `occupied` are blocks someone stands in (plates).
    pub fn update<G: Grid>(&mut self, g: &mut G, dt: f32, changed: &[IVec3], occupied: &[IVec3]) {
        self.time += dt as f64;
        for &p in changed {
            self.touch(p);
        }
        self.occupied.clear();
        for &p in occupied {
            self.occupied.insert(p);
            let id = self.at(g, p);
            if let Kind::Plate(_) = self.kind(id) {
                if self.v(id) & 1 == 0 {
                    let pressed = self.reg.variant(id, 1);
                    self.put(g, p, pressed);
                    self.events.push(Event::Click(p));
                    self.schedule(p, 10.0 * RT);
                }
            }
        }
        let now = (self.time * 1e6) as i64;
        let mut fired = 0;
        while let Some(t) = self.timers.peek() {
            if t.due.0 > now || fired >= 4096 {
                break;
            }
            let t = self.timers.pop().unwrap();
            let p = IVec3::from_array(t.p);
            self.timed.remove(&p);
            self.fire(g, p);
            fired += 1;
        }
        self.settle(g);
    }

    fn settle<G: Grid>(&mut self, g: &mut G) {
        for _ in 0..64 {
            if self.queue.is_empty() {
                break;
            }
            let batch = std::mem::take(&mut self.queue);
            self.queued.clear();
            let mut wires = Vec::new();
            for p in batch {
                let id = self.at(g, p);
                match self.kind(id) {
                    Kind::None => {}
                    Kind::Wire => wires.push(p),
                    _ => self.eval(g, p, id),
                }
            }
            if !wires.is_empty() {
                self.update_wires(g, wires);
            }
        }
    }

    /// Recompute the power of whole dust networks touching `seeds`.
    fn update_wires<G: Grid>(&mut self, g: &mut G, seeds: Vec<IVec3>) {
        let mut seen: HashSet<IVec3> = HashSet::new();
        let mut links = Vec::new();
        for s in seeds {
            if seen.contains(&s) || self.kind(self.at(g, s)) != Kind::Wire {
                continue;
            }
            if !self.supported(g, s, IVec3::NEG_Y) {
                self.pop(g, s);
                continue;
            }
            // One network.
            let mut net = vec![s];
            seen.insert(s);
            let mut k = 0;
            while k < net.len() && net.len() < 4096 {
                links.clear();
                self.wire_links(g, net[k], &mut links);
                for &n in &links {
                    if seen.insert(n) {
                        net.push(n);
                    }
                }
                k += 1;
            }
            let mut power: HashMap<IVec3, u8> = net.iter().map(|&p| (p, self.wire_input(g, p))).collect();
            let mut heap: BinaryHeap<(u8, [i32; 3])> = power.iter().filter(|(_, &v)| v > 1).map(|(p, &v)| (v, p.to_array())).collect();
            while let Some((v, p)) = heap.pop() {
                let p = IVec3::from_array(p);
                if power[&p] != v {
                    continue;
                }
                links.clear();
                self.wire_links(g, p, &mut links);
                for &n in &links {
                    if let Some(w) = power.get_mut(&n) {
                        if *w < v - 1 {
                            *w = v - 1;
                            if v > 2 {
                                heap.push((v - 1, n.to_array()));
                            }
                        }
                    }
                }
            }
            for (p, v) in power {
                let id = self.at(g, p);
                if self.v(id) != v {
                    let new = self.reg.variant(self.wire, v);
                    if g.set(p, new) {
                        // Neighbours, not the network again (it is settled).
                        for d in SIDES {
                            for e in std::iter::once(IVec3::ZERO).chain(SIDES) {
                                let q = p + d + e;
                                if self.kind(self.at(g, q)) != Kind::Wire && self.queued.insert(q) {
                                    self.queue.push(q);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn eval<G: Grid>(&mut self, g: &mut G, p: IVec3, id: BlockId) {
        let v = self.v(id);
        match self.kind(id) {
            Kind::Torch(lit) => {
                if !self.supported(g, p, IVec3::NEG_Y) {
                    return self.pop(g, p);
                }
                let want = self.block_power(g, p - IVec3::Y) == 0 && !self.burnt.get(&p).is_some_and(|&t| t > self.time);
                if want != lit {
                    self.schedule(p, RT);
                }
            }
            Kind::Repeater(on) => {
                if !self.supported(g, p, IVec3::NEG_Y) {
                    return self.pop(g, p);
                }
                let out = h4(v);
                let want = self.power_from(g, p, -out) > 0;
                if want != on {
                    self.schedule(p, RT * (((v >> 2) & 3) + 1) as f64);
                }
            }
            Kind::Lamp(lit) => {
                let powered = self.input(g, p, IVec3::ZERO) > 0;
                if powered && !lit {
                    let on = self.lamp.1;
                    self.put(g, p, on);
                } else if !powered && lit {
                    self.schedule(p, 2.0 * RT);
                }
            }
            Kind::Door | Kind::Trapdoor => {
                let door = self.kind(id) == Kind::Door;
                let lower = if door && v & 8 != 0 { p - IVec3::Y } else { p };
                let powered = self.input(g, lower, IVec3::ZERO) > 0 || (door && self.input(g, lower + IVec3::Y, IVec3::ZERO) > 0);
                let last = self.door_power.insert(lower, powered).unwrap_or(false);
                if last == powered {
                    return;
                }
                let lid = self.at(g, lower);
                if (self.v(lid) & 4 != 0) != powered {
                    let new = self.reg.variant(lid, self.v(lid) ^ 4);
                    self.put(g, lower, new);
                    if door {
                        let up = self.at(g, lower + IVec3::Y);
                        if self.kind(up) == Kind::Door {
                            let new = self.reg.variant(up, self.v(up) ^ 4);
                            self.put(g, lower + IVec3::Y, new);
                        }
                    }
                    self.events.push(Event::Click(lower));
                }
            }
            Kind::Piston(_) => {
                let d = dir6(v);
                if v & 8 != 0 && self.kind(self.at(g, p + d)) != Kind::Head {
                    // Its head was broken off.
                    let new = self.reg.variant(id, v & 7);
                    return self.put(g, p, new);
                }
                if (self.input(g, p, d) > 0) != (v & 8 != 0) {
                    self.schedule(p, 0.05);
                }
            }
            Kind::Head => {
                let d = dir6(v);
                let base = self.at(g, p - d);
                let ok = matches!(self.kind(base), Kind::Piston(_)) && self.v(base) & 8 != 0 && dir6_of(self.v(base)) == dir6_of(v);
                if !ok {
                    g.set(p, AIR);
                    self.touch(p);
                }
            }
            Kind::Tnt => {
                if self.input(g, p, IVec3::ZERO) > 0 {
                    g.set(p, AIR);
                    self.touch(p);
                    self.events.push(Event::Prime(p));
                }
            }
            Kind::Lever | Kind::Button(_) => {
                if !self.supported(g, p, attach(v)) {
                    self.pop(g, p);
                }
            }
            Kind::Plate(_) => {
                if !self.supported(g, p, IVec3::NEG_Y) {
                    self.pop(g, p);
                }
            }
            Kind::None | Kind::Wire | Kind::Block => {}
        }
    }

    fn fire<G: Grid>(&mut self, g: &mut G, p: IVec3) {
        let id = self.at(g, p);
        let v = self.v(id);
        match self.kind(id) {
            Kind::Torch(lit) => {
                let burnt = self.burnt.get(&p).is_some_and(|&t| t > self.time);
                let want = self.block_power(g, p - IVec3::Y) == 0 && !burnt;
                if want == lit {
                    return;
                }
                if want {
                    // More than eight flips a second burns a torch out for a few seconds.
                    let e = self.flips.entry(p).or_insert((self.time, 0));
                    if self.time - e.0 > 1.0 {
                        *e = (self.time, 0);
                    }
                    e.1 += 1;
                    if e.1 > 8 {
                        self.flips.remove(&p);
                        self.burnt.insert(p, self.time + 3.0);
                        self.events.push(Event::Burnout(p));
                        self.schedule(p, 3.0 + RT);
                        return;
                    }
                }
                let new = if want { self.torch.1 } else { self.torch.0 };
                self.put(g, p, new);
            }
            Kind::Repeater(on) => {
                let base = if on { self.repeater.0 } else { self.repeater.1 };
                let new = self.reg.variant(base, v);
                self.put(g, p, new);
            }
            Kind::Lamp(true) => {
                if self.input(g, p, IVec3::ZERO) == 0 {
                    let off = self.lamp.0;
                    self.put(g, p, off);
                }
            }
            Kind::Button(_) if v & 4 != 0 => {
                let new = self.reg.variant(id, v & !4);
                self.put(g, p, new);
                self.events.push(Event::Click(p));
            }
            Kind::Plate(_) if v & 1 != 0 => {
                if self.occupied.contains(&p) {
                    self.schedule(p, 10.0 * RT);
                } else {
                    let new = self.reg.variant(id, 0);
                    self.put(g, p, new);
                    self.events.push(Event::Click(p));
                }
            }
            Kind::Piston(sticky) => self.piston(g, p, id, sticky),
            _ => {}
        }
    }

    fn piston<G: Grid>(&mut self, g: &mut G, p: IVec3, id: BlockId, sticky: bool) {
        let v = self.v(id);
        let d = dir6(v);
        let out = v & 8 != 0;
        let powered = self.input(g, p, d) > 0;
        if powered && !out {
            // Up to twelve blocks move; the first free or fragile block ends the line.
            let mut line: Vec<(IVec3, BlockId)> = Vec::new();
            let mut q = p + d;
            loop {
                let Some(b) = g.get(q) else { return };
                match self.push[b as usize] {
                    Push::Free => break,
                    Push::Break => {
                        self.pop(g, q);
                        break;
                    }
                    Push::Stop => return,
                    Push::Move => {
                        line.push((q, b));
                        if line.len() > 12 {
                            return;
                        }
                        q += d;
                    }
                }
            }
            for &(q, b) in line.iter().rev() {
                self.put(g, q + d, b);
            }
            let head = self.reg.variant(self.head, (v & 7) | if sticky { 8 } else { 0 });
            self.put(g, p + d, head);
            let new = self.reg.variant(id, v | 8);
            self.put(g, p, new);
            self.events.push(Event::Piston(p, true));
            self.events.push(Event::Moved { from: p + d, dir: d, count: line.len() as i32 + 1 });
        } else if !powered && out {
            let new = self.reg.variant(id, v & 7);
            self.put(g, p, new);
            let h = p + d;
            if self.kind(self.at(g, h)) == Kind::Head {
                self.put(g, h, AIR);
            }
            if sticky {
                let far = p + d * 2;
                let b = self.at(g, far);
                if self.push[b as usize] == Push::Move {
                    self.put(g, h, b);
                    self.put(g, far, AIR);
                    self.events.push(Event::Moved { from: far, dir: -d, count: 1 });
                }
            }
            self.events.push(Event::Piston(p, false));
        }
    }

    /// Right click: flip a lever, press a button, change a repeater's delay. True when it did.
    pub fn interact<G: Grid>(&mut self, g: &mut G, p: IVec3) -> bool {
        let id = self.at(g, p);
        let v = self.v(id);
        match self.kind(id) {
            Kind::Lever => {
                let new = self.reg.variant(id, v ^ 4);
                self.put(g, p, new);
                self.events.push(Event::Click(p));
            }
            Kind::Button(wood) => {
                if v & 4 == 0 {
                    let new = self.reg.variant(id, v | 4);
                    self.put(g, p, new);
                    self.schedule(p, if wood { 15.0 } else { 10.0 } * RT);
                    self.events.push(Event::Click(p));
                }
            }
            Kind::Repeater(_) => {
                let delay = ((v >> 2) + 1) & 3;
                let new = self.reg.variant(id, (v & 3) | delay << 2);
                self.put(g, p, new);
                self.events.push(Event::Click(p));
            }
            _ => return false,
        }
        true
    }
}

/// The side a lever or button hangs on: the wall it faces, or the floor.
fn attach(v: u8) -> IVec3 {
    if v & 8 != 0 {
        h4(v)
    } else {
        IVec3::NEG_Y
    }
}

impl Grid for crate::world::World {
    fn get(&self, p: IVec3) -> Option<BlockId> {
        self.block(p)
    }
    fn set(&mut self, p: IVec3, id: BlockId) -> bool {
        self.set_block(p, id)
    }
    fn remove(&mut self, p: IVec3) {
        self.break_block(p);
    }
}

/// Lit TNT: falls like sand and goes off when the fuse runs out.
pub struct PrimedTnt {
    pub body: voxel::physics::Body,
    pub fuse: f32,
}

impl crate::Game {
    /// Settle redstone for this frame and act on what it did.
    pub(crate) fn update_redstone(&mut self, dt: f32) {
        use crate::audio::Sfx;
        use glam::Vec3;
        // Plates: the player and mobs press any; items only wooden ones.
        let feet = |p: Vec3| (p + Vec3::Y * 0.05).floor().as_ivec3();
        let mut occupied = vec![feet(self.body.pos)];
        let me = self.body.pos;
        for m in &self.mobs.list {
            if m.alive() && (m.body.pos - me).length_squared() < 64.0 * 64.0 {
                occupied.push(feet(m.body.pos));
            }
        }
        for d in &self.surv.drops {
            let p = feet(d.body.pos);
            if self.redstone.kind(self.world.block_or_air(p)) == Kind::Plate(true) {
                occupied.push(p);
            }
        }
        let changed = self.world.changed.clone();
        self.redstone.update(&mut self.world, dt, &changed, &occupied);
        for e in std::mem::take(&mut self.redstone.events) {
            let near = |p: IVec3| (p.as_vec3() - me).length() < 24.0;
            match e {
                Event::Popped(p, id) => {
                    if self.survival {
                        self.blast_drops(p, id, 1.0);
                    }
                    let (layer, tint) = self.block_look(id);
                    self.particles.block_bits(p, layer, tint, 6, 1.0);
                }
                Event::Prime(p) => self.prime_tnt(p, 4.0),
                Event::Click(p) if near(p) => self.sfx(Sfx::Click),
                Event::Piston(p, out) if near(p) => self.sfx(Sfx::Piston(out)),
                Event::Burnout(p) if near(p) => self.sfx(Sfx::Fizz),
                Event::Moved { from, dir, count } => {
                    // Whoever stands where the blocks went rides along.
                    let step = dir.as_vec3();
                    for k in 0..count {
                        let b = from + dir * (k + 1);
                        if self.body.intersects_block(b) {
                            self.body.pos += step;
                            if dir.y > 0 {
                                self.body.vel.y = self.body.vel.y.max(0.0);
                            }
                        }
                        for m in self.mobs.list.iter_mut() {
                            if m.body.intersects_block(b) {
                                m.body.pos += step;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        self.update_tnt(dt);
    }

    /// Test scene (--redstone): a few classic circuits on a stone floor in front of the player.
    pub(crate) fn build_redstone(&mut self) {
        let p = self.body.pos.floor().as_ivec3();
        let floor = self.reg.id_or_air("stone");
        for x in -8..=8 {
            for z in 2..=15 {
                self.world.set_block(IVec3::new(p.x + x, p.y - 1, p.z - z), floor);
                for y in 0..5 {
                    self.world.set_block(IVec3::new(p.x + x, p.y + y, p.z - z), AIR);
                }
            }
        }
        let reg = self.reg.clone();
        let put = |w: &mut crate::world::World, x: i32, y: i32, z: i32, name: &str, v: u8| {
            let id = reg.variant(reg.id_or_air(name), v);
            w.set_block(IVec3::new(p.x + x, p.y + y, p.z - z), id);
        };
        let w = &mut self.world;
        // A lever turned on lights a lamp down a line of dust; one turned off leaves it dark.
        put(w, -7, 0, 4, "lever", 4);
        for x in -6..=-3 {
            put(w, x, 0, 4, "redstone_wire", 0);
        }
        put(w, -2, 0, 4, "redstone_lamp", 0);
        put(w, 1, 0, 4, "lever", 0);
        for x in 2..=4 {
            put(w, x, 0, 4, "redstone_wire", 0);
        }
        put(w, 5, 0, 4, "redstone_lamp", 0);
        // Power fades one level a block from a block of redstone.
        put(w, -7, 0, 7, "redstone_block", 0);
        for x in -6..=7 {
            put(w, x, 0, 7, "redstone_wire", 0);
        }
        // A torch, two repeaters (they boost back to 15) and a sticky piston pushing dirt.
        put(w, -7, 0, 10, "redstone_torch", 0);
        put(w, -6, 0, 10, "redstone_wire", 0);
        put(w, -5, 0, 10, "repeater", 1 | 1 << 2);
        put(w, -4, 0, 10, "redstone_wire", 0);
        put(w, -3, 0, 10, "redstone_wire", 0);
        put(w, -2, 0, 10, "repeater", 1 | 3 << 2);
        put(w, -1, 0, 10, "redstone_wire", 0);
        put(w, 0, 0, 10, "sticky_piston", 1);
        put(w, 1, 0, 10, "dirt", 0);
        // A piston pointing up, under planks, next to a block of redstone.
        put(w, 4, 0, 10, "piston", 4);
        put(w, 4, 1, 10, "oak_planks", 0);
        put(w, 5, 0, 10, "redstone_block", 0);
        // Dust climbing steps, an inverter (torch on a powered block) and a door opened by a lever.
        put(w, -7, 0, 13, "redstone_torch", 0);
        put(w, -6, 0, 13, "redstone_wire", 0);
        put(w, -5, 0, 13, "stone", 0);
        put(w, -5, 1, 13, "redstone_wire", 0);
        put(w, -4, 0, 13, "stone", 0);
        put(w, -4, 1, 13, "stone", 0);
        put(w, -4, 2, 13, "redstone_wire", 0);
        put(w, -1, 0, 13, "stone", 0);
        put(w, -1, 1, 13, "redstone_torch", 0);
        put(w, -2, 0, 13, "lever", 4 | 8 | 1);
        put(w, 1, 0, 13, "stone", 0);
        put(w, 1, 1, 13, "redstone_torch", 0);
        put(w, 3, 0, 13, "lever", 4);
        put(w, 4, 0, 13, "oak_door", 2);
        put(w, 4, 1, 13, "oak_door", 2 | 8);
        put(w, 6, 0, 13, "redstone_lamp", 0);
        put(w, 6, 1, 13, "stone_button", 0);
        put(w, 6, 0, 12, "stone_pressure_plate", 0);
        put(w, 7, 0, 13, "tnt", 0);
        if self.args.screenshot.is_some() {
            // Step back and up to see it all.
            self.body.pos += glam::Vec3::new(0.0, 6.0, 2.0);
            self.body.flying = true;
        }
    }

    /// Light TNT at a block (taking it away) with a fuse in seconds.
    pub(crate) fn prime_tnt(&mut self, p: IVec3, fuse: f32) {
        use glam::Vec3;
        if self.redstone.kind(self.world.block_or_air(p)) == Kind::Tnt {
            self.world.set_block(p, AIR);
        }
        let mut body = voxel::physics::Body::sized(p.as_vec3() + Vec3::new(0.5, 0.0, 0.5), 0.98, 0.98);
        let a = (p.x * 31 + p.z * 17) as f32;
        body.vel = Vec3::new(a.cos() * 0.4, 2.0, a.sin() * 0.4);
        self.tnt.push(PrimedTnt { body, fuse });
        if (p.as_vec3() - self.body.pos).length() < 24.0 {
            self.sfx(crate::audio::Sfx::Fizz);
        }
    }

    fn update_tnt(&mut self, dt: f32) {
        if self.tnt.is_empty() {
            return;
        }
        let still = voxel::physics::Input::default();
        let mut boom = Vec::new();
        for t in self.tnt.iter_mut() {
            t.fuse -= dt;
            if self.world.block(t.body.pos.floor().as_ivec3()).is_some() {
                t.body.step(&self.world, &still, dt);
            }
            if t.fuse <= 0.0 {
                boom.push(t.body.pos + glam::Vec3::Y * 0.49);
            }
        }
        self.tnt.retain(|t| t.fuse > 0.0);
        for at in boom {
            self.explode(at, 4.0);
        }
    }

    /// Lit TNT for the item renderer: a block that flashes white.
    pub(crate) fn tnt_instances(&self, cam_block: IVec3) -> Vec<crate::item_render::GpuItem> {
        let tnt = self.reg.id_or_air("tnt");
        let faces = self.reg.get(tnt).faces;
        self.tnt
            .iter()
            .map(|t| {
                let p = t.body.pos + glam::Vec3::Y * 0.49 - cam_block.as_vec3();
                let l = self.world.light((t.body.pos + glam::Vec3::Y * 0.5).floor().as_ivec3());
                let flash = if (t.fuse * 4.0) as i32 % 2 == 0 { 0.8 } else { 0.0 };
                // Swells a little just before it goes.
                let size = 0.98 + (0.4f32 - t.fuse).max(0.0) * 0.2;
                crate::item_render::GpuItem {
                    pos: [p.x, p.y, p.z, size],
                    info: [0.0, 0.0, (l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0],
                    layers: [faces[0] as f32, faces[2] as f32, faces[3] as f32, flash],
                    rot: [0.0; 4],
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Map(HashMap<IVec3, BlockId>);

    impl Grid for Map {
        fn get(&self, p: IVec3) -> Option<BlockId> {
            Some(self.0.get(&p).copied().unwrap_or(AIR))
        }
        fn set(&mut self, p: IVec3, id: BlockId) -> bool {
            if id == AIR {
                self.0.remove(&p);
            } else {
                self.0.insert(p, id);
            }
            true
        }
    }

    fn setup() -> (Redstone, Map, Arc<BlockRegistry>) {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../packs/vanilla/blocks.ron")).unwrap();
        let reg = Arc::new(BlockRegistry::from_ron(&text).unwrap());
        let mut m = Map(HashMap::new());
        let stone = reg.id("stone").unwrap();
        for x in -20..=20 {
            for z in -20..=20 {
                m.0.insert(IVec3::new(x, 0, z), stone);
            }
        }
        (Redstone::new(reg.clone()), m, reg)
    }

    /// Place blocks and let the redstone run for `secs`.
    fn run(rs: &mut Redstone, m: &mut Map, puts: &[(IVec3, BlockId)], secs: f32) {
        let mut changed = Vec::new();
        for &(p, id) in puts {
            m.set(p, id);
            changed.push(p);
        }
        rs.update(m, 0.0, &changed, &[]);
        let steps = (secs / 0.05).ceil() as usize;
        for _ in 0..steps {
            rs.update(m, 0.05, &[], &[]);
        }
    }

    fn power(m: &Map, reg: &BlockRegistry, p: IVec3) -> u8 {
        reg.get(m.get(p).unwrap()).variant
    }

    #[test]
    fn dust_fades_one_level_a_block() {
        let (mut rs, mut m, reg) = setup();
        let wire = reg.id("redstone_wire").unwrap();
        let mut puts: Vec<(IVec3, BlockId)> = (0..18).map(|x| (IVec3::new(x, 1, 0), wire)).collect();
        puts.push((IVec3::new(-1, 1, 0), reg.id("redstone_block").unwrap()));
        run(&mut rs, &mut m, &puts, 0.1);
        assert_eq!(power(&m, &reg, IVec3::new(0, 1, 0)), 15);
        assert_eq!(power(&m, &reg, IVec3::new(5, 1, 0)), 10);
        assert_eq!(power(&m, &reg, IVec3::new(14, 1, 0)), 1);
        assert_eq!(power(&m, &reg, IVec3::new(16, 1, 0)), 0);
        // Taking the source away drains it all.
        run(&mut rs, &mut m, &[(IVec3::new(-1, 1, 0), AIR)], 0.1);
        assert!((0..18).all(|x| power(&m, &reg, IVec3::new(x, 1, 0)) == 0));
    }

    #[test]
    fn lever_lights_a_lamp_through_dust() {
        let (mut rs, mut m, reg) = setup();
        let wire = reg.id("redstone_wire").unwrap();
        let lever = reg.id("lever").unwrap();
        let lamp = reg.id("redstone_lamp").unwrap();
        let mut puts: Vec<(IVec3, BlockId)> = (1..5).map(|x| (IVec3::new(x, 1, 0), wire)).collect();
        puts.push((IVec3::new(0, 1, 0), lever));
        puts.push((IVec3::new(5, 1, 0), lamp));
        run(&mut rs, &mut m, &puts, 0.2);
        assert_eq!(m.get(IVec3::new(5, 1, 0)), Some(lamp));
        assert!(rs.interact(&mut m, IVec3::new(0, 1, 0)));
        run(&mut rs, &mut m, &[], 0.1);
        assert_eq!(m.get(IVec3::new(5, 1, 0)), reg.id("redstone_lamp_on"));
        rs.interact(&mut m, IVec3::new(0, 1, 0));
        run(&mut rs, &mut m, &[], 0.5);
        assert_eq!(m.get(IVec3::new(5, 1, 0)), Some(lamp));
    }

    #[test]
    fn torch_inverts_the_block_under_it() {
        let (mut rs, mut m, reg) = setup();
        let stone = reg.id("stone").unwrap();
        let torch = reg.id("redstone_torch").unwrap();
        let lever = reg.id("lever").unwrap();
        // A torch on a stone block with a lever on the stone's side (wall lever facing west).
        let lever_wall = reg.variant(lever, 3 | 8);
        run(&mut rs, &mut m, &[(IVec3::new(0, 1, 0), stone), (IVec3::new(0, 2, 0), torch), (IVec3::new(1, 1, 0), lever_wall)], 0.3);
        assert_eq!(m.get(IVec3::new(0, 2, 0)), Some(torch));
        rs.interact(&mut m, IVec3::new(1, 1, 0));
        run(&mut rs, &mut m, &[], 0.3);
        assert_eq!(m.get(IVec3::new(0, 2, 0)), reg.id("redstone_torch_off"));
    }

    #[test]
    fn repeater_waits_its_delay() {
        let (mut rs, mut m, reg) = setup();
        let wire = reg.id("redstone_wire").unwrap();
        // Facing east (1) with four ticks of delay.
        let rep = reg.variant(reg.id("repeater").unwrap(), 1 | 3 << 2);
        let block = reg.id("redstone_block").unwrap();
        run(&mut rs, &mut m, &[(IVec3::new(1, 1, 0), rep), (IVec3::new(2, 1, 0), wire), (IVec3::new(0, 1, 0), block)], 0.25);
        assert_eq!(power(&m, &reg, IVec3::new(2, 1, 0)), 0);
        run(&mut rs, &mut m, &[], 0.25);
        assert_eq!(power(&m, &reg, IVec3::new(2, 1, 0)), 15);
    }

    #[test]
    fn piston_pushes_and_sticky_pulls_back() {
        let (mut rs, mut m, reg) = setup();
        let dirt = reg.id("dirt").unwrap();
        // Sticky piston facing east (1), a lever behind it on the floor.
        let piston = reg.variant(reg.id("sticky_piston").unwrap(), 1);
        let lever = reg.id("lever").unwrap();
        run(&mut rs, &mut m, &[(IVec3::new(0, 1, 0), piston), (IVec3::new(1, 1, 0), dirt), (IVec3::new(2, 1, 0), dirt), (IVec3::new(-1, 1, 0), lever)], 0.1);
        rs.interact(&mut m, IVec3::new(-1, 1, 0));
        run(&mut rs, &mut m, &[], 0.2);
        assert_eq!(rs.kind(m.get(IVec3::new(1, 1, 0)).unwrap()), Kind::Head);
        assert_eq!(m.get(IVec3::new(2, 1, 0)), Some(dirt));
        assert_eq!(m.get(IVec3::new(3, 1, 0)), Some(dirt));
        rs.interact(&mut m, IVec3::new(-1, 1, 0));
        run(&mut rs, &mut m, &[], 0.2);
        assert_eq!(m.get(IVec3::new(1, 1, 0)), Some(dirt));
        assert_eq!(m.get(IVec3::new(2, 1, 0)), Some(AIR));
        assert_eq!(m.get(IVec3::new(3, 1, 0)), Some(dirt));
    }
}
