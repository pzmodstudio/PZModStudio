//! Save backups: a zip of a save folder, checked after it is written, before anything risky
//! happens to that save.
//!
//! A save is `<Zomboid>/Saves/<mode>/<world>/`, thousands of small files the game rewrites as it
//! plays. Losing one to a mod update, a mod taken out, or a list the game emptied is what players
//! fear most, and Steam keeps no history of either. So the Studio zips a save:
//! - when the player leaves it (back to the menu, or the game closes while in it),
//! - before it loads with mods that changed since its last session (the Link mod asks and waits),
//! - before the Studio or the Link changes it (giving it the profile's mods, a restore),
//! - when the player asks.
//!
//! Each backup is `<Zomboid>/PZModStudio_SaveBackups/<mode>/<world>/<stamp>-<reason>.zip` with a
//! `.json` beside it. The zip is written as `.part`, opened again and read to the end (which
//! checks every entry's CRC) and compared with the files it was made from; only then is it named
//! `.zip` and given its `.json`, so a backup with a `.json` is a whole, readable one. Nothing is
//! started without the room for it on the disk. A save that has not changed since its newest
//! backup is not zipped again. The newest `KEEP_AUTOMATIC` automatic backups of a save are kept;
//! the ones the player made or marked are kept until the player removes them.

pub mod inside;
pub mod maps;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub const BACKUP_DIR: &str = "PZModStudio_SaveBackups";
/// Automatic backups kept per save; older ones go when a new one is made.
pub const KEEP_AUTOMATIC: usize = 5;
/// Room left free on the disk after a backup, on top of the save's own size.
const SPARE_BYTES: u64 = 256 * 1024 * 1024;

/// Why a backup was made. The ones the player makes are kept until removed.
pub const REASON_MANUAL: &str = "manual";
pub const REASON_EXIT: &str = "exit";
pub const REASON_BEFORE_LOAD: &str = "before-load";
pub const REASON_BEFORE_FIX: &str = "before-fix";
pub const REASON_BEFORE_RESTORE: &str = "before-restore";
const REASONS: &[&str] = &[REASON_MANUAL, REASON_EXIT, REASON_BEFORE_LOAD, REASON_BEFORE_FIX, REASON_BEFORE_RESTORE];

/// One backup of one save, as its `.json` describes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupEntry {
    /// The zip's file name.
    pub file: String,
    pub reason: String,
    /// When it was made (unix seconds).
    pub at: u64,
    /// The zip's size on disk.
    pub zip_bytes: u64,
    /// The save's size and file count when it was made.
    pub save_bytes: u64,
    pub files: u64,
    /// Newest modification time in the save then: unchanged since means nothing to back up.
    pub save_newest: u64,
    /// The save's mod list then (its mods.txt).
    pub mods: Vec<String>,
    /// Kept until the player removes it (never pruned).
    pub kept: bool,
    /// Opened again after writing and every entry read back with a matching CRC.
    pub verified: bool,
}

/// What a backup call did.
#[derive(Debug, Clone, Serialize)]
pub struct BackupOutcome {
    pub entry: BackupEntry,
    /// The save had not changed since this (already existing) backup: nothing was written.
    pub unchanged: bool,
    /// Automatic backups taken out to keep `KEEP_AUTOMATIC`.
    pub pruned: Vec<String>,
}

