-- PZ Mod Studio Link: which mods replace a Lua file that another mod that is on also ships.
--
-- The game keeps one copy of each file path: when two mods ship media/lua/client/X.lua, the copy
-- of the mod whose files load later is the one that runs, and the other mod's copy is never read.
-- The game says so in console.txt (`mod "B" overrides media/lua/client/x.lua`), a file Lua cannot
-- read. So the Link works it out the game's way, from what Lua can see:
--   - getActivatedMods(): the mods the game loaded, in the order it took their files (each one
--     after what it requires). Of the mods that ship a path, the last one's copy is the one used
--     (and a mod's version folder, 42/, wins over its own common/: the same mod);
--   - getLoadedLua(i): the Lua files the game ran, so the folders mods keep Lua in
--     (media/lua/client/..., media/lua/shared/...), and the same folders under media/lua/server
--     (that Lua runs only in a game, so it is not in the list on the main menu);
--   - listFilesInModDirectory(mod, folder): each mod's files in each of those folders (common/
--     and the version folder).
-- Only Lua files, and only in folders some mod's Lua already ran from (or their server twins):
-- what is marked is what the game does, but a folder no running file is in is not looked in.
-- Scripts, textures, sounds also go by "the last copy is the one used", but there is no way to
-- know their folders from Lua.
--
-- Mods x folders can be tens of thousands of listings: done BUDGET_MS a frame on the main menu,
-- once per load of the game's Lua (the mods loaded change only with a reload, as after Accept).
-- Result, by plain mod id: Over.byMod[id] = { replaces = { [otherId] = { path, ... } },
-- replacedBy = { [winnerId] = { path, ... } }, nReplaces = <paths>, nReplacedBy = <paths> }.

require "PZModStudio_Link"

local Link = PZModStudioLink
local plainId = Link.plainId

local BUDGET_MS = 4
local START_AFTER_MS = 1500

local Over = { state = "idle", byMod = {}, count = 0, listeners = {} }
Link.Overwrites = Over

local function norm(path)
    local s = string.gsub(tostring(path), "\\", "/")
    s = string.gsub(s, "/+$", "")
    return s
end

local function nowMs()
    local ok, ms = pcall(getTimestampMs)
    return ok and ms or 0
end

