-- =============================================================================
-- PZ Mod Studio — Live Companion Bridge Mod
-- Allows AI Agents & Developer Tools to execute in-game commands, spawn/equip items,
-- evaluate dynamic Lua, broadcast server alerts, and stream player telemetry.
-- =============================================================================

local Bridge = {}
Bridge.tickCount = 0
Bridge.wasConnected = false
Bridge.RESP_FILE = "pz_ipc_resp.json"
Bridge.PLAYERS_FILE = "pz_server_players.json"
Bridge.QUEUE_FILES = { "pz_ipc_queue.json", "pz_server_commands.json" }

function Bridge.ClearPlayersState()
    pcall(function()
        local writer = getFileWriter(Bridge.PLAYERS_FILE, true, false)
        if writer then
            writer:write("[]")
            writer:close()
        end
        Bridge.lastPlayersJson = nil
    end)
end

local function safeHaloText(player, text, r, g, b)
    if not player or not text then return end
    pcall(function()
        if player.setHaloNote then
            player:setHaloNote(tostring(text), r or 255, g or 215, b or 0, 350)
        elseif HaloTextHelper and HaloTextHelper.addText then
            HaloTextHelper.addText(player, tostring(text))
        end
    end)
end

function Bridge.UpdatePlayersState(player)
    if not player then return end
    local username = player:getUsername() or "Player"
    local x = math.floor(player:getX() or 0)
    local y = math.floor(player:getY() or 0)
    local z = math.floor(player:getZ() or 0)
    local health = 1.0
    if player.getBodyDamage and player:getBodyDamage() then
        health = (player:getBodyDamage():getOverallBodyHealth() or 100) / 100
    end
    local isGod = false
    if player.isGodMod then
        isGod = player:isGodMod() == true
    end
    local role = Bridge.customRole or "User"
    if player.getAccessLevel then
        local acc = player:getAccessLevel()
        if acc and tostring(acc):lower() ~= "none" and tostring(acc) ~= "" then
            role = acc
        end
    end
    local steamId = ""
    if player.getSteamID then
        local s = player:getSteamID()
        if s then steamId = tostring(s) end
    end
    local ping = 35
    if player.getPing then
        ping = player:getPing() or 35
    end

    local jsonStr = string.format(
        '[{"username":"%s","role":"%s","ping":%d,"health":%.2f,"is_godmode":%s,"x":%d,"y":%d,"z":%d,"steam_id":"%s"}]',
        username:gsub('"', '\\"'),
        role:gsub('"', '\\"'),
        ping,
        health,
        tostring(isGod),
        x,
        y,
        z,
        steamId:gsub('"', '\\"')
    )

    -- Twice a second, but the file is written only when something in it changed: a player
    -- standing still costs no disk write.
    if jsonStr == Bridge.lastPlayersJson then return end
    local writer = getFileWriter(Bridge.PLAYERS_FILE, true, false)
    if writer then
        writer:write(jsonStr)
        writer:close()
        Bridge.lastPlayersJson = jsonStr
    end
end

function Bridge.ProcessQueueFile(queueFile, player)
    local reader = getFileReader(queueFile, false)
    if not reader then return end

    local raw = ""
    local line = reader:readLine()
    while line do
        raw = raw .. line
        line = reader:readLine()
    end
    reader:close()

    if raw ~= "" and raw ~= "{}" and not raw:match("^%s*$") then
        Bridge.lastCommandTick = Bridge.tickCount
        -- Immediately clear queue file to prevent repeated execution on error
        local writer = getFileWriter(queueFile, true, false)
        if writer then
            writer:write("{}")
            writer:close()
        end

        local success, err = pcall(function()
            Bridge.ExecuteCommand(raw, player)
        end)

        -- Write response
        local respWriter = getFileWriter(Bridge.RESP_FILE, true, false)
        if respWriter then
            if success then
                respWriter:write('{"status": "ok", "timestamp": "' .. tostring(getTimeInMillis()) .. '"}')
            else
                local msg = tostring(err):gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n'):gsub('\r', '\\r'):gsub('\t', '\\t')
                respWriter:write('{"status": "error", "message": "' .. msg .. '"}')
            end
            respWriter:close()
        end
    end
end

