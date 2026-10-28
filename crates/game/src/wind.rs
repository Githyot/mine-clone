//! The wind: one gentle breeze over the whole world that slowly veers, with gusts that sweep
//! across the land along it. Falling leaves, pollen, dust and gnats drift with it, and the
//! louder rustle of the trees follows its gusts. Rain makes it stronger, storms more so.
//!
//! Only a few sines of the game clock (`Game::now`, the same seconds the shaders get in
//! `cam_pos.w`), so a shader can work out the same wind for swaying plants.

use glam::{Vec2, Vec3};

/// Which way the wind blows (unit vector on x/z) at time `t`.
pub fn direction(t: f32) -> Vec2 {
    let a = 0.7 + 0.35 * (t * 0.013).sin() + 0.2 * (t * 0.031 + 1.0).sin();
    Vec2::new(a.cos(), a.sin())
}

/// How gusty it is at `t` in a place `at` (x/z): 0 calm .. 1 the top of a gust. Gusts move
/// along the wind at 6 blocks a second.
pub fn gust(t: f32, at: Vec2) -> f32 {
    let p = t - direction(t).dot(at) / 6.0;
    let g = ((p * 0.37).sin() + (p * 0.23 + 2.0).sin() + (p * 0.71 + 4.0).sin()) / 3.0 * 0.5 + 0.5;
    g * g
}

/// The wind at `t` in a place `at` (x/z), in blocks a second: about 0.4 in calm weather up to
/// 3 in a storm gust. `rain` 0..1, `thunder` true in a storm.
pub fn at(t: f32, at: Vec2, rain: f32, thunder: bool) -> Vec3 {
    let d = direction(t);
    let base = 0.35 + rain * 0.45 + if thunder { 0.6 } else { 0.0 };
    let s = base * (0.6 + gust(t, at) * 1.6);
    Vec3::new(d.x * s, 0.0, d.y * s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wind_stays_gentle_and_turns_slowly() {
        let mut prev = direction(0.0);
        for k in 1..2000 {
            let t = k as f32 * 0.5;
            let w = at(t, Vec2::new(t, -t), 0.0, false);
            assert!(w.length() < 1.2, "calm wind too strong at {t}: {w}");
            let d = direction(t);
            assert!(d.dot(prev) > 0.999, "wind turned too fast at {t}");
            prev = d;
        }
        let storm = (0..200).map(|k| at(k as f32, Vec2::ZERO, 1.0, true).length()).fold(0.0f32, f32::max);
        assert!(storm > 1.5 && storm < 4.0, "{storm}");
    }
}
