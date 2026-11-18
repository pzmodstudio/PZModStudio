//! The Studio's "brain": what it can tell from the sessions this PC learned (`rules::learned`).
//!
//! The learner keeps facts: which mods each session loaded and in which order, how far it got,
//! how it ended, what loading took, and the errors the game blamed on each mod. This module puts
//! them side by side:
//!
//! - **What changed**: when an error began, the last session with the mod on that went without it
//!   and what was different in the first one with it (a mod added, removed or updated, its place
//!   in the order, the game's build).
//! - **Fixes remembered**: each fix Diagnostics applies is noted (`PZModStudio_BrainFixes.json`)
//!   and the sessions after it say whether the mod's error stopped.
//! - **Before playing**: the last sessions with (nearly) the list about to be played; when they
//!   failed to start or ran out of memory, what is different from the last one that worked.
//! - **A mod's history**, **loading that got slower**, and **a game update** that made several
//!   mods fail the same week.
//!
//! Everything said comes with the sessions it was read from; nothing is guessed and nothing is sent.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::learned::{self, Learned, LearnedSession, UNKNOWN};

const FIXES_FILE: &str = "PZModStudio_BrainFixes.json";
/// How alike two lists of mods must be to count as "the same list" (shared / all).
const SAME_LIST: f32 = 0.85;
/// Mods named in a list of changes before "and N more".
const NAMED: usize = 6;

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// One difference between two sessions (or a session and the list about to be played).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Change {
    /// "build", "added", "removed", "updated", "moved".
    pub kind: String,
    #[serde(default)]
    pub mod_id: String,
    /// build: from / to; moved: positions; updated: versions when known.
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
}

/// What was different when something began.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Onset {
    /// The first session with it.
    pub first_bad: u64,
    /// The last session before it that went without it (with the mod on, for a mod's error).
    pub last_good: Option<u64>,
    pub changes: Vec<Change>,
    /// Changes left out of `changes` (more than [`NAMED`] of one kind).
    pub more: u32,
}

/// What the Studio knows of the mods now: version and when its files last changed.
pub struct ModsNow {
    pub version: BTreeMap<String, String>,
    pub changed: BTreeMap<String, u64>,
    /// Mod id -> the mods its mod.info requires.
    pub requires: BTreeMap<String, Vec<String>>,
}

impl ModsNow {
    /// How close `other` is to `focus`: 0 the mod itself, 1 one requires the other, 2 any other.
    fn closeness(&self, focus: Option<&str>, other: &str) -> u8 {
        let Some(f) = focus else { return 2 };
        if f == other {
            return 0;
        }
        let needs = |a: &str, b: &str| self.requires.get(a).is_some_and(|r| r.iter().any(|x| x == b));
        if needs(f, other) || needs(other, f) {
            1
        } else {
            2
        }
    }
}

impl ModsNow {
    pub fn of(catalog: &crate::catalog::ModCatalog) -> ModsNow {
        let all = learned::installed_mods(catalog);
        ModsNow {
            version: all.iter().map(|(k, v)| (k.clone(), v.version.clone())).collect(),
            changed: all.iter().map(|(k, v)| (k.clone(), v.changed)).collect(),
            requires: catalog
                .mods
                .iter()
                .filter(|m| m.shadowed_by.is_none())
                .map(|m| (m.id.clone(), m.info.require.iter().map(|r| r.trim().trim_start_matches('\\').to_string()).collect()))
                .collect(),
        }
    }
}

/// The mods a session loaded, in order, the Studio's own left out.
fn loaded_ids<'a>(l: &'a Learned, s: &LearnedSession) -> Vec<&'a str> {
    s.loaded.iter().filter_map(|i| l.ids.get(*i as usize).map(String::as_str)).filter(|m| !crate::security::is_studio_mod(m)).collect()
}

fn list_of<'a>(l: &'a Learned, s: &LearnedSession) -> BTreeSet<&'a str> {
    loaded_ids(l, s).into_iter().collect()
}

/// The version of a mod a session recorded, when known.
fn version_in(l: &Learned, s: &LearnedSession, mod_id: &str) -> Option<String> {
    s.mods
        .iter()
        .filter_map(|i| l.mods.get(*i as usize))
        .find(|(m, v)| m == mod_id && v != UNKNOWN)
        .map(|(_, v)| v.clone())
}

/// The differences from `before` to `after` (ids in load order), `focus` first.
fn diff_lists(
    before: &[&str],
    after: &[&str],
    focus: Option<&str>,
    updated: &dyn Fn(&str) -> Option<(String, String)>,
    close: &dyn Fn(&str) -> u8,
) -> (Vec<Change>, u32) {
    let b: BTreeSet<&str> = before.iter().copied().collect();
    let a: BTreeSet<&str> = after.iter().copied().collect();
    let mut out = Vec::new();
    let mut more = 0u32;
    let take = |kind: &str, ids: Vec<&str>, out: &mut Vec<Change>, more: &mut u32| {
        let mut ids = ids;
        // the mod the question is about first, then the mods it requires or that require it
        ids.sort_by_key(|m| close(m));
        for (n, m) in ids.iter().enumerate() {
            if n < NAMED {
                out.push(Change { kind: kind.to_string(), mod_id: m.to_string(), from: String::new(), to: String::new() });
            } else {
                *more += 1;
            }
        }
    };
    let mut updated_list: Vec<(String, (String, String))> = Vec::new();
    for m in after.iter().filter(|m| b.contains(*m)) {
        if let Some(v) = updated(m) {
            updated_list.push((m.to_string(), v));
        }
    }
    updated_list.sort_by_key(|(m, _)| close(m));
    for (n, (m, (from, to))) in updated_list.into_iter().enumerate() {
        if n < NAMED {
            out.push(Change { kind: "updated".into(), mod_id: m, from, to });
        } else {
            more += 1;
        }
    }
    take("added", after.iter().copied().filter(|m| !b.contains(m)).collect(), &mut out, &mut more);
    take("removed", before.iter().copied().filter(|m| !a.contains(m)).collect(), &mut out, &mut more);
    // the focus mod's place among the mods both had on
    if let Some(f) = focus {
        let common_b: Vec<&str> = before.iter().copied().filter(|m| a.contains(m)).collect();
        let common_a: Vec<&str> = after.iter().copied().filter(|m| b.contains(m)).collect();
        if let (Some(pb), Some(pa)) = (common_b.iter().position(|m| *m == f), common_a.iter().position(|m| *m == f)) {
            if pb != pa {
                out.push(Change { kind: "moved".into(), mod_id: f.to_string(), from: (pb + 1).to_string(), to: (pa + 1).to_string() });
            }
        }
    }
    (out, more)
}

