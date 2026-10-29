//! Studio Link: the game shows the profile that is active in the Studio, and what the player
//! changes in the game comes back to that profile.
//!
//! - `Z_StudioLink` (source: `mods/Z_StudioLink/`) is a companion mod that is always on. It reads
//!   `<Zomboid>/Lua/PZModStudio_Link.txt` (`profile=` and one `member=` line per mod of the
//!   profile, on or off), leaves everything else out of the game's Mods screen and refuses a save
//!   that uses a mod outside the profile.
//! - Turning mods on or off or reordering them in the game makes the game rewrite
//!   `mods/default.txt`; [`watch_mod_list`] notices and puts that into the active profile.
//! - Zombie Buddy's core is not loaded through the mod list but by `-agentlib:zbNative` in the
//!   game's `ProjectZomboid64.json`, so it ran whatever the profile said. [`sync_java_agents`]
//!   takes that line out for a profile without Zombie Buddy and puts it back for one with it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime};
use tauri::Emitter;

pub const LINK_MOD_ID: &str = "Z_StudioLink";

/// Other mod managers that draw the game's Mods screen (the Link's OTHER_MANAGERS).
pub const OTHER_MANAGERS: [&str; 4] = ["DefinitiveModManager", "PZMMClient", "ModManager", "ModLoadOrderSorter_b42"];

/// Where the Link keeps which Mods screen the player chose when another manager is on:
/// `screen=studio|other|ask` and `other=<mod id>`, in Zomboid/Lua.
pub const SCREEN_FILE: &str = "PZModStudio_LinkScreen.txt";

/// The player's choice: "studio", "other" or "ask" (nothing chosen yet), and for which manager.
pub fn screen_choice(user_zomboid_dir: &Path) -> (String, String) {
    let text = std::fs::read_to_string(user_zomboid_dir.join("Lua").join(SCREEN_FILE)).unwrap_or_default();
    let field = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(k).map(|v| v.trim().to_string())).unwrap_or_default();
    let screen = field("screen=");
    let screen = if screen == "studio" || screen == "other" { screen } else { "ask".to_string() };
    (screen, field("other="))
}

/// Sets it from the Studio ("ask" makes the game ask again next time it opens).
pub fn set_screen_choice(user_zomboid_dir: &Path, screen: &str, other: &str) -> Result<(), String> {
    let screen = match screen {
        "studio" | "other" => screen,
        _ => "ask",
    };
    let dir = user_zomboid_dir.join("Lua");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(dir.join(SCREEN_FILE), format!("screen={}\nother={}\n", screen, other.trim())).map_err(|e| e.to_string())
}
pub const LINK_FILE: &str = "PZModStudio_Link.txt";
/// Sent to the window when the game changed the mod list of the active profile.
pub const PROFILE_CHANGED_EVENT: &str = "profile-changed-in-game";

const MOD_INFO: &str = include_str!("../../../mods/Z_StudioLink/mod.info");
const MOD_INFO_42: &str = include_str!("../../../mods/Z_StudioLink/42/mod.info");
const CLIENT_LUA: &str = include_str!("../../../mods/Z_StudioLink/42/media/lua/client/PZModStudio_Link.lua");
const SCREEN_LUA: &str = include_str!("../../../mods/Z_StudioLink/42/media/lua/client/PZModStudio_ModsScreen.lua");
const ERRORS_LUA: &str = include_str!("../../../mods/Z_StudioLink/42/media/lua/client/PZModStudio_Errors.lua");

const UI_EN: &str = include_str!("../../../mods/Z_StudioLink/42/media/lua/shared/Translate/EN/UI.json");
const UI_ES: &str = include_str!("../../../mods/Z_StudioLink/42/media/lua/shared/Translate/ES/UI.json");

fn link_files(user_zomboid_dir: &Path) -> [(PathBuf, &'static str); 7] {
    let target = user_zomboid_dir.join("mods").join(LINK_MOD_ID);
    let lua = target.join("42").join("media").join("lua");
    let translate = lua.join("shared").join("Translate");
    [
        (target.join("mod.info"), MOD_INFO),
        (target.join("42").join("mod.info"), MOD_INFO_42),
        (lua.join("client").join("PZModStudio_Link.lua"), CLIENT_LUA),
        (lua.join("client").join("PZModStudio_ModsScreen.lua"), SCREEN_LUA),
        (lua.join("client").join("PZModStudio_Errors.lua"), ERRORS_LUA),
        (translate.join("EN").join("UI.json"), UI_EN),
        (translate.join("ES").join("UI.json"), UI_ES),
    ]
}

/// The Workshop folder of a subscribed copy of `Z_StudioLink`, if the player has one.
fn workshop_copy() -> Option<PathBuf> {
    let workshop = crate::vfs::studio_paths().workshop_dir;
    if workshop.is_empty() {
        return None;
    }
    std::fs::read_dir(&workshop).ok()?.flatten().find_map(|item| {
        let dir = item.path().join("mods").join(LINK_MOD_ID);
        dir.join("mod.info").is_file().then_some(dir)
    })
}

/// `modversion=` of a mod.info text, as numbers ("1.7.0" -> [1, 7, 0]).
pub fn mod_version(info: &str) -> Vec<u32> {
    info.lines()
        .find_map(|l| l.trim().strip_prefix("modversion="))
        .map(|v| v.trim().split('.').map(|n| n.trim().parse().unwrap_or(0)).collect())
        .unwrap_or_default()
}

/// The Link this Studio carries.
pub fn bundled_version() -> Vec<u32> {
    mod_version(MOD_INFO_42)
}

/// The Workshop copy, when there is one at least as new as the Studio's own: that one runs and
/// the local copy goes. An older one (Steam not updated yet, or this Studio newer than the
/// published mod) loses to the local copy the Studio keeps.
fn usable_workshop_copy() -> Option<PathBuf> {
    let dir = workshop_copy()?;
    let info = std::fs::read_to_string(dir.join("42").join("mod.info")).or_else(|_| std::fs::read_to_string(dir.join("mod.info"))).ok()?;
    (mod_version(&info) >= bundled_version()).then_some(dir)
}

/// The local copy must win over an older Workshop one (the Studio starts the game reading
/// Zomboid/mods first, as for compatible copies).
pub fn local_link_must_win(user_zomboid_dir: &Path) -> bool {
    !cfg!(test) && workshop_copy().is_some() && usable_workshop_copy().is_none() && user_zomboid_dir.join("mods").join(LINK_MOD_ID).join("mod.info").is_file()
}

/// Installs `Z_StudioLink` into `<Zomboid>/mods`, or brings it up to this build's version.
/// Files that are already right are not touched. With the mod subscribed on the Workshop the
/// local copy goes instead: two copies with one id would leave the game picking one (the
/// Workshop's, by its folder order), and which version runs would depend on that.
pub fn install_link_mod(user_zomboid_dir: &Path) -> Result<(), String> {
    let local = user_zomboid_dir.join("mods").join(LINK_MOD_ID);
    if !cfg!(test) && usable_workshop_copy().is_some() {
        // Only ever the Studio's own files: anything else the player put there stays.
        for (path, _) in link_files(user_zomboid_dir) {
            let _ = std::fs::remove_file(path);
        }
        let lua = local.join("42").join("media").join("lua");
        let translate = lua.join("shared").join("Translate");
        for dir in [translate.join("EN"), translate.join("ES"), translate.clone(), lua.join("shared"), lua.join("client"), lua.clone(), local.join("42").join("media"), local.join("42"), local.join("common"), local.clone()] {
            let _ = std::fs::remove_dir(dir); // only when empty
        }
        return Ok(());
    }
    let _ = std::fs::create_dir_all(local.join("common"));
    for (path, content) in link_files(user_zomboid_dir) {
        if std::fs::read_to_string(&path).is_ok_and(|now| now == content) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Cannot create {}: {}", parent.display(), e))?;
        }
        std::fs::write(&path, content).map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
    }
    Ok(())
}

/// A local Link that is not the one this Studio carries (none installed: nothing to update).
pub fn link_needs_update(user_zomboid_dir: &Path) -> bool {
    let info = user_zomboid_dir.join("mods").join(LINK_MOD_ID).join("42").join("mod.info");
    info.is_file() && link_files(user_zomboid_dir).iter().any(|(path, content)| std::fs::read_to_string(path).map_or(true, |now| now != *content))
}

