//! A mod that is no longer on disk is no longer in the app.
//!
//! A mod is on disk (on or off) or it is not there at all. When its folder goes (unsubscribed on
//! Steam, deleted by hand), the profiles still named it, on or off, and so did the game's list:
//! the Mod List showed it as "not installed" and the profile cards counted it. After each scan the
//! ids no folder has any more are taken out of every profile, of the game's list, of the Studio's
//! saved order, of each server's `Mods=` (with its `WorkshopItems=` entry when no mod left there
//! comes from that item) and of each save's own list (`mods.txt`), with a backup of each file
//! changed. A server's `.ini` is left alone while that server runs, and the saves while the game
//! runs.
//!
//! Never when the scan may simply have missed them: the Workshop folder or `Zomboid/mods` not
//! there, nothing found at all, or more ids missing than half of what is installed (a wrong
//! folder in Settings looks like that). Never the Studio's own mods, nor an id that still has a
//! folder or link of its name in `Zomboid/mods` (a mod in development whose link points at a
//! drive that is not there now), nor a mod that is on and whose author gave it another id.
//!
//! A Workshop item the player unsubscribed from counts as gone at once, though Steam keeps its
//! folder for a while: the scan leaves it out (see `unsubscribed`). The Mod List's groups lose
//! the ids too; the player's notes are kept.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::load_order::ini_parser::{read_mod_list_ini, write_file_with_backup, write_mod_list_ini};
use crate::load_order::mod_info::{match_installed_key, sanitize_mod_id, ModManifest};
use crate::vfs::StudioPaths;

/// The ids taken out by the last scans, for the app to say and to read the profiles again; true
/// for a mod of a Workshop item the player unsubscribed from.
static FORGOTTEN: std::sync::Mutex<Vec<(String, bool)>> = std::sync::Mutex::new(Vec::new());

/// The ids forgotten since the app last asked.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Forgotten {
    /// Mods of Workshop items the player unsubscribed from.
    pub unsubscribed: Vec<String>,
    /// Mods whose folder is no longer on disk for another reason (deleted by hand, Steam removed it).
    pub gone: Vec<String>,
}

/// What the app takes after a scan: the ids forgotten since it last asked.
pub fn take_forgotten() -> Forgotten {
    let all = FORGOTTEN.lock().map(|mut f| std::mem::take(&mut *f)).unwrap_or_default();
    let mut out = Forgotten::default();
    for (id, unsubscribed) in all {
        if unsubscribed { out.unsubscribed.push(id) } else { out.gone.push(id) }
    }
    out
}

/// The Studio's own mods: made, removed and switched by the Studio itself.
fn studio_own(id: &str) -> bool {
    id.starts_with("Z_PZModStudio_") || id == "Z_StudioLink" || id == "PZModStudio_Profiler"
}

/// The mod id of a list entry: `ModId`, `\ModId` (B42), or `<workshopId>/<ModId>` and
/// `<workshopId>\<ModId>`, the forms newer B42 lists use for mods that share an id.
fn plain_id(raw: &str) -> String {
    let id = sanitize_mod_id(raw);
    for sep in ['/', '\\'] {
        if let Some((ws, rest)) = id.split_once(sep) {
            if !ws.is_empty() && ws.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
                return rest.to_string();
            }
        }
    }
    id
}

/// How an entry is compared with the ids taken out.
fn key(raw: &str) -> String {
    plain_id(raw).to_lowercase()
}

/// The ids among `named` that no scanned mod has. `installed` is the scan; `mods_dir` is
/// `Zomboid/mods`, whose entries keep an id that still has a folder or link there.
pub fn missing_ids<'a>(named: impl IntoIterator<Item = &'a String>, installed: &[ModManifest], mods_dir: &Path) -> Vec<String> {
    let on_disk: HashMap<String, ()> = installed.iter().filter(|m| !m.not_installed).map(|m| (m.id.clone(), ())).collect();
    let folders: HashSet<String> = std::fs::read_dir(mods_dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_lowercase()).collect())
        .unwrap_or_default();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for raw in named {
        let full = sanitize_mod_id(raw);
        let id = plain_id(raw);
        if id.is_empty() || studio_own(&id) || !seen.insert(id.to_lowercase()) {
            continue;
        }
        let here = |x: &str| match_installed_key(&on_disk, x).is_some() || folders.contains(&x.to_lowercase());
        // A mod whose own id has a slash in it ("2858201402/dailystatisticsandgains") is there by its full id.
        if here(&full) || here(&id) {
            continue;
        }
        out.push(id);
    }
    out
}

