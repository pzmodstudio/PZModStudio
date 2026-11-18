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
use std::collections::BTreeMap;
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

//// A hunt in progress, as the Studio and the player see it.
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
    /// How suspect each candidate is (1 = nothing known). What this PC learned raises some at the
    /// start, so a trial holds about half the suspicion rather than half the mods; with `flaky`,
    /// a "no longer happens" lowers the ones that were on instead of clearing them.
    #[serde(default)]
    pub weights: BTreeMap<String, f64>,
    /// The mods the hunt started from because of what this PC learned, most suspect first.
    #[serde(default)]
    pub leads: Vec<String>,
    /// It does not happen every time: "no longer happens" is not proof.
    #[serde(default)]
    pub flaky: bool,
    /// Looking for the partner of this mod: it stays on, and the problem needs both.
    #[serde(default)]
    pub pair_with: Option<String>,
    /// The suspects at the start (for the pair search).
    #[serde(default)]
    pub all_suspects: Vec<String>,
    /// Text of the problem as the game's log writes it, to suggest each answer.
    #[serde(default)]
    pub target: Option<String>,
    /// Unix seconds when the current trial was switched on.
    #[serde(default)]
    pub trial_started: u64,
    /// What the log of the game played since the trial began suggests: "still" or "gone".
    /// Worked out when the hunt is read, never stored.
    #[serde(default, skip_deserializing)]
    pub suggested: Option<String>,
    /// The suspects in the order the search goes through them, the most suspect first.
    #[serde(default)]
    pub order: Vec<String>,
    /// `order[..lo]` was tried and does not do it; `order[..hi]` does (or is taken to).
    #[serde(default)]
    pub lo: usize,
    #[serde(default)]
    pub hi: usize,
    /// The trial on now is `order[..mid]`.
    #[serde(default)]
    pub mid: usize,
    /// A first "no longer happens" for this trial, when it does not happen every time.
    #[serde(default)]
    pub gone_once: bool,
    /// The trial on now has none of the suspects (only what must stay on): it tells whether this
    /// one, the most suspect, does it at all. "Still" means none of them does; "gone", this one.
    #[serde(default)]
    pub empty_check: Option<String>,
    /// `order[..hi]` was seen doing it (a trial answered "still"), not only taken to: the whole
    /// list is never tried on its own.
    #[serde(default)]
    pub hi_seen: bool,
}

/// How a hunt starts: what this PC already suspects, whether the problem shows every time, and
/// the log text that shows it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BisectOptions {
    #[serde(default)]
    pub priors: BTreeMap<String, f64>,
    #[serde(default)]
    pub flaky: bool,
    #[serde(default)]
    pub target: Option<String>,
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

