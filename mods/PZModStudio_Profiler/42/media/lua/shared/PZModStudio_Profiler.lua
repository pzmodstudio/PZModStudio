-- PZ Mod Studio Profiler: how much of the game's time each mod's Lua takes: its event handlers,
-- and the render / prerender / update of its classes (windows, bars, timed actions).
--
-- The Studio puts this mod first in the load order while the player measures. The game runs
-- every mod's shared Lua before any mod's client Lua, and the mods in their load order, so from
-- here on every Events.X.Add of every other mod goes through the Add below: the handler is
-- wrapped in one that clocks it, keyed by the file and first line of the function.
--
-- The game gives Lua one clock, in milliseconds (getTimestampMs). Most handlers take far less;
-- a call that straddles a millisecond tick counts 1 and one that does not counts 0, so over
-- thousands of calls the sum is the real time spent (the chance of straddling a tick is the
-- fraction of a millisecond the call takes).
--
-- Every few seconds the totals go to Zomboid/Lua/pz_profile.json, which the Studio reads. Every
-- 2 s a short live sample goes to Zomboid/Lua/pz_profile_live.json (overwritten, never growing):
-- each mod's Lua time over the last 10 s, and the frames of those 10 s, for the Studio's Live
-- panel and the Studio Link's overlay (which reads PZStudioProfiler.live in the game itself).

if PZStudioProfiler then return end

local now = getTimestampMs
local fileOf = getFilenameOfClosure
local lineOf = getFirstLineOfClosure

PZStudioProfiler = {
    version = 4,
    since = now(),
    handlers = {},   -- key -> { file, line, event, ms, calls, worst }
    order = {},      -- keys in the order first seen, to write a stable file
    ticks = 0,
    wrapped = 0,
    -- Frames in the save: how long each took, in 1 ms steps up to 200 (longer ones count in
    -- the last step), how many went over 25, 50 and 100 ms, and the chunks and squares the
    -- game loaded meanwhile. The Studio turns the steps into percentiles and the 1% low.
    frames = { count = 0, sum = 0, max = 0, over25 = 0, over50 = 0, over100 = 0, chunks = 0, squares = 0, hist = {} },
}
local P = PZStudioProfiler
local HIST_STEPS = 200
for i = 1, HIST_STEPS do P.frames.hist[i] = 0 end

-- One frame of `d` ms. A frame over 5 s is the game stopped (a loading screen, alt-tab), not
-- a frame, and is left out.
local live -- the live sample (below), counted here too

local function frameTook(d)
    if d <= 0 or d > 5000 then return end
    live.count = live.count + 1
    live.sum = live.sum + d
    if d > live.max then live.max = d end
    local f = P.frames
    f.count = f.count + 1
    f.sum = f.sum + d
    if d > f.max then f.max = d end
    if d > 25 then f.over25 = f.over25 + 1 end
    if d > 50 then f.over50 = f.over50 + 1 end
    if d > 100 then f.over100 = f.over100 + 1 end
    local step = math.ceil(d)
    if step > HIST_STEPS then step = HIST_STEPS end
    f.hist[step] = f.hist[step] + 1
end

-- Where a function was written, or nil for one the game made (a Java function, say).
local function whereOf(fn)
    local ok, file = pcall(fileOf, fn)
    if not ok or not file then return nil end
    local okLine, line = pcall(lineOf, fn)
    return file, (okLine and line) or 0
end

local function statFor(event, file, line)
    local key = event .. "|" .. file .. "|" .. tostring(line)
    local s = P.handlers[key]
    if not s then
        s = { file = file, line = line, event = event, ms = 0, calls = 0, worst = 0, mark = 0, callMark = 0, slotWorst = 0 }
        P.handlers[key] = s
        table.insert(P.order, key)
    end
    return s
end

-- A mod that wraps other mods' handlers before adding them (the Studio's own Polyfills does,
-- in a pcall) registers wrapper -> inner function here, so the time goes to the inner one's mod.
PZStudioInnerOf = PZStudioInnerOf or {}

