//! Disk use: what every installed mod takes on the disk, and which of them nothing uses.
//!
//! A mod is in use when a profile has it (on, or among its members), the game's active list has
//! it, a save's own list (`mods.txt`) names it, or a kept server loaded it the last time it was
//! joined. Anything else only takes room. The unit of room is what can be given back as one: a
//! subscribed Workshop item (all the mods inside it), a folder in `Zomboid/mods`, and a Workshop
//! folder Steam left behind after an unsubscribe. A Workshop item goes by unsubscribing (Steam
//! removes its files); a local folder or a leftover goes to the Recycle Bin, so it can come back.
//! The player's own uploads (`Zomboid/Workshop`) are listed, never offered for removal.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::catalog::{ModCatalog, ModSource};

#[derive(Debug, Clone, Default, Serialize)]
pub struct UsedBy {
    /// Profiles that have it on.
    pub profiles: Vec<String>,
    /// Profiles that keep it among their mods, switched off.
    pub profiles_off: Vec<String>,
    /// Save ids (`<mode>/<world>`).
    pub saves: Vec<String>,
    /// Kept servers' names.
    pub servers: Vec<String>,
    /// On in the game's active list (`default.txt`).
    pub active: bool,
}

impl UsedBy {
    pub fn is_empty(&self) -> bool {
        self.profiles_off.is_empty() && self.is_idle()
    }

