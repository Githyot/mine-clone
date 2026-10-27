//! Money (Terraria's idea): copper, silver, gold and platinum coins, a hundred of each making one
//! of the next. The player keeps them in a wallet beside the inventory that changes them up on
//! its own, so 250 copper simply shows as 2 silver 50 copper. Every monster drops its own amount
//! when it dies (tougher monsters more, bosses a small fortune; `coins` in mobs.ron sets it by
//! hand). The town folk and the village settlers sell for coins and buy things back.
//!
//! The shop screen lists each offer on its own row: what it is, how many come in one purchase and
//! the price in coins, in green when you can pay and red when you can't. Click buys one lot
//! straight into the inventory, shift-click five. Shift-click a stack of your own (or drop it on
//! the sell slot) sells it.

use glam::Vec3;
use voxel::item::{ItemId, ItemStack};
use voxel::mob::Kind;

use crate::audio::Sfx;
use crate::survival::{item_icon, Open};
use crate::Game;

pub const SILVER: u64 = 100;
pub const GOLD: u64 = 100 * SILVER;
pub const PLATINUM: u64 = 100 * GOLD;

/// The coins from the most valuable down: (item, worth in copper, name in the money text).
pub const COINS: [(&str, u64, &str); 4] = [("platinum_coin", PLATINUM, "platina"), ("gold_coin", GOLD, "ouro"), ("silver_coin", SILVER, "prata"), ("copper_coin", 1, "cobre")];

/// Prices in the shop tables, in copper.
pub const fn s(n: u32) -> u32 {
    n * SILVER as u32
}
pub const fn g(n: u32) -> u32 {
    n * GOLD as u32
}

/// Copper changed up into platinum, gold, silver and copper.
pub fn split(v: u64) -> [u64; 4] {
    [v / PLATINUM, v / GOLD % 100, v / SILVER % 100, v % 100]
}

/// "1 ouro 20 prata" (nothing below the biggest coins that are zero).
pub fn text(v: u64) -> String {
    if v == 0 {
        return "0 cobre".into();
    }
    let parts: Vec<String> = split(v).iter().zip(COINS).filter(|(n, _)| **n > 0).map(|(n, c)| format!("{n} {}", c.2)).collect();
    parts.join(" ")
}

pub fn coin_worth(name: &str) -> Option<u64> {
    COINS.iter().find(|c| c.0 == name).map(|c| c.1)
}

/// What the traders pay for things they don't sell themselves (per item, in copper).
const BUYS: &[(&str, u64)] = &[
    ("emerald", 150),
    ("diamond", 300),
    ("ruby", 120),
    ("topaz", 120),
    ("sapphire", 120),
    ("amethyst", 100),
    ("gold_ingot", 60),
    ("iron_ingot", 30),
    ("copper_ingot", 10),
    ("lapis_lazuli", 15),
    ("rotten_flesh", 2),
    ("bone", 3),
    ("string", 4),
    ("gunpowder", 8),
    ("leather", 8),
    ("feather", 3),
    ("wheat", 25),
    ("coal", 30),
    ("paper", 20),
    ("white_wool", 30),
];

/// Lots bought at once with shift-click.
const MANY: u32 = 5;
const ROW: f32 = 22.0;
pub(crate) const LIST_W: f32 = 176.0;

const DARK: [u8; 4] = [64, 64, 64, 255];
const GREY: [u8; 4] = [110, 110, 110, 255];
const CAN: [u8; 4] = [30, 120, 40, 255];
const CANT: [u8; 4] = [180, 40, 40, 255];

impl Game {
    pub(crate) fn can_pay(&self, v: u64) -> bool {
        !self.survival || self.surv.money >= v
    }

    /// Takes the money if there is enough (free in creative).
    pub(crate) fn pay(&mut self, v: u64) -> bool {
        if !self.survival {
            return true;
        }
        if self.surv.money < v {
            return false;
        }
        self.surv.money -= v;
        true
    }

    pub(crate) fn earn(&mut self, v: u64) {
        if v == 0 {
            return;
        }
        self.surv.money = self.surv.money.saturating_add(v);
        // Coins picked up in a row add up in the flash beside the hotbar.
        let sum = if self.surv.money_flash.1 > self.now { self.surv.money_flash.0 } else { 0 };
        self.surv.money_flash = (sum + v, self.now + 2.5);
    }

