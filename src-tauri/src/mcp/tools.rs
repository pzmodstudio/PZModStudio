use super::protocol::{McpTool, ToolCallResult, ToolContent};
use crate::claude_agent;
use crate::diff_engine::lua::{three_way_merge_lua, validate_lua_syntax};
use crate::load_order::mod_info::scan_all_installed_mods;
use crate::instance_manager::{activate_instance, create_instance, list_instances};
use crate::patch_generator::{get_master_patch_status, list_merged_packages, save_draft_resolution};
use crate::sandbox::{
    find_running_pz_pid, install_bridge_companion_mod, kill_pz_game, launch_pz_direct,
    list_available_log_files, read_game_ipc_response, read_log_file, translate_log_error,
    write_game_ipc_command, TranslatedErrorPayload,
};
use crate::vfs::{scan_conflicts, StudioPaths};
use serde_json::{json, Value};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Returns all tool definitions exposed by the MCP server.
pub fn get_available_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "get_studio_paths".to_string(),
            description: "Auto-detects and returns Project Zomboid installation, Steam workshop, and user data directory paths.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        McpTool {
            name: "get_game_status".to_string(),
            description: "Checks if ProjectZomboid64.exe is currently running on the system and returns its Process ID (PID).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        McpTool {
            name: "launch_game".to_string(),
            description: "Launches ProjectZomboid64.exe process with configurable flags such as debug mode, windowed mode, and nosteam. With agent_mode=true it starts windowed, without -debug, parked behind other windows and without keeping focus (for Claude Agent play).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pz_install_dir": { "type": "string", "description": "Optional custom game installation path" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "debug_mode": { "type": "boolean", "description": "If true, launches with -debug flag (default: true, or false when agent_mode is true; under -debug the Lua debugger breaks on errors even inside pcall)" },
                    "windowed": { "type": "boolean", "description": "If true, forces windowed execution with -windowed (default: true)" },
                    "nosteam": { "type": "boolean", "description": "If true, launches with -nosteam (default: false)" },
                    "extra_args": { "type": "array", "items": { "type": "string" }, "description": "Optional extra CLI arguments" },
                    "agent_mode": { "type": "boolean", "description": "Agent play preset: no -debug, -windowed, park the window and give focus back (default: false)" },
                    "park": { "type": "boolean", "description": "Keep the new game window parked at the bottom of the z-order, not activated (default: agent_mode)" },
                    "no_focus_steal": { "type": "boolean", "description": "Hand focus back to the previously focused window while the game boots (default: agent_mode)" },
                    "park_width": { "type": "integer", "description": "Parked window width (default: 1280)" },
                    "park_height": { "type": "integer", "description": "Parked window height (default: 720)" }
                }
            }),
        },
        McpTool {
            name: "terminate_game".to_string(),
            description: "Terminates the running ProjectZomboid64.exe process cleanly or forcefully via OS process management.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pid": { "type": "integer", "description": "Optional specific Process ID (PID) to terminate. If omitted, terminates any ProjectZomboid64.exe" },
                    "force": { "type": "boolean", "description": "If true, forces immediate termination (/F flag) (default: true)" }
                }
            }),
        },
        McpTool {
            name: "send_game_ipc_command".to_string(),
            description: "Sends an in-game IPC command (e.g. give_item, set_godmode, eval_lua) to a live Project Zomboid session via the Z_PZModStudio_Bridge companion mod.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "command": {
                        "type": "object",
                        "description": "The command object to execute. Example: {\"action\": \"give_item\", \"item\": \"Base.Axe\", \"equip\": true} or {\"action\": \"eval_lua\", \"code\": \"getPlayer():Say('Hello!')\"} or {\"action\": \"set_godmode\"}"
                    }
                },
                "required": ["command"]
            }),
        },
        McpTool {
            name: "get_game_ipc_response".to_string(),
            description: "Reads the latest execution response emitted by the in-game companion bridge mod.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "read_app_doc".to_string(),
            description: "Reads PZ Mod Studio's own help pages, written for the player: what each screen of the app does and how to use it (profiles, saves, monitor, mod-list, whats-new, health, browse, maps, merger, developer, servers, settings; \"index\" is the overview and the in-game Studio Link mod). With no page, lists them. Use it to answer how-to questions about the Studio as it really works.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page": { "type": "string", "description": "Page name, e.g. \"mod-list\" (omit to list the pages)" },
                    "language": { "type": "string", "description": "\"en\" (default) or \"es\"" }
                }
            }),
        },
        McpTool {
            name: "install_bridge_companion_mod".to_string(),
            description: "Installs the Z_PZModStudio_Bridge companion mod into the user's Zomboid/mods directory to enable live in-game Lua execution and item equipping. Optionally also installs the Z_ClaudeAgent mod.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "include_claude_agent": { "type": "boolean", "description": "If true, also installs the Z_ClaudeAgent companion mod used by the agent_* tools (default: false)" }
                }
            }),
        },
        McpTool {
            name: "install_claude_agent_mod".to_string(),
            description: "Installs the Z_ClaudeAgent companion mod into Zomboid/mods. It runs Lua sent by the agent_* tools in the main menu, new-game/sandbox/character screens, spawn select and in game (also while paused). Enable it in the game's Mods list afterwards.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "agent_eval_lua".to_string(),
            description: "Runs Lua in the live game through the Z_ClaudeAgent mod (works in the main menu and in game) and waits for the answer. The code may `return` a value; it comes back serialized (JSON when possible). The global CA has helpers: CA.state(), CA.ui(filter), CA.find(text), CA.click(text), CA.answer('YES'|'NO'|'OK'), CA.stats(), CA.zombies(r), CA.speed(n), CA.walkTo(x,y,z), CA.inventory(), CA.methods(obj,filter), CA.fields(obj,filter), CA.task(name,ms,fn)/CA.untask(name), CA.log(...).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "Lua source to run, e.g. \"return CA.state()\"" },
                    "wait_idle_ms": { "type": "integer", "description": "Before running the code, wait up to this long for the player's timed-action queue to empty (polls CA.busy().actions until 0). Default: 0 (no wait)" },
                    "timeout_ms": { "type": "integer", "description": "How long to wait for the answer (default: 15000)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                },
                "required": ["code"]
            }),
        },
        McpTool {
            name: "agent_ui_list".to_string(),
            description: "Lists visible UI elements (buttons, ticks, combos, text entries, labels) with type, position, size and label via CA.ui(filter). Works in menus and in game.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "filter": { "type": "string", "description": "Optional case-insensitive label substring or exact type name (e.g. ISButton)" },
                    "timeout_ms": { "type": "integer", "description": "How long to wait for the answer (default: 15000)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "agent_ui_click".to_string(),
            description: "Clicks a visible button or menu entry by its text via CA.click(text) (exact label first, then substring). Retries while the element is not found yet, since menus build over several frames.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Visible label of the element to click" },
                    "type_name": { "type": "string", "description": "Optional UI type to restrict the match (e.g. ISButton)" },
                    "retry_ms": { "type": "integer", "description": "Keep retrying (about once per second) while the answer is 'not found' for up to this long (default: 20000, 0 = single try)" },
                    "timeout_ms": { "type": "integer", "description": "How long to wait for the answer (default: 15000)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                },
                "required": ["text"]
            }),
        },
        McpTool {
            name: "agent_dialog_answer".to_string(),
            description: "Answers the front-most modal dialog via CA.answer(answer). Works even when UI mods hide the vanilla buttons.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "answer": { "type": "string", "enum": ["YES", "NO", "OK"], "description": "Dialog answer (default: YES)" },
                    "timeout_ms": { "type": "integer", "description": "How long to wait for the answer (default: 15000)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "agent_game_state".to_string(),
            description: "Returns CA.state(): whether in game, main screen visibility, paused, game speed, in-game time and player position/dead flag.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timeout_ms": { "type": "integer", "description": "How long to wait for the answer (default: 15000)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "capture_game_screenshot".to_string(),
            description: "Captures the Project Zomboid main window to a PNG with PrintWindow (works while the window is covered by other windows; a minimized window is restored behind the others first). Returns the saved path.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Output PNG path (default: <temp>/pz-mod-studio/game_screenshot.png)" },
                    "max_width": { "type": "integer", "description": "Downscale to at most this width in pixels, 0 = full size (default: 1280)" }
                }
            }),
        },
        McpTool {
            name: "game_window".to_string(),
            description: "Controls the Project Zomboid window without stealing focus: park (restore if minimized, move to 0,0, resize, send behind all windows, no activation), minimize (no activation), show (restore and bring to the foreground), info (rect, client size, foreground, minimized).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mode": { "type": "string", "enum": ["park", "minimize", "show", "info"], "description": "Window action" },
                    "width": { "type": "integer", "description": "Width for park (default: 1280)" },
                    "height": { "type": "integer", "description": "Height for park (default: 720)" }
                },
                "required": ["mode"]
            }),
        },
        McpTool {
            name: "game_click_window".to_string(),
            description: "Posts a left click (WM_MOUSEMOVE + WM_LBUTTONDOWN/UP) at a fraction of the game window's client area without focusing it. Used e.g. to get past the black 'click to start' screen, where Lua does not run.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x_frac": { "type": "number", "description": "Horizontal position as a fraction of the client width, 0..1 (default: 0.5)" },
                    "y_frac": { "type": "number", "description": "Vertical position as a fraction of the client height, 0..1 (default: 0.5)" }
                }
            }),
        },
        McpTool {
            name: "agent_enter_game".to_string(),
            description: "After Continue/Play: waits for the world to load and gets past the black 'click to start' screen (GameLoadingState suppresses Lua, so the mod cannot answer there). Polls CA.state() every ~3 s; once the mod has been silent for more than 20 s it posts a centre click each round. Returns clicks sent and the final state.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timeout_ms": { "type": "integer", "description": "Give up after this long (default: 300000)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "game_send_key".to_string(),
            description: "Posts a key press (WM_KEYDOWN/WM_KEYUP with scan codes) to the Project Zomboid window without focusing it. Modifiers are pressed first and released last. Keys: A-Z, 0-9, F1-F12, ESC, SPACE, TAB, ENTER, BACKSPACE, DELETE, INSERT, HOME, END, PAGEUP, PAGEDOWN, UP/DOWN/LEFT/RIGHT, SHIFT, CTRL, ALT, SEMICOLON, APOSTROPHE, COMMA, PERIOD, MINUS, EQUALS, TILDE.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Key or combination, e.g. \"O\", \"SHIFT+O\", \"F9\", \"ESC\"" },
                    "hold_ms": { "type": "integer", "description": "How long to hold the keys down (default: 80)" }
                },
                "required": ["key"]
            }),
        },
        McpTool {
            name: "get_monitor_logs".to_string(),
            description: "Reads recent lines from Project Zomboid's console.txt log file, with optional line limit and error filtering.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": {
                        "type": "string",
                        "description": "Optional custom path to the user Zomboid folder (e.g. C:\\Users\\Name\\Zomboid)"
                    },
                    "max_lines": {
                        "type": "integer",
                        "description": "Maximum number of lines to retrieve from the end of console.txt (default: 100)"
                    },
                    "errors_only": {
                        "type": "boolean",
                        "description": "If true, only returns lines containing ERROR, Exception, Stacktrace, or Callframe"
                    }
                }
            }),
        },
        McpTool {
            name: "get_crash_diagnostics".to_string(),
            description: "Scans console.txt in real time to extract Lua and Java exceptions, stacktraces, and actionable repair cards for PZ Build 41 & 42.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": {
                        "type": "string",
                        "description": "Optional custom path to the user Zomboid folder"
                    }
                }
            }),
        },
        McpTool {
            name: "game_memory".to_string(),
            description: "The memory the game's Java may use (-Xmx in ProjectZomboid64.json): now, the PC's, the most the Studio offers (half the PC's), what it suggests for the active mod list, the active profile and whether it has its own value, and the recent sessions that ran out of memory. With mb it sets the value from the next start, keeps it as the active profile's (switching that profile on gives it back) and as the last chosen; refused while the game runs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mb": { "type": "integer", "description": "Memory to give the game, in MB (2048 up to half the PC's); leave out to only read" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom path to the user Zomboid folder" },
                    "pz_install_dir": { "type": "string", "description": "Optional game folder" }
                }
            }),
        },
        McpTool {
            name: "get_mod_game_time".to_string(),
            description: "Which mods cost the game the most time while it runs: the Studio's in-game profiler (PZModStudio_Profiler) times every event handler of every mod and this adds them up per mod (ms, share of real time, longest single call, top handlers with file:line). With on=true/false it switches measuring on (the profiler first in default.txt and in the last-played save's mods.txt; it starts with the next game launch) or off. Without on it only reads.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "on": { "type": "boolean", "description": "Switch measuring on or off; leave out to only read the last measurement" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom path to the user Zomboid folder" },
                    "workshop_dir": { "type": "string", "description": "Optional Steam Workshop content folder (108600)" }
                }
            }),
        },
        McpTool {
            name: "list_installed_mods".to_string(),
            description: "Scans and lists all installed Project Zomboid mods from Steam Workshop and local user mods folder, including mod IDs, names, versions, and compatibility tags.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pz_install_dir": { "type": "string", "description": "Optional custom game install directory" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "workshop_dir": { "type": "string", "description": "Optional custom workshop content directory (e.g. 108600)" }
                }
            }),
        },
        McpTool {
            name: "sort_mod_load_order".to_string(),
            description: "Sorts a load order with the game's own mod.info rules: each mod after what it requires and its loadModAfter mods, before its loadModBefore mods, everything else kept in place, PZ Mod Studio packages last. Read-only: returns {order, moved, cycle}. Defaults to the active list in default.txt.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string" },
                    "workshop_dir": { "type": "string" },
                    "order": { "type": "array", "items": { "type": "string" }, "description": "Mod ids to sort instead of default.txt" }
                }
            }),
        },
        McpTool {
            name: "get_mod_health".to_string(),
            description: "Checks the active mod list (Zomboid/mods/default.txt) against the installed mods using the game's own rules: mods not installed, not loadable on the installed build (B41-only, versionMin/versionMax), invalid mod.info, missing or disabled dependencies, dependencies loading after their dependents, loadModAfter/loadModBefore order, incompatible pairs, duplicates. Read-only. Each issue has a severity, a code and suggested fixes.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string" },
                    "workshop_dir": { "type": "string" },
                    "active_mod_ids": { "type": "array", "items": { "type": "string" }, "description": "Load order to check instead of default.txt" },
                    "include_catalog": { "type": "boolean", "description": "Also return every installed mod (large)" }
                }
            }),
        },
        McpTool {
            name: "get_error_report".to_string(),
            description: "Everything needed to fix one error from the player's last session, as Markdown: the log line, the lines around it, the installed game build, the mod the game blamed (id, version, Workshop link, folder, requirements) and the source of the file that failed. Reads console.txt and the installed mods; changes nothing. Without arguments it reports the most recent error the game attributed to a mod, and lists the others. Use `index` to walk back through them, or `raw_error` to pick one by its log line.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "index": { "type": "integer", "description": "0 is the most recent attributed error, 1 the one before it, and so on" },
                    "raw_error": { "type": "string", "description": "Report on the attributed error whose log line contains this text" },
                    "user_zomboid_dir": { "type": "string" },
                    "workshop_dir": { "type": "string" },
                    "pz_install_dir": { "type": "string" }
                }
            }),
        },
        McpTool {
            name: "scan_mod_conflicts".to_string(),
            description: "Scans active mods for Virtual File System (VFS) conflicts where multiple mods overwrite the same Lua scripts or PZ data definitions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pz_install_dir": { "type": "string" },
                    "user_zomboid_dir": { "type": "string" },
                    "workshop_dir": { "type": "string" }
                }
            }),
        },
        McpTool {
            name: "validate_lua_syntax".to_string(),
            description: "Validates Lua code syntax using full_moon AST parser, returning precise line, column, and syntax error messages if invalid.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "The raw Lua code to validate" }
                },
                "required": ["code"]
            }),
        },
        McpTool {
            name: "merge_lua_scripts".to_string(),
            description: "Performs a 3-way AST merge on Lua scripts: Base (vanilla), Target A (Mod A), and Target B (Mod B), producing a unified, conflict-free script.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "base": { "type": "string", "description": "Vanilla base Lua script" },
                    "target_a": { "type": "string", "description": "Variant A Lua script" },
                    "target_b": { "type": "string", "description": "Variant B Lua script" }
                },
                "required": ["base", "target_a", "target_b"]
            }),
        },
        McpTool {
            name: "get_master_patch_status".to_string(),
            description: "Queries the current status of Z_PZModStudio_MasterPatch package, including generated patches and draft resolutions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "package_folder_name": { "type": "string", "description": "Package folder name (default: MasterPatch)" }
                }
            }),
        },
        McpTool {
            name: "save_draft_resolution".to_string(),
            description: "Saves a resolved Lua script or PZ data definition directly into the active Master Patch draft resolutions.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "package_folder_name": { "type": "string", "description": "Package folder name (default: MasterPatch)" },
                    "relative_path": { "type": "string", "description": "Relative file path inside the mod (e.g. media/lua/shared/ISInventoryPane.lua)" },
                    "resolved_content": { "type": "string", "description": "The merged or patched file content" },
                    "status": { "type": "string", "description": "Resolution status: MANUAL, AST_AUTO, or AI_RESOLVED (default: AI_RESOLVED)" }
                },
                "required": ["relative_path", "resolved_content"]
            }),
        },
        McpTool {
            name: "publish_file_into_package".to_string(),
            description: "Puts one corrected file into a Studio package and publishes it in place: everything the package already holds is published again beside it, a packaged package stays packaged, and the game reads the file on its next launch. This is how a mod's own file is patched without touching the mod: the package loads last, so the game reads the package's copy of that path. Prefer this over save_draft_resolution + a manual publish.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "package_folder_name": { "type": "string", "description": "Package folder (default: Z_PZModStudio_MergedPatch)" },
                    "relative_path": { "type": "string", "description": "The path the game reads, e.g. media/lua/shared/aFancyHandwork.lua" },
                    "content": { "type": "string", "description": "The whole corrected file" }
                },
                "required": ["relative_path", "content"]
            }),
        },
        McpTool {
            name: "get_frame_report".to_string(),
            description: "Frame times and per-mod Lua costs of the current session, from what the Viewpoint Smooth Frames mod (Workshop 3816784011) writes to console.txt once a minute: fps, 1% low, frames over 25/50/100 ms, GC time, chunks loaded, and for each mod its ms per second of play, its worst single frame and its costliest handlers (file:line). Read-only; null when the mod is not on or wrote nothing yet.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "remove_file_from_package".to_string(),
            description: "Takes one file out of a Studio package and publishes it again without it; the game reads the mod's own copy of that path on its next launch. `file` is the path in the package or just the file's name.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "package_folder_name": { "type": "string", "description": "Package folder (default: Z_PZModStudio_MergedPatch)" },
                    "file": { "type": "string", "description": "media/lua/client/MoodlesIn.lua, or MoodlesIn.lua" }
                },
                "required": ["file"]
            }),
        },
        McpTool {
            name: "keep_mod_version".to_string(),
            description: "Keeps a copy of a mod as it is installed today under Zomboid/PZModStudio_Snapshots, for when a Workshop update breaks a save. Returns the kept version (path, date, modversion). The Studio's Mod List can put it back as a local copy that wins over the Workshop one.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "workshop_dir": { "type": "string", "description": "Optional Workshop content directory" },
                    "mod_id": { "type": "string", "description": "The mod id as in mod.info" }
                },
                "required": ["mod_id"]
            }),
        },
        McpTool {
            name: "mp_check_mods".to_string(),
            description: "The multiplayer check: the calls a mod's client or shared Lua makes directly that the game, in multiplayer, only does on the server (the handlers of media/lua/server/ClientCommands.lua and Vehicles/VehicleCommands.lua, 42.21), one row per call with file, line, the game's handler as evidence, whether it sits in an isClient() branch (guarded) and whether the file mentions isClient() at all (aware). Checks only those calls. Default: every active mod.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mod_ids": { "type": "array", "items": { "type": "string" }, "description": "Mod ids to check; default: the active list" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "workshop_dir": { "type": "string" }
                }
            }),
        },
        McpTool {
            name: "scan_mod_security".to_string(),
            description: "What mods could do beyond modding, as heuristics (never a verdict): program files shipped (.exe, .dll, .bat, .ps1...), Java code whose classes reference running programs, shell commands, the network, class loading, Steam or browser credential files, Discord webhooks, input control (Build 42 runs a mod's Java with the player's rights, through ZombieBuddy), and Lua that calls what the game's own scripts never call (os.execute, io.popen, loadstring, dofile, a URL, a path outside the mod, obfuscation). Per mod: risk (info/low/medium/high), findings with rule, file, line and detail. Cached per mod until its files change. Default: the active list.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mod_ids": { "type": "array", "items": { "type": "string" }, "description": "Mod ids to scan (default: the active list)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "workshop_dir": { "type": "string", "description": "Optional Workshop content directory" }
                }
            }),
        },
        McpTool {
            name: "check_mod_api".to_string(),
            description: "Names a mod's Lua calls that the installed build does not have, before the game is launched: a global call no one defines (not the game's Java globals or Lua, not the mod, not another active mod, not Lua itself), a method call (:name() no Java class and no Lua file defines, Kahlua's @LuaMethod names included), an Events.X.Add whose X nothing triggers or adds. Read from the installed projectzomboid.jar and media/lua (cached per build) and from each mod's Lua for the installed build's layers (cached per mod). A call the file also names without calling (if foo then, type(foo) == 'function') is taken as guarded and left out. Per finding: kind, name, file, line and a near known name when there is one. By name only: a method that some class has passes, a name built at run time is not seen, and a mod's Java (has_java) may define names Lua cannot show. Default: the active list.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mod_ids": { "type": "array", "items": { "type": "string" }, "description": "Mod ids to check (default: the active list)" },
                    "pz_install_dir": { "type": "string", "description": "Optional game install directory" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "workshop_dir": { "type": "string", "description": "Optional Workshop content directory" }
                }
            }),
        },
        McpTool {
            name: "save_inside".to_string(),
            description: "What a save holds, from its own databases (players.db, vehicles.db): each character with name, position (x, y, z), dead or alive and the item types it carries (counted); each vehicle with its script (Base.CarNormal), position, distance in squares from the first character and the item types in it. The items are read off the type names inside the game's blobs, so they are approximate (a reference to a type counts as one). Read-only.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "save_id": { "type": "string", "description": "<mode>/<world>, e.g. Sandbox/2026-10-29_00-05-06 (list_saves of the Saves screen, or the Saves folder)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                },
                "required": ["save_id"]
            }),
        },
        McpTool {
            name: "save_mod_maps".to_string(),
            description: "Mod maps a save does not load. A save keeps the map folders it was made with (map_ver.bin) and the game never adds to them, so a map mod, or a mod with interiors on cells of its own (the RV interior mods), switched on after the save was made is loaded without its map: where its cells should be the game generates bare land. Without fix: for each save, the folders of its own mods that its list misses, each with the chunks the save already generated bare in its cells. With fix: true and save_id: backs the save up, puts those folders in front of its list (as a new game would) and moves the bare chunks out of the save into the backups folder. Refused while the game runs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "save_id": { "type": "string", "description": "<mode>/<world>, e.g. Sandbox/2026-10-26_23-06-16; required with fix, otherwise narrows the report" },
                    "fix": { "type": "boolean", "description": "Apply the fix to save_id (game closed)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "workshop_dir": { "type": "string" },
                    "pz_install_dir": { "type": "string" }
                }
            }),
        },
        McpTool {
            name: "community_index_lookup".to_string(),
            description: "What the community mod index says about one mod (or every mod, without mod_id): builds it runs on, requires, incompatible with, load after, its Build 42 successor, and known issues each with the fix the Studio can apply (shim_patch, require_path, compatible_copy, none), every entry with the build it was seen on and whether it was verified or reported. Fetched once a day from the Studio's repository (community/index.json), cached beside the mod list; null when none is at hand.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "mod_id": { "type": "string", "description": "The id= line of mod.info; omit for the whole index" }
                }
            }),
        },
        McpTool {
            name: "refresh_community".to_string(),
            description: "Fetches the community rules (community/rules.json) and the mod index (community/index.json) now, whatever the age of the cached copies, and says how many of each came.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "set_community_downloads".to_string(),
            description: "Switches the daily download of the community rules and index on or off (Settings > Download the community rules and mod index). Off keeps the last copies and fetches nothing.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "enabled": { "type": "boolean" }
                },
                "required": ["enabled"]
            }),
        },
        McpTool {
            name: "list_load_rules".to_string(),
            description: "The player's own load-order rules (\"mod_id loads after after\", with a note), from Zomboid/PZModStudio_Rules.json. Auto-Sort follows them like loadModAfter; Mod Health reports a broken one as user_rule_order.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "add_load_rule".to_string(),
            description: "Adds a load-order rule of the player's own: mod_id must load after `after`. Say why in `note`. Refused when the pair exists the other way round.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "mod_id": { "type": "string" },
                    "after": { "type": "string" },
                    "note": { "type": "string", "description": "Why, in a few words" }
                },
                "required": ["mod_id", "after"]
            }),
        },
        McpTool {
            name: "remove_load_rule".to_string(),
            description: "Removes the rule that mod_id loads after `after`.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "mod_id": { "type": "string" },
                    "after": { "type": "string" }
                },
                "required": ["mod_id", "after"]
            }),
        },
        McpTool {
            name: "list_mod_profiles".to_string(),
            description: "Lists all saved mod profiles/instances, their enabled mod IDs, custom load orders, and active status.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "create_mod_profile".to_string(),
            description: "Creates a new named mod profile with specific active mod IDs and load order.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Profile name (e.g. Brita Overhaul B42)" },
                    "description": { "type": "string", "description": "Optional profile description" },
                    "active_mod_ids": { "type": "array", "items": { "type": "string" }, "description": "List of active mod IDs" },
                    "load_order": { "type": "array", "items": { "type": "string" }, "description": "List of mod IDs in load order" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                },
                "required": ["name"]
            }),
        },
        McpTool {
            name: "activate_mod_profile".to_string(),
            description: "Activates a saved mod profile by ID, writing its active mods and load order directly to default.txt / ModListData.ini.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "profile_id": { "type": "string", "description": "The unique ID of the profile to activate (e.g. inst_1787112345)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                },
                "required": ["profile_id"]
            }),
        },
        McpTool {
            name: "list_available_logs".to_string(),
            description: "Discovers and lists all Project Zomboid log files on disk (console.txt and timestamped session logs in Zomboid/Logs/).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "read_log_file".to_string(),
            description: "Reads lines from any specific log file on disk with optional line limits and error filtering.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Absolute path to the log file to read" },
                    "max_lines": { "type": "integer", "description": "Maximum number of lines to read from the end (default: 200)" },
                    "errors_only": { "type": "boolean", "description": "If true, only returns lines with errors or exceptions" }
                },
                "required": ["file_path"]
            }),
        },
        McpTool {
            name: "list_merged_packages".to_string(),
            description: "Lists all fusion packages (Z_PZModStudio_*), their active state in mod list, and packaged files.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" },
                    "mod_list_ini_path": { "type": "string", "description": "Optional path to default.txt or ModListData.ini" }
                }
            }),
        },
    ]
}

