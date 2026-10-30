//! Progression (Terraria's idea of a long road with a goal at the end): chapters of goals from
//! `packs/vanilla/progression.ron`, checked against what the player has done, with the Guide at
//! the centre. The Guide is there from the first minute of a survival world, moves into the
//! first house, explains the next step when you talk to him and hands over each chapter's reward.
//! A small list at the top left shows what is next; J opens the whole journal.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use glam::{IVec2, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use voxel::item::{ItemId, ItemStack};
use voxel::worldgen::{Biome, Dim};
use winit::keyboard::KeyCode;

use crate::audio::Sfx;
use crate::tr_world::Land;
use crate::ui::{GRAY, WHITE, YELLOW};
use crate::{Game, Screen};

const BUILT_IN: &str = include_str!("../../../packs/vanilla/progression.ron");
const GUIDE: &str = "npc_guide";
const GREEN: [u8; 4] = [130, 220, 130, 255];
const ORANGE: [u8; 4] = [255, 170, 80, 255];
const TEAL: [u8; 4] = [120, 200, 210, 255];

/// The six stages of the game, as balance.rs names them (a test keeps the two the same).
const ERAS: [&str; 6] = ["Pré-chefe", "Início", "Pré-hardmode", "Hardmode", "Pós-mecânicos", "Fim de jogo"];

#[derive(Deserialize)]
struct Book {
    chapters: Vec<Chapter>,
}

#[derive(Deserialize)]
struct Chapter {
    id: String,
    /// Which of the six stages of balance.rs it belongs to.
    #[serde(default)]
    era: u8,
    /// A side path: in the journal and claimable, never the chapter being played.
    #[serde(default)]
    side: bool,
    title: String,
    /// What the Guide says when the chapter opens, and when you bring it to an end.
    intro: String,
    outro: String,
    #[serde(default)]
    reward: Vec<(String, u8)>,
    /// Coins handed over with the reward, in copper.
    #[serde(default)]
    money: u64,
    /// Health and defense the Guide recommends before the chapter's boss.
    #[serde(default)]
    ready: Option<(f32, u32)>,
    goals: Vec<Goal>,
}

#[derive(Deserialize)]
struct Goal {
    id: String,
    text: String,
    /// How to do it, in the Guide's words.
    hint: String,
    need: Need,
    #[serde(default)]
    optional: bool,
    /// The item that wakes the goal's boss: its recipe and where it works go in the journal.
    #[serde(default)]
    summon: Option<String>,
    /// Where the Guide's compass points (Depth, Biome, Structure and Land goals point by themselves).
    #[serde(default)]
    find: Option<Find>,
    /// What the Guide says once it is done.
    #[serde(default)]
    said: Option<String>,
}

/// What a goal asks for (see the top of `progression.ron`).
#[derive(Deserialize, Clone)]
enum Need {
    Item(Vec<String>),
    Boss(String),
    Bosses(Vec<String>, u32),
    Npc(String),
    Town(u32),
    Dim(String),
    Health(f32),
    Mana(f32),
    Defense(u32),
    Depth(i32),
    Biome(Vec<String>),
    Structure(Vec<String>),
    Land(String),
    Kill(String, u32),
    Skill(u32),
    Money(u64),
    Hard,
    Night,
    House,
}

/// Where a goal is done, for the compass.
#[derive(Deserialize, Clone, PartialEq)]
enum Find {
    Structure(Vec<String>),
    Biome(Vec<String>),
    Land(String),
    Down(i32),
}

/// Saved per world in `progress.ron`, next to `level.ron`.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct ProgressSave {
    /// Goals reached, as "chapter/goal".
    done: Vec<String>,
    /// Chapters whose reward the Guide already handed over.
    claimed: Vec<String>,
    kills: HashMap<String, u32>,
    nights: u32,
    /// The Guide lives at the spawn until there is a house for him.
    guide_homeless: bool,
    greeted: bool,
    /// Things the Guide already remarked on (once per world).
    told: Vec<String>,
    /// The goal followed on the screen, as "chapter/goal" (none: the next one of the chapter).
    track: Option<String>,
}

struct Dialog {
    who: String,
    text: String,
    /// Where the speaker stands; walking away ends the talk. None: he calls out from afar.
    at: Option<Vec3>,
    until: f32,
}

/// Where the followed goal is, worked out now and then.
#[derive(Default)]
struct Aim {
    key: String,
    /// The spot to walk to, or how deep to dig.
    to: Option<Vec3>,
    down: Option<i32>,
    from: Vec3,
    t: f32,
    /// A far search found nothing (it tries again after a longer walk).
    missed: bool,
}

#[derive(Default)]
pub struct Progress {
    book: Option<Book>,
    save: ProgressSave,
    /// World the save belongs to (empty until one is loaded).
    world: String,
    /// Per chapter and goal: the items it accepts, and whether the game has what it names.
    items: Vec<Vec<Vec<ItemId>>>,
    known: Vec<Vec<bool>>,
    check_t: f32,
    /// The first check after loading records goals without announcing them.
    quiet: bool,
    /// Saw the night begin without dying since.
    night: bool,
    guide_t: f32,
    /// Which thing the Guide talks about next.
    page: usize,
    dialog: Option<Dialog>,
    pub journal: bool,
    /// Chapter and goal shown in the journal.
    view: usize,
    pick: usize,
    /// "Capítulo 3: ..." across the screen when a chapter opens.
    card: Option<(String, String, f32)>,
    shown: Option<usize>,
    aim: Aim,
    /// Blocks whose "needs a better pickaxe" was said this session.
    gated: HashSet<u16>,
    was_dead: bool,
}

impl Progress {
    pub fn new() -> Progress {
        let text = std::fs::read_to_string(crate::base_dir().join("packs/vanilla/progression.ron")).unwrap_or_else(|_| BUILT_IN.to_string());
        let book = ron::from_str::<Book>(&text).or_else(|e| {
            log::error!("progression.ron: {e}; using the built-in one");
            ron::from_str::<Book>(BUILT_IN)
        });
        Progress { book: book.map_err(|e| log::error!("{e}")).ok(), ..Default::default() }
    }

    fn chapters(&self) -> &[Chapter] {
        self.book.as_ref().map(|b| b.chapters.as_slice()).unwrap_or(&[])
    }

    fn key(&self, c: usize, g: usize) -> String {
        let ch = &self.chapters()[c];
        format!("{}/{}", ch.id, ch.goals[g].id)
    }

    fn done(&self, c: usize, g: usize) -> bool {
        let k = self.key(c, g);
        self.save.done.contains(&k)
    }

    fn known(&self, c: usize, g: usize) -> bool {
        self.known.get(c).and_then(|v| v.get(g)).copied().unwrap_or(false)
    }

    /// The chapter has at least one required goal the game can already offer.
    fn active(&self, c: usize) -> bool {
        self.chapters()[c].goals.iter().enumerate().any(|(g, goal)| !goal.optional && self.known(c, g))
    }

    fn claimed(&self, c: usize) -> bool {
        self.save.claimed.iter().any(|id| *id == self.chapters()[c].id)
    }

    fn complete(&self, c: usize) -> bool {
        self.active(c) && self.chapters()[c].goals.iter().enumerate().all(|(g, goal)| goal.optional || !self.known(c, g) || self.done(c, g))
    }

    /// The chapter being played: the first one of the main road whose reward is still to come.
    fn current(&self) -> Option<usize> {
        (0..self.chapters().len()).find(|&c| !self.chapters()[c].side && self.active(c) && !self.claimed(c))
    }

    /// A side path whose reward waits.
    fn side_ready(&self) -> Option<usize> {
        (0..self.chapters().len()).find(|&c| self.chapters()[c].side && self.complete(c) && !self.claimed(c))
    }

    /// The chapter whose reward the Guide hands over next, if any.
    fn reward_ready(&self) -> Option<usize> {
        self.current().filter(|&c| self.complete(c)).or_else(|| self.side_ready())
    }

    /// The chapter's number on the main road (side paths have none).
    fn number(&self, c: usize) -> usize {
        (0..=c).filter(|&k| !self.chapters()[k].side).count()
    }

    /// Goals still open in a chapter, required ones first.
    fn pending(&self, c: usize) -> Vec<usize> {
        let goals = &self.chapters()[c].goals;
        let open = |g: &usize| self.known(c, *g) && !self.done(c, *g);
        let mut out: Vec<usize> = (0..goals.len()).filter(|g| !goals[*g].optional).filter(open).collect();
        out.extend((0..goals.len()).filter(|g| goals[*g].optional).filter(open));
        out
    }

    /// The goal followed on the screen: the one picked in the journal, or the next of the chapter.
    fn tracked(&self) -> Option<(usize, usize)> {
        if let Some(k) = &self.save.track {
            for c in 0..self.chapters().len() {
                for g in 0..self.chapters()[c].goals.len() {
                    if self.key(c, g) == *k && self.known(c, g) && !self.done(c, g) {
                        return Some((c, g));
                    }
                }
            }
        }
        let c = self.current()?;
        self.pending(c).first().map(|&g| (c, g))
    }

    fn kills(&self, pattern: &str) -> u32 {
        self.save.kills.iter().filter(|(k, _)| matches(pattern, k)).map(|(_, n)| *n).sum()
    }

    fn told(&self, what: &str) -> bool {
        self.save.told.iter().any(|t| t == what)
    }
}

