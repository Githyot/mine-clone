//! F9 performance panel: frame rate statistics measured every frame, and machine stats (CPU, RAM,
//! temperatures, GPU, disk) sampled once a second on a background thread so the game never waits
//! on the operating system.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Seconds of frames the min/avg/max numbers cover.
const WINDOW: f32 = 10.0;

pub struct FpsStats {
    frames: VecDeque<f32>,
    total: f32,
}

#[derive(Clone, Copy, Default)]
pub struct FpsSummary {
    pub current: f32,
    pub average: f32,
    pub high: f32,
    /// Average of the slowest 1% of frames, as frames per second.
    pub low1: f32,
    pub frame_ms: f32,
    pub worst_ms: f32,
}

impl FpsStats {
    pub fn new() -> FpsStats {
        FpsStats { frames: VecDeque::new(), total: 0.0 }
    }

    pub fn push(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        self.frames.push_back(dt);
        self.total += dt;
        while self.total > WINDOW && self.frames.len() > 1 {
            self.total -= self.frames.pop_front().unwrap();
        }
    }

    pub fn reset(&mut self) {
        self.frames.clear();
        self.total = 0.0;
    }

    /// The most recent frame times, oldest first, for the graph.
    pub fn recent(&self, n: usize) -> impl Iterator<Item = f32> + '_ {
        self.frames.iter().skip(self.frames.len().saturating_sub(n)).copied()
    }

    pub fn summary(&self) -> FpsSummary {
        if self.frames.is_empty() {
            return FpsSummary::default();
        }
        // "Current" is the last half second, so the number is readable instead of flickering.
        let (mut t, mut n) = (0.0, 0);
        for &dt in self.frames.iter().rev() {
            t += dt;
            n += 1;
            if t >= 0.5 {
                break;
            }
        }
        let mut sorted: Vec<f32> = self.frames.iter().copied().collect();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let worst = (sorted.len() / 100).max(1);
        let worst_avg = sorted[..worst].iter().sum::<f32>() / worst as f32;
        // "High" ignores the fastest 1% so a single empty frame doesn't show a silly number.
        let fastest = sorted[sorted.len() - 1 - (sorted.len() / 100).min(sorted.len() - 1)];
        FpsSummary {
            current: n as f32 / t,
            average: self.frames.len() as f32 / self.total,
            high: 1.0 / fastest,
            low1: 1.0 / worst_avg,
            frame_ms: t / n as f32 * 1000.0,
            worst_ms: sorted[0] * 1000.0,
        }
    }
}

/// One sample of the machine. `None` means the system doesn't tell us (shown as "n/d").
#[derive(Clone, Default)]
pub struct SysStats {
    pub ready: bool,
    pub cores: usize,
    pub cpu_name: String,
    pub cpu_total: f32,
    pub cpu_game: f32,
    pub ram_game_gb: f64,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub cpu_temp: Option<f32>,
    pub gpu_usage: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub vram_used_gb: Option<f64>,
    pub vram_total_gb: Option<f64>,
    pub game_read_mbs: f64,
    pub game_write_mbs: f64,
    pub disk_read_mbs: Option<f64>,
    pub disk_write_mbs: Option<f64>,
}

pub struct SysMonitor {
    stats: Arc<Mutex<SysStats>>,
    active: Arc<AtomicBool>,
    started: bool,
}

impl SysMonitor {
    pub fn new() -> SysMonitor {
        SysMonitor { stats: Arc::new(Mutex::new(SysStats::default())), active: Arc::new(AtomicBool::new(false)), started: false }
    }

    /// Sampling only runs while the panel is open.
    pub fn set_active(&mut self, on: bool) {
        self.active.store(on, Ordering::Relaxed);
        if on && !self.started {
            self.started = true;
            let stats = self.stats.clone();
            let active = self.active.clone();
            let _ = std::thread::Builder::new().name("monitor".into()).spawn(move || sampler(stats, active));
        }
    }