/// Dispatches a tool execution request by name.
pub fn execute_tool(name: &str, args: Value) -> ToolCallResult {
    match name {
        "get_studio_paths" => handle_get_studio_paths(),
        "get_game_status" => handle_get_game_status(),
        "launch_game" => handle_launch_game(args),
        "terminate_game" => handle_terminate_game(args),
        "send_game_ipc_command" => handle_send_game_ipc_command(args),
        "get_game_ipc_response" => handle_get_game_ipc_response(args),
        "read_app_doc" => handle_read_app_doc(args),
        "install_bridge_companion_mod" => handle_install_bridge_companion_mod(args),
        "install_claude_agent_mod" => handle_install_claude_agent_mod(args),
        "agent_eval_lua" => handle_agent_eval_lua(args),
        "agent_ui_list" => handle_agent_ui_list(args),
        "agent_ui_click" => handle_agent_ui_click(args),
        "agent_dialog_answer" => handle_agent_dialog_answer(args),
        "agent_game_state" => handle_agent_game_state(args),
        "capture_game_screenshot" => handle_capture_game_screenshot(args),
        "game_window" => handle_game_window(args),
        "game_send_key" => handle_game_send_key(args),
        "game_click_window" => handle_game_click_window(args),
        "agent_enter_game" => handle_agent_enter_game(args),
        "get_monitor_logs" => handle_get_monitor_logs(args),
        "get_crash_diagnostics" => handle_get_crash_diagnostics(args),
        "get_error_report" => handle_get_error_report(args),
        "list_installed_mods" => handle_list_installed_mods(args),
        "sort_mod_load_order" => handle_sort_mod_load_order(args),
        "scan_mod_conflicts" => handle_scan_mod_conflicts(args),
        "get_mod_health" => handle_get_mod_health(args),
        "game_memory" => {
            let paths = resolve_paths(&args);
            match args.get("mb").and_then(|v| v.as_u64()) {
                Some(mb) => json_result(crate::game_memory::set(&paths.pz_install_dir, &paths.user_zomboid_dir, mb)),
                None => json_result(Ok::<_, String>(crate::game_memory::status(&paths.pz_install_dir, &paths.user_zomboid_dir))),
            }
        }
        "get_mod_game_time" => {
            let paths = resolve_paths(&args);
            match args.get("on").and_then(|v| v.as_bool()) {
                Some(on) => json_result(crate::sandbox::profiler::set_on(&paths.user_zomboid_dir, &paths.workshop_dir, on)),
                None => json_result(Ok::<_, String>(crate::sandbox::profiler::status(&paths.user_zomboid_dir, &paths.workshop_dir))),
            }
        }
        "validate_lua_syntax" => handle_validate_lua_syntax(args),
        "merge_lua_scripts" => handle_merge_lua_scripts(args),
        "get_master_patch_status" => handle_get_master_patch_status(args),
        "save_draft_resolution" => handle_save_draft_resolution(args),
        "list_mod_profiles" => handle_list_mod_profiles(args),
        "create_mod_profile" => handle_create_mod_profile(args),
        "activate_mod_profile" => handle_activate_mod_profile(args),
        "list_available_logs" => handle_list_available_logs(args),
        "read_log_file" => handle_read_log_file(args),
        "list_merged_packages" => handle_list_merged_packages(args),
        "publish_file_into_package" => handle_publish_file_into_package(args),
        "remove_file_from_package" => handle_remove_file_from_package(args),
        "get_frame_report" => handle_get_frame_report(args),
        "keep_mod_version" => handle_keep_mod_version(args),
        "community_index_lookup" => handle_community_index_lookup(args),
        "mp_check_mods" => handle_mp_check_mods(args),
        "save_mod_maps" => handle_save_mod_maps(args),
        "save_inside" => handle_save_inside(args),
        "scan_mod_security" => handle_scan_mod_security(args),
        "check_mod_api" => handle_check_mod_api(args),
        "refresh_community" => handle_refresh_community(args),
        "set_community_downloads" => handle_set_community_downloads(args),
        "list_load_rules" => handle_list_load_rules(args),
        "add_load_rule" => handle_add_load_rule(args),
        "remove_load_rule" => handle_remove_load_rule(args),
        _ => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Unknown tool: '{}'", name),
            }],
            is_error: true,
        },
    }
}

