# 09 — Game Process Control & Live IPC Bridge

This chapter explains how an AI Agent or external tooling (such as **PZ Mod Studio**) can manage the lifecycle of `ProjectZomboid64.exe` and communicate bidireccionaly with an active game session via an **Inter-Process Communication (IPC) Bridge**.

---

## 1. Project Zomboid Process Control

### Key Command-Line Flags:
| CLI Flag | Purpose |
| :--- | :--- |
| `-debug` | Enables the in-game debug menu, cheat options, Lua reload shortcuts, and on-screen exception viewers. |
| `-windowed` | Forces the game into windowed mode (ideal for automated developer workflows). |
| `-nosteam` | Runs the game without active Steam client authentication (useful for isolated testing). |
| `-debugtranslation` | Generates `translationProblems.txt` for translation key verification. |

### Process Lifecycle in Rust:
1. **Detection:** Scans the operating system process tree (`tasklist` on Windows) for `ProjectZomboid64.exe` and captures its Process ID (PID).
2. **Launch:** Spawns a child process with `std::process::Command::new("ProjectZomboid64.exe").arg("-debug").spawn()`.
3. **Termination:** Issues graceful termination signals or force kills with `taskkill /PID <pid> /F`.

---

## 2. Companion Mod Architecture: `Z_PZModStudio_Bridge`

Because the vanilla engine does not expose a local REPL socket in singleplayer mode, we implement a **Synchronized File-Based IPC Bridge**:

```
┌─────────────────────────┐                        ┌─────────────────────────┐
│     AI AGENT / MCP      │                        │  PROJECT ZOMBOID (GAME) │
└────────────┬────────────┘                        └────────────┬────────────┘
             │                                                  │
             │ 1. Writes pz_ipc_queue.json                      │
             ├─────────────────────────────────┐                │
             │                                 │                │
             │                                 ▼                │
             │                  ┌──────────────────────────────┐│
             │                  │  Zomboid/pz_ipc_queue.json   ││
             │                  └──────────────┬───────────────┘│
             │                                 │                │
             │                                 │ 2. Events.OnTick (Every 30 frames)
             │                                 │    Polls & executes commands
             │                                 ▼                │
             │                  ┌──────────────────────────────┐│
             │                  │    PZModStudio_Bridge.lua    │├┘
             │                  │  - getPlayer()               │
             │                  │  - AddItem / Equip / Teleport│
             │                  │  - loadstring(lua_code)()    │
             │                  └──────────────┬───────────────┘
             │                                 │
             │                                 │ 3. Writes execution result
             │                                 ▼
             │                  ┌──────────────────────────────┐
             │                  │   Zomboid/pz_ipc_resp.json   │
             │                  └──────────────┬───────────────┘
             │                                 │
             │ 4. Reads response & confirms    │
             │◄────────────────────────────────┘
             ▼
```

---

## 3. JSON IPC Protocol Specification

The command queue file at `C:/Users/<User>/Zomboid/pz_ipc_queue.json` receives command objects:

### 1. Give / Equip Item (`give_item`)
```json
{
  "id": "cmd_001",
  "action": "give_item",
  "item": "Base.Axe",
  "count": 1,
  "equip": "primary"
}
```
*Effect:* Instantiates the item and equips it into the primary hand of the player character.

### 2. Teleport Character (`teleport`)
```json
{
  "id": "cmd_002",
  "action": "teleport",
  "x": 10520,
  "y": 9410,
  "z": 0
}
```
*Effect:* Instantly teleports the local player to target grid coordinates.

### 3. Modify Player Stats (`set_stat`)
```json
{
  "id": "cmd_003",
  "action": "set_stat",
  "godmode": true,
  "health": 1.0,
  "hunger": 0.0,
  "thirst": 0.0
}
```
*Effect:* Sets godmode and resets health, hunger, and thirst stats.

### 4. Dynamic Lua Evaluation (`eval_lua`)
```json
{
  "id": "cmd_004",
  "action": "eval_lua",
  "code": "local p = getPlayer(); p:Say('AI Command Executed!'); HaloTextHelper.addText(p, 'AI Link Active', 0, 255, 0);"
}
```
*Effect:* Executes arbitrary Lua code in the live game environment.

