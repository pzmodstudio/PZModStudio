# 06 — ModData Persistence, Networking & Security

Project Zomboid implements a semi-authoritative client-server networking model. To sustain smooth performance with hundreds of zombies simulated on screen, portion of physics and character state is simulated on the local client. This requires careful coordination of persistent state (`ModData`) and network packet exchange.

---

## 1. Persistence Scopes with `ModData`

`ModData` is the engine's built-in key-value storage system serialized directly into world savegames:

```
┌─────────────────────────────────────────────────────────────┐
│                       MODDATA SCOPES                        │
├───────────────────┬───────────────────┬─────────────────────┤
│   Object / Item   │      Player       │    World / Global   │
│ item:getModData() │player:getModData()│ModData.getOrCreate()│
│ Serialized into   │ Serialized into   │ Serialized into     │
│ item save bytearr │ player save file  │ map_meta.bin        │
└───────────────────┴───────────────────┴─────────────────────┘
```

### Usage Example:
```lua
-- Attach persistent metadata to an item
local modData = item:getModData()
modData.CustomDurability = 150
modData.OwnerUUID = player:getUsername()
```

---

## 2. Multiplayer Synchronization: `ModData` is NOT Automatically Replicated

⚠️ **Critical Pitfall:** Modifying `item:getModData()` or `player:getModData()` inside a client-side script (`media/lua/client/`) only mutates local memory. The dedicated server and other players **will never see the update**, leading to desynchronization and state overwrites upon reloading.

### Authoritative Network Exchange Pattern:

```
┌──────────────────┐                               ┌──────────────────┐
│     CLIENT       │                               │     SERVER       │
└────────┬─────────┘                               └────────┬─────────┘
         │                                                  │
         │  1. sendClientCommand("MyMod", "DoAction", args) │
         ├─────────────────────────────────────────────────►│
         │                                                  │
         │                                                  │ 2. Events.OnClientCommand
         │                                                  │    - Validates permissions/range
         │                                                  │    - Mutates true ModData state
         │                                                  │    - Persists changes to disk
         │                                                  │
         │  3. sendServerCommand("MyMod", "SyncState", data)│
         │◄─────────────────────────────────────────────────┤
         │                                                  │
         │ 4. Events.OnServerCommand                        │
         │    - Updates local UI and client state           │
         ▼                                                  ▼
```

### Implementation Code:

#### Client Layer (`media/lua/client/MyMod_Client.lua`):
```lua
-- Send request to server (primitive types only: IDs, coordinates, numbers, strings)
local args = { targetSquareX = 10520, targetSquareY = 9412, targetZ = 0 }
sendClientCommand(player, "MyModModule", "RequestRepair", args)

-- Receive confirmation from server
local function OnServerCommand(module, command, args)
    if module == "MyModModule" and command == "SyncRepairSuccess" then
        HaloTextHelper.addText(getPlayer(), "Repair completed successfully!", 0, 255, 0)
    end
end
Events.OnServerCommand.Add(OnServerCommand)
```

#### Server Layer (`media/lua/server/MyMod_Server.lua`):
```lua
local function OnClientCommand(module, command, player, args)
    if module == "MyModModule" and command == "RequestRepair" then
        -- 1. Validate player distance and materials on the server
        local square = getCell():getGridSquare(args.targetSquareX, args.targetSquareY, args.targetZ)
        if square and player:DistTo(square:getX(), square:getY()) < 4 then
            -- 2. Modify authoritative ModData
            local modData = square:getModData()
            modData.isRepaired = true
            
            -- 3. Broadcast update to nearby players
            sendServerCommand("MyModModule", "SyncRepairSuccess", { success = true })
        end
    end
end
Events.OnClientCommand.Add(OnClientCommand)
```

### 🚫 Do Not Pass Java Objects Over the Network:
Never place direct Java references (such as `InventoryItem`, `IsoPlayer`, or `IsoGridSquare`) inside the `args` table of `sendClientCommand`. The network serializer only accepts primitives (strings, numbers, booleans, and nested tables). Pass item IDs (`item:getID()`) or grid coordinates (`x, y, z`).

---

## 3. Cryptographic Anti-Cheat Attestation (CHAP)

Project Zomboid incorporates a Challenge-Handshake Authentication Protocol (CHAP) pipeline to prevent unauthorized script injections:

1. **Server Probe (`Server Challenge`):** The server periodically transmits an encrypted challenge token containing high-entropy random nonces.
2. **In-Memory Integrity Canaries:** The client runtime scans active Kahlua tables to verify that core combat, health, and movement functions have not been replaced by cheat wrappers.
3. **Hexadecimal Attestation Payload:** The client responds with a compressed attestation packet (`version + requestId + nonce + protectedFlags + luaSurfaceMask + signature`). Discrepancies in the resulting hash cause immediate disconnection for integrity violations.