/// The differences between two sessions.
pub fn changes_between(l: &Learned, good: &LearnedSession, bad: &LearnedSession, focus: Option<&str>, now: &ModsNow) -> (Vec<Change>, u32) {
    let before = loaded_ids(l, good);
    let after = loaded_ids(l, bad);
    let updated = |m: &str| -> Option<(String, String)> {
        let (vb, va) = (version_in(l, good, m), version_in(l, bad, m));
        if let (Some(vb), Some(va)) = (&vb, &va) {
            if vb != va {
                return Some((vb.clone(), va.clone()));
            }
        }
        // its files changed between the two sessions (the Workshop updated it)
        let changed = now.changed.get(m).copied().unwrap_or(0);
        (changed > good.started && changed <= bad.started).then(|| (vb.unwrap_or_default(), va.unwrap_or_default()))
    };
    let close = |m: &str| now.closeness(focus, m);
    let (mut changes, more) = diff_lists(&before, &after, focus, &updated, &close);
    if !good.build.is_empty() && good.build != bad.build {
        changes.insert(0, Change { kind: "build".into(), mod_id: String::new(), from: good.build.clone(), to: bad.build.clone() });
    }
    (changes, more)
}

/// Whether a session has an error of `mod_id` (any version) with this key (any key when None).
fn has_error(l: &Learned, s: &LearnedSession, mod_id: &str, key: Option<&str>) -> bool {
    s.errors.iter().filter_map(|i| l.errors.get(*i as usize)).any(|e| e.mod_id == mod_id && key.is_none_or(|k| e.key == k))
}

fn mod_on(l: &Learned, s: &LearnedSession, mod_id: &str) -> bool {
    s.loaded.iter().any(|i| l.ids.get(*i as usize).is_some_and(|m| m == mod_id))
}

/// When a mod's error began and what changed then.
pub fn error_onset(l: &Learned, mod_id: &str, key: &str, now: &ModsNow) -> Option<Onset> {
    let with_mod: Vec<&LearnedSession> = l.sessions.iter().filter(|s| mod_on(l, s, mod_id)).collect();
    let first_bad_at = with_mod.iter().position(|s| has_error(l, s, mod_id, Some(key)))?;
    let first_bad = with_mod[first_bad_at];
    // The last session that could have shown it (got as far as the one that did) and went without
    // it; one with nearly the same list first, so the differences are the ones that matter.
    let could_show = |s: &&&LearnedSession| s.kind == first_bad.kind && !s.failed_to_start(u64::MAX) && (first_bad.reached != "game" || s.reached == "game");
    let bad_list = list_of(l, first_bad);
    let earlier: Vec<&&LearnedSession> = with_mod[..first_bad_at].iter().filter(could_show).collect();
    let same_build = |s: &&&LearnedSession| learned::same_build(&s.build, &first_bad.build);
    let similar = |s: &&&LearnedSession| likeness(&list_of(l, s), &bad_list) >= SAME_LIST;
    let last_good = earlier
        .iter()
        .rev()
        .find(|s| same_build(s) && similar(s))
        .or_else(|| earlier.iter().rev().find(|s| same_build(s)))
        .or_else(|| earlier.iter().rev().find(|s| similar(s)))
        .or_else(|| earlier.last())
        .map(|s| **s);
    let Some(good) = last_good else {
        // From its first session on: it came with the mod.
        return Some(Onset { first_bad: first_bad.started, last_good: None, changes: vec![], more: 0 });
    };
    let (changes, more) = changes_between(l, good, first_bad, Some(mod_id), now);
    Some(Onset { first_bad: first_bad.started, last_good: Some(good.started), changes, more })
}

// ---------------------------------------------------------------------------------------------
// Fixes remembered
// ---------------------------------------------------------------------------------------------

/// A fix Diagnostics applied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixRecord {
    /// Unix seconds.
    pub at: u64,
    pub mod_id: String,
    /// The fix's action (disable_mod, make_compatible_copy, enable_shim_patch, move_before...).
    pub action: String,
    /// The finding's code it was applied for.
    #[serde(default)]
    pub code: String,
    /// The learned error it was for, when it was one.
    #[serde(default)]
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixOutcome {
    pub fix: FixRecord,
    /// Sessions with the mod on before the fix (the last ten) and how many had its error.
    pub before: u32,
    pub before_bad: u32,
    /// Sessions with the mod on after the fix and how many had its error.
    pub after: u32,
    pub after_bad: u32,
    /// "waiting" (fewer than 2 sessions since), "worked", "not_worked", "mod_off" (switched off and
    /// not played since), "no_errors_before" (nothing to compare).
    pub verdict: String,
}

fn fixes_path(zomboid: &Path) -> std::path::PathBuf {
    zomboid.join(FIXES_FILE)
}

