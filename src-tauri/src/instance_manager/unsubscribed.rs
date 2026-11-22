//! A Workshop item the player unsubscribed from is gone for the app at once.
//!
//! Steam keeps an unsubscribed item's folder, and its entry in `appworkshop_108600.acf`, for
//! minutes or hours before it deletes them; until then every scan found the mod again and the
//! profiles kept it. So the app keeps its own record of the items unsubscribed
//! (`Zomboid/PZModStudio_Unsubscribed.json`) and the scan leaves their Workshop folders out:
//! `forget_missing` then takes their mods out of every profile as for a folder that went.
//!
//! An item gets on the record two ways: the app unsubscribed it (Steam said yes), or Steam's own
//! list of the account's subscriptions, read now and then through the running Steam client, no
//! longer has an item that is still on disk and that this account had subscribed (it was on the
//! list read before, or Steam's `.acf` names the account as its subscriber). An item the
//! account never subscribed (downloaded to join a server) is never put on the record.
//!
//! Never when Steam's answer may be wrong: no answer, an empty list, a list much shorter than the
//! one before or than what is on disk, or more than half of what is on disk gone at once. Local
//! mods (`Zomboid/mods`) are never affected, and nothing under the Workshop folder is ever
//! written or deleted: Steam does that. The record forgets an item when Steam deleted its folder,
//! when the app subscribes to it again, or when Steam lists it as subscribed again.
//!
//! Unsubscribed is not the same as gone from Steam. An item its author removed or hid, or that
//! Steam banned, may leave the account's list too, and then the copies the player has of it are
//! all that is left: those stay. So an item missing from Steam's list is put on the record only
//! when Steam's public API still finds it (public, not banned); without an answer from Steam
//! nothing is put on it. Unsubscribing in the app always counts.
//!
//! What the app made of an unsubscribed item goes too (`clean_up`, never while the game runs):
//! the versions the version store kept by itself are dropped, and a local copy in `Zomboid/mods`
//! made from the item (a compatible copy, a kept version put back) is moved to
//! `Zomboid/PZModStudio_Backups/mods`. Versions the player kept by hand (and the plain copies
//! from before the store, also made by hand) stay, listed with the kept versions, never as a mod.
//!
//! Every place that lists Workshop items reads `pending_in` (or the catalog, which does).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::vfs::StudioPaths;

pub const FILE: &str = "PZModStudio_Unsubscribed.json";
/// After the app unsubscribes an item, Steam's list may still have it for a moment.
const GRACE_SECS: u64 = 600;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
struct Record {
    /// Workshop item -> when it was unsubscribed (unix seconds).
    #[serde(default)]
    items: BTreeMap<String, u64>,
    /// Steam's list of subscriptions as last read.
    #[serde(default)]
    known_subscribed: Vec<String>,
    /// When Steam's list was last read (unix seconds).
    #[serde(default)]
    checked_at: u64,
    /// Unsubscribed items whose app-made leftovers (versions kept by the app, local copies) are
    /// not cleared yet: kept after Steam deleted the folder, until `clean_up` is done with them.
    #[serde(default)]
    to_clean: BTreeSet<String>,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn zomboid(paths: &StudioPaths) -> Option<PathBuf> {
    zomboid_of(&paths.user_zomboid_dir)
}

fn zomboid_of(user_zomboid_dir: &str) -> Option<PathBuf> {
    crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next()
}

fn load(z: &Path) -> Record {
    std::fs::read_to_string(z.join(FILE)).ok().and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok()).unwrap_or_default()
}

fn store(z: &Path, r: &Record) {
    if let Ok(text) = serde_json::to_string_pretty(r) {
        let _ = crate::load_order::ini_parser::atomic_write(z.join(FILE), text);
    }
}

fn numeric(ids: &[String]) -> Vec<String> {
    ids.iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())).collect()
}

/// The app unsubscribed these items and Steam said yes.
pub fn note_unsubscribed(paths: &StudioPaths, items: &[String]) {
    let Some(z) = zomboid(paths) else { return };
    let items = numeric(items);
    if items.is_empty() {
        return;
    }
    let mut r = load(&z);
    let now = now_secs();
    for i in items {
        r.items.insert(i.clone(), now);
        r.to_clean.insert(i.clone());
        r.known_subscribed.retain(|k| *k != i);
    }
    store(&z, &r);
}

