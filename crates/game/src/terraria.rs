//! The Terraria side (Etapa 5) in one place: its data files joined onto the base lists, the
//! state it keeps per world (accessories, glyph spells, hard mode, events) saved in
//! `terraria.ron`, and the one update and HUD call the main loop makes for it. The pieces
//! live in their own files: accessories, hook, affixes, glyphs, hardmode and boss_ai.

use std::collections::HashSet;
use std::path::Path;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use voxel::item::{ItemRegistry, ItemStack};
use voxel::recipe::Recipes;

use crate::rpg::MAX_ACC;
use crate::Game;

/// `main` with the entries of `extra` added at the end of its list. Both are RON files of the
/// form `(key: [ ... ])`; comments before the list are fine.
pub fn with_extra(main: &str, extra: &str) -> String {
    let (Some(open), Some(close)) = (extra.find('['), extra.rfind(']')) else { return main.to_string() };
    let body = extra[open + 1..close].trim();
    let Some(end) = main.rfind(']') else { return main.to_string() };
    if body.is_empty() {
        return main.to_string();
    }
    let head = main[..end].trim_end();
    let sep = if head.ends_with(',') || head.ends_with('[') { "" } else { "," };
    format!("{head}{sep}\n{body}\n{}", &main[end..])
}

/// Adds the Etapa 5 recipes (their own file, with its own tags) to the base ones.
pub fn add_recipes(recipes: &mut Recipes, text: &str, items: &ItemRegistry) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    let more = Recipes::from_ron(text, items).map_err(|e| format!("etapa5/{e}"))?;
    recipes.shaped.extend(more.shaped);
    recipes.shapeless.extend(more.shapeless);
    recipes.smelting.extend(more.smelting);
    Ok(())
}

type SavedStack = Option<(String, u8, u16)>;

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    #[serde(default)]
    acc: Vec<SavedStack>,
    #[serde(default)]
    learned: Vec<String>,
    #[serde(default)]
    spells: Vec<Vec<String>>,
    #[serde(default)]
    active: usize,
    #[serde(default)]
    hard: bool,
    #[serde(default)]
    seeded: Vec<(i32, i32)>,
    #[serde(default)]
    verdite: Vec<(i32, i32)>,
    #[serde(default)]
    goblins_beaten: bool,
    #[serde(default)]
    mechanic_freed: bool,
    /// rpg.rs: accessory slots opened, the loadout worn and what the other two hold
    /// (four armour pieces, then the accessories).
    #[serde(default)]
    extra_slots: u8,
    /// Lote 11: the pet and light pet called out (item names), vanity armour and dyes.
    #[serde(default)]
    pet: Option<String>,
    #[serde(default)]
    light_pet: Option<String>,
    #[serde(default)]
    vanity: Vec<SavedStack>,
    #[serde(default)]
    dyes: Vec<SavedStack>,
    #[serde(default)]
    loadout: usize,
    #[serde(default)]
    loadouts: Vec<Vec<SavedStack>>,
}

/// What the Terraria side keeps for the world that is open.
#[derive(Default)]
pub struct Terraria {
    /// Worn accessories.
    pub acc: [Option<ItemStack>; MAX_ACC],
    /// Stats, minions, loadouts (rpg.rs).
    pub rpg: crate::rpg::Rpg,
    pub glyphs: crate::glyphs::Grimoire,
    pub hook: crate::hook::Hook,
    pub moves: crate::accessories::Moves,
    pub hard: crate::hardmode::HardMode,
    pub fight: crate::boss_ai::Fight,
    /// Damage a ward spell still soaks up, and for how long.
    pub ward: f32,
    pub ward_t: f32,
    /// The rest of the Terraria side (tr.rs), saved in tr.ron.
    pub tr: crate::tr::Tr,
    /// Spirits from the spirit staff, circling the player.
    pub spirits: Vec<crate::magic::Spirit>,
    /// Vortexes of the void book, pulling mobs in.
    pub vortexes: Vec<crate::magic::Vortex>,
    /// Lote 11: summoned mounts and pets, wings, vanity and dyes, sentries.
    pub comp: crate::companions::Companions,
    pub look: crate::vanity::Looks,
    pub sentries: Vec<crate::sentries::Sentry>,
    /// Read from the world's folder yet (it loads on the first frame in a world).
    pub(crate) loaded: bool,
}

