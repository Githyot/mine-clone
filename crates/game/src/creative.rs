//! The creative inventory, laid out like the classic one: seven tabs on top (building, coloured,
//! natural, functional, doors and lights, saved hotbars, search), six below (tools, combat, food,
//! ingredients, spawn eggs and the player's own inventory). Item tabs show a 9×5 grid that scrolls;
//! the inventory tab shows the player in 3D with armour, second hand, the rows and a bin.

use voxel::block::{Shape, ToolKind};
use voxel::item::{ItemId, ItemStack};
use winit::keyboard::KeyCode;

use crate::survival::{item_icon, rect_around, Open, Slot};
use crate::ui::WHITE;
use crate::{Game, Screen};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Building,
    Colored,
    Natural,
    Functional,
    Doors,
    Saved,
    Search,
    Tools,
    Combat,
    Food,
    Ingredients,
    Eggs,
    Inventory,
}

/// Top row, then bottom row; the index is what `Survival::ctab` stores.
pub const TABS: [Tab; 13] = [
    Tab::Building,
    Tab::Colored,
    Tab::Natural,
    Tab::Functional,
    Tab::Doors,
    Tab::Saved,
    Tab::Search,
    Tab::Tools,
    Tab::Combat,
    Tab::Food,
    Tab::Ingredients,
    Tab::Eggs,
    Tab::Inventory,
];

impl Tab {
    fn title(self) -> &'static str {
        match self {
            Tab::Building => "Blocos de construção",
            Tab::Colored => "Blocos coloridos",
            Tab::Natural => "Blocos naturais",
            Tab::Functional => "Blocos funcionais",
            Tab::Doors => "Portas e luzes",
            Tab::Saved => "Barras salvas",
            Tab::Search => "Buscar",
            Tab::Tools => "Ferramentas",
            Tab::Combat => "Combate",
            Tab::Food => "Comidas",
            Tab::Ingredients => "Ingredientes",
            Tab::Eggs => "Ovos de criatura",
            Tab::Inventory => "Inventário",
        }
    }

    /// Item shown on the tab.
    fn icon(self) -> &'static str {
        match self {
            Tab::Building => "bricks",
            Tab::Colored => "cyan_wool",
            Tab::Natural => "grass_block",
            Tab::Functional => "crafting_table",
            Tab::Doors => "oak_door",
            Tab::Saved => "bookshelf",
            Tab::Search => "",
            Tab::Tools => "diamond_pickaxe",
            Tab::Combat => "diamond_sword",
            Tab::Food => "apple",
            Tab::Ingredients => "iron_ingot",
            Tab::Eggs => "zombie_spawn_egg",
            Tab::Inventory => "chest",
        }
    }

    /// Left edge of the tab in panel pixels, and whether it hangs below the panel.
    fn place(i: usize) -> (f32, bool) {
        let col = if i < 7 { i } else { i - 7 };
        let bottom = i >= 7;
        // The last tabs of each row sit at the right edge, like the classic screen.
        let x = match (bottom, col) {
            (false, 5) => 140.0,
            (false, 6) | (true, 5) => 169.0,
            _ => col as f32 * 29.0,
        };
        (x, bottom)
    }
}

const COLORS: [&str; 16] = [
    "white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black",
];
const PANEL: (f32, f32) = (195.0, 136.0);

