# 03 — Item Definitions, Crafting & Fluid API in Build 42

In Project Zomboid, item properties, crafting recipes, 3D models, and animations are not coded directly in Lua. Instead, they reside in declarative `.txt` script files located within the `media/scripts/` directory.

With **Build 42**, The Indie Stone shifted the engine toward a **Data-Driven Architecture**, delegating game mechanics away from the Kahlua Lua VM and into optimized native Java/C++ parsing pipelines.

---

## 1. Declarative Syntax in `scripts/*.txt`

Script definitions use structured key-value syntax within module blocks:

```text
module Base {
    item CustomMachete {
        Type = Weapon,
        DisplayName = Custom Machete,
        Icon = MacheteCustom,
        MinDamage = 1.8,
        MaxDamage = 2.8,
        Weight = 2.0,
        Categories = Blade,
        SubCategory = Slash,
        AttachmentType = Knife,
        Tags = SharpKnife;ButcherTool,
    }
}
```

---

## 2. Recipe Architecture: `Recipe` (B41) vs `craftRecipe` (B42)

### The Legacy Model (Build 41):
In Build 41, recipes used the `Recipe` block. Any non-trivial physical property transfer (e.g. copying calories, preserving rotten state, or tracking thermal temperature) required custom `OnCreate` Lua scripts.

### The Modern Standard (Build 42): `craftRecipe`
Build 42 introduces `craftRecipe`, featuring:
- Dedicated workstation requirements (`Workstation = Forge/Anvil/Workbench`).
- Fractional crafting times and partial tool durability consumption.
- Native integration with fluid containers and semantic tag requirements.

```text
module Base {
    craftRecipe CraftCustomSpear {
        category = Survivalist,
        Time = 80,
        Anim = MakingSpear,
        Workstation = Workbench,

        inputs {
            item 1 tags[SharpKnife],
            item 1 WoodenStick,
            item 1 DuctTape { consume = 2 },
        }

        outputs {
            item 1 CustomSpear,
        }
    }
}
```

---

## 3. Semantic Categorization & Tag Indexing (`Tags`)

Instead of hardcoding concrete item IDs into recipes (which required modders to duplicate recipes for item variants), Build 42 uses semantic `Tags`:

- **Item Definition:**
  ```text
  Tags = SharpKnife;CutPlant;OpenCan;ButcherAnimal,
  ```
- **Recipe Requirement:**
  ```text
  inputs {
      item 1 tags[SharpKnife],
  }
  ```
Any new weapon or tool with the `SharpKnife` tag will automatically work across all vanilla and modded recipes requiring a sharp knife without requiring compatibility patches.

---

## 4. Physical Inheritance Directives (`InheritFood` and `InheritCooked`)

Build 42 transfers physical variables automatically between ingredients and products through declarative directives:

| Directive | Physical Effect on Crafted Item |
| :--- | :--- |
| `InheritFood` | Automatically transfers calories, protein, carbohydrates, lipids, rot progression, and poison levels. |
| `InheritCooked` | Preserves cooking state (raw, cooked, burnt) and heat temperature from the original ingredient. |

---

## 5. Fluid Dynamics & the `FluidContainer` Component API

Build 42 introduces an entity-component fluid model for liquids and gases using the `FluidContainer` class:

```
┌────────────────────────────────────────┐
│             InventoryItem              │
│  - DisplayName: "Gasoline Can"         │
│  - Weight: 1.5                         │
│  ┌──────────────────────────────────┐  │
│  │      FluidContainer (Component)  │  │
│  │  - Capacity: 10.0 L              │  │
│  │  - CurrentAmount: 7.5 L          │  │
│  │  - FluidType: "Gasoline"         │  │
│  │  - Properties: Density, Flammab. │  │
│  └──────────────────────────────────┘  │
└────────────────────────────────────────┘
```

### Guidelines for Modders:
- For vehicle parts storing fuel, always set `category = "gastank"` and `contentType = "Gasoline"`.
- Liquid containers dynamically recalculate character encumbrance based on liquid volume and density.

### ⚠️ Preventing Harmonic Feedback Loops
The `Events.OnWaterAmountChange` event fires whenever a container's liquid level changes.

**The Failure Mode:**
1. A Lua handler listens to `OnWaterAmountChange`.
2. It computes visual attributes and invokes `container:UpdateWaterLevel()`.
3. Java updates the container and **re-triggers** `OnWaterAmountChange`.
4. If fluid calculations have floating-point discrepancies, an infinite recursion occurs, spamming `console.txt` and overwhelming multiplayer networking with `ChunkObjectState` packets.

**The Solution (Early Bail-Out Guards):**
```lua
local function Safe_OnWaterAmountChange(object)
    -- Guard: verify object validity and fluid container existence
    if not object or not object:getFluidContainer() then return end
    
    local container = object:getFluidContainer()
    local currentAmount = container:getAmount()
    
    -- Bail out if delta is negligible
    if math.abs((object:getModData().lastAmount or 0) - currentAmount) < 0.001 then
        return
    end
    
    object:getModData().lastAmount = currentAmount
    -- Execute safe logic
end
Events.OnWaterAmountChange.Add(Safe_OnWaterAmountChange)
```
