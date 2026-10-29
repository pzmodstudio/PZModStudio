use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::load_order::mod_info::sanitize_mod_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModListData {
    pub active_mods: Vec<String>,
    pub raw_ini_content: String,
}

/// Helper function to reliably resolve Zomboid/mods/default.txt path regardless of ini_path state
fn resolve_default_txt_path(ini_path: &str) -> PathBuf {
    if !ini_path.is_empty() {
        let p = Path::new(ini_path);
        if let Some(parent) = p.parent() {
            if parent.file_name().map_or(false, |n| n == "mods") {
                return parent.join("default.txt");
            } else if parent.file_name().map_or(false, |n| n == "Lua") {
                if let Some(pz_dir) = parent.parent() {
                    return pz_dir.join("mods").join("default.txt");
                }
            } else if parent.join("mods").is_dir() {
                // e.g. <Zomboid>/ModListData.ini
                return parent.join("mods").join("default.txt");
            }
        }
    }
    // Fallback to standard user home Zomboid/mods/default.txt
    let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"));
    home.join("Zomboid").join("mods").join("default.txt")
}

fn parse_ini_file(path: &Path) -> Vec<String> {
    let mut active = Vec::new();
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("activeMods=") || trimmed.starts_with("Mods=") || trimmed.starts_with("mods=") {
                let parts: Vec<&str> = trimmed.split('=').collect();
                if parts.len() == 2 {
                    active = parts[1]
                        .split(';')
                        .map(|s| sanitize_mod_id(s.trim()))
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }
    }
    active
}

pub(crate) fn parse_default_txt(path: &Path) -> Vec<String> {
    read_patiently(path).map(|text| parse_default_txt_text(&text)).unwrap_or_default()
}

fn parse_default_txt_text(content: &str) -> Vec<String> {
    let mut active = Vec::new();
    {
        let mut in_mods_block = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "mods" || trimmed.starts_with("mods") {
                in_mods_block = true;
                continue;
            }
            if in_mods_block && (trimmed == "maps" || trimmed.starts_with("maps")) {
                break;
            }

            if in_mods_block && trimmed.starts_with("mod") {
                let raw_id = trimmed[3..]
                    .trim()
                    .trim_start_matches('=')
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim_end_matches(',')
                    .trim();

                let clean_id = sanitize_mod_id(raw_id);
                if !clean_id.is_empty() && clean_id != "{" && clean_id != "}" {
                    active.push(clean_id);
                }
            }
        }
    }
    active
}

fn parse_plain_list_file(path: &Path) -> Vec<String> {
    let mut active = Vec::new();
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let clean = sanitize_mod_id(line);
            if !clean.is_empty() {
                active.push(clean);
            }
        }
    }
    active
}

/// Reads the active mods from Zomboid/mods/default.txt; when that file does not exist, falls back
/// to the most recently modified legacy list (ModListData.ini, mods.txt, Lua/loadorder.ini).
pub fn read_mod_list_ini(ini_path: &str) -> Result<ModListData, String> {
    let default_txt_path = resolve_default_txt_path(ini_path);
    let ini_p = Path::new(ini_path);

    let mut candidate_paths: Vec<(PathBuf, std::time::SystemTime, &str)> = Vec::new();

    // 1. ModListData.ini
    if !ini_path.is_empty() && ini_p.exists() {
        if let Ok(meta) = fs::metadata(ini_p) {
            if let Ok(mtime) = meta.modified() {
                candidate_paths.push((ini_p.to_path_buf(), mtime, "ini"));
            }
        }
    }

    // 2. mods/default.txt
    if default_txt_path.exists() {
        if let Ok(meta) = fs::metadata(&default_txt_path) {
            if let Ok(mtime) = meta.modified() {
                candidate_paths.push((default_txt_path.clone(), mtime, "default_txt"));
            }
        }
    }

    // 3. mods.txt
    if let Some(mods_dir) = default_txt_path.parent() {
        if let Some(z_dir) = mods_dir.parent() {
            let mods_txt_path = z_dir.join("mods.txt");
            if mods_txt_path.exists() {
                if let Ok(meta) = fs::metadata(&mods_txt_path) {
                    if let Ok(mtime) = meta.modified() {
                        candidate_paths.push((mods_txt_path, mtime, "plain"));
                    }
                }
            }

            // 4. loadorder.ini
            let loadorder_ini = z_dir.join("Lua").join("loadorder.ini");
            if loadorder_ini.exists() {
                if let Ok(meta) = fs::metadata(&loadorder_ini) {
                    if let Ok(mtime) = meta.modified() {
                        candidate_paths.push((loadorder_ini, mtime, "ini"));
                    }
                }
            }
        }
    }

    // mods/default.txt is the only list the game reads, so when it exists it is the truth,
    // even when it is empty (no mods active). The other files are legacy fallbacks only.
    if default_txt_path.exists() {
        return Ok(ModListData {
            active_mods: parse_default_txt(&default_txt_path),
            raw_ini_content: String::new(),
        });
    }

    // Sort candidates by modification time DESCENDING (newest file modified on disk wins!)
    candidate_paths.sort_by(|a, b| b.1.cmp(&a.1));

    let mut active_mods = Vec::new();

    for (path, _mtime, file_type) in candidate_paths {
        let parsed = match file_type {
            "ini" => parse_ini_file(&path),
            "default_txt" => parse_default_txt(&path),
            "plain" => parse_plain_list_file(&path),
            _ => Vec::new(),
        };

        if !parsed.is_empty() {
            active_mods = parsed;
            break;
        }
    }

    Ok(ModListData {
        active_mods,
        raw_ini_content: String::new(),
    })
}

