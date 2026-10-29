//! Halving the mod list until one mod is left: the hunt players do by hand today.
//!
//! The game only names a mod when it writes a Lua error. A freeze, a missing recipe, a map that
//! will not load or a save that will not open names nobody, and the way to find the culprit is to
//! switch half the list off, play, and keep halving whichever half still does it. Seven launches
//! settle a hundred mods.
//!
//! Everything a trial needs stays on in every trial: anything an active mod requires (and what
//! those require, all the way down) plus the Studio's own bridge, so no trial ever launches with
//! a broken requirement, which would look like a second bug. The hunt is written to
//! `Zomboid/PZModStudio_Bisect.json` after every answer, so closing the Studio between launches
//! does not lose it, and the list switched on before it started is kept there to be restored.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::load_order::ini_parser::write_mod_list_ini;

/// What the hunt is waiting for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BisectPhase {
    /// Half the suspects are on; the answer halves them again.
    Narrowing,
    /// One suspect is left, on its own, to be sure it is the one.
    Confirming,
    /// Over: either a culprit or a reason there is none.
    Done,
}

/// One answered launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BisectRound {
    pub round: u32,
    /// How many suspects were switched on for it.
    pub tried: usize,
    /// How many were still under suspicion when it started.
    pub suspects: usize,
    /// "still" or "gone", as the player answered.
    pub answer: String,
}

/// A hunt in progress, as the Studio and the player see it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BisectSession {
    pub started_at: String,
    pub phase: BisectPhase,
    /// The list that was switched on when the hunt started, in its order. Restored at the end.
    pub original_active: Vec<String>,
    /// On in every trial: requirements of the active list, and the bridge.
    pub always_on: Vec<String>,
    /// Still under suspicion.
    pub candidates: Vec<String>,
    /// The suspects switched on right now.
    pub trial: Vec<String>,
    pub round: u32,
    /// Launches still needed, at worst.
    pub launches_left: u32,
    pub history: Vec<BisectRound>,
    /// The mod the hunt ended on.
    pub culprit: Option<String>,
    /// Why the hunt ended without one, in words for the player.
    pub note: Option<String>,
}

/// One mod as the hunt needs it: its id, what it requires, and whether it is on.
#[derive(Debug, Clone, Deserialize)]
pub struct BisectMod {
    pub mod_id: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub enabled: bool,
}

/// The game's own cleanup of a `require=` value: backslashes removed, trimmed.
fn clean(raw: &str) -> String {
    raw.replace('\\', "").trim().to_string()
}

fn same_mod(a: &str, b: &str) -> bool {
    let norm = |s: &str| clean(s).to_lowercase();
    norm(a) == norm(b)
}

/// The first half, rounded up, so a list of one is never split into nothing.
pub fn half(candidates: &[String]) -> Vec<String> {
    let take = (candidates.len() + 1) / 2;
    candidates[..take].to_vec()
}

/// Who is still under suspicion after an answer.
pub fn narrow(candidates: &[String], trial: &[String], still_happens: bool) -> Vec<String> {
    if still_happens {
        // It is among the ones that were on.
        candidates.iter().filter(|c| trial.contains(c)).cloned().collect()
    } else {
        // It is among the ones that were off.
        candidates.iter().filter(|c| !trial.contains(c)).cloned().collect()
    }
}

/// Launches still needed to get from `n` suspects to one, at worst.
pub fn launches_left(n: usize) -> u32 {
    let mut left = 0;
    let mut n = n;
    while n > 1 {
        n = (n + 1) / 2;
        left += 1;
    }
    // Plus the one that confirms the last suspect on its own.
    left + 1
}

