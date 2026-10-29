//! The memory the game's Java may use (`-Xmx` in `ProjectZomboid64.json`), and what to give it.
//!
//! Project Zomboid runs on Java, which never grows past the `-Xmx` of its launch file. With a big
//! mod list the game's own value runs short: the game stutters while Java frees memory over and
//! over, and at the end writes `java.lang.OutOfMemoryError` and closes. The Studio shows the
//! value, the PC's memory, what it suggests for the active list, and the out-of-memory errors of
//! the last sessions, and changes the value (a copy of the file before it goes to
//! `PZModStudio_Backups`).
//!
//! Each profile keeps its own value (a light list needs less than a heavy one): switching a
//! profile on puts its value in the launch file. A profile without one takes the last value
//! chosen, kept in `<Zomboid>/PZModStudio_GameMemory.txt`. A game update rewrites the launch
//! file with its own value, and the next start from the Studio puts the chosen one back.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CHOSEN_FILE: &str = "PZModStudio_GameMemory.txt";
/// The least the game starts with sensibly, and the step values are given in.
pub const MIN_MB: u64 = 2048;
const STEP_MB: u64 = 512;
/// Logs looked through for out-of-memory errors, newest first.
const LOGS_LOOKED_AT: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameMemory {
    /// The launch file, when found.
    pub file: Option<String>,
    /// `-Xmx` now, in MB.
    pub current_mb: Option<u64>,
    /// What the player chose in the Studio for the active profile (or last, for a profile with
    /// none of its own), if anything.
    pub chosen_mb: Option<u64>,
    /// The active profile, and whether the value is its own.
    pub profile: Option<String>,
    pub profile_has_own: bool,
    /// The PC's memory, in MB.
    pub total_mb: Option<u64>,
    /// The most the Studio offers: half the PC's memory (Windows, the GPU driver and the game's
    /// own native side need the rest).
    pub max_mb: u64,
    pub recommended_mb: u64,
    pub active_mods: usize,
    /// The sessions whose logs have `java.lang.OutOfMemoryError`, newest first (file names).
    pub out_of_memory: Vec<String>,
    /// Windows ran out of memory with the game open (the last 14 days): what held it and
    /// whether the game was closed for it. The game's Java heap is not the whole story.
    pub windows_low_memory: Vec<crate::sandbox::windows_memory::LowMemoryEvent>,
    pub game_running: bool,
}

/// `-Xmx6144m`, `-Xmx6g`, `-Xmx6G` as MB.
pub fn parse_xmx(arg: &str) -> Option<u64> {
    let value = arg.strip_prefix("-Xmx")?;
    let (digits, unit) = value.split_at(value.find(|c: char| !c.is_ascii_digit()).unwrap_or(value.len()));
    let n: u64 = digits.parse().ok()?;
    match unit.to_ascii_lowercase().as_str() {
        "g" => Some(n * 1024),
        "m" => Some(n),
        "k" => Some(n / 1024),
        "" => Some(n / (1024 * 1024)),
        _ => None,
    }
}

/// What the Studio suggests: more for more mods (models, textures and Lua all live in Java's
/// memory), more again when the game ran out, never past `max_mb`.
pub fn recommend(active_mods: usize, current_mb: Option<u64>, ran_out: bool, max_mb: u64) -> u64 {
    let by_mods = match active_mods {
        0..=30 => 3072,
        31..=100 => 4096,
        101..=200 => 6144,
        _ => 8192,
    };
    let wanted = if ran_out { by_mods.max(current_mb.unwrap_or(0) + 2048) } else { by_mods };
    wanted.min(max_mb).max(MIN_MB)
}

/// The launch file with `-Xmx` set to `mb`, or None when it has no `vmArgs` to put it in.
pub fn with_xmx(text: &str, mb: u64) -> Result<String, String> {
    let mut json: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("The launch file is not valid JSON: {}", e))?;
    let args = json.get_mut("vmArgs").and_then(|v| v.as_array_mut()).ok_or("The launch file has no vmArgs.")?;
    let arg = serde_json::Value::String(format!("-Xmx{}m", mb));
    match args.iter().position(|a| a.as_str().is_some_and(|s| s.starts_with("-Xmx"))) {
        Some(i) => args[i] = arg,
        None => args.push(arg),
    }
    serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
}

fn current_from(text: &str) -> Option<u64> {
    let json: serde_json::Value = serde_json::from_str(text).ok()?;
    json.get("vmArgs")?.as_array()?.iter().filter_map(|a| a.as_str()).find_map(parse_xmx)
}

#[cfg(windows)]
pub fn total_memory_mb() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: a zeroed MEMORYSTATUSEX with its length set, as the call asks.
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return None;
    }
    Some(status.ullTotalPhys / (1024 * 1024))
}

