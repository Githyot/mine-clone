//! Ofícios (the Aged modpack's idea of professions, HP's request of 2026-10-16): seven crafts
//! that grow with use, each to level 20, each with a small lasting reward.
//!
//! - Mineração: stone, ores, dirt and sand dug out. Digs faster, and ores sometimes give more.
//! - Lenhador: wood cut. Chops faster, and logs sometimes come in pairs.
//! - Lavoura: ripe crops harvested. Sometimes an extra handful.
//! - Pesca: fish caught. Fish bite sooner.
//! - Combate: damage dealt by the player's own hands and weapons. Hits a little harder.
//! - Alquimia: potions brewed near the player and potions drunk. Potions last longer.
//! - Exploração: ground covered on foot, swimming or flying. Gets hungry more slowly.
//!
//! Every reward is small on purpose (a level-20 miner digs half again as fast, not twice), so
//! the crafts reward the way someone likes to play without making the rest of the game trivial.
//! Y opens the page with all seven; a short line above the hotbar shows experience as it comes
//! in. Saved per world in `oficios.ron`, next to `level.ron`. Creative worlds don't train them.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use voxel::block::{BlockId, ToolKind};
use voxel::item::ItemStack;
use winit::keyboard::KeyCode;

use crate::audio::Sfx;
use crate::ui::{GRAY, WHITE, YELLOW};
use crate::{Game, Screen};

pub const MAX_LEVEL: u32 = 20;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Skill {
    Mining,
    Wood,
    Farming,
    Fishing,
    Combat,
    Alchemy,
    Travel,
}

pub const ALL: [Skill; 7] = [Skill::Mining, Skill::Wood, Skill::Farming, Skill::Fishing, Skill::Combat, Skill::Alchemy, Skill::Travel];

impl Skill {
    pub fn name(self) -> &'static str {
        match self {
            Skill::Mining => "Mineração",
            Skill::Wood => "Lenhador",
            Skill::Farming => "Lavoura",
            Skill::Fishing => "Pesca",
            Skill::Combat => "Combate",
            Skill::Alchemy => "Alquimia",
            Skill::Travel => "Exploração",
        }
    }

    /// The item drawn beside the craft on its page.
    fn icon(self) -> &'static str {
        match self {
            Skill::Mining => "iron_pickaxe",
            Skill::Wood => "iron_axe",
            Skill::Farming => "wheat",
            Skill::Fishing => "fishing_rod",
            Skill::Combat => "iron_sword",
            Skill::Alchemy => "brewing_stand",
            Skill::Travel => "compass",
        }
    }

    /// What its training brings, at a level.
    pub fn reward(self, l: u32) -> String {
        let l = l as f32;
        match self {
            Skill::Mining => format!("picareta e pá {:.0}% mais rápidas, {:.0}% de minério extra", l * 2.5, l * 1.5),
            Skill::Wood => format!("machado {:.0}% mais rápido, {:.0}% de tora extra", l * 3.0, l * 2.0),
            Skill::Farming => format!("{:.0}% de colheita extra", l * 2.5),
            Skill::Fishing => format!("peixes mordem {:.0}% mais rápido", l * 1.5),
            Skill::Combat => format!("+{:.0}% de dano corpo a corpo", l * 1.0),
            Skill::Alchemy => format!("poções duram {:.0}% a mais", l * 2.0),
            Skill::Travel => format!("{:.0}% menos fome", l * 1.5),
        }
    }

    pub fn index(self) -> usize {
        ALL.iter().position(|&s| s == self).unwrap_or(0)
    }
}

/// Total experience a level asks for: 30, 90, 180... (6300 for level 20).
pub fn xp_for(level: u32) -> u32 {
    15 * level * (level + 1)
}

pub fn level_of(xp: u32) -> u32 {
    (0..=MAX_LEVEL).rev().find(|&l| xp >= xp_for(l)).unwrap_or(0)
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Saved {
    xp: [u32; 7],
    /// The statistics already counted (combat, alchemy and travel are read off them).
    #[serde(default)]
    seen: [u64; 4],
    #[serde(default)]
    started: bool,
}

#[derive(Default)]
pub struct Skills {
    save: Saved,
    world: String,
    /// Experience gained a moment ago, shown above the hotbar: (craft, amount, time left).
    recent: Option<(Skill, u32, f32)>,
    /// Hunger already counted, so Exploração can take its share off what was added since.
    exhaustion: f32,
    pub page: bool,
}

/// The statistics the crafts read: damage dealt (tenths), potions brewed, potions drunk, and
/// centimetres travelled.
fn stat_keys() -> [&'static [&'static str]; 4] {
    [&["damage_dealt"], &["potions_brewed"], &["potions_drunk"], &["cm_walked", "cm_swum", "cm_flown"]]
}