/// Where a goal is done, by its `find` or by what it asks for.
fn find_of(goal: &Goal) -> Option<Find> {
    goal.find.clone().or_else(|| match &goal.need {
        Need::Depth(y) => Some(Find::Down(*y)),
        Need::Biome(b) => Some(Find::Biome(b.clone())),
        Need::Structure(s) => Some(Find::Structure(s.clone())),
        Need::Land(l) => Some(Find::Land(l.clone())),
        _ => None,
    })
}

fn land_ok(want: &str, land: Land) -> bool {
    match want {
        "ruin" => land == Land::Ruin,
        "blight" => land == Land::Blight,
        "prism" => land == Land::Prism,
        _ => land != Land::Prism,
    }
}

fn dim_name(d: Dim) -> &'static str {
    match d {
        Dim::Overworld => "overworld",
        Dim::Nether => "nether",
        Dim::End => "end",
        Dim::Aether => "aether",
        Dim::Otherside => "otherside",
    }
}

fn jungle(b: Biome) -> bool {
    matches!(b, Biome::Jungle | Biome::BambooJungle | Biome::SparseJungle | Biome::MataAtlantica)
}

fn snowy(b: Biome) -> bool {
    matches!(b, Biome::Snowy | Biome::SnowyPeaks | Biome::SnowyTaiga | Biome::IceSpikes | Biome::SnowySlopes | Biome::Grove | Biome::FrozenRiver | Biome::SnowyBeach | Biome::FrozenOcean)
}

/// "norte", "sudeste"... towards (dx, dz) (north is -z).
fn heading(dx: f32, dz: f32) -> &'static str {
    let a = dx.atan2(-dz).to_degrees().rem_euclid(360.0);
    const NAMES: [&str; 8] = ["norte", "nordeste", "leste", "sudeste", "sul", "sudoeste", "oeste", "noroeste"];
    NAMES[((a + 22.5) / 45.0) as usize % 8]
}

/// Names with a * at the start or end match many ("*_log", "cooked_*").
fn matches(pattern: &str, name: &str) -> bool {
    match (pattern.strip_prefix('*'), pattern.strip_suffix('*')) {
        (Some(end), _) if end.ends_with('*') => name.contains(end.trim_end_matches('*')),
        (Some(end), _) => name.ends_with(end),
        (None, Some(start)) => name.starts_with(start),
        _ => name == pattern,
    }
}