    /// Coins tossed out of `at`, as few as make up `v`.
    pub(crate) fn drop_coins(&mut self, at: Vec3, v: u64) {
        for (n, c) in split(v).into_iter().zip(COINS) {
            let Some(id) = self.items.id(c.0) else { continue };
            let mut n = n;
            while n > 0 {
                let k = n.min(100);
                n -= k;
                let vel = Vec3::new(self.surv.rng.f32() - 0.5, 3.5, self.surv.rng.f32() - 0.5);
                self.spawn_drop(at, ItemStack::new(id, k as u8), vel);
            }
        }
    }

    /// A coin stack picked up goes straight into the wallet.
    pub(crate) fn pocket_coins(&mut self, st: &ItemStack) -> bool {
        let Some(w) = coin_worth(&self.items.get(st.item).name) else { return false };
        self.earn(w * st.count as u64);
        true
    }

    /// What this monster drops when it dies: `coins` from mobs.ron, or worked out from how tough
    /// it is. Animals and town folk carry nothing.
    pub(crate) fn mob_coins(&mut self, def: usize) -> u64 {
        let d = &self.mobs.reg.defs[def];
        let base = match d.coins {
            Some(c) => c as f32,
            None if d.town || d.kind == Kind::Passive => 0.0,
            None => {
                let mut v = d.health * 3.0 + d.attack * 10.0;
                if d.explodes {
                    v += 40.0;
                }
                if d.ranged {
                    v += 20.0;
                }
                if d.boss {
                    v *= 25.0;
                }
                v
            }
        };
        if base <= 0.0 {
            return 0;
        }
        let mut k = 0.75 + self.surv.rng.f32() * 0.5;
        if self.t5.hard.on {
            k *= 1.5;
        }
        if self.wears("lucky_coin") {
            k *= 1.25;
        }
        (base * k).round().max(1.0) as u64
    }

    /// What a trader pays for one of these, if they want it: the fixed list, what the open
    /// settler is after, or a fifth of what any shop sells it for.
    pub(crate) fn sell_price(&self, item: ItemId) -> Option<u64> {
        let name = self.items.get(item).name.as_str();
        if coin_worth(name).is_some() {
            return None;
        }
        if let Some((w, p)) = self.surv.shop_want {
            if w == item {
                return Some(p);
            }
        }
        if let Some(b) = BUYS.iter().find(|b| b.0 == name) {
            return Some(b.1);
        }
        let shops = crate::npcs::SHOPS.iter().chain(crate::hardmode::SHOPS).map(|s| s.1).chain(crate::villages::SETTLERS.iter().map(|s| s.1));
        let mut best: Option<u64> = None;
        for offers in shops {
            for &(n, count, price) in offers {
                if n == name {
                    let v = (price as u64 / count.max(1) as u64 / 5).max(1);
                    best = Some(best.map_or(v, |b| b.min(v)));
                }
            }
        }
        best
    }

    /// Sells a whole stack; what's left (when nobody wants it) comes back.
    fn sell_stack(&mut self, st: ItemStack) -> Option<ItemStack> {
        let Some(unit) = self.sell_price(st.item) else {
            self.toast(format!("{}: não compro isso", self.surv.shop_name), 2.0);
            return Some(st);
        };
        let v = unit * st.count as u64;
        self.earn(v);
        let name = self.item_label(&st);
        self.toast(format!("Vendeu {name} x{} por {}", st.count, text(v)), 2.5);
        self.sfx(Sfx::Fx(crate::sfx_extra::Fx::Armor(true)));
        None
    }

    /// Shift-click on an inventory slot while a shop is open sells it.
    pub(crate) fn sell_slot(&mut self, k: usize) {
        let Some(st) = self.surv.inv.slots.get(k).copied().flatten() else { return };
        self.surv.inv.slots[k] = self.sell_stack(st);
    }

