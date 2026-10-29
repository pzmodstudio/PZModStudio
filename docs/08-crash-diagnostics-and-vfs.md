# 08 — Crash Diagnostics, VFS & 3-Way AST Merging

Diagnosing silent crashes, load order conflicts, and script collisions is one of the most demanding tasks in Project Zomboid development. This chapter outlines the error taxonomy of `console.txt`, the Virtual File System (VFS) collision model, and the 3-Way Abstract Syntax Tree (AST) merge engine.

---

## 1. Error Taxonomy & Crash Signatures in `console.txt`

The active log file at `C:/Users/<User>/Zomboid/console.txt` captures all JVM stdout/stderr streams and Kahlua VM tracebacks.

```
┌─────────────────────────────────────────────────────────────┐
│                    COMMON CRASH SIGNATURES                  │
├──────────────────────────┬──────────────────────────────────┤
│ Signature in console.txt │ Root Cause & Recommended Action  │
├──────────────────────────┼──────────────────────────────────┤
│ KahluaThreadException    │ Lua runtime exception in script  │
│ NoSuchMethodError        │ B41 vs B42 Java API mismatch     │
│ ChooseGameInfo$Mod.getId │ Malformed versionMin format      │
│ attempted index of non-t │ Premature global table access    │
│ UnknownFormatConversion  │ Unescaped % symbol in locale str │
└──────────────────────────┴──────────────────────────────────┘
```

### 1. `KahluaThreadException` / `se.krka.kahlua.vm.KahluaException`
- **Diagnostic:** An unhandled error occurred in a Lua script.
- **Callframe Inspection:**
  ```text
  STACK TRACE
  -----------------------------------------
  function: perform -- file: MyCustomAction.lua line # 42 | MOD: MyMod
  function: update -- file: ISTimedActionQueue.lua line # 78
  ```
- **Resolution:** Inspect the exact line in the failing mod script (`MyCustomAction.lua:42`).

### 2. `java.lang.NoSuchMethodError` / `java.lang.ClassCastException`
- **Diagnostic:** A legacy B41 mod is invoking a Java method whose signature was changed or removed in Build 42 (e.g. `ItemContainer` or `ItemPickerJava` methods).
- **Resolution:** Refactor Lua code to the modern API or isolate the script inside `/41/media/`.

### 3. `NullPointerException` at `ChooseGameInfo$Mod.getId()`
- **Diagnostic:** The `mod.info` manifest specifies `versionMin=42` as an integer without decimals.
- **Resolution:** Change to `versionMin=42.00` or remove the directive and use `versionMax=42.14.1`.

### 4. `attempt to index a nil value` / `attempted index of non-table`
- **Diagnostic:** A script attempted to access a global table before it was initialized (load order race condition).
- **Resolution:** Defer initialization to `Events.OnGameBoot.Add()` or apply the `SAFE_GLOBAL_TABLE_ACCESS` polyfill.

---

## 2. Virtual File System (VFS) Collision Model

When two or more mods provide conflicting copies of the same relative path within `media/` (e.g. both shipping their own `media/lua/client/ISUI/ISInventoryPane.lua`), Project Zomboid performs a **destructive overwrite based on load order**: the last mod loaded completely replaces the previous one.

```
Mod A: media/lua/client/ISUI/ISInventoryPane.lua  ─┐
                                                   ├─► VFS COLLISION!
Mod B: media/lua/client/ISUI/ISInventoryPane.lua  ─┘
                                │
                                ▼ (Game only mounts Mod B)
                     Mod A breaks completely!
```

---

## 3. 3-Way AST Merge Engine in Rust

To resolve VFS collisions non-destructively without discarding changes from either mod, **PZ Mod Studio** executes a 3-Way Abstract Syntax Tree (AST) merge powered by the `full_moon` Lua parser:

```
           [Base Vanilla Script]
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
    [Mod A AST]             [Mod B AST]
         │                       │
         └───────────┬───────────┘
                     ▼
       [3-Way AST Merge Engine (Rust)]
                     │
                     ▼
    [Z_PZModStudio_MasterPatch Unified Script]
```

### Core Principles of the AST Merger:
1. **Preservation of New Functions:** If Mod A adds `MyModA_Helper()` and Mod B adds `MyModB_Helper()`, both are cleanly inserted into the unified AST.
2. **Orthogonal Block Merging:** If both mods edit the same base function, the engine verifies whether changes are orthogonal and weaves them sequentially while preserving `self` references and the return call chain.
3. **Master Patch Packaging (`Z_PZModStudio_MasterPatch`):** The merged resolution is automatically packaged into the user's mod folder with the `Z_` prefix to guarantee it loads at the end of the mod chain.
