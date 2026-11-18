//! Restarts for the Studio's server while PZ Mod Studio is open: at times of day or every N hours,
//! when one of its Workshop mods is updated, or asked for now, each with in-game warnings over
//! RCON (`servermsg`), then `save` and `quit`, and started again the same way; and started again
//! when it stops by itself, up to a limit.
//!
//! The schedule (`Zomboid/PZModStudio_ServerSchedule.json`) is written by the window and by
//! assistants; what happened and what is coming (`Zomboid/PZModStudio_ServerRestarts.json`) only
//! by the watcher, which runs inside the app. A restart asked from another process (an assistant
//! through MCP) is a request file the watcher picks up. A stop asked anywhere leaves its time in
//! `Zomboid/PZModStudio_ServerStopAsked.txt`, so the watcher never takes it for a crash.

use chrono::{Local, NaiveDateTime, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const SCHEDULE_FILE: &str = "PZModStudio_ServerSchedule.json";
const STATE_FILE: &str = "PZModStudio_ServerRestarts.json";
const REQUEST_FILE: &str = "PZModStudio_ServerRestartRequest.json";
const STOP_FILE: &str = "PZModStudio_ServerStopAsked.txt";
const EVERY_SECS: u64 = 5;
/// The watcher writes its heartbeat this often; older than three of them, it is not running.
const HEARTBEAT_SECS: u64 = 30;
const HISTORY_MAX: usize = 40;
/// A scheduled restart this soon after the server started is skipped: it was just restarted.
const SKIP_AFTER_START_SECS: i64 = 30 * 60;

/// One server's restart settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Schedule {
    /// Scheduled restarts on (times and every_hours).
    pub enabled: bool,
    /// Times of day, "HH:MM", this PC's clock.
    pub times: Vec<String>,
    /// Every N hours from the server's start; 0 for none.
    pub every_hours: u32,
    /// Minutes before a restart the players are told (30, 15, 5, 1).
    pub warn_minutes: Vec<u32>,
    /// Start it again when it stops by itself.
    pub on_crash: bool,
    /// At most this many crash restarts in `crash_window_minutes`; then it stays off.
    pub crash_limit: u32,
    pub crash_window_minutes: u32,
    /// Restart (with the warnings) when a Workshop item of its WorkshopItems= is updated.
    pub on_mod_update: bool,
    pub mod_check_minutes: u32,
    /// The language of the in-game messages: "en" or "es".
    pub language: String,
}

impl Default for Schedule {
    fn default() -> Self {
        Schedule {
            enabled: false,
            times: Vec::new(),
            every_hours: 0,
            warn_minutes: vec![30, 15, 5, 1],
            on_crash: true,
            crash_limit: 3,
            crash_window_minutes: 10,
            on_mod_update: false,
            mod_check_minutes: 15,
            language: "en".into(),
        }
    }
}

impl Schedule {
    /// The same settings within their limits: valid times only, sorted, warnings 1-120 min.
    pub fn cleaned(mut self) -> Self {
        let mut times: Vec<String> = self.times.iter().filter_map(|t| parse_hhmm(t)).map(|t| t.format("%H:%M").to_string()).collect();
        times.sort();
        times.dedup();
        self.times = times;
        self.every_hours = self.every_hours.min(168);
        let mut w: Vec<u32> = self.warn_minutes.into_iter().filter(|m| (1..=120).contains(m)).collect();
        w.sort_unstable_by(|a, b| b.cmp(a));
        w.dedup();
        self.warn_minutes = w;
        self.crash_limit = self.crash_limit.clamp(1, 20);
        self.crash_window_minutes = self.crash_window_minutes.clamp(1, 240);
        self.mod_check_minutes = self.mod_check_minutes.clamp(5, 24 * 60);
        if self.language != "es" {
            self.language = "en".into();
        }
        self
    }

    fn longest_warning(&self) -> u32 {
        self.warn_minutes.iter().copied().max().unwrap_or(0)
    }
}

pub fn parse_hhmm(text: &str) -> Option<NaiveTime> {
    let (h, m) = text.trim().split_once(':')?;
    NaiveTime::from_hms_opt(h.trim().parse().ok()?, m.trim().parse().ok()?, 0)
}

/// The next scheduled restart after `now`, or None. Times of day are this PC's clock; every N hours
/// counts from the server's start; one less than 30 minutes after the start is skipped.
pub fn next_scheduled(s: &Schedule, now: NaiveDateTime, started: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
    if !s.enabled {
        return None;
    }
    let too_soon = |t: NaiveDateTime| started.is_some_and(|st| (t - st).num_seconds() < SKIP_AFTER_START_SECS);
    let mut best: Option<NaiveDateTime> = None;
    let mut consider = |t: NaiveDateTime| {
        if t > now && !too_soon(t) && best.is_none_or(|b| t < b) {
            best = Some(t);
        }
    };
    for time in s.times.iter().filter_map(|t| parse_hhmm(t)) {
        for day in 0..3 {
            consider((now.date() + chrono::Duration::days(day)).and_time(time));
        }
    }
    if let (Some(st), true) = (started, s.every_hours > 0) {
        let step = s.every_hours as i64 * 3600;
        let since = (now - st).num_seconds().max(0);
        let k = since / step + 1;
        consider(st + chrono::Duration::seconds(k * step));
        consider(st + chrono::Duration::seconds((k + 1) * step));
    }
    best
}

