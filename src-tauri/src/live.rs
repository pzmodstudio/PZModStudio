//! The game as it runs, seen as it happens: frames, memory, CPU, the mods whose Lua takes the
//! most time now and the session's Lua errors. The Monitor's Live panel and the MCP tool
//! `game_live` show it; the Studio Link's overlay shows the same in the game.
//!
//! Where each number comes from (files in `<Zomboid>/Lua`, the only folder the game's Lua can use):
//!   PZModStudio_LinkLive.txt   game -> app  the Link, every 2 s in a game while the app runs:
//!                                           fps, frame and longest frame, Java heap used, the
//!                                           session's errors by the mod blamed, the overlay's own
//!                                           cost (see mods/Z_StudioLink/.../PZModStudio_Live.lua)
//!   pz_profile_live.json       game -> app  the profiler, every 2 s when it is on: each mod's Lua
//!                                           time over the last 10 s (PZModStudio_Profiler.lua)
//!   PZModStudio_LinkMeter.txt  app -> game  what Windows says of the game's process: memory, the
//!                                           Java limit (-Xmx), CPU; for the overlay
//! The process itself (memory, CPU) is read natively (crate::resources). The app's watcher calls
//! `tick` every 2 s and keeps the last 10 minutes as points, also written to
//! `PZModStudio_Cache/live.json` so the MCP server (another process) can give them too.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const LINK_LIVE_FILE: &str = "PZModStudio_LinkLive.txt";
pub const METER_FILE: &str = "PZModStudio_LinkMeter.txt";
pub const PROFILER_LIVE_FILE: &str = "pz_profile_live.json";
const HISTORY_FILE: &str = "live.json";
/// Ten minutes of points, one every 2 s.
const HISTORY_POINTS: usize = 300;
/// A file older than this is from a game that is no longer writing it.
const FRESH_MS: u64 = 8_000;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ErrorMod {
    /// The mod the game named on the error's stack; empty for the game's own code.
    pub name: String,
    pub count: u64,
}

/// What the Link said last.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct LinkLive {
    pub age_ms: u64,
    pub link_version: String,
    /// Frames per second and the average frame, over the last 2 s.
    pub fps: f64,
    pub frame_ms: f64,
    /// The longest frame of the last 10 s.
    pub worst_ms: u64,
    /// Java heap used now (the game's own count) and reserved, in MB.
    pub heap_mb: Option<u64>,
    pub heap_total_mb: Option<u64>,
    /// Lua errors of this session, and by the mod blamed (the most first).
    pub errors: u64,
    pub error_mods: Vec<ErrorMod>,
    /// The overlay shows now.
    pub overlay: bool,
    /// The Link's own live code (frame meter, overlay, this file) over the last 10 s, per second.
    pub cost_ms_per_s: f64,
    /// The profiler runs in this game.
    pub profiler: bool,
}

pub fn parse_link_live(text: &str, now_ms: u64) -> Option<LinkLive> {
    let mut out = LinkLive::default();
    let mut at = None;
    let (mut cost, mut window) = (0f64, 0f64);
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim_end_matches('\r');
        let num = || value.trim().parse::<f64>().ok();
        match key {
            "at" => at = value.trim().parse::<u64>().ok(),
            "link" => out.link_version = value.trim().to_string(),
            "fps" => out.fps = num().unwrap_or(0.0),
            "frame_ms" => out.frame_ms = num().unwrap_or(0.0),
            "worst_ms" => out.worst_ms = num().unwrap_or(0.0) as u64,
            "heap_mb" => out.heap_mb = num().map(|v| v as u64),
            "heap_total_mb" => out.heap_total_mb = num().map(|v| v as u64),
            "errors" => out.errors = num().unwrap_or(0.0) as u64,
            "err" => {
                if let Some((count, name)) = value.split_once('\t') {
                    out.error_mods.push(ErrorMod { name: name.trim().to_string(), count: count.trim().parse().unwrap_or(0) });
                }
            }
            "overlay" => out.overlay = value.trim() == "1",
            "cost_ms" => cost = num().unwrap_or(0.0),
            "window_ms" => window = num().unwrap_or(0.0),
            "profiler" => out.profiler = value.trim() == "1",
            _ => {}
        }
    }
    let at = at?;
    out.age_ms = now_ms.saturating_sub(at);
    out.cost_ms_per_s = if window > 0.0 { cost * 1000.0 / window } else { 0.0 };
    Some(out)
}