fn resolve_paths(args: &Value) -> StudioPaths {
    let mut paths = crate::vfs::studio_paths();
    if let Some(pz) = args.get("pz_install_dir").and_then(|v| v.as_str()) {
        if !pz.is_empty() {
            paths.pz_install_dir = pz.to_string();
        }
    }
    if let Some(uz) = args.get("user_zomboid_dir").and_then(|v| v.as_str()) {
        if !uz.is_empty() {
            paths.user_zomboid_dir = uz.to_string();
            paths.mod_list_ini_path = Path::new(uz)
                .join("mods")
                .join("default.txt")
                .to_string_lossy()
                .to_string();
        }
    }
    if let Some(ws) = args.get("workshop_dir").and_then(|v| v.as_str()) {
        if !ws.is_empty() {
            paths.workshop_dir = ws.to_string();
        }
    }
    paths
}

fn handle_get_studio_paths() -> ToolCallResult {
    let paths = crate::vfs::studio_paths();
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(&paths).unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

fn handle_get_game_status() -> ToolCallResult {
    let pid_opt = find_running_pz_pid();
    let result = json!({
        "is_running": pid_opt.is_some(),
        "process_id": pid_opt,
        "process_name": "ProjectZomboid64.exe",
        "description": if pid_opt.is_some() {
            "Project Zomboid is actively running."
        } else {
            "Project Zomboid is not running."
        }
    });

    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

fn handle_get_monitor_logs(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let max_lines = args.get("max_lines").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
    let errors_only = args.get("errors_only").and_then(|v| v.as_bool()).unwrap_or(false);

    let console_path = Path::new(&paths.user_zomboid_dir).join("console.txt");
    if !console_path.exists() {
        return ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("console.txt log file not found at: {}", console_path.display()),
            }],
            is_error: true,
        };
    }

    match File::open(&console_path) {
        Ok(file) => {
            // Streamed, keeping only the last max_lines matches: the log can be tens of megabytes.
            let mut kept: std::collections::VecDeque<String> = std::collections::VecDeque::new();
            let mut matched = 0usize;
            for line in crate::sandbox::lossy_lines(BufReader::new(file)) {
                let wanted = !errors_only
                    || line.contains("ERROR")
                    || line.contains("Exception")
                    || line.contains("Kahlua")
                    || line.contains("Callframe:")
                    || line.contains("Stacktrace:")
                    || line.contains("at zombie.");
                if wanted {
                    matched += 1;
                    kept.push_back(line);
                    if kept.len() > max_lines {
                        kept.pop_front();
                    }
                }
            }

            let result = json!({
                "log_file": console_path.to_string_lossy(),
                "total_matched_lines": matched,
                "returned_lines_count": kept.len(),
                "lines": kept
            });

            ToolCallResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {}", e)),
                }],
                is_error: false,
            }
        }
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to read console.txt: {}", e),
            }],
            is_error: true,
        },
    }
}

