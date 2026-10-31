//! Lote 15 of the Terraria side, second round (plan/terraria-completo.md): Journey research.
//!
//! - The Research Altar turns its world into a Journey world (asked once, for good). Then a
//!   click with a stack in hand gives it to research: one of a tool or armour, ten of a
//!   potion, fifty of anything else. Once researched, a click while sneaking gives a full
//!   stack of it, as often as you like. A click with an empty hand shows the progress.
//! - The Stone of Powers (Journey worlds only): sneak-click picks a power, a click uses it:
//!   dawn, noon, dusk, midnight, freeze time, rain, invincible, and how many monsters come.

use std::collections::HashMap;

use glam::Vec3;
use voxel::item::ItemStack;

use crate::audio::Sfx;
use crate::Game;

const POWERS: &[&str] = &["Amanhecer", "Meio-dia", "Anoitecer", "Meia-noite", "Congelar o tempo", "Chuva", "Invencível", "Quantidade de monstros"];
const SPAWNS: &[(f32, &str)] = &[(1.0, "normal"), (0.4, "poucos"), (0.0, "nenhum"), (2.0, "muitos")];

#[derive(Default)]
pub struct Journey {
    pub on: bool,
    /// How many of each item were given to research.
    pub research: HashMap<String, u32>,
    /// Asked to confirm turning the world into a Journey world.
    confirm: bool,
    power: usize,
    pub freeze: Option<f32>,
    pub god: bool,
    pub spawn: usize,
}

impl Journey {
    pub fn spawn_factor(&self) -> f32 {
        if self.on {
            SPAWNS[self.spawn % SPAWNS.len()].0
        } else {
            1.0
        }
    }
}

pub(crate) fn is_block(name: &str) -> bool {
    matches!(name, "research_altar" | "power_stone")
}

impl Game {
    /// How many of an item it takes to research it.
    fn research_need(&self, item: voxel::item::ItemId) -> u32 {
        match self.items.get(item).vanilla_stack {
            0 | 1 => 1,
            2..=16 => 10,
            _ => 50,
        }
    }

    pub(crate) fn update_tr_journey(&mut self) {
        let j = &self.t5.tr.journey;
        if !j.on {
            return;
        }
        if let Some(t) = j.freeze {
            self.time_of_day = t;
        }
        if j.god && self.survival {
            self.health = self.max_health;
        }
    }

    pub(crate) fn use_journey_block(&mut self, name: &str) -> bool {
        let sneak = self.keys.contains(&winit::keyboard::KeyCode::ShiftLeft) || self.keys.contains(&winit::keyboard::KeyCode::ShiftRight);
        if !self.t5.tr.journey.on {
            if !self.t5.tr.journey.confirm {
                self.t5.tr.journey.confirm = true;
                self.toast("Isto transforma este mundo num Mundo Jornada, para sempre: pesquisar itens e copiá-los, e usar os poderes. Clique de novo para confirmar.", 6.0);
                return true;
            }
            self.t5.tr.journey.on = true;
            self.toast("Este agora é um Mundo Jornada!", 3.0);
            self.sfx(Sfx::Xp);
            return true;
        }
        if name == "power_stone" {
            if sneak {
                let j = &mut self.t5.tr.journey;
                j.power = (j.power + 1) % POWERS.len();
                let p = POWERS[j.power];
                self.toast(format!("Poder: {p} (clique para usar)"), 2.5);
                self.sfx(Sfx::Click);
            } else {
                self.use_power();
            }
            return true;
        }
        let Some(st) = self.held_stack() else {
            let n = self.t5.tr.journey.research.iter().filter(|(k, &v)| self.items.id(k).is_some_and(|id| v >= self.research_need(id))).count();
            self.toast(format!("Pesquisa: {n} itens completos. Clique com uma pilha na mão para pesquisar; agachado, para copiar um item já pesquisado."), 5.0);
            return true;
        };
        let item_name = self.items.get(st.item).name.clone();
        let need = self.research_need(st.item);
        let have = self.t5.tr.journey.research.get(&item_name).copied().unwrap_or(0);
        let display = self.items.get(st.item).display_name.clone();
        if sneak {
            if have < need {
                self.toast(format!("{display}: {have}/{need} pesquisados"), 2.5);
                return true;
            }
            let max = self.items.get(st.item).max_stack.max(1);
            let at = self.body.eye() + self.forward() * 0.6;
            self.spawn_drop(at, ItemStack { count: max, ..st }, Vec3::Y);
            self.sfx(Sfx::Pickup);
            return true;
        }
        if have >= need {
            self.toast(format!("{display} já foi pesquisado. Agache e clique para copiar."), 2.5);
            return true;
        }
        let give = (need - have).min(st.count as u32) as u16;
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, give);
        }
        let now = have + give as u32;
        self.t5.tr.journey.research.insert(item_name, now);
        if now >= need {
            self.toast(format!("Pesquisado: {display}! Agache e clique para copiar."), 3.0);
            self.sfx(Sfx::Xp);
            let glow = self.glow_layer();
            let at = self.body.eye() + self.forward() * 1.2;
            for i in 0..24 {
                let a = i as f32 / 24.0 * std::f32::consts::TAU;
                self.particles.glow(at, Vec3::new(a.cos(), 1.0, a.sin()) * 2.0, glow, [1.6, 1.4, 0.6], 0.08, 0.6);
            }
        } else {
            self.toast(format!("{display}: {now}/{need} pesquisados"), 2.0);
            self.sfx(Sfx::Place(crate::audio::Material::Cloth));
        }
        true
    }

    fn use_power(&mut self) {
        let j = &mut self.t5.tr.journey;
        let text = match j.power {
            k @ 0..=3 => {
                let t = [0.0, 6000.0, 12000.0, 18000.0][k];
                self.time_of_day = t;
                if j.freeze.is_some() {
                    j.freeze = Some(t);
                }
                POWERS[k].to_string()
            }
            4 => {
                j.freeze = if j.freeze.is_some() { None } else { Some(self.time_of_day) };
                if j.freeze.is_some() { "Tempo parado" } else { "O tempo volta a correr" }.to_string()
            }
            5 => {
                let w = &mut self.weather.state;
                w.raining = !w.raining;
                w.timer = 600.0;
                if w.raining { "Chuva" } else { "Tempo firme" }.to_string()
            }
            6 => {
                j.god = !j.god;
                if j.god { "Invencível" } else { "Vulnerável de novo" }.to_string()
            }
            _ => {
                j.spawn = (j.spawn + 1) % SPAWNS.len();
                format!("Monstros: {}", SPAWNS[j.spawn].1)
            }
        };
        self.toast(text, 2.5);
        self.sfx(Sfx::Cast(2));
    }
}
