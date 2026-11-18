//! Isolated profiles: a profile kept apart from the others (the field is still `own_saves`).
//!
//! An isolated profile has its own single-player saves, its own game options and its own mod
//! settings. Everything else in the Zomboid folder is the same for every profile: above all the
//! mods themselves, so a mod (or one of its sub-mods) on in one profile and off in another needs
//! nothing special.
//!
//! **Saves.** The game reads its single-player saves from `Zomboid/Saves/<mode>/<world>`. The worlds
//! of the profile that is active are in `Zomboid/Saves`, and every other set waits in
//! `Zomboid/PZModStudio_ProfileSaves/<set>/Saves/<mode>/<world>` (`shared` for the profiles that are
//! not isolated). The multiplayer folder (`Saves/Multiplayer`: servers' worlds, joined servers) is
//! never moved; `latestSave.ini` (the game's Continue) goes with the set.
//!
//! **Settings.** What the game and the mods write for the player, waiting in
//! `PZModStudio_ProfileSaves/<set>/Settings/` (the same names, `Lua/<name>` for the ones in
//! `Zomboid/Lua`). The rule is a SHARED list and "everything else is the profile's":
//! - at the root of `Zomboid`, shared: `Saves` (moved world by world, above), `Lua` (entry by
//!   entry, below), `mods`, `mods_backup`, `Logs`, `console.txt`, `server-console.txt`,
//!   `coop-console.txt`, `Server`, `db`, `backups`, `Workshop`, `logs.zip`, `version.txt`,
//!   `mods.txt`, `saved_modlists`, `Sandbox Presets`, `ModListData.ini`, `latestSave.ini`,
//!   `Screenshots`, `statistic`, `Recording`;
//! - in `Zomboid/Lua`, shared: `mods`, `saved_modlists`, `ModListData.ini`, `loadorder.ini`,
//!   `pz_modlist_settings.cfg` (the game's favourites, kept in step with the app's), the Claude
//!   Agent's `claude_cmd.lua` / `claude_out.txt` / `claude_log.txt`, and the modding-alerts file the
//!   app reads (`mod_changelog::SEEN_IN_GAME_FILE`);
//! - anywhere, shared: names starting `PZModStudio_` (the Studio's and the Link's files) or `pz_`
//!   (the bridge, the profiler, the dev reload, the server's players), caches (a name with `cache`
//!   in it, and the known big ones in `CACHES`), logs (`*.log`), the app's temporary files
//!   (`.pzms-tmp-`) and links (junctions): those point somewhere else and stay where they are.
//!
//! So the profile's: `options.ini`, `InputBindings/`, the game's `Lua/keys*.ini`, `layout.ini`,
//! `screenresolution.ini`, and every other file or folder a mod writes in `Zomboid` or
//! `Zomboid/Lua` (its options, its config, its world data).
//!
//! Every switch moves the worlds and settings there and back with a rename: nothing is copied, and
//! the game, the Monitor, the Link and the backups keep working on the folder in place. A profile
//! that becomes isolated starts with a COPY of the options and mod settings in place (the others
//! keep their own); a set isolated before settings were (saves only) gets that copy the first time
//! it is switched to. Turning isolation off, or deleting the profile, gives its worlds to the
//! shared set and keeps its settings in `PZModStudio_Backups/ProfileSettings/<profile>/<when>/`,
//! never over the shared ones. Something that would be overwritten goes to
//! `PZModStudio_Backups/ProfileSettings/replaced-<when>/` instead: nothing is ever lost.
//!
//! **Big unknown data is never copied.** The names in `CACHES` are only the big folders seen on one
//! PC; any other PC has mods of its own that keep caches, shader dumps, recordings or worlds of
//! their own in `Zomboid`. So the copy made when a profile is isolated skips any single file over
//! `BIG_FILE` (20 MB) and any folder over `BIG_FOLDER` (50 MB) that is not one of the game's known
//! settings (`KNOWN_SETTINGS`, copied whatever their size): such an entry is LEFT SHARED, the same
//! for every profile from then on, and written down in `PZModStudio_ProfileSavesShared.json`
//! (name, size counted, when, for which profile: `left_shared`). From then on it counts as shared
//! like the names above: it is never moved by a switch either. A set that already had its own copy
//! of it (isolated before) gets it put in the replaced backups the next time it comes in, never
//! laid over the one in place.
//!
//! Each switch is written down first (`PZModStudio_ProfileSaves.json`, step out/in); one cut short
//! (the PC went off) is finished the next time the Studio starts or switches.
//!
//! **Refused while Project Zomboid runs**: the game (any `ProjectZomboid64.exe`, player 2 of a
//! multiplayer test too) or any Project Zomboid server (Java running `zombie.network.GameServer`:
//! the Studio's dedicated server, the game's own StartServer64.bat, a co-op host). Either can hold
//! files in `Zomboid` open or write them while they move. A switch cut short is not finished then
//! either: it waits for both to close.
//!
//! **Every move is logged**: each world, Continue file, setting moved, copied, left shared, kept in
//! the backups or thrown away (an unfinished copy) is one line in the Studio's log of that Zomboid
//! folder (`PZModStudio_Logs/studio.log`, `[Profiles]`), from where to where, so a save or a setting
//! that seems missing can always be traced.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const STORE: &str = "PZModStudio_ProfileSaves";
const STATE: &str = "PZModStudio_ProfileSaves.json";
pub const SHARED: &str = "shared";
const KEEP_IN_PLACE: [&str; 1] = ["Multiplayer"];
const LATEST: &str = "latestSave.ini";
/// A set's options and mod settings while another set is in place (its mark: the set has its own).
const SETTINGS: &str = "Settings";
/// A copy being made; only ever a copy, so one left by a cut is thrown away.
const COPYING: &str = "Settings.copying";
const LUA: &str = "Lua";
/// Under PZModStudio_Backups: a profile's settings when it stops being isolated, and anything a
/// move would have overwritten.
const SETTINGS_BACKUPS: &str = "ProfileSettings";

