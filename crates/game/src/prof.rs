//! Where a frame's CPU time goes: the update is cut into named stretches (`mark` at the end of
//! each one), and the time of each is summed. The benchmark (bench.rs) prints the stretches that
//! cost the most per frame, so a slow part of the game shows by name instead of hiding in the
//! frame time. A mark costs one clock read.

use std::time::Instant;

pub struct Prof {
    last: Instant,
    /// (name, seconds in total, times seen), in the order first seen.
    acc: Vec<(&'static str, f64, u32)>,
    pub frames: u32,
    /// The same over the last couple of seconds, for the F3 performance panel: what is being
    /// summed now, its frames and age, and the last finished window's costliest stretches.
    win: Vec<(&'static str, f64)>,
    win_frames: u32,
    win_t: f64,
    pub recent: Vec<(&'static str, f64)>,
}

impl Default for Prof {
    fn default() -> Self {
        Prof { last: Instant::now(), acc: Vec::new(), frames: 0, win: Vec::new(), win_frames: 0, win_t: 0.0, recent: Vec::new() }
    }
}

impl Prof {
    /// A new frame starts: what happened since the last mark is not counted.
    pub fn begin(&mut self) {
        self.last = Instant::now();
        self.frames += 1;
    }

    /// The stretch since the last mark (or `begin`) was `name`.
    pub fn mark(&mut self, name: &'static str) {
        let now = Instant::now();
        let d = (now - self.last).as_secs_f64();
        self.last = now;
        self.add(name, d);
    }

    /// Time measured elsewhere (the renderer's own clock), counted under `name`.
    pub fn add(&mut self, name: &'static str, secs: f64) {
        match self.win.iter_mut().find(|a| a.0 == name) {
            Some(a) => a.1 += secs,
            None => self.win.push((name, secs)),
        }
        match self.acc.iter_mut().find(|a| a.0 == name) {
            Some(a) => {
                a.1 += secs;
                a.2 += 1;
            }
            None => self.acc.push((name, secs, 1)),
        }
    }

    /// A frame ended (`dt` seconds): every 2 seconds the window's table moves to `recent`.
    pub fn roll(&mut self, dt: f64) {
        self.win_frames += 1;
        self.win_t += dt;
        if self.win_t < 2.0 {
            return;
        }
        let n = self.win_frames.max(1) as f64;
        let mut v: Vec<(&'static str, f64)> = self.win.iter().map(|a| (a.0, a.1 * 1000.0 / n)).collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v.truncate(6);
        self.recent = v;
        self.win.clear();
        self.win_frames = 0;
        self.win_t = 0.0;
    }

    pub fn reset(&mut self) {
        self.acc.clear();
        self.frames = 0;
    }

    /// The costliest stretches: (name, milliseconds per frame), most first.
    pub fn top(&self, n: usize) -> Vec<(&'static str, f64)> {
        let frames = self.frames.max(1) as f64;
        let mut v: Vec<(&'static str, f64)> = self.acc.iter().map(|a| (a.0, a.1 * 1000.0 / frames)).collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v.truncate(n);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stretches_add_up_by_name() {
        let mut p = Prof::default();
        for _ in 0..3 {
            p.begin();
            std::thread::sleep(std::time::Duration::from_millis(2));
            p.mark("slow");
            p.mark("fast");
        }
        let top = p.top(5);
        assert_eq!(top[0].0, "slow");
        assert!(top[0].1 >= 1.5, "{top:?}");
        assert_eq!(top.len(), 2);
    }
}
