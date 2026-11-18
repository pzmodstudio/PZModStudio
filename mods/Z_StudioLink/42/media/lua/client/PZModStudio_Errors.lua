-- PZ Mod Studio Link: a notice when the game records a Lua error, naming the mod it came from.
--
-- The game keeps every Lua error it caught in a list (getLuaDebuggerErrors, with or without
-- -debug). One error leaves, in order: "-----\n<message>", a "STACK TRACE" whose every level
-- names its mod (`Lua((MOD:Name)).fn(File.lua:48)`, `Lua(Vanilla).fn(...)`), the Java exception,
-- and the stack again. The first level with a mod is the mod that failed; the mods below it
-- called it.
--
-- The notice, in the menu and in game: how many new errors, in which mod (called from which),
-- the last message, and:
--   - Copy report: the error, the mods and the stack on the clipboard, to paste where it helps;
--   - Show in Mods (menu): the Mods screen with that mod picked, to turn it off with what
--     depends on it;
--   - See in the Studio (only while PZ Mod Studio runs): its Monitor, with the error's card;
--   - Details: where it failed (file and line) and the steps that led there, in the notice
--     itself, without leaving the game;
--   - Close: gone until the next error. The session's errors are kept: "Lua errors (N)" on the
--     game's pause menu brings them back;
--   - "-" (top right): hidden as a small badge with the count, which takes no clicks around it;
--     a click on the badge shows the notice again, a drag moves it. New errors keep it a badge.
-- In game it starts left of the clock and the speed controls, not over their buttons; it and
-- the badge can be dragged anywhere, and come back where they were left.
-- With another mod that already shows these errors on, it shows nothing.
--
-- Every error is also written to Zomboid/Lua/PZModStudio_LinkErrorLog.txt within a second of
-- coming (see "kept on disk" below), so a crash, a disconnect or the game reloading its Lua does
-- not lose them; "Error log" in the list of the session's errors opens its folder.
--
-- For other mods (see "what other mods add"): PZModStudioLink.registerReport(modId, fn). What fn
-- returns goes into "Copy all", and for an error of that mod into "Copy report" and the log. The
-- debug reports mods give another error mod's report API (with that mod on) go there too.

require "PZModStudio_Link"
require "ISUI/ISPanel"
require "ISUI/ISButton"

local Link = PZModStudioLink
local text = Link.text
local plainId = Link.plainId

-- What was already read of the list, kept across Lua reloads (the list is the game's, and a
-- reload does not empty it): seen=<count>, first=<the first entry's start>, at=<when it was
-- last looked at>. A file not looked at for a while is from an earlier game: all is new.
local SEEN_FILE = "PZModStudio_LinkErrors.txt"
local SEEN_FRESH_SECONDS = 45
local SEEN_WRITE_SECONDS = 15
local CHECK_SECONDS = 1
local MAX_KEPT = 200

local Errors = { batch = {}, total = 0, history = {} }
Link.Errors = Errors

-- ------------------------------------------------------------------ reading the list

local function firstLine(s)
    return string.match(s, "^[^\r\n]*") or s
end

local function readSeen()
    local seen, first, at = nil, nil, nil
    local file = getFileReader(SEEN_FILE, false)
    if not file then return nil, nil, nil end
    local line = file:readLine()
    while line ~= nil do
        local key, value = string.match(line, "^(%a+)=(.*)$")
        if key == "seen" then seen = tonumber(value) end
        if key == "first" then first = value end
        if key == "at" then at = tonumber(value) end
        line = file:readLine()
    end
    file:close()
    return seen, first, at
end

local function writeSeen()
    local file = getFileWriter(SEEN_FILE, true, false)
    if not file then return end
    file:write("seen=" .. tostring(Errors.seen or 0) .. "\n")
    file:write("first=" .. (Errors.first or "") .. "\n")
    file:write("at=" .. tostring(Link.now()) .. "\n")
    file:close()
    Errors.wroteAt = Link.now()
end

