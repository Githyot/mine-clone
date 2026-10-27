//! Item modifiers (Terraria's prefixes, Apotheosis' affixes): a weapon, caster or accessory
//! comes out of the crafting grid with a random modifier that changes its damage, speed,
//! critical chance or mana cost. At the reforge table two emeralds roll a new one.
//! The modifier lives in the top 4 bits of the stack's `damage` (see `ItemStack::affix`), so
//! saves and stacks need nothing new.

use voxel::block::ToolKind;
use voxel::item::ItemStack;

use crate::audio::{Material, Sfx};
use crate::Game;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Melee,
    Magic,
    Accessory,
}

pub struct Affix {
    pub name: &'static str,
    class: Class,
    /// Multipliers.
    pub damage: f32,
    pub speed: f32,
    pub mana: f32,
    pub move_speed: f32,
    /// Extra chance of a critical hit (0..1).
    pub crit: f32,
    pub armor: u32,
}

const fn a(name: &'static str, class: Class, damage: f32, speed: f32, crit: f32, mana: f32) -> Affix {
    Affix { name, class, damage, speed, mana, move_speed: 1.0, crit, armor: 0 }
}

/// Index + 1 is what is stored on the stack.
const AFFIXES: [Affix; 15] = [
    a("Afiado", Class::Melee, 1.15, 1.0, 0.0, 1.0),
    a("Lendário", Class::Melee, 1.2, 1.1, 0.05, 1.0),
    a("Veloz", Class::Melee, 1.0, 1.2, 0.0, 1.0),
    a("Cruel", Class::Melee, 1.1, 1.0, 0.1, 1.0),
    a("Pesado", Class::Melee, 1.15, 0.9, 0.0, 1.0),
    a("Mortal", Class::Melee, 1.1, 1.05, 0.05, 1.0),
    a("Quebrado", Class::Melee, 0.8, 1.0, 0.0, 1.0),
    a("Lento", Class::Melee, 1.0, 0.85, 0.0, 1.0),
    a("Arcano", Class::Magic, 1.15, 1.0, 0.0, 0.9),
    a("Místico", Class::Magic, 1.1, 1.0, 0.0, 0.85),
    a("Mítico", Class::Magic, 1.15, 1.1, 0.0, 0.9),
    a("Desperdiçado", Class::Magic, 1.0, 1.0, 0.0, 1.2),
    a("Preciso", Class::Melee, 1.0, 1.0, 0.1, 1.0),
    Affix { name: "Guardião", class: Class::Accessory, damage: 1.0, speed: 1.0, mana: 1.0, move_speed: 1.0, crit: 0.0, armor: 2 },
    Affix { name: "Ágil", class: Class::Accessory, damage: 1.0, speed: 1.0, mana: 1.0, move_speed: 1.05, crit: 0.0, armor: 0 },
];

/// The modifier on a stack, if it has one.
pub fn of(st: &ItemStack) -> Option<&'static Affix> {
    let k = st.affix() as usize;
    (k > 0).then(|| AFFIXES.get(k - 1)).flatten()
}

fn pct(k: f32) -> String {
    let p = ((k - 1.0) * 100.0).round() as i32;
    if p >= 0 {
        format!("+{p}%")
    } else {
        format!("{p}%")
    }
}

impl Game {
    fn affix_class(&self, st: &ItemStack) -> Option<Class> {
        let it = self.items.get(st.item);
        if crate::accessories::is_accessory(&it.name) {
            return Some(Class::Accessory);
        }
        if crate::magic::is_caster(&it.name) || it.name == "glyph_book" {
            return Some(Class::Magic);
        }
        match it.tool.as_ref().map(|t| t.kind) {
            Some(ToolKind::Sword | ToolKind::Axe) => Some(Class::Melee),
            _ => None,
        }
    }

    /// A fresh modifier for something just made (none for everything else).
    pub(crate) fn roll_affix(&mut self, st: ItemStack) -> ItemStack {
        let Some(class) = self.affix_class(&st) else { return st };
        // One in five comes out plain.
        if self.surv.rng.f32() < 0.2 {
            return st;
        }
        self.pick_affix(st, class, 0)
    }

