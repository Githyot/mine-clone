//! Automatic benchmark (Lote 8 of plan/o-que-falta.md): `mine-clone --bench [seconds]
//! [--world NAME] [--render-distance N] [--lod BLOCKS]`. Once the world has loaded, the
//! player flies straight ahead at 20 blocks a second, high over the ground, so new chunks and
//! distant terrain keep coming in; every frame time is kept. At the end a report goes to the
//! terminal and to `bench/bench-<time>.txt` next to the game, then the game closes. Run it
//! before and after a change (same world, seed and settings) to compare.

use glam::Vec3;

use crate::{Game, Screen};

const SPEED: f32 = 20.0;
const HEIGHT: f32 = 150.0;

#[derive(Default)]
pub(crate) struct Bench {
    pub(crate) started: bool,
    t: f32,
    frames: Vec<f32>,
    meshed_at_start: usize,
    from: Vec3,
}

/// Frame-time summary: (average fps, median ms, 1% low fps, worst ms).
pub(crate) fn summary(frames: &[f32]) -> (f32, f32, f32, f32) {
    if frames.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let mut v = frames.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let total: f32 = v.iter().sum();
    let median = v[v.len() / 2];
    // The slowest 1% of frames, averaged.
    let k = (v.len() / 100).max(1);
    let low = v[v.len() - k..].iter().sum::<f32>() / k as f32;
    (v.len() as f32 / total.max(1e-6), median * 1000.0, 1.0 / low.max(1e-6), v[v.len() - 1] * 1000.0)
}

impl Game {
    /// Every frame with `--bench`: fly, measure, and at the end report. True = close the game.
    pub(crate) fn bench_tick(&mut self, raw_dt: f32) -> bool {
        let Some(secs) = self.args.bench else { return false };
        if !self.world_open || self.screen != Screen::Playing {
            return false;
        }
        if !self.bench.started {
            self.bench.started = true;
            self.survival = false;
            self.body.flying = true;
            self.yaw = -std::f32::consts::FRAC_PI_2;
            self.pitch = -0.15;
            self.bench.meshed_at_start = self.world.meshed_count;
            self.body.pos.y = HEIGHT;
            self.bench.from = self.body.pos;
            log::info!("benchmark: flying for {secs:.0} s");
            return false;
        }
        self.bench.t += raw_dt;
        self.bench.frames.push(raw_dt);
        let p = self.bench.from + Vec3::X * SPEED * self.bench.t;
        self.body.pos = p;
        self.body.vel = Vec3::ZERO;
        if self.bench.t < secs {
            return false;
        }
        let (avg, median, low, worst) = summary(&self.bench.frames);
        let meshed = self.world.meshed_count - self.bench.meshed_at_start;
        let report = format!(
            "Mine Clone {} benchmark\n\
             world: {} (seed {}), flew {:.0} blocks in {:.1} s\n\
             settings: render distance {}, distant terrain +{} blocks, shader {}, window {}x{}\n\
             frames: {}\n\
             average: {:.1} fps\n\
             median frame: {:.2} ms\n\
             1% low: {:.1} fps\n\
             worst frame: {:.1} ms\n\
             chunks meshed: {} ({:.1} per second)\n\
             distant terrain tiles: {}\n",
            env!("CARGO_PKG_VERSION"),
            self.world_name,
            self.seed,
            SPEED * self.bench.t,
            self.bench.t,
            self.settings.render_distance,
            self.settings.lod_distance,
            self.shader_name(),
            self.window.inner_size().width,
            self.window.inner_size().height,
            self.bench.frames.len(),
            avg,
            median,
            low,
            worst,
            meshed,
            meshed as f32 / self.bench.t.max(0.001),
            self.lod.tiles.len(),
        );
        println!("{report}");
        let dir = self.base.join("bench");
        let _ = std::fs::create_dir_all(&dir);
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let path = dir.join(format!("bench-{stamp}.txt"));
        match std::fs::write(&path, &report) {
            Ok(()) => log::info!("benchmark report: {}", path.display()),
            Err(e) => log::error!("could not write the benchmark report: {e}"),
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_of_frame_times() {
        let mut f = vec![0.01f32; 99];
        f.push(0.1);
        let (avg, median, low, worst) = summary(&f);
        assert!((avg - 100.0 / 1.09).abs() < 0.5, "{avg}");
        assert!((median - 10.0).abs() < 0.01);
        assert!((low - 10.0).abs() < 0.01);
        assert!((worst - 100.0).abs() < 0.01);
    }
}
