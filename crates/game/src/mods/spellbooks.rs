//! Iron's Spells 'n Spellbooks' idea, our own spells and code: eleven schools (the six of the
//! wands plus holy, ender, blood, eldritch and evocation) and 26 spells with five levels each
//! (a higher level hits harder and costs less mana). Every spell has a single-use scroll; a
//! spellbook holds several, written in at the inscription table, and casts the one picked with V.
//! Wizard robes raise the most mana and their school's power. Pyromancers, cryomancers and
//! necromancers cast from the same table, and so does the Corrupted Archmage, a boss called with
//! a seal. The wizard tower (tools/gen_mod_structures.py) keeps scrolls and ink.
//!
//! Spells are made of a few shapes: bolts (some home in or burst), beams, cones, novas round the
//! caster, areas where you look, lightning strikes, a line of fangs, summoned allies, a blink,
//! a heal and three buffs. A book's spells live in `ModSave::spellbooks`, found by the number the
//! book stack carries in the low bits of its `damage` (scrolls keep their level there).

use glam::Vec3;
use serde::{Deserialize, Serialize};
use voxel::item::ItemStack;
use voxel::mob::Kind;
use voxel::physics::Terrain;
use winit::keyboard::KeyCode;

use crate::audio::Sfx;
use crate::elements::El;
use crate::Game;

/// The low bits of a stack's `damage` that hold a book's number or a scroll's level (the top
/// four may hold an item modifier).
const ID_MASK: u16 = 0x0FFF;
pub const MAX_LEVEL: u8 = 5;

pub const SCHOOLS: [&str; 11] = ["Fogo", "Gelo", "Faísca", "Arcano", "Natureza", "Tempestade", "Sagrado", "Fim", "Sangue", "Abissal", "Evocação"];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Buff {
    /// Soaks up this many points of damage.
    Shield,
    /// Half of every hit is turned aside, for a while.
    Ward,
    /// Life comes back over ten seconds.
    Regen,
    /// Faster feet and shorter cooldowns.
    Haste,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Form {
    /// A flying bolt: speed, fall, homing, burst radius (0 = none).
    Bolt { speed: f32, gravity: f32, homing: bool, burst: f32 },
    /// A straight ray hitting everything in line; `through` goes through walls.
    Beam { range: f32, through: bool },
    /// Everything in front, close by; `push` throws them back.
    Cone { range: f32, push: f32 },
    /// Everything around the caster.
    Nova { radius: f32, push: f32 },
    /// Everything round the spot looked at; `lift` throws them up.
    Area { radius: f32, lift: f32 },
    /// Lightning onto the monsters round the spot looked at, one more per level.
    Strikes,
    /// A line of jaws snapping up out of the ground, ahead.
    Fangs,
    /// Allies that fight for a while.
    Summon(&'static str),
    /// A jump to where you look.
    Blink,
    Heal,
    Buff(Buff),
}

pub struct Spell {
    pub id: &'static str,
    pub name: &'static str,
    pub school: u8,
    form: Form,
    /// Damage at level 1 (heal, shield, range or seconds for the spells that don't hurt).
    base: f32,
    /// Added per level above the first.
    per_level: f32,
    /// Mana at level 1; each level above costs 8% less.
    cost: f32,
    cooldown: f32,
}

const fn sp(id: &'static str, name: &'static str, school: u8, form: Form, base: f32, per_level: f32, cost: f32, cooldown: f32) -> Spell {
    Spell { id, name, school, form, base, per_level, cost, cooldown }
}

const fn bolt(speed: f32, gravity: f32, homing: bool, burst: f32) -> Form {
    Form::Bolt { speed, gravity, homing, burst }
}

/// The spells, in the order of tools/moddata/spellbooks.py.
pub const SPELLS: [Spell; 26] = [
    sp("fireball", "Bola de Fogo", 0, bolt(18.0, 3.0, false, 3.0), 7.0, 2.5, 12.0, 1.0),
    sp("fire_breath", "Sopro de Fogo", 0, Form::Cone { range: 6.0, push: 0.5 }, 2.0, 0.8, 3.0, 0.25),
    sp("ice_spike", "Estilhaço de Gelo", 1, bolt(30.0, 0.0, false, 0.0), 5.0, 2.0, 6.0, 0.5),
    sp("frost_nova", "Nova Gélida", 1, Form::Nova { radius: 5.0, push: 4.0 }, 4.0, 1.5, 14.0, 2.5),
    sp("lightning_lance", "Lança Relâmpago", 2, bolt(55.0, 0.0, false, 0.0), 7.0, 3.0, 10.0, 0.9),
    sp("arc_beam", "Arco Elétrico", 2, Form::Beam { range: 14.0, through: false }, 1.6, 0.6, 2.5, 0.2),
    sp("magic_missile", "Míssil Mágico", 3, bolt(16.0, 0.0, true, 0.0), 4.0, 1.5, 6.0, 0.6),
    sp("arcane_barrier", "Barreira Arcana", 3, Form::Buff(Buff::Ward), 10.0, 3.0, 18.0, 6.0),
    sp("thorn_roots", "Raízes Espinhosas", 4, Form::Area { radius: 3.0, lift: 0.0 }, 3.0, 1.5, 12.0, 2.0),
    sp("nature_blessing", "Bênção da Natureza", 4, Form::Buff(Buff::Regen), 6.0, 3.0, 16.0, 8.0),
    sp("storm_call", "Chamado da Tempestade", 5, Form::Strikes, 8.0, 2.0, 20.0, 3.0),
    sp("gust", "Rajada de Vento", 5, Form::Cone { range: 8.0, push: 14.0 }, 2.0, 1.0, 6.0, 1.0),
    sp("heal", "Cura", 6, Form::Heal, 4.0, 2.0, 12.0, 2.0),
    sp("sacred_light", "Luz Sagrada", 6, Form::Nova { radius: 6.0, push: 2.0 }, 4.0, 2.0, 16.0, 2.5),
    sp("fortitude", "Fortaleza", 6, Form::Buff(Buff::Shield), 6.0, 3.0, 20.0, 10.0),
    sp("teleport", "Teletransporte", 7, Form::Blink, 10.0, 3.0, 8.0, 1.0),
    sp("void_rift", "Fenda do Vazio", 7, Form::Area { radius: 3.0, lift: 12.0 }, 4.0, 2.0, 14.0, 2.0),
    sp("ender_orb", "Orbe do Fim", 7, bolt(12.0, 0.0, true, 0.0), 6.0, 2.5, 10.0, 1.2),
    sp("life_drain", "Drenar Vida", 8, Form::Beam { range: 10.0, through: false }, 1.5, 0.6, 2.5, 0.2),
    sp("raise_dead", "Erguer os Mortos", 8, Form::Summon("risen_skeleton"), 25.0, 8.0, 22.0, 12.0),
    sp("blood_slash", "Corte Sangrento", 8, Form::Cone { range: 4.5, push: 4.0 }, 7.0, 3.0, 6.0, 0.8),
    sp("abyss_ray", "Raio Abissal", 9, Form::Beam { range: 28.0, through: true }, 12.0, 5.0, 26.0, 3.0),
    sp("abyss_scream", "Grito do Abismo", 9, Form::Nova { radius: 8.0, push: 10.0 }, 8.0, 3.0, 24.0, 4.0),
    sp("fangs", "Presas do Evocador", 10, Form::Fangs, 5.0, 2.0, 14.0, 1.5),
    sp("summon_wolves", "Invocar Lobos", 10, Form::Summon("spectral_wolf"), 25.0, 8.0, 20.0, 12.0),
    sp("haste", "Pressa", 10, Form::Buff(Buff::Haste), 10.0, 4.0, 14.0, 10.0),
];

impl Spell {
    /// Damage (or heal, shield, range, seconds) at a level.
    fn amount(&self, level: u8) -> f32 {
        self.base + self.per_level * (level.clamp(1, MAX_LEVEL) - 1) as f32
    }

    fn mana(&self, level: u8) -> f32 {
        self.cost * (1.0 - 0.08 * (level.clamp(1, MAX_LEVEL) - 1) as f32)
    }
}

pub fn spell_index(id: &str) -> Option<usize> {
    SPELLS.iter().position(|s| s.id == id)
}

/// The spell a scroll item carries.
pub fn scroll_spell(item: &str) -> Option<usize> {
    item.strip_prefix("scroll_").and_then(spell_index)
}

/// Books and their number of slots.
const BOOKS: [(&str, usize); 4] = [("spellbook_apprentice", 3), ("spellbook_iron", 5), ("spellbook_diamond", 8), ("spellbook_archmage", 10)];

pub fn is_book(item: &str) -> bool {
    BOOKS.iter().any(|b| b.0 == item)
}

fn book_slots(item: &str) -> usize {
    BOOKS.iter().find(|b| b.0 == item).map(|b| b.1).unwrap_or(0)
}

/// The robes: item prefix and school.
const ROBES: [(&str, u8); 8] = [
    ("pyromancer", 0),
    ("cryomancer", 1),
    ("electromancer", 2),
    ("priest", 6),
    ("voidwalker", 7),
    ("bloodmage", 8),
    ("cultist", 9),
    ("archevoker", 10),
];
/// Mana each robe piece adds (hat, robe, trousers, boots).
const ROBE_MANA: [f32; 4] = [2.0, 4.0, 3.0, 1.0];

/// Monsters that cast from the spell table: name and their spells.
const CASTERS: [(&str, &[&str]); 4] = [
    ("pyromancer", &["fireball", "fireball", "fire_breath"]),
    ("cryomancer", &["ice_spike", "ice_spike", "frost_nova"]),
    ("necromancer", &["life_drain", "raise_dead", "blood_slash"]),
    ("archmage", &["fireball", "storm_call", "void_rift", "teleport", "frost_nova", "magic_missile", "fangs", "abyss_ray"]),
];

const ALLIES: [&str; 2] = ["risen_skeleton", "spectral_wolf"];

/// A book's contents, saved with the world.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Book {
    pub id: u16,
    /// Spell ids and levels.
    pub spells: Vec<(String, u8)>,
    pub active: usize,
}

struct Proj {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    spell: usize,
    damage: f32,
    /// A monster's spell: hurts the player instead of mobs.
    hostile: bool,
}

#[derive(Clone, Copy)]
enum Later {
    Fang,
    Strike,
    /// A monster's area spell landing where it was marked.
    Blast(f32),
}

struct Pending {
    t: f32,
    pos: Vec3,
    kind: Later,
    damage: f32,
    school: u8,
    hostile: bool,
}

struct Ally {
    /// The mob's `phase`, which tells it apart from the others.
    phase: f32,
    def: usize,
    until: f32,
}

/// Casting state (not saved: buffs and spells in flight end with the session).
#[derive(Default)]
pub struct Spellbooks {
    /// Mana the robes add on top of `Magic::max_mana` right now.
    pub mana_bonus: f32,
    cooldowns: Vec<f32>,
    projs: Vec<Proj>,
    later: Vec<Pending>,
    allies: Vec<Ally>,
    shield: f32,
    ward_t: f32,
    regen_t: f32,
    regen_rate: f32,
    haste_t: f32,
    hasted: bool,
    last_health: f32,
    /// The use button was let go since the last scroll (one scroll per click).
    scroll_ready: bool,
    allies_hunt: bool,
    /// The Archmage's charged beam: seconds left.
    boss_charge: f32,
    /// Seconds since a book or scroll spell: mana refills slowly for a moment after one, as
    /// it does after a wand (`Magic` keeps its own timer to itself).
    since_cast: f32,
}

fn roman(n: u8) -> &'static str {
    ["I", "II", "III", "IV", "V"][(n.clamp(1, MAX_LEVEL) - 1) as usize]
}