/// The text of `PZModStudio_Link.txt` for a profile: its name and id (the saves are told apart
/// by it), its mods in load order, and the mods other profiles hold (so the game can tell a mod
/// that is in no profile at all, a new one).
pub fn link_file_text(profile_id: &str, profile_name: &str, members: &[String], others: &[String]) -> String {
    let mut out = format!("profile={}\nprofileid={}\n", profile_name.replace(['\r', '\n'], " "), profile_id.replace(['\r', '\n'], ""));
    for id in members {
        out.push_str(&format!("member={}\n", id));
    }
    for id in others {
        if !members.contains(id) {
            out.push_str(&format!("other={}\n", id));
        }
    }
    out
}

/// Writes what the game's Mods screen should show for the active profile.
pub fn write_link_file(user_zomboid_dir: &Path, profile_id: &str, profile_name: &str, members: &[String], others: &[String]) -> Result<(), String> {
    let lua_dir = user_zomboid_dir.join("Lua");
    std::fs::create_dir_all(&lua_dir).map_err(|e| format!("Cannot create {}: {}", lua_dir.display(), e))?;
    let path = lua_dir.join(LINK_FILE);
    let text = link_file_text(profile_id, profile_name, members, others);
    if std::fs::read_to_string(&path).is_ok_and(|now| now == text) {
        return Ok(());
    }
    crate::load_order::ini_parser::atomic_write(&path, text).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

/// The load order after the game reordered or switched mods: the inactive mods stay where they
/// were and the active ones fill, in the game's order, the places active mods had before. Active
/// mods the profile had never placed go last.
pub fn merge_game_order(previous: &[String], game_active: &[String]) -> Vec<String> {
    let active: std::collections::HashSet<&String> = game_active.iter().collect();
    let mut queue = game_active.iter();
    let mut out = Vec::with_capacity(previous.len().max(game_active.len()));
    for id in previous {
        if active.contains(id) {
            if let Some(next) = queue.next() {
                out.push(next.clone());
            }
        } else {
            out.push(id.clone());
        }
    }
    out.extend(queue.cloned());
    out
}

// ------------------------------------------------------------------ talking with the game
//
// Everything goes through small text files in `<Zomboid>/Lua`, the only folder the game's Lua can
// read and write. Both sides poll; nothing waits on the other.
//
//   PZModStudio_LinkApp.txt       app  -> game  protocol, version, alive=<unix seconds> (every few
//                                               seconds while the app runs)
//   PZModStudio_LinkRequests.txt  game -> app   <id>\t<verb>\t<argument> per request; the app reads
//                                               and removes it
//   PZModStudio_LinkAcks.txt      app  -> game  <id>\tok|error\t<text> per request handled (the
//                                               last ACKS_KEPT)
//   PZModStudio_LinkGame.txt      game -> app   where the player is: state=menu|ingame, the save,
//                                               its mods (written on entering the menu or a game)

/// Bumped when one of the files above changes meaning. The Link mod says which one it speaks in
/// the game file, the app in its own.
pub const PROTOCOL: u32 = 2;
pub const APP_FILE: &str = "PZModStudio_LinkApp.txt";
pub const REQUESTS_FILE: &str = "PZModStudio_LinkRequests.txt";
pub const ACKS_FILE: &str = "PZModStudio_LinkAcks.txt";
pub const GAME_FILE: &str = "PZModStudio_LinkGame.txt";
/// Sent to the window when the game file changed: the player entered the menu or a game.
pub const GAME_STATE_EVENT: &str = "link-game-state";
/// Sent to the window with the screen the game asked to see ("monitor").
pub const NAVIGATE_EVENT: &str = "studio-navigate";
const ACKS_KEPT: usize = 50;

/// One thing the game asked for. `id` is empty for the first protocol's `add=<mod id>` lines.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkRequest {
    pub id: String,
    pub verb: String,
    pub arg: String,
}

/// The requests in a requests file, each (verb, argument) once.
pub fn parse_requests(text: &str) -> Vec<LinkRequest> {
    let mut out: Vec<LinkRequest> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let request = if let Some(id) = line.strip_prefix("add=") {
            LinkRequest { id: String::new(), verb: "add".into(), arg: id.trim().into() }
        } else {
            let mut parts = line.splitn(3, '\t');
            let (Some(id), Some(verb)) = (parts.next(), parts.next()) else { continue };
            LinkRequest { id: id.trim().into(), verb: verb.trim().into(), arg: parts.next().unwrap_or("").trim().into() }
        };
        if request.verb.is_empty() || (request.verb == "add" && request.arg.is_empty()) || out.iter().any(|r| r.verb == request.verb && r.arg == request.arg) {
            continue;
        }
        out.push(request);
    }
    out
}

/// The app file: which protocol and version this app speaks, and when it last said so.
pub fn app_file_text(now_secs: u64) -> String {
    format!("protocol={}\nversion={}\nalive={}\n", PROTOCOL, env!("CARGO_PKG_VERSION"), now_secs)
}

/// The acks file after these answers: the newest `ACKS_KEPT` lines, the given ones last.
pub fn acks_text(previous: &str, answers: &[(String, bool, String)]) -> String {
    let clean = |s: &str| s.replace(['\t', '\r', '\n'], " ");
    let mut lines: Vec<String> = previous.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect();
    for (id, ok, text) in answers {
        if id.is_empty() {
            continue; // first-protocol requests carry no id to answer to
        }
        lines.push(format!("{}\t{}\t{}", clean(id), if *ok { "ok" } else { "error" }, clean(text)));
    }
    let skip = lines.len().saturating_sub(ACKS_KEPT);
    lines.into_iter().skip(skip).map(|l| l + "\n").collect()
}

/// Where the player is, as the Link mod last wrote it.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameState {
    pub protocol: u32,
    /// The Link mod's version (`modversion` of its mod.info).
    pub link_version: String,
    /// "menu" or "ingame".
    pub state: String,
    /// The save folder (`<mode>/<world>`), empty on the menu.
    pub save: String,
    /// The mods the game runs with, in load order.
    pub mods: Vec<String>,
    /// Unix seconds, from the game's clock.
    pub at: u64,
}

pub fn parse_game_state(text: &str) -> Option<GameState> {
    let mut state = GameState::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim();
        match key.trim() {
            "protocol" => state.protocol = value.parse().unwrap_or(0),
            "link" => state.link_version = value.to_string(),
            "state" => state.state = value.to_string(),
            "save" => state.save = value.to_string(),
            "mod" if !value.is_empty() => state.mods.push(value.to_string()),
            "at" => state.at = value.parse().unwrap_or(0),
            _ => {}
        }
    }
    (!state.state.is_empty()).then_some(state)
}

/// What a save last ran with, recorded each time the player enters it: the base for telling,
/// before it loads again, what changed since.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SaveReference {
    pub mods: Vec<String>,
    pub at: u64,
    /// The Studio profile that was active then (its id, and its name then, for one deleted since).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
}

/// `<Zomboid>/PZModStudio_SaveRefs.json`: save folder -> its reference.
const SAVE_REFS_FILE: &str = "PZModStudio_SaveRefs.json";

