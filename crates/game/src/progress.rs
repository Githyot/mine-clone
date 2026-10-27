//! Progression (Terraria's idea of a long road with a goal at the end): chapters of goals from
//! `packs/vanilla/progression.ron`, checked against what the player has done, with the Guide at
//! the centre. The Guide is there from the first minute of a survival world, moves into the
//! first house, explains the next step when you talk to him and hands over each chapter's reward.
//! A small list at the top left shows what is next; J opens the whole journal.

use std::collections::HashMap;
use std::path::PathBuf;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use voxel::item::{ItemId, ItemStack};
use winit::keyboard::KeyCode;

use crate::audio::Sfx;
use crate::ui::{GRAY, WHITE, YELLOW};
use crate::{Game, Screen};

const BUILT_IN: &str = include_str!("../../../packs/vanilla/progression.ron");
const GUIDE: &str = "npc_guide";

#[derive(Deserialize)]
struct Book {
    chapters: Vec<Chapter>,
}

#[derive(Deserialize)]
struct Chapter {
    id: String,
    title: String,
    /// What the Guide says when the chapter opens, and when you bring it to an end.
    intro: String,
    outro: String,
    #[serde(default)]
    reward: Vec<(String, u8)>,
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
}

/// What a goal asks for (see the top of `progression.ron`).
#[derive(Deserialize, Clone)]
enum Need {
    Item(Vec<String>),
    Boss(String),
    Npc(String),
    Dim(String),
    Health(f32),
    Mana(f32),
    Depth(i32),
    Kill(String, u32),
    Night,
    House,
}

/// Saved per world in `progress.ron`, next to `level.ron`.
#[derive(Serialize, Deserialize, Default)]
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
}

struct Dialog {
    who: String,
    text: String,
    /// Where the speaker stands; walking away ends the talk.
    at: Vec3,
    until: f32,
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
    /// Chapter shown in the journal.
    view: usize,
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

    /// The chapter being played: the first one whose reward is still to come.
    fn current(&self) -> Option<usize> {
        (0..self.chapters().len()).find(|&c| self.active(c) && !self.claimed(c))
    }

    /// Goals still open in a chapter, required ones first.
    fn pending(&self, c: usize) -> Vec<usize> {
        let goals = &self.chapters()[c].goals;
        let open = |g: &usize| self.known(c, *g) && !self.done(c, *g);
        let mut out: Vec<usize> = (0..goals.len()).filter(|g| !goals[*g].optional).filter(open).collect();
        out.extend((0..goals.len()).filter(|g| goals[*g].optional).filter(open));
        out
    }

