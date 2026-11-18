//! MCP tools to host a dedicated server from a config in `Zomboid/Server/` and to join one with
//! the game: the same launcher as the Servers screen (ZombieBuddy's agent included), with the
//! admin password taken from the Studio's stored value or a file and never sent back.

use super::protocol::McpTool;
use super::protocol::ToolCallResult;
use crate::server_manager::{self, java_mods, mod_sync};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "server_start".to_string(),
            description: "Starts a dedicated server from a config in Zomboid/Server/<name>.ini, the way the Servers screen does: java.exe from the game folder (or from the dedicated server program when dedicated_server chose it: program and program_dir say which), headless, its log in Zomboid/server-console.txt. When Mods= has ZombieBuddy the JVM gets -agentlib:zbNative=policy=deny-new (only Java jars already approved on this PC load; a server cannot show the approval window), when that folder has zbNative.dll; without it the server starts without the agent and will_not_load says so (server_java_no_agent*). The admin password comes from admin_password_file (its first line) or the value the Studio stored the last time this server was started with one; it is never returned. Returns the PID, whether the agent was added, and what the Mods and sync check says will not load (unapproved jars; Workshop mods linked into Zomboid/mods or copied there and outdated; with nosteam, mods only in the Workshop folder: the Servers screen's Mods & Sync button copies them). One server at a time. Poll server_status until started is true (a big mod list takes minutes).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Server config name (Zomboid/Server/<name>.ini), letters, digits, '_' and '-'" },
                    "memory_gb": { "type": "integer", "description": "Java heap (-Xmx) in GB, 2-32 (default 4)" },
                    "nosteam": { "type": "boolean", "description": "Start without Steam (-nosteam, LAN; players join with -nosteam). Default false" },
                    "admin_password_file": { "type": "string", "description": "A file whose first line is the admin password (it is read, never echoed). Default: the Studio's stored value for this server, else none" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                },
                "required": ["name"]
            }),
        },
        McpTool {
            name: "server_stop".to_string(),
            description: "Stops the dedicated server the Studio started (the PID it keeps in Zomboid/pz_server_state.json): \"quit\" over RCON first (saves the world), killed only if it does not go within 90 s. Never touches any other java.exe. Returns whether a server was stopped.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "server_status".to_string(),
            description: "Whether the Studio's dedicated server runs: PID, config name, memory, start time, nosteam, started (its log says SERVER STARTED since it was launched), the port from its .ini, the address to join it from this PC (the LAN address; a Steam client cannot join 127.0.0.1 or this PC's own public IP), and the last lines of Zomboid/server-console.txt.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "log_lines": { "type": "integer", "description": "Lines of server-console.txt to return (default 20, max 200; 0 for none)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "server_join".to_string(),
            description: "Launches the game straight into a server: ProjectZomboid64.exe +connect <ip:port>, windowed. The game opens on its connect window for that address, where the player types the account username and password (a new username makes a new account) and presses Connect. Default address: the running Studio server on this PC (its .ini's DefaultPort) at this PC's LAN address with Steam, 127.0.0.1 without. nosteam defaults to how that server was started (a NoSteam server needs a -nosteam client, a Steam server a Steam client). A Steam client cannot join 127.0.0.1 or this PC's own public IP: use the LAN address. A -debug client may only join as an admin. Refused while the game already runs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "address": { "type": "string", "description": "ip:port or ip (port 16261). Default: this PC's running Studio server" },
                    "nosteam": { "type": "boolean", "description": "Launch with -nosteam (default: how the running server was started, else false)" },
                    "debug": { "type": "boolean", "description": "Launch with -debug (default false; only admins may join with it)" },
                    "extra_args": { "type": "array", "items": { "type": "string" }, "description": "Optional extra game arguments" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "server_schedule".to_string(),
            description: "A Studio server's restarts: without set, reads them; with set, changes them (only the fields given). Scheduled restarts at times of day (times, \"HH:MM\" on this PC's clock) and/or every_hours from the server's start (enabled turns both on); players warned in the game over RCON (servermsg) warn_minutes before (default 30, 15, 5, 1), then save and quit (it saves) and started again the same way. on_crash (default true): started again when it stops by itself after it had come up, at most crash_limit times in crash_window_minutes (default 3 in 10), then left off and said. on_mod_update: every mod_check_minutes it asks Steam's public Web API (GetPublishedFileDetails) for the time_updated of each WorkshopItems= item, and an item updated after the server started gets a warned restart. language: en/es for the in-game messages. All of this runs inside the PZ Mod Studio app only while it is open (app_watching says whether it is); the server itself keeps running when the app is closed. Returns the schedule, next_restart (unix s), the pending restart with its warnings given, the last restarts with their reason (schedule, mod_update, manual, crash, gave_up, crash_no_restart, stopped_while_closed) and gave_up_at. Kept in Zomboid/PZModStudio_ServerSchedule.json.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string", "description": "Server config name (Zomboid/Server/<name>.ini). Default: the running Studio server" },
                    "set": {
                        "type": "object",
                        "description": "Fields to change: enabled (bool), times (array of \"HH:MM\"), every_hours (int, 0 none), warn_minutes (array of int), on_crash (bool), crash_limit (int), crash_window_minutes (int), on_mod_update (bool), mod_check_minutes (int, min 5), language (en/es)"
                    },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "server_restart".to_string(),
            description: "Restarts the running Studio server with in-game warnings: the players are told warn_minutes before (and at the schedule's warnings in between, e.g. 5 and 1), then it saves, quits over RCON and is started again the same way. With the PZ Mod Studio app open the app counts down and restarts (returns at once; follow it with server_schedule). With the app closed only warn_minutes 0 works: restarted now from here (save, quit, start; blocks until it is started). cancel: true calls off a restart counting down (the players are told).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "warn_minutes": { "type": "integer", "description": "Minutes of warning, 0-120 (default 5)" },
                    "cancel": { "type": "boolean", "description": "Call off the restart counting down instead" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "dedicated_server".to_string(),
            description: "The official dedicated server program (Steam app 380870) as an alternative to starting servers from the game's own folder. action info (default): which program starts the Studio's servers (source game/dedicated), the chosen folder and branch, steamcmd.exe when found on this PC, dedicated servers found (the chosen folder, Steam libraries' steamapps/common/Project Zomboid Dedicated Server, beside SteamCMD) with build id, branch and size from SteamCMD's appmanifest_380870.acf, the install job's progress (phase, percent, bytes, folder size, last SteamCMD line, error) and the SteamCMD command an update runs. action set: source (game/dedicated: dedicated needs the program installed there), dir, branch (default: the branch the installed game is on, given as game_branch; public, unstable or another; a branch Steam keeps behind a password is refused with bad_branch), steamcmd (a steamcmd.exe path). action install: installs or updates into dir (an empty folder or an existing dedicated server; not the game's or the Zomboid folder) with SteamCMD: +force_install_dir <dir> +login anonymous +app_update 380870 -beta <branch> [validate] +quit, in the background (poll with info; action cancel stops it). Downloads several GB from Steam. When SteamCMD is not on this PC it is downloaded from Valve's address (https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip, about 3 MB, its Valve signature checked) only with allow_download: true; ask the user first. Refused while the Studio's server runs from that folder.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["info", "set", "install", "cancel"], "description": "Default info" },
                    "source": { "type": "string", "enum": ["game", "dedicated"], "description": "set: which program starts servers" },
                    "dir": { "type": "string", "description": "set/install: the dedicated server's folder" },
                    "branch": { "type": "string", "description": "set/install: public, unstable or another branch name (default: the branch the installed game is on)" },
                    "steamcmd": { "type": "string", "description": "set: path of a steamcmd.exe already on this PC" },
                    "validate": { "type": "boolean", "description": "install: check every file (many minutes for ~7 GB; mends a broken install). Default: true when the folder has no complete install, else false (an update then only fetches what changed)" },
                    "allow_download": { "type": "boolean", "description": "install: download SteamCMD from Valve when it is not on this PC. Default false" }
                }
            }),
        },
        McpTool {
            name: "mp_test_prepare".to_string(),
            description: "Gets the second player's own Zomboid folder (<home>/Zomboid2, beside Zomboid) ready for a multiplayer test on this PC against the server <name>: the player's options.ini the first time, and in Zomboid2/mods every mod of the server's Mods=, copied with the Studio's marker (copied again when its source got newer) or, for a mod in development that is a link elsewhere, linked to the same place. A folder there the Studio did not put is kept. Returns copied, linked, kept and missing (in Mods= but installed nowhere). Two games of one Steam account cannot both join, so the test runs without Steam, and a NoSteam game reads mods only from its own folder's mods.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string", "description": "Server config name (Zomboid/Server/<name>.ini)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                },
                "required": ["server"]
            }),
        },
        McpTool {
            name: "mp_test_start".to_string(),
            description: "Starts a multiplayer test on this PC: the server <name>, then, once its log says SERVER STARTED, one or two games joined to it (windowed). players 1: with Steam (server_start without nosteam; Workshop items it cannot download are left out first), the game joined at this PC's LAN address (a Steam client refuses 127.0.0.1). players 2: without Steam (-nosteam, joined at 127.0.0.1), since two games on one PC cannot share a Steam account: player 1 with the player's Zomboid folder, player 2 with Zomboid2 (run mp_test_prepare first). Each game opens on the server's connect window, where an account name and password are typed (a new name makes an account). In multiplayer each game loads the server's mod list, not the player's profile. Refused while the game or a Studio server already runs. Poll mp_test_status.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string", "description": "Server config name (Zomboid/Server/<name>.ini)" },
                    "players": { "type": "integer", "description": "1 or 2 (default 2)" },
                    "memory_gb": { "type": "integer", "description": "The server's Java heap in GB, 2-32 (default 4)" },
                    "admin_password_file": { "type": "string", "description": "A file whose first line is the admin password (default: the Studio's stored value)" },
                    "auto_login": { "type": "boolean", "description": "Fill in each game's account on the connect window and press Connect (the Studio Link). Default true" },
                    "all_admin": { "type": "boolean", "description": "Player 2 an admin too (given over RCON once its account exists; player 1 signs in as admin). Default true" },
                    "auto_spawn": { "type": "boolean", "description": "Take the first spawn place the server offers. Default true" },
                    "auto_start_click": { "type": "boolean", "description": "Click each game's black 'click to start' screen once its world has loaded. Default true" },
                    "character": { "type": "string", "enum": ["random", "first", "manual"], "description": "A new character: random (first spawn, random traits and look), first (what each screen shows first) or manual (nothing pressed). Default random. Needs the Studio Link in the server's Mods= (added with a backup)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                },
                "required": ["server"]
            }),
        },
        McpTool {
            name: "mp_test_status".to_string(),
            description: "The multiplayer test as it is now: every server and game process on this PC with role, PID, name (server config, or player 1/2), its Zomboid folder and log, RAM used and its Java heap limit, CPU %, state (server: starting/started; game: menu/connecting/in_game, from its console.txt), ERROR lines in its log, the address it joins; the PC's total and free memory; the second player's folder and whether it is ready; the started test's phase (waiting_for_server, players_launched, watching, all_in_game, server_did_not_start, player_N_failed, player_N_closed, player_N_out_of_memory, watch_ended, stopped).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "mp_test_restart_player".to_string(),
            description: "Starts one game of the running multiplayer test again (player 1 or 2), closed first when it is open, with the same account and the same automation (fill in the account, spawn, character, click to start, admin): the server and the other game keep running. Needs a test started from the app or mp_test_start.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "player": { "type": "integer", "description": "1 or 2" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                },
                "required": ["player"]
            }),
        },
        McpTool {
            name: "mp_test_stop".to_string(),
            description: "Ends the multiplayer test: the games on this PC are closed and the Studio's server quits over RCON (it saves; killed after 90 s if it does not go). Returns the PIDs of the games closed.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
    ]
}