    /// Nothing plays with it: on nowhere, in no save, on no server (a profile may keep it off).
    pub fn is_idle(&self) -> bool {
        self.profiles.is_empty() && self.saves.is_empty() && self.servers.is_empty() && !self.active
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskUnit {
    /// `workshop` (subscribed item), `local` (Zomboid/mods), `upload` (the player's own Workshop
    /// upload) or `leftover` (a Workshop folder that is no longer subscribed).
    pub kind: String,
    pub workshop_id: Option<String>,
    pub folder: String,
    /// The first mod's name, or the folder's name.
    pub name: String,
    pub mod_ids: Vec<String>,
    pub bytes: u64,
    pub files: u64,
    pub used_by: UsedBy,
    pub unused: bool,
    /// Only kept switched off by some profile: nothing plays with it.
    pub idle: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StudioFolder {
    /// The folder's name in `Zomboid` (`PZModStudio_SaveBackups`, `Logs`...), or the folder's name.
    pub name: String,
    /// What it is, for the UI's words: `game` (the game's install), `workshop` (the subscribed
    /// items), `mods` (`Zomboid/mods`), `saves` (`Zomboid/Saves`), or the name for the rest.
    pub kind: String,
    pub path: String,
    pub bytes: u64,
}

/// One disk among those holding the game, the Workshop items and the Zomboid folder.
#[derive(Debug, Clone, Serialize)]
pub struct DiskSpace {
    /// `C:` on Windows, the mount otherwise.
    pub name: String,
    pub free_bytes: u64,
    pub total_bytes: u64,
    /// What is on it: `game`, `workshop`, `zomboid`.
    pub holds: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DiskReport {
    /// Biggest first.
    pub units: Vec<DiskUnit>,
    pub folders: Vec<StudioFolder>,
    pub mods_bytes: u64,
    pub unused_bytes: u64,
    pub unused_count: usize,
    pub idle_bytes: u64,
    pub idle_count: usize,
    /// The disk holding the Workshop folder (or Zomboid when there is none).
    pub free_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    /// Every disk the game, the Workshop items and Zomboid are on, each named, once.
    pub disks: Vec<DiskSpace>,
}

/// The disk a path is on, by name: `D:` on Windows, `/` or the mount elsewhere.
fn disk_name(path: &Path) -> String {
    let root = path.ancestors().last().map(|r| r.to_string_lossy().to_string()).unwrap_or_default();
    let name = root.trim_end_matches(['\\', '/']);
    if name.is_empty() { root } else { name.to_string() }
}

/// Bytes and files under `dir`.
fn size_of(dir: &Path) -> (u64, u64) {
    let mut bytes = 0;
    let mut files = 0;
    for entry in walkdir::WalkDir::new(dir).follow_links(false).into_iter().flatten() {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                bytes += meta.len();
                files += 1;
            }
        }
    }
    (bytes, files)
}

fn plain(id: &str) -> String {
    let id = crate::load_order::mod_info::sanitize_mod_id(id);
    id.rsplit('/').next().unwrap_or_default().to_string()
}

/// Who uses each mod id (plain ids).
fn uses(user_zomboid_dir: &Path) -> BTreeMap<String, UsedBy> {
    let mut out: BTreeMap<String, UsedBy> = BTreeMap::new();
    let profiles = crate::instance_manager::list_instances(user_zomboid_dir.to_string_lossy().to_string()).unwrap_or_default();
    for p in &profiles {
        let on: BTreeSet<String> = p.active_mod_ids.iter().map(|m| plain(m)).collect();
        let off: BTreeSet<String> = p.members.iter().flatten().map(|m| plain(m)).filter(|m| !on.contains(m)).collect();
        for id in on {
            out.entry(id).or_default().profiles.push(p.name.clone());
        }
        for id in off {
            out.entry(id).or_default().profiles_off.push(p.name.clone());
        }
    }
    for id in crate::load_order::ini_parser::parse_default_txt(&user_zomboid_dir.join("mods").join("default.txt")) {
        out.entry(plain(&id)).or_default().active = true;
    }
    for save in crate::save_backup::list_saves(user_zomboid_dir) {
        let ids: BTreeSet<String> = crate::save_backup::save_mods(&crate::save_backup::saves_dir(user_zomboid_dir).join(&save)).iter().map(|m| plain(m)).collect();
        for id in ids {
            out.entry(id).or_default().saves.push(save.clone());
        }
    }
    for server in crate::server_browser::read_saved(user_zomboid_dir) {
        let ids: BTreeSet<String> = server.mods.iter().map(|m| plain(m)).collect();
        for id in ids {
            out.entry(id).or_default().servers.push(server.name.clone());
        }
    }
    out
}

fn merge(into: &mut UsedBy, from: &UsedBy) {
    for (a, b) in [(&mut into.profiles, &from.profiles), (&mut into.profiles_off, &from.profiles_off), (&mut into.saves, &from.saves), (&mut into.servers, &from.servers)] {
        for x in b {
            if !a.contains(x) {
                a.push(x.clone());
            }
        }
    }
    into.active |= from.active;
}

pub fn report(pz_install_dir: &str, workshop_dir: &str, user_zomboid_dir: &Path, catalog: &ModCatalog) -> DiskReport {
    let used = uses(user_zomboid_dir);
    // One unit per folder that goes as one: a Workshop item, a folder in Zomboid/mods, an upload.
    let mut units: BTreeMap<String, DiskUnit> = BTreeMap::new();
    for m in &catalog.mods {
        let root = PathBuf::from(&m.root_dir);
        let (kind, workshop_id, folder) = match &m.source {
            // <workshop>/<item>/mods/<mod>: the item folder.
            ModSource::Workshop { workshop_id } => ("workshop", Some(workshop_id.clone()), root.parent().and_then(Path::parent).map(Path::to_path_buf)),
            // <Zomboid>/Workshop/<item>/Contents/mods/<mod>: the item folder.
            ModSource::WorkshopUpload { .. } => ("upload", None, root.parent().and_then(Path::parent).and_then(Path::parent).map(Path::to_path_buf)),
            ModSource::Local => ("local", None, Some(root.clone())),
        };
        let Some(folder) = folder else { continue };
        let key = folder.to_string_lossy().to_string();
        let unit = units.entry(key.clone()).or_insert_with(|| DiskUnit {
            kind: kind.to_string(),
            workshop_id,
            folder: key,
            name: m.name.clone(),
            mod_ids: Vec::new(),
            bytes: 0,
            files: 0,
            used_by: UsedBy::default(),
            unused: false,
            idle: false,
        });
        if !unit.mod_ids.contains(&m.id) {
            unit.mod_ids.push(m.id.clone());
        }
        if let Some(u) = used.get(&plain(&m.id)) {
            merge(&mut unit.used_by, u);
        }
    }
    // Workshop folders that are no longer subscribed: Steam sometimes leaves them behind.
    let workshop = Path::new(workshop_dir);
    if !workshop_dir.is_empty() {
        if let Some(subscribed) = crate::load_order::mod_info::get_subscribed_workshop_ids(workshop) {
            for entry in std::fs::read_dir(workshop).into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !entry.path().is_dir() || name.is_empty() || !name.chars().all(|c| c.is_ascii_digit()) || subscribed.contains(&name) {
                    continue;
                }
                let key = entry.path().to_string_lossy().to_string();
                units.entry(key.clone()).or_insert_with(|| DiskUnit {
                    kind: "leftover".into(),
                    workshop_id: Some(name.clone()),
                    folder: key,
                    name: name.clone(),
                    mod_ids: Vec::new(),
                    bytes: 0,
                    files: 0,
                    used_by: UsedBy::default(),
                    unused: true,
                    idle: false,
                });
            }
        }
    }
    let mut units: Vec<DiskUnit> = units.into_values().collect();
    for unit in &mut units {
        let (bytes, files) = size_of(Path::new(&unit.folder));
        unit.bytes = bytes;
        unit.files = files;
        unit.unused = unit.kind != "upload" && unit.used_by.is_empty();
        unit.idle = unit.kind != "upload" && !unit.unused && unit.used_by.is_idle();
    }
    units.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));

    let mut folders: Vec<StudioFolder> = std::fs::read_dir(user_zomboid_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("PZModStudio_") || n == "Logs"
        })
        .map(|e| StudioFolder { name: e.file_name().to_string_lossy().to_string(), kind: e.file_name().to_string_lossy().to_string(), path: e.path().to_string_lossy().to_string(), bytes: size_of(&e.path()).0 })
        .filter(|f| f.bytes > 0)
        .collect();
    // The game's own folders too: the install, the Workshop items, the local mods, the saves.
    let game = Path::new(pz_install_dir);
    let big: [(&str, &Path); 4] = [("game", game), ("workshop", workshop), ("mods", &user_zomboid_dir.join("mods")), ("saves", &user_zomboid_dir.join("Saves"))];
    for (kind, dir) in big {
        if dir.as_os_str().is_empty() || !dir.is_dir() {
            continue;
        }
        let bytes = size_of(dir).0;
        if bytes > 0 {
            folders.push(StudioFolder { name: dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), kind: kind.to_string(), path: dir.to_string_lossy().to_string(), bytes });
        }
    }
    folders.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    // The disks, each once, with what is on it.
    let mut disks: Vec<DiskSpace> = Vec::new();
    for (what, dir) in [("game", game), ("workshop", workshop), ("zomboid", user_zomboid_dir)] {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let name = disk_name(dir);
        if let Some(d) = disks.iter_mut().find(|d| d.name.eq_ignore_ascii_case(&name)) {
            d.holds.push(what.to_string());
            continue;
        }
        if let Some((free, total)) = crate::save_backup::disk_space(dir) {
            disks.push(DiskSpace { name, free_bytes: free, total_bytes: total, holds: vec![what.to_string()] });
        }
    }

    let space = crate::save_backup::disk_space(if workshop_dir.is_empty() { user_zomboid_dir } else { workshop });
    DiskReport {
        mods_bytes: units.iter().map(|u| u.bytes).sum(),
        unused_bytes: units.iter().filter(|u| u.unused).map(|u| u.bytes).sum(),
        unused_count: units.iter().filter(|u| u.unused).count(),
        idle_bytes: units.iter().filter(|u| u.idle).map(|u| u.bytes).sum(),
        idle_count: units.iter().filter(|u| u.idle).count(),
        units,
        folders,
        free_bytes: space.map(|s| s.0),
        total_bytes: space.map(|s| s.1),
        disks,
    }
}

