//! The version store: earlier versions of mods, kept on this PC so a Workshop update that breaks
//! something can be undone.
//!
//! Steam keeps no old versions of a Workshop item and has no public way to download one; it writes
//! an update over the item's folder. So the only way back is a copy made before Steam replaces the
//! files. The Studio keeps one of the version installed now of every Workshop item a profile uses
//! (the "current" version); when Steam later replaces it, that copy is the previous version, and
//! the new files become the next current one.
//!
//! Each file is stored once by its content (SHA-256), deflated unless it is already compressed
//! (textures, sounds, archives), under `<store>/blobs/`; the store is `Zomboid/PZModStudio_Versions`
//! unless the player chose another folder (`place`). A version is a list
//! of path -> hash under `items/<item>/<stamp>-<tree>.json` (the head: dates, mods, sizes) and
//! `.files.json` beside it (the files). Two versions of a mod share every file that did not
//! change, so a version after an update costs only what the update changed.
//!
//! The store has a cap; past it, versions are dropped in this order: older versions behind a
//! newer earlier one, current copies of mods not in the active profile (biggest first), the last
//! earlier version of each mod (oldest first), current copies of the active profile's mods. A
//! version the player kept by hand, or one in use as a local copy, is never dropped. Whatever the
//! cap says, a round never leaves the store's drive with less than 15% of its size free
//! (`disk_guard`).
//! Nothing here writes to the Workshop folder: it is only read.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const STORE_DIR: &str = "PZModStudio_Versions";
const SETTINGS_FILE: &str = "settings.json";
const STATUS_FILE: &str = "status.json";
pub(crate) const LOCK_FILE: &str = "lock";
/// 10 GB: on the PC this was written on, 5 GB held the current version of 639 of the 664 Workshop
/// items its profiles use (27 GB raw on Steam's side); twice that leaves room for every one of
/// them and for the earlier versions updates leave behind. The drive guard keeps it from filling
/// a small drive.
pub const DEFAULT_CAP_BYTES: u64 = 10 << 30;
/// The default before 2026-11-18: a store with it and no cap chosen by the player moves to the new one.
pub const OLD_DEFAULT_CAP_BYTES: u64 = 5 << 30;
/// The share of the drive a round always leaves free, in percent.
pub const DRIVE_RESERVE_PERCENT: u64 = 15;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreSettings {
    /// Keep versions by itself in the background.
    pub enabled: bool,
    pub cap_bytes: u64,
    /// The player chose this cap (Settings, or an agent): a new default does not change it.
    #[serde(default)]
    pub cap_by_player: bool,
}

impl Default for StoreSettings {
    fn default() -> Self {
        StoreSettings { enabled: true, cap_bytes: DEFAULT_CAP_BYTES, cap_by_player: false }
    }
}

impl StoreSettings {
    /// Settings written before `cap_by_player` existed: the old default is the default's (now
    /// 10 GB); any other value was the player's choice and stays.
    pub fn migrated(mut self) -> StoreSettings {
        if !self.cap_by_player {
            if self.cap_bytes == OLD_DEFAULT_CAP_BYTES || self.cap_bytes == DEFAULT_CAP_BYTES {
                self.cap_bytes = DEFAULT_CAP_BYTES;
            } else {
                self.cap_by_player = true;
            }
        }
        self
    }
}

/// The bytes a drive of `total` bytes always keeps free.
pub fn drive_reserve(total: u64) -> u64 {
    (total as u128 * DRIVE_RESERVE_PERCENT as u128 / 100) as u64
}

/// The bytes the store may take on a drive: the cap, but never so much that the drive is left with
/// less than `DRIVE_RESERVE_PERCENT` of its size free, and never below what it holds that the cap
/// cannot drop (`protected`: kept by hand or in use). Returns the cap that applies and whether the
/// drive is what limits it.
pub fn disk_guard(cap: u64, store_bytes: u64, free: u64, total: u64, protected: u64) -> (u64, bool) {
    let reserve = drive_reserve(total);
    let room = store_bytes.saturating_add(free.saturating_sub(reserve));
    if room < cap {
        (room.max(protected), true)
    } else {
        (cap, false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    pub hash: String,
    pub size: u64,
    /// Modified time (unix milliseconds) when it was kept: an unchanged size and time on the next
    /// round means the same file, so it is not read again.
    #[serde(default)]
    pub mtime: u64,
}

/// One mod inside a kept item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionMod {
    pub id: String,
    pub name: String,
    /// Its folder's name: what goes under Zomboid/mods when it is put back.
    pub folder: String,
    /// Its folder inside the item, "/"-separated ("mods/Foo"); "" when the item is the mod folder.
    pub rel: String,
    /// `modversion=` from its mod.info.
    pub version: Option<String>,
    /// Bytes of its files (raw).
    #[serde(default)]
    pub bytes: u64,
}

/// The head of one kept version (its files are in the `.files.json` beside it).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionHead {
    /// The Workshop id, or "local-<folder>" for a mod kept from Zomboid/mods.
    pub item: String,
    pub workshop_id: Option<String>,
    /// "2026-11-18 14:05".
    pub kept_at: String,
    pub kept_unix: i64,
    /// Kept with "Keep this version": never dropped by the cap.
    pub by_player: bool,
    /// What Steam's record said about this version: its manifest (or "t<publish time>").
    pub steam_ident: Option<String>,
    /// When the author published this version (unix seconds), from Steam's record.
    pub published: Option<u64>,
    /// Hash over every path and file hash: two versions with the same tree are the same.
    pub tree: String,
    pub mods: Vec<VersionMod>,
    pub files: usize,
    /// Raw bytes of every file.
    pub bytes: u64,
    /// The folder it was kept from.
    pub source_dir: String,
}