fn mp_prepare(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    let server = args.get("server").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).ok_or("server is required.")?;
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let r = server_manager::mp_test::prepare_second(&catalog, &zomboid_dir(paths), server)?;
    serde_json::to_value(r).map_err(|e| e.to_string())
}

fn mp_start(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    let server = args.get("server").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).ok_or("server is required.")?.to_string();
    let zomboid = zomboid_dir(paths);
    if let Some(pid) = crate::sandbox::find_running_pz_pid() {
        return Err(format!("The game is already running (PID {}): close it first, the test opens its own.", pid));
    }
    if server_manager::get_dedicated_server_status(zomboid.to_string_lossy().to_string()).is_ok_and(|s| s.is_running) {
        return Err("A Studio server is already running: server_stop first.".to_string());
    }
    let (password, _) = admin_password(&server, args.get("admin_password_file").and_then(Value::as_str))?;
    let players = args.get("players").and_then(Value::as_u64).unwrap_or(2).clamp(1, 2) as u32;
    let memory = args.get("memory_gb").and_then(Value::as_u64).unwrap_or(4).clamp(2, 32) as u32;
    crate::claude_agent::keep_std_handles_private();
    let automation = server_manager::mp_test::MpAutomation {
        login: args.get("auto_login").and_then(Value::as_bool).unwrap_or(true),
        start_click: args.get("auto_start_click").and_then(Value::as_bool).unwrap_or(true),
        character: args.get("character").and_then(Value::as_str).unwrap_or("random").to_string(),
        spawn: args.get("auto_spawn").and_then(Value::as_bool).unwrap_or(true),
        admin_all: args.get("all_admin").and_then(Value::as_bool).unwrap_or(true),
    };
    let pid = server_manager::mp_test::start(PathBuf::from(&paths.pz_install_dir), zomboid, server.clone(), memory, players, password, automation)?;
    Ok(json!({ "status": "launched", "server_pid": pid, "server": server, "players": players, "next": "Poll mp_test_status: the games start once the server says SERVER STARTED." }))
}