/// Sends an unused or idle local mod folder, or a leftover Workshop folder, to the Recycle Bin.
/// Only a folder this report lists as such, of those two kinds, and never while the game runs.
pub fn recycle(report: &DiskReport, folder: &str, game_running: bool) -> Result<(), String> {
    if game_running {
        return Err("Close Project Zomboid first: the game may be reading that folder.".to_string());
    }
    let unit = report
        .units
        .iter()
        .find(|u| u.folder == folder)
        .ok_or_else(|| format!("{} is not a mod folder the Studio knows.", folder))?;
    if !(unit.unused || unit.idle) || !(unit.kind == "local" || unit.kind == "leftover") {
        return Err(format!("{} is in use or is not a folder the Studio removes.", unit.name));
    }
    to_recycle_bin(Path::new(&unit.folder))
}

#[cfg(windows)]
pub(crate) fn to_recycle_bin(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::{SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW};
    if !path.is_dir() {
        return Err(format!("{} is not there any more.", path.display()));
    }
    // A list of paths, each ended by a NUL, the list by another.
    let mut from: Vec<u16> = path.as_os_str().encode_wide().collect();
    from.push(0);
    from.push(0);
    let mut op: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
    op.wFunc = FO_DELETE;
    op.pFrom = from.as_ptr();
    op.fFlags = (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT) as u16;
    let result = unsafe { SHFileOperationW(&mut op) };
    if result != 0 || op.fAnyOperationsAborted != 0 {
        return Err(format!("Windows could not move {} to the Recycle Bin (code {}).", path.display(), result));
    }
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn to_recycle_bin(_path: &Path) -> Result<(), String> {
    Err("The Recycle Bin is only used on Windows.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pzms-disk-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_disk_is_named_by_its_root() {
        assert_eq!(disk_name(Path::new("D:\\Users\\x\\Zomboid")), "D:");
        assert_eq!(disk_name(Path::new("/home/x/Zomboid")), "/");
    }

    #[test]
    fn a_mod_nothing_uses_is_unused_and_one_a_save_or_profile_names_is_not() {
        let z = temp("uses");
        let workshop = z.join("steamapps").join("workshop").join("content").join("108600");
        for (item, id) in [("111", "Used"), ("222", "Spare"), ("333", "InSave"), ("555", "KeptOff")] {
            let dir = workshop.join(item).join("mods").join(id);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("mod.info"), format!("name={}\nid={}\n", id, id)).unwrap();
            std::fs::write(dir.join("big.bin"), vec![0u8; 1000]).unwrap();
        }
        // Steam's record of subscriptions: 444 is a folder left after an unsubscribe.
        std::fs::create_dir_all(workshop.join("444").join("mods")).unwrap();
        std::fs::write(workshop.join("444").join("x.bin"), vec![0u8; 50]).unwrap();
        std::fs::write(
            workshop.parent().unwrap().parent().unwrap().join("appworkshop_108600.acf"),
            "\"AppWorkshop\"\n{\n\t\"WorkshopItemsInstalled\"\n\t{\n\t\t\"111\"\n\t\t{\n\t\t}\n\t\t\"222\"\n\t\t{\n\t\t}\n\t\t\"333\"\n\t\t{\n\t\t}\n\t\t\"555\"\n\t\t{\n\t\t}\n\t}\n}\n",
        )
        .unwrap();
        let zomboid = z.join("Zomboid");
        std::fs::create_dir_all(zomboid.join("PZModStudio_Instances")).unwrap();
        std::fs::write(
            zomboid.join("PZModStudio_Instances").join("inst_1.json"),
            r#"{"id":"inst_1","name":"Main","created_at":"0","is_active":true,"active_mod_ids":["Used"],"load_order":["Used"],"members":["Used","KeptOff"]}"#,
        )
        .unwrap();
        let save = zomboid.join("Saves").join("Sandbox").join("W");
        std::fs::create_dir_all(&save).unwrap();
        std::fs::write(save.join("mods.txt"), "VERSION = 1,\nmods\n{\n    mod = 333/InSave,\n}\n").unwrap();
        let wd = workshop.to_string_lossy().to_string();
        let catalog = crate::catalog::build_catalog(&wd, &zomboid.to_string_lossy(), Some(42_021));
        let r = report("", &wd, &zomboid, &catalog);
        let unit = |id: &str| r.units.iter().find(|u| u.workshop_id.as_deref() == Some(id)).unwrap();
        assert!(!unit("111").unused);
        assert_eq!(unit("111").used_by.profiles, vec!["Main".to_string()]);
        assert!(unit("222").unused);
        assert!(!unit("333").unused);
        assert_eq!(unit("333").used_by.saves, vec!["Sandbox/W".to_string()]);
        assert_eq!(unit("444").kind, "leftover");
        assert!(unit("444").unused);
        assert_eq!(r.unused_count, 2);
        assert!(unit("555").idle && !unit("555").unused);
        assert_eq!(unit("555").used_by.profiles_off, vec!["Main".to_string()]);
        assert!(!unit("111").idle);
        assert_eq!(r.idle_count, 1);
        assert!(unit("222").bytes >= 1000);
        // The game's folders are listed by kind, and the disk is named.
        assert!(r.folders.iter().any(|f| f.kind == "workshop" && f.bytes > 0));
        assert!(r.folders.iter().any(|f| f.kind == "saves"));
        assert!(r.disks.iter().any(|d| d.holds.contains(&"zomboid".to_string()) && !d.name.is_empty()));
        // In use, or not a local/leftover folder: never removed.
        assert!(recycle(&r, &unit("111").folder, false).is_err());
        assert!(recycle(&r, &unit("222").folder, false).is_err());
        assert!(recycle(&r, &unit("444").folder, true).is_err());
        let _ = std::fs::remove_dir_all(&z);
    }

    /// Live, read only: `cargo test --lib -- --ignored live_disk_report --nocapture`.
    #[test]
    #[ignore]
    fn live_disk_report() {
        let paths = crate::vfs::studio_paths();
        let z = PathBuf::from(&paths.user_zomboid_dir);
        let started = std::time::Instant::now();
        let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
        let r = report(&paths.pz_install_dir, &paths.workshop_dir, &z, &catalog);
        println!("{} units, {} MB, unused {} ({} MB), idle {} ({} MB) in {:?}", r.units.len(), r.mods_bytes >> 20, r.unused_count, r.unused_bytes >> 20, r.idle_count, r.idle_bytes >> 20, started.elapsed());
        for u in r.units.iter().filter(|u| u.unused || u.idle).take(25) {
            println!("  unused {:>8} MB {:9} {} {:?}", u.bytes >> 20, u.kind, u.name, u.workshop_id);
        }
        for f in &r.folders {
            println!("  folder {:>8} MB {}", f.bytes >> 20, f.name);
        }
    }
}