/// Helper to detect custom map folders for active mods across workshop and local mod paths
pub fn detect_active_maps(active_mods: &[String], user_zomboid_str: &str) -> Vec<String> {
    let mut detected_maps = Vec::new();
    let mut seen_maps = HashSet::new();

    let paths = crate::vfs::auto_detect_paths();
    let mut search_dirs = Vec::new();

    if !paths.workshop_dir.is_empty() {
        search_dirs.push(PathBuf::from(&paths.workshop_dir));
    }
    if !paths.pz_install_dir.is_empty() {
        search_dirs.push(PathBuf::from(&paths.pz_install_dir).join("mods"));
    }
    for u_dir in crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_str) {
        search_dirs.push(u_dir.join("mods"));
    }

    let clean_active_ids: HashSet<String> = active_mods
        .iter()
        .map(|s| sanitize_mod_id(s).to_lowercase())
        .collect();

    // Map mod ID (lowercase) -> list of map folder names
    let mut mod_to_maps: HashMap<String, Vec<String>> = HashMap::new();

    for root in &search_dirs {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root).max_depth(6).into_iter().filter_map(|e| e.ok()) {
            if entry.file_name() == "map.info" {
                if let Some(map_dir) = entry.path().parent() {
                    let map_folder_name = map_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if map_folder_name.is_empty() {
                        continue;
                    }

                    // Traverse up to find the enclosing mod.info
                    let mut current = map_dir.parent();
                    let mut found_mod_id = None;
                    for _ in 0..5 {
                        if let Some(p) = current {
                            let mod_info = p.join("mod.info");
                            if mod_info.exists() {
                                if let Some(manifest) = crate::load_order::mod_info::parse_mod_info(&mod_info) {
                                    let clean_id = sanitize_mod_id(&manifest.id).to_lowercase();
                                    if clean_active_ids.contains(&clean_id) {
                                        found_mod_id = Some(clean_id);
                                        break;
                                    }
                                }
                            }
                            current = p.parent();
                        } else {
                            break;
                        }
                    }

                    if let Some(mod_id) = found_mod_id {
                        let list = mod_to_maps.entry(mod_id).or_default();
                        if !list.contains(&map_folder_name) {
                            list.push(map_folder_name);
                        }
                    }
                }
            }
        }
    }

    // Preserve active_mods load order when ordering map folders
    for mod_id in active_mods {
        let clean = sanitize_mod_id(mod_id).to_lowercase();
        if let Some(maps) = mod_to_maps.get(&clean) {
            for m in maps {
                if seen_maps.insert(m.clone()) {
                    detected_maps.push(m.clone());
                }
            }
        }
    }

    detected_maps
}

/// Zomboid folder that owns the active mod list for `ini_path` (parent of `mods/default.txt`).
fn zomboid_dir_for(default_txt_path: &Path) -> Option<PathBuf> {
    default_txt_path.parent().and_then(|m| m.parent()).map(Path::to_path_buf)
}