/// Never moved, at the root of Zomboid.
const SHARED_ROOT: &[&str] = &[
    "Saves", // world by world, with the saves
    LUA,     // entry by entry
    "mods",
    "mods_backup",
    "Logs",
    "console.txt",
    "server-console.txt",
    "coop-console.txt",
    "Server",
    "db",
    "backups",
    "Workshop",
    "logs.zip",
    "version.txt",
    "mods.txt",
    "saved_modlists",
    "Sandbox Presets",
    "ModListData.ini",
    LATEST, // with the saves
    "Screenshots",
    "statistic",
    "Recording",
];
/// Never moved, in Zomboid/Lua: the game's mod lists and what the app writes or reads there.
const SHARED_LUA: &[&str] = &[
    "mods",
    "saved_modlists",
    "ModListData.ini",
    "loadorder.ini",
    "pz_modlist_settings.cfg",
    crate::claude_agent::CMD_FILE,
    crate::claude_agent::OUT_FILE,
    "claude_log.txt",
    crate::mod_changelog::SEEN_IN_GAME_FILE,
];
/// Never moved, anywhere: the Studio's and the Link's files, and the app's bridge files.
const SHARED_PREFIXES: &[&str] = &["pzmodstudio_", "pz_"];
/// Big data that is not a setting: above these (a file, a folder), an entry that is not in
/// `KNOWN_SETTINGS` is not copied when a profile is isolated, it is left shared.
const BIG_FILE: u64 = 20 * 1024 * 1024;
const BIG_FOLDER: u64 = 50 * 1024 * 1024;
/// The game's own settings (and the mod options file nearly every list has): always the profile's
/// and always copied, whatever their size. `Lua/keys*.ini` too.
const KNOWN_SETTINGS: &[&str] = &["options.ini", "InputBindings", "layout.ini", "screenresolution.ini", "Lua/layout.ini", "Lua/screenresolution.ini", "Lua/ModOptions.ini"];
/// What was left shared because it was too big to copy (see the module doc).
const LEFT_SHARED: &str = "PZModStudio_ProfileSavesShared.json";
/// Caches and dumps mods keep, big and rebuilt by them: the same for every profile.
const CACHES: &[&str] = &[
    "apf-cellspread.txt",
    "apf-prewarm.txt",
    "apf-hitches.txt",
    "viewpoint-programs",
    "viewpoint-lod",
    "viewpoint-look-blocks",
    "viewpoint-shaderpacks",
    "viewpoint-classes.txt",
    "viewpoint-missing-textures.txt",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct State {
    /// The set whose worlds are in Zomboid/Saves now.
    pub current: String,
    /// A switch under way: (from, to, step): "out" while the active set's worlds go away, "in" while
    /// the other set's come in. A switch cut short goes on from its step.
    #[serde(default)]
    pub moving: Option<(String, String, String)>,
}

fn state_path(z: &Path) -> PathBuf {
    z.join(STATE)
}

pub fn state(z: &Path) -> State {
    let mut s: State = fs::read_to_string(state_path(z)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    if s.current.is_empty() {
        s.current = SHARED.to_string();
    }
    s
}

fn write_state(z: &Path, s: &State) -> Result<(), String> {
    crate::load_order::ini_parser::atomic_write(&state_path(z), serde_json::to_string_pretty(s).unwrap_or_default()).map_err(|e| e.to_string())
}

/// The set a profile's saves belong to.
pub fn set_of(own_saves: bool, profile_id: &str) -> String {
    if own_saves {
        profile_id.to_string()
    } else {
        SHARED.to_string()
    }
}

fn safe(set: &str) -> Result<&str, String> {
    if set.is_empty() || set.contains(['/', '\\', ':']) || set.contains("..") {
        return Err(format!("\"{}\" is not a saves set", set));
    }
    Ok(set)
}

/// Where a set waits while another is active.
pub fn store_of(z: &Path, set: &str) -> Result<PathBuf, String> {
    Ok(z.join(STORE).join(safe(set)?))
}

/// A free name for a world in a folder: the same, or `<world>_2`, `_3`...
fn free_name(dir: &Path, world: &str) -> PathBuf {
    let first = dir.join(world);
    if !first.exists() {
        return first;
    }
    (2..).map(|n| dir.join(format!("{}_{}", world, n))).find(|p| !p.exists()).unwrap_or(first)
}

/// Moves every single-player world of `from` (a saves folder) into `to`, mode by mode.
fn move_worlds(z: &Path, from: &Path, to: &Path) -> Result<usize, String> {
    let mut moved = 0;
    let Ok(modes) = fs::read_dir(from) else { return Ok(0) };
    for mode in modes.flatten().filter(|e| e.path().is_dir()) {
        let mode_name = mode.file_name().to_string_lossy().to_string();
        if KEEP_IN_PLACE.iter().any(|k| k.eq_ignore_ascii_case(&mode_name)) {
            continue;
        }
        let Ok(worlds) = fs::read_dir(mode.path()) else { continue };
        for world in worlds.flatten().filter(|e| e.path().is_dir()) {
            let dest_dir = to.join(&mode_name);
            fs::create_dir_all(&dest_dir).map_err(|e| format!("Cannot make {}: {}", dest_dir.display(), e))?;
            let dest = free_name(&dest_dir, &world.file_name().to_string_lossy());
            fs::rename(world.path(), &dest).map_err(|e| format!("Cannot move {} to {}: {}", world.path().display(), dest.display(), e))?;
            note(z, format!("moved world {} -> {}", world.path().display(), dest.display()));
            moved += 1;
        }
    }
    Ok(moved)
}

/// Moves the Continue file (`latestSave.ini`), the one already at `to` replaced.
fn move_file(z: &Path, from: &Path, to: &Path) {
    if from.is_file() {
        if let Some(dir) = to.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let replaced = to.is_file();
        let _ = fs::remove_file(to);
        match fs::rename(from, to) {
            Ok(()) => note(z, format!("moved {} -> {}{}", from.display(), to.display(), if replaced { " (replacing the one there)" } else { "" })),
            Err(e) => note(z, format!("could not move {} -> {}: {}", from.display(), to.display(), e)),
        }
    }
}

/// One line in the Studio's log of this Zomboid folder: what moved from where to where.
fn note(z: &Path, text: String) {
    crate::diagnostics::log_in(z, "INFO", &format!("[Profiles] {}", text));
}

#[cfg(test)]
thread_local! {
    /// What the unit tests pretend runs (the reason it gives), on their own thread only.
    static TEST_BUSY: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Why saves and settings cannot be moved now: the game or a Project Zomboid server runs. Never
/// the real processes in the unit tests: they work on folders of their own in the temp folder,
/// whatever this PC's game is doing (they set `TEST_BUSY` to try a refusal).
fn busy() -> Option<String> {
    #[cfg(test)]
    {
        TEST_BUSY.with(|b| b.borrow().clone())
    }
    #[cfg(not(test))]
    {
        // The game first: one tasklist. The servers' command lines only when Java runs.
        if let Some(pid) = crate::sandbox::find_running_pz_pid() {
            return refusal(Some(pid), &[]);
        }
        refusal(None, &crate::server_manager::mp_test::running_servers())
    }
}

/// The refusal for the game's process (if it runs) or the first server found.
fn refusal(game: Option<u32>, servers: &[(u32, String)]) -> Option<String> {
    const WHY: &str = "the saves, game options and mod settings of isolated profiles are moved only while no Project Zomboid game or server runs, so nothing is moved while one of them has the files open.";
    if let Some(pid) = game {
        return Some(format!("Close Project Zomboid first (process {}): {}", pid, WHY));
    }
    let (pid, name) = servers.first()?;
    let which = if name.is_empty() { "A Project Zomboid server".to_string() } else { format!("The Project Zomboid server \"{}\"", name) };
    Some(format!("{} is running (process {}). Stop it first (Servers screen, or its console): {}", which, pid, WHY))
}

fn refuse_if_busy() -> Result<(), String> {
    match busy() {
        Some(why) => Err(why),
        None => Ok(()),
    }
}

// ---- Settings: the game's options and what mods save -------------------------------------------

/// Whether a name in Zomboid (`in_lua`: in Zomboid/Lua) is the same for every profile.
pub fn is_shared(name: &str, in_lua: bool) -> bool {
    let lower = name.to_ascii_lowercase();
    let list = if in_lua { SHARED_LUA } else { SHARED_ROOT };
    list.iter().any(|s| s.eq_ignore_ascii_case(name))
        || SHARED_PREFIXES.iter().any(|p| lower.starts_with(p))
        || CACHES.iter().any(|c| c.eq_ignore_ascii_case(name))
        || lower.contains("cache")
        || lower.ends_with(".log")
        || lower.contains(".pzms-tmp-")
}

/// A link (symlink or junction): it points somewhere else, and stays where it is.
fn is_link(p: &Path) -> bool {
    fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(false)
}

/// The entries of a folder laid out like Zomboid (the root, and `Lua/<name>` for the ones in its Lua
/// folder), each with whether it is shared: by name, or left shared in `z` (too big to copy).
fn entries(z: &Path, root: &Path) -> Vec<(String, bool)> {
    let left: Vec<String> = left_shared(z).into_iter().map(|l| l.rel.to_ascii_lowercase()).collect();
    let mut out = Vec::new();
    let Ok(top) = fs::read_dir(root) else { return out };
    for e in top.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.eq_ignore_ascii_case(LUA) && e.path().is_dir() && !is_link(&e.path()) {
            if let Ok(lua) = fs::read_dir(e.path()) {
                for l in lua.flatten() {
                    let n = l.file_name().to_string_lossy().to_string();
                    let rel = format!("{}/{}", name, n);
                    let shared = is_shared(&n, true) || is_link(&l.path()) || left.contains(&rel.to_ascii_lowercase());
                    out.push((rel, shared));
                }
            }
            continue;
        }
        let shared = is_shared(&name, false) || is_link(&e.path()) || left.contains(&name.to_ascii_lowercase());
        out.push((name, shared));
    }
    out.sort();
    out
}

/// The options and mod settings in place now: the active set's (`options.ini`, `Lua/keysB42.ini`...).
pub fn settings_in_place(z: &Path) -> Vec<String> {
    entries(z, z).into_iter().filter(|(_, shared)| !shared).map(|(rel, _)| rel).collect()
}

/// One entry left shared because it was too big to copy when a profile was isolated.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LeftShared {
    /// As in Zomboid: `name`, or `Lua/name`.
    pub rel: String,
    /// Its size as counted (a folder is counted until it is over the limit: at least this).
    pub bytes: u64,
    pub when: String,
    /// The profile being isolated then.
    pub profile: String,
}

/// What was left shared because it was too big to copy (the screen and the log can say it).
pub fn left_shared(z: &Path) -> Vec<LeftShared> {
    fs::read_to_string(z.join(LEFT_SHARED)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn add_left_shared(z: &Path, more: &[LeftShared]) -> Result<(), String> {
    if more.is_empty() {
        return Ok(());
    }
    let mut all = left_shared(z);
    for m in more {
        if !all.iter().any(|a| a.rel.eq_ignore_ascii_case(&m.rel)) {
            all.push(m.clone());
        }
    }
    crate::load_order::ini_parser::atomic_write(&z.join(LEFT_SHARED), serde_json::to_string_pretty(&all).unwrap_or_default()).map_err(|e| e.to_string())
}

/// One of the game's own settings: copied whatever its size.
fn is_known_setting(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    KNOWN_SETTINGS.iter().any(|k| k.eq_ignore_ascii_case(rel)) || (lower.starts_with("lua/keys") && lower.ends_with(".ini"))
}

/// The size of a file, or of a folder's files (links not followed), counted until it is over
/// `limit`: `Some(counted)` when it is over, `None` when it is not.
fn over(path: &Path, limit: u64) -> Option<u64> {
    fn walk(p: &Path, total: &mut u64, limit: u64) {
        if *total > limit || is_link(p) {
            return;
        }
        let Ok(meta) = fs::metadata(p) else { return };
        if meta.is_file() {
            *total += meta.len();
            return;
        }
        if let Ok(rd) = fs::read_dir(p) {
            for e in rd.flatten() {
                walk(&e.path(), total, limit);
                if *total > limit {
                    return;
                }
            }
        }
    }
    let mut total = 0;
    walk(path, &mut total, limit);
    (total > limit).then_some(total)
}

/// The limit an entry is held to: `BIG_FOLDER` for a folder, `BIG_FILE` for a file.
fn limit_of(p: &Path) -> u64 {
    if p.is_dir() {
        BIG_FOLDER
    } else {
        BIG_FILE
    }
}

/// Too big to copy: a file over `BIG_FILE` or a folder over `BIG_FOLDER` that is not a known
/// setting. Its size when it is.
fn too_big_to_copy(z: &Path, rel: &str) -> Option<u64> {
    if is_known_setting(rel) {
        return None;
    }
    let p = z.join(rel);
    over(&p, limit_of(&p))
}

fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn settings_backups(z: &Path) -> PathBuf {
    z.join("PZModStudio_Backups").join(SETTINGS_BACKUPS)
}

/// Moves `rel` from under `from` to under `to`. Something already at the destination is kept in
/// the replaced backups, never overwritten.
fn put(z: &Path, from: &Path, to: &Path, rel: &str) -> Result<(), String> {
    let src = from.join(rel);
    let dest = to.join(rel);
    if fs::symlink_metadata(&dest).is_ok() {
        keep_replaced(z, &dest, rel)?;
    }
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("Cannot make {}: {}", dir.display(), e))?;
    }
    fs::rename(&src, &dest).map_err(|e| format!("Cannot move {} to {}: {}", src.display(), dest.display(), e))?;
    note(z, format!("moved setting {} -> {}", src.display(), dest.display()));
    Ok(())
}

/// Puts `path` (known as `rel`) in the replaced backups.
fn keep_replaced(z: &Path, path: &Path, rel: &str) -> Result<(), String> {
    let base = settings_backups(z).join(format!("replaced-{}", stamp()));
    let rel_path = Path::new(rel);
    let dir = rel_path.parent().map(|p| base.join(p)).unwrap_or_else(|| base.clone());
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot make {}: {}", dir.display(), e))?;
    let name = rel_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| rel.to_string());
    let keep = free_name(&dir, &name);
    fs::rename(path, &keep).map_err(|e| format!("Cannot keep {} in {}: {}", path.display(), keep.display(), e))?;
    note(z, format!("kept in the backups (it was in the way): {} -> {}", path.display(), keep.display()));
    Ok(())
}

