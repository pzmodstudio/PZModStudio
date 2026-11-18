# Project Zomboid Mod Studio — AI Agent & MCP Integration Guide

This file serves as the **master technical reference guide for AI Agents** (Antigravity, Claude, Cursor, Cline, Windsurf, Roo Code, etc.) working on this repository or connecting to **PZ Mod Studio** via the **Model Context Protocol (MCP)**.

---

## 1. 🏗️ Overall Project Architecture

- **Frontend:** React 19, TypeScript, Vite, Tailwind CSS, Monaco Editor, Lucide Icons.
- **Backend:** Tauri 2.0 (Rust) with specialized crates:
  - `full_moon`: AST parsing and static analysis for Lua code.
  - `similar`: Line-by-line and chunk-by-chunk diffing algorithms.
  - `dirs-next` / `walkdir`: Filesystem resolution and recursive scanning.
- **MCP Server:** Pure Rust implementation (`src-tauri/src/mcp/`) adhering to the **MCP 2024-11-05 (JSON-RPC 2.0 over stdio)** standard.

---

## 2. 🔌 Connecting an Agent to the MCP Server

The MCP server can be launched as a dedicated console binary or via the `--mcp` CLI flag on the main executable.

### Executable Paths
- **Portable console binary (Recommended):**
  `C:\Path\To\Project-Zomboid-Mod-Studio\pz-mcp-server.exe`
- **Development build:**
  `C:\Path\To\src-tauri\target\release\pz-mcp-server.exe`
- **Main application CLI flag:**
  `C:\Path\To\Project-Zomboid-Mod-Studio\Project-Zomboid-Mod-Studio.exe --mcp`

### JSON Configuration Snippets

#### For Antigravity / Gemini CLI (`.gemini/settings.json`):
```json
{
  "mcpServers": {
    "pz-mod-studio": {
      "command": "C:\\Path\\To\\Project-Zomboid-Mod-Studio\\pz-mcp-server.exe",
      "args": [],
      "env": {}
    }
  }
}
```