impl Game {
    /// Which tab an item belongs to (None: not shown, like the hidden orientations of a block).
    fn creative_tab_of(&self, id: ItemId) -> Option<Tab> {
        let it = self.items.get(id);
        let name = it.name.as_str();
        if name.ends_with("_spawn_egg") {
            return Some(Tab::Eggs);
        }
        if name == "fishing_bobber" || name.starts_with("minion_") || name.starts_with("gear_") && it.tool.is_some() && !name.ends_with("_stick_string") {
            return None;
        }
        if name == "flint_and_steel" || name == "minecart" || name.ends_with("_backpack") {
            return Some(Tab::Tools);
        }
        if let Some(t) = &it.tool {
            return Some(if t.kind == ToolKind::Sword { Tab::Combat } else { Tab::Tools });
        }
        if it.armor.is_some() {
            return Some(Tab::Combat);
        }
        if it.food.is_some() {
            return Some(Tab::Food);
        }
        let Some(b) = it.places else { return Some(Tab::Ingredients) };
        let bl = self.reg.get(b);
        if !bl.in_inventory && !matches!(it.look, voxel::item::Look::Sprite(_)) {
            return None;
        }
        if COLORS.iter().any(|c| name.starts_with(&format!("{c}_"))) && !name.ends_with("_bed") && bl.shape != Shape::Cross && name != "red_sand" {
            return Some(Tab::Colored);
        }
        // Redstone parts sit with the functional blocks.
        let redstone = matches!(bl.shape, Shape::Wire | Shape::Lever | Shape::Button | Shape::Plate | Shape::Repeater | Shape::Piston | Shape::Comparator | Shape::Oriented | Shape::Hopper | Shape::Rail)
            || ["redstone_torch", "redstone_block", "redstone_lamp", "tnt"].contains(&name)
            || name.ends_with("copper_bulb");
        if redstone {
            return Some(Tab::Functional);
        }
        if matches!(bl.shape, Shape::Door | Shape::Trapdoor | Shape::FenceGate | Shape::Torch) || bl.light > 0 {
            return Some(Tab::Doors);
        }
        if ["crafting_table", "furnace", "chest", "barrel", "bookshelf", "ladder", "jukebox"].contains(&name) || name.ends_with("_bed") || matches!(bl.shape, Shape::Chair | Shape::Table | Shape::Bench | Shape::Shelf) {
            return Some(Tab::Functional);
        }
        let natural = bl.shape == Shape::Cross
            || ["_ore", "_log", "_leaves", "_sapling", "_roots"].iter().any(|s| name.ends_with(s))
            || ["grass_block", "dirt", "stone", "sand", "red_sand", "mud", "lily_pad", "gravel", "clay", "ice", "snow", "snow_block", "cactus", "pumpkin", "melon", "obsidian", "bedrock", "sandstone", "mossy_cobblestone", "podzol", "coarse_dirt"].contains(&name);
        Some(if natural { Tab::Natural } else { Tab::Building })
    }

    fn creative_list(&self, tab: Tab) -> Vec<ItemId> {
        let all = 1..self.items.items.len() as ItemId;
        match tab {
            Tab::Search => {
                let q = self.surv.csearch.to_lowercase();
                all.filter(|&i| self.creative_tab_of(i).is_some())
                    .filter(|&i| {
                        let it = self.items.get(i);
                        q.is_empty() || it.display_name.to_lowercase().contains(&q) || it.name.contains(&q)
                    })
                    .collect()
            }
            Tab::Saved | Tab::Inventory => Vec::new(),
            t => all.filter(|&i| self.creative_tab_of(i) == Some(t)).collect(),
        }
    }

    /// An old creative world keeps its blocks: the hotbar of block ids becomes item stacks.
    pub(crate) fn fill_creative_hotbar(&mut self) {
        if self.survival || self.surv.inv.slots[..9].iter().any(|s| s.is_some()) {
            return;
        }
        for i in 0..9 {
            if let Some(it) = self.items.of_block(self.hotbar[i]) {
                self.surv.inv.slots[i] = Some(ItemStack::new(it, 1));
            }
        }
    }

    pub(crate) fn creative_screen_open(&self) -> bool {
        self.screen == Screen::Inventory && !self.survival && self.surv.open == Open::Craft
    }

    /// Keys while the search tab has the text box: letters go into it. Returns true if used.
    pub(crate) fn creative_key(&mut self, code: KeyCode) -> bool {
        // The search box of the survival item list has the keyboard.
        if self.screen == Screen::Inventory && self.survival && self.surv.bsearch_on {
            match code {
                KeyCode::Escape | KeyCode::Enter => self.surv.bsearch_on = false,
                KeyCode::Backspace => {
                    self.surv.bsearch.pop();
                }
                _ => {}
            }
            self.surv.book_scroll = 0;
            return true;
        }
        if !self.creative_screen_open() || TABS[self.surv.ctab] != Tab::Search || code == KeyCode::Escape {
            return false;
        }
        if code == KeyCode::Backspace {
            self.surv.csearch.pop();
            self.surv.cscroll = 0;
        }
        true
    }

