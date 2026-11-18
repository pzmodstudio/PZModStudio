//! Kept versions of a mod, so that when Steam replaces one with an update that breaks a save,
//! the version that worked can be put back.
//!
//! Steam only ever gives you the newest copy of a Workshop item, and overwrites the folder in
//! place; there is no going back once it has. So the Studio keeps a copy of the version installed
//! now of each Workshop item a profile uses, by itself, in the background (`store`: every file
//! once by content, so an update costs only what it changed; `Zomboid/PZModStudio_Versions` or a
//! folder the player chose, `place`), and the player can keep one by hand too. Copies made before the store existed (`Zomboid/PZModStudio_Snapshots/<folder>/<when>/`,
//! plain folders) are still listed and can still be put back.
//!
//! Putting one back does not touch the Workshop folder (Steam would just overwrite it again): it
//! goes into `Zomboid/mods` as a local copy, the way a compatible copy does, and the Studio
//! launches the game with `-modfolders mods,workshop,steam` so the copy wins over the Workshop
//! original. "Back to the Workshop version" moves that copy to the backups.

pub mod place;
pub mod store;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::catalog::{ModCatalog, ModSource};
use crate::load_order::mod_info::get_existing_user_zomboid_dirs;
use crate::origins::{self, ModOrigin};
use store::{CapContext, KeepInfo, Store, StoreSettings, StoreStatus, VersionMod};

const SNAPSHOTS_DIR: &str = "PZModStudio_Snapshots";
const MANIFEST: &str = "pzms_kept.json";
/// Between a store version's head and the mod folder in `KeptVersion::path`.
const PATH_SEP: &str = "::";

/// One kept version of a mod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeptVersion {
    pub mod_id: String,
    pub mod_name: String,
    /// The mod folder's name: what goes under Zomboid/mods when it is put back.
    pub folder: String,
    /// "2026-10-17 14:05".
    pub kept_at: String,
    /// `modversion=` from mod.info at the time, when it had one.
    pub version: Option<String>,
    pub workshop_id: Option<String>,
    /// Where the copy was taken from.
    pub source_dir: String,
    /// What names this kept version to put it back or delete it.
    pub path: String,
    pub bytes: u64,
    /// "auto" (kept by the Studio by itself), "kept" (by the player), "copy" (a plain copy from
    /// before the store).
    #[serde(default)]
    pub kind: String,
    /// The same version Steam has installed now.
    #[serde(default)]
    pub current: bool,
    /// Put back: the game uses this one, from Zomboid/mods.
    #[serde(default)]
    pub in_use: bool,
    /// When its author published it ("2026-11-02"), from Steam's record.
    #[serde(default)]
    pub published: Option<String>,
}

fn zomboid_dir(user_zomboid_dir: &str) -> Result<PathBuf, String> {
    get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))
}

fn date_of(unix: u64) -> Option<String> {
    chrono::DateTime::from_timestamp(unix as i64, 0).map(|d| d.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string())
}

/// The folder of a Workshop item, and the mod's folder inside it ("mods/Foo"), when `root_dir`
/// lies in `workshop_dir/<id>`.
fn workshop_item_root(workshop_dir: &str, workshop_id: &str, root_dir: &Path) -> Option<(PathBuf, String)> {
    let item = Path::new(workshop_dir).join(workshop_id);
    let rel = root_dir.strip_prefix(&item).ok()?.to_string_lossy().replace('\\', "/");
    Some((item, rel))
}

/// The mods of one Workshop item, as the catalog has them.
fn mods_of_item(catalog: &ModCatalog, workshop_dir: &str, workshop_id: &str) -> Vec<VersionMod> {
    let mut out: Vec<VersionMod> = Vec::new();
    for m in catalog.mods.iter().filter(|m| matches!(&m.source, ModSource::Workshop { workshop_id: w } if w == workshop_id)) {
        let root = Path::new(&m.root_dir);
        let Some((_, rel)) = workshop_item_root(workshop_dir, workshop_id, root) else { continue };
        let folder = root.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        if folder.is_empty() || out.iter().any(|o| o.rel == rel) {
            continue;
        }
        out.push(VersionMod { id: m.id.clone(), name: m.name.clone(), folder, rel, version: m.info.mod_version.clone(), bytes: 0 });
    }
    out
}

/// Keeps the mod as it is installed today ("Keep this version"): its whole Workshop item, or its
/// folder for a mod that lives in Zomboid/mods. Kept by hand, the cap never drops it.
pub fn keep_mod(catalog: &ModCatalog, user_zomboid_dir: &str, workshop_dir: &str, mod_id: &str) -> Result<KeptVersion, String> {
    let zomboid = zomboid_dir(user_zomboid_dir)?;
    let m = catalog.resolve(mod_id).ok_or_else(|| format!("{} is not installed.", mod_id))?;
    let (store, _lock) = Store::open_locked(&zomboid, 120)?;
    if let Some(g) = drive_guard(&store, u64::MAX, 0, 0) {
        if g.free_bytes <= g.reserve_bytes {
            return Err(format!(
                "{} has {} MB free and the Studio leaves {}% of a drive free: nothing more is kept there. Free some space or move the kept versions to another drive (Settings, App).",
                g.disk,
                g.free_bytes >> 20,
                store::DRIVE_RESERVE_PERCENT
            ));
        }
    }
    let root = Path::new(&m.root_dir);
    let folder = root.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let steam = store::read_steam_items(Path::new(workshop_dir));
    let (item, item_root, info) = match &m.source {
        ModSource::Workshop { workshop_id } => match workshop_item_root(workshop_dir, workshop_id, root) {
            Some((item_root, _)) => {
                let s = steam.get(workshop_id);
                (
                    workshop_id.clone(),
                    item_root,
                    KeepInfo {
                        workshop_id: Some(workshop_id.clone()),
                        by_player: true,
                        steam_ident: s.and_then(|s| s.ident.clone()),
                        published: s.and_then(|s| s.published),
                        mods: mods_of_item(catalog, workshop_dir, workshop_id),
                    },
                )
            }
            None => (format!("local-{}", folder), root.to_path_buf(), local_info(m, &folder)),
        },
        _ => (format!("local-{}", folder), root.to_path_buf(), local_info(m, &folder)),
    };
    let previous = store.versions_of(&item).into_iter().next();
    let (v, _) = store.keep(&item, &item_root, info, previous.as_ref())?;
    let installed: HashMap<String, String> = steam.into_iter().filter_map(|(id, s)| s.ident.map(|i| (id, i))).collect();
    entries_of(&v, &installed, &in_use_heads(&zomboid))
        .into_iter()
        .find(|k| k.mod_id == m.id)
        .ok_or_else(|| "The version was kept, but its mod was not found in it.".to_string())
}