pub fn load_fixes(zomboid: &Path) -> Vec<FixRecord> {
    std::fs::read_to_string(fixes_path(zomboid)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn note_fix(zomboid: &Path, mod_id: &str, action: &str, code: &str, key: Option<&str>) -> Result<(), String> {
    let mut all = load_fixes(zomboid);
    all.push(FixRecord { at: now_secs(), mod_id: mod_id.to_string(), action: action.to_string(), code: code.to_string(), key: key.map(str::to_string) });
    if all.len() > 300 {
        let drop = all.len() - 300;
        all.drain(..drop);
    }
    std::fs::write(fixes_path(zomboid), serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub fn fix_outcome(l: &Learned, fix: &FixRecord) -> FixOutcome {
    let key = fix.key.as_deref();
    let with_mod = |s: &&LearnedSession| mod_on(l, s, &fix.mod_id);
    let before: Vec<&LearnedSession> = l.sessions.iter().filter(|s| s.started < fix.at).filter(with_mod).collect();
    let before: Vec<&LearnedSession> = before.into_iter().rev().take(10).collect();
    let after: Vec<&LearnedSession> = l.sessions.iter().filter(|s| s.started >= fix.at).filter(with_mod).collect();
    let bad = |s: &LearnedSession| has_error(l, s, &fix.mod_id, key);
    let before_bad = before.iter().filter(|s| bad(s)).count() as u32;
    let after_bad = after.iter().filter(|s| bad(s)).count() as u32;
    let verdict = if after.is_empty() && fix.action == "disable_mod" {
        "mod_off"
    } else if after.len() < 2 {
        "waiting"
    } else if before_bad == 0 {
        "no_errors_before"
    } else if after_bad == 0 {
        "worked"
    } else {
        "not_worked"
    };
    FixOutcome { fix: fix.clone(), before: before.len() as u32, before_bad, after: after.len() as u32, after_bad, verdict: verdict.to_string() }
}

// ---------------------------------------------------------------------------------------------
// Before playing
// ---------------------------------------------------------------------------------------------

fn likeness(a: &BTreeSet<&str>, b: &BTreeSet<&str>) -> f32 {
    let union = a.union(b).count();
    if union == 0 {
        return 1.0;
    }
    a.intersection(b).count() as f32 / union as f32
}

/// The warning shown before launching: the last sessions with (nearly) this list went badly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Prelaunch {
    /// The last sessions with this list looked at, how many went badly, and how: failed to start,
    /// out of memory (the game's limit, or Windows had none left), Java fell over (its crash report).
    pub looked_at: u32,
    #[serde(default)]
    pub bad: u32,
    pub failed_to_start: u32,
    pub out_of_memory: u32,
    #[serde(default)]
    pub crashed: u32,
    pub last_bad: u64,
    /// The last session with this list that got into a save, and what is different now.
    pub last_good: Option<u64>,
    pub changes: Vec<Change>,
    pub more: u32,
}

pub fn prelaunch(l: &Learned, active: &[String], build: &str, now: &ModsNow) -> Option<Prelaunch> {
    let t = now_secs();
    let want: BTreeSet<&str> = active.iter().map(String::as_str).filter(|m| !crate::security::is_studio_mod(m)).collect();
    if want.is_empty() {
        return None;
    }
    let similar: Vec<&LearnedSession> = l
        .sessions
        .iter()
        .filter(|s| s.kind == "client" && learned::same_build(&s.build, build) && !s.is_live(t))
        .filter(|s| {
            likeness(&want, &list_of(l, s)) >= SAME_LIST
        })
        .collect();
    let last3: Vec<&&LearnedSession> = similar.iter().rev().take(3).collect();
    let crashed = |s: &LearnedSession| l.crash_of(s).is_some();
    let failed = last3.iter().filter(|s| s.failed_to_start(t)).count() as u32;
    let oom = last3.iter().filter(|s| s.oom || l.crash_of(s).is_some_and(|c| c.kind == "out_of_memory")).count() as u32;
    let fatal = last3.iter().filter(|s| l.crash_of(s).is_some_and(|c| c.kind == "fatal")).count() as u32;
    let bad = last3.iter().filter(|s| s.failed_to_start(t) || s.oom || crashed(s)).count() as u32;
    if bad < 2 {
        return None;
    }
    let last_bad = last3.iter().filter(|s| s.failed_to_start(t) || s.oom || crashed(s)).map(|s| s.started).max().unwrap_or(0);
    let good = similar.iter().rev().find(|s| s.reached == "game" && !s.oom && !crashed(s)).copied();
    let (changes, more) = match good {
        Some(g) => {
            let before = loaded_ids(l, g);
            let after: Vec<&str> = active.iter().map(String::as_str).collect();
            let updated = |m: &str| -> Option<(String, String)> {
                let changed = now.changed.get(m).copied().unwrap_or(0);
                (changed > g.started).then(|| (version_in(l, g, m).unwrap_or_default(), now.version.get(m).cloned().unwrap_or_default()))
            };
            let (mut c, more) = diff_lists(&before, &after, None, &updated, &|_| 2);
            if !learned::same_build(&g.build, build) || g.build.split('.').count() > 2 && !build.is_empty() && !g.build.starts_with(build) {
                c.insert(0, Change { kind: "build".into(), mod_id: String::new(), from: g.build.clone(), to: build.to_string() });
            }
            (c, more)
        }
        None => (vec![], 0),
    };
    Some(Prelaunch { looked_at: last3.len() as u32, bad, failed_to_start: failed, out_of_memory: oom, crashed: fatal, last_bad, last_good: good.map(|g| g.started), changes, more })
}

// ---------------------------------------------------------------------------------------------
// A mod's history
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionSpan {
    pub version: String,
    pub sessions: u32,
    pub first: u64,
    pub last: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModError {
    pub title: String,
    pub code: String,
    pub params: BTreeMap<String, String>,
    pub key: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub sample: String,
    /// Sessions with it / with the mod on since it first showed, and the last ones' verdict.
    pub seen: u32,
    pub on: u32,
    pub first: u64,
    pub last: u64,
    pub stopped: bool,
    pub onset: Option<Onset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ModHistory {
    pub mod_id: String,
    pub sessions: u32,
    pub first: Option<u64>,
    pub last: Option<u64>,
    /// Its place in the last session's load order (1-based) and how many mods there were.
    pub place: Option<(u32, u32)>,
    pub versions: Vec<VersionSpan>,
    pub errors: Vec<ModError>,
    pub fixes: Vec<FixOutcome>,
    /// Sessions with it on that failed to start / ran out of memory / Java fell over.
    pub failed_to_start: u32,
    pub out_of_memory: u32,
    #[serde(default)]
    pub crashed: u32,
}

pub fn mod_history(l: &Learned, fixes: &[FixRecord], mod_id: &str, now: &ModsNow) -> ModHistory {
    let t = now_secs();
    let on: Vec<&LearnedSession> = l.sessions.iter().filter(|s| mod_on(l, s, mod_id)).collect();
    let mut h = ModHistory { mod_id: mod_id.to_string(), sessions: on.len() as u32, ..Default::default() };
    h.first = on.first().map(|s| s.started);
    h.last = on.last().map(|s| s.started);
    if let Some(s) = on.last() {
        let ids = loaded_ids(l, s);
        h.place = ids.iter().position(|m| *m == mod_id).map(|p| (p as u32 + 1, ids.len() as u32));
    }
    h.failed_to_start = on.iter().filter(|s| s.failed_to_start(t)).count() as u32;
    h.out_of_memory = on.iter().filter(|s| s.oom || l.crash_of(s).is_some_and(|c| c.kind == "out_of_memory")).count() as u32;
    h.crashed = on.iter().filter(|s| l.crash_of(s).is_some_and(|c| c.kind == "fatal")).count() as u32;
    let mut spans: Vec<VersionSpan> = Vec::new();
    for s in &on {
        let Some(v) = version_in(l, s, mod_id) else { continue };
        match spans.iter_mut().find(|x| x.version == v) {
            Some(x) => {
                x.sessions += 1;
                x.last = s.started;
            }
            None => spans.push(VersionSpan { version: v, sessions: 1, first: s.started, last: s.started }),
        }
    }
    h.versions = spans;
    // its errors, one per key (any version)
    let mut keys: Vec<(String, usize)> = Vec::new();
    for (i, e) in l.errors.iter().enumerate() {
        if e.mod_id == mod_id && !keys.iter().any(|(k, _)| *k == e.key) {
            keys.push((e.key.clone(), i));
        }
    }
    for (key, i) in keys {
        let e = &l.errors[i];
        let bad: Vec<&&LearnedSession> = on.iter().filter(|s| has_error(l, s, mod_id, Some(&key))).collect();
        let Some(first) = bad.first().map(|s| s.started) else { continue };
        let since: Vec<&&LearnedSession> = on.iter().filter(|s| s.started >= first).collect();
        let stopped = since.len() >= learned::CLOSED_AFTER && since.iter().rev().take(learned::CLOSED_AFTER).all(|s| !has_error(l, s, mod_id, Some(&key)));
        h.errors.push(ModError {
            title: e.title.clone(),
            code: e.code.clone(),
            params: e.params.clone(),
            key: key.clone(),
            file: e.file.clone(),
            line: e.line,
            sample: e.sample.clone(),
            seen: bad.len() as u32,
            on: since.len() as u32,
            first,
            last: bad.last().map(|s| s.started).unwrap_or(first),
            stopped,
            onset: error_onset(l, mod_id, &key, now),
        });
    }
    h.errors.sort_by(|a, b| b.last.cmp(&a.last));
    h.fixes = fixes.iter().filter(|f| f.mod_id == mod_id).map(|f| fix_outcome(l, f)).collect();
    h
}

// ---------------------------------------------------------------------------------------------
// What the brain noticed
// ---------------------------------------------------------------------------------------------

/// One thing worth saying, with the values its text is made of.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Insight {
    /// "error_onset", "load_slower", "game_update", "failed_to_start", "out_of_memory".
    pub code: String,
    #[serde(default)]
    pub mod_id: String,
    pub params: BTreeMap<String, String>,
    #[serde(default)]
    pub onset: Option<Onset>,
    /// Mods it is about (game_update).
    #[serde(default)]
    pub mods: Vec<String>,
}

fn median(v: &mut [u32]) -> u32 {
    v.sort_unstable();
    v.get(v.len() / 2).copied().unwrap_or(0)
}

/// Loading a save takes much longer in the last sessions than before, and what changed then.
pub fn load_slower(l: &Learned, build: &str, now: &ModsNow) -> Option<Insight> {
    let loads: Vec<&LearnedSession> = l.sessions.iter().filter(|s| s.kind == "client" && learned::same_build(&s.build, build) && s.load_secs.is_some()).collect();
    if loads.len() < 6 {
        return None;
    }
    let (old, new) = loads.split_at(loads.len() - 3);
    let old = &old[old.len().saturating_sub(7)..];
    let mut o: Vec<u32> = old.iter().filter_map(|s| s.load_secs).collect();
    let mut n: Vec<u32> = new.iter().filter_map(|s| s.load_secs).collect();
    let (mo, mn) = (median(&mut o), median(&mut n));
    if mo == 0 || mn < mo + 30 || (mn as f32) < mo as f32 * 1.5 {
        return None;
    }
    // the session where it turned slow, and the fast one just before
    let slow_at = loads.iter().rposition(|s| s.load_secs.unwrap_or(0) as f32 <= mo as f32 * 1.2)?;
    let (good, bad) = (loads[slow_at], *loads.get(slow_at + 1)?);
    let (changes, more) = changes_between(l, good, bad, None, now);
    let mut params = BTreeMap::new();
    params.insert("before".into(), mo.to_string());
    params.insert("now".into(), mn.to_string());
    params.insert("since".into(), bad.started.to_string());
    Some(Insight { code: "load_slower".into(), mod_id: String::new(), params, onset: Some(Onset { first_bad: bad.started, last_good: Some(good.started), changes, more }), mods: vec![] })
}

/// A game update after which three or more mods began failing, none of them updated in between:
/// the game changed, not the mods.
pub fn game_update(l: &Learned, build: &str, now: &ModsNow) -> Option<Insight> {
    let cut = |b: &str| b.split('.').take(2).collect::<Vec<_>>().join(".");
    let first_new = l.sessions.iter().find(|s| learned::same_build(&s.build, build))?;
    let before: Vec<&LearnedSession> = l.sessions.iter().filter(|s| s.started < first_new.started && !learned::same_build(&s.build, build)).collect();
    let last_old = before.last()?;
    let new: Vec<&LearnedSession> = l.sessions.iter().filter(|s| learned::same_build(&s.build, build)).take(5).collect();
    let mut mods: BTreeSet<String> = BTreeSet::new();
    for s in &new {
        for e in s.errors.iter().filter_map(|i| l.errors.get(*i as usize)) {
            let had_it_before = before.iter().any(|b| has_error(l, b, &e.mod_id, Some(&e.key)));
            let was_on_before = before.iter().rev().take(10).any(|b| mod_on(l, b, &e.mod_id));
            let changed = now.changed.get(&e.mod_id).copied().unwrap_or(0);
            let updated_between = changed > last_old.started && changed <= s.started;
            if was_on_before && !had_it_before && !updated_between {
                mods.insert(e.mod_id.clone());
            }
        }
    }
    if mods.len() < 3 {
        return None;
    }
    let mut params = BTreeMap::new();
    params.insert("from".into(), cut(&last_old.build));
    params.insert("to".into(), cut(&first_new.build));
    params.insert("count".into(), mods.len().to_string());
    params.insert("since".into(), first_new.started.to_string());
    Some(Insight { code: "game_update".into(), mod_id: String::new(), params, onset: None, mods: mods.into_iter().collect() })
}

/// One session, briefly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionBrief {
    pub started: u64,
    pub ended: u64,
    pub kind: String,
    pub build: String,
    pub reached: String,
    pub clean_exit: bool,
    pub oom: bool,
    pub load_secs: Option<u32>,
    pub mods: u32,
    pub errors: u32,
    pub live: bool,
    /// Java's own crash report for it: "out_of_memory" or "fatal" ("" when none).
    #[serde(default)]
    pub crash: String,
    #[serde(default)]
    pub crash_reason: String,
    #[serde(default)]
    pub crash_frame: String,
    /// What that library is, in plain words (see learned::frame_cause).
    #[serde(default)]
    pub crash_cause: String,
    /// From the Link's diary: the save it loaded, and that the player went back to the menu.
    #[serde(default)]
    pub save: String,
    #[serde(default)]
    pub quit_to_menu: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrainOverview {
    pub summary: learned::LearnedSummary,
    pub recent: Vec<SessionBrief>,
    pub insights: Vec<Insight>,
    pub fixes: Vec<FixOutcome>,
    pub prelaunch: Option<Prelaunch>,
    /// The last game session that ended, told: how it went and what was new in it.
    #[serde(default)]
    pub last_report: Option<SessionReport>,
    /// Java's crash reports in the game's folder, newest first (at most 20).
    #[serde(default)]
    pub crashes: Vec<learned::JvmCrash>,
    /// Mod id -> its name, for every mod the overview names.
    pub names: BTreeMap<String, String>,
}

pub fn overview(zomboid: &Path, catalog: &crate::catalog::ModCatalog, active: &[String]) -> BrainOverview {
    let l = learned::load(zomboid);
    let build = catalog.game_version.clone().unwrap_or_default();
    let now = ModsNow::of(catalog);
    let t = now_secs();
    let summary = learned::summary(zomboid, catalog);
    let mut insights = Vec::new();
    for issue in summary.issues.iter().filter(|i| !i.closed && active.contains(&i.mod_id)) {
        if let Some(onset) = error_onset(&l, &issue.mod_id, &issue.key, &now) {
            let mut params = BTreeMap::new();
            params.insert("title".into(), issue.title.clone());
            params.insert("cardCode".into(), issue.code.clone());
            params.insert("cardParams".into(), serde_json::to_string(&issue.params).unwrap_or_default());
            params.insert("seen".into(), issue.seen.to_string());
            params.insert("on".into(), issue.on.to_string());
            insights.push(Insight { code: "error_onset".into(), mod_id: issue.mod_id.clone(), params, onset: Some(onset), mods: vec![] });
        }
    }
    insights.extend(load_slower(&l, &build, &now));
    insights.extend(game_update(&l, &build, &now));
    let fixes: Vec<FixOutcome> = load_fixes(zomboid).iter().rev().take(30).map(|f| fix_outcome(&l, f)).collect();
    let recent = l
        .sessions
        .iter()
        .rev()
        .take(15)
        .map(|s| SessionBrief {
            started: s.started,
            ended: s.last_line,
            kind: s.kind.clone(),
            build: s.build.clone(),
            reached: s.reached.clone(),
            clean_exit: s.clean_exit,
            oom: s.oom,
            load_secs: s.load_secs,
            mods: s.loaded.len() as u32,
            errors: s.errors.len() as u32,
            live: s.is_live(t),
            crash: l.crash_of(s).map(|c| c.kind.clone()).unwrap_or_default(),
            crash_reason: l.crash_of(s).map(|c| c.reason.clone()).unwrap_or_default(),
            crash_frame: l.crash_of(s).map(|c| c.frame.clone()).unwrap_or_default(),
            crash_cause: l.crash_of(s).map(|c| crate::rules::learned::frame_cause(&c.frame).to_string()).unwrap_or_default(),
            save: l.diary_of(s).0,
            quit_to_menu: l.diary_of(s).1,
        })
        .collect();
    let prelaunch = prelaunch(&l, active, &build, &now);
    let mut names = BTreeMap::new();
    let mut name = |id: &str| {
        if !id.is_empty() {
            let n = catalog.get(id).map(|m| m.name.clone()).unwrap_or_else(|| id.to_string());
            names.insert(id.to_string(), n);
        }
    };
    for i in &insights {
        name(&i.mod_id);
        i.mods.iter().for_each(|m| name(m));
        if let Some(o) = &i.onset {
            o.changes.iter().for_each(|c| name(&c.mod_id));
        }
    }
    for f in &fixes {
        name(&f.fix.mod_id);
    }
    if let Some(p) = &prelaunch {
        p.changes.iter().for_each(|c| name(&c.mod_id));
    }
    for i in &summary.issues {
        name(&i.mod_id);
    }
    let crashes = l.crashes.iter().rev().take(20).cloned().map(|mut c| {
        c.cause = learned::frame_cause(&c.frame).to_string();
        c
    }).collect();
    let last_report = l.sessions.iter().rev().find(|s| s.kind == "client" && !s.is_live(t)).map(|s| session_report(&l, &load_fixes(zomboid), s));
    BrainOverview { summary, recent, insights, fixes, prelaunch, last_report, crashes, names }
}

/// Sessions with the error that make a learned error worth proposing for the app's own index.
pub const DRAFT_SESSIONS: u32 = 5;

/// What this PC confirmed, written as entries of the mod index the app is built with
/// (community/index/<id>.json), for whoever keeps that index to read and decide: each open
/// learned error seen in [`DRAFT_SESSIONS`] or more sessions of the installed version, that the
/// index does not say yet. The note and the fix are left for a person to write.
pub fn index_drafts(zomboid: &Path, catalog: &crate::catalog::ModCatalog) -> Vec<serde_json::Value> {
    let index = crate::rules::index::current(&zomboid.to_string_lossy());
    let mut by_mod: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for i in learned::current_issues(zomboid, catalog).into_iter().filter(|i| !i.closed && i.seen >= DRAFT_SESSIONS) {
        let installed = catalog.get(&i.mod_id).and_then(|m| m.info.mod_version.as_deref());
        if index.get(&i.mod_id).is_some_and(|r| learned::index_says_it(&i, r, installed)) {
            continue;
        }
        let date = |t: u64| chrono::DateTime::<chrono::Local>::from(std::time::UNIX_EPOCH + std::time::Duration::from_secs(t)).format("%Y-%m-%d").to_string();
        let mut issue = serde_json::json!({
            "title": i.title,
            "note": "",
            "log": i.sample,
            "fix": { "kind": "none" },
            "build": i.build,
            "date": date(i.last_seen),
            "source": format!("PZ Mod Studio, game log: in {} of {} sessions with the mod on {} ({} to {})", i.seen, i.on, i.build, date(i.first_seen), date(i.last_seen)),
            "status": "verified",
        });
        if let Some(f) = &i.file {
            issue["file"] = serde_json::json!(f);
        }
        if let Some(l) = i.line {
            issue["line"] = serde_json::json!(l);
        }
        if i.version != learned::UNKNOWN && !i.version.starts_with("files of ") {
            issue["mod_versions"] = serde_json::json!([i.version]);
        }
        by_mod.entry(i.mod_id.clone()).or_default().push(issue);
    }
    by_mod
        .into_iter()
        .map(|(id, issues)| {
            let m = catalog.get(&id);
            let workshop = m.and_then(|m| match &m.source {
                crate::catalog::ModSource::Workshop { workshop_id } => Some(workshop_id.clone()),
                _ => None,
            });
            serde_json::json!({
                "id": id,
                "name": m.map(|m| m.name.clone()).unwrap_or_default(),
                "workshop_id": workshop,
                "builds": ["42"],
                "issues": issues,
            })
        })
        .collect()
}

/// What changed between the last game session before `since` and the latest one: the list, the
/// versions, the build. None when there is no session on one side.
pub fn changes_since(l: &Learned, since: u64, now: &ModsNow) -> Option<Onset> {
    let games: Vec<&LearnedSession> = l.sessions.iter().filter(|s| s.kind == "client").collect();
    let latest = *games.last()?;
    let before = games.iter().rev().find(|s| s.started < since).copied()?;
    if before.started == latest.started {
        return Some(Onset { first_bad: latest.started, last_good: Some(before.started), changes: vec![], more: 0 });
    }
    let (changes, more) = changes_between(l, before, latest, None, now);
    Some(Onset { first_bad: latest.started, last_good: Some(before.started), changes, more })
}

pub fn changes_since_for(paths: &crate::vfs::StudioPaths, since: u64) -> Option<Onset> {
    let (z, catalog, _) = context(paths, Some(vec![]))?;
    changes_since(&learned::load(&z), since, &ModsNow::of(&catalog))
}

// ---------------------------------------------------------------------------------------------
// Which mod goes with what goes wrong
// ---------------------------------------------------------------------------------------------

/// A mod that is on in the sessions that go wrong more than in the ones that do not.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Suspect {
    pub mod_id: String,
    /// Sessions with the mod on, and how many of them went wrong; the same without it.
    pub with: u32,
    pub with_bad: u32,
    pub without: u32,
    pub without_bad: u32,
    /// "strong" (5+ bad sessions with it, rarely without), "likely" (3+), "weak".
    pub confidence: String,
}

/// What "went wrong" means for [`suspects`].
pub fn went_wrong(l: &Learned, s: &LearnedSession, what: &str, t: u64) -> bool {
    match what {
        "failed_to_start" => s.failed_to_start(t),
        "crash" => l.crash_of(s).is_some() || s.oom,
        _ => s.failed_to_start(t) || s.oom || l.crash_of(s).is_some(),
    }
}

/// The mods whose sessions went wrong far more often than the sessions without them, on this
/// build: each with the counts it is judged by. A mod on in nearly every session cannot be told
/// apart (no sessions without it to compare), so it is left out.
pub fn suspects(l: &Learned, build: &str, what: &str, since: Option<u64>) -> Vec<Suspect> {
    let t = now_secs();
    let sessions: Vec<&LearnedSession> = l
        .sessions
        .iter()
        .filter(|s| s.kind == "client" && learned::same_build(&s.build, build) && !s.is_live(t) && since.is_none_or(|x| s.started >= x))
        .collect();
    let bad: Vec<bool> = sessions.iter().map(|s| went_wrong(l, s, what, t)).collect();
    let total_bad = bad.iter().filter(|b| **b).count() as u32;
    if total_bad < 2 {
        return vec![];
    }
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for s in &sessions {
        ids.extend(loaded_ids(l, s));
    }
    let mut out = Vec::new();
    for id in ids {
        let (mut with, mut with_bad, mut without, mut without_bad) = (0u32, 0u32, 0u32, 0u32);
        for (s, b) in sessions.iter().zip(&bad) {
            if mod_on(l, s, id) {
                with += 1;
                with_bad += *b as u32;
            } else {
                without += 1;
                without_bad += *b as u32;
            }
        }
        if with_bad < 2 || without < 3 {
            continue;
        }
        let rate_with = with_bad as f32 / with as f32;
        let rate_without = without_bad as f32 / without as f32;
        // it must go wrong clearly more with it than without it
        if rate_with < 0.3 || rate_with < rate_without * 2.0 + 0.1 {
            continue;
        }
        let confidence = if with_bad >= 5 && rate_without <= 0.1 {
            "strong"
        } else if with_bad >= 3 {
            "likely"
        } else {
            "weak"
        };
        out.push(Suspect { mod_id: id.to_string(), with, with_bad, without, without_bad, confidence: confidence.to_string() });
    }
    // the surest first (more sessions behind it), then the widest gap between with and without
    let rank = |c: &str| match c {
        "strong" => 2,
        "likely" => 1,
        _ => 0,
    };
    out.sort_by(|a, b| {
        let ra = a.with_bad as f32 / a.with as f32 - a.without_bad as f32 / a.without.max(1) as f32;
        let rb = b.with_bad as f32 / b.with as f32 - b.without_bad as f32 / b.without.max(1) as f32;
        rank(&b.confidence)
            .cmp(&rank(&a.confidence))
            .then(rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal))
            .then(b.with_bad.cmp(&a.with_bad))
    });
    out.truncate(8);
    out
}

// ---------------------------------------------------------------------------------------------
// Reports: one session, or a period
// ---------------------------------------------------------------------------------------------

/// One error, said briefly (title and its card's code and values, for the player's language).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorBrief {
    pub mod_id: String,
    pub title: String,
    pub code: String,
    pub params: BTreeMap<String, String>,
}

fn brief_of(e: &learned::LearnedError) -> ErrorBrief {
    ErrorBrief { mod_id: e.mod_id.clone(), title: e.title.clone(), code: e.code.clone(), params: e.params.clone() }
}

/// The report of one session: how it went and what was new in it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionReport {
    pub brief: SessionBrief,
    /// Minutes between its first and last line.
    pub minutes: u32,
    /// Errors that showed for the first time in it, and the others it repeated.
    pub new_errors: Vec<ErrorBrief>,
    pub repeated: u32,
    /// The fixes whose verdict this session settled or moved.
    pub fixes: Vec<FixOutcome>,
}

