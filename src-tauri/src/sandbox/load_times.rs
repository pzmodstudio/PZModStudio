//! How long each mod's files took to load in the last session, from the game's own DebugLog.
//!
//! console.txt carries no clock, only a frame counter, so nothing in it can be timed. The game
//! also writes `Zomboid/Logs/logs_<date>/<date>_DebugLog.txt` with a millisecond stamp on every
//! line, and one `LOG : Mod f:0> loading <id>.` line per mod as it reads the mod's files
//! (mod.info, overrides, translations). The gap to the next mod's line is what this mod's files
//! cost — measured 2 ms for most and 1.3 s for a tile pack. The Lua that runs after is loaded in
//! one pass with no line per mod, so it is not timed here, and the chip says so.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModLoadTime {
    /// The id the game wrote, which is the `id=` of mod.info.
    pub mod_id: String,
    pub millis: u64,
}

/// Everything measured in one log: per mod, and the log it came from.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoadTimes {
    pub log_file: Option<String>,
    /// When that session started, as the log's first stamp says ("17-10-26 12:20:02").
    pub session: Option<String>,
    pub mods: Vec<ModLoadTime>,
}

/// `[17-10-26 12:20:18.962]` as milliseconds within the day (a session does not cross midnight
/// between two mods' lines, and only differences are used).
fn stamp_millis(line: &str) -> Option<(u64, &str)> {
    let inner = line.strip_prefix('[')?;
    let end = inner.find(']')?;
    let stamp = &inner[..end];
    let (_, time) = stamp.split_once(' ')?;
    let mut parts = time.split(':');
    let h: u64 = parts.next()?.parse().ok()?;
    let m: u64 = parts.next()?.parse().ok()?;
    let (s, ms) = parts.next()?.split_once('.')?;
    let s: u64 = s.parse().ok()?;
    let ms: u64 = ms.parse().ok()?;
    Some((((h * 60 + m) * 60 + s) * 1000 + ms, stamp))
}

/// The per-mod times out of the log's lines.
pub fn parse(lines: impl Iterator<Item = String>) -> LoadTimes {
    let mut out = LoadTimes::default();
    let mut pending: Option<(String, u64)> = None;
    for line in lines {
        let Some((at, stamp)) = stamp_millis(&line) else { continue };
        if out.session.is_none() {
            out.session = Some(stamp.to_string());
        }
        let rest = &line[line.find(']').map(|i| i + 1).unwrap_or(0)..];
        let is_mod_line = rest.trim_start().starts_with("LOG  : Mod");
        let loading = rest.split_once("> loading ").map(|(_, id)| id.trim().trim_end_matches('.').to_string());
        // A mod's files end where the next line of any kind begins after its last override line;
        // the next "loading" is the usual end, and the first non-Mod line closes the last mod.
        if let Some((id, start)) = pending.take() {
            if loading.is_some() || !is_mod_line {
                out.mods.push(ModLoadTime { mod_id: id, millis: at.saturating_sub(start) });
            } else {
                pending = Some((id, start));
            }
        }
        if let Some(id) = loading.filter(|_| is_mod_line) {
            pending = Some((id, at));
        }
    }
    // A log cut off right after a "loading" line: that mod's time is unknown, not zero.
    if let Some((id, _)) = pending {
        out.mods.push(ModLoadTime { mod_id: id, millis: 0 });
    }
    out
}

/// The newest DebugLog under `Zomboid/Logs` (in `logs_<date>/` folders and loose), by its
/// modification time.
pub fn newest_debug_log(user_zomboid_dir: &str) -> Option<PathBuf> {
    debug_logs_newest_first(user_zomboid_dir).into_iter().next()
}

/// Every DebugLog under `Zomboid/Logs`, newest first.
fn debug_logs_newest_first(user_zomboid_dir: &str) -> Vec<PathBuf> {
    let Some(logs) = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .map(|d| d.join("Logs"))
        .find(|p| p.is_dir())
    else {
        return Vec::new();
    };
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    let mut consider = |path: &Path| {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with("_DebugLog.txt") {
            if let Ok(meta) = path.metadata() {
                candidates.push((meta.modified().unwrap_or(std::time::UNIX_EPOCH), path.to_path_buf()));
            }
        }
    };
    for entry in std::fs::read_dir(&logs).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            for inner in std::fs::read_dir(&path).ok().into_iter().flatten().flatten() {
                consider(&inner.path());
            }
        } else {
            consider(&path);
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().map(|(_, p)| p).collect()
}