/// Builds `mods/default.txt` the way the game writes it (ActiveModsFile, VERSION = 1).
/// The game splits values on every comma and never strips quotes, so values are written bare
/// and map names containing a comma (they cannot round-trip) are skipped.
pub fn build_default_txt(active_mods: &[String], maps: &[String]) -> String {
    // Same line ending as the game (Java's System.lineSeparator()).
    let eol = if cfg!(windows) { "\r\n" } else { "\n" };
    let mut out = format!("VERSION = 1,{eol}{eol}mods{eol}{{{eol}");
    for mod_id in active_mods {
        let clean_id = sanitize_mod_id(mod_id);
        if !clean_id.is_empty() && !clean_id.contains(',') {
            out.push_str(&format!("    mod = {},{eol}", clean_id));
        }
    }
    out.push_str(&format!("}}{eol}{eol}maps{eol}{{{eol}"));
    for map_name in maps {
        let clean = map_name.trim();
        if !clean.is_empty() && !clean.contains(',') {
            out.push_str(&format!("    map = {},{eol}", clean));
        }
    }
    out.push_str(&format!("}}{eol}"));
    out
}

/// Map entries currently in a default.txt `maps { }` block, in order.
fn read_existing_maps(path: &Path) -> Vec<String> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut maps = Vec::new();
    let mut in_maps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("maps") {
            in_maps = true;
            continue;
        }
        if in_maps {
            if trimmed.starts_with('}') {
                break;
            }
            if let Some(rest) = trimmed.strip_prefix("map") {
                let value = rest.trim().trim_start_matches('=').trim().trim_end_matches(',').trim();
                if !value.is_empty() {
                    maps.push(value.to_string());
                }
            }
        }
    }
    maps
}

/// Keeps the newest `keep` files in `dir` whose names start with `prefix`.
fn prune_backups(dir: &Path, prefix: &str, keep: usize) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut backups: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(prefix)))
        .collect();
    // Names embed a sortable timestamp, so name order is age order.
    backups.sort();
    let excess = backups.len().saturating_sub(keep);
    for old in backups.into_iter().take(excess) {
        let _ = fs::remove_file(old);
    }
}

/// Whether an error is Windows saying someone else has the file open right now (the game while
/// it writes, an antivirus scanning it, an editor): worth waiting a moment for, not a failure.
fn is_held(e: &std::io::Error) -> bool {
    matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33)) || e.kind() == std::io::ErrorKind::PermissionDenied
}

/// Tries `op` again for about two seconds while the file is held by someone else.
fn while_held<T>(mut op: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    let mut tries = 0;
    loop {
        match op() {
            Err(e) if is_held(&e) && tries < 20 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            other => return other,
        }
    }
}

/// `fs::read_to_string`, waiting a moment while the file is held by someone else.
pub fn read_patiently(path: &Path) -> std::io::Result<String> {
    while_held(|| fs::read_to_string(path))
}