/// The app subscribed to these items again: they are the player's again.
pub fn note_subscribed(paths: &StudioPaths, items: &[String]) {
    let Some(z) = zomboid(paths) else { return };
    let items = numeric(items);
    let mut r = load(&z);
    let before = r.clone();
    for i in items {
        r.items.remove(&i);
        r.to_clean.remove(&i);
        if !r.known_subscribed.contains(&i) {
            r.known_subscribed.push(i);
        }
    }
    if r != before {
        store(&z, &r);
    }
}

/// The unsubscribed items whose folder Steam has not deleted yet: the scan leaves them out. The
/// ones Steam deleted leave the record.
pub fn pending(paths: &StudioPaths) -> HashSet<String> {
    pending_in(&paths.workshop_dir, &paths.user_zomboid_dir)
}

/// `pending` for code that has the two folders rather than `StudioPaths`: every reader of the
/// Workshop folder leaves these items out.
pub fn pending_in(workshop_dir: &str, user_zomboid_dir: &str) -> HashSet<String> {
    let Some(z) = zomboid_of(user_zomboid_dir) else { return HashSet::new() };
    let mut r = load(&z);
    if r.items.is_empty() {
        return HashSet::new();
    }
    let workshop = Path::new(workshop_dir);
    if !workshop_dir.is_empty() && workshop.is_dir() {
        let before = r.items.len();
        r.items.retain(|id, _| workshop.join(id).is_dir());
        if r.items.len() != before {
            store(&z, &r);
        }
    }
    r.items.keys().cloned().collect()
}

/// The mod ids (lowercase) the Workshop folders of `items` hold.
pub fn mod_ids_of_items(paths: &StudioPaths, items: &HashSet<String>) -> HashSet<String> {
    let workshop = Path::new(&paths.workshop_dir);
    let mut out = HashSet::new();
    for item in items {
        for e in walkdir::WalkDir::new(workshop.join(item)).max_depth(8).into_iter().filter_map(|e| e.ok()) {
            if e.file_name() == "mod.info" {
                if let Some(m) = crate::load_order::mod_info::parse_mod_info(e.path()) {
                    out.insert(m.id.to_lowercase());
                }
            }
        }
    }
    out
}

/// The items on disk that Steam's list (`subscribed_now`) no longer has and that this account
/// had subscribed (`known_before`: the list read before; `acf_subscribed`: what Steam's `.acf`
/// names a subscriber for). None when the list cannot be trusted.
pub fn newly_unsubscribed(
    on_disk: &HashSet<String>,
    subscribed_now: &HashSet<String>,
    known_before: &HashSet<String>,
    acf_subscribed: &HashSet<String>,
) -> Option<Vec<String>> {
    if subscribed_now.is_empty() || on_disk.is_empty() {
        return None;
    }
    // A list much shorter than before, or than the items on disk: Steam half-loaded, not a
    // mass unsubscribe.
    if subscribed_now.len() < known_before.len() / 2 || subscribed_now.len() < on_disk.len() / 2 {
        return None;
    }
    let mut gone: Vec<String> = on_disk
        .iter()
        .filter(|i| !subscribed_now.contains(*i) && (known_before.contains(*i) || acf_subscribed.contains(*i)))
        .cloned()
        .collect();
    if gone.len() > on_disk.len() / 2 {
        return None;
    }
    gone.sort();
    Some(gone)
}

/// Whether an item on the record stays there now that Steam's list was read: not when Steam
/// lists it again, unless the app unsubscribed it a moment ago (Steam's list may lag).
fn stays_unsubscribed(noted_at: u64, now: u64, listed: bool) -> bool {
    !listed || now.saturating_sub(noted_at) < GRACE_SECS
}

/// Of the items Steam's list no longer has, the ones the player unsubscribed from: those Steam's
/// public API still finds (`answers`: id -> public and not banned). An item that is not public
/// any more (removed, hidden, banned) left the list because it left Steam: its copies are the
/// player's last ones and it is not taken for unsubscribed. No answer from Steam: none.
pub fn unsubscribed_not_gone(candidates: &[String], answers: Option<&[(String, bool)]>) -> Vec<String> {
    let Some(answers) = answers else { return Vec::new() };
    candidates.iter().filter(|c| answers.iter().any(|(id, public)| id == *c && *public)).cloned().collect()
}