/// Removes the empty folders of a tree, the tree too when it ends up empty.
fn remove_empty(dir: &Path) {
    if is_link(dir) || !dir.is_dir() {
        return;
    }
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                remove_empty(&e.path());
            }
        }
    }
    let _ = fs::remove_dir(dir);
}

fn has_settings(z: &Path, set: &str) -> Result<bool, String> {
    Ok(store_of(z, set)?.join(SETTINGS).is_dir())
}

/// The active set's options and mod settings go away into `set`'s store (made even when there is
/// nothing to move: it marks the set as having settings of its own).
fn settings_out(z: &Path, set: &str) -> Result<(), String> {
    let store = store_of(z, set)?.join(SETTINGS);
    fs::create_dir_all(&store).map_err(|e| format!("Cannot make {}: {}", store.display(), e))?;
    for rel in settings_in_place(z) {
        put(z, z, &store, &rel)?;
    }
    Ok(())
}

/// `set`'s options and mod settings come into place. One whose name is shared by now (a newer list)
/// is kept in the replaced backups rather than laid over the shared one.
fn settings_in(z: &Path, set: &str) -> Result<(), String> {
    let store = store_of(z, set)?.join(SETTINGS);
    if !store.is_dir() {
        return Ok(());
    }
    for (rel, shared) in entries(z, &store) {
        if shared {
            keep_replaced(z, &store.join(&rel), &rel)?;
        } else {
            put(z, &store, z, &rel)?;
        }
    }
    remove_empty(&store);
    Ok(())
}

fn copy_any(from: &Path, to: &Path) -> std::io::Result<()> {
    if is_link(from) {
        return Ok(());
    }
    if from.is_dir() {
        fs::create_dir_all(to)?;
        for e in fs::read_dir(from)?.flatten() {
            copy_any(&e.path(), &to.join(e.file_name()))?;
        }
        Ok(())
    } else {
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::copy(from, to).map(|_| ())
    }
}

/// `set` gets a copy of the options and mod settings in place now (the set in place keeps its
/// own). Made aside and put in with one rename; what `set` had before goes to the backups. Big
/// unknown data is not copied but left shared (see the module doc); returns what was.
fn copy_settings_to(z: &Path, set: &str) -> Result<Vec<LeftShared>, String> {
    let base = store_of(z, set)?;
    let staging = base.join(COPYING);
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|e| format!("Cannot make {}: {}", staging.display(), e))?;
    let in_place = settings_in_place(z);
    // Written down before anything is copied: from now on they count as shared.
    let left: Vec<LeftShared> = in_place
        .iter()
        .filter_map(|rel| too_big_to_copy(z, rel).map(|bytes| LeftShared { rel: rel.clone(), bytes, when: stamp(), profile: set.to_string() }))
        .collect();
    add_left_shared(z, &left)?;
    for l in &left {
        let p = z.join(&l.rel);
        note(z, format!("left shared, not copied for {} (over {} MB and not a known setting): {} ({} bytes counted)", set, limit_of(&p) / (1024 * 1024), p.display(), l.bytes));
    }
    for rel in in_place.iter().filter(|r| !left.iter().any(|l| &l.rel == *r)) {
        copy_any(&z.join(rel), &staging.join(rel)).map_err(|e| format!("Cannot copy {}: {}", rel, e))?;
        note(z, format!("copied setting for {}: {} -> {}", set, z.join(rel).display(), staging.join(rel).display()));
    }
    let dest = base.join(SETTINGS);
    if dest.exists() {
        keep_replaced(z, &dest, &format!("{}-{}", set, SETTINGS))?;
    }
    fs::rename(&staging, &dest).map_err(|e| format!("Cannot put the copy in {}: {}", dest.display(), e))?;
    note(z, format!("the copy of the settings for {} is in place: {}", set, dest.display()));
    Ok(left)
}

