-- PZ Mod Studio Link: a small live overlay while playing, and the same numbers for the app.
--
-- Shown and hidden from the pause menu ("Live overlay") or with a key the player chooses in the
-- game's Options > Mods > PZ Mod Studio Link (none by default, so it never takes one of the
-- game's). It says, refreshed twice a second:
--   - FPS and the average frame time of the last 2 s, the longest frame of the last 10 s;
--   - the memory Java uses (the game's own count, collectgarbage("count")) against its limit
--     (-Xmx, which the app tells: Lua cannot read it), and the CPU, while PZ Mod Studio is open;
--   - the 5 mods whose Lua took the most time over the last 10 s, from the Studio's profiler
--     (PZModStudio_Profiler, PZStudioProfiler.live) when it is on. When it is off nothing is
--     timed here: it says how to turn it on;
--   - the Lua errors of this session and the mods the game blamed (PZModStudio_Errors);
--   - its own cost: the time its own code took over the last 10 s, per second of play.
--
-- Cheap: per frame two clock reads and a few sums; the texts are made only on refresh, and drawn
-- as they are. Hidden, with the app closed, it only reads the clock once a frame.
--
-- Files in Zomboid/Lua:
--   PZModStudio_LinkLive.txt     this: every 2 s while in a game and the app runs: at=<ms>,
--                                       fps, frame_ms, worst_ms, frames, heap_mb, heap_total_mb,
--                                       errors, err=<count>\t<mod> per mod blamed, overlay=0|1,
--                                       cost_ms, window_ms, profiler=0|1 (overwritten)
--   PZModStudio_LinkMeter.txt    app:  at=<unix s>, ram_mb, ram_max_mb (the game's -Xmx), cpu
--   PZModStudio_LinkOverlay.txt  this: shown=0|1, x, y: where the player left it

require "PZModStudio_Link"
require "PZModStudio_Errors"
require "ISUI/ISPanel"
require "ISUI/ISButton"
require "OptionScreens/MainScreen"

local Link = PZModStudioLink
local text = Link.text

local LIVE_FILE = "PZModStudio_LinkLive.txt"
local METER_FILE = "PZModStudio_LinkMeter.txt"
local STATE_FILE = "PZModStudio_LinkOverlay.txt"
local SLOT_MS = 500        -- one slot of frames; the overlay refreshes as each one ends
local SLOTS = 20           -- 10 s of slots
local FPS_SLOTS = 4        -- FPS and frame time: the last 2 s
local WRITE_MS = 2000      -- the app's file
local METER_SECONDS = 10   -- the app's meter, older than this, is not said
local TOP = 5
local OPTIONS_ID = "Z_StudioLink"

local now = getTimestampMs

local Live = { shown = false, costCalls = 0 }
Link.Live = Live

-- ------------------------------------------------------------------ the frames

local fm = { last = 0, count = 0, sum = 0, max = 0, cost = 0, slot = 1, slotStart = 0, writeAt = 0,
    sCount = {}, sSum = {}, sMax = {}, sCost = {}, sMs = {} }
for i = 1, SLOTS do fm.sCount[i], fm.sSum[i], fm.sMax[i], fm.sCost[i], fm.sMs[i] = 0, 0, 0, 0, 0 end

local function clearFrames()
    fm.last, fm.count, fm.sum, fm.max, fm.cost = 0, 0, 0, 0, 0
    for i = 1, SLOTS do fm.sCount[i], fm.sSum[i], fm.sMax[i], fm.sCost[i], fm.sMs[i] = 0, 0, 0, 0, 0 end
    fm.slotStart = now()
end

--- The window's numbers: fps and frame ms of the last 2 s, worst frame, frames, own cost and
--- real time of the last 10 s.
function Live.numbers()
    local n, sum, worst, frames, cost, window = 0, 0, 0, 0, 0, 0
    for back = 0, SLOTS - 1 do
        -- The slots already closed, newest first. No % on a negative number: Kahlua's follows
        -- Java's sign, not Lua's.
        local i = fm.slot - 1 - back
        if i < 1 then i = i + SLOTS end
        if back < FPS_SLOTS then
            n = n + fm.sCount[i]
            sum = sum + fm.sSum[i]
        end
        if fm.sMax[i] > worst then worst = fm.sMax[i] end
        frames = frames + fm.sCount[i]
        cost = cost + fm.sCost[i]
        window = window + fm.sMs[i]
    end
    local fps = (sum > 0) and (n * 1000 / sum) or 0
    local frameMs = (n > 0) and (sum / n) or 0
    return fps, frameMs, worst, frames, cost, window
end

-- ------------------------------------------------------------------ what else it says

--- The memory Java holds now, in MB: used and reserved (the game's Kahlua answers
--- collectgarbage("count") with used, free and total, in KB). nil when it does not.
function Live.heap()
    local ok, used, free, total = pcall(collectgarbage, "count")
    if not ok or type(used) ~= "number" then return nil, nil end
    return used / 1024, (type(total) == "number") and (total / 1024) or nil
end

--- What the app last said of the game's process, while it is recent.
local meter = { readAt = 0 }
function Live.meter()
    local t = Link.now()
    if t - meter.readAt < 2 then return meter.fresh and meter or nil end
    meter.readAt = t
    meter.fresh = false
    local file = getFileReader(METER_FILE, false)
    if not file then return nil end
    local line = file:readLine()
    while line ~= nil do
        local key, value = string.match(line, "^([%w_]+)=(.*)$")
        if key then meter[key] = tonumber(value) end
        line = file:readLine()
    end
    file:close()
    meter.fresh = meter.at ~= nil and math.abs(t - meter.at) <= METER_SECONDS
    return meter.fresh and meter or nil
end

--- The profiler's live window, when it runs in this game: nil when it is off.
local function profilerLive()
    local P = PZStudioProfiler
    return P and P.live or nil
end

--- The session's errors by the mod the game blamed, the most first: { { mod, count }, ... }.
local errorsSeen = { total = -1, list = {} }
function Live.errorsByMod()
    local Errors = Link.Errors
    if not Errors then return 0, errorsSeen.list end
    -- Every 2 s even when the count stayed: an error's stack (its mod) can come a frame later.
    local t = Link.now()
    if Errors.total == errorsSeen.total and errorsSeen.at and t - errorsSeen.at < 2 then return Errors.total, errorsSeen.list end
    errorsSeen.total, errorsSeen.at = Errors.total, t
    local byMod, list = {}, {}
    for _, err in ipairs(Errors.history or {}) do
        local key = err.mod or ""
        local g = byMod[key]
        if not g then
            g = { mod = err.mod, count = 0 }
            byMod[key] = g
            table.insert(list, g)
        end
        g.count = g.count + 1
    end
    table.sort(list, function(a, b) return a.count > b.count end)
    errorsSeen.list = list
    return Errors.total, list
end

--- A whole number as text (rounded), as string.format("%d") would want it.
local function whole(x)
    return string.format("%d", math.floor(x + 0.5))
end

local function oneDecimal(x)
    return string.format("%.1f", x)
end

-- ------------------------------------------------------------------ the overlay

local Overlay = ISPanel:derive("StudioLinkLiveOverlay")

function Overlay:new(x, y)
    local o = ISPanel.new(self, x, y, 280, 120)
    o.backgroundColor = { r = 0.03, g = 0.04, b = 0.06, a = 0.72 }
    o.borderColor = { r = 0.35, g = 0.75, b = 1.0, a = 0.55 }
    o.moveWithMouse = true
    o.lines = {}
    o.n = 0
    return o
end

--- Sets line i (made once, kept): left text, right text (or nil), colour.
function Overlay:setLine(i, left, right, r, g, b)
    local l = self.lines[i]
    if not l then
        l = {}
        self.lines[i] = l
    end
    l.left, l.right, l.r, l.g, l.b = left, right, r, g, b
end

function Overlay:render()
    local started = now()
    local font = UIFont.Small
    local h = self.lineH or 14
    local y = 6
    for i = 1, self.n do
        local l = self.lines[i]
        self:drawText(l.left, 8, y, l.r, l.g, l.b, 1, font)
        if l.right then self:drawTextRight(l.right, self.width - 8, y, l.r, l.g, l.b, 1, font) end
        y = y + h
    end
    fm.cost = fm.cost + (now() - started)
end

function Overlay:onMouseUp(x, y)
    ISPanel.onMouseUp(self, x, y)
    Live.saveState()
end

function Overlay:onMouseUpOutside(x, y)
    ISPanel.onMouseUpOutside(self, x, y)
    Live.saveState()
end

local function fit(s, width)
    local tm = getTextManager()
    if tm:MeasureStringX(UIFont.Small, s) <= width then return s end
    while string.len(s) > 1 and tm:MeasureStringX(UIFont.Small, s .. "...") > width do s = string.sub(s, 1, -2) end
    return s .. "..."
end

local NAME_WIDTH = 190
local DIM, WHITE, AMBER, RED, BLUE = { 0.6, 0.65, 0.72 }, { 0.93, 0.95, 0.97 }, { 1.0, 0.78, 0.35 }, { 0.97, 0.5, 0.5 }, { 0.5, 0.8, 1.0 }

--- The texts made again from the numbers: twice a second while it shows.
function Overlay:refresh()
    local i = 0
    local function line(left, right, c)
        i = i + 1
        self:setLine(i, left, right, c[1], c[2], c[3])
    end
    local fps, frameMs, worst, _, cost, window = Live.numbers()
    line(text("liveTitle"), nil, BLUE)
    line(text("liveFps", whole(fps), oneDecimal(frameMs), whole(worst)), nil, (fps > 0 and fps < 30) and AMBER or WHITE)

    local used, total = Live.heap()
    local m = Live.meter()
    if used then
        local limit = m and m.ram_max_mb
        if limit and limit > 0 then
            line(text("liveMem", oneDecimal(used / 1024), oneDecimal(limit / 1024)), nil, (used / limit > 0.85) and AMBER or WHITE)
        else
            line(text("liveMemNoLimit", oneDecimal(used / 1024), oneDecimal((total or used) / 1024)), nil, WHITE)
        end
    elseif m and m.ram_mb then
        line(text("liveMemProcess", oneDecimal(m.ram_mb / 1024)), nil, WHITE)
    end
    if m and m.cpu then
        line(text("liveCpu", whole(m.cpu) .. " %"), nil, WHITE)
    elseif not Link.appAlive() then
        line(text("liveNoApp"), nil, DIM)
    end

    local P = profilerLive()
    if not P then
        line(text("liveLuaOff1"), nil, DIM)
        line(text("liveLuaOff2"), nil, DIM)
    elseif (P.windowMs or 0) <= 0 then
        line(text("liveLuaWaiting"), nil, DIM)
    else
        local seconds = P.windowMs / 1000
        line(text("liveLuaTitle", oneDecimal(P.totalMs / seconds), oneDecimal(P.totalMs / P.windowMs * 100) .. " %"), nil, BLUE)
        local shown = math.min(P.topN or 0, TOP)
        for k = 1, shown do
            local o = P.top[k]
            local name = (o.id == "" and text("liveGame")) or o.name or o.id
            local perS = o.win / seconds
            line("  " .. fit(name, NAME_WIDTH), text("livePerSecond", oneDecimal(perS)), (perS >= 5 or o.winWorst >= 100) and AMBER or WHITE)
        end
        if shown == 0 then line(text("liveLuaNone"), nil, DIM) end
    end

    local count, byMod = Live.errorsByMod()
    if count > 0 then
        line(text("liveErrors", count), nil, RED)
        for k = 1, math.min(#byMod, 2) do
            local g = byMod[k]
            line("  " .. fit(g.mod or text("liveErrorGame"), NAME_WIDTH), "x" .. g.count, RED)
        end
    else
        line(text("liveErrorsNone"), nil, DIM)
    end

    local perS = (window > 0) and (cost * 1000 / window) or 0
    line(text("liveCost", string.format("%.2f", perS), string.format("%.3f", perS / 10) .. " %"), nil, DIM)

    for k = i + 1, self.n do self.lines[k] = nil end
    self.n = i
    -- Its size follows what it says.
    local tm = getTextManager()
    self.lineH = tm:getFontHeight(UIFont.Small) + 1
    local w = 200
    for k = 1, i do
        local l = self.lines[k]
        local lw = tm:MeasureStringX(UIFont.Small, l.left) + (l.right and (tm:MeasureStringX(UIFont.Small, l.right) + 16) or 0)
        if lw > w then w = lw end
    end
    self:setWidth(w + 16)
    self:setHeight(i * self.lineH + 12)
end

-- ------------------------------------------------------------------ shown, hidden, kept

function Live.saveState()
    local panel = Live.panel
    if panel then Live.x, Live.y = panel:getX(), panel:getY() end
    local w = getFileWriter(STATE_FILE, true, false)
    if not w then return end
    w:write("shown=" .. (Live.shown and "1" or "0") .. "\n")
    if Live.x and Live.y then
        w:write("x=" .. whole(Live.x) .. "\n" .. "y=" .. whole(Live.y) .. "\n")
    end
    w:close()
end

local function readState()
    local file = getFileReader(STATE_FILE, false)
    if not file then return end
    local line = file:readLine()
    while line ~= nil do
        local key, value = string.match(line, "^(%a+)=(.*)$")
        if key == "shown" then Live.shown = value == "1" end
        if key == "x" then Live.x = tonumber(value) end
        if key == "y" then Live.y = tonumber(value) end
        line = file:readLine()
    end
    file:close()
end

local function onScreen(x, y)
    local sw, sh = getCore():getScreenWidth(), getCore():getScreenHeight()
    return x and y and x >= 0 and y >= 0 and x < sw - 40 and y < sh - 40
end

function Live.open()
    if Live.panel then return end
    local x, y = Live.x, Live.y
    if not onScreen(x, y) then x, y = 80, 30 end
    local panel = Overlay:new(x, y)
    panel:initialise()
    panel:addToUIManager()
    panel:setAlwaysOnTop(true)
    Live.panel = panel
    panel:refresh()
end

function Live.close()
    local panel = Live.panel
    if not panel then return end
    Live.x, Live.y = panel:getX(), panel:getY()
    panel:setVisible(false)
    panel:removeFromUIManager()
    Live.panel = nil
end

function Live.toggle()
    Live.shown = not Live.shown
    if Live.shown and Link.inGame then Live.open() else Live.close() end
    Live.saveState()
end

-- ------------------------------------------------------------------ the app's file

local function clean(s)
    return (string.gsub(tostring(s), "[%c]", " "))
end

function Live.write(t)
    local fps, frameMs, worst, frames, cost, window = Live.numbers()
    local used, total = Live.heap()
    local count, byMod = Live.errorsByMod()
    local w = getFileWriter(LIVE_FILE, true, false)
    if not w then return end
    local version = ""
    pcall(function() version = getModInfoByID(Link.SELF_ID):getModVersion() or "" end)
    w:write("at=" .. whole(t) .. "\n")
    w:write("link=" .. tostring(version) .. "\n")
    w:write("fps=" .. oneDecimal(fps) .. "\n")
    w:write("frame_ms=" .. oneDecimal(frameMs) .. "\n")
    w:write("worst_ms=" .. whole(worst) .. "\n")
    w:write("frames=" .. whole(frames) .. "\n")
    if used then w:write("heap_mb=" .. whole(used) .. "\n") end
    if total then w:write("heap_total_mb=" .. whole(total) .. "\n") end
    w:write("errors=" .. whole(count) .. "\n")
    for k = 1, math.min(#byMod, 10) do
        w:write("err=" .. byMod[k].count .. "\t" .. clean(byMod[k].mod or "") .. "\n")
    end
    w:write("overlay=" .. (Live.panel and "1" or "0") .. "\n")
    w:write("cost_ms=" .. whole(cost) .. "\n")
    w:write("window_ms=" .. whole(window) .. "\n")
    w:write("profiler=" .. (profilerLive() and "1" or "0") .. "\n")
    w:close()
end

-- ------------------------------------------------------------------ every frame

--- The meter runs in a game while the overlay shows or the app is there to read it.
local function onFrame()
    if not Link.inGame then return end
    local t = now()
    if fm.slotStart == 0 then fm.slotStart = t end
    local measuring = Live.panel ~= nil or Live.appThere
    if measuring and fm.last > 0 then
        local d = t - fm.last
        if d > 0 and d <= 5000 then
            fm.count = fm.count + 1
            fm.sum = fm.sum + d
            if d > fm.max then fm.max = d end
        end
    end
    fm.last = t
    if t - fm.slotStart < SLOT_MS then
        fm.cost = fm.cost + (now() - t)
        return
    end
    -- A slot ended: it goes in the ring, the overlay says it, the app's file every 2 s.
    local k = fm.slot
    fm.sCount[k], fm.sSum[k], fm.sMax[k], fm.sMs[k] = fm.count, fm.sum, fm.max, t - fm.slotStart
    fm.count, fm.sum, fm.max = 0, 0, 0
    fm.slot = k % SLOTS + 1
    fm.slotStart = t
    Live.appThere = Link.appAlive()
    if Live.panel then Live.panel:refresh() end
    if Live.appThere and t - fm.writeAt >= WRITE_MS then
        fm.writeAt = t
        Live.write(t)
    end
    -- Its own time goes in the slot it was spent in (the one just started).
    fm.sCost[fm.slot] = 0
    fm.sCost[k] = fm.sCost[k] + fm.cost
    fm.cost = now() - t
end

local function safeFrame()
    local ok, err = pcall(onFrame)
    if not ok and not Live.failed then
        Live.failed = true
        print("[StudioLink] the live overlay failed: " .. tostring(err))
    end
end

-- ------------------------------------------------------------------ the key and the pause menu

local function addOptions()
    if not (PZAPI and PZAPI.ModOptions and PZAPI.ModOptions.create) then return end
    local options = PZAPI.ModOptions:create(OPTIONS_ID, "PZ Mod Studio Link")
    -- No key by default (0): the player picks one that is free.
    Live.keyOption = options:addKeyBind("liveOverlayKey", text("liveKeyName"), 0, text("liveKeyTip"))
end
pcall(addOptions)

local function onKey(key)
    if not Link.inGame or not key or key == 0 then return end
    local option = Live.keyOption
    local chosen = option and option.getValue and option:getValue()
    if chosen and chosen ~= 0 and key == chosen then Live.toggle() end
end

--- "Live overlay" on the game's pause menu, left of "Mark this moment".
function Live.menuButton(screen)
    local button = Live.menuBtn
    if not Link.inGame then
        if button then button:setVisible(false) end
        return
    end
    if not button or button.screen ~= screen then
        button = ISButton:new(0, 0, 100, 22, "", screen, function() Live.toggle() end)
        button:initialise()
        button:instantiate()
        button:setFont(UIFont.Small)
        button.borderColor = { r = 0.35, g = 0.75, b = 1.0, a = 0.9 }
        button.screen = screen
        screen:addChild(button)
        Live.menuBtn = button
    end
    local label = Live.shown and text("liveHide") or text("liveShow")
    local w = math.max(getTextManager():MeasureStringX(UIFont.Small, text("liveHide")), getTextManager():MeasureStringX(UIFont.Small, text("liveShow"))) + 18
    local mark = Link.markBtn
    local beside = screen.reportBug
    local h = beside and beside:getHeight() or 22
    local x, y
    if mark and mark:isVisible() then
        x, y = mark:getX() - w - 8, mark:getY()
    else
        x = beside and (beside:getX() + beside:getWidth() - w) or (screen.width - w - 20)
        y = beside and (beside:getY() - 2 * (h + 6)) or (screen.height - 120)
    end
    button:setTitle(label)
    button.tooltip = text("liveTip")
    button:setWidth(w)
    button:setHeight(h)
    button:setX(x)
    button:setY(y)
    button:setVisible(true)
end

local menuPrerender = MainScreen.prerender
function MainScreen:prerender(...)
    menuPrerender(self, ...)
    local ok, err = pcall(Live.menuButton, self)
    if not ok and not Live.menuFailed then
        Live.menuFailed = true
        print("[StudioLink] the live overlay's menu button failed: " .. tostring(err))
    end
end

if Events.OnTickEvenPaused then Events.OnTickEvenPaused.Add(safeFrame) elseif Events.OnTick then Events.OnTick.Add(safeFrame) end
if Events.OnKeyPressed then Events.OnKeyPressed.Add(function(key) pcall(onKey, key) end) end
if Events.OnGameStart then
    Events.OnGameStart.Add(function()
        clearFrames()
        Live.panel = nil
        pcall(readState)
        if Live.shown then pcall(Live.open) end
    end)
end
if Events.OnMainMenuEnter then
    Events.OnMainMenuEnter.Add(function() pcall(Live.close) end)
end
-- A reload of the Lua in a game (the game's own, or this mod sent again) finds the overlay as left.
if Link.inGame then
    pcall(readState)
    if Live.shown then pcall(Live.open) end
end