    /// Buys `lots` of offer `i` (as many as you can pay for), into the inventory.
    pub(crate) fn buy(&mut self, i: usize, lots: u32) {
        let (Some(good), Some(&price)) = (self.surv.shop.slots.get(i).copied().flatten(), self.surv.shop_prices.get(i)) else { return };
        let price = price as u64;
        let mut bought = 0;
        for _ in 0..lots {
            if !self.pay(price) {
                break;
            }
            bought += 1;
            if let Some(left) = self.surv.inv.add(&self.items, good) {
                self.pop_drop(self.body.pos + Vec3::Y, left);
            }
        }
        if bought == 0 {
            let need = price.saturating_sub(self.surv.money);
            self.toast(format!("Falta {} para comprar isso", text(need)), 2.0);
            return;
        }
        let name = self.item_label(&good);
        self.toast(format!("Comprou {name} x{} por {}", good.count as u32 * bought, text(price * bought as u64)), 2.5);
        self.sfx(Sfx::Fx(crate::sfx_extra::Fx::Armor(true)));
        self.prog.stats.add("trades", bought as u64);
        self.prog.event(crate::progression::Event::Token("traded".into()));
    }

    /// An amount in coins, its right edge at `xr`: "1 [gold] 20 [silver]". Returns its width.
    pub(crate) fn draw_money(&mut self, xr: f32, y: f32, v: u64, col: [u8; 4], shadow: bool) -> f32 {
        let s = self.ui.scale;
        let parts: Vec<(u64, &str)> = split(v).into_iter().zip(COINS).filter(|(n, _)| *n > 0).map(|(n, c)| (n, c.0)).collect();
        let parts = if parts.is_empty() { vec![(0, "copper_coin")] } else { parts };
        let icon = 10.0 * s;
        let widths: Vec<f32> = parts.iter().map(|(n, _)| self.ui.text_width(&n.to_string(), s) + s + icon + 2.0 * s).collect();
        let total: f32 = widths.iter().sum::<f32>() - 2.0 * s;
        let mut x = xr - total;
        for ((n, coin), wd) in parts.into_iter().zip(widths) {
            let t = n.to_string();
            if shadow {
                self.ui.text(x, y, &t, col);
            } else {
                self.ui.label(x, y, &t, col, s);
            }
            let tw = self.ui.text_width(&t, s);
            if let Some(id) = self.items.id(coin) {
                item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(id, 1), x + tw + s, y - s, icon);
            }
            x += wd;
        }
        total
    }

    /// The wallet: four coin slots stuck to the right of the inventory panel. Click takes coins
    /// out (right click one), click with coins on the cursor puts them back.
    /// Returns whether the mouse is over it.
    pub(crate) fn draw_wallet(&mut self, x: f32, y: f32, mouse: (f32, f32)) -> bool {
        let s = self.ui.scale;
        let cell = 18.0 * s;
        let (pw, ph) = (24.0 * s, (4.0 * 18.0 + 15.0) * s);
        self.ui.rect(x, y, pw, ph, [198, 198, 198, 255]);
        self.ui.frame(x, y, pw, ph, s, [255, 255, 255, 255]);
        self.ui.rect(x + pw - s, y, s, ph, [85, 85, 85, 255]);
        self.ui.rect(x, y + ph - s, pw, s, [85, 85, 85, 255]);
        let tw = self.ui.text_width("$", s);
        self.ui.label(x + (pw - tw) / 2.0, y + 3.0 * s, "$", DARK, s);
        let counts = split(self.surv.money);
        let mut over_any = false;
        let mut hovered: Option<usize> = None;
        for (k, c) in COINS.iter().enumerate() {
            let (sx, sy) = (x + 4.0 * s, y + (13.0 + k as f32 * 18.0) * s);
            self.ui.rect(sx - s, sy - s, cell, cell, [55, 55, 55, 255]);
            self.ui.rect(sx, sy, cell - s, cell - s, [255, 255, 255, 255]);
            self.ui.rect(sx, sy, cell - 2.0 * s, cell - 2.0 * s, [139, 139, 139, 255]);
            let over = mouse.0 >= sx && mouse.0 < sx + cell - s && mouse.1 >= sy && mouse.1 < sy + cell - s;
            if over {
                over_any = true;
                hovered = Some(k);
                self.ui.rect(sx, sy, cell - 2.0 * s, cell - 2.0 * s, [170, 170, 170, 255]);
            }
            let Some(id) = self.items.id(c.0) else { continue };
            item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(id, 1), sx, sy, 16.0 * s);
            let n = counts[k];
            if n == 0 {
                // An empty compartment shows its coin faded.
                self.ui.rect(sx, sy, cell - 2.0 * s, cell - 2.0 * s, [139, 139, 139, 170]);
            } else if n > 1 {
                let t = n.to_string();
                let tw = self.ui.text_width(&t, s);
                self.ui.text(sx + 17.0 * s - tw, sy + 9.0 * s, &t, [255, 255, 255, 255]);
            }
        }
        if let Some(k) = hovered {
            let c = COINS[k];
            if self.clicked || self.rclicked {
                self.wallet_click(k, self.rclicked && !self.clicked);
            } else if self.surv.cursor.is_none() {
                let name = self.items.id(c.0).map(|id| self.items.get(id).display_name.clone()).unwrap_or_default();
                let lines = vec![
                    (format!("{name}: {}", counts[k]), [255, 255, 255, 255]),
                    (format!("Total: {}", text(self.surv.money)), [255, 220, 120, 255]),
                    ("Clique: tirar moedas  Dir.: tirar uma".to_string(), [170, 170, 170, 255]),
                ];
                self.draw_tooltip_lines(&lines, mouse);
            }
        }
        over_any
    }

    fn wallet_click(&mut self, k: usize, right: bool) {
        let (name, worth, _) = COINS[k];
        let Some(id) = self.items.id(name) else { return };
        // Coins on the cursor (any kind) go in.
        if let Some(c) = self.surv.cursor {
            if self.pocket_coins(&c) {
                self.surv.cursor = None;
                self.sfx(Sfx::Fx(crate::sfx_extra::Fx::Armor(true)));
            }
            return;
        }
        // Out come as many of this coin as the money makes (breaking bigger ones if needed).
        let have = self.surv.money / worth;
        let n = if right { have.min(1) } else { have.min(100) };
        if n == 0 {
            return;
        }
        self.surv.money -= n * worth;
        self.surv.cursor = Some(ItemStack::new(id, n as u8));
        self.sfx(Sfx::Fx(crate::sfx_extra::Fx::Armor(true)));
    }

    /// The shop: the offers in a list on the left of the inventory and the sell slot on top.
    /// Returns whether the mouse is over any of it.
    pub(crate) fn draw_shop(&mut self, x0: f32, y0: f32, ph: f32, mouse: (f32, f32)) -> bool {
        let s = self.ui.scale;
        let cell = 18.0 * s;
        let shift = self.keys.contains(&winit::keyboard::KeyCode::ShiftLeft) || self.keys.contains(&winit::keyboard::KeyCode::ShiftRight);
        // Top of the inventory panel: who, your money, the sell slot.
        let title = self.surv.shop_name.clone();
        self.ui.label(x0 + 8.0 * s, y0 + 6.0 * s, &title, DARK, s);
        let tw = self.ui.text_width("Loja", s);
        self.ui.label(x0 + 168.0 * s - tw, y0 + 6.0 * s, "Loja", GREY, s);
        self.ui.label(x0 + 8.0 * s, y0 + 20.0 * s, "Você tem:", DARK, s);
        let money = self.surv.money;
        let mx = x0 + 8.0 * s + self.ui.text_width("Você tem: ", s);
        let w = self.money_width(money);
        self.draw_money(mx + w, y0 + 20.0 * s, money, [140, 100, 10, 255], false);
        // Sell slot.
        let (sx, sy) = (x0 + 8.0 * s, y0 + 36.0 * s);
        self.ui.rect(sx - s, sy - s, cell, cell, [55, 55, 55, 255]);
        self.ui.rect(sx, sy, cell - s, cell - s, [255, 255, 255, 255]);
        self.ui.rect(sx, sy, cell - 2.0 * s, cell - 2.0 * s, [139, 139, 139, 255]);
        let over_sell = mouse.0 >= sx && mouse.0 < sx + cell - s && mouse.1 >= sy && mouse.1 < sy + cell - s;
        if over_sell {
            self.ui.rect(sx, sy, cell - 2.0 * s, cell - 2.0 * s, [170, 170, 170, 255]);
        }
        if let Some(id) = self.items.id("gold_coin") {
            item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(id, 1), sx + 3.0 * s, sy + 3.0 * s, 10.0 * s);
        }
        let (line1, line2, col) = match self.surv.cursor {
            Some(c) => match self.sell_price(c.item) {
                Some(p) => ("Vender por:".to_string(), Some(p * c.count as u64), CAN),
                None => ("Não compro isso".to_string(), None, CANT),
            },
            None => ("Vender: solte aqui".to_string(), None, DARK),
        };
        self.ui.label(sx + 22.0 * s, sy + 1.0 * s, &line1, col, s);
        match line2 {
            Some(v) => {
                let w = self.money_width(v);
                self.draw_money(sx + 22.0 * s + w, sy + 10.0 * s, v, CAN, false);
            }
            None => self.ui.label(sx + 22.0 * s, sy + 10.0 * s, "ou Shift+clique", GREY, s),
        }
        if over_sell && self.clicked {
            if let Some(c) = self.surv.cursor.take() {
                self.surv.cursor = self.sell_stack(c);
            }
        }

        // The offers, one per row.
        let n = self.surv.shop.slots.len();
        let lw = LIST_W * s;
        let lx = (x0 - lw - 4.0 * s).max(2.0 * s);
        let head = 16.0 * s;
        let fit = (((ph - head - 4.0 * s) / (ROW * s)).floor() as usize).max(1);
        let max_scroll = n.saturating_sub(fit);
        self.surv.shop_scroll = self.surv.shop_scroll.min(max_scroll);
        let first = self.surv.shop_scroll;
        self.ui.rect(lx, y0, lw, ph, [198, 198, 198, 255]);
        self.ui.frame(lx, y0, lw, ph, s, [255, 255, 255, 255]);
        self.ui.rect(lx + lw - s, y0, s, ph, [85, 85, 85, 255]);
        self.ui.rect(lx, y0 + ph - s, lw, s, [85, 85, 85, 255]);
        self.ui.label(lx + 7.0 * s, y0 + 6.0 * s, "À venda", DARK, s);
        let tq = "Preço";
        let tqw = self.ui.text_width(tq, s);
        self.ui.label(lx + lw - 8.0 * s - tqw, y0 + 6.0 * s, tq, GREY, s);
        let over_list = mouse.0 >= lx && mouse.0 < lx + lw && mouse.1 >= y0 && mouse.1 < y0 + ph;
        self.surv.shop_hover = over_list;
        let mut tip: Option<ItemStack> = None;
        let rw = lw - if max_scroll > 0 { 12.0 * s } else { 8.0 * s };
        for k in first..(first + fit).min(n) {
            let Some(good) = self.surv.shop.slots[k] else { continue };
            let price = self.surv.shop_prices.get(k).copied().unwrap_or(0) as u64;
            let ok = self.can_pay(price);
            let (rx, ry) = (lx + 4.0 * s, y0 + head + (k - first) as f32 * ROW * s);
            let over = mouse.0 >= rx && mouse.0 < rx + rw && mouse.1 >= ry && mouse.1 < ry + (ROW - 1.0) * s;
            // Rows you can pay for look raised; the others sink back.
            let bg = match (ok, over) {
                (true, true) => [222, 236, 214, 255],
                (true, false) => [214, 214, 214, 255],
                (false, true) => [190, 170, 170, 255],
                (false, false) => [168, 168, 168, 255],
            };
            self.ui.rect(rx, ry, rw, (ROW - 1.0) * s, bg);
            self.ui.rect(rx, ry + (ROW - 1.0) * s - s, rw, s, [139, 139, 139, 255]);
            if over {
                self.ui.frame(rx, ry, rw, (ROW - 1.0) * s, s, if ok { [60, 150, 60, 255] } else { [170, 60, 60, 255] });
            }
            // The item in a slot, with how many come in one purchase.
            let (ix, iy) = (rx + 2.0 * s, ry + 2.0 * s);
            self.ui.rect(ix, iy, cell - s, cell - s, [139, 139, 139, 255]);
            self.ui.rect(ix, iy, cell - s, s, [55, 55, 55, 255]);
            self.ui.rect(ix, iy, s, cell - s, [55, 55, 55, 255]);
            // The count is written beside it, so the icon shows just one.
            item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack { count: 1, ..good }, ix + 0.5 * s, iy + 0.5 * s, 16.0 * s);
            if !ok {
                self.ui.rect(ix, iy, cell - s, cell - s, [120, 120, 120, 120]);
            }
            let mut name = self.item_label(&good);
            let room = rw - 26.0 * s;
            if self.ui.text_width(&name, s) > room {
                while name.chars().count() > 1 && self.ui.text_width(&format!("{name}.."), s) > room {
                    name.pop();
                }
                name = format!("{}..", name.trim_end());
            }
            self.ui.label(rx + 22.0 * s, ry + 2.0 * s, &name, if ok { DARK } else { [90, 90, 90, 255] }, s);
            self.ui.label(rx + 22.0 * s, ry + 11.0 * s, &format!("x{}", good.count), GREY, s);
            self.draw_money(rx + rw - 3.0 * s, ry + 11.0 * s, price, if ok { CAN } else { CANT }, false);
            // The item's details when the mouse is on its picture (not over the prices below).
            if over && mouse.0 < ix + cell {
                tip = Some(good);
            }
            if over {
                if self.clicked {
                    self.buy(k, if shift { MANY } else { 1 });
                }
            }
        }
        // Scroll bar when there are more offers than rows.
        if max_scroll > 0 {
            let (bx, by, bh) = (lx + lw - 7.0 * s, y0 + head, fit as f32 * ROW * s - s);
            self.ui.rect(bx, by, 4.0 * s, bh, [139, 139, 139, 255]);
            let kh = (bh * fit as f32 / n as f32).max(6.0 * s);
            let ky = by + (bh - kh) * first as f32 / max_scroll as f32;
            self.ui.rect(bx, ky, 4.0 * s, kh, [90, 90, 90, 255]);
            let t = "Roda: mais ofertas";
            let tw = self.ui.text_width(t, s);
            self.ui.label(lx + (lw - tw) / 2.0, y0 + ph - 10.0 * s, t, GREY, s);
        }
        if let (Some(st), None) = (tip, self.surv.cursor) {
            let mut lines = vec![(self.item_label(&st), self.gem_name_color(&st))];
            lines.extend(self.item_details(&st));
            self.draw_tooltip_lines(&lines, mouse);
        }
        over_list || over_sell
    }

    fn money_width(&self, v: u64) -> f32 {
        let s = self.ui.scale;
        let parts: Vec<u64> = split(v).into_iter().filter(|n| *n > 0).collect();
        let parts = if parts.is_empty() { vec![0] } else { parts };
        parts.iter().map(|n| self.ui.text_width(&n.to_string(), s) + s + 10.0 * s + 2.0 * s).sum::<f32>() - 2.0 * s
    }

    /// Coins just picked up, beside the hotbar for a moment.
    pub(crate) fn draw_money_flash(&mut self, hx: f32, hy: f32, cell: f32) {
        let (v, until) = self.surv.money_flash;
        let t = until - self.now;
        if t <= 0.0 || v == 0 {
            return;
        }
        let s = self.ui.scale;
        let a = (t / 0.5).min(1.0);
        let col = [255, 225, 120, (255.0 * a) as u8];
        let w = self.money_width(v) + self.ui.text_width("+", s);
        let x = hx + cell * 9.0 + 6.0 * s;
        self.ui.text(x, hy + 5.0 * s, "+", col);
        self.draw_money(x + w, hy + 5.0 * s, v, col, true);
    }

    /// Whether the open screen is a shop (for the wheel).
    pub(crate) fn shop_wheel(&mut self, up: bool) -> bool {
        if self.surv.open != Open::Shop || !self.surv.shop_hover {
            return false;
        }
        self.surv.shop_scroll = if up { self.surv.shop_scroll.saturating_sub(1) } else { self.surv.shop_scroll + 1 };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coins_change_up() {
        assert_eq!(split(1_02_03_04), [1, 2, 3, 4]);
        assert_eq!(split(250), [0, 0, 2, 50]);
        assert_eq!(text(250), "2 prata 50 cobre");
        assert_eq!(text(GOLD), "1 ouro");
        assert_eq!(s(3), 300);
        assert_eq!(g(1), 10_000);
    }

    #[test]
    fn every_coin_and_trade_item_exists() {
        let (_, items, _) = crate::game_data(crate::embedded::BLOCKS_RON, crate::embedded::ITEMS_RON, crate::embedded::RECIPES_RON).unwrap();
        for c in COINS {
            assert!(items.id(c.0).is_some(), "no item {}", c.0);
        }
        for b in BUYS {
            assert!(items.id(b.0).is_some(), "no item {}", b.0);
        }
        for t in crate::villages::SETTLERS {
            for g in t.1 {
                assert!(items.id(g.0).is_some(), "no item {}", g.0);
            }
        }
    }
}