fn brief(l: &Learned, s: &LearnedSession, t: u64) -> SessionBrief {
    SessionBrief {
        started: s.started,
        ended: s.last_line,
        kind: s.kind.clone(),
        build: s.build.clone(),
        reached: s.reached.clone(),
        clean_exit: s.clean_exit,
        oom: s.oom,
        load_secs: s.load_secs,
        mods: s.loaded.len() as u32,
        errors: s.errors.len() as u32,
        live: s.is_live(t),
        crash: l.crash_of(s).map(|c| c.kind.clone()).unwrap_or_default(),
        crash_reason: l.crash_of(s).map(|c| c.reason.clone()).unwrap_or_default(),
        crash_frame: l.crash_of(s).map(|c| c.frame.clone()).unwrap_or_default(),
        crash_cause: l.crash_of(s).map(|c| crate::rules::learned::frame_cause(&c.frame).to_string()).unwrap_or_default(),
        save: l.diary_of(s).0,
        quit_to_menu: l.diary_of(s).1,
    }
}

pub fn session_report(l: &Learned, fixes: &[FixRecord], s: &LearnedSession) -> SessionReport {
    let t = now_secs();
    let earlier: Vec<&LearnedSession> = l.sessions.iter().filter(|x| x.started < s.started).collect();
    let mut new_errors = Vec::new();
    let mut repeated = 0;
    for e in s.errors.iter().filter_map(|i| l.errors.get(*i as usize)) {
        if earlier.iter().any(|x| has_error(l, x, &e.mod_id, Some(&e.key))) {
            repeated += 1;
        } else {
            new_errors.push(brief_of(e));
        }
    }
    let fixes = fixes.iter().filter(|f| f.at <= s.started && mod_on(l, s, &f.mod_id)).map(|f| fix_outcome(l, f)).collect();
    SessionReport { brief: brief(l, s, t), minutes: (s.last_line.saturating_sub(s.started) / 60) as u32, new_errors, repeated, fixes }
}

