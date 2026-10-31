//! Keeping worlds safe (lote 17): automatic backups (the 3 newest of each world), and renaming,
//! duplicating, deleting and bringing back worlds from the world list (menus.rs).
//!
//! Everything lives next to `saves/`: `backups/<world>/<YYYYMMDD-HHMMSS>/` holds a full copy of
//! the world folder. Deleting a world moves it there as `<stamp>-apagado`, so a deleted world can
//! be brought back from the list until three newer backups of that name push it out.

use std::path::{Path, PathBuf};

/// Backups kept per world (the oldest go).
pub const KEEP: usize = 3;
/// A new backup when entering a world only if the newest is older than this (seconds), so going
/// in and out a few times doesn't push the good ones out.
pub const MIN_GAP: u64 = 20 * 60;
/// Suffix of a deleted world's folder in `backups/`.
pub const DELETED: &str = "-apagado";

pub fn saves(base: &Path) -> PathBuf {
    base.join("saves")
}

pub fn backups(base: &Path) -> PathBuf {
    base.join("backups")
}

/// A world name the file system takes: no path or reserved characters, trimmed, not empty.
pub fn clean_name(text: &str) -> String {
    let name: String = text.trim().chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') && !c.is_control()).collect();
    let name = name.trim().trim_matches('.').trim().to_string();
    if name.is_empty() {
        "Novo mundo".into()
    } else {
        name
    }
}

/// `name`, or `name (2)`, `name (3)`... whichever has no world folder yet.
pub fn unique_name(base: &Path, name: &str) -> String {
    let mut out = name.to_string();
    let mut n = 2;
    while saves(base).join(&out).exists() {
        out = format!("{name} ({n})");
        n += 1;
    }
    out
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Seconds since the epoch as `YYYYMMDD-HHMMSS` (UTC), which sorts by time.
pub fn stamp(secs: u64) -> String {
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// Copy a folder with everything in it, leaving half-written `.tmp` files out.
pub fn copy_dir(from: &Path, to: &Path) -> std::io::Result<u64> {
    std::fs::create_dir_all(to)?;
    let mut bytes = 0;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let path = e.path();
        let dest = to.join(e.file_name());
        let kind = e.file_type()?;
        if kind.is_dir() {
            bytes += copy_dir(&path, &dest)?;
        } else if kind.is_file() && path.extension().is_none_or(|x| x != "tmp") {
            bytes += std::fs::copy(&path, &dest)?;
        }
    }
    Ok(bytes)
}

/// One backup of a world: its folder name (the stamp, maybe with `-apagado`) and full path.
#[derive(Clone, Debug, PartialEq)]
pub struct Backup {
    pub stamp: String,
    pub path: PathBuf,
    pub deleted: bool,
}

/// A world's backups, newest first.
pub fn list(base: &Path, world: &str) -> Vec<Backup> {
    let mut v: Vec<Backup> = std::fs::read_dir(backups(base).join(world))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("level.ron").exists())
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            Backup { deleted: name.ends_with(DELETED), stamp: name.trim_end_matches(DELETED).to_string(), path: e.path() }
        })
        .collect();
    v.sort_by(|a, b| b.stamp.cmp(&a.stamp));
    v
}

/// Drop the oldest backups past `KEEP`.
fn prune(base: &Path, world: &str) {
    for old in list(base, world).into_iter().skip(KEEP) {
        let _ = std::fs::remove_dir_all(old.path);
    }
}

/// Back the world up now (unless the newest backup is younger than `min_gap` seconds).
/// Returns the new backup's folder.
pub fn backup(base: &Path, world: &str, min_gap: u64) -> Option<PathBuf> {
    let src = saves(base).join(world);
    if !src.join("level.ron").exists() || src.join(crate::testworld::MARKER).exists() {
        return None;
    }
    let t = now();
    let newest = list(base, world).into_iter().find(|b| !b.deleted);
    if min_gap > 0 && newest.is_some_and(|b| b.stamp >= stamp(t.saturating_sub(min_gap))) {
        return None;
    }
    let mut dest = backups(base).join(world).join(stamp(t));
    let mut n = 2;
    while dest.exists() {
        dest = backups(base).join(world).join(format!("{}-{n}", stamp(t)));
        n += 1;
    }
    let part = dest.with_extension("tmp");
    let _ = std::fs::remove_dir_all(&part);
    match copy_dir(&src, &part).and_then(|_| std::fs::rename(&part, &dest)) {
        Ok(()) => {
            prune(base, world);
            Some(dest)
        }
        Err(e) => {
            log::warn!("backup de {world}: {e}");
            let _ = std::fs::remove_dir_all(&part);
            None
        }
    }
}

/// The same, on a thread of its own (entering a big world shouldn't wait for the copy).
pub fn backup_later(base: &Path, world: &str) {
    let (base, world) = (base.to_path_buf(), world.to_string());
    let _ = std::thread::Builder::new().name("world-backup".into()).spawn(move || {
        if let Some(p) = backup(&base, &world, MIN_GAP) {
            log::info!("backup do mundo em {}", p.display());
        }
    });
}

