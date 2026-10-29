//! The test world ("Mundo de teste", picked on the new-world screen): a flat, always sunny,
//! creative world with everything the game has set out in labelled rows, so all of it can be
//! looked over in one walk. Nothing is listed by hand: the rows come from the registries
//! (blocks and items by their creative tab, every mob, every building), so the world stays
//! complete as content is added. It is laid out again each time it is opened.
//!
//! Walking south (+Z) from the spawn: the blocks by tab, armour sets on stands, tools and
//! weapons held by stands, the other items lying on the ground, then the mobs in pens (animals
//! behind fences, monsters behind glass, bosses in big glass rooms, townsfolk on open plots,
//! vehicles), then one of every building. Each thing has a sign beside it with its name, and
//! the mobs and stands wear their name too. Mobs stand still (`Mob::still`, like NoAI), no
//! monster spawns and the sun stays at noon.
//!
//! The generator only stamps the blocks into the chunks they fall in (voxel's
//! worldgen/showcase.rs), so the world loads as fast as a superflat one.

use std::sync::Arc;

use glam::Vec3;
use voxel::item::ItemStack;
use voxel::mob::Kind;
use voxel::structures::Placement;
use voxel::worldgen::showcase::Showcase;
use voxel::BlockId;

use crate::creative::Tab;
use crate::stands::Gear;
use crate::Game;

/// The marker file in a world's folder that makes it a test world.
pub const MARKER: &str = "teste.ron";
const FLOOR: i32 = 64;
const Y: i32 = FLOOR + 1;
/// The rows run from x = X0 for W blocks.
const X0: i32 = -48;
const W: i32 = 96;
/// The buildings have a wider field of their own, east of the rows.
const BX0: i32 = X0 + W + 16;
const BW: i32 = 320;
/// How far away the items lying on the ground are drawn.
const GROUND_RANGE: f32 = 64.0;

/// Mobs that are parts of something else or drawn by their own system: not shown.
const HIDDEN_MOBS: &[&str] = &["player", "armor_stand", "windmill_sails", "harness", "sentry", "wagon", "wagon_cover", "wagon_cargo", "wagon_hay", "wagon_shafts", "wagon_pole", "wagon_wheel"];
const VEHICLES: &[&str] = &["boat", "minecart", "biplane", "airship", "cargo_airship", "gyrodyne", "quadrocopter", "warship"];

#[derive(Default)]
pub(crate) struct TestWorld {
    pub on: bool,
    pub layout: Option<Arc<Showcase>>,
    /// Where each thing goes, put in the world once it has loaded.
    stands: Vec<(Vec3, Gear, String)>,
    mobs: Vec<(String, Vec3, String)>,
    signs: Vec<([i32; 3], String)>,
    /// Items lying on the ground: drawn like drops, never picked up (`drop_instances`).
    pub ground: Vec<(Vec3, ItemStack, f32)>,
    spawned: bool,
}