#[cfg(not(windows))]
pub fn total_memory_mb() -> Option<u64> {
    None
}

fn launch_file(pz_install_dir: &str) -> Option<PathBuf> {
    let path = Path::new(pz_install_dir).join("ProjectZomboid64.json");
    path.is_file().then_some(path)
}

fn zomboid_dir(user_zomboid_dir: &str) -> Option<PathBuf> {
    crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next()
}

fn read_last_chosen(z_dir: &Path) -> Option<u64> {
    std::fs::read_to_string(z_dir.join(CHOSEN_FILE)).ok()?.trim().parse().ok()
}

/// The value the game should have now: the active profile's own, else the last chosen.
fn read_chosen(z_dir: &Path) -> Option<u64> {
    crate::instance_manager::active_memory(z_dir).and_then(|(own, _)| own).or_else(|| read_last_chosen(z_dir))
}

/// The sessions (newest first) whose log says the game ran out of memory.
fn out_of_memory_sessions(z_dir: &Path) -> Vec<String> {
    let mut logs: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    let console = z_dir.join("console.txt");
    if let Ok(m) = std::fs::metadata(&console) {
        logs.push((m.modified().unwrap_or(std::time::UNIX_EPOCH), console));
    }
    for entry in walkdir::WalkDir::new(z_dir.join("Logs")).max_depth(2).into_iter().flatten() {
        let name = entry.file_name().to_string_lossy();
        if name.ends_with("_DebugLog.txt") {
            if let Ok(m) = entry.metadata() {
                logs.push((m.modified().unwrap_or(std::time::UNIX_EPOCH), entry.path().to_path_buf()));
            }
        }
    }
    logs.sort_by(|a, b| b.0.cmp(&a.0));
    logs.into_iter()
        .take(LOGS_LOOKED_AT)
        .filter(|(_, path)| std::fs::read(path).is_ok_and(|bytes| bytes.windows(26).any(|w| w == b"java.lang.OutOfMemoryError")))
        .map(|(_, path)| path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
        .collect()
}

pub fn status(pz_install_dir: &str, user_zomboid_dir: &str) -> GameMemory {
    let file = launch_file(pz_install_dir);
    let current_mb = file.as_ref().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| current_from(&t));
    let z_dir = zomboid_dir(user_zomboid_dir);
    let total_mb = total_memory_mb();
    let max_mb = total_mb.map(|t| (t / 2) / STEP_MB * STEP_MB).unwrap_or(8192).max(MIN_MB);
    let active_mods = z_dir
        .as_ref()
        .map(|z| crate::load_order::ini_parser::parse_default_txt(&z.join("mods").join("default.txt")).len())
        .unwrap_or(0);
    let out_of_memory = z_dir.as_ref().map(|z| out_of_memory_sessions(z)).unwrap_or_default();
    let active = z_dir.as_ref().and_then(|z| crate::instance_manager::active_memory(z));
    GameMemory {
        file: file.map(|f| f.to_string_lossy().to_string()),
        current_mb,
        chosen_mb: z_dir.as_ref().and_then(|z| read_chosen(z)),
        profile_has_own: active.as_ref().is_some_and(|(own, _)| own.is_some()),
        profile: active.map(|(_, name)| name),
        total_mb,
        max_mb,
        recommended_mb: recommend(active_mods, current_mb, !out_of_memory.is_empty(), max_mb),
        active_mods,
        out_of_memory,
        windows_low_memory: crate::sandbox::windows_memory::recent(14),
        game_running: crate::sandbox::find_running_pz_pid().is_some(),
    }
}

/// Writes `-Xmx` into the launch file (a copy of it before goes to PZModStudio_Backups).
fn write(file: &Path, z_dir: &Path, mb: u64) -> Result<(), String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("Cannot read {}: {}", file.display(), e))?;
    if current_from(&text) == Some(mb) {
        return Ok(());
    }
    let new_text = with_xmx(&text, mb)?;
    crate::load_order::ini_parser::write_file_with_backup(file, &new_text, &z_dir.join("PZModStudio_Backups")).map(|_| ())
}

