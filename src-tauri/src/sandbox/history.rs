//! Which errors are new: the error lines of every game session, by shape, kept across sessions in
//! `<Zomboid>/PZModStudio_ErrorHistory.json`, so a card can say "first time" or "in 6 of the
//! last 8 sessions, since the 12th".
//!
//! A line's shape is what the Monitor counts repeats by: the level prefix off, every number
//! replaced by N, the first 90 characters (the same rule as `errorShape` in the frontend, so the
//! two agree on what is one error).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const FILE: &str = "PZModStudio_ErrorHistory.json";
/// Sessions kept; older ones go.
const SESSIONS_KEPT: usize = 40;
const WRITE_EVERY: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionRecord {
    /// Unix seconds when the session's console started (or when the Studio first saw it).
    pub started: u64,
    /// The Studio opened after the game did, so the lines before that were read from the file
    /// as it was then.
    #[serde(default)]
    pub seeded: bool,
    /// Error shape -> how many lines of it.
    #[serde(default)]
    pub errors: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorHistory {
    #[serde(default)]
    pub sessions: Vec<SessionRecord>,
}

/// What the history says about one error shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyHistory {
    pub key: String,
    /// Sessions before the latest in which it appeared.
    pub sessions_seen: u32,
    /// Unix seconds of the first and last of those sessions.
    pub first_seen: Option<u64>,
    pub last_seen: Option<u64>,
    /// In none of the earlier sessions.
    pub new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorHistoryView {
    /// Sessions before the latest that the history holds.
    pub earlier_sessions: u32,
    pub latest_started: Option<u64>,
    pub keys: Vec<KeyHistory>,
}

/// The shape of an error line, as the frontend's `errorShape` makes it.
pub fn error_key(line: &str) -> String {
    let mut rest = line;
    for level in ["ERROR", "WARN"] {
        if let Some(after) = rest.strip_prefix(level) {
            let after = after.trim_start();
            if let Some(after) = after.strip_prefix(':') {
                let after = after.trim_start();
                // the logger name (a word), then `f:<digits>`
                let after = after.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_').trim_start();
                if let Some(after) = after.strip_prefix("f:") {
                    let after = after.trim_start_matches(|c: char| c.is_ascii_digit()).trim_start();
                    // optional `at Some.Name`
                    let after = match after.strip_prefix("at ") {
                        Some(a) => a.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_' || c == '.').trim_start(),
                        None => after,
                    };
                    rest = after.strip_prefix('>').unwrap_or(after).trim_start();
                }
            }
            break;
        }
    }
    let mut out = String::with_capacity(rest.len());
    let mut in_digits = false;
    for c in rest.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                out.push('N');
                in_digits = true;
            }
        } else {
            in_digits = false;
            out.push(c);
        }
    }
    out.trim().chars().take(90).collect()
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn path_in(user_zomboid_dir: &Path) -> PathBuf {
    user_zomboid_dir.join(FILE)
}