impl Game {
    fn skills_path(&self) -> PathBuf {
        crate::level_path_hint(&self.base, &self.world_name).join("oficios.ron")
    }

    fn skills_sync(&mut self) {
        if self.skills.world == self.world_name {
            return;
        }
        self.skills.save = std::fs::read_to_string(self.skills_path()).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default();
        self.skills.world = self.world_name.clone();
        self.skills.recent = None;
        self.skills.page = false;
        self.skills.exhaustion = self.surv.exhaustion;
        if !self.skills.save.started {
            // An old world: what was done before the crafts existed doesn't count all at once.
            self.skills.save.seen = self.skill_stats();
            self.skills.save.started = true;
        }
    }

    pub(crate) fn skills_save(&mut self) {
        if self.skills.world != self.world_name || self.world_name.is_empty() {
            return;
        }
        if let Ok(text) = ron::ser::to_string_pretty(&self.skills.save, Default::default()) {
            if let Err(e) = voxel::save::write_atomic(self.skills_path(), text) {
                log::error!("could not save oficios.ron: {e}");
            }
        }
    }

    fn skill_stats(&self) -> [u64; 4] {
        stat_keys().map(|keys| keys.iter().map(|k| self.prog.stats.get(k)).sum())
    }

    pub(crate) fn skill_level(&self, s: Skill) -> u32 {
        if self.skills.world != self.world_name {
            return 0;
        }
        level_of(self.skills.save.xp[s.index()])
    }

    pub(crate) fn skill_gain(&mut self, s: Skill, n: u32) {
        self.skill_add(s, n, false);
    }

    fn skill_add(&mut self, s: Skill, n: u32, quiet: bool) {
        if !self.survival || n == 0 {
            return;
        }
        self.skills_sync();
        let i = s.index();
        let before = level_of(self.skills.save.xp[i]);
        self.skills.save.xp[i] = self.skills.save.xp[i].saturating_add(n).min(xp_for(MAX_LEVEL));
        let after = level_of(self.skills.save.xp[i]);
        if !quiet {
            let shown = match self.skills.recent {
                Some((k, m, _)) if k == s => m + n,
                _ => n,
            };
            self.skills.recent = Some((s, shown, 2.5));
        }
        if after > before {
            self.toast(format!("{} subiu para o nível {after}!", s.name()), 3.5);
            self.sfx(Sfx::Melody(0, 3));
            self.sfx(Sfx::Xp);
        }
    }

    /// A chance roll against a percentage from a craft.
    fn skill_roll(&mut self, pct: f32) -> bool {
        pct > 0.0 && self.surv.rng.f32() * 100.0 < pct
    }

    /// A block broken by the player in survival trains Mineração, Lenhador or Lavoura, and their
    /// rewards may add to what it drops.
    pub(crate) fn skill_block_broken(&mut self, id: BlockId, drops: &mut Vec<ItemStack>) {
        if !self.survival {
            return;
        }
        let b = self.reg.get(id);
        let name = b.name.clone();
        let (tool, hard, xp) = (b.tool, b.hardness, b.xp.1);
        let ore = name.ends_with("_ore") || name == "ancient_debris" || (xp > 0 && tool == Some(ToolKind::Pickaxe));
        if self.farm.is_crop(id) {
            if !self.farm.ripe(id) {
                return;
            }
            self.skill_gain(Skill::Farming, 4);
            if self.skill_roll(self.skill_level(Skill::Farming) as f32 * 2.5) {
                if let Some(first) = drops.first().copied() {
                    drops.push(ItemStack::new(first.item, 1));
                }
            }
            return;
        }
        match tool {
            Some(ToolKind::Pickaxe) | Some(ToolKind::Shovel) if hard > 0.0 => {
                let n = if ore { 8 + 2 * xp as u32 } else if tool == Some(ToolKind::Pickaxe) { 1 + (hard >= 3.0) as u32 } else { 1 };
                self.skill_gain(Skill::Mining, n);
                if ore && self.skill_roll(self.skill_level(Skill::Mining) as f32 * 1.5) {
                    if let Some(first) = drops.first().copied() {
                        drops.push(ItemStack::new(first.item, 1));
                    }
                }
            }
            Some(ToolKind::Axe) if hard > 0.0 => {
                let log = ["_log", "_wood", "_stem", "_hyphae"].iter().any(|k| name.ends_with(k));
                self.skill_gain(Skill::Wood, if log { 3 } else { 1 });
                if log && self.skill_roll(self.skill_level(Skill::Wood) as f32 * 2.0) {
                    if let Some(first) = drops.first().copied() {
                        drops.push(ItemStack::new(first.item, 1));
                    }
                }
            }
            _ => {}
        }
    }