    pub fn stats(&self) -> SysStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

fn sampler(out: Arc<Mutex<SysStats>>, active: Arc<AtomicBool>) {
    use sysinfo::{Components, DiskRefreshKind, Disks, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

    let pid = sysinfo::get_current_pid().ok();
    let mut sys = System::new_with_specifics(RefreshKind::nothing());
    sys.refresh_cpu_all();
    let cpu_name = sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
    let cores = sys.cpus().len().max(1);
    let mut components = Components::new_with_refreshed_list();
    let disk_kind = DiskRefreshKind::nothing().with_io_usage();
    let mut disks = Disks::new_with_refreshed_list_specifics(disk_kind);
    let mut gpu = gpu::Gpu::new();
    let proc_kind = ProcessRefreshKind::nothing().with_cpu().with_memory().with_disk_usage();
    if let Some(pid) = pid {
        // CPU use is a difference between two readings; take the first one now.
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, proc_kind);
    }
    let mut last = std::time::Instant::now();
    loop {
        if !active.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(250));
            continue;
        }
        std::thread::sleep(Duration::from_millis(1000));
        let secs = last.elapsed().as_secs_f64().max(0.001);
        last = std::time::Instant::now();

        sys.refresh_cpu_usage();
        sys.refresh_memory();
        if let Some(pid) = pid {
            sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, proc_kind);
        }
        components.refresh(false);
        disks.refresh_specifics(true, disk_kind);

        let mut s = SysStats { ready: true, cores, cpu_name: cpu_name.clone(), ..Default::default() };
        s.cpu_total = sys.global_cpu_usage();
        s.ram_used_gb = sys.used_memory() as f64 / 1e9;
        s.ram_total_gb = sys.total_memory() as f64 / 1e9;
        if let Some(p) = pid.and_then(|pid| sys.process(pid)) {
            // sysinfo counts 100% per core; show it as a share of the whole CPU like Task Manager.
            s.cpu_game = p.cpu_usage() / cores as f32;
            s.ram_game_gb = p.memory() as f64 / 1e9;
            let d = p.disk_usage();
            s.game_read_mbs = d.read_bytes as f64 / 1e6 / secs;
            s.game_write_mbs = d.written_bytes as f64 / 1e6 / secs;
        }
        s.cpu_temp = cpu_temperature(&components);
        let (mut r, mut w, mut any) = (0u64, 0u64, false);
        for d in disks.list() {
            let u = d.usage();
            r += u.read_bytes;
            w += u.written_bytes;
            any |= u.total_read_bytes > 0 || u.total_written_bytes > 0;
        }
        if any {
            s.disk_read_mbs = Some(r as f64 / 1e6 / secs);
            s.disk_write_mbs = Some(w as f64 / 1e6 / secs);
        }
        gpu.sample(&mut s, &components);
        if let Ok(mut o) = out.lock() {
            *o = s;
        }
    }
}

fn cpu_temperature(components: &sysinfo::Components) -> Option<f32> {
    let mut best: Option<f32> = None;
    for c in components.list() {
        let l = c.label().to_lowercase();
        let cpu_like = ["cpu", "package", "tctl", "tdie", "core", "k10temp", "coretemp", "acpi", "thermal"].iter().any(|k| l.contains(k));
        if !cpu_like || l.contains("gpu") || l.contains("amdgpu") || l.contains("nvme") {
            continue;
        }
        if let Some(t) = c.temperature().filter(|t| t.is_finite() && *t > 0.0 && *t < 150.0) {
            best = Some(best.map_or(t, |b: f32| b.max(t)));
        }
    }
    best
}

/// GPU usage, temperature and memory. NVIDIA cards answer through NVML (the driver's own library);
/// on Windows every other card answers through the same counters Task Manager reads; on Linux
/// AMD cards answer through sysfs.
mod gpu {
    use super::SysStats;

    pub struct Gpu {
        nvml: Option<nvml_wrapper::Nvml>,
        #[cfg(windows)]
        pdh: Option<pdh::Query>,
    }

    impl Gpu {
        pub fn new() -> Gpu {
            Gpu {
                nvml: nvml_wrapper::Nvml::init().ok(),
                #[cfg(windows)]
                pdh: pdh::Query::new(),
            }
        }

