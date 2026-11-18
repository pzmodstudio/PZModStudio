//! What this PC has learned from its own game sessions, kept in `Zomboid/PZModStudio_Learned.json`.
//!
//! The mod index the Studio is built with is the starting point; this is what each PC adds to it.
//! Every session log the game keeps (`Zomboid/Logs/**/<time>_DebugLog.txt` and
//! `_DebugLog-server.txt`) is read once, with the Monitor's own card maker, and the Studio notes
//! which mods were on (and which version of each) and which errors the game blamed on each. An
//! error a mod gave in [`MIN_SESSIONS`] or more sessions with the same version, on the installed
//! build, becomes a finding of Diagnostics, "learned on this PC", with the count as its evidence.
//!
//! It forgets by itself: a mod's version is part of what is counted, so an update starts the count
//! again; an error the mod was on for in its last [`CLOSED_AFTER`] sessions without giving it is
//! closed (a fix that worked shows the same way). Nothing leaves the PC.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::sandbox::{CardMaker, Severity as CardSeverity, TranslatedErrorPayload};

const FILE: &str = "PZModStudio_Learned.json";
/// How errors are told apart; a file of an older one is read again from the logs.
const VERSION: u32 = 6;
/// The version of a mod in a session read after the mod's files changed.
pub const UNKNOWN: &str = "?";
/// Sessions with the same error, the same mod version and the same build that make it a finding.
pub const MIN_SESSIONS: u32 = 3;
/// The mod's latest sessions without the error that close it.
pub const CLOSED_AFTER: usize = 5;
/// Sessions kept; older ones go.
const SESSIONS_KEPT: usize = 1500;

/// One error the game blamed on one mod (one version of it).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LearnedError {
    pub mod_id: String,
    pub version: String,
    /// What makes it one error: the card's kind, its file and the shape of its line.
    pub key: String,
    /// The Monitor card's words, its code and values (so the app can say it in the player's language).
    pub title: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub line: Option<usize>,
    /// The log line, as evidence.
    pub sample: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct LearnedSession {
    /// `client|server` and the log's first time: the same session archived later is the same id.
    pub id: String,
    /// Unix seconds of the log's first line.
    pub started: u64,
    pub kind: String,
    pub build: String,
    /// Indices into `mods`: what was on.
    pub mods: Vec<u32>,
    /// Indices into `errors`: what the game blamed on them.
    pub errors: Vec<u32>,
    /// Indices into `ids`: every mod the game said it loaded, in its order (installed now or not).
    #[serde(default)]
    pub loaded: Vec<u32>,
    /// How far it got: "game" (a save loaded), "menu", "started" (a server), or "" (none of them).
    #[serde(default)]
    pub reached: String,
    /// The game wrote that its thread ended (quit from its own menu, or after a fatal error). Most
    /// sessions do not: closing the window (X, Alt+F4) or from outside ends the log mid-way, so
    /// this says how it was closed, never that it went wrong.
    #[serde(default)]
    pub clean_exit: bool,
    /// Java ran out of memory.
    #[serde(default)]
    pub oom: bool,
    /// What the game said loading a save took.
    #[serde(default)]
    pub load_secs: Option<u32>,
    /// Unix seconds of the log's last line with a time.
    #[serde(default)]
    pub last_line: u64,
    /// Its log, under the Zomboid folder (Logs/...), to read a moment of it again.
    #[serde(default)]
    pub file: String,
}

/// One line of the Studio Link's diary (`Lua/PZModStudio_LinkDiary.txt`): what the log does not
/// say. `start`: a save loaded; `menu`: back to the main menu from a game; `mark`: the player
/// marked the moment from the pause menu.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DiaryEvent {
    pub event: String,
    pub at: u64,
    #[serde(default)]
    pub save: String,
    #[serde(default)]
    pub place: String,
}

pub const DIARY_FILE: &str = "PZModStudio_LinkDiary.txt";
/// Lines the diary keeps (the app trims it).
const DIARY_KEPT: usize = 300;

/// The diary's lines: event, time, save, place (tab-separated).
pub fn read_diary_text(text: &str) -> Vec<DiaryEvent> {
    text.lines()
        .filter_map(|l| {
            let mut p = l.split('\t');
            let event = p.next()?.trim().to_string();
            let at = p.next()?.trim().parse().ok()?;
            if !["start", "menu", "mark"].contains(&event.as_str()) {
                return None;
            }
            Some(DiaryEvent { event, at, save: p.next().unwrap_or("").trim().to_string(), place: p.next().unwrap_or("").trim().to_string() })
        })
        .collect()
}

impl Learned {
    /// The game session a diary event happened in.
    pub fn session_at(&self, at: u64) -> Option<&LearnedSession> {
        self.sessions.iter().rev().find(|s| s.kind == "client" && s.started <= at + 5 && at <= s.last_line + 120)
    }

    /// The save a game session loaded and whether it went back to the menu, from the diary.
    pub fn diary_of(&self, s: &LearnedSession) -> (String, bool) {
        let inside = |e: &&DiaryEvent| e.at + 5 >= s.started && e.at <= s.last_line + 120;
        let save = self.diary.iter().filter(inside).find(|e| e.event == "start").map(|e| e.save.clone()).unwrap_or_default();
        let menu = self.diary.iter().filter(inside).any(|e| e.event == "menu");
        (save, menu)
    }
}

impl LearnedSession {
    /// It ended before reaching the menu, a save or (a server) its start.
    pub fn failed_to_start(&self, now: u64) -> bool {
        self.reached.is_empty() && !self.is_live(now)
    }

    /// Its log was written in the last three minutes: the game may still be running.
    pub fn is_live(&self, now: u64) -> bool {
        now.saturating_sub(self.last_line) < 180
    }
}