/// The report of a period: sessions, how many went badly, Java's crashes, errors that began in
/// it, the fixes applied in it, and what changed from before it to now.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PeriodReport {
    pub since: u64,
    pub sessions: u32,
    pub servers: u32,
    pub went_badly: u32,
    pub crashes: u32,
    pub minutes: u32,
    pub new_errors: Vec<ErrorBrief>,
    pub fixes: Vec<FixOutcome>,
    pub changes: Option<Onset>,
}

pub fn period_report(l: &Learned, fixes: &[FixRecord], since: u64, now: &ModsNow) -> PeriodReport {
    let t = now_secs();
    let inside: Vec<&LearnedSession> = l.sessions.iter().filter(|s| s.started >= since && !s.is_live(t)).collect();
    let before: Vec<&LearnedSession> = l.sessions.iter().filter(|s| s.started < since).collect();
    let mut new_errors: Vec<ErrorBrief> = Vec::new();
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for s in &inside {
        for e in s.errors.iter().filter_map(|i| l.errors.get(*i as usize)) {
            let k = (e.mod_id.clone(), e.key.clone());
            if seen.contains(&k) || before.iter().any(|b| has_error(l, b, &e.mod_id, Some(&e.key))) {
                continue;
            }
            seen.insert(k);
            new_errors.push(brief_of(e));
        }
    }
    PeriodReport {
        since,
        sessions: inside.iter().filter(|s| s.kind == "client").count() as u32,
        servers: inside.iter().filter(|s| s.kind == "server").count() as u32,
        went_badly: inside.iter().filter(|s| went_wrong(l, s, "any", t)).count() as u32,
        crashes: l.crashes.iter().filter(|c| c.at >= since).count() as u32,
        minutes: inside.iter().filter(|s| s.kind == "client").map(|s| (s.last_line.saturating_sub(s.started) / 60) as u32).sum(),
        new_errors,
        fixes: fixes.iter().filter(|f| f.at >= since).map(|f| fix_outcome(l, f)).collect(),
        changes: changes_since(l, since, now),
    }
}

