//! Test hook for screenshots of the gameplay mods: MC_GAMEPLAY=<scene>[,<scene>...] builds a
//! small scene in front of the player once the world has loaded.
//!
//! * `lamps`: glowstone in the cave roof around the player, to light up a cave biome.
//! * `macaws`: fences, fence gates (one open), barbed wire and Macaw's doors.
//! * `snow`: snowy slabs, stairs, fences and plants on a snow floor.
//! * `rod`: a modular fishing rod in hand with hook, line and bait fitted.
//! * `farm`: a farmer's field with bare farmland, a chest of seeds and a farmer at home.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::AIR;

use crate::Game;

impl Game {
    pub(crate) fn gameplay_test_hook(&mut self) {
        let Ok(v) = std::env::var("MC_GAMEPLAY") else { return };
        for scene in v.split(',') {
            match scene {
                "lamps" => self.scene_lamps(),
                "macaws" => self.scene_macaws(),
                "snow" => self.scene_snow(),
                "rod" => self.scene_rod(),
                "farm" => self.scene_farm(),
                _ => log::warn!("unknown MC_GAMEPLAY scene {scene}"),
            }
        }
    }

    /// Forward along the nearest axis, the axis to its right, the facing variant (as placing
    /// would give) and the ground cell `dist` ahead.
    fn scene_frame(&self, dist: i32) -> (IVec3, IVec3, u8, IVec3) {
        let f = self.forward();
        let (fwd, facing) = if f.x.abs() > f.z.abs() {
            if f.x > 0.0 { (IVec3::X, 1) } else { (-IVec3::X, 3) }
        } else if f.z > 0.0 {
            (IVec3::Z, 2)
        } else {
            (-IVec3::Z, 0)
        };
        let right = IVec3::new(-fwd.z, 0, fwd.x);
        let at = self.body.pos.floor().as_ivec3() + fwd * dist;
        let y = self.world.surface_y(at.x, at.z).unwrap_or(at.y - 1);
        (fwd, right, facing, IVec3::new(at.x, y, at.z))
    }

    fn put(&mut self, p: IVec3, name: &str, v: u8) {
        if let Some(id) = self.reg.id(name) {
            let id = self.reg.variant(id, v);
            self.world.set_block(p, id);
        }
    }

    fn clear_above(&mut self, p: IVec3, h: i32) {
        for k in 1..=h {
            self.world.set_block(p + IVec3::Y * k, AIR);
        }
    }

    /// Light up the cave around the player: glowstone set into the roof.
    fn scene_lamps(&mut self) {
        let me = self.body.pos.floor().as_ivec3();
        let glow = self.reg.id_or_air("glowstone");
        for dz in (-16..=16).step_by(4) {
            for dx in (-16..=16).step_by(4) {
                let p = me + IVec3::new(dx, 0, dz);
                for k in 1..32 {
                    let b = self.world.block_or_air(p + IVec3::Y * k);
                    if b != AIR && self.reg.get(b).solid {
                        self.world.set_block(p + IVec3::Y * k, glow);
                        break;
                    }
                }
            }
        }
    }

    fn scene_macaws(&mut self) {
        let (fwd, right, facing, g) = self.scene_frame(5);
        let grass = self.reg.id_or_air("grass_block");
        for d in -1..=5 {
            for s in -8..=8 {
                let p = IVec3::new(g.x, g.y, g.z) + fwd * d + right * s;
                self.world.set_block(p, grass);
                self.clear_above(p, 4);
            }
        }
        let y1 = IVec3::Y;
        // Front row: fences with their gates and a stretch of barbed wire.
        let row = [
            ("picket_fence", 0), ("picket_fence_gate", facing), ("picket_fence", 0), ("wired_fence", 0), ("wired_fence", 0),
            ("metal_fence", 0), ("metal_fence_gate", facing | 4), ("metal_fence", 0), ("oak_fence", 0), ("oak_fence_gate", facing),
            ("oak_fence", 0), ("maple_fence", 0), ("maple_fence_gate", facing), ("maple_fence", 0),
        ];
        for (k, (name, v)) in row.iter().enumerate() {
            self.put(g + y1 + right * (k as i32 - 7), name, *v);
        }
        // Back row: the doors, two blocks apart, with trapdoors between.
        let doors = ["shoji_door", "barn_door", "glass_door", "jail_door", "modern_door", "metal_door", "maple_door"];
        for (k, name) in doors.iter().enumerate() {
            let p = g + y1 + fwd * 3 + right * (k as i32 * 2 - 6);
            self.put(p, name, facing);
            self.put(p + y1, name, facing | 8);
        }
        for (k, name) in ["ranch_trapdoor", "glass_trapdoor", "coconut_trapdoor"].iter().enumerate() {
            self.put(g + y1 + fwd * 3 + right * (k as i32 * 4 - 5), name, 0);
        }
    }