pub fn save_references(user_zomboid_dir: &Path) -> std::collections::BTreeMap<String, SaveReference> {
    std::fs::read_to_string(user_zomboid_dir.join(SAVE_REFS_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Notes what a save ran with. Only games, and only a list that changed, are written.
pub fn record_save_reference(user_zomboid_dir: &Path, state: &GameState) -> Result<bool, String> {
    if state.state != "ingame" || state.save.is_empty() {
        return Ok(false);
    }
    // Refs that could not be read must not be replaced by this one save's.
    crate::load_order::ini_parser::guard_overwrite::<std::collections::BTreeMap<String, SaveReference>>(&user_zomboid_dir.join(SAVE_REFS_FILE))?;
    let mut refs = save_references(user_zomboid_dir);
    let (profile, profile_name) = crate::instance_manager::active_profile(&user_zomboid_dir.to_string_lossy()).unzip();
    if refs.get(&state.save).is_some_and(|r| r.mods == state.mods && r.profile == profile) {
        return Ok(false);
    }
    refs.insert(state.save.clone(), SaveReference { mods: state.mods.clone(), at: state.at, profile, profile_name });
    let text = serde_json::to_string_pretty(&refs).map_err(|e| e.to_string())?;
    let path = user_zomboid_dir.join(SAVE_REFS_FILE);
    crate::load_order::ini_parser::atomic_write(&path, text).map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
    Ok(true)
}

/// What the app knows about the game side: the last game file and the saves' references.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct LinkStatus {
    pub game: Option<GameState>,
    pub saves: std::collections::BTreeMap<String, SaveReference>,
}

/// app -> game: the save the next launch loads (`save=<mode>/<world>`, `at=<unix seconds>`). The
/// Link reads it once, when the main menu is ready, empties it and loads that save; a request
/// older than ten minutes is ignored.
pub const LOAD_FILE: &str = "PZModStudio_LinkLoad.txt";

pub fn write_load_file(user_zomboid_dir: &Path, save: Option<&str>) -> Result<(), String> {
    let lua = user_zomboid_dir.join("Lua");
    let text = match save {
        Some(id) => {
            if !crate::save_backup::list_saves(user_zomboid_dir).iter().any(|s| s == id) {
                return Err(format!("\"{}\" is not a save on this PC.", id));
            }
            if id.starts_with("Multiplayer/") {
                return Err("A server's save is played by joining the server.".to_string());
            }
            format!("save={}
at={}
", id, now_secs())
        }
        None => String::new(),
    };
    if !lua.is_dir() {
        return if save.is_some() { Err("The game's Lua folder is not there yet: start the game once first.".to_string()) } else { Ok(()) };
    }
    crate::load_order::ini_parser::atomic_write(lua.join(LOAD_FILE), text).map_err(|e| e.to_string())
}

pub fn link_status(user_zomboid_dir: &Path) -> LinkStatus {
    LinkStatus {
        game: std::fs::read_to_string(user_zomboid_dir.join("Lua").join(GAME_FILE)).ok().and_then(|t| parse_game_state(&t)),
        saves: save_references(user_zomboid_dir),
    }
}

/// Does what the game asked and says how it went, per request.
fn handle_requests<R: tauri::Runtime>(app: &tauri::AppHandle<R>, z_dir: &Path, requests: &[LinkRequest]) -> Vec<(String, bool, String)> {
    let mut answers = Vec::new();
    let adds: Vec<&LinkRequest> = requests.iter().filter(|r| r.verb == "add" && !r.arg.is_empty()).collect();
    if !adds.is_empty() {
        let ids: Vec<String> = adds.iter().map(|r| r.arg.clone()).collect();
        let result = crate::instance_manager::add_members_to_active(z_dir, &ids);
        match &result {
            Ok(Some(profile)) => {
                let _ = app.emit(PROFILE_CHANGED_EVENT, profile.clone());
            }
            Ok(None) => {}
            Err(e) => crate::diagnostics::error(format!("[StudioLink] could not add the mods the game asked for: {}", e)),
        }
        for r in &adds {
            answers.push(match &result {
                Ok(_) => (r.id.clone(), true, String::new()),
                Err(e) => (r.id.clone(), false, e.clone()),
            });
        }
    }
    for r in requests.iter().filter(|r| r.verb == "backup") {
        // "<mode>/<world>\t<reason>": answered when the backup is done, from its own thread.
        let (save, reason) = r.arg.split_once('\t').unwrap_or((r.arg.as_str(), ""));
        let reason = match reason.trim() {
            crate::save_backup::REASON_BEFORE_FIX => crate::save_backup::REASON_BEFORE_FIX,
            _ => crate::save_backup::REASON_BEFORE_LOAD,
        };
        start_backup(app, z_dir, save.trim(), reason, Some(r.id.clone()));
    }
    for r in requests.iter().filter(|r| r.verb == "copy") {
        // "<mode>/<world>": answered with the copy's world folder.
        start_copy(app, z_dir, r.arg.trim(), r.id.clone());
    }
    // "Open PZ Mod Studio" on the game's Mods screen: the window comes to the front.
    for r in requests.iter().filter(|r| r.verb == "show") {
        crate::window_prefs::bring_to_front(app);
        // "show monitor": the error notice in the game opens the Monitor, which has its card.
        if !r.arg.is_empty() {
            let _ = app.emit(NAVIGATE_EVENT, r.arg.clone());
        }
        answers.push((r.id.clone(), true, String::new()));
    }
    for r in requests.iter().filter(|r| !["add", "backup", "copy", "show"].contains(&r.verb.as_str())) {
        answers.push((r.id.clone(), false, format!("This version of PZ Mod Studio does not know \"{}\".", r.verb)));
    }
    answers
}

// ------------------------------------------------------------------ save backups

/// app -> game: what the Link needs to know of each save before it loads, one line per save:
/// `save\t<mode>/<world>\tbacked=0|1\tbytes=<n>\tsession=<unix seconds>\tupdated=<n>\tmods=<a;b;...>`.
/// `backed`: unchanged since its newest backup. `session` and `mods`: its last session (0 and
/// empty when the Studio never saw it played). `updated`: mods of that session whose Workshop
/// version on disk was published after it.
pub const SAVES_FILE: &str = "PZModStudio_LinkSaves.txt";
/// Sent to the window when a save's backups changed.
pub const SAVES_CHANGED_EVENT: &str = "save-backups-changed";

/// Mod id -> Workshop item, from the mod.info files in the Workshop folder (the B42 one too).
fn workshop_ids_by_mod(workshop_dir: &Path) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for item in std::fs::read_dir(workshop_dir).into_iter().flatten().flatten() {
        let wid = item.file_name().to_string_lossy().to_string();
        for m in std::fs::read_dir(item.path().join("mods")).into_iter().flatten().flatten() {
            for info in [m.path().join("mod.info"), m.path().join("42").join("mod.info")] {
                let Ok(text) = std::fs::read_to_string(&info) else { continue };
                if let Some(id) = text.lines().find_map(|l| l.trim().strip_prefix("id=")) {
                    out.insert(crate::load_order::mod_info::sanitize_mod_id(id), wid.clone());
                }
            }
        }
    }
    out
}

/// The session's mods whose Workshop version on disk was published after that session.
fn updated_since(session: &SaveReference, by_mod: &std::collections::HashMap<String, String>, state: Option<&crate::workshop::WorkshopLocalState>) -> Vec<String> {
    let Some(state) = state else { return Vec::new() };
    session
        .mods
        .iter()
        .filter(|id| {
            by_mod
                .get(&crate::load_order::mod_info::sanitize_mod_id(id))
                .and_then(|wid| state.items.get(wid))
                .and_then(|i| i.installed_time)
                .is_some_and(|t| t > session.at)
        })
        .cloned()
        .collect()
}

/// What changed for a save since its last session: mods its list has now and did not then, and
/// the other way round, and mods updated on the Workshop since.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct SessionDiff {
    /// When the last session was (unix seconds).
    pub at: u64,
    /// The profile it was played with (id, and its name then).
    pub profile: Option<String>,
    pub profile_name: Option<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub updated: Vec<String>,
    /// The order of the mods that are in both changed.
    pub reordered: bool,
}

pub fn session_diff(session: &SaveReference, now: &[String], updated: Vec<String>) -> SessionDiff {
    let plain = |id: &String| crate::load_order::mod_info::sanitize_mod_id(id);
    let then: Vec<String> = session.mods.iter().map(plain).collect();
    let now: Vec<String> = now.iter().map(plain).collect();
    let added: Vec<String> = now.iter().filter(|m| !then.contains(m)).cloned().collect();
    let removed: Vec<String> = then.iter().filter(|m| !now.contains(m)).cloned().collect();
    let kept_then: Vec<&String> = then.iter().filter(|m| now.contains(m)).collect();
    let kept_now: Vec<&String> = now.iter().filter(|m| then.contains(m)).collect();
    SessionDiff { at: session.at, profile: session.profile.clone(), profile_name: session.profile_name.clone(), added, removed, updated, reordered: kept_then != kept_now }
}

/// `Multiplayer/<ip>_<port>_<hash>` -> `<ip>:<port>`.
fn server_address_of(save_id: &str) -> Option<String> {
    let world = save_id.strip_prefix("Multiplayer/")?;
    let mut parts = world.rsplitn(3, '_');
    let hash = parts.next()?;
    let port = parts.next()?;
    let ip = parts.next()?;
    (hash.len() >= 16 && hash.chars().all(|c| c.is_ascii_hexdigit()) && !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) && !ip.is_empty())
        .then(|| format!("{}:{}", ip, port))
}

/// Servers' saves (`Multiplayer/<ip>_<port>_<hash>`) the game wrote to after `since`. The game
/// rewrites files at the top of a save (`map_t.bin`, `InGameMap.ini`...) whenever it saves.
fn server_saves_changed_since(user_zomboid_dir: &Path, since: SystemTime) -> Vec<String> {
    crate::save_backup::list_saves(user_zomboid_dir)
        .into_iter()
        .filter(|id| server_address_of(id).is_some())
        .filter(|id| {
            let dir = crate::save_backup::saves_dir(user_zomboid_dir).join(id);
            std::fs::read_dir(dir).map(|entries| entries.flatten().filter_map(|e| e.metadata().ok()?.modified().ok()).any(|m| m > since)).unwrap_or(false)
        })
        .collect()
}