        pub fn sample(&mut self, s: &mut SysStats, components: &sysinfo::Components) {
            if let Some(dev) = self.nvml.as_ref().and_then(|n| n.device_by_index(0).ok()) {
                s.gpu_usage = dev.utilization_rates().ok().map(|u| u.gpu as f32);
                s.gpu_temp = dev.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu).ok().map(|t| t as f32);
                if let Ok(m) = dev.memory_info() {
                    s.vram_used_gb = Some(m.used as f64 / 1e9);
                    s.vram_total_gb = Some(m.total as f64 / 1e9);
                }
            }
            #[cfg(windows)]
            if let Some(q) = self.pdh.as_mut() {
                if let Some((usage, vram)) = q.sample() {
                    s.gpu_usage.get_or_insert(usage);
                    if vram > 0.0 {
                        s.vram_used_gb.get_or_insert(vram / 1e9);
                    }
                }
            }
            #[cfg(target_os = "linux")]
            linux(s);
            if s.gpu_temp.is_none() {
                for c in components.list() {
                    let l = c.label().to_lowercase();
                    if l.contains("gpu") || l.contains("amdgpu") || l.contains("radeon") || l.contains("nouveau") {
                        if let Some(t) = c.temperature().filter(|t| t.is_finite() && *t > 0.0) {
                            s.gpu_temp = Some(t);
                            break;
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn linux(s: &mut SysStats) {
        let Ok(dir) = std::fs::read_dir("/sys/class/drm") else { return };
        for e in dir.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }
            let dev = e.path().join("device");
            let read = |f: &str| std::fs::read_to_string(dev.join(f)).ok().and_then(|t| t.trim().parse::<f64>().ok());
            if let Some(busy) = read("gpu_busy_percent") {
                s.gpu_usage.get_or_insert(busy as f32);
                if let (Some(u), Some(t)) = (read("mem_info_vram_used"), read("mem_info_vram_total")) {
                    s.vram_used_gb.get_or_insert(u / 1e9);
                    s.vram_total_gb.get_or_insert(t / 1e9);
                }
                return;
            }
        }
    }

    #[cfg(windows)]
    mod pdh {
        use windows_sys::Win32::System::Performance::*;

        const MORE_DATA: u32 = 0x800007D2;

        pub struct Query {
            query: PDH_HQUERY,
            engine: PDH_HCOUNTER,
            memory: PDH_HCOUNTER,
        }

        // The handles are only used from the monitor thread.
        unsafe impl Send for Query {}

        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }

        impl Query {
            pub fn new() -> Option<Query> {
                unsafe {
                    let mut query: PDH_HQUERY = std::mem::zeroed();
                    if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != 0 {
                        return None;
                    }
                    let mut engine: PDH_HCOUNTER = std::mem::zeroed();
                    let mut memory: PDH_HCOUNTER = std::mem::zeroed();
                    let e = wide("\\GPU Engine(*engtype_3D)\\Utilization Percentage");
                    let m = wide("\\GPU Adapter Memory(*)\\Dedicated Usage");
                    if PdhAddEnglishCounterW(query, e.as_ptr(), 0, &mut engine) != 0 {
                        PdhCloseQuery(query);
                        return None;
                    }
                    PdhAddEnglishCounterW(query, m.as_ptr(), 0, &mut memory);
                    PdhCollectQueryData(query);
                    Some(Query { query, engine, memory })
                }
            }

            fn sum(counter: PDH_HCOUNTER) -> Option<f64> {
                unsafe {
                    let mut size = 0u32;
                    let mut count = 0u32;
                    let r = PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, std::ptr::null_mut());
                    if r != MORE_DATA || size == 0 {
                        return None;
                    }
                    // `size` is in bytes and covers the items plus the instance names after them.
                    let mut buf = vec![0u64; (size as usize).div_ceil(8)];
                    let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
                    if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, items) != 0 {
                        return None;
                    }
                    let mut total = 0.0;
                    for i in 0..count as usize {
                        let v = &(*items.add(i)).FmtValue;
                        if v.CStatus == 0 {
                            total += v.Anonymous.doubleValue;
                        }
                    }
                    Some(total)
                }
            }

            /// (3D engine usage %, dedicated video memory in bytes)
            pub fn sample(&mut self) -> Option<(f32, f64)> {
                unsafe {
                    if PdhCollectQueryData(self.query) != 0 {
                        return None;
                    }
                }
                let usage = Self::sum(self.engine)?;
                let vram = Self::sum(self.memory).unwrap_or(0.0);
                Some((usage.min(100.0) as f32, vram))
            }
        }

        impl Drop for Query {
            fn drop(&mut self) {
                unsafe {
                    PdhCloseQuery(self.query);
                }
            }
        }
    }
}