fn mp_restart(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    let slot = args.get("player").and_then(Value::as_u64).ok_or("player (1 or 2) is required.")? as u32;
    let zomboid = zomboid_dir(paths);
    let server = server_manager::get_dedicated_server_status(zomboid.to_string_lossy().to_string()).ok().and_then(|s| s.server_name).unwrap_or_default();
    let (password, _) = admin_password(&server, None)?;
    crate::claude_agent::keep_std_handles_private();
    let pid = server_manager::mp_test::restart_player(PathBuf::from(&paths.pz_install_dir), zomboid, slot, password)?;
    Ok(json!({ "status": "launched", "player": slot, "pid": pid }))
}

pub fn execute(name: &str, args: &Value, paths: &crate::vfs::StudioPaths) -> Option<ToolCallResult> {
    Some(match name {
        "mp_test_prepare" => result(mp_prepare(args, paths)),
        "mp_test_start" => result(mp_start(args, paths)),
        "mp_test_status" => result(serde_json::to_value(server_manager::mp_test::status(&zomboid_dir(paths), Path::new(&paths.pz_install_dir))).map_err(|e| e.to_string())),
        "mp_test_restart_player" => result(mp_restart(args, paths)),
        "mp_test_stop" => result(server_manager::mp_test::stop(&zomboid_dir(paths)).map(|closed| json!({ "games_closed": closed }))),
        "server_start" => result(start(args, paths)),
        "server_stop" => result(server_manager::stop_dedicated_server(paths.user_zomboid_dir.clone(), None).map(|stopped| json!({ "stopped": stopped }))),
        "server_status" => result(status(args, paths)),
        "server_join" => result(join(args, paths)),
        "server_schedule" => result(schedule(args, paths)),
        "server_restart" => result(restart(args, paths)),
        "dedicated_server" => result(dedicated(args, paths)),
        _ => return None,
    })
}