function Bridge.OnTick()
    Bridge.tickCount = Bridge.tickCount + 1

    local player = getPlayer()
    if not player then
        if Bridge.wasConnected then
            Bridge.wasConnected = false
            Bridge.ClearPlayersState()
        end
        return
    end

    Bridge.wasConnected = true

    -- Someone is talking to the bridge when a command arrived in the last minute (3600 frames):
    -- then the queue is read every 10 frames and the players file written every 30. Otherwise
    -- every 60 and 120. Each read opens two files and the write one, and a slow disk or an
    -- antivirus can hold an open for 30 ms: a visible hitch when it happens six times a second
    -- for the whole session, measured on a real list (Smooth Frames: worst frame 27.7 ms).
    local talking = Bridge.lastCommandTick ~= nil and (Bridge.tickCount - Bridge.lastCommandTick) < 3600
    if Bridge.tickCount % (talking and 30 or 120) == 0 then
        pcall(function() Bridge.UpdatePlayersState(player) end)
    end
    if Bridge.tickCount % (talking and 10 or 60) ~= 0 then return end

    for _, qFile in ipairs(Bridge.QUEUE_FILES) do
        Bridge.ProcessQueueFile(qFile, player)
    end
end

-- Actions that run code or cheat. On a multiplayer server they are only honoured for staff:
-- the queue files can be written by any local process or mod, and these would otherwise act
-- as a client-side cheat (desync, anti-cheat kicks).
Bridge.PRIVILEGED_ACTIONS = {
    give_item = true, spawn_vehicle = true, eval_lua = true, teleport = true,
    set_access_level = true, grant_admin = true, set_role = true,
    set_godmode = true, godmode = true,
}

function Bridge.IsStaff(player)
    local ok, level = pcall(function() return player:getAccessLevel() end)
    local lvl = ok and level and tostring(level):lower() or "none"
    return lvl ~= "none" and lvl ~= "" and lvl ~= "nil" and lvl ~= "player"
end