fn to_unix(t: NaiveDateTime) -> u64 {
    Local.from_local_datetime(&t).earliest().map(|d| d.timestamp().max(0) as u64).unwrap_or(0)
}

fn to_local(secs: u64) -> NaiveDateTime {
    Local.timestamp_opt(secs as i64, 0).single().map(|d| d.naive_local()).unwrap_or_default()
}

/// A restart coming: when, why, and the warnings already given.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Pending {
    pub server: String,
    /// Unix seconds.
    pub at: u64,
    /// "schedule", "mod_update" or "manual".
    pub reason: String,
    /// For a mod update, the items' titles; for a manual one, who asked (app / assistant).
    pub detail: String,
    pub warned: BTreeSet<u32>,
    /// The first message was given (for a countdown shorter than any warning).
    pub announced: bool,
}

impl Pending {
    /// A countdown to `at`: warnings longer than the time left are not due (they would lie).
    pub fn new(server: &str, now: u64, at: u64, reason: &str, detail: &str, warn: &[u32]) -> Self {
        let left = at.saturating_sub(now);
        let warned = warn.iter().copied().filter(|m| *m as u64 * 60 > left + 30).collect();
        Pending { server: server.into(), at, reason: reason.into(), detail: detail.into(), warned, announced: false }
    }

    /// The warnings due now, in minutes left (each once; the first one said is the time actually
    /// left when no warning is due yet), or None once it is time.
    pub fn step(&mut self, now: u64, warn: &[u32]) -> Option<Vec<u32>> {
        if now >= self.at {
            return None;
        }
        let left = self.at - now;
        let mut due: Vec<u32> = warn.iter().copied().filter(|m| left <= *m as u64 * 60 && !self.warned.contains(m)).collect();
        due.sort_unstable_by(|a, b| b.cmp(a));
        // Several due at once (the app was busy): only the nearest is said.
        if due.len() > 1 {
            due.drain(..due.len() - 1);
        }
        for m in warn.iter().filter(|m| left <= **m as u64 * 60) {
            self.warned.insert(*m);
        }
        if !self.announced {
            self.announced = true;
            if due.is_empty() {
                due.push(left.div_ceil(60).max(1) as u32);
            }
        }
        Some(due)
    }
}

/// What the in-game messages say.
pub fn game_message(lang: &str, what: &str, minutes: u32, reason: &str) -> String {
    let es = lang == "es";
    let why = match (reason, es) {
        ("mod_update", false) => " (a mod was updated)",
        ("mod_update", true) => " (se actualizó un mod)",
        ("schedule", false) => " (scheduled)",
        ("schedule", true) => " (programado)",
        _ => "",
    };
    match (what, es) {
        ("warn", false) if minutes == 1 => format!("Server restart in 1 minute{}. Find a safe place.", why),
        ("warn", false) => format!("Server restart in {} minutes{}.", minutes, why),
        ("warn", true) if minutes == 1 => format!("Reinicio del servidor en 1 minuto{}. Buscá un lugar seguro.", why),
        ("warn", true) => format!("Reinicio del servidor en {} minutos{}.", minutes, why),
        ("now", false) => "Server restarting now: the world is being saved. Back in a few minutes.".into(),
        ("now", true) => "El servidor se reinicia ahora: se guarda el mundo. Volvé en unos minutos.".into(),
        ("cancel", false) => "The server restart was called off.".into(),
        (_, true) => "Se canceló el reinicio del servidor.".into(),
        _ => String::new(),
    }
}

/// One restart, or a stop that was not followed by one.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct RestartEvent {
    pub at: u64,
    pub server: String,
    /// "schedule", "mod_update", "manual", "crash", "gave_up" (too many crashes), "crash_no_restart"
    /// (stopped by itself; not started again, `detail` says why), "stopped_while_closed".
    pub reason: String,
    pub ok: bool,
    pub detail: String,
}

/// The server the watcher last saw running, as it was started.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Watched {
    pub server: String,
    pub pid: Option<u32>,
    pub memory_gb: Option<u32>,
    pub nosteam: Option<bool>,
    pub started_at: u64,
    /// Its log said SERVER STARTED this run.
    pub started_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct RestartState {
    /// Unix seconds of the watcher's last look (the app is open while it is recent).
    pub heartbeat: u64,
    pub pending: Option<Pending>,
    /// Newest first.
    pub history: Vec<RestartEvent>,
    /// Crash restarts lately (unix seconds).
    pub crashes: Vec<u64>,
    /// Too many crashes: no more crash restarts until the server is started again.
    pub gave_up_at: Option<u64>,
    pub watching: Option<Watched>,
    /// A restart under way, since.
    pub restarting_since: Option<u64>,
    pub mods_checked_at: u64,
    /// The Workshop updates already acted on ("id@time_updated").
    pub mods_seen: BTreeSet<String>,
    /// The next run is the watcher's own restart (not someone starting the server by hand).
    pub expect_own_run: bool,
    /// A scheduled restart called off (its time, unix seconds): not counted down again.
    pub skipped_at: u64,
}

impl RestartState {
    fn record(&mut self, e: RestartEvent) {
        self.history.insert(0, e);
        self.history.truncate(HISTORY_MAX);
    }
}