/// A head with where it lives.
#[derive(Debug, Clone)]
pub struct StoredVersion {
    pub head: VersionHead,
    pub head_path: PathBuf,
}

impl StoredVersion {
    fn files_path(&self) -> PathBuf {
        files_path_of(&self.head_path)
    }
}

fn files_path_of(head: &Path) -> PathBuf {
    head.with_extension("files.json")
}

/// The store under a Zomboid folder.
pub struct Store {
    pub root: PathBuf,
}

/// Held while the store is written; removed when dropped.
pub struct StoreLock(PathBuf);
impl Drop for StoreLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn now_unix() -> i64 {
    chrono::Local::now().timestamp()
}

/// Modified time in milliseconds.
fn mtime_of(meta: &fs::Metadata) -> u64 {
    meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// File types that are compressed already: stored as they are.
fn compressible(path: &str, size: u64) -> bool {
    if size < 256 {
        return false;
    }
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    !matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "ogg" | "mp3" | "bank" | "pack" | "zip" | "7z" | "rar" | "gz" | "jar" | "mp4" | "webm" | "fsb"
    )
}

fn safe_item(item: &str) -> String {
    item.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect()
}

impl Store {
    /// The store of a Zomboid folder: the folder the player chose, else `Zomboid/PZModStudio_Versions`.
    pub fn at(zomboid: &Path) -> Store {
        Store { root: super::place::chosen_root(zomboid).unwrap_or_else(|| zomboid.join(STORE_DIR)) }
    }

    pub fn in_dir(root: &Path) -> Store {
        Store { root: root.to_path_buf() }
    }

    /// The store, taken for writing. Refused while it is being moved to another folder; a store
    /// that moved while this waited for it is followed to its new folder.
    pub fn open_locked(zomboid: &Path, wait_secs: u64) -> Result<(Store, StoreLock), String> {
        for _ in 0..3 {
            if super::place::moving(zomboid) {
                return Err(super::place::MOVING.to_string());
            }
            let store = Store::at(zomboid);
            let existed = store.root.exists();
            let lock = store.lock(wait_secs)?;
            if !super::place::moving(zomboid) && Store::at(zomboid).root == store.root {
                return Ok((store, lock));
            }
            drop(lock);
            if !existed {
                // Made by `lock` in the folder it left: only removed when empty.
                let _ = fs::remove_dir(&store.root);
            }
        }
        Err(super::place::MOVING.to_string())
    }

    fn blobs(&self) -> PathBuf {
        self.root.join("blobs")
    }

    fn items(&self) -> PathBuf {
        self.root.join("items")
    }

    pub fn settings(&self) -> StoreSettings {
        fs::read_to_string(self.root.join(SETTINGS_FILE))
            .ok()
            .and_then(|t| serde_json::from_str::<StoreSettings>(&t).ok())
            .map(StoreSettings::migrated)
            .unwrap_or_default()
    }

    /// The player saved settings (Settings, the first-time question, or an agent).
    pub fn has_settings(&self) -> bool {
        self.root.join(SETTINGS_FILE).is_file()
    }