fn result(r: Result<Value, String>) -> ToolCallResult {
    use super::protocol::ToolContent;
    match r {
        Ok(v) => ToolCallResult {
            content: vec![ToolContent { content_type: "text".to_string(), text: serde_json::to_string_pretty(&v).unwrap_or_default() }],
            is_error: false,
        },
        Err(e) => ToolCallResult { content: vec![ToolContent { content_type: "text".to_string(), text: e }], is_error: true },
    }
}

fn zomboid_dir(paths: &crate::vfs::StudioPaths) -> PathBuf {
    if paths.user_zomboid_dir.is_empty() {
        dirs_next::home_dir().unwrap_or_default().join("Zomboid")
    } else {
        PathBuf::from(&paths.user_zomboid_dir)
    }
}

/// The admin password and where it came from ("file", "stored" or "none").
fn admin_password(server: &str, file: Option<&str>) -> Result<(Option<String>, &'static str), String> {
    if let Some(file) = file.filter(|f| !f.trim().is_empty()) {
        let text = std::fs::read_to_string(file).map_err(|e| format!("Cannot read admin_password_file: {}", e))?;
        let first = text.trim_start_matches('\u{feff}').lines().next().unwrap_or("").trim().to_string();
        if first.is_empty() {
            return Err("admin_password_file is empty.".to_string());
        }
        return Ok((Some(first), "file"));
    }
    match crate::secrets::get(&server_manager::admin_password_secret(server)) {
        Ok(Some(p)) if !p.is_empty() => Ok((Some(p), "stored")),
        _ => Ok((None, "none")),
    }
}