    pub(crate) fn creative_type(&mut self, text: &str) {
        if self.screen == Screen::Inventory && self.survival && self.surv.bsearch_on {
            for c in text.chars() {
                if !c.is_control() && self.surv.bsearch.chars().count() < 20 {
                    self.surv.bsearch.push(c);
                }
            }
            self.surv.book_scroll = 0;
            return;
        }
        if !self.creative_screen_open() || TABS[self.surv.ctab] != Tab::Search {
            return;
        }
        for c in text.chars() {
            if !c.is_control() && self.surv.csearch.chars().count() < 24 {
                self.surv.csearch.push(c);
            }
        }
        self.surv.cscroll = 0;
    }

    pub(crate) fn creative_scroll(&mut self, up: bool) {
        self.surv.cscroll = if up { self.surv.cscroll.saturating_sub(1) } else { self.surv.cscroll + 1 };
    }

    pub(crate) fn draw_creative_inventory(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        let (pw, ph) = (PANEL.0 * s, PANEL.1 * s);
        let x0 = ((w - pw) / 2.0).floor();
        let y0 = ((h - ph) / 2.0).floor();
        let tab = TABS[self.surv.ctab.min(TABS.len() - 1)];
        let at = |x: f32, y: f32| (x0 + x * s, y0 + y * s);
        let inside = |p: (f32, f32), wd: f32, ht: f32| mouse.0 >= p.0 && mouse.0 < p.0 + wd * s && mouse.1 >= p.1 && mouse.1 < p.1 + ht * s;
        let shift = self.keys.contains(&KeyCode::ShiftLeft) || self.keys.contains(&KeyCode::ShiftRight);
        let (click, rclick) = (self.clicked, self.rclicked);

        let hole = (tab == Tab::Inventory).then(|| {
            let (hx, hy) = at(73.0, 6.0);
            (hx, hy, 32.0 * s, 43.0 * s)
        });
        self.surv.figure_box = hole;
        rect_around(&mut self.ui, (0.0, 0.0, w, h), hole, [0, 0, 0, 110]);

        // Tabs behind the panel first, then the panel, then the open tab joined to it.
        let mut over_tab: Option<usize> = None;
        for pass in 0..2 {
            for i in 0..TABS.len() {
                let on = i == self.surv.ctab;
                if (pass == 1) != on {
                    continue;
                }
                let (tx, bottom) = Tab::place(i);
                let (x, y) = at(tx, if bottom { PANEL.1 - 4.0 } else { -28.0 });
                let (tw, th) = (26.0 * s, 32.0 * s);
                // A pack's tab picture when it has one: number 1 joins the left edge, 7 the right.
                let col = if bottom { i - 7 } else { i };
                let n = if tx >= 169.0 { 7 } else { col + 1 };
                let name = format!(
                    "sprites/container/creative_inventory/tab_{}_{}_{n}",
                    if bottom { "bottom" } else { "top" },
                    if on { "selected" } else { "unselected" }
                );
                if !self.ui.sprite(&name, x, y, tw, th) {
                    let bg = if on { [198, 198, 198, 255] } else { [160, 160, 160, 255] };
                    self.ui.rect(x, y, tw, th, bg);
                    self.ui.rect(x, y, s, th, [255, 255, 255, 255]);
                    self.ui.rect(x + tw - s, y, s, th, [85, 85, 85, 255]);
                    if bottom {
                        self.ui.rect(x, y + th - s, tw, s, [85, 85, 85, 255]);
                    } else {
                        self.ui.rect(x, y, tw, s, [255, 255, 255, 255]);
                    }
                }
                let iy = if bottom { y + 10.0 * s } else { y + 7.0 * s };
                self.draw_tab_icon(TABS[i], x + 5.0 * s, iy);
                if mouse.0 >= x && mouse.0 < x + tw && mouse.1 >= y && mouse.1 < y + th {
                    over_tab = Some(i);
                }
            }
            if pass == 0 {
                let bg = match tab {
                    Tab::Inventory => "container/creative_inventory/tab_inventory",
                    Tab::Search => "container/creative_inventory/tab_item_search",
                    _ => "container/creative_inventory/tab_items",
                };
                self.surv.cart = self.ui.has_sprite(bg);
                if self.surv.cart {
                    // The pack's picture, cut around the hole the 3D figure shows through.
                    sprite_around(&mut self.ui, bg, (x0, y0, pw, ph), hole, s);
                } else {
                    // The panel, with the classic bevel, around the hole the 3D figure shows through.
                    rect_around(&mut self.ui, (x0, y0, pw, ph), hole, [198, 198, 198, 255]);
                    self.ui.rect(x0, y0, pw, s, [255, 255, 255, 255]);
                    self.ui.rect(x0, y0, s, ph, [255, 255, 255, 255]);
                    self.ui.rect(x0 + pw - s, y0, s, ph, [85, 85, 85, 255]);
                    self.ui.rect(x0, y0 + ph - s, pw, s, [85, 85, 85, 255]);
                }
            }
        }

        let mut hovered: Option<Slot> = None;
        let mut tip: Option<ItemId> = None;
        let mut slot_at = |game: &mut Game, slot: Slot, p: (f32, f32)| {
            draw_slot(game, p.0, p.1);
            if inside(p, 18.0, 18.0) {
                hovered = Some(slot);
                game.ui.rect(p.0 + s, p.1 + s, 16.0 * s, 16.0 * s, [255, 255, 255, 80]);
            }
            if let Some(st) = game.slot_get(slot) {
                item_icon(&mut game.ui, &game.reg, &game.items, game.item_base, &st, p.0 + s, p.1 + s, 16.0 * s);
            }
        };
        let dark = [64, 64, 64, 255];

        match tab {
            Tab::Inventory => {
                // Helmet and chestplate left of the figure, leggings and boots right, like the classic screen.
                let spots = [(54.0, 6.0), (54.0, 33.0), (108.0, 6.0), (108.0, 33.0)];
                for (i, &(sx, sy)) in spots.iter().enumerate() {
                    let p = at(sx, sy);
                    slot_at(self, Slot::Armor(i), p);
                    if self.surv.armor[i].is_none() {
                        slot_outline(self, i, p.0, p.1);
                    }
                }
                let op = at(35.0, 20.0);
                slot_at(self, Slot::Offhand, op);
                if self.surv.offhand.is_none() {
                    slot_outline(self, 4, op.0, op.1);
                }
                for i in 9..36 {
                    let k = i - 9;
                    slot_at(self, Slot::Inv(i), at(9.0 + (k % 9) as f32 * 18.0, 54.0 + (k / 9) as f32 * 18.0));
                }
                // Bin: drop the held stack here to delete it (shift-click empties the inventory).
                let (bx, by) = at(173.0, 112.0);
                if !self.surv.cart {
                    self.ui.rect(bx, by, 18.0 * s, 18.0 * s, [85, 55, 55, 255]);
                    self.ui.rect(bx + s, by + s, 16.0 * s, 16.0 * s, [184, 131, 131, 255]);
                    for k in 0..12 {
                        let d = (k as f32 + 2.0) * s;
                        self.ui.rect(bx + d, by + d, 2.0 * s, s, [60, 40, 40, 255]);
                        self.ui.rect(bx + 16.0 * s - d, by + d, 2.0 * s, s, [60, 40, 40, 255]);
                    }
                }
                if inside((bx, by), 18.0, 18.0) && click {
                    if shift {
                        self.surv.inv.slots.iter_mut().for_each(|x| *x = None);
                    }
                    self.surv.cursor = None;
                    self.clicked = false;
                }
            }
            Tab::Saved => {
                self.ui.label(x0 + 8.0 * s, y0 + 6.0 * s, "Clique carrega, Shift+clique salva", dark, s);
                let saved = self.settings.saved_hotbars.clone();
                for r in 0..5 {
                    let row: Vec<String> = saved.get(r).cloned().unwrap_or_default();
                    let y = 18.0 + r as f32 * 18.0;
                    for c in 0..9 {
                        let p = at(9.0 + c as f32 * 18.0, y);
                        draw_slot(self, p.0, p.1);
                        if let Some(id) = row.get(c).and_then(|n| self.items.id(n)) {
                            item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(id, 1), p.0 + s, p.1 + s, 16.0 * s);
                        }
                    }
                    let rp = at(9.0, y);
                    if inside(rp, 162.0, 18.0) {
                        self.ui.rect(rp.0, rp.1, 162.0 * s, 18.0 * s, [255, 255, 255, 50]);
                        if click {
                            self.clicked = false;
                            if shift {
                                let names: Vec<String> = (0..9).map(|i| self.surv.inv.slots[i].map(|st| self.items.get(st.item).name.clone()).unwrap_or_default()).collect();
                                while self.settings.saved_hotbars.len() <= r {
                                    self.settings.saved_hotbars.push(Vec::new());
                                }
                                self.settings.saved_hotbars[r] = names;
                                self.settings.save(&self.base.join("settings.ron"));
                                self.toast("Barra salva", 1.5);
                            } else if !row.is_empty() {
                                for i in 0..9 {
                                    self.surv.inv.slots[i] = row.get(i).and_then(|n| self.items.id(n)).map(|id| ItemStack::new(id, self.items.get(id).max_stack.max(1)));
                                }
                            }
                        }
                    }
                }
            }
            t => {
                let list = self.creative_list(t);
                let rows = list.len().div_ceil(9).max(1);
                let max_scroll = rows.saturating_sub(5);
                self.surv.cscroll = self.surv.cscroll.min(max_scroll);
                if t == Tab::Search {
                    let (bx, by) = at(82.0, 5.0);
                    if !self.surv.cart {
                        self.ui.rect(bx, by, 89.0 * s, 12.0 * s, [0, 0, 0, 255]);
                        self.ui.frame(bx, by, 89.0 * s, 12.0 * s, s, [160, 160, 160, 255]);
                    }
                    let blink = if (self.now * 2.0) as i32 % 2 == 0 { "_" } else { "" };
                    let text = format!("{}{}", self.surv.csearch, blink);
                    self.ui.text(bx + 2.0 * s, by + 2.0 * s, &text, WHITE);
                    self.ui.label(x0 + 8.0 * s, y0 + 6.0 * s, t.title(), dark, s);
                } else {
                    self.ui.label(x0 + 8.0 * s, y0 + 6.0 * s, t.title(), dark, s);
                }
                let mut grid_hover: Option<ItemId> = None;
                for r in 0..5 {
                    for c in 0..9 {
                        let p = at(9.0 + c as f32 * 18.0, 18.0 + r as f32 * 18.0);
                        draw_slot(self, p.0, p.1);
                        let Some(&id) = list.get((r + self.surv.cscroll) * 9 + c) else { continue };
                        if inside(p, 18.0, 18.0) {
                            grid_hover = Some(id);
                            self.ui.rect(p.0 + s, p.1 + s, 16.0 * s, 16.0 * s, [255, 255, 255, 80]);
                        }
                        item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(id, 1), p.0 + s, p.1 + s, 16.0 * s);
                    }
                }
                // Scroll bar.
                let (sx, sy) = at(175.0, 18.0);
                if !self.surv.cart {
                    self.ui.rect(sx, sy, 14.0 * s, 90.0 * s, [139, 139, 139, 255]);
                    self.ui.rect(sx, sy, 14.0 * s, s, [55, 55, 55, 255]);
                    self.ui.rect(sx, sy, s, 90.0 * s, [55, 55, 55, 255]);
                }
                let k = if max_scroll > 0 { self.surv.cscroll as f32 / max_scroll as f32 } else { 0.0 };
                let ty = sy + s + k * (90.0 - 17.0) * s;
                let thumb_art = if max_scroll > 0 { "sprites/container/creative_inventory/scroller" } else { "sprites/container/creative_inventory/scroller_disabled" };
                if !self.ui.sprite(thumb_art, sx + s, ty, 12.0 * s, 15.0 * s) {
                    let thumb = if max_scroll > 0 { [198, 198, 198, 255] } else { [150, 150, 150, 255] };
                    self.ui.rect(sx + s, ty, 12.0 * s, 15.0 * s, thumb);
                    self.ui.rect(sx + s, ty, 12.0 * s, s, [255, 255, 255, 255]);
                    self.ui.rect(sx + 12.0 * s, ty, s, 15.0 * s, [85, 85, 85, 255]);
                }
                if self.rdown || self.mouse_held[0] {
                    if mouse.0 >= sx && mouse.0 < sx + 14.0 * s && mouse.1 >= sy && mouse.1 < sy + 90.0 * s && max_scroll > 0 {
                        let f = ((mouse.1 - sy) / (90.0 * s)).clamp(0.0, 1.0);
                        self.surv.cscroll = (f * max_scroll as f32).round() as usize;
                    }
                }
                if let Some(id) = grid_hover {
                    tip = Some(id);
                    let max = self.items.get(id).max_stack.max(1);
                    // Items are endless in creative, so they come one at a time; middle click
                    // (or shift-click into the inventory) gives a full stack.
                    if let Some(k) = self.inv_number_key.take() {
                        self.surv.inv.slots[k] = Some(ItemStack::new(id, 1));
                    }
                    if self.mclicked && self.surv.cursor.is_none() {
                        self.mclicked = false;
                        self.surv.cursor = Some(ItemStack::new(id, max));
                    }
                    if click || rclick {
                        self.clicked = false;
                        self.rclicked = false;
                        match self.surv.cursor {
                            // Holding something: clicking the list throws it away.
                            Some(c) if !(c.item == id && c.count < max) => self.surv.cursor = None,
                            Some(c) => self.surv.cursor = Some(ItemStack { count: c.count + 1, ..c }),
                            None if shift => {
                                let _ = self.surv.inv.add(&self.items, ItemStack::new(id, max));
                            }
                            None => self.surv.cursor = Some(ItemStack::new(id, 1)),
                        }
                    }
                } else if (click || rclick) && self.surv.cursor.is_some() && inside(at(9.0, 18.0), 162.0, 90.0) {
                    self.surv.cursor = None;
                    self.clicked = false;
                }
            }
        }
        // Hotbar row, on every tab but the saved ones.
        if tab != Tab::Saved {
            for i in 0..9 {
                slot_at(self, Slot::Inv(i), at(9.0 + i as f32 * 18.0, 112.0));
            }
        }
        // Clicks on the player's own slots work like the survival screen.
        if let Some(slot) = hovered {
            if let Some(st) = self.slot_get(slot) {
                tip = Some(st.item);
            }
            if self.clicked || self.rclicked {
                let right = self.rclicked && !self.clicked;
                self.click_slot(slot, right, shift);
            }
            if let Some(k) = self.inv_number_key.take() {
                let cur = self.slot_get(slot);
                let hot = self.surv.inv.slots[k];
                self.surv.inv.slots[k] = cur;
                if slot != Slot::Inv(k) {
                    self.slot_set(slot, hot);
                }
            }
        }
        self.inv_number_key = None;
        if let Some(i) = over_tab {
            if self.clicked {
                self.surv.ctab = i;
                self.surv.cscroll = 0;
            }
            if tip.is_none() {
                let t = TABS[i].title();
                self.tooltip(t, mouse);
            }
        }
        // Clicking outside the window drops what the cursor holds.
        let in_panel = mouse.0 >= x0 && mouse.0 < x0 + pw && mouse.1 >= y0 - 28.0 * s && mouse.1 < y0 + ph + 28.0 * s;
        // Clicking outside the window throws the held stack (right click: one item).
        if (self.clicked || self.rclicked) && !in_panel && over_tab.is_none() {
            if let Some(c) = self.surv.cursor {
                let n = if self.rclicked && !self.clicked { 1 } else { c.count };
                self.surv.cursor = (c.count > n).then_some(ItemStack { count: c.count - n, ..c });
                self.throw_stack(ItemStack { count: n, ..c });
            }
        }
        if let Some(id) = tip {
            let st = ItemStack::new(id, 1);
            let mut lines = vec![(self.items.get(id).display_name.clone(), WHITE)];
            lines.extend(self.item_details(&st));
            self.draw_tooltip_lines(&lines, mouse);
        }
        if let Some(c) = self.surv.cursor {
            item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &c, mouse.0 - 8.0 * s, mouse.1 - 8.0 * s, 16.0 * s);
        }
    }

    fn draw_tab_icon(&mut self, tab: Tab, x: f32, y: f32) {
        let s = self.ui.scale;
        if tab == Tab::Search {
            // A magnifying glass.
            let c = [60, 60, 60, 255];
            self.ui.frame(x + 2.0 * s, y + 2.0 * s, 9.0 * s, 9.0 * s, 2.0 * s, c);
            self.ui.rect(x + 4.0 * s, y + 4.0 * s, 5.0 * s, 5.0 * s, [170, 210, 230, 255]);
            for k in 0..4 {
                self.ui.rect(x + (10.0 + k as f32) * s, y + (10.0 + k as f32) * s, 3.0 * s, 2.0 * s, [110, 80, 50, 255]);
            }
            return;
        }
        if let Some(id) = self.items.id(tab.icon()) {
            item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(id, 1), x, y, 16.0 * s);
        }
    }

    fn tooltip(&mut self, text: &str, mouse: (f32, f32)) {
        let s = self.ui.scale;
        let tw = self.ui.text_width(text, s);
        let tx = (mouse.0 + 8.0 * s).min(self.ui.width - tw - 8.0 * s);
        self.ui.rect(tx, mouse.1 - 14.0 * s, tw + 6.0 * s, 12.0 * s, [16, 0, 16, 235]);
        self.ui.text(tx + 3.0 * s, mouse.1 - 12.0 * s, text, WHITE);
    }
}