    pub fn set_settings(&self, settings: &StoreSettings) -> Result<(), String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        fs::write(self.root.join(SETTINGS_FILE), text).map_err(|e| e.to_string())
    }

    pub fn status(&self) -> Option<StoreStatus> {
        fs::read_to_string(self.root.join(STATUS_FILE)).ok().and_then(|t| serde_json::from_str(&t).ok())
    }

    pub fn write_status(&self, status: &StoreStatus) {
        if fs::create_dir_all(&self.root).is_ok() {
            if let Ok(text) = serde_json::to_string_pretty(status) {
                let _ = fs::write(self.root.join(STATUS_FILE), text);
            }
        }
    }

    /// Takes the store for writing. Another round (the app's, an agent's) holding it is waited
    /// for up to `wait_secs`; a lock left by a process that ended long ago is taken over.
    pub fn lock(&self, wait_secs: u64) -> Result<StoreLock, String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let path = self.root.join(LOCK_FILE);
        let start = std::time::Instant::now();
        let mut holder_checked = false;
        loop {
            match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut f) => {
                    let _ = write!(f, "{} {}", std::process::id(), now_unix());
                    return Ok(StoreLock(path));
                }
                Err(_) => {
                    let old = fs::metadata(&path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.elapsed().ok())
                        .is_some_and(|age| age.as_secs() > 2 * 3600);
                    // A round cut short (the app closed while it ran) leaves its lock: the process
                    // named in it is gone, so the lock is too (looked at once, not every wait).
                    let holder_gone = !holder_checked && {
                        holder_checked = true;
                        fs::read_to_string(&path)
                            .ok()
                            .and_then(|t| t.split_whitespace().next().and_then(|p| p.parse::<u32>().ok()))
                            .is_some_and(|pid| pid != std::process::id() && crate::platform::process_image_name(pid).is_none())
                    };
                    let stale = old || holder_gone;
                    if stale {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    if start.elapsed().as_secs() >= wait_secs {
                        return Err("Kept versions are being written by another round; try again in a minute.".to_string());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
    }

    fn blob_path(&self, hash: &str) -> Option<PathBuf> {
        let dir = self.blobs().join(&hash[..2.min(hash.len())]);
        let raw = dir.join(hash);
        if raw.is_file() {
            return Some(raw);
        }
        let z = dir.join(format!("{}.z", hash));
        z.is_file().then_some(z)
    }

    /// Reads a file and stores it by content in one pass. Returns its hash, raw size and the bytes
    /// the store grew by (0 when the same content was already there).
    fn put_file(&self, src: &Path, rel: &str) -> io::Result<(String, u64, u64)> {
        let size_hint = fs::metadata(src)?.len();
        let deflate = compressible(rel, size_hint);
        let tmp_dir = self.blobs().join("tmp");
        fs::create_dir_all(&tmp_dir)?;
        let tmp = tmp_dir.join(format!("{}-{}", std::process::id(), TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        let mut hasher = Sha256::new();
        let mut input = fs::File::open(src)?;
        let out = io::BufWriter::new(fs::File::create(&tmp)?);
        let copied = if deflate {
            let mut enc = flate2::write::DeflateEncoder::new(out, flate2::Compression::fast());
            copy_hashing(&mut input, &mut enc, &mut hasher).and_then(|n| enc.finish().and_then(|mut o| o.flush()).map(|_| n))
        } else {
            let mut out = out;
            copy_hashing(&mut input, &mut out, &mut hasher).and_then(|n| out.flush().map(|_| n))
        };
        let raw_len = match copied {
            Ok(n) => n,
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                return Err(e);
            }
        };
        let hash = format!("{:x}", hasher.finalize());
        if self.blob_path(&hash).is_some() {
            let _ = fs::remove_file(&tmp);
            return Ok((hash, raw_len, 0));
        }
        let dir = self.blobs().join(&hash[..2]);
        fs::create_dir_all(&dir)?;
        let dest = if deflate { dir.join(format!("{}.z", hash)) } else { dir.join(&hash) };
        let stored = fs::metadata(&tmp)?.len();
        if let Err(e) = fs::rename(&tmp, &dest) {
            let _ = fs::remove_file(&tmp);
            if self.blob_path(&hash).is_none() {
                return Err(e);
            }
            return Ok((hash, raw_len, 0));
        }
        Ok((hash, raw_len, stored))
    }

    /// Writes one blob's content to `dest`.
    fn write_blob_to(&self, hash: &str, dest: &Path) -> io::Result<()> {
        let blob = self.blob_path(hash).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("a kept file is missing ({})", hash)))?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let input = fs::File::open(&blob)?;
        let mut out = io::BufWriter::new(fs::File::create(dest)?);
        if blob.extension().is_some_and(|e| e == "z") {
            io::copy(&mut flate2::read::DeflateDecoder::new(io::BufReader::new(input)), &mut out)?;
        } else {
            io::copy(&mut io::BufReader::new(input), &mut out)?;
        }
        out.flush()
    }

    /// Every version on disk (heads only), newest first.
    pub fn versions(&self) -> Vec<StoredVersion> {
        let mut out = Vec::new();
        let Ok(items) = fs::read_dir(self.items()) else { return out };
        for item in items.flatten() {
            read_heads(&item.path(), &mut out);
        }
        out.sort_by(|a, b| b.head.kept_unix.cmp(&a.head.kept_unix).then_with(|| b.head_path.cmp(&a.head_path)));
        out
    }

    /// The versions of one item, newest first.
    pub fn versions_of(&self, item: &str) -> Vec<StoredVersion> {
        let mut out = Vec::new();
        read_heads(&self.items().join(safe_item(item)), &mut out);
        out.retain(|v| v.head.item == item);
        out.sort_by(|a, b| b.head.kept_unix.cmp(&a.head.kept_unix).then_with(|| b.head_path.cmp(&a.head_path)));
        out
    }

    pub fn files_of(&self, v: &StoredVersion) -> Result<BTreeMap<String, FileEntry>, String> {
        let text = fs::read_to_string(v.files_path()).map_err(|e| format!("The kept version's file list cannot be read: {}", e))?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    /// The version whose head is at `path`, which has to be inside this store.
    pub fn version_at(&self, path: &Path) -> Result<StoredVersion, String> {
        let inside = path
            .canonicalize()
            .ok()
            .zip(self.items().canonicalize().ok())
            .is_some_and(|(p, r)| p.starts_with(&r));
        if !inside {
            return Err("That is not one of the Studio's kept versions.".to_string());
        }
        let head = fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<VersionHead>(&t).ok())
            .ok_or("This kept version cannot be read.")?;
        Ok(StoredVersion { head, head_path: path.to_path_buf() })
    }

    /// Keeps the folder `root` as a version of `item`. `previous` (the item's newest kept
    /// version) lets unchanged files (same size and time) go without being read again. When
    /// the item already has a version with the same files, that one is returned (marked as the
    /// player's when `by_player`), and nothing new is written.
    pub fn keep(&self, item: &str, root: &Path, mut info: KeepInfo, previous: Option<&StoredVersion>) -> Result<(StoredVersion, u64), String> {
        if !root.is_dir() {
            return Err(format!("{} is not a folder.", root.display()));
        }
        let prev_files = previous.and_then(|p| self.files_of(p).ok()).unwrap_or_default();
        let mut files: BTreeMap<String, FileEntry> = BTreeMap::new();
        let mut grew = 0u64;
        let mut to_read: Vec<(String, PathBuf, u64)> = Vec::new();
        for entry in walkdir::WalkDir::new(root).follow_links(false) {
            let entry = entry.map_err(|e| format!("Reading {} failed: {}", root.display(), e))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(root).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            let mtime = mtime_of(&meta);
            if let Some(was) = prev_files.get(&rel) {
                if was.size == meta.len() && was.mtime == mtime && mtime != 0 && self.blob_path(&was.hash).is_some() {
                    files.insert(rel, was.clone());
                    continue;
                }
            }
            to_read.push((rel, entry.path().to_path_buf(), mtime));
        }
        // Most of the time of a file is the file system's (open, create, rename): a few at once.
        let next = std::sync::atomic::AtomicUsize::new(0);
        let results: Vec<Result<Vec<(String, FileEntry, u64)>, String>> = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..WORKERS.min(to_read.len().max(1)))
                .map(|_| {
                    scope.spawn(|| {
                        let mut done = Vec::new();
                        loop {
                            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some((rel, path, mtime)) = to_read.get(i) else { return Ok(done) };
                            let (hash, size, added) = self.put_file(path, rel).map_err(|e| format!("Keeping {} failed: {}", rel, e))?;
                            done.push((rel.clone(), FileEntry { hash, size, mtime: *mtime }, added));
                        }
                    })
                })
                .collect();
            workers.into_iter().map(|w| w.join().unwrap_or_else(|_| Err("A worker stopped.".to_string()))).collect()
        });
        for r in results {
            for (rel, entry, added) in r? {
                grew += added;
                files.insert(rel, entry);
            }
        }
        let tree = tree_hash(&files);
        for m in info.mods.iter_mut() {
            let prefix = if m.rel.is_empty() { String::new() } else { format!("{}/", m.rel) };
            m.bytes = files.iter().filter(|(p, _)| p.starts_with(&prefix)).map(|(_, f)| f.size).sum();
        }
        // The same files as a version already kept: that one.
        if let Some(same) = self.versions_of(item).into_iter().find(|v| v.head.tree == tree) {
            let mut same = same;
            let mut changed = false;
            if info.by_player && !same.head.by_player {
                same.head.by_player = true;
                changed = true;
            }
            // Steam's newest is these files again (an author undoing an update): this version is
            // the installed one now.
            if info.steam_ident.is_some() && same.head.steam_ident != info.steam_ident {
                same.head.steam_ident = info.steam_ident.clone();
                same.head.published = info.published;
                changed = true;
            }
            if changed {
                write_json(&same.head_path, &same.head)?;
            }
            return Ok((same, grew));
        }
        let now = chrono::Local::now();
        let dir = self.items().join(safe_item(item));
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut stamp = format!("{}-{}", now.format("%Y%m%d-%H%M%S"), &tree[..8]);
        let mut n = 1;
        while dir.join(format!("{}.json", stamp)).exists() {
            n += 1;
            stamp = format!("{}-{}-{}", now.format("%Y%m%d-%H%M%S"), &tree[..8], n);
        }
        let head_path = dir.join(format!("{}.json", stamp));
        let head = VersionHead {
            item: item.to_string(),
            workshop_id: info.workshop_id,
            kept_at: now.format("%Y-%m-%d %H:%M").to_string(),
            kept_unix: now.timestamp(),
            by_player: info.by_player,
            steam_ident: info.steam_ident,
            published: info.published,
            tree,
            mods: info.mods,
            files: files.len(),
            bytes: files.values().map(|f| f.size).sum(),
            source_dir: root.to_string_lossy().to_string(),
        };
        // The file list first: a head without it is not a version.
        write_json(&files_path_of(&head_path), &files)?;
        write_json(&head_path, &head)?;
        Ok((StoredVersion { head, head_path }, grew))
    }

    /// Writes one mod of a version into `dest` (which must not exist).
    pub fn restore_mod(&self, v: &StoredVersion, folder: &str, dest: &Path) -> Result<VersionMod, String> {
        let m = v.head.mods.iter().find(|m| m.folder == folder).cloned().ok_or_else(|| format!("This kept version has no mod folder {}.", folder))?;
        let files = self.files_of(v)?;
        let prefix = if m.rel.is_empty() { String::new() } else { format!("{}/", m.rel) };
        let mut written = 0;
        for (path, entry) in files.iter().filter(|(p, _)| p.starts_with(&prefix)) {
            let rel = &path[prefix.len()..];
            if rel.split('/').any(|c| c == ".." || c.is_empty()) {
                continue;
            }
            let target = rel.split('/').fold(dest.to_path_buf(), |p, c| p.join(c));
            self.write_blob_to(&entry.hash, &target).map_err(|e| format!("Writing {} failed: {}", rel, e))?;
            written += 1;
        }
        if written == 0 {
            return Err(format!("This kept version has no files for {}.", folder));
        }
        Ok(m)
    }

    /// Removes a version's head and file list (its files go with the next `gc`).
    pub fn drop_version(&self, v: &StoredVersion) {
        let _ = fs::remove_file(&v.head_path);
        let _ = fs::remove_file(v.files_path());
        if let Some(parent) = v.head_path.parent() {
            if fs::read_dir(parent).map(|mut d| d.next().is_none()).unwrap_or(false) {
                let _ = fs::remove_dir(parent);
            }
        }
    }

    /// Every stored blob with its size on disk.
    fn blob_sizes(&self) -> HashMap<String, (PathBuf, u64)> {
        let mut out = HashMap::new();
        for e in walkdir::WalkDir::new(self.blobs()).min_depth(2).max_depth(2).into_iter().flatten() {
            if !e.file_type().is_file() || e.path().parent().and_then(|p| p.file_name()).is_some_and(|n| n == "tmp") {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            let hash = name.trim_end_matches(".z").to_string();
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            out.insert(hash, (e.path().to_path_buf(), size));
        }
        out
    }

    /// Deletes what no version names any more, and leftovers of rounds cut short. Returns the
    /// bytes the blobs take after it.
    pub fn gc(&self) -> u64 {
        let mut named: HashSet<String> = HashSet::new();
        for v in self.versions() {
            match self.files_of(&v) {
                Ok(files) => named.extend(files.into_values().map(|f| f.hash)),
                // A version without its file list is no version.
                Err(_) => self.drop_version(&v),
            }
        }
        let mut total = 0;
        for (hash, (path, size)) in self.blob_sizes() {
            if named.contains(&hash) {
                total += size;
            } else {
                let _ = fs::remove_file(path);
            }
        }
        if let Ok(tmp) = fs::read_dir(self.blobs().join("tmp")) {
            for t in tmp.flatten() {
                let _ = fs::remove_file(t.path());
            }
        }
        total
    }

    /// Bytes of the files that versions the cap never drops (kept by hand, in use) name.
    pub fn protected_bytes(&self, in_use: &HashSet<PathBuf>) -> u64 {
        let sizes = self.blob_sizes();
        let mut named: HashSet<String> = HashSet::new();
        for v in self.versions().iter().filter(|v| v.head.by_player || in_use.contains(&v.head_path)) {
            if let Ok(files) = self.files_of(v) {
                named.extend(files.into_values().map(|f| f.hash));
            }
        }
        named.iter().filter_map(|h| sizes.get(h)).map(|(_, s)| *s).sum()
    }

    /// Drops versions, in the order of `eviction_order`, until the blobs take no more than `cap`.
    /// Returns the versions dropped and the bytes left.
    pub fn enforce_cap(&self, cap: u64, ctx: &CapContext) -> (Vec<VersionHead>, u64) {
        let versions = self.versions();
        let sizes = self.blob_sizes();
        let mut files: HashMap<PathBuf, Vec<String>> = HashMap::new();
        let mut refs: HashMap<String, u32> = HashMap::new();
        for v in &versions {
            let hashes: Vec<String> = self.files_of(v).map(|f| f.into_values().map(|e| e.hash).collect()).unwrap_or_default();
            let unique: HashSet<&String> = hashes.iter().collect();
            for h in unique {
                *refs.entry(h.clone()).or_default() += 1;
            }
            files.insert(v.head_path.clone(), hashes);
        }
        let mut total: u64 = refs.keys().filter_map(|h| sizes.get(h)).map(|(_, s)| *s).sum();
        let mut dropped = Vec::new();
        if total <= cap {
            return (dropped, total);
        }
        for v in eviction_order(&versions, ctx) {
            if total <= cap {
                break;
            }
            let hashes: HashSet<String> = files.remove(&v.head_path).unwrap_or_default().into_iter().collect();
            for h in hashes {
                if let Some(n) = refs.get_mut(&h) {
                    *n -= 1;
                    if *n == 0 {
                        refs.remove(&h);
                        if let Some((path, size)) = sizes.get(&h) {
                            total = total.saturating_sub(*size);
                            let _ = fs::remove_file(path);
                        }
                    }
                }
            }
            self.drop_version(&v);
            dropped.push(v.head.clone());
        }
        (dropped, total)
    }
}