/// A name cut into sign lines (four of fifteen letters), breaking between words.
fn sign_text(name: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in name.split_whitespace() {
        let mut word: String = word.to_string();
        while word.chars().count() > crate::signs::LINE_LEN {
            let head: String = word.chars().take(crate::signs::LINE_LEN).collect();
            word = word.chars().skip(crate::signs::LINE_LEN).collect();
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            lines.push(head);
        }
        if cur.is_empty() {
            cur = word;
        } else if cur.chars().count() + 1 + word.chars().count() <= crate::signs::LINE_LEN {
            cur = format!("{cur} {word}");
        } else {
            lines.push(std::mem::replace(&mut cur, word));
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines.truncate(crate::signs::LINES);
    lines.join("\n")
}

/// "vila_carvalho" -> "Vila carvalho".
fn pretty(name: &str) -> String {
    let s = name.replace('_', " ");
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// Where things go as the plan is drawn up.
struct Plan {
    sc: Showcase,
    sign: BlockId,
    stands: Vec<(Vec3, Gear, String)>,
    mobs: Vec<(String, Vec3, String)>,
    signs: Vec<([i32; 3], String)>,
    ground: Vec<(Vec3, ItemStack, f32)>,
    /// The next free row, and the next free slot in it.
    z: i32,
    col: i32,
    bricks: BlockId,
    /// Each section's name and where it starts, for the board at the spawn.
    index: Vec<(String, i32, i32)>,
}

impl Plan {
    fn sign(&mut self, x: i32, y: i32, z: i32, text: &str) {
        // Facing north, toward whoever walks the row in front.
        self.sc.set(x, y, z, self.sign);
        self.signs.push(([x, y, z], sign_text(text)));
    }

    /// A section's name on a paved strip across the whole width.
    fn header(&mut self, title: &str) {
        self.end_row();
        self.index.push((title.to_string(), 0, self.z + 3));
        self.z += 2;
        for x in X0 - 2..X0 + W + 2 {
            self.sc.set(x, FLOOR, self.z, self.bricks);
        }
        for x in (X0..X0 + W).step_by(12) {
            self.sign(x + 5, Y, self.z, title);
        }
        self.z += 3;
    }

    fn end_row(&mut self) {
        if self.col > 0 {
            self.col = 0;
            self.z += 3;
        }
    }

    /// The next slot of a row of small things: the thing at x, its sign just west of it (to
    /// its right for whoever faces the row). Rows fill from east to west, so they read left
    /// to right from the walkway.
    fn slot(&mut self, label: &str) -> (i32, i32) {
        if self.col >= W / 2 {
            self.end_row();
        }
        let x = X0 + W - 1 - self.col * 2;
        self.col += 1;
        self.sign(x - 1, Y, self.z, label);
        (x, self.z)
    }
}

/// A row of pens or buildings of different sizes, filled west to east.
struct Shelf {
    x0: i32,
    w: i32,
    x: i32,
    depth: i32,
}

impl Shelf {
    fn new(x0: i32, w: i32) -> Shelf {
        Shelf { x0, w, x: x0, depth: 0 }
    }

    fn place(&mut self, plan: &mut Plan, w: i32, d: i32, gap: i32) -> (i32, i32) {
        if self.x > self.x0 && self.x + w > self.x0 + self.w {
            plan.z += self.depth + gap;
            self.x = self.x0;
            self.depth = 0;
        }
        let at = (self.x, plan.z);
        self.x += w + gap;
        self.depth = self.depth.max(d);
        at
    }

    fn close(&mut self, plan: &mut Plan, gap: i32) {
        if self.depth > 0 {
            plan.z += self.depth + gap;
        }
        self.x = self.x0;
        self.depth = 0;
    }
}

impl Game {
    /// Lay out the test world from what the game has loaded.
    pub(crate) fn plan_test_world(&mut self) {
        let g = |n: &str| self.reg.id_or_air(n);
        let (grass, dirt, stone) = (g("grass_block"), g("dirt"), g("stone"));
        let sign = self.reg.variant(g("oak_sign"), 2);
        let mut plan = Plan {
            sc: Showcase::new(FLOOR, grass, dirt, stone),
            sign,
            stands: Vec::new(),
            mobs: Vec::new(),
            signs: Vec::new(),
            ground: Vec::new(),
            z: 0,
            col: 0,
            bricks: g("stone_bricks"),
            index: Vec::new(),
        };

        // Blocks, by creative tab.
        for tab in [Tab::Building, Tab::Colored, Tab::Natural, Tab::Functional, Tab::Doors] {
            plan.header(tab.title());
            for id in self.creative_list(tab) {
                let it = self.items.get(id);
                let Some(b) = it.places else { continue };
                let label = it.display_name.clone();
                let (x, z) = plan.slot(&label);
                plan.sc.set(x, Y, z, b);
                // MC_TW_LIST=1 prints where each block stands (for screenshots).
                if std::env::var_os("MC_TW_LIST").is_some() {
                    eprintln!("tw {} {x} {Y} {z}", self.reg.get(b).name);
                }
            }
        }

        // Items: armour sets on stands, held things on stands, the rest on the ground.
        let mut armour: Vec<(String, Vec<voxel::item::ItemId>)> = Vec::new();
        let mut held: Vec<voxel::item::ItemId> = Vec::new();
        let mut loose: Vec<(Tab, voxel::item::ItemId)> = Vec::new();
        for id in 1..self.items.items.len() as voxel::item::ItemId {
            let Some(tab) = self.creative_tab_of(id) else { continue };
            let it = self.items.get(id);
            if it.places.is_some() && !matches!(tab, Tab::Tools | Tab::Combat | Tab::Food | Tab::Ingredients | Tab::Eggs) {
                continue;
            }
            if it.armor.is_some() {
                // A set is the pieces sharing a name but the last word (iron_helmet, iron_boots).
                let key = it.name.rsplit_once('_').map(|(a, _)| a.to_string()).unwrap_or_else(|| it.name.clone());
                match armour.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, v)) => v.push(id),
                    None => armour.push((key, vec![id])),
                }
            } else if it.tool.is_some() || matches!(tab, Tab::Tools | Tab::Combat) {
                held.push(id);
            } else {
                loose.push((tab, id));
            }
        }
        plan.header("Armaduras");
        for (_, pieces) in &armour {
            // Pieces for the same slot (a hat and a helmet of one set) go on more stands.
            let mut stands: Vec<Gear> = Vec::new();
            for &id in pieces {
                let slot = self.items.get(id).armor.map(|a| a.slot.min(3) as usize).unwrap_or(0);
                match stands.iter_mut().find(|g| g[slot].is_none()) {
                    Some(g) => g[slot] = Some(ItemStack::new(id, 1)),
                    None => {
                        let mut g: Gear = Default::default();
                        g[slot] = Some(ItemStack::new(id, 1));
                        stands.push(g);
                    }
                }
            }
            for gear in stands {
                let names: Vec<&str> = gear.iter().flatten().map(|s| self.items.get(s.item).display_name.as_str()).collect();
                let label = common_suffix(&names).map(|s| format!("Conjunto {s}")).unwrap_or_else(|| names.join(", "));
                let (x, z) = plan.slot(&label);
                plan.stands.push((Vec3::new(x as f32 + 0.5, Y as f32, z as f32 + 0.5), gear, label));
            }
        }
        plan.header("Ferramentas e armas");
        for &id in &held {
            let label = self.items.get(id).display_name.clone();
            let (x, z) = plan.slot(&label);
            let mut gear: Gear = Default::default();
            gear[4] = Some(ItemStack::new(id, 1));
            plan.stands.push((Vec3::new(x as f32 + 0.5, Y as f32, z as f32 + 0.5), gear, label));
        }
        for tab in [Tab::Food, Tab::Ingredients, Tab::Eggs, Tab::Tools, Tab::Combat] {
            let list: Vec<_> = loose.iter().filter(|(t, _)| *t == tab).map(|(_, id)| *id).collect();
            if list.is_empty() {
                continue;
            }
            plan.header(&format!("Itens: {}", tab.title()));
            for id in list {
                let label = self.items.get(id).display_name.clone();
                let (x, z) = plan.slot(&label);
                plan.ground.push((Vec3::new(x as f32 + 0.5, Y as f32, z as f32 + 0.5), ItemStack::new(id, 1), 0.6));
            }
        }

        // Mobs in pens.
        let defs = &self.mobs.reg.defs;
        let mut groups: [(&str, Vec<usize>); 5] =
            [("Animais", Vec::new()), ("Monstros", Vec::new()), ("Chefes", Vec::new()), ("Moradores e NPCs", Vec::new()), ("Veiculos", Vec::new())];
        for (i, d) in defs.iter().enumerate() {
            let n = d.name.as_str();
            if HIDDEN_MOBS.contains(&n) || n.ends_with("_body") || n.ends_with("_tail") || d.parts.is_empty() {
                continue;
            }
            let g = if VEHICLES.contains(&n) {
                4
            } else if d.boss {
                2
            } else if d.town || n.starts_with("settler_") || n.starts_with("npc_") {
                3
            } else if d.kind == Kind::Hostile {
                1
            } else {
                0
            };
            groups[g].1.push(i);
        }
        let (fence, glass) = (g("oak_fence"), g("glass"));
        for (title, list) in &groups {
            if list.is_empty() {
                continue;
            }
            plan.header(title);
            let mut shelf = Shelf::new(X0, W);
            for &i in list {
                let d = &defs[i];
                let size = d.scale.max(0.3);
                let reach = (d.width.max(d.height * 0.5) * size).ceil() as i32;
                // Bosses' models reach well past their hit boxes.
                let inner = if d.boss { (reach * 2 + 2).max(9) } else { (reach + 2).max(3) } | 1;
                let (w, dd) = (inner + 2, inner + 2);
                let (x, z) = shelf.place(&mut plan, w, dd, 2);
                let (wall, h) = match *title {
                    "Monstros" | "Chefes" => (glass, ((d.height * size).ceil() as i32 + 1).clamp(2, 14)),
                    "Animais" => (fence, 1),
                    _ => (AIR_BLOCK, 0),
                };
                for y in 0..h {
                    for k in 0..w {
                        for (px, pz) in [(x + k, z), (x + k, z + dd - 1), (x, z + k), (x + w - 1, z + k)] {
                            plan.sc.set(px, Y + y, pz, wall);
                        }
                    }
                }
                let label = d.display_name.clone();
                plan.sign(x + w / 2, Y, z - 1, &label);
                let at = Vec3::new(x as f32 + w as f32 / 2.0, Y as f32, z as f32 + dd as f32 / 2.0);
                plan.mobs.push((d.name.clone(), at, label));
            }
            shelf.close(&mut plan, 1);
        }

        // One of every building, in their own field east of the rows.
        let rows_end = plan.z;
        plan.z = 4;
        for z in 0..3 {
            for x in BX0 - 2..BX0 + BW {
                plan.sc.set(x, FLOOR, z, plan.bricks);
            }
        }
        for x in (BX0..BX0 + BW).step_by(16) {
            plan.sign(x + 2, Y, 1, "Construcoes: uma de cada");
        }
        plan.index.push(("Construcoes".into(), BX0 + 4, 2));
        let mut shelf = Shelf::new(BX0, BW);
        for (si, s) in self.structures.list.iter().enumerate() {
            let f = s.turned_footprint(0);
            let (x, z) = shelf.place(&mut plan, f.x.max(3), f.y + 1, 4);
            let sink = s.def.sink.min(1);
            plan.sc.buildings.push(Placement { index: si, origin: glam::IVec3::new(x, Y - sink, z + 1), turns: 0 });
            plan.sign(x + f.x / 2, Y, z - 1, &pretty(&s.def.name));
        }
        shelf.close(&mut plan, 4);
        let field_end = plan.z;
        self.plan_rail_track(&mut plan);
        plan.z = rows_end.max(field_end);
        // The board at the spawn: what this is, and where each section starts (/tp goes there).
        let mut board: Vec<String> = ["Mundo de teste", "Ande para o sul: blocos, itens e mobs", "Refeito ao abrir: o que mudar se perde"].iter().map(|t| sign_text(t)).collect();
        for (title, x, z) in std::mem::take(&mut plan.index) {
            let mut lines: Vec<String> = sign_text(&title).lines().take(3).map(str::to_string).collect();
            lines.push(format!("/tp {x} {} {z}", Y + 1));
            board.push(lines.join("\n"));
        }
        let n = board.len() as i32;
        for (k, text) in board.into_iter().enumerate() {
            let at = [n - k as i32 * 2, Y, -3];
            plan.sc.set(at[0], at[1], at[2], plan.sign);
            plan.signs.push((at, text));
        }
        for x in -n - 1..n + 1 {
            plan.sc.set(x, FLOOR, -3, plan.bricks);
        }
        log::info!(
            "test world: {} blocks set, {} stands, {} items on the ground, {} mobs, {} buildings, {} rows deep",
            plan.sc.block_count(),
            plan.stands.len(),
            plan.ground.len(),
            plan.mobs.len(),
            plan.sc.buildings.len(),
            plan.z
        );
        self.testw = TestWorld { on: true, layout: Some(Arc::new(plan.sc)), stands: plan.stands, mobs: plan.mobs, signs: plan.signs, ground: plan.ground, spawned: false };
    }

    /// A loop of rails west of the rows, with its four curves, and a ramp up onto a step, so
    /// curves and slopes can be seen (straight rails stand in the rows).
    fn plan_rail_track(&self, plan: &mut Plan) {
        let rail = self.reg.id_or_air("rail");
        let powered = self.reg.id_or_air("powered_rail");
        let r = |v: u8| self.reg.variant(rail, v);
        let (x0, x1, z0, z1) = (X0 - 14, X0 - 6, 4, 12);
        for x in x0 + 1..x1 {
            plan.sc.set(x, Y, z0, r(1));
            plan.sc.set(x, Y, z1, self.reg.variant(powered, if x % 2 == 0 { 11 } else { 1 }));
        }
        for z in z0 + 1..z1 {
            plan.sc.set(x0, Y, z, r(0));
            plan.sc.set(x1, Y, z, r(0));
        }
        // Curves: north-west joins south and east, and so on round the loop.
        plan.sc.set(x0, Y, z0, r(6));
        plan.sc.set(x1, Y, z0, r(7));
        plan.sc.set(x1, Y, z1, r(8));
        plan.sc.set(x0, Y, z1, r(9));
        // A ramp climbing north onto a step two blocks long, and down again.
        let x = x0 + 4;
        let stone = self.reg.id_or_air("stone");
        plan.sc.set(x, Y, z0 + 4, r(4));
        for z in z0 + 2..z0 + 4 {
            plan.sc.set(x, Y, z, stone);
        }
        plan.sc.set(x, Y + 1, z0 + 3, r(0));
        plan.sc.set(x, Y + 1, z0 + 2, r(0));
        plan.sc.set(x, Y, z0 + 1, r(5));
        plan.sign(x0 + 2, Y, z0 - 2, "Trilhos: curvas e rampas");
    }

    /// Every frame in a test world: noon, clear sky, no spawning; the things set out once the
    /// world has loaded.
    pub(crate) fn update_test_world(&mut self) {
        if !self.testw.on || !self.world_open || self.dim != voxel::worldgen::Dim::Overworld {
            return;
        }
        self.time_of_day = 6000.0;
        self.weather.state.raining = false;
        self.weather.state.thunder = false;
        self.mobs.spawning = false;
        if self.testw.spawned || self.screen == crate::Screen::Loading {
            return;
        }
        self.testw.spawned = true;
        for (p, text) in &self.testw.signs {
            self.signs.insert(*p, text.clone());
        }
        let face = std::f32::consts::PI;
        for (name, at, label) in self.testw.mobs.clone() {
            if self.mobs.spawn(&name, at) {
                let m = self.mobs.list.last_mut().unwrap();
                m.still = true;
                m.yaw = face;
                m.head_yaw = 0.0;
                m.name = Some(label);
            }
        }
        for (at, gear, label) in self.testw.stands.clone() {
            if self.mobs.spawn("armor_stand", at) {
                let m = self.mobs.list.last_mut().unwrap();
                m.still = true;
                m.yaw = face;
                m.name = Some(label);
                let (id, pos) = (m.net_id, m.body.pos);
                self.stands.gear.insert(id, (gear, pos));
            }
        }
    }

    /// The items lying on the ground near the camera: (where, what, turn).
    pub(crate) fn test_ground_near(&self, cam: Vec3) -> Vec<(Vec3, ItemStack, f32)> {
        if !self.testw.on || self.dim != voxel::worldgen::Dim::Overworld {
            return Vec::new();
        }
        self.testw.ground.iter().filter(|(p, _, _)| (*p - cam).length_squared() < GROUND_RANGE * GROUND_RANGE).copied().collect()
    }
}