/// Whether a server that stopped may be started again: it was not asked to stop, it had come up
/// (one that fails at boot will fail again), and not too many times lately. Prunes `crashes`
/// and, when the answer is to restart, notes this one.
pub fn crash_decision(crashes: &mut Vec<u64>, now: u64, limit: u32, window_minutes: u32) -> bool {
    let window = window_minutes as u64 * 60;
    crashes.retain(|t| now.saturating_sub(*t) < window);
    if crashes.len() as u32 >= limit {
        return false;
    }
    crashes.push(now);
    true
}

// ------------------------------------------------------------------ files

fn read_json<T: for<'a> Deserialize<'a> + Default>(path: &Path) -> T {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(value).unwrap_or_default()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn schedules(z: &Path) -> BTreeMap<String, Schedule> {
    read_json::<BTreeMap<String, Schedule>>(&z.join(SCHEDULE_FILE))
}

pub fn schedule_of(z: &Path, server: &str) -> Schedule {
    schedules(z).remove(server).unwrap_or_default()
}

pub fn save_schedule(z: &Path, server: &str, s: Schedule) -> Result<Schedule, String> {
    if server.trim().is_empty() {
        return Err("No server name.".into());
    }
    let s = s.cleaned();
    let mut all = schedules(z);
    all.insert(server.to_string(), s.clone());
    write_json(&z.join(SCHEDULE_FILE), &all)?;
    Ok(s)
}

pub fn state(z: &Path) -> RestartState {
    read_json(&z.join(STATE_FILE))
}

fn save_state(z: &Path, st: &RestartState) {
    let _ = write_json(&z.join(STATE_FILE), st);
}

/// Called by every stop the Studio is asked for, in any process: what follows is not a crash.
pub fn note_stop_asked(z: &Path) {
    let _ = std::fs::write(z.join(STOP_FILE), now_secs().to_string());
}

fn stop_asked_at(z: &Path) -> u64 {
    std::fs::read_to_string(z.join(STOP_FILE)).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0)
}

/// A restart or a cancel asked from outside the watcher.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Request {
    /// "restart" or "cancel".
    pub kind: String,
    pub warn_minutes: u32,
    /// "app" or "assistant".
    pub by: String,
    pub at: u64,
}

pub fn request(z: &Path, r: &Request) -> Result<(), String> {
    write_json(&z.join(REQUEST_FILE), r)
}

fn take_request(z: &Path) -> Option<Request> {
    let path = z.join(REQUEST_FILE);
    let r: Option<Request> = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok());
    let _ = std::fs::remove_file(&path);
    // A request left from long ago (the app was closed) is not acted on.
    r.filter(|r| now_secs().saturating_sub(r.at) < 120)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Whether the watcher (the app) is running: its heartbeat is recent.
pub fn watcher_alive(st: &RestartState) -> bool {
    now_secs().saturating_sub(st.heartbeat) <= HEARTBEAT_SECS * 3
}

// ------------------------------------------------------------------ what the screen shows

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartView {
    pub server: String,
    pub schedule: Schedule,
    /// Unix seconds of the next scheduled restart (while the server runs).
    pub next_restart: Option<u64>,
    pub pending: Option<Pending>,
    pub history: Vec<RestartEvent>,
    pub gave_up_at: Option<u64>,
    /// The app is open and following the server (restarts happen only then).
    pub app_watching: bool,
    pub restarting_since: Option<u64>,
    pub running: bool,
}

pub fn view(z: &Path, server: &str) -> RestartView {
    let st = state(z);
    let schedule = schedule_of(z, server);
    let status = super::get_dedicated_server_status(z.to_string_lossy().to_string()).ok();
    let running = status.as_ref().is_some_and(|s| s.is_running && s.server_name.as_deref() == Some(server));
    let started = status.as_ref().and_then(|s| s.start_timestamp).map(to_local);
    let next_restart = if running { next_scheduled(&schedule, Local::now().naive_local(), started).map(to_unix) } else { None };
    RestartView {
        server: server.to_string(),
        next_restart,
        pending: st.pending.clone().filter(|p| p.server == server),
        history: st.history.iter().filter(|e| e.server == server).cloned().collect(),
        gave_up_at: st.gave_up_at.filter(|_| st.watching.as_ref().is_none_or(|w| w.server == server)),
        app_watching: watcher_alive(&st),
        restarting_since: st.restarting_since,
        running,
        schedule,
    }
}

// ------------------------------------------------------------------ doing it

fn zomboid_dir() -> Option<PathBuf> {
    let chosen = crate::vfs::saved_user_zomboid_dir();
    crate::load_order::mod_info::get_existing_user_zomboid_dirs(&chosen).into_iter().next()
}

fn say(z: &Path, server: &str, text: &str) -> Result<(), String> {
    let clean = crate::discord_commands::servermsg_text(text).ok_or("empty message")?;
    super::rcon_command(z, server, &format!("servermsg \"{}\"", clean))
}

/// Whether the log of this run says the server came up.
fn came_up(z: &Path, since: u64) -> bool {
    let path = z.join("server-console.txt");
    let fresh = path
        .metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .is_some_and(|t| t.as_secs() + 5 >= since);
    fresh && std::fs::read(&path).map(|b| String::from_utf8_lossy(&b).contains("*** SERVER STARTED")).unwrap_or(false)
}