-- Each measured function counts its own time only: what other measured functions took inside it
-- (a mod's render that calls the game's render it replaced, an event that opens a window) is
-- theirs. `inner` adds up the measured time spent inside the call running now; a call that
-- throws leaves it off, so every frame starts it again from 0 (see everyFrame).
local inner = 0

local function timed(s, fn)
    return function(...)
        local outer = inner
        inner = 0
        local t = now()
        local a, b = fn(...)
        local d = now() - t
        local own = d - inner
        inner = outer + d
        s.ms = s.ms + own
        s.calls = s.calls + 1
        if own > s.slotWorst then
            s.slotWorst = own
            if own > s.worst then s.worst = own end
        end
        return a, b
    end
end

local function wrap(event, fn)
    local file, line = whereOf(PZStudioInnerOf[fn] or fn)
    if not file then return nil end
    P.wrapped = P.wrapped + 1
    return timed(statFor(event, file, line), fn)
end

-- ---------------------------------------------------------------- what windows draw

-- Windows and bars are drawn by the game calling their class's render / prerender, and timed
-- actions run their update every tick, outside any event. Classes are global tables made with
-- derive (they carry a Type); their methods are looked up through the class at every call, so
-- swapping the class's function for a clocked one covers every window of that class. The globals
-- are looked at again every few seconds: a mod that replaces a method later (or a class made
-- after the first look) gets its own clock, and the function it replaced keeps the one it had.
local METHODS = { "render", "prerender", "update" }
local clocked = {} -- the clocked functions, so they are not clocked twice

function P.sweepClasses()
    local t = now()
    local count = 0
    for name, cls in pairs(_G) do
        if type(cls) == "table" and type(rawget(cls, "Type")) == "string" then
            for _, m in ipairs(METHODS) do
                local fn = rawget(cls, m)
                if type(fn) == "function" and not clocked[fn] then
                    local file, line = whereOf(fn)
                    if file then
                        local w = timed(statFor(tostring(name) .. ":" .. m, file, line), fn)
                        clocked[w] = true
                        rawset(cls, m, w)
                        count = count + 1
                    end
                end
            end
        end
    end
    P.classMethods = (P.classMethods or 0) + count
    P.lastSweepMs = now() - t
end

-- Each event's Add and Remove, rerouted through a wrapper per function. Remove finds the wrapper
-- of the function it is given, so a mod that takes its handler out still does.
local function hook(event, ev)
    local add, remove = ev.Add, ev.Remove
    if type(add) ~= "function" or type(remove) ~= "function" then return end
    local wrappers = {}
    ev.Add = function(fn)
        if type(fn) ~= "function" then return add(fn) end
        local w = wrappers[fn]
        if not w then
            w = wrap(event, fn)
            if not w then return add(fn) end
            wrappers[fn] = w
        end
        return add(w)
    end
    ev.Remove = function(fn)
        return remove(wrappers[fn] or fn)
    end
    -- The original, for this file's own handlers, which are not measured.
    ev.PZStudioRawAdd = add
end

for name, ev in pairs(Events) do
    if type(ev) == "table" then hook(name, ev) end
end

-- ---------------------------------------------------------------- the file the Studio reads

local function quote(s)
    s = tostring(s):gsub("\\", "\\\\"):gsub("\"", "\\\"")
    return "\"" .. s .. "\""
end