/// `fs::write` through a temp file + rename, for the Studio's own state and for files the game
/// reads: a crash, a full disk or a power cut mid-write leaves the previous file, never a
/// truncated one that reads as "no profile". The temp file is flushed to the disk before it
/// takes the name, and the rename waits a moment while the game or an antivirus holds the file.
pub fn atomic_write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> std::io::Result<()> {
    use std::io::Write;
    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = path.as_ref();
    let n = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{}.pzms-tmp-{}-{}", name, std::process::id(), n));
    let written = (|| {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(content.as_ref())?;
        file.sync_all()
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    while_held(|| fs::rename(&tmp, path)).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// To call before a file of the Studio's is written from what was read of it. A file that is
/// there but cannot be read (held, or no permission) is refused: what was read of it was
/// nothing, and writing would replace it with that. One that reads but does not parse is kept
/// beside it as `<name>.unreadable-<stamp>.json` first, so the write is never all that is left.
pub fn guard_overwrite<T: serde::de::DeserializeOwned>(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let text = read_patiently(path).map_err(|e| format!("Cannot read {} ({}), so it is not written over.", path.display(), e))?;
    if text.trim().is_empty() || serde_json::from_str::<T>(&text).is_ok() {
        return Ok(());
    }
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let copy = path.with_extension(format!("unreadable-{}.json", stamp));
    fs::copy(path, &copy).map_err(|e| format!("{} does not parse and could not be kept aside ({}), so it is not written over.", path.display(), e))?;
    Ok(())
}

/// The mods `default.txt` switches on, or None when that cannot be told: the file could not be
/// read, or it is cut short (the game writes it in place, so a read can land mid-write). Never
/// an empty list for a file that was not really read.
pub fn try_parse_default_txt(path: &Path) -> Option<Vec<String>> {
    let text = read_patiently(path).ok()?;
    let mods_at = text.lines().position(|l| l.trim().starts_with("mods"))?;
    // The mods block has to be closed for the list to be whole.
    if !text.lines().skip(mods_at + 1).any(|l| l.trim().starts_with('}')) {
        return None;
    }
    Some(parse_default_txt_text(&text))
}

/// Replaces `path` with `content`: skips the write when nothing changed, copies the previous
/// version into `backup_dir` first, and writes through a temp file + rename so a crash can
/// never leave a truncated file behind.
pub fn write_file_with_backup(path: &Path, content: &str, backup_dir: &Path) -> Result<bool, String> {
    if let Ok(existing) = fs::read_to_string(path) {
        if existing == content {
            return Ok(false);
        }
        fs::create_dir_all(backup_dir)
            .map_err(|e| format!("Cannot create backup folder {}: {}", backup_dir.display(), e))?;
        let stem = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".into());
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S%.3f");
        let backup_path = backup_dir.join(format!("{}.{}.bak", stem, stamp));
        fs::copy(path, &backup_path)
            .map_err(|e| format!("Cannot back up {}: {}", path.display(), e))?;
        prune_backups(backup_dir, &format!("{}.", stem), 20);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create {}: {}", parent.display(), e))?;
    }
    // Unique per writer so two writes can never share (and truncate) the same temp file.
    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp_path = path.with_extension(format!("pzms-tmp-{}-{}", std::process::id(), n));
    fs::write(&tmp_path, content).map_err(|e| format!("Cannot write {}: {}", tmp_path.display(), e))?;
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Cannot replace {}: {}", path.display(), e)
    })?;
    Ok(true)
}

/// One earlier version of the mod list, as it sits in `Zomboid/PZModStudio_Backups`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModListVersion {
    /// The backup file itself, which is what `restore_mod_list_version` is given.
    pub file: String,
    /// When it was replaced, read from the name the writer gave it: "2026-10-16 22:01:22".
    pub saved_at: String,
    pub active_count: usize,
    /// Mods this version had switched on that the current list does not.
    pub added: Vec<String>,
    /// Mods the current list has switched on that this version did not.
    pub removed: Vec<String>,
}

/// "default.txt.20261016-220122.349.bak" -> "2026-10-16 22:01:22".
fn stamp_from_backup_name(name: &str) -> Option<String> {
    let rest = name.strip_prefix("default.txt.")?.strip_suffix(".bak")?;
    let (date, rest) = rest.split_once('-')?;
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let time: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if time.len() < 6 {
        return None;
    }
    Some(format!(
        "{}-{}-{} {}:{}:{}",
        &date[0..4],
        &date[4..6],
        &date[6..8],
        &time[0..2],
        &time[2..4],
        &time[4..6],
    ))
}

/// Every earlier version of the mod list the Studio kept, newest first, each next to what it
/// would change about the list that is active now.
///
/// These are not a new feature's files: every write of `default.txt` has always copied the
/// previous one aside (twenty are kept). Until now nothing could read them back, so a mod list
/// taken apart by hand — or by a bisect, a profile switch, an import — was gone.
pub fn list_mod_list_history(ini_path: &str, user_zomboid_dir: &str) -> Vec<ModListVersion> {
    let default_txt_path = resolve_default_txt_path(ini_path);
    let current: HashSet<String> = parse_default_txt(&default_txt_path).into_iter().collect();
    let backups = Path::new(user_zomboid_dir).join("PZModStudio_Backups");

    let mut versions: Vec<(String, ModListVersion)> = Vec::new();
    let Ok(entries) = fs::read_dir(&backups) else { return Vec::new() };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(saved_at) = stamp_from_backup_name(&name) else { continue };
        let active = parse_default_txt(&entry.path());
        if active.is_empty() && !entry.path().is_file() {
            continue;
        }
        let was: HashSet<String> = active.iter().cloned().collect();
        versions.push((
            name,
            ModListVersion {
                file: entry.path().to_string_lossy().to_string(),
                saved_at,
                active_count: active.len(),
                added: active.iter().filter(|id| !current.contains(*id)).cloned().collect(),
                removed: current.iter().filter(|id| !was.contains(*id)).cloned().collect(),
            },
        ));
    }
    // The names sort by time, which is what the writer's stamp is for.
    versions.sort_by(|a, b| b.0.cmp(&a.0));
    versions.into_iter().map(|(_, v)| v).collect()
}