fn local_info(m: &crate::catalog::CatalogMod, folder: &str) -> KeepInfo {
    KeepInfo {
        workshop_id: None,
        by_player: true,
        steam_ident: None,
        published: None,
        mods: vec![VersionMod { id: m.id.clone(), name: m.name.clone(), folder: folder.to_string(), rel: String::new(), version: m.info.mod_version.clone(), bytes: 0 }],
    }
}

/// The store versions in use as local copies: origin heads of Zomboid/mods folders.
fn in_use_heads(zomboid: &Path) -> HashSet<PathBuf> {
    origins::load(zomboid)
        .into_iter()
        .filter(|(folder, _)| zomboid.join("mods").join(folder).is_dir())
        .filter_map(|(_, r)| match r.origin {
            ModOrigin::Compat { source_dir, .. } => Some(PathBuf::from(source_dir)),
            _ => None,
        })
        .collect()
}

/// One entry per mod of a stored version.
fn entries_of(v: &store::StoredVersion, installed: &HashMap<String, String>, in_use: &HashSet<PathBuf>) -> Vec<KeptVersion> {
    let current = v.head.steam_ident.is_some() && installed.get(&v.head.item) == v.head.steam_ident.as_ref();
    v.head
        .mods
        .iter()
        .map(|m| KeptVersion {
            mod_id: m.id.clone(),
            mod_name: m.name.clone(),
            folder: m.folder.clone(),
            kept_at: v.head.kept_at.clone(),
            version: m.version.clone(),
            workshop_id: v.head.workshop_id.clone(),
            source_dir: v.head.source_dir.clone(),
            path: format!("{}{}{}", v.head_path.to_string_lossy(), PATH_SEP, m.folder),
            bytes: m.bytes,
            kind: if v.head.by_player { "kept" } else { "auto" }.to_string(),
            current,
            in_use: in_use.contains(&v.head_path),
            published: v.head.published.and_then(date_of),
        })
        .collect()
}

/// Every kept version on disk, newest first: the store's (one entry per mod) and the plain
/// copies from before it.
pub fn list_kept_versions(user_zomboid_dir: &str, workshop_dir: &str) -> Vec<KeptVersion> {
    let Ok(zomboid) = zomboid_dir(user_zomboid_dir) else { return Vec::new() };
    let in_use = in_use_heads(&zomboid);
    let installed: HashMap<String, String> = store::read_steam_items(Path::new(workshop_dir)).into_iter().filter_map(|(id, s)| s.ident.map(|i| (id, i))).collect();
    let mut found: Vec<(i64, KeptVersion)> = Vec::new();
    for v in Store::at(&zomboid).versions() {
        for k in entries_of(&v, &installed, &in_use) {
            found.push((v.head.kept_unix, k));
        }
    }
    for k in legacy_versions(&zomboid) {
        let unix = chrono::NaiveDateTime::parse_from_str(&k.kept_at, "%Y-%m-%d %H:%M").map(|d| d.and_utc().timestamp()).unwrap_or(0);
        let mut k = k;
        k.in_use = in_use.contains(Path::new(&k.path));
        found.push((unix, k));
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.kept_at.cmp(&a.1.kept_at)));
    found.into_iter().map(|(_, k)| k).collect()
}

/// The plain copies from before the store.
fn legacy_versions(zomboid: &Path) -> Vec<KeptVersion> {
    let mut found = Vec::new();
    let Ok(folders) = fs::read_dir(zomboid.join(SNAPSHOTS_DIR)) else { return found };
    for folder in folders.flatten() {
        let Ok(versions) = fs::read_dir(folder.path()) else { continue };
        for version in versions.flatten() {
            let Ok(text) = fs::read_to_string(version.path().join(MANIFEST)) else { continue };
            if let Ok(mut kept) = serde_json::from_str::<KeptVersion>(&text) {
                // The folder may have been moved: what is on disk is the truth.
                kept.path = version.path().to_string_lossy().to_string();
                kept.kind = "copy".to_string();
                found.push(kept);
            }
        }
    }
    found
}

/// The plain copy at `path`, which has to be inside the snapshots folder.
fn legacy_at(zomboid: &Path, path: &str) -> Result<(PathBuf, KeptVersion), String> {
    let dir = PathBuf::from(path);
    let root = zomboid.join(SNAPSHOTS_DIR);
    let inside = dir
        .canonicalize()
        .ok()
        .zip(root.canonicalize().ok())
        .map(|(d, r)| d.starts_with(&r))
        .unwrap_or(false);
    if !inside {
        return Err("That is not one of the Studio's kept versions.".to_string());
    }
    let text = fs::read_to_string(dir.join(MANIFEST)).map_err(|_| "This kept version has no manifest.".to_string())?;
    let kept = serde_json::from_str::<KeptVersion>(&text).map_err(|e| e.to_string())?;
    Ok((dir, kept))
}