function P.write()
    local w = getFileWriter("pz_profile.json", true, false)
    if not w then return end
    local t = now()
    w:write("{\"version\":" .. P.version
        .. ",\"since\":" .. string.format("%d", P.since)
        .. ",\"now\":" .. string.format("%d", t)
        .. ",\"in_game\":" .. tostring(P.inGame == true)
        .. ",\"ticks\":" .. P.ticks
        .. ",\"sweep_ms\":" .. string.format("%d", P.lastSweepMs or 0)
        .. ",\"handlers\":[")
    local first = true
    for _, key in ipairs(P.order) do
        local s = P.handlers[key]
        if s.calls > 0 then
            w:write((first and "" or ",")
                .. "{\"file\":" .. quote(s.file)
                .. ",\"line\":" .. s.line
                .. ",\"event\":" .. quote(s.event)
                .. ",\"ms\":" .. string.format("%d", s.ms)
                .. ",\"calls\":" .. s.calls
                .. ",\"worst\":" .. string.format("%d", s.worst) .. "}")
            first = false
        end
    end
    local f = P.frames
    w:write("],\"frames\":{\"count\":" .. f.count
        .. ",\"sum\":" .. string.format("%d", f.sum)
        .. ",\"max\":" .. string.format("%d", f.max)
        .. ",\"over25\":" .. f.over25 .. ",\"over50\":" .. f.over50 .. ",\"over100\":" .. f.over100
        .. ",\"chunks\":" .. f.chunks .. ",\"squares\":" .. f.squares
        .. ",\"hist\":[" .. table.concat(f.hist, ",") .. "]}}")
    w:close()
end

-- ---------------------------------------------------------------- the live sample

-- Every LIVE_EVERY ms what each handler took since the last look goes to its mod's slot; the
-- last LIVE_SLOTS slots are the window (10 s). Per frame only frameTook's three numbers; the rest
-- runs once per sample. A handler's mod is the active mod whose folder holds its file (the
-- longest folder wins), found once per file.
local LIVE_EVERY = 2000
local LIVE_SLOTS = 5
local LIVE_FILE = "pz_profile_live.json"
local LIVE_WRITTEN = 10 -- mods in the file

live = {
    version = 1,
    slot = 0,
    count = 0, sum = 0, max = 0,         -- frames of the slot being filled
    fCount = {}, fSum = {}, fMax = {}, slotMs = {}, cost = {},
    owners = {}, ownerList = {}, fileOwner = {}, dirs = {}, dirsFor = -1,
    top = {}, topN = 0,                  -- the mods by time in the window, the most first
    windowMs = 0, totalMs = 0, frames = 0, frameSum = 0, frameMax = 0, costMs = 0, at = 0,
}
P.live = live
for i = 1, LIVE_SLOTS do
    live.fCount[i], live.fSum[i], live.fMax[i], live.slotMs[i], live.cost[i] = 0, 0, 0, 0, 0
end

local function ownerNamed(id, name)
    local o = live.owners[id]
    if not o then
        o = { id = id, name = name, ms = {}, calls = {}, worst = {}, win = 0, winCalls = 0, winWorst = 0 }
        for i = 1, LIVE_SLOTS do o.ms[i], o.calls[i], o.worst[i] = 0, 0, 0 end
        live.owners[id] = o
        table.insert(live.ownerList, o)
    end
    return o
end
local gameOwner = ownerNamed("", "")

local function norm(path)
    return string.lower((string.gsub(tostring(path), "\\", "/")))
end

--- The folders of the mods that are on, made again when the list changes.
local function refreshDirs()
    local okList, list = pcall(getActivatedMods)
    if not okList or not list then return end
    local n = list:size()
    if n == live.dirsFor then return end
    live.dirsFor = n
    local dirs = {}
    for i = 0, n - 1 do
        local id = tostring(list:get(i))
        local ok, info = pcall(getModInfoByID, id)
        if ok and info then
            local okDir, dir = pcall(function() return info:getDir() end)
            local okName, name = pcall(function() return info:getName() end)
            local okId, realId = pcall(function() return info:getId() end)
            if okDir and dir then
                local d = norm(dir)
                if string.sub(d, -1) ~= "/" then d = d .. "/" end
                table.insert(dirs, { dir = d, len = string.len(d), id = (okId and realId) and tostring(realId) or id, name = (okName and name) and tostring(name) or id })
            end
        end
    end
    table.sort(dirs, function(a, b) return a.len > b.len end)
    live.dirs = dirs
    live.fileOwner = {}
    for _, s in pairs(P.handlers) do s.owner = nil end
