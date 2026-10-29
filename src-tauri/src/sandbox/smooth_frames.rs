//! What Viewpoint's Smooth Frames mod writes to the console while the game runs, as numbers:
//! frame times by the minute, chunk loading, and the Lua handlers (per mod) that cost the most.
//!
//! Lines, as the mod writes them (42.21, Smooth Frames of 2026-10):
//! `[SmoothFrames] last 60s: 3595 frames, 59.9 fps avg, 1% low 44.0 fps | frame time p50 17.0
//! p95 17.0 p99 18.5 max 51.4 ms | over 25ms 6, 50ms 1, 100ms 0 | main-thread work p50 12.0
//! p99 18.0 max 51.2 ms | GC 10 collections 1065 ms`
//! `[SmoothFrames] chunk pacing: 114 chunks finished (avg 5.4 ms each), 27 deferred to a later frame`
//! `[SmoothFrames] Lua event handlers, last 60s: 102.1 ms per second of play across 565 handlers. Top 10:`
//! `[SmoothFrames]    18.43 ms/s  OnPlayerUpdate      1872 calls, worst frame   1.4 ms  Mod: client/x.lua:379 fn`
//! `[SmoothFrames] slow Lua: 14.3 ms in LoadChunk (361 calls, longest 0.7 ms) <- Mod: server/x.lua:652 fn`

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

const TAG: &str = "[SmoothFrames] ";
/// A single frame this long is a hitch the player sees.
pub const HITCH_MS: f64 = 25.0;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Handler {
    pub ms_per_s: f64,
    pub event: String,
    pub calls: u64,
    pub worst_ms: f64,
    /// The mod id as the mod names it; "vanilla" for the game's own Lua.
    pub mod_id: String,
    pub file: String,
    pub line: u32,
    pub func: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SlowCall {
    pub ms: f64,
    pub event: String,
    pub calls: u64,
    pub longest_ms: f64,
    pub mod_id: String,
    pub file: String,
    pub line: u32,
    pub func: String,
}

/// One minute of play as the mod summed it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Window {
    pub index: usize,
    pub frames: u64,
    pub fps_avg: f64,
    pub low1_fps: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max_ms: f64,
    pub over25: u64,
    pub over50: u64,
    pub over100: u64,
    pub gc_count: u64,
    pub gc_ms: u64,
    pub chunks_done: u64,
    pub chunk_avg_ms: f64,
    pub chunks_deferred: u64,
    /// All Lua handlers together, ms per second of play.
    pub lua_ms_per_s: f64,
    pub lua_handlers: u64,
    pub handlers: Vec<Handler>,
    pub slow: Vec<SlowCall>,
}

/// One mod across the windows it appeared in (the mod prints its ten costliest handlers a
/// minute, so a mod below that line is not here).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModFrames {
    pub mod_id: String,
    pub windows: usize,
    pub ms_per_s_avg: f64,
    pub ms_per_s_max: f64,
    pub worst_frame_ms: f64,
    /// Windows in which one of its handlers took a whole frame of 25 ms or more.
    pub hitches: u64,
    /// Its costliest handlers, averaged over the windows they appeared in.
    pub top: Vec<Handler>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SmoothFramesReport {
    pub log_file: String,
    pub minutes: usize,
    pub fps_avg: f64,
    pub low1_min: f64,
    pub over25: u64,
    pub over50: u64,
    pub over100: u64,
    pub gc_ms: u64,
    pub chunks_done: u64,
    pub chunks_deferred: u64,
    pub lua_ms_per_s_avg: f64,
    pub mods: Vec<ModFrames>,
    pub windows: Vec<Window>,
}