/// Makes room for `folder` in Zomboid/mods: refused for a folder the Studio did not put there; a
/// copy it did is moved to the backups.
fn clear_destination(zomboid: &Path, folder: &str) -> Result<PathBuf, String> {
    let dest = zomboid.join("mods").join(folder);
    if dest.exists() {
        let is_copy = matches!(origins::load(zomboid).get(folder).map(|r| &r.origin), Some(ModOrigin::Compat { .. }));
        if !is_copy {
            return Err(format!("Zomboid/mods/{} already exists and is not a copy the Studio made; move it away first.", folder));
        }
        let backup = zomboid
            .join("PZModStudio_Backups")
            .join("mods")
            .join(format!("{}-{}", folder, chrono::Local::now().format("%Y%m%d-%H%M%S")));
        fs::create_dir_all(backup.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::rename(&dest, &backup).map_err(|e| format!("Cannot move the old copy aside: {}", e))?;
    }
    Ok(dest)
}

fn put_back_note(kept_at: &str, version: Option<&String>) -> String {
    format!(
        "The version kept on {}{}, put back by the Studio; the Workshop copy is ignored while this is here.",
        kept_at,
        version.map(|v| format!(" (modversion {})", v)).unwrap_or_default()
    )
}

/// Puts a kept version back, as a local copy in Zomboid/mods that wins over the Workshop
/// original. The kept version itself stays, so this can be done again. The caller refuses while
/// the game runs.
pub fn restore_kept_version(user_zomboid_dir: &str, path: &str) -> Result<KeptVersion, String> {
    let zomboid = zomboid_dir(user_zomboid_dir)?;
    if let Some((head, folder)) = path.rsplit_once(PATH_SEP) {
        let store = Store::at(&zomboid);
        let v = store.version_at(Path::new(head))?;
        let dest = clear_destination(&zomboid, folder)?;
        let m = store.restore_mod(&v, folder, &dest).inspect_err(|_| {
            let _ = fs::remove_dir_all(&dest);
        })?;
        origins::record(
            &zomboid,
            std::slice::from_ref(&m.folder),
            &ModOrigin::Compat {
                workshop_id: v.head.workshop_id.clone(),
                source_dir: v.head_path.to_string_lossy().to_string(),
                changes: vec![put_back_note(&v.head.kept_at, m.version.as_ref())],
            },
        )?;
        let mut in_use = HashSet::new();
        in_use.insert(v.head_path.clone());
        return entries_of(&v, &HashMap::new(), &in_use).into_iter().find(|k| k.folder == folder).ok_or_else(|| "Put back.".to_string());
    }
    let (dir, mut kept) = legacy_at(&zomboid, path)?;
    let dest = clear_destination(&zomboid, &kept.folder)?;
    crate::mod_install::copy_tree(&dir, &dest).map_err(|e| {
        let _ = fs::remove_dir_all(&dest);
        format!("Copying the kept version failed: {}", e)
    })?;
    let _ = fs::remove_file(dest.join(MANIFEST));
    kept.path = dir.to_string_lossy().to_string();
    origins::record(
        &zomboid,
        std::slice::from_ref(&kept.folder),
        &ModOrigin::Compat { workshop_id: kept.workshop_id.clone(), source_dir: kept.path.clone(), changes: vec![put_back_note(&kept.kept_at, kept.version.as_ref())] },
    )?;
    kept.kind = "copy".to_string();
    kept.in_use = true;
    Ok(kept)
}

/// Deletes one kept version from disk. For a store version, every mod of that item version goes
/// (they are one version of one Workshop item); files other versions share stay.
pub fn delete_kept_version(user_zomboid_dir: &str, path: &str) -> Result<(), String> {
    let zomboid = zomboid_dir(user_zomboid_dir)?;
    if let Some((head, _)) = path.rsplit_once(PATH_SEP) {
        let (store, _lock) = Store::open_locked(&zomboid, 120)?;
        let v = store.version_at(Path::new(head))?;
        store.drop_version(&v);
        let bytes = store.gc();
        if let Some(mut status) = store.status() {
            status.bytes = bytes;
            status.versions = status.versions.saturating_sub(1);
            store.write_status(&status);
        }
        return Ok(());
    }
    let (dir, _) = legacy_at(&zomboid, path)?;
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    // An empty folder for the mod is noise.
    if let Some(parent) = dir.parent() {
        if fs::read_dir(parent).map(|mut d| d.next().is_none()).unwrap_or(false) {
            let _ = fs::remove_dir(parent);
        }
    }
    Ok(())
}

/// What a local copy in Zomboid/mods put back from a kept version is: when it was kept and its
/// modversion. `source_dir` is the copy's origin, `folder` its folder in Zomboid/mods; None when
/// it is not a kept version.
pub fn kept_copy_info(source_dir: &str, folder: &str) -> Option<(String, Option<String>)> {
    let p = Path::new(source_dir);
    // A store version's head, wherever the store is.
    if p.extension().is_some_and(|e| e == "json") {
        let head: store::VersionHead = serde_json::from_str(&fs::read_to_string(p).ok()?).ok()?;
        let version = head.mods.iter().find(|m| m.folder == folder).and_then(|m| m.version.clone());
        return Some((head.kept_at, version));
    }
    if p.components().any(|c| c.as_os_str() == SNAPSHOTS_DIR) {
        let kept: KeptVersion = fs::read_to_string(p.join(MANIFEST)).ok().and_then(|t| serde_json::from_str(&t).ok())?;
        return Some((kept.kept_at, kept.version));
    }
    None
}

/// The folder in Zomboid/mods where a kept version of `mod_id` is in use, if one is.
pub fn kept_copy_folder(catalog: &ModCatalog, user_zomboid_dir: &str, mod_id: &str) -> Option<String> {
    let zomboid = zomboid_dir(user_zomboid_dir).ok()?;
    let origins = origins::load(&zomboid);
    catalog.mods.iter().filter(|m| m.id == mod_id && m.source == ModSource::Local).find_map(|m| {
        let folder = Path::new(&m.root_dir).file_name()?.to_string_lossy().to_string();
        match &origins.get(&folder)?.origin {
            ModOrigin::Compat { source_dir, .. } if kept_copy_info(source_dir, &folder).is_some() => Some(folder.clone()),
            _ => None,
        }
    })
}

/// "Back to the Workshop version": the kept version in use goes to the backups and the game reads
/// the Workshop copy again. The caller refuses while the game runs. Returns the folder.
pub fn back_to_workshop(catalog: &ModCatalog, user_zomboid_dir: &str, mod_id: &str) -> Result<String, String> {
    let folder = kept_copy_folder(catalog, user_zomboid_dir, mod_id).ok_or_else(|| format!("{} does not run from a kept version.", mod_id))?;
    crate::compat::remove_compatible_copy(user_zomboid_dir, &folder)?;
    Ok(folder)
}

/// The drive the store is on, and what its free space allows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriveGuard {
    /// "D:".
    pub disk: String,
    pub free_bytes: u64,
    pub total_bytes: u64,
    /// What the Studio always leaves free on it (15% of its size).
    pub reserve_bytes: u64,
    /// The cap that applies: the chosen one, or less when the drive has no more room.
    pub cap_effective: u64,
    /// The drive's free space is what limits the cap.
    pub limited: bool,
}

