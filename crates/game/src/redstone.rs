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
    /// Powered or not (its strength is in the variant).
    Comparator(bool),
    Observer,
    /// Droppers (false) and dispensers (true): act once each time power arrives.
    Dispenser(bool),
    /// Copper bulbs (lit or not): switch on or off each time power arrives.
    Bulb(bool),
    /// Rails: 0 plain, 1 powered, 2 detector, 3 activator.
    Rail(u8),
    /// A note block: plays once each time power arrives.
    Note,
    /// Runs its command each time power arrives.
    Command,
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
    /// A dropper or dispenser fires one item out of its front.
    Dispense(IVec3),
    /// A note block got power: it plays its note.
    Note(IVec3),
    /// A command block got power: it runs its command.
    Command(IVec3),
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
    comparator: (BlockId, BlockId),
    /// Container fullness last reported (for comparators).
    levels: HashMap<IVec3, u8>,
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
    /// Plates something stands on this frame (and detector rails a cart is on).
    occupied: HashSet<IVec3>,
    /// Copper bulbs remember whether they were powered (they switch on the rising edge), and
    /// each one's lit or unlit twin.
    bulb_power: HashMap<IVec3, bool>,
    bulb_twin: HashMap<BlockId, BlockId>,
    pub events: Vec<Event>,
    /// Note blocks that are powered now (they play again only after the power goes away).
    notes_on: std::collections::HashSet<IVec3>,
    /// Command blocks that are powered now.
    commands_on: HashSet<IVec3>,
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
                    Shape::Comparator => Kind::Comparator(base.ends_with("_on")),
                    Shape::Oriented if base == "observer" => Kind::Observer,
                    Shape::Oriented if base == "dropper" => Kind::Dispenser(false),
                    Shape::Oriented if base == "dispenser" => Kind::Dispenser(true),
                    // The crafter fires like a dropper; what it does then is up to the game.
                    Shape::Oriented if base == "crafter" => Kind::Dispenser(false),
                    Shape::Rail => Kind::Rail(match base {
                        "powered_rail" => 1,
                        "detector_rail" => 2,
                        "activator_rail" => 3,
                        _ => 0,
                    }),
                    _ if base.ends_with("copper_bulb") => Kind::Bulb(false),
                    _ if base.ends_with("copper_bulb_lit") => Kind::Bulb(true),
                    _ => match base {
                        "redstone_torch" => Kind::Torch(true),
                        "redstone_torch_off" => Kind::Torch(false),
                        "redstone_block" | "sculk_sensor_active" | "calibrated_sculk_sensor_active" => Kind::Block,
                        "redstone_lamp" => Kind::Lamp(false),
                        "redstone_lamp_on" => Kind::Lamp(true),
                        "tnt" => Kind::Tnt,
                        "note_block" => Kind::Note,
                        "command_block" => Kind::Command,
                        // A season sensor in its season (mods/seasons.rs) powers like a redstone block.
                        n if n.starts_with("season_sensor_") && n.ends_with("_on") => Kind::Block,
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
                } else if b.hardness < 0.0 || ["obsidian", "bedrock", "chest", "furnace", "hopper", "dropper", "dispenser", "crafter", "command_block"].contains(&base) || base.ends_with("_bed") || kind[i] == Kind::Head || (b.shape == Shape::Piston && b.variant & 8 != 0) {
                    Push::Stop
                } else if !b.solid || matches!(b.shape, Shape::Cross | Shape::Torch | Shape::Layer | Shape::Door | Shape::Repeater) || kind[i] != Kind::None && !matches!(kind[i], Kind::Block | Kind::Lamp(_) | Kind::Tnt | Kind::Piston(_) | Kind::Observer | Kind::Note) {
                    Push::Break
                } else {
                    Push::Move
                }
            })
            .collect();
        let join = (0..reg.len()).map(|i| voxel::mesh::redstone_join(&reg, i as BlockId)).collect();
        let mut bulb_twin = HashMap::new();
        for (i, b) in reg.blocks.iter().enumerate() {
            if b.name.ends_with("copper_bulb") {
                if let Some(lit) = reg.id(&format!("{}_lit", b.name)) {
                    bulb_twin.insert(i as BlockId, lit);
                    bulb_twin.insert(lit, i as BlockId);
                }
            }
        }
        Redstone {
            kind,
            opaque,
            support,
            push,
            join,
            wire: id("redstone_wire"),
            torch: (id("redstone_torch_off"), id("redstone_torch")),
            comparator: (id("comparator"), id("comparator_on")),
            levels: HashMap::new(),
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
            bulb_power: HashMap::new(),
            bulb_twin,
            events: Vec::new(),
            notes_on: Default::default(),
            commands_on: Default::default(),
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
            self.notice(g, p);
        }
    }

    /// Observers looking at `p` see it change: each sends a short pulse out of its back.
    fn notice<G: Grid>(&mut self, g: &G, p: IVec3) {
        for e in SIDES {
            let q = p + e;
            let id = self.at(g, q);
            if self.kind(id) == Kind::Observer && dir6(self.v(id)) == -e && self.v(id) & 8 == 0 {
                self.schedule(q, RT);
            }
        }
    }

    /// Report how full each container is; comparators reading one that changed wake up.
    pub fn set_levels(&mut self, levels: HashMap<IVec3, u8>) {
        let changed: Vec<IVec3> = levels.iter().filter(|(p, l)| self.levels.get(p) != Some(l)).map(|(p, _)| *p).chain(self.levels.keys().filter(|p| !levels.contains_key(p)).copied()).collect();
        self.levels = levels;
        for p in changed {
            self.touch(p);
        }
    }

    /// Whether the block at `p` is full enough to hold things up (items rest on it).
    pub fn support_at<G: Grid>(&self, g: &G, p: IVec3) -> bool {
        self.support[self.at(g, p) as usize]
    }

    /// Whether anything powers the block at `p` (hoppers lock, for one).
    pub fn powered<G: Grid>(&self, g: &G, p: IVec3) -> bool {
        self.input(g, p, IVec3::ZERO) > 0
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
            Kind::Torch(true) => d != torch_attach(v),
            Kind::Lever | Kind::Button(_) => v & 4 != 0,
            Kind::Plate(_) => v & 1 != 0,
            Kind::Repeater(true) => d == h4(v),
            Kind::Comparator(true) => return if d == h4(v) { v >> 3 } else { 0 },
            Kind::Observer => v & 8 != 0 && d == -dir6(v),
            Kind::Rail(2) => v >= 10,
            _ => false,
        };
        if on {
            15
        } else {
            0
        }
    }

    /// How strongly the source at `q` powers the block at `q + d` (0 when it doesn't).
    fn strong_into<G: Grid>(&self, g: &G, q: IVec3, d: IVec3) -> u8 {
        let id = self.at(g, q);
        let v = self.v(id);
        let on = match self.kind(id) {
            Kind::Torch(true) => d == IVec3::Y,
            Kind::Lever | Kind::Button(_) => v & 4 != 0 && d == attach(v),
            Kind::Plate(_) => v & 1 != 0 && d == IVec3::NEG_Y,
            Kind::Repeater(true) => d == h4(v),
            Kind::Comparator(true) => return if d == h4(v) { v >> 3 } else { 0 },
            Kind::Observer => v & 8 != 0 && d == -dir6(v),
            Kind::Rail(2) => v >= 10 && d == IVec3::NEG_Y,
            _ => false,
        };
        if on {
            15
        } else {
            0
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

    fn strong_power<G: Grid>(&self, g: &G, p: IVec3) -> u8 {
        SIDES.iter().map(|&d| self.strong_into(g, p + d, -d)).max().unwrap_or(0)
    }

    /// The power a full block holds: 15 when strongly powered, else what dust gives it.
    fn block_power<G: Grid>(&self, g: &G, p: IVec3) -> u8 {
        if !self.opaque[self.at(g, p) as usize] {
            return 0;
        }
        let strong = self.strong_power(g, p);
        SIDES.iter().map(|&d| self.wire_into(g, p + d, -d)).max().unwrap_or(0).max(strong)
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
            if self.opaque[self.at(g, q) as usize] {
                best = best.max(self.strong_power(g, q));
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
            self.notice(g, p);
            // A bulb just placed lights if it is placed into power.
            if matches!(self.kind(self.at(g, p)), Kind::Bulb(_)) {
                self.bulb_power.entry(p).or_insert(false);
            }
        }
        self.occupied.clear();
        for &p in occupied {
            self.occupied.insert(p);
            let id = self.at(g, p);
            if self.kind(id) == Kind::Rail(2) && self.v(id) < 10 {
                let on = self.reg.variant(id, self.v(id) + 10);
                self.put(g, p, on);
                self.schedule(p, 10.0 * RT);
            }
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
                let a = torch_attach(v);
                if !self.supported(g, p, a) {
                    return self.pop(g, p);
                }
                let want = self.block_power(g, p + a) == 0 && !self.burnt.get(&p).is_some_and(|&t| t > self.time);
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
            Kind::Note => {
                // Plays on the rising edge only.
                if self.input(g, p, IVec3::ZERO) > 0 {
                    if self.notes_on.insert(p) {
                        self.events.push(Event::Note(p));
                    }
                } else {
                    self.notes_on.remove(&p);
                }
            }
            Kind::Command => {
                // Runs on the rising edge only, like a note block plays.
                if self.input(g, p, IVec3::ZERO) > 0 {
                    if self.commands_on.insert(p) {
                        self.events.push(Event::Command(p));
                    }
                } else {
                    self.commands_on.remove(&p);
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
            Kind::Comparator(_) => {
                if !self.supported(g, p, IVec3::NEG_Y) {
                    return self.pop(g, p);
                }
                if self.comparator_output(g, p, v) != v >> 3 {
                    self.schedule(p, RT);
                }
            }
            Kind::Dispenser(_) => {
                let powered = self.input(g, p, IVec3::ZERO) > 0;
                if powered && v & 8 == 0 {
                    let new = self.reg.variant(id, v | 8);
                    g.set(p, new);
                    self.schedule(p, 2.0 * RT);
                } else if !powered && v & 8 != 0 {
                    let new = self.reg.variant(id, v & 7);
                    g.set(p, new);
                }
            }
            Kind::Bulb(_) => {
                let powered = self.input(g, p, IVec3::ZERO) > 0;
                let was = *self.bulb_power.entry(p).or_insert(powered);
                if powered != was {
                    self.bulb_power.insert(p, powered);
                    if powered {
                        if let Some(&twin) = self.bulb_twin.get(&id) {
                            self.put(g, p, twin);
                            self.events.push(Event::Click(p));
                        }
                    }
                }
            }
            Kind::Rail(k) => {
                if !self.supported(g, p, IVec3::NEG_Y) {
                    return self.pop(g, p);
                }
                let on = if k == 1 || k == 3 { self.rail_powered(g, p, id) } else { v >= 10 };
                if on != (v >= 10) && k != 2 {
                    let new = self.reg.variant(id, if on { v + 10 } else { v - 10 });
                    self.put(g, p, new);
                    // The rails down the line feel it too.
                    for q in self.rail_links(g, p, new) {
                        if self.queued.insert(q) {
                            self.queue.push(q);
                        }
                    }
                }
            }
            Kind::None | Kind::Wire | Kind::Block | Kind::Observer => {}
        }
    }

    /// Rails this rail at `p` joins: one block along each end, a block up for the end it climbs
    /// toward, or a block down when the rail there climbs up to this one.
    pub fn rail_links<G: Grid>(&self, g: &G, p: IVec3, id: BlockId) -> Vec<IVec3> {
        let v = self.v(id);
        let s = voxel::shapes::rail_shape(v);
        let mut out = Vec::new();
        for (k, e) in voxel::shapes::RAIL_ENDS[s].iter().enumerate() {
            let q = p + IVec3::new(e[0], 0, e[1]);
            let up = k == 0 && voxel::shapes::rail_rises(v).is_some();
            for c in if up { [q + IVec3::Y, q] } else { [q, q - IVec3::Y] } {
                if matches!(self.kind(self.at(g, c)), Kind::Rail(_)) {
                    out.push(c);
                    break;
                }
            }
        }
        out
    }

    /// A powered or activator rail is on when power reaches it or a rail of the same kind up
    /// to eight blocks down the line.
    fn rail_powered<G: Grid>(&self, g: &G, p: IVec3, id: BlockId) -> bool {
        let base = self.reg.get(id).base;
        let mut seen = vec![p];
        let mut k = 0;
        while k < seen.len() && seen.len() <= 17 {
            let q = seen[k];
            if self.input(g, q, IVec3::ZERO) > 0 {
                return true;
            }
            let qid = self.at(g, q);
            for n in self.rail_links(g, q, qid) {
                if self.reg.get(self.at(g, n)).base == base && !seen.contains(&n) && (n - p).abs().max_element() <= 8 {
                    seen.push(n);
                }
            }
            k += 1;
        }
        false
    }

    /// What a comparator at `p` puts out: the back signal (or how full a container behind it
    /// is), compared with or reduced by the strongest side.
    fn comparator_output<G: Grid>(&self, g: &G, p: IVec3, v: u8) -> u8 {
        let out = h4(v);
        let back = p - out;
        let rear = match self.levels.get(&back) {
            Some(&l) => l,
            None if self.kind(self.at(g, back)) == Kind::Bulb(true) => 15,
            None => self.power_from(g, p, -out),
        };
        let left = IVec3::new(out.z, 0, -out.x);
        let side = [left, -left].iter().map(|&d| self.wire_into(g, p + d, -d).max(self.source_power(g, p + d, -d))).max().unwrap_or(0);
        if v & 4 != 0 {
            rear.saturating_sub(side)
        } else if rear >= side {
            rear
        } else {
            0
        }
    }

    fn fire<G: Grid>(&mut self, g: &mut G, p: IVec3) {
        let id = self.at(g, p);
        let v = self.v(id);
        match self.kind(id) {
            Kind::Torch(lit) => {
                let burnt = self.burnt.get(&p).is_some_and(|&t| t > self.time);
                let want = self.block_power(g, p + torch_attach(v)) == 0 && !burnt;
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
                let new = self.reg.variant(if want { self.torch.1 } else { self.torch.0 }, v);
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
            Kind::Rail(2) if v >= 10 => {
                if self.occupied.contains(&p) {
                    self.schedule(p, 10.0 * RT);
                } else {
                    let new = self.reg.variant(id, v - 10);
                    self.put(g, p, new);
                }
            }
            Kind::Piston(sticky) => self.piston(g, p, id, sticky),
            Kind::Comparator(_) => {
                let level = self.comparator_output(g, p, v);
                let base = if level > 0 { self.comparator.1 } else { self.comparator.0 };
                let new = self.reg.variant(base, (v & 7) | level << 3);
                self.put(g, p, new);
            }
            Kind::Observer => {
                let new = self.reg.variant(id, v ^ 8);
                self.put(g, p, new);
                if v & 8 == 0 {
                    // The pulse lasts one tick.
                    self.schedule(p, RT);
                }
            }
            Kind::Dispenser(_) => {
                if v & 8 != 0 {
                    self.events.push(Event::Dispense(p));
                }
            }
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
            Kind::Comparator(_) => {
                let new = self.reg.variant(id, v ^ 4);
                self.put(g, p, new);
                self.events.push(Event::Click(p));
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

/// The block a torch stands on, or the wall it hangs on.
fn torch_attach(v: u8) -> IVec3 {
    if v == 0 {
        IVec3::NEG_Y
    } else {
        h4(v - 1)
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
        // Detector rails feel carts, not feet.
        occupied.retain(|&p| self.redstone.kind(self.world.block_or_air(p)) != Kind::Rail(2));
        occupied.extend(self.cart_cells());
        let levels = self.container_levels();
        self.redstone.set_levels(levels);
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
                Event::Dispense(p) => self.dispense(p),
                Event::Note(p) => self.play_note_block(p),
                Event::Command(p) => self.cmdblock_run(p),
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
        self.update_hoppers(dt);
    }

    /// How full every container is, 0..15, the way comparators read it.
    fn container_levels(&self) -> HashMap<IVec3, u8> {
        use crate::survival::Container;
        self.surv
            .containers
            .iter()
            .map(|(p, c)| {
                let inv = match c {
                    Container::Chest(i) => i,
                    Container::Furnace(f) => &f.inv,
                };
                let mut fill = 0.0;
                let mut any = false;
                for st in inv.slots.iter().flatten() {
                    any = true;
                    fill += st.count as f32 / self.items.get(st.item).max_stack.max(1) as f32;
                }
                let level = if any { (1.0 + fill / inv.slots.len() as f32 * 14.0) as u8 } else { 0 };
                (*p, level.min(15))
            })
            .collect()
    }

    /// Hoppers move one item every 0.4 s: out of the spout into the container there, and in
    /// from the container above or items lying on top. Power locks them.
    fn update_hoppers(&mut self, dt: f32) {
        use crate::survival::Container;
        self.hopper_t -= dt;
        if self.hopper_t > 0.0 {
            return;
        }
        self.hopper_t = 0.4;
        let hopper = self.reg.id_or_air("hopper");
        let hoppers: Vec<IVec3> = self.surv.containers.keys().copied().filter(|&p| self.reg.get(self.world.block_or_air(p)).base == hopper && hopper != AIR).collect();
        for p in hoppers {
            if self.redstone.powered(&self.world, p) {
                continue;
            }
            let v = self.reg.get(self.world.block_or_air(p)).variant;
            let spout = if v == 0 { IVec3::NEG_Y } else { h4(v - 1) };
            let Some(Container::Chest(mut inv)) = self.surv.containers.remove(&p) else { continue };
            // Out of the spout.
            if let Some(i) = inv.slots.iter().position(|s| s.is_some()) {
                let one = voxel::item::ItemStack { count: 1, ..inv.slots[i].unwrap() };
                let to = p + spout;
                let taken = match self.surv.containers.get_mut(&to) {
                    Some(Container::Chest(dest)) => dest.add(&self.items, one).is_none(),
                    Some(Container::Furnace(f)) => {
                        // From above into the input, from the side into the fuel.
                        if spout == IVec3::NEG_Y {
                            f.inv.add_in(&self.items, one, 0..1).is_none()
                        } else {
                            self.items.get(one.item).fuel > 0.0 && f.inv.add_in(&self.items, one, 1..2).is_none()
                        }
                    }
                    None => false,
                };
                if taken {
                    inv.take(i, 1);
                }
            }
            // In from above: a container's first item (a furnace's result), or items on top.
            let above = p + IVec3::Y;
            let pulled = match self.surv.containers.get_mut(&above) {
                Some(Container::Chest(src)) => src.slots.iter().position(|s| s.is_some()).map(|i| (i, src.slots[i].unwrap())).filter(|(_, st)| inv.clone().add(&self.items, voxel::item::ItemStack { count: 1, ..*st }).is_none()).map(|(i, _)| src.take(i, 1)).flatten(),
                Some(Container::Furnace(f)) => match f.inv.slots[2] {
                    Some(st) if inv.clone().add(&self.items, voxel::item::ItemStack { count: 1, ..st }).is_none() => f.inv.take(2, 1),
                    _ => None,
                },
                None => None,
            };
            if let Some(st) = pulled {
                inv.add(&self.items, st);
            } else if !self.redstone.support_at(&self.world, above) {
                let mut k = 0;
                while k < self.surv.drops.len() {
                    let d = &self.surv.drops[k];
                    let at = d.body.pos.floor().as_ivec3();
                    if at == above || at == p {
                        let st = d.stack;
                        match inv.add(&self.items, st) {
                            None => {
                                self.surv.drops.swap_remove(k);
                                continue;
                            }
                            Some(left) => self.surv.drops[k].stack = left,
                        }
                    }
                    k += 1;
                }
            }
            self.surv.containers.insert(p, Container::Chest(inv));
        }
    }

    /// A dropper or dispenser fires: one item from a random slot leaves through its front.
    fn dispense(&mut self, p: IVec3) {
        use crate::survival::Container;
        use glam::Vec3;
        let id = self.world.block_or_air(p);
        let b = self.reg.get(id);
        let dispenser = self.reg.get(b.base).name == "dispenser";
        if self.reg.get(b.base).name == "crafter" {
            return self.crafter_fire(p);
        }
        let d = dir6(b.variant);
        let front = p + d;
        let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&p) else { return };
        let full: Vec<usize> = (0..inv.slots.len()).filter(|&i| inv.slots[i].is_some()).collect();
        if full.is_empty() {
            self.sfx(crate::audio::Sfx::Click);
            return;
        }
        let i = full[self.surv.rng.below(full.len() as u32) as usize];
        let Some(one) = inv.take(i, 1) else { return };
        let name = self.items.get(one.item).name.clone();
        let out = p.as_vec3() + Vec3::splat(0.5) + d.as_vec3() * 0.7;
        let dir = d.as_vec3();
        let give_back = |g: &mut crate::Game, st: voxel::item::ItemStack| {
            if let Some(Container::Chest(inv)) = g.surv.containers.get_mut(&p) {
                if let Some(left) = inv.add(&g.items, st) {
                    g.spawn_drop(out, left, dir * 2.0);
                }
            }
        };
        let front_id = self.world.block_or_air(front);
        let open = front_id == AIR || self.reg.get(front_id).shape == Shape::Cross;
        if dispenser {
            match name.as_str() {
                "arrow" => {
                    self.mobs.arrows.push(crate::mobs::Arrow { pos: out, vel: dir * 30.0 + Vec3::Y * 1.0, age: 0.0, stuck: false, damage: 3.0 });
                    self.sfx(crate::audio::Sfx::Click);
                    return;
                }
                "tnt" if open => {
                    self.prime_tnt(front, 4.0);
                    return;
                }
                "water_bucket" | "lava_bucket" if open || self.world.is_liquid(front_id) => {
                    let f = if name == "water_bucket" { 1 } else { 2 };
                    let src = self.fluids.id(f, 0);
                    self.world.set_block(front, src);
                    if let Some(bucket) = self.items.id("bucket") {
                        give_back(self, voxel::item::ItemStack::new(bucket, 1));
                    }
                    return;
                }
                "bucket" if self.world.fluid(front_id) != 0 && self.reg.get(front_id).variant == 0 => {
                    let full = if self.world.fluid(front_id) == 1 { "water_bucket" } else { "lava_bucket" };
                    self.world.set_block(front, AIR);
                    if let Some(it) = self.items.id(full) {
                        give_back(self, voxel::item::ItemStack::new(it, 1));
                    }
                    return;
                }
                n if n.ends_with("_spawn_egg") && open => {
                    let mob = n.trim_end_matches("_spawn_egg").to_string();
                    if self.mobs.spawn(&mob, front.as_vec3() + Vec3::new(0.5, 0.0, 0.5)) {
                        return;
                    }
                }
                _ => {}
            }
        }
        // Into a container in front, or out onto the ground.
        if let Some(Container::Chest(dest)) = self.surv.containers.get_mut(&front) {
            if let Some(left) = dest.add(&self.items, one) {
                give_back(self, left);
            }
            return;
        }
        if self.world.block(front).is_some_and(|f| self.reg.is_opaque(f)) {
            give_back(self, one);
            return;
        }
        let spread = Vec3::new(self.surv.rng.f32() - 0.5, 0.0, self.surv.rng.f32() - 0.5) * 0.6;
        self.spawn_drop(out - Vec3::Y * 0.12, one, dir * 4.0 + spread + Vec3::Y * 1.0);
        self.sfx(crate::audio::Sfx::Click);
    }

    /// A crafter fires: its nine slots are a 3×3 crafting grid. One of each ingredient is used
    /// up and the result leaves through the front, into a container there or onto the ground.
    fn crafter_fire(&mut self, p: IVec3) {
        use crate::survival::Container;
        use glam::Vec3;
        let d = dir6(self.reg.get(self.world.block_or_air(p)).variant);
        let Some(Container::Chest(inv)) = self.surv.containers.get(&p) else { return };
        let grid: Vec<Option<voxel::item::ItemStack>> = (0..9).map(|i| inv.slots.get(i).copied().flatten()).collect();
        let Some(result) = self.recipes.craft(&grid, 3) else {
            self.sfx(crate::audio::Sfx::Click);
            return;
        };
        if let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&p) {
            for i in 0..inv.slots.len().min(9) {
                if inv.slots[i].is_some() {
                    inv.take(i, 1);
                }
            }
        }
        let front = p + d;
        let left = match self.surv.containers.get_mut(&front) {
            Some(Container::Chest(dest)) => dest.add(&self.items, result),
            _ => Some(result),
        };
        if let Some(st) = left {
            let out = p.as_vec3() + Vec3::splat(0.5) + d.as_vec3() * 0.7;
            self.spawn_drop(out - Vec3::Y * 0.12, st, d.as_vec3() * 2.0 + Vec3::Y * 0.5);
        }
        self.sfx(crate::audio::Sfx::Place(crate::audio::Material::Wood));
    }

    /// Test scene (--redstone): a few classic circuits on a stone floor in front of the player.
    pub(crate) fn build_redstone(&mut self) {
        let p = self.body.pos.floor().as_ivec3();
        let floor = self.reg.id_or_air("stone");
        for x in -8..=8 {
            for z in 2..=19 {
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
        // A torch on a wall, a comparator reading a chest, two observers watching each other
        // (a clock) and a hopper passing items from one chest to another.
        put(w, -7, 0, 17, "stone", 0);
        put(w, -6, 0, 17, "redstone_torch", 4);
        put(w, -4, 0, 17, "chest", 0);
        put(w, -3, 0, 17, "comparator", 1);
        put(w, -2, 0, 17, "redstone_wire", 0);
        put(w, -1, 0, 17, "redstone_wire", 0);
        put(w, 1, 0, 17, "redstone_wire", 0);
        put(w, 2, 0, 17, "observer", 1);
        put(w, 3, 0, 17, "observer", 3);
        put(w, 5, 1, 17, "chest", 0);
        put(w, 5, 0, 17, "hopper", 2);
        put(w, 6, 0, 17, "chest", 0);
        put(w, 6, 0, 19, "dispenser", 2);
        let at = |x: i32, y: i32, z: i32| IVec3::new(p.x + x, p.y + y, p.z - z);
        let stack = |g: &crate::Game, n: &str, c: u8| g.items.id(n).map(|i| voxel::item::ItemStack::new(i, c));
        let mut a = voxel::item::Inventory::new(27);
        for k in 0..10 {
            a.slots[k] = stack(self, "cobblestone", 64);
        }
        self.surv.containers.insert(at(-4, 0, 17), crate::survival::Container::Chest(a));
        let mut b = voxel::item::Inventory::new(27);
        b.slots[0] = stack(self, "iron_ingot", 32);
        self.surv.containers.insert(at(5, 1, 17), crate::survival::Container::Chest(b));
        self.surv.containers.insert(at(5, 0, 17), crate::survival::Container::Chest(voxel::item::Inventory::new(5)));
        self.surv.containers.insert(at(6, 0, 17), crate::survival::Container::Chest(voxel::item::Inventory::new(27)));
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
                    ..bytemuck::Zeroable::zeroed()
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
    fn command_block_runs_once_per_pulse() {
        let (mut rs, mut m, reg) = setup();
        let cb = reg.id("command_block").unwrap();
        let lever = reg.id("lever").unwrap();
        run(&mut rs, &mut m, &[(IVec3::new(0, 1, 0), cb), (IVec3::new(1, 1, 0), lever)], 0.2);
        let count = |rs: &mut Redstone| std::mem::take(&mut rs.events).iter().filter(|e| matches!(e, Event::Command(_))).count();
        assert_eq!(count(&mut rs), 0);
        rs.interact(&mut m, IVec3::new(1, 1, 0));
        run(&mut rs, &mut m, &[], 0.5);
        assert_eq!(count(&mut rs), 1, "one run while the power stays on");
        rs.interact(&mut m, IVec3::new(1, 1, 0));
        run(&mut rs, &mut m, &[], 0.3);
        rs.interact(&mut m, IVec3::new(1, 1, 0));
        run(&mut rs, &mut m, &[], 0.3);
        assert_eq!(count(&mut rs), 1, "and once more for the next pulse");
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

    #[test]
    fn wall_torch_inverts_its_wall() {
        let (mut rs, mut m, reg) = setup();
        let stone = reg.id("stone").unwrap();
        // A torch on the east face of a stone block (its wall is to the west: facing 3, + 1),
        // and a block of redstone on the stone's other side.
        let torch = reg.variant(reg.id("redstone_torch").unwrap(), 4);
        run(&mut rs, &mut m, &[(IVec3::new(0, 1, 0), stone), (IVec3::new(1, 1, 0), torch)], 0.3);
        assert_eq!(rs.kind(m.get(IVec3::new(1, 1, 0)).unwrap()), Kind::Torch(true));
        let lever = reg.variant(reg.id("lever").unwrap(), 1 | 4 | 8);
        run(&mut rs, &mut m, &[(IVec3::new(-1, 1, 0), lever)], 0.3);
        let t = m.get(IVec3::new(1, 1, 0)).unwrap();
        assert_eq!(rs.kind(t), Kind::Torch(false));
        assert_eq!(reg.get(t).variant, 4, "keeps its wall");
    }

    #[test]
    fn comparator_subtracts_its_side() {
        let (mut rs, mut m, reg) = setup();
        let wire = reg.id("redstone_wire").unwrap();
        let block = reg.id("redstone_block").unwrap();
        // Facing east in subtract mode: 15 from the back, a side line that has faded to 12.
        let comp = reg.variant(reg.id("comparator").unwrap(), 1 | 4);
        let mut puts = vec![(IVec3::new(-1, 1, 0), block), (IVec3::new(0, 1, 0), comp), (IVec3::new(1, 1, 0), wire)];
        for z in 1..=4 {
            puts.push((IVec3::new(0, 1, z), wire));
        }
        puts.push((IVec3::new(0, 1, 5), block));
        run(&mut rs, &mut m, &puts, 0.3);
        let side = power(&m, &reg, IVec3::new(0, 1, 1));
        assert_eq!(side, 12);
        assert_eq!(power(&m, &reg, IVec3::new(1, 1, 0)), 3);
        // A container behind it: the output follows how full it is.
        let chest = reg.id("chest").unwrap();
        rs.set_levels([(IVec3::new(-1, 1, 0), 7)].into_iter().collect());
        let comp = reg.variant(reg.id("comparator").unwrap(), 1);
        run(&mut rs, &mut m, &[(IVec3::new(-1, 1, 0), chest), (IVec3::new(0, 1, 0), comp), (IVec3::new(0, 1, 1), AIR)], 0.3);
        assert_eq!(power(&m, &reg, IVec3::new(1, 1, 0)), 7);
    }

    #[test]
    fn observer_pulses_when_its_block_changes() {
        let (mut rs, mut m, reg) = setup();
        let wire = reg.id("redstone_wire").unwrap();
        // Looking north (0) at z = -1, its back to dust at z = 1.
        let obs = reg.variant(reg.id("observer").unwrap(), 0);
        run(&mut rs, &mut m, &[(IVec3::new(0, 1, 0), obs), (IVec3::new(0, 1, 1), wire)], 0.3);
        assert_eq!(power(&m, &reg, IVec3::new(0, 1, 1)), 0);
        run(&mut rs, &mut m, &[(IVec3::new(0, 1, -1), reg.id("dirt").unwrap())], 0.12);
        assert_eq!(power(&m, &reg, IVec3::new(0, 1, 1)), 15);
        run(&mut rs, &mut m, &[], 0.3);
        assert_eq!(power(&m, &reg, IVec3::new(0, 1, 1)), 0);
    }

    #[test]
    fn copper_bulb_toggles_on_each_pulse() {
        let (mut rs, mut m, reg) = setup();
        let bulb = reg.id("copper_bulb").unwrap();
        let lit = reg.id("copper_bulb_lit").unwrap();
        let lever = reg.id("lever").unwrap();
        let b = IVec3::new(0, 1, 0);
        let l = IVec3::new(1, 1, 0);
        run(&mut rs, &mut m, &[(b, bulb), (l, reg.variant(lever, 1))], 0.5);
        assert_eq!(m.get(b), Some(bulb));
        // On: it lights and stays lit when the lever goes off again.
        run(&mut rs, &mut m, &[(l, reg.variant(lever, 1 | 4))], 0.5);
        assert_eq!(m.get(b), Some(lit));
        run(&mut rs, &mut m, &[(l, reg.variant(lever, 1))], 0.5);
        assert_eq!(m.get(b), Some(lit));
        // The next pulse puts it out.
        run(&mut rs, &mut m, &[(l, reg.variant(lever, 1 | 4))], 0.5);
        assert_eq!(m.get(b), Some(bulb));
    }

    #[test]
    fn powered_rails_pass_power_down_the_line() {
        let (mut rs, mut m, reg) = setup();
        let rail = reg.id("powered_rail").unwrap();
        let block = reg.id("redstone_block").unwrap();
        let mut puts: Vec<(IVec3, BlockId)> = (0..12).map(|x| (IVec3::new(x, 1, 0), reg.variant(rail, 1))).collect();
        puts.push((IVec3::new(-1, 1, 0), block));
        run(&mut rs, &mut m, &puts, 0.5);
        for x in 0..=8 {
            assert!(power(&m, &reg, IVec3::new(x, 1, 0)) >= 10, "rail {x} should be on");
        }
        for x in 10..12 {
            assert!(power(&m, &reg, IVec3::new(x, 1, 0)) < 10, "rail {x} should be off");
        }
    }

}