/// A real crash: Java's own report (`hs_err_pid<pid>.log`, in the game's folder), written only when
/// the Java process itself fell over. Closing the window never writes one.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct JvmCrash {
    pub file: String,
    /// Unix seconds when it fell, and how long the process had been running.
    pub at: u64,
    pub elapsed: u64,
    /// "client" (the game: Java started by the game's exe) or "server" (zombie.network.GameServer).
    pub role: String,
    /// The second player's game of a multiplayer test (its Zomboid2 folder).
    #[serde(default)]
    pub second_player: bool,
    /// "out_of_memory": Windows gave Java no more memory (the PC ran out, not the game's limit);
    /// "fatal": Java hit an error it cannot survive (`reason`, in `frame`).
    pub kind: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub frame: String,
    /// What the frame's library is (frame_cause), filled when it is read out.
    #[serde(default)]
    pub cause: String,
}

/// What the library Java fell in is, in plain words: `graphics_nvidia` / `graphics_amd` /
/// `graphics_intel` (the video card's driver), `window_opengl` (LWJGL / GLFW, the game's window
/// and OpenGL layer), `audio` (FMOD / OpenAL), `network` (the game's RakNet / ZNet),
/// `zombiebuddy` (zbNative, Java mods' loader), `java` (jvm.dll: Java itself), `steam`,
/// `physics` (pzBullet), `windows` (a system library); empty when unknown.
pub fn frame_cause(frame: &str) -> &'static str {
    let f = frame.to_ascii_lowercase();
    let starts = |p: &[&str]| p.iter().any(|x| f.starts_with(x));
    if starts(&["nvoglv", "nvwgf2um", "nvd3dum", "nvcuda", "nvlddmkm"]) {
        "graphics_nvidia"
    } else if starts(&["atio6axx", "atioglxx", "amdxc", "atiumd", "amdvlk", "aticfx"]) {
        "graphics_amd"
    } else if starts(&["ig9icd", "ig75icd", "ig8icd", "igxelpicd", "igdumd", "igd10", "igc64", "ig4icd", "ig7icd", "igdgmm"]) || (f.starts_with("ig") && f.contains("icd")) {
        "graphics_intel"
    } else if starts(&["lwjgl", "glfw", "opengl32"]) {
        "window_opengl"
    } else if starts(&["fmod", "openal", "soft_oal"]) {
        "audio"
    } else if starts(&["znetjni", "raknet"]) {
        "network"
    } else if starts(&["zbnative"]) {
        "zombiebuddy"
    } else if starts(&["jvm.dll", "jvm"]) {
        "java"
    } else if starts(&["steam_api", "steamclient", "gameoverlayrenderer"]) {
        "steam"
    } else if starts(&["pzbullet"]) {
        "physics"
    } else if starts(&["ntdll", "kernel32", "kernelbase", "ucrtbase", "msvcr", "vcruntime"]) {
        "windows"
    } else {
        ""
    }
}

/// Reads one of Java's crash reports.
pub fn read_jvm_crash(name: &str, text: &str) -> Option<JvmCrash> {
    let mut c = JvmCrash { file: name.to_string(), ..Default::default() };
    let head: Vec<&str> = text.lines().take(12).collect();
    if head.iter().any(|l| l.contains("insufficient memory for the Java Runtime") || l.contains("Out of Memory Error")) {
        c.kind = "out_of_memory".into();
    } else if head.iter().any(|l| l.contains("A fatal error has been detected")) {
        c.kind = "fatal".into();
        c.reason = head.iter().find_map(|l| l.trim_start_matches(['#', ' ']).split_whitespace().next().filter(|w| w.starts_with("EXCEPTION_") || w.starts_with("SIG")).map(str::to_string)).unwrap_or_default();
    } else {
        return None;
    }
    let mut lines = text.lines();
    while let Some(l) = lines.next() {
        if l.starts_with("# Problematic frame:") {
            // "# C  [ZNetJNI64.dll+0x17cf2]" -> the library
            if let Some(next) = lines.next() {
                c.frame = next.split('[').nth(1).and_then(|r| r.split(['+', ']']).next()).unwrap_or("").to_string();
            }
        } else if let Some(rest) = l.strip_prefix("Time: ") {
            // "Mon Nov 16 00:34:40 2026 Argentina Standard Time elapsed time: 468.286953 seconds ..."
            let words: Vec<&str> = rest.split_whitespace().collect();
            if words.len() >= 5 {
                let when = format!("{} {} {} {}", words[1], words[2], words[3], words[4]);
                if let Ok(t) = chrono::NaiveDateTime::parse_from_str(&when, "%b %d %H:%M:%S %Y") {
                    if let Some(local) = t.and_local_timezone(chrono::Local).earliest() {
                        c.at = local.timestamp().max(0) as u64;
                    }
                }
            }
            if let Some(e) = rest.split("elapsed time: ").nth(1) {
                c.elapsed = e.split(['.', ' ']).next().and_then(|n| n.parse().ok()).unwrap_or(0);
            }
        } else if l.starts_with("java_command:") {
            c.role = if l.contains("zombie.network.GameServer") { "server" } else { "client" }.into();
        } else if l.starts_with("Command Line:") && l.contains("Zomboid2") {
            c.second_player = true;
        }
    }
    if c.role.is_empty() {
        c.role = "client".into();
    }
    (c.at > 0).then_some(c)
}