    /// Divides dig time (weapons.rs `dig_speed_bonus`).
    pub(crate) fn skill_dig_speed(&self, id: BlockId) -> f32 {
        let Some(t) = self.held_tool() else { return 1.0 };
        if self.reg.get(id).tool != Some(t.kind) {
            return 1.0;
        }
        match t.kind {
            ToolKind::Pickaxe | ToolKind::Shovel => 1.0 + self.skill_level(Skill::Mining) as f32 * 0.025,
            ToolKind::Axe => 1.0 + self.skill_level(Skill::Wood) as f32 * 0.03,
            _ => 1.0,
        }
    }

    /// Multiplies the wait for a bite.
    pub(crate) fn skill_bite_factor(&self) -> f32 {
        1.0 - self.skill_level(Skill::Fishing) as f32 * 0.015
    }

    pub(crate) fn skill_fish_caught(&mut self, fish: bool) {
        self.skill_gain(Skill::Fishing, if fish { 12 } else { 5 });
    }

    /// Multiplies melee damage.
    pub(crate) fn skill_melee(&self) -> f32 {
        1.0 + self.skill_level(Skill::Combat) as f32 * 0.01
    }

    /// Multiplies how long a drunk potion lasts.
    pub(crate) fn skill_potion_time(&self) -> f32 {
        1.0 + self.skill_level(Skill::Alchemy) as f32 * 0.02
    }

    pub(crate) fn update_skills(&mut self, dt: f32) {
        if !self.world_open || self.screen == Screen::Loading || self.world_name.is_empty() {
            return;
        }
        self.skills_sync();
        if let Some(r) = &mut self.skills.recent {
            r.2 -= dt;
            if r.2 <= 0.0 {
                self.skills.recent = None;
            }
        }
        // Combat, alchemy and travel come off the statistics the game already keeps.
        let now = self.skill_stats();
        let seen = self.skills.save.seen;
        self.skills.save.seen = now;
        if self.survival {
            let d = |k: usize| now[k].saturating_sub(seen[k]);
            // Damage in tenths: one point per heart's half dealt.
            let dmg = (d(0) / 10) as u32;
            if dmg > 0 {
                self.skill_gain(Skill::Combat, dmg.min(200));
            }
            let brewed = d(1) as u32 * 10 + d(2) as u32 * 3;
            if brewed > 0 {
                self.skill_gain(Skill::Alchemy, brewed.min(200));
            }
            // One point per 20 metres; what is left over waits for the next metres.
            let cm = d(3);
            let pts = (cm / 2000) as u32;
            self.skills.save.seen[3] = now[3] - cm % 2000;
            if pts > 0 && pts < 50 {
                // Exploração's line above the hotbar would show all the time: it trains quietly.
                self.skill_add(Skill::Travel, pts, true);
            }
            // Exploração takes its share off the hunger added since the last frame.
            let added = self.surv.exhaustion - self.skills.exhaustion;
            if added > 0.0 {
                self.surv.exhaustion -= added * self.skill_level(Skill::Travel) as f32 * 0.015;
            }
        }
        self.skills.exhaustion = self.surv.exhaustion;
    }

    pub(crate) fn skills_key(&mut self, code: KeyCode) -> bool {
        if !self.world_open || self.screen != Screen::Playing {
            return false;
        }
        if code == KeyCode::KeyY {
            self.skills_sync();
            self.skills.page = !self.skills.page;
            return true;
        }
        if self.skills.page && code == KeyCode::Escape {
            self.skills.page = false;
            return true;
        }
        false
    }

    /// The line above the hotbar and, with Y, the page of crafts.
    pub(crate) fn draw_skills(&mut self, w: f32, hy: f32) {
        if self.skills.world != self.world_name {
            return;
        }
        let s = self.ui.scale;
        if let Some((k, n, t)) = self.skills.recent {
            if !self.skills.page {
                let l = self.skill_level(k);
                let xp = self.skills.save.xp[k.index()];
                let a = (t.min(1.0) * 255.0) as u8;
                let text = format!("+{n} {}", k.name());
                let bw = 70.0 * s;
                let y = hy - 62.0 * s;
                self.ui.text_centered(w / 2.0, y, &text, [150, 220, 255, a]);
                if l < MAX_LEVEL {
                    let (lo, hi) = (xp_for(l), xp_for(l + 1));
                    let f = (xp - lo) as f32 / (hi - lo).max(1) as f32;
                    self.ui.rect(w / 2.0 - bw / 2.0, y + 10.0 * s, bw, 2.0 * s, [0, 0, 0, a / 2]);
                    self.ui.rect(w / 2.0 - bw / 2.0, y + 10.0 * s, bw * f, 2.0 * s, [110, 190, 255, a]);
                }
            }
        }
    }