---

## 4. Companion Mod Script Implementation

The `Z_PZModStudio_Bridge` mod resides inside the user's mod directory (`Zomboid/mods/Z_PZModStudio_Bridge/`):

```lua
-- media/lua/client/PZModStudio_Bridge.lua
local Bridge = {}
Bridge.tickCounter = 0
Bridge.IPC_PATH = "pz_ipc_queue.json"
Bridge.RESP_PATH = "pz_ipc_resp.json"

function Bridge.OnTick()
    Bridge.tickCounter = Bridge.tickCounter + 1
    -- Poll the queue every 30 ticks (~0.5s)
    if Bridge.tickCounter % 30 ~= 0 then return end

    local player = getPlayer()
    if not player then return end

    local fileReader = getFileReader(Bridge.IPC_PATH, false)
    if not fileReader then return end

    local content = ""
    local line = fileReader:readLine()
    while line do
        content = content .. line
        line = fileReader:readLine()
    end
    fileReader:close()

    if content ~= "" and content ~= "{}" then
        -- Process JSON payload
        Bridge.ProcessCommand(content, player)
        
        -- Flush queue
        local fileWriter = getFileWriter(Bridge.IPC_PATH, true, false)
        if fileWriter then
            fileWriter:write("{}")
            fileWriter:close()
        end
    end
end

function Bridge.ProcessCommand(jsonStr, player)
    -- Parse command object and trigger actions (AddItem, Teleport, eval_lua)
end

Events.OnTick.Add(Bridge.OnTick)
```

---

## 5. Claude Agent Protocol (`Z_ClaudeAgent`)

`Z_ClaudeAgent` (source: `mods/Z_ClaudeAgent/`, installed by `install_claude_agent_mod` or `install_bridge_companion_mod` with `include_claude_agent: true`) is a second companion mod for agents that play the game. The bridge above only polls from `OnTick` with a player present; the Claude Agent polls from `OnFETick` (main menu), `OnTickEvenPaused` (in game, also paused) and `OnPreUIDraw`, so it also drives the new-game, sandbox-options, character-creation and spawn-select screens, and it returns values.

Files in `<Zomboid>/Lua/`:

| File | Writer | Content |
| :--- | :--- | :--- |
| `claude_cmd.lua` | agent | Line 1 `--id:<n>`, then Lua; the chunk may `return` a value. Runs once per new id. |
| `claude_out.txt` | mod | `id:<n>\n<ok\|err>\n<result>`; result is `CA.ser` output (JSON when the value serializes cleanly) or the error message. |
| `claude_log.txt` | mod | `CA.log(...)` lines, cleared at boot. |

The MCP server writes the command atomically (temp file + rename) with a fresh id and polls `claude_out.txt` until the same id appears or `timeout_ms` passes. Sending the whole `ClaudeAgent.lua` as a command hot-reloads it (event handlers are registered only once, guarded by `CA.registered`).

**Helpers on the global `CA`:** `state()`, `ui(filter)`, `find(text, type)`, `click(text, type)`, `answer("YES"|"NO"|"OK")`, `stats()`, `zombies(r)`, `speed(n)` (presses the speed-widget buttons: "Play", "Fast Forward x 1" = x5, "Fast Forward x 2" = x20, "Wait" = x40, since `SetCurrentGameSpeed` alone does not move the clock; fast-forward is refused near zombies), `walkTo(x,y,z)`, `travel(tx, ty, alert, hop)`, `findItem/take/invItem/equip/eat/inventory`, `findFood(r)`, `drink(r)`, `sleep(r)`, `craft(recipe, item)`, `wounds()`, `bandageAll()`, `busy()` (`{actions, current, moving}`), `flee(dist)`, `fight(radius, minHealth, crowd)`, `buildings(r)`, `enter(b)`, `door(x, y, want)` / `doorAt(x, y, z)`, `eachObject(r, fn, z)`, `zombies(r)` (with inside / sameBuilding flags), `methods(obj, filter)`, `fields(obj, filter)`, `try(fn)`, `task(name, ms, fn)` / `untask(name)`, `log(...)`. Distances use euclidean `getDistanceSq` (`DistTo` is ~1.41x off on diagonals). Under `-debug` the mod calls `UIManager.setShowLuaDebuggerOnError(false)` so errors do not freeze the game; cheats (`setGhostMode`, `setGodMod`, `setInvisible`) only work under `-debug`. `IsoStove` objects are replaced when toggled or burnt, so look them up again instead of keeping references. A hot reload keeps the running `CA.tasks`.