/// Break text into lines no wider than `max`.
fn wrap(ui: &crate::ui::UiBatch, text: &str, max: f32, s: f32) -> Vec<String> {
    // Translated whole, before it is cut into lines.
    let text = &*crate::lang::tr(text);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if ui.text_width(&next, s) > max && !line.is_empty() {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

impl Game {
    fn progress_path(&self) -> PathBuf {
        crate::level_path_hint(&self.base, &self.world_name).join("progress.ron")
    }

    /// Load the progress of the world being played, once per world.
    fn progress_sync(&mut self) {
        if self.progress.world == self.world_name {
            return;
        }
        let path = self.progress_path();
        self.progress.save = std::fs::read_to_string(&path).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default();
        self.progress.world = self.world_name.clone();
        self.progress.quiet = true;
        self.progress.dialog = None;
        self.progress.journal = false;
        self.progress.guide_t = 0.0;
        self.progress.card = None;
        self.progress.shown = None;
        self.progress.aim = Aim::default();
        // Which items each goal accepts, and which goals the game can offer yet.
        let mut items = Vec::new();
        let mut known = Vec::new();
        for ch in self.progress.chapters() {
            let mut ci = Vec::new();
            let mut ck = Vec::new();
            for goal in &ch.goals {
                let mob = |p: &str| self.mobs.reg.defs.iter().any(|d| matches(p, &d.name));
                let (ids, ok) = match &goal.need {
                    Need::Item(names) => {
                        let ids: Vec<ItemId> = (0..self.items.items.len()).filter(|&i| names.iter().any(|n| matches(n, &self.items.items[i].name))).map(|i| i as ItemId).collect();
                        let ok = !ids.is_empty();
                        (ids, ok)
                    }
                    Need::Boss(n) | Need::Npc(n) | Need::Kill(n, _) => (Vec::new(), mob(n)),
                    Need::Bosses(names, n) => (Vec::new(), names.iter().filter(|b| mob(b)).count() as u32 >= *n),
                    Need::Dim(d) => (Vec::new(), matches!(d.as_str(), "nether" | "end" | "aether" | "otherside")),
                    _ => (Vec::new(), true),
                };
                ci.push(ids);
                ck.push(ok);
            }
            items.push(ci);
            known.push(ck);
        }
        self.progress.items = items;
        self.progress.known = known;
    }

    pub(crate) fn progress_save(&mut self) {
        if self.progress.world != self.world_name || self.world_name.is_empty() {
            return;
        }
        if let Ok(text) = ron::ser::to_string_pretty(&self.progress.save, ron::ser::PrettyConfig::default()) {
            if let Err(e) = voxel::save::write_atomic(self.progress_path(), text) {
                log::error!("could not save progress: {e}");
            }
        }
    }

    /// The building of one of these names the player stands in.
    fn inside_structure(&self, names: &[String]) -> bool {
        let p = self.body.pos;
        let (x, z) = (p.x.floor() as i32, p.z.floor() as i32);
        names.iter().any(|n| {
            self.world.gen.find_structure(n, x, z, 1).is_some_and(|(o, size)| {
                let lo = o.as_vec3() - Vec3::new(2.0, 4.0, 2.0);
                let hi = (o + size).as_vec3() + Vec3::new(2.0, 4.0, 2.0);
                p.cmpge(lo).all() && p.cmple(hi).all()
            })
        })
    }

    fn need_met(&self, c: usize, g: usize) -> bool {
        let p = &self.progress;
        let over = self.dim == Dim::Overworld;
        let (x, z) = (self.body.pos.x.floor() as i32, self.body.pos.z.floor() as i32);
        match &p.chapters()[c].goals[g].need {
            Need::Item(_) => {
                let ids = &p.items[c][g];
                let surv = &self.surv;
                surv.inv.slots.iter().chain(surv.armor.iter()).chain(std::iter::once(&surv.cursor)).flatten().any(|s| ids.contains(&s.item))
            }
            Need::Boss(n) => self.boss.defeated.iter().any(|b| matches(n, b)),
            Need::Bosses(names, n) => names.iter().filter(|want| self.boss.defeated.iter().any(|b| matches(want, b))).count() as u32 >= *n,
            Need::Npc(n) => self.town.npcs.iter().any(|t| matches(n, &t.name)),
            Need::Town(n) => self.town.npcs.len() as u32 >= *n,
            Need::Dim(d) => dim_name(self.dim) == d,
            Need::Health(n) => self.max_health >= *n,
            Need::Mana(n) => self.magic.max_mana >= *n,
            Need::Defense(n) => self.defense_points() >= *n,
            Need::Depth(y) => over && self.body.pos.y <= *y as f32,
            Need::Biome(names) => over && {
                let b = self.world.gen.biome_at(x, z).name();
                names.iter().any(|n| matches(n, b))
            },
            Need::Structure(names) => self.inside_structure(names),
            Need::Land(want) => self.t5.tr.lands.here.is_some_and(|l| land_ok(want, l)),
            Need::Kill(n, count) => p.kills(n) >= *count,
            Need::Skill(n) => crate::skills::ALL.iter().map(|s| self.skill_level(*s)).max().unwrap_or(0) >= *n,
            Need::Money(v) => self.surv.money >= *v,
            Need::Hard => self.t5.hard.on,
            Need::Night => p.save.nights > 0,
            Need::House => (!p.save.guide_homeless && self.town.npcs.iter().any(|t| t.name == GUIDE)) || !self.town.houses.is_empty(),
        }
    }

    /// Creatures that died this frame count toward "defeat N" goals.
    pub(crate) fn progress_deaths(&mut self, deaths: &[(usize, Vec3)]) {
        if !self.survival {
            return;
        }
        for &(def, _) in deaths {
            let name = self.mobs.reg.defs[def].name.clone();
            *self.progress.save.kills.entry(name).or_default() += 1;
        }
    }

    pub(crate) fn update_progress(&mut self, dt: f32) {
        if !self.world_open || self.screen == Screen::Loading {
            return;
        }
        self.progress_sync();
        if let Some(d) = &self.progress.dialog {
            if self.now > d.until || d.at.is_some_and(|at| at.distance(self.body.pos) > 8.0) {
                self.progress.dialog = None;
            }
        }
        if !self.survival {
            return;
        }
        self.update_aim(dt);
        if dt == 0.0 {
            return;
        }
        let overworld = self.dim == Dim::Overworld;
        // A night counts once it turns to day with the player alive the whole time.
        if self.screen == Screen::Dead {
            self.progress.night = false;
            self.progress.was_dead = true;
        } else if overworld {
            if self.is_night() {
                if !self.progress.night && self.progress.save.nights == 0 && self.town.houses.is_empty() {
                    self.guide_remark("anoitecer", "Está escurecendo! Os monstros vão sair. Feche-se num abrigo com uma tocha, ou cave um buraco na encosta e tampe a entrada.");
                }
                self.progress.night = true;
            } else if self.progress.night {
                self.progress.night = false;
                self.progress.save.nights += 1;
            }
        }
        if self.screen == Screen::Playing {
            if std::mem::take(&mut self.progress.was_dead) {
                self.guide_remark("morte", "Você voltou! O que você carregava ficou num corpo, onde você caiu. Volte lá para pegar tudo de novo.");
            } else if self.health > 0.0 && self.health < self.max_health * 0.35 {
                self.guide_remark("vida_baixa", "Cuidado, você está ferido! Coma algo para a vida voltar devagar, beba uma poção de cura ou fuja para casa.");
            }
        }
        // The Guide is there from the start, standing near the player.
        if overworld && !self.town.npcs.iter().any(|n| n.name == GUIDE) {
            self.progress.guide_t += dt;
            if self.progress.guide_t > 2.0 {
                self.spawn_guide();
            }
        }
        self.progress.check_t -= dt;
        if self.progress.check_t > 0.0 {
            return;
        }
        self.progress.check_t = 0.5;
        if overworld {
            self.guide_move_in();
        }
        let before = self.progress.current();
        let quiet = self.progress.quiet;
        self.progress.quiet = false;
        let mut said = None;
        for c in 0..self.progress.chapters().len() {
            for g in 0..self.progress.chapters()[c].goals.len() {
                if !self.progress.known(c, g) || self.progress.done(c, g) || !self.need_met(c, g) {
                    continue;
                }
                let k = self.progress.key(c, g);
                self.progress.save.done.push(k);
                if quiet {
                    continue;
                }
                let goal = &self.progress.chapters()[c].goals[g];
                if goal.said.is_some() {
                    said = goal.said.clone();
                }
                if Some(c) == before || self.progress.chapters()[c].side {
                    let text = goal.text.clone();
                    self.toast(format!("Objetivo concluído: {text}"), 4.0);
                    self.sfx(Sfx::Xp);
                }
            }
        }
        if let Some(text) = said {
            self.guide_call(text);
        }
        for c in 0..self.progress.chapters().len() {
            let ch = &self.progress.chapters()[c];
            let main = Some(c) == before;
            if quiet || !(main || ch.side) || self.progress.claimed(c) || !self.progress.complete(c) {
                continue;
            }
            let k = format!("{}/*", ch.id);
            if self.progress.save.done.contains(&k) {
                continue;
            }
            let title = ch.title.clone();
            self.progress.save.done.push(k);
            self.toast(format!("Capítulo concluído: {title}! Fale com o Guia."), 7.0);
            self.sfx(Sfx::Melody(0, 2));
        }
        // A new chapter opens: its name across the screen.
        let now = self.progress.current();
        if now != self.progress.shown {
            // (The first check after loading only takes note.)
            if let Some(c) = now.filter(|_| !quiet) {
                self.chapter_card(c);
            }
            self.progress.shown = now;
        }
    }

    fn chapter_card(&mut self, c: usize) {
        let ch = &self.progress.chapters()[c];
        let era = ERAS.get(ch.era as usize).copied().unwrap_or("");
        let title = format!("Capítulo {}: {}", self.progress.number(c), ch.title);
        self.progress.card = Some((title, format!("Era: {era}"), self.now + 6.0));
    }

    /// The Guide remarks on something once per world, from wherever he is.
    fn guide_remark(&mut self, what: &str, text: &str) {
        if self.progress.told(what) {
            return;
        }
        self.progress.save.told.push(what.to_string());
        self.guide_call(text.to_string());
    }

    /// The Guide's words in the dialog box, without having to walk up to him.
    fn guide_call(&mut self, text: String) {
        let who = self.guide_name();
        self.progress.dialog = Some(Dialog { who, text, at: None, until: self.now + 12.0 });
        self.sfx(Sfx::Click);
    }

    /// Bring the Guide in next to the player, living at the spawn until a house is ready.
    fn spawn_guide(&mut self) {
        self.progress.guide_t = 0.0;
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let at = self.body.pos + flat * 3.0;
        let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
        let y = self.world.surface_y(x, z).map(|y| y as f32 + 1.0).unwrap_or(self.body.pos.y);
        let home = Vec3::new(x as f32 + 0.5, y, z as f32 + 0.5);
        if !self.mobs.spawn(GUIDE, home) {
            return;
        }
        if let Some(m) = self.mobs.list.last_mut() {
            m.home = Some(home);
            m.fed = 0.0;
            let to = self.body.pos - home;
            m.yaw = to.x.atan2(to.z);
        }
        self.town.npcs.push(crate::npcs::NpcSave { name: GUIDE.to_string(), home: home.to_array() });
        self.progress.save.guide_homeless = true;
        if !self.progress.save.greeted {
            self.progress.save.greeted = true;
            let intro = self.progress.current().map(|c| self.progress.chapters()[c].intro.clone()).unwrap_or_default();
            self.say(&self.guide_name(), intro, home);
            if let Some(c) = self.progress.current() {
                self.chapter_card(c);
            }
        }
    }

    /// The Guide takes the first free house.
    fn guide_move_in(&mut self) {
        if !self.progress.save.guide_homeless {
            return;
        }
        let taken = |spot: Vec3, me: &Game| me.town.npcs.iter().any(|n| n.name != GUIDE && Vec3::from(n.home).distance(spot) < 1.5);
        let Some(spot) = self.town.houses.iter().map(|h| h.1).find(|s| !taken(*s, self)) else { return };
        if let Some(n) = self.town.npcs.iter_mut().find(|n| n.name == GUIDE) {
            n.home = spot.to_array();
        }
        if let Some(m) = self.mobs.list.iter_mut().find(|m| m.alive() && self.mobs.reg.defs[m.def].name == GUIDE) {
            m.home = Some(spot);
            m.go_to(spot, 2.0);
        }
        self.progress.save.guide_homeless = false;
        self.toast(format!("{} se mudou para a sua casa!", self.guide_name()), 4.0);
    }

    fn guide_name(&self) -> String {
        self.mobs.reg.id(GUIDE).map(|i| self.mobs.reg.defs[i].display_name.clone()).unwrap_or_else(|| "Guia".into())
    }

    fn say(&mut self, who: &str, text: String, at: Vec3) {
        self.progress.dialog = Some(Dialog { who: who.to_string(), text, at: Some(at), until: self.now + 16.0 });
    }

    /// Right click on the Guide: hand over a finished chapter's reward, or explain what is next.
    pub(crate) fn guide_talk(&mut self, at: Vec3, who: &str) {
        self.progress_sync();
        let tips = crate::npcs::GUIDE_TIPS;
        if !self.survival {
            let tip = tips[self.progress.page % tips.len()];
            self.progress.page += 1;
            self.say(who, tip.to_string(), at);
            return;
        }
        if let Some(c) = self.progress.reward_ready() {
            let mut got = Vec::new();
            for (item, n) in self.progress.chapters()[c].reward.clone() {
                let Some(id) = self.items.id(&item) else { continue };
                got.push(format!("{n} {}", self.items.get(id).display_name));
                if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(id, n)) {
                    self.pop_drop(self.body.pos + Vec3::Y, left);
                }
            }
            let money = self.progress.chapters()[c].money;
            if money > 0 {
                self.earn(money);
                got.push(crate::money::text(money));
            }
            let ch = &self.progress.chapters()[c];
            let mut text = ch.outro.clone();
            if !got.is_empty() {
                text += &format!(" Tome: {}.", got.join(", "));
            }
            let id = ch.id.clone();
            self.progress.save.claimed.push(id);
            self.progress.page = 0;
            self.say(who, text, at);
            self.sfx(Sfx::Melody(0, 3));
            self.sfx(Sfx::Xp);
            return;
        }
        let Some(c) = self.progress.current() else {
            let tip = tips[self.progress.page % tips.len()];
            self.progress.page += 1;
            self.say(who, format!("Você já fez tudo o que eu conheço. Que aventura! {tip}"), at);
            return;
        };
        // What to talk about, in turn: the chapter, the next goals (how, where, what it takes),
        // whether you are ready for its boss, the item in hand, a tip.
        let ch = &self.progress.chapters()[c];
        let mut pages = vec![format!("{}: {}", ch.title, ch.intro)];
        let mut goals: Vec<(usize, usize)> = self.progress.tracked().into_iter().collect();
        for g in self.progress.pending(c) {
            if goals.len() < 2 && !goals.contains(&(c, g)) {
                goals.push((c, g));
            }
        }
        for (gc, g) in goals {
            let goal = &self.progress.chapters()[gc].goals[g];
            let mut text = format!("{}{}: {}", goal.text, self.goal_count(gc, g), goal.hint);
            if let Some(r) = self.goal_recipe(gc, g) {
                text += &format!(" Precisa de: {r}.");
            }
            if let Some(w) = self.goal_where(gc, g) {
                text += &format!(" {w}");
            }
            pages.push(text);
        }
        if let Some(text) = self.ready_text(c) {
            pages.push(text);
        }
        if let Some(text) = self.held_item_uses() {
            pages.push(text);
        }
        pages.push(tips[self.progress.page % tips.len()].to_string());
        let text = pages[self.progress.page % pages.len()].clone();
        self.progress.page += 1;
        self.say(who, text, at);
    }

    /// " (2/5)" on "defeat N" goals.
    fn goal_count(&self, c: usize, g: usize) -> String {
        match &self.progress.chapters()[c].goals[g].need {
            Need::Kill(n, k) => format!(" ({}/{k})", self.progress.kills(n).min(*k)),
            Need::Bosses(names, k) if *k > 1 => {
                let n = names.iter().filter(|w| self.boss.defeated.iter().any(|b| matches(w, b))).count() as u32;
                format!(" ({}/{k})", n.min(*k))
            }
            Need::Town(k) => format!(" ({}/{k})", (self.town.npcs.len() as u32).min(*k)),
            _ => String::new(),
        }
    }

    /// The Guide's word on the chapter's boss: are you strong enough?
    fn ready_text(&self, c: usize) -> Option<String> {
        let (hp, def) = self.progress.chapters()[c].ready?;
        let (have_hp, have_def) = (self.max_health, self.defense_points());
        let hearts = |v: f32| (v / 2.0).round() as u32;
        if have_hp >= hp && have_def >= def {
            return Some(format!("Você parece pronto para o chefe deste capítulo: {} corações e {have_def} de defesa. Leve poções!", hearts(have_hp)));
        }
        Some(format!(
            "Para o chefe deste capítulo eu teria pelo menos {} corações e {def} de defesa. Você tem {} e {have_def}. Cristais de vida e armadura melhor ajudam.",
            hearts(hp),
            hearts(have_hp)
        ))
    }

    /// Whether health and defense reach what the chapter recommends.
    fn ready_ok(&self, c: usize) -> Option<(bool, bool)> {
        let (hp, def) = self.progress.chapters()[c].ready?;
        Some((self.max_health >= hp, self.defense_points() >= def))
    }

    /// What the goal's item (or the item that wakes its boss) is made of, with what you have:
    /// "4 Muda de Carvalho (2/4), 1 Pedregulho com Musgo (0/1) + Bancada".
    fn goal_recipe(&self, c: usize, g: usize) -> Option<String> {
        let goal = &self.progress.chapters()[c].goals[g];
        let names: Vec<&str> = match (&goal.summon, &goal.need) {
            (Some(s), _) => vec![s.as_str()],
            (None, Need::Item(names)) => names.iter().filter(|n| !n.contains('*')).map(|n| n.as_str()).collect(),
            _ => return None,
        };
        let target = names.iter().filter_map(|n| self.items.id(n)).find(|&id| self.recipe_of(id).is_some())?;
        if goal.summon.is_some() && self.carrying_id(target) > 0 {
            return None;
        }
        let (parts, smelt) = self.recipe_of(target)?;
        let mut out: Vec<String> = parts
            .iter()
            .map(|(ing, n)| {
                let have: u32 = ing.iter().map(|&i| self.carrying_id(i)).sum();
                let shown = ing.iter().copied().find(|&i| self.carrying_id(i) > 0).unwrap_or(ing[0]);
                format!("{n} {} ({}/{n})", self.items.get(shown).display_name, have.min(*n))
            })
            .collect();
        if smelt {
            out.push("na fornalha".into());
        }
        let mut text = format!("{} = {}", self.items.get(target).display_name, out.join(", "));
        text += &self.station_label(target);
        Some(text)
    }

    /// The first recipe that makes `id`: its ingredients with counts, and whether it is smelted.
    fn recipe_of(&self, id: ItemId) -> Option<(Vec<(Vec<ItemId>, u32)>, bool)> {
        let r = &self.recipes;
        let group = |cells: &mut dyn Iterator<Item = &Vec<ItemId>>| {
            let mut out: Vec<(Vec<ItemId>, u32)> = Vec::new();
            for ing in cells {
                if ing.is_empty() {
                    continue;
                }
                match out.iter_mut().find(|(i, _)| i == ing) {
                    Some(e) => e.1 += 1,
                    None => out.push((ing.clone(), 1)),
                }
            }
            out
        };
        // Metals and gems come out of the furnace; that beats nine nuggets.
        if let Some(s) = r.smelting.iter().find(|s| s.result.item == id) {
            return Some((vec![(s.input.clone(), 1)], true));
        }
        // Unpacking a storage block ("1 block of diamond = 9 diamonds") is no way to get one.
        let unpack = |parts: &Vec<(Vec<ItemId>, u32)>, count: u8| count > 1 && parts.len() == 1 && parts[0].1 == 1;
        for s in r.shaped.iter().filter(|s| s.result.item == id) {
            let parts = group(&mut s.cells.iter().flatten());
            if !unpack(&parts, s.result.count) {
                return Some((parts, false));
            }
        }
        for s in r.shapeless.iter().filter(|s| s.result.item == id) {
            let parts = group(&mut s.ingredients.iter());
            if !unpack(&parts, s.result.count) {
                return Some((parts, false));
            }
        }
        None
    }

    fn carrying_id(&self, id: ItemId) -> u32 {
        self.surv.inv.slots.iter().flatten().filter(|s| s.item == id).map(|s| s.count as u32).sum()
    }

    /// Where the goal is done, in the Guide's words ("Fica a 340 blocos daqui, para o norte.").
    fn goal_where(&self, c: usize, g: usize) -> Option<String> {
        let goal = &self.progress.chapters()[c].goals[g];
        let mut out = String::new();
        if let Some(item) = &goal.summon {
            if let Some((rule, _)) = self.summon_where(item) {
                out += &format!("Use {rule}. ");
            }
        }
        if self.progress.aim.key == self.progress.key(c, g) {
            if let Some(y) = self.progress.aim.down {
                out += &format!("Desça mais {} blocos.", (self.body.pos.y - y as f32).max(1.0) as i32);
            } else if let Some(to) = self.progress.aim.to {
                let d = to - self.body.pos;
                out += &format!("O mais perto fica a {} blocos daqui, para o {}.", Vec2::new(d.x, d.z).length() as i32, heading(d.x, d.z));
            }
        }
        (!out.is_empty()).then(|| out.trim_end().to_string())
    }

    /// Where a summon item works, and whether it would work right here, right now (None: can't tell).
    pub(crate) fn summon_where(&self, item: &str) -> Option<(&'static str, Option<bool>)> {
        let over = self.dim == Dim::Overworld;
        let y = self.body.pos.y;
        let night = self.is_night();
        let hard = self.t5.hard.on;
        let (x, z) = (self.body.pos.x.floor() as i32, self.body.pos.z.floor() as i32);
        let biome = self.world.gen.base_biome_at(x, z);
        let beat = |b: &str| self.boss.defeated.iter().any(|d| d == b);
        let prism = self.t5.tr.lands.here == Some(Land::Prism);
        let r = |text, ok: bool| Some((text, Some(ok)));
        match item {
            "jelly_crown" => r("na superfície", over && y > 30.0),
            "eye_lure" => r("à noite, na superfície", over && night && y > 30.0),
            "blight_spine" => r("abaixo da altura 0", over && y < 0.0),
            "royal_comb" => r("na selva", over && (jungle(biome) || self.ubiome() == voxel::worldgen::deep::UBiome::Jungle)),
            "frost_horn" => r("na neve", over && (snowy(biome) || self.ubiome() == voxel::worldgen::deep::UBiome::Ice)),
            "worm_bait" => r("no subsolo, abaixo da altura 50", over && y < 50.0),
            "cursed_skull" | "ancient_seed" | "lich_phylactery" => r("à noite", over && night),
            "breaker_idol" => r("no Nether", self.dim == Dim::Nether),
            "gear_eye" | "iron_worm" | "steel_skull" => r("à noite, depois do Quebra-Mundos", over && night && hard),
            "queen_bulb" => r("na selva, depois do Quebra-Mundos", over && hard && biome == Biome::Jungle),
            "celestial_sigil" => r("depois da Rainha das Vinhas", beat("vine_queen")),
            "gelatin_crystal" => r("no Prisma, depois do Quebra-Mundos", over && hard && prism),
            "tide_bait" => r("no mar ou na praia", over && matches!(biome, Biome::Ocean | Biome::Beach)),
            "prism_lamp" => r("no Prisma, à noite", over && prism && night),
            "lunar_tablet" => r("na superfície, depois do Ídolo Escamado", over && beat("scale_idol") && y > 30.0),
            "idol_core" => Some(("no Altar Escamado, no Templo Escamado, depois da Rainha das Vinhas", None)),
            "obsidian_idol" => Some(("na cratera de um vulcão", None)),
            "pumpkin_medallion" | "frost_medallion" => r("à noite", over && night),
            "obsidian_heart" | "void_seed" | "gauntlet_core" | "corrupted_seal" => Some(("em qualquer lugar, com espaço para lutar", None)),
            _ => None,
        }
    }

    /// Recompute now and then where the followed goal is.
    fn update_aim(&mut self, dt: f32) {
        let Some((c, g)) = self.progress.tracked() else {
            self.progress.aim = Aim::default();
            return;
        };
        let key = self.progress.key(c, g);
        self.progress.aim.t -= dt;
        // Far searches that found nothing wait for a longer walk before trying again.
        let walked = self.progress.aim.from.distance(self.body.pos);
        let moved = walked > if self.progress.aim.missed { 256.0 } else { 48.0 };
        if self.progress.aim.key == key && self.progress.aim.t > 0.0 && !moved {
            // Close enough to see it: the arrow stays, only the depth is live.
            return;
        }
        let find = find_of(&self.progress.chapters()[c].goals[g]);
        let (to, down) = match &find {
            Some(f) => self.locate(f),
            None => (None, None),
        };
        let missed = to.is_none() && matches!(find, Some(Find::Structure(_) | Find::Biome(_) | Find::Land(_)));
        let t = if missed { 60.0 } else { 8.0 };
        self.progress.aim = Aim { key, to, down, from: self.body.pos, t, missed };
    }

    /// The spot to walk to for `find`, or the height to dig down to; nothing when you are there.
    fn locate(&self, find: &Find) -> (Option<Vec3>, Option<i32>) {
        let p = self.body.pos;
        let (x, z) = (p.x.floor() as i32, p.z.floor() as i32);
        let over = self.dim == Dim::Overworld;
        match find {
            Find::Down(y) => (None, (over && p.y > *y as f32 + 1.0).then_some(*y)),
            Find::Structure(names) => {
                if self.inside_structure(names) {
                    return (None, None);
                }
                let best = names
                    .iter()
                    .filter_map(|n| self.world.gen.find_structure(n, x, z, 14))
                    .map(|(o, size)| o.as_vec3() + size.as_vec3() * 0.5)
                    .min_by(|a, b| a.distance_squared(p).total_cmp(&b.distance_squared(p)));
                (best, None)
            }
            Find::Biome(names) => {
                let ok = |bx: i32, bz: i32| names.iter().any(|n| matches(n, self.world.gen.biome_at(bx, bz).name()));
                if !over || ok(x, z) {
                    return (None, None);
                }
                // Rings outwards until one of those biomes turns up.
                for r in (64..3200).step_by(64) {
                    let n = (r / 20).max(8);
                    for k in 0..n {
                        let a = k as f32 / n as f32 * std::f32::consts::TAU;
                        let (bx, bz) = (x + (a.cos() * r as f32) as i32, z + (a.sin() * r as f32) as i32);
                        if ok(bx, bz) {
                            let y = self.world.gen.height_at(bx, bz) as f32 + 2.0;
                            return (Some(Vec3::new(bx as f32 + 0.5, y, bz as f32 + 0.5)), None);
                        }
                    }
                }
                (None, None)
            }
            Find::Land(want) => {
                if !over || self.t5.tr.lands.here.is_some_and(|l| land_ok(want, l)) {
                    return (None, None);
                }
                let at: Option<IVec2> = crate::tr_world::nearest_land(self.world.gen.seed, x, z, self.t5.hard.on, |l| land_ok(want, l));
                (at.map(|a| Vec3::new(a.x as f32 + 0.5, self.world.gen.height_at(a.x, a.y) as f32 + 2.0, a.y as f32 + 0.5)), None)
            }
        }
    }

    /// Breaking a block that needs a stronger pickaxe: the Guide says which one, once per block.
    pub(crate) fn progress_gate(&mut self, id: voxel::BlockId) {
        if !self.survival || !self.progress.gated.insert(id) {
            return;
        }
        let b = self.reg.get(id);
        if b.tier == 0 {
            return;
        }
        let Some(kind) = b.tool else { return };
        let tier = b.tier;
        let block = b.display_name.clone();
        // The weakest tool of that kind that does it (the classic ones first).
        let classic = ["wooden_pickaxe", "stone_pickaxe", "iron_pickaxe", "diamond_pickaxe", "netherite_pickaxe", "wooden_axe", "stone_axe", "iron_axe", "diamond_axe", "wooden_shovel", "stone_shovel", "iron_shovel", "diamond_shovel"];
        let fits = |i: &voxel::item::Item| i.tool.as_ref().is_some_and(|t| t.kind == kind && t.tier >= tier);
        let tool = classic
            .iter()
            .filter_map(|n| self.items.id(n))
            .map(|i| self.items.get(i))
            .filter(|i| fits(i))
            .min_by_key(|i| i.tool.as_ref().map_or(99, |t| t.tier))
            .or_else(|| self.items.items.iter().filter(|i| fits(i)).min_by_key(|i| i.tool.as_ref().map_or(99, |t| t.tier)));
        let text = match tool {
            Some(t) => format!("{block} não soltou nada: precisa de {} ou melhor.", t.display_name),
            None => format!("{block} não soltou nada: precisa de uma ferramenta mais forte."),
        };
        self.toast(text, 4.0);
    }

    /// What the item in hand is good for (Terraria's guide looks at what you show him).
    fn held_item_uses(&self) -> Option<String> {
        let st = self.surv.inv.slots.get(self.selected).copied().flatten()?;
        let id = st.item;
        let name = &self.items.get(id).display_name;
        if let Some((rule, _)) = self.summon_where(&self.items.get(id).name) {
            return Some(format!("{name} acorda um chefe. Use {rule}. Antes, prepare uma arena, poções e uma cama perto."));
        }
        let r = &self.recipes;
        let mut out: Vec<ItemId> = Vec::new();
        let results = r.shaped.iter().filter(|x| x.cells.iter().flatten().any(|c| c.contains(&id))).map(|x| x.result.item);
        let results = results.chain(r.shapeless.iter().filter(|x| x.ingredients.iter().any(|c| c.contains(&id))).map(|x| x.result.item));
        let results = results.chain(r.smelting.iter().filter(|x| x.input.contains(&id)).map(|x| x.result.item));
        for res in results {
            if !out.contains(&res) {
                out.push(res);
            }
        }
        if out.is_empty() {
            return Some(format!("{name}? Não conheço nenhuma receita com isso. Talvez sirva de outro jeito."));
        }
        let names: Vec<&str> = out.iter().take(5).map(|&i| self.items.get(i).display_name.as_str()).collect();
        let more = if out.len() > 5 { format!(" e mais {}", out.len() - 5) } else { String::new() };
        Some(format!("Com {name} você faz: {}{more}. Veja no livro de receitas (U no inventário).", names.join(", ")))
    }

    /// The journal's left column, top to bottom: era headings, chapters, then the side paths.
    fn journal_rows(&self) -> Vec<(Option<usize>, String)> {
        let chs = self.progress.chapters();
        let mut rows = Vec::new();
        let mut era = None;
        for (c, ch) in chs.iter().enumerate().filter(|(_, ch)| !ch.side) {
            if era != Some(ch.era) {
                era = Some(ch.era);
                rows.push((None, ERAS.get(ch.era as usize).copied().unwrap_or("").to_string()));
            }
            rows.push((Some(c), format!("{}. {}", self.progress.number(c), ch.title)));
        }
        if chs.iter().any(|ch| ch.side) {
            rows.push((None, "Caminhos paralelos".to_string()));
            for (c, ch) in chs.iter().enumerate().filter(|(_, ch)| ch.side) {
                rows.push((Some(c), ch.title.clone()));
            }
        }
        rows
    }

    /// J opens the journal; while it's open the up and down arrows turn its chapters, left and
    /// right pick a goal, Enter follows it on the screen and Esc closes it.
    pub(crate) fn progress_key(&mut self, code: KeyCode) -> bool {
        if !self.world_open || self.screen != Screen::Playing {
            return false;
        }
        let n = self.progress.chapters().len();
        if code == KeyCode::KeyJ {
            self.progress_sync();
            self.progress.journal = !self.progress.journal;
            let (c, g) = self.progress.tracked().unwrap_or((self.progress.current().unwrap_or(n.saturating_sub(1)), 0));
            self.progress.view = c;
            self.progress.pick = g;
            return true;
        }
        if !self.progress.journal || n == 0 {
            return false;
        }
        let order: Vec<usize> = self.journal_rows().into_iter().filter_map(|r| r.0).collect();
        let at = order.iter().position(|&c| c == self.progress.view).unwrap_or(0);
        let goals = self.progress.chapters()[self.progress.view.min(n - 1)].goals.len().max(1);
        match code {
            KeyCode::ArrowUp => {
                self.progress.view = order[(at + order.len() - 1) % order.len()];
                self.progress.pick = 0;
            }
            KeyCode::ArrowDown => {
                self.progress.view = order[(at + 1) % order.len()];
                self.progress.pick = 0;
            }
            KeyCode::ArrowLeft => self.progress.pick = (self.progress.pick + goals - 1) % goals,
            KeyCode::ArrowRight => self.progress.pick = (self.progress.pick + 1) % goals,
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let (c, g) = (self.progress.view.min(n - 1), self.progress.pick.min(goals - 1));
                let k = self.progress.key(c, g);
                if self.progress.save.track.as_deref() == Some(k.as_str()) || self.progress.done(c, g) || !self.progress.known(c, g) {
                    self.progress.save.track = None;
                } else {
                    self.progress.save.track = Some(k);
                }
                self.progress.aim.t = 0.0;
                self.sfx(Sfx::Click);
            }
            KeyCode::Escape => self.progress.journal = false,
            _ => return false,
        }
        true
    }

    /// The goal list, the compass, the Guide's mark, his words, the chapter's name and the journal.
    pub(crate) fn draw_progress(&mut self, w: f32, h: f32) {
        if self.progress.world != self.world_name {
            return;
        }
        let s = self.ui.scale;
        let current = self.progress.current();
        let ready = self.survival && self.progress.reward_ready().is_some();
        if ready {
            self.draw_guide_mark(w, h);
        }
        // The goal list stays off while a screen is open, so it never shows through its panels.
        if self.survival && !self.show_debug && !self.progress.journal && self.screen == crate::Screen::Playing {
            self.draw_aim_marker(w, h);
            self.draw_goal_list(current, ready);
            self.draw_summon_hint(w, h);
        }
        if let Some((title, era, until)) = self.progress.card.clone() {
            let left = until - self.now;
            if left <= 0.0 || self.progress.journal {
                self.progress.card = None;
            } else {
                let a = (left.min(1.0) * 255.0) as u8;
                let big = s * 2.0;
                let tw = self.ui.text_width(&title, big);
                let y = h * 0.38;
                self.ui.rect((w - tw) / 2.0 - 8.0 * s, y - 6.0 * s, tw + 16.0 * s, 32.0 * s, [0, 0, 0, a / 2]);
                self.ui.text_scaled((w - tw) / 2.0, y, &title, [255, 220, 120, a], big);
                let ew = self.ui.text_width(&era, s);
                self.ui.text_scaled((w - ew) / 2.0, y + 18.0 * s, &era, [TEAL[0], TEAL[1], TEAL[2], a], s);
            }
        }
        if let Some(d) = &self.progress.dialog {
            let (who, text) = (d.who.clone(), d.text.clone());
            let bw = (360.0 * s).min(w - 20.0 * s);
            let lines = wrap(&self.ui, &text, bw - 12.0 * s, s);
            let bh = (lines.len() as f32 + 1.0) * 10.0 * s + 10.0 * s;
            let x = (w - bw) / 2.0;
            let y = h - 66.0 * s - bh;
            self.ui.rect(x, y, bw, bh, [16, 20, 36, 215]);
            self.ui.frame(x, y, bw, bh, s, [120, 140, 200, 255]);
            self.ui.text(x + 6.0 * s, y + 5.0 * s, &who, YELLOW);
            for (k, l) in lines.iter().enumerate() {
                self.ui.text(x + 6.0 * s, y + 5.0 * s + (k as f32 + 1.0) * 10.0 * s, l, WHITE);
            }
        }
        if self.progress.journal {
            self.draw_journal(w, h);
        }
    }

    /// Top left: the chapter, its next goals, where the followed one is and what it takes.
    fn draw_goal_list(&mut self, current: Option<usize>, ready: bool) {
        let Some(c) = current.or(self.progress.reward_ready()) else { return };
        let s = self.ui.scale;
        // The details under the followed goal are a size smaller.
        let small = (s - 1.0).max(1.0);
        let max_w = 200.0 * s;
        let ch = &self.progress.chapters()[c];
        let title = if ch.side { ch.title.clone() } else { format!("{}. {}", self.progress.number(c), ch.title) };
        let mut lines: Vec<Line> = vec![Line::new(title, YELLOW)];
        if ready {
            lines.push(Line::new("Fale com o Guia!".into(), WHITE));
        } else {
            let tracked = self.progress.tracked();
            let mut goals: Vec<(usize, usize)> = tracked.into_iter().collect();
            for g in self.progress.pending(c) {
                if goals.len() < 3 && !goals.contains(&(c, g)) {
                    goals.push((c, g));
                }
            }
            for (gc, g) in goals {
                let goal = &self.progress.chapters()[gc].goals[g];
                let text = format!("{}{}", goal.text, self.goal_count(gc, g));
                lines.push(Line { check: true, ..Line::new(text, if goal.optional { GRAY } else { WHITE }) });
                if Some((gc, g)) != tracked {
                    continue;
                }
                let detail = |text: String, col| Line { indent: 9.0 * s, scale: small, ..Line::new(text, col) };
                if self.progress.aim.key == self.progress.key(gc, g) {
                    if let Some(y) = self.progress.aim.down {
                        let dy = (self.body.pos.y - y as f32).max(1.0) as i32;
                        lines.push(detail(format!("Desça mais {dy} blocos (até a altura {y})"), TEAL));
                    } else if let Some(to) = self.progress.aim.to {
                        let d = to - self.body.pos;
                        let text = format!("{} m para o {}", Vec2::new(d.x, d.z).length() as i32, heading(d.x, d.z));
                        lines.push(Line { compass: true, indent: 20.0 * s, ..Line::new(text, TEAL) });
                    }
                }
                if let Some(r) = self.goal_recipe(gc, g) {
                    lines.push(detail(r, GRAY));
                }
                if let Some(item) = &goal.summon {
                    if let Some((rule, _)) = self.summon_where(item) {
                        lines.push(detail(format!("Use {rule}"), GRAY));
                    }
                }
            }
            let boss_open = self.progress.pending(c).iter().any(|&g| matches!(self.progress.chapters()[c].goals[g].need, Need::Boss(_) | Need::Bosses(..)));
            if let (true, Some((hp_ok, def_ok)), Some((hp, def))) = (boss_open, self.ready_ok(c), self.progress.chapters()[c].ready) {
                let col = if hp_ok && def_ok { GREEN } else { ORANGE };
                let t = format!("Pronto para o chefe? {}/{} corações, {}/{def} de defesa", (self.max_health / 2.0).round(), (hp / 2.0).round(), self.defense_points());
                lines.push(Line { scale: small, ..Line::new(t, col) });
            }
        }
        lines.push(Line { scale: small, ..Line::new("J: diário".into(), GRAY) });
        // Small lines wrap (three lines at most), big ones are cut to fit.
        let mut out: Vec<Line> = Vec::new();
        for l in lines {
            let l = Line { scale: if l.scale == 0.0 { s } else { l.scale }, ..l };
            let room = max_w - l.indent - if l.check { 9.0 * s } else { 0.0 };
            if l.scale < s {
                let mut parts = wrap(&self.ui, &l.text, room, l.scale);
                parts.truncate(3);
                for (k, p) in parts.into_iter().enumerate() {
                    out.push(Line { text: p, compass: l.compass && k == 0, ..l.clone() });
                }
                continue;
            }
            let mut l = l;
            if self.ui.text_width(&l.text, l.scale) > room {
                while self.ui.text_width(&format!("{}...", l.text), l.scale) > room && l.text.chars().count() > 4 {
                    l.text.pop();
                }
                l.text = format!("{}...", l.text.trim_end());
            }
            out.push(l);
        }
        let x = 4.0 * s;
        let mut y = 4.0 * s;
        let tw = out.iter().map(|l| self.ui.text_width(&l.text, l.scale) + l.indent + if l.check { 9.0 * s } else { 0.0 }).fold(0.0, f32::max);
        let th: f32 = out.iter().map(|l| 10.0 * l.scale).sum();
        self.ui.rect(x - 2.0 * s, y - 2.0 * s, tw + 6.0 * s, th + 2.0 * s, [0, 0, 0, 110]);
        for l in out {
            let mut tx = x + l.indent;
            if l.check {
                draw_box(&mut self.ui, x, y, s, false);
                tx += 9.0 * s;
            }
            if l.compass {
                self.draw_compass(x + 13.0 * s, y + 3.5 * s, 3.5 * s);
            }
            self.ui.text_scaled(tx, y, &l.text, l.color, l.scale);
            y += 10.0 * l.scale;
        }
    }

    /// The direction of the followed goal, from where you look: an arrow that turns as you do.
    fn draw_compass(&mut self, cx: f32, cy: f32, r: f32) {
        let Some(to) = self.progress.aim.to else { return };
        let f = self.forward();
        let fwd = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let right = fwd.cross(Vec3::Y);
        let d = Vec3::new(to.x - self.body.pos.x, 0.0, to.z - self.body.pos.z).normalize_or_zero();
        let dir = Vec2::new(d.dot(right), -d.dot(fwd)).normalize_or(Vec2::NEG_Y);
        let side = Vec2::new(-dir.y, dir.x);
        let c = Vec2::new(cx, cy);
        let tip = c + dir * r * 1.2;
        let back = c - dir * r;
        let p = [tip, back + side * r * 0.8, c - dir * r * 0.35, back - side * r * 0.8].map(|v| [v.x, v.y]);
        self.ui.rect(cx - r * 1.4, cy - r * 1.4, r * 2.8, r * 2.8, [0, 0, 0, 90]);
        self.ui.quad(p, [[0.0; 2]; 4], [255, 220, 90, 255], -2);
    }

    /// A small diamond over the followed goal's spot, when it is in view.
    fn draw_aim_marker(&mut self, w: f32, h: f32) {
        let (Some(to), Some((vp, cam_block))) = (self.progress.aim.to, self.ui_view_proj) else { return };
        let s = self.ui.scale;
        let c = vp * (to - cam_block.as_vec3()).extend(1.0);
        if c.w < 0.1 {
            return;
        }
        let (nx, ny) = (c.x / c.w, c.y / c.w);
        if nx.abs() > 0.95 || ny.abs() > 0.95 {
            return;
        }
        let (x, y) = ((nx * 0.5 + 0.5) * w, (0.5 - ny * 0.5) * h);
        let r = 4.0 * s;
        self.ui.quad([[x, y - r - s], [x + r + s, y], [x, y + r + s], [x - r - s, y]], [[0.0; 2]; 4], [0, 0, 0, 160], -2);
        self.ui.quad([[x, y - r], [x + r, y], [x, y + r], [x - r, y]], [[0.0; 2]; 4], [255, 220, 90, 230], -2);
        let dist = format!("{} m", to.distance(self.body.pos) as i32);
        let tw = self.ui.text_width(&dist, s);
        self.ui.text(x - tw / 2.0, y + r + 3.0 * s, &dist, [255, 230, 150, 255]);
    }

    /// Holding an item that wakes a boss: where it works, whether it would here and now, and
    /// whether the Guide thinks you are ready.
    fn draw_summon_hint(&mut self, w: f32, h: f32) {
        if self.progress.dialog.is_some() || self.screen != Screen::Playing {
            return;
        }
        let Some(st) = self.surv.inv.slots.get(self.selected).copied().flatten() else { return };
        let name = self.items.get(st.item).name.clone();
        let Some((rule, ok)) = self.summon_where(&name) else { return };
        let s = (self.ui.scale - 1.0).max(1.0);
        let (now, col) = match ok {
            Some(true) => (": aqui funciona", GREEN),
            Some(false) => (": aqui não", ORANGE),
            None => ("", WHITE),
        };
        let mut lines = vec![(format!("Acorda um chefe. Use {rule}{now}"), col)];
        let chapter = (0..self.progress.chapters().len()).find(|&c| self.progress.chapters()[c].goals.iter().any(|g| g.summon.as_deref() == Some(name.as_str())));
        if let (Some(c), Some((hp, def))) = (chapter, chapter.and_then(|c| self.progress.chapters()[c].ready)) {
            if self.ready_ok(c) != Some((true, true)) {
                let text = format!("O Guia acha cedo: {}/{} corações, {}/{def} de defesa", (self.max_health / 2.0).round(), (hp / 2.0).round(), self.defense_points());
                lines.push((text, ORANGE));
            }
        }
        let mut y = h - 66.0 * self.ui.scale - lines.len() as f32 * 10.0 * s;
        for (text, col) in lines {
            let tw = self.ui.text_width(&text, s);
            self.ui.rect((w - tw) / 2.0 - 3.0 * s, y - 2.0 * s, tw + 6.0 * s, 10.0 * s, [0, 0, 0, 120]);
            self.ui.text_scaled((w - tw) / 2.0, y, &text, col, s);
            y += 10.0 * s;
        }
    }

    /// A bobbing "!" over the Guide while a reward waits for you.
    fn draw_guide_mark(&mut self, w: f32, h: f32) {
        let Some((vp, cam_block)) = self.ui_view_proj else { return };
        let s = self.ui.scale;
        let Some(m) = self.mobs.list.iter().find(|m| m.alive() && self.mobs.reg.defs[m.def].name == GUIDE) else { return };
        let def = &self.mobs.reg.defs[m.def];
        let top = m.body.pos + Vec3::Y * (def.height + 0.9 + (self.now * 3.0).sin() * 0.1);
        if (top - self.body.eye()).length() > 48.0 {
            return;
        }
        let c = vp * (top - cam_block.as_vec3()).extend(1.0);
        if c.w < 0.1 {
            return;
        }
        let (nx, ny) = (c.x / c.w, c.y / c.w);
        if nx.abs() > 1.0 || ny.abs() > 1.0 {
            return;
        }
        let (x, y) = ((nx * 0.5 + 0.5) * w, (0.5 - ny * 0.5) * h);
        self.ui.rect(x - 3.0 * s, y - 12.0 * s, 6.0 * s, 16.0 * s, [0, 0, 0, 150]);
        self.ui.rect(x - 2.0 * s, y - 11.0 * s, 4.0 * s, 9.0 * s, [255, 220, 60, 255]);
        self.ui.rect(x - 2.0 * s, y, 4.0 * s, 3.0 * s, [255, 220, 60, 255]);
    }

    fn draw_journal(&mut self, w: f32, h: f32) {
        // A size smaller than the rest of the interface, so a whole chapter fits.
        let s = (self.ui.scale - 1.0).max(1.0);
        let n = self.progress.chapters().len();
        if n == 0 {
            return;
        }
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 120]);
        // Above the hotbar and hearts, which stay on top.
        let hud = 64.0 * self.ui.scale;
        let pw = (470.0 * s).min(w - 16.0 * s);
        let ph = (260.0 * s).min(h - hud - 12.0 * s);
        let (px, py) = ((w - pw) / 2.0, ((h - hud - ph) / 2.0).max(6.0 * s));
        self.ui.rect(px, py, pw, ph, [24, 22, 30, 235]);
        self.ui.frame(px, py, pw, ph, s, [150, 130, 90, 255]);
        let tw = self.ui.text_width("Diário de aventura", s);
        self.ui.text_scaled((w - tw) / 2.0, py + 5.0 * s, "Diário de aventura", YELLOW, s);
        let current = self.progress.current();
        let view = self.progress.view.min(n - 1);
        // Left: eras and chapters, scrolled so the chosen one shows.
        let lw = 150.0 * s;
        let rows = self.journal_rows();
        let fit = ((ph - 34.0 * s) / (10.0 * s)).floor().max(1.0) as usize;
        let at = rows.iter().position(|r| r.0 == Some(view)).unwrap_or(0);
        let first = at.saturating_sub(fit / 2).min(rows.len().saturating_sub(fit));
        for (k, (ch, label)) in rows.iter().enumerate().skip(first).take(fit) {
            let y = py + 18.0 * s + (k - first) as f32 * 10.0 * s;
            let Some(c) = *ch else {
                self.ui.text_scaled(px + 4.0 * s, y, label, TEAL, s);
                continue;
            };
            let p = &self.progress;
            let (mark, col) = if !p.active(c) {
                ("  ", [90, 90, 90, 255])
            } else if p.claimed(c) {
                ("+ ", [110, 210, 110, 255])
            } else if p.complete(c) {
                ("! ", YELLOW)
            } else if Some(c) == current {
                ("> ", YELLOW)
            } else {
                ("  ", GRAY)
            };
            if c == view {
                self.ui.rect(px + 3.0 * s, y - 1.0 * s, lw, 10.0 * s, [70, 64, 50, 255]);
            }
            let mut title = format!("{mark}{label}");
            while self.ui.text_width(&title, s) > lw - 8.0 * s && title.chars().count() > 4 {
                title.pop();
            }
            self.ui.text_scaled(px + 9.0 * s, y, &title, col, s);
        }
        // Right: the chapter's goals, then everything about the goal picked.
        let c = view;
        let rx = px + lw + 10.0 * s;
        let rw = pw - lw - 16.0 * s;
        let bottom = py + ph - 14.0 * s;
        let mut y = py + 18.0 * s;
        let active = self.progress.active(c);
        let ch = &self.progress.chapters()[c];
        let status = if !active {
            "em breve"
        } else if self.progress.claimed(c) {
            "concluído"
        } else if self.progress.complete(c) {
            "pronto: fale com o Guia"
        } else if Some(c) == current || ch.side {
            "em andamento"
        } else {
            "ainda fechado"
        };
        let era = if ch.side { "Caminho paralelo".to_string() } else { format!("Era: {}", ERAS.get(ch.era as usize).copied().unwrap_or("")) };
        let mut rows: Vec<(String, [u8; 4], Option<bool>, bool)> = Vec::new();
        rows.push((format!("{} ({status})", ch.title), YELLOW, None, false));
        rows.push((era, TEAL, None, false));
        if let Some(text) = self.ready_text(c).filter(|_| !self.progress.claimed(c)) {
            let col = if self.ready_ok(c) == Some((true, true)) { GREEN } else { ORANGE };
            rows.push((text, col, None, false));
        }
        let pick = self.progress.pick.min(ch.goals.len().saturating_sub(1));
        let track = self.progress.save.track.clone();
        for (g, goal) in ch.goals.iter().enumerate() {
            let known = self.progress.known(c, g);
            let done = self.progress.done(c, g);
            let mut t = goal.text.clone();
            if known && !done {
                t += &self.goal_count(c, g);
            }
            if !known {
                t += " (em breve)";
            } else if goal.optional {
                t += " (extra)";
            }
            if track.as_deref() == Some(self.progress.key(c, g).as_str()) {
                t += " [seguindo]";
            }
            let col = if !known { [100, 100, 100, 255] } else if done { [140, 210, 140, 255] } else if goal.optional { GRAY } else { WHITE };
            rows.push((t, col, Some(done), g == pick));
        }
        for (t, col, check, picked) in rows {
            let indent = if check.is_some() { 9.0 * s } else { 0.0 };
            let lines = wrap(&self.ui, &t, rw - indent, s);
            if picked {
                self.ui.rect(rx - 2.0 * s, y - 1.0 * s, rw + 2.0 * s, lines.len() as f32 * 10.0 * s, [60, 56, 44, 255]);
            }
            if let Some(done) = check {
                draw_box(&mut self.ui, rx, y, s, done);
            }
            for l in lines {
                if y > bottom - 10.0 * s {
                    break;
                }
                self.ui.text_scaled(rx + indent, y, &l, col, s);
                y += 10.0 * s;
            }
        }
        // The goal picked: how, with what, where.
        if let Some(goal) = ch.goals.get(pick) {
            y += 4.0 * s;
            let mut info = vec![(goal.hint.clone(), WHITE)];
            if let Some(r) = self.goal_recipe(c, pick) {
                info.push((format!("Receita: {r}"), GRAY));
            }
            if let Some(wh) = self.goal_where(c, pick) {
                info.push((wh, TEAL));
            }
            self.ui.rect(rx - 2.0 * s, y - 2.0 * s, rw + 2.0 * s, s, [150, 130, 90, 255]);
            y += 2.0 * s;
            'out: for (t, col) in info {
                for l in wrap(&self.ui, &t, rw, s) {
                    if y > bottom - 10.0 * s {
                        break 'out;
                    }
                    self.ui.text_scaled(rx, y, &l, col, s);
                    y += 10.0 * s;
                }
            }
        }
        let help = "Setas: capítulo e objetivo   Enter: seguir na tela   J/Esc: fechar";
        let tw = self.ui.text_width(help, s);
        self.ui.text_scaled((w - tw) / 2.0, py + ph - 11.0 * s, help, GRAY, s);
    }

    /// Test hook (MC_PROGRESS=N): chapters before N finished, the Guide in front of you;
    /// MC_TALK=1 talks to him, MC_JOURNAL=1 opens the journal, MC_TRACK=chapter/goal follows a
    /// goal, MC_CARD=1 shows the chapter's name, MC_HOLD=item puts that item in your hand,
    /// MC_GATE=block breaks it with the wrong tool, MC_SAY=1 has the Guide call out.
    pub(crate) fn test_progress(&mut self, upto: usize) {
        self.progress_sync();
        let main: Vec<usize> = (0..self.progress.chapters().len()).filter(|&c| !self.progress.chapters()[c].side).collect();
        for &c in main.iter().take(upto) {
            for g in 0..self.progress.chapters()[c].goals.len() {
                let k = self.progress.key(c, g);
                self.progress.save.done.push(k);
            }
            let id = self.progress.chapters()[c].id.clone();
            self.progress.save.claimed.push(id);
        }
        // The chapter in play: its first goal done, and its reward waiting with MC_REWARD.
        if let Some(c) = self.progress.current() {
            let all = std::env::var("MC_REWARD").is_ok();
            for g in 0..self.progress.chapters()[c].goals.len() {
                if g == 0 || all {
                    let k = self.progress.key(c, g);
                    self.progress.save.done.push(k);
                }
            }
        }
        self.progress.save.greeted = true;
        self.progress.quiet = false;
        self.progress.shown = self.progress.current();
        if let Ok(k) = std::env::var("MC_TRACK") {
            self.progress.save.track = Some(k);
        }
        if let Ok(item) = std::env::var("MC_HOLD") {
            if let Some(id) = self.items.id(&item) {
                let sel = self.selected;
                self.surv.inv.slots[sel] = Some(ItemStack::new(id, 1));
            }
        }
        if !self.town.npcs.iter().any(|n| n.name == GUIDE) {
            self.spawn_guide();
        }
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let eye = self.body.eye();
        let to_me = (-flat.x).atan2(-flat.z);
        if let Some(m) = self.mobs.list.iter_mut().find(|m| self.mobs.reg.defs[m.def].name == GUIDE) {
            let y = self.world.surface_y((eye.x + flat.x * 3.0).floor() as i32, (eye.z + flat.z * 3.0).floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(eye.y - 1.6);
            m.body.pos = Vec3::new(eye.x + flat.x * 3.0, y, eye.z + flat.z * 3.0);
            m.yaw = to_me;
        }
        self.update_aim(0.0);
        if std::env::var("MC_TALK").is_ok() {
            let (at, who) = (self.body.pos, self.guide_name());
            for _ in 0..std::env::var("MC_TALK").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).max(1) {
                self.guide_talk(at, &who);
            }
        }
        if std::env::var("MC_SAY").is_ok() {
            self.guide_call("O Olho caiu! Daqui a algumas noites, meteoros podem riscar o céu: o meteorito vira armas de fogo.".into());
        }
        if std::env::var("MC_CARD").is_ok() {
            if let Some(c) = self.progress.current() {
                self.chapter_card(c);
                if let Some(card) = &mut self.progress.card {
                    card.2 = self.now + 1000.0;
                }
            }
        }
        if let Ok(b) = std::env::var("MC_GATE") {
            if let Some(id) = self.reg.id(&b) {
                self.progress_gate(id);
            }
        }
        if std::env::var("MC_JOURNAL").is_ok() {
            self.progress.journal = true;
            let (c, g) = self.progress.tracked().unwrap_or((self.progress.current().unwrap_or(0), 0));
            self.progress.view = c;
            self.progress.pick = g;
        }
    }
}

