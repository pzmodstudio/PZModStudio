# 📚 Project Zomboid Technical Wiki & Modding Architecture

Welcome to the **Project Zomboid Technical Wiki**, a comprehensive and modular knowledge base designed for software engineers, modders, and AI agents (connected via MCP, Antigravity, Claude, Cursor, etc.).

This documentation details the internal mechanics of The Indie Stone's engine, the **Kahlua (Lua-in-Java)** virtual machine, critical architectural differences between **Build 41 (Legacy)** and **Build 42 (Unstable/Modern)**, the multiplayer networking protocol, and real-time process control systems.

---

## 🗺️ Table of Contents

| Module | Document | Technical Summary |
| :--- | :--- | :--- |
| **01** | [**Engine Architecture, JVM & Kahlua VM**](01-engine-architecture-kahlua-jvm.md) | Java-Lua integration, Kahlua interpreter, Garbage Collection, type interoperability (List vs Table, 0-based vs 1-based indexing), and OpenGL/LWJGL rendering limits. |
| **02** | [**Lua Script Lifecycle & Event Bus**](02-lua-lifecycle-and-events.md) | `shared`, `client`, `server` domains. Alphabetical load order and vanilla precedence. Core engine events, safe subscription models, and advanced Monkey Patching (Function Wrapping). |
| **03** | [**Item Definitions, Crafting & Fluid API (B42)**](03-crafting-items-and-fluids-b42.md) | Transition from `Recipe` to `craftRecipe`, semantic tag indexing (`Tags`), physical inheritance flags (`InheritCooked`, `InheritFood`), and the `FluidContainer` API (preventing harmonic feedback loops). |
| **04** | [**UI Hierarchy, Context Menus & Timed Actions**](04-ui-context-menu-and-timedactions.md) | `ISUIElement` hierarchy, frame rendering lifecycle, safe context menu injection (`OnFillWorldObjectContextMenu` and the `test` flag), and the full `ISBaseTimedAction` state machine. |
| **05** | [**Loot Distribution & Procedural Spawning**](05-loot-distribution-and-spawns.md) | `ItemPickerJava` mechanics, `SuburbsDistributions.lua`, `ProceduralDistributions.lua`, room/container mapping, and Java collection signature changes in B42 (`ArrayList` vs `Set`). |
| **06** | [**ModData Persistence, Networking & Security**](06-networking-moddata-and-security.md) | `ModData` persistence scopes (Item, Player, IsoObject, Global). Client-server sync via `sendClientCommand`/`sendServerCommand`. CHAP cryptographic attestation and anti-injection canaries. |
| **07** | [**FMOD Audio, JSON Translations & 3D Z-Levels**](07-sound-translations-and-b42-space.md) | Sound definition scripts (`media/scripts/*.txt`), mandatory JSON translation format (B42.15+), `-debugtranslation` validation, and expanded vertical space (Z-coordinates from -32 to +32). |
| **08** | [**Crash Diagnostics, VFS & 3-Way AST Merging**](08-crash-diagnostics-and-vfs.md) | `console.txt` error taxonomy (`KahluaThreadException`, `NoSuchMethodError`, `UnknownFormatConversionException`), Virtual File System overwrites, and 3-way AST merge algorithms. |
| **09** | [**Game Process Control & Live IPC Bridge**](09-game-control-and-ipc-bridge.md) | `ProjectZomboid64.exe` lifecycle control (launching with `-debug`, terminating process), and the companion mod `Z_PZModStudio_Bridge` for real-time Lua evaluation and hot item equipping. |
| — | [**Releases**](RELEASES.md) | How a release is built (version check, tests, no caches, pinned Actions), signed (ed25519 sums, GitHub attestations) and checked by a player. |

---

## 🎯 Usage Guidelines for AI Agents

1. **Contextual Reading:** Before generating or modifying Project Zomboid scripts, review the relevant chapter to ensure compatibility with the target version (`Build 41` vs `Build 42`).
2. **Declarative Priority:** If a game mechanic can be expressed using declarative `.txt` scripts (`craftRecipe`, `item`, `sound`), avoid injecting procedural Lua code.
3. **Runtime Safety:** Never use anonymous functions in `Events.*.Add()` subscriptions and never break the call chain in monkey patches.
4. **Live Diagnostics:** Use PZ Mod Studio's MCP tools to monitor `console.txt`, execute IPC bridge commands, and validate Lua syntax using the AST parser (`full_moon`).