/// Steam's public answer for Workshop items: (id, public and not banned); None when Steam did not
/// answer.
pub type PublicCheck<'a> = &'a dyn Fn(&[String]) -> Option<Vec<(String, bool)>>;

/// The real `PublicCheck`: GetPublishedFileDetails (no key, cached).
pub fn steam_public(ids: &[String]) -> Option<Vec<(String, bool)>> {
    crate::workshop::get_details(ids).ok().map(|items| items.into_iter().map(|i| (i.workshop_id.clone(), i.found && !i.banned)).collect())
}

/// Steam's list of subscriptions, read now: puts the items unsubscribed elsewhere (and still on
/// the Workshop) on the record and takes off it the ones subscribed again. Returns the items
/// newly put on it.
pub fn apply_steam_list(paths: &StudioPaths, subscribed_now: &[u64], public: PublicCheck) -> Vec<String> {
    let Some(z) = zomboid(paths) else { return Vec::new() };
    let workshop = Path::new(&paths.workshop_dir);
    if paths.workshop_dir.is_empty() || !workshop.is_dir() {
        return Vec::new();
    }
    let on_disk: HashSet<String> = std::fs::read_dir(workshop)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                .collect()
        })
        .unwrap_or_default();
    let now_set: HashSet<String> = subscribed_now.iter().map(|n| n.to_string()).collect();
    let mut r = load(&z);
    let known: HashSet<String> = r.known_subscribed.iter().cloned().collect();
    let acf = crate::load_order::mod_info::get_subscribed_workshop_ids(workshop).unwrap_or_default();
    let Some(gone) = newly_unsubscribed(&on_disk, &now_set, &known, &acf) else { return Vec::new() };
    // Only the ones still on the Workshop: the rest left Steam, not the player's subscriptions.
    let fresh: Vec<String> = gone.into_iter().filter(|id| !r.items.contains_key(id)).collect();
    let gone = if fresh.is_empty() { fresh } else { unsubscribed_not_gone(&fresh, public(&fresh).as_deref()) };

    let now = now_secs();
    let before: Vec<String> = r.items.keys().cloned().collect();
    r.items.retain(|id, at| stays_unsubscribed(*at, now, now_set.contains(id)));
    for id in before.iter().filter(|id| !r.items.contains_key(*id)) {
        r.to_clean.remove(id);
    }
    let mut added = Vec::new();
    for id in gone {
        if !r.items.contains_key(&id) {
            r.items.insert(id.clone(), now);
            r.to_clean.insert(id.clone());
            added.push(id);
        }
    }
    let mut list: Vec<String> = now_set.into_iter().collect();
    list.sort();
    r.known_subscribed = list;
    r.checked_at = now;
    store(&z, &r);
    added
}

/// What `clean_up` did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CleanUp {
    /// Versions the store had kept by itself, dropped.
    pub versions_dropped: usize,
    /// Local copies moved to `Zomboid/PZModStudio_Backups/mods` (their folder names).
    pub copies_moved: Vec<String>,
    /// The game runs: nothing done, done after it closes.
    pub waiting_for_game: bool,
}

/// Clears what the app made of the items the player unsubscribed from: the versions the store
/// kept by itself are dropped, local copies made from them (compatible copies, kept versions put
/// back) are moved to the backups. Versions kept by hand stay. Never while the game runs.
pub fn clean_up(paths: &StudioPaths) -> CleanUp {
    let Some(z) = zomboid(paths) else { return CleanUp::default() };
    if load(&z).to_clean.is_empty() {
        return CleanUp::default();
    }
    if crate::sandbox::find_running_pz_pid().is_some() {
        return CleanUp { waiting_for_game: true, ..Default::default() };
    }
    clean_up_in(&z)
}