/// Everything that has to stay on: what the active mods require, transitively, and the bridge.
pub fn always_on_set(mods: &[BisectMod]) -> Vec<String> {
    let enabled: Vec<&BisectMod> = mods.iter().filter(|m| m.enabled).collect();
    let resolve = |want: &str| -> Option<String> {
        enabled
            .iter()
            .find(|m| same_mod(&m.mod_id, want))
            .map(|m| m.mod_id.clone())
    };

    let mut keep: Vec<String> = Vec::new();
    let mut queue: Vec<String> = Vec::new();
    for m in &enabled {
        // The bridge, and Studio Link which keeps the game on the active profile, are never suspects.
        if m.mod_id == "Z_PZModStudio_Bridge" || m.mod_id == crate::studio_link::LINK_MOD_ID {
            keep.push(m.mod_id.clone());
        }
        for dep in &m.dependencies {
            if let Some(id) = resolve(dep) {
                queue.push(id);
            }
        }
    }
    // What the requirements themselves require, all the way down.
    while let Some(id) = queue.pop() {
        if keep.contains(&id) {
            continue;
        }
        keep.push(id.clone());
        if let Some(m) = enabled.iter().find(|m| m.mod_id == id) {
            for dep in &m.dependencies {
                if let Some(dep_id) = resolve(dep) {
                    if !keep.contains(&dep_id) {
                        queue.push(dep_id);
                    }
                }
            }
        }
    }
    keep
}

/// The trial written to `default.txt`: always-on plus the suspects on trial, in the player's own
/// load order, because the order is part of what is being tested.
fn trial_list(session: &BisectSession) -> Vec<String> {
    session
        .original_active
        .iter()
        .filter(|id| session.always_on.contains(id) || session.trial.contains(id))
        .cloned()
        .collect()
}

fn session_path(user_zomboid_dir: &str) -> PathBuf {
    Path::new(user_zomboid_dir).join("PZModStudio_Bisect.json")
}

/// The hunt on disk, if there is one. A file that cannot be understood is treated as none.
pub fn load(user_zomboid_dir: &str) -> Option<BisectSession> {
    let text = std::fs::read_to_string(session_path(user_zomboid_dir)).ok()?;
    serde_json::from_str(&text).ok()
}