/// A moment the player marked in the game, and what the log says around it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarkReport {
    pub at: u64,
    pub save: String,
    pub place: String,
    /// The session it was in (its start), when the log is known.
    pub session: Option<u64>,
    /// The cards the Monitor makes of the log from a minute and a half before to half a minute after.
    pub cards: Vec<ErrorBrief>,
    /// The error and warning lines of that window (the first few, trimmed).
    pub lines: Vec<String>,
}

/// The marks of the Link's diary, newest first (at most `max`), each with its log window read.
pub fn marks(zomboid: &Path, l: &Learned, max: usize) -> Vec<MarkReport> {
    let mut out = Vec::new();
    for e in l.diary.iter().rev().filter(|e| e.event == "mark").take(max) {
        let mut report = MarkReport { at: e.at, save: e.save.clone(), place: e.place.clone(), session: None, cards: vec![], lines: vec![] };
        if let Some(s) = l.session_at(e.at) {
            report.session = Some(s.started);
            if !s.file.is_empty() {
                if let Ok(bytes) = std::fs::read(zomboid.join(&s.file)) {
                    let text = String::from_utf8_lossy(&bytes);
                    let (from, to) = (e.at.saturating_sub(90), e.at + 30);
                    let mut maker = crate::sandbox::CardMaker::new();
                    let mut cards = Vec::new();
                    let mut inside = false;
                    for line in text.lines() {
                        if let Some(t) = learned::line_time(line) {
                            inside = t >= from && t <= to;
                            if t > to {
                                break;
                            }
                        }
                        if !inside {
                            continue;
                        }
                        cards.extend(maker.line(line));
                        let body = line.split_once("] ").map(|(_, b)| b).unwrap_or(line);
                        if (body.starts_with("ERROR") || body.starts_with("WARN")) && report.lines.len() < 6 {
                            report.lines.push(body.chars().take(220).collect());
                        }
                    }
                    cards.extend(maker.flush());
                    let mut seen = BTreeSet::new();
                    for c in cards {
                        if seen.insert((c.mod_name.clone(), c.title.clone())) {
                            report.cards.push(ErrorBrief { mod_id: c.mod_name.clone().unwrap_or_default(), title: c.title, code: c.code, params: c.params });
                        }
                    }
                }
            }
        }
        out.push(report);
    }
    out
}