impl Terraria {
    pub fn load(dir: &Path, items: &ItemRegistry) -> Terraria {
        let mut t = Terraria { loaded: true, tr: crate::tr::Tr::load(dir, items), ..Default::default() };
        let Some(s) = std::fs::read_to_string(dir.join("terraria.ron")).ok().and_then(|x| ron::from_str::<Saved>(&x).ok()) else {
            return t;
        };
        let stack = |st: &SavedStack| st.as_ref().and_then(|(name, count, damage)| items.id(name).map(|id| ItemStack { item: id, count: *count, damage: *damage, ench: Default::default(), sockets: 0 }));
        for (i, st) in s.acc.iter().enumerate().take(MAX_ACC) {
            t.acc[i] = stack(st);
        }
        t.rpg.extra_slots = s.extra_slots.min(2);
        t.rpg.loadout = s.loadout.min(2);
        for (k, g) in s.loadouts.iter().enumerate().take(3) {
            for (i, st) in g.iter().enumerate() {
                if i < 4 {
                    t.rpg.loadouts[k].armor[i] = stack(st);
                } else if i - 4 < MAX_ACC {
                    t.rpg.loadouts[k].acc[i - 4] = stack(st);
                }
            }
        }
        t.glyphs.learned = s.learned.into_iter().collect();
        for (i, sp) in s.spells.into_iter().enumerate().take(crate::glyphs::SPELL_SLOTS) {
            t.glyphs.spells[i] = sp;
        }
        t.glyphs.active = s.active.min(crate::glyphs::SPELL_SLOTS - 1);
        t.hard.on = s.hard;
        t.hard.seeded = s.seeded.into_iter().collect::<HashSet<_>>();
        t.hard.verdite = s.verdite.into_iter().collect::<HashSet<_>>();
        t.hard.goblins_beaten = s.goblins_beaten;
        t.hard.mechanic_freed = s.mechanic_freed;
        t.comp.pet = s.pet.as_deref().and_then(crate::companions::pet_of);
        t.comp.light = s.light_pet.as_deref().and_then(crate::companions::pet_of);
        for (i, st) in s.vanity.iter().enumerate().take(4) {
            t.look.vanity[i] = stack(st);
        }
        for (i, st) in s.dyes.iter().enumerate().take(crate::vanity::DYE_SLOTS) {
            t.look.dyes[i] = stack(st);
        }
        t
    }

    pub fn save(&self, dir: &Path, items: &ItemRegistry) {
        self.tr.save(dir, items);
        let s = Saved {
            acc: self.acc.iter().map(|s| s.map(|s| (items.get(s.item).name.clone(), s.count, s.damage))).collect(),
            learned: self.glyphs.learned.iter().cloned().collect(),
            spells: self.glyphs.spells.to_vec(),
            active: self.glyphs.active,
            hard: self.hard.on,
            seeded: self.hard.seeded.iter().copied().collect(),
            verdite: self.hard.verdite.iter().copied().collect(),
            goblins_beaten: self.hard.goblins_beaten,
            mechanic_freed: self.hard.mechanic_freed,
            extra_slots: self.rpg.extra_slots,
            pet: self.comp.pet.map(|k| crate::companions::PETS[k].item.to_string()),
            light_pet: self.comp.light.map(|k| crate::companions::PETS[k].item.to_string()),
            vanity: self.look.vanity.iter().map(|s| s.map(|s| (items.get(s.item).name.clone(), s.count, s.damage))).collect(),
            dyes: self.look.dyes.iter().map(|s| s.map(|s| (items.get(s.item).name.clone(), s.count, s.damage))).collect(),
            loadout: self.rpg.loadout,
            loadouts: self.rpg.loadouts.iter().map(|g| g.armor.iter().chain(g.acc.iter()).map(|s| s.map(|s| (items.get(s.item).name.clone(), s.count, s.damage))).collect()).collect(),
        };
        if let Ok(text) = ron::ser::to_string_pretty(&s, Default::default()) {
            let _ = std::fs::create_dir_all(dir);
            if let Err(e) = voxel::save::write_atomic(dir.join("terraria.ron"), text) {
                log::error!("could not save terraria.ron: {e}");
            }
        }
    }
}

impl Game {
    /// The world's own folder (whatever dimension is loaded): the Terraria state lives there.
    fn t5_dir(&self) -> std::path::PathBuf {
        crate::level_path_hint(&self.base, &self.world_name)
    }