    fn kills(&self, pattern: &str) -> u32 {
        self.save.kills.iter().filter(|(k, _)| matches(pattern, k)).map(|(_, n)| *n).sum()
    }
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
                    Need::Dim(d) => (Vec::new(), matches!(d.as_str(), "nether" | "end")),
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
            if let Err(e) = std::fs::write(self.progress_path(), text) {
                log::error!("could not save progress: {e}");
            }
        }
    }

    fn need_met(&self, c: usize, g: usize) -> bool {
        let p = &self.progress;
        match &p.chapters()[c].goals[g].need {
            Need::Item(_) => {
                let ids = &p.items[c][g];
                let surv = &self.surv;
                surv.inv.slots.iter().chain(surv.armor.iter()).chain(std::iter::once(&surv.cursor)).flatten().any(|s| ids.contains(&s.item))
            }
            Need::Boss(n) => self.boss.defeated.iter().any(|b| matches(n, b)),
            Need::Npc(n) => self.town.npcs.iter().any(|t| matches(n, &t.name)),
            Need::Dim(d) => match d.as_str() {
                "nether" => self.dim == voxel::worldgen::Dim::Nether,
                "end" => self.dim == voxel::worldgen::Dim::End,
                _ => false,
            },
            Need::Health(n) => self.max_health >= *n,
            Need::Mana(n) => self.magic.max_mana >= *n,
            Need::Depth(y) => self.dim == voxel::worldgen::Dim::Overworld && self.body.pos.y <= *y as f32,
            Need::Kill(n, count) => p.kills(n) >= *count,
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
            if self.now > d.until || d.at.distance(self.body.pos) > 8.0 {
                self.progress.dialog = None;
            }
        }
        if !self.survival || dt == 0.0 {
            return;
        }
        let overworld = self.dim == voxel::worldgen::Dim::Overworld;
        // A night counts once it turns to day with the player alive the whole time.
        if self.screen == Screen::Dead {
            self.progress.night = false;
        } else if overworld {
            if self.is_night() {
                self.progress.night = true;
            } else if self.progress.night {
                self.progress.night = false;
                self.progress.save.nights += 1;
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
        for c in 0..self.progress.chapters().len() {
            for g in 0..self.progress.chapters()[c].goals.len() {
                if !self.progress.known(c, g) || self.progress.done(c, g) || !self.need_met(c, g) {
                    continue;
                }
                let k = self.progress.key(c, g);
                self.progress.save.done.push(k);
                if !quiet && Some(c) == before {
                    let text = self.progress.chapters()[c].goals[g].text.clone();
                    self.toast(format!("Objetivo concluído: {text}"), 4.0);
                    self.sfx(Sfx::Xp);
                }
            }
        }
        if let Some(c) = before.filter(|&c| !quiet && self.progress.complete(c)) {
            if !self.progress.save.done.iter().any(|d| *d == format!("{}/*", self.progress.chapters()[c].id)) {
                let k = format!("{}/*", self.progress.chapters()[c].id);
                self.progress.save.done.push(k);
                let title = self.progress.chapters()[c].title.clone();
                self.toast(format!("Capítulo concluído: {title}! Fale com o Guia."), 7.0);
                self.sfx(Sfx::Melody(0, 2));
            }
        }
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
        self.progress.dialog = Some(Dialog { who: who.to_string(), text, at, until: self.now + 16.0 });
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
        if let Some(c) = self.progress.current().filter(|&c| self.progress.complete(c)) {
            let mut got = Vec::new();
            for (item, n) in self.progress.chapters()[c].reward.clone() {
                let Some(id) = self.items.id(&item) else { continue };
                got.push(format!("{n} {}", self.items.get(id).display_name));
                if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(id, n)) {
                    self.pop_drop(self.body.pos + Vec3::Y, left);
                }
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
        // What to talk about, in turn: the chapter, the next goals, the item in hand, a tip.
        let ch = &self.progress.chapters()[c];
        let mut pages = vec![format!("{}: {}", ch.title, ch.intro)];
        for g in self.progress.pending(c).into_iter().take(2) {
            let goal = &ch.goals[g];
            let count = match &goal.need {
                Need::Kill(n, k) => format!(" ({}/{k})", self.progress.kills(n).min(*k)),
                _ => String::new(),
            };
            pages.push(format!("{}{count}: {}", goal.text, goal.hint));
        }
        if let Some(text) = self.held_item_uses() {
            pages.push(text);
        }
        pages.push(tips[self.progress.page % tips.len()].to_string());
        let text = pages[self.progress.page % pages.len()].clone();
        self.progress.page += 1;
        self.say(who, text, at);
    }

    /// What the item in hand is good for (Terraria's guide looks at what you show him).
    fn held_item_uses(&self) -> Option<String> {
        let st = self.surv.inv.slots.get(self.selected).copied().flatten()?;
        let id = st.item;
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
        let name = &self.items.get(id).display_name;
        if out.is_empty() {
            return Some(format!("{name}? Não conheço nenhuma receita com isso. Talvez sirva de outro jeito."));
        }
        let names: Vec<&str> = out.iter().take(5).map(|&i| self.items.get(i).display_name.as_str()).collect();
        let more = if out.len() > 5 { format!(" e mais {}", out.len() - 5) } else { String::new() };
        Some(format!("Com {name} você faz: {}{more}. Veja no livro de receitas (U no inventário).", names.join(", ")))
    }

    /// J opens the journal; while it's open the arrows turn its pages and Esc closes it.
    pub(crate) fn progress_key(&mut self, code: KeyCode) -> bool {
        if !self.world_open || self.screen != Screen::Playing {
            return false;
        }
        let n = self.progress.chapters().len();
        if code == KeyCode::KeyJ {
            self.progress_sync();
            self.progress.journal = !self.progress.journal;
            self.progress.view = self.progress.current().unwrap_or(n.saturating_sub(1));
            return true;
        }
        if !self.progress.journal || n == 0 {
            return false;
        }
        match code {
            KeyCode::ArrowLeft | KeyCode::ArrowUp => self.progress.view = (self.progress.view + n - 1) % n,
            KeyCode::ArrowRight | KeyCode::ArrowDown => self.progress.view = (self.progress.view + 1) % n,
            KeyCode::Escape => self.progress.journal = false,
            _ => return false,
        }
        true
    }

    /// The goal list, the Guide's mark, his words and the journal.
    pub(crate) fn draw_progress(&mut self, w: f32, h: f32) {
        if self.progress.world != self.world_name {
            return;
        }
        let s = self.ui.scale;
        let current = self.progress.current();
        let reward_ready = self.survival && current.is_some_and(|c| self.progress.complete(c));
        if reward_ready {
            self.draw_guide_mark(w, h);
        }
        if self.survival && !self.show_debug && !self.progress.journal {
            if let Some(c) = current {
                let ch = &self.progress.chapters()[c];
                let mut lines: Vec<(String, [u8; 4], Option<bool>)> = vec![(ch.title.clone(), YELLOW, None)];
                if reward_ready {
                    lines.push(("Fale com o Guia!".into(), WHITE, None));
                } else {
                    for g in self.progress.pending(c).into_iter().take(3) {
                        let goal = &ch.goals[g];
                        let mut t = goal.text.clone();
                        if let Need::Kill(n, k) = &goal.need {
                            t += &format!(" ({}/{k})", self.progress.kills(n).min(*k));
                        }
                        lines.push((t, if goal.optional { GRAY } else { WHITE }, Some(false)));
                    }
                }
                lines.push(("J: diário".into(), GRAY, None));
                let x = 4.0 * s;
                let mut y = 4.0 * s;
                let tw = lines.iter().map(|l| self.ui.text_width(&l.0, s) + if l.2.is_some() { 9.0 * s } else { 0.0 }).fold(0.0, f32::max);
                self.ui.rect(x - 2.0 * s, y - 2.0 * s, tw + 6.0 * s, lines.len() as f32 * 10.0 * s + 2.0 * s, [0, 0, 0, 110]);
                for (t, col, check) in lines {
                    let mut tx = x;
                    if let Some(done) = check {
                        draw_box(&mut self.ui, x, y, s, done);
                        tx += 9.0 * s;
                    }
                    self.ui.text(tx, y, &t, col);
                    y += 10.0 * s;
                }
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
        let s = self.ui.scale;
        let n = self.progress.chapters().len();
        if n == 0 {
            return;
        }
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 120]);
        let pw = (400.0 * s).min(w - 16.0 * s);
        let ph = (220.0 * s).min(h - 16.0 * s);
        let (px, py) = ((w - pw) / 2.0, (h - ph) / 2.0);
        self.ui.rect(px, py, pw, ph, [24, 22, 30, 235]);
        self.ui.frame(px, py, pw, ph, s, [150, 130, 90, 255]);
        self.ui.text_centered(w / 2.0, py + 5.0 * s, "Diário de aventura", YELLOW);
        let current = self.progress.current();
        // Left: the chapters.
        let lw = 150.0 * s;
        for c in 0..n {
            let y = py + 20.0 * s + c as f32 * 11.0 * s;
            if y > py + ph - 20.0 * s {
                break;
            }
            let p = &self.progress;
            let (mark, col) = if !p.active(c) {
                ("  ", [90, 90, 90, 255])
            } else if p.claimed(c) {
                ("+ ", [110, 210, 110, 255])
            } else if Some(c) == current {
                ("> ", YELLOW)
            } else {
                ("  ", GRAY)
            };
            if c == p.view {
                self.ui.rect(px + 3.0 * s, y - 2.0 * s, lw, 11.0 * s, [70, 64, 50, 255]);
            }
            let mut title = format!("{mark}{}. {}", c + 1, p.chapters()[c].title);
            while self.ui.text_width(&title, s) > lw - 4.0 * s && title.chars().count() > 4 {
                title.pop();
            }
            self.ui.text(px + 5.0 * s, y, &title, col);
        }
        // Right: the goals of the chapter being looked at.
        let c = self.progress.view.min(n - 1);
        let rx = px + lw + 10.0 * s;
        let rw = pw - lw - 16.0 * s;
        let mut y = py + 20.0 * s;
        let active = self.progress.active(c);
        let ch = &self.progress.chapters()[c];
        let mut rows: Vec<(String, [u8; 4], Option<bool>)> = Vec::new();
        let status = if !active {
            "Em breve"
        } else if self.progress.claimed(c) {
            "Concluído"
        } else if self.progress.complete(c) {
            "Pronto: fale com o Guia"
        } else if Some(c) == current {
            "Em andamento"
        } else {
            "Ainda fechado"
        };
        rows.push((format!("{} ({status})", ch.title), YELLOW, None));
        for (g, goal) in ch.goals.iter().enumerate() {
            let known = self.progress.known(c, g);
            let done = self.progress.done(c, g);
            let mut t = goal.text.clone();
            if let Need::Kill(n, k) = &goal.need {
                if known && !done {
                    t += &format!(" ({}/{k})", self.progress.kills(n).min(*k));
                }
            }
            if !known {
                t += " (em breve)";
            } else if goal.optional {
                t += " (extra)";
            }
            let col = if !known { [100, 100, 100, 255] } else if done { [140, 210, 140, 255] } else if goal.optional { GRAY } else { WHITE };
            rows.push((t, col, Some(done)));
        }
        for (t, col, check) in rows {
            let indent = if check.is_some() { 9.0 * s } else { 0.0 };
            let lines = wrap(&self.ui, &t, rw - indent, s);
            if let Some(done) = check {
                draw_box(&mut self.ui, rx, y, s, done);
            }
            for l in lines {
                if y > py + ph - 22.0 * s {
                    break;
                }
                self.ui.text(rx + indent, y, &l, col);
                y += 10.0 * s;
            }
            y += 1.0 * s;
        }
        self.ui.text_centered(w / 2.0, py + ph - 11.0 * s, "Setas: capítulos   J ou Esc: fechar", GRAY);
    }

    /// Test hook (MC_PROGRESS=N): chapters before N finished, the Guide in front of you;
    /// MC_TALK=1 talks to him, MC_JOURNAL=1 opens the journal.
    pub(crate) fn test_progress(&mut self, upto: usize) {
        self.progress_sync();
        for c in 0..upto.min(self.progress.chapters().len()) {
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
        if std::env::var("MC_TALK").is_ok() {
            let (at, who) = (self.body.pos, self.guide_name());
            self.guide_talk(at, &who);
        }
        if std::env::var("MC_JOURNAL").is_ok() {
            self.progress.journal = true;
            self.progress.view = self.progress.current().unwrap_or(0);
        }
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
    fn patterns() {
        assert!(matches("*_log", "oak_log"));
        assert!(!matches("*_log", "oak_planks"));
        assert!(matches("cooked_*", "cooked_beef"));
        assert!(matches("diamond", "diamond"));
        assert!(!matches("diamond", "diamond_pickaxe"));
        assert!(matches("*potion*", "fire_potion_long"));
    }
}