/// A save folder as it is on disk now.
#[derive(Debug, Clone, Default)]
struct Scan {
    bytes: u64,
    /// Files, relative to the save, with `/`, and their sizes.
    files: Vec<(String, u64)>,
    /// Folders with nothing in them, so a restore gives them back.
    empty_dirs: Vec<String>,
    newest: u64,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn mtime_secs(meta: &fs::Metadata) -> u64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn saves_dir(user_zomboid_dir: &Path) -> PathBuf {
    user_zomboid_dir.join("Saves")
}

fn backups_root(user_zomboid_dir: &Path) -> PathBuf {
    user_zomboid_dir.join(BACKUP_DIR)
}

/// `<mode>/<world>` -> the two names, refusing anything that is not exactly two plain names.
fn split_id(save_id: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = save_id.split(['/', '\\']).collect();
    let plain = |p: &str| {
        !p.is_empty() && p != "." && p != ".." && !p.starts_with('.') && Path::new(p).components().all(|c| matches!(c, Component::Normal(_)))
    };
    match parts.as_slice() {
        [mode, world] if plain(mode) && plain(world) => Ok((mode.to_string(), world.to_string())),
        _ => Err(format!("\"{}\" is not a save (expected <mode>/<world>).", save_id)),
    }
}

fn save_dir(user_zomboid_dir: &Path, save_id: &str) -> Result<PathBuf, String> {
    let (mode, world) = split_id(save_id)?;
    Ok(saves_dir(user_zomboid_dir).join(mode).join(world))
}

fn backup_dir_of(user_zomboid_dir: &Path, save_id: &str) -> Result<PathBuf, String> {
    let (mode, world) = split_id(save_id)?;
    Ok(backups_root(user_zomboid_dir).join(mode).join(world))
}

/// Every save on disk, as `<mode>/<world>`.
pub fn list_saves(user_zomboid_dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(modes) = fs::read_dir(saves_dir(user_zomboid_dir)) else { return out };
    for mode in modes.flatten().filter(|e| e.path().is_dir()) {
        let Ok(worlds) = fs::read_dir(mode.path()) else { continue };
        for world in worlds.flatten().filter(|e| e.path().is_dir()) {
            let id = format!("{}/{}", mode.file_name().to_string_lossy(), world.file_name().to_string_lossy());
            if split_id(&id).is_ok() {
                out.push(id);
            }
        }
    }
    out.sort();
    out
}

fn scan(dir: &Path) -> Result<Scan, String> {
    let mut out = Scan::default();
    for entry in walkdir::WalkDir::new(dir).follow_links(false).min_depth(1) {
        let entry = entry.map_err(|e| format!("Cannot read the save: {}", e))?;
        let rel = entry.path().strip_prefix(dir).map_err(|e| e.to_string())?;
        let rel = rel.to_string_lossy().replace('\\', "/");
        let meta = entry.metadata().map_err(|e| format!("Cannot read {}: {}", rel, e))?;
        out.newest = out.newest.max(mtime_secs(&meta));
        if meta.is_dir() {
            if fs::read_dir(entry.path()).map(|mut d| d.next().is_none()).unwrap_or(false) {
                out.empty_dirs.push(rel);
            }
        } else if meta.is_file() {
            out.bytes += meta.len();
            out.files.push((rel, meta.len()));
        }
    }
    Ok(out)
}

/// The save's own mod list (its mods.txt), in order.
pub fn save_mods(save_dir: &Path) -> Vec<String> {
    let path = save_dir.join("mods.txt");
    if !path.is_file() {
        return Vec::new();
    }
    crate::load_order::ini_parser::parse_default_txt(&path)
}

/// Free and total bytes of the disk holding `path` (the nearest folder that exists).
pub fn disk_space(path: &Path) -> Option<(u64, u64)> {
    let mut probe = path.to_path_buf();
    while !probe.exists() {
        probe = probe.parent()?.to_path_buf();
    }
    disk_space_of(&probe)
}

#[cfg(windows)]
fn disk_space_of(path: &Path) -> Option<(u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let (mut free, mut total, mut _all_free) = (0u64, 0u64, 0u64);
    // SAFETY: a NUL-terminated path and three valid out pointers.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut _all_free) };
    (ok != 0).then_some((free, total))
}

#[cfg(not(windows))]
fn disk_space_of(_path: &Path) -> Option<(u64, u64)> {
    None
}

/// The backups of a save, newest first. Zips without their `.json` are not counted: they never
/// finished (or were never checked).
pub fn backups_of(user_zomboid_dir: &Path, save_id: &str) -> Vec<BackupEntry> {
    let Ok(dir) = backup_dir_of(user_zomboid_dir, save_id) else { return Vec::new() };
    let mut out: Vec<BackupEntry> = fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| serde_json::from_str::<BackupEntry>(&fs::read_to_string(e.path()).ok()?).ok())
        .filter(|b| dir.join(&b.file).is_file())
        .collect();
    out.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| b.file.cmp(&a.file)));
    out
}

fn sidecar(zip: &Path) -> PathBuf {
    zip.with_extension("json")
}

// Two backups of one save never run at once (the exit one and a before-load one could meet).
static BUSY: Mutex<Option<HashSet<String>>> = Mutex::new(None);

struct Busy(String);
impl Busy {
    fn take(save_id: &str) -> Result<Busy, String> {
        let mut guard = BUSY.lock().map_err(|_| "Backups are stuck; restart PZ Mod Studio.".to_string())?;
        let set = guard.get_or_insert_with(HashSet::new);
        if !set.insert(save_id.to_string()) {
            return Err(format!("This save is already being backed up ({}).", save_id));
        }
        Ok(Busy(save_id.to_string()))
    }
}
impl Drop for Busy {
    fn drop(&mut self) {
        if let Ok(mut guard) = BUSY.lock() {
            if let Some(set) = guard.as_mut() {
                set.remove(&self.0);
            }
        }
    }
}

/// Whether a backup of the save is running now.
pub fn is_busy(save_path: &str) -> bool {
    BUSY.lock().ok().and_then(|g| g.as_ref().map(|s| s.contains(save_path))).unwrap_or(false)
}

fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = n as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 { format!("{} B", n) } else { format!("{:.1} {}", v, UNITS[unit]) }
}

/// Zips the files of `scan` from `src` into `dest`.
fn write_zip(src: &Path, scan: &Scan, dest: &Path) -> Result<(), String> {
    let file = fs::File::create(dest).map_err(|e| format!("Cannot create {}: {}", dest.display(), e))?;
    let mut zip = zip::ZipWriter::new(io::BufWriter::new(file));
    // Fast deflate: a save is mostly binary chunks, and the player is waiting.
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(1))
        .large_file(true);
    for dir in &scan.empty_dirs {
        zip.add_directory(dir.as_str(), options).map_err(|e| format!("Cannot write the backup: {}", e))?;
    }
    for (rel, _) in &scan.files {
        zip.start_file(rel.as_str(), options).map_err(|e| format!("Cannot write the backup: {}", e))?;
        let mut input = fs::File::open(src.join(rel)).map_err(|e| format!("Cannot read {} of the save: {}", rel, e))?;
        io::copy(&mut input, &mut zip).map_err(|e| format!("Cannot back up {}: {}", rel, e))?;
    }
    let writer = zip.finish().map_err(|e| format!("Cannot finish the backup: {}", e))?;
    let file = writer.into_inner().map_err(|e| format!("Cannot finish the backup: {}", e.error()))?;
    file.sync_all().map_err(|e| format!("Cannot finish the backup: {}", e))
}

/// Opens a backup and reads every entry to the end, which checks its CRC. With `expected`, the
/// entries must also be exactly those files with those sizes. Returns the files it holds.
fn verify_zip(zip_path: &Path, expected: Option<&Scan>) -> Result<u64, String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Cannot open the backup: {}", e))?;
    let mut archive = zip::ZipArchive::new(io::BufReader::new(file)).map_err(|e| format!("The backup is not a readable zip: {}", e))?;
    let mut files = 0u64;
    let mut sizes: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("The backup has a damaged entry: {}", e))?;
        if entry.enclosed_name().is_none() {
            return Err(format!("The backup has an unsafe path: {}", entry.name()));
        }
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let read = io::copy(&mut entry, &mut io::sink()).map_err(|e| format!("{} does not read back from the backup: {}", name, e))?;
        if read != entry.size() {
            return Err(format!("{} reads back {} bytes from the backup, not {}.", name, read, entry.size()));
        }
        sizes.insert(name, read);
        files += 1;
    }
    if let Some(scan) = expected {
        if files != scan.files.len() as u64 {
            return Err(format!("The backup holds {} files; the save has {}.", files, scan.files.len()));
        }
        for (rel, size) in &scan.files {
            match sizes.get(rel) {
                Some(s) if s == size => {}
                Some(s) => return Err(format!("{} is {} bytes in the backup and {} in the save.", rel, s, size)),
                None => return Err(format!("{} is missing from the backup.", rel)),
            }
        }
    }
    Ok(files)
}

fn write_sidecar(zip_path: &Path, entry: &BackupEntry) -> Result<(), String> {
    let text = serde_json::to_string_pretty(entry).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(sidecar(zip_path), text).map_err(|e| format!("Cannot record the backup: {}", e))
}

/// Takes out the automatic backups of a save past the newest `KEEP_AUTOMATIC`. Never one that
/// is kept, and never `spare` (the one just made).
fn prune(dir: &Path, all: &[BackupEntry], spare: &str) -> Vec<String> {
    let mut removed = Vec::new();
    for entry in all.iter().filter(|b| !b.kept).skip(KEEP_AUTOMATIC) {
        if entry.file == spare {
            continue;
        }
        let zip = dir.join(&entry.file);
        if fs::remove_file(&zip).is_ok() {
            let _ = fs::remove_file(sidecar(&zip));
            removed.push(entry.file.clone());
        }
    }
    removed
}

/// Leftovers of backups that never finished (the app closed while zipping).
fn sweep_parts(dir: &Path) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_name().to_string_lossy().ends_with(".zip.part") {
            let _ = fs::remove_file(e.path());
        }
    }
}