/// One error, with everything around it, written for whoever has to fix it.
fn handle_get_error_report(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let Some(console_path) = crate::load_order::mod_info::get_all_user_zomboid_dirs(&paths.user_zomboid_dir)
        .into_iter()
        .map(|d| d.join("console.txt"))
        .find(|p| p.is_file())
    else {
        return error_result(
            "No console.txt yet. Start Project Zomboid once; the game writes its log there.".to_string(),
        );
    };
    let Ok(text) = std::fs::read_to_string(&console_path) else {
        return error_result(format!("{} could not be read.", console_path.display()));
    };

    // The game names the mod itself, on the line or in its "overrides" lines at startup.
    let lines: Vec<&str> = text.lines().collect();
    let mut blame = crate::sandbox::ModBlame::default();
    let mut attributed: Vec<(usize, String, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (i, line) in lines.iter().enumerate() {
        blame.observe(line);
        if !(line.starts_with("ERROR") || line.starts_with("WARN")) {
            continue;
        }
        if let Some(name) = blame.of(line) {
            let key = format!("{}|{}", name, line.chars().take(80).collect::<String>());
            if seen.insert(key) {
                attributed.push((i, (*line).to_string(), name));
            }
        }
    }
    if attributed.is_empty() {
        return error_result(format!(
            "No error in {} was attributed to a mod. The log may be from a clean session.",
            console_path.display()
        ));
    }

    // Newest first, which is what someone asking about "the error" means.
    attributed.reverse();
    let wanted = match args.get("raw_error").and_then(|v| v.as_str()) {
        Some(needle) if !needle.trim().is_empty() => {
            attributed.iter().position(|(_, line, _)| line.contains(needle.trim()))
        }
        _ => Some(args.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize),
    };
    let Some(pick) = wanted.filter(|i| *i < attributed.len()) else {
        return error_result(format!(
            "No attributed error matches that. There are {} to choose from (index 0 to {}).",
            attributed.len(),
            attributed.len() - 1
        ));
    };

    let (at, line, mod_name) = attributed[pick].clone();
    let from = at.saturating_sub(15);
    let to = (at + 10).min(lines.len());
    let request = crate::error_report::ErrorReportRequest {
        raw_error: line.clone(),
        title: format!("{} reported a problem", mod_name),
        explanation: line.split_once('>').map(|(_, r)| r.trim().to_string()).unwrap_or_default(),
        mod_name: Some(mod_name),
        source_file: line
            .split(")).")
            .nth(1)
            .and_then(|r| r.split('>').next())
            .map(|f| f.trim().to_string())
            .filter(|f| f.ends_with(".lua")),
        log_context: lines[from..to].iter().map(|l| l.to_string()).collect(),
    };

    let mut report = match crate::error_report::build(&paths, &request) {
        Ok(text) => text,
        Err(e) => return error_result(e),
    };

    if attributed.len() > 1 {
        report.push_str(&format!(
            "\n## The other {} attributed error(s) in this log\n\nAsk for these with `index`:\n\n",
            attributed.len() - 1
        ));
        for (i, (_, other_line, other_mod)) in attributed.iter().enumerate() {
            if i == pick {
                continue;
            }
            let detail = other_line.split_once('>').map(|(_, r)| r.trim()).unwrap_or(other_line);
            report.push_str(&format!(
                "- index {}: {} — {}\n",
                i,
                other_mod,
                detail.chars().take(110).collect::<String>()
            ));
        }
    }

    ToolCallResult {
        content: vec![ToolContent { content_type: "text".to_string(), text: report }],
        is_error: false,
    }
}

fn handle_get_crash_diagnostics(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let console_path = Path::new(&paths.user_zomboid_dir).join("console.txt");

    if !console_path.exists() {
        return ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("console.txt not found at: {}", console_path.display()),
            }],
            is_error: true,
        };
    }

    match File::open(&console_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut error_cards: Vec<TranslatedErrorPayload> = Vec::new();
            let mut raw_error_snippets: Vec<String> = Vec::new();
            let mut counter = 1;
            // The game writes the same failure once per frame: a session's log holds a thousand
            // error lines and a handful of problems. One card each, with how often it happened,
            // and the mod named a line or two above where the error itself names nobody.
            let mut times: Vec<usize> = Vec::new();
            let mut blame = crate::sandbox::ModBlame::default();

            for line_res in reader.lines() {
                if let Ok(line) = line_res {
                    blame.observe(&line);
                    if let Some(mut card) = translate_log_error(&line, counter) {
                        if card.mod_name.is_none() {
                            card.mod_name = blame.of(&line);
                        }
                        let shape = error_shape(&card.raw_error);
                        match error_cards.iter().position(|c: &TranslatedErrorPayload| {
                            c.title == card.title && error_shape(&c.raw_error) == shape
                        }) {
                            Some(at) => times[at] += 1,
                            None => {
                                error_cards.push(card);
                                times.push(1);
                                counter += 1;
                            }
                        }
                    } else if line.contains("Exception") || line.contains("ERROR:") {
                        if raw_error_snippets.len() < 30 {
                            raw_error_snippets.push(line);
                        }
                    }
                }
            }

            let cards: Vec<Value> = error_cards
                .iter()
                .enumerate()
                .map(|(i, card)| {
                    let mut value = serde_json::to_value(card).unwrap_or_else(|_| json!({}));
                    if let Some(object) = value.as_object_mut() {
                        object.insert("occurrences".to_string(), json!(times[i]));
                    }
                    value
                })
                .collect();
            let total: usize = times.iter().sum();

            let pid_opt = find_running_pz_pid();
            let result = json!({
                "game_running": pid_opt.is_some(),
                "pid": pid_opt,
                "diagnosed_error_cards_count": cards.len(),
                "diagnosed_error_lines": total,
                "diagnosed_error_cards": cards,
                "raw_error_sample_lines": raw_error_snippets
            });

            ToolCallResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Error: {}", e)),
                }],
                is_error: false,
            }
        }
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to read console.txt: {}", e),
            }],
            is_error: true,
        },
    }
}