/// A profile's settings and Continue file, out of its store and into
/// `PZModStudio_Backups/ProfileSettings/<profile>/<when>/`: kept, not laid over the shared ones.
fn keep_aside(z: &Path, profile_id: &str) -> Result<Option<PathBuf>, String> {
    let store = store_of(z, profile_id)?;
    let _ = fs::remove_dir_all(store.join(COPYING));
    let settings = store.join(SETTINGS);
    let latest = store.join(LATEST);
    let mut kept = None;
    if settings.is_dir() || latest.is_file() {
        let dir = settings_backups(z).join(profile_id);
        fs::create_dir_all(&dir).map_err(|e| format!("Cannot make {}: {}", dir.display(), e))?;
        let dest = free_name(&dir, &stamp());
        if settings.is_dir() {
            fs::rename(&settings, &dest).map_err(|e| format!("Cannot keep {} in {}: {}", settings.display(), dest.display(), e))?;
            note(z, format!("settings of {} kept in the backups: {} -> {}", profile_id, settings.display(), dest.display()));
        }
        move_file(z, &latest, &dest.join(LATEST));
        kept = Some(dest);
    }
    remove_empty(&store);
    Ok(kept)
}

/// Finishes a switch cut short, if one was, and brings back anything of the set in place that was
/// left put away (a take or give back cut short): its worlds, and the settings not in place (the
/// ones in place win; a leftover copy of them goes to the backups). Nothing while the game runs.
pub fn recover(z: &Path) -> Result<(), String> {
    if busy().is_some() {
        return Ok(());
    }
    recover_now(z)
}

/// Whether `recover` has anything to do for the set in place: worlds or settings of its own left
/// put away, or a copy cut short.
fn leftovers(z: &Path, current: &str) -> bool {
    let away = store_of(z, current).ok();
    let has_world = |saves: &Path| fs::read_dir(saves).into_iter().flatten().flatten().any(|m| fs::read_dir(m.path()).into_iter().flatten().flatten().any(|w| w.path().is_dir()));
    let left = away.as_ref().is_some_and(|a| has_world(&a.join("Saves")) || a.join(SETTINGS).is_dir());
    left || fs::read_dir(z.join(STORE)).into_iter().flatten().flatten().any(|s| s.path().join(COPYING).is_dir())
}

/// `recover` once nothing runs.
fn recover_now(z: &Path) -> Result<(), String> {
    let s = state(z);
    if let Some((from, to, step)) = s.moving.clone() {
        note(z, format!("finishing a switch cut short: {} -> {} (step {})", from, to, step));
        run_switch_from(z, &from, &to, &step)?;
    }
    // Copies being made when the PC went off: only copies.
    if let Ok(sets) = fs::read_dir(z.join(STORE)) {
        for set in sets.flatten() {
            let copying = set.path().join(COPYING);
            if copying.is_dir() {
                let _ = fs::remove_dir_all(&copying);
                note(z, format!("threw away an unfinished copy of settings (only a copy): {}", copying.display()));
            }
        }
    }
    let s = state(z);
    if s.moving.is_none() {
        let away = store_of(z, &s.current)?.join("Saves");
        if away.is_dir() {
            move_worlds(z, &away, &z.join("Saves"))?;
        }
        let left = store_of(z, &s.current)?.join(SETTINGS);
        if left.is_dir() {
            for (rel, shared) in entries(z, &left) {
                if shared || fs::symlink_metadata(z.join(&rel)).is_ok() {
                    keep_replaced(z, &left.join(&rel), &rel)?;
                } else {
                    put(z, &left, z, &rel)?;
                }
            }
            remove_empty(&left);
        }
    }
    Ok(())
}

fn run_switch(z: &Path, from: &str, to: &str) -> Result<(), String> {
    // A set isolated before settings were (saves only) starts with a copy of the ones in place.
    if !has_settings(z, to)? {
        copy_settings_to(z, to)?;
    }
    note(z, format!("switch {} -> {}", from, to));
    run_switch_from(z, from, to, "out")?;
    note(z, format!("switch {} -> {} done", from, to));
    Ok(())
}

fn run_switch_from(z: &Path, from: &str, to: &str, step: &str) -> Result<(), String> {
    let saves = z.join("Saves");
    let from_store = store_of(z, from)?;
    let to_store = store_of(z, to)?;
    let record = |step: &str| write_state(z, &State { current: from.to_string(), moving: Some((from.to_string(), to.to_string(), step.to_string())) });
    // Each step is recorded before it moves anything: a cut is finished by doing that step again.
    if step == "out" {
        record("out")?;
        move_worlds(z, &saves, &from_store.join("Saves"))?;
        move_file(z, &z.join(LATEST), &from_store.join(LATEST));
        settings_out(z, from)?;
    }
    record("in")?;
    move_worlds(z, &to_store.join("Saves"), &saves)?;
    move_file(z, &to_store.join(LATEST), &z.join(LATEST));
    settings_in(z, to)?;
    write_state(z, &State { current: to.to_string(), moving: None })
}

/// Puts the worlds and settings of `set` in place and the others away. Refused while the game runs.
pub fn switch_to(z: &Path, set: &str) -> Result<bool, String> {
    safe(set)?;
    // Nothing to move and nothing left over: no process check (every activation comes here).
    let s = state(z);
    if s.current == set && s.moving.is_none() && !leftovers(z, &s.current) {
        return Ok(false);
    }
    let busy = busy();
    if busy.is_none() {
        recover_now(z)?;
    }
    let s = state(z);
    if s.current == set && s.moving.is_none() {
        return Ok(false);
    }
    if let Some(why) = busy {
        return Err(why);
    }
    run_switch(z, &s.current, set)?;
    Ok(true)
}

/// The worlds a set has (mode/world), wherever they are now.
pub fn worlds_of(z: &Path, set: &str) -> Vec<String> {
    let dir = if state(z).current == set { z.join("Saves") } else { store_of(z, set).map(|d| d.join("Saves")).unwrap_or_default() };
    let mut out = Vec::new();
    let Ok(modes) = fs::read_dir(&dir) else { return out };
    for mode in modes.flatten().filter(|e| e.path().is_dir()) {
        let m = mode.file_name().to_string_lossy().to_string();
        if KEEP_IN_PLACE.iter().any(|k| k.eq_ignore_ascii_case(&m)) {
            continue;
        }
        if let Ok(worlds) = fs::read_dir(mode.path()) {
            out.extend(worlds.flatten().filter(|e| e.path().is_dir()).map(|w| format!("{}/{}", m, w.file_name().to_string_lossy())));
        }
    }
    out.sort();
    out
}

/// Whether a save (mode/world) waits in another set: not lost, only put away.
pub fn is_put_away(z: &Path, save_id: &str) -> bool {
    let Ok(sets) = fs::read_dir(z.join(STORE)) else { return false };
    sets.flatten().any(|s| s.path().join("Saves").join(save_id).is_dir())
}