/// Delete a world: its folder moves into its backups as `<stamp>-apagado`.
pub fn delete(base: &Path, world: &str) -> std::io::Result<PathBuf> {
    let src = saves(base).join(world);
    let dir = backups(base).join(world);
    std::fs::create_dir_all(&dir)?;
    let mut dest = dir.join(format!("{}{DELETED}", stamp(now())));
    let mut n = 2;
    while dest.exists() {
        dest = dir.join(format!("{}-{n}{DELETED}", stamp(now())));
        n += 1;
    }
    if std::fs::rename(&src, &dest).is_err() {
        // Another drive: copy, then remove.
        copy_dir(&src, &dest)?;
        std::fs::remove_dir_all(&src)?;
    }
    Ok(dest)
}

/// Rename a world (its folder and its backups). Returns the name it got.
pub fn rename(base: &Path, from: &str, to: &str) -> std::io::Result<String> {
    let to = clean_name(to);
    if to == from {
        return Ok(to);
    }
    let to = unique_name(base, &to);
    std::fs::rename(saves(base).join(from), saves(base).join(&to))?;
    let (old_b, new_b) = (backups(base).join(from), backups(base).join(&to));
    if old_b.exists() && !new_b.exists() {
        let _ = std::fs::rename(old_b, new_b);
    }
    Ok(to)
}

/// A copy of a world under a new name, e.g. `Mundo 1 (cópia)`. Returns its name.
pub fn duplicate(base: &Path, world: &str) -> std::io::Result<String> {
    let name = unique_name(base, &format!("{world} (cópia)"));
    copy_dir(&saves(base).join(world), &saves(base).join(&name))?;
    Ok(name)
}

/// Bring a backup back as a world of its own (never over the world it came from):
/// a deleted world gets its own name back if it is free, a backup is `Mundo 1 (backup)`.
pub fn restore(base: &Path, world: &str, b: &Backup) -> std::io::Result<String> {
    let want = if b.deleted { world.to_string() } else { format!("{world} (backup)") };
    let name = unique_name(base, &want);
    let dest = saves(base).join(&name);
    if b.deleted {
        std::fs::rename(&b.path, &dest).or_else(|_| copy_dir(&b.path, &dest).map(|_| ()))?;
    } else {
        copy_dir(&b.path, &dest)?;
    }
    Ok(name)
}

/// Worlds that were deleted and can still come back: (world name, its newest deleted copy).
pub fn deleted(base: &Path) -> Vec<(String, Backup)> {
    let mut v = Vec::new();
    for e in std::fs::read_dir(backups(base)).into_iter().flatten().flatten() {
        let world = e.file_name().to_string_lossy().to_string();
        if saves(base).join(&world).exists() {
            continue;
        }
        if let Some(b) = list(base, &world).into_iter().find(|b| b.deleted) {
            v.push((world, b));
        }
    }
    v.sort_by(|a, b| b.1.stamp.cmp(&a.1.stamp));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> PathBuf {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("mc-worlds-{}-{k}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("saves")).unwrap();
        d
    }

    fn world(base: &Path, name: &str) {
        let d = saves(base).join(name);
        std::fs::create_dir_all(d.join("chunks")).unwrap();
        std::fs::write(d.join("level.ron"), "()").unwrap();
        std::fs::write(d.join("chunks/0_0.bin"), [1u8; 64]).unwrap();
        std::fs::write(d.join("inv.ron.tmp"), "half").unwrap();
    }

    #[test]
    fn stamps_sort_and_read() {
        assert_eq!(stamp(0), "19700101-000000");
        assert_eq!(stamp(1_793_458_743), "20261031-145903");
        assert_eq!(clean_name("  a/b:c?  "), "abc");
        assert_eq!(clean_name(" .. "), "Novo mundo");
    }

    #[test]
    fn backups_keep_three_and_skip_tmp() {
        let b = base();
        world(&b, "M");
        for _ in 0..5 {
            assert!(backup(&b, "M", 0).is_some());
        }
        let l = list(&b, "M");
        assert_eq!(l.len(), KEEP);
        assert!(l[0].path.join("chunks/0_0.bin").exists());
        assert!(!l[0].path.join("inv.ron.tmp").exists());
        // Too soon for the entering backup.
        assert!(backup(&b, "M", MIN_GAP).is_none());
        let _ = std::fs::remove_dir_all(b);
    }

    #[test]
    fn delete_rename_duplicate_restore() {
        let b = base();
        world(&b, "M");
        world(&b, "N");
        assert_eq!(duplicate(&b, "M").unwrap(), "M (cópia)");
        assert_eq!(rename(&b, "M (cópia)", "N").unwrap(), "N (2)");
        assert!(saves(&b).join("N (2)/level.ron").exists());
        delete(&b, "N").unwrap();
        assert!(!saves(&b).join("N").exists());
        let gone = deleted(&b);
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].0, "N");
        assert_eq!(restore(&b, "N", &gone[0].1).unwrap(), "N");
        assert!(saves(&b).join("N/chunks/0_0.bin").exists());
        assert!(deleted(&b).is_empty());
        let bk = backup(&b, "M", 0).unwrap();
        let back = list(&b, "M").into_iter().find(|x| x.path == bk).unwrap();
        let name = restore(&b, "M", &back).unwrap();
        assert_eq!(name, "M (backup)");
        assert!(saves(&b).join("M/level.ron").exists(), "o original fica");
        let _ = std::fs::remove_dir_all(b);
    }
}