/// An error with its numbers blurred, so the same failure on a hundred frames is one problem.
fn error_shape(line: &str) -> String {
    let body = line
        .split_once('>')
        .map(|(_, rest)| rest)
        .unwrap_or(line)
        .trim();
    let blurred: String = body
        .chars()
        .map(|c| if c.is_ascii_digit() { 'N' } else { c })
        .collect();
    blurred.chars().take(90).collect()
}

fn handle_list_installed_mods(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let manifests = scan_all_installed_mods(&paths);

    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(&json!({
                "total_installed_mods": manifests.iter().filter(|m| !m.not_installed).count(),
                "mods": manifests
            }))
            .unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

fn handle_sort_mod_load_order(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let order: Vec<String> = match args.get("order").and_then(|v| v.as_array()) {
        Some(list) => list.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        None => crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default(),
    };
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let analysis = crate::catalog::sort_load_order(&catalog, &order);

    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(&analysis).unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

fn handle_get_mod_health(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let active: Vec<String> = match args.get("active_mod_ids").and_then(|v| v.as_array()) {
        Some(list) => list.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        None => crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default(),
    };
    let health = crate::catalog::mod_health_full(&paths, &active);
    let body = if args.get("include_catalog").and_then(|v| v.as_bool()).unwrap_or(false) {
        serde_json::to_value(&health)
    } else {
        serde_json::to_value(&health.report)
    };
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: body.map(|v| serde_json::to_string_pretty(&v).unwrap_or_default()).unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

fn handle_scan_mod_conflicts(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let conflicts = scan_conflicts(&paths);

    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(&json!({
                "total_vfs_conflicts": conflicts.len(),
                "conflicts": conflicts
            }))
            .unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

fn handle_validate_lua_syntax(args: Value) -> ToolCallResult {
    let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code.is_empty() {
        return ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: "Missing or empty 'code' argument to validate_lua_syntax.".to_string(),
            }],
            is_error: true,
        };
    }

    let check = validate_lua_syntax(code);
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(&check).unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

fn handle_merge_lua_scripts(args: Value) -> ToolCallResult {
    let base = args.get("base").and_then(|v| v.as_str()).unwrap_or("");
    let target_a = args.get("target_a").and_then(|v| v.as_str()).unwrap_or("");
    let target_b = args.get("target_b").and_then(|v| v.as_str()).unwrap_or("");

    let res = three_way_merge_lua(base, target_a, target_b);
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(&res).unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

fn handle_get_master_patch_status(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let pkg_name = args.get("package_folder_name").and_then(|v| v.as_str()).map(|s| s.to_string());

    let status = get_master_patch_status(&paths.user_zomboid_dir, &paths.mod_list_ini_path, pkg_name);
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(&status).unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

fn handle_save_draft_resolution(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let pkg_name = args.get("package_folder_name").and_then(|v| v.as_str()).unwrap_or("MasterPatch");
    let relative_path = args.get("relative_path").and_then(|v| v.as_str()).unwrap_or("");
    let resolved_content = args.get("resolved_content").and_then(|v| v.as_str()).unwrap_or("");
    let status = args.get("status").and_then(|v| v.as_str()).unwrap_or("AI_RESOLVED");

    if relative_path.is_empty() || resolved_content.is_empty() {
        return ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: "Both 'relative_path' and 'resolved_content' are required arguments.".to_string(),
            }],
            is_error: true,
        };
    }

    match save_draft_resolution(
        &paths.user_zomboid_dir,
        pkg_name,
        relative_path,
        resolved_content,
        status,
    ) {
        Ok(ok) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&json!({
                    "success": ok,
                    "relative_path": relative_path,
                    "package": pkg_name,
                    "status": status,
                    "message": "Draft resolution successfully saved to Master Patch folder."
                }))
                .unwrap_or_else(|e| format!("Error: {}", e)),
            }],
            is_error: false,
        },
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to save draft resolution: {}", e),
            }],
            is_error: true,
        },
    }
}