/// Glow colour of a school (bright, the particles add light).
fn color(school: u8) -> [f32; 3] {
    match school {
        0 => [1.6, 0.55, 0.15],
        1 => [0.5, 0.85, 1.6],
        2 => [1.6, 1.3, 0.35],
        3 => [0.55, 0.65, 1.7],
        4 => [0.45, 1.5, 0.5],
        5 => [0.8, 0.9, 1.7],
        6 => [1.7, 1.5, 0.8],
        7 => [1.1, 0.35, 1.6],
        8 => [1.4, 0.12, 0.2],
        9 => [0.15, 1.2, 1.1],
        _ => [1.3, 1.1, 0.6],
    }
}

/// Which low voice of the spell sounds a school uses.
fn voice(school: u8) -> u8 {
    match school {
        0..=5 => school,
        6 => 3,
        7 => 5,
        8 => 0,
        9 => 5,
        _ => 4,
    }
}

impl Game {
    fn sb_held_name(&self) -> Option<String> {
        self.held_stack().map(|s| self.items.get(s.item).name.clone())
    }

    fn book_of(&self, st: &ItemStack) -> Option<&Book> {
        let id = st.damage & ID_MASK;
        (id != 0).then(|| self.mods.save.spellbooks.iter().find(|b| b.id == id)).flatten()
    }

    /// The book in the selected slot (giving it a number first when it has none).
    fn held_book_mut(&mut self) -> Option<&mut Book> {
        let sel = self.selected;
        let st = self.surv.inv.slots[sel]?;
        if !is_book(&self.items.get(st.item).name) {
            return None;
        }
        let mut id = st.damage & ID_MASK;
        if id == 0 || !self.mods.save.spellbooks.iter().any(|b| b.id == id) {
            if id == 0 {
                id = self.mods.save.spellbooks.iter().map(|b| b.id).max().unwrap_or(0) + 1;
                if id > ID_MASK {
                    return None;
                }
                self.surv.inv.slots[sel] = Some(ItemStack { damage: (st.damage & !ID_MASK) | id, ..st });
            }
            self.mods.save.spellbooks.push(Book { id, ..Default::default() });
        }
        self.mods.save.spellbooks.iter_mut().find(|b| b.id == id)
    }

    /// The active spell of the book in hand, with its level.
    fn active_spell(&self) -> Option<(usize, u8)> {
        let st = self.held_stack()?;
        if !is_book(&self.items.get(st.item).name) {
            return None;
        }
        let b = self.book_of(&st)?;
        let (id, lvl) = b.spells.get(b.active.min(b.spells.len().saturating_sub(1)))?;
        Some((spell_index(id)?, *lvl))
    }

    /// Spell power for a school: robes of that school (+8% a piece) and the Archmage's tome.
    fn spell_power(&self, school: u8) -> f32 {
        let mut k = 1.0;
        for st in self.surv.armor.iter().flatten() {
            let n = &self.items.get(st.item).name;
            if ROBES.iter().any(|(p, s)| *s == school && n.starts_with(&format!("{p}_"))) {
                k += 0.08;
            }
        }
        if self.sb_held_name().as_deref() == Some("spellbook_archmage") {
            k += 0.15;
        }
        k
    }

    fn robe_mana(&self) -> f32 {
        let mut m = 0.0;
        for (slot, st) in self.surv.armor.iter().enumerate() {
            let Some(st) = st else { continue };
            let n = &self.items.get(st.item).name;
            if ROBES.iter().any(|(p, _)| n.starts_with(&format!("{p}_"))) {
                m += ROBE_MANA[slot.min(3)];
            }
        }
        m
    }

    /// How many of an item the player has.
    fn count_item(&self, name: &str) -> u32 {
        self.items.id(name).map(|id| self.surv.inv.count_of(id)).unwrap_or(0)
    }

    /// Take `n` of an item (plain stacks), if there are that many.
    fn take_item(&mut self, name: &str, n: u32) -> bool {
        if !self.survival {
            return true;
        }
        let Some(id) = self.items.id(name) else { return false };
        if self.surv.inv.count_of(id) < n {
            return false;
        }
        let mut left = n;
        for k in 0..self.surv.inv.slots.len() {
            if left == 0 {
                break;
            }
            if let Some(s) = self.surv.inv.slots[k] {
                if s.item == id {
                    let t = (s.count as u32).min(left);
                    self.surv.inv.take(k, t as u8);
                    left -= t;
                }
            }
        }
        true
    }

    fn give(&mut self, st: ItemStack) {
        if let Some(left) = self.surv.inv.add(&self.items, st) {
            let at = self.body.eye();
            self.pop_drop(at, left);
        }
    }

    fn scroll_stack(&self, spell: usize, level: u8) -> Option<ItemStack> {
        let id = self.items.id(&format!("scroll_{}", SPELLS[spell].id))?;
        Some(ItemStack { damage: if level <= 1 { 0 } else { level as u16 }, ..ItemStack::new(id, 1) })
    }

    // -----------------------------------------------------------------------------------------
    // Every frame.

    pub(crate) fn update_spellbooks(&mut self, dt: f32) {
        // Robes: the extra mana follows what is worn.
        let want = self.robe_mana();
        let sb = &mut self.mods.spellbooks;
        if (want - sb.mana_bonus).abs() > 0.01 {
            self.magic.max_mana += want - sb.mana_bonus;
            sb.mana_bonus = want;
            self.magic.mana = self.magic.mana.min(self.magic.max_mana);
        }
        if sb.cooldowns.len() != SPELLS.len() {
            sb.cooldowns = vec![0.0; SPELLS.len()];
            sb.last_health = self.health;
        }
        if !sb.allies_hunt {
            sb.allies_hunt = true;
            self.allies_learn_prey();
        }
        if dt <= 0.0 {
            return;
        }
        let use_held = (self.mouse_held[1] || self.pad_held.contains(&crate::controller::Action::Use)) && self.screen == crate::Screen::Playing;
        let sb = &mut self.mods.spellbooks;
        let haste = if sb.haste_t > 0.0 { 1.6 } else { 1.0 };
        for c in sb.cooldowns.iter_mut() {
            *c -= dt * haste;
        }
        if !use_held {
            sb.scroll_ready = true;
        }
        sb.since_cast += dt;
        if sb.since_cast < 1.2 && self.magic.mana < self.magic.max_mana {
            // update_magic just gave the quick refill: take back all but the slow one.
            self.magic.mana = (self.magic.mana - 3.4 * dt).max(0.0);
        }
        self.update_buffs(dt);
        if use_held && self.screen == crate::Screen::Playing {
            if let Some((spell, level)) = self.active_spell() {
                self.cast(spell, level, false);
            }
        }
        self.update_projs(dt);
        self.update_later(dt);
        self.update_allies();
        self.update_casters(dt);
    }