pub fn marks_for(paths: &crate::vfs::StudioPaths, max: usize) -> Vec<MarkReport> {
    match crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next() {
        Some(z) => marks(&z, &learned::load(&z), max),
        None => vec![],
    }
}

pub fn suspects_for(paths: &crate::vfs::StudioPaths, what: &str, since: Option<u64>) -> Vec<Suspect> {
    match context(paths, Some(vec![])) {
        Some((z, catalog, _)) => suspects(&learned::load(&z), &catalog.game_version.clone().unwrap_or_default(), what, since),
        None => vec![],
    }
}

pub fn period_report_for(paths: &crate::vfs::StudioPaths, since: u64) -> PeriodReport {
    match context(paths, Some(vec![])) {
        Some((z, catalog, _)) => period_report(&learned::load(&z), &load_fixes(&z), since, &ModsNow::of(&catalog)),
        None => PeriodReport::default(),
    }
}

/// The Zomboid folder, the catalog and the active list (ids, the Studio's own mods left out) the
/// brain reads with; `active` given, or default.txt's.
pub fn context(paths: &crate::vfs::StudioPaths, active: Option<Vec<String>>) -> Option<(std::path::PathBuf, crate::catalog::ModCatalog, Vec<String>)> {
    let zomboid = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next()?;
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let entries = active.unwrap_or_else(|| crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default());
    let ids = entries
        .iter()
        .map(|e| catalog.resolve(e.trim()).map(|m| m.id.clone()).unwrap_or_else(|| e.trim().trim_start_matches('\\').to_string()))
        .collect();
    Some((zomboid, catalog, ids))
}

pub fn overview_for(paths: &crate::vfs::StudioPaths, active: Option<Vec<String>>) -> BrainOverview {
    match context(paths, active) {
        Some((z, catalog, ids)) => overview(&z, &catalog, &ids),
        None => BrainOverview::default(),
    }
}

pub fn mod_history_for(paths: &crate::vfs::StudioPaths, mod_id: &str) -> ModHistory {
    match context(paths, Some(vec![])) {
        Some((z, catalog, _)) => mod_history(&learned::load(&z), &load_fixes(&z), mod_id, &ModsNow::of(&catalog)),
        None => ModHistory::default(),
    }
}