end

local function ownerOf(file)
    local o = live.fileOwner[file]
    if o then return o end
    local f = norm(file)
    o = gameOwner
    for _, d in ipairs(live.dirs) do
        if string.sub(f, 1, d.len) == d.dir then
            o = ownerNamed(d.id, d.name)
            break
        end
    end
    live.fileOwner[file] = o
    return o
end

local function clean(s)
    return (string.gsub(tostring(s), "%c", " "))
end

local function writeLive(t)
    local w = getFileWriter(LIVE_FILE, true, false)
    if not w then return end
    w:write("{\"version\":" .. live.version
        .. ",\"now\":" .. string.format("%d", t)
        .. ",\"in_game\":" .. tostring(P.inGame == true)
        .. ",\"window_ms\":" .. string.format("%d", live.windowMs)
        .. ",\"frames\":{\"count\":" .. string.format("%d", live.frames)
        .. ",\"sum\":" .. string.format("%d", live.frameSum)
        .. ",\"max\":" .. string.format("%d", live.frameMax) .. "}"
        .. ",\"total_ms\":" .. string.format("%d", live.totalMs)
        .. ",\"cost_ms\":" .. string.format("%d", live.costMs)
        .. ",\"handlers\":" .. #P.order
        .. ",\"mods\":[")
    local n = math.min(live.topN, LIVE_WRITTEN)
    for i = 1, n do
        local o = live.top[i]
        w:write((i > 1 and "," or "")
            .. "{\"id\":" .. quote(clean(o.id))
            .. ",\"name\":" .. quote(clean(o.name))
            .. ",\"ms\":" .. string.format("%d", o.win)
            .. ",\"calls\":" .. string.format("%d", o.winCalls)
            .. ",\"worst\":" .. string.format("%d", o.winWorst) .. "}")
    end
    w:write("]}")
    w:close()
end

local function byTime(a, b)
    if a.win ~= b.win then return a.win > b.win end
    return a.winWorst > b.winWorst
end

--- One sample: the slot just ended goes in, the window is added up, the file written. Its own
--- time (live.cost, live.costMs over the window) is said in the file and on the overlay.
function P.liveSample(t)
    local started = now()
    refreshDirs()
    live.slot = live.slot % LIVE_SLOTS + 1
    local k = live.slot
    for _, o in ipairs(live.ownerList) do
        o.ms[k], o.calls[k], o.worst[k] = 0, 0, 0
    end
    for _, key in ipairs(P.order) do
        local s = P.handlers[key]
        local dc = s.calls - s.callMark
        if dc > 0 then
            local o = s.owner
            if not o then
                o = ownerOf(s.file)
                s.owner = o
            end
            o.ms[k] = o.ms[k] + (s.ms - s.mark)
            o.calls[k] = o.calls[k] + dc
            if s.slotWorst > o.worst[k] then o.worst[k] = s.slotWorst end
            s.mark, s.callMark = s.ms, s.calls
        end
        s.slotWorst = 0
    end
    live.fCount[k], live.fSum[k], live.fMax[k] = live.count, live.sum, live.max
    live.count, live.sum, live.max = 0, 0, 0
    live.slotMs[k] = (live.at > 0) and (t - live.at) or 0
    live.at = t

    local windowMs, frames, frameSum, frameMax, cost = 0, 0, 0, 0, 0
    for i = 1, LIVE_SLOTS do
        windowMs = windowMs + live.slotMs[i]
        frames = frames + live.fCount[i]
        frameSum = frameSum + live.fSum[i]
        if live.fMax[i] > frameMax then frameMax = live.fMax[i] end
        if i ~= k then cost = cost + live.cost[i] end
    end
    live.windowMs, live.frames, live.frameSum, live.frameMax = windowMs, frames, frameSum, frameMax
    local top, n, total = live.top, 0, 0
    for _, o in ipairs(live.ownerList) do
        local ms, calls, worst = 0, 0, 0
        for i = 1, LIVE_SLOTS do
            ms = ms + o.ms[i]
            calls = calls + o.calls[i]
            if o.worst[i] > worst then worst = o.worst[i] end
        end
        o.win, o.winCalls, o.winWorst = ms, calls, worst
        if calls > 0 then
            n = n + 1
            top[n] = o
            total = total + ms
        end
    end
    for i = n + 1, live.topN do top[i] = nil end
    live.topN = n
    live.totalMs = total
    table.sort(top, byTime)
    live.cost[k] = now() - started
    live.costMs = cost + live.cost[k]
    writeLive(t)