impl Learned {
    /// The crash report of a session: same role, the process began when the session did (or the
    /// crash fell inside the session's log).
    pub fn crash_of(&self, s: &LearnedSession) -> Option<&JvmCrash> {
        // Two games of a multiplayer test can fall the same second: the closest start is this one's.
        self.crashes
            .iter()
            .filter(|c| {
                c.role == s.kind && {
                    let began = c.at.saturating_sub(c.elapsed);
                    began.abs_diff(s.started) <= 90 || (c.at >= s.started && c.at <= s.last_line + 120)
                }
            })
            .min_by_key(|c| c.at.saturating_sub(c.elapsed).abs_diff(s.started))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Learned {
    /// The Studio Link's diary, as read so far.
    #[serde(default)]
    pub diary: Vec<DiaryEvent>,
    /// Java's crash reports read from the game's folder.
    #[serde(default)]
    pub crashes: Vec<JvmCrash>,
    /// Mod ids the sessions' `loaded` point to.
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub version: u32,
    /// Log file (its path under the Zomboid folder) -> the size it had when it was read.
    #[serde(default)]
    pub files: BTreeMap<String, u64>,
    /// (mod id, version) pairs the sessions point to.
    #[serde(default)]
    pub mods: Vec<(String, String)>,
    #[serde(default)]
    pub errors: Vec<LearnedError>,
    #[serde(default)]
    pub sessions: Vec<LearnedSession>,
}

/// What one log says, before it is matched with the mods on disk.
#[derive(Debug, Clone, Default)]
pub struct SessionFacts {
    pub started: u64,
    pub stamp: String,
    pub build: String,
    pub server: bool,
    /// The mods the game said it loaded, in order, each once.
    pub loaded: Vec<String>,
    /// The error cards (the game threw) that name a mod.
    pub cards: Vec<TranslatedErrorPayload>,
    pub reached: String,
    pub clean_exit: bool,
    pub oom: bool,
    pub load_secs: Option<u32>,
    pub last_line: u64,
}

/// What the Studio knows of an installed mod, to record a session of it.
#[derive(Debug, Clone)]
pub struct ModNow {
    /// `modversion=`, or the date its files were installed when it has none.
    pub version: String,
    /// Unix seconds its mod.info was last written: a session before that ran other files.
    pub changed: u64,
    pub name: String,
    /// The Studio's own mods and the player's mods in development are not learned about.
    pub skip: bool,
}

/// A finding: an error one mod gave in several sessions of this PC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LearnedIssue {
    pub mod_id: String,
    pub version: String,
    pub key: String,
    pub title: String,
    pub code: String,
    pub params: BTreeMap<String, String>,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub sample: String,
    pub build: String,
    /// Sessions with the error / sessions with the mod on (same version, same build).
    pub seen: u32,
    pub on: u32,
    pub first_seen: u64,
    pub last_seen: u64,
    /// The mod's latest sessions went without it.
    pub closed: bool,
}

pub fn path_in(zomboid: &Path) -> PathBuf {
    zomboid.join(FILE)
}

pub fn load(zomboid: &Path) -> Learned {
    std::fs::read_to_string(path_in(zomboid)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save(zomboid: &Path, learned: &Learned) -> Result<(), String> {
    let text = serde_json::to_string(learned).map_err(|e| e.to_string())?;
    let tmp = zomboid.join(format!("{}.tmp", FILE));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path_in(zomboid)).map_err(|e| e.to_string())
}

/// One build for what is counted: major and minor ("42.22.2" and "42.22" are the same).
pub fn same_build(a: &str, b: &str) -> bool {
    let cut = |v: &str| v.split('.').take(2).collect::<Vec<_>>().join(".");
    !a.is_empty() && cut(a) == cut(b)
}

/// The game build a log names (`version=42.22.2 ...`).
fn build_of(line: &str) -> Option<String> {
    // The game's own line (`...> version=42.22.2 3d39148846 ...`), not java.version= or a mod's.
    let at = line.find("> version=")?;
    let rest = &line[at + "> version=".len()..];
    let v: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    (v.matches('.').count() >= 1 && v.starts_with(|c: char| c.is_ascii_digit())).then_some(v)
}

/// `[17-11-26 05:47:58.253] ...` -> the time as unix seconds (local time) and the stamp.
fn time_of(line: &str) -> Option<(u64, String)> {
    let rest = line.strip_prefix('[')?;
    let (stamp, _) = rest.split_once(']')?;
    let base = stamp.split('.').next()?;
    let t = chrono::NaiveDateTime::parse_from_str(base, "%d-%m-%y %H:%M:%S").ok()?;
    let local = t.and_local_timezone(chrono::Local).earliest()?;
    Some((local.timestamp().max(0) as u64, base.to_string()))
}

/// The unix time of a log line that starts with one (`[17-11-26 05:47:58.253] ...`).
pub fn line_time(line: &str) -> Option<u64> {
    time_of(line).map(|(t, _)| t)
}

/// `LOG  : Mod          f:0> loading RC_RealisticColdMod.` -> the mod id.
fn loaded_mod(line: &str) -> Option<&str> {
    let (head, rest) = line.split_once("> loading ")?;
    if !head.contains(": Mod ") && !head.contains(":Mod ") && !head.trim_end().ends_with("Mod") && !head.contains(" Mod ") {
        return None;
    }
    let id = rest.trim();
    let id = id.strip_suffix('.').unwrap_or(id).trim();
    (!id.is_empty() && !id.contains(char::is_whitespace)).then_some(id)
}

/// Reads one session's log.
pub fn session_facts(text: &str, server: bool) -> SessionFacts {
    let mut facts = SessionFacts { server, ..Default::default() };
    let mut seen_mods = BTreeSet::new();
    let mut maker = CardMaker::new();
    let mut cards: Vec<TranslatedErrorPayload> = Vec::new();
    for line in text.lines() {
        if facts.stamp.is_empty() {
            if let Some((t, s)) = time_of(line) {
                facts.started = t;
                facts.stamp = s;
            }
        }
        if facts.build.is_empty() {
            if let Some(b) = build_of(line) {
                facts.build = b;
            }
        }
        if let Some(id) = loaded_mod(line) {
            if seen_mods.insert(id.to_string()) {
                facts.loaded.push(id.to_string());
            }
        }
        if let Some((t, _)) = time_of(line) {
            facts.last_line = t;
        }
        if let Some(rest) = line.split_once("game loading took ").map(|(_, r)| r) {
            facts.reached = "game".to_string();
            facts.load_secs = rest.split_whitespace().next().and_then(|n| n.parse().ok());
        } else if facts.reached.is_empty()
            && line.contains("STATE: exit zombie.gameStates.")
            && (line.contains("TermsOfServiceState") || line.contains("MainScreenState") || line.contains("LoadingQueueState"))
        {
            // The game writes only when it leaves a screen: past the terms screen it is at the main menu.
            facts.reached = "menu".to_string();
        } else if server && line.contains("SERVER STARTED") {
            facts.reached = "started".to_string();
        }
        if line.contains("GameThread exited") {
            facts.clean_exit = true;
        }
        if line.contains("java.lang.OutOfMemoryError") {
            facts.oom = true;
        }
        cards.extend(maker.line(line));
    }
    cards.extend(maker.flush());
    facts.cards = cards.into_iter().filter(|c| c.severity == CardSeverity::Exception && c.mod_name.is_some()).collect();
    facts
}

/// What makes two cards one error.
pub fn error_key(card: &TranslatedErrorPayload) -> String {
    format!("{}|{}|{}", card.code, card.source_file.as_deref().unwrap_or(""), line_shape(&card.raw_error))
}

/// A log line's shape, the server's ` st:<ticks>` after `f:<frame>` taken out first (the same
/// line in a server log and in a game log is one error).
pub fn line_shape(line: &str) -> String {
    let mut text = line.to_string();
    if let Some(at) = text.find(" st:") {
        let end = text[at + 4..].find(|c: char| !(c.is_ascii_digit() || c == ',')).map(|e| at + 4 + e).unwrap_or(text.len());
        text.replace_range(at..end, "");
    }
    crate::sandbox::history::error_key(&text)
}

fn index_of<T: PartialEq>(list: &mut Vec<T>, item: T) -> u32 {
    match list.iter().position(|x| *x == item) {
        Some(at) => at as u32,
        None => {
            list.push(item);
            (list.len() - 1) as u32
        }
    }
}

/// Puts one session in: the mods it had on (those still installed as they were then) and the
/// errors it blamed on them. A session read before (the same id) is replaced.
pub fn record(learned: &mut Learned, facts: &SessionFacts, installed: &dyn Fn(&str) -> Option<ModNow>) {
    if facts.stamp.is_empty() || facts.build.is_empty() {
        return;
    }
    let kind = if facts.server { "server" } else { "client" };
    let id = format!("{}|{}", kind, facts.stamp);
    let mut session = LearnedSession {
        id: id.clone(),
        started: facts.started,
        kind: kind.to_string(),
        build: facts.build.clone(),
        reached: facts.reached.clone(),
        clean_exit: facts.clean_exit,
        oom: facts.oom,
        load_secs: facts.load_secs,
        last_line: facts.last_line.max(facts.started),
        ..Default::default()
    };
    for mod_id in &facts.loaded {
        let at = index_of(&mut learned.ids, mod_id.clone());
        session.loaded.push(at);
    }
    // name -> id, among this session's mods
    let mut by_name: BTreeMap<String, Vec<(String, ModNow)>> = BTreeMap::new();
    for mod_id in &facts.loaded {
        let Some(mut now) = installed(mod_id) else { continue };
        if now.skip {
            continue;
        }
        // Its files changed after the session: which version ran then is not known.
        if now.changed > facts.started {
            now.version = UNKNOWN.to_string();
        }
        let at = index_of(&mut learned.mods, (mod_id.clone(), now.version.clone()));
        if !session.mods.contains(&at) {
            session.mods.push(at);
        }
        by_name.entry(now.name.clone()).or_default().push((mod_id.clone(), now));
    }
    for card in &facts.cards {
        let Some(name) = card.mod_name.as_deref() else { continue };
        // A Java patch two or more mods make to the same method: the card names one of them, but
        // the log does not say whose failed.
        if card.params.get("suspects").is_some_and(|s| s.contains(',')) {
            continue;
        }
        let Some(owners) = by_name.get(name) else { continue };
        // A name two mods of the session share says nothing about which one it was.
        if owners.len() != 1 {
            continue;
        }
        let (mod_id, now) = &owners[0];
        let error = LearnedError {
            mod_id: mod_id.clone(),
            version: now.version.clone(),
            key: error_key(card),
            title: card.title.clone(),
            code: card.code.clone(),
            params: card.params.clone(),
            file: card.source_file.clone(),
            line: card.line_number,
            sample: card.raw_error.chars().take(300).collect(),
        };
        let at = match learned.errors.iter().position(|e| e.mod_id == error.mod_id && e.version == error.version && e.key == error.key) {
            Some(at) => at as u32,
            None => {
                learned.errors.push(error);
                (learned.errors.len() - 1) as u32
            }
        };
        if !session.errors.contains(&at) {
            session.errors.push(at);
        }
    }
    learned.sessions.retain(|s| s.id != id);
    learned.sessions.push(session);
    learned.sessions.sort_by_key(|s| s.started);
    if learned.sessions.len() > SESSIONS_KEPT {
        let drop = learned.sessions.len() - SESSIONS_KEPT;
        learned.sessions.drain(..drop);
    }
}

/// The findings: errors seen in [`MIN_SESSIONS`] or more sessions on `build`, for the version
/// installed now (`installed_version`), closed or not.
pub fn issues(learned: &Learned, build: &str, installed_version: &dyn Fn(&str) -> Option<String>) -> Vec<LearnedIssue> {
    let sessions: Vec<&LearnedSession> = learned.sessions.iter().filter(|s| same_build(&s.build, build)).collect();
    let mut out = Vec::new();
    for (e_at, error) in learned.errors.iter().enumerate() {
        if installed_version(&error.mod_id).as_deref() != Some(error.version.as_str()) {
            continue;
        }
        let Some(m_at) = learned.mods.iter().position(|(m, v)| *m == error.mod_id && *v == error.version) else { continue };
        let (m_at, e_at) = (m_at as u32, e_at as u32);
        let with_mod: Vec<&&LearnedSession> = sessions.iter().filter(|s| s.mods.contains(&m_at)).collect();
        let seen: Vec<&&LearnedSession> = with_mod.iter().copied().filter(|s| s.errors.contains(&e_at)).collect();
        if (seen.len() as u32) < MIN_SESSIONS {
            continue;
        }
        let closed = with_mod.len() >= CLOSED_AFTER && with_mod.iter().rev().take(CLOSED_AFTER).all(|s| !s.errors.contains(&e_at));
        out.push(LearnedIssue {
            mod_id: error.mod_id.clone(),
            version: error.version.clone(),
            key: error.key.clone(),
            title: error.title.clone(),
            code: error.code.clone(),
            params: error.params.clone(),
            file: error.file.clone(),
            line: error.line,
            sample: error.sample.clone(),
            build: build.to_string(),
            seen: seen.len() as u32,
            on: with_mod.len() as u32,
            first_seen: seen.iter().map(|s| s.started).min().unwrap_or(0),
            last_seen: seen.iter().map(|s| s.started).max().unwrap_or(0),
            closed,
        });
    }
    out.sort_by(|a, b| b.seen.cmp(&a.seen).then(a.mod_id.cmp(&b.mod_id)));
    out
}

/// The session logs the game kept: `Logs/*_DebugLog*.txt` and the same in its `logs_*` folders.
fn session_logs(zomboid: &Path) -> Vec<(PathBuf, bool)> {
    let logs = zomboid.join("Logs");
    let mut dirs = vec![logs.clone()];
    if let Ok(rd) = std::fs::read_dir(&logs) {
        dirs.extend(rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
    }
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("_DebugLog.txt") {
                out.push((e.path(), false));
            } else if name.ends_with("_DebugLog-server.txt") {
                out.push((e.path(), true));
            }
        }
    }
    out
}

/// The installed mods as the learner needs them.
pub fn installed_mods(catalog: &crate::catalog::ModCatalog) -> BTreeMap<String, ModNow> {
    let mut out = BTreeMap::new();
    for m in catalog.mods.iter().filter(|m| m.shadowed_by.is_none()) {
        let root = Path::new(&m.root_dir);
        let mut infos = vec![root.join("mod.info"), root.join("common").join("mod.info")];
        infos.extend(m.version_folders.iter().map(|v| root.join(v).join("mod.info")));
        let changed = infos
            .iter()
            .filter_map(|p| std::fs::metadata(p).and_then(|md| md.modified()).ok())
            .max()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let version = match m.info.mod_version.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            Some(v) => v.to_string(),
            None => format!(
                "files of {}",
                chrono::DateTime::<chrono::Local>::from(std::time::UNIX_EPOCH + std::time::Duration::from_secs(changed)).format("%Y-%m-%d")
            ),
        };
        let skip = crate::security::is_studio_mod(&m.id) || m.in_development;
        out.insert(m.id.clone(), ModNow { version, changed, name: m.name.clone(), skip });
    }
    out
}