function Bridge.ExecuteCommand(jsonStr, player)
    local actionMatch = jsonStr:match('"action"%s*:%s*"([^"]+)"')
    if not actionMatch then error("missing \"action\"") end

    if Bridge.PRIVILEGED_ACTIONS[actionMatch] and isClient() and not Bridge.IsStaff(player) then
        error("'" .. actionMatch .. "' is refused on multiplayer servers unless you are admin/staff")
    end

    if actionMatch == "broadcast" then
        local msgMatch = jsonStr:match('"message"%s*:%s*"([^"]+)"')
        if msgMatch then
            local unescaped = msgMatch:gsub('\\n', ' '):gsub('\\"', '"')
            safeHaloText(player, "[SERVER]: " .. unescaped, 255, 69, 0)
            pcall(function()
                if processSayMessage then
                    processSayMessage("[ANNOUNCEMENT]: " .. unescaped)
                elseif ISChat and ISChat.addLineToChat then
                    ISChat.addLineToChat("[ANNOUNCEMENT]: " .. unescaped, 0)
                end
            end)
            print("[PZModStudio_Bridge] Broadcast received: " .. unescaped)
        end
    elseif actionMatch == "kick" then
        local targetMatch = jsonStr:match('"target"%s*:%s*"([^"]+)"')
        local reasonMatch = jsonStr:match('"reason"%s*:%s*"([^"]+)"') or "Kicked by admin"
        if not targetMatch or targetMatch == player:getUsername() then
            safeHaloText(player, "YOU HAVE BEEN KICKED: " .. reasonMatch, 255, 0, 0)
            pcall(function()
                if isClient() and getCore() then
                    getCore():quit()
                end
            end)
        end
    elseif actionMatch == "ban" then
        local targetMatch = jsonStr:match('"target"%s*:%s*"([^"]+)"')
        local reasonMatch = jsonStr:match('"reason"%s*:%s*"([^"]+)"') or "Banned by admin"
        if not targetMatch or targetMatch == player:getUsername() then
            safeHaloText(player, "YOU HAVE BEEN BANNED: " .. reasonMatch, 255, 0, 0)
            pcall(function()
                if isClient() and getCore() then
                    getCore():quit()
                end
            end)
        end
    elseif actionMatch == "give_item" then
        local itemMatch = jsonStr:match('"item"%s*:%s*"([^"]+)"')
        local countMatch = tonumber(jsonStr:match('"count"%s*:%s*(%d+)')) or 1
        local equipMatch = jsonStr:match('"equip"%s*:%s*true') or jsonStr:match('"equip"%s*:%s*"primary"')
        if itemMatch then
            local lastItem = nil
            for i = 1, countMatch do
                lastItem = player:getInventory():AddItem(itemMatch)
            end
            if lastItem and equipMatch then
                player:setPrimaryHandItem(lastItem)
                player:setSecondaryHandItem(lastItem)
            end
            safeHaloText(player, "Added: " .. itemMatch .. (countMatch > 1 and (" x" .. tostring(countMatch)) or ""), 0, 255, 0)
            print("[PZModStudio_Bridge] Successfully added item: " .. itemMatch)
        end
    elseif actionMatch == "spawn_vehicle" then
        local vehicleMatch = jsonStr:match('"vehicle"%s*:%s*"([^"]+)"') or "Base.StepVan"
        local cell = getCell()
        local dir = player:getDir()
        local dx, dy = 0, 0
        if dir == IsoDirections.N then dy = -4
        elseif dir == IsoDirections.S then dy = 4
        elseif dir == IsoDirections.W then dx = -4
        elseif dir == IsoDirections.E then dx = 4
        elseif dir == IsoDirections.NW then dx = -3; dy = -3
        elseif dir == IsoDirections.NE then dx = 3; dy = -3
        elseif dir == IsoDirections.SW then dx = -3; dy = 3
        elseif dir == IsoDirections.SE then dx = 3; dy = 3
        else dx = 3 end
        
        local targetSq = cell:getGridSquare(player:getX() + dx, player:getY() + dy, player:getZ()) or player:getCurrentSquare()
        
        local sm = ScriptManager.instance
        local chosenScript = vehicleMatch
        if not (sm and sm:getVehicle(chosenScript)) then
            local candidateScripts = {
                "Base.86bounder",
                "Base.73Winnebago",
                "Base.pzkBounder86",
                "Base.fr_fl_bounder_86",
                "Base.86econolinerv",
                "Base.StepVan",
                "Base.VanSeats",
                "Base.Van"
            }
            for _, s in ipairs(candidateScripts) do
                if sm and sm:getVehicle(s) then
                    chosenScript = s
                    break
                end
            end
        end
        
        local v = addVehicleDebug(chosenScript, dir, nil, targetSq)
        if v then
            local key = v:createVehicleKey()
            if key then
                player:getInventory():AddItem(key)
                if sendClientCommand then
                    pcall(sendClientCommand, player, "vehicle", "getKey", { vehicle = v:getId() })
                end
            end
            local gas = v:getPartById("GasTank")
            if gas then
                gas:setContainerContentAmount(gas:getContainerCapacity())
            end
            safeHaloText(player, "Vehicle Spawned: " .. chosenScript .. " (Key Added)", 0, 255, 128)
            print("[PZModStudio_Bridge] Successfully spawned " .. chosenScript .. " with key at (" .. tostring(targetSq:getX()) .. "," .. tostring(targetSq:getY()) .. ")")
        else
            safeHaloText(player, "Failed to spawn vehicle: " .. tostring(chosenScript), 255, 0, 0)
        end
    elseif actionMatch == "eval_lua" then
        local codeMatch = nil
        local sStart, sEnd = jsonStr:find('"code"%s*:%s*"')
        if sStart and sEnd then
            local i = sEnd + 1
            local len = #jsonStr
            local codeChars = {}
            while i <= len do
                local c = jsonStr:sub(i, i)
                if c == '\\' then
                    local nextC = jsonStr:sub(i + 1, i + 1)
                    if nextC == '"' then
                        table.insert(codeChars, '"')
                        i = i + 2
                    elseif nextC == 'n' then
                        table.insert(codeChars, '\n')
                        i = i + 2
                    elseif nextC == 'r' then
                        table.insert(codeChars, '\r')
                        i = i + 2
                    elseif nextC == 't' then
                        table.insert(codeChars, '\t')
                        i = i + 2
                    elseif nextC == '\\' then
                        table.insert(codeChars, '\\')
                        i = i + 2
                    else
                        table.insert(codeChars, c)
                        i = i + 1
                    end
                elseif c == '"' then
                    break
                else
                    table.insert(codeChars, c)
                    i = i + 1
                end
            end
            codeMatch = table.concat(codeChars)
        end
        if codeMatch then
            local loadFn = loadstring or load
            if loadFn then
                local func, loadErr = loadFn(codeMatch)
                if func then
                    local execOk, execErr = pcall(func)
                    if not execOk then
                        print("[PZModStudio_Bridge] Lua runtime error in eval: " .. tostring(execErr))
                        safeHaloText(player, "Lua Error: " .. tostring(execErr), 255, 0, 0)
                        -- the response must say it failed, not "ok"
                        error("Lua runtime error: " .. tostring(execErr), 0)
                    else
                        safeHaloText(player, "Lua Executed", 100, 200, 255)
                    end
                else
                    print("[PZModStudio_Bridge] Lua syntax error in eval: " .. tostring(loadErr))
                    error("Lua syntax error: " .. tostring(loadErr), 0)
                end
            else
                print("[PZModStudio_Bridge] dynamic load/loadstring not supported in this Kahlua scope")
            end
        end
    elseif actionMatch == "set_access_level" or actionMatch == "grant_admin" or actionMatch == "set_role" then
        local targetMatch = jsonStr:match('"target"%s*:%s*"([^"]+)"')
        local roleMatch = jsonStr:match('"message"%s*:%s*"([^"]+)"') or jsonStr:match('"role"%s*:%s*"([^"]+)"') or "admin"
        if not targetMatch or targetMatch == player:getUsername() then
            local isRevoke = roleMatch:lower() == "none" or roleMatch:lower() == "user" or roleMatch == ""
            Bridge.customRole = isRevoke and "User" or roleMatch
            pcall(function()
                if player.setAccessLevel then
                    player:setAccessLevel(isRevoke and "None" or roleMatch)
                end
                if roleMatch:lower() == "admin" then
                    if player.setGodMod then player:setGodMod(true) end
                    if player.setGhostMode then player:setGhostMode(true) end
                elseif isRevoke then
                    if player.setGodMod then player:setGodMod(false) end
                    if player.setGhostMode then player:setGhostMode(false) end
                end
            end)
            safeHaloText(player, "ACCESS LEVEL: " .. (isRevoke and "USER" or string.upper(roleMatch)), 255, 215, 0)
            print("[PZModStudio_Bridge] Access level updated to " .. roleMatch .. " for " .. (targetMatch or player:getUsername()))
        end
    elseif actionMatch == "set_godmode" or actionMatch == "godmode" then
        local current = false
        if player.isGodMod then current = player:isGodMod() == true end
        player:setGodMod(not current)
        if not current then
            safeHaloText(player, "Godmode ENABLED", 255, 215, 0)
        else
            safeHaloText(player, "Godmode DISABLED", 200, 200, 200)
        end
    elseif actionMatch == "teleport" then
        -- z is negative for B42 basements
        local xMatch = tonumber(jsonStr:match('"x"%s*:%s*(%-?[%d%.]+)'))
        local yMatch = tonumber(jsonStr:match('"y"%s*:%s*(%-?[%d%.]+)'))
        local zMatch = tonumber(jsonStr:match('"z"%s*:%s*(%-?[%d%.]+)')) or 0
        if xMatch and yMatch then
            if player.teleportTo then
                -- B42: setX/setY alone snap back and setLx/setLy/setLz no longer exist
                player:teleportTo(xMatch, yMatch, zMatch)
            else
                player:setX(xMatch)
                player:setY(yMatch)
                player:setZ(zMatch)
                if player.setLx then
                    player:setLx(xMatch)
                    player:setLy(yMatch)
                    player:setLz(zMatch)
                end
            end
            safeHaloText(player, string.format("Teleported to %d, %d, %d", math.floor(xMatch), math.floor(yMatch), math.floor(zMatch)), 200, 100, 255)
        end
    else
        error("unknown action '" .. actionMatch .. "'")
    end
