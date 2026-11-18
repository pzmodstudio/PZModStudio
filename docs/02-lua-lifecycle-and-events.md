# 02 — Lua Script Lifecycle & Event Bus

The engine runs Lua in **lifecycle phases** (`shared`, `client`, `server`), and inside each phase in a fixed order: the game's own files first, then each mod in the order the game loads the mods (what a mod requires comes before it), the files of one mod sorted by path. Subsystems talk through a centralized event bus (`Events`).

---

## 1. The Three Engine Loading Phases

A script's physical location within the `media/lua/` directory dictates precisely when and where its code is executed:

```
media/lua/
├── shared/   ──> Phase 1: Loaded on game boot. Shared by Client and Server.
├── client/   ──> Phase 2: Loaded after shared. UI, rendering, and timed actions.
└── server/   ──> Phase 3: Loaded ONLY when starting or joining a live game/server.
```

### Domain Breakdown:

1. **`media/lua/shared/` (Shared Domain):**
   - Compiled and injected into the Kahlua VM immediately when Project Zomboid starts.
   - Houses foundational logic: class definitions, global tables, mathematical utilities, zone definitions (`VehicleZoneDefinition.lua`), Sandbox presets, and base recipe registrations.
2. **`media/lua/client/` (Client Domain):**
   - Evaluated sequentially immediately after the shared phase.
   - Governs all visual and input systems: UI panels (`ISUIElement`), context menus (`ISContextMenu`), map cursor, status bars, and timed actions (`ISBaseTimedAction`).
   - Exists exclusively in local player memory.
3. **`media/lua/server/` (Server Domain):**
   - Loaded **asynchronously on demand**: not compiled when opening the game client, but rather when clicking "Start Game" (Singleplayer) or launching the dedicated server (Multiplayer).
   - Holds authoritative simulation authority: farming engines, weather simulation, loot spawning/respawning, and processing incoming player network commands (`sendClientCommand`).

---

## 2. Load Order: Requirements First, Then the List

Checked against the game's own loader (`ZomboidFileSystem.loadMods`, `LuaManager.LoadDirBase`, 42.22).

### Which mods load, and in what order

The game walks the active list (`default.txt`, or a save's own `mods.txt`) from top to bottom. For each mod it first loads everything that mod names in `require=`, recursively, and only then the mod itself:

1. **A requirement comes first, wherever it sits.** A required mod further down the list, or not in the list at all (installed but switched off), is loaded right before the first mod that needs it. A requirement placed after its mod is therefore not a problem.
2. **Everything else keeps the list's order.** `loadModAfter=` and `loadModBefore=` are **not** applied when the game loads a list: they are hints for whoever sorts the list (the game's mod screen, or a tool). If a mod must run after another it does not require, the list itself has to say so.
3. **A mod that cannot load is left out**, with every mod that requires it: outside its `versionMin`/`versionMax`, no folder for the installed build, or a requirement missing or unloadable at any depth.
4. **Required ids are looked up exactly as written.** `require=A, B` asks for a mod called `" B"` (with the space): none exists, so the mod is left out, though the game's mod screen (which trims the names) shows it on. Write the ids separated by commas only.

### Which Lua files run, and when

For each phase (`shared` and `client` at the main menu, `server` when a game starts or a server boots):

1. **The game's own files** of that phase, sorted by their path (case-insensitive).
2. **Then each mod, in the load order above.** A mod's files from `common/` and from its version folder (`42/`, `42.13/`...) are sorted together by path, and run one mod after another. Sorting is per mod, not across all mods: `ZZZ_Patch.lua` in a mod that loads early still runs before `AAA.lua` of a mod that loads later.
3. **One path runs once.** When several sources ship the same relative path (`media/lua/client/ISUI/ISInventoryPane.lua`), the copy of the **last** mod in the load order is the one kept, and it runs **at the place of the first** source that has the path: a mod's copy of a game file runs in the game's turn, before every mod.

### What this means for a mod that patches another

- To be sure your file runs after mod A's, **require mod A** (`require=A` in `mod.info`). That is the only order the game enforces; a file name starting with `Z_` does nothing across mods.
- For code that needs every mod's files to have run, use an event: `OnGameBoot` fires after all `shared` and `client` files of all mods, `OnGameStart` after the `server` ones too.
- A file of yours at the same path as a game file replaces the game's file and runs where the game's file ran, before any mod's code.

---

## 3. The Central Event Bus (`Events`)

The Java core of Project Zomboid interacts with the Lua layer by dispatching delegates to registered event handlers.

### Universal Subscription Syntax:
```lua
-- Subscribe function to event
Events.OnGameStart.Add(MyMod_OnGameStart)

-- Unsubscribe function when no longer needed (saves CPU cycles)
Events.OnTick.Remove(MyMod_OnTick)
```

### ⚠️ Critical Rule: Never Use Anonymous Functions in Core Events!
Anonymous functions **cannot be unsubscribed** via `.Remove()` by other scripts or cleanup routines:
```lua
-- ❌ ANTI-PATTERN: Memory leak and unremovable callback
Events.OnPlayerUpdate.Add(function(player)
    -- Logic
end)

-- ✅ CORRECT APPROACH: Named function pointer
local function MyMod_Update(player)
    -- Logic
end
Events.OnPlayerUpdate.Add(MyMod_Update)
```

### Core Engine Event Catalog

| Event | Trigger Point | Parameters | Typical Usage |
| :--- | :--- | :--- | :--- |
| `OnGameBoot` | After all Lua files from all mods have been compiled. | None | Safe cross-mod monkey patching. |
| `OnGameStart` | When the player spawns into the world and takes control. | None | Initialize player data or initial UI widgets. |
| `OnFillWorldObjectContextMenu` | When right-clicking a world tile or entity. | `player, context, worldobjects, test` | Inject custom right-click context menu options. |
| `OnPlayerUpdate` | Every tick of the player simulation loop (~60 times/sec). | `player` | Player state monitoring (use early returns!). |
| `OnClientCommand` | On the server, when receiving a payload via `sendClientCommand`. | `module, command, player, args` | Server-authoritative validation and state changes. |
| `OnServerCommand` | On the client, when receiving a broadcast via `sendServerCommand`. | `module, command, args` | Update local UI/state following server confirmation. |

---

## 4. Safe Monkey Patching (*Function Wrapping*)

When multiple mods need to modify the same native engine function without overwriting each other, **Function Wrapping** is mandatory:

### Step-by-Step Wrapping Pattern:

```lua
-- 1. Capture the original function pointer before redefining
local original_ISInventoryPane_render = ISInventoryPane.render

-- 2. Redefine the method, preserving 'self' and all underlying arguments
function ISInventoryPane:render(...)
    -- A. Execute custom pre-logic (optional)
    if self.myCustomFlag then
        -- Custom behavior
    end

    -- B. MANDATORY: Invoke original function with 'self' and forwarded arguments
    local result = original_ISInventoryPane_render(self, ...)

    -- C. Execute custom post-logic (optional)
    return result
end
```

### Why Breaking the Call Chain is Fatal:
If a mod replaces a function directly without delegating to `original_function(self, ...)`, it silently destroys all modifications made by previously loaded mods and breaks the engine's internal assumptions, causing severe desyncs and runtime crashes in multiplayer.