    pub(crate) fn t5_load(&mut self) {
        self.t5 = Terraria::load(&self.t5_dir(), &self.items);
    }

    pub(crate) fn t5_save(&self) {
        if !self.t5.loaded {
            return;
        }
        self.t5.save(&self.t5_dir(), &self.items);
    }

    /// Everything of the Terraria side that runs each frame in a world.
    pub(crate) fn update_t5(&mut self, dt: f32, use_held: bool) {
        if !self.t5.loaded {
            self.t5_load();
        }
        self.update_rpg(dt);
        self.update_accessories(dt);
        self.update_companions(dt);
        self.update_sentries(dt);
        self.update_hook(dt, use_held);
        self.update_glyph_casting(dt, use_held);
        self.update_spell_extras(dt);
        self.update_fight(dt);
        self.update_hardmode(dt);
        self.update_tr(dt, use_held);
    }

    /// Right click with an item of the Terraria side; true when it did something.
    pub(crate) fn t5_use(&mut self, name: &str) -> bool {
        if crate::accessories::is_accessory(name) {
            return self.wear_accessory();
        }
        if let Some(r) = self.rpg_use(name) {
            return r;
        }
        if let Some(r) = self.companion_use(name) {
            return r;
        }
        if let Some(r) = self.sentry_use(name) {
            return r;
        }
        if crate::hook::is_hook(name) {
            return self.throw_hook(name);
        }
        if name == "glyph_book" {
            if self.keys.contains(&winit::keyboard::KeyCode::ShiftLeft) || self.keys.contains(&winit::keyboard::KeyCode::ShiftRight) {
                self.open_grimoire();
            }
            // Casting happens while the button is held.
            return true;
        }
        if name.starts_with("glyph_") {
            return self.learn_glyph(name);
        }
        if let Some(r) = self.tr_use(name) {
            return r;
        }
        if let Some(r) = self.t5_summon(name) {
            return r;
        }
        if name == "goblin_battle_standard" {
            return self.call_goblins();
        }
        if name == "bloody_tear" {
            return self.call_blood_moon();
        }
        false
    }

    /// Right click on a block of the Terraria side (the reforge table).
    pub(crate) fn t5_use_block(&mut self, block_name: &str) -> bool {
        if block_name == "reforge_table" {
            return self.reforge_held();
        }
        false
    }

    /// Damage coming at the player after what the Terraria side does to it: the ward soaks it
    /// up, hard mode makes monsters hit harder, armour above the 20 points softens it more.
    pub(crate) fn t5_incoming(&mut self, amount: f32, from: Option<Vec3>) -> f32 {
        let mut amount = amount;
        if from.is_some() {
            if self.t5.hard.on {
                amount *= crate::mob_tiers::HARD_HIT;
            }
            if self.t5.hard.blood_moon() {
                amount *= 1.2;
            }
            amount = self.tr3_incoming(amount);
        }
        // Defense above the classic 20 points and endurance (rpg.rs).
        let mut amount = self.rpg_reduce(amount, from);
        if self.t5.ward > 0.0 && amount > 0.0 {
            let soak = amount.min(self.t5.ward);
            self.t5.ward -= soak;
            amount -= soak;
            let glow = self.glow_layer();
            let c = self.body.pos + Vec3::Y;
            for _ in 0..10 {
                self.particles.glow(c, Vec3::new(self.surv.rng.f32() - 0.5, 0.3, self.surv.rng.f32() - 0.5) * 3.0, glow, [0.7, 0.85, 1.5], 0.1, 0.4);
            }
        }
        amount
    }

    /// The HUD of the Terraria side: event bar, depth meter, ward, the active glyph spell.
    pub(crate) fn draw_t5_hud(&mut self, w: f32, h: f32) {
        if self.t5.hard.blood_moon() {
            // The world looks red under a blood moon.
            self.ui.rect(0.0, 0.0, w, h, [120, 0, 0, 38]);
        }
        self.draw_event_bars(w);
        self.draw_tr_hud(w, h);
        self.draw_accessory_hud(w, h);
        self.draw_info_hud(h);
        if self.t5.rpg.sheet && self.screen == crate::Screen::Playing {
            let s = self.ui.scale;
            self.draw_stats_sheet(w - 4.0 * s, 40.0 * s);
        }
        self.draw_glyph_hud(w, h);
    }
}