fn start(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    let name = args.get("name").and_then(Value::as_str).map(str::trim).unwrap_or("").to_string();
    if name.is_empty() {
        return Err("name is required (a config in Zomboid/Server/<name>.ini).".to_string());
    }
    let zomboid = zomboid_dir(paths);
    let ini = zomboid.join("Server").join(format!("{}.ini", name));
    let content = std::fs::read_to_string(&ini).map_err(|e| format!("Cannot read {}: {}", ini.display(), e))?;
    let memory = args.get("memory_gb").and_then(Value::as_u64).map(|m| m as u32);
    let nosteam = args.get("nosteam").and_then(Value::as_bool).unwrap_or(false);
    let (password, password_source) = admin_password(&name, args.get("admin_password_file").and_then(Value::as_str))?;

    // What will not load, said before the minutes it takes the server to find out.
    let report = server_manager::check_server_mods(paths.clone(), ini.to_string_lossy().to_string(), Some(nosteam))?;
    let will_not_load: Vec<Value> = report
        .issues
        .iter()
        .filter(|i| i.code.starts_with("server_java_") || i.code.starts_with("server_mod_workshop_") || i.code == "server_mod_copy_stale")
        .map(|i| json!({ "code": i.code, "mod": i.mod_id, "message": i.message }))
        .collect();

    // The folder the server starts from (the game's, or the dedicated server's when chosen): the
    // agent goes on the JVM only when that folder has zbNative.dll.
    let (program_dir, dedicated) = server_manager::server_program_dir();
    let zb_agent = java_mods::agent_arg_for(&content).filter(|_| program_dir.join(java_mods::ZOMBIEBUDDY_FILES[0]).is_file());

    crate::claude_agent::keep_std_handles_private();
    let pid = server_manager::launch_dedicated_server(
        paths.pz_install_dir.clone(),
        zomboid.to_string_lossy().to_string(),
        name.clone(),
        memory,
        Some(nosteam),
        password,
    )?;
    Ok(json!({
        "status": "launched",
        "pid": pid,
        "name": name,
        "nosteam": nosteam,
        "zombiebuddy_agent": zb_agent,
        "program_dir": program_dir.to_string_lossy(),
        "program": if dedicated { "dedicated" } else { "game" },
        "admin_password": password_source,
        "mods": report.mods,
        "will_not_load": will_not_load,
        // Items a server with Steam cannot download (removed, private, hidden): taken out of
        // WorkshopItems= before the start, their mods run from a copy in Zomboid/mods.
        "left_out_of_workshop_items": server_manager::LAST_LEFT_OUT.lock().map(|l| serde_json::to_value(&*l).unwrap_or_default()).unwrap_or_default(),
        "log": zomboid.join("server-console.txt").to_string_lossy(),
        "next": "Poll server_status until started is true, then server_join.",
    }))
}

/// This PC's address on its network: the interface a packet to the internet would leave by
/// (nothing is sent).
pub fn lan_address() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then(|| ip.to_string())
}

fn ini_port(zomboid: &Path, server: &str) -> u16 {
    std::fs::read_to_string(zomboid.join("Server").join(format!("{}.ini", server)))
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.trim().strip_prefix("DefaultPort=").and_then(|v| v.trim().parse::<u16>().ok())))
        .unwrap_or(16261)
}