**MCP tools:** `agent_eval_lua` (any code; `wait_idle_ms` waits for `CA.busy().actions == 0` first), `agent_ui_list` (`CA.ui`), `agent_ui_click` (`CA.click`), `agent_dialog_answer` (`CA.answer`), `agent_game_state` (`CA.state`).

**Window tools (Win32, never take focus unless asked):**
- `game_window` `park` restores a minimized window (minimized GL windows stop presenting), moves it to 0,0 at 1280x720 (or `width`/`height`) and puts it at the bottom of the z-order with `SWP_NOACTIVATE`. `minimize` uses `SW_SHOWMINNOACTIVE`; `show` restores and brings it forward (thread-input attach + ALT tap, so `SetForegroundWindow` is allowed); `info` reports rect, client size, foreground and minimized state.
- `capture_game_screenshot` uses `PrintWindow(PW_CLIENTONLY | PW_RENDERFULLCONTENT)`, which captures the game while other windows cover it, and saves a PNG (default `<temp>/pz-mod-studio/game_screenshot.png`, downscaled to `max_width`). `looks_blank: true` means the capture came back a single color.
- **Click to start:** after Continue/Play the game sits on a black "click to start" screen (GameLoadingState) that suppresses Lua callbacks, so the mod is silent there. `agent_enter_game` polls `CA.state()` (2 s timeout); once the mod has been silent >20 s it posts a centre click every ~3 s via `game_click_window` (WM_MOUSEMOVE + WM_LBUTTONDOWN/UP, no focus) until `"ingame":true`. Clicks are only sent while the mod is silent, so nothing in game gets clicked. `agent_ui_click` retries while `CA.click` answers "not found" (`retry_ms`, default 20 s), because menus build over several frames.
- `game_send_key` posts `WM_KEYDOWN` / `WM_KEYUP` to the game window (lParam = repeat 1 + `MapVirtualKeyW` scan code, extended bit for arrows/navigation keys); modifiers go down first and up last, so `SHIFT+O` works while the window is parked and unfocused.
- `launch_game` with `agent_mode: true` starts without `-debug` (under `-debug` the Lua debugger halts on any error, even inside `pcall`), adds `-windowed`, and runs a background guard that waits for the window, keeps it parked and hands focus back to the previously focused window for about 45 s while the game boots (`park` / `no_focus_steal` toggle each part).


## 6. Studio Link (`Z_StudioLink`)

Makes the game's Mods screen show the profile that is active in the Studio, and brings what the player changes there back to that profile. Source: `mods/Z_StudioLink/` (B42 only), compiled into the app and installed into `Zomboid/mods` whenever a profile is activated or changed (`studio_link::install_link_mod`). Every profile has it on and counts it as one of its mods; the Mod List does not let it be switched off while a profile is active.

**Profile members.** A profile holds `members` (its mods, on or off) besides `active_mod_ids`; anything else installed is outside it. Profiles from before have no `members` and are read as "the mods it has on". A mod switched on becomes a member; "Take out of the profile" in a Mod List row removes it and switches it off (`set_profile_members`).

