//! Test hook for Phase 6 pictures: `MC_FASE6=<scene>` builds a scene in front of the player
//! (run with `--survival --inventory` for the station screens).

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::AIR;

use crate::enchanting as ench;
use crate::progression::Event;
use crate::stations::Station;
use crate::survival::{Container, Open};

impl crate::Game {
    fn t_stack(&self, name: &str, n: u8) -> Option<ItemStack> {
        self.items.id(name).map(|i| ItemStack::new(i, n))
    }

    fn t_put(&mut self, p: IVec3, name: &str) {
        let id = self.reg.id_or_air(name);
        self.world.set_block(p, id);
    }

    /// Clears plants and anything else above a patch of ground.
    fn t_clear(&mut self, c: IVec3, r: i32) {
        for dz in -r..=r {
            for dx in -r..=r {
                for dy in 1..=3 {
                    self.world.set_block(c + IVec3::new(dx, dy, dz), AIR);
                }
            }
        }
    }

    /// The spot two blocks in front of the feet, on the ground.
    fn t_front(&self, dist: f32) -> IVec3 {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        (self.body.pos + flat * dist).floor().as_ivec3()
    }

    pub(crate) fn test_fase6(&mut self, scene: &str) {
        let front = self.t_front(2.5);
        self.surv.xp = crate::stations::xp_total(30) + 40;
        match scene {
            "enchant" => {
                self.t_put(front, "enchanting_table");
                for dx in -2i32..=2 {
                    for dz in -2..=2 {
                        if dx * dx + dz * dz >= 4 && (dx.abs() == 2 || dz.abs() == 2) {
                            self.t_put(front + IVec3::new(dx, 0, dz), "bookshelf");
                            self.t_put(front + IVec3::new(dx, 1, dz), "air");
                        }
                    }
                }
                self.station_use("enchanting_table", front);
                self.prog.bench.slots[0] = self.t_stack("diamond_sword", 1);
                self.prog.bench.slots[1] = self.t_stack("lapis_lazuli", 12);
            }
            "anvil" => {
                self.t_put(front, "anvil");
                self.station_use("anvil", front);
                let mut sword = self.t_stack("diamond_sword", 1);
                if let Some(s) = sword.as_mut() {
                    s.ench.set(ench::SHARPNESS, 3);
                    s.damage = 400;
                }
                let mut book = self.t_stack("enchanted_book", 1);
                if let Some(b) = book.as_mut() {
                    b.ench.set(ench::UNBREAKING, 3);
                    b.ench.set(ench::MENDING, 1);
                }
                self.prog.bench.slots[0] = sword;
                self.prog.bench.slots[1] = book;
                self.station_refresh(Station::Anvil(front));
            }
            "grind" => {
                self.t_put(front, "grindstone");
                self.station_use("grindstone", front);
                let mut bow = self.t_stack("bow", 1);
                if let Some(b) = bow.as_mut() {
                    b.ench.set(ench::POWER, 4);
                    b.ench.set(ench::FLAME, 1);
                }
                self.prog.bench.slots[0] = bow;
                self.station_refresh(Station::Grindstone);
            }
            "smith" => {
                self.t_put(front, "smithing_table");
                self.station_use("smithing_table", front);
                self.prog.bench.slots[0] = self.t_stack("wild_armor_trim_smithing_template", 1);
                self.prog.bench.slots[1] = self.t_stack("diamond_chestplate", 1);
                self.prog.bench.slots[2] = self.t_stack("emerald", 4);
                self.station_refresh(Station::Smithing);
            }
            "brew" => {
                self.t_put(front, "brewing_stand");
                self.station_use("brewing_stand", front);
                if let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&front) {
                    let aw = self.items.id("potion_awkward").map(|i| ItemStack::new(i, 1));
                    inv.slots[0] = aw;
                    inv.slots[1] = aw;
                    inv.slots[2] = self.items.id("potion_swiftness").map(|i| ItemStack::new(i, 1));
                    inv.slots[3] = self.items.id("blaze_powder").map(|i| ItemStack::new(i, 3));
                    inv.slots[4] = self.items.id("blaze_powder").map(|i| ItemStack::new(i, 5));
                }
                self.prog.brews.insert(front, crate::progression::Brew { time: 9.0, fuel: 14 });
            }
            "beacon" => {
                let top = front + IVec3::new(0, 4, 2);
                for l in 1..=4 {
                    for dz in -l..=l {
                        for dx in -l..=l {
                            self.t_put(top + IVec3::new(dx, -l, dz), if l == 1 { "diamond_block" } else if l == 2 { "gold_block" } else { "iron_block" });
                        }
                    }
                }
                self.t_put(top, "beacon");
                self.station_use("beacon", top);
                if let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&top) {
                    inv.slots[0] = self.items.id("emerald").map(|i| ItemStack::new(i, 1));
                }
                if let Some(b) = self.prog.beacons.get_mut(&top) {
                    b.levels = 4;
                    b.primary = crate::effects::HASTE;
                }
            }
            "effects" => {
                use crate::effects::*;
                self.screen = crate::Screen::Playing;
                for (e, a, t) in [(SPEED, 1, 120.0), (STRENGTH, 0, 300.0), (REGENERATION, 1, 40.0), (NIGHT_VISION, 0, 200.0), (FIRE_RESISTANCE, 0, 170.0), (POISON, 0, 9.0), (RESISTANCE, 2, 60.0)] {
                    self.add_effect(e, a, t, false);
                }
                self.add_effect(ABSORPTION, 1, 120.0, false);
                let sel = self.selected;
                self.surv.inv.slots[sel] = self.t_stack("splash_potion_strong_healing", 1);
                for k in 1..5 {
                    self.surv.inv.slots[(sel + k) % 9] = self.t_stack(["potion_night_vision", "lingering_potion_poison", "tipped_arrow_slowness", "potion_turtle_master"][k - 1], 1);
                }
                let at = self.body.pos + self.forward() * 4.0;
                if let Some(p) = potion("poison") {
                    self.potion_burst(Vec3::new(at.x, self.body.pos.y, at.z), p, true);
                }
            }
            "adv" | "stats" => {
                for a in crate::advancements::ADVANCEMENTS.iter().take(30).step_by(2) {
                    self.prog.earned.push(a.1.to_string());
                }
                self.prog.pending_toasts.push("story/enchant_item".into());
                self.prog.stats.add("cm_walked", 1_234_567);
                self.prog.stats.add("blocks_mined", 842);
                self.prog.stats.add("mobs_killed", 97);
                self.prog.stats.add("killed:zombie", 41);
                self.prog.stats.add("killed:skeleton", 30);
                self.prog.stats.add("play_seconds", 7_342);
                self.prog.show_advancements = true;
                self.prog.adv_tab = if scene == "stats" { 5 } else { 0 };
            }
            "weapons" => {
                let names = ["crossbow", "trident", "mace", "diamond_spear", "netherite_sword", "bow", "wind_charge", "brush", "recovery_compass"];
                for (k, n) in names.iter().enumerate() {
                    let mut s = self.t_stack(n, if *n == "wind_charge" { 16 } else { 1 });
                    if let Some(s) = s.as_mut() {
                        match *n {
                            "trident" => {
                                s.ench.set(ench::LOYALTY, 3);
                                s.ench.set(ench::CHANNELING, 1);
                            }
                            "mace" => {
                                s.ench.set(ench::DENSITY, 5);
                                s.ench.set(ench::WIND_BURST, 1);
                            }
                            "netherite_sword" => {
                                s.ench.set(ench::SHARPNESS, 5);
                                s.ench.set(ench::LOOTING, 3);
                            }
                            _ => {}
                        }
                    }
                    self.surv.inv.slots[k] = s;
                }
                for (k, n) in ["netherite_helmet", "netherite_chestplate", "netherite_leggings", "netherite_boots"].iter().enumerate() {
                    let mut s = self.t_stack(n, 1);
                    if let Some(s) = s.as_mut() {
                        s.ench.set(ench::PROTECTION, 4);
                        s.ench.set(crate::enchanting::TRIM, (2 * 12 + 2) as u8);
                    }
                    self.surv.armor[k] = s;
                }
                self.prog.death = Some((self.body.pos + Vec3::new(30.0, 0.0, -20.0), self.dim.as_u8()));
                self.selected = 8;
            }
            "warden" | "breeze" | "sniffer" => {
                let at = self.body.pos + Vec3::new(self.forward().x, 0.0, self.forward().z).normalize_or_zero() * 6.0;
                let y = self.body.pos.y;
                self.t_clear(IVec3::new(at.x.floor() as i32, y as i32 - 1, at.z.floor() as i32), 4);
                let n = if scene == "breeze" { 2 } else { 1 };
                for k in 0..n {
                    let p = Vec3::new(at.x + k as f32 * 2.5, y, at.z);
                    if self.mobs.spawn(scene, p) {
                        let flat = self.body.pos - p;
                        if let Some(m) = self.mobs.list.last_mut() {
                            m.yaw = flat.x.atan2(flat.z);
                            m.head_yaw = m.yaw;
                        }
                    }
                }
                if scene == "warden" {
                    self.add_effect(crate::effects::NIGHT_VISION, 0, 600.0, false);
                }
                if scene == "breeze" {
                    for k in 0..3 {
                        let from = at + Vec3::new(k as f32, 1.5 + k as f32 * 0.3, 0.0);
                        self.prog.charges.push(crate::trials::Charge { pos: from + (self.body.eye() - from) * 0.4, vel: (self.body.eye() - from).normalize() * 11.0, age: 0.0, reflected: false });
                    }
                }
                if scene == "sniffer" {
                    let g = self.t_front(3.0);
                    let gy = self.world.surface_y(g.x, g.z).unwrap_or(g.y);
                    self.t_put(IVec3::new(g.x - 2, gy + 1, g.z), "sniffer_egg");
                    for k in 0..3 {
                        self.t_put(IVec3::new(g.x + 1 + k, gy, g.z - 1), "farmland");
                        self.t_put(IVec3::new(g.x + 1 + k, gy + 1, g.z - 1), &format!("torchflower_stage{k}"));
                        self.t_put(IVec3::new(g.x + 1 + k, gy, g.z - 2), "farmland");
                        self.t_put(IVec3::new(g.x + 1 + k, gy + 1, g.z - 2), &format!("pitcher_stage{}", k * 2));
                    }
                }
            }
            "brush" => {
                let gy = self.world.surface_y(front.x, front.z).unwrap_or(front.y);
                self.t_clear(IVec3::new(front.x, gy, front.z), 3);
                for dx in -1..=1 {
                    self.t_put(IVec3::new(front.x + dx, gy, front.z), if dx == 0 { "suspicious_sand" } else { "suspicious_gravel" });
                }
                self.t_put(IVec3::new(front.x - 2, gy + 1, front.z), "decorated_pot");
                let sel = self.selected;
                self.surv.inv.slots[sel] = self.t_stack("brush", 1);
                self.prog.brushing = Some((IVec3::new(front.x, gy, front.z), 1.0));
            }
            "sculk" => {
                // A small sculk field: sensors, a shrieker, a catalyst, veins.
                let gy = self.world.surface_y(front.x, front.z).unwrap_or(front.y);
                self.t_clear(IVec3::new(front.x, gy, front.z + 1), 4);
                for dz in -2..=3 {
                    for dx in -3..=3 {
                        self.t_put(IVec3::new(front.x + dx, gy, front.z + dz), "sculk");
                    }
                }
                let above = |dx: i32, dz: i32| IVec3::new(front.x + dx, gy + 1, front.z + dz);
                self.t_put(above(-2, 0), "sculk_sensor");
                self.t_put(above(0, 1), "sculk_shrieker");
                self.t_put(above(2, 0), "calibrated_sculk_sensor_active");
                self.t_put(above(1, 3), "sculk_catalyst");
                self.t_put(above(-1, 2), "sculk_vein");
                self.t_put(above(3, 2), "sculk_sensor_active");
            }
            "blocks" => {
                // A wall of the new blocks to look at.
                let names = [
                    "enchanting_table", "anvil", "grindstone", "smithing_table", "brewing_stand", "beacon", "conduit", "sea_lantern", "prismarine", "prismarine_bricks",
                    "dark_prismarine", "netherite_block", "ancient_debris", "emerald_block", "lapis_block", "deepslate", "cobbled_deepslate", "polished_deepslate",
                    "deepslate_bricks", "deepslate_tiles", "chiseled_deepslate", "reinforced_deepslate", "tuff", "polished_tuff", "tuff_bricks", "chiseled_tuff",
                    "chiseled_tuff_bricks", "trial_spawner", "trial_spawner_active", "trial_spawner_ominous", "vault", "ominous_vault", "heavy_core", "suspicious_sand",
                    "suspicious_gravel", "decorated_pot", "sniffer_egg", "sculk", "sculk_catalyst", "sculk_shrieker", "sculk_sensor", "slime_block", "soul_lantern", "candle",
                ];
                let f = self.forward();
                let right = Vec3::new(-f.z, 0.0, f.x).normalize_or_zero();
                let base = self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 7.0;
                for (k, n) in names.iter().enumerate() {
                    let col = (k % 11) as f32 - 5.0;
                    let row = (k / 11) as i32;
                    let p = (base + right * col * 1.5).floor().as_ivec3() + IVec3::new(0, row * 2, 0);
                    self.t_put(p, n);
                    self.t_put(p - IVec3::Y, "stone");
                }
            }
            "trial" => {
                // Wake up the nearest spawner.
                self.scan_nearby(1.0);
                self.update_trials(0.05);
                for _ in 0..40 {
                    self.update_trials(0.1);
                }
            }
            "night_vision" => self.add_effect(crate::effects::NIGHT_VISION, 0, 600.0, false),
            _ => log::warn!("MC_FASE6: unknown scene {scene}"),
        }
        let _ = (AIR, Open::Craft, Station::Grindstone, Event::Brewed);
    }
}