/// Reads the session logs not read yet (or grown since), for the installed build only. Returns
/// how many sessions were learned from.
/// The logs not read yet (or grown since): path, server, path under the Zomboid folder, size.
fn pending(zomboid: &Path, learned: &Learned) -> Vec<(PathBuf, bool, String, u64)> {
    let mut todo = Vec::new();
    for (path, server) in session_logs(zomboid) {
        let Ok(md) = std::fs::metadata(&path) else { continue };
        let rel = path.strip_prefix(zomboid).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        if learned.files.get(&rel) == Some(&md.len()) {
            continue;
        }
        todo.push((path, server, rel, md.len()));
    }
    todo
}

/// Whether there is a log or a crash report to learn from.
pub fn has_pending(zomboid: &Path) -> bool {
    let learned = load(zomboid);
    !pending(zomboid, &learned).is_empty() || diary_changed(zomboid, &learned) || crash_files().iter().any(|(name, _)| !learned.files.contains_key(&format!("hs_err:{}", name)))
}

/// Java's crash reports in the game's folder (name, path).
fn crash_files() -> Vec<(String, PathBuf)> {
    let dir = crate::vfs::trusted_install_dir();
    if dir.is_empty() {
        return vec![];
    }
    let Ok(rd) = std::fs::read_dir(&dir) else { return vec![] };
    rd.flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            (name.starts_with("hs_err_pid") && name.ends_with(".log")).then(|| (name, e.path()))
        })
        .collect()
}