end

-- -----------------------------------------------------------------------------
-- Developer reload. The Studio writes pz_dev_reload_queue.txt ("id=<id>", then a "file=<path>"
-- line per file) the moment a Lua file of the mod under development is saved. It is looked at
-- every few frames on the main menu, in game and paused (the queue above needs a player), and
-- only while the Studio watches: pz_dev_watch.txt says on=1, looked at every two seconds.
-- reloadLuaFile reroutes the file's event handlers to the new functions itself (an Events.X.Add
-- during a reload does nothing), so handlers are not doubled.
-- -----------------------------------------------------------------------------
Bridge.DEV_QUEUE = "pz_dev_reload_queue.txt"
Bridge.DEV_RESULT = "pz_dev_reload.json"
Bridge.DEV_WATCH = "pz_dev_watch.txt"
Bridge.devTicks = 0

local function readLines(name)
    local reader = getFileReader(name, false)
    if not reader then return nil end
    local out = {}
    local line = reader:readLine()
    while line do
        table.insert(out, line)
        line = reader:readLine()
    end
    reader:close()
    return out
end

local function jsonString(s)
    s = string.gsub(tostring(s), '\\', '\\\\')
    s = string.gsub(s, '"', '\\"')
    s = string.gsub(s, '\n', '\\n')
    s = string.gsub(s, '\r', '\\r')
    s = string.gsub(s, '\t', '\\t')
    return '"' .. s .. '"'