    fn scene_snow(&mut self) {
        let (fwd, right, facing, g) = self.scene_frame(5);
        let snow = self.reg.id_or_air("snow");
        for d in -1..=4 {
            for s in -7..=7 {
                let p = g + fwd * d + right * s;
                self.world.set_block(p, snow);
                self.clear_above(p, 4);
            }
        }
        let y1 = IVec3::Y;
        let names = [
            ("snowy_oak_fence", 0), ("snowy_oak_fence", 0), ("snowy_oak_fence", 0), ("snowy_spruce_stairs", facing ^ 2),
            ("snowy_spruce_slab", 0), ("snowy_cobblestone_wall", 0), ("snowy_cobblestone_wall", 0), ("snowy_stone_brick_stairs", facing ^ 2),
            ("snowy_brick_slab", 0), ("snowy_birch_fence", 0), ("snowy_birch_fence", 0),
        ];
        for (k, (name, v)) in names.iter().enumerate() {
            self.put(g + y1 + fwd + right * (k as i32 - 5), name, *v);
        }
        let plants = ["snowy_short_grass", "snowy_poppy", "snowy_dandelion", "snowy_dead_bush", "snowy_short_grass", "snowy_twig", "snowy_pebble"];
        for (k, name) in plants.iter().enumerate() {
            self.put(g + y1 + fwd * 3 + right * (k as i32 * 2 - 6), name, 0);
        }
        // A deep drift: snow layers piled up by a storm.
        for (k, v) in [2u8, 4, 6, 4, 2].iter().enumerate() {
            self.put(g + y1 - fwd + right * (k as i32 - 2), "snow_layer", *v);
        }
    }

    fn scene_rod(&mut self) {
        self.mods.save.rig.hook = Some("double_hook".into());
        self.mods.save.rig.line = Some("strong_line".into());
        self.mods.save.rig.bait = Some(("worm".into(), 6));
        if let Some(id) = self.items.id("modular_fishing_rod") {
            let sel = self.selected;
            self.surv.inv.slots[sel] = Some(ItemStack::new(id, 1));
        }
    }

    fn scene_farm(&mut self) {
        let (fwd, right, _, g) = self.scene_frame(6);
        let farmland = self.reg.id_or_air("farmland");
        let water = self.reg.id_or_air("water");
        let wheat = self.reg.id_or_air("wheat_stage7");
        for d in -2..=2 {
            for s in -4..=4 {
                let p = g + fwd * d + right * s;
                self.clear_above(p, 5);
                if d == 0 && s == 0 {
                    self.world.set_block(p, water);
                } else {
                    self.world.set_block(p, farmland);
                    if s < -1 {
                        self.world.set_block(p + IVec3::Y, wheat);
                    }
                }
            }
        }
        // The farmer's chest, stocked with seeds, by the field.
        let chest_at = g + IVec3::Y - fwd * 3;
        self.put(chest_at, "chest", 0);
        let mut inv = voxel::item::Inventory::new(27);
        for (k, name) in ["wheat_seeds", "carrot", "potato"].iter().enumerate() {
            if let Some(id) = self.items.id(name) {
                inv.slots[k] = Some(ItemStack::new(id, 16));
            }
        }
        self.surv.containers.insert(chest_at, crate::survival::Container::Chest(inv));
        let home = (g - fwd * 2 + IVec3::Y).as_vec3() + Vec3::new(0.5, 0.0, 0.5);
        if self.mobs.spawn(crate::farm::FARMER, home) {
            let i = self.mobs.list.len() - 1;
            self.mobs.list[i].home = Some(home);
            // Fast-forward a little work: walk (teleport) to each job and do it.
            for _ in 0..24 {
                self.mods.farmers.t = 0.0;
                self.mods.farmers.sent = None;
                self.update_smarter_farmers(1.0);
                if let Some(w) = self.mods.farmers.sent {
                    self.mobs.list[i].body.pos = w;
                }
            }
        }
    }
}