/// Gives the game `mb` of memory from its next start. Refused while it runs (it keeps the launch
/// file open) and outside 2 GB .. half the PC's memory.
pub fn set(pz_install_dir: &str, user_zomboid_dir: &str, mb: u64) -> Result<GameMemory, String> {
    let file = launch_file(pz_install_dir).ok_or("ProjectZomboid64.json was not found in the game folder.")?;
    let z_dir = zomboid_dir(user_zomboid_dir).ok_or("The Zomboid folder was not found.")?;
    let max_mb = status(pz_install_dir, user_zomboid_dir).max_mb;
    if !(MIN_MB..=max_mb).contains(&mb) {
        return Err(format!("{} MB is outside what the Studio gives: {} to {} MB.", mb, MIN_MB, max_mb));
    }
    if crate::sandbox::find_running_pz_pid().is_some() {
        return Err("Close the game first: it keeps its launch file open while it runs.".to_string());
    }
    write(&file, &z_dir, mb)?;
    crate::instance_manager::set_active_memory(&z_dir, mb)?;
    crate::load_order::ini_parser::atomic_write(z_dir.join(CHOSEN_FILE), format!("{}\n", mb)).map_err(|e| e.to_string())?;
    Ok(status(pz_install_dir, user_zomboid_dir))
}

/// Before a start from the Studio: the chosen value back in the launch file, if a game update
/// put its own there.
pub fn reapply_for_launch(install_dir: &Path, z_dir: &Path) {
    let Some(chosen) = read_chosen(z_dir) else { return };
    let file = install_dir.join("ProjectZomboid64.json");
    if !file.is_file() {
        return;
    }
    if let Err(e) = write(&file, z_dir, chosen) {
        crate::diagnostics::error(format!("[Game memory] {}", e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xmx_is_read_in_any_unit() {
        assert_eq!(parse_xmx("-Xmx6144m"), Some(6144));
        assert_eq!(parse_xmx("-Xmx6g"), Some(6144));
        assert_eq!(parse_xmx("-Xmx8G"), Some(8192));
        assert_eq!(parse_xmx("-Xms2g"), None);
        assert_eq!(parse_xmx("-Xmxlots"), None);
    }

    #[test]
    fn only_xmx_changes_and_the_rest_of_the_launch_file_stays() {
        let text = r#"{"mainClass":"x","vmArgs":["-agentlib:zbNative","-Xmx3072m","-Dzomboid.steam=1"],"windows":{"6.1":{"vmArgs":["-XX:+UseG1GC"]}}}"#;
        let out = with_xmx(text, 8192).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["vmArgs"], serde_json::json!(["-agentlib:zbNative", "-Xmx8192m", "-Dzomboid.steam=1"]));
        assert_eq!(v["windows"]["6.1"]["vmArgs"][0], "-XX:+UseG1GC");
        assert_eq!(current_from(&out), Some(8192));
        // No -Xmx at all: added.
        let added = with_xmx(r#"{"vmArgs":["-Dx=1"]}"#, 4096).unwrap();
        assert_eq!(current_from(&added), Some(4096));
        assert!(with_xmx(r#"{"mainClass":"x"}"#, 4096).is_err());
    }

    #[test]
    fn a_start_gives_the_game_the_active_profiles_memory_or_the_last_chosen_or_leaves_it() {
        let root = std::env::temp_dir().join(format!("pzms-mem-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (game, z) = (root.join("game"), root.join("Zomboid"));
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(z.join("PZModStudio_Instances")).unwrap();
        let file = game.join("ProjectZomboid64.json");
        let profile = |mem: &str| {
            std::fs::write(
                z.join("PZModStudio_Instances").join("inst_1.json"),
                format!(r#"{{"id":"inst_1","name":"Main","created_at":"0","is_active":true,"active_mod_ids":[],"load_order":[]{}}}"#, mem),
            )
            .unwrap()
        };
        let xmx = || current_from(&std::fs::read_to_string(&file).unwrap());
        std::fs::write(&file, r#"{"vmArgs":["-Xmx3072m"]}"#).unwrap();

        // Nothing chosen anywhere: the game's own value stays.
        profile("");
        reapply_for_launch(&game, &z);
        assert_eq!(xmx(), Some(3072));
        // The last value chosen, for a profile with none of its own.
        std::fs::write(z.join(CHOSEN_FILE), "4096\n").unwrap();
        reapply_for_launch(&game, &z);
        assert_eq!(xmx(), Some(4096));
        // The profile's own value wins.
        profile(r#","game_memory_mb":8192"#);
        reapply_for_launch(&game, &z);
        assert_eq!(xmx(), Some(8192));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_suggestion_grows_with_the_mods_and_after_running_out_and_never_past_half_the_pc() {
        assert_eq!(recommend(10, Some(3072), false, 16384), 3072);
        assert_eq!(recommend(135, Some(6144), false, 16384), 6144);
        assert_eq!(recommend(250, Some(6144), false, 16384), 8192);
        // Ran out with 6 GB: at least 2 GB more.
        assert_eq!(recommend(135, Some(6144), true, 16384), 8192);
        // An 8 GB PC: 4 GB at most.
        assert_eq!(recommend(250, Some(3072), true, 4096), 4096);
    }
}