fn clean_up_in(z: &Path) -> CleanUp {
    let mut out = CleanUp::default();
    let user = z.to_string_lossy().to_string();
    let mut done = Vec::new();
    for item in load(z).to_clean {
        let mut ok = true;
        // Local copies first: a copy of a kept version keeps that version in use.
        for (folder, rec) in crate::origins::load(z) {
            let crate::origins::ModOrigin::Compat { workshop_id: Some(w), .. } = &rec.origin else { continue };
            if *w != item {
                continue;
            }
            match crate::compat::remove_compatible_copy(&user, &folder) {
                Ok(()) => out.copies_moved.push(folder),
                Err(e) => {
                    ok = false;
                    eprintln!("Unsubscribed item {}: the copy {} stays for now: {}", item, folder, e);
                }
            }
        }
        match crate::snapshots::drop_auto_versions(&user, &item) {
            Ok(n) => out.versions_dropped += n,
            Err(e) => {
                ok = false;
                eprintln!("Unsubscribed item {}: its kept versions stay for now: {}", item, e);
            }
        }
        if ok {
            done.push(item);
        }
    }
    if !done.is_empty() {
        // Read again: the record may have changed meanwhile.
        let mut r = load(z);
        for d in &done {
            r.to_clean.remove(d);
        }
        store(z, &r);
    }
    out
}

/// When Steam's list was last read, and whether the game ran then.
static LAST_CHECK: std::sync::Mutex<Option<(Instant, bool)>> = std::sync::Mutex::new(None);

/// Whether to read Steam's list now. Reading it starts Steam's API in a helper, which shows the
/// player as in the game for a second when the game is not running: so at the app's first scan,
/// at the first scan after the game closed, and then at most every 30 minutes; every 5 minutes
/// while the game runs (Steam shows the player in the game already).
fn check_due(last: Option<(Duration, bool)>, running: bool) -> bool {
    match last {
        None => true,
        Some((ago, was_running)) => (was_running && !running) || ago >= Duration::from_secs(if running { 5 * 60 } else { 30 * 60 }),
    }
}