/// An admin's `quit` in the game (or the server console) since `since`: the game logs it as
/// "<user> closed server" in its admin log.
fn quit_by_admin(z: &Path, since: u64) -> bool {
    let Ok(rd) = std::fs::read_dir(z.join("Logs")) else { return false };
    rd.flatten().any(|e| {
        let name = e.file_name().to_string_lossy().to_lowercase();
        let recent = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).is_some_and(|t| t.as_secs() + 5 >= since);
        name.contains("admin") && name.ends_with(".txt") && recent && std::fs::read_to_string(e.path()).map(|t| t.lines().rev().take(20).any(|l| l.contains("closed server"))).unwrap_or(false)
    })
}

/// Stops the server the clean way (`save`, then `quit` over RCON, killed only if it does not go)
/// and starts it again as it was started. Returns what to say about it.
pub fn restart_now(z: &Path, w: &Watched, reason: &str, lang: &str) -> Result<String, String> {
    let z_str = z.to_string_lossy().to_string();
    let mut notes = Vec::new();
    if reason != "crash" {
        let _ = say(z, &w.server, &game_message(lang, "now", 0, reason));
        match super::rcon_command(z, &w.server, "save") {
            // `quit` saves too; the save first means less is lost if the quit goes wrong.
            Ok(()) => std::thread::sleep(std::time::Duration::from_secs(8)),
            Err(e) => notes.push(format!("save over RCON failed: {}", e)),
        }
        super::stop_dedicated_server(z_str.clone(), None)?;
        // Its ports are let go a moment after the process ends.
        std::thread::sleep(std::time::Duration::from_secs(4));
    }
    let password = crate::secrets::get(&super::admin_password_secret(&w.server)).ok().flatten();
    let pid = super::launch_dedicated_server(String::new(), z_str, w.server.clone(), w.memory_gb, w.nosteam, password)?;
    notes.insert(0, format!("started again (PID {})", pid));
    Ok(notes.join("; "))
}