pub fn load(user_zomboid_dir: &Path) -> ErrorHistory {
    std::fs::read_to_string(path_in(user_zomboid_dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// What the history says about these lines (one entry per line, aligned with `lines`), against
/// every session but the latest.
pub fn lookup(user_zomboid_dir: &Path, lines: &[String]) -> ErrorHistoryView {
    let history = load(user_zomboid_dir);
    view(&history, lines)
}

pub fn view(history: &ErrorHistory, lines: &[String]) -> ErrorHistoryView {
    let latest = history.sessions.last();
    let earlier: Vec<&SessionRecord> = history.sessions.iter().take(history.sessions.len().saturating_sub(1)).collect();
    let mut keys = Vec::with_capacity(lines.len());
    for line in lines {
        let key = error_key(line);
        let in_sessions: Vec<u64> = earlier.iter().filter(|s| s.errors.contains_key(&key)).map(|s| s.started).collect();
        keys.push(KeyHistory {
            sessions_seen: in_sessions.len() as u32,
            first_seen: in_sessions.iter().min().copied(),
            last_seen: in_sessions.iter().max().copied(),
            new: in_sessions.is_empty(),
            key,
        });
    }
    ErrorHistoryView { earlier_sessions: earlier.len() as u32, latest_started: latest.map(|s| s.started), keys }
}

/// The console watcher's side: counts the session's error lines and writes them out now and then.
pub struct Tracker {
    dir: PathBuf,
    history: ErrorHistory,
    dirty: bool,
    last_write: Instant,
}

impl Tracker {
    pub fn load(user_zomboid_dir: &Path) -> Tracker {
        Tracker { dir: user_zomboid_dir.to_path_buf(), history: load(user_zomboid_dir), dirty: false, last_write: Instant::now() }
    }

    /// A session began (a fresh console.txt). `seed`: the lines already in the file when the
    /// Studio started following it, so a session the Studio joined late is counted whole.
    pub fn start_session(&mut self, seed: &[String]) {
        let mut record = SessionRecord { started: now_secs(), seeded: !seed.is_empty(), errors: BTreeMap::new() };
        for line in seed {
            count(&mut record, line);
        }
        self.history.sessions.push(record);
        let extra = self.history.sessions.len().saturating_sub(SESSIONS_KEPT);
        if extra > 0 {
            self.history.sessions.drain(..extra);
        }
        self.dirty = true;
        self.write();
    }

    pub fn see(&mut self, line: &str) {
        if self.history.sessions.is_empty() {
            self.start_session(&[]);
        }
        if let Some(record) = self.history.sessions.last_mut() {
            if count(record, line) {
                self.dirty = true;
            }
        }
    }

    /// Writes when something changed and the last write is a while ago.
    pub fn flush_if_due(&mut self) {
        if self.dirty && self.last_write.elapsed() >= WRITE_EVERY {
            self.write();
        }
    }

    fn write(&mut self) {
        if let Ok(text) = serde_json::to_string(&self.history) {
            let _ = crate::load_order::ini_parser::atomic_write(path_in(&self.dir), text);
        }
        self.dirty = false;
        self.last_write = Instant::now();
    }
}

/// Counts the line when it is an error line; says whether it did.
fn count(record: &mut SessionRecord, line: &str) -> bool {
    if !line.starts_with("ERROR") {
        return false;
    }
    let key = error_key(line);
    if key.is_empty() {
        return false;
    }
    *record.errors.entry(key).or_insert(0) += 1;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_is_the_frontends_shape() {
        assert_eq!(
            error_key("ERROR: General      f:121092> Lua((MOD:PZ Mod Studio Patch: MergedPatch)).HandleZombiesAmputations(ZombiesAmputation.lua:86) > Exception thrown"),
            "Lua((MOD:PZ Mod Studio Patch: MergedPatch)).HandleZombiesAmputations(ZombiesAmputation.lua"
        );
        assert_eq!(error_key("ERROR: General      f:0 at KahluaThread.flushErrorMessage > dumping Lua stack trace"), "dumping Lua stack trace");
        // `at Lua` is eaten as the "at Some.Name" part, as the frontend's regex does: the two must agree, not be pretty.
        assert_eq!(error_key("WARN : Lua          f:0 at Lua((MOD:X)).MoodlesIn.lua> require(\"LSMoodleManager\") failed"), "((MOD:X)).MoodlesIn.lua> require(\"LSMoodleManager\") failed");
        assert_eq!(error_key("\tjava.lang.RuntimeException: attempted index: prerender of non-table: null at KahluaThread.tableget(KahluaThread.java:1430)."), "java.lang.RuntimeException: attempted index: prerender of non-table: null at KahluaThread.");
    }

    #[test]
    fn an_error_is_new_until_an_earlier_session_had_it() {
        let a = "ERROR: General      f:1> Lua((MOD:A)).tick(A.lua:10) > boom".to_string();
        let b = "ERROR: General      f:2> Lua((MOD:B)).tick(B.lua:20) > bang".to_string();
        let mut history = ErrorHistory::default();
        // only the latest session: everything is new, nothing earlier
        let mut s1 = SessionRecord { started: 100, ..Default::default() };
        count(&mut s1, &a);
        history.sessions.push(s1);
        let v = view(&history, &[a.clone(), b.clone()]);
        assert_eq!((v.earlier_sessions, v.latest_started), (0, Some(100)));
        assert!(v.keys.iter().all(|k| k.new));
        // a second session: A was seen in the first, B was not
        let mut s2 = SessionRecord { started: 200, ..Default::default() };
        count(&mut s2, &b);
        history.sessions.push(s2);
        let v = view(&history, &[a.clone(), b.clone(), a.clone()]);
        assert_eq!(v.keys.len(), 3, "one entry per line, in order");
        assert_eq!(v.keys[0], v.keys[2]);
        let ka = &v.keys[0];
        let kb = &v.keys[1];
        assert_eq!((ka.new, ka.sessions_seen, ka.first_seen), (false, 1, Some(100)));
        assert_eq!((kb.new, kb.sessions_seen), (true, 0));
    }

    #[test]
    fn the_tracker_counts_seeds_and_keeps_forty_sessions() {
        let dir = std::env::temp_dir().join(format!("pzms_history_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut t = Tracker::load(&dir);
        t.start_session(&["ERROR: General f:1> Lua((MOD:A)).x(A.lua:1) > seed".to_string(), "LOG  : General f:1> fine".to_string()]);
        t.see("ERROR: General f:2> Lua((MOD:A)).x(A.lua:1) > seed");
        t.see("not an error");
        t.write();
        let h = load(&dir);
        assert_eq!(h.sessions.len(), 1);
        assert!(h.sessions[0].seeded);
        assert_eq!(h.sessions[0].errors.values().sum::<u32>(), 2);
        for _ in 0..45 {
            t.start_session(&[]);
        }
        assert_eq!(load(&dir).sessions.len(), SESSIONS_KEPT);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
