# 02 — Lua Script Lifecycle & Event Bus

Project Zomboid does not feature a dynamic dependency resolution graph (such as modern package managers). Instead, the engine evaluates scripts through **strict lifecycle phases** and **deterministic alphabetical ordering**, orchestrating subsystems through a centralized event bus (`Events`).

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

## 2. Alphabetical Load Order & Vanilla Precedence

Within each loading phase (`shared`, `client`, `server`), the engine indexes and executes scripts in **strict alphabetical order by filename**:

1. **Absolute Precedence of Vanilla Code:**
   The game's core vanilla scripts are always parsed and evaluated **before** any third-party mod files.
2. **Mod Evaluation Order:**
   Third-party mod scripts are evaluated alphabetically by `.lua` filename.
3. **The `Z_` Prefix Convention:**
   If Mod B needs to extend or patch a table defined in Mod A (e.g. `ModA_Core.lua`), Mod B's script must start with a later letter in the alphabet (e.g. `Z_ModB_Patch.lua`) to guarantee Mod A's global tables exist prior to Mod B's execution.

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