/// A server's save the Link never saw: on a server the game loads only the server's mods, so the
/// Link is not running there to say which save it is. The Studio's own list of servers knows the
/// profile it last joined each one with, and that is the session noted for its save.
fn add_server_sessions(user_zomboid_dir: &Path, refs: &mut std::collections::BTreeMap<String, SaveReference>) {
    let servers = crate::server_browser::read_saved(user_zomboid_dir);
    if servers.is_empty() {
        return;
    }
    let profiles = crate::instance_manager::list_instances(user_zomboid_dir.to_string_lossy().to_string()).unwrap_or_default();
    for id in crate::save_backup::list_saves(user_zomboid_dir) {
        if refs.get(&id).is_some_and(|r| r.profile.is_some()) {
            continue;
        }
        let Some(address) = server_address_of(&id) else { continue };
        let Some(server) = servers.iter().find(|s| s.address.eq_ignore_ascii_case(&address)) else { continue };
        let Some(profile) = server.profile_id.clone().or_else(|| server.made_profile_id.clone()) else { continue };
        let profile_name = profiles.iter().find(|p| p.id == profile).map(|p| p.name.clone());
        let at = server.last_joined.unwrap_or(0).max(0) as u64;
        refs.insert(id, SaveReference { mods: Vec::new(), at, profile: Some(profile), profile_name });
    }
}

/// For each save with a recorded session: what changed since.
pub fn session_diffs(user_zomboid_dir: &Path) -> std::collections::BTreeMap<String, SessionDiff> {
    let paths = crate::vfs::studio_paths();
    let by_mod = if paths.workshop_dir.is_empty() { Default::default() } else { workshop_ids_by_mod(Path::new(&paths.workshop_dir)) };
    let state = (!paths.workshop_dir.is_empty()).then(|| crate::workshop::local_state(&paths.workshop_dir)).flatten();
    let mut refs = save_references(user_zomboid_dir);
    add_server_sessions(user_zomboid_dir, &mut refs);
    let mut out = std::collections::BTreeMap::new();
    for id in crate::save_backup::list_saves(user_zomboid_dir) {
        let Some(session) = refs.get(&id) else { continue };
        let Ok(dir) = crate::save_backup::saves_dir(user_zomboid_dir).join(&id).canonicalize() else { continue };
        // A server's save keeps no mods.txt: the server says which mods, so there is no list to
        // set beside the last session's. Its profile is still worth showing.
        if !dir.join("mods.txt").is_file() {
            out.insert(id, SessionDiff { at: session.at, profile: session.profile.clone(), profile_name: session.profile_name.clone(), ..Default::default() });
            continue;
        }
        let now = crate::save_backup::save_mods(&dir);
        out.insert(id, session_diff(session, &now, updated_since(session, &by_mod, state.as_ref())));
    }
    out
}

/// The profile each save belongs to, by save id: the one that was active when it was last played
/// with the Studio open, or, for a save played without it, a profile whose mods are exactly the
/// save's. A save with neither belongs to none (it is not guessed).
pub fn save_profiles(user_zomboid_dir: &Path) -> std::collections::BTreeMap<String, String> {
    let mut refs = save_references(user_zomboid_dir);
    add_server_sessions(user_zomboid_dir, &mut refs);
    let profiles = crate::instance_manager::list_instances(user_zomboid_dir.to_string_lossy().to_string()).unwrap_or_default();
    let plain = |id: &String| crate::load_order::mod_info::sanitize_mod_id(id).rsplit('/').next().unwrap_or_default().to_string();
    let mut out = std::collections::BTreeMap::new();
    for id in crate::save_backup::list_saves(user_zomboid_dir) {
        if let Some(profile) = refs.get(&id).and_then(|r| r.profile.clone()) {
            out.insert(id, profile);
            continue;
        }
        let dir = crate::save_backup::saves_dir(user_zomboid_dir).join(&id);
        let mods: std::collections::BTreeSet<String> = crate::save_backup::save_mods(&dir).iter().map(plain).collect();
        if mods.is_empty() {
            continue;
        }
        let same = profiles.iter().find(|p| {
            let theirs: std::collections::BTreeSet<String> = p.active_mod_ids.iter().map(plain).collect();
            theirs == mods && theirs.len() == p.active_mod_ids.len()
        });
        if let Some(p) = same {
            out.insert(id, p.id.clone());
        }
    }
    out
}

pub fn saves_file_text(user_zomboid_dir: &Path) -> String {
    let paths = crate::vfs::studio_paths();
    let owners = save_profiles(user_zomboid_dir);
    let by_mod = if paths.workshop_dir.is_empty() { Default::default() } else { workshop_ids_by_mod(Path::new(&paths.workshop_dir)) };
    let state = (!paths.workshop_dir.is_empty()).then(|| crate::workshop::local_state(&paths.workshop_dir)).flatten();
    let refs = save_references(user_zomboid_dir);
    let mut out = String::new();
    for id in crate::save_backup::list_saves(user_zomboid_dir) {
        let Some(summary) = crate::save_backup::summary(user_zomboid_dir, &id) else { continue };
        let session = refs.get(&id);
        let updated = session.map(|s| updated_since(s, &by_mod, state.as_ref())).unwrap_or_default();
        out.push_str(&format!(
            "save\t{}\tbacked={}\tbytes={}\tsession={}\tupdated={}\tupdatedmods={}\tmods={}\tprofile={}\tmodified={}\n",
            id,
            u8::from(summary.backed_up),
            summary.bytes,
            session.map(|s| s.at).unwrap_or(0),
            updated.len(),
            updated.join(";"),
            session.map(|s| s.mods.join(";")).unwrap_or_default(),
            owners.get(&id).map(String::as_str).unwrap_or(""),
            summary.modified
        ));
    }
    out
}

/// The mods updated on the Workshop since the last session, and the ones Steam has a newer
/// version of waiting, for the Link's Mods screen ("Updated" filter): `updated=<mod id>` and
/// `waiting=<mod id>` lines. Nothing leaves the PC (Steam's own record on disk).
pub const UPDATED_FILE: &str = "PZModStudio_LinkUpdated.txt";

pub fn updated_file_text(items: &[crate::whats_new::UpdatedItem]) -> String {
    let mut out = String::new();
    // (a local mod with new changelog entries was not updated on the Workshop)
    for item in items.iter().filter(|i| !i.local) {
        let key = if item.pending { "waiting" } else if item.since_last_play { "updated" } else { continue };
        for id in &item.mod_ids {
            out.push_str(&format!("{}={}\n", key, crate::load_order::mod_info::sanitize_mod_id(id)));
        }
    }
    out
}

pub fn write_updated_file(user_zomboid_dir: &Path) {
    let lua_dir = user_zomboid_dir.join("Lua");
    let workshop = crate::vfs::studio_paths().workshop_dir;
    if !lua_dir.is_dir() || workshop.is_empty() {
        return;
    }
    let news = crate::whats_new::whats_new(user_zomboid_dir, &workshop, false, None);
    let text = updated_file_text(&news.items);
    let path = lua_dir.join(UPDATED_FILE);
    if std::fs::read_to_string(&path).is_ok_and(|now| now == text) {
        return;
    }
    let _ = crate::load_order::ini_parser::atomic_write(path, text);
}

pub fn write_saves_file(user_zomboid_dir: &Path) {
    let lua_dir = user_zomboid_dir.join("Lua");
    if !lua_dir.is_dir() {
        return;
    }
    let path = lua_dir.join(SAVES_FILE);
    let text = saves_file_text(user_zomboid_dir);
    if std::fs::read_to_string(&path).is_ok_and(|now| now == text) {
        return;
    }
    let _ = crate::load_order::ini_parser::atomic_write(path, text);
}

// The game's answers file is written from the watcher and from backup threads.
static ACKS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn write_acks(lua_dir: &Path, answers: &[(String, bool, String)]) {
    if !answers.iter().any(|(id, _, _)| !id.is_empty()) {
        return;
    }
    let _guard = ACKS_LOCK.lock();
    let acks = lua_dir.join(ACKS_FILE);
    let previous = std::fs::read_to_string(&acks).unwrap_or_default();
    let _ = crate::load_order::ini_parser::atomic_write(&acks, acks_text(&previous, &answers));
}