/// Whether the scan can be trusted to say a mod is gone.
fn scan_is_whole(paths: &StudioPaths, installed: &[ModManifest], missing: usize) -> bool {
    let z = Path::new(&paths.user_zomboid_dir);
    if paths.user_zomboid_dir.is_empty() || !z.join("mods").is_dir() {
        return false;
    }
    if !paths.workshop_dir.is_empty() {
        let has_items = std::fs::read_dir(&paths.workshop_dir).map(|mut rd| rd.next().is_some()).unwrap_or(false);
        if !has_items {
            return false;
        }
    }
    let count = installed.iter().filter(|m| !m.not_installed).count();
    count > 0 && missing <= count / 2
}

/// Takes the ids in `gone` out of a list.
fn without(list: &[serde_json::Value], gone: &HashSet<String>) -> Option<Vec<serde_json::Value>> {
    let kept: Vec<serde_json::Value> = list
        .iter()
        .filter(|v| v.as_str().map(|s| !gone.contains(&key(s))).unwrap_or(true))
        .cloned()
        .collect();
    (kept.len() != list.len()).then_some(kept)
}

/// Takes out of a JSON file's lists (`keys`) the ids in `gone`; backed up when changed.
fn prune_json(path: &Path, keys: &[&str], gone: &HashSet<String>, backups: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else { return false };
    let Ok(mut doc) = serde_json::from_str::<serde_json::Value>(&text) else { return false };
    let mut changed = false;
    for k in keys {
        if let Some(list) = doc.get(*k).and_then(|v| v.as_array()) {
            if let Some(kept) = without(list, gone) {
                doc[*k] = serde_json::Value::Array(kept);
                changed = true;
            }
        }
    }
    changed && write_file_with_backup(path, &serde_json::to_string_pretty(&doc).unwrap_or_default(), backups).is_ok()
}

/// A server's `.ini` without the ids in `gone` in its `Mods=`, and without the `WorkshopItems=`
/// entries those mods came from (`item_of`) when no mod left in `Mods=` comes from the same item.
/// Every other line is kept as it was. None when nothing changes.
fn server_ini_without(content: &str, gone: &HashSet<String>, item_of: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let value_of = |line: &str, k: &str| line.trim_start().strip_prefix(&format!("{}=", k)).map(|v| v.to_string());
    let mods_value = content.lines().find_map(|l| value_of(l, "Mods"))?;
    let tokens: Vec<&str> = mods_value.split(';').collect();
    let (kept, dropped): (Vec<&str>, Vec<&str>) = tokens.iter().partition(|t| t.trim().is_empty() || !gone.contains(&key(t)));
    if dropped.is_empty() {
        return None;
    }
    let kept_items: HashSet<String> = kept.iter().filter_map(|t| item_of(&plain_id(t))).collect();
    let dropped_items: HashSet<String> = dropped.iter().filter_map(|t| item_of(&plain_id(t))).filter(|i| !kept_items.contains(i)).collect();

    let mut out = String::with_capacity(content.len());
    for piece in content.split_inclusive('\n') {
        let ending = if piece.ends_with("\r\n") { "\r\n" } else if piece.ends_with('\n') { "\n" } else { "" };
        let line = &piece[..piece.len() - ending.len()];
        if value_of(line, "Mods").is_some() {
            out.push_str(&format!("Mods={}", kept.join(";")));
        } else if let Some(items) = value_of(line, "WorkshopItems") {
            let left: Vec<&str> = items.split(';').filter(|i| !dropped_items.contains(i.trim())).collect();
            out.push_str(&format!("WorkshopItems={}", left.join(";")));
        } else {
            out.push_str(line);
        }
        out.push_str(ending);
    }
    Some(out)
}