#[derive(Debug, Clone, Deserialize, Default)]
struct RawFrames {
    count: u64,
    sum: u64,
    max: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct RawMod {
    id: String,
    name: String,
    ms: u64,
    calls: u64,
    worst: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct RawLive {
    now: u64,
    #[serde(default)]
    in_game: bool,
    window_ms: u64,
    #[serde(default)]
    frames: RawFrames,
    #[serde(default)]
    total_ms: u64,
    #[serde(default)]
    cost_ms: u64,
    #[serde(default)]
    handlers: u64,
    #[serde(default)]
    mods: Vec<RawMod>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct LiveMod {
    /// None for the game's own Lua (a handler in one of its files).
    pub mod_id: Option<String>,
    pub name: String,
    /// Lua time over the window, and per second of it.
    pub ms: u64,
    pub ms_per_s: f64,
    /// Share of the window's real time.
    pub share_percent: f64,
    pub calls: u64,
    /// The longest single call in the window.
    pub worst: u64,
}

/// The profiler's last 10 s.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ProfilerLive {
    pub age_ms: u64,
    pub in_game: bool,
    pub window_ms: u64,
    pub fps: f64,
    pub frame_ms: f64,
    pub worst_ms: u64,
    /// Every mod's Lua together, over the window and per second.
    pub total_ms: u64,
    pub total_ms_per_s: f64,
    pub share_percent: f64,
    /// The profiler's own sampling, per second.
    pub cost_ms_per_s: f64,
    /// Handlers it clocks.
    pub handlers: u64,
    pub mods: Vec<LiveMod>,
}

pub fn parse_profiler_live(text: &str, now_ms: u64) -> Option<ProfilerLive> {
    let raw: RawLive = serde_json::from_str(text).ok()?;
    let window = raw.window_ms;
    let per_s = |ms: u64| if window > 0 { ms as f64 * 1000.0 / window as f64 } else { 0.0 };
    let share = |ms: u64| if window > 0 { ms as f64 * 100.0 / window as f64 } else { 0.0 };
    Some(ProfilerLive {
        age_ms: now_ms.saturating_sub(raw.now),
        in_game: raw.in_game,
        window_ms: window,
        fps: if raw.frames.sum > 0 { raw.frames.count as f64 * 1000.0 / raw.frames.sum as f64 } else { 0.0 },
        frame_ms: if raw.frames.count > 0 { raw.frames.sum as f64 / raw.frames.count as f64 } else { 0.0 },
        worst_ms: raw.frames.max,
        total_ms: raw.total_ms,
        total_ms_per_s: per_s(raw.total_ms),
        share_percent: share(raw.total_ms),
        cost_ms_per_s: per_s(raw.cost_ms),
        handlers: raw.handlers,
        mods: raw
            .mods
            .into_iter()
            .map(|m| LiveMod {
                mod_id: (!m.id.is_empty()).then_some(m.id),
                name: m.name,
                ms: m.ms,
                ms_per_s: per_s(m.ms),
                share_percent: share(m.ms),
                calls: m.calls,
                worst: m.worst,
            })
            .collect(),
    })
}

/// One moment of the last minutes.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct LivePoint {
    /// Unix ms.
    pub t: u64,
    pub fps: Option<f64>,
    pub worst_ms: Option<u64>,
    /// The process's memory (Windows) and the Java heap used (the game's count), MB.
    pub ram_mb: Option<u64>,
    pub heap_mb: Option<u64>,
    pub cpu: Option<f64>,
    /// Every mod's Lua per second, when the profiler is on.
    pub lua_ms_per_s: Option<f64>,
    pub errors: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LiveSnapshot {
    /// The game's process is open.
    pub running: bool,
    pub pid: Option<u32>,
    /// The process's memory as Windows counts it, the Java limit (-Xmx), CPU share of the PC.
    pub ram_mb: Option<u64>,
    pub ram_max_mb: Option<u64>,
    pub cpu_percent: Option<f64>,
    pub pc_free_mb: u64,
    pub pc_total_mb: u64,
    /// What the Link says; None when it wrote nothing lately (not in a game, an older Link).
    pub link: Option<LinkLive>,
    /// The profiler is in the active list: it runs from the next start.
    pub profiler_on: bool,
    /// What the profiler says; None when it is not running in this game.
    pub profiler: Option<ProfilerLive>,
    /// The last minutes, oldest first.
    pub history: Vec<LivePoint>,
}

static HISTORY: Mutex<VecDeque<LivePoint>> = Mutex::new(VecDeque::new());
/// The last reading the watcher made, for the window's asks in between.
static LAST: Mutex<Option<(Instant, LiveSnapshot)>> = Mutex::new(None);
static WAS_RUNNING: Mutex<bool> = Mutex::new(false);

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn zomboid(paths: &crate::vfs::StudioPaths) -> Option<PathBuf> {
    crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next()
}

/// Reads everything now (no history).
fn read_now(paths: &crate::vfs::StudioPaths) -> LiveSnapshot {
    let res = crate::resources::read(paths);
    let game = res.processes.iter().find(|p| p.kind == "game");
    let mut snap = LiveSnapshot {
        running: game.is_some(),
        pid: game.map(|p| p.pid),
        ram_mb: game.map(|p| p.ram_mb),
        ram_max_mb: game.and_then(|p| p.ram_max_mb),
        cpu_percent: game.and_then(|p| p.cpu_percent),
        pc_free_mb: res.pc_free_mb,
        pc_total_mb: res.pc_total_mb,
        ..Default::default()
    };
    let Some(z) = zomboid(paths) else { return snap };
    snap.profiler_on = crate::sandbox::profiler::is_on(&z);
    if !snap.running {
        return snap;
    }
    let lua = z.join("Lua");
    let now = now_ms();
    snap.link = std::fs::read_to_string(lua.join(LINK_LIVE_FILE)).ok().and_then(|t| parse_link_live(&t, now)).filter(|l| l.age_ms <= FRESH_MS);
    snap.profiler = std::fs::read_to_string(lua.join(PROFILER_LIVE_FILE)).ok().and_then(|t| parse_profiler_live(&t, now)).filter(|p| p.age_ms <= FRESH_MS);
    snap
}

/// The point a reading makes.
fn point_of(snap: &LiveSnapshot, t: u64) -> LivePoint {
    let link = snap.link.as_ref();
    let prof = snap.profiler.as_ref().filter(|p| p.in_game);
    LivePoint {
        t,
        fps: link.map(|l| l.fps).filter(|f| *f > 0.0).or_else(|| prof.map(|p| p.fps).filter(|f| *f > 0.0)),
        worst_ms: link.map(|l| l.worst_ms).or_else(|| prof.map(|p| p.worst_ms)),
        ram_mb: snap.ram_mb,
        heap_mb: link.and_then(|l| l.heap_mb),
        cpu: snap.cpu_percent,
        lua_ms_per_s: prof.map(|p| p.total_ms_per_s),
        errors: link.map(|l| l.errors),
    }
}

/// The meter the overlay reads: the process's memory, the Java limit, the CPU.
pub fn meter_text(snap: &LiveSnapshot, now_secs: u64) -> String {
    let mut out = format!("at={}\n", now_secs);
    if let Some(v) = snap.ram_mb {
        out.push_str(&format!("ram_mb={}\n", v));
    }
    if let Some(v) = snap.ram_max_mb {
        out.push_str(&format!("ram_max_mb={}\n", v));
    }
    if let Some(v) = snap.cpu_percent {
        out.push_str(&format!("cpu={:.1}\n", v));
    }
    out
}

fn history_path(z: &Path) -> PathBuf {
    z.join("PZModStudio_Cache").join(HISTORY_FILE)
}

/// The watcher's beat (every 2 s): reads the game, tells the overlay the process's numbers, keeps
/// the point. A game that starts begins a new history. Returns whether the game runs.
pub fn tick(paths: &crate::vfs::StudioPaths) -> bool {
    let snap = read_now(paths);
    let t = now_ms();
    {
        let mut was = WAS_RUNNING.lock().unwrap_or_else(|e| e.into_inner());
        let mut history = HISTORY.lock().unwrap_or_else(|e| e.into_inner());
        if snap.running && !*was {
            history.clear();
        }
        *was = snap.running;
        if snap.running {
            history.push_back(point_of(&snap, t));
            while history.len() > HISTORY_POINTS {
                history.pop_front();
            }
            if let Some(z) = zomboid(paths) {
                let lua = z.join("Lua");
                if lua.is_dir() {
                    let _ = crate::load_order::ini_parser::atomic_write(lua.join(METER_FILE), meter_text(&snap, t / 1000));
                }
                // For the MCP server, every 10 s.
                if history.len() % 5 == 1 {
                    let path = history_path(&z);
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let points: Vec<&LivePoint> = history.iter().collect();
                    if let Ok(json) = serde_json::to_string(&points) {
                        let _ = crate::load_order::ini_parser::atomic_write(path, json);
                    }
                }
            }
        }
    }
    let running = snap.running;
    *LAST.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), snap));
    running
}