/// Reads the Link's diary: the events not kept yet (a few hundred kept), and trims the file. How many were new.
fn read_diary(zomboid: &Path, learned: &mut Learned) -> usize {
    let path = zomboid.join("Lua").join(DIARY_FILE);
    let Ok(text) = std::fs::read_to_string(&path) else { return 0 };
    let key = "diary:size".to_string();
    if learned.files.get(&key) == Some(&(text.len() as u64)) {
        return 0;
    }
    let events = read_diary_text(&text);
    let mut n = 0;
    for e in &events {
        if !learned.diary.iter().any(|x| x.event == e.event && x.at == e.at) {
            learned.diary.push(e.clone());
            n += 1;
        }
    }
    learned.diary.sort_by_key(|e| e.at);
    if learned.diary.len() > 1000 {
        let drop = learned.diary.len() - 1000;
        learned.diary.drain(..drop);
    }
    // the game only ever appends to it: the app keeps it short
    let lines: Vec<&str> = text.lines().collect();
    let kept = if lines.len() > DIARY_KEPT {
        let t = lines[lines.len() - DIARY_KEPT..].join("\n") + "\n";
        let _ = std::fs::write(&path, &t);
        t.len()
    } else {
        text.len()
    };
    learned.files.insert(key, kept as u64);
    n
}