/// Moves worlds of the active set into another set (a profile's own, or `shared`): a save taken to
/// a profile. Refused while the game runs.
pub fn move_saves(z: &Path, save_ids: &[String], to_set: &str) -> Result<usize, String> {
    safe(to_set)?;
    refuse_if_busy()?;
    recover_now(z)?;
    let s = state(z);
    if s.current == to_set {
        return Ok(0);
    }
    let dest = store_of(z, to_set)?.join("Saves");
    let mut moved = 0;
    for id in save_ids {
        let Some((mode, world)) = id.split_once('/') else { continue };
        if KEEP_IN_PLACE.iter().any(|k| k.eq_ignore_ascii_case(mode)) || world.contains(['/', '\\']) || world.contains("..") {
            continue;
        }
        let src = z.join("Saves").join(mode).join(world);
        if !src.is_dir() {
            continue;
        }
        let dir = dest.join(mode);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dest = free_name(&dir, world);
        fs::rename(&src, &dest).map_err(|e| format!("Cannot move {}: {}", id, e))?;
        note(z, format!("moved world {} -> {} (taken to {})", src.display(), dest.display(), to_set));
        moved += 1;
    }
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pzms-psaves-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn world(z: &Path, mode: &str, name: &str) {
        let d = z.join("Saves").join(mode).join(name);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("map_ver.bin"), name).unwrap();
    }

    #[test]
    fn switching_puts_each_profiles_worlds_in_place_and_leaves_multiplayer_alone() {
        let z = tmp("switch");
        world(&z, "Sandbox", "Shared World");
        world(&z, "Multiplayer", "servertest");
        fs::write(z.join(LATEST), "Sandbox/Shared World").unwrap();
        // To a profile with saves of its own: the shared world goes away, the server's stays.
        assert!(run_switch(&z, SHARED, "p1").is_ok());
        assert!(!z.join("Saves/Sandbox/Shared World").exists());
        assert!(z.join("Saves/Multiplayer/servertest").is_dir());
        assert!(z.join(STORE).join("shared/Saves/Sandbox/Shared World/map_ver.bin").is_file());
        assert!(!z.join(LATEST).exists(), "Continue belongs to the shared set");
        // The profile's own world, then back: each set gets its own.
        world(&z, "Sandbox", "P1 World");
        assert!(run_switch(&z, "p1", SHARED).is_ok());
        assert!(z.join("Saves/Sandbox/Shared World").is_dir() && !z.join("Saves/Sandbox/P1 World").exists());
        assert_eq!(fs::read_to_string(z.join(LATEST)).unwrap(), "Sandbox/Shared World");
        assert_eq!(worlds_of(&z, "p1"), vec!["Sandbox/P1 World".to_string()]);
        assert!(is_put_away(&z, "Sandbox/P1 World"));
        assert_eq!(state(&z).current, SHARED);
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn a_switch_cut_short_is_finished() {
        let z = tmp("recover");
        world(&z, "Sandbox", "A");
        write_state(&z, &State { current: SHARED.into(), moving: Some((SHARED.into(), "p2".into(), "out".into())) }).unwrap();
        recover(&z).unwrap();
        assert_eq!(state(&z), State { current: "p2".into(), moving: None });
        assert!(z.join(STORE).join("shared/Saves/Sandbox/A").is_dir());
        // Cut while the other set's worlds came in: those already in stay in.
        let p2_store = z.join(STORE).join("p2/Saves/Sandbox");
        fs::create_dir_all(p2_store.join("B")).unwrap();
        world(&z, "Sandbox", "C");
        write_state(&z, &State { current: "p2".into(), moving: Some(("p2".into(), "p4".into(), "in".into())) }).unwrap();
        fs::create_dir_all(z.join(STORE).join("p4/Saves/Sandbox/D")).unwrap();
        recover(&z).unwrap();
        assert!(z.join("Saves/Sandbox/C").is_dir() && z.join("Saves/Sandbox/D").is_dir(), "C came in before the cut, D after");
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn a_world_of_the_same_name_is_not_overwritten() {
        let z = tmp("names");
        world(&z, "Sandbox", "Same");
        let store = z.join(STORE).join("shared/Saves/Sandbox/Same");
        fs::create_dir_all(&store).unwrap();
        run_switch(&z, SHARED, "p3").unwrap();
        assert!(z.join(STORE).join("shared/Saves/Sandbox/Same_2").is_dir());
        let _ = fs::remove_dir_all(&z);
    }
}

/// A profile becomes isolated. `bring` (worlds now in Zomboid/Saves of the shared set) go with it;
/// the rest stay shared. It starts with a copy of the options and mod settings in place now: the
/// set in place keeps its own. When it is the active profile its worlds and settings are the ones
/// left in place, and the shared set takes the copy.
pub fn take_own(z: &Path, profile_id: &str, active: bool, bring: &[String]) -> Result<Vec<LeftShared>, String> {
    safe(profile_id)?;
    refuse_if_busy()?;
    recover_now(z)?;
    note(z, format!("isolating profile {}{}, bringing {:?}", profile_id, if active { " (the active one)" } else { "" }, bring));
    let s = state(z);
    if s.current != SHARED {
        if bring.is_empty() && !active {
            // Nothing to bring: the profile starts with no saves, and the settings in place copied.
            return copy_settings_to(z, profile_id);
        }
        return Err("Switch to a profile that is not isolated first: the saves to bring are the shared ones.".to_string());
    }
    let saves = z.join("Saves");
    if active {
        // The copy first: cut short before the state, recover() finds it beside the ones in place
        // and keeps it in the backups.
        let left = copy_settings_to(z, SHARED)?;
        // Everything but what it brings goes to the shared set, put away; its own stay in place.
        // The state first: cut short, a shared world left behind stays in sight, never hidden.
        let in_place = worlds_of(z, SHARED);
        write_state(z, &State { current: profile_id.to_string(), moving: None })?;
        let shared = store_of(z, SHARED)?.join("Saves");
        let keep: std::collections::HashSet<&str> = bring.iter().map(String::as_str).collect();
        for id in in_place {
            if keep.contains(id.as_str()) {
                continue;
            }
            let (mode, world) = id.split_once('/').unwrap_or(("", ""));
            let dir = shared.join(mode);
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let (src, dest) = (saves.join(mode).join(world), free_name(&dir, world));
            fs::rename(&src, &dest).map_err(|e| format!("Cannot move {}: {}", id, e))?;
            note(z, format!("moved world {} -> {} (stays with the shared profiles)", src.display(), dest.display()));
        }
        move_file(z, &z.join(LATEST), &store_of(z, SHARED)?.join(LATEST));
        Ok(left)
    } else {
        let left = copy_settings_to(z, profile_id)?;
        move_saves(z, bring, profile_id)?;
        Ok(left)
    }
}

/// A profile stops being isolated (or is deleted): its worlds become shared ones, and its options
/// and mod settings are kept in `PZModStudio_Backups/ProfileSettings/<profile>/<when>/` while the
/// shared ones are the ones in place. Nothing is lost. Returns where its settings were kept.
pub fn give_back(z: &Path, profile_id: &str) -> Result<Option<PathBuf>, String> {
    safe(profile_id)?;
    refuse_if_busy()?;
    recover_now(z)?;
    note(z, format!("ending the isolation of profile {} (its worlds become shared, its settings go to the backups)", profile_id));
    let s = state(z);
    if s.current == profile_id {
        // A shared set from before settings were (saves only) takes a copy of these: they were
        // everyone's then.
        if !has_settings(z, SHARED)? {
            copy_settings_to(z, SHARED)?;
        }
        // Its settings go into its store first: cut short, recover() brings them back in.
        settings_out(z, profile_id)?;
        // Its worlds are in place: they become the shared ones, with the shared set brought back.
        // The state first: cut short, recover() brings the rest of the shared set in.
        write_state(z, &State { current: SHARED.to_string(), moving: None })?;
        move_worlds(z, &store_of(z, SHARED)?.join("Saves"), &z.join("Saves"))?;
        settings_in(z, SHARED)?;
        let latest_away = store_of(z, SHARED)?.join(LATEST);
        if !z.join(LATEST).exists() {
            move_file(z, &latest_away, &z.join(LATEST));
        }
    } else {
        let target = if s.current == SHARED { z.join("Saves") } else { store_of(z, SHARED)?.join("Saves") };
        move_worlds(z, &store_of(z, profile_id)?.join("Saves"), &target)?;
    }
    keep_aside(z, profile_id)
}

#[cfg(test)]
mod own_tests {
    use super::*;

    #[test]
    fn a_profile_takes_some_saves_and_gives_them_back() {
        let z = std::env::temp_dir().join(format!("pzms-psaves-own-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        for w in ["Mine", "Other"] {
            fs::create_dir_all(z.join("Saves/Sandbox").join(w)).unwrap();
        }
        take_own(&z, "p9", true, &["Sandbox/Mine".to_string()]).unwrap();
        assert_eq!(state(&z).current, "p9");
        assert!(z.join("Saves/Sandbox/Mine").is_dir() && !z.join("Saves/Sandbox/Other").exists());
        assert_eq!(worlds_of(&z, SHARED), vec!["Sandbox/Other".to_string()]);
        give_back(&z, "p9").unwrap();
        assert_eq!(state(&z).current, SHARED);
        assert!(z.join("Saves/Sandbox/Mine").is_dir() && z.join("Saves/Sandbox/Other").is_dir());
        let _ = fs::remove_dir_all(&z);
    }
}

#[cfg(test)]
mod settings_tests {
    use super::*;

    /// What is never moved, with what it holds: checked unchanged after every step.
    const SHARED_ITEMS: &[&str] = &[
        "mods/ModA/mod.info",
        "Logs/session.txt",
        "console.txt",
        "server-console.txt",
        "Server/servertest.ini",
        "db/servertest.db",
        "backups/b.zip",
        "Workshop/item.txt",
        "logs.zip",
        "version.txt",
        "mods.txt",
        "saved_modlists/list.txt",
        "Sandbox Presets/preset.cfg",
        "ModListData.ini",
        "PZModStudio_Learned.json",
        "PZModStudio_Backups/profiles/x.json",
        "apf-texcache/t.bin",
        "viewpoint-programs/p.program",
        "Saves/Multiplayer/servertest/map_ver.bin",
        "Lua/PZModStudio_Link.txt",
        "Lua/pz_ipc_queue.json",
        "Lua/ModListData.ini",
        "Lua/loadorder.ini",
        "Lua/mods/ModA/x.txt",
        "Lua/saved_modlists/l.txt",
        "Lua/pz_modlist_settings.cfg",
        "Lua/claude_out.txt",
        "Lua/apf-hitches.txt",
        "ItemTracker.log",
    ];
    /// The profile's: options, keys, what mods save at the root and in Lua.
    const PROFILE_ITEMS: &[&str] = &[
        "options.ini",
        "InputBindings/bind.txt",
        "Lua/keysB42.ini",
        "Lua/ModOptions.ini",
        "Lua/KnoxLives_world/world.json",
        "ViewpointRTX/rtx.properties",
        "SmoothFrames.properties",
    ];

    fn zomboid(tag: &str) -> PathBuf {
        let z = std::env::temp_dir().join(format!("pzms-psettings-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&z);
        for rel in SHARED_ITEMS {
            write(&z, rel, &format!("shared item {}", rel));
        }
        for rel in PROFILE_ITEMS {
            write(&z, rel, "shared");
        }
        write(&z, "Saves/Sandbox/Shared World/map_ver.bin", "Shared World");
        z
    }

    fn write(z: &Path, rel: &str, text: &str) {
        let p = z.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn read(z: &Path, rel: &str) -> String {
        fs::read_to_string(z.join(rel)).unwrap_or_else(|_| "<none>".into())
    }

    fn shared_untouched(z: &Path) {
        for rel in SHARED_ITEMS {
            assert_eq!(read(z, rel), format!("shared item {}", rel), "{} never moves", rel);
        }
    }

    /// Every file under a folder, relative, sorted.
    fn files(dir: &Path) -> Vec<String> {
        fn walk(base: &Path, d: &Path, out: &mut Vec<String>) {
            for e in fs::read_dir(d).into_iter().flatten().flatten() {
                if e.path().is_dir() {
                    walk(base, &e.path(), out);
                } else {
                    out.push(e.path().strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"));
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, dir, &mut out);
        out.sort();
        out
    }

    /// Moves a file or folder under `z`, making the destination's folder.
    fn mv(z: &Path, from: &str, to: &Path) {
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::rename(z.join(from), to).unwrap();
    }

    #[test]
    fn what_is_shared_and_what_is_the_profiles() {
        let z = zomboid("lists");
        let mut want: Vec<String> = PROFILE_ITEMS.iter().map(|r| r.split('/').take(if r.starts_with("Lua/") { 2 } else { 1 }).collect::<Vec<_>>().join("/")).collect();
        want.sort();
        want.dedup();
        assert_eq!(settings_in_place(&z), want);
        assert!(is_shared("PZModStudio_Anything.json", false) && is_shared("pzmodstudio_x", true));
        assert!(is_shared("sandbox presets", false) && is_shared("Some-Cache", false) && is_shared(".options.ini.pzms-tmp-1-2", false));
        assert!(!is_shared("options.ini", false) && !is_shared("InputBindings", false) && !is_shared("ModOptions.ini", true));
        assert!(!is_shared("mods.txt", true), "only the root's mods.txt is the game's list");
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn switching_both_ways_gives_each_set_its_own_settings() {
        let z = zomboid("switch");
        take_own(&z, "p1", false, &[]).unwrap();
        // Copied, not moved: the shared set keeps its own in place.
        let p1 = z.join(STORE).join("p1").join(SETTINGS);
        for rel in PROFILE_ITEMS {
            assert_eq!(read(&z, rel), "shared");
            assert_eq!(read(&p1, rel), "shared", "{} copied", rel);
        }
        assert!(!p1.join("mods").exists() && !p1.join("Lua/PZModStudio_Link.txt").exists());

        assert!(switch_to(&z, "p1").unwrap());
        shared_untouched(&z);
        assert_eq!(read(&z, "options.ini"), "shared", "starts with the copy");
        assert!(!z.join("Saves/Sandbox/Shared World").exists());
        // Changed while it is on, and a mod writes something new.
        write(&z, "options.ini", "p1");
        write(&z, "Lua/ModOptions.ini", "p1");
        write(&z, "Lua/NewMod.ini", "p1");
        write(&z, "NewModFolder/data.txt", "p1");

        assert!(switch_to(&z, SHARED).unwrap());
        shared_untouched(&z);
        for rel in PROFILE_ITEMS {
            assert_eq!(read(&z, rel), "shared", "{} back to the shared one", rel);
        }
        assert!(!z.join("Lua/NewMod.ini").exists() && !z.join("NewModFolder").exists());
        assert_eq!(read(&p1, "Lua/NewMod.ini"), "p1");
        assert!(z.join("Saves/Sandbox/Shared World").is_dir());
        assert!(!z.join(STORE).join(SHARED).join(SETTINGS).exists(), "the set in place has nothing put away");

        assert!(switch_to(&z, "p1").unwrap());
        shared_untouched(&z);
        let now = ["options.ini", "Lua/ModOptions.ini", "Lua/NewMod.ini", "NewModFolder/data.txt"].map(|r| read(&z, r));
        assert_eq!(now, ["p1", "p1", "p1", "p1"].map(String::from));
        assert_eq!(read(&z, "Lua/keysB42.ini"), "shared");
        assert!(!settings_backups(&z).exists(), "nothing was in the way");
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn a_switch_cut_short_at_each_step_is_finished() {
        // Cut while the shared set's settings were going away: options.ini already gone, the rest not.
        let z = zomboid("cut-out");
        let p2 = z.join(STORE).join("p2").join(SETTINGS);
        write(&p2, "options.ini", "p2");
        write(&p2, "Lua/ModOptions.ini", "p2");
        let away = z.join(STORE).join(SHARED).join(SETTINGS);
        mv(&z, "options.ini", &away.join("options.ini"));
        mv(&z, "Saves/Sandbox/Shared World", &z.join(STORE).join(SHARED).join("Saves/Sandbox/Shared World"));
        write_state(&z, &State { current: SHARED.into(), moving: Some((SHARED.into(), "p2".into(), "out".into())) }).unwrap();
        recover(&z).unwrap();
        assert_eq!(state(&z), State { current: "p2".into(), moving: None });
        assert_eq!((read(&z, "options.ini"), read(&z, "Lua/ModOptions.ini")), ("p2".into(), "p2".into()));
        assert!(!z.join("Lua/keysB42.ini").exists(), "p2 has none of its own");
        for rel in PROFILE_ITEMS {
            assert_eq!(read(&away, rel), "shared", "{} put away whole", rel);
        }
        assert!(!p2.exists(), "all of p2's came in");
        shared_untouched(&z);

        // Back: cut while the shared set's came in, options.ini in already, the rest still away.
        mv(&z, "options.ini", &p2.join("options.ini"));
        mv(&z, "Lua/ModOptions.ini", &p2.join("Lua/ModOptions.ini"));
        fs::rename(away.join("options.ini"), z.join("options.ini")).unwrap();
        write_state(&z, &State { current: "p2".into(), moving: Some(("p2".into(), SHARED.into(), "in".into())) }).unwrap();
        recover(&z).unwrap();
        assert_eq!(state(&z), State { current: SHARED.into(), moving: None });
        for rel in PROFILE_ITEMS {
            assert_eq!(read(&z, rel), "shared", "{} back in", rel);
        }
        assert!(!away.exists() && z.join("Saves/Sandbox/Shared World").is_dir());
        assert_eq!((read(&p2, "options.ini"), read(&p2, "Lua/ModOptions.ini")), ("p2".into(), "p2".into()));
        shared_untouched(&z);
        assert!(!settings_backups(&z).exists(), "nothing was in the way");
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn isolating_the_active_profile_copies_and_turning_it_off_keeps_its_settings_aside() {
        let z = zomboid("active");
        take_own(&z, "p3", true, &[]).unwrap();
        assert_eq!(state(&z).current, "p3");
        for rel in PROFILE_ITEMS {
            assert_eq!(read(&z, rel), "shared", "{} stays in place for the profile", rel);
            assert_eq!(read(&z.join(STORE).join(SHARED).join(SETTINGS), rel), "shared", "{}: the shared set keeps a copy", rel);
        }
        write(&z, "options.ini", "p3");
        write(&z, "Lua/OnlyInP3.ini", "p3");
        write(&z, "Saves/Sandbox/P3 World/map_ver.bin", "P3 World");

        let kept = give_back(&z, "p3").unwrap().expect("its settings kept");
        assert_eq!(state(&z).current, SHARED);
        shared_untouched(&z);
        for rel in PROFILE_ITEMS {
            assert_eq!(read(&z, rel), "shared", "{}: the shared one, not overwritten", rel);
        }
        assert!(!z.join("Lua/OnlyInP3.ini").exists());
        assert_eq!((read(&kept, "options.ini"), read(&kept, "Lua/OnlyInP3.ini")), ("p3".into(), "p3".into()));
        assert!(kept.starts_with(settings_backups(&z).join("p3")));
        assert!(z.join("Saves/Sandbox/P3 World").is_dir() && z.join("Saves/Sandbox/Shared World").is_dir(), "no world lost");
        assert!(!z.join(STORE).join("p3").exists() && !z.join(STORE).join(SHARED).join(SETTINGS).exists());
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn turning_off_the_active_profile_cut_short_is_finished() {
        // Cut after its settings went into its store, before the state: they come back in.
        let z = zomboid("off-cut");
        take_own(&z, "p9", true, &[]).unwrap();
        write(&z, "options.ini", "p9");
        settings_out(&z, "p9").unwrap();
        assert!(!z.join("options.ini").exists());
        recover(&z).unwrap();
        assert_eq!((state(&z).current, read(&z, "options.ini")), ("p9".to_string(), "p9".to_string()));
        assert!(!z.join(STORE).join("p9").join(SETTINGS).exists());
        // Cut after the state, the shared settings half in: the rest come in, the profile's wait.
        settings_out(&z, "p9").unwrap();
        write_state(&z, &State { current: SHARED.into(), moving: None }).unwrap();
        mv(&z.join(STORE).join(SHARED).join(SETTINGS), "options.ini", &z.join("options.ini"));
        recover(&z).unwrap();
        for rel in PROFILE_ITEMS {
            assert_eq!(read(&z, rel), "shared", "{} in", rel);
        }
        assert_eq!(read(&z.join(STORE).join("p9").join(SETTINGS), "options.ini"), "p9");
        // Asked again (the profile is still marked isolated): its settings are kept aside.
        let kept = give_back(&z, "p9").unwrap().unwrap();
        assert_eq!(read(&kept, "options.ini"), "p9");
        shared_untouched(&z);
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn isolating_the_active_profile_cut_before_the_state_keeps_what_is_in_place() {
        let z = zomboid("active-cut");
        copy_settings_to(&z, SHARED).unwrap();
        fs::create_dir_all(z.join(STORE).join("p7").join(COPYING).join("Lua")).unwrap();
        recover(&z).unwrap();
        assert_eq!(state(&z).current, SHARED);
        for rel in PROFILE_ITEMS {
            assert_eq!(read(&z, rel), "shared");
        }
        assert!(!z.join(STORE).join(SHARED).join(SETTINGS).exists() && !z.join(STORE).join("p7").join(COPYING).exists());
        let replaced: Vec<_> = fs::read_dir(settings_backups(&z)).unwrap().flatten().collect();
        assert_eq!(replaced.len(), 1, "the leftover copy is kept, never laid over");
        shared_untouched(&z);
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn turning_off_or_deleting_a_profile_that_is_not_on() {
        let z = zomboid("off");
        take_own(&z, "p4", false, &["Sandbox/Shared World".to_string()]).unwrap();
        assert!(z.join(STORE).join("p4/Saves/Sandbox/Shared World").is_dir());
        switch_to(&z, "p4").unwrap();
        write(&z, "options.ini", "p4");
        switch_to(&z, SHARED).unwrap();
        assert_eq!(read(&z, "options.ini"), "shared");
        let kept = give_back(&z, "p4").unwrap().unwrap();
        assert_eq!(read(&kept, "options.ini"), "p4");
        assert_eq!(read(&z, "options.ini"), "shared");
        assert!(z.join("Saves/Sandbox/Shared World").is_dir());
        assert!(!z.join(STORE).join("p4").exists());
        shared_untouched(&z);

        // A profile turned off while another isolated one is on: its worlds wait with the shared ones.
        take_own(&z, "p5", false, &[]).unwrap();
        take_own(&z, "p6", false, &[]).unwrap();
        switch_to(&z, "p5").unwrap();
        write(&z, "options.ini", "p5");
        switch_to(&z, "p6").unwrap();
        write(&z, "Saves/Sandbox/P6 World/map_ver.bin", "P6");
        switch_to(&z, "p5").unwrap();
        let kept = give_back(&z, "p6").unwrap().unwrap();
        assert_eq!(read(&kept, "options.ini"), "shared", "p6 started with a copy of the shared ones");
        assert_eq!(read(&z, "options.ini"), "p5");
        assert!(z.join(STORE).join(SHARED).join("Saves/Sandbox/P6 World").is_dir());
        switch_to(&z, SHARED).unwrap();
        assert_eq!(read(&z, "options.ini"), "shared");
        assert!(z.join("Saves/Sandbox/P6 World").is_dir());
        shared_untouched(&z);
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn a_set_isolated_before_settings_were_starts_with_a_copy() {
        // Saves only, from before: the switch gives it a copy rather than the game's defaults.
        let z = zomboid("old");
        write(&z.join(STORE).join("old/Saves/Sandbox/Old World"), "map_ver.bin", "Old");
        assert!(switch_to(&z, "old").unwrap());
        for rel in PROFILE_ITEMS {
            assert_eq!(read(&z, rel), "shared", "{}: a copy", rel);
            assert_eq!(read(&z.join(STORE).join(SHARED).join(SETTINGS), rel), "shared", "{}: the shared set's own", rel);
        }
        assert!(z.join("Saves/Sandbox/Old World").is_dir());

        // Old one in place, shared set from before (saves only): turning it off loses nothing.
        let z2 = zomboid("old-on");
        write_state(&z2, &State { current: "q".into(), moving: None }).unwrap();
        mv(&z2, "Saves/Sandbox/Shared World", &z2.join(STORE).join("shared/Saves/Sandbox/Shared World"));
        write(&z2, "options.ini", "q");
        let kept = give_back(&z2, "q").unwrap().unwrap();
        assert_eq!(read(&z2, "options.ini"), "q", "they were everyone's then");
        assert_eq!(read(&kept, "options.ini"), "q");
        assert!(z2.join("Saves/Sandbox/Shared World").is_dir());
        shared_untouched(&z2);
        let _ = fs::remove_dir_all(&z);
        let _ = fs::remove_dir_all(&z2);
    }

    #[test]
    fn nothing_in_the_way_is_overwritten() {
        let z = zomboid("collide");
        let store = z.join(STORE).join("p8").join(SETTINGS);
        write(&store, "options.ini", "p8");
        write(&store, "Lua/PZModStudio_Link.txt", "stale link file");
        settings_in(&z, "p8").unwrap();
        assert_eq!(read(&z, "options.ini"), "p8");
        assert_eq!(read(&z, "Lua/PZModStudio_Link.txt"), "shared item Lua/PZModStudio_Link.txt", "a shared name is never brought in");
        let kept = files(&settings_backups(&z));
        assert_eq!(kept.len(), 2, "{:?}", kept);
        assert!(kept.iter().any(|f| f.ends_with("/options.ini")) && kept.iter().any(|f| f.ends_with("Lua/PZModStudio_Link.txt")));
        assert!(!store.exists());
        let _ = fs::remove_dir_all(&z);
    }
}

#[cfg(test)]
mod hardening_tests {
    use super::*;

    fn zomboid(tag: &str) -> PathBuf {
        let z = std::env::temp_dir().join(format!("pzms-pharden-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&z);
        for (rel, text) in [("options.ini", "shared"), ("Lua/ModOptions.ini", "shared"), ("Saves/Sandbox/W/map_ver.bin", "W")] {
            let p = z.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        }
        z
    }

    fn pretend(why: Option<&str>) {
        TEST_BUSY.with(|b| *b.borrow_mut() = why.map(String::from));
    }

    fn log_text(z: &Path) -> String {
        fs::read_to_string(z.join("PZModStudio_Logs/studio.log")).unwrap_or_default()
    }

    #[test]
    fn nothing_moves_while_a_game_or_a_server_runs() {
        let z = zomboid("busy");
        let why = refusal(None, &[(4242, "knoxtest".into())]).unwrap();
        assert!(why.contains("\"knoxtest\"") && why.contains("4242") && why.contains("Stop it first"), "{}", why);
        assert!(refusal(Some(7), &[(1, String::new())]).unwrap().starts_with("Close Project Zomboid first (process 7)"));
        assert!(refusal(None, &[(9, String::new())]).unwrap().starts_with("A Project Zomboid server is running (process 9)"));
        assert_eq!(refusal(None, &[]), None);

        pretend(Some(&why));
        assert_eq!(take_own(&z, "p1", false, &["Sandbox/W".into()]), Err(why.clone()));
        assert_eq!(switch_to(&z, "p1"), Err(why.clone()));
        assert_eq!(give_back(&z, "p1"), Err(why.clone()));
        assert_eq!(move_saves(&z, &["Sandbox/W".into()], "p1"), Err(why.clone()));
        assert!(!z.join(STORE).exists() && z.join("Saves/Sandbox/W").is_dir(), "nothing moved");
        // The set already in place: nothing to move, nothing refused.
        assert_eq!(switch_to(&z, SHARED), Ok(false));
        // A switch cut short waits for the server to close too.
        pretend(None);
        take_own(&z, "p1", false, &[]).unwrap();
        write_state(&z, &State { current: SHARED.into(), moving: Some((SHARED.into(), "p1".into(), "out".into())) }).unwrap();
        pretend(Some(&why));
        recover(&z).unwrap();
        assert!(state(&z).moving.is_some() && z.join("Saves/Sandbox/W").is_dir());
        pretend(None);
        recover(&z).unwrap();
        assert_eq!(state(&z), State { current: "p1".into(), moving: None });
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn big_unknown_data_is_left_shared_not_copied() {
        let z = zomboid("big");
        let big = |p: &Path, bytes: u64| {
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::File::create(p).unwrap().set_len(bytes).unwrap();
        };
        // A mod's big file, a mod's big folder (in 3 files, each under the file limit), a big
        // known setting, and a small mod folder.
        big(&z.join("SomeModDump.bin"), BIG_FILE + 1);
        for n in 0..3 {
            big(&z.join(format!("SomeModTV/part{}.bin", n)), 18 * 1024 * 1024);
        }
        big(&z.join("Lua/keysB42.ini"), BIG_FILE + 1);
        big(&z.join("Lua/HugeModLog.txt"), BIG_FILE + 1);
        fs::create_dir_all(z.join("SmallMod")).unwrap();
        fs::write(z.join("SmallMod/cfg.txt"), "shared").unwrap();

        let left = take_own(&z, "p1", false, &[]).unwrap();
        let mut names: Vec<&str> = left.iter().map(|l| l.rel.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["Lua/HugeModLog.txt", "SomeModDump.bin", "SomeModTV"]);
        assert!(left.iter().all(|l| l.profile == "p1" && l.bytes > BIG_FILE));
        assert_eq!(left_shared(&z), left, "written down");
        let copy = z.join(STORE).join("p1").join(SETTINGS);
        assert!(copy.join("options.ini").is_file() && copy.join("Lua/keysB42.ini").is_file() && copy.join("SmallMod/cfg.txt").is_file(), "settings and small data copied, a known setting whatever its size");
        assert!(!copy.join("SomeModDump.bin").exists() && !copy.join("SomeModTV").exists() && !copy.join("Lua/HugeModLog.txt").exists());

        // Shared from now on: a switch leaves them in place both ways.
        switch_to(&z, "p1").unwrap();
        for rel in ["SomeModDump.bin", "SomeModTV/part2.bin", "Lua/HugeModLog.txt"] {
            assert!(z.join(rel).is_file(), "{} stays in place", rel);
        }
        assert!(!z.join(STORE).join(SHARED).join(SETTINGS).join("SomeModTV").exists());
        assert!(z.join(STORE).join(SHARED).join(SETTINGS).join("SmallMod/cfg.txt").is_file(), "the small one is the shared set's own");
        switch_to(&z, SHARED).unwrap();
        assert!(z.join("SomeModTV/part0.bin").is_file() && z.join("SmallMod/cfg.txt").is_file());

        // A set that had its own copy from before: kept in the backups, never laid over.
        let p2 = z.join(STORE).join("p2").join(SETTINGS);
        fs::create_dir_all(&p2).unwrap();
        fs::write(p2.join("SomeModDump.bin"), "p2's old copy").unwrap();
        switch_to(&z, "p2").unwrap();
        assert_eq!(fs::metadata(z.join("SomeModDump.bin")).unwrap().len(), BIG_FILE + 1);
        let log = log_text(&z);
        assert!(log.contains("left shared, not copied for p1") && log.contains("SomeModTV"), "{}", log);
        assert!(log.contains("kept in the backups (it was in the way)") && log.contains("SomeModDump.bin"));
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn every_move_is_in_the_log() {
        let z = zomboid("log");
        fs::write(z.join(LATEST), "Sandbox/W").unwrap();
        take_own(&z, "p1", false, &["Sandbox/W".into()]).unwrap();
        switch_to(&z, "p1").unwrap();
        switch_to(&z, SHARED).unwrap();
        give_back(&z, "p1").unwrap();
        let log = log_text(&z);
        let sep = std::path::MAIN_SEPARATOR;
        let lines: Vec<&str> = log.lines().filter(|l| l.contains("[Profiles]")).collect();
        let has = |needle: &str| lines.iter().any(|l| l.contains(needle));
        assert!(has(&format!("moved world {}", z.join("Saves").join("Sandbox").join("W").display())), "{}", log);
        assert!(has("(taken to p1)"));
        assert!(has(&format!("copied setting for p1: {}", z.join("options.ini").display())));
        assert!(has(&format!("moved setting {}{}options.ini", z.display(), sep)));
        assert!(has("switch shared -> p1 done") && has("switch p1 -> shared done"));
        assert!(has(&format!("moved {} -> ", z.join(LATEST).display())), "the Continue file");
        assert!(has("settings of p1 kept in the backups"));
        assert!(has("ending the isolation of profile p1"));
        let _ = fs::remove_dir_all(&z);
    }
}
