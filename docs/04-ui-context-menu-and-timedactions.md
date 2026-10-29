# 04 — UI Hierarchy, Context Menus & Timed Actions

Interactive gameplay systems in Project Zomboid's client layer are split into two major components: the **visual UI widget tree** (`ISUIElement`) and the **timed action state machine** (`ISBaseTimedAction`).

---

## 1. The Widget Class Tree (`ISUIElement` and `ISPanel`)

Every UI window, button, progress bar, and menu in Project Zomboid derives from `ISUIElement`:

```
ISUIElement (Base Coordinates, Mouse Clicks, Drag & Drop)
 └── ISPanel (Background Panels, Borders, Alpha Rendering)
      ├── ISScrollingListBox (Scrollable Option Lists)
      ├── ISContextMenu (Right-Click Context Menus)
      └── ISInventoryPane (Container and Backpack UI View)
```

### UI Component Lifecycle:
1. **`:new(x, y, width, height)`**: Sets initial screen bounds and core properties.
2. **`:initialise()`**: Initializes internal state variables.
3. **`:createChildren()`**: Instantiates and attaches sub-widgets via `self:addChild(widget)`.
4. **`:prerender()`**: Renders backgrounds and lower graphical layers (executes every frame).
5. **`:render()`**: Renders text, foreground icons, and borders (executes every frame).

---

## 2. World Context Menu Injections (`OnFillWorldObjectContextMenu`)

To add custom actions to the right-click world menu, mods subscribe to `OnFillWorldObjectContextMenu`:

```lua
local function OnFillCustomContextMenu(playerNum, context, worldobjects, test)
    -- ⚠️ THE 'test' PARAMETER: Controller / Gamepad Support
    -- If test == true, the engine is only checking if actions are available for quick-menus.
    -- You MUST return true immediately if valid, WITHOUT constructing widgets.
    if test then return true end

    local player = getSpecificPlayer(playerNum)
    local targetTree = nil

    for _, obj in ipairs(worldobjects) do
        if instanceof(obj, "IsoTree") then
            targetTree = obj
            break
        end
    end

    if targetTree then
        -- Add menu option
        local option = context:addOption("Harvest Pine Resin", targetTree, OnHarvestResin, player)
        
        -- Attach tooltip if prerequisites or descriptions are needed
        local tooltip = ISWorldObjectContextMenu.addToolTip()
        tooltip:setName("Harvest Pine Resin")
        tooltip.description = "Collect natural resin useful for advanced crafting."
        option.toolTip = tooltip
    end
end

Events.OnFillWorldObjectContextMenu.Add(OnFillCustomContextMenu)
```

---

## 3. The `ISBaseTimedAction` State Machine

Player-world interactions do not resolve instantaneously; they queue and execute through `ISTimedActionQueue`.

```
                    ┌───────────────┐
                    │     new()     │
                    └───────┬───────┘
                            │
                    ┌───────▼───────┐
                    │   isValid()   │ ◄─── Evaluated on every tick
                    └───────┬───────┘
                            │ (true)
                    ┌───────▼───────┐
                    │ waitToStart() │
                    └───────┬───────┘
                            │ (false -> ready)
                    ┌───────▼───────┐
                    │    start()    │ ───> Configures animations/sounds
                    └───────┬───────┘
                            │
                    ┌───────▼───────┐
       ┌─────────── │   update()    │ ◄─── Progress tick loop
       │            └───────┬───────┘
 (Cancelled)                │ (Complete)
       │            ┌───────▼───────┐
       ├──────────> │   perform()   │ ───> MANDATORY: ISBaseTimedAction.perform(self)
       │            └───────────────┘
┌──────▼──────┐
│   stop()    │ ───> MANDATORY: ISBaseTimedAction.stop(self)
└─────────────┘
```

### Key Methods & Responsibilities:

| Method | Purpose | Critical Rule |
| :--- | :--- | :--- |
| `new(character, target, time)` | Action constructor | Define `self.stopOnWalk`, `self.stopOnRun`, `self.maxTime`. |
| `isValid()` | Verifies if the action remains physically valid. | Return `false` if the object was destroyed or the character moved out of range. |
| `waitToStart()` | Holds the action until character orientation finishes. | Return `self.character:shouldBeTurning()`. |
| `start()` | Triggers visual animations and audio cues. | Call `self:setActionAnim("Loot")` or equivalent. |
| `update()` | Runs on every frame during action execution. | Update custom progress bars or partial tool usage. |
| `stop()` | Triggered if the action is cancelled by the player. | **MANDATORY:** Call `ISBaseTimedAction.stop(self)` to clean the queue. |
| `perform()` | Triggered when progress reaches 100%. | **MANDATORY:** Call `ISBaseTimedAction.perform(self)` to advance to the next action. |

### ⚠️ The Frozen Queue Bug
Omitting `ISBaseTimedAction.perform(self)` at the end of `:perform()` prevents `ISTimedActionQueue` from receiving the completion signal:
- The character remains permanently locked in the current animation state.
- All subsequent actions in the queue remain blocked indefinitely.