const AIR_BLOCK: BlockId = voxel::AIR;

/// The words the names end with in common ("Capacete de Ferro", "Botas de Ferro" -> "de Ferro").
fn common_suffix(names: &[&str]) -> Option<String> {
    let first: Vec<&str> = names.first()?.split_whitespace().collect();
    if names.len() < 2 {
        return None;
    }
    let mut n = first.len();
    for other in &names[1..] {
        let w: Vec<&str> = other.split_whitespace().collect();
        let common = first.iter().rev().zip(w.iter().rev()).take_while(|(a, b)| a == b).count();
        n = n.min(common);
    }
    (n > 0 && n < first.len()).then(|| first[first.len() - n..].join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_text_wraps_words() {
        assert_eq!(sign_text("Capacete de Diamante"), "Capacete de\nDiamante");
        assert_eq!(sign_text("Mundo de teste"), "Mundo de teste");
        assert!(sign_text("a b c d e f g h i j k l m n o p q r s t u v w x y z a b c d e f g h i j k l m n o p").lines().count() <= crate::signs::LINES);
    }

    #[test]
    fn armour_sets_get_a_common_name() {
        assert_eq!(common_suffix(&["Capacete de Ferro", "Botas de Ferro"]), Some("de Ferro".into()));
        assert_eq!(common_suffix(&["Capacete de Ferro"]), None);
    }
}