fn read_heads(dir: &Path, out: &mut Vec<StoredVersion>) {
    let Ok(files) = fs::read_dir(dir) else { return };
    for f in files.flatten() {
        let p = f.path();
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if !name.ends_with(".json") || name.ends_with(".files.json") {
            continue;
        }
        let Some(head) = fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str::<VersionHead>(&t).ok()) else { continue };
        out.push(StoredVersion { head, head_path: p });
    }
}

static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Files read and stored at once.
const WORKERS: usize = 6;

fn copy_hashing(input: &mut impl Read, out: &mut impl Write, hasher: &mut Sha256) -> io::Result<u64> {
    let mut buf = vec![0u8; 1 << 16];
    let mut total = 0u64;
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            return Ok(total);
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n])?;
        total += n as u64;
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let text = serde_json::to_string(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text).map_err(|e| format!("Cannot write {}: {}", tmp.display(), e))?;
    fs::rename(&tmp, path).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

fn tree_hash(files: &BTreeMap<String, FileEntry>) -> String {
    let mut h = Sha256::new();
    for (path, f) in files {
        h.update(path.as_bytes());
        h.update([0]);
        h.update(f.hash.as_bytes());
        h.update([b'\n']);
    }
    format!("{:x}", h.finalize())
}

/// What a version is kept with.
#[derive(Debug, Clone, Default)]
pub struct KeepInfo {
    pub workshop_id: Option<String>,
    pub by_player: bool,
    pub steam_ident: Option<String>,
    pub published: Option<u64>,
    pub mods: Vec<VersionMod>,
}