/// Everything at once, with the last minutes. In the app the watcher's last reading is used when
/// it is a moment old; elsewhere (the MCP server) the game is read now and the minutes come from
/// the app's file, while it is recent.
pub fn snapshot(paths: &crate::vfs::StudioPaths) -> LiveSnapshot {
    let recent = LAST.lock().unwrap_or_else(|e| e.into_inner()).as_ref().filter(|(at, _)| at.elapsed().as_millis() < 3_000).map(|(_, s)| s.clone());
    let mut snap = recent.unwrap_or_else(|| read_now(paths));
    let history = HISTORY.lock().unwrap_or_else(|e| e.into_inner());
    if !history.is_empty() {
        snap.history = history.iter().cloned().collect();
    } else if snap.running {
        let points: Vec<LivePoint> = zomboid(paths)
            .and_then(|z| std::fs::read_to_string(history_path(&z)).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        // Only a file the app is still keeping: its last point a moment ago.
        let t = now_ms();
        if points.last().is_some_and(|p: &LivePoint| t.saturating_sub(p.t) < 30_000) {
            snap.history = points;
        } else {
            snap.history = vec![point_of(&snap, t)];
        }
    }
    // A first reading has no CPU yet (it takes two); the app's last point has one.
    if snap.running && snap.cpu_percent.is_none() {
        let t = now_ms();
        snap.cpu_percent = snap.history.iter().rev().find(|p| t.saturating_sub(p.t) < 10_000).and_then(|p| p.cpu);
    }
    snap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_links_file_reads_with_its_errors_by_mod() {
        let text = "at=10000\nlink=1.20.0\nfps=58.5\nframe_ms=17.1\nworst_ms=45\nframes=580\nheap_mb=2100\nheap_total_mb=3000\nerrors=3\nerr=2\tSome Mod\nerr=1\t\noverlay=1\ncost_ms=4\nwindow_ms=10000\nprofiler=1\n";
        let l = parse_link_live(text, 12_500).expect("parsed");
        assert_eq!(l.age_ms, 2_500);
        assert_eq!((l.fps, l.frame_ms, l.worst_ms), (58.5, 17.1, 45));
        assert_eq!((l.heap_mb, l.heap_total_mb), (Some(2100), Some(3000)));
        assert_eq!(l.errors, 3);
        assert_eq!(l.error_mods, vec![ErrorMod { name: "Some Mod".into(), count: 2 }, ErrorMod { name: String::new(), count: 1 }]);
        assert!(l.overlay && l.profiler);
        assert!((l.cost_ms_per_s - 0.4).abs() < 1e-9);
        assert!(parse_link_live("fps=3\n", 1).is_none(), "no time, no reading");
    }

    #[test]
    fn the_profilers_window_becomes_ms_per_second_and_shares() {
        let text = r#"{"version":1,"now":20000,"in_game":true,"window_ms":10000,"frames":{"count":600,"sum":10000,"max":80},"total_ms":250,"cost_ms":3,"handlers":420,"mods":[{"id":"A","name":"Alpha","ms":200,"calls":600,"worst":12},{"id":"","name":"","ms":50,"calls":10,"worst":1}]}"#;
        let p = parse_profiler_live(text, 21_000).expect("parsed");
        assert_eq!(p.age_ms, 1_000);
        assert!((p.fps - 60.0).abs() < 1e-9);
        assert_eq!(p.worst_ms, 80);
        assert!((p.total_ms_per_s - 25.0).abs() < 1e-9);
        assert!((p.share_percent - 2.5).abs() < 1e-9);
        assert_eq!(p.mods[0].mod_id.as_deref(), Some("A"));
        assert!((p.mods[0].ms_per_s - 20.0).abs() < 1e-9);
        assert_eq!(p.mods[1].mod_id, None, "the game's own Lua");
    }

    #[test]
    fn a_point_takes_the_links_frames_first_and_the_meter_says_the_process() {
        let snap = LiveSnapshot {
            running: true,
            ram_mb: Some(9000),
            ram_max_mb: Some(6144),
            cpu_percent: Some(12.34),
            link: Some(LinkLive { fps: 50.0, worst_ms: 40, heap_mb: Some(2000), errors: 2, ..Default::default() }),
            profiler: Some(ProfilerLive { in_game: true, fps: 49.0, total_ms_per_s: 7.5, ..Default::default() }),
            ..Default::default()
        };
        let p = point_of(&snap, 5);
        assert_eq!((p.fps, p.worst_ms, p.heap_mb, p.errors, p.lua_ms_per_s), (Some(50.0), Some(40), Some(2000), Some(2), Some(7.5)));
        assert_eq!(meter_text(&snap, 77), "at=77\nram_mb=9000\nram_max_mb=6144\ncpu=12.3\n");
        let no_link = LiveSnapshot { link: None, ..snap };
        assert_eq!(point_of(&no_link, 5).fps, Some(49.0));
    }
}