/// The guard for a store taking `store_bytes`, `protected` of them not droppable, under `cap`.
fn drive_guard(store: &Store, cap: u64, store_bytes: u64, protected: u64) -> Option<DriveGuard> {
    let (free, total) = crate::save_backup::disk_space(&store.root)?;
    let (cap_effective, limited) = store::disk_guard(cap, store_bytes, free, total, protected);
    Some(DriveGuard { disk: crate::disk_usage::disk_name(&store.root), free_bytes: free, total_bytes: total, reserve_bytes: store::drive_reserve(total), cap_effective, limited })
}

/// The store's settings and what its last round found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSummary {
    pub settings: StoreSettings,
    pub status: Option<StoreStatus>,
    /// Where the store is.
    pub folder: String,
    /// Where it is when the player chose none (`Zomboid/PZModStudio_Versions`).
    pub default_folder: String,
    /// The player chose the folder.
    pub folder_chosen: bool,
    /// The drive it is on, with the store's size from the last round.
    pub guard: Option<DriveGuard>,
    /// It is being moved now.
    pub moving: bool,
}

pub fn summary(user_zomboid_dir: &str) -> Result<StoreSummary, String> {
    let zomboid = zomboid_dir(user_zomboid_dir)?;
    let store = Store::at(&zomboid);
    let settings = store.settings();
    let status = store.status();
    let guard = drive_guard(&store, settings.cap_bytes, status.as_ref().map(|s| s.bytes).unwrap_or(0), status.as_ref().map(|s| s.protected_bytes).unwrap_or(0));
    Ok(StoreSummary {
        settings,
        status,
        folder: store.root.to_string_lossy().to_string(),
        default_folder: place::default_root(&zomboid).to_string_lossy().to_string(),
        folder_chosen: place::chosen_root(&zomboid).is_some(),
        guard,
        moving: place::moving(&zomboid),
    })
}

/// Changes the settings; a lower cap applies on the next round (the app's every few minutes). A
/// cap other than the one in force is the player's from then on (a new default leaves it).
pub fn set_settings(user_zomboid_dir: &str, enabled: bool, cap_bytes: u64) -> Result<StoreSummary, String> {
    let zomboid = zomboid_dir(user_zomboid_dir)?;
    let store = Store::at(&zomboid);
    let was = store.settings();
    let cap_bytes = cap_bytes.max(256 << 20);
    let cap_by_player = was.cap_by_player || cap_bytes != was.cap_bytes;
    store.set_settings(&StoreSettings { enabled, cap_bytes, cap_by_player })?;
    summary(user_zomboid_dir)
}

/// A move of the store and the summary after it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderMoved {
    pub moved: place::MoveOutcome,
    pub summary: StoreSummary,
}

/// Moves the store to the folder `folder` gives (that folder when empty, a `PZModStudio_Versions`
/// folder inside it otherwise): copied and checked, then the old one removed (renamed on one
/// drive). Refused while the game runs, while a round runs, and for a drive's root, the Workshop
/// folder, the game's folder, Zomboid/mods and Zomboid/Saves.
pub fn set_folder(paths: &crate::vfs::StudioPaths, folder: &str) -> Result<FolderMoved, String> {
    let zomboid = zomboid_dir(&paths.user_zomboid_dir)?;
    let folder = folder.trim();
    if folder.is_empty() {
        return Err("Choose a folder for the kept versions.".to_string());
    }
    if let Some(pid) = crate::sandbox::find_running_pz_pid() {
        return Err(format!("The game is running (PID {}): the kept versions are moved once it is closed.", pid));
    }
    let moved = place::move_store(&zomboid, Path::new(folder), &place::forbidden_dirs(paths, &zomboid), true)?;
    Ok(FolderMoved { moved, summary: summary(&paths.user_zomboid_dir)? })
}

