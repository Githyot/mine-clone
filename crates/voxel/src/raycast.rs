//! Voxel ray traversal (Amanatides & Woo): visits every block the ray passes through, in order.

use glam::{IVec3, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub block: IVec3,
    /// Normal of the face that was hit; `block + normal` is where a new block goes.
    pub normal: IVec3,
    pub distance: f32,
}

/// Walk from `origin` along `dir` up to `max_dist`, returning the first block for which `hit` is true.
pub fn raycast(origin: Vec3, dir: Vec3, max_dist: f32, mut hit: impl FnMut(IVec3) -> bool) -> Option<RayHit> {
    let dir = dir.normalize_or_zero();
    if dir == Vec3::ZERO {
        return None;
    }
    let mut pos = origin.floor().as_ivec3();
    let step = IVec3::new(dir.x.signum() as i32, dir.y.signum() as i32, dir.z.signum() as i32);
    let inv = Vec3::new(1.0 / dir.x.abs(), 1.0 / dir.y.abs(), 1.0 / dir.z.abs());
    let frac = |o: f32, d: f32| if d > 0.0 { o.floor() + 1.0 - o } else { o - o.floor() };
    let mut t_max = Vec3::new(
        if dir.x != 0.0 { frac(origin.x, dir.x) * inv.x } else { f32::INFINITY },
        if dir.y != 0.0 { frac(origin.y, dir.y) * inv.y } else { f32::INFINITY },
        if dir.z != 0.0 { frac(origin.z, dir.z) * inv.z } else { f32::INFINITY },
    );
    let t_delta = Vec3::new(
        if dir.x != 0.0 { inv.x } else { f32::INFINITY },
        if dir.y != 0.0 { inv.y } else { f32::INFINITY },
        if dir.z != 0.0 { inv.z } else { f32::INFINITY },
    );
    let mut normal = IVec3::ZERO;
    let mut t = 0.0;
    while t <= max_dist {
        if hit(pos) {
            return Some(RayHit { block: pos, normal, distance: t });
        }
        if t_max.x < t_max.y && t_max.x < t_max.z {
            pos.x += step.x;
            t = t_max.x;
            t_max.x += t_delta.x;
            normal = IVec3::new(-step.x, 0, 0);
        } else if t_max.y < t_max.z {
            pos.y += step.y;
            t = t_max.y;
            t_max.y += t_delta.y;
            normal = IVec3::new(0, -step.y, 0);
        } else {
            pos.z += step.z;
            t = t_max.z;
            t_max.z += t_delta.z;
            normal = IVec3::new(0, 0, -step.z);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_block_in_front() {
        let target = IVec3::new(3, 0, 0);
        let h = raycast(Vec3::new(0.5, 0.5, 0.5), Vec3::X, 10.0, |p| p == target).unwrap();
        assert_eq!(h.block, target);
        assert_eq!(h.normal, IVec3::new(-1, 0, 0));
        assert!((h.distance - 2.5).abs() < 1e-4);
        assert!(raycast(Vec3::new(0.5, 0.5, 0.5), Vec3::X, 2.0, |p| p == target).is_none());
    }

    #[test]
    fn diagonal_down() {
        let h = raycast(Vec3::new(0.2, 5.5, 0.7), Vec3::new(0.3, -1.0, 0.1), 20.0, |p| p.y <= 0).unwrap();
        assert_eq!(h.block.y, 0);
        assert_eq!(h.normal, IVec3::Y);
    }
}