/// Takes the turn to read Steam's list when it is due (and marks it taken).
pub fn take_check_turn(running: bool) -> bool {
    let Ok(mut last) = LAST_CHECK.lock() else { return false };
    if !check_due(last.map(|(at, r)| (at.elapsed(), r)), running) {
        return false;
    }
    *last = Some((Instant::now(), running));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn an_item_on_disk_steam_no_longer_lists_is_unsubscribed_when_the_account_had_it() {
        let disk = set(&["1", "2", "3", "4", "5", "6"]);
        let now = set(&["1", "2", "3", "4", "9"]);
        // 5 was on the list before, 6 only in the .acf: both were the account's.
        assert_eq!(newly_unsubscribed(&disk, &now, &set(&["1", "2", "3", "4", "5"]), &set(&["6"])), Some(vec!["5".to_string(), "6".to_string()]));
        // 6 never subscribed by this account (downloaded for a server): stays.
        assert_eq!(newly_unsubscribed(&disk, &now, &set(&["1", "2", "3", "4", "5"]), &HashSet::new()), Some(vec!["5".to_string()]));
        // Nothing gone.
        assert_eq!(newly_unsubscribed(&set(&["1", "2"]), &set(&["1", "2", "3"]), &HashSet::new(), &set(&["1", "2"])), Some(vec![]));
    }

    #[test]
    fn a_list_that_cannot_be_trusted_takes_nothing_out() {
        let disk = set(&["1", "2", "3", "4", "5", "6"]);
        let acf = disk.clone();
        assert_eq!(newly_unsubscribed(&disk, &HashSet::new(), &disk, &acf), None, "empty answer");
        assert_eq!(newly_unsubscribed(&HashSet::new(), &set(&["1"]), &HashSet::new(), &HashSet::new()), None, "nothing on disk");
        assert_eq!(newly_unsubscribed(&disk, &set(&["1", "2"]), &HashSet::new(), &acf), None, "far fewer than on disk");
        let many: HashSet<String> = (0..100).map(|i| i.to_string()).collect();
        let few: HashSet<String> = (0..40).map(|i| i.to_string()).collect();
        assert_eq!(newly_unsubscribed(&few, &few, &many, &few), None, "far fewer than the list before");
        // More than half of what is on disk gone at once.
        let mostly_other: HashSet<String> = (100..110).map(|i| i.to_string()).chain(["1", "2"].iter().map(|s| s.to_string())).collect();
        assert_eq!(newly_unsubscribed(&disk, &mostly_other, &HashSet::new(), &acf), None);
    }

    #[test]
    fn steam_listing_an_item_again_takes_it_off_the_record_after_a_grace() {
        assert!(stays_unsubscribed(1000, 2000, false));
        assert!(stays_unsubscribed(1000, 1000 + GRACE_SECS - 1, true), "the app just unsubscribed it");
        assert!(!stays_unsubscribed(1000, 1000 + GRACE_SECS, true), "subscribed again");
    }

    #[test]
    fn steam_s_list_is_read_at_start_after_the_game_closes_and_now_and_then() {
        assert!(check_due(None, false));
        assert!(check_due(Some((Duration::from_secs(10), true)), false), "the game just closed");
        assert!(!check_due(Some((Duration::from_secs(10), false)), false));
        assert!(!check_due(Some((Duration::from_secs(29 * 60), false)), false));
        assert!(check_due(Some((Duration::from_secs(30 * 60), false)), false));
        assert!(!check_due(Some((Duration::from_secs(4 * 60), true)), true));
        assert!(check_due(Some((Duration::from_secs(5 * 60), true)), true));
    }

    #[test]
    fn the_record_keeps_what_the_app_unsubscribed_until_steam_deletes_the_folder() {
        let dir = std::env::temp_dir().join(format!("pzms_unsub_{}", std::process::id()));
        let z = dir.join("Zomboid");
        let ws = dir.join("108600");
        std::fs::create_dir_all(z.join("mods")).unwrap();
        std::fs::create_dir_all(ws.join("111")).unwrap();
        std::fs::create_dir_all(ws.join("222")).unwrap();
        let paths = StudioPaths {
            pz_install_dir: String::new(),
            workshop_dir: ws.to_string_lossy().to_string(),
            user_zomboid_dir: z.to_string_lossy().to_string(),
            mod_list_ini_path: String::new(),
            carrier_workshop_id: None,
            is_valid: true,
        };
        note_unsubscribed(&paths, &["111".to_string(), "222".to_string(), "not a number".to_string()]);
        assert_eq!(pending(&paths), set(&["111", "222"]));
        std::fs::remove_dir_all(ws.join("222")).unwrap();
        assert_eq!(pending(&paths), set(&["111"]), "Steam deleted 222's folder");
        note_subscribed(&paths, &["111".to_string()]);
        assert!(pending(&paths).is_empty(), "subscribed again");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A Zomboid folder and a Workshop folder with items holding one mod each.
    fn setup(name: &str, items: &[(&str, &str)]) -> (PathBuf, StudioPaths) {
        let dir = std::env::temp_dir().join(format!("pzms_unsub_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let z = dir.join("Zomboid");
        let ws = dir.join("108600");
        std::fs::create_dir_all(z.join("mods")).unwrap();
        for (item, id) in items {
            let m = ws.join(item).join("mods").join(id);
            std::fs::create_dir_all(&m).unwrap();
            std::fs::write(m.join("mod.info"), format!("name={}
id={}
", id, id)).unwrap();
        }
        let paths = StudioPaths {
            pz_install_dir: String::new(),
            workshop_dir: ws.to_string_lossy().to_string(),
            user_zomboid_dir: z.to_string_lossy().to_string(),
            mod_list_ini_path: String::new(),
            carrier_workshop_id: None,
            is_valid: true,
        };
        (dir, paths)
    }

    #[test]
    fn an_item_missing_from_steam_s_list_is_unsubscribed_only_while_the_workshop_still_has_it() {
        let c = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        // 1 is public: the player unsubscribed. 2 was removed or hidden by its author (not found),
        // 3 is not in the answer at all: both left Steam, their copies stay.
        let answers = vec![("1".to_string(), true), ("2".to_string(), false)];
        assert_eq!(unsubscribed_not_gone(&c, Some(&answers)), vec!["1".to_string()]);
        assert!(unsubscribed_not_gone(&c, None).is_empty(), "no answer from Steam: nothing");
        assert!(unsubscribed_not_gone(&c, Some(&[])).is_empty());
    }

    #[test]
    fn steam_s_list_puts_on_the_record_only_items_still_public() {
        let items: Vec<(String, String)> = (1..=6).map(|i| (format!("{}00", i), format!("Mod{}", i))).collect();
        let refs: Vec<(&str, &str)> = items.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let (dir, paths) = setup("steamlist", &refs);
        // The account had all six (the list read before).
        let all: Vec<String> = items.iter().map(|(i, _)| i.clone()).collect();
        note_subscribed(&paths, &all);
        // Now Steam lists 100..400: 500 was unsubscribed on Steam's site, 600 was removed by its author.
        let now: Vec<u64> = vec![100, 200, 300, 400];
        let public = |ids: &[String]| -> Option<Vec<(String, bool)>> { Some(ids.iter().map(|i| (i.clone(), i == "500")).collect()) };
        assert_eq!(apply_steam_list(&paths, &now, &public), vec!["500".to_string()]);
        assert_eq!(pending(&paths), set(&["500"]));
        let z = zomboid(&paths).unwrap();
        assert_eq!(load(&z).to_clean, ["500".to_string()].into_iter().collect::<BTreeSet<_>>());
        // Steam does not answer: nothing more is put on the record.
        let silent = |_: &[String]| -> Option<Vec<(String, bool)>> { None };
        assert!(apply_steam_list(&paths, &now, &silent).is_empty());
        assert_eq!(pending(&paths), set(&["500"]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_catalog_and_the_scan_leave_an_unsubscribed_item_out() {
        let (dir, paths) = setup("catalog", &[("111", "Kept"), ("222", "Dropped")]);
        note_unsubscribed(&paths, &["222".to_string()]);
        assert_eq!(pending_in(&paths.workshop_dir, &paths.user_zomboid_dir), set(&["222"]));
        let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, Some(42_021));
        let ids: Vec<&str> = catalog.mods.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"Kept"), "{:?}", ids);
        assert!(!ids.contains(&"Dropped"), "{:?}", ids);
        let scanned = crate::load_order::mod_info::scan_all_installed_mods(&paths);
        assert!(scanned.iter().all(|m| m.id != "Dropped"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clean_up_drops_the_app_s_versions_moves_copies_and_keeps_what_the_player_kept() {
        use crate::snapshots::store::{KeepInfo, Store};
        let (dir, paths) = setup("cleanup", &[("111", "Foo"), ("222", "Bar")]);
        let z = zomboid(&paths).unwrap();
        let ws = PathBuf::from(&paths.workshop_dir);
        let info = |id: &str, ident: &str, by_player: bool| KeepInfo { workshop_id: Some(id.to_string()), by_player, steam_ident: Some(ident.to_string()), published: None, mods: Vec::new() };
        let store = Store::at(&z);
        let (auto1, _) = store.keep("111", &ws.join("111"), info("111", "a", false), None).unwrap();
        std::fs::write(ws.join("111/mods/Foo/extra.txt"), "2").unwrap();
        let (by_hand, _) = store.keep("111", &ws.join("111"), info("111", "b", true), Some(&auto1)).unwrap();
        std::fs::write(ws.join("111/mods/Foo/extra.txt"), "3").unwrap();
        store.keep("111", &ws.join("111"), info("111", "c", false), Some(&by_hand)).unwrap();
        let (other, _) = store.keep("222", &ws.join("222"), info("222", "a", false), None).unwrap();
        // A compatible copy of 111 in Zomboid/mods, and a local mod of the player's.
        std::fs::create_dir_all(z.join("mods/Foo_compat")).unwrap();
        crate::origins::record(&z, &["Foo_compat".to_string()], &crate::origins::ModOrigin::Compat { workshop_id: Some("111".into()), source_dir: "x".into(), changes: vec![] }).unwrap();
        std::fs::create_dir_all(z.join("mods/Mine")).unwrap();

        // Gone from Steam (not unsubscribed): nothing to clean.
        assert_eq!(clean_up_in(&z), CleanUp::default());

        note_unsubscribed(&paths, &["111".to_string()]);
        let done = clean_up_in(&z);
        assert_eq!(done.versions_dropped, 2, "the two the app kept by itself");
        assert_eq!(done.copies_moved, vec!["Foo_compat".to_string()]);
        assert!(!z.join("mods/Foo_compat").exists());
        assert!(z.join("mods/Mine").is_dir(), "the player's own local mod stays");
        let backups: Vec<String> = std::fs::read_dir(z.join("PZModStudio_Backups/mods")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert!(backups.iter().any(|b| b.starts_with("Foo_compat")), "moved, never deleted: {:?}", backups);
        let left: Vec<_> = Store::at(&z).versions();
        assert_eq!(left.len(), 2);
        assert!(left.iter().any(|v| v.head_path == by_hand.head_path), "kept by hand: stays");
        assert!(left.iter().any(|v| v.head_path == other.head_path), "another item's: stays");
        assert!(load(&z).to_clean.is_empty(), "done once");
        assert_eq!(clean_up_in(&z), CleanUp::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