pub fn prelaunch_for(paths: &crate::vfs::StudioPaths, active: Option<Vec<String>>) -> Option<Prelaunch> {
    let (z, catalog, ids) = context(paths, active)?;
    prelaunch(&learned::load(&z), &ids, &catalog.game_version.clone().unwrap_or_default(), &ModsNow::of(&catalog))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(l: &mut Learned, started: u64, mods: &[&str], errors: &[(&str, &str)], reached: &str) {
        let mut s = LearnedSession { id: started.to_string(), started, kind: "client".into(), build: "42.22.2".into(), reached: reached.into(), last_line: started + 60, ..Default::default() };
        for m in mods {
            let at = l.ids.iter().position(|x| x == m).unwrap_or_else(|| {
                l.ids.push(m.to_string());
                l.ids.len() - 1
            });
            s.loaded.push(at as u32);
            let v = l.mods.iter().position(|(x, _)| x == m).unwrap_or_else(|| {
                l.mods.push((m.to_string(), "1".into()));
                l.mods.len() - 1
            });
            s.mods.push(v as u32);
        }
        for (m, k) in errors {
            let at = l.errors.iter().position(|e| e.mod_id == *m && e.key == *k).unwrap_or_else(|| {
                l.errors.push(learned::LearnedError { mod_id: m.to_string(), version: "1".into(), key: k.to_string(), title: "t".into(), code: "c".into(), params: BTreeMap::new(), file: None, line: None, sample: String::new() });
                l.errors.len() - 1
            });
            s.errors.push(at as u32);
        }
        l.sessions.push(s);
    }

    fn nothing_changed() -> ModsNow {
        ModsNow { version: BTreeMap::new(), changed: BTreeMap::new(), requires: BTreeMap::new() }
    }

    #[test]
    fn an_error_begins_when_a_mod_is_added_and_the_onset_names_it() {
        let mut l = Learned::default();
        session(&mut l, 100, &["A", "B"], &[], "game");
        session(&mut l, 200, &["A", "B"], &[], "game");
        session(&mut l, 300, &["A", "C", "B"], &[("B", "k")], "game");
        let o = error_onset(&l, "B", "k", &nothing_changed()).unwrap();
        assert_eq!((o.first_bad, o.last_good), (300, Some(200)));
        assert!(o.changes.iter().any(|c| c.kind == "added" && c.mod_id == "C"), "{:?}", o.changes);
    }

    #[test]
    fn an_update_between_two_sessions_is_a_change() {
        let mut l = Learned::default();
        session(&mut l, 100, &["A", "B"], &[], "game");
        session(&mut l, 300, &["A", "B"], &[("B", "k")], "game");
        let mut now = nothing_changed();
        now.changed.insert("B".into(), 200);
        let o = error_onset(&l, "B", "k", &now).unwrap();
        assert_eq!(o.changes[0].kind, "updated");
        assert_eq!(o.changes[0].mod_id, "B");
    }

    #[test]
    fn a_fix_worked_when_the_error_stops_after_it() {
        let mut l = Learned::default();
        session(&mut l, 100, &["A"], &[("A", "k")], "game");
        session(&mut l, 200, &["A"], &[("A", "k")], "game");
        let fix = FixRecord { at: 250, mod_id: "A".into(), action: "make_compatible_copy".into(), code: "learned_issue".into(), key: Some("k".into()) };
        assert_eq!(fix_outcome(&l, &fix).verdict, "waiting");
        session(&mut l, 300, &["A"], &[], "game");
        session(&mut l, 400, &["A"], &[], "game");
        let o = fix_outcome(&l, &fix);
        assert_eq!((o.verdict.as_str(), o.before_bad, o.after, o.after_bad), ("worked", 2, 2, 0));
        session(&mut l, 500, &["A"], &[("A", "k")], "game");
        assert_eq!(fix_outcome(&l, &fix).verdict, "not_worked");
    }

    #[test]
    fn before_playing_it_warns_when_the_same_list_failed_to_start_twice() {
        let mut l = Learned::default();
        let list = ["A", "B", "C", "D", "E", "F", "G"];
        session(&mut l, 100, &list, &[], "game");
        session(&mut l, 200, &["A", "B", "C", "D", "E", "F", "G", "H"], &[], "");
        session(&mut l, 300, &["A", "B", "C", "D", "E", "F", "G", "H"], &[], "");
        let active: Vec<String> = ["A", "B", "C", "D", "E", "F", "G", "H"].iter().map(|s| s.to_string()).collect();
        let p = prelaunch(&l, &active, "42.22", &nothing_changed()).unwrap();
        assert_eq!((p.failed_to_start, p.last_good), (2, Some(100)));
        assert!(p.changes.iter().any(|c| c.kind == "added" && c.mod_id == "H"));
        // a list that worked last time says nothing
        session(&mut l, 400, &["A", "B", "C", "D", "E", "F", "G", "H"], &[], "game");
        session(&mut l, 500, &["A", "B", "C", "D", "E", "F", "G", "H"], &[], "game");
        assert!(prelaunch(&l, &active, "42.22", &nothing_changed()).is_none());
    }

    #[test]
    fn the_mod_on_in_the_sessions_that_fail_is_the_suspect() {
        let mut l = Learned::default();
        // without Bad: fine; with Bad: most fail to start
        for n in 0..6 {
            session(&mut l, 100 + n, &["A", "B"], &[], "game");
        }
        for n in 0..5 {
            session(&mut l, 200 + n, &["A", "B", "Bad"], &[], "");
        }
        session(&mut l, 300, &["A", "B", "Bad"], &[], "game");
        let s = suspects(&l, "42.22", "any", None);
        assert_eq!(s[0].mod_id, "Bad");
        assert_eq!((s[0].with, s[0].with_bad, s[0].without, s[0].without_bad, s[0].confidence.as_str()), (6, 5, 6, 0, "strong"));
        // A and B are on in every session: they cannot be told apart, so they are not named
        assert!(s.iter().all(|x| x.mod_id == "Bad"));
    }

    #[test]
    fn a_session_report_names_the_errors_new_in_it() {
        let mut l = Learned::default();
        session(&mut l, 100, &["A"], &[("A", "old")], "game");
        session(&mut l, 200, &["A"], &[("A", "old"), ("A", "new")], "game");
        let r = session_report(&l, &[], &l.sessions[1].clone());
        assert_eq!((r.new_errors.len(), r.repeated), (1, 1));
        let p = period_report(&l, &[], 150, &nothing_changed());
        assert_eq!((p.sessions, p.new_errors.len()), (1, 1));
    }

    #[test]
    fn a_mod_s_history_counts_its_sessions_versions_and_errors() {
        let mut l = Learned::default();
        session(&mut l, 100, &["A", "B"], &[], "game");
        session(&mut l, 200, &["A", "B"], &[("B", "k")], "game");
        let h = mod_history(&l, &[], "B", &nothing_changed());
        assert_eq!((h.sessions, h.place, h.errors.len()), (2, Some((2, 2)), 1));
        assert_eq!(h.versions[0].sessions, 2);
        assert_eq!(h.errors[0].onset.as_ref().unwrap().last_good, Some(100));
    }

    #[test]
    fn slower_loading_names_the_session_it_began() {
        let mut l = Learned::default();
        for (n, secs) in [60, 62, 58, 61, 59, 190, 200, 185].iter().enumerate() {
            let mods: Vec<&str> = if n >= 5 { vec!["A", "Big"] } else { vec!["A"] };
            session(&mut l, 100 * (n as u64 + 1), &mods, &[], "game");
            l.sessions.last_mut().unwrap().load_secs = Some(*secs);
        }
        let i = load_slower(&l, "42.22", &nothing_changed()).unwrap();
        assert_eq!(i.params["before"], "60");
        let onset = i.onset.unwrap();
        assert_eq!((onset.last_good, onset.first_bad), (Some(500), 600));
        assert!(onset.changes.iter().any(|c| c.kind == "added" && c.mod_id == "Big"));
    }
}