impl Game {
    /// Test scenes for screenshots (MC_T5=name): the new bosses mid-fight, a blood moon, a
    /// goblin invasion, the new blocks, the accessory slots, a glyph spell, the hook.
    pub(crate) fn test_t5(&mut self, scene: &str) {
        if !self.t5.loaded {
            self.t5_load();
        }
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        let face_me = |g: &mut Game| {
            if let Some(m) = g.mobs.list.last_mut() {
                m.yaw = (-flat.x).atan2(-flat.z);
                m.aggro = true;
            }
        };
        let give = |g: &mut Game, name: &str, slot: usize| {
            if let Some(id) = g.items.id(name) {
                g.surv.inv.slots[slot] = Some(ItemStack::new(id, 1));
            }
        };
        match scene {
            "deep_worm" | "mech_worm" => {
                // Arching out of the ground ahead, head and tail still in the earth.
                let mech = scene == "mech_worm";
                let r = if mech { 6.0 } else { 5.0 };
                let mut c = self.body.pos + flat * 13.0;
                c.y = ground(self, c) - 1.0;
                let step = if mech { 1.5 } else { 1.25 } / r;
                let arc = |k: usize| c + side * (r * (0.5 + k as f32 * step).cos()) + Vec3::Y * (r * (0.5 + k as f32 * step).sin());
                self.mobs.spawn(scene, arc(0));
                self.spawn_worm_body(scene, arc(0));
                let n = self.mobs.list.len();
                let first = n - if mech { 15 } else { 11 };
                for k in first..n {
                    self.mobs.list[k].body.pos = arc(k - first) - Vec3::Y * 0.6;
                }
                // Heading down, the way the arc goes.
                self.t5.fight.vel_for_test((arc(0) - arc(1)).normalize() * 10.0);
                self.update_fight(0.0001);
            }
            "mech_eye" | "bone_king" | "mech_skull" | "celestial_herald" | "demon_eye" => {
                let at = self.body.pos + flat * 9.0 + Vec3::Y * 3.0;
                self.mobs.spawn(scene, at);
                face_me(self);
                if scene != "demon_eye" {
                    if let Some(m) = self.mobs.list.last_mut() {
                        m.health = m.max_health * 0.4;
                    }
                }
                for _ in 0..30 {
                    self.update_fight(0.03);
                }
                // Back in view for the picture, looking at the player, without the bolts in the way.
                let eye = self.body.eye();
                if let Some(m) = self.mobs.list.iter_mut().rev().find(|m| m.alive()) {
                    m.body.pos = self.body.pos + flat * 9.0 + Vec3::Y * 2.5;
                    let look = eye - m.body.pos - Vec3::Y;
                    m.yaw = look.x.atan2(look.z);
                    m.head_pitch = (-look.y / look.length()).asin();
                }
                self.magic.clear_bolts_for_test();
            }
            "world_breaker" | "vine_queen" => {
                let at = self.body.pos + flat * 14.0;
                let y = ground(self, at);
                self.mobs.spawn(scene, Vec3::new(at.x, y, at.z));
                face_me(self);
                if let Some(m) = self.mobs.list.last_mut() {
                    m.health = m.max_health * 0.4;
                }
                self.boss.called_help = true;
                for _ in 0..8 {
                    self.update_boss(0.5);
                    self.update_magic(0.03, false);
                }
            }
            "blood" => {
                self.t5.hard.on = true;
                self.t5.hard.blood = true;
                for k in 0..5 {
                    let at = self.body.pos + flat * (6.0 + k as f32 * 1.5) + side * (k as f32 - 2.0) * 1.8;
                    let y = ground(self, at);
                    self.mobs.spawn("blood_zombie", Vec3::new(at.x, y, at.z));
                    face_me(self);
                }
                for k in 0..3 {
                    let at = self.body.pos + flat * 7.0 + side * (k as f32 - 1.0) * 3.0 + Vec3::Y * (3.0 + k as f32);
                    self.mobs.spawn("demon_eye", at);
                    face_me(self);
                }
            }
            "goblins" => {
                self.start_invasion();
                self.t5.hard.invasion_left_for_test(18);
                let names = ["goblin_warrior", "goblin_archer", "goblin_sorcerer", "goblin_warrior", "goblin_archer"];
                for (k, n) in names.iter().enumerate() {
                    let at = self.body.pos + flat * (6.0 + (k % 2) as f32 * 2.0) + side * (k as f32 - 2.0) * 1.6;
                    let y = ground(self, at);
                    self.mobs.spawn(n, Vec3::new(at.x, y, at.z));
                    face_me(self);
                }
            }
            "town" => {
                self.t5.hard.on = true;
                let names = ["npc_tinkerer", "npc_mechanic", "npc_sorceress"];
                for (k, n) in names.iter().enumerate() {
                    let at = self.body.pos + flat * 4.0 + side * (k as f32 - 1.0) * 1.6;
                    let y = ground(self, at);
                    self.mobs.spawn(n, Vec3::new(at.x, y, at.z));
                    face_me(self);
                }
            }
            "blocks" => {
                let names = [
                    "cobalt_ore", "palladium_ore", "mythril_ore", "orichalcum_ore", "adamantite_ore", "titanium_ore", "verdite_ore", "reforge_table",
                    "cobalt_block", "palladium_block", "mythril_block", "orichalcum_block", "adamantite_block", "titanium_block", "radiant_block", "verdite_block", "lumite_block",
                ];
                let base = self.body.pos + flat * 6.0;
                let y0 = ground(self, base) as i32;
                for (k, n) in names.iter().enumerate() {
                    let (col, row) = if k < 8 { (k as f32 - 3.5, 1) } else { (k as f32 - 12.0, 0) };
                    let p = base + side * col * 1.0;
                    let id = self.reg.id_or_air(n);
                    self.world.set_block(glam::IVec3::new(p.x.floor() as i32, y0 + row, p.z.floor() as i32), id);
                }
            }
            "inventory" => {
                self.survival = true;
                let names = [
                    "glyph_book", "grappling_hook", "ruby_hook", "verdite_hook", "meteor_staff", "spirit_staff", "star_tome", "void_tome", "ward_tome",
                    "cobalt_sword", "mythril_pickaxe", "adamantite_chestplate", "titanium_helmet", "lumite_sword", "celestial_blade", "vine_blade", "breaker_hammer", "bone_whip",
                    "worm_bait", "cursed_skull", "breaker_idol", "gear_eye", "iron_worm", "steel_skull", "queen_bulb", "celestial_sigil", "goblin_battle_standard",
                    "glyph_projectile", "glyph_harm", "glyph_ignite", "glyph_freeze", "glyph_heal", "glyph_blink", "glyph_amplify", "glyph_aoe", "bloody_tear",
                ];
                for (k, n) in names.iter().enumerate() {
                    give(self, n, k);
                }
                for (k, n) in ["hermes_boots", "angel_wings", "cloud_bottle", "lucky_horseshoe", "guardian_shield"].iter().enumerate() {
                    let n = if *n == "hermes_boots" { "frost_spark_boots" } else { n };
                    if let Some(id) = self.items.id(n) {
                        self.t5.acc[k] = Some(ItemStack::new(id, 1).with_affix(14 + (k % 2) as u8));
                    }
                }
                if let Some(st) = self.surv.inv.slots[9] {
                    self.surv.inv.slots[9] = Some(st.with_affix(2));
                }
                self.args.show_inventory = true;
            }
            "grimoire" | "glyph" => {
                self.selected = 0;
                give(self, "glyph_book", 0);
                for g in ["projectile", "touch", "self", "harm", "ignite", "freeze", "heal", "break", "blink", "split", "amplify", "aoe"] {
                    self.t5.glyphs.learned.insert(g.to_string());
                }
                self.t5.glyphs.spells[0] = ["projectile", "split", "ignite", "harm"].iter().map(|s| s.to_string()).collect();
                self.t5.glyphs.spells[1] = ["self", "heal", "amplify"].iter().map(|s| s.to_string()).collect();
                self.t5.glyphs.spells[2] = ["touch", "break", "aoe"].iter().map(|s| s.to_string()).collect();
                if scene == "glyph" {
                    self.screen = crate::Screen::Playing;
                    self.test_glyph_cast();
                }
            }
            "hook" => {
                self.selected = 0;
                give(self, "ruby_hook", 0);
                self.throw_hook("ruby_hook");
                for _ in 0..4 {
                    self.update_hook(0.03, false);
                }
            }
            _ => log::warn!("unknown MC_T5 scene {scene}"),
        }
    }
}