/// Puts an earlier version back, and returns the mods it switches on.
///
/// The file has to be one of the Studio's own backups in this Zomboid folder — nothing else is
/// read, whatever path is passed — and it is written through the same backed-up, atomic writer,
/// so going back is itself a version you can go back from.
pub fn restore_mod_list_version(
    ini_path: &str,
    user_zomboid_dir: &str,
    backup_file: &str,
) -> Result<Vec<String>, String> {
    let backups = Path::new(user_zomboid_dir).join("PZModStudio_Backups");
    let file = Path::new(backup_file);
    let inside = file
        .canonicalize()
        .ok()
        .zip(backups.canonicalize().ok())
        .map(|(f, dir)| f.starts_with(&dir))
        .unwrap_or(false);
    let named_right = file
        .file_name()
        .and_then(|n| stamp_from_backup_name(&n.to_string_lossy()))
        .is_some();
    if !inside || !named_right {
        return Err(format!(
            "{} is not one of the mod list versions the Studio saved.",
            backup_file
        ));
    }

    let active = parse_default_txt(file);
    if active.is_empty() {
        return Err("That version has no mods switched on; it would empty your list.".to_string());
    }
    write_mod_list_ini(ini_path, &active)?;
    Ok(active)
}

/// Writes the active mod list to `Zomboid/mods/default.txt`, the only global mod list file the
/// game reads (ZomboidFileSystem / ActiveModsFile). The previous file is backed up to
/// `Zomboid/PZModStudio_Backups/`. Save folders are not touched: the game rewrites each save's
/// own `mods.txt` itself whenever that save is loaded.
///
/// The `maps` block is kept as it is: the game only uses it to order conflicting map mods and
/// maintains it from its own mod selector, so it does not need a Workshop scan on every save.
pub fn write_mod_list_ini(ini_path: &str, active_mods: &[String]) -> Result<(), String> {
    let default_txt_path = resolve_default_txt_path(ini_path);
    let zomboid_dir = zomboid_dir_for(&default_txt_path)
        .ok_or_else(|| format!("Cannot resolve the Zomboid folder for {}", default_txt_path.display()))?;
    if !zomboid_dir.is_dir() {
        return Err(format!("Zomboid folder not found: {}", zomboid_dir.display()));
    }

    // Saves from the UI thread and background scans can overlap; one writer at a time.
    let _guard = MOD_LIST_WRITE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let maps = read_existing_maps(&default_txt_path);
    let content = build_default_txt(active_mods, &maps);
    write_file_with_backup(&default_txt_path, &content, &zomboid_dir.join("PZModStudio_Backups"))?;
    *LAST_WRITTEN.lock().unwrap_or_else(|p| p.into_inner()) = Some(content);
    Ok(())
}

