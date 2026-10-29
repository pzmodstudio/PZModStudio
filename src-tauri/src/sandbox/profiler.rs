//! How much of the game's time each mod's Lua takes while the game runs.
//!
//! The game keeps no such number, so the Studio measures it with a small mod of its own,
//! `PZModStudio_Profiler` (source in mods/PZModStudio_Profiler). It goes first in the load
//! order: the game runs every mod's shared Lua before any client Lua, and the mods in their
//! order, so from there it sees every `Events.X.Add` of every other mod and wraps the handler in
//! one that clocks it. Every 5 s it writes the totals to `Zomboid/Lua/pz_profile.json`, each
//! handler keyed by the full path of the file that defines it (the game's own
//! getFilenameOfClosure) and the line it starts on.
//!
//! A save loads with its own list (its `mods.txt`), not default.txt, so switching measuring on
//! adds the profiler to both: default.txt for new games and the menu, and the save the player
//! played last. Switching it off takes it out of default.txt and of every save that has it; the
//! profiler keeps nothing in a save, so a save that still lists it loads the same either way.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const PROFILER_ID: &str = "PZModStudio_Profiler";
const PROFILER_LUA: &str = include_str!("../../../mods/PZModStudio_Profiler/42/media/lua/shared/PZModStudio_Profiler.lua");
const PROFILER_MOD_INFO: &str = include_str!("../../../mods/PZModStudio_Profiler/42/mod.info");
const OUTPUT_FILE: &str = "pz_profile.json";
/// Handlers shown per mod; the rest only count in the mod's total.
const HANDLERS_PER_MOD: usize = 5;

/// What the profiler wrote, as it wrote it.
#[derive(Debug, Clone, Deserialize)]
struct RawProfile {
    since: u64,
    now: u64,
    #[serde(default)]
    in_game: bool,
    #[serde(default)]
    ticks: u64,
    #[serde(default)]
    handlers: Vec<RawHandler>,
    #[serde(default)]
    frames: Option<RawFrames>,
}