/// What the cap needs to know to choose what goes first.
#[derive(Debug, Clone, Default)]
pub struct CapContext {
    /// Item -> the identity of the version Steam has installed now.
    pub installed: HashMap<String, String>,
    /// Items with a mod on in the active profile.
    pub active_items: HashSet<String>,
    /// Heads in use as a local copy in Zomboid/mods.
    pub in_use: HashSet<PathBuf>,
}

/// The versions the cap may drop, first to go first (see the module's comment).
pub fn eviction_order(versions: &[StoredVersion], ctx: &CapContext) -> Vec<StoredVersion> {
    let is_current = |v: &StoredVersion| v.head.steam_ident.is_some() && ctx.installed.get(&v.head.item) == v.head.steam_ident.as_ref();
    // The newest earlier (not current) version of each item.
    let mut newest_earlier: HashMap<&str, &StoredVersion> = HashMap::new();
    for v in versions.iter().filter(|v| !is_current(v)) {
        let slot = newest_earlier.entry(v.head.item.as_str()).or_insert(v);
        if v.head.kept_unix > slot.head.kept_unix {
            *slot = v;
        }
    }
    let mut ranked: Vec<(u8, i64, &StoredVersion)> = Vec::new();
    for v in versions {
        if v.head.by_player || ctx.in_use.contains(&v.head_path) {
            continue;
        }
        let active = ctx.active_items.contains(&v.head.item);
        let (rank, key) = if is_current(v) {
            // Biggest first.
            (if active { 4 } else { 2 }, -(v.head.bytes as i64))
        } else if newest_earlier.get(v.head.item.as_str()).is_some_and(|n| n.head_path == v.head_path) {
            (3, v.head.kept_unix)
        } else {
            (1, v.head.kept_unix)
        };
        ranked.push((rank, key, v));
    }
    ranked.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, _, v)| v.clone()).collect()
}

