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
local function settle(err)
    if not err.frames then err.frames = {} end
    err.callers = {}
    for _, f in ipairs(err.frames) do
        if f.mod then
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
    table.insert(lines, text("errReportTitle", gameVersion()))
    table.insert(lines, text("errReportMod", group.mod or text("errNoMod")))
    if #err.callers > 0 then table.insert(lines, text("errReportCalled", table.concat(err.callers, ", "))) end
    table.insert(lines, text("errReportTimes", group.count))
    table.insert(lines, "")
    table.insert(lines, err.message or "")
    if #err.frames > 0 then
        table.insert(lines, "")
        for _, f in ipairs(err.frames) do table.insert(lines, "    " .. f.line) end
    end
    return table.concat(lines, "\n")
end

--- The mod id of a name the game wrote in a stack (its mod.info name), or nil.
function Errors.idOf(name)
    if not name then return nil end
    for _, dir in ipairs(getModDirectoryTable()) do
        local info = getModInfo(dir)
        if info and info:getName() == name then return plainId(info:getId()) end
    end
    return nil
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
        button = ISButton:new(0, 0, 100, 22, "", screen, function() Errors.showAll() end)
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
    if not Errors.collect() then return end
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
    end)
end