static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Follows the Studio's server while the app runs: warnings, scheduled, mod-update and asked
/// restarts, and starts after a crash.
pub fn watch() {
    if STARTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(|| {
        let mut first = true;
        let mut st = RestartState::default();
        let mut last_saved = RestartState::default();
        let mut last_heartbeat = 0u64;
        loop {
            if let Some(z) = zomboid_dir() {
                if first {
                    st = state(&z);
                    first = false;
                    on_app_start(&z, &mut st);
                }
                round(&z, &mut st);
                let now = now_secs();
                // The heartbeat only matters while a server runs (an assistant asks for a restart).
                let beat = st.watching.is_some() && now.saturating_sub(last_heartbeat) >= HEARTBEAT_SECS;
                if st != last_saved || beat {
                    st.heartbeat = now;
                    last_heartbeat = now;
                    save_state(&z, &st);
                    last_saved = st.clone();
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(EVERY_SECS));
        }
    });
}

/// The app was closed: a server that stopped meanwhile is said, not started again (the PC may
/// have been off for days); a restart counting down then is dropped.
fn on_app_start(z: &Path, st: &mut RestartState) {
    st.restarting_since = None;
    let running = super::get_dedicated_server_status(z.to_string_lossy().to_string()).is_ok_and(|s| s.is_running);
    if let Some(w) = st.watching.clone() {
        if !running {
            let asked = stop_asked_at(z) >= w.started_at;
            if !asked {
                st.record(RestartEvent { at: now_secs(), server: w.server.clone(), reason: "stopped_while_closed".into(), ok: false, detail: String::new() });
            }
            st.watching = None;
        }
    }
    if st.pending.as_ref().is_some_and(|p| p.at <= now_secs()) {
        st.pending = None;
    }
}

fn round(z: &Path, st: &mut RestartState) {
    let now = now_secs();
    let Ok(status) = super::get_dedicated_server_status(z.to_string_lossy().to_string()) else { return };
    let request = take_request(z);

    if !status.is_running {
        if let Some(p) = st.pending.take() {
            st.record(RestartEvent { at: now, server: p.server, reason: p.reason, ok: false, detail: "the server stopped before the restart".into() });
        }
        if let Some(w) = st.watching.take() {
            went_down(z, st, w, now);
        }
        return;
    }

    let server = status.server_name.clone().unwrap_or_default();
    let started_at = status.start_timestamp.unwrap_or(now);
    let schedule = schedule_of(z, &server);
    let same_run = st.watching.as_ref().is_some_and(|w| w.server == server && w.started_at == started_at);
    if !same_run {
        // A new run. Started by hand (the window, an assistant): the crash count and a give-up
        // are over; after the watcher's own restart they go on.
        if !st.expect_own_run {
            st.gave_up_at = None;
            st.crashes.clear();
        }
        st.expect_own_run = false;
        st.watching = Some(Watched { server: server.clone(), pid: status.pid, memory_gb: status.memory_gb, nosteam: status.nosteam, started_at, started_ok: false });
    }
    if let Some(w) = st.watching.as_mut() {
        if !w.started_ok && came_up(z, w.started_at) {
            w.started_ok = true;
        }
    }

    match request {
        Some(r) if r.kind == "cancel" => {
            if let Some(p) = st.pending.take() {
                if p.reason == "schedule" {
                    st.skipped_at = p.at;
                }
                let _ = say(z, &p.server, &game_message(&schedule.language, "cancel", 0, ""));
                st.record(RestartEvent { at: now, server: p.server, reason: p.reason, ok: false, detail: "called off".into() });
            }
        }
        Some(r) if r.kind == "restart" => {
            let warn = r.warn_minutes.min(120);
            st.pending = Some(Pending::new(&server, now, now + warn as u64 * 60, "manual", &r.by, &schedule.warn_minutes));
        }
        _ => {}
    }

    if st.pending.is_none() {
        if let Some(next) = next_scheduled(&schedule, to_local(now), Some(to_local(started_at))).map(to_unix) {
            if next != st.skipped_at && next.saturating_sub(now) <= schedule.longest_warning() as u64 * 60 + EVERY_SECS {
                st.pending = Some(Pending::new(&server, now, next, "schedule", "", &schedule.warn_minutes));
            }
        }
    }
    if st.pending.is_none() && schedule.on_mod_update && now.saturating_sub(st.mods_checked_at) >= schedule.mod_check_minutes as u64 * 60 {
        st.mods_checked_at = now;
        let updated = updated_mods(z, &server, started_at, &mut st.mods_seen);
        if !updated.is_empty() {
            let at = now + schedule.longest_warning() as u64 * 60;
            st.pending = Some(Pending::new(&server, now, at, "mod_update", &updated.join(", "), &schedule.warn_minutes));
        }
    }

    let Some(p) = st.pending.as_mut() else { return };
    if p.server != server {
        st.pending = None;
        return;
    }
    let warn = schedule.warn_minutes.clone();
    match p.step(now, &warn) {
        Some(due) => {
            for m in due {
                if let Err(e) = say(z, &server, &game_message(&schedule.language, "warn", m, &p.reason)) {
                    crate::diagnostics::error(format!("[Server restarts] Could not warn the players of {}: {}", server, e));
                }
            }
        }
        None => {
            let p = st.pending.take().unwrap_or_default();
            let Some(w) = st.watching.clone() else { return };
            st.restarting_since = Some(now);
            save_state(z, st);
            let result = restart_now(z, &w, &p.reason, &schedule.language);
            st.restarting_since = None;
            // The next look sees the new run.
            st.watching = None;
            st.expect_own_run = result.is_ok();
            let detail = match (&result, p.detail.is_empty()) {
                (Ok(d), true) => d.clone(),
                (Ok(d), false) => format!("{} ({})", p.detail, d),
                (Err(e), _) => e.clone(),
            };
            st.record(RestartEvent { at: now_secs(), server: w.server, reason: p.reason, ok: result.is_ok(), detail });
        }
    }
}

/// The server is down and was not asked to stop: started again, or why not.
fn went_down(z: &Path, st: &mut RestartState, w: Watched, now: u64) {
    if stop_asked_at(z) >= w.started_at.saturating_sub(2) || quit_by_admin(z, w.started_at) {
        return;
    }
    let schedule = schedule_of(z, &w.server);
    let no = |st: &mut RestartState, why: &str| {
        st.record(RestartEvent { at: now, server: w.server.clone(), reason: "crash_no_restart".into(), ok: false, detail: why.into() });
    };
    if !schedule.on_crash {
        return no(st, "restarting after a crash is off");
    }
    if !w.started_ok {
        return no(st, "it stopped before it had started (see server-console.txt): starting it again would stop the same way");
    }
    if super::mp_test::active(z) {
        return no(st, "a multiplayer test is running");
    }
    if st.gave_up_at.is_some() {
        return;
    }
    if !crash_decision(&mut st.crashes, now, schedule.crash_limit, schedule.crash_window_minutes) {
        st.gave_up_at = Some(now);
        st.record(RestartEvent {
            at: now,
            server: w.server.clone(),
            reason: "gave_up".into(),
            ok: false,
            detail: format!("stopped by itself {} times in {} minutes: left off", schedule.crash_limit + 1, schedule.crash_window_minutes),
        });
        crate::diagnostics::error(format!("[Server restarts] {} keeps stopping by itself: not started again.", w.server));
        return;
    }
    // A moment for the ports and the files.
    std::thread::sleep(std::time::Duration::from_secs(10));
    st.restarting_since = Some(now);
    let result = restart_now(z, &w, "crash", &schedule.language);
    st.restarting_since = None;
    st.expect_own_run = result.is_ok();
    st.record(RestartEvent { at: now_secs(), server: w.server, reason: "crash".into(), ok: result.is_ok(), detail: result.unwrap_or_else(|e| e) });
}

/// The titles of the server's Workshop items published after it started and not acted on yet.
fn updated_mods(z: &Path, server: &str, started_at: u64, seen: &mut BTreeSet<String>) -> Vec<String> {
    let ids = std::fs::read_to_string(z.join("Server").join(format!("{}.ini", server))).map(|t| super::mod_sync::ini_list(&t, "WorkshopItems")).unwrap_or_default();
    if ids.is_empty() {
        return Vec::new();
    }
    match crate::workshop::get_details(&ids) {
        Ok(items) => updated_since(&items, started_at, seen),
        Err(e) => {
            crate::diagnostics::error(format!("[Server restarts] Could not ask Steam about {}'s Workshop items: {}", server, e));
            Vec::new()
        }
    }
}

pub fn updated_since(items: &[crate::workshop::WorkshopItem], started_at: u64, seen: &mut BTreeSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if let Some(t) = item.time_updated.filter(|t| *t > started_at) {
            if seen.insert(format!("{}@{}", item.workshop_id, t)) {
                out.push(if item.title.is_empty() { item.workshop_id.clone() } else { item.title.clone() });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    #[test]
    fn the_next_restart_is_the_nearest_time_or_interval() {
        let mut s = Schedule { enabled: true, times: vec!["06:00".into(), "18:30".into()], ..Default::default() };
        let start = at("2026-11-18 01:00");
        assert_eq!(next_scheduled(&s, at("2026-11-18 05:00"), Some(start)), Some(at("2026-11-18 06:00")));
        assert_eq!(next_scheduled(&s, at("2026-11-18 06:00"), Some(start)), Some(at("2026-11-18 18:30")), "the same minute is past");
        assert_eq!(next_scheduled(&s, at("2026-11-18 20:00"), Some(start)), Some(at("2026-11-19 06:00")), "tomorrow");
        // Started at 05:45: 06:00 is too soon after, the next one is kept.
        assert_eq!(next_scheduled(&s, at("2026-11-18 05:50"), Some(at("2026-11-18 05:45"))), Some(at("2026-11-18 18:30")));
        s.every_hours = 4;
        assert_eq!(next_scheduled(&s, at("2026-11-18 02:00"), Some(start)), Some(at("2026-11-18 05:00")), "every 4 h from the start beats 06:00");
        s.times.clear();
        assert_eq!(next_scheduled(&s, at("2026-11-18 09:10"), Some(start)), Some(at("2026-11-18 13:00")));
        assert_eq!(next_scheduled(&s, at("2026-11-18 09:10"), None), None, "no start, no interval");
        s.enabled = false;
        assert_eq!(next_scheduled(&s, at("2026-11-18 09:10"), Some(start)), None);
    }

    #[test]
    fn a_schedule_is_kept_within_its_limits() {
        let s = Schedule { times: vec!["6:00".into(), "25:00".into(), "06:00".into(), "x".into(), "23:59".into()], warn_minutes: vec![1, 30, 0, 500, 5, 30], crash_limit: 0, mod_check_minutes: 1, language: "fr".into(), ..Default::default() }.cleaned();
        assert_eq!(s.times, ["06:00", "23:59"]);
        assert_eq!(s.warn_minutes, [30, 5, 1]);
        assert_eq!((s.crash_limit, s.mod_check_minutes, s.language.as_str()), (1, 5, "en"));
    }

    #[test]
    fn players_are_warned_at_30_15_5_and_1_minutes() {
        let warn = [30, 15, 5, 1];
        let t0 = 10_000;
        let mut p = Pending::new("srv", t0, t0 + 30 * 60, "schedule", "", &warn);
        let mut said = Vec::new();
        let mut now = t0;
        while let Some(due) = p.step(now, &warn) {
            said.extend(due.into_iter().map(|m| (m, (p.at - now) / 60)));
            now += 5;
        }
        assert_eq!(said.iter().map(|(m, _)| *m).collect::<Vec<_>>(), [30, 15, 5, 1]);
        assert!(now >= p.at);

        // Asked for in 5 minutes: 30 and 15 would lie, 5 is said at once.
        let mut q = Pending::new("srv", t0, t0 + 300, "manual", "app", &warn);
        assert_eq!(q.step(t0, &warn), Some(vec![5]));
        assert_eq!(q.step(t0 + 5, &warn), Some(vec![]));
        assert_eq!(q.step(t0 + 241, &warn), Some(vec![1]));
        assert_eq!(q.step(t0 + 300, &warn), None);

        // 3 minutes: no warning fits, the time left is said first.
        let mut r = Pending::new("srv", t0, t0 + 180, "manual", "", &warn);
        assert_eq!(r.step(t0, &warn), Some(vec![3]));
        // Restart right away.
        let mut s = Pending::new("srv", t0, t0, "manual", "", &warn);
        assert_eq!(s.step(t0, &warn), None);
        // The app was busy: 15 and 5 both due, only 5 said.
        let mut u = Pending::new("srv", t0, t0 + 20 * 60, "schedule", "", &warn);
        assert_eq!(u.step(t0, &warn), Some(vec![20]));
        assert_eq!(u.step(t0 + 16 * 60, &warn), Some(vec![5]));
    }

    #[test]
    fn crashes_restart_until_the_limit_then_it_stays_off() {
        let mut c = Vec::new();
        assert!(crash_decision(&mut c, 1000, 3, 10));
        assert!(crash_decision(&mut c, 1100, 3, 10));
        assert!(crash_decision(&mut c, 1200, 3, 10));
        assert!(!crash_decision(&mut c, 1300, 3, 10), "a fourth in ten minutes");
        assert!(crash_decision(&mut c, 1000 + 601, 3, 10), "the first one is out of the window");
    }

    #[test]
    fn messages_in_the_game_are_one_line_in_the_players_language() {
        assert_eq!(game_message("en", "warn", 30, "schedule"), "Server restart in 30 minutes (scheduled).");
        assert_eq!(game_message("es", "warn", 1, "mod_update"), "Reinicio del servidor en 1 minuto (se actualizó un mod). Buscá un lugar seguro.");
        for (what, m) in [("warn", 5), ("now", 0), ("cancel", 0)] {
            for lang in ["en", "es"] {
                let text = game_message(lang, what, m, "manual");
                assert!(!text.is_empty() && crate::discord_commands::servermsg_text(&text).as_deref() == Some(text.as_str()), "{}", text);
            }
        }
    }

    #[test]
    fn only_updates_after_the_start_count_and_each_once() {
        let item = |id: &str, t: u64| crate::workshop::WorkshopItem {
            workshop_id: id.into(), found: true, title: format!("Mod {}", id), summary: String::new(), preview_url: None,
            time_updated: Some(t), time_created: None, subscriptions: 0, favorited: 0, file_size: 0, tags: vec![], builds: vec![],
            is_project_zomboid: true, banned: false, page_url: String::new(), children: vec![], is_collection: false, support: vec![], creator: None,
        };
        let mut seen = BTreeSet::new();
        let items = [item("1", 900), item("2", 1100)];
        assert_eq!(updated_since(&items, 1000, &mut seen), ["Mod 2"]);
        assert!(updated_since(&items, 1000, &mut seen).is_empty(), "said once");
        assert_eq!(updated_since(&[item("2", 1200)], 1000, &mut seen), ["Mod 2"], "a newer update of it");
    }

    #[test]
    fn schedules_and_requests_round_trip_through_their_files() {
        let z = std::env::temp_dir().join(format!("pzms-restarts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&z);
        std::fs::create_dir_all(&z).unwrap();
        assert_eq!(schedule_of(&z, "srv"), Schedule::default());
        let saved = save_schedule(&z, "srv", Schedule { enabled: true, times: vec!["5:30".into()], ..Default::default() }).unwrap();
        assert_eq!(saved.times, ["05:30"]);
        assert_eq!(schedule_of(&z, "srv"), saved);
        assert_eq!(schedule_of(&z, "other"), Schedule::default());
        request(&z, &Request { kind: "restart".into(), warn_minutes: 5, by: "assistant".into(), at: now_secs() }).unwrap();
        assert_eq!(take_request(&z).map(|r| r.warn_minutes), Some(5));
        assert!(take_request(&z).is_none(), "taken once");
        request(&z, &Request { kind: "restart".into(), warn_minutes: 5, by: "assistant".into(), at: 1 }).unwrap();
        assert!(take_request(&z).is_none(), "an old request is not acted on");
        note_stop_asked(&z);
        assert!(stop_asked_at(&z) + 5 >= now_secs());
        let _ = std::fs::remove_dir_all(&z);
    }
}

// ------------------------------------------------------------------ the window's calls

#[tauri::command(async)]
pub fn server_restarts_view(paths: crate::vfs::StudioPaths, server: String) -> RestartView {
    view(&super::primary_dir(&paths.user_zomboid_dir), &server)
}

#[tauri::command(async)]
pub fn server_schedule_save(paths: crate::vfs::StudioPaths, server: String, schedule: Schedule) -> Result<Schedule, String> {
    save_schedule(&super::primary_dir(&paths.user_zomboid_dir), &server, schedule)
}

/// "Restart now (warn players)": the watcher counts down `warn_minutes`, then restarts.
#[tauri::command(async)]
pub fn server_restart_request(paths: crate::vfs::StudioPaths, warn_minutes: u32, cancel: Option<bool>) -> Result<(), String> {
    let z = super::primary_dir(&paths.user_zomboid_dir);
    let kind = if cancel.unwrap_or(false) { "cancel" } else { "restart" };
    request(&z, &Request { kind: kind.into(), warn_minutes: warn_minutes.min(120), by: "app".into(), at: now_secs() })
}

/// Against a real server, by hand (it starts, restarts and stops a server):
/// `PZMS_LIVE_ZOMBOID=<Zomboid> PZMS_LIVE_SERVER=<a test server> cargo test --lib live_restart_cycle -- --ignored --nocapture`
#[cfg(test)]
mod live_tests {
    use super::*;

    fn log(m: &str) {
        eprintln!("[{}] {}", Local::now().format("%H:%M:%S"), m);
    }

    /// Runs the watcher's rounds until `done` says so (or the minutes run out).
    fn rounds_until(z: &Path, st: &mut RestartState, minutes: u64, what: &str, done: impl Fn(&RestartState) -> bool) {
        let end = now_secs() + minutes * 60;
        let mut said = String::new();
        while now_secs() < end {
            round(z, st);
            let line = format!("pending={:?} watching={:?} history={}", st.pending.as_ref().map(|p| (p.reason.clone(), p.at.saturating_sub(now_secs()), p.warned.clone())), st.watching.as_ref().map(|w| (w.pid, w.started_ok)), st.history.len());
            if line != said {
                log(&line);
                said = line;
            }
            if done(st) {
                log(&format!("{}: done", what));
                return;
            }
            std::thread::sleep(std::time::Duration::from_secs(EVERY_SECS));
        }
        panic!("{}: not done in {} minutes; history {:?}", what, minutes, st.history);
    }

    /// Stands in for the open app's watcher (rounds and heartbeat, as `watch` does) so a warned
    /// restart asked from outside (MCP server_restart) runs, until one asked restart is done and
    /// the server is up again. With PZMS_LIVE_DEDICATED_CONFIG (the Studio's dedicated_server.json)
    /// it restarts from the program chosen there.
    /// `PZMS_LIVE_ZOMBOID=<Zomboid> cargo test --lib live_watch_like_the_app -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_watch_like_the_app() {
        let z = PathBuf::from(std::env::var("PZMS_LIVE_ZOMBOID").expect("PZMS_LIVE_ZOMBOID"));
        let minutes: u64 = std::env::var("PZMS_LIVE_WATCH_MINUTES").ok().and_then(|m| m.parse().ok()).unwrap_or(20);
        let mut st = state(&z);
        on_app_start(&z, &mut st);
        let before = st.history.iter().filter(|e| e.reason == "manual").count();
        let end = now_secs() + minutes * 60;
        let mut said = String::new();
        let mut restarted = false;
        while now_secs() < end {
            round(&z, &mut st);
            st.heartbeat = now_secs();
            save_state(&z, &st);
            let line = format!("pending={:?} watching={:?} history={}", st.pending.as_ref().map(|p| (p.reason.clone(), p.at.saturating_sub(now_secs()), p.warned.clone())), st.watching.as_ref().map(|w| (w.server.clone(), w.pid, w.started_ok)), st.history.len());
            if line != said {
                log(&line);
                said = line;
            }
            restarted |= st.history.iter().filter(|e| e.reason == "manual").count() > before;
            if restarted && st.watching.as_ref().is_some_and(|w| w.started_ok) {
                log(&format!("restart done: {:?}", st.history.first()));
                let status = super::super::get_dedicated_server_status(z.to_string_lossy().to_string()).unwrap();
                log(&format!("now running from {:?} (PID {:?})", status.program_dir, status.pid));
                assert!(st.history.first().is_some_and(|e| e.ok));
                return;
            }
            std::thread::sleep(std::time::Duration::from_secs(EVERY_SECS));
        }
        panic!("no asked restart done in {} minutes; history {:?}", minutes, st.history.first());
    }

    #[test]
    #[ignore]
    fn live_restart_cycle() {
        let z = PathBuf::from(std::env::var("PZMS_LIVE_ZOMBOID").expect("PZMS_LIVE_ZOMBOID"));
        let server = std::env::var("PZMS_LIVE_SERVER").expect("PZMS_LIVE_SERVER");
        assert!(server.to_lowercase().contains("test"), "only a test server");
        let z_str = z.to_string_lossy().to_string();
        save_schedule(&z, &server, Schedule { warn_minutes: vec![2, 1], language: "es".into(), ..Default::default() }).unwrap();
        let mut st = RestartState::default();
        let up = |st: &RestartState| st.watching.as_ref().is_some_and(|w| w.started_ok);

        // A new world asks for the admin password at its first start (with none it stops there).
        log("start");
        super::super::launch_dedicated_server(String::new(), z_str.clone(), server.clone(), Some(3), Some(true), Some(format!("t{}", now_secs()))).unwrap();
        rounds_until(&z, &mut st, 12, "first start", up);
        log(&format!("RCON servermsg: {:?}", super::super::rcon_query(&z, &server, "servermsg \"Prueba de PZ Mod Studio\"")));

        log("1. asked for, 2 minutes of warning");
        request(&z, &Request { kind: "restart".into(), warn_minutes: 2, by: "live test".into(), at: now_secs() }).unwrap();
        rounds_until(&z, &mut st, 8, "asked restart", |s| s.history.iter().any(|e| e.reason == "manual"));
        log(&format!("{:?}", st.history[0]));
        assert!(st.history[0].ok);
        rounds_until(&z, &mut st, 12, "up after the asked restart", up);

        log("2. a scheduled one (its countdown made as the schedule makes it, 2 minutes ahead)");
        let w = st.watching.clone().unwrap();
        st.pending = Some(Pending::new(&w.server, now_secs(), now_secs() + 120, "schedule", "", &[2, 1]));
        rounds_until(&z, &mut st, 8, "scheduled restart", |s| s.history.first().is_some_and(|e| e.reason == "schedule"));
        log(&format!("{:?}", st.history[0]));
        assert!(st.history[0].ok);
        rounds_until(&z, &mut st, 12, "up after the scheduled restart", up);

        log("3. a crash: java.exe killed");
        let pid = st.watching.as_ref().and_then(|w| w.pid).unwrap();
        let out = std::process::Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).output().unwrap();
        log(&String::from_utf8_lossy(&out.stdout));
        rounds_until(&z, &mut st, 3, "crash restart", |s| s.history.first().is_some_and(|e| e.reason == "crash"));
        log(&format!("{:?}", st.history[0]));
        assert!(st.history[0].ok);
        rounds_until(&z, &mut st, 12, "up after the crash restart", up);

        log("4. a stop asked for is not a crash");
        let before = st.history.len();
        super::super::stop_dedicated_server(z_str.clone(), None).unwrap();
        for _ in 0..4 {
            round(&z, &mut st);
            std::thread::sleep(std::time::Duration::from_secs(EVERY_SECS));
        }
        assert_eq!(st.history.len(), before, "{:?}", st.history.first());
        assert!(st.watching.is_none());
        log(&format!("history: {:#?}", st.history));
    }
}