/// What the last round of keeping found, shown in Settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoreStatus {
    /// Unix seconds of the last round.
    pub last_run: i64,
    /// Workshop items a profile uses.
    pub items_used: usize,
    /// Of those, how many have their installed version kept.
    pub items_kept: usize,
    /// Mods (names) left without a copy because the cap had no room.
    pub no_room: Vec<String>,
    /// Bytes the store takes on disk.
    pub bytes: u64,
    pub versions: usize,
    /// Earlier versions kept (a mod Steam updated since it was kept).
    pub earlier_versions: usize,
    /// What the last round could not read, in words.
    #[serde(default)]
    pub errors: Vec<String>,
    /// The cap that applied: the chosen one, or less when the drive had no more room.
    #[serde(default)]
    pub cap_effective: u64,
    /// The drive ("D:") whose free space limited the cap, when it did.
    #[serde(default)]
    pub limited_by_disk: Option<String>,
    /// Bytes of versions the cap never drops (kept by hand, in use).
    #[serde(default)]
    pub protected_bytes: u64,
}

/// One installed item, as Steam's record says.
#[derive(Debug, Clone, Default)]
pub struct SteamItem {
    pub ident: Option<String>,
    pub published: Option<u64>,
    pub size: u64,
    /// Steam knows of a newer version it has not downloaded yet: the files on disk are still the old ones.
    pub pending: bool,
}

/// Steam's record of installed Project Zomboid items (`appworkshop_108600.acf`, two folders up
/// from `content/108600`), read only.
pub fn read_steam_items(workshop_content_dir: &Path) -> HashMap<String, SteamItem> {
    let mut out = HashMap::new();
    let Some(acf) = workshop_content_dir.parent().and_then(|p| p.parent()).map(|p| p.join("appworkshop_108600.acf")) else { return out };
    let Ok(text) = fs::read_to_string(acf) else { return out };
    let root = crate::vdf::parse(&text);
    let Some(app) = root.get("AppWorkshop") else { return out };
    let details = app.get("WorkshopItemDetails");
    for (id, installed) in app.get("WorkshopItemsInstalled").and_then(|b| b.block()).into_iter().flatten() {
        let detail = details.and_then(|d| d.get(id));
        let time = installed.u64("timeupdated").or_else(|| detail.and_then(|d| d.u64("timeupdated")));
        let manifest = installed.str("manifest").or_else(|| detail.and_then(|d| d.str("manifest"))).filter(|m| !m.is_empty() && *m != "0");
        let latest_time = detail.and_then(|d| d.u64("latest_timeupdated"));
        let latest_manifest = detail.and_then(|d| d.str("latest_manifest"));
        let pending = matches!((latest_time, time), (Some(l), Some(i)) if l > i) || matches!((latest_manifest, manifest), (Some(l), Some(m)) if l != m && l != "0");
        out.insert(
            id.clone(),
            SteamItem {
                ident: manifest.map(|m| m.to_string()).or_else(|| time.map(|t| format!("t{}", t))),
                published: time,
                size: installed.u64("size").unwrap_or(0),
                pending,
            },
        );
    }
    out
}