    /// The page of crafts (Y), drawn over the rest of the HUD.
    pub(crate) fn draw_skill_page(&mut self, w: f32, h: f32) {
        if !self.skills.page || self.skills.world != self.world_name {
            return;
        }
        let s = self.ui.scale;
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 120]);
        let row = (31.0 * s).min((h - 58.0 * s) / 7.0);
        let pw = (400.0 * s).min(w - 16.0 * s);
        let ph = (row * 7.0 + 42.0 * s).min(h - 16.0 * s);
        let (px, py) = ((w - pw) / 2.0, (h - ph) / 2.0);
        self.ui.rect(px, py, pw, ph, [24, 22, 30, 235]);
        self.ui.frame(px, py, pw, ph, s, [150, 130, 90, 255]);
        self.ui.text_centered(w / 2.0, py + 5.0 * s, "Ofícios", YELLOW);
        if !self.survival {
            self.ui.text_centered(w / 2.0, py + 15.0 * s, "No modo criativo os ofícios não treinam", GRAY);
        }
        for (r, k) in ALL.iter().copied().enumerate() {
            let y = py + 27.0 * s + r as f32 * row;
            if y + row > py + ph - 8.0 * s {
                break;
            }
            if let Some(id) = self.items.id(k.icon()) {
                crate::survival::item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(id, 1), px + 6.0 * s, y + 4.0 * s, 16.0 * s);
            }
            let xp = self.skills.save.xp[k.index()];
            let l = level_of(xp);
            let tx = px + 28.0 * s;
            let right = px + pw - 8.0 * s;
            let tw = right - tx;
            self.ui.text(tx, y, &format!("{} - nível {l}", k.name()), WHITE);
            let left = if l >= MAX_LEVEL { "máximo".to_string() } else { format!("{}/{}", xp - xp_for(l), xp_for(l + 1) - xp_for(l)) };
            self.ui.text(right - self.ui.text_width(&left, s), y, &left, GRAY);
            let f = if l >= MAX_LEVEL { 1.0 } else { (xp - xp_for(l)) as f32 / (xp_for(l + 1) - xp_for(l)) as f32 };
            self.ui.rect(tx, y + 10.0 * s, tw, 2.0 * s, [50, 48, 60, 255]);
            self.ui.rect(tx, y + 10.0 * s, tw * f, 2.0 * s, if l >= MAX_LEVEL { [240, 200, 80, 255] } else { [110, 190, 255, 255] });
            let reward = if l == 0 { format!("Nível 1: {}", k.reward(1)) } else { k.reward(l) };
            let mut reward = reward;
            while self.ui.text_width(&reward, s) > tw && reward.chars().count() > 4 {
                reward.pop();
            }
            self.ui.text(tx, y + 14.0 * s, &reward, [150, 200, 150, 255]);
        }
        self.ui.text_centered(w / 2.0, py + ph - 11.0 * s, "Y ou Esc: fechar", GRAY);
    }

    /// Test hook (MC_SKILLS=xp,xp,...): sets the crafts' experience and opens their page
    /// (MC_SKILLS=page) or shows a gain above the hotbar (MC_SKILLS=gain).
    pub(crate) fn test_skills(&mut self, spec: &str) {
        self.skills_sync();
        for (i, v) in spec.split(',').filter_map(|v| v.trim().parse::<u32>().ok()).enumerate().take(7) {
            self.skills.save.xp[i] = v;
        }
        if spec.contains("gain") {
            self.skill_gain(Skill::Mining, 12);
        } else {
            self.skills.page = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_climb_and_stop_at_twenty() {
        assert_eq!(level_of(0), 0);
        assert_eq!(level_of(29), 0);
        assert_eq!(level_of(30), 1);
        assert_eq!(level_of(90), 2);
        assert_eq!(level_of(xp_for(MAX_LEVEL)), MAX_LEVEL);
        assert_eq!(level_of(u32::MAX), MAX_LEVEL);
        for l in 1..=MAX_LEVEL {
            assert!(xp_for(l) > xp_for(l - 1));
        }
    }

    #[test]
    fn rewards_stay_small() {
        // Level 20 is half again as fast at most, never twice.
        assert!(MAX_LEVEL as f32 * 0.03 <= 0.6);
        assert!(MAX_LEVEL as f32 * 0.015 <= 0.3);
        for k in ALL {
            assert!(!k.reward(MAX_LEVEL).is_empty());
        }
    }
}