--- The levels of a "STACK TRACE" entry: { mod = name or nil (the game's own), line = text }.
local function framesOf(entry)
    local frames = {}
    for line in string.gmatch(entry, "[^\r\n]+") do
        local trimmed = string.match(line, "^%s*(.-)%s*$")
        local mod = string.match(trimmed, "^Lua%(%(MOD:(.-)%)%)%.")
        if mod or string.find(trimmed, "^Lua%(Vanilla%)") then
            table.insert(frames, { mod = mod, line = trimmed })
        end
    end
    return frames
end

--- An error made of its entries: the mod that failed and the ones that called it.
-- The Studio's profiler wraps every handler to clock it: its frame is on the stack of each error
-- in a handler, and is never the one to blame or a caller worth naming.
local PROFILER_NAME = "PZ Mod Studio Profiler"

local function settle(err)
    if not err.frames then err.frames = {} end
    err.callers = {}
    for _, f in ipairs(err.frames) do
        if f.mod and f.mod ~= PROFILER_NAME then
            if not err.mod then
                err.mod = f.mod
            elseif f.mod ~= err.mod then
                local known = false
                for _, c in ipairs(err.callers) do
                    if c == f.mod then known = true end
                end
                if not known and #err.callers < 3 then table.insert(err.callers, f.mod) end
            end
        end
    end
end

local function add(err)
    err.frames = err.frames or {}
    settle(err)
    -- Kept even when the notice is closed: the rest of its entries may still come.
    Errors.lastRead = err
    Errors.total = Errors.total + 1
    table.insert(Errors.batch, err)
    if #Errors.batch > MAX_KEPT then table.remove(Errors.batch, 1) end
    -- Every error of the session, for the pause menu's button to show again after a Close.
    table.insert(Errors.history, err)
    if #Errors.history > MAX_KEPT then table.remove(Errors.history, 1) end
    -- On disk too, once the rest of its entries had their chance to come (Errors.logFlush).
    if Errors.logAdd then pcall(Errors.logAdd, err) end
end

--- Reads what the game added to its list since the last look. True when there is something new.
function Errors.collect()
    local ok, list = pcall(getLuaDebuggerErrors)
    if not ok or not list then return false end
    local n = list:size()
    if n == 0 then
        Errors.seen = 0
        return false
    end
    local first = string.sub(firstLine(tostring(list:get(0))), 1, 80)
    Errors.first = first
    if Errors.seen == nil then
        -- After a Lua reload the game's list still holds what was shown before it.
        local seen, firstSeen, at = readSeen()
        local fresh = at ~= nil and math.abs(Link.now() - at) <= SEEN_FRESH_SECONDS
        if fresh and seen and firstSeen == first and seen <= n then Errors.seen = seen else Errors.seen = 0 end
    end
    if n < Errors.seen then Errors.seen = 0 end
    if n == Errors.seen then return false end
    -- The game can add one error's entries over two frames, so the last error read stays open:
    -- its stack, or its Java exception, may come on the next look. An error from an event
    -- handler has no "-----" entry: it starts with its stack, and its message is the Java
    -- exception's first line. The stack written again after the exception closes the error.
    for i = Errors.seen, n - 1 do
        local entry = tostring(list:get(i))
        local last = Errors.lastRead
        if last and last.closed then last = nil end
        if string.find(entry, "^%-%-%-%-%-") then
            local message = string.match(entry, "^[^\n]*\n(.*)$") or entry
            add({ message = string.match(message, "^%s*(.-)%s*$") })
        elseif string.find(entry, "STACK TRACE", 1, true) then
            if last and #last.frames == 0 then
                last.frames = framesOf(entry)
                settle(last)
            elseif last and last.sawJava then
                last.closed = true
            else
                add({ frames = framesOf(entry) })
            end
        elseif last and (string.find(entry, "^java%.") or string.find(entry, "KahluaThread", 1, true)) then
            if not last.sawJava then
                last.sawJava = true
                if not last.message then last.message = string.gsub(firstLine(entry), "^[%w%.]+Exception: ", "") end
            end
        elseif not last or not (string.find(entry, "^java%.") or string.find(entry, "KahluaThread", 1, true)) then
            -- An entry of another shape starts an error of its own.
            add({ message = firstLine(entry) })
        end
    end
    Errors.seen = n
    pcall(writeSeen)
    return true
end

-- ------------------------------------------------------------------ what the notice says

--- The batch by mod, the one with most errors first: { { mod, count, last }, ... }.
local function byMod()
    local groups, order = {}, {}
    for _, err in ipairs(Errors.batch) do
        local key = err.mod or ""
        if not groups[key] then
            groups[key] = { mod = err.mod, count = 0 }
            table.insert(order, groups[key])
        end
        groups[key].count = groups[key].count + 1
        groups[key].last = err
    end
    table.sort(order, function(a, b) return a.count > b.count end)
    return order
end

local function gameVersion()
    local ok, v = pcall(function() return getCore():getVersion() end)
    return ok and v and tostring(v) or "42"
end

--- The report Copy puts on the clipboard: what a mod's author needs to fix it.
function Errors.report(group)
    local err = group.last
    local lines = {}
    table.insert(lines, text("errReportTitle", gameVersion()) .. " (" .. Errors.setting() .. ")")
    local mod = group.mod and (group.mod .. Errors.modFacts(group.mod)) or text("errNoMod")
    table.insert(lines, text("errReportMod", mod))
    if #err.callers > 0 then table.insert(lines, text("errReportCalled", table.concat(err.callers, ", "))) end
    table.insert(lines, text("errReportTimes", group.count))
    table.insert(lines, "")
    table.insert(lines, err.message or "")
    if #err.frames > 0 then
        table.insert(lines, "")
        for _, f in ipairs(err.frames) do table.insert(lines, "    " .. f.line) end
    end
    -- What the mod itself adds to its reports (PZModStudioLink.registerReport, or its debug report
    -- given to another report API).
    local own, label = Errors.reportOf(Errors.idOf(group.mod), err, group.mod)
    if own then
        table.insert(lines, "")
        table.insert(lines, text("errReportFrom", label))
        for _, line in ipairs(own) do table.insert(lines, "    " .. line) end
    end
    return table.concat(lines, "\n")
end

--- The mod id of a name the game wrote in a stack (its mod.info name), or nil. Kept: looking
--- through every installed mod for each line of a report would be slow with hundreds of them.
local idByName = {}
function Errors.idOf(name)
    if not name then return nil end
    local known = idByName[name]
    if known ~= nil then return known or nil end
    idByName[name] = false
    pcall(function()
        for _, dir in ipairs(getModDirectoryTable()) do
            local info = getModInfo(dir)
            if info and info:getName() == name then
                idByName[name] = plainId(info:getId())
                return
            end
        end
    end)
    return idByName[name] or nil
end

--- " (id, version 1.2, Workshop 123)" of a mod named in a stack: what its author asks first.
--- english: in English whatever the player's language (for the error log).
function Errors.modFacts(name, english)
    local id = Errors.idOf(name)
    if not id then return "" end
    local parts = { id }
    pcall(function()
        local info = getModInfoByID(id)
        local version = info and info:getModVersion()
        if version and tostring(version) ~= "" then table.insert(parts, english and ("version " .. tostring(version)) or text("errReportVersion", tostring(version))) end
        local ws = info and info:getWorkshopID()
        if ws and tostring(ws) ~= "" and tostring(ws) ~= "0" then table.insert(parts, "Workshop " .. tostring(ws)) end
    end)
    return " (" .. table.concat(parts, ", ") .. ")"
end

--- Another mod already shows these errors: two popups for one error is one too many.
function Errors.otherShows()
    local ok, on = pcall(function()
        local mods = getActivatedMods()
        for i = 0, mods:size() - 1 do
            if plainId(tostring(mods:get(i))) == "errorMagnifier" then return true end
        end
        return false
    end)
    return ok and on
end

-- ------------------------------------------------------------------ the notice

local Notice = ISPanel:derive("StudioLinkErrorNotice")

function Notice:new(x, y, w, h)
    local o = ISPanel.new(self, x, y, w, h)
    o.backgroundColor = { r = 0.06, g = 0.05, b = 0.06, a = 0.9 }
    o.borderColor = { r = 0.97, g = 0.44, b = 0.44, a = 0.85 }
    o.moveWithMouse = true
    return o
end

-- How many steps of the stack the notice lists when its details are open.
local DETAIL_STEPS = 8

--- `Lua((MOD:Name)).fn(File.lua:353)` -> "fn (File.lua:353)" and whose it is; the game's own
--- steps are said as such.
local function stepOf(frame)
    local rest = string.match(frame.line, "^Lua%(%(MOD:.-%)%)%.(.*)$") or string.match(frame.line, "^Lua%(Vanilla%)%.(.*)$") or frame.line
    local fn, place = string.match(rest, "^(.-)%((.-)%)$")
    if fn and place then rest = fn .. "  (" .. place .. ")" end
    return rest, frame.mod or text("errStepGame")
end

--- The file and line of the step that failed: "RemoteSpawn.lua:353", or nil.
local function failedAt(err)
    local first = err and err.frames and err.frames[1]
    return first and string.match(first.line, "%(([^()]-%.lua:%d+)%)$") or nil
end

local function fit(s, font, width)
    local tm = getTextManager()
    if tm:MeasureStringX(font, s) <= width then return s end
    while string.len(s) > 1 and tm:MeasureStringX(font, s .. "...") > width do s = string.sub(s, 1, -2) end
    return s .. "..."
end

function Notice:addButton(label, onClick)
    local bh = 22
    local w = getTextManager():MeasureStringX(UIFont.Small, label) + 18
    local b = ISButton:new(self.nextX, self.height - bh - 8, w, bh, label, self, onClick)
    b:initialise()
    b:instantiate()
    b:setFont(UIFont.Small)
    b.borderColor = { r = 0.5, g = 0.5, b = 0.55, a = 0.9 }
    self:addChild(b)
    self.nextX = self.nextX + w + 6
    return b
end

function Notice:createChildren()
    self.nextX = 10
    self:addButton(text("errCopy"), function(panel)
        local groups = byMod()
        if groups[1] then
            local ok = pcall(function() Clipboard.setClipboard(Errors.report(groups[1])) end)
            panel.said = ok and text("errCopied") or nil
            panel.saidAt = Link.now()
        end
    end)
    -- Not for an error of the Link itself: the Mods screen leaves the Link out of its list, so
    -- there would be nothing to show there and the button would seem to do nothing.
    local first = byMod()[1]
    local firstId = first and Errors.idOf(first.mod)
    if not Link.inGame and Link.ModsScreen and firstId and firstId ~= "Z_StudioLink" then
        self:addButton(text("errShowMods"), function(panel)
            local groups = byMod()
            local id = groups[1] and Errors.idOf(groups[1].mod)
            Errors.close()
            pcall(Link.ModsScreen.open, { filter = "all", sort = "name", group = "none", search = "", collapsed = {}, shown = id })
        end)
    end
    if Link.appInstalled() and Link.appAlive() then
        self:addButton(text("errSeeStudio"), function(panel)
            Link.requestShow("monitor")
            panel.said = text("modsAppShown")
            panel.saidAt = Link.now()
        end)
    end
    self:addButton(Errors.detailsOpen and text("errDetailsHide") or text("errDetails"), function()
        Errors.detailsOpen = not Errors.detailsOpen
        Errors.reopen()
    end)
    -- Every kind of error of the session, when there is more than the one shown.
    if #Errors.history > 1 then
        self:addButton(text("errAll", #Errors.history), function() Errors.showList() end)
    end
    self:addButton(text("errClose"), function() Errors.close() end)
    -- Hide, top right: the notice becomes a small badge there, which brings it back.
    local size = 18
    local hide = ISButton:new(self.width - size - 6, 6, size, size, "-", self, function() Errors.hide() end)
    hide:initialise()
    hide:instantiate()
    hide:setFont(UIFont.Small)
    hide.borderColor = { r = 0.5, g = 0.5, b = 0.55, a = 0.9 }
    hide.tooltip = text("errHideTip")
    self:addChild(hide)
end

function Notice:prerender()
    ISPanel.prerender(self)
    local groups = byMod()
    local top = groups[1]
    if not top then return end
    local small = UIFont.Small
    local w = self.width - 20
    local count = 0
    for _, g in ipairs(groups) do count = count + g.count end
    self:drawText(text("errTitle", count), 10, 8, 0.97, 0.55, 0.55, 1, UIFont.Medium)
    local y = 8 + getTextManager():getFontHeight(UIFont.Medium) + 4
    local where = top.mod and text("errIn", top.mod) or text("errNoMod")
    if #top.last.callers > 0 then where = where .. "  " .. text("errCalledFrom", top.last.callers[1]) end
    self:drawText(fit(where, small, w), 10, y, 0.95, 0.95, 0.95, 1, small)
    y = y + getTextManager():getFontHeight(small) + 2
    if #groups > 1 then
        self:drawText(fit(text("errOthers", count - top.count, #groups - 1), small, w), 10, y, 0.7, 0.7, 0.75, 1, small)
    end
    y = y + getTextManager():getFontHeight(small) + 2
    local said = self.said and self.saidAt and Link.now() - self.saidAt < 6 and self.said
    if said then
        self:drawText(fit(said, small, w), 10, y, 0.45, 0.9, 0.65, 1, small)
    else
        self:drawText(fit(top.last.message or "", small, w), 10, y, 0.7, 0.7, 0.75, 1, small)
    end
    if not Errors.detailsOpen then return end
    -- The details: where it failed, then the steps, the failing one first.
    local lineH = getTextManager():getFontHeight(small) + 2
    y = y + lineH + 4
    local at = failedAt(top.last)
    if at then
        self:drawText(fit(text("errFailedAt", at), small, w), 10, y, 1, 0.8, 0.45, 1, small)
    end
    y = y + lineH
    local frames = top.last.frames or {}
    if #frames > 0 then
        self:drawText(fit(text("errSteps"), small, w), 10, y, 0.7, 0.7, 0.75, 1, small)
        y = y + lineH
        for i = 1, math.min(#frames, DETAIL_STEPS) do
            local step, whose = stepOf(frames[i])
            local line = tostring(i) .. ". " .. step .. "   [" .. whose .. "]"
            if frames[i].mod then
                self:drawText(fit(line, small, w), 14, y, 0.92, 0.92, 0.95, 1, small)
            else
                self:drawText(fit(line, small, w), 14, y, 0.55, 0.55, 0.6, 1, small)
            end
            y = y + lineH
        end
        if #frames > DETAIL_STEPS then
            self:drawText(fit(text("errMoreSteps", #frames - DETAIL_STEPS), small, w), 14, y, 0.55, 0.55, 0.6, 1, small)
        end
    end
end

-- ------------------------------------------------------------------ every error of the session
-- The session's errors, each kind once: the same mod, message and place count as one, with how
-- many times it came. A row copies that error's report; "Copy all" copies them all, with the game's
-- version, the mode and how many mods are on at the top.

--- How the game runs now: "single player, Steam" and the like, in the player's language.
function Errors.setting()
    local mp = false
    pcall(function() mp = isClient() == true end)
    local steam = true
    pcall(function() steam = getSteamModeActive() == true end)
    local mods = 0
    pcall(function() mods = getActivatedMods():size() end)
    return text(mp and "errModeMp" or "errModeSp") .. ", " .. text(steam and "errSteam" or "errNoSteam") .. ", " .. text("errModsOn", mods)
end

local function kindKey(err)
    return (err.mod or "") .. "\1" .. (err.message or "") .. "\1" .. (failedAt(err) or "")
end

--- Each kind of error of the session once: { mod, count, last, at }, the most frequent first.
function Errors.kinds()
    local byKey, order = {}, {}
    for _, err in ipairs(Errors.history) do
        local key = kindKey(err)
        local kind = byKey[key]
        if not kind then
            kind = { mod = err.mod, count = 0 }
            byKey[key] = kind
            table.insert(order, kind)
        end
        kind.count = kind.count + 1
        kind.last = err
        kind.at = failedAt(err)
    end
    table.sort(order, function(a, b) return a.count > b.count end)
    return order
end

--- Every kind on the clipboard, numbered, with what a mod's author needs.
function Errors.reportAll()
    local lines = { text("errReportAllTitle", gameVersion(), Errors.setting()), "" }
    for i, kind in ipairs(Errors.kinds()) do
        local err = kind.last
        local head = "#" .. tostring(i) .. "  x" .. tostring(kind.count) .. "  " .. (kind.mod and (kind.mod .. Errors.modFacts(kind.mod)) or text("errNoMod"))
        if err.callers and #err.callers > 0 then head = head .. "  (" .. text("errReportCalled", table.concat(err.callers, ", ")) .. ")" end
        table.insert(lines, head)
        table.insert(lines, err.message or "")
        for _, f in ipairs(err.frames or {}) do table.insert(lines, "    " .. f.line) end
        table.insert(lines, "")
    end
    -- What mods add to the reports (PZModStudioLink.registerReport, and other mods' debug
    -- reports), every one that registered.
    for _, id in ipairs(Errors.reporters()) do
        local own, label = Errors.reportOf(id, nil)
        if own then
            table.insert(lines, text("errReportFrom", label))
            for _, line in ipairs(own) do table.insert(lines, "    " .. line) end
            table.insert(lines, "")
        end
    end
    return table.concat(lines, "\n")
end

-- ------------------------------------------------------------------ what other mods add
-- A mod can attach its own debug information to the Link's reports:
--
--     if PZModStudioLink and PZModStudioLink.registerReport then
--         PZModStudioLink.registerReport("MyModId", function(err)
--             return { "version 1.4", "players: " .. tostring(getNumActivePlayers()) }
--         end)
--     end
--
-- fn is called only when a report is made (Copy report and Copy all in the error notice and its
-- list, and the first time an error of that mod goes to the error log), inside pcall. It gets the
-- error ({ mod, message, at = "File.lua:48" }) or nil (Copy all), and returns a string, a list of
-- lines, or a table of name = value pairs (a table in it is shown indented, REPORT_DEPTH deep);
-- the first REPORT_MAX_LINES lines are kept. A mod whose Lua runs before the Link's can set
-- PZModStudioLinkReports["MyModId"] = fn instead: the same.
-- Its text goes in "Copy all" always, and in a single error's report when the error is that mod's.
--
-- Other mods' debug reports: a mod may hand its report to another error mod's report API instead
-- (registerDebugReport(modId, fn, displayName), kept in that API's modReports as
-- { func = fn, displayName = ... }; fn gets nothing and returns a string or a table). Those go in
-- the same places, under their display name, read from that API's own table while its mod is on
-- (what its require gave, which the game keeps). A mod registered in both is shown once, with
-- what it gave the Link.
-- With that mod off there is nothing to read, and the Link offers no stand-in: answering that
-- require would mean replacing the game's global require, and the game then names the replacing
-- mod in every "require(...) failed" warning and in the stack of every file loaded through it
-- (seen live on 42.22: other mods' and the game's own failed requires all read as the Link's).
-- Shipping a file of the same name instead would replace the real one. The mods that use that API
-- list its mod as a requirement in their mod.info, so the game loads it with them anyway.

PZModStudioLinkReports = PZModStudioLinkReports or {}
local REPORT_MAX_LINES = 60
local REPORT_MAX_CHARS = 4000
local REPORT_DEPTH = 4
-- The file the other API's users require (a technical name: never shown).
local OTHER_API_FILE = "errorMagnifier_Main"

--- Registers (or, with fn nil, takes back) what modId adds to the reports. True when taken.
function Link.registerReport(modId, fn)
    if type(modId) ~= "string" or modId == "" then return false end
    if fn ~= nil and type(fn) ~= "function" then return false end
    PZModStudioLinkReports[plainId(modId)] = fn
    return true
end

--- The other report API's table, or nil: that mod's own, asked only while it is on and only
--- when a report is made (long after it loaded: the game hands back the table it keeps).
local otherApi = nil
function Errors.otherApi()
    if otherApi then return otherApi end
    if not Errors.otherShows() then return nil end
    local ok, value = pcall(require, OTHER_API_FILE)
    if ok and type(value) == "table" and type(value.modReports) == "table" then otherApi = value end
    return otherApi
end

--- The other API's entry for a mod: by its key (modId, or the same id written another way), or by
--- the name a stack gives it (name). Returns the entry and its key, or nil.
local function otherEntry(modId, name)
    local api = Errors.otherApi()
    if not api then return nil end
    local reports = api.modReports
    if modId then
        local entry = reports[modId]
        if type(entry) == "table" and type(entry.func) == "function" then return entry, modId end
    end
    for key, entry in pairs(reports) do
        if type(entry) == "table" and type(entry.func) == "function" then
            local k = tostring(key)
            if (modId and plainId(k) == plainId(modId)) or (name and (k == name or tostring(entry.displayName) == name)) then
                return entry, k
            end
        end
    end
    return nil
end

--- The ids of the mods that registered a report (with the Link or with the other API), sorted.
function Errors.reporters()
    local ids, seen = {}, {}
    if type(PZModStudioLinkReports) == "table" then
        for id, fn in pairs(PZModStudioLinkReports) do
            if type(fn) == "function" then
                table.insert(ids, tostring(id))
                seen[plainId(tostring(id))] = true
            end
        end
    end
    local api = Errors.otherApi()
    if api then
        for key, entry in pairs(api.modReports) do
            local k = tostring(key)
            if type(entry) == "table" and type(entry.func) == "function" and not seen[plainId(k)] then
                table.insert(ids, k)
                seen[plainId(k)] = true
            end
        end
    end
    table.sort(ids)
    return ids
end

--- A report's value as lines: a string, a list, a table of name = value (sorted by name), tables
--- in it indented below their name. At most REPORT_MAX_LINES lines and REPORT_MAX_CHARS chars.
local function reportLines(value)
    local lines, chars = {}, 0
    local function addText(s, indent)
        for line in string.gmatch(tostring(s) .. "\n", "([^\n]*)\n") do
            if #lines < REPORT_MAX_LINES and chars < REPORT_MAX_CHARS then
                local cut = string.sub(indent .. string.gsub(line, "\r", ""), 1, REPORT_MAX_CHARS - chars)
                table.insert(lines, cut)
                chars = chars + string.len(cut)
            end
        end
    end
    local function addTable(t, indent, depth)
        if #t > 0 then
            for _, v in ipairs(t) do
                if type(v) == "table" then
                    if depth < REPORT_DEPTH then addTable(v, indent .. "  ", depth + 1) else addText("{...}", indent) end
                else
                    addText(v, indent)
                end
            end
            return
        end
        local pairsOf = {}
        for k, v in pairs(t) do table.insert(pairsOf, { key = tostring(k), value = v }) end
        table.sort(pairsOf, function(a, b) return a.key < b.key end)
        for _, p in ipairs(pairsOf) do
            if type(p.value) == "table" then
                if depth < REPORT_DEPTH then
                    addText(p.key .. ":", indent)
                    addTable(p.value, indent .. "  ", depth + 1)
                else
                    addText(p.key .. " = {...}", indent)
                end
            else
                addText(p.key .. " = " .. tostring(p.value), indent)
            end
        end
    end
    if type(value) == "table" then
        addTable(value, "", 1)
    elseif value ~= nil then
        addText(value, "")
    end
    return lines
end

--- What a mod adds for err (nil: for every error): its lines and the name to show them under, or
--- nil when it registered none. modId: its id (may be nil); name: the name a stack gives it.
function Errors.reportOf(modId, err, name)
    local reports = PZModStudioLinkReports
    local fn = nil
    if modId and type(reports) == "table" then fn = reports[modId] or reports[plainId(modId)] end
    local label = modId
    local ok, value
    if type(fn) == "function" then
        local arg = nil
        if err then arg = { mod = err.mod, message = err.message, at = failedAt(err) } end
        ok, value = pcall(fn, arg)
    else
        local entry, key = otherEntry(modId, name)
        if not entry then return nil end
        label = (entry.displayName ~= nil and tostring(entry.displayName) ~= "") and tostring(entry.displayName) or key
        ok, value = pcall(entry.func)
    end
    if not ok then return { text("errReportFailed", tostring(value)) }, label end
    local lines = reportLines(value)
    if #lines == 0 then return nil end
    return lines, label
end

-- ------------------------------------------------------------------ kept on disk
-- Every error also goes to Zomboid/Lua/PZModStudio_LinkErrorLog.txt (the only folder the game's
-- Lua can write), appended within a second of coming:
--   ==== <date time>  Project Zomboid <build> (<mode>, <Steam or not>, <n> mods on)  <where>
--   [<time>] <mod> (<id>, version <v>, Workshop <n>) called from <mod>  at <File.lua:48>
--       <message>
--           <the stack, the failing step first>
--       <what the mod adds, PZModStudioLink.registerReport>
--   [<time>] x<n> again: <mod>: <message>  at <File.lua:48>
--   ---- <time> back to the main menu / disconnected from the server
-- A heading starts each load of the game's Lua that has an error to write (the game reloads it
-- going into a save and back). Each kind of error is written in full once per load; repeats are
-- counted and said every LOG_REPEAT_SECONDS. In English, as the game's console.txt: it is for
-- the mods' authors. Past LOG_MAX_LINES lines the file keeps its last LOG_KEEP_LINES.

local LOG_FILE = "PZModStudio_LinkErrorLog.txt"
local LOG_REPEAT_SECONDS = 30
local LOG_MAX_LINES = 6000
local LOG_KEEP_LINES = 3000
local LOG_FULL_PER_LOAD = 100
local LOG_STEPS = 30

local Log = { pending = {}, kinds = {}, repeats = {}, repeatOrder = {}, tick = 0, written = false, full = 0, repeatAt = 0 }
Errors.Log = Log
Errors.LOG_FILE = LOG_FILE

--- The PC's local time (as the game's own new-save names have it); os.date is UTC in the game.
local formats = {}
local function clock(withDate)
    local pattern = withDate and "yyyy-MM-dd HH:mm:ss" or "HH:mm:ss"
    local okLocal, here = pcall(function()
        if not formats[pattern] then formats[pattern] = SimpleDateFormat.new(pattern, Locale.ENGLISH) end
        return formats[pattern]:format(Calendar.getInstance():getTime())
    end)
    if okLocal and here then return tostring(here) end
    local ok, s = pcall(os.date, withDate and "!%Y-%m-%d %H:%M:%S UTC" or "!%H:%M:%S")
    if ok and s then return tostring(s) end
    return tostring(Link.now())
end

--- The setting in English, for the log.
local function logSetting()
    local mp, steam, mods = false, true, 0
    pcall(function() mp = isClient() == true end)
    pcall(function() steam = getSteamModeActive() == true end)
    pcall(function() mods = getActivatedMods():size() end)
    return (mp and "multiplayer" or "single player") .. ", " .. (steam and "Steam" or "no Steam") .. ", " .. tostring(mods) .. " mods on"
end

local function logWhere()
    if not Link.inGame then return "main menu" end
    local save = Link.currentSave and Link.currentSave() or ""
    return save ~= "" and ("in game: " .. save) or "in game"
end

--- Past LOG_MAX_LINES, the file from a heading near its last LOG_KEEP_LINES lines.
local function trimLog()
    local file = getFileReader(LOG_FILE, false)
    if not file then return end
    local lines = {}
    local line = file:readLine()
    while line ~= nil do
        table.insert(lines, line)
        line = file:readLine()
    end
    file:close()
    if #lines <= LOG_MAX_LINES then return end
    local from = #lines - LOG_KEEP_LINES + 1
    for i = from, #lines do
        if string.find(lines[i], "^====") then
            from = i
            break
        end
    end
    local writer = getFileWriter(LOG_FILE, true, false)
    if not writer then return end
    for i = from, #lines do writer:write(lines[i] .. "\n") end
    writer:close()
end

local function appendLog(lines)
    if #lines == 0 then return end
    if not Log.written then
        Log.written = true
        pcall(trimLog)
        table.insert(lines, 1, "==== " .. clock(true) .. "  Project Zomboid " .. gameVersion() .. " (" .. logSetting() .. ")  " .. logWhere())
    end
    local writer = getFileWriter(LOG_FILE, true, true)
    if not writer then return end
    writer:write(table.concat(lines, "\n") .. "\n")
    writer:close()
end

local function shortOf(err)
    local s = (err.mod or "the game's own code") .. ": " .. string.gsub(firstLine(err.message or ""), "^%s+", "")
    local at = failedAt(err)
    if at then s = s .. "  at " .. at end
    return s
end

local function entryLines(err, out)
    local head = "[" .. clock() .. "] " .. (err.mod and (err.mod .. Errors.modFacts(err.mod, true)) or "the game's own code (no mod named)")
    if err.callers and #err.callers > 0 then head = head .. " called from " .. table.concat(err.callers, ", ") end
    local at = failedAt(err)
    if at then head = head .. "  at " .. at end
    table.insert(out, head)
    for line in string.gmatch((err.message or "") .. "\n", "([^\n]*)\n") do
        if line ~= "" then table.insert(out, "    " .. string.gsub(line, "\r", "")) end
    end
    local frames = err.frames or {}
    for i = 1, math.min(#frames, LOG_STEPS) do table.insert(out, "        " .. frames[i].line) end
    if #frames > LOG_STEPS then table.insert(out, "        ... " .. tostring(#frames - LOG_STEPS) .. " more step(s)") end
    local own, label = Errors.reportOf(Errors.idOf(err.mod), err, err.mod)
    if own then
        table.insert(out, "    Added by " .. label .. ":")
        for _, line in ipairs(own) do table.insert(out, "        " .. line) end
    end
end

function Errors.logAdd(err)
    err.logTick = Log.tick
    table.insert(Log.pending, err)
end

--- Writes what is ready: an error read on an earlier look (its stack has come by now) or closed.
--- all: everything, and the repeats counted so far (before a marker).
function Errors.logFlush(all)
    local out, keep = {}, {}
    for _, err in ipairs(Log.pending) do
        if all or err.closed or err.logTick < Log.tick then
            local key = kindKey(err)
            if not Log.kinds[key] and Log.full < LOG_FULL_PER_LOAD then
                Log.kinds[key] = true
                Log.full = Log.full + 1
                entryLines(err, out)
            else
                local r = Log.repeats[key]
                if not r then
                    r = { count = 0, err = err }
                    Log.repeats[key] = r
                    table.insert(Log.repeatOrder, key)
                end
                r.count = r.count + 1
            end
        else
            table.insert(keep, err)
        end
    end
    Log.pending = keep
    Log.tick = Log.tick + 1
    local t = Link.now()
    if #Log.repeatOrder > 0 and (all or t - Log.repeatAt >= LOG_REPEAT_SECONDS) then
        Log.repeatAt = t
        for _, key in ipairs(Log.repeatOrder) do
            local r = Log.repeats[key]
            table.insert(out, "[" .. clock() .. "] x" .. tostring(r.count) .. " again: " .. shortOf(r.err))
        end
        Log.repeats, Log.repeatOrder = {}, {}
    end
    appendLog(out)
end

--- A line between what came before and after (back to the menu, a disconnect), when this load
--- of the game's Lua wrote something.
function Errors.logMark(what)
    pcall(Errors.logFlush, true)
    if not Log.written then return end
    appendLog({ "---- " .. clock() .. " " .. what })
end

--- The folder of the log opened (Windows Explorer and the like) and the file's path on the
--- clipboard. Returns whether the folder opened, and the path.
function Errors.openLog()
    pcall(Errors.logFlush, true)
    local folder = nil
    pcall(function() folder = tostring(getMyDocumentFolder()) end)
    local sep = "/"
    pcall(function() sep = tostring(getFileSeparator()) end)
    if not folder or folder == "" or folder == "nil" then return false, "Zomboid" .. sep .. "Lua" .. sep .. LOG_FILE end
    local dir = folder .. sep .. "Lua"
    local path = dir .. sep .. LOG_FILE
    pcall(function() Clipboard.setClipboard(path) end)
    local opened = false
    pcall(function()
        if isDesktopOpenSupported() then
            showFolderInDesktop(dir)
            opened = true
        end
    end)
    return opened, path
end

local ListWindow = ISPanel:derive("StudioLinkErrorList")
local LIST_W, LIST_H = 640, 420

function ListWindow:new(x, y)
    local o = ISPanel.new(self, x, y, LIST_W, LIST_H)
    o.backgroundColor = { r = 0.06, g = 0.05, b = 0.06, a = 0.95 }
    o.borderColor = { r = 0.97, g = 0.44, b = 0.44, a = 0.85 }
    o.moveWithMouse = true
    return o
end

local function drawKind(list, y, item, alt)
    local kind = item.item
    local small = UIFont.Small
    local fh = getTextManager():getFontHeight(small)
    local w = list.width - 16
    if list.selected == item.index then
        list:drawRect(0, y, list.width, item.height - 1, 0.25, 0.97, 0.44, 0.44)
    elseif alt then
        list:drawRect(0, y, list.width, item.height - 1, 0.15, 0.2, 0.2, 0.25)
    end
    local head = "x" .. tostring(kind.count) .. "   " .. (kind.mod or text("errNoMod"))
    if kind.at then head = head .. "   " .. kind.at end
    list:drawText(fit(head, small, w), 8, y + 4, 0.95, 0.95, 0.95, 1, small)
    list:drawText(fit(kind.last.message or "", small, w), 8, y + 6 + fh, 0.7, 0.7, 0.75, 1, small)
    return y + item.height
end

function ListWindow:createChildren()
    local small = UIFont.Small
    local fh = getTextManager():getFontHeight(small)
    local top = 8 + getTextManager():getFontHeight(UIFont.Medium) + 8
    local list = ISScrollingListBox:new(10, top, self.width - 20, self.height - top - 44)
    list:initialise()
    list:instantiate()
    list.itemheight = fh * 2 + 12
    list.font = small
    list.drawBorder = true
    list.backgroundColor = { r = 0, g = 0, b = 0, a = 0.4 }
    list.doDrawItem = drawKind
    list:setOnMouseDownFunction(self, function(panel, kind)
        local ok = pcall(function() Clipboard.setClipboard(Errors.report(kind)) end)
        panel.said = ok and text("errCopied") or nil
        panel.saidAt = Link.now()
    end)
    self:addChild(list)
    self.list = list
    for _, kind in ipairs(Errors.kinds()) do list:addItem(kind.mod or "", kind) end
    local x = 10
    local function button(label, onClick)
        local bw = getTextManager():MeasureStringX(small, label) + 18
        local b = ISButton:new(x, self.height - 32, bw, 22, label, self, onClick)
        b:initialise()
        b:instantiate()
        b:setFont(small)
        b.borderColor = { r = 0.5, g = 0.5, b = 0.55, a = 0.9 }
        self:addChild(b)
        x = x + bw + 6
        return b
    end
    button(text("errCopyAll"), function(panel)
        local ok = pcall(function() Clipboard.setClipboard(Errors.reportAll()) end)
        panel.said = ok and text("errCopiedAll") or nil
        panel.saidAt = Link.now()
    end)
    -- The file every error also goes to: its folder opened, its path copied.
    local logButton = button(text("errLog"), function(panel)
        local opened, path = Errors.openLog()
        panel.said = opened and text("errLogShown") or text("errLogWhere", path)
        panel.saidAt = Link.now()
    end)
    logButton.tooltip = text("errLogTip")
    button(text("errClose"), function(panel) Errors.closeList() end)
    self.hintX = x + 6
end

function ListWindow:prerender()
    ISPanel.prerender(self)
    local kinds = #self.list.items
    self:drawText(text("errListTitle", #Errors.history, kinds), 10, 8, 0.97, 0.55, 0.55, 1, UIFont.Medium)
    local said = self.said and self.saidAt and Link.now() - self.saidAt < 6 and self.said
    local hint = said or text("errListHint")
    local small = UIFont.Small
    local hx = self.width - 10 - getTextManager():MeasureStringX(small, hint)
    hint = fit(hint, small, self.width - 10 - (self.hintX or 200))
    hx = self.width - 10 - getTextManager():MeasureStringX(small, hint)
    self:drawText(hint, math.max(self.hintX or 200, hx), self.height - 28, said and 0.45 or 0.6, said and 0.9 or 0.6, said and 0.65 or 0.65, 1, small)
end

function Errors.closeList()
    local w = Errors.listWindow
    if not w then return end
    Errors.listAt = { x = w:getX(), y = w:getY() }
    w:setVisible(false)
    if w.onMenu and MainScreen.instance then pcall(function() MainScreen.instance:removeChild(w) end) end
    w:removeFromUIManager()
    Errors.listWindow = nil
end

--- The list of every kind of error of the session, over everything (the pause menu too).
function Errors.showList()
    Errors.closeList()
    if #Errors.history == 0 then return end
    local sw, sh = getCore():getScreenWidth(), getCore():getScreenHeight()
    local at = Errors.listAt or { x = math.floor((sw - LIST_W) / 2), y = math.floor((sh - LIST_H) / 2) }
    local x = math.max(0, math.min(at.x, sw - LIST_W))
    local y = math.max(0, math.min(at.y, sh - LIST_H))
    local w = ListWindow:new(x, y)
    w:initialise()
    w:instantiate()
    local menuUp = false
    pcall(function() menuUp = Link.inGame and MainScreen.instance ~= nil and MainScreen.instance:getIsVisible() end)
    if menuUp then
        w.onMenu = true
        MainScreen.instance:addChild(w)
        w:bringToTop()
    else
        w:addToUIManager()
        w:setAlwaysOnTop(true)
    end
    Errors.listWindow = w
end

-- ------------------------------------------------------------------ the badge
-- The notice hidden: a small "N Lua error(s)" that clicks bring the notice back from and that
-- can be dragged anywhere. It takes clicks only where it is drawn.

local Badge = ISPanel:derive("StudioLinkErrorBadge")
local BADGE_PAD = 8

function Badge:new(x, y, w, h)
    local o = ISPanel.new(self, x, y, w, h)
    o.backgroundColor = { r = 0.3, g = 0.05, b = 0.06, a = 0.85 }
    o.borderColor = { r = 0.97, g = 0.44, b = 0.44, a = 0.9 }
    o.moveWithMouse = true
    return o
end

function Badge:onMouseDown(x, y)
    self.pressX, self.pressY = getMouseX(), getMouseY()
    ISPanel.onMouseDown(self, x, y)
    return true
end

function Badge:onMouseUp(x, y)
    local wasPressed = self.pressX ~= nil
    local still = wasPressed and math.abs(getMouseX() - self.pressX) + math.abs(getMouseY() - self.pressY) <= 4
    self.pressX, self.pressY = nil, nil
    ISPanel.onMouseUp(self, x, y)
    if still then Errors.unhide() end
    return true
end

function Badge:onMouseUpOutside(x, y)
    self.pressX, self.pressY = nil, nil
    ISPanel.onMouseUpOutside(self, x, y)
end

function Badge:label()
    return text("errBadge", #Errors.batch)
end

--- Its size for its label; the right edge stays where it is, as the count grows.
function Badge:fit()
    local w = getTextManager():MeasureStringX(UIFont.Small, self:label()) + BADGE_PAD * 2
    if w ~= self.width then
        self:setX(self:getX() + self.width - w)
        self:setWidth(w)
    end
end

function Badge:prerender()
    self:fit()
    -- Errors that came after it was hidden: it pulses until the notice is looked at.
    local newer = #Errors.batch > (Errors.hiddenWith or 0)
    if newer then
        self.backgroundColor.a = 0.65 + 0.3 * math.abs(math.sin(getTimestampMs() / 300))
    else
        self.backgroundColor.a = 0.85
    end
    ISPanel.prerender(self)
    local fh = getTextManager():getFontHeight(UIFont.Small)
    self:drawText(self:label(), BADGE_PAD, math.floor((self.height - fh) / 2), 1, 0.85, 0.85, 1, UIFont.Small)
end

-- ------------------------------------------------------------------ showing and hiding

local NOTICE_W = 420

local function remove(panel)
    if not panel then return end
    panel:setVisible(false)
    -- On the pause menu the notice is the menu's child (the game hides every other window there).
    if panel.onMenu and MainScreen.instance then pcall(function() MainScreen.instance:removeChild(panel) end) end
    panel:removeFromUIManager()
end

--- The game's pause menu is up: it hides every window that is not its own.
local function pauseMenuUp()
    local ok, up = pcall(function() return Link.inGame and MainScreen.instance ~= nil and MainScreen.instance:getIsVisible() end)
    return ok and up == true
end

local function onScreen(x, y, w, h)
    local sw, sh = getCore():getScreenWidth(), getCore():getScreenHeight()
    return math.max(0, math.min(x, sw - w)), math.max(0, math.min(y, sh - h))
end

--- Where the notice goes first. On the menu, under PZ Mod Studio's card (or its invitation).
--- In game, at the top, left of the clock and the speed controls: over them it took their clicks.
local function firstSpot(w)
    local sw = getCore():getScreenWidth()
    if not Link.inGame then return sw - w - 20, 150 end
    local right = sw - 20
    for _, get in ipairs({ UIManager.getSpeedControls, UIManager.getClock }) do
        local ok, x = pcall(function()
            local e = get and get()
            return e and e:isVisible() and e:getX() or nil
        end)
        if ok and x and x - 20 < right then right = x - 20 end
    end
    return math.max(10, right - w), 10
end

function Errors.close()
    Errors.batch = {}
    Errors.hidden = false
    if Errors.panel then
        Errors.at = { x = Errors.panel:getX(), y = Errors.panel:getY() }
        remove(Errors.panel)
        Errors.panel = nil
    end
    remove(Errors.badge)
    Errors.badge = nil
end

--- The notice as a badge where its hide button was. New errors keep it a badge, counting them.
function Errors.hide()
    local panel = Errors.panel
    if not panel then return end
    Errors.hidden = true
    Errors.hiddenWith = #Errors.batch
    Errors.at = { x = panel:getX(), y = panel:getY() }
    Errors.badgeAt = { right = panel:getX() + panel.width - 6, y = panel:getY() + 6 }
    remove(panel)
    Errors.panel = nil
    Errors.show()
end

--- The notice again, its hide button where the badge was.
function Errors.unhide()
    local badge = Errors.badge
    Errors.hidden = false
    if badge then
        Errors.at = { x = badge:getX() + badge.width + 6 - NOTICE_W, y = badge:getY() - 6 }
        remove(badge)
        Errors.badge = nil
    end
    Errors.show()
end

local function showBadge()
    if Errors.badge then return end
    local h = getTextManager():getFontHeight(UIFont.Small) + 8
    local at = Errors.badgeAt
    local right = at and at.right or (getCore():getScreenWidth() - 20)
    local x, y = onScreen(right - 60, at and at.y or 10, 60, h)
    local badge = Badge:new(x, y, 60, h)
    badge:initialise()
    badge:instantiate()
    badge:addToUIManager()
    badge:setAlwaysOnTop(true)
    badge:fit()
    Errors.badge = badge
end

function Errors.show()
    if Errors.hidden then return showBadge() end
    if Errors.panel then return end
    local w = NOTICE_W
    local h = 8 + getTextManager():getFontHeight(UIFont.Medium) + 4 + (getTextManager():getFontHeight(UIFont.Small) + 2) * 3 + 38
    -- With the details open: where it failed, the heading, the steps and the "more" line.
    if Errors.detailsOpen then
        local top = byMod()[1]
        local steps = top and top.last and top.last.frames and #top.last.frames or 0
        local lines = 1 + (steps > 0 and 1 or 0) + math.min(steps, DETAIL_STEPS) + (steps > DETAIL_STEPS and 1 or 0)
        h = h + 8 + (getTextManager():getFontHeight(UIFont.Small) + 2) * lines
    end
    local x, y
    if Errors.at then x, y = Errors.at.x, Errors.at.y else x, y = firstSpot(w) end
    x, y = onScreen(x, y, w, h)
    local panel = Notice:new(x, y, w, h)
    panel:initialise()
    panel:instantiate()
    if pauseMenuUp() then
        panel.onMenu = true
        MainScreen.instance:addChild(panel)
        panel:bringToTop()
    else
        panel:addToUIManager()
        panel:setAlwaysOnTop(true)
    end
    Errors.panel = panel
end

--- The session's errors again, after a Close: from the pause menu's button.
function Errors.showAll()
    if #Errors.history == 0 then return end
    Errors.batch = {}
    for _, err in ipairs(Errors.history) do table.insert(Errors.batch, err) end
    Errors.hidden = false
    if Errors.badge then
        remove(Errors.badge)
        Errors.badge = nil
    end
    Errors.reopen()
end

--- "Lua errors (N)" on the game's pause menu, beside Report Bug, while the session has any.
function Errors.menuButton(screen)
    local button = Errors.menuBtn
    local n = #Errors.history
    if not Link.inGame or n == 0 or Errors.otherShows() then
        if button then button:setVisible(false) end
        return
    end
    if not button or button.screen ~= screen then
        button = ISButton:new(0, 0, 100, 22, "", screen, function() Errors.showList() end)
        button:initialise()
        button:instantiate()
        button:setFont(UIFont.Small)
        button.borderColor = { r = 0.97, g = 0.44, b = 0.44, a = 0.9 }
        button.screen = screen
        screen:addChild(button)
        Errors.menuBtn = button
    end
    local label = text("errMenuButton", n)
    local w = getTextManager():MeasureStringX(UIFont.Small, label) + 18
    local beside = screen.reportBug
    local x = beside and (beside:getX() - w - 8) or (screen.width - w - 20)
    local y = beside and beside:getY() or (screen.height - 60)
    local h = beside and beside:getHeight() or 22
    button:setTitle(label)
    button.tooltip = text("errMenuButtonTip")
    button:setWidth(w)
    button:setHeight(h)
    button:setX(x)
    button:setY(y)
    button:setVisible(true)
end

--- The notice made again where it is (its size and its buttons follow what it shows).
function Errors.reopen()
    local panel = Errors.panel
    if panel then
        Errors.at = { x = panel:getX(), y = panel:getY() }
        remove(panel)
        Errors.panel = nil
    end
    Errors.show()
end

-- ------------------------------------------------------------------ watching

local function check()
    local t = Link.now()
    if Errors.checkedAt and t - Errors.checkedAt < CHECK_SECONDS then return end
    Errors.checkedAt = t
    -- Said every few seconds, so a reload knows the file is this game's.
    if Errors.seen and (not Errors.wroteAt or t - Errors.wroteAt >= SEEN_WRITE_SECONDS) then pcall(writeSeen) end
    -- The pause menu came up or went away: the notice moves to where it can be seen.
    if Errors.panel and (Errors.panel.onMenu == true) ~= pauseMenuUp() then Errors.reopen() end
    local fresh = Errors.collect()
    -- On disk what is ready, every look: a crash a second later still finds it there.
    local okLog, errLog = pcall(Errors.logFlush, false)
    if not okLog and not Errors.logFailed then
        Errors.logFailed = true
        print("[StudioLink] the error log failed: " .. tostring(errLog))
    end
    if not fresh then return end
    if #Errors.batch > 0 and not Errors.otherShows() then Errors.show() end
end

local function safeCheck()
    local ok, err = pcall(check)
    if not ok and not Errors.failed then
        Errors.failed = true
        print("[StudioLink] the error notice failed: " .. tostring(err))
    end
end

-- The pause menu's own drawing: its button for the session's errors.
local menuPrerender = MainScreen.prerender
function MainScreen:prerender(...)
    menuPrerender(self, ...)
    local ok, err = pcall(Errors.menuButton, self)
    if not ok and not Errors.menuFailed then
        Errors.menuFailed = true
        print("[StudioLink] the pause menu's error button failed: " .. tostring(err))
    end
end

-- The menu (OnFETick runs there) and the game, paused too.
if Events.OnFETick then Events.OnFETick.Add(safeCheck) end
if Events.OnTickEvenPaused then Events.OnTickEvenPaused.Add(safeCheck) elseif Events.OnTick then Events.OnTick.Add(safeCheck) end
-- Entering a game or the menu rebuilds the screen: the notice is made again for what is new.
-- Its spots from the menu are not the game's (the game has its clock up there).
if Events.OnGameStart then
    Events.OnGameStart.Add(function()
        Errors.panel = nil
        Errors.badge = nil
        Errors.at = nil
        Errors.badgeAt = nil
        Errors.logInGame = true
    end)
end
-- The log's markers: back to the main menu from a game, and a server that dropped the game.
if Events.OnMainMenuEnter then
    Events.OnMainMenuEnter.Add(function()
        if Errors.logInGame then
            Errors.logInGame = false
            pcall(Errors.logMark, "back to the main menu")
        end
    end)
end
if Events.OnDisconnect then
    Events.OnDisconnect.Add(function(reason)
        local why = type(reason) == "string" and reason ~= "" and (": " .. reason) or ""
        pcall(Errors.logMark, "disconnected from the server" .. why)
    end)
end