/// The last `n` lines of the server's console log, and whether it said SERVER STARTED after `since`.
fn console(zomboid: &Path, n: usize, since: Option<u64>) -> (Vec<String>, bool) {
    let path = zomboid.join("server-console.txt");
    let fresh = since.is_none_or(|s| {
        path.metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .is_some_and(|t| t.as_secs() + 5 >= s)
    });
    let Ok(bytes) = std::fs::read(&path) else { return (Vec::new(), false) };
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().collect();
    let started = fresh && lines.iter().any(|l| l.contains("*** SERVER STARTED"));
    let tail = lines[lines.len().saturating_sub(n)..].iter().map(|l| l.to_string()).collect();
    (tail, started)
}

fn status(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    let zomboid = zomboid_dir(paths);
    let st = server_manager::get_dedicated_server_status(zomboid.to_string_lossy().to_string())?;
    let n = args.get("log_lines").and_then(Value::as_u64).unwrap_or(20).min(200) as usize;
    let (tail, started) = if st.is_running { console(&zomboid, n, st.start_timestamp) } else { (console(&zomboid, n, None).0, false) };
    let port = st.server_name.as_deref().map(|s| ini_port(&zomboid, s));
    let join_at = match (st.is_running, port) {
        (true, Some(port)) => Some(if st.nosteam == Some(true) {
            format!("127.0.0.1:{}", port)
        } else {
            format!("{}:{}", lan_address().unwrap_or_else(|| "<this PC's LAN address>".into()), port)
        }),
        _ => None,
    };
    Ok(json!({
        "is_running": st.is_running,
        "pid": st.pid,
        "name": st.server_name,
        "memory_gb": st.memory_gb,
        "start_timestamp": st.start_timestamp,
        "nosteam": st.nosteam,
        "started": started,
        "port": port,
        "join_at": join_at,
        "log_tail": tail,
    }))
}

fn schedule(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    use server_manager::restarts;
    let zomboid = zomboid_dir(paths);
    let running = server_manager::get_dedicated_server_status(zomboid.to_string_lossy().to_string()).ok().filter(|s| s.is_running).and_then(|s| s.server_name);
    let server = args.get("server").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).or(running).ok_or("server is required (no Studio server is running).")?;
    if !zomboid.join("Server").join(format!("{}.ini", server)).is_file() {
        return Err(format!("There is no Zomboid/Server/{}.ini.", server));
    }
    if let Some(set) = args.get("set").filter(|v| v.is_object()) {
        // Only the fields given change: the stored schedule with them laid over it.
        let mut current = serde_json::to_value(restarts::schedule_of(&zomboid, &server)).map_err(|e| e.to_string())?;
        for (k, v) in set.as_object().into_iter().flatten() {
            if current.get(k).is_none() {
                return Err(format!("Unknown schedule field: {}", k));
            }
            current[k] = v.clone();
        }
        let s: restarts::Schedule = serde_json::from_value(current).map_err(|e| format!("Bad schedule: {}", e))?;
        restarts::save_schedule(&zomboid, &server, s)?;
    }
    serde_json::to_value(restarts::view(&zomboid, &server)).map_err(|e| e.to_string())
}