/// Backs a save up. `kept`: never pruned (the player's own backups). A save unchanged since its
/// newest backup is not zipped again; that backup is returned with `unchanged`.
pub fn backup_save(user_zomboid_dir: &Path, save_id: &str, reason: &str, kept: bool) -> Result<BackupOutcome, String> {
    if !REASONS.contains(&reason) {
        return Err(format!("Unknown backup reason \"{}\".", reason));
    }
    let src = save_dir(user_zomboid_dir, save_id)?;
    if !src.is_dir() {
        return Err(format!("There is no save {}.", save_id));
    }
    let _busy = Busy::take(&src.to_string_lossy())?;
    let dir = backup_dir_of(user_zomboid_dir, save_id)?;
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {}", dir.display(), e))?;
    sweep_parts(&dir);

    let scan = scan(&src)?;
    let existing = backups_of(user_zomboid_dir, save_id);
    if let Some(newest) = existing.first() {
        if newest.verified && newest.save_newest == scan.newest && newest.save_bytes == scan.bytes && newest.files == scan.files.len() as u64 {
            let mut entry = newest.clone();
            if kept && !entry.kept {
                entry.kept = true;
                write_sidecar(&dir.join(&entry.file), &entry)?;
            }
            return Ok(BackupOutcome { entry, unchanged: true, pruned: Vec::new() });
        }
    }

    // Room for it: the zip can be as big as the save, and the disk keeps some spare.
    if let Some((free, _)) = disk_space(&dir) {
        let needed = scan.bytes + SPARE_BYTES;
        if free < needed {
            return Err(format!(
                "Not enough free space to back up {} ({}): the disk has {} free and the backup needs up to {}.",
                save_id,
                human_bytes(scan.bytes),
                human_bytes(free),
                human_bytes(needed)
            ));
        }
    }

    let at = now_secs();
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S-%3f").to_string();
    let mut name = format!("{}-{}.zip", stamp, reason);
    let mut n = 1;
    while dir.join(&name).exists() {
        n += 1;
        name = format!("{}-{}-{}.zip", stamp, reason, n);
    }
    let final_path = dir.join(&name);
    let part = dir.join(format!("{}.part", name));
    let result = write_zip(&src, &scan, &part).and_then(|_| verify_zip(&part, Some(&scan)));
    if let Err(e) = result {
        let _ = fs::remove_file(&part);
        return Err(e);
    }
    fs::rename(&part, &final_path).map_err(|e| {
        let _ = fs::remove_file(&part);
        format!("Cannot finish the backup: {}", e)
    })?;
    let entry = BackupEntry {
        file: name.clone(),
        reason: reason.to_string(),
        at,
        zip_bytes: fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0),
        save_bytes: scan.bytes,
        files: scan.files.len() as u64,
        save_newest: scan.newest,
        mods: save_mods(&src),
        kept,
        verified: true,
    };
    if let Err(e) = write_sidecar(&final_path, &entry) {
        let _ = fs::remove_file(&final_path);
        return Err(e);
    }
    let pruned = prune(&dir, &backups_of(user_zomboid_dir, save_id), &name);
    Ok(BackupOutcome { entry, unchanged: false, pruned })
}

/// The backup's file inside its save's folder, refusing any other path.
fn backup_file(user_zomboid_dir: &Path, save_id: &str, file: &str) -> Result<PathBuf, String> {
    if file.is_empty() || file.contains(['/', '\\']) || !file.ends_with(".zip") || file.starts_with('.') {
        return Err(format!("\"{}\" is not a backup.", file));
    }
    let path = backup_dir_of(user_zomboid_dir, save_id)?.join(file);
    if !path.is_file() {
        return Err(format!("There is no backup {} of {}.", file, save_id));
    }
    Ok(path)
}

/// Marks a backup kept (never pruned) or automatic again.
pub fn set_kept(user_zomboid_dir: &Path, save_id: &str, file: &str, kept: bool) -> Result<BackupEntry, String> {
    let zip = backup_file(user_zomboid_dir, save_id, file)?;
    let mut entry = backups_of(user_zomboid_dir, save_id)
        .into_iter()
        .find(|b| b.file == file)
        .ok_or_else(|| format!("{} has no record; it may not have finished.", file))?;
    entry.kept = kept;
    write_sidecar(&zip, &entry)?;
    Ok(entry)
}

/// Removes one backup (the zip and its record).
pub fn delete_backup(user_zomboid_dir: &Path, save_id: &str, file: &str) -> Result<(), String> {
    let zip = backup_file(user_zomboid_dir, save_id, file)?;
    fs::remove_file(&zip).map_err(|e| format!("Cannot remove {}: {}", file, e))?;
    let _ = fs::remove_file(sidecar(&zip));
    Ok(())
}