/// An empty slot with the classic sunken bevel.
fn draw_slot(game: &mut Game, x: f32, y: f32) {
    if game.surv.cart {
        return;
    }
    let s = game.ui.scale;
    game.ui.rect(x, y, 18.0 * s, 18.0 * s, [55, 55, 55, 255]);
    game.ui.rect(x + s, y + s, 17.0 * s, 17.0 * s, [255, 255, 255, 255]);
    game.ui.rect(x + s, y + s, 16.0 * s, 16.0 * s, [139, 139, 139, 255]);
}

/// Faint outline of what goes in an empty armour slot: helmet, chestplate, leggings, boots,
/// then the shield of the second hand.
fn slot_outline(game: &mut Game, kind: usize, x: f32, y: f32) {
    let s = game.ui.scale;
    let art = ["helmet", "chestplate", "leggings", "boots", "shield"][kind.min(4)];
    if game.ui.sprite(&format!("sprites/container/slot/{art}"), x + s, y + s, 16.0 * s, 16.0 * s) {
        return;
    }
    let c = [110, 110, 110, 255];
    let mut r = |px: f32, py: f32, w: f32, h: f32| game.ui.rect(x + px * s, y + py * s, w * s, h * s, c);
    match kind {
        0 => {
            r(4.0, 4.0, 10.0, 1.0);
            r(3.0, 5.0, 1.0, 8.0);
            r(14.0, 5.0, 1.0, 8.0);
            r(4.0, 13.0, 3.0, 1.0);
            r(11.0, 13.0, 3.0, 1.0);
        }
        1 => {
            r(3.0, 3.0, 4.0, 1.0);
            r(11.0, 3.0, 4.0, 1.0);
            r(7.0, 4.0, 4.0, 1.0);
            r(3.0, 4.0, 1.0, 11.0);
            r(14.0, 4.0, 1.0, 11.0);
            r(4.0, 15.0, 10.0, 1.0);
        }
        2 => {
            r(4.0, 3.0, 10.0, 1.0);
            r(4.0, 4.0, 1.0, 11.0);
            r(13.0, 4.0, 1.0, 11.0);
            r(8.0, 6.0, 2.0, 1.0);
            r(8.0, 6.0, 1.0, 9.0);
            r(9.0, 6.0, 1.0, 9.0);
        }
        3 => {
            r(4.0, 6.0, 1.0, 8.0);
            r(7.0, 6.0, 1.0, 6.0);
            r(4.0, 14.0, 4.0, 1.0);
            r(10.0, 6.0, 1.0, 6.0);
            r(13.0, 6.0, 1.0, 8.0);
            r(10.0, 14.0, 4.0, 1.0);
        }
        _ => {
            r(4.0, 3.0, 10.0, 1.0);
            r(4.0, 3.0, 1.0, 8.0);
            r(13.0, 3.0, 1.0, 8.0);
            r(5.0, 11.0, 2.0, 2.0);
            r(11.0, 11.0, 2.0, 2.0);
            r(7.0, 13.0, 4.0, 1.0);
        }
    }
}

/// A pack's 256×256 container picture over the panel, leaving out the `hole` rectangle.
fn sprite_around(ui: &mut crate::ui::UiBatch, name: &str, panel: (f32, f32, f32, f32), hole: Option<(f32, f32, f32, f32)>, s: f32) {
    let (x0, y0, pw, ph) = panel;
    // Screen rectangle -> the same part of the picture (one panel pixel is 1/256 of it).
    let mut part = |x: f32, y: f32, w: f32, h: f32| {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let f = |v: f32| v / s / 256.0;
        let p = [f(x - x0), f(y - y0), f(x - x0 + w), f(y - y0 + h)];
        ui.sprite_part(name, p, x, y, w, h, crate::ui::WHITE);
    };
    match hole {
        None => part(x0, y0, pw, ph),
        Some((hx, hy, hw, hh)) => {
            part(x0, y0, pw, hy - y0);
            part(x0, hy + hh, pw, y0 + ph - hy - hh);
            part(x0, hy, hx - x0, hh);
            part(hx + hw, hy, x0 + pw - hx - hw, hh);
        }
    }
}