end

--- A new start (the save entered): the live window starts empty too.
local function liveReset()
    for _, s in pairs(P.handlers) do s.mark, s.callMark, s.slotWorst = 0, 0, 0 end
    for _, o in ipairs(live.ownerList) do
        for i = 1, LIVE_SLOTS do o.ms[i], o.calls[i], o.worst[i] = 0, 0, 0 end
        o.win, o.winCalls, o.winWorst = 0, 0, 0
    end
    for i = 1, LIVE_SLOTS do
        live.fCount[i], live.fSum[i], live.fMax[i], live.slotMs[i], live.cost[i] = 0, 0, 0, 0, 0
    end
    live.count, live.sum, live.max = 0, 0, 0
    for i = 1, live.topN do live.top[i] = nil end
    live.topN, live.totalMs, live.windowMs, live.frames, live.frameSum, live.frameMax, live.costMs = 0, 0, 0, 0, 0, 0, 0
    live.at = now()
end

-- Starts the count again: what the main menu cost is not what playing costs.
function P.reset(inGame)
    for _, s in pairs(P.handlers) do
        s.ms, s.calls, s.worst = 0, 0, 0
    end
    P.since = now()
    P.ticks = 0
    P.inGame = inGame
    local f = P.frames
    f.count, f.sum, f.max, f.over25, f.over50, f.over100, f.chunks, f.squares = 0, 0, 0, 0, 0, 0, 0, 0
    for i = 1, HIST_STEPS do f.hist[i] = 0 end
    liveReset()
end

local lastWrite = now()
local lastLive = now()
local lastSweep = 0
local lastFrame = nil
local function everyFrame()
    inner = 0
    local t = now()
    if P.inGame then
        P.ticks = P.ticks + 1
        if lastFrame then frameTook(t - lastFrame) end
    end
    lastFrame = t
    if t - lastSweep >= 10000 then
        lastSweep = t
        P.sweepClasses()
    end
    if t - lastWrite >= 5000 then
        lastWrite = t
        P.write()
    end
    if t - lastLive >= LIVE_EVERY then
        lastLive = t
        P.liveSample(t)
    end
end

-- Registered with the game's own Add, so the profiler does not measure itself.
Events.OnFETick.PZStudioRawAdd(everyFrame)
Events.OnTickEvenPaused.PZStudioRawAdd(everyFrame)
Events.OnGameBoot.PZStudioRawAdd(function() P.sweepClasses() end)
Events.OnGameStart.PZStudioRawAdd(function() P.sweepClasses(); P.reset(true); P.write() end)
-- What the game loaded while the frames were counted: the map work behind a hitch that is
-- nobody's Lua. Counted only, never timed.
if Events.LoadChunk and Events.LoadChunk.PZStudioRawAdd then
    Events.LoadChunk.PZStudioRawAdd(function() if P.inGame then P.frames.chunks = P.frames.chunks + 1 end end)
end
if Events.LoadGridsquare and Events.LoadGridsquare.PZStudioRawAdd then
    Events.LoadGridsquare.PZStudioRawAdd(function() if P.inGame then P.frames.squares = P.frames.squares + 1 end end)
end