| File | Writer | Content |
| :--- | :--- | :--- |
| `Zomboid/Lua/PZModStudio_Link.txt` | Studio | `profile=<name>` then `member=<mod id>` per member, in load order, then `other=<mod id>` per mod another profile holds. Missing = no profile active: the mod leaves the game's screens alone. |
| `Zomboid/Lua/PZModStudio_LinkHealth.txt` | Studio | `<severity>\t<mod id>\t<text>` (tab-separated) per Mod Health error or warning, in the Studio's language (`write_link_health_cmd`, written whenever the report changes). |
| `Zomboid/Lua/PZModStudio_LinkApp.txt` | Studio | `protocol=`, `version=`, `alive=<unix seconds>`, rewritten every 4 s by the watcher. Older than 15 s = the app is closed: the profile label says so and requests wait for it. |
| `Zomboid/Lua/PZModStudio_LinkRequests.txt` | game | `<id>\t<verb>\t<argument>` per request (`<unix seconds>-<n>`, verb `add` + mod id; the first protocol's `add=<mod id>` lines still work). The watcher reads and deletes it, does it (`add`: `instance_manager::add_members_to_active`, switched off, then `profile-changed-in-game`) and answers. A request written while the app is closed is done when it opens. |
| `Zomboid/Lua/PZModStudio_LinkAcks.txt` | Studio | `<id>\tok|error\t<text>` per request answered, the last 50. The game shows "Adding..." until its id is there, the error on the button when it failed, and "did not answer" after 30 s. An unknown verb is answered with an error, so a newer Link never waits on an older app. |
| `Zomboid/Lua/PZModStudio_LinkGame.txt` | game | `protocol`, `link` (its version), `state=menu|ingame`, `save=<mode>/<world>`, `mod=` per active mod, `at=`; written on `OnMainMenuEnter` and `OnGameStart`. The watcher emits `link-game-state` and keeps each save's last mods in `Zomboid/PZModStudio_SaveRefs.json` (`studio_link::record_save_reference`; `link_status_cmd` returns both), the base for comparing a save with what it last ran before loading it. |
| `Zomboid/mods/default.txt` | both | The game rewrites it when its Mods screen is accepted. `studio_link::watch_mod_list` polls it every 2 s; a change that is not the Studio's own last write updates the active profile (active mods, order merged with `merge_game_order`, new members) and emits `profile-changed-in-game`. |
| `Zomboid/PZModStudio_JavaAgents.json` | Studio | The Java-agent VM args the Studio took out of `ProjectZomboid64.json`, so it only ever puts back its own. |

**In the game** (`PZModStudio_Link.lua`): wraps `ModSelector.Model:filterMods` to keep only members (and anything switched on), hides itself, adds a "Show all subscribed mods" tick and the profile name next to the game's "enabled only" tick, and wraps `MainScreen.checkSaveFile`: after the game's own checks pass, a save whose `mods.txt` uses a mod outside the profile is refused with the game's own error dialog (`displayCheckSavefileModal(text, true)`), naming the mods. A save without `mods.txt` uses the default list and is never refused.

**Other mod managers.** With Definitive Mod Manager, PZMM's client, Mod Manager or Mod Load Order Sorter on, the Link leaves the Mods screen alone (no filtering, no row) and keeps only the save checks and the Load badges: two mods reshaping one list fight. **Texts** are the game's translations, `42/media/lua/shared/Translate/<LANG>/UI.json` (`UI_StudioLink_*`, EN and ES), with the English in the Lua as the fallback. Placeholders are `{1}`, `{2}`: a `%` would be read by the game as a format of its own. A test checks the keys match in all three, and that the Lua uses nothing Kahlua lacks (`next`, `goto`, `//`).

It also: shows a "+N new -> profile" button for installed mods no profile holds (`member`/`other`), which writes the requests file; draws a mark left of the favourite star on each mod in the health file, with its text as the row tooltip (`ModListBox:doDrawItem`, read in `Model:reloadMods`); labels each save of the Load screen (no mods / N outside the profile / profile mods, `LoadGameScreen.drawMap` wrapped in `create`); and asks before loading a save whose `mods.txt` is empty. Yes there copies `default` into `currentGame`, the game's own Yes writes it into the save (`WriteModsDotTxt`), and `checkSaveFile` copies it again when `continueLatestSaveAux` comes back with the save's empty list. No there (and any refused save) puts `default` back into `currentGame` (`Link.stagingFromDefault`): Continue had copied the save's list into the list the next new game is made with, and the one way into New Game that skips the Solo click (the Load screen after deleting its last save) then created a world with that stale list, so a save "with no mods" while 147 ran (seen twice on 2026-10-26). The same copy runs after `LoadGameScreen:onDeleteModalClick` when New Game opens, and `OnInitWorld` fills an empty `currentGame` from `default` as a last net (a loaded save keeps its own list).

**Workshop.** `workshop/Z_StudioLink/` holds the page (`workshop.txt`, `preview.png`) and `assemble.ps1`, which builds `Zomboid/Workshop/Z_StudioLink` for the game's uploader. When a Workshop copy is installed the app removes its own from `Zomboid/mods` and stops installing it, so the game never sees the id twice.

**Save backups** (`save_backup`, the Saves screen). A backup is `Zomboid/PZModStudio_SaveBackups/<mode>/<world>/<stamp>-<reason>.zip` plus a `.json` beside it. It is written as `.zip.part`, opened again and every entry read to the end (the CRC check) and compared with the files and sizes it was made from; only then is it renamed and given its `.json`. A zip without one is never listed and `.part` leftovers are swept. Nothing starts without the save's size plus 256 MB free on the disk. A save unchanged since its newest backup (same newest mtime, size and file count) is not zipped again. The newest 5 automatic backups per save are kept; `manual` ones, and any the player pins, are never pruned. Restore refuses while the game runs, checks the zip, backs the current save up (`before-restore`), unpacks beside it and swaps the two folders, putting the old one back if the swap fails.

When: `exit` when the game file goes from `ingame` to `menu`, or the game process is gone while it said `ingame`; `before-load` and `before-fix` when the Link asks (`<id>\tbackup\t<mode>/<world>\t<reason>`, answered from the backup's own thread once it is done); `manual` from the Saves screen.

| File | Writer | Content |
| :--- | :--- | :--- |
| `Zomboid/Lua/PZModStudio_LinkSaves.txt` | Studio | One line per save: `save\t<mode>/<world>\tbacked=0|1\tbytes=\tsession=\tupdated=\tmods=a;b;...`. `backed`: unchanged since its newest backup. `session`/`mods`: its last session (`PZModStudio_SaveRefs.json`). `updated`: mods of that session whose Workshop version on disk (`timeupdated` in `appworkshop_108600.acf`) was published after it. Rewritten about once a minute, on every game state change, and after each backup **before** its answer, so a Link that reloads Lua finds the save backed up. |
| `Zomboid/Lua/PZModStudio_LinkPass.txt` | game | `<world>\t<unix seconds>`: the player went on (a backup done, or "go on without a backup"). Honoured for 120 s, because the game resets Lua between the two save checks of one load. |

**What changed since the last session.** When a save's list differs from its last session (`PZModStudio_SaveRefs.json`) or some of its mods were updated since (`updatedmods=` in the saves file), the Link first shows a panel of its own (`WaitPanel.choose`) listing mods taken out (map mods, by `getMapFoldersForMod`, marked and warned about), added, updated, and whether the order changed, with Back, Play on a copy and Load it. Play on a copy sends `<id>	copy	<mode>/<world>`; the app copies the save to `<world>_copy` (`save_backup::copy_save`: free space checked, made in a hidden folder first, the session reference copied too) and answers with the copy's folder, which then loads through `MainScreen.continueLatestSave`. `save_session_diffs_cmd` gives the Saves screen the same summary.

In the game, the save check backs the save up before it loads when it is not backed up as it is and either its mods are not those of its last session, some were updated since, or the Studio never saw a session of it. While the app works the Link shows a panel of its own (`StudioLinkWaitPanel`, not the game's dialog, which UI mods such as Neat Rocco's UI restyle with a green tick for OK) with the size, the reason, a seconds counter and Cancel; on `ok` the game's own load (`continueLatestSaveAux`, with the arguments it started with) goes on. On an error, or with the app closed, the player is asked whether to go on without a backup. Any Yes in the game's save dialogs (`onCheckSavefileModalClick`), which writes the save's mod list, is preceded by a `before-fix` backup the same way.

**Zombie Buddy.** Its core is loaded by `-agentlib:zbNative` in the game's `ProjectZomboid64.json` (with `zbNative.dll` in the game folder), not through the mod list, so it ran with every profile; the mod list only decides which of its Java mods load (`[ZB] java mod list to load:` in `console.txt`). `studio_link::sync_java_agents` takes the line out when the active list lacks `ZombieBuddy` and puts it back first in `vmArgs` when it has it, at profile activation and at every launch from the Studio. Each change keeps the previous file in `Zomboid/PZModStudio_Backups`. Steam's "Verify integrity" restores the shipped file, which has no agent line at all.