fn handle_launch_game(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let agent_mode = args.get("agent_mode").and_then(|v| v.as_bool()).unwrap_or(false);
    let debug_mode = args.get("debug_mode").and_then(|v| v.as_bool()).unwrap_or(!agent_mode);
    let windowed = args.get("windowed").and_then(|v| v.as_bool()).unwrap_or(true);
    let nosteam = args.get("nosteam").and_then(|v| v.as_bool()).unwrap_or(false);
    let park = args.get("park").and_then(|v| v.as_bool()).unwrap_or(agent_mode);
    let no_focus_steal = args.get("no_focus_steal").and_then(|v| v.as_bool()).unwrap_or(agent_mode);
    let park_width = args.get("park_width").and_then(|v| v.as_i64()).unwrap_or(1280) as i32;
    let park_height = args.get("park_height").and_then(|v| v.as_i64()).unwrap_or(720) as i32;
    let mut extra_args = args.get("extra_args").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect::<Vec<String>>()
    }).unwrap_or_default();
    if agent_mode && windowed && !extra_args.iter().any(|a| a.eq_ignore_ascii_case("-windowed")) {
        extra_args.push("-windowed".to_string());
    }

    let prev_foreground = claude_agent::current_foreground_window();
    match launch_pz_direct(&paths.pz_install_dir, &paths.user_zomboid_dir, debug_mode, windowed, nosteam, &extra_args) {
        Ok(pid) => {
            claude_agent::start_launch_window_guard(prev_foreground, park, no_focus_steal, park_width, park_height);
            ToolCallResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: serde_json::to_string_pretty(&json!({
                        "status": "launched",
                        "pid": pid,
                        "debug_mode": debug_mode,
                        "windowed": windowed,
                        "nosteam": nosteam,
                        "agent_mode": agent_mode,
                        "park": park,
                        "no_focus_steal": no_focus_steal,
                        "extra_args": extra_args,
                        "message": if park || no_focus_steal {
                            format!("Project Zomboid launched with PID: {}. A background guard waits for the window and keeps it parked / returns focus for ~45s after it appears.", pid)
                        } else {
                            format!("Project Zomboid launched successfully with PID: {}", pid)
                        }
                    })).unwrap_or_else(|e| format!("Error: {}", e)),
                }],
                is_error: false,
            }
        }
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to launch Project Zomboid: {}", e),
            }],
            is_error: true,
        },
    }
}

fn handle_terminate_game(args: Value) -> ToolCallResult {
    let pid = args.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32);
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(true);

    match kill_pz_game(pid, force) {
        Ok(msg) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&json!({
                    "status": "terminated",
                    "pid": pid,
                    "force": force,
                    "message": msg
                })).unwrap_or_else(|e| format!("Error: {}", e)),
            }],
            is_error: false,
        },
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to terminate game process: {}", e),
            }],
            is_error: true,
        },
    }
}

fn handle_send_game_ipc_command(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let command = match args.get("command") {
        Some(cmd) => cmd.clone(),
        None => return ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: "Missing required argument 'command'.".to_string(),
            }],
            is_error: true,
        },
    };

    match write_game_ipc_command(&paths.user_zomboid_dir, command) {
        Ok(msg) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&json!({
                    "status": "enqueued",
                    "message": msg
                })).unwrap_or_else(|e| format!("Error: {}", e)),
            }],
            is_error: false,
        },
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to enqueue IPC command: {}", e),
            }],
            is_error: true,
        },
    }
}

fn handle_get_game_ipc_response(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);

    match read_game_ipc_response(&paths.user_zomboid_dir) {
        Ok(resp) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&json!({
                    "has_response": resp.is_some(),
                    "response": resp
                })).unwrap_or_else(|e| format!("Error: {}", e)),
            }],
            is_error: false,
        },
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to read IPC response: {}", e),
            }],
            is_error: true,
        },
    }
}

fn handle_install_bridge_companion_mod(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);

    let include_claude_agent = args.get("include_claude_agent").and_then(|v| v.as_bool()).unwrap_or(false);

    match install_bridge_companion_mod(&paths.user_zomboid_dir) {
        Ok(msg) => {
            let agent = if include_claude_agent {
                Some(claude_agent::install_claude_agent_mod(&paths.user_zomboid_dir))
            } else {
                None
            };
            let agent_failed = matches!(agent, Some(Err(_)));
            ToolCallResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: serde_json::to_string_pretty(&json!({
                        "status": if agent_failed { "partial" } else { "installed" },
                        "message": msg,
                        "claude_agent": match &agent {
                            None => Value::Null,
                            Some(Ok(m)) => json!({ "status": "installed", "message": m }),
                            Some(Err(e)) => json!({ "status": "failed", "message": e }),
                        }
                    })).unwrap_or_else(|e| format!("Error: {}", e)),
                }],
                is_error: agent_failed,
            }
        }
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to install bridge companion mod: {}", e),
            }],
            is_error: true,
        },
    }
}

fn handle_read_app_doc(args: Value) -> ToolCallResult {
    let language = args.get("language").and_then(|v| v.as_str()).unwrap_or("en");
    match args.get("page").and_then(|v| v.as_str()).filter(|p| !p.trim().is_empty()) {
        None => text_result(&json!({ "pages": crate::app_docs::list().into_iter().map(|(name, title)| json!({ "page": name, "title": title })).collect::<Vec<_>>() }), false),
        Some(page) => match crate::app_docs::page(page, language) {
            Some(text) => ToolCallResult { content: vec![ToolContent { content_type: "text".to_string(), text: text.to_string() }], is_error: false },
            None => error_result(format!("There is no help page called \"{}\". Call read_app_doc with no page to list them.", page)),
        },
    }
}

fn text_result(value: &Value, is_error: bool) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error,
    }
}

fn error_result(message: String) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: message,
        }],
        is_error: true,
    }
}

fn handle_install_claude_agent_mod(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    match claude_agent::install_claude_agent_mod(&paths.user_zomboid_dir) {
        Ok(msg) => text_result(&json!({ "status": "installed", "mod_id": claude_agent::MOD_ID, "message": msg }), false),
        Err(e) => error_result(format!("Failed to install Claude Agent mod: {}", e)),
    }
}

/// Sends Lua to the Z_ClaudeAgent mod and formats the answer. `status` err/timeout => is_error.
fn run_agent_lua(args: &Value, code: &str) -> ToolCallResult {
    let paths = resolve_paths(args);
    let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(15000);

    match claude_agent::eval_lua(&paths.user_zomboid_dir, code, timeout_ms) {
        Ok(outcome) => {
            let parsed: Option<Value> = serde_json::from_str(outcome.raw.trim()).ok();
            let mut body = json!({
                "id": outcome.id,
                "status": outcome.status,
                "elapsed_ms": outcome.elapsed_ms,
                "result_is_json": parsed.is_some(),
                "result": parsed.unwrap_or_else(|| Value::String(outcome.raw.clone())),
            });
            if outcome.status == "timeout" {
                body["game_running"] = json!(find_running_pz_pid().is_some());
            }
            text_result(&body, outcome.status != "ok")
        }
        Err(e) => error_result(format!("Failed to send command to Z_ClaudeAgent: {}", e)),
    }
}

fn handle_agent_eval_lua(args: Value) -> ToolCallResult {
    let code = match args.get("code").and_then(|v| v.as_str()) {
        Some(c) if !c.trim().is_empty() => c.to_string(),
        _ => return error_result("Missing or empty required argument 'code'.".to_string()),
    };
    let wait_idle_ms = args.get("wait_idle_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    if wait_idle_ms == 0 {
        return run_agent_lua(&args, &code);
    }

    // Poll CA.busy().actions until the timed-action queue is empty or wait_idle_ms passes.
    let paths = resolve_paths(&args);
    let start = std::time::Instant::now();
    let deadline = start + std::time::Duration::from_millis(wait_idle_ms);
    let mut idle = json!({ "waited_ms": 0, "idle": false });
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now()).as_millis() as u64;
        let poll = claude_agent::eval_lua(&paths.user_zomboid_dir, "return CA.busy().actions", remaining.clamp(500, 5000));
        match poll {
            Ok(o) if o.status == "ok" => {
                if o.raw.trim().parse::<f64>().map(|n| n <= 0.0).unwrap_or(false) {
                    idle["idle"] = json!(true);
                    break;
                }
            }
            Ok(o) => {
                idle["poll_error"] = json!(format!("{}: {}", o.status, o.raw));
                break;
            }
            Err(e) => {
                idle["poll_error"] = json!(e);
                break;
            }
        }
        if std::time::Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    idle["waited_ms"] = json!(start.elapsed().as_millis() as u64);

    let mut result = run_agent_lua(&args, &code);
    if let Some(first) = result.content.first_mut() {
        if let Ok(mut body) = serde_json::from_str::<Value>(&first.text) {
            body["idle_wait"] = idle;
            first.text = serde_json::to_string_pretty(&body).unwrap_or_else(|e| format!("Error: {}", e));
        }
    }
    result
}