/// Whether the Link's diary changed since it was read.
fn diary_changed(zomboid: &Path, learned: &Learned) -> bool {
    std::fs::metadata(zomboid.join("Lua").join(DIARY_FILE)).is_ok_and(|m| learned.files.get("diary:size") != Some(&m.len()))
}

/// Reads the crash reports not read yet; how many were new.
fn read_new_crashes(learned: &mut Learned) -> usize {
    let mut n = 0;
    for (name, path) in crash_files() {
        let key = format!("hs_err:{}", name);
        if learned.files.contains_key(&key) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        if let Some(c) = read_jvm_crash(&name, &String::from_utf8_lossy(&bytes)) {
            learned.crashes.retain(|x| x.file != c.file);
            learned.crashes.push(c);
            n += 1;
        }
        learned.files.insert(key, bytes.len() as u64);
    }
    learned.crashes.sort_by_key(|c| c.at);
    n
}

pub fn learn(zomboid: &Path, catalog: &crate::catalog::ModCatalog) -> Result<usize, String> {
    let mut learned = load(zomboid);
    // Read with an older way of telling errors apart: read again.
    if learned.version < VERSION {
        learned = Learned { version: VERSION, ..Default::default() };
    }
    let todo = pending(zomboid, &learned);
    let new_crashes = read_new_crashes(&mut learned) + read_diary(zomboid, &mut learned);
    if todo.is_empty() {
        if new_crashes > 0 {
            save(zomboid, &learned)?;
        }
        return Ok(0);
    }
    let mods = installed_mods(catalog);
    let installed = |id: &str| mods.get(id).cloned();
    let mut learned_from = 0;
    for (path, server, rel, len) in todo {
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let text = String::from_utf8_lossy(&bytes);
        // Every build is kept: what a game update changed is told from the sessions before it.
        let facts = session_facts(&text, server);
        if !facts.build.is_empty() {
            record(&mut learned, &facts, &installed);
            let id = format!("{}|{}", if server { "server" } else { "client" }, facts.stamp);
            if let Some(s) = learned.sessions.iter_mut().find(|s| s.id == id) {
                s.file = rel.clone();
            }
            learned_from += 1;
        }
        learned.files.insert(rel, len);
    }
    // Files gone from disk are forgotten (their sessions stay).
    learned.files.retain(|rel, _| rel.starts_with("hs_err:") || rel.starts_with("diary:") || zomboid.join(rel).exists());
    save(zomboid, &learned)?;
    Ok(learned_from)
}

/// The findings for the mods installed now.
pub fn current_issues(zomboid: &Path, catalog: &crate::catalog::ModCatalog) -> Vec<LearnedIssue> {
    let build = catalog.game_version.clone().unwrap_or_default();
    let learned = load(zomboid);
    if learned.sessions.is_empty() {
        return vec![];
    }
    let mods = installed_mods(catalog);
    issues(&learned, &build, &|id: &str| mods.get(id).map(|m| m.version.clone()))
}

/// The mod index the app is built with already says it: an issue of the same mod, for the
/// installed version, whose log line makes the same kind of card (and names the same file).
pub fn index_says_it(issue: &LearnedIssue, registry: &crate::rules::index::Registry, installed_version: Option<&str>) -> bool {
    let base = |f: &str| f.rsplit(['/', '\\']).next().unwrap_or(f).to_string();
    registry.issues.iter().filter(|i| crate::rules::index::issue_applies_to_version(i, installed_version)).any(|i| {
        let Some(log) = i.log.as_deref() else {
            return issue.file.is_some() && issue.file.as_deref().map(base) == i.file.as_deref().map(base);
        };
        if line_shape(log) == line_shape(&issue.sample) {
            return true;
        }
        let mut maker = CardMaker::new();
        let mut cards = maker.line(log);
        cards.extend(maker.flush());
        cards.iter().any(|c| c.code == issue.code && (issue.file.is_none() || issue.file.as_deref().map(base) == i.file.as_deref().map(base)))
    })
}

/// How much the PC has learned: sessions, mods and errors counted, and the findings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LearnedSummary {
    pub sessions: usize,
    pub sessions_on_build: usize,
    pub build: String,
    pub logs_read: usize,
    pub first_session: Option<u64>,
    pub last_session: Option<u64>,
    pub issues: Vec<LearnedIssue>,
}

pub fn summary(zomboid: &Path, catalog: &crate::catalog::ModCatalog) -> LearnedSummary {
    let build = catalog.game_version.clone().unwrap_or_default();
    let learned = load(zomboid);
    let mods = installed_mods(catalog);
    LearnedSummary {
        sessions: learned.sessions.len(),
        sessions_on_build: learned.sessions.iter().filter(|s| same_build(&s.build, &build)).count(),
        logs_read: learned.files.len(),
        first_session: learned.sessions.first().map(|s| s.started),
        last_session: learned.sessions.last().map(|s| s.started),
        issues: issues(&learned, &build, &|id: &str| mods.get(id).map(|m| m.version.clone())),
        build,
    }
}