fn save(user_zomboid_dir: &str, session: &BisectSession) -> Result<(), String> {
    let path = session_path(user_zomboid_dir);
    let text = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(&path, text).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

fn forget(user_zomboid_dir: &str) {
    let _ = std::fs::remove_file(session_path(user_zomboid_dir));
}

fn now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

/// Starts a hunt and switches the first half on. The list on disk is replaced, with a backup, by
/// the trial; what was there is kept in the session to be put back.
pub fn start(
    mod_list_ini_path: &str,
    user_zomboid_dir: &str,
    mods: &[BisectMod],
) -> Result<BisectSession, String> {
    let original_active: Vec<String> =
        mods.iter().filter(|m| m.enabled).map(|m| m.mod_id.clone()).collect();
    let always_on = always_on_set(mods);
    let candidates: Vec<String> = original_active
        .iter()
        .filter(|id| !always_on.contains(id))
        .cloned()
        .collect();

    if candidates.len() < 2 {
        return Err(format!(
            "There is nothing to halve: {} of the {} switched-on mods are required by another one, \
             so they stay on in every trial. Switch more mods on, or test the remaining one by hand.",
            always_on.len(),
            original_active.len()
        ));
    }

    let trial = half(&candidates);
    let session = BisectSession {
        started_at: now(),
        phase: BisectPhase::Narrowing,
        original_active,
        always_on,
        launches_left: launches_left(candidates.len()),
        candidates,
        trial,
        round: 1,
        history: Vec::new(),
        culprit: None,
        note: None,
    };

    // The session (with the list to put back) is on disk before the list is touched.
    save(user_zomboid_dir, &session)?;
    write_mod_list_ini(mod_list_ini_path, &trial_list(&session))?;
    Ok(session)
}

/// Takes the player's answer, narrows the suspects and switches the next trial on.
pub fn answer(
    mod_list_ini_path: &str,
    user_zomboid_dir: &str,
    still_happens: bool,
) -> Result<BisectSession, String> {
    let mut session = load(user_zomboid_dir)
        .ok_or_else(|| "There is no hunt in progress to answer.".to_string())?;
    if session.phase == BisectPhase::Done {
        return Ok(session);
    }

    session.history.push(BisectRound {
        round: session.round,
        tried: session.trial.len(),
        suspects: session.candidates.len(),
        answer: if still_happens { "still".to_string() } else { "gone".to_string() },
    });

    match session.phase {
        BisectPhase::Confirming => {
            if still_happens {
                session.culprit = session.trial.first().cloned();
            } else {
                let alone = session.trial.first().cloned().unwrap_or_default();
                session.note = Some(format!(
                    "{} on its own does not do it. Two mods together are doing this, and halving \
                     the list cannot separate a pair. Switch your list back on and try switching \
                     off {} together with whichever mod changes the same thing.",
                    alone, alone
                ));
            }
            session.phase = BisectPhase::Done;
            session.launches_left = 0;
        }
        BisectPhase::Narrowing => {
            session.candidates = narrow(&session.candidates, &session.trial, still_happens);
            session.round += 1;
            match session.candidates.len() {
                0 => {
                    session.phase = BisectPhase::Done;
                    session.launches_left = 0;
                    session.note = Some(
                        "No mod under suspicion is doing it. It is either one of the mods that had \
                         to stay on because another mod requires it, something outside the mod list \
                         (the game itself, a save, your settings), or it does not happen every time."
                            .to_string(),
                    );
                }
                1 => {
                    session.phase = BisectPhase::Confirming;
                    session.trial = session.candidates.clone();
                    session.launches_left = 1;
                }
                n => {
                    session.trial = half(&session.candidates);
                    session.launches_left = launches_left(n);
                }
            }
        }
        BisectPhase::Done => {}
    }

    // Done or not, the list on disk matches what the session says is on.
    let to_write = if session.phase == BisectPhase::Done {
        session.original_active.clone()
    } else {
        trial_list(&session)
    };
    write_mod_list_ini(mod_list_ini_path, &to_write)?;

    if session.phase == BisectPhase::Done {
        forget(user_zomboid_dir);
    } else {
        save(user_zomboid_dir, &session)?;
    }
    Ok(session)
}

/// Ends the hunt and puts the player's list back, optionally without one mod.
pub fn stop(
    mod_list_ini_path: &str,
    user_zomboid_dir: &str,
    switch_off: Option<&str>,
) -> Result<Vec<String>, String> {
    let session = load(user_zomboid_dir)
        .ok_or_else(|| "There is no hunt in progress to stop.".to_string())?;
    let restored: Vec<String> = session
        .original_active
        .iter()
        .filter(|id| switch_off.map(|off| *id != off).unwrap_or(true))
        .cloned()
        .collect();
    write_mod_list_ini(mod_list_ini_path, &restored)?;
    forget(user_zomboid_dir);
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mods switched on in the file the game reads, in order (`    mod = Id,` lines).
    fn active_in(ini: &str) -> Vec<String> {
        std::fs::read_to_string(ini)
            .unwrap()
            .lines()
            .filter_map(|l| l.trim().strip_prefix("mod = ").map(|r| r.trim_end_matches(',').to_string()))
            .collect()
    }

    fn m(id: &str, deps: &[&str], enabled: bool) -> BisectMod {
        BisectMod {
            mod_id: id.to_string(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            enabled,
        }
    }

    #[test]
    fn halving_rounds_up_so_a_single_suspect_is_still_tried() {
        let four: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        assert_eq!(half(&four), vec!["a".to_string(), "b".to_string()]);
        let three: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert_eq!(half(&three), vec!["a".to_string(), "b".to_string()]);
        let one = vec!["a".to_string()];
        assert_eq!(half(&one), one);
    }

    #[test]
    fn still_happening_keeps_what_was_on_and_gone_keeps_what_was_off() {
        let all: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        let on = vec!["a".to_string(), "b".to_string()];
        assert_eq!(narrow(&all, &on, true), on);
        assert_eq!(narrow(&all, &on, false), vec!["c".to_string(), "d".to_string()]);
    }

    #[test]
    fn a_hundred_mods_take_eight_launches() {
        assert_eq!(launches_left(100), 8);
        assert_eq!(launches_left(2), 2);
        assert_eq!(launches_left(1), 1);
    }

    #[test]
    fn requirements_stay_on_all_the_way_down() {
        let mods = vec![
            m("Gun", &["Framework"], true),
            // The game removes every backslash from a require value before matching it.
            m("Framework", &["\\Core"], true),
            m("Core", &[], true),
            m("Hats", &[], true),
            m("OffMod", &["Core"], false),
        ];
        let mut keep = always_on_set(&mods);
        keep.sort();
        assert_eq!(keep, vec!["Core".to_string(), "Framework".to_string()]);
    }

    #[test]
    fn a_hunt_narrows_to_one_mod_and_puts_the_list_back() {
        let dir = std::env::temp_dir().join(format!("pzms-bisect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("mods")).unwrap();
        let zomboid = dir.to_string_lossy().to_string();
        let ini = dir.join("mods").join("default.txt").to_string_lossy().to_string();

        let mods = vec![
            m("Lib", &[], true),
            m("A", &["Lib"], true),
            m("B", &[], true),
            m("C", &[], true),
            m("D", &[], true),
        ];
        let s = start(&ini, &zomboid, &mods).unwrap();
        // Lib is required by A, so it is never a suspect.
        assert_eq!(s.candidates, vec!["A", "B", "C", "D"]);
        assert_eq!(s.trial, vec!["A", "B"]);
        assert_eq!(active_in(&ini), vec!["Lib", "A", "B"]);

        // "Still happens" with A and B on, then with B alone: B is the one.
        let s = answer(&ini, &zomboid, true).unwrap();
        assert_eq!(s.candidates, vec!["A", "B"]);
        assert_eq!(s.trial, vec!["A"]);
        let s = answer(&ini, &zomboid, false).unwrap();
        assert_eq!(s.phase, BisectPhase::Confirming);
        assert_eq!(s.trial, vec!["B"]);
        let s = answer(&ini, &zomboid, true).unwrap();
        assert_eq!(s.phase, BisectPhase::Done);
        assert_eq!(s.culprit.as_deref(), Some("B"));
        // The hunt is over: the player's own list is back on disk, and nothing is left behind.
        assert_eq!(active_in(&ini), vec!["Lib", "A", "B", "C", "D"]);
        assert!(load(&zomboid).is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_hunt_that_finds_nothing_says_why() {
        let dir = std::env::temp_dir().join(format!("pzms-bisect-none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("mods")).unwrap();
        let zomboid = dir.to_string_lossy().to_string();
        let ini = dir.join("mods").join("default.txt").to_string_lossy().to_string();

        let mods = vec![m("A", &[], true), m("B", &[], true)];
        start(&ini, &zomboid, &mods).unwrap();
        // Gone with A on, and gone again with B on: no single mod does it.
        let s = answer(&ini, &zomboid, false).unwrap();
        assert_eq!(s.phase, BisectPhase::Confirming);
        let s = answer(&ini, &zomboid, false).unwrap();
        assert_eq!(s.phase, BisectPhase::Done);
        assert!(s.culprit.is_none());
        assert!(s.note.unwrap().contains("on its own does not do it"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_list_with_nothing_to_halve_is_refused() {
        let dir = std::env::temp_dir().join(format!("pzms-bisect-small-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("mods")).unwrap();
        let zomboid = dir.to_string_lossy().to_string();
        let ini = dir.join("mods").join("default.txt").to_string_lossy().to_string();

        let mods = vec![m("Only", &[], true), m("Off", &[], false)];
        let err = start(&ini, &zomboid, &mods).unwrap_err();
        assert!(err.contains("nothing to halve"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