fn handle_game_send_key(args: Value) -> ToolCallResult {
    let key = match args.get("key").and_then(|v| v.as_str()) {
        Some(k) if !k.trim().is_empty() => k.to_string(),
        _ => return error_result("Missing or empty required argument 'key'.".to_string()),
    };
    let hold_ms = args.get("hold_ms").and_then(|v| v.as_u64()).unwrap_or(80).min(10000);
    match claude_agent::send_game_key(&key, hold_ms) {
        Ok(msg) => text_result(&json!({ "status": "posted", "key": key, "hold_ms": hold_ms, "message": msg }), false),
        Err(e) => error_result(format!("game_send_key '{}' failed: {}", key, e)),
    }
}

fn handle_agent_ui_list(args: Value) -> ToolCallResult {
    let filter = args.get("filter").and_then(|v| v.as_str());
    let code = format!("return CA.ui({})", claude_agent::lua_opt(filter));
    run_agent_lua(&args, &code)
}

fn handle_agent_ui_click(args: Value) -> ToolCallResult {
    let text = match args.get("text").and_then(|v| v.as_str()) {
        Some(t) if !t.is_empty() => t,
        _ => return error_result("Missing or empty required argument 'text'.".to_string()),
    };
    let type_name = args.get("type_name").and_then(|v| v.as_str());
    let code = format!(
        "return CA.click({}, {})",
        claude_agent::lua_quote(text),
        claude_agent::lua_opt(type_name)
    );
    // Screens build over several frames: retry while CA.click answers "not found".
    let retry_ms = args.get("retry_ms").and_then(|v| v.as_u64()).unwrap_or(20000);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(retry_ms);
    let mut attempts = 0u32;
    loop {
        attempts += 1;
        let mut result = run_agent_lua(&args, &code);
        let mut body: Option<Value> = result.content.first().and_then(|c| serde_json::from_str(&c.text).ok());
        let not_found = body
            .as_ref()
            .and_then(|b| b.get("result"))
            .and_then(|r| r.as_str())
            .map(|s| s.contains("not found"))
            .unwrap_or(false);
        if !not_found || std::time::Instant::now() >= deadline {
            if let (Some(b), Some(first)) = (body.as_mut(), result.content.first_mut()) {
                b["attempts"] = json!(attempts);
                if not_found {
                    b["found"] = json!(false);
                }
                first.text = serde_json::to_string_pretty(b).unwrap_or_else(|e| format!("Error: {}", e));
            }
            if not_found {
                result.is_error = true;
            }
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}

fn handle_game_click_window(args: Value) -> ToolCallResult {
    let fx = args.get("x_frac").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let fy = args.get("y_frac").and_then(|v| v.as_f64()).unwrap_or(0.5);
    match claude_agent::click_game_window(fx, fy) {
        Ok((x, y)) => text_result(&json!({ "status": "clicked", "x": x, "y": y, "x_frac": fx, "y_frac": fy }), false),
        Err(e) => error_result(format!("game_click_window failed: {}", e)),
    }
}

fn handle_agent_enter_game(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(300000);
    let outcome = claude_agent::enter_game(&paths.user_zomboid_dir, timeout_ms);
    let state = outcome.last_state.as_deref().map(|s| {
        serde_json::from_str::<Value>(s.trim()).unwrap_or_else(|_| Value::String(s.to_string()))
    });
    text_result(&json!({
        "status": if outcome.in_game { "in_game" } else { "not_in_game" },
        "in_game": outcome.in_game,
        "clicks": outcome.clicks,
        "elapsed_ms": outcome.elapsed_ms,
        "state": state,
        "game_running": find_running_pz_pid().is_some()
    }), !outcome.in_game)
}

fn handle_agent_dialog_answer(args: Value) -> ToolCallResult {
    let answer = args.get("answer").and_then(|v| v.as_str()).unwrap_or("YES").to_uppercase();
    if !matches!(answer.as_str(), "YES" | "NO" | "OK") {
        return error_result(format!("Invalid 'answer' value '{}'. Use YES, NO or OK.", answer));
    }
    let code = format!("return CA.answer({})", claude_agent::lua_quote(&answer));
    run_agent_lua(&args, &code)
}

fn handle_agent_game_state(args: Value) -> ToolCallResult {
    run_agent_lua(&args, "return CA.state()")
}

fn handle_capture_game_screenshot(args: Value) -> ToolCallResult {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(claude_agent::default_screenshot_path);
    let max_width = args.get("max_width").and_then(|v| v.as_u64()).unwrap_or(1280) as u32;

    match claude_agent::capture_game_screenshot(&path, max_width) {
        Ok(info) => text_result(&json!({
            "status": "saved",
            "path": info.path,
            "width": info.width,
            "height": info.height,
            "source_width": info.source_width,
            "source_height": info.source_height,
            "print_window_ok": info.print_window_ok,
            "looks_blank": info.looks_blank
        }), false),
        Err(e) => error_result(format!("Failed to capture game screenshot: {}", e)),
    }
}

fn handle_game_window(args: Value) -> ToolCallResult {
    let mode = match args.get("mode").and_then(|v| v.as_str()) {
        Some(m) => m.to_lowercase(),
        None => return error_result("Missing required argument 'mode' (park, minimize, show, info).".to_string()),
    };
    let width = args.get("width").and_then(|v| v.as_i64()).unwrap_or(1280) as i32;
    let height = args.get("height").and_then(|v| v.as_i64()).unwrap_or(720) as i32;

    match claude_agent::game_window_action(&mode, width, height) {
        Ok((msg, info)) => text_result(&json!({ "status": "ok", "mode": mode, "message": msg, "window": info }), false),
        Err(e) => error_result(format!("game_window '{}' failed: {}", mode, e)),
    }
}

fn handle_list_mod_profiles(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    match list_instances(paths.user_zomboid_dir) {
        Ok(list) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&json!({
                    "total_profiles": list.len(),
                    "profiles": list
                })).unwrap_or_else(|e| format!("Error: {}", e)),
            }],
            is_error: false,
        },
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to list mod profiles: {}", e),
            }],
            is_error: true,
        },
    }
}

fn handle_create_mod_profile(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let name = match args.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: "Missing required argument 'name'.".to_string(),
            }],
            is_error: true,
        },
    };

    let description = args.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
    let active_mod_ids: Vec<String> = args.get("active_mod_ids")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let load_order: Vec<String> = args.get("load_order")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    match create_instance(paths.user_zomboid_dir, name, description, active_mod_ids, load_order) {
        Ok(inst) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&inst).unwrap_or_else(|e| format!("Error: {}", e)),
            }],
            is_error: false,
        },
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to create mod profile: {}", e),
            }],
            is_error: true,
        },
    }
}

fn handle_activate_mod_profile(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let profile_id = match args.get("profile_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: "Missing required argument 'profile_id'.".to_string(),
            }],
            is_error: true,
        },
    };

    match activate_instance(paths.user_zomboid_dir, profile_id) {
        Ok(_) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&json!({
                    "status": "activated",
                    "message": "Mod profile activated and written to default.txt / ModListData.ini"
                })).unwrap_or_else(|e| format!("Error: {}", e)),
            }],
            is_error: false,
        },
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to activate mod profile: {}", e),
            }],
            is_error: true,
        },
    }
}

fn handle_list_available_logs(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let list = list_available_log_files(&paths.user_zomboid_dir);
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(&json!({
                "total_logs": list.len(),
                "logs": list
            })).unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

fn handle_read_log_file(args: Value) -> ToolCallResult {
    let file_path = match args.get("file_path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: "Missing required argument 'file_path'.".to_string(),
            }],
            is_error: true,
        },
    };

    let max_lines = args.get("max_lines").and_then(|v| v.as_u64()).map(|n| n as usize).or(Some(200));
    let errors_only = args.get("errors_only").and_then(|v| v.as_bool()).unwrap_or(false);

    match read_log_file(file_path, max_lines) {
        Ok(lines) => {
            let final_lines: Vec<String> = if errors_only {
                lines.into_iter().filter(|l| {
                    let low = l.to_lowercase();
                    low.contains("error") || low.contains("exception") || low.contains("crash") || low.contains("stacktrace")
                }).collect()
            } else {
                lines
            };

            ToolCallResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: serde_json::to_string_pretty(&json!({
                        "file_path": file_path,
                        "returned_lines": final_lines.len(),
                        "lines": final_lines
                    })).unwrap_or_else(|e| format!("Error: {}", e)),
                }],
                is_error: false,
            }
        },
        Err(e) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Failed to read log file: {}", e),
            }],
            is_error: true,
        },
    }
}

