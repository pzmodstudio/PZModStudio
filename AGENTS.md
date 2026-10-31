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
| `launch_game` | `debug_mode` (bool), `windowed` (bool), `nosteam` (bool), `extra_args` (array)<br>`agent_mode`, `park`, `no_focus_steal` (bool), `park_width`, `park_height` (int) | Launches `ProjectZomboid64.exe` with configurable debug flags and windowed mode. `agent_mode: true` = no `-debug`, `-windowed`, window parked behind others and focus handed back while it boots. |
| `terminate_game` | `pid` (int optional), `force` (bool default true) | Closes or forcefully terminates the Project Zomboid process. |
| `send_game_ipc_command` | `command` (object: `give_item`, `eval_lua`, `set_godmode`, etc.) | Sends a command to the active game session through the companion mod `Z_PZModStudio_Bridge`. |
| `get_game_ipc_response` | `{}` | Reads the latest execution response returned by the companion mod. |
| `install_bridge_companion_mod` | `user_zomboid_dir` (optional)<br>`include_claude_agent` (bool, default false) | Installs the companion mod `Z_PZModStudio_Bridge` into `Zomboid/mods` to enable the IPC bridge; optionally also `Z_ClaudeAgent`. |
| `get_monitor_logs` | `max_lines` (int, default 100)<br>`errors_only` (bool)<br>`user_zomboid_dir` (string optional) | Reads and filters recent lines from the active log file `console.txt`. |
| `list_available_logs` | `user_zomboid_dir` (optional) | Lists all session log files on disk (`console.txt` and `Zomboid/Logs/`). |
| `read_log_file` | `file_path` (string required)<br>`max_lines` (int)<br>`errors_only` (bool) | Reads a specific log file with exception filtering. |
| `get_crash_diagnostics` | `user_zomboid_dir` (string optional) | Parses Java/Lua exceptions and generates Build 41 & 42 diagnostic cards with recommended fixes and polyfills. Each card carries `severity`: `EXCEPTION` (an `ERROR` line or a stack trace — something threw) or `WARNING` (a `WARN` line — the game noticed and carried on). |
| `get_error_report` | `index` (int, default 0)<br>`raw_error` (string optional)<br>path parameters | Everything needed to fix **one** error from the player's last session, as Markdown: the log line, the lines around it, the installed build, the mod the game blamed (id, version, Workshop link, folder, requirements) and the source of the Lua file that failed, with a note when another active mod ships the same path. Read-only. With no arguments it reports the most recent error the game attributed to a mod and lists the others with their indices; `index` walks back through them, `raw_error` picks one by its log line. |