fn weight(weights: &BTreeMap<String, f64>, id: &str) -> f64 {
    weights.get(id).copied().unwrap_or(1.0).max(0.0001)
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

/// The trial written to `default.txt`: always-on plus the suspects on trial (and, looking for a
/// pair, the mod whose partner is sought), in the player's own load order, because the order is
/// part of what is being tested.
fn trial_list(session: &BisectSession) -> Vec<String> {
    session
        .original_active
        .iter()
        .filter(|id| session.always_on.contains(id) || session.trial.contains(id) || session.pair_with.as_deref() == Some(id.as_str()))
        .cloned()
        .collect()
}

fn session_path(user_zomboid_dir: &str) -> PathBuf {
    Path::new(user_zomboid_dir).join("PZModStudio_Bisect.json")
}

/// The hunt on disk, if there is one, with the answer the log of the game played since the trial
/// began suggests. A file that cannot be understood is treated as none.
pub fn load(user_zomboid_dir: &str) -> Option<BisectSession> {
    let text = std::fs::read_to_string(session_path(user_zomboid_dir)).ok()?;
    let mut session: BisectSession = serde_json::from_str(&text).ok()?;
    if session.phase != BisectPhase::Done {
        let console = Path::new(user_zomboid_dir).join("console.txt");
        let written = std::fs::metadata(&console).ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
        if written > session.trial_started && session.trial_started > 0 {
            if let Ok(bytes) = std::fs::read(&console) {
                session.suggested = suggest(&session, &String::from_utf8_lossy(&bytes), written);
            }
        }
    }
    Some(session)
}

/// What a game's log says about the trial: the problem's text is in it ("still"), or the game
/// loaded a save and went on for three minutes, or closed, without it ("gone"). None: nothing to go
/// by yet, or no text to look for.
pub fn suggest(session: &BisectSession, console: &str, written: u64) -> Option<String> {
    let target = session.target.as_deref().map(str::trim).filter(|t| t.len() >= 3)?;
    if console.to_lowercase().contains(&target.to_lowercase()) {
        return Some("still".to_string());
    }
    let loaded = console.contains("game loading took ");
    let closed = console.contains("GameThread exited");
    let long_enough = written.saturating_sub(session.trial_started) >= 180;
    (loaded && (closed || long_enough)).then(|| "gone".to_string())
}

fn save(user_zomboid_dir: &str, session: &BisectSession) -> Result<(), String> {
    let path = session_path(user_zomboid_dir);
    let mut stored = session.clone();
    stored.suggested = None;
    let text = serde_json::to_string_pretty(&stored).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(&path, text).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

fn forget(user_zomboid_dir: &str) {
    let _ = std::fs::remove_file(session_path(user_zomboid_dir));
}

fn now() -> String {
    now_secs().to_string()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Starts a hunt and switches the first half on. The list on disk is replaced, with a backup, by
/// the trial; what was there is kept in the session to be put back.
pub fn start(mod_list_ini_path: &str, user_zomboid_dir: &str, mods: &[BisectMod]) -> Result<BisectSession, String> {
    start_with(mod_list_ini_path, user_zomboid_dir, mods, &BisectOptions::default())
}

//// The prefix trial: the suspects in `order[..mid]`, where `mid` splits the suspicion still in
/// `order[lo..hi]` about in half (at least one more than `lo`, at most one less than `hi`).
fn prefix_mid(order: &[String], lo: usize, hi: usize, weights: &BTreeMap<String, f64>) -> usize {
    if hi <= lo + 1 {
        return hi;
    }
    let total: f64 = order[lo..hi].iter().map(|c| weight(weights, c)).sum();
    let mut mass = 0.0;
    for (k, c) in order[lo..hi].iter().enumerate() {
        let w = weight(weights, c);
        // Stop before the one that takes it further past half than short of it.
        if k > 0 && (mass + w - total / 2.0) > (total / 2.0 - mass) {
            return lo + k;
        }
        mass += w;
    }
    hi - 1
}

/// Launches left to settle `order[lo..hi]` by halving, plus the one that confirms it.
fn left_for(lo: usize, hi: usize) -> u32 {
    launches_left(hi.saturating_sub(lo).max(1))
}

/// The trial for where the search stands: the confirming one when one suspect is left, else the
/// prefix that halves the suspicion left. Done when there is nothing left to search.
fn next_trial(session: &mut BisectSession) {
    let (lo, hi) = (session.lo, session.hi);
    if hi == 0 || hi <= lo {
        session.phase = BisectPhase::Done;
        session.launches_left = 0;
        session.note = Some(match &session.pair_with {
            Some(with) => format!("No other mod makes it happen together with {}. It may not happen every time, or it needs more than two mods.", with),
            None => "No mod under suspicion is doing it. It is either one of the mods that had \
                     to stay on because another mod requires it, something outside the mod list \
                     (the game itself, a save, your settings), or it does not happen every time."
                .to_string(),
        });
        return;
    }
    session.candidates = session.order[lo..hi].to_vec();
    session.empty_check = None;
    if hi == lo + 1 && lo == 0 {
        if session.pair_with.is_some() && session.hi_seen {
            // It and the partner sought were on in a trial that did it, and it alone did not:
            // the first of the order is the partner, with no launch more.
            session.culprit = Some(session.order[0].clone());
            session.phase = BisectPhase::Done;
            session.launches_left = 0;
            return;
        }
        if session.pair_with.is_none() {
            // The first of the order alone was on in a trial that did it; whether it is the one, or
            // none of them is, takes a trial without any.
            session.phase = BisectPhase::Confirming;
            session.empty_check = Some(session.order[0].clone());
            session.trial = Vec::new();
            session.launches_left = 1;
            return;
        }
    }
    if hi == lo + 1 {
        // The shortest list that does it ends with this one: on its own (with the partner when
        // looking for a pair), to be sure.
        session.phase = BisectPhase::Confirming;
        session.trial = vec![session.order[lo].clone()];
        session.launches_left = 1;
        return;
    }
    let mid = prefix_mid(&session.order, lo, hi, &session.weights);
    session.phase = BisectPhase::Narrowing;
    session.trial = session.order[..mid].to_vec();
    session.mid = mid;
    session.launches_left = left_for(lo, hi);
}

/// A hunt that starts from what is already suspected, and knows whether the problem shows every
/// time and which log text shows it.
///
/// The search: the suspects in an order, the most suspect first, and the shortest beginning of
/// that order that still does it. Its last mod is one that does it. On its own it either does it
/// (the culprit) or needs another: then that one stays on and the same search runs over the
/// mods before it, which finds its partner. A single culprit and a pair take about as many launches
/// as halving does; when the problem does not show every time, a "no longer happens" counts only
/// once it is said twice for the same trial.
pub fn start_with(mod_list_ini_path: &str, user_zomboid_dir: &str, mods: &[BisectMod], options: &BisectOptions) -> Result<BisectSession, String> {
    let original_active: Vec<String> = mods.iter().filter(|m| m.enabled).map(|m| m.mod_id.clone()).collect();
    let always_on = always_on_set(mods);
    let candidates: Vec<String> = original_active.iter().filter(|id| !always_on.contains(id)).cloned().collect();

    if candidates.len() < 2 {
        return Err(format!(
            "There is nothing to halve: {} of the {} switched-on mods are required by another one, \
             so they stay on in every trial. Switch more mods on, or test the remaining one by hand.",
            always_on.len(),
            original_active.len()
        ));
    }

    let weights: BTreeMap<String, f64> = candidates.iter().map(|c| (c.clone(), options.priors.get(c).copied().unwrap_or(1.0).max(1.0))).collect();
    // The most suspect first; the list's own order among equals.
    let mut order = candidates.clone();
    order.sort_by(|a, b| weight(&weights, b).partial_cmp(&weight(&weights, a)).unwrap_or(std::cmp::Ordering::Equal));
    let leads: Vec<String> = order.iter().filter(|c| weight(&weights, c) > 1.0).cloned().collect();
    let mut session = BisectSession {
        started_at: now(),
        phase: BisectPhase::Narrowing,
        original_active,
        always_on,
        launches_left: 0,
        all_suspects: candidates.clone(),
        candidates,
        trial: Vec::new(),
        round: 1,
        history: Vec::new(),
        culprit: None,
        note: None,
        weights,
        leads,
        flaky: options.flaky,
        pair_with: None,
        target: options.target.clone().filter(|t| !t.trim().is_empty()),
        trial_started: now_secs(),
        suggested: None,
        lo: 0,
        hi: order.len(),
        mid: 0,
        order,
        gone_once: false,
        empty_check: None,
        hi_seen: false,
    };
    next_trial(&mut session);

    // The session (with the list to put back) is on disk before the list is touched.
    save(user_zomboid_dir, &session)?;
    write_mod_list_ini(mod_list_ini_path, &trial_list(&session))?;
    Ok(session)
}

/// A hunt written before the search had an order (an older Studio): its suspects become the order.
fn upgrade(session: &mut BisectSession) {
    if session.order.is_empty() && !session.candidates.is_empty() {
        session.order = session.candidates.clone();
        session.lo = 0;
        session.hi = session.order.len();
        if session.all_suspects.is_empty() {
            session.all_suspects = session.candidates.clone();
        }
        if session.phase != BisectPhase::Done {
            next_trial(session);
        }
    }
}

/// Takes the player's answer, narrows the suspects and switches the next trial on.
pub fn answer(mod_list_ini_path: &str, user_zomboid_dir: &str, still_happens: bool) -> Result<BisectSession, String> {
    let mut session = load(user_zomboid_dir).ok_or_else(|| "There is no hunt in progress to answer.".to_string())?;
    session.suggested = None;
    if session.phase == BisectPhase::Done {
        return Ok(session);
    }
    upgrade(&mut session);

    session.history.push(BisectRound {
        round: session.round,
        tried: session.trial.len(),
        suspects: session.candidates.len(),
        answer: if still_happens { "still".to_string() } else { "gone".to_string() },
    });
    session.round += 1;

    // It does not show every time: a first "no longer happens" is tried once more.
    let repeat = session.flaky && !still_happens && !session.gone_once;
    if repeat {
        session.gone_once = true;
    } else {
        session.gone_once = false;
        match session.phase {
            BisectPhase::Confirming if session.empty_check.is_some() => {
                let first = session.empty_check.take().unwrap_or_default();
                session.phase = BisectPhase::Done;
                session.launches_left = 0;
                if still_happens {
                    session.note = Some(
                        "No mod under suspicion is doing it. It is either one of the mods that had \
                         to stay on because another mod requires it, something outside the mod list \
                         (the game itself, a save, your settings), or it does not happen every time."
                            .to_string(),
                    );
                } else {
                    session.culprit = Some(first);
                }
            }
            BisectPhase::Confirming => {
                let alone = session.trial.first().cloned().unwrap_or_default();
                if still_happens {
                    session.culprit = Some(alone);
                    session.phase = BisectPhase::Done;
                    session.launches_left = 0;
                } else if session.pair_with.is_none() && session.lo > 0 {
                    // On its own it does not do it: it needs one of the mods before it. That one is
                    // sought the same way, with this one on in every trial.
                    let before = session.order[..session.lo].to_vec();
                    session.pair_with = Some(alone);
                    session.order = before;
                    session.lo = 0;
                    session.hi = session.order.len();
                    next_trial(&mut session);
                } else {
                    session.phase = BisectPhase::Done;
                    session.launches_left = 0;
                    session.note = Some(match &session.pair_with {
                        Some(with) => format!(
                            "{} and {} together do not do it either. It may not happen every time, or it needs more than two mods.",
                            with, alone
                        ),
                        None => format!(
                            "{} on its own does not do it. Two mods together are doing this, and halving \
                             the list cannot separate a pair. Switch your list back on and try switching \
                             off {} together with whichever mod changes the same thing.",
                            alone, alone
                        ),
                    });
                }
            }
            BisectPhase::Narrowing => {
                if still_happens {
                    session.hi = session.mid;
                    session.hi_seen = true;
                } else {
                    session.lo = session.mid;
                }
                next_trial(&mut session);
            }
            BisectPhase::Done => {}
        }
    }
    session.trial_started = now_secs();

    // Done or not, the list on disk matches what the session says is on.
    let to_write = if session.phase == BisectPhase::Done { session.original_active.clone() } else { trial_list(&session) };
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
        // Gone with A on, gone again with B on: no single mod does it; then B is kept on to find
        // its partner, and A and B together do not do it either.
        let s = answer(&ini, &zomboid, false).unwrap();
        assert_eq!(s.phase, BisectPhase::Confirming);
        let s = answer(&ini, &zomboid, false).unwrap();
        assert_eq!((s.phase, s.pair_with.as_deref()), (BisectPhase::Confirming, Some("B")));
        assert_eq!(active_in(&ini), vec!["A", "B"], "the partner's trial has both on");
        let s = answer(&ini, &zomboid, false).unwrap();
        assert_eq!(s.phase, BisectPhase::Done);
        assert!(s.culprit.is_none());
        assert!(s.note.unwrap().contains("together do not do it either"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Plays a whole hunt against a pretend game: `happens` says, for the mods on, whether the
    /// problem shows. Returns the end and how many launches it took.
    fn hunt(tag: &str, mods: &[&str], options: BisectOptions, happens: impl Fn(&[String], u32) -> bool) -> (BisectSession, u32) {
        let dir = std::env::temp_dir().join(format!("pzms-bisect-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("mods")).unwrap();
        let zomboid = dir.to_string_lossy().to_string();
        let ini = dir.join("mods").join("default.txt").to_string_lossy().to_string();
        let list: Vec<BisectMod> = mods.iter().map(|id| m(id, &[], true)).collect();
        let mut s = start_with(&ini, &zomboid, &list, &options).unwrap();
        let mut launches = 0;
        while s.phase != BisectPhase::Done && launches < 60 {
            launches += 1;
            s = answer(&ini, &zomboid, happens(&active_in(&ini), launches)).unwrap();
        }
        let _ = std::fs::remove_dir_all(&dir);
        (s, launches)
    }

    fn names(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("M{:02}", i)).collect()
    }

    #[test]
    fn a_pair_that_fails_only_together_is_found() {
        let all = names(16);
        let ids: Vec<&str> = all.iter().map(String::as_str).collect();
        let (s, _) = hunt("pair", &ids, BisectOptions::default(), |on, _| on.iter().any(|m| m == "M03") && on.iter().any(|m| m == "M12"));
        let mut pair = vec![s.culprit.clone().unwrap_or_default(), s.pair_with.clone().unwrap_or_default()];
        pair.sort();
        assert_eq!(pair, vec!["M03".to_string(), "M12".to_string()]);
    }

    #[test]
    fn what_this_pc_suspects_is_tried_first_and_takes_fewer_launches() {
        let all = names(64);
        let ids: Vec<&str> = all.iter().map(String::as_str).collect();
        let culprit = |on: &[String], _: u32| on.iter().any(|m| m == "M50");
        let (plain, plain_launches) = hunt("plain", &ids, BisectOptions::default(), culprit);
        let priors: BTreeMap<String, f64> = [("M50".to_string(), 30.0)].into();
        let (led, led_launches) = hunt("led", &ids, BisectOptions { priors, ..Default::default() }, culprit);
        assert_eq!(plain.culprit.as_deref(), Some("M50"));
        assert_eq!(led.culprit.as_deref(), Some("M50"));
        assert_eq!(led.leads, vec!["M50".to_string()]);
        assert!(led_launches < plain_launches, "{} launches with the lead, {} without", led_launches, plain_launches);
    }

    #[test]
    fn a_problem_no_suspect_causes_names_no_one() {
        let all = names(10);
        let ids: Vec<&str> = all.iter().map(String::as_str).collect();
        // It happens whatever is on: the game, a save, a mod that must stay on.
        let led: BTreeMap<String, f64> = [("M04".to_string(), 20.0)].into();
        let (s, _) = hunt("nosuspect", &ids, BisectOptions { priors: led, ..Default::default() }, |_, _| true);
        assert_eq!(s.culprit, None, "the lead is not blamed");
        assert!(s.note.unwrap_or_default().contains("No mod under suspicion"));
    }

    #[test]
    fn a_problem_that_does_not_show_every_time_is_still_found() {
        let all = names(12);
        let ids: Vec<&str> = all.iter().map(String::as_str).collect();
        // M07 does it, but only on every other launch it is on.
        let (s, _) = hunt("flaky", &ids, BisectOptions { flaky: true, ..Default::default() }, |on, launch| on.iter().any(|m| m == "M07") && launch % 2 == 0);
        assert_eq!(s.culprit.as_deref(), Some("M07"), "note: {:?}", s.note);
    }

    #[test]
    fn the_log_of_the_trial_suggests_the_answer() {
        let mut s = BisectSession {
            started_at: String::new(),
            phase: BisectPhase::Narrowing,
            original_active: vec![],
            always_on: vec![],
            candidates: vec![],
            trial: vec![],
            round: 1,
            launches_left: 1,
            history: vec![],
            culprit: None,
            note: None,
            weights: BTreeMap::new(),
            leads: vec![],
            flaky: false,
            pair_with: None,
            all_suspects: vec![],
            target: Some("attempted index: getDuration".into()),
            trial_started: 1000,
            suggested: None,
            order: vec![],
            lo: 0,
            hi: 0,
            mid: 0,
            gone_once: false,
            empty_check: None,
            hi_seen: false,
        };
        assert_eq!(suggest(&s, "...
ERROR: attempted index: getDuration of non-table
", 1100).as_deref(), Some("still"));
        assert_eq!(suggest(&s, "game loading took 30 seconds
", 1100), None, "a minute and a half in, not enough to say");
        assert_eq!(suggest(&s, "game loading took 30 seconds
", 1300).as_deref(), Some("gone"));
        s.target = None;
        assert_eq!(suggest(&s, "attempted index: getDuration", 1300), None);
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