static MOD_LIST_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static LAST_WRITTEN: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Whether `default.txt` holds exactly what this Studio last wrote into it, so that a watcher
/// can tell its own writes from the game's.
pub fn is_own_write(default_txt: &Path) -> bool {
    let Ok(now) = fs::read_to_string(default_txt) else { return false };
    LAST_WRITTEN.lock().unwrap_or_else(|p| p.into_inner()).as_deref() == Some(now.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway Zomboid folder under the system temp dir (never the user's real one).
    fn temp_zomboid(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pzms-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("mods")).unwrap();
        dir
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn default_txt_round_trips_and_keeps_workshop_prefixed_ids() {
        let mods = ids(&["ModA", "\\ModB", "2858201402/dailystatisticsandgains"]);
        let text = build_default_txt(&mods, &ids(&["MyMap", "Bad, Map"]));
        assert!(text.starts_with("VERSION = 1,"));
        assert!(!text.contains('"'), "the game does not strip quotes");
        assert!(!text.contains("Bad"), "comma map names cannot round-trip");

        let dir = temp_zomboid("roundtrip");
        let path = dir.join("mods").join("default.txt");
        fs::write(&path, &text).unwrap();
        assert_eq!(parse_default_txt(&path), ids(&["ModA", "ModB", "2858201402/dailystatisticsandgains"]));
        assert_eq!(read_existing_maps(&path), ids(&["MyMap"]));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_default_txt_cut_short_or_unreadable_is_not_a_list() {
        let dir = temp_zomboid("trydefault");
        let path = dir.join("default.txt");
        assert_eq!(try_parse_default_txt(&path), None, "missing");
        fs::write(&path, "VERSION = 1,\n\nmods\n{\n    mod = A,\n    mod = B,\n").unwrap();
        assert_eq!(try_parse_default_txt(&path), None, "the game was still writing it");
        fs::write(&path, "VERSION = 1,\n\nmods\n{\n    mod = A,\n    mod = B,\n}\n\nmaps\n{\n}\n").unwrap();
        assert_eq!(try_parse_default_txt(&path), Some(ids(&["A", "B"])));
        fs::write(&path, "VERSION = 1,\n\nmods\n{\n}\n").unwrap();
        assert_eq!(try_parse_default_txt(&path), Some(vec![]), "really nothing on");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_does_not_parse_is_kept_before_it_is_written_over() {
        let dir = temp_zomboid("guard");
        let path = dir.join("state.json");
        assert!(guard_overwrite::<Vec<String>>(&path).is_ok(), "nothing there yet");
        fs::write(&path, "[\"a\", \"b\"]").unwrap();
        assert!(guard_overwrite::<Vec<String>>(&path).is_ok());
        fs::write(&path, "{ half a file").unwrap();
        assert!(guard_overwrite::<Vec<String>>(&path).is_ok());
        let kept: Vec<_> = fs::read_dir(&dir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains("unreadable")).collect();
        assert_eq!(kept.len(), 1);
        assert_eq!(fs::read_to_string(kept[0].path()).unwrap(), "{ half a file");
        let _ = fs::remove_dir_all(&dir);
    }

    /// Windows: a file another program holds open without sharing (the game, an antivirus).
    #[cfg(windows)]
    #[test]
    fn writes_wait_for_a_held_file_and_reads_never_turn_it_into_nothing() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = temp_zomboid("held");
        let path = dir.join("held.json");
        fs::write(&path, "[\"before\"]").unwrap();

        // Held for a moment: the write waits and goes through.
        let hold = fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(400));
            drop(hold);
        });
        atomic_write(&path, "[\"after\"]").unwrap();
        release.join().unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "[\"after\"]");

        // Held for longer than the wait: never written over, never read as an empty list.
        let _hold = fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        assert!(guard_overwrite::<Vec<String>>(&path).is_err());
        assert_eq!(try_parse_default_txt(&path), None);
        drop(_hold);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_with_backup_keeps_previous_version_and_skips_no_ops() {
        let dir = temp_zomboid("backup");
        let path = dir.join("mods").join("default.txt");
        let backups = dir.join("PZModStudio_Backups");
        fs::write(&path, "old").unwrap();

        assert!(write_file_with_backup(&path, "new", &backups).unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        let saved: Vec<_> = fs::read_dir(&backups).unwrap().filter_map(|e| e.ok()).collect();
        assert_eq!(saved.len(), 1);
        assert_eq!(fs::read_to_string(saved[0].path()).unwrap(), "old");
        assert!(!path.with_extension("pzms-tmp").exists());

        // Same content again: no write, no extra backup.
        assert!(!write_file_with_backup(&path, "new", &backups).unwrap());
        assert_eq!(fs::read_dir(&backups).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_backup_name_gives_the_time_it_was_replaced() {
        assert_eq!(
            stamp_from_backup_name("default.txt.20261016-220122.349.bak").as_deref(),
            Some("2026-10-16 22:01:22")
        );
        // Anything else is not one of ours.
        assert!(stamp_from_backup_name("default.txt.bak").is_none());
        assert!(stamp_from_backup_name("servertest.ini.20261016-220122.349.bak").is_none());
        assert!(stamp_from_backup_name("default.txt.notadate-220122.bak").is_none());
    }

    #[test]
    fn the_history_says_what_each_version_would_change_and_restores_one() {
        let dir = std::env::temp_dir().join(format!("pzms-history-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("mods")).unwrap();
        fs::create_dir_all(dir.join("PZModStudio_Backups")).unwrap();
        let zomboid = dir.to_string_lossy().to_string();
        let ini = dir.join("mods").join("default.txt").to_string_lossy().to_string();

        // Now: A and B are on.
        fs::write(&ini, build_default_txt(&["A".into(), "B".into()], &[])).unwrap();
        // Then: A and C were on.
        fs::write(
            dir.join("PZModStudio_Backups").join("default.txt.20261016-100000.000.bak"),
            build_default_txt(&["A".into(), "C".into()], &[]),
        )
        .unwrap();
        // Earlier still: only A.
        fs::write(
            dir.join("PZModStudio_Backups").join("default.txt.20261015-090000.000.bak"),
            build_default_txt(&["A".into()], &[]),
        )
        .unwrap();
        // Something else's backup, which is none of this list's business.
        fs::write(
            dir.join("PZModStudio_Backups").join("servertest.ini.20261016-110000.000.bak"),
            "Mods=X",
        )
        .unwrap();

        let history = list_mod_list_history(&ini, &zomboid);
        assert_eq!(history.len(), 2, "{:?}", history);
        // Newest first.
        assert_eq!(history[0].saved_at, "2026-10-16 10:00:00");
        assert_eq!(history[0].active_count, 2);
        assert_eq!(history[0].added, vec!["C".to_string()]);
        assert_eq!(history[0].removed, vec!["B".to_string()]);
        assert_eq!(history[1].added, Vec::<String>::new());

        // Going back writes the list and keeps what was there as a version of its own.
        let restored = restore_mod_list_version(&ini, &zomboid, &history[0].file).unwrap();
        assert_eq!(restored, vec!["A", "C"]);
        assert_eq!(parse_default_txt(Path::new(&ini)), vec!["A", "C"]);
        assert_eq!(list_mod_list_history(&ini, &zomboid).len(), 3);

        // Nothing outside the backups folder is read, whatever the path says.
        let elsewhere = dir.join("default.txt.20261016-120000.000.bak");
        fs::write(&elsewhere, build_default_txt(&["Z".into()], &[])).unwrap();
        let err = restore_mod_list_version(&ini, &zomboid, &elsewhere.to_string_lossy()).unwrap_err();
        assert!(err.contains("not one of the mod list versions"), "{}", err);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_mod_list_only_touches_default_txt() {
        let dir = temp_zomboid("writer");
        let save = dir.join("Saves").join("Sandbox").join("MySave");
        fs::create_dir_all(&save).unwrap();
        fs::write(save.join("mods.txt"), "save's own list").unwrap();
        let ini_path = dir.join("mods").join("default.txt");
        fs::write(&ini_path, "VERSION = 1,\nmods\n{\n    mod = Old,\n}\nmaps\n{\n}\n").unwrap();

        write_mod_list_ini(&ini_path.to_string_lossy(), &ids(&["A", "B"])).unwrap();

        assert_eq!(read_mod_list_ini(&ini_path.to_string_lossy()).unwrap().active_mods, ids(&["A", "B"]));
        assert_eq!(fs::read_to_string(save.join("mods.txt")).unwrap(), "save's own list");
        for never_written in ["mods.txt", "Lua"] {
            assert!(!dir.join(never_written).exists(), "{} was created", never_written);
        }
        assert_eq!(fs::read_dir(dir.join("PZModStudio_Backups")).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_default_txt_wins_over_stale_legacy_lists() {
        let dir = temp_zomboid("reader");
        let ini_path = dir.join("mods").join("default.txt");
        fs::write(&ini_path, build_default_txt(&[], &[])).unwrap();
        fs::write(dir.join("mods.txt"), "Stale\nList").unwrap();
        assert!(read_mod_list_ini(&ini_path.to_string_lossy()).unwrap().active_mods.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod live_history_tests {
    use super::*;

    /// Reads the real backups on this machine. Read-only: nothing is restored.
    /// PZMS_ZOMBOID_DIR=... cargo test the_real_mod_list_history -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_real_mod_list_history() {
        let Ok(zomboid) = std::env::var("PZMS_ZOMBOID_DIR") else { return };
        let ini = format!("{}/mods/default.txt", zomboid);
        let history = list_mod_list_history(&ini, &zomboid);
        println!("{} earlier version(s) of the mod list:", history.len());
        for v in &history {
            println!(
                "  {}  {} mods   +{} / -{}{}",
                v.saved_at,
                v.active_count,
                v.added.len(),
                v.removed.len(),
                if v.added.is_empty() && v.removed.is_empty() { "   (same as now)" } else { "" }
            );
            if !v.added.is_empty() {
                println!("      back on: {}", v.added.join(", "));
            }
            if !v.removed.is_empty() {
                println!("      would switch off: {}", v.removed.join(", "));
            }
        }
        assert!(!history.is_empty(), "this machine has saved lists; the history should find them");
    }
}