### Claude Agent (menus + in-game control, window, screenshots)
Uses the `Z_ClaudeAgent` companion mod (source: `mods/Z_ClaudeAgent/`). Unlike the bridge it runs in the main menu, new-game / sandbox / character / spawn screens and while paused, and returns values. Protocol and helpers: [docs/09 §5](docs/09-game-control-and-ipc-bridge.md#5-claude-agent-protocol-z_claudeagent). The `agent_*` tools accept `timeout_ms` (default 15000) and `user_zomboid_dir`; `status` `err`/`timeout` sets `isError`.

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
| `game_window` | `mode` (`park`/`minimize`/`show`/`info`), `width`, `height` | Park = 0,0 at width x height, bottom of the z-order, not activated; show = restore + foreground. |
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
| `get_mod_health` | `active_mod_ids` (array, optional; default: `default.txt`)<br>`include_catalog` (bool)<br>path parameters | Read-only check of the active list using the game's own rules: not installed, not loadable on the installed build, invalid `mod.info`, missing/disabled dependencies, dependency or `loadModAfter`/`loadModBefore` order, incompatible pairs, duplicates; and the active package's copies of mod files the mod has changed since (`package_file_stale`, with `remove_from_package`) or that no mod in the list ships (`package_file_orphan`). Issues carry severity, code and suggested fixes. |
| `sort_mod_load_order` | `order` (array, optional; default: `default.txt`)<br>path parameters | Game-exact sort: each mod after what it requires and its `loadModAfter` mods, before its `loadModBefore` mods, everything else kept in place, Studio packages last. Read-only; returns `{order, moved, cycle}`. |
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
| `publish_file_into_package` | `relative_path`, `content` (required)<br>`package_folder_name` (default `Z_PZModStudio_MergedPatch`) | Puts one corrected file into a package and publishes it **in place**: everything the package holds is published again beside it, a packaged package stays packaged, the game reads it on its next launch. The way to patch a mod's own file without touching the mod (the package loads last). |
| `remove_file_from_package` | `file` (required; a path in the package or just the file name)<br>`package_folder_name` | Takes a file out of a package and publishes it again without it; the game reads the mod's own copy again. |
| `keep_mod_version` | `mod_id` (required), path parameters | Copies the mod as installed today to `Zomboid/PZModStudio_Snapshots/<folder>/<when>/`, for when a Workshop update breaks a save. The Mod List can put it back as a local copy that wins over the Workshop one. |
| `game_memory` | `mb` (int, optional), path parameters | The memory the game's Java may use (`-Xmx` in `ProjectZomboid64.json`): now, the PC's, the most offered (half the PC's), the suggestion for the active list, the active profile and whether it has its own value, recent out-of-memory sessions, and `windows_low_memory`: the times Windows itself ran out of memory with the game open in the last 14 days (Resource-Exhaustion-Detector event 2004: commit limit and charge, the game's share, the other processes holding memory, whether the game was closed for it). With `mb` it sets it from the next start (backup kept), as the active profile's value and the last chosen; refused while the game runs. Switching a profile on gives the game its value. |
| `get_frame_report` | `user_zomboid_dir` (optional) | Frame times and per-mod Lua costs of the current session, from what the Viewpoint Smooth Frames mod (Workshop 3816784011) writes to `console.txt` once a minute: fps, 1% low, frames over 25/50/100 ms, GC time, chunks loaded, and per mod its ms per second of play, worst single frame, minutes with a hitch and costliest handlers (file:line). Read-only; `null` when the mod is off or wrote nothing yet. |
| `get_mod_game_time` | `on` (bool, optional), path parameters | Which mods cost the game the most time while it runs, from the Studio's in-game profiler (`PZModStudio_Profiler`, first in the load order): every event handler of every mod timed and summed per mod (ms, share of real time, longest single call, top handlers with file:line). `on: true/false` switches measuring on (profiler first in `default.txt` and in the last-played save's `mods.txt`; starts with the next launch) or off; without `on` it only reads. |
| `mp_check_mods` | `mod_ids` (array, optional; default: the active list), path parameters | Does the list work in multiplayer: `list_verdict` (`no` / `partial` / `ok`), `not_for_mp` and `partial` (mod ids), and per mod a `verdict` (`no`: its description says single player only; `partial`: an unhandled call in a file that knows nothing of MP; `likely_ok`; `ok`: its description says MP compatible), `declared` with the quoted words, and the calls its client or shared Lua makes directly that the game routes through the server (the handlers of `server/ClientCommands.lua` and `Vehicles/VehicleCommands.lua`, 42.21), each with file, line, handler, `guarded`, `aware` and `same_as_game` (the game's own file makes the same call: a copied vanilla file, not held against the mod). Only matters on a server or in co-op. |
| `scan_mod_security` | `mod_ids` (array, optional; default: the active list), path parameters | What mods could do beyond modding, as heuristics (never a verdict): program files shipped, Java whose classes reference running programs, shell commands, the network, class loading, credential files, Discord webhooks or input control, and Lua that calls what the game's scripts never call (`os.execute`, `loadstring`, a URL, a path outside the mod, obfuscation). Per mod: risk and findings with rule, file, line, detail; cached until the mod's files change. Mod Health carries the same as `security_executable` / `security_java` / `security_lua`. |
| `check_mod_api` | `mod_ids` (array, optional; default: the active list), path parameters | Names a mod's Lua calls that the installed build does not have, before launching: global calls no one defines, `:method(` calls no Java class (Kahlua `@LuaMethod` names included) and no Lua file defines, `Events.X.Add` of events nothing triggers or adds; the mod's own and the other active mods' definitions count as known, a call the file also names without calling (`if foo then`) is guarded and left out. Per finding: kind, name, file, line, a near known name. Read from the installed `projectzomboid.jar` and `media/lua` (cached per build) and each mod's Lua (cached per mod). By name only; `has_java` says the mod's Java may define names Lua cannot show. |
| `save_inside` | `save_id` (`<mode>/<world>`, required), path parameters | What a save holds, from its own databases: each character with name, position, dead or alive and the item types it carries (counted); each vehicle with its script, position, distance from the first character and the item types in it. Items are read off the type names inside the game's blobs: approximate. Read-only. |
| `save_mod_maps` | `save_id` (`<mode>/<world>`, optional; required with fix), `fix` (bool), path parameters | Mod maps a save does not load: a save keeps the map folders it was made with (`map_ver.bin`) and the game never adds to them, so a map mod or an RV-interior mod switched on later is loaded without its map (bare land where its cells should be). Reports per save the folders of its own mods that its list misses, with the chunks already generated bare in their cells; with `fix` it backs the save up, puts those folders in front of the list and moves the bare chunks to the backups folder. Refused while the game runs. |
| `community_index_lookup` | `mod_id` (optional), `user_zomboid_dir` | What the Studio's mod index (`community/index.json` in this repository, built into the app; nothing is downloaded) says about one mod, or the whole index: builds, requires, incompatible, load-after, successor, known issues with the fix the Studio applies, each entry verified or reported on a build. The issues this player dismissed are left out. |
| `dismiss_index_issue` | `mod_id`, `title` (required), `dismissed` (bool, default true) | Marks one known issue of the index as no longer happening on this PC (Mod Health stops showing it), or brings it back; kept in `Zomboid/PZModStudio_LocalIndex.json`. |
| `list_load_rules` / `add_load_rule` / `remove_load_rule` | `mod_id`, `after` (required for add/remove), `note` (add) | The player's own load-order rules ("mod_id loads after after"), in `Zomboid/PZModStudio_Rules.json`. Auto-Sort follows them like `loadModAfter`; Mod Health reports a broken one as `user_rule_order`. |

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
