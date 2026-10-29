# 07 — FMOD Audio, JSON Translations & 3D Space (B42)

This chapter covers the technical evolution in Build 42 across three core subsystems: the **FMOD audio engine**, the mandatory transition to the **JSON translation standard (B42.15+)**, and the expanded **3D vertical coordinate space**.

---

## 1. FMOD Audio Engine & Sound Definitions

Project Zomboid uses **FMOD** for high-fidelity spatial audio. Mods register audio events declaratively inside `media/scripts/` (e.g. `media/scripts/MySounds.txt`):

```text
module MyMod {
    sound CustomGunshot {
        category = Weapon,
        clip {
            file = media/sound/CustomGunshot.ogg,
            volume = 0.8,
            distanceMax = 80,
        }
    }
}
```

### Playback from Lua:
- **Positional World Audio (With Spatial Occlusion & Falloff):**
  ```lua
  getSoundManager():PlayWorldSound("CustomGunshot", square, 0.2, 80, 1.0, true)
  ```
- **Attached Character Audio:**
  ```lua
  character:playSound("CustomGunshot")
  ```

---

## 2. Localization & JSON Translations (Build 42.15+)

Starting with **Build 42.15**, The Indie Stone phased out legacy `.txt` translation files in favor of **UTF-8 encoded JSON files**:

```
media/lua/shared/Translate/
├── EN/
│   ├── ItemName_EN.json
│   ├── ContextMenu_EN.json
│   └── IG_UI_EN.json
└── ES/
    ├── ItemName_ES.json
    ├── ContextMenu_ES.json
    └── IG_UI_ES.json
```

### JSON Translation Structure (`ContextMenu_EN.json`):
```json
{
  "ContextMenu_CustomOption": "Craft Reinforced Plank",
  "ContextMenu_CustomOption_Tooltip": "Requires Carpentry level 4 and steel nails."
}
```

### Retrieving Translations in Lua:
```lua
local localizedText = getText("ContextMenu_CustomOption")
```

### 🛠️ Debugging Translations with `-debugtranslation`:
Launching the game with the `-debugtranslation` launch parameter instructs the engine to write a `translationProblems.txt` report inside `C:/Users/<User>/Zomboid/`, listing missing keys and syntax formatting errors.

### ⚠️ Escaping Percent Characters (`UnknownFormatConversionException`):
If a localized translation string contains a literal percent sign `%` (e.g. `"Critical Hit +15%"`), Java string formatters throw `UnknownFormatConversionException`. In strings that are not template formatters (`%s`, `%d`), the percent symbol must be escaped as `%%` (`"Critical Hit +15%%"`).

---

## 3. Expanded 3D Space & Vertical Z-Levels in Build 42

Build 42 redesigned vertical spatial handling across the rendering and collision pipelines:

| Spatial Dimension | Build 41 (Legacy) | Build 42 (Unstable/Modern) |
| :--- | :--- | :--- |
| **Z-Coordinate Range** | `0` to `7` levels | **`-32` to `+32`** levels |
| **Basements & Bunkers** | Simulated (teleporting to $Z=0$ map islands) | **True Physical Basements** ($Z < 0$ coordinates) |
| **Skyscrapers / Tall Structures** | Maximum 7 stories | **Up to 32 stories** in urban cores |
| **Lighting Pipeline** | Static 2D lightmaps | Dynamic vector shadows and volumetric occlusion |
| **Roofs & Slopes** | Fixed 45-degree isometric planes | Parametric 30-degree multi-pitch roofs |

### Safe Vertical Tile Queries (`IsoGridSquare`):
Mods querying adjacent squares along the Z-axis must account for negative coordinates:

```lua
local currentZ = player:getZ()
-- Check if player is underground (basement level)
if currentZ < 0 then
    -- Apply subterranean light and temperature modifiers
end
```