/// The mod ids switched on in some profile, and those of the active one (with the game's list).
fn used_ids(user_zomboid_dir: &str, mod_list_ini_path: &str) -> (HashSet<String>, HashSet<String>) {
    let clean = |s: &str| s.trim().trim_start_matches('\\').to_string();
    let mut used = HashSet::new();
    let mut active = HashSet::new();
    for inst in crate::instance_manager::list_instances(user_zomboid_dir.to_string()).unwrap_or_default() {
        for id in &inst.active_mod_ids {
            used.insert(clean(id));
            if inst.is_active {
                active.insert(clean(id));
            }
        }
    }
    if let Ok(list) = crate::load_order::ini_parser::read_mod_list_ini(mod_list_ini_path) {
        for id in &list.active_mods {
            used.insert(clean(id));
            active.insert(clean(id));
        }
    }
    (used, active)
}

/// One round of keeping by itself: the version installed now of every Workshop item a profile
/// uses, items Steam has an update waiting for first, then the active profile's, then the smallest.
/// Nothing is done while the game runs or with keeping turned off (`force` keeps anyway, for an
/// agent asking for a round now; the game still stops it).
pub fn keep_round(paths: &crate::vfs::StudioPaths, force: bool) -> Result<StoreStatus, String> {
    let zomboid = zomboid_dir(&paths.user_zomboid_dir)?;
    let settings = Store::at(&zomboid).settings();
    if !settings.enabled && !force {
        return Err("Keeping versions by itself is turned off (Settings).".to_string());
    }
    if crate::sandbox::find_running_pz_pid().is_some() {
        return Err("The game is running: versions are kept once it is closed.".to_string());
    }
    let (store, _lock) = Store::open_locked(&zomboid, 5)?;
    let workshop = Path::new(&paths.workshop_dir);
    let steam = store::read_steam_items(workshop);
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let (used, active) = used_ids(&paths.user_zomboid_dir, &paths.mod_list_ini_path);

    // The Workshop items behind the used ids (the Workshop copy, also when a local one wins).
    let mut items: HashMap<String, String> = HashMap::new(); // id -> first mod's name
    let mut active_items: HashSet<String> = HashSet::new();
    for m in &catalog.mods {
        let ModSource::Workshop { workshop_id } = &m.source else { continue };
        if !used.contains(&m.id) {
            continue;
        }
        items.entry(workshop_id.clone()).or_insert_with(|| m.name.clone());
        if active.contains(&m.id) {
            active_items.insert(workshop_id.clone());
        }
    }
    let mut order: Vec<&String> = items.keys().filter(|id| steam.contains_key(*id)).collect();
    order.sort_by_key(|id| {
        let s = &steam[*id];
        (!s.pending, !active_items.contains(*id), s.size)
    });

    let mut versions = store.versions();
    let mut bytes = store.gc();
    let mut no_room = Vec::new();
    let mut errors = Vec::new();
    let mut last_game_check = std::time::Instant::now();
    for id in order {
        if last_game_check.elapsed().as_secs() >= 10 {
            last_game_check = std::time::Instant::now();
            if crate::sandbox::find_running_pz_pid().is_some() {
                errors.push("The game started: the round stopped and goes on once it is closed.".to_string());
                break;
            }
        }
        let s = &steam[id];
        let Some(ident) = s.ident.clone() else { continue };
        if versions.iter().any(|v| &v.head.item == id && v.head.steam_ident.as_deref() == Some(ident.as_str())) {
            continue;
        }
        if store::steam_busy(workshop, id) {
            continue;
        }
        let previous = versions.iter().find(|v| &v.head.item == id).cloned();
        // A first copy only when it fits, under the cap and the drive's reserve; the copy after an
        // update costs only what it changed, and is left out only once the drive is at its reserve.
        let estimate = s.size / 4 * 3;
        let (cap_now, at_reserve) = match crate::save_backup::disk_space(&store.root) {
            Some((free, total)) => (store::disk_guard(settings.cap_bytes, bytes, free, total, 0).0, free <= store::drive_reserve(total)),
            None => (settings.cap_bytes, false),
        };
        if at_reserve || (previous.is_none() && bytes + estimate > cap_now) {
            no_room.push(items[id].clone());
            continue;
        }
        let root = workshop.join(id);
        let info = KeepInfo { workshop_id: Some(id.clone()), by_player: false, steam_ident: Some(ident.clone()), published: s.published, mods: mods_of_item(&catalog, &paths.workshop_dir, id) };
        match store.keep(id, &root, info, previous.as_ref()) {
            Ok((v, grew)) => {
                // Steam changed the item while it was read: not a version of anything.
                let after = store::read_steam_items(workshop).get(id).and_then(|s| s.ident.clone());
                if after.as_deref() != Some(ident.as_str()) || store::steam_busy(workshop, id) {
                    if !versions.iter().any(|o| o.head_path == v.head_path) {
                        store.drop_version(&v);
                    }
                    continue;
                }
                bytes += grew;
                versions.retain(|o| o.head_path != v.head_path);
                versions.insert(0, v);
            }
            Err(e) => errors.push(format!("{}: {}", items[id], e)),
        }
    }

    let installed: HashMap<String, String> = steam.iter().filter_map(|(id, s)| s.ident.clone().map(|i| (id.clone(), i))).collect();
    let ctx = CapContext { installed: installed.clone(), active_items, in_use: in_use_heads(&zomboid) };
    // The cap, or less when the drive has no more room past its reserve.
    let protected = store.protected_bytes(&ctx.in_use);
    let guard = drive_guard(&store, settings.cap_bytes, store.gc(), protected);
    let cap = guard.as_ref().map(|g| g.cap_effective).unwrap_or(settings.cap_bytes);
    let (dropped, _) = store.enforce_cap(cap, &ctx);
    let bytes = store.gc();
    let versions = store.versions();
    let is_current = |v: &store::StoredVersion| v.head.steam_ident.is_some() && installed.get(&v.head.item) == v.head.steam_ident.as_ref();
    let kept_items: HashSet<&String> = versions.iter().filter(|v| is_current(v) && items.contains_key(&v.head.item)).map(|v| &v.head.item).collect();
    for d in &dropped {
        if is_current_head(d, &installed) && items.contains_key(&d.item) {
            no_room.push(items[&d.item].clone());
        }
    }
    let status = StoreStatus {
        last_run: chrono::Local::now().timestamp(),
        items_used: items.len(),
        items_kept: kept_items.len(),
        no_room,
        bytes,
        versions: versions.len(),
        earlier_versions: versions.iter().filter(|v| !is_current(v)).count(),
        errors,
        cap_effective: cap,
        limited_by_disk: guard.filter(|g| g.limited).map(|g| g.disk),
        protected_bytes: protected,
    };
    store.write_status(&status);
    Ok(status)
}