--- The mods the game loaded, in the order it took their files: { id = plain, call = id the
--- game's functions take }.
local function loadedMods()
    local out = {}
    local mods = getActivatedMods()
    for i = 0, mods:size() - 1 do
        local raw = tostring(mods:get(i))
        local id = plainId(raw)
        local call = id
        local okInfo, info = pcall(getModInfoByID, id)
        if not (okInfo and info) then call = raw end
        table.insert(out, { id = id, call = call })
    end
    return out
end

--- Each loaded mod's folders (lower case, "/"), to the mod's plain id.
local function folderOwners(mods)
    local owners = {}
    for _, m in ipairs(mods) do
        local ok, info = pcall(getModInfoByID, m.call)
        if ok and info then
            local dirs = {}
            pcall(function() table.insert(dirs, info:getCommonDir()) end)
            pcall(function() table.insert(dirs, info:getVersionDir()) end)
            pcall(function() table.insert(dirs, info:getDir()) end)
            for _, dir in ipairs(dirs) do
                if dir then owners[string.lower(norm(dir))] = m.id end
            end
        end
    end
    return owners
end

--- The folders some mod's Lua ran from, and their twins under media/lua/server:
--- { { path = as written, key = lower case } }.
local function luaFolders(owners)
    local dirs, seen = {}, {}
    local function add(path)
        local key = string.lower(path)
        if seen[key] then return end
        seen[key] = true
        table.insert(dirs, { path = path, key = key })
    end
    add("media/lua/server")
    local n = getLoadedLuaCount()
    for i = 0, n - 1 do
        local okPath, raw = pcall(getLoadedLua, i)
        if okPath and raw then
            local path = norm(raw)
            local lower = string.lower(path)
            local at = string.find(lower, "/media/lua/", 1, true)
            if at and owners[string.sub(lower, 1, at - 1)] then
                local rel = string.sub(path, at + 1)
                local dir = string.match(rel, "^(.*)/[^/]*$")
                if dir then
                    add(dir)
                    local head = string.lower(string.sub(dir, 1, 16))
                    if head == "media/lua/client" or head == "media/lua/shared" then
                        add("media/lua/server" .. string.sub(dir, 17))
                    end
                end
            end
        end
    end
    return dirs
end

function Over.start()
    if Over.state ~= "idle" then return end
    local mods = loadedMods()
    Over.mods = mods
    Over.dirs = luaFolders(folderOwners(mods))
    Over.files = {}
    Over.mi, Over.di = 1, 1
    Over.state = (#mods > 1 and #Over.dirs > 0) and "running" or "done"
    Over.startedAt = nowMs()
end

--- Lists until the frame's budget is spent. True when every mod and folder was looked at.
function Over.step()
    local t0 = nowMs()
    local mods, dirs, files = Over.mods, Over.dirs, Over.files
    while Over.mi <= #mods do
        local m = mods[Over.mi]
        local dir = dirs[Over.di]
        local ok, list = pcall(listFilesInModDirectory, m.call, dir.path)
        if ok and list then
            for i = 0, list:size() - 1 do
                local name = tostring(list:get(i))
                local lname = string.lower(name)
                if string.sub(lname, -4) == ".lua" then
                    local key = dir.key .. "/" .. lname
                    local f = files[key]
                    if not f then
                        f = { path = dir.path .. "/" .. name, mods = {} }
                        files[key] = f
                    end
                    if f.mods[#f.mods] ~= m.id then table.insert(f.mods, m.id) end
                end
            end
        end
        Over.di = Over.di + 1
        if Over.di > #dirs then
            Over.di = 1
            Over.mi = Over.mi + 1
        end
        if nowMs() - t0 >= BUDGET_MS then return false end
    end
    return true
end

--- From who ships what: who replaces whom.
function Over.finish()
    local byMod, count = {}, 0
    local function rec(id)
        local r = byMod[id]
        if not r then
            r = { replaces = {}, replacedBy = {}, nReplaces = 0, nReplacedBy = 0 }
            byMod[id] = r
            count = count + 1
        end
        return r
    end
    for _, f in pairs(Over.files or {}) do
        local n = #f.mods
        if n > 1 then
            local winner = f.mods[n]
            local w = rec(winner)
            local counted = false
            for i = 1, n - 1 do
                local loser = f.mods[i]
                if loser ~= winner then
                    local l = rec(loser)
                    w.replaces[loser] = w.replaces[loser] or {}
                    table.insert(w.replaces[loser], f.path)
                    l.replacedBy[winner] = l.replacedBy[winner] or {}
                    table.insert(l.replacedBy[winner], f.path)
                    l.nReplacedBy = l.nReplacedBy + 1
                    if not counted then
                        w.nReplaces = w.nReplaces + 1
                        counted = true
                    end
                end
            end
        end
    end
    for _, r in pairs(byMod) do
        for _, paths in pairs(r.replaces) do table.sort(paths) end
        for _, paths in pairs(r.replacedBy) do table.sort(paths) end
    end
    Over.byMod, Over.count = byMod, count
    Over.files, Over.mods, Over.dirs = nil, nil, nil
    Over.state = "done"
    Over.tookMs = nowMs() - (Over.startedAt or nowMs())
    for _, fn in ipairs(Over.listeners) do pcall(fn) end
end

--- What a mod replaces and what replaces it, or nil (none, or not known yet).
function Over.of(id)
    if Over.state ~= "done" then return nil end
    return Over.byMod[id]
end

function Over.ready()
    return Over.state == "done"
end

--- fn() runs when the result is ready (the Mods screen marks its rows then).
function Over.onReady(fn)
    table.insert(Over.listeners, fn)
end

local firstTick = nil
local function tick()
    if Over.state == "done" or Over.state == "failed" then return end
    if Link.inGame then return end
    if Over.state == "idle" then
        local t = nowMs()
        if not firstTick then firstTick = t end
        if t - firstTick < START_AFTER_MS then return end
        Over.start()
        if Over.state == "done" then Over.finish() end
        return
    end
    if Over.step() then Over.finish() end
end

local function safeTick()
    local ok, err = pcall(tick)
    if not ok then
        Over.state = "failed"
        print("[StudioLink] the check of files mods share failed: " .. tostring(err))
    end
end

if Events.OnFETick then Events.OnFETick.Add(safeTick) end