end

-- What a reload keeps, beyond what reloadLuaFile does itself:
-- - Event handlers. During reloadLuaFile an Events.X.Add does nothing; the game points the
--   file's handlers that were already registered at the new functions. So a handler the file
--   adds for the first time never registers, and one taken out of the file keeps running. Here
--   the file's Add calls are noted while it reloads, then registered for real once it is done
--   (removed first: never twice), and the ones of its last reload it no longer adds go.
-- - Tables. A file that makes its tables again (`MyPanel = ISPanel:derive(...)`,
--   `MyMod = {}`) leaves open windows and saved references on the old ones, with the old code.
--   For each global table the file replaced, the new functions (and any field the old one
--   lacks) go into the old table, and the old table stays: same identity, same state, new code.
-- - PZStudio.onReload(fn): called after each reload of the file that registered it.
Bridge.devHandlers = Bridge.devHandlers or {}

-- For the mod being developed: `if PZStudio then PZStudio.onReload(function(file) ... end) end`
-- (the bridge may load after the mod the first time; on a reload it is always there). Called
-- after the file that registered it is reloaded, to set up again what OnGameStart did.
PZStudio = PZStudio or {}
PZStudio.reloadHooks = PZStudio.reloadHooks or {}
function PZStudio.onReload(fn)
    if type(fn) ~= "function" then return end
    local file = getFilenameOfClosure and getFilenameOfClosure(fn) or "?"
    local line = getFirstLineOfClosure and getFirstLineOfClosure(fn) or 0
    PZStudio.reloadHooks[tostring(file) .. ":" .. tostring(line)] = fn
end

local function eventNames()
    local names = {}
    if type(Events) ~= "table" then return names end
    for name, ev in pairs(Events) do
        if type(ev) == "table" and ev.Add and ev.Remove then table.insert(names, name) end
    end
    return names
end

function Bridge.ReloadKeepingState(f)
    -- The global tables before.
    local before = {}
    for k, v in pairs(_G) do
        if type(v) == "table" then before[k] = v end
    end
    -- Hooks of this file are registered again by it.
    local base = string.match(f, "([^/\\]+)$") or f
    for key, _ in pairs(PZStudio.reloadHooks) do
        if string.find(key, base, 1, true) then PZStudio.reloadHooks[key] = nil end
    end
    -- Its Add calls, noted.
    local added, originals = {}, {}
    for _, name in ipairs(eventNames()) do
        local ev = Events[name]
        local add = ev.Add
        originals[name] = add
        ev.Add = function(fn, ...)
            table.insert(added, { event = name, fn = fn })
            return add(fn, ...)
        end
    end
    local ok, err = pcall(reloadLuaFile, f)
    for name, add in pairs(originals) do Events[name].Add = add end
    if not ok then return false, err, "" end

    -- Handlers: the last reload's out, this one's in, once each.
    local handlers = 0
    for _, h in ipairs(Bridge.devHandlers[f] or {}) do
        pcall(Events[h.event].Remove, h.fn)
    end
    for _, h in ipairs(added) do
        pcall(Events[h.event].Remove, h.fn)
        if pcall(Events[h.event].Add, h.fn) then handlers = handlers + 1 end
    end
    Bridge.devHandlers[f] = added

    -- Tables: the old ones kept, with the new code.
    local tables = 0
    for k, old in pairs(before) do
        local new = rawget(_G, k)
        if type(new) == "table" and new ~= old and k ~= "_G" then
            for field, v in pairs(new) do
                if type(v) == "function" or old[field] == nil then old[field] = v end
            end
            -- The new table's metatable (a derived class's parent) wins: the code is new.
            local mt = getmetatable(new)
            if mt and mt ~= old then setmetatable(old, mt) end
            rawset(_G, k, old)
            tables = tables + 1
        end
    end
    local kept = {}
    if handlers > 0 then table.insert(kept, tostring(handlers) .. " handler(s)") end
    if tables > 0 then table.insert(kept, tostring(tables) .. " table(s) kept") end
    return true, nil, table.concat(kept, ", ")
