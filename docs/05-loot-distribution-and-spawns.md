# 05 — Loot Distribution & Procedural Spawning

Item spawning in containers, shelves, and vehicle compartments is managed by a hybrid subsystem: **probability distribution tables in Lua** (`SuburbsDistributions.lua` and `ProceduralDistributions.lua`) and the **Java resolution engine** (`ItemPickerJava`).

---

## 1. Loot Generation Pipeline

When a player opens a room or container (`ItemContainer`) for the first time, the Java engine determines whether procedural loot should spawn:

```
Player opens container for the 1st time
               │
               ▼
┌──────────────────────────────┐
│  ItemPickerJava.fill(cont)   │
└──────────────┬───────────────┘
               │ Queries room type and container name
               ▼
┌─────────────────────────────────────────────────────────────┐
│                   SuburbsDistributions.lua                  │
│  Room: "bedroom" -> Container: "wardrobe"                   │
│  Points to -> ProceduralDistributions: "WardrobeManClassy"  │
└──────────────┬───────────────┘
               │
               ▼
┌─────────────────────────────────────────────────────────────┐
│                 ProceduralDistributions.lua                 │
│  - rolls: 4 (sampling attempts)                             │
│  - items: { "Base.Shirt_FormalWhite", 20.0,                 │
│             "Base.Trousers_Suit", 15.0,                     │
│             "Base.Watch_Gold", 0.5 }                        │
└──────────────┬───────────────┘
               │
               ▼
┌─────────────────────────────────────────────────────────────┐
│          Java Instantiation into ItemContainer              │
│  - Applies Sandbox multipliers (Rare, Normal, Abundant)     │
│  - Inserts spawned items into container inventory           │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Table Structure in `ProceduralDistributions.lua`

Entries in `ProceduralDistributions.list` define weighted probability distributions:

```lua
ProceduralDistributions.list["GunStoreDisplayCase"] = {
    rolls = 3,
    items = {
        "Base.Pistol", 10.0,
        "Base.Pistol2", 8.0,
        "Base.Shotgun", 4.0,
        "Base.9mmClip", 15.0,
        "Base.Bullets9mmBox", 20.0,
    },
    junk = {
        rolls = 1,
        items = {
            "Base.HolsterSimple", 10.0,
            "Base.CleaningKit", 5.0,
        }
    }
}
```

### Safe Item Injection from Mods:
To prevent overwriting tables defined by vanilla or other mods, never reassign the entire table. Append items directly using `table.insert`:

```lua
local function InjectCustomLoot()
    local targetDist = ProceduralDistributions.list["GunStoreDisplayCase"]
    if targetDist and targetDist.items then
        table.insert(targetDist.items, "MyMod.CustomRevolver")
        table.insert(targetDist.items, 6.0) -- Weight / Probability
    end
end
Events.OnGameBoot.Add(InjectCustomLoot)
```

---

## 3. Critical Container API Changes in Build 42

Build 42 refactored internal container hierarchies within `ItemContainer` and `ItemPickerJava`:

### 1. Collection Type Signature Changes:
- In Build 41, many Java getter methods returned `java.util.ArrayList`.
- In Build 42 (revisions 42.13+), several query methods return `java.util.Set` or immutable collections to improve lookup efficiency.
- **Impact:** Legacy Build 41 mods calling list-specific methods directly trigger `java.lang.NoSuchMethodError` or `java.lang.ClassCastException`.

### 2. Component-Aware Containers:
- Containers in Build 42 distinguish between solid inventory items, fluid storage components (`FluidContainer`), and modular vehicle parts.
- Scripts iterating over container items must check item validity before assuming standard `InventoryItem` behavior:
  ```lua
  if container:isItemAllowed(item) then
      container:AddItem(item)
  end
  ```