/// The last session's load times, or nothing when no DebugLog exists.
pub fn from_logs(user_zomboid_dir: &str) -> LoadTimes {
    let Some(path) = newest_debug_log(user_zomboid_dir) else { return LoadTimes::default() };
    let Ok(file) = std::fs::File::open(&path) else { return LoadTimes::default() };
    use std::io::BufRead;
    let mut times = parse(std::io::BufReader::new(file).lines().map_while(Result::ok));
    times.log_file = Some(path.to_string_lossy().to_string());
    times
}

// ------------------------------------------------------------------------------------------
// The game's start, phase by phase.
//
// From the launch to the main menu the game writes a line as it starts each stage ("Loading
// Mods.", "Loading Lua.", "Loading models and animations." ...), each with its millisecond stamp,
// so the time between two is what that stage took. Every launch leaves its DebugLog, so the last
// start can be set beside the usual one: a mod added last week that put 20 s on the models
// stage shows.
// ------------------------------------------------------------------------------------------

/// The stages, in the order the game runs them: the id the screen names it by, and the text
/// that starts it. A stage runs until the next one found starts.
const BOOT_MARKS: &[(&str, &str)] = &[
    ("mods", "> Loading Mods."),
    ("translations", "> Loading Translations."),
    ("scripts", "> Loading Scripts."),
    ("clothing", "> Loading Clothing."),
    ("lua", "> Loading Lua."),
    // The game's tile packs; mods' own packs load earlier, with their files.
    ("textures", "> texturepack: loading Tiles"),
    ("models", "> Loading models and animations."),
    // The models stage ends at the next line the game writes; from there to the menu is the
    // game's own Lua start (OnGameBoot) and the logo screens.
    ("menu", ""),
];
/// The main menu is up when the logo screen is left.
const MENU_REACHED: &str = "STATE: exit zombie.gameStates.TISLogoState";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BootPhase {
    pub id: String,
    pub millis: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BootTimes {
    /// When the game started, as the log's first stamp says ("22-10-26 18:22:34.767").
    pub session: String,
    /// From the first line to the main menu.
    pub total_ms: u64,
    /// The engine's own start, before the first stage, counts as "start".
    pub phases: Vec<BootPhase>,
    /// Mods the game read the files of.
    pub mods: usize,
}

/// The start's stages out of one DebugLog; None when the game never reached the menu.
pub fn boot_times(lines: impl Iterator<Item = String>) -> Option<BootTimes> {
    let mut first: Option<(u64, String)> = None;
    // (stage id, when it started)
    let mut starts: Vec<(&str, u64)> = Vec::new();
    let mut models_started = false;
    let mut mods = std::collections::HashSet::new();
    for line in lines {
        let Some((at, stamp)) = stamp_millis(&line) else { continue };
        let first_at = first.get_or_insert_with(|| (at, stamp.to_string())).0;
        // Past midnight the stamps start again from 0.
        let at = if at < first_at { at + 24 * 3600 * 1000 } else { at };
        if line.contains(MENU_REACHED) {
            starts.push(("end", at));
            break;
        }
        if models_started {
            starts.push(("menu", at));
            models_started = false;
        }
        if let Some((_, id)) = line.split_once("> loading ").filter(|_| line.contains("LOG  : Mod")) {
            mods.insert(id.trim().trim_end_matches('.').to_string());
        }
        for (id, mark) in BOOT_MARKS {
            if !mark.is_empty() && line.contains(mark) && !starts.iter().any(|(s, _)| s == id) {
                starts.push((id, at));
                if *id == "models" {
                    models_started = true;
                }
            }
        }
    }
    let (first_at, session) = first?;
    let (_, end) = *starts.iter().find(|(id, _)| *id == "end")?;
    let mut phases = Vec::new();
    let mut previous = ("start", first_at);
    for &(id, at) in &starts {
        phases.push(BootPhase { id: previous.0.to_string(), millis: at.saturating_sub(previous.1) });
        previous = (id, at);
    }
    Some(BootTimes { session, total_ms: end.saturating_sub(first_at), phases, mods: mods.len() })
}

/// The last `max` starts that reached the menu, newest first.
pub fn boot_history(user_zomboid_dir: &str, max: usize) -> Vec<BootTimes> {
    use std::io::BufRead;
    debug_logs_newest_first(user_zomboid_dir)
        .into_iter()
        .filter_map(|path| {
            let file = std::fs::File::open(path).ok()?;
            boot_times(std::io::BufReader::new(file).lines().map_while(Result::ok))
        })
        .take(max)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_start_is_cut_into_its_stages_up_to_the_menu() {
        let log = [
            "[22-10-26 18:00:13.000] LOG  : General      f:0> 22-10-2026 18:00:13.",
            "[22-10-26 18:00:17.000] LOG  : General      f:0> Loading Mods.",
            "[22-10-26 18:00:17.500] LOG  : Mod          f:0> loading OmiLibrary.",
            "[22-10-26 18:00:18.000] LOG  : Mod          f:0> loading OmiChat.",
            "[22-10-26 18:00:21.000] LOG  : General      f:0> Loading Translations.",
            "[22-10-26 18:00:22.000] LOG  : General      f:0> Loading Scripts.",
            "[22-10-26 18:00:25.000] LOG  : General      f:0> Loading Clothing.",
            "[22-10-26 18:00:28.000] LOG  : General      f:0> Loading Lua.",
            "[22-10-26 18:00:32.000] LOG  : General      f:0> texturepack: loading Tiles2x.",
            "[22-10-26 18:00:32.500] LOG  : General      f:0> texturepack: loading Overlays2x.",
            "[22-10-26 18:00:33.000] LOG  : General      f:0> Loading models and animations.",
            "[22-10-26 18:00:45.000] LOG  : General      f:0> fmod: globalSystem = 0x1.",
            "[22-10-26 18:00:51.000] LOG  : General      f:0> STATE: exit zombie.gameStates.TISLogoState.",
            "[22-10-26 18:01:10.000] LOG  : Mod          f:0 st:0> loading NotCounted.",
        ];
        let b = boot_times(log.iter().map(|l| l.to_string())).unwrap();
        let ms: Vec<(&str, u64)> = b.phases.iter().map(|p| (p.id.as_str(), p.millis)).collect();
        assert_eq!(
            ms,
            vec![("start", 4000), ("mods", 4000), ("translations", 1000), ("scripts", 3000), ("clothing", 3000), ("lua", 4000), ("textures", 1000), ("models", 12000), ("menu", 6000)]
        );
        assert_eq!(b.total_ms, 38000);
        assert_eq!(b.mods, 2);
        assert_eq!(b.session, "22-10-26 18:00:13.000");
        assert!(boot_times(log[..6].iter().map(|l| l.to_string())).is_none(), "no menu, no start to show");
    }

    #[test]
    fn each_mod_is_timed_to_the_next_mods_line_and_the_last_to_the_next_phase() {
        let log = [
            "[17-10-26 12:20:02.270] LOG  : General      f:0> 17-10-2026 12:20:02.",
            "[17-10-26 12:20:18.962] LOG  : Mod          f:0> loading ZombieBuddy.",
            "[17-10-26 12:20:18.963] LOG  : Mod          f:0> mod \"ZombieBuddy\" overrides media/lua/shared/translate/en/ui.json.",
            "[17-10-26 12:20:18.964] LOG  : Mod          f:0> loading Viewpoint.",
            "[17-10-26 12:20:20.255] LOG  : Mod          f:0> loading ViewpointTileFix.",
            "[17-10-26 12:20:21.546] LOG  : General      f:0> Loading models and animations",
            "LOG  : General      f:0> a console line without a stamp",
        ];
        let t = parse(log.iter().map(|l| l.to_string()));
        assert_eq!(t.session.as_deref(), Some("17-10-26 12:20:02.270"));
        assert_eq!(
            t.mods,
            vec![
                ModLoadTime { mod_id: "ZombieBuddy".into(), millis: 2 },
                ModLoadTime { mod_id: "Viewpoint".into(), millis: 1291 },
                ModLoadTime { mod_id: "ViewpointTileFix".into(), millis: 1291 },
            ]
        );
    }
}