/// The frame counts as the profiler writes them (version 3 and up).
#[derive(Debug, Clone, Deserialize, Default)]
struct RawFrames {
    count: u64,
    sum: u64,
    max: u64,
    over25: u64,
    over50: u64,
    over100: u64,
    chunks: u64,
    squares: u64,
    /// Frames per whole millisecond, 1 ms to 200 ms (the last step holds the longer ones).
    #[serde(default)]
    hist: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawHandler {
    file: String,
    line: u32,
    event: String,
    ms: u64,
    calls: u64,
    worst: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandlerCost {
    pub event: String,
    /// The file, from the mod's folder on (`42/media/lua/client/x.lua`), or the game's own path.
    pub file: String,
    pub line: u32,
    pub ms: u64,
    pub calls: u64,
    pub worst: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModCost {
    /// The mod's id; None for the game's own files (a mod can `require` one after the profiler
    /// is in place, and its handlers are timed with the rest).
    pub mod_id: Option<String>,
    pub name: String,
    pub ms: u64,
    pub calls: u64,
    /// The longest single call, in ms: a hitch, even when the total is small.
    pub worst: u64,
    /// The handler lives in a Studio package that carries this mod's file merged.
    pub via_package: bool,
    pub handlers: Vec<HandlerCost>,
}

/// The frames of the save, from the profiler's own count: the same numbers Smooth Frames gives,
/// without that mod.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FrameStats {
    pub frames: u64,
    pub fps_avg: f64,
    /// The average of the slowest 1 % of frames, as fps.
    pub low1_fps: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max_ms: u64,
    pub over25: u64,
    pub over50: u64,
    pub over100: u64,
    pub chunks: u64,
    pub squares: u64,
}

/// Percentiles and the 1 % low from the whole-millisecond steps.
fn frame_stats(raw: &RawFrames) -> Option<FrameStats> {
    if raw.count == 0 || raw.sum == 0 {
        return None;
    }
    let total: u64 = raw.hist.iter().sum::<u64>().max(1);
    let percentile = |q: f64| -> f64 {
        let target = ((total as f64) * q).ceil().max(1.0) as u64;
        let mut seen = 0u64;
        for (i, n) in raw.hist.iter().enumerate() {
            seen += n;
            if seen >= target {
                return (i + 1) as f64;
            }
        }
        raw.hist.len() as f64
    };
    // The slowest 1 % (at least one frame), averaged; the last step is open-ended, so the
    // frames in it take the longest frame seen.
    let slowest = ((total as f64) * 0.01).ceil().max(1.0) as u64;
    let mut left = slowest;
    let mut sum_ms = 0.0;
    for (i, n) in raw.hist.iter().enumerate().rev() {
        if left == 0 {
            break;
        }
        let take = (*n).min(left);
        let ms = if i + 1 == raw.hist.len() { raw.max.max((i + 1) as u64) as f64 } else { (i + 1) as f64 };
        sum_ms += take as f64 * ms;
        left -= take;
    }
    let taken = slowest - left;
    let low1_fps = if taken > 0 && sum_ms > 0.0 { 1000.0 / (sum_ms / taken as f64) } else { 0.0 };
    Some(FrameStats {
        frames: raw.count,
        fps_avg: 1000.0 / (raw.sum as f64 / raw.count as f64),
        low1_fps,
        p50: percentile(0.50),
        p95: percentile(0.95),
        p99: percentile(0.99),
        max_ms: raw.max,
        over25: raw.over25,
        over50: raw.over50,
        over100: raw.over100,
        chunks: raw.chunks,
        squares: raw.squares,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileReport {
    /// Real time measured, in ms: since the game started (in a save, since it was entered).
    pub window_ms: u64,
    pub in_game: bool,
    /// Frames counted in the save.
    pub ticks: u64,
    /// Seconds since the profiler last wrote: more than a few means the game is not running it.
    pub age_secs: u64,
    pub total_ms: u64,
    pub mods: Vec<ModCost>,
    /// The frames counted, when the profiler is version 3 or newer.
    pub frames: Option<FrameStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfilerStatus {
    /// In default.txt.
    pub on: bool,
    /// The saves whose own list has it (`<mode>/<world>`).
    pub saves: Vec<String>,
    pub report: Option<ProfileReport>,
}

fn zomboid_dir(user_zomboid_dir: &str) -> Result<PathBuf, String> {
    crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| "The Zomboid folder was not found.".to_string())
}

/// Writes the profiler into `Zomboid/mods`, always this build's copy.
fn install(z_dir: &Path) -> Result<(), String> {
    let root = z_dir.join("mods").join(PROFILER_ID);
    let lua_dir = root.join("42").join("media").join("lua").join("shared");
    std::fs::create_dir_all(&lua_dir).map_err(|e| format!("Cannot create {}: {}", lua_dir.display(), e))?;
    // B42 lists a mod only with a common/ folder beside its version folder.
    std::fs::create_dir_all(root.join("common")).map_err(|e| e.to_string())?;
    std::fs::write(root.join("42").join("mod.info"), PROFILER_MOD_INFO).map_err(|e| e.to_string())?;
    std::fs::write(lua_dir.join("PZModStudio_Profiler.lua"), PROFILER_LUA).map_err(|e| e.to_string())?;
    Ok(())
}

const MOD_LINE: &str = "mod = PZModStudio_Profiler,";

/// A mods.txt / default.txt with the profiler's line first in its `mods` block, or without it.
/// The rest of the file is left exactly as it was.
fn with_profiler(text: &str, on: bool) -> String {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out: Vec<String> = Vec::new();
    let mut in_mods = false;
    let mut added = false;
    for line in text.lines() {
        let t = line.trim();
        if t == MOD_LINE {
            continue;
        }
        out.push(line.to_string());
        if t == "mods" {
            in_mods = true;
        } else if in_mods && t == "{" {
            in_mods = false;
            if on && !added {
                out.push(format!("    {}", MOD_LINE));
                added = true;
            }
        }
    }
    let mut joined = out.join(eol);
    if text.ends_with('\n') {
        joined.push_str(eol);
    }
    joined
}

fn lists_profiler(path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|t| t.lines().any(|l| l.trim() == MOD_LINE))
}

/// The save played last: the one whose `map_t.bin` (written on every save) is newest.
fn last_played_save(z_dir: &Path) -> Option<String> {
    let saves = crate::save_backup::saves_dir(z_dir);
    crate::save_backup::list_saves(z_dir)
        .into_iter()
        .filter_map(|id| {
            let dir = saves.join(&id);
            let stamp = std::fs::metadata(dir.join("map_t.bin")).or_else(|_| std::fs::metadata(&dir)).and_then(|m| m.modified()).ok()?;
            dir.join("mods.txt").is_file().then_some((stamp, id))
        })
        .max_by_key(|(stamp, _)| *stamp)
        .map(|(_, id)| id)
}

fn saves_with_profiler(z_dir: &Path) -> Vec<String> {
    let saves = crate::save_backup::saves_dir(z_dir);
    crate::save_backup::list_saves(z_dir).into_iter().filter(|id| lists_profiler(&saves.join(id).join("mods.txt"))).collect()
}

/// Switches measuring on or off (see the module's comment for where the profiler is listed).
pub fn set_on(user_zomboid_dir: &str, workshop_dir: &str, on: bool) -> Result<ProfilerStatus, String> {
    let z_dir = zomboid_dir(user_zomboid_dir)?;
    let default_txt = z_dir.join("mods").join("default.txt");
    if on {
        install(&z_dir)?;
    }
    let mut active = crate::load_order::ini_parser::parse_default_txt(&default_txt);
    active.retain(|id| id != PROFILER_ID);
    if on {
        active.insert(0, PROFILER_ID.to_string());
    }
    crate::load_order::ini_parser::write_mod_list_ini(&default_txt.to_string_lossy(), &active)?;

    let saves_root = crate::save_backup::saves_dir(&z_dir);
    let targets: Vec<String> = if on { last_played_save(&z_dir).into_iter().collect() } else { saves_with_profiler(&z_dir) };
    for id in targets {
        let path = saves_root.join(&id).join("mods.txt");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        let changed = with_profiler(&text, on);
        if changed != text {
            std::fs::write(&path, changed).map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
        }
    }
    Ok(status(user_zomboid_dir, workshop_dir))
}

/// Whether measuring is on, and the last measurement.
pub fn status(user_zomboid_dir: &str, workshop_dir: &str) -> ProfilerStatus {
    let Ok(z_dir) = zomboid_dir(user_zomboid_dir) else { return ProfilerStatus::default() };
    ProfilerStatus {
        on: lists_profiler(&z_dir.join("mods").join("default.txt")),
        saves: saves_with_profiler(&z_dir),
        report: report(&z_dir, workshop_dir, user_zomboid_dir),
    }
}

fn norm(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

/// Who a file belongs to: the mod whose folder holds it, the longest folder winning (a mod
/// inside another's folder is its own). Returns the owner and the path from its folder on.
struct Owners {
    roots: Vec<(String, usize)>, // normalised root with a trailing '/', index into mods
    mods: Vec<(String, String, PathBuf)>, // id, name, root
}

impl Owners {
    fn new(catalog: &crate::catalog::ModCatalog) -> Self {
        let mut mods = Vec::new();
        let mut roots = Vec::new();
        for m in catalog.mods.iter().filter(|m| m.shadowed_by.is_none()) {
            roots.push((format!("{}/", norm(&m.root_dir).trim_end_matches('/')), mods.len()));
            mods.push((m.id.clone(), m.name.clone(), PathBuf::from(&m.root_dir)));
        }
        roots.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Owners { roots, mods }
    }

    fn of(&self, file: &str) -> Option<(usize, String)> {
        let f = norm(file);
        self.roots
            .iter()
            .find(|(root, _)| f.starts_with(root.as_str()))
            .map(|(root, i)| {
                let file = file.replace('\\', "/");
                (*i, file.get(root.len()..).unwrap_or(&file).to_string())
            })
    }

    /// For a file in a Studio package: the mod that ships the same `media/...` path, which is
    /// whose code the package carries (merged).
    fn shipped_by(&self, rel: &str, skip: usize) -> Option<usize> {
        let media = rel.find("media/").map(|i| &rel[i..])?;
        self.mods.iter().enumerate().find_map(|(i, (id, _, root))| {
            if i == skip || id.starts_with("Z_PZModStudio_") {
                return None;
            }
            let dirs = std::fs::read_dir(root).ok()?;
            let mut candidates: Vec<PathBuf> = dirs.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
            candidates.push(root.clone());
            candidates.iter().any(|d| d.join(media).is_file()).then_some(i)
        })
    }
}

/// The last report and the file state it was made from: the screen asks every 5 s, and the
/// catalog behind it takes about a second to build, so it is made again only when the profiler
/// wrote.
static LAST_REPORT: std::sync::Mutex<Option<(PathBuf, std::time::SystemTime, u64, ProfileReport)>> = std::sync::Mutex::new(None);

fn report(z_dir: &Path, workshop_dir: &str, user_zomboid_dir: &str) -> Option<ProfileReport> {
    let path = z_dir.join("Lua").join(OUTPUT_FILE);
    let meta = std::fs::metadata(&path).ok()?;
    let modified = meta.modified().ok()?;
    let age_secs = modified.elapsed().map(|d| d.as_secs()).unwrap_or(0);
    let mut cache = LAST_REPORT.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((p, m, len, report)) = cache.as_ref() {
        if *p == path && *m == modified && *len == meta.len() {
            return Some(ProfileReport { age_secs, ..report.clone() });
        }
    }
    let text = std::fs::read_to_string(&path).ok()?;
    let raw: RawProfile = serde_json::from_str(&text).ok()?;
    // While the game runs the file changes every 5 s; which folder is whose does not.
    let mut owners = LAST_OWNERS.lock().unwrap_or_else(|p| p.into_inner());
    if owners.as_ref().is_none_or(|(made, _)| made.elapsed() > std::time::Duration::from_secs(60)) {
        let catalog = crate::catalog::build_catalog(workshop_dir, user_zomboid_dir, None);
        *owners = Some((std::time::Instant::now(), Owners::new(&catalog)));
    }
    let report = summarise(raw, &owners.as_ref()?.1, age_secs);
    *cache = Some((path, modified, meta.len(), report.clone()));
    Some(report)
}

static LAST_OWNERS: std::sync::Mutex<Option<(std::time::Instant, Owners)>> = std::sync::Mutex::new(None);

fn summarise(raw: RawProfile, owners: &Owners, age_secs: u64) -> ProfileReport {
    let frames = raw.frames.as_ref().and_then(frame_stats);
    let mut by_key: HashMap<String, ModCost> = HashMap::new();
    let mut package_owner: HashMap<String, Option<usize>> = HashMap::new();
    for h in raw.handlers.into_iter().filter(|h| h.calls > 0) {
        let (key, mod_id, name, via_package, file) = match owners.of(&h.file) {
            Some((i, rel)) => {
                let (id, name, _) = &owners.mods[i];
                if id.starts_with("Z_PZModStudio_") && id != "Z_PZModStudio_Bridge" {
                    let shipped = *package_owner.entry(rel.clone()).or_insert_with(|| owners.shipped_by(&rel, i));
                    match shipped {
                        Some(j) => {
                            let (oid, oname, _) = &owners.mods[j];
                            (oid.clone(), Some(oid.clone()), oname.clone(), true, rel)
                        }
                        None => (id.clone(), Some(id.clone()), name.clone(), false, rel),
                    }
                } else {
                    (id.clone(), Some(id.clone()), name.clone(), false, rel)
                }
            }
            None => {
                let shown = h.file.replace('\\', "/");
                let shown = shown.find("media/").map(|i| shown[i..].to_string()).unwrap_or(shown);
                ("\u{0}game".to_string(), None, String::new(), false, shown)
            }
        };
        let entry = by_key.entry(key).or_insert_with(|| ModCost { mod_id, name, ms: 0, calls: 0, worst: 0, via_package, handlers: Vec::new() });
        entry.ms += h.ms;
        entry.calls += h.calls;
        entry.worst = entry.worst.max(h.worst);
        entry.via_package |= via_package;
        entry.handlers.push(HandlerCost { event: h.event, file, line: h.line, ms: h.ms, calls: h.calls, worst: h.worst });
    }
    let mut mods: Vec<ModCost> = by_key.into_values().collect();
    for m in &mut mods {
        m.handlers.sort_by(|a, b| b.ms.cmp(&a.ms).then(b.worst.cmp(&a.worst)));
        m.handlers.truncate(HANDLERS_PER_MOD);
    }
    mods.sort_by(|a, b| b.ms.cmp(&a.ms).then(b.worst.cmp(&a.worst)));
    ProfileReport {
        window_ms: raw.now.saturating_sub(raw.since),
        in_game: raw.in_game,
        ticks: raw.ticks,
        age_secs,
        total_ms: mods.iter().map(|m| m.ms).sum(),
        mods,
        frames,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_steps_become_percentiles_and_the_one_percent_low() {
        // 100 frames: 90 of 16 ms, 9 of 30 ms, 1 of 180 ms (a frame of d ms goes to step ceil(d)).
        let mut hist = vec![0u64; 200];
        hist[15] = 90;
        hist[29] = 9;
        hist[179] = 1;
        let raw = RawFrames { count: 100, sum: 90 * 16 + 9 * 30 + 180, max: 180, over25: 10, over50: 1, over100: 1, chunks: 7, squares: 300, hist };
        let f = frame_stats(&raw).expect("stats");
        assert_eq!((f.frames, f.max_ms, f.over25, f.chunks, f.squares), (100, 180, 10, 7, 300));
        assert_eq!((f.p50, f.p95, f.p99), (16.0, 30.0, 30.0));
        assert!((f.fps_avg - 1000.0 / 18.9).abs() < 0.01);
        // the slowest 1 % is the one 180 ms frame
        assert!((f.low1_fps - 1000.0 / 180.0).abs() < 0.01);
        assert!(frame_stats(&RawFrames::default()).is_none());
    }

    #[test]
    fn the_profiler_line_goes_first_in_the_mods_block_and_comes_out_again() {
        let text = "VERSION = 1,\r\n\r\nmods\r\n{\r\n    mod = A,\r\n    mod = B,\r\n}\r\n\r\nmaps\r\n{\r\n    map = Muldraugh, KY,\r\n}\r\n";
        let on = with_profiler(text, true);
        assert_eq!(on, "VERSION = 1,\r\n\r\nmods\r\n{\r\n    mod = PZModStudio_Profiler,\r\n    mod = A,\r\n    mod = B,\r\n}\r\n\r\nmaps\r\n{\r\n    map = Muldraugh, KY,\r\n}\r\n");
        assert_eq!(with_profiler(&on, true), on, "never twice");
        assert_eq!(with_profiler(&on, false), text);
    }

    fn owners(root: &Path) -> Owners {
        let mk = |id: &str, dir: &Path| (id.to_string(), id.to_string(), dir.to_path_buf());
        let a = root.join("108600/1/mods/Alpha");
        let pkg = root.join("Zomboid/mods/Z_PZModStudio_MergedPatch");
        std::fs::create_dir_all(a.join("42/media/lua/client")).unwrap();
        std::fs::write(a.join("42/media/lua/client/Merged.lua"), "").unwrap();
        let mods = vec![mk("Alpha", &a), mk("Z_PZModStudio_MergedPatch", &pkg)];
        let mut roots: Vec<(String, usize)> = mods.iter().enumerate().map(|(i, (_, _, r))| (format!("{}/", norm(&r.to_string_lossy())), i)).collect();
        roots.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Owners { roots, mods }
    }

    #[test]
    fn handlers_add_up_per_mod_and_a_package_file_counts_for_the_mod_it_merged() {
        let root = std::env::temp_dir().join(format!("pzms-profiler-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let owners = owners(&root);
        let file = |rel: &str| root.join(rel).to_string_lossy().replace('\\', "/");
        let h = |file: String, event: &str, ms: u64, calls: u64, worst: u64| RawHandler { file, line: 3, event: event.into(), ms, calls, worst };
        let raw = RawProfile {
            since: 1_000,
            now: 61_000,
            in_game: true,
            ticks: 3_600,
            frames: None,
            handlers: vec![
                h(file("108600/1/mods/Alpha/42/media/lua/client/A.lua"), "OnTick", 300, 3_600, 2),
                h(file("108600/1/mods/Alpha/42/media/lua/client/B.lua"), "OnPlayerUpdate", 100, 3_600, 40),
                h(file("Zomboid/mods/Z_PZModStudio_MergedPatch/42/media/lua/client/Merged.lua"), "OnTick", 50, 3_600, 1),
                h("D:/Steam/common/ProjectZomboid/media/lua/client/ISUI/X.lua".into(), "OnTick", 70, 3_600, 1),
                h(file("108600/1/mods/Alpha/42/media/lua/client/Never.lua"), "OnZombieDead", 0, 0, 0),
            ],
        };
        let r = summarise(raw, &owners, 2);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(r.window_ms, 60_000);
        assert_eq!(r.total_ms, 520);
        assert_eq!(r.mods[0].mod_id.as_deref(), Some("Alpha"));
        assert_eq!(r.mods[0].ms, 450);
        assert_eq!(r.mods[0].worst, 40);
        assert!(r.mods[0].via_package);
        assert_eq!(r.mods[0].handlers.len(), 3, "a handler never called is not listed");
        assert_eq!(r.mods[0].handlers[0].file, "42/media/lua/client/A.lua");
        assert_eq!(r.mods[1].mod_id, None);
        assert_eq!(r.mods[1].handlers[0].file, "media/lua/client/ISUI/X.lua");
    }
}