/// Steam is writing this item now (its download or staging folder exists): not a moment to read it.
pub fn steam_busy(workshop_content_dir: &Path, id: &str) -> bool {
    let Some(workshop) = workshop_content_dir.parent().and_then(|p| p.parent()) else { return false };
    ["downloads", "temp"].iter().any(|d| workshop.join(d).join("108600").join(id).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct Tmp(pub PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn tmp(tag: &str) -> Tmp {
        let t = Tmp(std::env::temp_dir().join(format!("pzms-store-{}-{}", tag, std::process::id())));
        let _ = fs::remove_dir_all(&t.0);
        fs::create_dir_all(&t.0).unwrap();
        t
    }
    fn write(p: &Path, text: &str) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    fn info(id: &str, ident: &str) -> KeepInfo {
        KeepInfo {
            workshop_id: Some(id.into()),
            by_player: false,
            steam_ident: Some(ident.into()),
            published: Some(1),
            mods: vec![VersionMod { id: "Foo".into(), name: "Foo".into(), folder: "Foo".into(), rel: "mods/Foo".into(), version: Some("1".into()), bytes: 0 }],
        }
    }

    #[test]
    fn a_file_is_stored_once_and_an_update_costs_only_what_changed() {
        let t = tmp("dedup");
        let store = Store::at(&t.0.join("Zomboid"));
        let item = t.0.join("ws/111");
        let big = "x".repeat(50_000);
        write(&item.join("mods/Foo/42/media/lua/shared/big.lua"), &big);
        write(&item.join("mods/Foo/42/media/lua/shared/a.lua"), "return 1");
        write(&item.join("mods/Foo/42/media/lua/shared/copy.lua"), &big);
        write(&item.join("mods/Foo/42/mod.info"), "id=Foo\nmodversion=1\n");
        let (v1, grew1) = store.keep("111", &item, info("111", "m1"), None).unwrap();
        assert_eq!(v1.head.files, 4);
        assert_eq!(v1.head.mods[0].bytes, v1.head.bytes, "the mod is the whole item here");
        // Two files with the same content: one blob; text deflates far below its size.
        let blobs = store.blob_sizes();
        assert_eq!(blobs.len(), 3);
        assert!(grew1 < 10_000, "deflated: {}", grew1);

        // Steam replaces one file: the next version adds one blob.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        write(&item.join("mods/Foo/42/media/lua/shared/a.lua"), "return 2");
        let (v2, grew2) = store.keep("111", &item, info("111", "m2"), Some(&v1)).unwrap();
        assert_ne!(v1.head.tree, v2.head.tree);
        assert_eq!(store.blob_sizes().len(), 4);
        assert!(grew2 > 0 && grew2 < 100, "{}", grew2);

        // The same files again: the version already kept, nothing new.
        let (v3, grew3) = store.keep("111", &item, KeepInfo { by_player: true, ..info("111", "m2") }, Some(&v2)).unwrap();
        assert_eq!(v3.head_path, v2.head_path);
        assert_eq!(grew3, 0);
        assert!(store.versions().iter().find(|v| v.head_path == v2.head_path).unwrap().head.by_player, "kept by hand now");
        assert_eq!(store.versions().len(), 2);

        // The author undoes the update: the first version is the installed one again.
        write(&item.join("mods/Foo/42/media/lua/shared/a.lua"), "return 1");
        let (back, _) = store.keep("111", &item, info("111", "m3"), Some(&v2)).unwrap();
        assert_eq!(back.head_path, v1.head_path);
        assert_eq!(store.version_at(&v1.head_path).unwrap().head.steam_ident.as_deref(), Some("m3"));
        write(&item.join("mods/Foo/42/media/lua/shared/a.lua"), "return 2");

        // The first version comes back as it was.
        let dest = t.0.join("Zomboid/mods/Foo");
        let m = store.restore_mod(&v1, "Foo", &dest).unwrap();
        assert_eq!(m.id, "Foo");
        assert_eq!(fs::read_to_string(dest.join("42/media/lua/shared/a.lua")).unwrap(), "return 1");
        assert_eq!(fs::read_to_string(dest.join("42/media/lua/shared/big.lua")).unwrap(), big);
        assert!(item.join("mods/Foo/42/media/lua/shared/a.lua").is_file(), "the source is only read");

        // Dropping a version deletes only the files no other version names.
        store.drop_version(&v1);
        store.gc();
        assert_eq!(store.blob_sizes().len(), 3);
        let dest2 = t.0.join("again");
        store.restore_mod(&v2, "Foo", &dest2).unwrap();
        assert_eq!(fs::read_to_string(dest2.join("42/media/lua/shared/a.lua")).unwrap(), "return 2");
    }

    #[test]
    fn the_cap_drops_older_versions_first_and_never_the_players() {
        let t = tmp("cap");
        let store = Store::at(&t.0.join("Zomboid"));
        let mut kept = Vec::new();
        // Item 1: three versions over time (incompressible-ish content, distinct per version).
        let item = t.0.join("ws/1");
        for (n, ident) in ["a", "b", "c"].iter().enumerate() {
            let body: String = (0..20_000).map(|i| char::from(b'a' + ((i * 7 + n * 5 + i / 3) % 26) as u8)).collect();
            write(&item.join("mods/Foo/f.png"), &body);
            std::thread::sleep(std::time::Duration::from_millis(1100));
            let (v, _) = store.keep("1", &item, info("1", ident), kept.last()).unwrap();
            kept.push(v);
        }
        // Item 2: one current version, kept by the player.
        let item2 = t.0.join("ws/2");
        write(&item2.join("mods/Bar/f.png"), &"q".repeat(20_000));
        let (by_hand, _) = store.keep("2", &item2, KeepInfo { by_player: true, ..info("2", "z") }, None).unwrap();

        let ctx = CapContext { installed: [("1".to_string(), "c".to_string()), ("2".to_string(), "z".to_string())].into(), ..Default::default() };
        let order: Vec<String> = eviction_order(&store.versions(), &ctx).iter().map(|v| v.head.steam_ident.clone().unwrap()).collect();
        // The oldest earlier one, then the current copy (not in the active profile), then the last earlier one.
        assert_eq!(order, vec!["a", "c", "b"]);

        // Room for about two versions: "a" goes, the player's stays.
        let (dropped, left) = store.enforce_cap(45_000, &ctx);
        assert_eq!(dropped.iter().map(|d| d.steam_ident.clone().unwrap()).collect::<Vec<_>>(), vec!["a", "c"]);
        assert!(left <= 45_000, "{}", left);
        let idents: Vec<String> = store.versions().iter().map(|v| v.head.steam_ident.clone().unwrap()).collect();
        assert!(idents.contains(&"b".to_string()) && idents.contains(&"z".to_string()), "{:?}", idents);
        assert!(store.versions().iter().any(|v| v.head_path == by_hand.head_path));

        // In the active profile, the current copy is kept over the last earlier version.
        let ctx2 = CapContext { active_items: ["1".to_string()].into(), ..ctx };
        let order2: Vec<String> = eviction_order(&kept_now(&store), &ctx2).iter().map(|v| v.head.steam_ident.clone().unwrap()).collect();
        assert_eq!(order2, vec!["b"]);
    }

    fn kept_now(store: &Store) -> Vec<StoredVersion> {
        store.versions()
    }

    const GB: u64 = 1 << 30;

    #[test]
    fn the_drive_guard_leaves_fifteen_percent_free() {
        // A 100 GB drive keeps 15 GB free. With 40 GB free and 2 GB kept, the store may grow by
        // 25 GB: the 10 GB cap applies.
        assert_eq!(drive_reserve(100 * GB), 15 * GB);
        assert_eq!(disk_guard(10 * GB, 2 * GB, 40 * GB, 100 * GB, 0), (10 * GB, false));
        // 20 GB free: room for 5 GB more, 7 GB in all.
        assert_eq!(disk_guard(10 * GB, 2 * GB, 20 * GB, 100 * GB, 0), (7 * GB, true));
        // Below the reserve already: no more than it holds now...
        assert_eq!(disk_guard(10 * GB, 4 * GB, 10 * GB, 100 * GB, 0), (4 * GB, true));
        // ...and never below what the cap cannot drop (kept by hand, in use).
        assert_eq!(disk_guard(10 * GB, 4 * GB, 5 * GB, 100 * GB, 3 * GB), (4 * GB, true));
        assert_eq!(disk_guard(10 * GB, 4 * GB, 0, 100 * GB, 6 * GB), (6 * GB, true));
        // Exactly at the cap: the cap, not the drive.
        assert_eq!(disk_guard(10 * GB, 0, 25 * GB, 100 * GB, 0), (10 * GB, false));
        // A drive's size that does not divide by 100.
        assert_eq!(drive_reserve(1001), 150);
    }

    #[test]
    fn the_old_default_cap_moves_to_ten_gigabytes_and_a_chosen_one_stays() {
        let t = tmp("cap-default");
        let store = Store::at(&t.0.join("Zomboid"));
        assert_eq!(store.settings().cap_bytes, 10 * GB, "a new store");
        // Written by the first version: the old default, no choice recorded.
        fs::create_dir_all(&store.root).unwrap();
        fs::write(store.root.join(SETTINGS_FILE), r#"{"enabled":true,"cap_bytes":5368709120}"#).unwrap();
        assert_eq!(store.settings(), StoreSettings { enabled: true, cap_bytes: 10 * GB, cap_by_player: false });
        // A value the player picked then (20 GB, 2 GB) stays, and counts as chosen.
        fs::write(store.root.join(SETTINGS_FILE), r#"{"enabled":false,"cap_bytes":21474836480}"#).unwrap();
        assert_eq!(store.settings(), StoreSettings { enabled: false, cap_bytes: 20 * GB, cap_by_player: true });
        fs::write(store.root.join(SETTINGS_FILE), r#"{"enabled":true,"cap_bytes":2147483648}"#).unwrap();
        assert_eq!(store.settings().cap_bytes, 2 * GB);
        // 5 GB chosen by the player after this change stays 5 GB.
        store.set_settings(&StoreSettings { enabled: true, cap_bytes: 5 * GB, cap_by_player: true }).unwrap();
        assert_eq!(store.settings().cap_bytes, 5 * GB);
    }

    #[test]
    fn a_path_outside_the_store_is_refused() {
        let t = tmp("outside");
        let store = Store::at(&t.0.join("Zomboid"));
        fs::create_dir_all(store.items()).unwrap();
        let elsewhere = t.0.join("x.json");
        fs::write(&elsewhere, "{}").unwrap();
        assert!(store.version_at(&elsewhere).is_err());
    }

    #[test]
    fn steams_record_gives_each_item_its_version() {
        let t = tmp("acf");
        let content = t.0.join("steamapps/workshop/content/108600");
        fs::create_dir_all(&content).unwrap();
        fs::write(
            t.0.join("steamapps/workshop/appworkshop_108600.acf"),
            "\"AppWorkshop\"\n{\n\"WorkshopItemsInstalled\"\n{\n\"5\"\n{\n\"size\"\t\"10\"\n\"timeupdated\"\t\"100\"\n\"manifest\"\t\"777\"\n}\n\"6\"\n{\n\"size\"\t\"3\"\n\"timeupdated\"\t\"50\"\n}\n}\n\"WorkshopItemDetails\"\n{\n\"5\"\n{\n\"manifest\"\t\"777\"\n\"timeupdated\"\t\"100\"\n\"latest_timeupdated\"\t\"200\"\n\"latest_manifest\"\t\"888\"\n}\n}\n}\n",
        )
        .unwrap();
        let items = read_steam_items(&content);
        assert_eq!(items["5"].ident.as_deref(), Some("777"));
        assert!(items["5"].pending);
        assert_eq!(items["6"].ident.as_deref(), Some("t50"));
        assert!(!items["6"].pending);
        assert!(!steam_busy(&content, "5"));
        fs::create_dir_all(t.0.join("steamapps/workshop/downloads/108600/5")).unwrap();
        assert!(steam_busy(&content, "5"));
    }
}
