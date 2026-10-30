//! Furniture you can use: sitting on chairs (ideas from Handcrafted and Macaw's Furniture).

use glam::{IVec3, Vec3};
use voxel::block::Shape;

use crate::Game;

impl Game {
    /// Right click on a chair: sit down on it.
    pub(crate) fn sit_use(&mut self) -> bool {
        if self.crouch > 0.5 || self.riding.is_some() || self.seat.is_some() {
            return false;
        }
        let Some(hit) = self.target else { return false };
        let id = self.world.block_or_air(hit.block);
        if !matches!(self.reg.get(id).shape, Shape::Chair | Shape::Bench) {
            return false;
        }
        // No room to sit with a block right over the seat.
        let above = self.world.block_or_air(hit.block + IVec3::Y);
        if self.reg.get(above).solid {
            return false;
        }
        self.seat = Some(hit.block);
        self.body.vel = Vec3::ZERO;
        self.toast("Agache (Shift) para levantar", 2.5);
        true
    }

    /// Where the player is while seated: hips on the seat. True while sitting.
    pub(crate) fn seat_step(&mut self, sneak: bool) -> bool {
        let Some(p) = self.seat else { return false };
        let id = self.world.block_or_air(p);
        if !matches!(self.reg.get(id).shape, Shape::Chair | Shape::Bench) || sneak {
            self.seat = None;
            self.body.pos = p.as_vec3() + Vec3::new(0.5, 0.63, 0.5);
            self.body.vel = Vec3::ZERO;
            return false;
        }
        self.body.pos = p.as_vec3() + Vec3::new(0.5, -0.12, 0.5);
        self.body.vel = Vec3::ZERO;
        self.body.on_ground = true;
        self.body.fall_distance = 0.0;
        true
    }
}

/// Every wood with furniture, in the order the test hook lays them out.
pub(crate) const FURNITURE_WOODS: [&str; 20] = [
    "oak",
    "birch",
    "spruce",
    "jungle",
    "acacia",
    "dark_oak",
    "cherry",
    "mangrove",
    "pale_oak",
    "crimson",
    "warped",
    "maple",
    "walnut",
    "araucaria",
    "buriti",
    "coconut",
    "ipe",
    "jabuticaba",
    "echo",
    "skyroot",
];

impl Game {
    /// Test hook MC_MOVEIS=x,y,z: a showroom of every wood's furniture on a grass floor from
    /// that corner (the player's feet when no corner is given): five woods a row, nine blocks
    /// apart, each with a two-block table, a chair on either side, a bench and a shelf on a wall.
    pub(crate) fn build_furniture_test(&mut self) {
        static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !self.world_open
            || self.now < 1.0
            || DONE.swap(true, std::sync::atomic::Ordering::Relaxed)
        {
            return;
        }
        let Ok(v) = std::env::var("MC_MOVEIS") else {
            return;
        };
        let c: Vec<i32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        let o = if c.len() == 3 {
            IVec3::new(c[0], c[1], c[2])
        } else {
            self.body.pos.floor().as_ivec3()
        };
        let reg = self.reg.clone();
        let w = &mut self.world;
        let grass = reg.id_or_air("grass_block");
        for x in -2..5 * 9 + 1 {
            for z in -2..4 * 9 + 1 {
                w.set_block(o + IVec3::new(x, -1, z), grass);
                for y in 0..5 {
                    w.set_block(o + IVec3::new(x, y, z), voxel::AIR);
                }
            }
        }
        for (k, wood) in FURNITURE_WOODS.iter().enumerate() {
            let cell = o + IVec3::new((k % 5) as i32 * 9, 0, (k / 5) as i32 * 9);
            let put = |w: &mut crate::world::World, x: i32, y: i32, z: i32, name: &str, v: u8| {
                w.set_block(
                    cell + IVec3::new(x, y, z),
                    reg.variant(reg.id_or_air(name), v),
                );
            };
            // Chairs face the table from north and south; the wall with the shelf is behind.
            for x in [1, 2] {
                put(w, x, 0, 2, &format!("{wood}_table"), 0);
                put(w, x, 0, 3, &format!("{wood}_chair"), 0);
                put(w, x, 0, 1, &format!("{wood}_chair"), 2);
            }
            put(w, 4, 0, 2, &format!("{wood}_bench"), 1);
            put(w, 0, 0, 0, &format!("{wood}_planks"), 0);
            put(w, 0, 1, 0, &format!("{wood}_planks"), 0);
            put(w, 0, 1, 1, &format!("{wood}_shelf"), 0);
        }
    }
}
