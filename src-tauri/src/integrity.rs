//! What changed on disk while the game ran: the game's folder, the Zomboid folder (saves, logs and
//! the Studio's own files left out) and ZombieBuddy's folder, before and after each session.
//!
//! The game itself writes into its Zomboid folder and nowhere else; its own folder stays as Steam
//! installed it. A file that changes there while it runs was written by something else: a Java
//! mod, a mod's helper program, another tool. The app watches for the game's process every few
//! seconds; when it appears it notes every file's size and time, and when it goes it compares.
//! The last sessions are kept in `Zomboid/PZModStudio_SessionChanges.json`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FILE: &str = "PZModStudio_SessionChanges.json";
const KEEP: usize = 10;
const MAX_LISTED: usize = 200;
const POLL_SECONDS: u64 = 5;

/// What one place had: relative path -> (size, modified seconds).
type Snapshot = BTreeMap<String, (u64, u64)>;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PlaceChanges {
    /// `game` (the game's folder), `zomboid`, `zombiebuddy`.
    pub place: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
    /// More than listed.
    pub total: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SessionChanges {
    /// Unix seconds the game was first seen and last seen running.
    pub started: u64,
    pub ended: u64,
    pub places: Vec<PlaceChanges>,
}

impl SessionChanges {
    /// Files of the game's own folder that changed: what the game never does itself.
    pub fn game_folder(&self) -> Option<&PlaceChanges> {
        self.places.iter().find(|p| p.place == "game" && p.total > 0)
    }
}

fn secs(t: std::time::SystemTime) -> u64 {
    t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Left out of the Zomboid folder: what the game writes every session (saves, logs, its console,
/// options), the Studio's own files and the Link's.
fn skip_in_zomboid(rel: &str) -> bool {
    let first = rel.split('/').next().unwrap_or("").to_ascii_lowercase();
    let lower = rel.to_ascii_lowercase();
    matches!(first.as_str(), "saves" | "logs" | "lua" | "backups" | "db" | "messaging" | "screenshots" | "statistic" | "multiplayer" | "server")
        || first.starts_with("pzmodstudio")
        || lower.ends_with(".txt") && !lower.contains('/')
        || lower.ends_with(".ini") && !lower.contains('/')
        || lower.ends_with(".log")
        || lower.ends_with(".bin") && !lower.contains('/')
}

/// Left out of the game's folder: Java's crash reports (written there when it falls over).
fn skip_in_game(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    // The launch files are the Studio's own to change (a profile's Java agent line, written when
    // the game closes): not something else writing in the game's folder.
    if lower == "projectzomboid64.json" || lower == "projectzomboid64.bat" || lower == "projectzomboid32.json" {
        return true;
    }
    (lower.starts_with("hs_err_pid") || lower.starts_with("replay_pid")) && lower.ends_with(".log")
}

fn snapshot(root: &Path, skip: fn(&str) -> bool) -> Snapshot {
    let mut out = Snapshot::new();
    if !root.is_dir() {
        return out;
    }
    // A folder left out is not walked at all (the saves alone can be tens of thousands of files).
    let walker = walkdir::WalkDir::new(root).into_iter().filter_entry(|e| {
        !e.file_type().is_dir() || e.depth() == 0 || e.path().strip_prefix(root).map(|r| !skip(&format!("{}/", r.to_string_lossy().replace('\\', "/")))).unwrap_or(true)
    });
    for e in walker.filter_map(Result::ok) {
        if !e.file_type().is_file() {
            continue;
        }
        let Ok(rel) = e.path().strip_prefix(root) else { continue };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if skip(&rel) {
            continue;
        }
        if let Ok(md) = e.metadata() {
            out.insert(rel, (md.len(), md.modified().map(secs).unwrap_or(0)));
        }
    }
    out
}

/// What differs between two snapshots of one place.
pub fn compare(place: &str, before: &Snapshot, after: &Snapshot) -> PlaceChanges {
    let mut c = PlaceChanges { place: place.to_string(), ..Default::default() };
    for (path, was) in before {
        match after.get(path) {
            None => c.removed.push(path.clone()),
            Some(now) if now != was => c.changed.push(path.clone()),
            _ => {}
        }
    }
    for path in after.keys() {
        if !before.contains_key(path) {
            c.added.push(path.clone());
        }
    }
    c.total = c.added.len() + c.removed.len() + c.changed.len();
    c.added.truncate(MAX_LISTED);
    c.removed.truncate(MAX_LISTED);
    c.changed.truncate(MAX_LISTED);
    c
}

fn places(install_dir: &Path, zomboid: &Path) -> Vec<(&'static str, PathBuf, fn(&str) -> bool)> {
    let home = dirs_next::home_dir().unwrap_or_default();
    vec![
        ("game", install_dir.to_path_buf(), skip_in_game as fn(&str) -> bool),
        ("zomboid", zomboid.to_path_buf(), skip_in_zomboid as fn(&str) -> bool),
        ("zombiebuddy", home.join(".zombie_buddy"), (|_: &str| false) as fn(&str) -> bool),
    ]
}

/// The sessions kept, newest last.
pub fn load(zomboid: &Path) -> Vec<SessionChanges> {
    std::fs::read_to_string(zomboid.join(FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn keep(zomboid: &Path, session: SessionChanges) {
    let mut all = load(zomboid);
    all.push(session);
    let drop = all.len().saturating_sub(KEEP);
    all.drain(..drop);
    if let Ok(text) = serde_json::to_string_pretty(&all) {
        let _ = crate::load_order::ini_parser::atomic_write(&zomboid.join(FILE), text);
    }
}

/// Watches for the game in the background: a snapshot when it starts, the comparison when it ends.
pub fn watch_in_background() {
    std::thread::spawn(|| {
        let mut before: Option<(u64, PathBuf, PathBuf, Vec<Snapshot>)> = None;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(POLL_SECONDS));
            let running = crate::sandbox::find_running_pz_pid().is_some();
            match (&before, running) {
                (None, true) => {
                    let paths = crate::vfs::studio_paths();
                    let Some(zomboid) = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next() else { continue };
                    let install = PathBuf::from(&paths.pz_install_dir);
                    if !install.is_dir() {
                        continue;
                    }
                    let shots = places(&install, &zomboid).into_iter().map(|(_, root, skip)| snapshot(&root, skip)).collect();
                    before = Some((secs(std::time::SystemTime::now()), install, zomboid, shots));
                }
                (Some((started, install, zomboid, shots)), false) => {
                    let mut session = SessionChanges { started: *started, ended: secs(std::time::SystemTime::now()), places: Vec::new() };
                    for ((name, root, skip), was) in places(install, zomboid).into_iter().zip(shots.iter()) {
                        session.places.push(compare(name, was, &snapshot(&root, skip)));
                    }
                    if let Some(g) = session.game_folder() {
                        crate::diagnostics::log("WARN", &format!("{} file(s) of the game's folder changed while it ran", g.total));
                    }
                    keep(zomboid, session);
                    before = None;
                }
                _ => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_removed_and_changed_are_told_apart() {
        let before: Snapshot = [("a.jar".to_string(), (10, 1)), ("b.txt".to_string(), (5, 1)), ("c.dll".to_string(), (7, 1))].into();
        let after: Snapshot = [("a.jar".to_string(), (11, 2)), ("c.dll".to_string(), (7, 1)), ("d.dll".to_string(), (3, 2))].into();
        let c = compare("game", &before, &after);
        assert_eq!((c.added, c.removed, c.changed, c.total), (vec!["d.dll".to_string()], vec!["b.txt".to_string()], vec!["a.jar".to_string()], 3));
    }

    #[test]
    fn what_the_game_writes_itself_is_left_out() {
        assert!(skip_in_zomboid("Saves/Sandbox/World/map_1_1.bin"));
        assert!(skip_in_zomboid("console.txt") && skip_in_zomboid("options.ini") && skip_in_zomboid("PZModStudio_Learned.json"));
        assert!(!skip_in_zomboid("mods/MyMod/42/mod.info"));
        assert!(skip_in_game("hs_err_pid123.log") && skip_in_game("ProjectZomboid64.json") && !skip_in_game("projectzomboid.jar"));
    }
}

/// Diagnostics: files of the game's own folder that changed during the last session the app
/// watched. Something other than the game wrote them.
pub fn issues(zomboid: &Path) -> Vec<crate::catalog::HealthIssue> {
    use crate::catalog::{issue_params, issue_state, HealthIssue, Severity};
    let Some(last) = load(zomboid).pop() else { return Vec::new() };
    let Some(g) = last.game_folder() else { return Vec::new() };
    let files: Vec<&str> = g.added.iter().chain(&g.changed).chain(&g.removed).take(5).map(String::as_str).collect();
    let when = chrono::DateTime::from_timestamp(last.ended as i64, 0).map(|d| d.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default();
    let message = format!(
        "While the game ran (session that ended {}), {} file(s) of the game's own folder changed: {}{}. The game never writes there itself: a Java mod, a mod's helper program or another tool did. If you did not install or update something there yourself, look at which mod could have.",
        when,
        g.total,
        files.join(", "),
        if g.total > files.len() { ", ..." } else { "" }
    );
    let severity = Severity::Warning;
    let mut params = issue_params(&[("when", &when), ("files", &files.join(", "))]);
    params.insert("count".into(), g.total.to_string());
    vec![HealthIssue {
        severity,
        state: issue_state(severity, "game_folder_changed"),
        code: "game_folder_changed".into(),
        mod_id: None,
        message,
        fixes: Vec::new(),
        related: Vec::new(),
        details: vec![],
        params,
        own_mod: false,
    }]
}

#[cfg(test)]
mod live {
    /// How long a snapshot of this PC's places takes. Ignored: needs the PC's game.
    #[test]
    #[ignore]
    fn snapshot_time_on_this_pc() {
        let install = std::path::Path::new(r"D:\Apps\Steam\steamapps\common\ProjectZomboid");
        let zomboid = std::path::Path::new(r"C:\Users\javie\Zomboid");
        for (name, root, skip) in super::places(install, zomboid) {
            let t = std::time::Instant::now();
            let s = super::snapshot(&root, skip);
            eprintln!("{}: {} files in {} ms", name, s.len(), t.elapsed().as_millis());
        }
    }
}