/// One result shape for the tools below: JSON on success, the reason with `isError` on failure.
fn json_result<T: serde::Serialize>(result: Result<T, String>) -> ToolCallResult {
    match result {
        Ok(value) => ToolCallResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&value).unwrap_or_else(|e| format!("Error: {}", e)),
            }],
            is_error: false,
        },
        Err(e) => ToolCallResult {
            content: vec![ToolContent { content_type: "text".to_string(), text: e }],
            is_error: true,
        },
    }
}

fn arg_str<'a>(args: &'a Value, key: &str) -> &'a str {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

fn handle_publish_file_into_package(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let package = args.get("package_folder_name").and_then(|v| v.as_str()).unwrap_or("Z_PZModStudio_MergedPatch");
    let (relative_path, content) = (arg_str(&args, "relative_path"), arg_str(&args, "content"));
    if relative_path.is_empty() || content.is_empty() {
        return json_result::<()>(Err("Both 'relative_path' and 'content' are required.".to_string()));
    }
    json_result(crate::patch_generator::publish_file_into_package(&paths.user_zomboid_dir, &paths.mod_list_ini_path, package, relative_path, content))
}

fn handle_get_frame_report(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    json_result::<Option<crate::sandbox::smooth_frames::SmoothFramesReport>>(Ok(crate::sandbox::smooth_frames::read(&paths.user_zomboid_dir)))
}

fn handle_remove_file_from_package(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let package = args.get("package_folder_name").and_then(|v| v.as_str()).unwrap_or("Z_PZModStudio_MergedPatch");
    let file = arg_str(&args, "file");
    if file.is_empty() {
        return json_result::<()>(Err("'file' is required.".to_string()));
    }
    json_result(crate::patch_generator::remove_file_from_package(&paths.user_zomboid_dir, &paths.mod_list_ini_path, package, file))
}

fn handle_keep_mod_version(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let mod_id = arg_str(&args, "mod_id");
    if mod_id.is_empty() {
        return json_result::<()>(Err("'mod_id' is required.".to_string()));
    }
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let result = catalog
        .resolve(mod_id)
        .ok_or_else(|| format!("{} is not installed.", mod_id))
        .and_then(|m| {
            let workshop_id = match &m.source {
                crate::catalog::ModSource::Workshop { workshop_id } => Some(workshop_id.clone()),
                _ => None,
            };
            crate::snapshots::keep_version(&paths.user_zomboid_dir, &m.id, &m.name, Path::new(&m.root_dir), m.info.mod_version.clone(), workshop_id)
        });
    json_result(result)
}

fn handle_mp_check_mods(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let ids: Vec<String> = match args.get("mod_ids").and_then(|v| v.as_array()) {
        Some(list) if !list.is_empty() => list.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default(),
    };
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let game = (!paths.pz_install_dir.is_empty()).then(|| std::path::PathBuf::from(&paths.pz_install_dir));
    let reports: Vec<Value> = ids
        .iter()
        .filter(|id| !id.starts_with("Z_PZModStudio_"))
        .filter_map(|id| catalog.resolve(id.trim()))
        .filter_map(|m| {
            crate::mplint::assess(std::path::Path::new(&m.root_dir), game.as_deref(), Some(m.info.description.as_str())).ok().map(|r| {
                json!({
                    "mod_id": m.id, "name": m.name, "verdict": r.verdict, "declared": r.declared, "declared_quote": r.declared_quote,
                    "files_checked": r.files_checked, "has_server_lua": r.has_server_lua,
                    "against": r.findings.iter().filter(|f| f.counts_against()).count(), "findings": r.findings
                })
            })
        })
        .collect();
    let count = |v: &str| reports.iter().filter(|r| r["verdict"] == v).count();
    let list_verdict = if count("no") > 0 { "no" } else if count("partial") > 0 { "partial" } else { "ok" };
    let not_for_mp: Vec<&Value> = reports.iter().filter(|r| r["verdict"] == "no").map(|r| &r["mod_id"]).collect();
    let partial: Vec<&Value> = reports.iter().filter(|r| r["verdict"] == "partial").map(|r| &r["mod_id"]).collect();
    json_result(Ok(json!({
        "list_verdict": list_verdict, "not_for_mp": not_for_mp, "partial": partial,
        "scope": "Only matters on a server or in co-op. Verdicts come from each mod's description (single player only / multiplayer compatible) and from calls its client Lua makes that the game only makes on the server; a copied vanilla file is not held against the mod. The check cannot see every way a mod can fail on a server.",
        "mods_checked": ids.len(), "reports": reports
    })))
}

fn handle_scan_mod_security(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let z = std::path::Path::new(&paths.user_zomboid_dir);
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let ids: Vec<String> = match args.get("mod_ids").and_then(|v| v.as_array()) {
        Some(a) if !a.is_empty() => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default(),
    };
    json_result::<Vec<crate::security::ModScan>>(Ok(crate::security::scan_mods(z, &catalog, &ids)))
}

fn handle_check_mod_api(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let active = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default();
    let ids: Vec<String> = match args.get("mod_ids").and_then(|v| v.as_array()) {
        Some(a) if !a.is_empty() => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => active.clone(),
    };
    json_result(crate::api_lint::check_mods(&paths, &catalog, &ids, &active))
}

fn handle_save_inside(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let save_id = arg_str(&args, "save_id");
    if save_id.is_empty() {
        return json_result::<()>(Err("'save_id' is required (<mode>/<world>).".to_string()));
    }
    json_result(crate::save_backup::inside::read(std::path::Path::new(&paths.user_zomboid_dir), save_id))
}

fn handle_save_mod_maps(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let z = std::path::Path::new(&paths.user_zomboid_dir);
    let cells = crate::map_cells::map_cells(&paths.pz_install_dir, &paths.workshop_dir, z);
    let fix = args.get("fix").and_then(|v| v.as_bool()).unwrap_or(false);
    let save_id = args.get("save_id").and_then(|v| v.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if fix {
        let Some(id) = save_id else { return json_result::<Value>(Err("fix needs save_id (<mode>/<world>).".to_string())) };
        let running = crate::sandbox::find_running_pz_pid().is_some();
        return json_result(crate::save_backup::maps::add_maps(z, &paths.pz_install_dir, &cells, &id, running).map(|f| json!(f)));
    }
    let mut saves = crate::save_backup::maps::missing_maps(z, &paths.pz_install_dir, &cells);
    if let Some(id) = &save_id {
        saves.retain(|k, _| k == id);
    }
    json_result(Ok(json!({
        "saves": saves,
        "note": "Each entry: `listed` (what the save loads, map_ver.bin), `missing` (folder, mod, cells, empty_chunks already generated bare, over_listed cells a loaded map covers too). Fix with fix: true and save_id, game closed."
    })))
}

fn handle_community_index_lookup(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let index = crate::rules::index::current(&paths.user_zomboid_dir);
    let mod_id = arg_str(&args, "mod_id");
    if mod_id.is_empty() {
        return json_result::<Option<crate::rules::index::CommunityIndex>>(Ok(index));
    }
    json_result(Ok(json!({
        "summary": crate::rules::community::summary(&paths.user_zomboid_dir),
        "registry": index.as_ref().and_then(|i| i.get(mod_id).cloned()),
    })))
}

fn handle_refresh_community(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    json_result(
        crate::rules::community::refresh(&paths.user_zomboid_dir)
            .and_then(|rules| crate::rules::index::refresh(&paths.user_zomboid_dir).map(|index| rules.summary().with_index(Some(&index)))),
    )
}

fn handle_set_community_downloads(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let enabled = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
    json_result(crate::rules::community::set_downloads_enabled(&paths.user_zomboid_dir, enabled).map(|_| json!({ "downloads": enabled })))
}

fn handle_list_load_rules(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    json_result::<Vec<crate::rules::UserRule>>(Ok(crate::rules::load_rules(&paths.user_zomboid_dir)))
}

fn handle_add_load_rule(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let note = args.get("note").and_then(|v| v.as_str()).map(str::to_string).filter(|n| !n.trim().is_empty());
    json_result(crate::rules::add_rule(&paths.user_zomboid_dir, arg_str(&args, "mod_id"), arg_str(&args, "after"), note))
}

fn handle_remove_load_rule(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    json_result(crate::rules::remove_rule(&paths.user_zomboid_dir, arg_str(&args, "mod_id"), arg_str(&args, "after")))
}

fn handle_list_merged_packages(args: Value) -> ToolCallResult {
    let paths = resolve_paths(&args);
    let list = list_merged_packages(&paths.user_zomboid_dir, &paths.mod_list_ini_path);
    ToolCallResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string_pretty(&json!({
                "total_packages": list.len(),
                "packages": list
            })).unwrap_or_else(|e| format!("Error: {}", e)),
        }],
        is_error: false,
    }
}

