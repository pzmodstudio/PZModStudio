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
-- Every few seconds the totals go to Zomboid/Lua/pz_profile.json, which the Studio reads.

if PZStudioProfiler then return end

local now = getTimestampMs
local fileOf = getFilenameOfClosure
local lineOf = getFirstLineOfClosure

PZStudioProfiler = {
    version = 3,
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
local function frameTook(d)
    if d <= 0 or d > 5000 then return end
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
        s = { file = file, line = line, event = event, ms = 0, calls = 0, worst = 0 }
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
        if own > s.worst then s.worst = own end
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
end

local lastWrite = now()
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