end

function Bridge.RunReloadHooks(files)
    for key, fn in pairs(PZStudio.reloadHooks) do
        for _, f in ipairs(files) do
            local base = string.match(f, "([^/\\]+)$") or f
            if string.find(key, base, 1, true) then
                local ok, err = pcall(fn, f)
                if not ok then print("[PZModStudio] onReload hook failed: " .. tostring(err)) end
                break
            end
        end
    end
end

function Bridge.DevReload(lines)
    local id, files = "", {}
    for _, l in ipairs(lines) do
        local key, value = string.match(l, "^(%a+)=(.*)$")
        if key == "id" then id = value end
        if key == "file" and value ~= "" then table.insert(files, value) end
    end
    local results = {}
    local player = getPlayer()
    if isClient() and not (player and Bridge.IsStaff(player)) then
        table.insert(results, jsonString("failed: reloading is refused on a multiplayer server unless you are admin/staff"))
        files = {}
    end
    -- Under -debug an error in the reloaded file would stop the game in the Lua debugger.
    local quiet = isDebugEnabled() and UIManager and UIManager.isShowLuaDebuggerOnError and UIManager.isShowLuaDebuggerOnError()
    if quiet then pcall(UIManager.setShowLuaDebuggerOnError, false) end
    for _, f in ipairs(files) do
        local ok, err, kept = Bridge.ReloadKeepingState(f)
        local line = (ok and "ok " or "failed ") .. f .. (ok and "" or (": " .. tostring(err)))
        if ok and kept ~= "" then line = line .. " (" .. kept .. ")" end
        table.insert(results, jsonString(line))
        print("[PZModStudio] reload " .. line)
    end
    if quiet then pcall(UIManager.setShowLuaDebuggerOnError, true) end
    Bridge.RunReloadHooks(files)
    local writer = getFileWriter(Bridge.DEV_RESULT, true, false)
    if writer then
        writer:write('{"id":' .. jsonString(id) .. ',"applied_ms":' .. string.format("%.0f", getTimestampMs()) .. ',"results":[' .. table.concat(results, ",") .. ']}')
        writer:close()
    end
end

function Bridge.DevTick()
    Bridge.devTicks = Bridge.devTicks + 1
    if Bridge.devWatching == nil or Bridge.devTicks % 120 == 0 then
        local watch = readLines(Bridge.DEV_WATCH)
        Bridge.devWatching = watch ~= nil and watch[1] == "on=1"
    end
    if not Bridge.devWatching or Bridge.devTicks % 3 ~= 0 then return end
    local lines = readLines(Bridge.DEV_QUEUE)
    if not lines or #lines == 0 then return end
    -- Emptied first: a file that fails to reload is not reloaded again every few frames.
    local writer = getFileWriter(Bridge.DEV_QUEUE, true, false)
    if writer then writer:close() end
    Bridge.DevReload(lines)
end

local function devTick()
    local ok, err = pcall(Bridge.DevTick)
    if not ok and not Bridge.devFailed then
        Bridge.devFailed = true
        print("[PZModStudio_Bridge] developer reload failed: " .. tostring(err))
    end
end

if Events then
    if Events.OnTickEvenPaused then Events.OnTickEvenPaused.Add(devTick) elseif Events.OnTick then Events.OnTick.Add(devTick) end
    if Events.OnFETick then Events.OnFETick.Add(devTick) end
    if Events.OnTick then Events.OnTick.Add(Bridge.OnTick) end
    if Events.OnDisconnect then Events.OnDisconnect.Add(Bridge.ClearPlayersState) end
    if Events.OnMainMenuEnter then Events.OnMainMenuEnter.Add(Bridge.ClearPlayersState) end
    if Events.OnDestroyIsoCode then Events.OnDestroyIsoCode.Add(Bridge.ClearPlayersState) end
end
print("[PZModStudio_Bridge] Live Companion Mod Initialized successfully!")