    fn update_buffs(&mut self, dt: f32) {
        let dead = self.screen == crate::Screen::Dead;
        let sb = &mut self.mods.spellbooks;
        // Shields and wards give back part of what was just lost.
        let lost = sb.last_health - self.health;
        if lost > 0.0 && !dead && self.health > 0.0 {
            let soak = lost.min(sb.shield);
            sb.shield -= soak;
            let ward = if sb.ward_t > 0.0 { (lost - soak) * 0.5 } else { 0.0 };
            self.health = (self.health + soak + ward).min(self.max_health);
        }
        sb.ward_t = (sb.ward_t - dt).max(0.0);
        if sb.regen_t > 0.0 {
            sb.regen_t -= dt;
            if !dead {
                self.health = (self.health + sb.regen_rate * dt).min(self.max_health);
            }
        }
        sb.haste_t = (sb.haste_t - dt).max(0.0);
        if sb.haste_t > 0.0 {
            self.body.speed_override = Some(voxel::physics::WALK_SPEED * 1.35);
            sb.hasted = true;
        } else if sb.hasted {
            sb.hasted = false;
            self.body.speed_override = None;
        }
        sb.last_health = self.health;
        // A faint shimmer while a shield or ward holds.
        let sb = &self.mods.spellbooks;
        if (sb.shield > 0.0 || sb.ward_t > 0.0) && self.surv.rng.f32() < dt * 10.0 {
            let layer = self.glow_layer();
            let a = self.surv.rng.f32() * std::f32::consts::TAU;
            let at = self.body.pos + Vec3::new(a.cos() * 0.6, self.surv.rng.f32() * 1.8, a.sin() * 0.6);
            let c = if sb.shield > 0.0 { color(6) } else { color(3) };
            self.particles.glow(at, Vec3::Y * 0.3, layer, c, 0.05, 0.5);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Using things: books, scrolls, the table, the seal.

    /// Right click. True when one of ours was used.
    pub(crate) fn spellbooks_use(&mut self) -> bool {
        let looking_at_table = self.target.is_some_and(|h| self.reg.get(self.world.block_or_air(h.block)).name == "inscription_table");
        let held = self.sb_held_name();
        if looking_at_table {
            return self.table_use(held.as_deref());
        }
        let Some(item) = held else { return false };
        if is_book(&item) {
            if self.active_spell().is_none() {
                self.toast("Livro vazio: escreva pergaminhos nele na Mesa de Inscrição", 3.0);
            }
            // Casting happens while the button is held (update_spellbooks).
            return true;
        }
        if let Some(spell) = scroll_spell(&item) {
            if !self.mods.spellbooks.scroll_ready {
                return true;
            }
            self.mods.spellbooks.scroll_ready = false;
            let st = self.held_stack().unwrap();
            let level = (st.damage & ID_MASK).max(1) as u8;
            if self.cast(spell, level, true) && self.survival {
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
            }
            return true;
        }
        if item == "corrupted_seal" {
            return self.summon_archmage();
        }
        if item == "mana_crystal" && self.mods.spellbooks.mana_bonus > 0.0 {
            // The crystal counts stars without the robes' extra ones.
            let bonus = self.mods.spellbooks.mana_bonus;
            self.magic.max_mana -= bonus;
            let used = self.use_mana_crystal();
            self.magic.max_mana += bonus;
            return used;
        }
        false
    }

    /// V picks the next spell in the book in hand (Shift+V the one before).
    pub(crate) fn spellbooks_key(&mut self, code: KeyCode) -> bool {
        if code != KeyCode::KeyV || self.screen != crate::Screen::Playing {
            return false;
        }
        let back = self.keys.contains(&KeyCode::ShiftLeft) || self.keys.contains(&KeyCode::ShiftRight);
        let Some(b) = self.held_book_mut() else { return false };
        let n = b.spells.len();
        if n == 0 {
            self.toast("Livro vazio", 1.5);
            return true;
        }
        b.active = if back { (b.active + n - 1) % n } else { (b.active + 1) % n };
        let (id, lvl) = b.spells[b.active].clone();
        if let Some(i) = spell_index(&id) {
            self.toast(format!("{} {}", SPELLS[i].name, roman(lvl)), 1.5);
        }
        self.sfx(Sfx::Click);
        true
    }

    /// The inscription table: write a scroll into a book, join two scrolls into a stronger
    /// one, take the picked spell back out of a book, or bind a book into a bigger one.
    fn table_use(&mut self, held: Option<&str>) -> bool {
        let sneak = self.crouch > 0.5;
        let Some(item) = held else {
            self.toast("Segure um pergaminho para escrevê-lo num livro, ou um livro de feitiços", 3.5);
            return true;
        };
        if let Some(spell) = scroll_spell(item) {
            let st = self.held_stack().unwrap();
            let level = (st.damage & ID_MASK).max(1) as u8;
            if sneak {
                return self.upgrade_scroll(spell, level);
            }
            return self.inscribe(spell, level);
        }
        if is_book(item) {
            if sneak {
                return self.bind_book(item);
            }
            let sel = self.selected;
            let Some(b) = self.held_book_mut() else { return true };
            if b.spells.is_empty() {
                self.toast("Este livro está vazio", 2.0);
                return true;
            }
            let k = b.active.min(b.spells.len() - 1);
            let (id, lvl) = b.spells.remove(k);
            b.active = b.active.min(b.spells.len().saturating_sub(1));
            let _ = sel;
            if let Some(i) = spell_index(&id) {
                if let Some(s) = self.scroll_stack(i, lvl) {
                    self.give(s);
                }
                self.toast(format!("{} {} voltou a ser um pergaminho", SPELLS[i].name, roman(lvl)), 3.0);
            }
            self.sfx(Sfx::Pickup);
            return true;
        }
        self.toast("A mesa só aceita pergaminhos e livros de feitiços", 2.5);
        true
    }

    /// Write the held scroll into the first book (hotbar, then the rest) with room for it.
    fn inscribe(&mut self, spell: usize, level: u8) -> bool {
        let id = SPELLS[spell].id;
        let books: Vec<usize> = (0..self.surv.inv.slots.len()).filter(|&k| self.surv.inv.slots[k].is_some_and(|s| is_book(&self.items.get(s.item).name))).collect();
        if books.is_empty() {
            self.toast("Você precisa de um livro de feitiços no inventário", 3.0);
            return true;
        }
        if !self.take_item_check("arcane_ink", 1) {
            self.toast("Escrever um feitiço gasta 1 Tinta Arcana", 3.0);
            return true;
        }
        let sel = self.selected;
        for k in books {
            let st = self.surv.inv.slots[k].unwrap();
            let slots = book_slots(&self.items.get(st.item).name);
            // Borrow the book through the selected slot so it gets a number if it needs one.
            let held = self.surv.inv.slots[sel];
            self.surv.inv.slots[sel] = Some(st);
            let done = {
                let b = self.held_book_mut().unwrap();
                if let Some(e) = b.spells.iter_mut().find(|e| e.0 == id) {
                    if e.1 >= level {
                        None
                    } else {
                        e.1 = level;
                        Some(true)
                    }
                } else if b.spells.len() < slots {
                    b.spells.push((id.to_string(), level));
                    Some(false)
                } else {
                    None
                }
            };
            let book = self.surv.inv.slots[sel];
            self.surv.inv.slots[sel] = held;
            self.surv.inv.slots[k] = book;
            if let Some(upgraded) = done {
                self.take_item("arcane_ink", 1);
                if self.survival {
                    self.surv.inv.take(sel, 1);
                }
                let name = SPELLS[spell].name;
                let what = if upgraded { "subiu para" } else { "escrito no livro," };
                self.toast(format!("{name} {what} nível {}", roman(level)), 3.0);
                self.table_glow(SPELLS[spell].school);
                return true;
            }
        }
        self.toast("Nenhum livro tem espaço (ou já sabe este feitiço nesse nível)", 3.0);
        true
    }

    fn take_item_check(&self, name: &str, n: u32) -> bool {
        !self.survival || self.count_item(name) >= n
    }

    /// Two scrolls of one spell and level, and ink, make one of the next level.
    fn upgrade_scroll(&mut self, spell: usize, level: u8) -> bool {
        if level >= MAX_LEVEL {
            self.toast("Este pergaminho já está no nível máximo", 2.5);
            return true;
        }
        let Some(item) = self.items.id(&format!("scroll_{}", SPELLS[spell].id)) else { return true };
        let want = if level <= 1 { 0 } else { level as u16 };
        let same: Vec<usize> = (0..self.surv.inv.slots.len()).filter(|&k| self.surv.inv.slots[k].is_some_and(|s| s.item == item && s.damage & ID_MASK == want)).collect();
        let have: u32 = same.iter().map(|&k| self.surv.inv.slots[k].unwrap().count as u32).sum();
        let ink = level as u32;
        if self.survival && have < 2 {
            self.toast(format!("Junte dois pergaminhos de {} {} para subir de nível", SPELLS[spell].name, roman(level)), 3.0);
            return true;
        }
        if !self.take_item_check("arcane_ink", ink) {
            self.toast(format!("Subir para o nível {} gasta {ink} Tinta Arcana", roman(level + 1)), 3.0);
            return true;
        }
        self.take_item("arcane_ink", ink);
        if self.survival {
            let mut left = 2;
            for k in same {
                while left > 0 && self.surv.inv.slots[k].is_some() {
                    self.surv.inv.take(k, 1);
                    left -= 1;
                }
            }
        }
        if let Some(s) = self.scroll_stack(spell, level + 1) {
            self.give(s);
        }
        self.toast(format!("{} {}!", SPELLS[spell].name, roman(level + 1)), 3.0);
        self.table_glow(SPELLS[spell].school);
        true
    }

    /// Bind the book in hand into the next kind, keeping its spells.
    fn bind_book(&mut self, item: &str) -> bool {
        let (next, cost): (&str, &[(&str, u32)]) = match item {
            "spellbook_apprentice" => ("spellbook_iron", &[("iron_ingot", 4), ("arcane_ink", 2)]),
            "spellbook_iron" => ("spellbook_diamond", &[("diamond", 2), ("arcane_ink", 4)]),
            _ => {
                self.toast("Este livro não pode ser encadernado de novo", 2.5);
                return true;
            }
        };
        if cost.iter().any(|(n, k)| !self.take_item_check(n, *k)) {
            let names: Vec<String> = cost.iter().map(|(n, k)| format!("{k} {}", self.items.id(n).map(|i| self.items.get(i).display_name.clone()).unwrap_or_default())).collect();
            self.toast(format!("Encadernar gasta {}", names.join(" e ")), 3.5);
            return true;
        }
        for (n, k) in cost {
            self.take_item(n, *k);
        }
        let sel = self.selected;
        let Some(st) = self.surv.inv.slots[sel] else { return true };
        let Some(id) = self.items.id(next) else { return true };
        self.surv.inv.slots[sel] = Some(ItemStack { item: id, ..st });
        self.toast(format!("Agora é um {} ({} espaços)", self.items.get(id).display_name, book_slots(next)), 3.0);
        self.table_glow(3);
        true
    }

    fn table_glow(&mut self, school: u8) {
        let Some(hit) = self.target else { return };
        let layer = self.glow_layer();
        let top = hit.block.as_vec3() + Vec3::new(0.5, 1.05, 0.5);
        for _ in 0..24 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() * 1.5, self.surv.rng.f32() - 0.5) * 1.5;
            self.particles.glow(top, v, layer, color(school), 0.07, 0.8);
        }
        self.sfx(Sfx::Cast(voice(school)));
    }

    // -----------------------------------------------------------------------------------------
    // Casting.