/// Puts a backup back as the save. The game must be closed. The backup is checked first, the
/// save as it is now is backed up first, the backup is unpacked beside the save, and only then
/// are the two swapped; if the swap fails the save is put back as it was.
pub fn restore_backup(user_zomboid_dir: &Path, save_id: &str, file: &str, game_running: bool) -> Result<Option<BackupOutcome>, String> {
    if game_running {
        return Err("Close Project Zomboid first: the game keeps the save open while it runs.".to_string());
    }
    let zip_path = backup_file(user_zomboid_dir, save_id, file)?;
    verify_zip(&zip_path, None).map_err(|e| format!("The backup cannot be restored: {}", e))?;
    let target = save_dir(user_zomboid_dir, save_id)?;
    let parent = target.parent().ok_or("The save has no folder.")?.to_path_buf();
    fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    let world = target.file_name().ok_or("The save has no name.")?.to_string_lossy().to_string();

    // Room to unpack it beside the save.
    let unpacked: u64 = {
        let f = fs::File::open(&zip_path).map_err(|e| e.to_string())?;
        let mut a = zip::ZipArchive::new(io::BufReader::new(f)).map_err(|e| e.to_string())?;
        (0..a.len()).filter_map(|i| a.by_index(i).ok().map(|e| e.size())).sum()
    };
    if let Some((free, _)) = disk_space(&parent) {
        if free < unpacked + SPARE_BYTES {
            return Err(format!("Not enough free space to restore: it unpacks to {} and the disk has {} free.", human_bytes(unpacked), human_bytes(free)));
        }
    }

    let before = if target.is_dir() { Some(backup_save(user_zomboid_dir, save_id, REASON_BEFORE_RESTORE, false)?) } else { None };

    let staging = parent.join(format!(".{}.pzms-restore", world));
    let old = parent.join(format!(".{}.pzms-old", world));
    if old.exists() {
        return Err(format!("{} is left from an earlier restore; move it away first.", old.display()));
    }
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let unpack = (|| -> Result<(), String> {
        let f = fs::File::open(&zip_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(io::BufReader::new(f)).map_err(|e| e.to_string())?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let rel = entry.enclosed_name().ok_or_else(|| format!("Unsafe path in the backup: {}", entry.name()))?;
            let out = staging.join(rel);
            if entry.is_dir() {
                fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(p) = out.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut w = fs::File::create(&out).map_err(|e| format!("Cannot write {}: {}", out.display(), e))?;
            io::copy(&mut entry, &mut w).map_err(|e| format!("Cannot unpack {}: {}", entry.name(), e))?;
        }
        Ok(())
    })();
    if let Err(e) = unpack {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    if target.exists() {
        fs::rename(&target, &old).map_err(|e| {
            let _ = fs::remove_dir_all(&staging);
            format!("Cannot move the save aside (is something using it?): {}", e)
        })?;
    }
    if let Err(e) = fs::rename(&staging, &target) {
        if old.exists() {
            let _ = fs::rename(&old, &target);
        }
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("Cannot put the backup in place; the save is as it was: {}", e));
    }
    // The save as it was is in the before-restore backup.
    let _ = fs::remove_dir_all(&old);
    Ok(before)
}

/// Copies a save to `<world>_copy` (or `_copy2`, ...) beside it, for playing a save whose mods
/// changed without risking the save itself. The copy is made in a hidden folder first and
/// only then given its name, so a half-made copy never shows in the game's Load list. Returns
/// the copy's `<mode>/<world>`.
pub fn copy_save(user_zomboid_dir: &Path, save_id: &str) -> Result<String, String> {
    let (mode, world) = split_id(save_id)?;
    let src = save_dir(user_zomboid_dir, save_id)?;
    if !src.is_dir() {
        return Err(format!("There is no save {}.", save_id));
    }
    let scan = scan(&src)?;
    let parent = src.parent().ok_or("The save has no folder.")?.to_path_buf();
    if let Some((free, _)) = disk_space(&parent) {
        if free < scan.bytes + SPARE_BYTES {
            return Err(format!("Not enough free space to copy {} ({}): the disk has {} free.", save_id, human_bytes(scan.bytes), human_bytes(free)));
        }
    }
    let mut name = format!("{}_copy", world);
    let mut n = 1;
    while parent.join(&name).exists() {
        n += 1;
        name = format!("{}_copy{}", world, n);
    }
    let staging = parent.join(format!(".{}.pzms-copy", name));
    let _ = fs::remove_dir_all(&staging);
    let copied = (|| -> Result<(), String> {
        for dir in &scan.empty_dirs {
            fs::create_dir_all(staging.join(dir)).map_err(|e| e.to_string())?;
        }
        for (rel, _) in &scan.files {
            let to = staging.join(rel);
            if let Some(p) = to.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            fs::copy(src.join(rel), &to).map_err(|e| format!("Cannot copy {}: {}", rel, e))?;
        }
        Ok(())
    })();
    if let Err(e) = copied {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    fs::rename(&staging, parent.join(&name)).map_err(|e| {
        let _ = fs::remove_dir_all(&staging);
        format!("Cannot name the copy: {}", e)
    })?;
    Ok(format!("{}/{}", mode, name))
}

/// The game build a save's world version (`map_ver.bin`) was written by. The game bumps the
/// version when the save format changes, not on every build, so a version names the first build
/// that wrote it and the ones after it until the next bump (pzdataspec's version_mapping, MIT,
/// checked against saves here: 42.21 writes 249 too).
pub fn build_of_world_version(version: i32) -> Option<&'static str> {
    Some(match version {
        143 => "40.43",
        195 => "41.78",
        241 => "42.13",
        243 => "42.14",
        244 => "42.15",
        245 => "42.16-42.18",
        247 => "42.19",
        249 => "42.20+",
        _ => return None,
    })
}

/// One save, for the Saves screen.
#[derive(Debug, Clone, Serialize)]
pub struct SaveSummary {
    /// `<mode>/<world>`.
    pub id: String,
    pub mode: String,
    pub world: String,
    pub bytes: u64,
    pub files: u64,
    /// Newest modification in the save (unix seconds): when it was last played or saved.
    pub modified: u64,
    pub mods: Vec<String>,
    pub backups: Vec<BackupEntry>,
    /// Unchanged since its newest backup.
    pub backed_up: bool,
    pub busy: bool,
    /// The save format's version (`map_ver.bin`), and the build that wrote it when known.
    pub world_version: Option<i32>,
    pub made_on: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavesOverview {
    pub saves: Vec<SaveSummary>,
    /// Where backups go.
    pub backup_dir: String,
    /// Everything the backups take.
    pub backups_bytes: u64,
    /// The disk holding the backups.
    pub free_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub keep_automatic: usize,
}

pub fn summary(user_zomboid_dir: &Path, save_id: &str) -> Option<SaveSummary> {
    let (mode, world) = split_id(save_id).ok()?;
    let dir = save_dir(user_zomboid_dir, save_id).ok()?;
    let scan = scan(&dir).ok()?;
    let backups = backups_of(user_zomboid_dir, save_id);
    let backed_up = backups.first().is_some_and(|b| b.verified && b.save_newest == scan.newest && b.save_bytes == scan.bytes);
    Some(SaveSummary {
        id: save_id.to_string(),
        mode,
        world,
        bytes: scan.bytes,
        files: scan.files.len() as u64,
        modified: scan.newest,
        mods: save_mods(&dir),
        backups,
        backed_up,
        busy: is_busy(&dir.to_string_lossy()),
        world_version: maps::read_map_ver(&dir).map(|(v, _)| v),
        made_on: maps::read_map_ver(&dir).and_then(|(v, _)| build_of_world_version(v)).map(str::to_string),
    })
}

/// A mod a save's list names that is not installed: the save will not load with it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MissingMod {
    /// The mod id, as the list names it (without its Workshop item).
    pub id: String,
    /// The Workshop item, when the list names one (`<item>/<id>`, the Build 42 form).
    pub workshop_id: Option<String>,
}

/// For each save whose own list (`mods.txt`) names mods that are not installed: those mods.
/// A server's save keeps no list (the server says which mods, and the game downloads them).
pub fn missing_mods(user_zomboid_dir: &Path, catalog: &crate::catalog::ModCatalog) -> BTreeMap<String, Vec<MissingMod>> {
    let mut out = BTreeMap::new();
    for id in list_saves(user_zomboid_dir) {
        let missing: Vec<MissingMod> = save_mods(&saves_dir(user_zomboid_dir).join(&id))
            .iter()
            .filter(|entry| catalog.resolve(entry).is_none())
            .map(|entry| {
                let entry = crate::load_order::mod_info::sanitize_mod_id(entry);
                match entry.split_once('/') {
                    Some((item, mod_id)) if !item.is_empty() && item.chars().all(|c| c.is_ascii_digit()) => MissingMod { id: mod_id.to_string(), workshop_id: Some(item.to_string()) },
                    _ => MissingMod { id: entry.to_string(), workshop_id: None },
                }
            })
            .collect();
        if !missing.is_empty() {
            out.insert(id, missing);
        }
    }
    out
}

pub fn overview(user_zomboid_dir: &Path) -> SavesOverview {
    let mut saves: Vec<SaveSummary> = list_saves(user_zomboid_dir).iter().filter_map(|id| summary(user_zomboid_dir, id)).collect();
    saves.sort_by(|a, b| b.modified.cmp(&a.modified));
    let root = backups_root(user_zomboid_dir);
    let backups_bytes = walkdir::WalkDir::new(&root).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum();
    let space = disk_space(&root);
    SavesOverview {
        saves,
        backup_dir: root.to_string_lossy().to_string(),
        backups_bytes,
        free_bytes: space.map(|s| s.0),
        total_bytes: space.map(|s| s.1),
        keep_automatic: KEEP_AUTOMATIC,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pzms-saves-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_save(z: &Path, id: &str) -> PathBuf {
        let dir = save_dir(z, id).unwrap();
        fs::create_dir_all(dir.join("map").join("0")).unwrap();
        fs::create_dir_all(dir.join("apop")).unwrap(); // stays empty
        fs::write(dir.join("map.bin"), vec![7u8; 5000]).unwrap();
        fs::write(dir.join("map").join("0").join("12.bin"), b"chunk").unwrap();
        fs::write(dir.join("mods.txt"), "VERSION = 1,\n\nmods\n{\n    mod = \\ModA,\n    mod = ModB,\n}\n").unwrap();
        dir
    }

    #[test]
    fn a_world_version_names_the_build_that_wrote_it() {
        assert_eq!(build_of_world_version(249), Some("42.20+"));
        assert_eq!(build_of_world_version(195), Some("41.78"));
        assert_eq!(build_of_world_version(245), Some("42.16-42.18"));
        assert_eq!(build_of_world_version(248), None);
    }

    #[test]
    fn a_backup_is_checked_and_a_save_unchanged_since_is_not_zipped_again() {
        let z = temp("backup");
        make_save(&z, "Sandbox/World 1");
        let first = backup_save(&z, "Sandbox/World 1", REASON_EXIT, false).unwrap();
        assert!(!first.unchanged && first.entry.verified);
        assert_eq!(first.entry.files, 3);
        assert_eq!(first.entry.mods, vec!["ModA".to_string(), "ModB".to_string()]);
        let again = backup_save(&z, "Sandbox/World 1", REASON_BEFORE_LOAD, false).unwrap();
        assert!(again.unchanged);
        assert_eq!(again.entry.file, first.entry.file);
        let listed = backups_of(&z, "Sandbox/World 1");
        assert_eq!(listed.len(), 1);
        assert!(summary(&z, "Sandbox/World 1").unwrap().backed_up);
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn mods_a_save_lists_and_nobody_installed_are_named() {
        let z = temp("missing");
        let save = make_save(&z, "Sandbox/W");
        fs::write(save.join("mods.txt"), "VERSION = 1,\nmods\n{\n    mod = 123/Gone,\n    mod = \\AlsoGone,\n}\n").unwrap();
        let server = make_save(&z, "Multiplayer/1.2.3.4_16261_f5e85281397522383aa0b8b39114e853");
        fs::remove_file(server.join("mods.txt")).unwrap();
        let catalog = crate::catalog::ModCatalog { game_version: None, mods: Vec::new() };
        let missing = missing_mods(&z, &catalog);
        assert_eq!(
            missing.get("Sandbox/W").cloned().unwrap_or_default(),
            vec![MissingMod { id: "Gone".into(), workshop_id: Some("123".into()) }, MissingMod { id: "AlsoGone".into(), workshop_id: None }]
        );
        assert_eq!(missing.len(), 1);
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn automatic_backups_are_pruned_and_kept_ones_are_not() {
        let z = temp("prune");
        let dir = make_save(&z, "Sandbox/W");
        let kept = backup_save(&z, "Sandbox/W", REASON_MANUAL, true).unwrap().entry.file;
        for i in 0..(KEEP_AUTOMATIC + 2) {
            fs::write(dir.join("map.bin"), vec![i as u8; 100 + i]).unwrap();
            backup_save(&z, "Sandbox/W", REASON_EXIT, false).unwrap();
        }
        let all = backups_of(&z, "Sandbox/W");
        assert_eq!(all.iter().filter(|b| !b.kept).count(), KEEP_AUTOMATIC);
        assert!(all.iter().any(|b| b.file == kept && b.kept));
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn a_damaged_backup_is_refused_and_a_restore_puts_the_save_back() {
        let z = temp("restore");
        let dir = make_save(&z, "Survival/W");
        let file = backup_save(&z, "Survival/W", REASON_MANUAL, true).unwrap().entry.file;
        // The save goes on: a file changes, one is added.
        fs::write(dir.join("map.bin"), b"changed").unwrap();
        fs::write(dir.join("new.bin"), b"new").unwrap();
        assert!(restore_backup(&z, "Survival/W", &file, true).is_err(), "never with the game running");
        let before = restore_backup(&z, "Survival/W", &file, false).unwrap().unwrap();
        assert_eq!(before.entry.reason, REASON_BEFORE_RESTORE);
        assert_eq!(fs::read(dir.join("map.bin")).unwrap(), vec![7u8; 5000]);
        assert!(!dir.join("new.bin").exists());
        assert!(dir.join("apop").is_dir(), "empty folders come back");
        assert!(!dir.parent().unwrap().join(".W.pzms-old").exists());

        // A zip that does not read back is refused before anything is touched.
        let zip = backup_dir_of(&z, "Survival/W").unwrap().join(&file);
        let mut bytes = fs::read(&zip).unwrap();
        let mid = bytes.len() / 3;
        for b in &mut bytes[mid..mid + 20] {
            *b ^= 0xFF;
        }
        fs::write(&zip, bytes).unwrap();
        assert!(restore_backup(&z, "Survival/W", &file, false).is_err());
        assert_eq!(fs::read(dir.join("map.bin")).unwrap(), vec![7u8; 5000]);
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn a_copy_is_whole_beside_the_save_and_never_takes_an_existing_name() {
        let z = temp("copy");
        let dir = make_save(&z, "Sandbox/W");
        assert_eq!(copy_save(&z, "Sandbox/W").unwrap(), "Sandbox/W_copy");
        assert_eq!(copy_save(&z, "Sandbox/W").unwrap(), "Sandbox/W_copy2");
        let copy = dir.parent().unwrap().join("W_copy");
        assert_eq!(fs::read(copy.join("map.bin")).unwrap(), vec![7u8; 5000]);
        assert!(copy.join("apop").is_dir());
        assert_eq!(save_mods(&copy), vec!["ModA".to_string(), "ModB".to_string()]);
        assert!(!dir.parent().unwrap().join(".W_copy.pzms-copy").exists());
        assert!(copy_save(&z, "Sandbox/Nope").is_err());
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn only_plain_save_ids_and_backup_names_are_taken() {
        let z = temp("ids");
        for bad in ["..", "Sandbox", "Sandbox/../x", "a/b/c", "/abs", "Sandbox/.hidden", ""] {
            assert!(split_id(bad).is_err(), "{}", bad);
        }
        make_save(&z, "Sandbox/W");
        for bad in ["../x.zip", "x.json", ".x.zip", "a/b.zip"] {
            assert!(backup_file(&z, "Sandbox/W", bad).is_err(), "{}", bad);
        }
        assert!(backup_save(&z, "Sandbox/W", "whatever", false).is_err());
        assert!(backup_save(&z, "Sandbox/None", REASON_EXIT, false).is_err());
        let _ = fs::remove_dir_all(&z);
    }

    /// A real save, copied first: `PZMS_REAL_ZOMBOID=<Zomboid folder> cargo test -- --ignored real_save`.
    /// The real folder is only read.
    #[test]
    #[ignore]
    fn a_real_save_backs_up_checks_and_restores() {
        let Ok(real) = std::env::var("PZMS_REAL_ZOMBOID") else { return };
        let real = PathBuf::from(real);
        let Some(id) = list_saves(&real).into_iter().find(|id| id.starts_with("Sandbox/")) else { return };
        let z = temp("real");
        let copy_to = save_dir(&z, &id).unwrap();
        for e in walkdir::WalkDir::new(save_dir(&real, &id).unwrap()).into_iter().flatten() {
            let rel = e.path().strip_prefix(save_dir(&real, &id).unwrap()).unwrap();
            let out = copy_to.join(rel);
            if e.file_type().is_dir() { fs::create_dir_all(&out).unwrap() } else { fs::copy(e.path(), &out).unwrap(); }
        }
        let started = std::time::Instant::now();
        let out = backup_save(&z, &id, REASON_EXIT, false).unwrap();
        println!("{}: {} files, {} -> {} in {:?}", id, out.entry.files, human_bytes(out.entry.save_bytes), human_bytes(out.entry.zip_bytes), started.elapsed());
        let before = scan(&copy_to).unwrap();
        fs::remove_file(copy_to.join("map.bin")).ok();
        restore_backup(&z, &id, &out.entry.file, false).unwrap();
        let after = scan(&copy_to).unwrap();
        assert_eq!(before.files.len(), after.files.len());
        assert_eq!(before.bytes, after.bytes);
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn a_backup_that_never_finished_is_not_listed_and_is_swept() {
        let z = temp("parts");
        make_save(&z, "Sandbox/W");
        let dir = backup_dir_of(&z, "Sandbox/W").unwrap();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("2026-01-01_00-00-00-exit.zip.part"), b"half").unwrap();
        fs::write(dir.join("2026-01-01_00-00-00-exit.zip"), b"no record").unwrap();
        assert!(backups_of(&z, "Sandbox/W").is_empty());
        backup_save(&z, "Sandbox/W", REASON_EXIT, false).unwrap();
        assert!(!dir.join("2026-01-01_00-00-00-exit.zip.part").exists());
        assert!(disk_space(&dir).is_some_and(|(free, total)| free > 0 && total >= free));
        let _ = fs::remove_dir_all(&z);
    }
}