fn is_current_head(h: &store::VersionHead, installed: &HashMap<String, String>) -> bool {
    h.steam_ident.is_some() && installed.get(&h.item) == h.steam_ident.as_ref()
}

/// What decides whether a round is worth running: Steam's record, the profiles, the game's list
/// and the settings, by size and time.
fn round_key(paths: &crate::vfs::StudioPaths) -> Vec<(u64, u64)> {
    let stamp = |p: &Path| fs::metadata(p).map(|m| (m.len(), m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0))).unwrap_or((0, 0));
    let mut key = Vec::new();
    if let Some(acf) = Path::new(&paths.workshop_dir).parent().and_then(|p| p.parent()).map(|p| p.join("appworkshop_108600.acf")) {
        key.push(stamp(&acf));
    }
    key.push(stamp(Path::new(&paths.mod_list_ini_path)));
    if let Ok(z) = zomboid_dir(&paths.user_zomboid_dir) {
        key.push(stamp(&z.join("mods").join("default.txt")));
        key.push(stamp(&z.join("PZModStudio_VersionsFolder.json")));
        key.push(stamp(&Store::at(&z).root.join("settings.json")));
        if let Ok(dir) = fs::read_dir(z.join("PZModStudio_Instances")) {
            let mut all: Vec<(u64, u64)> = dir.flatten().map(|e| stamp(&e.path())).collect();
            all.sort();
            key.extend(all);
        }
    }
    key
}