/// The number that follows `key` in `s` ("1% low 44.0 fps", "low " -> 44.0).
fn num_after(s: &str, key: &str) -> Option<f64> {
    let at = s.find(key)? + key.len();
    let rest = s[at..].trim_start();
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// The number that comes before `key` ("3595 frames", " frames" -> 3595).
fn num_before(s: &str, key: &str) -> Option<f64> {
    let at = s.find(key)?;
    let head = s[..at].trim_end();
    let start = head.rfind(|c: char| !(c.is_ascii_digit() || c == '.')).map(|i| i + 1).unwrap_or(0);
    head[start..].parse().ok()
}

/// `Mod: client/x.lua:379 fn` -> (mod, file, line, fn).
fn source(rest: &str) -> Option<(String, String, u32, String)> {
    let (mod_id, after) = rest.trim().split_once(':')?;
    let mut parts = after.trim().splitn(2, ' ');
    let file_line = parts.next()?;
    let func = parts.next().unwrap_or("").trim().to_string();
    let (file, line) = match file_line.rsplit_once(':') {
        Some((f, l)) => (f.to_string(), l.parse().unwrap_or(0)),
        None => (file_line.to_string(), 0),
    };
    Some((mod_id.trim().to_string(), file, line, func))
}

fn parse_handler(body: &str) -> Option<Handler> {
    // `18.43 ms/s  OnPlayerUpdate      1872 calls, worst frame   1.4 ms  Mod: file:line fn`
    let (left, src) = body.split_once(" ms  ")?;
    let ms_per_s = num_before(left, " ms/s")?;
    let after = left.split_once(" ms/s")?.1.trim();
    let mut it = after.split_whitespace();
    let event = it.next()?.to_string();
    let calls = num_before(after, " calls")? as u64;
    let worst_ms = num_after(after, "worst frame")?;
    let (mod_id, file, line, func) = source(src)?;
    Some(Handler { ms_per_s, event, calls, worst_ms, mod_id, file, line, func })
}

fn parse_slow(body: &str) -> Option<SlowCall> {
    // `14.3 ms in LoadChunk (361 calls, longest 0.7 ms) <- Mod: file:line fn`
    let (left, src) = body.split_once("<-")?;
    let ms = num_before(left, " ms in")?;
    let event = left.split_once(" ms in ")?.1.split_whitespace().next()?.to_string();
    let calls = num_before(left, " calls")? as u64;
    let longest_ms = num_after(left, "longest ")?;
    let (mod_id, file, line, func) = source(src)?;
    Some(SlowCall { ms, event, calls, longest_ms, mod_id, file, line, func })
}

/// The lines of one session as windows, oldest first. None when the mod wrote nothing.
pub fn parse(lines: &[&str]) -> Vec<Window> {
    let mut windows: Vec<Window> = Vec::new();
    for raw in lines {
        let Some(at) = raw.find(TAG) else { continue };
        let body = &raw[at + TAG.len()..];
        if let Some(rest) = body.strip_prefix("last ") {
            // the summary opens a window
            let parts: Vec<&str> = rest.split(" | ").collect();
            let mut w = Window { index: windows.len(), ..Default::default() };
            if let Some(p) = parts.first() {
                w.frames = num_before(p, " frames").unwrap_or(0.0) as u64;
                w.fps_avg = num_before(p, " fps avg").unwrap_or(0.0);
                w.low1_fps = num_after(p, "low ").unwrap_or(0.0);
            }
            for p in &parts[1..] {
                if let Some(ft) = p.strip_prefix("frame time ") {
                    w.p50 = num_after(ft, "p50 ").unwrap_or(0.0);
                    w.p95 = num_after(ft, "p95 ").unwrap_or(0.0);
                    w.p99 = num_after(ft, "p99 ").unwrap_or(0.0);
                    w.max_ms = num_after(ft, "max ").unwrap_or(0.0);
                } else if let Some(ov) = p.strip_prefix("over ") {
                    w.over25 = num_after(ov, "25ms ").unwrap_or(0.0) as u64;
                    w.over50 = num_after(ov, "50ms ").unwrap_or(0.0) as u64;
                    w.over100 = num_after(ov, "100ms ").unwrap_or(0.0) as u64;
                } else if let Some(gc) = p.strip_prefix("GC ") {
                    w.gc_count = num_before(gc, " collections").unwrap_or(0.0) as u64;
                    w.gc_ms = num_before(gc, " ms").unwrap_or(0.0) as u64;
                }
            }
            windows.push(w);
            continue;
        }
        let Some(w) = windows.last_mut() else { continue };
        if let Some(rest) = body.strip_prefix("chunk pacing: ") {
            w.chunks_done = num_before(rest, " chunks finished").unwrap_or(0.0) as u64;
            w.chunk_avg_ms = num_after(rest, "avg ").unwrap_or(0.0);
            w.chunks_deferred = num_before(rest, " deferred").unwrap_or(0.0) as u64;
        } else if let Some(rest) = body.strip_prefix("Lua event handlers, ") {
            w.lua_ms_per_s = num_before(rest, " ms per second").unwrap_or(0.0);
            w.lua_handlers = num_before(rest, " handlers").unwrap_or(0.0) as u64;
        } else if let Some(rest) = body.strip_prefix("slow Lua: ") {
            if let Some(s) = parse_slow(rest) {
                w.slow.push(s);
            }
        } else if body.starts_with(' ') {
            if let Some(h) = parse_handler(body.trim_start()) {
                w.handlers.push(h);
            }
        }
    }
    windows
}

/// The windows summed up: the session's numbers and each mod's.
pub fn report(log_file: &str, windows: Vec<Window>) -> Option<SmoothFramesReport> {
    if windows.is_empty() {
        return None;
    }
    let n = windows.len() as f64;
    struct Acc {
        windows: usize,
        total: f64,
        max: f64,
        worst: f64,
        hitches: u64,
        handlers: BTreeMap<(String, String, u32, String), (f64, f64, u64, usize)>,
    }
    let mut by_mod: BTreeMap<String, Acc> = BTreeMap::new();
    for w in &windows {
        let mut in_window: BTreeMap<&str, (f64, f64)> = BTreeMap::new();
        for h in &w.handlers {
            let e = in_window.entry(h.mod_id.as_str()).or_insert((0.0, 0.0));
            e.0 += h.ms_per_s;
            e.1 = e.1.max(h.worst_ms);
            let acc = by_mod.entry(h.mod_id.clone()).or_insert_with(|| Acc { windows: 0, total: 0.0, max: 0.0, worst: 0.0, hitches: 0, handlers: BTreeMap::new() });
            let k = (h.event.clone(), h.file.clone(), h.line, h.func.clone());
            let s = acc.handlers.entry(k).or_insert((0.0, 0.0, 0, 0));
            s.0 += h.ms_per_s;
            s.1 = s.1.max(h.worst_ms);
            s.2 = s.2.max(h.calls);
            s.3 += 1;
        }
        for (mod_id, (sum, worst)) in in_window {
            let acc = by_mod.get_mut(mod_id).expect("inserted above");
            acc.windows += 1;
            acc.total += sum;
            acc.max = acc.max.max(sum);
            acc.worst = acc.worst.max(worst);
            if worst >= HITCH_MS {
                acc.hitches += 1;
            }
        }
    }
    let mut mods: Vec<ModFrames> = by_mod
        .into_iter()
        .map(|(mod_id, acc)| {
            let mut top: Vec<Handler> = acc
                .handlers
                .into_iter()
                .map(|((event, file, line, func), (sum, worst, calls, seen))| Handler { ms_per_s: sum / seen as f64, event, calls, worst_ms: worst, mod_id: mod_id.clone(), file, line, func })
                .collect();
            top.sort_by(|a, b| b.ms_per_s.partial_cmp(&a.ms_per_s).unwrap_or(std::cmp::Ordering::Equal));
            top.truncate(3);
            ModFrames { mod_id, windows: acc.windows, ms_per_s_avg: acc.total / acc.windows as f64, ms_per_s_max: acc.max, worst_frame_ms: acc.worst, hitches: acc.hitches, top }
        })
        .collect();
    mods.sort_by(|a, b| b.ms_per_s_avg.partial_cmp(&a.ms_per_s_avg).unwrap_or(std::cmp::Ordering::Equal));
    Some(SmoothFramesReport {
        log_file: log_file.to_string(),
        minutes: windows.len(),
        fps_avg: windows.iter().map(|w| w.fps_avg).sum::<f64>() / n,
        low1_min: windows.iter().map(|w| w.low1_fps).fold(f64::INFINITY, f64::min),
        over25: windows.iter().map(|w| w.over25).sum(),
        over50: windows.iter().map(|w| w.over50).sum(),
        over100: windows.iter().map(|w| w.over100).sum(),
        gc_ms: windows.iter().map(|w| w.gc_ms).sum(),
        chunks_done: windows.iter().map(|w| w.chunks_done).sum(),
        chunks_deferred: windows.iter().map(|w| w.chunks_deferred).sum(),
        lua_ms_per_s_avg: windows.iter().map(|w| w.lua_ms_per_s).sum::<f64>() / n,
        mods,
        windows,
    })
}

/// The current session's console (`console.txt`); None when Smooth Frames wrote nothing there.
pub fn read(user_zomboid_dir: &str) -> Option<SmoothFramesReport> {
    let z = crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next()?;
    let file = z.join("console.txt");
    read_file(&file)
}

pub fn read_file(file: &Path) -> Option<SmoothFramesReport> {
    let text = std::fs::read_to_string(file).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    report(&file.to_string_lossy(), parse(&lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: &[&str] = &[
        "LOG  : General      f:233818> [SmoothFrames] last 60s: 3595 frames, 59.9 fps avg, 1% low 44.0 fps | frame time p50 17.0 p95 17.0 p99 18.5 max 51.4 ms | over 25ms 6, 50ms 1, 100ms 0 | main-thread work p50 12.0 p99 18.0 max 51.2 ms | GC 10 collections 1065 ms",
        "LOG  : General      f:233818> [SmoothFrames] chunk pacing: 114 chunks finished (avg 5.4 ms each), 27 deferred to a later frame",
        "LOG  : General      f:233818> [SmoothFrames] Lua event handlers, last 60s: 102.1 ms per second of play across 565 handlers. Top 10:",
        "LOG  : General      f:233818> [SmoothFrames]    24.33 ms/s  OnPlayerUpdate      3595 calls, worst frame   1.1 ms  FunctionalClothingPockets: client/FunctionalClothingPockets/FunctionalClothingPockets_EquipConverter.lua:379 managePlayerClothing",
        "LOG  : General      f:233818> [SmoothFrames]     6.29 ms/s  OnTick              3595 calls, worst frame  11.8 ms  ProjectALifeNPCs: server/ProjectALife/Core/ALifeServerBootstrap.lua:35 OnTick",
        "LOG  : General      f:233818> [SmoothFrames]     4.65 ms/s  OnTickEvenPaused+   14296 calls, worst frame   0.3 ms  ProjectALifeNPCs: client/ProjectALife/Combat/ALifeHordeRelations.lua:487 function",
        "LOG  : General      f:233818> [SmoothFrames]     2.56 ms/s  OnTick              3497 calls, worst frame   0.4 ms  vanilla: client/Vehicles/ISUI/ISVehicleDashboard.lua:660 damageChecker",
        "LOG  : General      f:233818> [SmoothFrames] slow Lua: 14.3 ms in LoadChunk (361 calls, longest 0.7 ms) <- SaucedCarts: server/SaucedCarts/WorldSpawning.lua:652 onLoadChunk",
        "LOG  : General      f:237315> [SmoothFrames] last 60s: 3497 frames, 58.3 fps avg, 1% low 31.1 fps | frame time p50 17.0 p95 23.5 p99 29.0 max 47.8 ms | over 25ms 118, 50ms 0, 100ms 0 | main-thread work p50 11.0 p99 28.5 max 47.6 ms | GC 5 collections 545 ms",
        "LOG  : General      f:237315> [SmoothFrames]     7.94 ms/s  OnTick              3497 calls, worst frame  28.4 ms  ProjectALifeNPCs: server/ProjectALife/Core/ALifeServerBootstrap.lua:35 OnTick",
        "LOG  : General      f:237315> [SmoothFrames] vehicle parts: 8759 bone updates skipped of 8772 parked-part checks (100%), 8 tracked",
    ];

    #[test]
    fn a_window_is_read_whole() {
        let w = parse(SESSION);
        assert_eq!(w.len(), 2);
        let first = &w[0];
        assert_eq!((first.frames, first.fps_avg, first.low1_fps), (3595, 59.9, 44.0));
        assert_eq!((first.p50, first.p95, first.p99, first.max_ms), (17.0, 17.0, 18.5, 51.4));
        assert_eq!((first.over25, first.over50, first.over100), (6, 1, 0));
        assert_eq!((first.gc_count, first.gc_ms), (10, 1065));
        assert_eq!((first.chunks_done, first.chunk_avg_ms, first.chunks_deferred), (114, 5.4, 27));
        assert_eq!((first.lua_ms_per_s, first.lua_handlers), (102.1, 565));
        assert_eq!(first.handlers.len(), 4);
        let h = &first.handlers[0];
        assert_eq!((h.ms_per_s, h.event.as_str(), h.calls, h.worst_ms), (24.33, "OnPlayerUpdate", 3595, 1.1));
        assert_eq!((h.mod_id.as_str(), h.file.as_str(), h.line, h.func.as_str()), ("FunctionalClothingPockets", "client/FunctionalClothingPockets/FunctionalClothingPockets_EquipConverter.lua", 379, "managePlayerClothing"));
        assert_eq!(first.handlers[3].mod_id, "vanilla");
        let s = &first.slow[0];
        assert_eq!((s.ms, s.event.as_str(), s.calls, s.longest_ms, s.mod_id.as_str(), s.line), (14.3, "LoadChunk", 361, 0.7, "SaucedCarts", 652));
    }

    #[test]
    fn mods_are_summed_over_the_windows_they_appear_in() {
        let r = report("console.txt", parse(SESSION)).expect("a report");
        assert_eq!(r.minutes, 2);
        assert_eq!((r.over25, r.gc_ms, r.chunks_done), (124, 1610, 114));
        assert_eq!(r.low1_min, 31.1);
        let alife = r.mods.iter().find(|m| m.mod_id == "ProjectALifeNPCs").expect("A-Life");
        // 6.29 + 4.65 in the first window, 7.94 in the second
        assert_eq!(alife.windows, 2);
        assert!((alife.ms_per_s_avg - (10.94 + 7.94) / 2.0).abs() < 0.001);
        assert!((alife.ms_per_s_max - 10.94).abs() < 0.001);
        assert_eq!(alife.worst_frame_ms, 28.4);
        assert_eq!(alife.hitches, 1);
        assert_eq!(alife.top[0].event, "OnTick");
        assert!((alife.top[0].ms_per_s - (6.29 + 7.94) / 2.0).abs() < 0.001);
        // The costliest mod comes first.
        assert_eq!(r.mods[0].mod_id, "FunctionalClothingPockets");
        assert!(report("x", Vec::new()).is_none());
    }
}