/// A line of the goal list.
#[derive(Clone)]
struct Line {
    text: String,
    color: [u8; 4],
    check: bool,
    indent: f32,
    scale: f32,
    compass: bool,
}

impl Line {
    fn new(text: String, color: [u8; 4]) -> Line {
        Line { text, color, check: false, indent: 0.0, scale: 0.0, compass: false }
    }
}

fn draw_box(ui: &mut crate::ui::UiBatch, x: f32, y: f32, s: f32, done: bool) {
    ui.frame(x, y, 7.0 * s, 7.0 * s, s, [200, 200, 200, 255]);
    if done {
        ui.rect(x + 2.0 * s, y + 2.0 * s, 3.0 * s, 3.0 * s, [120, 230, 120, 255]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_book_parses() {
        let book: Book = ron::from_str(BUILT_IN).expect("progression.ron parses");
        assert!(book.chapters.len() >= 9);
        let mut ids = std::collections::HashSet::new();
        for ch in &book.chapters {
            assert!(ids.insert(ch.id.clone()), "duplicate chapter {}", ch.id);
            let mut goals = std::collections::HashSet::new();
            for g in &ch.goals {
                assert!(goals.insert(g.id.clone()), "duplicate goal {}/{}", ch.id, g.id);
            }
        }
    }

    #[test]
    fn eras_match_the_balance_sheet() {
        assert_eq!(ERAS, crate::balance::STAGES);
        let book: Book = ron::from_str(BUILT_IN).unwrap();
        let mut last = 0;
        for ch in book.chapters.iter().filter(|c| !c.side) {
            assert!((ch.era as usize) < ERAS.len(), "{} has no era", ch.id);
            assert!(ch.era >= last, "{} goes back an era", ch.id);
            last = ch.era;
        }
        // What the Guide recommends must be reachable (crystals stop at 40, fruit at 50) and grow.
        let mut last = (0.0, 0);
        for (id, ready) in book.chapters.iter().filter_map(|c| c.ready.map(|r| (&c.id, r))) {
            assert!(ready.0 <= 50.0 && ready.0 >= last.0 && ready.1 >= last.1, "{id}: {ready:?}");
            last = ready;
        }
    }

    /// Every item, creature and building the road names exists (a typo would leave a goal
    /// "em breve" forever, or the compass pointing nowhere).
    #[test]
    fn everything_named_exists() {
        let read = |pattern: &str| -> String {
            let mut out = String::new();
            let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../packs/vanilla");
            let mut dirs = vec![std::path::PathBuf::from(root)];
            while let Some(d) = dirs.pop() {
                for e in std::fs::read_dir(&d).unwrap().flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        dirs.push(p);
                    } else if p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains(pattern) && n.ends_with(".ron")) {
                        out += &std::fs::read_to_string(&p).unwrap();
                    }
                }
            }
            out
        };
        let names = |text: &str| -> HashSet<String> { text.split("name: \"").skip(1).filter_map(|s| s.split('"').next()).map(String::from).collect() };
        let items: HashSet<String> = names(&(read("items") + &read("blocks")));
        let mobs = names(&read("mobs"));
        let buildings = names(&read("structures"));
        let any = |set: &HashSet<String>, p: &str| set.iter().any(|n| matches(p, n));
        let book: Book = ron::from_str(BUILT_IN).unwrap();
        let mut missing = Vec::new();
        for ch in &book.chapters {
            for (item, _) in &ch.reward {
                if !items.contains(item) {
                    missing.push(format!("{}: reward {item}", ch.id));
                }
            }
            for g in &ch.goals {
                let mut check = |set: &HashSet<String>, n: &str| {
                    if !any(set, n) {
                        missing.push(format!("{}/{}: {n}", ch.id, g.id));
                    }
                };
                match &g.need {
                    Need::Item(list) => list.iter().for_each(|n| check(&items, n)),
                    Need::Boss(n) | Need::Npc(n) | Need::Kill(n, _) => check(&mobs, n),
                    Need::Bosses(list, _) => list.iter().for_each(|n| check(&mobs, n)),
                    Need::Structure(list) => list.iter().for_each(|n| check(&buildings, n)),
                    _ => {}
                }
                if let Some(Find::Structure(list)) = &g.find {
                    // Deep buildings (worldgen/deep.rs) aren't in the files; the compass skips them.
                    let deep = ["cabana_subterranea", "mina_profunda"];
                    list.iter().filter(|n| !deep.contains(&n.as_str())).for_each(|n| check(&buildings, n));
                }
                if let Some(s) = &g.summon {
                    check(&items, s);
                }
            }
        }
        assert!(missing.is_empty(), "unknown names in progression.ron: {missing:?}");
    }

    #[test]
    fn headings() {
        assert_eq!(heading(0.0, -10.0), "norte");
        assert_eq!(heading(10.0, 0.0), "leste");
        assert_eq!(heading(0.0, 10.0), "sul");
        assert_eq!(heading(-10.0, 10.0), "sudoeste");
    }

    #[test]
    fn the_evil_land_can_be_found() {
        let at = crate::tr_world::nearest_land(12345, 0, 0, false, |l| land_ok("evil", l));
        assert!(at.is_some_and(|a| a.as_vec2().length() < 4000.0), "{at:?}");
        assert!(crate::tr_world::nearest_land(12345, 0, 0, false, |l| land_ok("prism", l)).is_none());
    }

    #[test]
    fn patterns() {
        assert!(matches("*_log", "oak_log"));
        assert!(!matches("*_log", "oak_planks"));
        assert!(matches("cooked_*", "cooked_beef"));
        assert!(matches("diamond", "diamond"));
        assert!(!matches("diamond", "diamond_pickaxe"));
        assert!(matches("*potion*", "fire_potion_long"));
    }
}