/// A save's `mods.txt` without the `mod = ...,` lines of the ids in `gone`; the rest (its maps
/// block included) kept as it was. None when nothing changes.
fn mods_txt_without(content: &str, gone: &HashSet<String>) -> Option<String> {
    let mut out = String::with_capacity(content.len());
    let mut in_mods = false;
    let mut changed = false;
    for piece in content.split_inclusive('\n') {
        let t = piece.trim();
        if t == "mods" {
            in_mods = true;
        } else if in_mods && t == "}" {
            in_mods = false;
        } else if in_mods {
            if let Some(rest) = t.strip_prefix("mod").filter(|r| r.trim_start().starts_with('=')) {
                let entry = rest.trim_start().trim_start_matches('=').trim().trim_end_matches(',').trim();
                if gone.contains(&key(entry)) {
                    changed = true;
                    continue;
                }
            }
        }
        out.push_str(piece);
    }
    changed.then_some(out)
}

/// After a scan: every id the app or the game keeps in a list and no folder has is taken out of
/// it. Returns the ids taken out (also kept for `take_forgotten`).
pub fn forget_missing_mods(paths: &StudioPaths, installed: &[ModManifest]) -> Vec<String> {
    forget(paths, installed, true)
}

/// `forget_missing_mods`; with `write` false it only says what it would take out.
fn forget(paths: &StudioPaths, installed: &[ModManifest], write: bool) -> Vec<String> {
    let z = Path::new(&paths.user_zomboid_dir);
    let instances = z.join("PZModStudio_Instances");
    let master = z.join("PZModStudio_MasterLoadOrder.json");
    let backups = z.join("PZModStudio_Backups");

    let read_lists = |path: &Path, keys: &[&str], into: &mut Vec<String>| {
        let Some(doc) = std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else { return };
        for k in keys {
            for v in doc.get(*k).and_then(|v| v.as_array()).into_iter().flatten() {
                if let Some(s) = v.as_str() {
                    into.push(s.to_string());
                }
            }
        }
    };
    let files_in = |dir: &Path, ext: &str| -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ext))).collect())
            .unwrap_or_default()
    };

    // Everything that keeps a mod id: what is on goes in `on` too.
    let mut named: Vec<String> = Vec::new();
    let mut on: Vec<String> = Vec::new();
    let profile_files = files_in(&instances, "json");
    const PROFILE_KEYS: [&str; 3] = ["active_mod_ids", "members", "load_order"];
    for f in &profile_files {
        read_lists(f, &PROFILE_KEYS, &mut named);
        read_lists(f, &["active_mod_ids"], &mut on);
    }
    read_lists(&master, &["load_order", "active_mod_ids"], &mut named);
    let game_list = read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default();
    on.extend(game_list.iter().cloned());
    let servers: Vec<(PathBuf, String)> = files_in(&z.join("Server"), "ini")
        .into_iter()
        .filter_map(|p| std::fs::read_to_string(&p).ok().map(|c| (p, c)))
        .collect();
    for (_, content) in &servers {
        on.extend(crate::server_manager::mod_sync::ini_list(content, "Mods"));
    }
    let saves: Vec<(String, PathBuf)> = crate::save_backup::list_saves(z)
        .into_iter()
        .map(|id| {
            let file = z.join("Saves").join(&id).join("mods.txt");
            (id, file)
        })
        .filter(|(_, f)| f.is_file())
        .collect();
    for (_, file) in &saves {
        on.extend(crate::save_backup::save_mods(file.parent().unwrap_or(z)));
    }
    named.extend(on.iter().cloned());

    let mut missing = missing_ids(named.iter(), installed, &z.join("mods"));
    if missing.is_empty() || !scan_is_whole(paths, installed, missing.len()) {
        return Vec::new();
    }
    // A mod that is on and whose author gave it another id in an update stays: Diagnostics offers
    // the new id in its place. One that is off has nothing to lose.
    let on: HashSet<String> = on.iter().map(|id| key(id)).collect();
    let mut catalog: Option<crate::catalog::ModCatalog> = None;
    let mut catalog_now = || catalog.get_or_insert_with(|| crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None)).clone();
    if missing.iter().any(|id| on.contains(&id.to_lowercase())) {
        let c = catalog_now();
        missing.retain(|id| !on.contains(&id.to_lowercase()) || crate::catalog::renamed_options(&c, id, &[]).is_empty());
        if missing.is_empty() {
            return Vec::new();
        }
    }
    if !write {
        return missing;
    }
    let gone: HashSet<String> = missing.iter().map(|id| id.to_lowercase()).collect();

    for f in &profile_files {
        prune_json(f, &PROFILE_KEYS, &gone, &backups);
    }
    prune_json(&master, &["load_order", "active_mod_ids"], &gone, &backups);
    // The Mod List's groups (every profile's); the player's notes stay.
    crate::mod_meta::take_out_of_groups(z, &gone, &backups);
    let kept: Vec<String> = game_list.iter().filter(|id| !gone.contains(&key(id))).cloned().collect();
    if kept.len() != game_list.len() {
        let _ = write_mod_list_ini(&paths.mod_list_ini_path, &kept);
    }

    // Servers: not the one running, whose .ini is in use.
    let running = crate::server_manager::get_dedicated_server_status(paths.user_zomboid_dir.clone())
        .ok()
        .filter(|s| s.is_running)
        .and_then(|s| s.server_name);
    for (path, content) in &servers {
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if running.as_deref().is_some_and(|r| r.eq_ignore_ascii_case(&name)) {
            continue;
        }
        if !crate::server_manager::mod_sync::ini_list(content, "Mods").iter().any(|m| gone.contains(&key(m))) {
            continue;
        }
        let c = catalog_now();
        let item_of = |id: &str| -> Option<String> {
            c.known_workshop.get(id).cloned().or_else(|| match &c.resolve(id)?.source {
                crate::catalog::ModSource::Workshop { workshop_id } => Some(workshop_id.clone()),
                _ => None,
            })
        };
        if let Some(new) = server_ini_without(content, &gone, &item_of) {
            let _ = write_file_with_backup(path, &new, &backups);
        }
    }

    // Saves: not while the game runs, which may have one open.
    if crate::sandbox::find_running_pz_pid().is_none() {
        for (id, file) in &saves {
            let Ok(content) = std::fs::read_to_string(file) else { continue };
            if let Some(new) = mods_txt_without(&content, &gone) {
                let _ = write_file_with_backup(file, &new, &backups.join("save-mod-lists").join(id.replace(['/', '\\'], "_")));
            }
        }
    }

    // Which of them the player unsubscribed from (their folders still there, left out by the scan).
    let pending = super::unsubscribed::pending(paths);
    let unsubscribed = if pending.is_empty() { HashSet::new() } else { super::unsubscribed::mod_ids_of_items(paths, &pending) };
    if let Ok(mut f) = FORGOTTEN.lock() {
        for id in &missing {
            if !f.iter().any(|(x, _)| x == id) {
                f.push((id.clone(), unsubscribed.contains(&id.to_lowercase())));
            }
        }
    }
    missing
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &str) -> ModManifest {
        let mut m = crate::load_order::mod_info::not_installed_row(id);
        m.not_installed = false;
        m
    }

    fn set(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_lowercase()).collect()
    }

    #[test]
    fn a_mod_with_no_folder_is_missing_and_one_with_a_folder_or_link_is_not() {
        let dir = std::env::temp_dir().join(format!("pzms_forget_{}", std::process::id()));
        let mods = dir.join("mods");
        std::fs::create_dir_all(mods.join("KnoxLives")).unwrap();
        let installed = vec![manifest("ModA"), manifest("Sub_Mod")];
        let named: Vec<String> = ["ModA", "moda", "\\Sub_Mod", "2392709985/ModA", "Gone", "\\Gone", "12345\\Gone", "KnoxLives", "Z_StudioLink", "Z_PZModStudio_MergedPatch", "AlsoGone"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(missing_ids(named.iter(), &installed, &mods), vec!["Gone".to_string(), "AlsoGone".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// What a scan would take out on this PC, written nowhere.
    #[test]
    #[ignore]
    fn live_what_would_be_forgotten() {
        let paths = crate::vfs::studio_paths();
        let installed = crate::load_order::mod_info::scan_all_installed_mods(&paths);
        let missing = forget(&paths, &installed, false);
        println!("installed {} | would forget {}", installed.iter().filter(|m| !m.not_installed).count(), missing.len());
        println!("{:?}", missing);
        let gone: HashSet<String> = missing.iter().map(|id| id.to_lowercase()).collect();
        let c = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
        let item_of = |id: &str| -> Option<String> {
            c.known_workshop.get(id).cloned().or_else(|| match &c.resolve(id)?.source {
                crate::catalog::ModSource::Workshop { workshop_id } => Some(workshop_id.clone()),
                _ => None,
            })
        };
        let z = Path::new(&paths.user_zomboid_dir);
        for e in std::fs::read_dir(z.join("Server")).unwrap().filter_map(|e| e.ok()).filter(|e| e.path().extension().is_some_and(|x| x == "ini")) {
            let content = std::fs::read_to_string(e.path()).unwrap();
            let Some(new) = server_ini_without(&content, &gone, &item_of) else { continue };
            let before = crate::server_manager::mod_sync::ini_list(&content, "Mods");
            let after = crate::server_manager::mod_sync::ini_list(&new, "Mods");
            let items_before = crate::server_manager::mod_sync::ini_list(&content, "WorkshopItems");
            let items_after = crate::server_manager::mod_sync::ini_list(&new, "WorkshopItems");
            println!(
                "{}: Mods {} -> {} {:?} | WorkshopItems {} -> {} {:?}",
                e.file_name().to_string_lossy(),
                before.len(),
                after.len(),
                before.iter().filter(|m| !after.contains(m)).collect::<Vec<_>>(),
                items_before.len(),
                items_after.len(),
                items_before.iter().filter(|m| !items_after.contains(m)).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn a_profile_loses_the_ids_and_keeps_everything_else() {
        let dir = std::env::temp_dir().join(format!("pzms_forget_json_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("inst_1.json");
        std::fs::write(&file, r#"{"id":"inst_1","name":"P","active_mod_ids":["A","Gone"],"members":["A","\\Gone","B"],"load_order":["Gone","A","B"],"game_memory_mb":4096}"#).unwrap();
        assert!(prune_json(&file, &["active_mod_ids", "members", "load_order"], &set(&["Gone"]), &dir.join("bak")));
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(doc["active_mod_ids"], serde_json::json!(["A"]));
        assert_eq!(doc["members"], serde_json::json!(["A", "B"]));
        assert_eq!(doc["load_order"], serde_json::json!(["A", "B"]));
        assert_eq!(doc["game_memory_mb"], serde_json::json!(4096));
        assert!(std::fs::read_dir(dir.join("bak")).unwrap().next().is_some(), "backed up");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_server_loses_the_mod_and_its_item_only_when_nothing_else_comes_from_it() {
        let ini = "# a comment\r\nPublic=false\r\nMods=\\A;\\Gone;\\GoneSub;\\Gone2\r\nMap=Muldraugh, KY\r\nWorkshopItems=100;200;300\r\n";
        // Gone comes from 200, as A does; GoneSub and Gone2 from 300, which keeps nothing.
        let item_of = |id: &str| match id {
            "A" | "Gone" => Some("200".to_string()),
            "GoneSub" | "Gone2" => Some("300".to_string()),
            _ => None,
        };
        let out = server_ini_without(ini, &set(&["Gone", "GoneSub", "Gone2"]), &item_of).unwrap();
        assert_eq!(out, "# a comment\r\nPublic=false\r\nMods=\\A\r\nMap=Muldraugh, KY\r\nWorkshopItems=100;200\r\n");
        assert!(server_ini_without(ini, &set(&["Other"]), &item_of).is_none());
    }

    #[test]
    fn a_save_list_loses_the_mod_lines_and_keeps_its_maps() {
        let txt = "VERSION = 1,\n\nmods\n{\n    mod = \\A,\n    mod = 12345\\Gone,\n    mod = \\B,\n}\n\nmaps\n{\n    map = Muldraugh, KY,\n}\n";
        let out = mods_txt_without(txt, &set(&["Gone"])).unwrap();
        assert_eq!(out, "VERSION = 1,\n\nmods\n{\n    mod = \\A,\n    mod = \\B,\n}\n\nmaps\n{\n    map = Muldraugh, KY,\n}\n");
        assert!(mods_txt_without(txt, &set(&["Other"])).is_none());
    }
}