/// Learns every few minutes in the background: at the start (the logs kept so far) and after each
/// session the game or a server writes.
pub fn learn_in_background() {
    std::thread::spawn(|| loop {
        let paths = crate::vfs::studio_paths();
        if !paths.user_zomboid_dir.is_empty() {
            if let Some(zomboid) = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next() {
                if !has_pending(&zomboid) {
                    std::thread::sleep(std::time::Duration::from_secs(180));
                    continue;
                }
                let started = std::time::Instant::now();
                let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
                match learn(&zomboid, &catalog) {
                    Ok(0) => {}
                    Ok(n) => crate::diagnostics::log("INFO", &format!("Learned from {} game session(s) in {:.1}s", n, started.elapsed().as_secs_f32())),
                    Err(e) => crate::diagnostics::log("WARN", &format!("Could not keep what this PC learned: {}", e)),
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(180));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "[17-11-26 05:47:58.253] LOG  : General      f:0> version=42.22.2 3d39148846 demo\n\
[17-11-26 05:48:00.000] LOG  : Mod          f:0> loading RC_RealisticColdMod.\n\
[17-11-26 05:48:00.001] LOG  : Mod          f:0> loading OtherMod.\n\
[17-11-26 05:49:00.000] ERROR: General      f:3> Lua((MOD:Realistic Temperature)).edgeHasBlockingOnSquare> Exception thrown\n\
\tjava.lang.RuntimeException: No implementation found for function: get at KahluaThread.call(KahluaThread.java:1).\n\
\tLua((MOD:Realistic Temperature)).edgeHasBlockingOnSquare(RC_RoomLogic.lua:439)\n\
[17-11-26 05:49:01.000] LOG  : General      f:3> after\n";

    fn now(name: &str, version: &str) -> ModNow {
        ModNow { version: version.into(), changed: 0, name: name.into(), skip: false }
    }

    fn installed(id: &str) -> Option<ModNow> {
        match id {
            "RC_RealisticColdMod" => Some(now("Realistic Temperature", "2.4.0")),
            "OtherMod" => Some(now("Other", "1.0")),
            _ => None,
        }
    }

    fn session(n: u32, with_error: bool) -> String {
        let text = LOG.replace("17-11-26 05:4", &format!("1{}-11-26 05:4", n));
        if with_error { text } else { text.lines().filter(|l| !l.contains("RuntimeException") && !l.contains("MOD:")).collect::<Vec<_>>().join("\n") }
    }

    #[test]
    fn reads_the_build_the_mods_and_the_errors_of_a_session() {
        let f = session_facts(LOG, false);
        assert_eq!(f.build, "42.22.2");
        assert_eq!(f.loaded, vec!["RC_RealisticColdMod", "OtherMod"]);
        assert_eq!(f.stamp, "17-11-26 05:47:58");
        assert_eq!(f.cards.len(), 1);
        assert_eq!(f.cards[0].mod_name.as_deref(), Some("Realistic Temperature"));
        assert_eq!((f.reached.as_str(), f.clean_exit, f.oom), ("", false, false));
        let more = format!("{}[17-11-26 05:50:00.000] LOG  : General      f:0> STATE: exit zombie.gameStates.TermsOfServiceState.
[17-11-26 05:52:00.000] LOG  : General      f:0> game loading took 95 seconds.
[17-11-26 06:00:00.000] LOG  : General      f:9> GameThread exited..
", LOG);
        let f = session_facts(&more, false);
        assert_eq!((f.reached.as_str(), f.clean_exit, f.load_secs), ("game", true, Some(95)));
    }

    #[test]
    fn reads_the_link_s_diary_and_ties_it_to_its_session() {
        let ev = read_diary_text("start\t1000\tSurvivor/My World\t10,20,0\nmark\t1500\tSurvivor/My World\t11,21,0\nmenu\t1900\tSurvivor/My World\t\njunk line\n");
        assert_eq!(ev.len(), 3);
        assert_eq!((ev[1].event.as_str(), ev[1].at, ev[1].place.as_str()), ("mark", 1500, "11,21,0"));
        let mut l = Learned { diary: ev, ..Default::default() };
        l.sessions.push(LearnedSession { kind: "client".into(), started: 990, last_line: 1880, ..Default::default() });
        let (save, menu) = l.diary_of(&l.sessions[0].clone());
        assert_eq!((save.as_str(), menu), ("Survivor/My World", true));
        assert!(l.session_at(1500).is_some());
        assert!(l.session_at(5000).is_none());
    }

    #[test]
    fn reads_java_s_crash_reports() {
        let oom = "#\n# There is insufficient memory for the Java Runtime Environment to continue.\n# Native memory allocation (malloc) failed\n#\nCommand Line: -Xmx6144m -cachedir=C:\\Users\\x\\Zomboid2\nTime: Mon Nov 16 00:34:40 2026 Argentina Standard Time elapsed time: 468.286953 seconds (0d 0h 7m 48s)\njava_command: zombie.network.GameServer -servername knoxtest\n";
        let c = read_jvm_crash("hs_err_pid7340.log", oom).unwrap();
        assert_eq!((c.kind.as_str(), c.role.as_str(), c.elapsed, c.second_player), ("out_of_memory", "server", 468, true));
        assert!(c.at > 0);
        let fatal = "#\n# A fatal error has been detected by the Java Runtime Environment:\n#\n#  EXCEPTION_ACCESS_VIOLATION (0xc0000005) at pc=0x00007ffd06337cf2, pid=26564, tid=740\n#\n# Problematic frame:\n# C  [ZNetJNI64.dll+0x17cf2]\n#\nTime: Thu Oct 22 17:35:49 2026 Argentina Standard Time elapsed time: 1006.283305 seconds\njava_command: <unknown>\n";
        let c = read_jvm_crash("hs_err_pid26564.log", fatal).unwrap();
        assert_eq!((c.kind.as_str(), c.reason.as_str(), c.frame.as_str(), c.role.as_str()), ("fatal", "EXCEPTION_ACCESS_VIOLATION", "ZNetJNI64.dll", "client"));
        // a session that began when the process did is the crash's
        let mut l = Learned::default();
        l.crashes.push(c.clone());
        let s = LearnedSession { kind: "client".into(), started: c.at - 1006 + 20, last_line: c.at - 5, ..Default::default() };
        assert!(l.crash_of(&s).is_some());
        let other = LearnedSession { kind: "server".into(), ..s.clone() };
        assert!(l.crash_of(&other).is_none());
    }

    #[test]
    fn three_sessions_make_a_finding_and_five_without_close_it() {
        let mut l = Learned::default();
        for n in 0..2 {
            record(&mut l, &session_facts(&session(n, true), false), &installed);
        }
        let v = |id: &str| installed(id).map(|m| m.version);
        assert!(issues(&l, "42.22.2", &v).is_empty(), "two sessions are not enough");
        record(&mut l, &session_facts(&session(2, true), false), &installed);
        let found = issues(&l, "42.22.2", &v);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].seen, found[0].on, found[0].closed), (3, 3, false));
        assert_eq!(found[0].mod_id, "RC_RealisticColdMod");
        // the same session read again (archived) is not counted twice
        record(&mut l, &session_facts(&session(2, true), false), &installed);
        assert_eq!(issues(&l, "42.22.2", &v)[0].seen, 3);
        // another build says nothing
        assert!(issues(&l, "42.21", &v).is_empty());
        assert_eq!(issues(&l, "42.22", &v).len(), 1, "the catalog says 42.22 for 42.22.2");
        for n in 3..8 {
            record(&mut l, &session_facts(&session(n, false), false), &installed);
        }
        let found = issues(&l, "42.22.2", &v);
        assert_eq!((found[0].seen, found[0].on, found[0].closed), (3, 8, true));
    }

    #[test]
    fn an_update_starts_the_count_again() {
        let mut l = Learned::default();
        for n in 0..3 {
            record(&mut l, &session_facts(&session(n, true), false), &installed);
        }
        let newer = |id: &str| if id == "RC_RealisticColdMod" { Some("2.5.0".to_string()) } else { installed(id).map(|m| m.version) };
        assert!(issues(&l, "42.22.2", &newer).is_empty());
    }

    #[test]
    fn a_session_before_the_mod_changed_and_the_studio_s_own_mods_are_left_out() {
        let mut l = Learned::default();
        let later = |id: &str| installed(id).map(|mut m| {
            m.changed = u64::MAX;
            m
        });
        record(&mut l, &session_facts(LOG, false), &later);
        // kept with the version unknown: history, never a finding
        assert!(l.errors.iter().all(|e| e.version == UNKNOWN));
        assert!(issues(&l, "42.22", &|id: &str| installed(id).map(|m| m.version)).is_empty());
        let mut l = Learned::default();
        let own = |id: &str| installed(id).map(|mut m| {
            m.skip = true;
            m
        });
        record(&mut l, &session_facts(LOG, false), &own);
        assert!(l.errors.is_empty());
    }
}

#[cfg(test)]
mod on_this_pc {
    /// `cargo test --lib learned::on_this_pc -- --ignored --nocapture`: learns from this PC's logs
    /// (writing its PZModStudio_Learned.json, as the app does) and prints the findings.
    #[test]
    #[ignore]
    fn learn_from_this_pc() {
        let paths = crate::vfs::auto_detect_paths();
        let zomboid = std::path::PathBuf::from(&paths.user_zomboid_dir);
        let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
        let t = std::time::Instant::now();
        let n = super::learn(&zomboid, &catalog).unwrap();
        println!("learned from {} sessions in {:?}; build {:?}", n, t.elapsed(), catalog.game_version);
        let s = super::summary(&zomboid, &catalog);
        println!("sessions {} on build {} logs {}", s.sessions, s.sessions_on_build, s.logs_read);
        for i in s.issues {
            println!("{:>3}/{:<3} {} {} [{}] {} {:?}:{:?} | {}", i.seen, i.on, if i.closed { "CLOSED" } else { "open  " }, i.mod_id, i.version, i.title, i.file, i.line, i.sample.chars().take(120).collect::<String>());
        }
    }
}

#[cfg(test)]
mod frame_cause_tests {
    #[test]
    fn libraries_are_said_by_what_they_are() {
        use super::frame_cause;
        assert_eq!(frame_cause("nvoglv64.dll"), "graphics_nvidia");
        assert_eq!(frame_cause("atio6axx.dll"), "graphics_amd");
        assert_eq!(frame_cause("ig9icd64.dll"), "graphics_intel");
        assert_eq!(frame_cause("igxelpicd64.dll"), "graphics_intel");
        assert_eq!(frame_cause("lwjgl_opengl.dll"), "window_opengl");
        assert_eq!(frame_cause("fmod.dll"), "audio");
        assert_eq!(frame_cause("ZNetJNI64.dll"), "network");
        assert_eq!(frame_cause("zbNative.dll"), "zombiebuddy");
        assert_eq!(frame_cause("jvm.dll"), "java");
        assert_eq!(frame_cause("something.dll"), "");
    }
}
