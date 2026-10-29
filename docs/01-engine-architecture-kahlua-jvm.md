# 01 — Engine Architecture, JVM & Kahlua Virtual Machine

Project Zomboid operates on a highly stratified two-tier software architecture: a **native Java core** running on the Java Virtual Machine (JVM) combined with a **pure Java Lua interpreter** called **Kahlua**.

```
┌─────────────────────────────────────────────────────────────┐
│                   LUA LAYER (Scripts / Mods)                │
│   media/lua/shared  │   media/lua/client  │  media/lua/server  │
└──────────────────────────────┬──────────────────────────────┘
                               │ Bidirectional Invocation
┌──────────────────────────────▼──────────────────────────────┐
│                  KAHLUA VIRTUAL MACHINE                     │
│       Lua 5.1 interpreter implemented in pure Java (JVM)    │
│  - No JNI / C bridges     - Direct method export & binding  │
└──────────────────────────────┬──────────────────────────────┘
                               │ Java Object Manipulation
┌──────────────────────────────▼──────────────────────────────┐
│                      GAME CORE (JAVA)                       │
│  - IsoWorld, IsoGridSquare, InventoryItem, IsoPlayer        │
│  - LWJGL / OpenGL Rendering, FMOD Audio, Bullet Physics     │
│  - Java Garbage Collector (GC)                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 1. The Kahlua Virtual Machine & Its Architectural Trade-offs

Unlike most game engines that embed LuaJIT or C-based Lua runtimes via foreign function interfaces (FFI/JNI), Project Zomboid runs **Kahlua**, a complete Lua 5.1 VM written entirely in Java.

### Design Benefits:
- **Seamless Integration:** Lua scripts can directly instantiate Java classes and invoke exposed native methods without glue code or C-bindings:
  ```lua
  local invItem = InventoryItemFactory.CreateItem("Base.Axe")
  player:getInventory():AddItem(invItem)
  ```
- **Platform Portability:** No need to compile architecture-specific binary dynamic libraries (`.dll`, `.so`, `.dylib`) for different OS targets (Windows/Linux/macOS).

### Performance Costs & Bottlenecks:
1. **JVM Garbage Collection Overhead:** Lua tables in Kahlua are represented in Java memory as `KahluaTable` instances. High-frequency allocations within frame or tick loops create objects directly on the Java heap, resulting in periodic GC pauses and frame stutters.
2. **Reflection & Dispatch Penalty:** Invoking Java methods from Lua requires reflection lookups and argument unboxing on every call.

---

## 2. Type Interoperability: Java vs. Lua Collections

The boundary between Java and Kahlua does not automatically convert Java collections into native Lua tables. This creates critical syntax differences:

### Structural Comparison Table

| Operation / Type | Native Lua Table | `java.util.List` / `ArrayList` (Java) |
| :--- | :--- | :--- |
| **Initial Index** | **1-based** (`table[1]`) | **0-based** (`list:get(0)`) |
| **Length / Size** | `#table` or `table.wipe(t)` | `list:size()` |
| **Standard Iteration** | `for i, v in ipairs(table) do` | `for i = 0, list:size() - 1 do local v = list:get(i)` |
| **Direct Search** | Requires $O(n)$ iteration or key hash | `list:contains("string")` $O(n)$ optimized in Java |
| **Adding Elements** | `table.insert(table, item)` | `list:add(item)` |
| **Clearing / Wiping** | `table.wipe(table)` | `list:clear()` |

### ⚠️ Common Anti-Pattern: Using `ipairs()` on Java Lists
```lua
-- ❌ FATAL ERROR: recipes is a java.util.ArrayList
local recipes = player:getKnownRecipes()
for i, recipe in ipairs(recipes) do -- Fails silently or returns nil
    print(recipe)
end

-- ✅ CORRECT APPROACH:
local recipes = player:getKnownRecipes()
for i = 0, recipes:size() - 1 do
    local recipe = recipes:get(i)
    print(recipe)
end
```

---

## 3. Rendering Engine & LWJGL / OpenGL Layer

Project Zomboid uses **LWJGL (Lightweight Java Game Library)** as its OpenGL wrapper:
- **Single-Threaded Render Pipeline:** All isometric drawing routines execute strictly on the main render thread (`MainThread`). Calling render routines or texture modifications from asynchronous threads or non-synchronized callbacks triggers `OpenGLException` or graphics context corruption.
- **Isometric Screen Coordinates:** The game world transforms 3D grid coordinates $(x, y, z)$ into 2D screen coordinates $(sx, sy)$ via the engine's isometric projection:
  $$sx = (x - y) \cdot (\text{tileWidth} / 2)$$
  $$sy = (x + y) \cdot (\text{tileHeight} / 2) - z \cdot \text{tileHeight}$$

---

## 4. Performance Best Practices for Kahlua

1. **Reuse Tables (Avoid Loop Allocations):** In frequent event handlers like `OnPlayerUpdate` or `OnTick`, declare scratch tables as file-scoped locals and clear them with `table.wipe(t)` instead of repeatedly allocating `t = {}`.
2. **Leverage Native Java Helpers:** Methods like `ArrayList:contains()` or `HashMap:get()` execute in compiled Java bytecode without entering the Lua interpreter loop, making them significantly faster for large collections.
3. **Minimize String Concatenations:** Frequent concatenations (`"a" .. "b" .. "c"`) inside loops allocate ephemeral `java.lang.String` objects in the JVM. Use string buffers or formatted template calls.