    /// Cast a spell at a level. Scrolls are free; books spend mana and wait out the cooldown.
    fn cast(&mut self, spell: usize, level: u8, scroll: bool) -> bool {
        let sp = &SPELLS[spell];
        if !scroll && self.mods.spellbooks.cooldowns.get(spell).is_some_and(|&c| c > 0.0) {
            return false;
        }
        let free = scroll || !self.survival;
        let cost = sp.mana(level);
        if !free && self.magic.mana < cost {
            return false;
        }
        if sp.id == "blood_slash" && self.survival && self.health <= 2.0 {
            self.toast("Sangue de menos para este feitiço", 1.5);
            return false;
        }
        if !free {
            self.magic.mana -= cost;
        }
        if let Some(c) = self.mods.spellbooks.cooldowns.get_mut(spell) {
            *c = sp.cooldown;
        }
        self.mods.spellbooks.since_cast = 0.0;
        let power = self.spell_power(sp.school);
        let dmg = sp.amount(level) * power;
        let eye = self.body.eye();
        let fwd = self.forward();
        let right = fwd.cross(Vec3::Y).normalize_or_zero();
        let from = eye + fwd * 0.6 + right * 0.25 - Vec3::Y * 0.15;
        let layer = self.glow_layer();
        let col = color(sp.school);
        match sp.form {
            Form::Bolt { speed, homing, .. } => {
                let n = if homing && sp.id == "magic_missile" { 1 + (level as usize - 1) / 2 } else { 1 };
                for k in 0..n {
                    let a = (k as f32 - (n - 1) as f32 / 2.0) * 0.15;
                    let dir = (fwd * a.cos() + right * a.sin()).normalize_or_zero();
                    self.mods.spellbooks.projs.push(Proj { pos: from, vel: dir * speed, life: 3.0, spell, damage: dmg, hostile: false });
                }
            }
            Form::Beam { range, through } => {
                let to = self.beam(eye, fwd, range, through, dmg, sp.school, spell, false);
                self.beam_glow(from, to, col);
                let end = self.spell_aim(range);
                if end.distance(eye) < range - 0.5 {
                    self.element_hit(end, crate::elements::of_school(sp.school), 1.0, 0.5);
                }
            }
            Form::Cone { range, push } => {
                let hits: Vec<usize> = self.targets(|c| {
                    let d = c - eye;
                    d.length() < range && d.normalize_or_zero().dot(fwd) > 0.6
                });
                for k in hits {
                    self.spell_hit_mob(k, dmg, eye, spell);
                    let m = &mut self.mobs.list[k];
                    let away = (m.body.pos - eye).normalize_or_zero();
                    m.body.vel += Vec3::new(away.x, 0.3, away.z) * push;
                }
                let end = self.spell_aim(range);
                if end.distance(eye) < range - 0.5 {
                    self.element_hit(end, crate::elements::of_school(sp.school), 1.5, if sp.cooldown < 0.5 { 0.3 } else { 1.0 });
                }
                let n = if sp.cooldown < 0.5 { 6 } else { 30 };
                for _ in 0..n {
                    let j = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5) * 0.5;
                    let v = (fwd + j).normalize_or_zero() * range * 1.6;
                    self.particles.glow(from, v, layer, col, 0.12, 0.6);
                }
            }
            Form::Nova { radius, push } => {
                let me = self.body.pos;
                self.element_hit(me + Vec3::Y * 0.2, crate::elements::of_school(sp.school), radius, 2.0);
                self.nova(me, radius, push, dmg, spell, false);
            }
            Form::Area { radius, lift } => {
                let at = self.look_spot(24.0);
                self.element_hit(at, crate::elements::of_school(sp.school), radius, 1.5);
                self.area(at, radius, lift, dmg, spell, false);
            }
            Form::Strikes => {
                let at = self.look_spot(32.0);
                let mut spots: Vec<Vec3> = self.targets(|c| c.distance(at) < 8.0).into_iter().map(|k| self.mobs.list[k].body.pos).collect();
                spots.truncate(1 + level as usize);
                if spots.is_empty() {
                    spots.push(at);
                }
                for (k, p) in spots.into_iter().enumerate() {
                    self.mods.spellbooks.later.push(Pending { t: 0.15 * k as f32, pos: p, kind: Later::Strike, damage: dmg, school: sp.school, hostile: false });
                }
            }
            Form::Fangs => {
                let flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
                let n = 5 + 2 * level as usize;
                for k in 0..n {
                    let p = self.body.pos + flat * (1.5 + k as f32 * 1.1);
                    self.mods.spellbooks.later.push(Pending { t: 0.06 * k as f32, pos: p, kind: Later::Fang, damage: dmg, school: sp.school, hostile: false });
                }
            }
            Form::Summon(mob) => {
                let n = 1 + (level as usize - 1) / 2 + usize::from(mob == "spectral_wolf");
                let until = self.now + sp.amount(level);
                self.summon_allies(mob, n, until);
            }
            Form::Blink => {
                let range = sp.amount(level);
                self.blink(range, col);
            }
            Form::Heal => {
                self.health = (self.health + dmg).min(self.max_health);
                for _ in 0..20 {
                    let v = Vec3::new(self.surv.rng.f32() - 0.5, 1.0, self.surv.rng.f32() - 0.5) * 2.0;
                    self.particles.glow(self.body.pos + Vec3::Y * 0.4, v, layer, col, 0.09, 0.9);
                }
            }
            Form::Buff(b) => {
                let a = sp.amount(level);
                let sb = &mut self.mods.spellbooks;
                match b {
                    Buff::Shield => sb.shield = sb.shield.max(a * power),
                    Buff::Ward => sb.ward_t = a,
                    Buff::Regen => {
                        sb.regen_t = 10.0;
                        sb.regen_rate = a * power / 10.0;
                    }
                    Buff::Haste => sb.haste_t = a,
                }
                for k in 0..24 {
                    let ang = k as f32 / 24.0 * std::f32::consts::TAU;
                    let p = self.body.pos + Vec3::new(ang.cos() * 0.8, 0.1, ang.sin() * 0.8);
                    self.particles.glow(p, Vec3::Y * 1.6, layer, col, 0.08, 0.8);
                }
            }
        }
        if sp.id == "blood_slash" && self.survival {
            // Blood magic is paid partly in blood.
            self.health -= 1.0;
            self.mods.spellbooks.last_health = self.health;
        }
        // Quick spells hum now and then, not on every pulse.
        if sp.cooldown >= 0.5 || self.surv.rng.f32() < 0.3 {
            self.sfx(Sfx::Cast(voice(sp.school)));
        }
        self.anim.start_swing();
        true
    }

    /// The point looked at (first mob or block), up to `range` away.
    fn look_spot(&self, range: f32) -> Vec3 {
        let eye = self.body.eye();
        let fwd = self.forward();
        let mut p = eye;
        let mut t = 0.0;
        while t < range {
            p += fwd * 0.25;
            t += 0.25;
            if self.target_at(p).is_some() || self.world.solid(p.floor().as_ivec3()) {
                return p - fwd * 0.25;
            }
        }
        p
    }

    /// Mobs our spells may hit: alive, not a townsperson, not one of our allies.
    fn spell_target(&self, k: usize) -> bool {
        let m = &self.mobs.list[k];
        let d = &self.mobs.reg.defs[m.def];
        m.alive() && !d.town && !ALLIES.contains(&d.name.as_str())
    }

    fn target_at(&self, p: Vec3) -> Option<usize> {
        (0..self.mobs.list.len()).find(|&k| {
            if !self.spell_target(k) {
                return false;
            }
            let m = &self.mobs.list[k];
            let d = &self.mobs.reg.defs[m.def];
            let b = m.body.pos;
            let r = d.width * 0.5 * d.scale.max(1.0) + 0.15;
            (p.x - b.x).abs() < r && (p.z - b.z).abs() < r && p.y > b.y - 0.1 && p.y < b.y + d.height * d.scale.max(1.0) + 0.1
        })
    }

    /// Mobs whose centre passes the test.
    fn targets(&self, test: impl Fn(Vec3) -> bool) -> Vec<usize> {
        (0..self.mobs.list.len()).filter(|&k| self.spell_target(k) && test(self.mobs.list[k].body.center())).collect()
    }

    /// A spell lands on a mob: damage and the school's touch.
    fn spell_hit_mob(&mut self, k: usize, dmg: f32, from: Vec3, spell: usize) {
        let sp = &SPELLS[spell];
        let def = &self.mobs.reg.defs[self.mobs.list[k].def];
        let undead = def.burns_in_daylight || def.name.contains("skeleton") || def.name.contains("zombie");
        let dmg = if sp.school == 6 && undead { dmg * 2.0 } else { dmg };
        let name = def.display_name.clone();
        // Fire, frost, lightning on the mob (elements.rs): burning, frozen, soaked twice as hurt.
        let dmg = dmg * self.element_mob(k, crate::elements::of_school(sp.school), dmg);
        let m = &mut self.mobs.list[k];
        m.hurt(dmg, from);
        if sp.school == 4 {
            m.chill = m.chill.max(2.5);
        }
        if sp.id == "ender_orb" && m.alive() {
            // Thrown somewhere else nearby.
            let a = (m.phase * 97.0 + m.age).sin() * std::f32::consts::PI;
            m.body.pos += Vec3::new(a.cos() * 5.0, 1.0, a.sin() * 5.0);
        }
        self.last_hit = Some((name, m.health.max(0.0), m.max_health, self.now + 3.0));
        if sp.id == "life_drain" && self.survival {
            self.health = (self.health + dmg * 0.4).min(self.max_health);
        }
    }

    /// A ray from `from`: hurts every mob on it (or the player, for a monster's beam). Returns
    /// where it ended.
    #[allow(clippy::too_many_arguments)]
    fn beam(&mut self, from: Vec3, dir: Vec3, range: f32, through: bool, dmg: f32, school: u8, spell: usize, hostile: bool) -> Vec3 {
        let mut p = from;
        let mut hit: Vec<usize> = Vec::new();
        let mut t = 0.0;
        while t < range {
            p += dir * 0.25;
            t += 0.25;
            if !through && self.world.solid(p.floor().as_ivec3()) {
                break;
            }
            if hostile {
                let d = p - self.body.pos;
                if d.x.abs() < 0.45 && d.z.abs() < 0.45 && d.y > -0.1 && d.y < 1.9 {
                    self.damage(dmg, Some(from));
                    self.sfx(Sfx::SpellHit(voice(school)));
                    break;
                }
                continue;
            }
            if let Some(k) = self.target_at(p) {
                if !hit.contains(&k) {
                    hit.push(k);
                    self.spell_hit_mob(k, dmg, from, spell);
                }
            }
        }
        if !hit.is_empty() {
            self.sfx(Sfx::SpellHit(voice(school)));
        }
        p
    }

    fn beam_glow(&mut self, from: Vec3, to: Vec3, col: [f32; 3]) {
        let layer = self.glow_layer();
        let len = from.distance(to);
        let dir = (to - from).normalize_or_zero();
        let mut t = 0.0;
        while t < len {
            let j = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5) * 0.08;
            self.particles.glow(from + dir * t, j, layer, col, 0.1, 0.3);
            t += 0.2;
        }
    }

    fn nova(&mut self, at: Vec3, radius: f32, push: f32, dmg: f32, spell: usize, hostile: bool) {
        let sp = &SPELLS[spell];
        let layer = self.glow_layer();
        let col = color(sp.school);
        for k in 0..48 {
            let a = k as f32 / 48.0 * std::f32::consts::TAU;
            let dir = Vec3::new(a.cos(), 0.05, a.sin());
            self.particles.glow(at + Vec3::Y * 0.6, dir * radius * 2.2, layer, col, 0.13, 0.45);
        }
        if hostile {
            let d = self.body.pos - at;
            if d.length() < radius {
                self.damage(dmg, Some(at));
                let away = Vec3::new(d.x, 0.0, d.z).normalize_or_zero();
                self.body.vel += away * push * 0.6 + Vec3::Y * 3.0;
                if sp.school == 1 {
                    self.body.vel *= 0.5;
                }
            }
        } else {
            let hits = self.targets(|c| c.distance(at) < radius + 0.5);
            for k in hits {
                self.spell_hit_mob(k, dmg, at, spell);
                let m = &mut self.mobs.list[k];
                let away = Vec3::new(m.body.pos.x - at.x, 0.0, m.body.pos.z - at.z).normalize_or_zero();
                m.body.vel += away * push + Vec3::Y * push * 0.3;
            }
        }
        self.sfx(Sfx::SpellHit(voice(sp.school)));
    }

    fn area(&mut self, at: Vec3, radius: f32, lift: f32, dmg: f32, spell: usize, hostile: bool) {
        let sp = &SPELLS[spell];
        let layer = self.glow_layer();
        let col = color(sp.school);
        for _ in 0..40 {
            let a = self.surv.rng.f32() * std::f32::consts::TAU;
            let r = self.surv.rng.f32().sqrt() * radius;
            let p = at + Vec3::new(a.cos() * r, 0.1, a.sin() * r);
            let up = if lift > 0.0 { 5.0 } else { 2.5 };
            self.particles.glow(p, Vec3::Y * up * (0.5 + self.surv.rng.f32()), layer, col, 0.1, 0.7);
        }
        if hostile {
            if self.body.pos.distance(at) < radius + 0.5 {
                self.damage(dmg, Some(at));
                self.body.vel.y += lift * 0.7;
            }
        } else {
            for k in self.targets(|c| c.distance(at) < radius + 0.8) {
                self.spell_hit_mob(k, dmg, at, spell);
                self.mobs.list[k].body.vel.y += lift;
            }
        }
        self.sfx(Sfx::SpellHit(voice(sp.school)));
    }

    fn blink(&mut self, range: f32, col: [f32; 3]) {
        let mut to = self.look_spot(range);
        let mut k = 0;
        while k < 8 && (self.world.solid(to.floor().as_ivec3()) || self.world.solid((to + Vec3::Y).floor().as_ivec3())) {
            to += Vec3::Y;
            k += 1;
        }
        let layer = self.glow_layer();
        for _ in 0..24 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5) * 3.0;
            self.particles.glow(self.body.pos + Vec3::Y, v, layer, col, 0.1, 0.6);
            self.particles.glow(to + Vec3::Y, v, layer, col, 0.1, 0.6);
        }
        self.body.pos = Vec3::new(to.x, to.y.floor(), to.z);
        self.body.vel = Vec3::ZERO;
        self.fall_peak = self.body.pos.y;
        self.sfx(Sfx::Portal);
    }

    // -----------------------------------------------------------------------------------------
    // Spells in flight and spells that land a moment later.

    fn update_projs(&mut self, dt: f32) {
        if self.mods.spellbooks.projs.is_empty() {
            return;
        }
        let layer = self.glow_layer();
        let mut projs = std::mem::take(&mut self.mods.spellbooks.projs);
        let mut landed: Vec<(Vec3, usize, Option<usize>, f32, bool)> = Vec::new();
        let me = self.body.pos;
        projs.retain_mut(|p| {
            let sp = &SPELLS[p.spell];
            let Form::Bolt { speed, gravity, homing, burst } = sp.form else { return false };
            p.life -= dt;
            if p.life <= 0.0 {
                if burst > 0.0 {
                    landed.push((p.pos, p.spell, None, p.damage, p.hostile));
                }
                return false;
            }
            if homing {
                let aim = if p.hostile {
                    Some(me + Vec3::Y)
                } else {
                    let dir = p.vel.normalize_or_zero();
                    self.targets(|c| c.distance(p.pos) < 14.0 && (c - p.pos).normalize_or_zero().dot(dir) > 0.3)
                        .into_iter()
                        .map(|k| self.mobs.list[k].body.center())
                        .min_by(|a, b| a.distance(p.pos).total_cmp(&b.distance(p.pos)))
                };
                if let Some(t) = aim {
                    let want = (t - p.pos).normalize_or_zero() * speed;
                    p.vel = p.vel.lerp(want, (dt * 4.0).min(1.0));
                }
            }
            p.vel.y -= gravity * dt;
            let step = p.vel * dt;
            let n = (step.length() / 0.25).ceil().max(1.0) as usize;
            for _ in 0..n {
                p.pos += step / n as f32;
                if p.hostile {
                    let d = p.pos - me;
                    if d.x.abs() < 0.45 && d.z.abs() < 0.45 && d.y > -0.1 && d.y < 1.9 {
                        landed.push((p.pos, p.spell, None, p.damage, true));
                        return false;
                    }
                } else if let Some(k) = self.target_at(p.pos) {
                    landed.push((p.pos, p.spell, Some(k), p.damage, false));
                    return false;
                }
                if self.world.solid(p.pos.floor().as_ivec3()) {
                    landed.push((p.pos - step / n as f32, p.spell, None, p.damage, p.hostile));
                    return false;
                }
                // Fire, frost and lightning stop on water and lava, and act on it.
                let el = crate::elements::of_school(SPELLS[p.spell].school);
                let cell = p.pos.floor().as_ivec3();
                let before = (p.pos - step / n as f32).floor().as_ivec3();
                if !p.hostile && matches!(el, El::Fire | El::Frost | El::Shock) && self.world.is_liquid(self.world.block_or_air(cell)) && !self.world.is_liquid(self.world.block_or_air(before)) {
                    landed.push((p.pos, p.spell, None, p.damage, false));
                    return false;
                }
            }
            true
        });
        for p in &projs {
            let sp = &SPELLS[p.spell];
            let big = matches!(sp.form, Form::Bolt { burst, .. } if burst > 0.0);
            let (size, count) = if big { (0.17, 3) } else { (0.09, 2) };
            for _ in 0..count {
                self.particles.glow(p.pos, -p.vel * 0.05, layer, color(sp.school), size, 0.35);
            }
        }
        self.mods.spellbooks.projs = projs;
        for (at, spell, mob, dmg, hostile) in landed {
            self.bolt_lands(at, spell, mob, dmg, hostile);
        }
    }

    fn bolt_lands(&mut self, at: Vec3, spell: usize, mob: Option<usize>, dmg: f32, hostile: bool) {
        let sp = &SPELLS[spell];
        let layer = self.glow_layer();
        let burst = match sp.form {
            Form::Bolt { burst, .. } => burst,
            _ => 0.0,
        };
        let el = crate::elements::of_school(sp.school);
        if burst > 0.0 {
            self.element_hit(at, el, burst * 0.8, 2.0);
            self.blast_world(at, burst);
        } else {
            self.element_hit(at, el, 1.3, 1.0);
        }
        if burst > 0.0 {
            if hostile {
                let d = self.body.center().distance(at);
                if d < burst {
                    self.damage(dmg * (1.0 - d / burst * 0.5), Some(at));
                }
            } else {
                for k in self.targets(|c| c.distance(at) < burst) {
                    let d = self.mobs.list[k].body.center().distance(at);
                    self.spell_hit_mob(k, dmg * (1.0 - d / burst * 0.5), at, spell);
                }
            }
            for _ in 0..40 {
                let dir = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.3, self.surv.rng.f32() - 0.5).normalize_or_zero();
                self.particles.glow(at, dir * 6.0, layer, color(sp.school), 0.14, 0.6);
            }
            self.sfx(Sfx::Explode);
            return;
        }
        if hostile {
            if self.body.center().distance(at) < 1.6 {
                self.damage(dmg, Some(at));
                if sp.school == 1 {
                    self.body.vel *= 0.4;
                }
            }
        } else if let Some(k) = mob {
            let eye = self.body.eye();
            self.spell_hit_mob(k, dmg, eye, spell);
        }
        for _ in 0..10 {
            let dir = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5);
            self.particles.glow(at, dir * 3.0, layer, color(sp.school), 0.07, 0.4);
        }
        self.sfx(Sfx::SpellHit(voice(sp.school)));
    }

    fn update_later(&mut self, dt: f32) {
        if self.mods.spellbooks.later.is_empty() {
            return;
        }
        let mut due = Vec::new();
        self.mods.spellbooks.later.retain_mut(|p| {
            p.t -= dt;
            if p.t <= 0.0 {
                due.push((p.pos, p.kind, p.damage, p.school, p.hostile));
                return false;
            }
            true
        });
        let layer = self.glow_layer();
        // Marks on the ground where a monster's spell is about to land.
        let marks: Vec<(Vec3, u8, f32)> = self.mods.spellbooks.later.iter().filter(|p| p.hostile).map(|p| (p.pos, p.school, p.t)).collect();
        for (pos, school, t) in marks {
            if self.surv.rng.f32() < dt * 30.0 {
                let a = self.surv.rng.f32() * std::f32::consts::TAU;
                let r = 1.5 + t.min(1.0);
                self.particles.glow(pos + Vec3::new(a.cos() * r, 0.15, a.sin() * r), Vec3::Y * 0.6, layer, color(school), 0.08, 0.4);
            }
        }
        for (pos, kind, dmg, school, hostile) in due {
            let ground = self.ground_under(pos);
            match kind {
                Later::Fang => {
                    for h in 0..6 {
                        let j = Vec3::new(self.surv.rng.f32() - 0.5, 0.0, self.surv.rng.f32() - 0.5) * 0.5;
                        self.particles.glow(ground + j + Vec3::Y * (h as f32 * 0.2), Vec3::Y * 2.5, layer, color(school), 0.12, 0.45);
                    }
                    if hostile {
                        if self.body.pos.distance(ground) < 1.2 {
                            self.damage(dmg, Some(ground));
                        }
                    } else {
                        let spell = spell_index("fangs").unwrap_or(0);
                        for k in self.targets(|c| Vec3::new(c.x - ground.x, 0.0, c.z - ground.z).length() < 1.1 && (c.y - ground.y).abs() < 2.5) {
                            self.spell_hit_mob(k, dmg, ground, spell);
                        }
                    }
                    self.sfx(Sfx::SpellHit(voice(school)));
                }
                Later::Strike => {
                    let mut x = ground;
                    let mut y = 0.0;
                    while y < 14.0 {
                        x += Vec3::new(self.surv.rng.f32() - 0.5, 0.0, self.surv.rng.f32() - 0.5) * 0.35;
                        self.particles.glow(x + Vec3::Y * y, Vec3::ZERO, layer, color(5), 0.13, 0.3);
                        y += 0.3;
                    }
                    if hostile {
                        if self.body.pos.distance(ground) < 2.2 {
                            self.damage(dmg, Some(ground + Vec3::Y * 8.0));
                        }
                    } else {
                        let spell = spell_index("storm_call").unwrap_or(0);
                        for k in self.targets(|c| c.distance(ground) < 2.5) {
                            self.spell_hit_mob(k, dmg, ground + Vec3::Y * 8.0, spell);
                        }
                    }
                    if !hostile {
                        self.element_hit(ground, crate::elements::El::Shock, 2.0, 2.5);
                    }
                    self.weather.flash = self.weather.flash.max(0.6);
                    self.sfx(Sfx::Thunder(1.0));
                }
                Later::Blast(lift) => {
                    let spell = SPELLS.iter().position(|s| s.school == school && matches!(s.form, Form::Area { .. })).unwrap_or(0);
                    self.area(ground, 3.0, lift, dmg, spell, true);
                }
            }
        }
    }

    /// The top of the ground at or below a point (a few blocks down at most).
    fn ground_under(&self, p: Vec3) -> Vec3 {
        let mut q = p.floor().as_ivec3();
        for _ in 0..4 {
            if self.world.solid(q) {
                q.y += 1;
            } else {
                break;
            }
        }
        for _ in 0..6 {
            if self.world.solid(q - glam::IVec3::Y) {
                break;
            }
            q.y -= 1;
        }
        Vec3::new(p.x, q.y as f32, p.z)
    }

    // -----------------------------------------------------------------------------------------
    // Allies.

    /// Summoned allies hunt every monster (their prey list is filled in at start).
    fn allies_learn_prey(&mut self) {
        let hostile: Vec<String> = self.mobs.reg.defs.iter().filter(|d| d.kind == Kind::Hostile).map(|d| d.name.clone()).collect();
        for d in self.mobs.reg.defs.iter_mut() {
            if ALLIES.contains(&d.name.as_str()) {
                d.prey = hostile.clone();
            }
        }
    }

    fn summon_allies(&mut self, mob: &str, n: usize, until: f32) {
        let Some(def) = self.mobs.reg.id(mob) else { return };
        // At most six at a time: the oldest go first.
        while self.mods.spellbooks.allies.len() + n > 6 {
            let a = self.mods.spellbooks.allies.remove(0);
            if let Some(m) = self.mobs.list.iter_mut().find(|m| m.def == a.def && m.phase == a.phase) {
                m.health = 0.0;
            }
        }
        let layer = self.glow_layer();
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        for k in 0..n {
            let off = (k as f32 - (n as f32 - 1.0) / 2.0) * 1.4;
            let at = self.ground_under(self.body.pos + flat * 3.0 + side * off + Vec3::Y);
            if !self.mobs.spawn(mob, at) {
                continue;
            }
            let m = self.mobs.list.last().unwrap();
            self.mods.spellbooks.allies.push(Ally { phase: m.phase, def, until });
            for _ in 0..16 {
                let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() * 2.0, self.surv.rng.f32() - 0.5) * 1.5;
                self.particles.glow(at + Vec3::Y * 0.5, v, layer, color(if mob == "risen_skeleton" { 8 } else { 10 }), 0.1, 0.8);
            }
        }
    }

    fn update_allies(&mut self) {
        if self.mods.spellbooks.allies.is_empty() {
            return;
        }
        let now = self.now;
        let me = self.body.pos;
        let mut allies = std::mem::take(&mut self.mods.spellbooks.allies);
        allies.retain(|a| {
            let Some(m) = self.mobs.list.iter_mut().find(|m| m.def == a.def && m.phase == a.phase && m.alive()) else { return false };
            if now > a.until {
                // Its time is up: it falls apart.
                m.health = 0.0;
                return false;
            }
            m.fed = 0.0;
            m.aggro = false;
            let d = m.body.pos - me;
            if d.length() > 28.0 {
                m.body.pos = me + Vec3::new(1.0, 0.5, 1.0);
            } else if m.prey.is_none() && Vec3::new(d.x, 0.0, d.z).length() > 5.0 {
                m.go_to(me, 1.0);
            }
            true
        });
        self.mods.spellbooks.allies = allies;
    }

    // -----------------------------------------------------------------------------------------
    // Monsters that cast.

    fn update_casters(&mut self, dt: f32) {
        let me = self.body.pos;
        let mut casts: Vec<(usize, usize)> = Vec::new();
        for (k, m) in self.mobs.list.iter().enumerate() {
            if !m.alive() || !m.aggro || !self.hunted() {
                continue;
            }
            let name = self.mobs.reg.defs[m.def].name.as_str();
            let Some(&(_, spells)) = CASTERS.iter().find(|c| c.0 == name) else { continue };
            let dist = m.body.pos.distance(me);
            if dist > 20.0 {
                continue;
            }
            let boss = name == "archmage";
            let angry = m.health < m.max_health * 0.5;
            let period = if boss { if angry { 1.6 } else { 2.4 } } else { 3.2 };
            // Each caster keeps its own beat (its age and phase), so no state is needed.
            let off = m.phase * period;
            let now = ((m.age + off) / period).floor();
            let before = ((m.age + off - dt) / period).floor();
            if now == before {
                continue;
            }
            let turn = now as usize;
            let mut pick = spells[turn % spells.len()];
            // Close in, the fire and frost casters use their short-range spell.
            if !boss && dist < 4.5 {
                pick = spells[spells.len() - 1];
            }
            if boss && pick == "abyss_ray" && !angry {
                pick = "fireball";
            }
            if let Some(i) = spell_index(pick) {
                casts.push((k, i));
            }
        }
        for (k, spell) in casts {
            self.monster_cast(k, spell);
        }
        // The Archmage's charged ray.
        if self.mods.spellbooks.boss_charge > 0.0 {
            self.mods.spellbooks.boss_charge -= dt;
            let Some(k) = self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].name == "archmage") else {
                self.mods.spellbooks.boss_charge = 0.0;
                return;
            };
            let head = self.mobs.list[k].body.pos + Vec3::Y * 3.2;
            let layer = self.glow_layer();
            if self.mods.spellbooks.boss_charge > 0.0 {
                for _ in 0..3 {
                    let j = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5) * 2.0;
                    self.particles.glow(head + j, -j * 1.5, layer, color(9), 0.09, 0.5);
                }
            } else {
                let dir = (self.body.eye() - Vec3::Y * 0.4 - head).normalize_or_zero();
                let i = spell_index("abyss_ray").unwrap_or(0);
                let to = self.beam(head, dir, 30.0, false, 9.0, 9, i, true);
                self.beam_glow(head, to, color(9));
                self.sfx(Sfx::Cast(voice(9)));
            }
        }
    }

    /// A monster casts a spell from the table at the player.
    fn monster_cast(&mut self, k: usize, spell: usize) {
        let sp = &SPELLS[spell];
        let m = &self.mobs.list[k];
        let def = &self.mobs.reg.defs[m.def];
        let boss = def.boss;
        let level = if boss { 4 } else { 2 };
        // Monsters hit less hard than the same spell in the player's hands.
        let dmg = sp.amount(level) * if boss { 0.55 } else { 0.45 };
        let head = m.body.pos + Vec3::Y * (def.height * def.scale.max(1.0) * 0.8);
        let pos = m.body.pos;
        let target = self.body.eye() - Vec3::Y * 0.4;
        let to = (target - head).normalize_or_zero();
        let col = color(sp.school);
        let layer = self.glow_layer();
        match sp.form {
            Form::Bolt { speed, .. } => {
                let n = if boss { 3 } else { 1 };
                let side = to.cross(Vec3::Y).normalize_or_zero();
                for j in 0..n {
                    let a = (j as f32 - (n - 1) as f32 / 2.0) * 0.16;
                    let dir = (to * a.cos() + side * a.sin()).normalize_or_zero();
                    self.mods.spellbooks.projs.push(Proj { pos: head + dir, vel: dir * speed.min(26.0), life: 3.0, spell, damage: dmg, hostile: true });
                }
            }
            Form::Cone { range, .. } => {
                let d = self.body.pos - pos;
                if d.length() < range {
                    self.damage(dmg, Some(pos));
                    if sp.school == 0 {
                        self.body.vel += Vec3::new(d.x, 0.0, d.z).normalize_or_zero() * 2.0;
                    }
                }
                for _ in 0..24 {
                    let j = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5) * 0.5;
                    self.particles.glow(head, (to + j).normalize_or_zero() * range * 1.6, layer, col, 0.12, 0.6);
                }
            }
            Form::Beam { range, .. } => {
                if boss {
                    // Telegraphed: it gathers the dark for a moment first.
                    self.mods.spellbooks.boss_charge = 1.2;
                    self.toast("O Arquimago reúne o abismo!", 1.5);
                    return;
                }
                let end = self.beam(head, to, range, false, dmg, sp.school, spell, true);
                self.beam_glow(head, end, col);
                if sp.id == "life_drain" {
                    let m = &mut self.mobs.list[k];
                    m.health = (m.health + dmg).min(m.max_health);
                }
            }
            Form::Nova { radius, push } => self.nova(pos, radius * 0.8, push, dmg, spell, true),
            Form::Area { lift, .. } => {
                let at = self.body.pos;
                self.mods.spellbooks.later.push(Pending { t: 1.0, pos: at, kind: Later::Blast(lift), damage: dmg, school: sp.school, hostile: true });
            }
            Form::Strikes => {
                let me = self.body.pos;
                for j in 0..if boss { 3 } else { 1 } {
                    let a = j as f32 * 2.1 + self.now;
                    let off = if j == 0 { Vec3::ZERO } else { Vec3::new(a.cos(), 0.0, a.sin()) * 2.5 };
                    self.mods.spellbooks.later.push(Pending { t: 1.0 + j as f32 * 0.3, pos: me + off, kind: Later::Strike, damage: dmg, school: 5, hostile: true });
                }
            }
            Form::Fangs => {
                let flat = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();
                for j in 0..10 {
                    let p = pos + flat * (1.5 + j as f32 * 1.1);
                    self.mods.spellbooks.later.push(Pending { t: 0.4 + 0.07 * j as f32, pos: p, kind: Later::Fang, damage: dmg, school: sp.school, hostile: true });
                }
            }
            Form::Summon(_) => {
                // The dead answer a necromancer (a few at most).
                let near = self.mobs.list.iter().filter(|o| o.alive() && o.body.pos.distance(pos) < 16.0 && matches!(self.mobs.reg.defs[o.def].name.as_str(), "skeleton" | "zombie")).count();
                if near < 3 {
                    for j in 0..2 {
                        let a = j as f32 * 3.1 + self.now;
                        let at = self.ground_under(pos + Vec3::new(a.cos() * 2.0, 1.0, a.sin() * 2.0));
                        self.mobs.spawn(if j == 0 { "zombie" } else { "skeleton" }, at);
                        for _ in 0..12 {
                            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() * 2.0, self.surv.rng.f32() - 0.5);
                            self.particles.glow(at + Vec3::Y * 0.3, v, layer, col, 0.1, 0.7);
                        }
                    }
                }
            }
            Form::Blink => {
                // The Archmage steps to another side of the player.
                let me = self.body.pos;
                let a = self.now * 1.7 + self.surv.rng.f32() * 3.0;
                let to = self.ground_under(me + Vec3::new(a.cos() * 8.0, 2.0, a.sin() * 8.0));
                for _ in 0..24 {
                    let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5) * 3.0;
                    self.particles.glow(pos + Vec3::Y, v, layer, col, 0.12, 0.6);
                    self.particles.glow(to + Vec3::Y, v, layer, col, 0.12, 0.6);
                }
                if !self.world.solid(to.floor().as_ivec3()) && !self.world.solid((to + Vec3::Y).floor().as_ivec3()) {
                    self.mobs.list[k].body.pos = to;
                }
            }
            Form::Heal | Form::Buff(_) => {
                let m = &mut self.mobs.list[k];
                m.health = (m.health + dmg * 2.0).min(m.max_health);
            }
        }
        self.sfx(Sfx::Cast(voice(sp.school)));
    }

    /// Use the seal: the Corrupted Archmage steps out of a rift ahead (one boss at a time).
    fn summon_archmage(&mut self) -> bool {
        if self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].boss) {
            self.toast("Já há um chefe acordado", 3.0);
            return true;
        }
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let at = self.body.pos + flat * 10.0;
        let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
        let y = self.world.surface_y(x, z).map(|y| y + 1).unwrap_or(self.body.pos.y.floor() as i32);
        // Indoors (a tower floor) the surface is the roof: stay on the player's level instead.
        let y = if (y as f32 - self.body.pos.y).abs() > 6.0 { self.body.pos.y.floor() as i32 } else { y };
        let spot = Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5);
        if !self.mobs.spawn("archmage", spot) {
            return false;
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.boss.called_help = false;
        let layer = self.glow_layer();
        for k in 0..60 {
            let a = k as f32 / 60.0 * std::f32::consts::TAU;
            self.particles.glow(spot + Vec3::new(a.cos() * 1.5, 0.2, a.sin() * 1.5), Vec3::Y * 4.0, layer, color(9), 0.14, 1.0);
        }
        self.weather.flash = 0.6;
        self.sfx(Sfx::Thunder(0.6));
        self.toast("O Arquimago Corrompido atravessa a fenda!", 4.0);
        true
    }

    // -----------------------------------------------------------------------------------------
    // What the screen shows.

    pub(crate) fn spellbooks_info_line(&self) -> String {
        let sb = &self.mods.spellbooks;
        format!("Feitiços: {} no ar, {} aliados, mana +{:.0} das vestes", sb.projs.len(), sb.allies.len(), sb.mana_bonus)
    }

    pub(crate) fn draw_spellbooks_hud(&mut self, w: f32, h: f32) {
        let s = self.ui.scale;
        let white = [235, 235, 235, 255];
        let gold = [240, 210, 120, 255];
        // Buffs, low on the left.
        let sb = &self.mods.spellbooks;
        let mut buffs = Vec::new();
        if sb.shield > 0.0 {
            buffs.push(format!("Fortaleza: {:.0}", sb.shield));
        }
        if sb.ward_t > 0.0 {
            buffs.push(format!("Barreira Arcana: {:.0}s", sb.ward_t));
        }
        if sb.regen_t > 0.0 {
            buffs.push(format!("Bênção da Natureza: {:.0}s", sb.regen_t));
        }
        if sb.haste_t > 0.0 {
            buffs.push(format!("Pressa: {:.0}s", sb.haste_t));
        }
        if !sb.allies.is_empty() {
            buffs.push(format!("Aliados: {}", sb.allies.len()));
        }
        for (k, b) in buffs.iter().enumerate() {
            let y = h - (60.0 + 10.0 * k as f32) * s;
            let tw = self.ui.text_width(b, s);
            self.ui.rect(4.0 * s, y - s, tw + 4.0 * s, 10.0 * s, [0, 0, 0, 120]);
            self.ui.text(6.0 * s, y, b, [200, 220, 255, 255]);
        }
        // The table in sight: what it does.
        let at_table = self.target.is_some_and(|hb| self.reg.get(self.world.block_or_air(hb.block)).name == "inscription_table");
        let held = self.held_stack();
        let held_name = held.map(|st| self.items.get(st.item).name.clone());
        if at_table && self.screen == crate::Screen::Playing {
            let lines = [
                "Mesa de Inscrição",
                "Pergaminho: escreve o feitiço num livro (1 tinta)",
                "Agachado + pergaminho: junta dois num nível acima",
                "Livro: tira o feitiço escolhido de volta",
                "Agachado + livro: encaderna num livro maior",
            ];
            let bw = lines.iter().map(|l| self.ui.text_width(l, s)).fold(0.0, f32::max) + 10.0 * s;
            let (x, y) = ((w - bw) / 2.0, 30.0 * s);
            self.ui.rect(x, y, bw, (lines.len() as f32 * 10.0 + 4.0) * s, [0, 0, 0, 150]);
            for (k, l) in lines.iter().enumerate() {
                self.ui.text(x + 5.0 * s, y + (3.0 + 10.0 * k as f32) * s, l, if k == 0 { gold } else { white });
            }
        }
        let Some(name) = held_name else { return };
        let st = held.unwrap();
        if let Some(spell) = scroll_spell(&name) {
            let lvl = (st.damage & ID_MASK).max(1) as u8;
            let sp = &SPELLS[spell];
            let t = format!("{} {} · {} · {} · uso único", sp.name, roman(lvl), SCHOOLS[sp.school as usize], self.spell_effect(spell, lvl));
            let tw = self.ui.text_width(&t, s);
            self.ui.rect((w - tw) / 2.0 - 3.0 * s, h - 79.0 * s, tw + 6.0 * s, 10.0 * s, [0, 0, 0, 130]);
            self.ui.text((w - tw) / 2.0, h - 78.0 * s, &t, gold);
            return;
        }
        if !is_book(&name) {
            return;
        }
        let slots = book_slots(&name);
        let book = self.book_of(&st).cloned().unwrap_or_default();
        let cell = 18.0 * s;
        let x0 = (w - cell * slots as f32) / 2.0;
        let y0 = h - 80.0 * s;
        self.ui.rect(x0 - s, y0 - s, cell * slots as f32 + 2.0 * s, cell + 2.0 * s, [0, 0, 0, 140]);
        for k in 0..slots {
            let x = x0 + k as f32 * cell;
            let active = k == book.active && k < book.spells.len();
            self.ui.frame(x, y0, cell, cell, s, if active { [255, 220, 120, 255] } else { [90, 90, 100, 255] });
            let Some((id, lvl)) = book.spells.get(k) else { continue };
            let Some(i) = spell_index(id) else { continue };
            if let Some(scroll) = self.scroll_stack(i, *lvl) {
                crate::survival::item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack { damage: 0, ..scroll }, x + s, y0 + s, 16.0 * s);
            }
            // Cooling down: the slot darkens from the top.
            let cd = self.mods.spellbooks.cooldowns.get(i).copied().unwrap_or(0.0).max(0.0) / SPELLS[i].cooldown.max(0.01);
            if cd > 0.0 {
                self.ui.rect(x + s, y0 + s, 16.0 * s, 16.0 * s * cd.min(1.0), [0, 0, 0, 140]);
            }
        }
        let text = match book.spells.get(book.active) {
            Some((id, lvl)) => match spell_index(id) {
                Some(i) => {
                    let sp = &SPELLS[i];
                    format!("{} {} · {} · Mana {:.0} · {} · [V] trocar", sp.name, roman(*lvl), SCHOOLS[sp.school as usize], sp.mana(*lvl), self.spell_effect(i, *lvl))
                }
                None => String::new(),
            },
            None => "Livro vazio: escreva pergaminhos nele na Mesa de Inscrição".to_string(),
        };
        let tw = self.ui.text_width(&text, s);
        self.ui.rect((w - tw) / 2.0 - 3.0 * s, y0 - 12.0 * s, tw + 6.0 * s, 10.0 * s, [0, 0, 0, 130]);
        self.ui.text((w - tw) / 2.0, y0 - 11.0 * s, &text, gold);
    }

    /// What a spell does at a level, in a few words.
    fn spell_effect(&self, spell: usize, level: u8) -> String {
        let sp = &SPELLS[spell];
        let a = sp.amount(level);
        let p = self.spell_power(sp.school);
        match sp.form {
            Form::Summon(_) => format!("{:.0}s", a),
            Form::Blink => format!("{:.0} blocos", a),
            Form::Heal => format!("Cura {:.0}", a * p),
            Form::Buff(Buff::Shield) => format!("Escudo {:.0}", a * p),
            Form::Buff(Buff::Regen) => format!("Cura {:.0} em 10s", a * p),
            Form::Buff(_) => format!("{:.0}s", a),
            _ => format!("Dano {:.0}", a * p),
        }
    }

    // -----------------------------------------------------------------------------------------
    // Test pictures (MC_SPELLS=scene).

    pub(crate) fn spellbooks_test_hook(&mut self) {
        let Ok(scene) = std::env::var("MC_SPELLS") else { return };
        self.screen = crate::Screen::Playing;
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let ground = |g: &Game, p: Vec3| -> Vec3 {
            let y = g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
            Vec3::new(p.x, y, p.z)
        };
        let give = |g: &mut Game, slot: usize, name: &str, damage: u16| {
            if let Some(id) = g.items.id(name) {
                g.surv.inv.slots[slot] = Some(ItemStack { damage, ..ItemStack::new(id, 1) });
            }
        };
        let wear = |g: &mut Game, set: &str| {
            for (k, piece) in ["helmet", "chestplate", "leggings", "boots"].iter().enumerate() {
                if let Some(id) = g.items.id(&format!("{set}_{piece}")) {
                    g.surv.armor[k] = Some(ItemStack::new(id, 1));
                }
            }
        };
        let book = |g: &mut Game, slot: usize, item: &str, spells: &[(&str, u8)], active: usize| {
            let id = g.mods.save.spellbooks.iter().map(|b| b.id).max().unwrap_or(0) + 1;
            give(g, slot, item, id);
            g.mods.save.spellbooks.push(Book { id, spells: spells.iter().map(|(s, l)| (s.to_string(), *l)).collect(), active });
        };
        let sel = self.selected;
        let step = |g: &mut Game, n: usize, cast: bool| {
            for k in 0..n {
                g.update_spellbooks_step(0.03, cast && (k % 6 == 0 || k + 1 == n));
                g.particles.update(&g.world, 0.03);
            }
        };
        match scene.as_str() {
            "hud" | "fireball" | "beam" | "fangs" => {
                wear(self, "pyromancer");
                let spells: &[(&str, u8)] = &[("fireball", 3), ("abyss_ray", 2), ("fangs", 4), ("heal", 1), ("teleport", 2), ("summon_wolves", 1), ("frost_nova", 5)];
                let active = match scene.as_str() {
                    "beam" => 1,
                    "fangs" => 2,
                    _ => 0,
                };
                book(self, sel, "spellbook_diamond", spells, active);
                give(self, (sel + 1) % 9, "scroll_storm_call", 3);
                give(self, (sel + 2) % 9, "scroll_heal", 0);
                for k in 0..3 {
                    let at = ground(self, self.body.pos + flat * (9.0 + k as f32 * 2.0) + side * (k as f32 - 1.0) * 2.0);
                    self.mobs.spawn(["zombie", "skeleton", "zombie"][k], at);
                }
                self.update_spellbooks(0.0);
                self.magic.mana = self.magic.max_mana * 0.7;
                if scene != "hud" {
                    step(self, 14, true);
                }
            }
            "casters" | "boss" => {
                let names: &[&str] = if scene == "boss" { &["archmage"] } else { &["pyromancer", "cryomancer", "necromancer"] };
                for (k, n) in names.iter().enumerate() {
                    let off = (k as f32 - (names.len() as f32 - 1.0) / 2.0) * 3.0;
                    let at = ground(self, self.body.pos + flat * if scene == "boss" { 12.0 } else { 7.0 } + side * off);
                    if self.mobs.spawn(n, at) {
                        let m = self.mobs.list.last_mut().unwrap();
                        m.yaw = (-flat.x).atan2(-flat.z);
                        m.aggro = true;
                        if scene == "boss" {
                            m.health = m.max_health * 0.4;
                        }
                    }
                }
                if scene == "boss" {
                    self.boss.called_help = true;
                    // Freeze the moment its ray gathers and a volley is in the air.
                    if let Some(k) = self.mobs.list.iter().position(|m| self.mobs.reg.defs[m.def].name == "archmage") {
                        if let Some(i) = spell_index("fireball") {
                            self.monster_cast(k, i);
                        }
                        if let Some(i) = spell_index("storm_call") {
                            self.monster_cast(k, i);
                        }
                    }
                    self.update_spellbooks(0.0);
                    step(self, 8, false);
                    self.mods.spellbooks.boss_charge = 0.6;
                    step(self, 4, false);
                } else {
                    for k in 0..self.mobs.list.len() {
                        let n = self.mobs.reg.defs[self.mobs.list[k].def].name.clone();
                        let spell = match n.as_str() {
                            "pyromancer" => "fireball",
                            "cryomancer" => "ice_spike",
                            "necromancer" => "life_drain",
                            _ => continue,
                        };
                        if let Some(i) = spell_index(spell) {
                            self.monster_cast(k, i);
                        }
                    }
                    self.update_spellbooks(0.0);
                    step(self, 6, false);
                }
            }
            "summon" => {
                wear(self, "archevoker");
                book(self, sel, "spellbook_iron", &[("summon_wolves", 3), ("raise_dead", 3), ("haste", 2)], 0);
                self.update_spellbooks(0.0);
                if let Some(i) = spell_index("summon_wolves") {
                    self.cast(i, 3, true);
                }
                if let Some(i) = spell_index("raise_dead") {
                    self.cast(i, 3, true);
                }
                if let Some(i) = spell_index("haste") {
                    self.cast(i, 2, true);
                }
                let at = ground(self, self.body.pos + flat * 10.0);
                self.mobs.spawn("zombie", at);
                step(self, 10, false);
            }
            "table" => {
                let at = (self.body.pos + flat * 2.5).floor().as_ivec3();
                let id = self.reg.id_or_air("inscription_table");
                self.world.set_block(at, id);
                give(self, sel, "scroll_fireball", 2);
                give(self, (sel + 1) % 9, "spellbook_apprentice", 0);
                give(self, (sel + 2) % 9, "arcane_ink", 0);
                wear(self, "cultist");
                self.update_spellbooks(0.0);
                // Look down at the table.
                let eye = self.body.eye();
                let to = at.as_vec3() + Vec3::splat(0.5) - eye;
                self.yaw = (-to.x).atan2(-to.z);
                self.pitch = (to.y / to.length()).asin();
            }
            "probe" => {
                let (x, z) = (self.body.pos.x as i32, self.body.pos.z as i32);
                if let Some((o, size)) = self.world.gen.find_structure("torre_arcana", x, z, 40) {
                    let c = o + glam::IVec3::new(size.x / 2, 0, 2);
                    let col: Vec<String> = (0..size.y + 4).map(|y| self.reg.get(self.world.block_or_air(c + glam::IVec3::Y * y)).name.clone()).collect();
                    log::info!("tower at {o} size {size}: {}", col.join(","));
                }
            }
            _ => {}
        }
    }

    /// One step of the casting systems (tests: the main loop gives update_spellbooks dt 0 while
    /// a test picture is taken).
    fn update_spellbooks_step(&mut self, dt: f32, cast: bool) {
        if cast {
            if let Some((spell, level)) = self.active_spell() {
                self.mods.spellbooks.cooldowns.iter_mut().for_each(|c| *c = 0.0);
                let mana = self.magic.mana;
                self.magic.mana = self.magic.max_mana;
                self.cast(spell, level, false);
                self.magic.mana = mana;
            }
        }
        self.update_projs(dt);
        self.update_later(dt);
        self.update_casters(dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEMS: &str = include_str!("../../../../packs/vanilla/mods/items.ron");
    const MOBS: &str = include_str!("../../../../packs/vanilla/mods/dims_mobs.ron");

    #[test]
    fn every_spell_has_a_scroll_and_every_school_spells() {
        for s in &SPELLS {
            assert!(ITEMS.contains(&format!("name: \"scroll_{}\"", s.id)), "{}", s.id);
        }
        for school in 0..SCHOOLS.len() as u8 {
            assert!(SPELLS.iter().any(|s| s.school == school), "{}", SCHOOLS[school as usize]);
        }
        for (p, _) in ROBES {
            for piece in ["helmet", "chestplate", "leggings", "boots"] {
                assert!(ITEMS.contains(&format!("name: \"{p}_{piece}\"")), "{p}_{piece}");
            }
        }
        for (b, _) in BOOKS {
            assert!(ITEMS.contains(&format!("name: \"{b}\"")), "{b}");
        }
        for (m, _) in CASTERS {
            assert!(MOBS.contains(&format!("name: \"{m}\"")), "{m}");
        }
        for m in ALLIES {
            assert!(MOBS.contains(&format!("name: \"{m}\"")), "{m}");
        }
        for s in &SPELLS {
            if let Form::Summon(m) = s.form {
                assert!(ALLIES.contains(&m));
            }
        }
        for (_, spells) in CASTERS {
            for s in spells {
                assert!(spell_index(s).is_some(), "{s}");
            }
        }
    }

    #[test]
    fn higher_levels_hit_harder_and_cost_less() {
        for s in &SPELLS {
            for l in 1..MAX_LEVEL {
                assert!(s.amount(l + 1) >= s.amount(l), "{}", s.id);
                assert!(s.mana(l + 1) < s.mana(l), "{}", s.id);
            }
            assert!(s.mana(MAX_LEVEL) > 0.0);
        }
        // Beams, areas, a summon, a teleport and buffs are all there.
        assert!(SPELLS.iter().any(|s| matches!(s.form, Form::Beam { .. })));
        assert!(SPELLS.iter().any(|s| matches!(s.form, Form::Area { .. } | Form::Nova { .. })));
        assert!(SPELLS.iter().any(|s| matches!(s.form, Form::Summon(_))));
        assert!(SPELLS.iter().any(|s| s.form == Form::Blink));
        assert!(SPELLS.iter().any(|s| matches!(s.form, Form::Buff(_))));
        assert!(SPELLS.len() >= 20);
    }

    #[test]
    fn scroll_names_find_their_spells() {
        assert_eq!(scroll_spell("scroll_fireball"), Some(0));
        assert_eq!(scroll_spell("scroll_nothing"), None);
        assert_eq!(scroll_spell("fireball"), None);
        assert!(is_book("spellbook_iron"));
        assert_eq!(book_slots("spellbook_archmage"), 10);
    }
}
