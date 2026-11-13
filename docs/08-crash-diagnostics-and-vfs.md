# 08 — Crash Diagnostics, VFS & 3-Way AST Merging

Diagnosing silent crashes, load order conflicts, and script collisions is one of the most demanding tasks in Project Zomboid development. This chapter outlines the error taxonomy of `console.txt`, the Virtual File System (VFS) collision model, and the 3-Way Abstract Syntax Tree (AST) merge engine.

---

## 1. Error Taxonomy & Crash Signatures in `console.txt`

The active log file at `C:/Users/<User>/Zomboid/console.txt` captures all JVM stdout/stderr streams and Kahlua VM tracebacks.

A card says only what its log line shows. What a line cannot show (who caused it, what else broke,
what happens next time) is not on the card. Checked against Project Zomboid 42.22 and a corpus of 753
session logs (2026-11-12).

| Line in console.txt | What it is | Card |
| :--- | :--- | :--- |
| `se.krka.kahlua.vm.KahluaException: <msg> at BaseLib.error` | A script called `error("msg")`. Under a `pcall` frame the script caught it itself. | "A Lua script raised an error itself" |
| `attempted index: K of non-table: null` | The script read or called `K` on a value that was nil. The file and line are in the stack beside it. | "A script used something that was not there (nil)", or the card of the mod named in the block |
| `java.lang.NullPointerException` | Java used a missing value; the class after `at` says where. Most are the game's or a Java mod's own code, not a Lua mod's call. | "Java met a missing value" |
| `Translator.reportMissingArgumentsFromPastAbuse` | A text with a bare `%` (formatting) or a placeholder given no value (missing arguments). The game shows the text and goes on. The key can be the game's own. | "A text has a % sign..." / "A text has a placeholder..." |
| `java.lang.OutOfMemoryError` | "Java heap space": the `-Xmx` limit. Anything else: memory outside the heap. | "The game's Java ran out of / could not get memory" |
| `AdvancedAnimator$1.visitFileFailed` + `NoSuchFileException ...\AnimSets` or `\actiongroups` | The game lists those folders in every active mod and logs each mod that has none. | None: the game's own noise |
| `BufferOverflowException at PathfindNative.findPath` | The native pathfinder's own buffer. | None |
| `require("X") failed` at `Lua(Vanilla)` | The game's own file asking for a module of another phase. | None |

How the game prints a Lua error (42.22):

```text
ERROR: General f:240> Lua((MOD:My Mod)).perform> Exception thrown
	java.lang.RuntimeException: attempted index: getName of non-table: null at KahluaThread.tableget(...)
	Stack trace:
		Lua((MOD:My Mod)).perform(MyCustomAction.lua:42)
		Lua(Vanilla).update(ISTimedActionQueue.lua:78)
```

The frames are innermost first. `Lua((MOD:X))` names the mod; `Lua(Vanilla)` is the game's own file.

Who a card blames:
- The mod the line names (`(MOD:X)`), or the mod that replaced the `media/` path the line names.
- A mod named on a line above only for lines of that mod's own block (the frame's echo, the indented stack, the bare exception under it). The card then says "named just above this line".
- Never a mod whose name merely appears in a path, and never whoever was mentioned before an ERROR that carries its own `at Class.method >` source.

Severity: a card is an exception when the game wrote the line at ERROR level or as part of a stack trace, and a warning when the game wrote it at WARN/LOG level. Many ERROR-level lines of the game throw nothing (a font texture that is not shipped, a bone a model lacks, a recipe falling back to a default): they make no card unless a rule above says what they are.

How `require` resolves (42.22 `LuaManager`): the game loads `shared`, then `client`, then `server`; within a phase its own files first, then the mods'. A `require` runs the file at once when it is on a path already added, and otherwise warns and returns nil (it never throws). So `shared` cannot see `client` or `server`, `client` cannot see `server`, and on a dedicated server `client/` is not loaded at all.

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