/// The app's background keeping: a first round a minute and a half after the start, then one
/// whenever Steam's record, a profile, the game's list or the settings changed (checked every
/// five minutes). Never while the game runs.
pub fn watch_in_background() {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(90));
        let mut last: Option<Vec<(u64, u64)>> = None;
        loop {
            let paths = crate::vfs::studio_paths();
            let key = round_key(&paths);
            let enabled = zomboid_dir(&paths.user_zomboid_dir).map(|z| Store::at(&z).settings().enabled).unwrap_or(false);
            if enabled && last.as_ref() != Some(&key) && crate::sandbox::find_running_pz_pid().is_none() {
                match keep_round(&paths, false) {
                    Ok(s) => {
                        let limit = s.limited_by_disk.as_ref().map(|d| format!(" (limited to {} MB by the free space on {})", s.cap_effective >> 20, d)).unwrap_or_default();
                        crate::diagnostics::log("INFO", &format!("Kept versions: {} of {} used Workshop items kept, {} earlier versions, {} MB{}", s.items_kept, s.items_used, s.earlier_versions, s.bytes >> 20, limit));
                        // A round the game cut short runs again.
                        if s.errors.iter().all(|e| !e.starts_with("The game started")) {
                            last = Some(key);
                        }
                    }
                    Err(e) => crate::diagnostics::log("WARN", &format!("Kept versions: {}", e)),
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(300));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn setup(tag: &str) -> (Tmp, PathBuf, String) {
        let t = Tmp(std::env::temp_dir().join(format!("pzms-kept-{}-{}", tag, std::process::id())));
        let _ = fs::remove_dir_all(&t.0);
        let src = t.0.join("workshop/108600/123/mods/SomeMod");
        fs::create_dir_all(src.join("42/media/lua/shared")).unwrap();
        fs::write(src.join("42/mod.info"), "name=Some Mod\nid=SomeMod\nmodversion=1.2\n").unwrap();
        fs::write(src.join("42/media/lua/shared/a.lua"), "return 1").unwrap();
        fs::create_dir_all(t.0.join("Zomboid/mods")).unwrap();
        let z = t.0.join("Zomboid").to_string_lossy().to_string();
        (t, src, z)
    }

    /// A plain copy as the Studio made them before the store.
    fn legacy_copy(z: &str, src: &Path) -> String {
        let dest = Path::new(z).join(SNAPSHOTS_DIR).join("SomeMod").join("20261017-140500");
        crate::mod_install::copy_tree(src, &dest).unwrap();
        let kept = KeptVersion {
            mod_id: "SomeMod".into(),
            mod_name: "Some Mod".into(),
            folder: "SomeMod".into(),
            kept_at: "2026-10-17 14:05".into(),
            version: Some("1.2".into()),
            workshop_id: Some("123".into()),
            source_dir: src.to_string_lossy().to_string(),
            path: dest.to_string_lossy().to_string(),
            bytes: 8,
            kind: String::new(),
            current: false,
            in_use: false,
            published: None,
        };
        fs::write(dest.join(MANIFEST), serde_json::to_string(&kept).unwrap()).unwrap();
        dest.to_string_lossy().to_string()
    }

    #[test]
    fn a_plain_copy_from_before_the_store_is_listed_put_back_and_deleted() {
        let (t, src, z) = setup("legacy");
        let path = legacy_copy(&z, &src);
        let listed = list_kept_versions(&z, "");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind, "copy");
        assert_eq!(listed[0].version.as_deref(), Some("1.2"));

        // Steam "updates" the original; the kept copy is what it was.
        fs::write(src.join("42/media/lua/shared/a.lua"), "return 2").unwrap();
        let back = restore_kept_version(&z, &path).expect("put back");
        let local = t.0.join("Zomboid/mods/SomeMod");
        assert_eq!(fs::read_to_string(local.join("42/media/lua/shared/a.lua")).unwrap(), "return 1");
        assert!(!local.join(MANIFEST).exists(), "the manifest does not travel into the mod");
        assert_eq!(back.mod_id, "SomeMod");
        let origin = origins::load(&t.0.join("Zomboid")).get("SomeMod").map(|r| r.origin.clone());
        assert!(matches!(&origin, Some(ModOrigin::Compat { source_dir, .. }) if kept_copy_info(source_dir, "SomeMod").is_some()), "{:?}", origin);
        assert!(crate::compat::local_first(&t.0.join("Zomboid")), "the Studio launches with Zomboid/mods first");
        assert!(list_kept_versions(&z, "")[0].in_use);

        // Putting it back twice moves the first copy aside instead of failing.
        restore_kept_version(&z, &path).expect("put back again");

        delete_kept_version(&z, &path).expect("deleted");
        assert!(list_kept_versions(&z, "").is_empty());
        assert!(!t.0.join("Zomboid").join(SNAPSHOTS_DIR).join("SomeMod").exists());
    }

    #[test]
    fn a_stored_version_is_put_back_as_a_local_copy_and_taken_out_again() {
        let (t, src, z) = setup("store");
        let zomboid = t.0.join("Zomboid");
        let store = Store::at(&zomboid);
        let item = t.0.join("workshop/108600/123");
        let info = KeepInfo {
            workshop_id: Some("123".into()),
            by_player: false,
            steam_ident: Some("m1".into()),
            published: Some(1_790_000_000),
            mods: vec![VersionMod { id: "SomeMod".into(), name: "Some Mod".into(), folder: "SomeMod".into(), rel: "mods/SomeMod".into(), version: Some("1.2".into()), bytes: 0 }],
        };
        store.keep("123", &item, info, None).unwrap();
        let listed = list_kept_versions(&z, "");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind, "auto");
        assert!(listed[0].bytes > 0);
        assert!(listed[0].published.is_some());

        // Steam replaces the mod with an update; going back gives the kept files.
        fs::write(src.join("42/media/lua/shared/a.lua"), "return 2 -- broken").unwrap();
        let back = restore_kept_version(&z, &listed[0].path).expect("put back");
        assert!(back.in_use);
        let local = zomboid.join("mods/SomeMod");
        assert_eq!(fs::read_to_string(local.join("42/media/lua/shared/a.lua")).unwrap(), "return 1");
        assert_eq!(fs::read_to_string(local.join("42/mod.info")).unwrap(), "name=Some Mod\nid=SomeMod\nmodversion=1.2\n");
        assert_eq!(fs::read_to_string(src.join("42/media/lua/shared/a.lua")).unwrap(), "return 2 -- broken", "the Workshop folder is not touched");
        assert!(crate::compat::local_first(&zomboid));
        let listed = list_kept_versions(&z, "");
        assert!(listed[0].in_use);
        // In use: the cap leaves it.
        let ctx = CapContext { in_use: in_use_heads(&zomboid), ..Default::default() };
        assert!(store::eviction_order(&store.versions(), &ctx).is_empty());

        // Back to the Workshop version: the copy goes to the backups, the record with it.
        crate::compat::remove_compatible_copy(&z, "SomeMod").unwrap();
        assert!(!local.exists());
        assert!(!list_kept_versions(&z, "")[0].in_use);
        assert!(zomboid.join("PZModStudio_Backups/mods").read_dir().unwrap().next().is_some());

        // A folder the Studio did not put there is not replaced.
        fs::create_dir_all(&local).unwrap();
        assert!(restore_kept_version(&z, &listed[0].path).is_err());

        delete_kept_version(&z, &listed[0].path).unwrap();
        assert!(list_kept_versions(&z, "").is_empty());
    }

    #[test]
    fn the_switch_keeps_the_default_cap_and_a_new_cap_is_the_players() {
        let (_t, _src, z) = setup("settings");
        let ten = store::DEFAULT_CAP_BYTES;
        // Switching off with the cap shown leaves it the default's.
        let s = set_settings(&z, false, ten).unwrap();
        assert!(!s.settings.enabled && s.settings.cap_bytes == ten && !s.settings.cap_by_player);
        assert!(!s.folder_chosen && s.folder == s.default_folder);
        // Another cap is the player's from then on.
        let s = set_settings(&z, true, 20 << 30).unwrap();
        assert!(s.settings.cap_by_player && s.settings.cap_bytes == 20 << 30);
        let s = set_settings(&z, true, ten).unwrap();
        assert!(s.settings.cap_by_player, "10 GB chosen by hand stays the player's");
    }

    #[test]
    fn only_the_studios_own_kept_versions_are_touched() {
        let (t, _src, z) = setup("outside");
        let elsewhere = t.0.join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        assert!(restore_kept_version(&z, &elsewhere.to_string_lossy()).is_err());
        assert!(delete_kept_version(&z, &elsewhere.to_string_lossy()).is_err());
        let fake = format!("{}{}SomeMod", elsewhere.join("x.json").to_string_lossy(), PATH_SEP);
        fs::write(elsewhere.join("x.json"), "{}").unwrap();
        assert!(restore_kept_version(&z, &fake).is_err());
        assert!(delete_kept_version(&z, &fake).is_err());
        assert!(elsewhere.is_dir());
    }

    /// Moves this PC's store to `PZMS_STORE_DIR` with the code the app's "Choose folder" runs (real
    /// paths; refused while the game runs). Run by hand:
    /// `PZMS_STORE_DIR=D:\PZModStudio_Versions cargo test --lib this_pc_store_folder -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn this_pc_store_folder() {
        let dir = std::env::var("PZMS_STORE_DIR").expect("PZMS_STORE_DIR not set");
        let paths = crate::vfs::studio_paths();
        let before = summary(&paths.user_zomboid_dir).expect("summary");
        println!("before: {} (chosen: {}), {} versions", before.folder, before.folder_chosen, list_kept_versions(&paths.user_zomboid_dir, &paths.workshop_dir).len());
        let out = set_folder(&paths, &dir).expect("moved");
        println!("moved: {:?}", out.moved);
        let after = summary(&paths.user_zomboid_dir).unwrap();
        println!("after: {} (chosen: {}), guard {:?}, settings {:?}", after.folder, after.folder_chosen, after.guard, after.settings);
        assert!(place::same_path(Path::new(&after.folder), Path::new(&out.moved.to)));
        assert!(Path::new(&after.folder).is_dir());
        assert!(!after.moving);
    }

    /// A round on this PC's real mods, into a scratch Zomboid folder (the profiles and the game's
    /// list are copied there; the Workshop folder is only read). Run by hand:
    /// `PZMS_REAL_WORKSHOP=... PZMS_REAL_ZOMBOID=... PZMS_REAL_OUT=... cargo test --lib real_pc_round -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_pc_round() {
        let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("{} not set", k));
        let (workshop, real, out) = (var("PZMS_REAL_WORKSHOP"), PathBuf::from(var("PZMS_REAL_ZOMBOID")), PathBuf::from(var("PZMS_REAL_OUT")));
        let z = out.join("Zomboid");
        fs::create_dir_all(z.join("mods")).unwrap();
        fs::create_dir_all(z.join("PZModStudio_Instances")).unwrap();
        for e in fs::read_dir(real.join("PZModStudio_Instances")).unwrap().flatten() {
            if e.path().extension().is_some_and(|x| x == "json") {
                fs::copy(e.path(), z.join("PZModStudio_Instances").join(e.file_name())).unwrap();
            }
        }
        for f in ["default.txt", "ModListData.ini"] {
            let _ = fs::copy(real.join("mods").join(f), z.join("mods").join(f));
        }
        let _ = fs::copy(real.join("version.txt"), z.join("version.txt"));
        if let Ok(cap) = std::env::var("PZMS_REAL_CAP_MB") {
            Store::at(&z).set_settings(&StoreSettings { enabled: true, cap_bytes: cap.parse::<u64>().unwrap() << 20, cap_by_player: true }).unwrap();
        }
        let paths = crate::vfs::StudioPaths {
            pz_install_dir: String::new(),
            workshop_dir: workshop.clone(),
            user_zomboid_dir: z.to_string_lossy().to_string(),
            mod_list_ini_path: z.join("mods/ModListData.ini").to_string_lossy().to_string(),
            carrier_workshop_id: None,
            is_valid: true,
        };
        let started = std::time::Instant::now();
        let status = keep_round(&paths, true).expect("round");
        println!("first round: {:?} in {:.0} s", status, started.elapsed().as_secs_f64());
        let started = std::time::Instant::now();
        let again = keep_round(&paths, true).expect("round 2");
        println!("second round (nothing changed): {} versions, {} MB, in {:.1} s", again.versions, again.bytes >> 20, started.elapsed().as_secs_f64());
        assert_eq!(again.versions, status.versions, "nothing new when Steam changed nothing");

        // One real mod back as a local copy, byte for byte.
        let store = Store::at(&z);
        let v = store.versions().into_iter().find(|v| v.head.bytes > 200_000 && v.head.mods.len() == 1).expect("a kept item");
        let m = v.head.mods[0].clone();
        let dest = z.join("mods").join(&m.folder);
        store.restore_mod(&v, &m.folder, &dest).unwrap();
        let src = Path::new(&v.head.source_dir).join(m.rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        let mut files = 0;
        for e in walkdir::WalkDir::new(&src).into_iter().flatten().filter(|e| e.file_type().is_file()) {
            let rel = e.path().strip_prefix(&src).unwrap();
            assert_eq!(fs::read(e.path()).unwrap(), fs::read(dest.join(rel)).unwrap(), "{}", rel.display());
            files += 1;
        }
        println!("put back {} ({} files, {} KB): identical to the Workshop copy", m.id, files, m.bytes >> 10);

        // An "update" of that item: a copy with one Lua file changed costs about that file.
        let fake = out.join("update").join(&v.head.item);
        crate::mod_install::copy_tree(Path::new(&v.head.source_dir), &fake).unwrap();
        let lua = walkdir::WalkDir::new(&fake).into_iter().flatten().find(|e| e.path().extension().is_some_and(|x| x == "lua")).expect("a lua file");
        let mut text = fs::read_to_string(lua.path()).unwrap_or_default();
        text.push_str("\n-- changed by an update\n");
        fs::write(lua.path(), text).unwrap();
        let before = store.gc();
        // The copy has new times: every file is read again, but only the changed one is new.
        let (v2, grew) = store.keep(&v.head.item, &fake, KeepInfo { steam_ident: Some("update".into()), mods: v.head.mods.clone(), ..Default::default() }, Some(&v)).unwrap();
        println!("update of {} ({} KB): the new version added {} bytes (store {} -> {} bytes)", m.id, v.head.bytes >> 10, grew, before, store.gc());
        assert!(grew < 64 * 1024, "{}", grew);
        store.drop_version(&v2);
        store.gc();
    }
}