/// Copies a save on a thread of its own (with its session, so the copy compares the same way),
/// rewrites the saves file and answers the request with the copy's world folder.
pub fn start_copy<R: tauri::Runtime>(app: &tauri::AppHandle<R>, z_dir: &Path, save_id: &str, request: String) {
    let app = app.clone();
    let z_dir = z_dir.to_path_buf();
    let save_id = save_id.to_string();
    thread::spawn(move || {
        let result = crate::save_backup::copy_save(&z_dir, &save_id);
        if let Ok(copy) = &result {
            if let Some(session) = save_references(&z_dir).get(&save_id).cloned() {
                let _ = record_save_reference(&z_dir, &GameState { protocol: PROTOCOL, state: "ingame".into(), save: copy.clone(), mods: session.mods, at: session.at, ..Default::default() });
            }
        }
        write_saves_file(&z_dir);
        let _ = app.emit(SAVES_CHANGED_EVENT, &save_id);
        let answer = match result {
            Ok(copy) => (request, true, copy.rsplit('/').next().unwrap_or(&copy).to_string()),
            Err(e) => (request, false, e),
        };
        write_acks(&z_dir.join("Lua"), &[answer]);
    });
}

/// Backs a save up on a thread of its own; when done, the saves file is rewritten first (so a
/// Link that reloads Lua finds the save backed up) and then the request, if any, is answered.
pub fn start_backup<R: tauri::Runtime>(app: &tauri::AppHandle<R>, z_dir: &Path, save_id: &str, reason: &'static str, request: Option<String>) {
    let app = app.clone();
    let z_dir = z_dir.to_path_buf();
    let save_id = save_id.to_string();
    thread::spawn(move || {
        let result = crate::save_backup::backup_save(&z_dir, &save_id, reason, false);
        if let Err(e) = &result {
            crate::diagnostics::error(format!("[StudioLink] could not back up {}: {}", save_id, e));
        }
        write_saves_file(&z_dir);
        let _ = app.emit(SAVES_CHANGED_EVENT, &save_id);
        if let Some(id) = request {
            let answer = match result {
                Ok(_) => (id, true, String::new()),
                Err(e) => (id, false, e),
            };
            write_acks(&z_dir.join("Lua"), &[answer]);
        }
    });
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Mod Health's warnings for the game's Mods screen, one per line: `<severity>\t<mod id>\t<text>`,
/// written by the app (already in the player's language) after each check.
pub const HEALTH_FILE: &str = "PZModStudio_LinkHealth.txt";

pub fn write_health_file(user_zomboid_dir: &Path, lines: &[(String, String, String)]) -> Result<(), String> {
    let lua_dir = user_zomboid_dir.join("Lua");
    std::fs::create_dir_all(&lua_dir).map_err(|e| format!("Cannot create {}: {}", lua_dir.display(), e))?;
    let clean = |s: &str| s.replace(['\t', '\r', '\n'], " ");
    let text: String = lines
        .iter()
        .map(|(severity, id, msg)| format!("{}\t{}\t{}\n", clean(severity), clean(id), clean(msg)))
        .collect();
    let path = lua_dir.join(HEALTH_FILE);
    if std::fs::read_to_string(&path).is_ok_and(|now| now == text) {
        return Ok(());
    }
    crate::load_order::ini_parser::atomic_write(&path, text).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

// ------------------------------------------------------------------ the game's mod list

/// What the game's Mods screen cannot tell by itself, for its filters: each mod's origin, whether
/// it is a library, its tags, and their words in the player's language (made by the app).
pub const MODS_FILE: &str = "PZModStudio_LinkMods.txt";

pub fn write_mods_file(user_zomboid_dir: &Path, text: &str) -> Result<(), String> {
    let lua_dir = user_zomboid_dir.join("Lua");
    std::fs::create_dir_all(&lua_dir).map_err(|e| format!("Cannot create {}: {}", lua_dir.display(), e))?;
    let path = lua_dir.join(MODS_FILE);
    if std::fs::read_to_string(&path).is_ok_and(|now| now == text) {
        return Ok(());
    }
    crate::load_order::ini_parser::atomic_write(&path, text).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

static WATCHER_STARTED: AtomicBool = AtomicBool::new(false);

/// app -> game: `mods/default.txt` as it is, copied where the game's Lua can read it. The game
/// keeps the active mods it read when it started; when this copy changes while it runs (a profile
/// switched, a mod turned on in the app, a fix package added) the Link takes the new list.
pub const ACTIVE_FILE: &str = "PZModStudio_LinkActive.txt";

/// Copies `default.txt` for the Link, only whole (its mods block closed) and read the same twice
/// (the game writes it in place). False when it should be tried again on the next round.
pub fn mirror_active_list(z_dir: &Path) -> bool {
    let lua = z_dir.join("Lua");
    let path = z_dir.join("mods").join("default.txt");
    if !lua.is_dir() || !path.is_file() {
        return true;
    }
    let Ok(first) = std::fs::read_to_string(&path) else { return false };
    if crate::load_order::ini_parser::try_parse_default_txt(&path).is_none() {
        return false;
    }
    thread::sleep(Duration::from_millis(200));
    if std::fs::read_to_string(&path).ok().as_ref() != Some(&first) {
        return false;
    }
    let file = lua.join(ACTIVE_FILE);
    if std::fs::read_to_string(&file).is_ok_and(|t| t == first) {
        return true;
    }
    crate::load_order::ini_parser::atomic_write(file, first).is_ok()
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Follows `mods/default.txt`. When it changes and no longer matches the active profile — the
/// player accepted the game's Mods screen — the profile takes what the game wrote and the window
/// is told to show it. The Studio's own writes match the profile and are let through.
pub fn watch_mod_list<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    if WATCHER_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    thread::spawn(move || {
        let mut last_seen: Option<(PathBuf, SystemTime)> = None;
        let mut game_seen: Option<SystemTime> = None;
        // The default.txt the Link's copy was made from.
        let mut mirrored: Option<SystemTime> = None;
        let mut app_written = 0u64;
        // The save the player is in, as the game file last said; backed up when they leave it.
        let mut playing: Option<String> = None;
        let mut saves_written = 0u64;
        let mut ticks = 0u64;
        // A Link installed by an older Studio is brought up to this one's version at once, not
        // only when a profile is next applied.
        let mut link_checked = false;
        // When the game was seen starting: a server's save changed after it is backed up when the
        // game closes (the Link does not run on a server, so it never says it left one).
        let mut game_since: Option<SystemTime> = None;
        // The game just closed: Windows' memory event, if any, lands a few seconds later.
        let mut crash_check: Option<u8> = None;
        loop {
            ticks += 1;
            thread::sleep(Duration::from_secs(2));
            let chosen = crate::vfs::saved_user_zomboid_dir();
            let Some(z_dir) = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&chosen).into_iter().next() else {
                continue;
            };
            let lua_dir = z_dir.join("Lua");
            if !link_checked {
                link_checked = true;
                if link_needs_update(&z_dir) {
                    if let Err(e) = install_link_mod(&z_dir) {
                        crate::diagnostics::error(format!("[StudioLink] {}", e));
                    }
                }
            }

            // The app is here: the game counts it gone when this is some seconds old.
            let now = now_secs();
            if now.saturating_sub(app_written) >= 4 && lua_dir.is_dir() {
                if crate::load_order::ini_parser::atomic_write(lua_dir.join(APP_FILE), app_file_text(now)).is_ok() {
                    app_written = now;
                }
            }

            // What the player asked for in the game, answered one by one.
            let requests = lua_dir.join(REQUESTS_FILE);
            if let Ok(text) = std::fs::read_to_string(&requests) {
                let _ = std::fs::remove_file(&requests);
                let answers = handle_requests(&app, &z_dir, &parse_requests(&text));
                write_acks(&lua_dir, &answers);
            }

            // Where the player is: a game entered becomes its save's reference.
            let game_file = lua_dir.join(GAME_FILE);
            let stamp = modified(&game_file);
            if stamp.is_some() && stamp != game_seen {
                game_seen = stamp;
                if let Some(state) = std::fs::read_to_string(&game_file).ok().and_then(|t| parse_game_state(&t)) {
                    if let Err(e) = record_save_reference(&z_dir, &state) {
                        crate::diagnostics::error(format!("[StudioLink] could not note the save's mods: {}", e));
                    }
                    if state.state == "ingame" && !state.save.is_empty() {
                        playing = Some(state.save.clone());
                    } else if state.state == "menu" {
                        // Back on the menu: the save just left is backed up (if it changed).
                        if let Some(save) = playing.take() {
                            start_backup(&app, &z_dir, &save, crate::save_backup::REASON_EXIT, None);
                        }
                    }
                    saves_written = 0;
                    let _ = app.emit(GAME_STATE_EVENT, state);
                }
            }
            // A launch-file change left for when the game is closed.
            if ticks % 5 == 0 {
                if let Some(changes) = finish_pending_agents(&z_dir) {
                    let _ = app.emit(AGENTS_SYNCED_EVENT, changes);
                }
            }
            // The game closed while in a save (quit to desktop, or a crash).
            if playing.is_some() && ticks % 5 == 0 && crate::sandbox::find_running_pz_pid().is_none() {
                if let Some(save) = playing.take() {
                    start_backup(&app, &z_dir, &save, crate::save_backup::REASON_EXIT, None);
                }
            }
            if ticks % 15 == 0 {
                match (crate::sandbox::find_running_pz_pid().is_some(), game_since) {
                    (true, None) => game_since = Some(SystemTime::now()),
                    (false, Some(since)) => {
                        game_since = None;
                        crash_check = Some(0);
                        for save in server_saves_changed_since(&z_dir, since) {
                            start_backup(&app, &z_dir, &save, crate::save_backup::REASON_EXIT, None);
                        }
                    }
                    _ => {}
                }
            }
            // Four looks over a minute: Windows writes its low-memory event moments after the crash.
            if let Some(tries) = crash_check {
                if ticks % 5 == 0 {
                    match crate::sandbox::windows_memory::event_behind_exit() {
                        Some(event) => {
                            let _ = app.emit("sandbox-error-card", crate::sandbox::windows_memory::low_memory_card(&event));
                            crash_check = None;
                        }
                        None if tries >= 3 => crash_check = None,
                        None => crash_check = Some(tries + 1),
                    }
                }
            }
            // What the Link needs to know of each save, about once a minute.
            if now.saturating_sub(saves_written) >= 60 {
                write_saves_file(&z_dir);
                write_updated_file(&z_dir);
                saves_written = now;
            }

            let default_txt = z_dir.join("mods").join("default.txt");
            let Some(stamp) = modified(&default_txt) else { continue };
            if mirrored != Some(stamp) && mirror_active_list(&z_dir) {
                mirrored = Some(stamp);
            }
            let changed = match &last_seen {
                Some((path, seen)) => path != &default_txt || *seen != stamp,
                None => false, // the file as it was when the Studio opened is not a change
            };
            last_seen = Some((default_txt.clone(), stamp));
            if !changed || crate::load_order::ini_parser::is_own_write(&default_txt) {
                continue;
            }
            // The game writes the file in place: read it twice, a moment apart, and take it only
            // when it is whole and the two agree. Otherwise look again on the next round, rather
            // than give the profile a list cut short (or "nothing on" for a file not really read).
            let first = crate::load_order::ini_parser::try_parse_default_txt(&default_txt);
            thread::sleep(Duration::from_millis(300));
            let second = crate::load_order::ini_parser::try_parse_default_txt(&default_txt);
            let game_active = match (first, second) {
                (Some(a), Some(b)) if a == b => a,
                _ => {
                    last_seen = Some((default_txt.clone(), SystemTime::UNIX_EPOCH));
                    continue;
                }
            };
            match crate::instance_manager::take_mod_list_from_game(&z_dir, &game_active) {
                Ok(Some(profile)) => {
                    let _ = app.emit(PROFILE_CHANGED_EVENT, profile);
                }
                Ok(None) => {}
                Err(e) => crate::diagnostics::error(format!("[StudioLink] could not update the profile from the game: {}", e)),
            }
        }
    });
}

// ------------------------------------------------------------------ Java agents

/// A Java agent some mod puts in `ProjectZomboid64.json`, and the mod it belongs to.
struct JavaAgent {
    mod_id: &'static str,
    vm_arg: &'static str,
}

const JAVA_AGENTS: &[JavaAgent] = &[JavaAgent { mod_id: "ZombieBuddy", vm_arg: "-agentlib:zbNative" }];

/// Where the Studio notes the agent lines it took out, so it only ever puts back its own.
fn removed_agents_file(user_zomboid_dir: &Path) -> PathBuf {
    user_zomboid_dir.join("PZModStudio_JavaAgents.json")
}

fn read_removed(user_zomboid_dir: &Path) -> Vec<String> {
    std::fs::read_to_string(removed_agents_file(user_zomboid_dir))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
        .unwrap_or_default()
}

/// What changed in `ProjectZomboid64.json`, for the log.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct AgentChanges {
    pub removed: Vec<String>,
    pub restored: Vec<String>,
    /// The launch lines themselves (`-agentlib:zbNative`), taken out and put back.
    #[serde(default)]
    pub removed_args: Vec<String>,
    #[serde(default)]
    pub restored_args: Vec<String>,
    /// The launch file.
    #[serde(default)]
    pub file: String,
}

/// `<Zomboid>/PZModStudio_JavaAgentsPending.txt`: the launch file needs the active list's Java
/// agents, but the game was running (it keeps `ProjectZomboid64.json` open, so Windows refuses
/// the write). The watcher does it once the game has closed.
pub const AGENTS_PENDING_FILE: &str = "PZModStudio_JavaAgentsPending.txt";
/// Sent to the window when a change left for later was made: `AgentChanges`.
pub const AGENTS_SYNCED_EVENT: &str = "java-agents-synced";

/// The Java agents for the active list now, or once the game has closed if it is running.
/// Returns None when it was left for later.
pub fn sync_java_agents_when_free(pz_install_dir: &Path, user_zomboid_dir: &Path, active_mods: &[String]) -> Result<Option<AgentChanges>, String> {
    let pending = user_zomboid_dir.join(AGENTS_PENDING_FILE);
    if crate::sandbox::find_running_pz_pid().is_some() {
        let _ = crate::load_order::ini_parser::atomic_write(&pending, "The game was running when the profile changed.\n");
        return Ok(None);
    }
    let changes = sync_java_agents(pz_install_dir, user_zomboid_dir, active_mods)?;
    let _ = std::fs::remove_file(&pending);
    Ok(Some(changes))
}

/// The change left for later, once the game is closed, from the list the game reads. Nothing
/// is done while `default.txt` cannot be read whole: an empty list would take the agents out.
fn finish_pending_agents(user_zomboid_dir: &Path) -> Option<AgentChanges> {
    if !user_zomboid_dir.join(AGENTS_PENDING_FILE).is_file() || crate::sandbox::find_running_pz_pid().is_some() {
        return None;
    }
    let install = crate::vfs::studio_paths().pz_install_dir;
    if install.is_empty() {
        return None;
    }
    let active = crate::load_order::ini_parser::try_parse_default_txt(&user_zomboid_dir.join("mods").join("default.txt"))?;
    match sync_java_agents_when_free(Path::new(&install), user_zomboid_dir, &active) {
        Ok(changes) => changes,
        Err(e) => {
            crate::diagnostics::error(format!("[StudioLink] {}", e));
            None
        }
    }
}

/// Puts each mod's Java agent line in `ProjectZomboid64.json` when the mod is on and takes it out
/// when it is off. A line is only put back if the Studio took it out (the mod's own installer is
/// what adds it the first time), and every change keeps a copy of the file before it in
/// `<Zomboid>/PZModStudio_Backups`.
pub fn sync_java_agents(pz_install_dir: &Path, user_zomboid_dir: &Path, active_mods: &[String]) -> Result<AgentChanges, String> {
    java_agents_change(pz_install_dir, user_zomboid_dir, active_mods, true)
}

/// What `sync_java_agents` would change for this list, without changing anything: shown before
/// a profile is switched on.
pub fn plan_java_agents(pz_install_dir: &Path, user_zomboid_dir: &Path, active_mods: &[String]) -> Result<AgentChanges, String> {
    java_agents_change(pz_install_dir, user_zomboid_dir, active_mods, false)
}

fn java_agents_change(pz_install_dir: &Path, user_zomboid_dir: &Path, active_mods: &[String], write: bool) -> Result<AgentChanges, String> {
    let json_path = pz_install_dir.join("ProjectZomboid64.json");
    let Ok(text) = std::fs::read_to_string(&json_path) else {
        return Ok(AgentChanges::default());
    };
    let mut json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{} is not valid JSON: {}", json_path.display(), e))?;
    let Some(vm_args) = json.get_mut("vmArgs").and_then(|v| v.as_array_mut()) else {
        return Ok(AgentChanges::default());
    };

    let mut removed_by_studio = read_removed(user_zomboid_dir);
    let mut changes = AgentChanges { file: json_path.to_string_lossy().to_string(), ..Default::default() };
    for agent in JAVA_AGENTS {
        let wanted = active_mods.iter().any(|m| m == agent.mod_id);
        let present = vm_args.iter().any(|a| a.as_str() == Some(agent.vm_arg));
        if !wanted && present {
            vm_args.retain(|a| a.as_str() != Some(agent.vm_arg));
            if !removed_by_studio.iter().any(|r| r == agent.vm_arg) {
                removed_by_studio.push(agent.vm_arg.to_string());
            }
            changes.removed.push(agent.mod_id.to_string());
            changes.removed_args.push(agent.vm_arg.to_string());
        } else if wanted && !present && removed_by_studio.iter().any(|r| r == agent.vm_arg) {
            // First, where its installer had it: agents load before the other VM options.
            vm_args.insert(0, serde_json::Value::String(agent.vm_arg.to_string()));
            removed_by_studio.retain(|r| r != agent.vm_arg);
            changes.restored.push(agent.mod_id.to_string());
            changes.restored_args.push(agent.vm_arg.to_string());
        }
    }
    if !write || (changes.removed.is_empty() && changes.restored.is_empty()) {
        return Ok(changes);
    }

    // Two-space indent, as the game ships it.
    let new_text = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::write_file_with_backup(&json_path, &new_text, &user_zomboid_dir.join("PZModStudio_Backups"))?;
    let _ = crate::load_order::ini_parser::atomic_write(
        removed_agents_file(user_zomboid_dir),
        serde_json::to_string_pretty(&removed_by_studio).unwrap_or_default(),
    );
    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_game_order_fills_the_places_of_the_active_mods() {
        let previous = ids(&["A", "x", "B", "y", "C"]);
        // The game moved C before A and switched B off.
        let merged = merge_game_order(&previous, &ids(&["C", "A"]));
        assert_eq!(merged, ids(&["C", "x", "B", "y", "A"]));
        // A mod switched on that the profile had never placed goes last.
        let merged = merge_game_order(&previous, &ids(&["A", "B", "C", "N"]));
        assert_eq!(merged, ids(&["A", "x", "B", "y", "C", "N"]));
    }

    #[test]
    fn the_link_file_lists_the_profile_and_its_mods() {
        let text = link_file_text("inst_1", "Vanilla+\nx", &ids(&["A", "B"]), &ids(&["B", "C"]));
        assert_eq!(text, "profile=Vanilla+ x\nprofileid=inst_1\nmember=A\nmember=B\nother=C\n");
    }

    #[test]
    fn requests_from_the_game_are_read_once_each() {
        let args = |text: &str| parse_requests(text).into_iter().map(|r| (r.id, r.verb, r.arg)).collect::<Vec<_>>();
        let s = |a: &str, b: &str, c: &str| (a.to_string(), b.to_string(), c.to_string());
        // The first protocol's lines, which carry no id.
        assert_eq!(args("add=A\r\nadd= B \nnoise\nadd=A\nadd=\n"), vec![s("", "add", "A"), s("", "add", "B")]);
        // The second's, mixed with an old one asking for the same thing.
        assert_eq!(
            args("17-1\tadd\tC\r\nadd=C\n17-2\tbisect\t\n\n"),
            vec![s("17-1", "add", "C"), s("17-2", "bisect", "")]
        );
    }

    #[test]
    fn every_link_text_has_its_translations() {
        let start = CLIENT_LUA.find("local TEXT = {").unwrap();
        let block = &CLIENT_LUA[start..start + CLIENT_LUA[start..].find("\n}").unwrap()];
        let mut keys: Vec<String> = block
            .lines()
            .filter_map(|l| l.trim().split_once(" = \"").map(|(k, _)| format!("UI_StudioLink_{}", k)))
            .collect();
        keys.sort();
        assert!(keys.len() > 10);
        for json in [UI_EN, UI_ES] {
            let map: std::collections::BTreeMap<String, String> = serde_json::from_str(json).unwrap();
            assert_eq!(map.keys().cloned().collect::<Vec<_>>(), keys);
            assert!(map.values().all(|v| !v.contains('%')), "a % would be read by the game as a format");
        }
        // The same {1}, {2}... in both languages, line by line.
        let marks = |v: &str| {
            let mut m: Vec<String> = v.match_indices('{').filter_map(|(i, _)| v[i..].find('}').map(|j| v[i..i + j + 1].to_string())).collect();
            m.sort();
            m
        };
        let en: std::collections::BTreeMap<String, String> = serde_json::from_str(UI_EN).unwrap();
        let es: std::collections::BTreeMap<String, String> = serde_json::from_str(UI_ES).unwrap();
        for (k, v) in &en {
            assert_eq!(marks(v), marks(&es[k]), "placeholders of {}", k);
        }
    }

    #[test]
    fn a_session_counts_the_mods_published_after_it() {
        let session = SaveReference { mods: ids(&["A", "\\B", "C", "Local"]), at: 1000, ..Default::default() };
        let by_mod: std::collections::HashMap<String, String> =
            [("A", "1"), ("B", "2"), ("C", "3")].into_iter().map(|(m, w)| (m.to_string(), w.to_string())).collect();
        let item = |t: u64| crate::workshop::InstalledItemState { installed_time: Some(t), latest_time: Some(t), size: None, update_pending: false, updated_since_last_play: false };
        let state = crate::workshop::WorkshopLocalState {
            time_last_app_ran: None,
            items: [("1".to_string(), item(900)), ("2".to_string(), item(1500)), ("3".to_string(), item(2000))].into_iter().collect(),
        };
        assert_eq!(updated_since(&session, &by_mod, Some(&state)), ids(&["\\B", "C"]));
        assert!(updated_since(&session, &by_mod, None).is_empty());
        let diff = session_diff(&session, &ids(&["C", "A", "New"]), vec![]);
        assert_eq!((diff.added.clone(), diff.removed.clone(), diff.reordered), (ids(&["New"]), ids(&["B", "Local"]), true));
        assert!(!session_diff(&session, &ids(&["A", "B", "C", "Local"]), vec![]).reordered);
    }

    #[test]
    fn the_updated_file_names_updated_and_waiting_mods() {
        let item = |ids: &[&str], pending: bool, since: bool| crate::whats_new::UpdatedItem {
            workshop_id: "1".into(),
            title: "t".into(),
            mod_ids: ids.iter().map(|s| s.to_string()).collect(),
            active: true,
            profiles: Vec::new(),
            installed_time: None,
            latest_time: None,
            pending,
            since_last_play: since,
            map_mod: false,
            notes: Vec::new(),
            notes_error: None,
            risks: Vec::new(),
            changelog: Vec::new(),
            changelog_file: None,
            support: Vec::new(),
            read_in_game: false,
            local: false,
        };
        let text = updated_file_text(&[item(&["A", "B"], false, true), item(&["C"], true, true), item(&["D"], false, false)]);
        assert_eq!(text, "updated=A
updated=B
waiting=C
");
    }

    #[test]
    fn versions_compare_as_numbers() {
        assert_eq!(mod_version("name=x\nmodversion=1.10.2\n"), vec![1, 10, 2]);
        assert!(mod_version("modversion=1.10.0") > mod_version("modversion=1.9.9"));
        assert!(mod_version("modversion=1.7") < mod_version("modversion=1.7.0"));
        assert!(!bundled_version().is_empty(), "the bundled mod.info has a version");
    }

    #[test]
    fn the_link_lua_uses_only_what_the_games_lua_has() {
        // Kahlua has no `next` (seen live: "Object tried to call nil"), and no goto or integer
        // division.
        // Code only: comments and the text inside "quotes" (a URL has "//") left out.
        let code = |line: &str| -> String {
            let mut out = String::new();
            let mut quoted = false;
            let mut chars = line.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '"' {
                    quoted = !quoted;
                } else if c == '\\' && quoted {
                    chars.next();
                } else if !quoted {
                    if c == '-' && chars.peek() == Some(&'-') {
                        break;
                    }
                    out.push(c);
                }
            }
            out
        };
        for (file, source) in [("PZModStudio_Link.lua", CLIENT_LUA), ("PZModStudio_ModsScreen.lua", SCREEN_LUA), ("PZModStudio_Errors.lua", ERRORS_LUA)] {
            for missing in ["next(", "goto ", "//"] {
                let used = source.lines().any(|l| code(l).contains(missing));
                assert!(!used, "{} uses {}", file, missing);
            }
        }
        assert!(code(r#"x = "a//b" -- c // d"#).trim() == "x =", "quotes and comments are not code");
        assert!(code("y = 7 // 2").contains("//"), "integer division in code is found");
    }

    #[test]
    fn answers_keep_the_newest_and_skip_requests_without_an_id() {
        let mut text = String::new();
        for i in 0..60 {
            text = acks_text(&text, &[(format!("r{}", i), i % 2 == 0, format!("line\twith\ntabs {}", i))]);
        }
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), ACKS_KEPT);
        assert_eq!(lines[0], "r10\tok\tline with tabs 10");
        assert_eq!(*lines.last().unwrap(), "r59\terror\tline with tabs 59");
        assert_eq!(acks_text("", &[(String::new(), true, String::new())]), "");
        assert!(app_file_text(42).contains("alive=42\n") && app_file_text(42).starts_with("protocol=2\n"));
    }

    #[test]
    fn a_game_entered_becomes_its_saves_reference() {
        let z = temp("saverefs");
        let text = "protocol=2\nlink=1.1.0\nstate=ingame\nsave=Sandbox/World 1\nmod=A\nmod=B\nat=100\n";
        let state = parse_game_state(text).unwrap();
        assert_eq!(state.mods, ids(&["A", "B"]));
        assert_eq!(state.save, "Sandbox/World 1");
        assert!(record_save_reference(&z, &state).unwrap());
        // The same list again is not a change; the menu never is.
        assert!(!record_save_reference(&z, &GameState { at: 200, ..state.clone() }).unwrap());
        assert!(!record_save_reference(&z, &parse_game_state("state=menu\n").unwrap()).unwrap());
        assert_eq!(save_references(&z)["Sandbox/World 1"], SaveReference { mods: ids(&["A", "B"]), at: 100, ..Default::default() });
        assert!(parse_game_state("noise\n").is_none());
        let _ = std::fs::remove_dir_all(&z);
    }

    #[test]
    fn a_session_notes_the_profile_that_was_active() {
        let z = temp("saverefs-profile");
        std::fs::create_dir_all(z.join("PZModStudio_Instances")).unwrap();
        std::fs::write(
            z.join("PZModStudio_Instances").join("inst_1.json"),
            r#"{"id":"inst_1","name":"Main","created_at":"0","is_active":true,"active_mod_ids":["A"],"load_order":["A"]}"#,
        )
        .unwrap();
        let state = parse_game_state("protocol=2\nstate=ingame\nsave=Sandbox/W\nmod=A\nat=5\n").unwrap();
        assert!(record_save_reference(&z, &state).unwrap());
        let r = &save_references(&z)["Sandbox/W"];
        assert_eq!((r.profile.as_deref(), r.profile_name.as_deref()), (Some("inst_1"), Some("Main")));
        let _ = std::fs::remove_dir_all(&z);
    }

    #[test]
    fn a_server_save_belongs_to_the_profile_its_server_was_joined_with() {
        let z = temp("server-save");
        std::fs::create_dir_all(z.join("PZModStudio_Instances")).unwrap();
        std::fs::write(
            z.join("PZModStudio_Instances").join("inst_9.json"),
            r#"{"id":"inst_9","name":"Online","created_at":"0","is_active":true,"active_mod_ids":["A"],"load_order":["A"]}"#,
        )
        .unwrap();
        let world = "208.75.182.221_27955_f5e85281397522383aa0b8b39114e853";
        std::fs::create_dir_all(z.join("Saves").join("Multiplayer").join(world)).unwrap();
        std::fs::create_dir_all(z.join("Saves").join("Multiplayer").join("servertest")).unwrap();
        std::fs::write(z.join("PZModStudio_Servers.json"), r#"[{"name":"S","address":"208.75.182.221:27955","profile_id":"inst_9","last_joined":77}]"#).unwrap();
        let id = format!("Multiplayer/{}", world);
        assert_eq!(server_address_of(&id).as_deref(), Some("208.75.182.221:27955"));
        assert_eq!(server_address_of("Multiplayer/servertest"), None);
        assert_eq!(server_address_of("Sandbox/1.2.3.4_16261_f5e85281397522383aa0b8b39114e853"), None);
        let diffs = session_diffs(&z);
        assert_eq!((diffs[&id].profile.as_deref(), diffs[&id].profile_name.as_deref(), diffs[&id].at), (Some("inst_9"), Some("Online"), 77));
        assert!(!diffs.contains_key("Multiplayer/servertest"));
        assert_eq!(save_profiles(&z).get(&id).map(String::as_str), Some("inst_9"));
        let _ = std::fs::remove_dir_all(&z);
    }

    #[test]
    fn a_launch_asks_only_for_a_save_on_disk_and_never_a_servers() {
        let z = temp("load");
        std::fs::create_dir_all(z.join("Lua")).unwrap();
        std::fs::create_dir_all(z.join("Saves").join("Sandbox").join("W")).unwrap();
        std::fs::create_dir_all(z.join("Saves").join("Multiplayer").join("1.2.3.4_16261_f5e85281397522383aa0b8b39114e853")).unwrap();
        write_load_file(&z, Some("Sandbox/W")).unwrap();
        let text = std::fs::read_to_string(z.join("Lua").join(LOAD_FILE)).unwrap();
        assert!(text.starts_with("save=Sandbox/W
at="));
        assert!(write_load_file(&z, Some("Sandbox/Nope")).is_err());
        assert!(write_load_file(&z, Some("Multiplayer/1.2.3.4_16261_f5e85281397522383aa0b8b39114e853")).is_err());
        assert!(write_load_file(&z, Some("../x")).is_err());
        write_load_file(&z, None).unwrap();
        assert_eq!(std::fs::read_to_string(z.join("Lua").join(LOAD_FILE)).unwrap(), "");
        let _ = std::fs::remove_dir_all(&z);
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pzms-link-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_link_gets_a_copy_of_the_active_list_only_when_whole() {
        let z = temp("active");
        std::fs::create_dir_all(z.join("Lua")).unwrap();
        std::fs::create_dir_all(z.join("mods")).unwrap();
        let copy = z.join("Lua").join(ACTIVE_FILE);
        std::fs::write(z.join("mods").join("default.txt"), "VERSION = 1,\nmods\n{\n    mod = \\A,\n").unwrap();
        assert!(!mirror_active_list(&z), "a file cut short is tried again");
        assert!(!copy.exists());
        let whole = "VERSION = 1,\nmods\n{\n    mod = \\A,\n}\nmaps\n{\n    map = Muldraugh, KY,\n}\n";
        std::fs::write(z.join("mods").join("default.txt"), whole).unwrap();
        assert!(mirror_active_list(&z));
        assert_eq!(std::fs::read_to_string(&copy).unwrap(), whole);
        let _ = std::fs::remove_dir_all(&z);
    }

    #[test]
    fn an_older_installed_link_is_brought_up_to_date() {
        let z = temp("older");
        assert!(!link_needs_update(&z), "not installed: nothing to update");
        let info = z.join("mods").join(LINK_MOD_ID).join("42");
        std::fs::create_dir_all(&info).unwrap();
        std::fs::write(info.join("mod.info"), "id=Z_StudioLink\nmodversion=1.0.0\n").unwrap();
        assert!(link_needs_update(&z));
        install_link_mod(&z).unwrap();
        assert!(!link_needs_update(&z));
        let screen = z.join("mods").join(LINK_MOD_ID).join("42").join("media").join("lua").join("client").join("PZModStudio_ModsScreen.lua");
        std::fs::write(&screen, "-- changed").unwrap();
        assert!(link_needs_update(&z), "a file that differs, same version");
        let _ = std::fs::remove_dir_all(&z);
    }

    #[test]
    fn the_game_can_ask_for_the_window() {
        assert_eq!(parse_requests("9-1\tshow\n9-2\tshow\n"), vec![LinkRequest { id: "9-1".into(), verb: "show".into(), arg: String::new() }]);
    }

    #[test]
    fn zombie_buddy_comes_out_and_back_with_a_backup() {
        let game = temp("game");
        let zomboid = temp("zomboid");
        let json = game.join("ProjectZomboid64.json");
        std::fs::write(&json, r#"{"mainClass":"x","vmArgs":["-agentlib:zbNative","-Xmx6144m"]}"#).unwrap();

        let c = sync_java_agents(&game, &zomboid, &ids(&["Other"])).unwrap();
        assert_eq!(c.removed, ids(&["ZombieBuddy"]));
        let now = std::fs::read_to_string(&json).unwrap();
        assert!(!now.contains("zbNative") && now.contains("-Xmx6144m"));
        assert_eq!(std::fs::read_dir(zomboid.join("PZModStudio_Backups")).unwrap().count(), 1);

        let c = sync_java_agents(&game, &zomboid, &ids(&["ZombieBuddy"])).unwrap();
        assert_eq!(c.restored, ids(&["ZombieBuddy"]));
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(v["vmArgs"][0], "-agentlib:zbNative");

        // Never added on its own: without the Studio having taken it out, it stays out.
        std::fs::write(&json, r#"{"vmArgs":["-Xmx6144m"]}"#).unwrap();
        let _ = std::fs::remove_file(removed_agents_file(&zomboid));
        let c = sync_java_agents(&game, &zomboid, &ids(&["ZombieBuddy"])).unwrap();
        assert!(c.restored.is_empty());
    }
}