fn restart(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    use server_manager::restarts;
    let zomboid = zomboid_dir(paths);
    let st = server_manager::get_dedicated_server_status(zomboid.to_string_lossy().to_string())?;
    let cancel = args.get("cancel").and_then(Value::as_bool).unwrap_or(false);
    let warn = args.get("warn_minutes").and_then(Value::as_u64).unwrap_or(5).min(120) as u32;
    let watching = restarts::watcher_alive(&restarts::state(&zomboid));
    if !st.is_running && !cancel {
        return Err("No Studio server is running: server_start it.".to_string());
    }
    if watching {
        let kind = if cancel { "cancel" } else { "restart" };
        restarts::request(&zomboid, &restarts::Request { kind: kind.into(), warn_minutes: warn, by: "assistant".into(), at: now() })?;
        return Ok(json!({
            "status": if cancel { "cancel_asked" } else { "restart_asked" },
            "warn_minutes": warn,
            "restart_at": (!cancel).then(|| now() + warn as u64 * 60),
            "next": "The app counts down and restarts: follow it with server_schedule (pending, history).",
        }));
    }
    if cancel {
        return Err("PZ Mod Studio is not open: no restart is counting down.".to_string());
    }
    if warn > 0 {
        return Err("PZ Mod Studio is not open: warned restarts are counted down by the app. Open it, or pass warn_minutes 0 to restart now.".to_string());
    }
    let server = st.server_name.clone().unwrap_or_default();
    let w = restarts::Watched { server: server.clone(), pid: st.pid, memory_gb: st.memory_gb, nosteam: st.nosteam, started_at: st.start_timestamp.unwrap_or(0), started_ok: true };
    let lang = restarts::schedule_of(&zomboid, &server).language;
    crate::claude_agent::keep_std_handles_private();
    let done = restarts::restart_now(&zomboid, &w, "manual", &lang)?;
    Ok(json!({ "status": "restarted", "server": server, "detail": done, "next": "Poll server_status until started is true." }))
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn dedicated(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    use server_manager::dedicated;
    let text = |k: &str| args.get(k).and_then(Value::as_str).map(str::to_string);
    match args.get("action").and_then(Value::as_str).unwrap_or("info") {
        "info" => serde_json::to_value(dedicated::view()).map_err(|e| e.to_string()),
        "set" => {
            dedicated::set(text("source"), text("dir"), text("branch"), text("steamcmd"))?;
            serde_json::to_value(dedicated::view()).map_err(|e| e.to_string())
        }
        "install" => {
            let cfg = dedicated::load_config();
            let dir = text("dir").filter(|d| !d.trim().is_empty()).unwrap_or(cfg.dir.clone());
            if dir.trim().is_empty() {
                return Err("dir is required (the folder for the dedicated server).".to_string());
            }
            let req = dedicated::InstallRequest {
                dir: dir.clone(),
                branch: text("branch").unwrap_or(cfg.branch),
                validate: args.get("validate").and_then(Value::as_bool).unwrap_or_else(|| dedicated::validate_by_default(Path::new(dir.trim()))),
                allow_download: args.get("allow_download").and_then(Value::as_bool).unwrap_or(false),
            };
            let zomboid = zomboid_dir(paths);
            let st = server_manager::get_dedicated_server_status(zomboid.to_string_lossy().to_string()).ok().filter(|s| s.is_running);
            let from = st.and_then(|s| s.program_dir).map(PathBuf::from);
            crate::claude_agent::keep_std_handles_private();
            let p = dedicated::start_install(req, Path::new(&paths.pz_install_dir), &zomboid, from.as_deref())?;
            Ok(json!({ "status": "started", "progress": p, "next": "Poll dedicated_server (action info) for progress; it takes minutes (several GB)." }))
        }
        "cancel" => dedicated::cancel().map(|c| json!({ "cancelled": c })),
        other => Err(format!("Unknown action {}: info, set, install or cancel.", other)),
    }
}

/// `ip:port` from what the caller wrote (`ip`, `ip:port`), or None when it is not an address.
pub fn parse_address(text: &str) -> Option<(String, u16)> {
    let text = text.trim();
    let (host, port) = match text.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().ok()?),
        None => (text, 16261),
    };
    let ok = !host.is_empty() && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    ok.then(|| (host.to_string(), port))
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn join(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    if let Some(pid) = crate::sandbox::find_running_pz_pid() {
        return Err(format!("The game is already running (PID {}): close it first (terminate_game), then join.", pid));
    }
    let zomboid = zomboid_dir(paths);
    let st = server_manager::get_dedicated_server_status(zomboid.to_string_lossy().to_string()).ok();
    let running = st.as_ref().filter(|s| s.is_running);
    let nosteam = args
        .get("nosteam")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| running.and_then(|s| s.nosteam).unwrap_or(false));
    let debug = args.get("debug").and_then(Value::as_bool).unwrap_or(false);

    let (host, port) = match args.get("address").and_then(Value::as_str).filter(|a| !a.trim().is_empty()) {
        Some(a) => parse_address(a).ok_or_else(|| format!("\"{}\" is not an address: give ip:port or ip.", a))?,
        None => {
            let Some(server) = running.and_then(|s| s.server_name.clone()) else {
                return Err("No Studio server runs on this PC: give an address (ip:port).".to_string());
            };
            let port = ini_port(&zomboid, &server);
            let host = if nosteam {
                "127.0.0.1".to_string()
            } else {
                lan_address().ok_or("Could not tell this PC's LAN address: give it as address (ip:port).")?
            };
            (host, port)
        }
    };
    if !nosteam && is_loopback(&host) {
        return Err(format!(
            "A Steam client cannot join {} (Steam refuses this PC's own loopback and public addresses): use the LAN address{}.",
            host,
            lan_address().map(|ip| format!(", {}:{}", ip, port)).unwrap_or_default()
        ));
    }

    let mut extra: Vec<String> = vec!["+connect".to_string(), format!("{}:{}", host, port)];
    if let Some(more) = args.get("extra_args").and_then(Value::as_array) {
        extra.extend(more.iter().filter_map(|v| v.as_str().map(str::to_string)));
    }
    let prev_foreground = crate::claude_agent::current_foreground_window();
    crate::claude_agent::keep_std_handles_private();
    let pid = crate::sandbox::launch_pz_direct(&paths.pz_install_dir, &paths.user_zomboid_dir, debug, true, nosteam, &extra)?;
    crate::claude_agent::start_launch_window_guard(prev_foreground, false, false, true);
    let mut note = Vec::new();
    if debug {
        note.push("A -debug client may only join as an admin.");
    }
    if running.is_some_and(|s| s.nosteam != Some(nosteam)) && running.and_then(|s| s.nosteam).is_some() {
        note.push("This client's Steam mode differs from how the running server was started: it will not get in.");
    }
    Ok(json!({
        "status": "launched",
        "pid": pid,
        "address": format!("{}:{}", host, port),
        "nosteam": nosteam,
        "debug": debug,
        "args": extra,
        "notes": note,
        "mods_line": running.and_then(|s| s.server_name.clone()).map(|s| {
            std::fs::read_to_string(zomboid.join("Server").join(format!("{}.ini", s))).map(|t| mod_sync::ini_list(&t, "Mods")).unwrap_or_default()
        }),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_read_with_the_default_port() {
        assert_eq!(parse_address("192.168.0.192:16261"), Some(("192.168.0.192".into(), 16261)));
        assert_eq!(parse_address(" 10.0.0.5 "), Some(("10.0.0.5".into(), 16261)));
        assert_eq!(parse_address("my-host.lan:16300"), Some(("my-host.lan".into(), 16300)));
        assert_eq!(parse_address("1.2.3.4:99999"), None);
        assert_eq!(parse_address("a b:1"), None);
        assert_eq!(parse_address("\"x\" -debug:1"), None, "nothing but an address reaches the command line");
        assert!(is_loopback("127.0.0.1") && is_loopback("localhost") && !is_loopback("192.168.0.192"));
    }

    #[test]
    fn the_admin_password_comes_from_the_file_first_line_and_is_never_empty() {
        let dir = std::env::temp_dir().join(format!("pzms-mcp-server-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("pw.txt");
        std::fs::write(&file, "\u{feff}secret one \r\nsecond line\r\n").unwrap();
        let (pw, from) = admin_password("x", Some(&file.to_string_lossy())).unwrap();
        assert_eq!((pw.as_deref(), from), (Some("secret one"), "file"));
        std::fs::write(&file, "\n").unwrap();
        assert!(admin_password("x", Some(&file.to_string_lossy())).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_tools_are_listed_and_dispatched() {
        let paths = crate::vfs::studio_paths();
        for t in tools() {
            assert!(t.description.len() > 40, "{} explains itself", t.name);
            if t.name != "server_start" {
                continue;
            }
            // Without a name it refuses before touching anything.
            let r = execute(&t.name, &json!({}), &paths).unwrap();
            assert!(r.is_error);
        }
        assert!(execute("not_a_server_tool", &json!({}), &paths).is_none());
    }
}