#### For Claude Desktop (`claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "pz-mod-studio": {
      "command": "C:\\Path\\To\\Project-Zomboid-Mod-Studio\\pz-mcp-server.exe",
      "args": []
    }
  }
}
```

#### For Cursor / Windsurf (`mcp.json`):
```json
{
  "mcpServers": {
    "pz-mod-studio": {
      "command": "C:\\Path\\To\\Project-Zomboid-Mod-Studio\\pz-mcp-server.exe",
      "args": []
    }
  }
}
```

#### For VS Code (Roo Code / Cline / Offline Ollama & LM Studio — `cline_mcp_settings.json`):
```json
{
  "mcpServers": {
    "pz-mod-studio": {
      "command": "C:\\Path\\To\\Project-Zomboid-Mod-Studio\\pz-mcp-server.exe",
      "args": [],
      "disabled": false,
      "autoApprove": []
    }
  }
}
```

#### For OpenAI / Codex / ChatGPT MCP Bridges:
```json
{
  "mcpServers": {
    "pz-mod-studio": {
      "command": "C:\\Path\\To\\Project-Zomboid-Mod-Studio\\pz-mcp-server.exe",
      "args": [],
      "description": "Project Zomboid Mod Studio MCP Server (Build 41/42 AST & Live Bridge)"
    }
  }
}
```

---

## 3. 🛠️ MCP Tool Catalog (*Tools*)

Connected MCP clients can invoke the following tools via `tools/call`:

### Diagnostics, Monitor Center & Process Control
| Tool | Parameters | Description |
| :--- | :--- | :--- |
| `get_game_status` | `{}` | Checks whether `ProjectZomboid64.exe` is running and returns its Process ID (PID). |
| `launch_game` | `debug_mode` (bool), `windowed` (bool), `nosteam` (bool), `extra_args` (array)<br>`agent_mode`, `park`, `no_focus_steal` (bool), `park_width`, `park_height` (int) | Launches `ProjectZomboid64.exe` with configurable debug flags and windowed mode. `agent_mode: true` = no `-debug`, `-windowed`. A windowed game is always maximized while it boots (a small helper process, `pz-mcp-server.exe --window-guard`, watches it for two minutes); `park`/`no_focus_steal` are no longer acted on. |
| `terminate_game` | `pid` (int optional), `force` (bool default true) | Closes or forcefully terminates the Project Zomboid process. |
| `send_game_ipc_command` | `command` (object: `give_item`, `eval_lua`, `set_godmode`, etc.) | Sends a command to the active game session through the companion mod `Z_PZModStudio_Bridge`. |
| `get_game_ipc_response` | `{}` | Reads the latest execution response returned by the companion mod. |
| `install_bridge_companion_mod` | `user_zomboid_dir` (optional)<br>`include_claude_agent` (bool, default false) | Installs the companion mod `Z_PZModStudio_Bridge` into `Zomboid/mods` to enable the IPC bridge; optionally also `Z_ClaudeAgent`. |
| `get_monitor_logs` | `max_lines` (int, default 100)<br>`errors_only` (bool)<br>`user_zomboid_dir` (string optional) | Reads and filters recent lines from the active log file `console.txt`. |
| `list_available_logs` | `user_zomboid_dir` (optional) | Lists all session log files on disk (`console.txt` and `Zomboid/Logs/`). |
| `read_log_file` | `file_path` (string required)<br>`max_lines` (int)<br>`errors_only` (bool) | Reads a specific log file with exception filtering. |
| `get_crash_diagnostics` | `user_zomboid_dir` (string optional) | The Monitor's cards for `console.txt` (the same card maker, so both say the same; saved logs with a time in front of each line read too). Each card carries `severity`: `EXCEPTION` (the game wrote the line at `ERROR` level or as part of a stack trace, or: a mod's Java patch ZombieBuddy could not apply, `java_patch_failed`, named after the mod whose jar was loading the first time; a Java mod saying it switched itself off, `java_mod_turned_off`) or `WARNING` (a `WARN`/`LOG` line: the game noticed and carried on; a Java mod's shader that did not compile, `java_shader_failed`, with the undefined name). A Java mod's log tag is said as the name in the mod list when a jar of that name was loaded (`[ViewpointHeadlightPlacement]` -> "Viewpoint - Light Fix"). An error in the game's own Lua gets its card once. A card says only what its line shows; a mod is named when the line names it, or, said as such, when it is in the stack frame printed just above. |
| `get_error_report` | `index` (int, default 0)<br>`raw_error` (string optional)<br>path parameters | Everything needed to fix **one** error from the player's last session, as Markdown: the log line, the lines around it, the installed build, the mod the game blamed (id, version, Workshop link, folder, requirements) and the source of the Lua file that failed, with a note when another active mod ships the same path. Read-only. With no arguments it reports the most recent error the game attributed to a mod and lists the others with their indices; `index` walks back through them, `raw_error` picks one by its log line. |

| `zb_exclusions` | `log` (`game` default: console.txt / `server`: server-console.txt), `user_zomboid_dir` | Java mods ZombieBuddy left out of the last session, read from its log: `mod_id`, `kind` (`signature`: its `.zbs` did not verify against the key its author shows on Steam; `denied`: `blocked by policy=prompt`, a "don't allow" saved for that jar in `~/.zombie_buddy/mod_approvals.json` or given in its window, asked again by holding Shift while the game loads (the Studio never edits that file); `unapproved`: another policy, e.g. a server's `deny-new`; `other`), the reason, the `addons` that failed for want of its classes (`NoClassDefFoundError` of its package) and `fatal` (one of them stopped the game or server at boot). Read-only; Diagnostics reports it as `zb_excluded` for mods on the list (the newest of the two logs first: a mod a later session put on the classpath drops off), with one button to switch the mod off with the mods that require it. |

### Dedicated server (host and join)
The Servers screen's launcher, for agents. One Studio server at a time; its log is `Zomboid/server-console.txt`.

| Tool | Parameters | Description |
| :--- | :--- | :--- |
| `server_start` | `name` (required: `Zomboid/Server/<name>.ini`), `memory_gb` (default 4), `nosteam` (bool), `admin_password_file` (optional), `user_zomboid_dir` | Starts the server headless with the game's `java.exe`, or the dedicated server program's when `dedicated_server` chose it (`program`, `program_dir`). When `Mods=` has ZombieBuddy (`\ZombieBuddy` in B42) the JVM gets `-agentlib:zbNative=policy=deny-new`: only jars already approved in `~/.zombie_buddy/mod_approvals.json` load, and ZombieBuddy's approval window (which would hang a server) never shows. Without `zbNative.dll` in that folder the server starts without it and `will_not_load` says what to do (`server_java_no_agent`, `_dedicated`: copy `zbNative.dll` and `ZombieBuddy.jar` from the game's folder). Admin password: the file's first line, else the value the Studio keeps in Windows Credential Manager from the last start with one; never returned. With Steam, items of `WorkshopItems=` the server cannot download (Steam has no public item: removed, private, hidden; or the last server log says it was refused), which stop it at boot, are taken out first (backup kept), their mods copied into `Zomboid/mods` when not there: `left_out_of_workshop_items`. Returns the PID, `zombiebuddy_agent` and `will_not_load` (unapproved jars; Workshop mods linked into `Zomboid/mods` (a NoSteam server crashes at world creation with some links) or copied there and outdated since; with `nosteam`, mods only in the Workshop folder, which a NoSteam server reports "not found" along with the mods requiring them; the Servers screen copies them). |
| `server_status` | `log_lines` (default 20), `user_zomboid_dir` | Running, PID, name, memory, `nosteam`, `started` (the log said `SERVER STARTED` since launch), port, `join_at` (LAN address with Steam, 127.0.0.1 without) and the log tail. |
| `server_stop` | `user_zomboid_dir` | `quit` over RCON (saves), killed after 90 s if still up; only the server the Studio started. |
| `server_join` | `address` (`ip[:port]`, default: the running Studio server), `nosteam` (default: how that server was started), `debug` (default false), `extra_args` | Launches the game with `+connect ip:port` (and `-nosteam`). A Steam client cannot join 127.0.0.1 or this PC's own public IP (use the LAN address; refused with loopback); a `-debug` client may only join as admin. Refused while the game runs. |
| `server_schedule` | `server` (default: the running Studio server), `set` (object: `enabled`, `times` ["HH:MM"], `every_hours`, `warn_minutes` (default 30/15/5/1), `on_crash` (default true), `crash_limit`/`crash_window_minutes` (default 3 in 10), `on_mod_update`, `mod_check_minutes`, `language` en/es), `user_zomboid_dir` | A server's restarts (`Zomboid/PZModStudio_ServerSchedule.json`; `set` changes only the fields given): scheduled at times of day and/or every N hours from its start, warned in the game over RCON (`servermsg`), then `save` + `quit` and started again the same way; started again after a crash once it had come up, up to the limit, then left off (`gave_up_at`); a warned restart when a `WorkshopItems=` item's `time_updated` (Steam Web API) is after the server's start. Done by the app's watcher **only while PZ Mod Studio is open** (`app_watching`); the server keeps running when it closes. Returns `next_restart`, `pending` (when, reason, warnings given) and `history` (reason `schedule`/`mod_update`/`manual`/`crash`/`gave_up`/`crash_no_restart`/`stopped_while_closed`, ok, detail). A stop asked anywhere (Stop, `server_stop`, Discord) or an admin's in-game `quit` is never taken for a crash. |
| `server_restart` | `warn_minutes` (0-120, default 5), `cancel` (bool), `user_zomboid_dir` | A warned restart of the running Studio server: with the app open, the app counts down (the schedule's warnings that fit) and restarts; follow it with `server_schedule`. With the app closed only `warn_minutes: 0` (restarted now from the MCP process). `cancel` calls off a countdown (players told). |
| `dedicated_server` | `action` (`info` default / `set` / `install` / `cancel` / `copy_zombiebuddy`), `source` (`game`/`dedicated`), `dir`, `branch` (default: the installed game's branch, `game_branch` in `info`; `public`, `unstable`), `steamcmd`, `validate` (default: true for a folder without a complete install, else false: an update fetches only what changed, a check of every file takes many minutes), `allow_download` (default false) | The official dedicated server (Steam app 380870) instead of the game's folder: `info` = source, folder, branch, steamcmd.exe found, installs found (build id/branch/size from `appmanifest_380870.acf`, `version`: the game version its jar starts as, beside `game_version`, the installed game's; a client joins only the same), install progress (phase, percent, bytes, folder size, last SteamCMD line, `up_to_date` when nothing was downloaded). `install` runs `steamcmd +force_install_dir <dir> +login anonymous +app_update 380870 -beta <branch> [validate] +quit` in the background (several GB from Steam); SteamCMD itself is downloaded from `https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip` (~3 MB, Valve signature checked) only with `allow_download: true` (ask the user). `copy_zombiebuddy` copies `zbNative.dll` and `ZombieBuddy.jar` from the game's folder into the dedicated folder (refused while the Studio's server runs from it). `set source: dedicated` makes `server_start` run java.exe from that folder (class path from its `StartServer64.bat`), same Zomboid folder. Config: `%APPDATA%/PZModStudio/dedicated_server.json`. |
| `mp_test_prepare` | `server` (required), `user_zomboid_dir` | Multiplayer test on one PC: gets player 2's own Zomboid folder (`<home>/Zomboid2`) ready — options.ini the first time, and every mod of the server's `Mods=` copied into `Zomboid2/mods` with the Studio's marker (again when the source is newer), or linked when it is a mod in development. A folder there without the Studio's marker (put by hand or by a script) is kept while it still holds what the server runs, and copied again once it differs (other files, or the mod got a compatible copy). Returns copied / linked / kept / missing. |
| `mp_test_start` | `server` (required), `players` (1-2, default 2), `memory_gb` (default 4), `admin_password_file`, `auto_login` (default true), `auto_start_click` (default true), `character` (`random`/`first`/`manual`, default random) | Starts the server, then, once it says SERVER STARTED, the games: 1 player with Steam (Workshop items the server cannot download left out first; the game joins at this PC's LAN address, a Steam client refuses 127.0.0.1), 2 players without Steam, joined at 127.0.0.1 (`-nosteam +connect`; player 2 with `-cachedir=Zomboid2`). The Studio Link fills in each account (player 1 admin, player 2 jugador2) and connects, makes a new character (random / first shown / manual; the Link is added to the server's `Mods=` with a backup when missing), and the app clicks each game's "click to start" once its world loaded; a watcher follows both games every 2 s (their log and the Link's `Lua/PZModStudio_MpState.txt`). In MP each game loads the server's list, not the profile. Refused while the game or a Studio server runs. |
| `mp_test_status` | `user_zomboid_dir` | Every server and game process on this PC: role, PID, name, Zomboid folder and log, RAM used and heap limit, CPU %, state (server starting/started; game menu/connecting/in_game, from its log), ERROR lines, the address joined; the PC's total/free memory; player 2's folder; the test's phase (`waiting_for_server`, `watching`, `all_in_game`, `server_did_not_start`, `player_N_failed`, `player_N_closed`, `player_N_out_of_memory`: that game ended before it got in, `watch_ended`, `stopped`). |
| `mp_test_restart_player` | `player` (1 or 2, required), `user_zomboid_dir` | Starts one game of the running test again (closed first when open) with the same account and automation; the server and the other game keep running. |
| `mp_test_stop` | `user_zomboid_dir` | Closes the games and stops the Studio's server over RCON (it saves). |

### Claude Agent (menus + in-game control, window, screenshots)
Uses the `Z_ClaudeAgent` companion mod (source: `mods/Z_ClaudeAgent/`). Unlike the bridge it runs in the main menu, new-game / sandbox / character / spawn screens and while paused, and returns values. Protocol and helpers: [docs/09 §5](docs/09-game-control-and-ipc-bridge.md#5-claude-agent-protocol-z_claudeagent). The `agent_*` tools accept `timeout_ms` (default 15000) and `user_zomboid_dir` (player 2 of a multiplayer test: `<home>/Zomboid2`; a folder that does not exist is refused); `status` `err`/`timeout` sets `isError`.

| Tool | Parameters | Description |
| :--- | :--- | :--- |
| `install_claude_agent_mod` | `user_zomboid_dir` (optional) | Installs `Z_ClaudeAgent` into `Zomboid/mods` (then enable it in the Mods list). |
| `agent_eval_lua` | `code` (string required), `wait_idle_ms` (int, default 0) | Runs Lua in the game and waits for the answer; `return` values come back as JSON when possible. `wait_idle_ms` first waits for `CA.busy().actions == 0` (timed-action queue empty). |
| `agent_ui_list` | `filter` (string optional) | `return CA.ui(filter)`: visible buttons, ticks, combos, entries and labels with position/size. |
| `agent_ui_click` | `text` (string required), `type_name` (optional), `retry_ms` (default 20000) | `return CA.click(text)`: clicks by visible label (exact, then substring); retries about once a second while the answer is "not found" (menus build over several frames). |
| `agent_enter_game` | `timeout_ms` (default 300000) | After Continue/Play: polls `CA.state()`; once the mod has been silent >20 s (black "click to start" screen, where Lua does not run) it posts a centre click every ~3 s until `ingame` is true. Returns clicks sent and the final state. |
| `agent_dialog_answer` | `answer` (`YES`/`NO`/`OK`, default `YES`) | `return CA.answer(answer)` on the front-most modal dialog. |
| `agent_game_state` | — | `return CA.state()`: in game, menu visible, paused, speed, time, player position. |
| `capture_game_screenshot` | `path` (optional), `max_width` (int, default 1280) | PrintWindow capture of the game window to PNG (works while covered). Returns the path and `looks_blank`. |
| `game_window` | `mode` (`park`/`minimize`/`show`/`info`), `width`, `height` | Park = 0,0 at width x height, bottom of the z-order, not activated; show = restore + foreground, maximized when it does not fit the screen. |
| `game_click_window` | `x_frac`, `y_frac` (0..1, default 0.5) | Posts WM_MOUSEMOVE + WM_LBUTTONDOWN/UP at that point of the client area without focusing the window. |
| `game_send_key` | `key` (e.g. `O`, `SHIFT+O`, `F9`, `ESC`), `hold_ms` (default 80) | Posts WM_KEYDOWN/WM_KEYUP (scan codes in lParam) to the game window without focusing it; modifiers pressed first, released last. |

> **Agent play notes (verified live, B42):**
> - Launch without `-debug` (`launch_game` with `agent_mode: true`) unless you need cheats: under `-debug` the Lua debugger stops the game on any error, even inside `pcall`.
> - Cheats (`setGhostMode`, `setGodMod`, `setInvisible`) only take effect when the game was launched with `-debug`. In that case `Z_ClaudeAgent` turns break-on-error off with `UIManager.setShowLuaDebuggerOnError(false)`.
> - `IsoMovingObject:DistTo` is not euclidean (about 1.41x on diagonals). Use `getDistanceSq` (compare with radius²) for ranges.
> - After Continue/Play the game waits on a black "click to start" screen (GameLoadingState) that suppresses Lua callbacks, so the mod cannot answer; use `agent_enter_game` (or `game_click_window`) to post the click.
> - Game speed: `CA.speed(n)` presses the speed-widget buttons ("Play", "Fast Forward x 1" = x5, "Fast Forward x 2" = x20, "Wait" = x40), because `SetCurrentGameSpeed` alone does not change the clock. The game refuses fast-forward when zombies are near.
> - `IsoStove` objects are replaced when toggled or burnt: look them up again instead of keeping references.
> - Hot-reloading the mod (sending `ClaudeAgent.lua` as a command) keeps running `CA.tasks`.
> - Teleport in B42 with `player:teleportTo(x, y, z)`; `setX/setY` alone snap back and `setLx/setLy/setLz` no longer exist. The bridge `teleport` action uses `teleportTo` when available.

### Paths, Mods, Profiles & Load Order
| Tool | Parameters | Description |
| :--- | :--- | :--- |
| `get_studio_paths` | `{}` | Auto-detects and returns installation paths for the game, Steam Workshop, and user Zomboid folder. |
| `list_installed_mods` | `pz_install_dir`, `user_zomboid_dir`, `workshop_dir` (optional) | Scans and lists all installed Steam Workshop and local mods along with metadata and IDs. |
| `get_mod_health` | `active_mod_ids` (array, optional; default: `default.txt`)<br>`include_catalog` (bool)<br>path parameters | Read-only check of the active list using the game's own rules: not installed, not loadable on the installed build, invalid `mod.info`, missing dependencies (one that is off is loaded by the game anyway: a tidy-up), `loadModAfter`/`loadModBefore` order (checked on the order the game really loads, each mod after what it requires; a requirement later in the list is no finding), incompatible pairs, duplicates; a `require=` with a space after a comma (`require_spaced`: the game looks the id up with the space, leaves the mod out and every mod requiring it, while its Mods screen still shows it on); and the active package's copies of mod files the mod has changed since (`package_file_stale`, with `remove_from_package`) or that no mod in the list ships (`package_file_orphan`); a package built for another list (`package_built_for_other_list`, fix `update_package`: rescan with the list now and publish); a mod whose author renamed its id (`missing_mod` variant `renamed`/`renamed_many`: the one to go on with first, a name saying experimental/beta/test after), a copy in `Zomboid/mods` from before such a rename (`workshop_copy_renamed`, `compatible_copy_renamed`); a Java mod ZombieBuddy left out of the last game or server session (`zb_excluded`, with the addons that failed without it); a game function one mod cuts off for another (`function_overridden`: a to-do only when moving it before fixes it, else something to know). Issues carry severity, code, state and suggested fixes. |
| `sort_mod_load_order` | `order` (array, optional; default: `default.txt`)<br>path parameters | Game-exact sort: each mod after what it requires and its `loadModAfter` mods, before its `loadModBefore` mods, everything else kept in place, Studio packages last. Read-only; returns `{order, moved, cycle, reasons}`: `reasons` is each rule the old order broke, now kept (`mod_id` after `other`, kind `require` / `load_after` / `load_before` / `package`; the app also says `rule` for the player's own and `index` for the Studio's index); the other moves only made room. |
| `scan_mod_conflicts` | Same path parameters | Scans the Virtual File System (VFS) to detect collisions between mods and the vanilla base game. |
| `list_mod_profiles` | `user_zomboid_dir` (optional) | Lists all saved mod profiles, active mod IDs, and custom load orders. |
| `create_mod_profile` | `name` (string required)<br>`description` (string)<br>`active_mod_ids` (array)<br>`load_order` (array) | Creates a new independent mod combination profile. |
| `activate_mod_profile` | `profile_id` (string required) | Activates a mod profile and writes it directly to `default.txt` / `ModListData.ini`. |

### Merge Engine & Patch Generator
| Tool | Parameters | Description |
| :--- | :--- | :--- |
| `validate_lua_syntax` | `code` (string required) | Validates Lua syntax using the AST parser (`full_moon`), providing exact line and column numbers on error. |
| `merge_lua_scripts` | `base`, `target_a`, `target_b` (strings required) | Performs a 3-way AST merge between a base script and two conflicting mod variants. |
| `list_merged_packages` | `user_zomboid_dir`, `mod_list_ini_path` (optional) | Lists all patch and merged packages (`Z_PZModStudio_*`) registered in the system. |
| `get_master_patch_status` | `user_zomboid_dir`, `package_folder_name` (optional) | Queries the status of the `Z_PZModStudio_MasterPatch` package and saved resolutions. |
| `save_draft_resolution` | `relative_path`, `resolved_content` (required)<br>`package_folder_name`, `status` (optional) | Saves a resolved code draft directly into the master patch package. |
| `publish_file_into_package` | `relative_path`, `content` (required)<br>`package_folder_name` (default `Z_PZModStudio_MergedPatch`) | Puts one corrected file into a package and publishes it **in place**: everything the package holds is published again beside it, a packaged package stays packaged, the game reads it on its next launch. The way to patch a mod's own file without touching the mod (the package loads last). The file stays in every later publish, the Merger's included, until `remove_file_from_package` takes it out. |
| `remove_file_from_package` | `file` (required; a path in the package or just the file name)<br>`package_folder_name` | Takes a file out of a package and publishes it again without it; the game reads the mod's own copy again. |
| `keep_mod_version` | `mod_id` (required), path parameters | Keeps the mod as installed today in the version store (`Zomboid/PZModStudio_Versions` or the folder chosen with `mod_versions_settings`: each file once by content, a version is a list of path -> hash), for when a Workshop update breaks a save. Kept by hand: the store's cap never drops it. |
| `list_mod_versions` | `mod_id` (optional), `keep_now` (bool), path parameters | The earlier versions of mods kept on this PC. The app keeps the installed version of every Workshop mod a profile uses by itself (a round ~90 s after start and whenever `appworkshop_108600.acf`, a profile, the game's list or the settings change; never while the game runs; Workshop files only read), so the one before a Steam update stays. With `mod_id`: its versions, newest first (`kept_at`, `version`, `published`, `kind` auto/kept/copy, `current`, `in_use`, `bytes`, `path`) and the store's `folder`. Without: the store's `folder` (`default_folder`, `folder_chosen`), settings (`enabled`, `cap_bytes`, default 10 GB; past it, older earlier versions go first, then copies of mods outside the active profile, biggest first), `guard` (its drive: `free_bytes`, `total_bytes`, `reserve_bytes` = 15% of the drive, always left free whatever the cap, `cap_effective`, `limited`) and the last round (items used/kept, `no_room`, bytes, `cap_effective`, `limited_by_disk`). `keep_now` runs a round first. |
| `mod_versions_settings` | `store_dir`, `cap_gb` (number), `enabled` (bool), path parameters | The version store's settings, as in Settings > App. `store_dir` moves the store there (an empty or new folder as it is; one with other files, or a drive's root, gets `PZModStudio_Versions` inside): every file copied and read back against its hash, then the old folder removed (renamed on one drive), local copies in use repointed; the choice is kept in `Zomboid/PZModStudio_VersionsFolder.json`. Refused while the game or a round runs, and inside Steam's Workshop folder, the game's folder, `Zomboid/mods` or `Zomboid/Saves`. `cap_gb` sets the cap (the player's from then on); `enabled` the background keeping. Returns `{ moved, summary }`. |
| `use_mod_version` | `path` (required, from `list_mod_versions`), `user_zomboid_dir` | Puts a kept version back in `Zomboid/mods` as a local copy that wins over the Workshop one (the Studio launches with `-modfolders mods,workshop,steam`). Refused while the game runs or when a folder of that name there is not the Studio's. Diagnostics then shows `kept_version_in_use`. |
| `back_to_workshop_version` | `mod_id` (required), path parameters | Undoes `use_mod_version`: the copy goes to `Zomboid/PZModStudio_Backups/mods`, the game reads the Workshop one again; the kept version stays. Refused while the game runs or a server's `Mods=` lists the mod. |
| `make_compatible_copy` | `mod_id` (required), `replace_existing` (bool, default false), path parameters | The Diagnostics fix of that name: copies a Workshop mod the installed build will not load, or loads with errors, into `Zomboid/mods` and adapts the copy (version limits, `require` paths Build 42 moved, item scripts, JSON translations). The Workshop original is never touched; the copy wins over it. `replace_existing` replaces a plain copy of the same mod already there. Returns the folder, `changes` and `warnings`. Refused while the game runs. |
| `remove_compatible_copy` | `folder` (required), `user_zomboid_dir` | Back to the original: the copy goes to `Zomboid/PZModStudio_Backups/mods`. Refused while the game runs or while a server's `Mods=` lists the mod. |
| `compat_author_suggestion` | `mod_id` (required), path parameters | For a mod the Studio keeps a compatible copy of: a note in English for its author, to paste in the Workshop comments: what was in the way on the installed build and, file and line, each line as published beside the line that works here (a suggestion with the working example). Read-only; nothing is sent. |
| `remove_local_mod` | `mod_id` (required), `user_zomboid_dir` | For a mod that is only a folder in `Zomboid/mods` (put by hand; nothing to unsubscribe from): the folder goes to `Zomboid/PZModStudio_Backups/mods`. Refused while the game runs, for a mod that is on, that a server's `Mods=` lists, that is a link to a mod in development, or the Studio's own. |
| `game_memory` | `mb` (int, optional), path parameters | The memory the game's Java may use (`-Xmx` in `ProjectZomboid64.json`): now, the PC's, the most offered (half the PC's), the suggestion for the active list, the active profile and whether it has its own value, recent out-of-memory sessions, and `windows_low_memory`: the times Windows itself ran out of memory with the game open in the last 14 days (Resource-Exhaustion-Detector event 2004: commit limit and charge, the game's share, the other processes holding memory, whether the game was closed for it). With `mb` it sets it from the next start (backup kept), as the active profile's value and the last chosen; refused while the game runs. Switching a profile on gives the game its value. |
| `function_overrides` | `active_mod_ids` (array, optional), `only_cut` (bool) | Lua functions two or more active mods redefine, in run order: the game's, and a mod's that another mod redefines without either requiring the other (`owner`). Each redefinition with mod, file, line, `keeps_original`, `deferred` and the `event` whose handler makes it (handlers run after every file, in the order the game fires the events); `reset` marks a mod making the whole table new (`Table = {}`). Per function the `winner` that replaces last, the mods it `cut` off (their change never runs), `fixed_by_moving_before` when the order alone fixes it, and `checked`: what the running game had (file, line, mod, `agrees`), read by Studio Link a few seconds after the main menu and after a save loads (`Lua/PZModStudio_LinkProbe*.txt`). Diagnostics carries the cuts as `function_overridden` (one finding for many functions of one mod cut by another) and a disagreement as `function_seen_other`. |
| `game_resources` | `{}` | What the game (each ProjectZomboid64.exe) and the Studio's server take now, read natively: memory against its Java limit, CPU share since the last call, the PC's free memory. The top bar's meter. |
| `game_live` | `user_zomboid_dir` (optional) | The game as it runs now, the Monitor's Live panel as data: `running`, `pid`; the process's memory (`ram_mb`), Java limit (`ram_max_mb`, -Xmx), `cpu_percent`; `link` (the Studio Link 1.20+, every 2 s in a game while the app runs, `Lua/PZModStudio_LinkLive.txt`): `fps` and `frame_ms` of the last 2 s, `worst_ms` of the last 10 s, `heap_mb` (Java heap used, the game's own count), `errors` and `error_mods` (the session's Lua errors by the mod the game blamed), `overlay` (its in-game overlay shows) and `cost_ms_per_s` (the Link's own live code); `profiler` (when `PZModStudio_Profiler` runs, `Lua/pz_profile_live.json` every 2 s): each mod's Lua time over the last 10 s (`ms`, `ms_per_s`, `share_percent`, `calls`, `worst`), its own sampling cost; `profiler_on` (in the active list: runs from the next start; `get_mod_game_time` with `on: true` turns it on); `history`: points of the last 10 minutes (fps, longest frame, memory, heap, CPU, Lua ms/s, errors) the app keeps while it runs. Read-only. |
| `game_code_archive` | `archive_now` (bool, default false), `user_zomboid_dir` | The game's own Lua and scripts of every build the Studio has seen, kept once per build at the Studio's start in `Zomboid/PZModStudio_GameArchive/<build>/game.zip` (~40 MB for 42.22): installed build, folder, each kept build (version, revision, jar size/date, files, bytes). `archive_now` keeps the installed one first when new. The base for telling, after an update, what a mod's copy of game code now duplicates or overwrites. |
| `get_frame_report` | `user_zomboid_dir` (optional) | Frame times and per-mod Lua costs of the current session, from what the Viewpoint Smooth Frames mod (Workshop 3816784011) writes to `console.txt` once a minute: fps, 1% low, frames over 25/50/100 ms, GC time, chunks loaded, and per mod its ms per second of play, worst single frame, minutes with a hitch and costliest handlers (file:line). Read-only; `null` when the mod is off or wrote nothing yet. |
| `get_mod_game_time` | `on` (bool, optional), path parameters | Which mods cost the game the most time while it runs, from the Studio's in-game profiler (`PZModStudio_Profiler`, first in the load order): every event handler of every mod timed and summed per mod (ms, share of real time, longest single call, top handlers with file:line). `on: true/false` switches measuring on (profiler first in `default.txt` and in the last-played save's `mods.txt`; starts with the next launch) or off; without `on` it only reads. |
| `mp_check_mods` | `mod_ids` (array, optional; default: the active list), path parameters | Does the list work in multiplayer: `list_verdict` (`no` / `partial` / `ok`), `not_for_mp` and `partial` (mod ids), and per mod a `verdict` (`no`: its description says single player only; `partial`: an unhandled call in a file that knows nothing of MP; `likely_ok`; `ok`: its description says MP compatible), `declared` with the quoted words, and the calls its client or shared Lua makes directly that the game routes through the server (the handlers of `server/ClientCommands.lua` and `Vehicles/VehicleCommands.lua`, 42.21), each with file, line, handler, `guarded`, `aware` and `same_as_game` (the game's own file makes the same call: a copied vanilla file, not held against the mod). Only matters on a server or in co-op. |
| `scan_mod_security` | `mod_ids` (array, optional; default: the active list), path parameters | What mods could do beyond modding, as heuristics (never a verdict): program files shipped, Java whose classes reference running programs, shell commands, the network, class loading, credential files, Discord webhooks or input control, and Lua that calls what the game's scripts never call (`os.execute`, `loadstring`, a URL, a path outside the mod, obfuscation). Per mod: risk and findings with rule, file, line, detail; cached until the mod's files change. Diagnostics carries the same as `security_executable` / `security_java` / `security_lua`. |
| `java_patches` | `mod_ids` (array, optional; default: the active list), `detail` (bool, default false), path parameters | Java patch radar: which classes each Java mod changes and where two change the same one. Read from each mod's jar (`javaJarFile=`, found the way ZombieBuddy finds it) without running it: the class-level `@Patch(className, methodName)` annotations in the package ZombieBuddy scans (`javaPkgName=`). Per mod: `jar`, `package`, `side` (client/server-only jar), `classes` (target class -> method names), `patch_count`, `not_applied` (in the jar but not applied: `outside_package`, `zombiebuddy_class`, `exposer`, `experimental`), `patches_in_code` (its own code uses Instrumentation/ByteBuddy/ASM directly: changes no annotation declares, unknown here), `unknown` (`jar_missing`, `no_package`, `unreadable`: nothing is guessed). `detail: true` adds every patch with `kind` (`advice` adds code and stacks, `delegation` replaces the method), declared `effects` (`enter`, `exit`, `skip`, `return`, `args`) and `by_code` (its package is handed to ZombieBuddy by the mod's own code). `overlaps`: one per class two or more mods patch, `same_method` (with the shared methods) or `same_class`, `owner_mod` when the class is another mod's rather than the game's, `delegations_meet`. An overlap is not a conflict: that depends on what each patch does. Diagnostics carries the overlaps on the game's classes as `java_patch_overlap` (warning: same method; info: same class, different methods). Cached per mod until its jar or `mod.info` changes. |
| `java_links` | `mod_ids` (array, optional; default: the active list), path parameters | What each Java mod's jar uses of the game that the installed build no longer has, read without running it: every class, method and field of the game its class files name, looked up in the installed `projectzomboid.jar` through superclasses and interfaces (a lookup that leaves the game's jar counts as there), and ZombieBuddy `@Patch` targets that are gone (ZombieBuddy skips such a patch). Per mod: `broken` (kind `class`/`method`/`field`/`patch`, class, member, descriptor, `used_by`), `total`, `checked`, `unknown`. Diagnostics shows it as `java_link_broken`. Cached per mod until its jar or the game's jar changes. |
| `session_changes` | `user_zomboid_dir` | What changed on disk while the game ran, for the last 10 sessions the app watched: per place (`game`: the game's own folder, which the game never writes; `zomboid` without saves, logs and the Studio's files; `zombiebuddy`) the files added, removed and changed. Diagnostics: `game_folder_changed`. |
| `server_settings` | `name`, `set_server`, `set_sandbox` (objects), `user_zomboid_dir` | Every option of a server's settings as the game lists them (the Studio Link writes `Lua/PZModStudio_LinkSettings.txt` at the main menu: server options and the sandbox screen's pages with the mods' options; type, default, range, label, help, choices). Without `name`: the list; with it: the values of `<name>.ini` and `<name>_SandboxVars.lua`; with `set_*`: writes those options only (checked against the list, comments and order kept, missing ones added where they belong, both files backed up to `PZModStudio_Backups/server_settings`), refused while the Studio runs that server. |
| `check_mod_api` | `mod_ids` (array, optional; default: the active list), `active_mod_ids` (the list whose definitions count, as the game loads it; default: `default.txt`), path parameters | Names a mod's Lua calls that the installed build does not have, before launching: global calls no one defines, `:method(` calls no Java class (Kahlua `@LuaMethod` names included) and no Lua file defines, `Events.X.Add` of events nothing triggers or adds; the mod's own and the other active mods' definitions count as known, a call the file also names without calling (`if foo then`) is guarded and left out. Per finding: kind, name, file, line, a near known name. Read from the installed `projectzomboid.jar` and `media/lua` (cached per build) and each mod's Lua (cached per mod). By name only; `has_java` says the mod's Java may define names Lua cannot show. |
| `save_inside` | `save_id` (`<mode>/<world>`, required), path parameters | What a save holds, from its own databases: each character with name, position, dead or alive and the item types it carries (counted); each vehicle with its script, position, distance from the first character and the item types in it. Items are read off the type names inside the game's blobs: approximate. Read-only. |
| `save_remove` | `save_id` (required), `user_zomboid_dir` | Takes a save out of the saves folder into the Studio's bin (`Zomboid/PZModStudio_Backups/saves`): nothing is deleted, its zip backups stay. Refused while the game runs and for the world of the running Studio server. |
| `save_rename` | `save_id`, `new_name` (required), `user_zomboid_dir` | Renames a save with its backups and what the Studio remembers of it. Refused while the game runs, for a taken or invalid name, and for a multiplayer save found by its name (a server's world, a server joined). |
| `saves_bin` | `action` (`list`/`restore`/`delete`/`empty`), `mode`, `folder`, `user_zomboid_dir` | The bin of removed saves: list them, put one back, delete one for good, or empty it. Deleting cannot be undone. |
| `saves_lost` | `action` (`list`/`recover`/`delete`), `save_id`, `file`, `user_zomboid_dir` | Saves no longer on disk that still have backups: list them, make one again from a backup (the newest by default), or delete all its backups for good. |
| `save_mod_maps` | `save_id` (`<mode>/<world>`, optional; required with fix), `fix` (bool), path parameters | Mod maps a save does not load: a save keeps the map folders it was made with (`map_ver.bin`) and the game never adds to them, so a map mod or an RV-interior mod switched on later is loaded without its map (bare land where its cells should be). Reports per save the folders of its own mods that its list misses, with the chunks already generated bare in their cells; with `fix` it backs the save up, puts those folders in front of the list and moves the bare chunks to the backups folder. Refused while the game runs. |
| `workshop_search` | `text`, `sort` (relevance/trending/most_subscribed/recently_updated/newest/top_rated), `build` ("41"/"42"), `category`, `page`, `per_page` | Searches the Workshop the way the Explore page does (its public browse page; no key or login, cached): id, title, summary, builds, tags, subscribers, favourites, size, last update, page link; `total`, `pages`. Read-only. |
| `workshop_comments` | `id` (Workshop id or page link, required), `start` (int, default 0 = newest), `count` (int, default 20, max 50) | What people say about one item in its public comment thread, newest first: author, profile link, when (unix seconds and Steam's wording), plain text with line breaks, `by_author` (the item's author replying); `total`; `unavailable` with Steam's reason when the thread is not public (author's profile private). Cached five minutes, polite; the Explore page shows the same. Read-only. |
| `workshop_item` | `id` (required: a Workshop id or its page link), `workshop_dir` (optional) | What the Steam Workshop says about one item: title, author, builds, posted/updated, subscribers, favourites, size, tags, full description, screenshots, videos, required items; and `installed` (Steam's record on this PC, null when not subscribed). Public Web API and page, no key or login, cached; the same data as the app's Explore page. Read-only. |
| `community_index_lookup` | `mod_id` (optional), `user_zomboid_dir` | What the Studio's mod index (`community/index.json` in this repository, built into the app; nothing is downloaded) says about one mod, or the whole index: builds, requires, incompatible, load-after, successor, known issues with the fix the Studio applies, each entry verified or reported on a build. The issues this player dismissed are left out. |
| `learned_on_this_pc` | `mod_id` (optional), `learn_now` (bool, default false), `user_zomboid_dir` | What this PC learned from its own sessions (`Zomboid/PZModStudio_Learned.json`; nothing is sent): every session log the game keeps in `Zomboid/Logs` read once with the Monitor's card maker (the app does it in the background every few minutes), which mods were on in which version, the errors the game blamed on each. Returns the sessions read on the installed build and each error a mod gave in 3 or more of them with the same version: `seen` of `on`, first/last, `closed` (its last 5 sessions with the mod went without it). A mod update starts its count again; a Java patch the log blames on several mods is not learned. Open ones for active mods are Diagnostics findings (`learned_issue`) unless the built-in index already says it. |
| `brain_overview` | `active_mod_ids` (optional; default `default.txt`), `user_zomboid_dir` | The brain's view of this PC's sessions, read-only: `recent` sessions (how far each got: `game`/`menu`/`started`/none, clean exit or cut off, out of memory, load seconds), `insights` (`error_onset`: when an active mod's repeated error began and what changed then, from the last session without it to the first with it: mods added, removed or updated, its place in the order, the game build, ranked: the build, the mod, what it requires or what requires it, the rest; `load_slower`; `game_update`: three or more mods began failing right after a game update with none of them updated), `fixes` (each fix Diagnostics applied and whether the mod's error stopped: `worked`/`not_worked`/`waiting`/`mod_off`/`no_errors_before`) `crashes` (Java's own crash reports, `hs_err_pid*.log` in the game's folder: `out_of_memory` when Windows gave Java no more memory, `fatal` with its reason and library; each matched to its session, which the briefs then show as fallen over; closing with X or Alt+F4 leaves none) and `prelaunch` (the active list's last 3 sessions failed to start, ran out of memory or crashed, with what differs from the last one that loaded a save; the app asks before launching). What changed is said as what was different then, not as the cause. |
| `brain_mod_history` | `mod_id` (required), `user_zomboid_dir` | One mod's history on this PC, read-only: sessions it was on, its place in the last load order, the versions seen, each error the game blamed on it (seen of on since it began, stopped, onset), the fixes applied to it and their outcome, sessions with it on that failed to start or ran out of memory. The Mod List shows it as "On this PC". |
| `mod_notes_and_groups` | `user_zomboid_dir` | The notes and the active profile's groups as the Mod List shows them: `notes` (the player's own), `studio_notes` (the Studio's), `groups` (name, mods, `note`, `studio_note`, `by_studio`). Read-only. |
| `brain_suspects` | `what` (`any`/`failed_to_start`/`crash`), `since` (unix s), `user_zomboid_dir` | Which mods go with what goes wrong: the mods on in the game sessions (installed build) that went wrong far more than the sessions without them, with the counts (`with`/`with_bad`/`without`/`without_bad`) and `confidence` (strong/likely/weak). A mod on in nearly every session is left out. A correlation, not proof. Read-only. |
| `brain_marks` | `user_zomboid_dir` | The moments the player marked in the game with the Studio Link's "Mark this moment" (pause menu), newest first: when, save, place, the session, and what the log says from 90 s before to 30 s after (the Monitor's cards and the first ERROR/WARN lines). From the Link's diary (`Lua/PZModStudio_LinkDiary.txt`: start / menu / mark), so it covers sessions played with the app closed. Read-only. |
| `brain_period` | `since` (unix s, required), `user_zomboid_dir` | What happened since then: game and server sessions, minutes played, how many went badly, Java crashes, errors that began in the period, fixes applied and their outcome, what changed. Read-only. |
| `learned_index_drafts` | `user_zomboid_dir` | Draft entries for the app's own mod index (`community/index/<id>.json`) from what this PC confirmed: each open learned error seen in 5+ sessions of the installed version that the index does not say yet, with log line, file, line, build, date, the session count as its source, `verified`, `mod_versions`. The note and the fix are left for a person to write; nothing is written. |
| `studio_note` | `mod_id` or `group`, `text` (empty removes), `user_zomboid_dir` | Writes, changes or removes the Studio's note on a mod (or a group of the active profile), shown apart from the player's and marked as the Studio's. The player's own notes are never written, changed or removed. |
| `mod_group` | `action` (`add`/`remove`/`rename`/`delete`), `group`, `mod_ids`, `to`, `user_zomboid_dir` | Changes the active profile's groups: puts mods in a group (made, marked `by_studio`, when new), takes them out (an empty group goes), renames or deletes a group (not its mods). The player's note on a group is never changed. |
| `dismiss_index_issue` | `mod_id`, `title` (required), `dismissed` (bool, default true) | Marks one known issue of the index as no longer happening on this PC (Diagnostics stops showing it), or brings it back; kept in `Zomboid/PZModStudio_LocalIndex.json`. |
| `list_load_rules` / `add_load_rule` / `remove_load_rule` | `mod_id`, `after` (required for add/remove), `note` (add) | The player's own load-order rules ("mod_id loads after after"), in `Zomboid/PZModStudio_Rules.json`. Auto-Sort follows them like `loadModAfter`; Diagnostics reports a broken one as `user_rule_order`. |

---

## 4. 📚 MCP Resource Catalog (*Resources*)

Passive read endpoints available via `resources/read`:

1. `pz://monitor/console-log`: Real-time streaming content of `console.txt`.
2. `pz://mods/installed-summary`: JSON summary of detected mods and load order.
3. `pz://paths/config`: System path configuration (`StudioPaths`).
4. `pz://patches/status`: Master patch package and draft resolution status.

---

## 5. 🧠 Technical Knowledge Base & Modular Wiki (`docs/`)

For in-depth architectural details, refer to the [**Modular Technical Wiki**](docs/INDEX.md):

- **[01. Engine Architecture, JVM & Kahlua VM](docs/01-engine-architecture-kahlua-jvm.md):** Java vs Lua types (List vs Table, 0-based vs 1-based indexing).
- **[02. Lua Script Lifecycle & Event Bus](docs/02-lua-lifecycle-and-events.md):** `shared`/`client`/`server` phases, alphabetical order, and Monkey Patching.
- **[03. Item Definitions, Crafting & Fluid API (B42)](docs/03-crafting-items-and-fluids-b42.md):** `craftRecipe`, `Tags`, physical inheritance, and `FluidContainer`.
- **[04. UI Hierarchy, Context Menus & Timed Actions](docs/04-ui-context-menu-and-timedactions.md):** `ISUIElement`, `OnFillWorldObjectContextMenu`, and `ISBaseTimedAction` state machine.
- **[05. Loot Distribution & Procedural Spawning](docs/05-loot-distribution-and-spawns.md):** `ItemPickerJava`, `ProceduralDistributions.lua`, and container APIs.
- **[06. ModData Persistence, Networking & Security](docs/06-networking-moddata-and-security.md):** Persistence scopes, client-server sync, and CHAP anti-cheat attestation.
- **[07. FMOD Audio, JSON Translations & 3D Z-Levels](docs/07-sound-translations-and-b42-space.md):** Sound scripts, mandatory JSON format (B42.15+), and Z-coordinates (-32 to +32).
- **[08. Crash Diagnostics, VFS & 3-Way AST Merging](docs/08-crash-diagnostics-and-vfs.md):** `console.txt` error taxonomy and 3-way AST merge.
- **[09. Game Process Control & Live IPC Bridge](docs/09-game-control-and-ipc-bridge.md):** `ProjectZomboid64.exe` lifecycle control, live in-game execution and the Claude Agent protocol.

---

## 6. 💻 Development & Build Commands

```powershell
# Compile the standalone MCP server (Fast, no GUI):
cargo build --bin pz-mcp-server --manifest-path "src-tauri/Cargo.toml"

# Run the full desktop application in development mode:
npm run tauri dev

# Build production bundle:
npm run build
```

**Linux:** the same app and `pz-mcp-server` build on Linux (Tauri's Linux packages: webkit2gtk-4.1, gtk3, libssl, appindicator, librsvg). From this Windows PC, `node scripts/build-linux.cjs [--setup] [--out <folder>]` builds the AppImage, the .deb (with `/usr/bin/pz-mcp-server`) and the console binary inside WSL (`Ubuntu-24.04`). Platform differences live in `src-tauri/src/platform.rs` (processes from `/proc`, signals, symlinks instead of junctions, memory). On Linux the game is `projectzomboid.sh` / `ProjectZomboid64` in `steamapps/common/ProjectZomboid` (Steam in `~/.local/share/Steam`, `~/.steam`, the Flatpak's, plus `libraryfolders.vdf`), servers start `jre64/bin/java` with the class path of `ProjectZomboid64.json`, SteamCMD is the distribution's (`steamcmd.sh`, never downloaded), and secrets are a 0600 file (`~/.config/PZModStudio/secrets.json`). Not available on Linux: `game_window`, `capture_game_screenshot`, `game_click_window`, `game_send_key`, the window guard, `windows_low_memory`, the in-place app update, subscribing through the Steam client and the Windows "who holds this file" check.
