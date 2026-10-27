//! Climbing onto ledges up to two blocks high (Terraria style): the physics lives in
//! `voxel::physics` (`Body::try_mantle`); this is the test scene for screenshots.

use glam::{IVec3, Vec3};
use voxel::AIR;
use voxel::physics::Input;

use crate::Game;

impl Game {
    /// Test hook MC_MANTLE=u: a two-block stone wall (or with MC_MANTLE_SHELF a shelf at the
    /// height of the second block) just north of the player, who is caught `u` (0..1) of the
    /// way through climbing it. The screenshot pass holds the body still, so the pose stays.
    pub(crate) fn test_mantle(&mut self, u: f32) {
        let feet = self.body.pos.floor().as_ivec3();
        let g = self.world.surface_y(feet.x, feet.z).unwrap_or(feet.y - 1) + 1;
        let stone = self.reg.id_or_air("stone_bricks");
        let grass = self.reg.id_or_air("grass_block");
        // Level ground, clear air above it.
        for x in -3..=3 {
            for z in -5..=3 {
                self.world.set_block(IVec3::new(feet.x + x, g - 1, feet.z + z), grass);
                for y in 0..6 {
                    self.world.set_block(IVec3::new(feet.x + x, g + y, feet.z + z), AIR);
                }
            }
        }
        if std::env::var("MC_MANTLE_SHELF").is_ok() {
            // A shelf on the second block, the board on the player's side.
            let shelf = self.reg.id_or_air("oak_shelf");
            let v = (0..4u8)
                .find(|&v| {
                    let mut b = Vec::new();
                    voxel::shapes::boxes(voxel::block::Shape::Shelf, v, 0, true, &mut b);
                    b.iter().any(|q| q[4] - q[1] < 4 && q[5] == 16 && q[3] - q[0] == 16)
                })
                .unwrap_or(0);
            for x in -1..=1 {
                self.world.set_block(IVec3::new(feet.x + x, g + 1, feet.z - 1), self.reg.variant(shelf, v));
            }
        } else {
            for x in -2..=2 {
                for z in -5..=-1 {
                    for y in 0..2 {
                        self.world.set_block(IVec3::new(feet.x + x, g + y, feet.z + z), stone);
                    }
                }
            }
        }
        // Face north, pressed against it, and catch the edge.
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.body.flying = false;
        self.body.on_ground = true;
        self.body.vel = Vec3::ZERO;
        self.body.pos = Vec3::new(feet.x as f32 + 0.5, g as f32, feet.z as f32 + 0.5);
        for _ in 0..40 {
            let probe = self.body.pos - Vec3::Z * 0.02;
            if !self.body.fits_at(&self.world, probe, self.body.height) {
                break;
            }
            self.body.pos = probe;
        }
        let input = Input { wish: -Vec3::Z, jump: true, ..Default::default() };
        if !self.body.try_mantle(&self.world, &input, -Vec3::Z) {
            log::error!("test mantle: no ledge to climb at {:?}", self.body.pos);
            return;
        }
        if let Some(m) = self.body.mantle.as_mut() {
            m.t = m.dur * u.clamp(0.0, 0.99);
            self.body.pos = m.at();
        }
    }
}