    fn pick_affix(&mut self, st: ItemStack, class: Class, not: u8) -> ItemStack {
        let choices: Vec<u8> = (0..AFFIXES.len()).filter(|&i| AFFIXES[i].class == class && i as u8 + 1 != not).map(|i| i as u8 + 1).collect();
        if choices.is_empty() {
            return st;
        }
        let k = choices[self.surv.rng.below(choices.len() as u32) as usize];
        st.with_affix(k)
    }

    /// Right click on the reforge table: two emeralds for a new modifier on the held item.
    pub(crate) fn reforge_held(&mut self) -> bool {
        let sel = self.selected;
        let Some(st) = self.surv.inv.slots[sel] else {
            self.toast("Segure uma arma, cajado ou acessório para reforjar (2 esmeraldas)", 3.0);
            return true;
        };
        let Some(class) = self.affix_class(&st) else {
            self.toast("Isso não pode ser reforjado", 2.5);
            return true;
        };
        if self.survival {
            let Some(em) = self.items.id("emerald") else { return true };
            let have: u32 = self.surv.inv.slots.iter().flatten().filter(|s| s.item == em).map(|s| s.count as u32).sum();
            if have < 2 {
                self.toast("Reforjar custa 2 esmeraldas", 2.5);
                return true;
            }
            let mut left = 2;
            for slot in self.surv.inv.slots.iter_mut() {
                if let Some(s) = slot {
                    if s.item == em && left > 0 {
                        let n = s.count.min(left);
                        s.count -= n;
                        left -= n;
                        if s.count == 0 {
                            *slot = None;
                        }
                    }
                }
            }
        }
        let new = self.pick_affix(st, class, st.affix());
        self.surv.inv.slots[sel] = Some(new);
        let label = self.item_label(&new);
        self.toast(format!("Reforjado: {label}"), 3.0);
        self.sfx(Sfx::Hit(Material::Metal));
        self.anim.start_swing();
        true
    }

    /// The item's name with its modifier.
    pub(crate) fn item_label(&self, st: &ItemStack) -> String {
        let n = self.items.get(st.item).display_name.clone();
        match of(st) {
            Some(a) if self.affix_class(st).is_some() => format!("{n} · {}", a.name),
            _ => n,
        }
    }

    /// Tooltip lines of the Terraria side: modifier effects, what an accessory does.
    pub(crate) fn t5_details(&self, st: &ItemStack, out: &mut Vec<(String, [u8; 4])>) {
        const GOOD: [u8; 4] = [120, 220, 120, 255];
        const BAD: [u8; 4] = [230, 100, 90, 255];
        let name = &self.items.get(st.item).name;
        if let Some(d) = crate::magic::describe(name) {
            out.push((d, [170, 200, 255, 255]));
        }
        if self.affix_class(st).is_none() {
            return;
        }
        let Some(a) = of(st) else { return };
        let mut line = |k: f32, what: &str, higher_good: bool| {
            if (k - 1.0).abs() > 0.001 {
                let good = (k > 1.0) == higher_good;
                out.push((format!("{} {what}", pct(k)), if good { GOOD } else { BAD }));
            }
        };
        line(a.damage, "de dano", true);
        line(a.speed, "de velocidade", true);
        line(a.mana, "de custo de mana", false);
        line(a.move_speed, "de velocidade ao andar", true);
        if a.crit > 0.0 {
            out.push((format!("+{:.0}% de chance de crítico", a.crit * 100.0), GOOD));
        }
        if a.armor > 0 {
            out.push((format!("+{} de armadura", a.armor), GOOD));
        }
    }

    /// Extra critical chance of the held weapon.
    pub(crate) fn held_crit(&self) -> f32 {
        self.held_stack().and_then(|s| of(&s)).map(|a| a.crit).unwrap_or(0.0)
    }

    /// Damage and mana multipliers of the held caster's modifier, with the arcane emblem.
    pub(crate) fn caster_mods(&self) -> (f32, f32, f32) {
        let a = self.held_stack().and_then(|s| of(&s));
        let dmg = a.map(|a| a.damage).unwrap_or(1.0) * self.magic_bonus();
        (dmg, a.map(|a| a.mana).unwrap_or(1.0), a.map(|a| a.speed).unwrap_or(1.0))
    }
}
