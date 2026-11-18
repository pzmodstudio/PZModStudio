-- PZ Mod Studio Link: when joining a server fails over a mod, say which mod and what to do.
--
-- Joining with Steam, the game compares each of the server's Workshop items with this PC's. When
-- one differs it stops the connection with "Workshop item version is different than the server's"
-- and the item's number, nothing more. The Link listens to the same events
-- (OnServerWorkshopItems "Error" with the item and the reason, then OnConnectFailed) and shows, over
-- the connect screen, the mod's name, which side is probably behind and what to do about it, with
-- the item's Workshop page to copy. A game build that differs from the server's is said plainly too.

require "PZModStudio_Link"
require "ISUI/ISPanel"
require "ISUI/ISButton"

local Link = PZModStudioLink
local text = Link.text

local Join = { failed = {}, at = 0 }
Link.Join = Join

local WINDOW_SECONDS = 20

--- The mods this PC has from one Workshop item: their names, or nil.
local function modsOfItem(itemID)
    local names = {}
    pcall(function()
        for _, dir in ipairs(getModDirectoryTable()) do
            local info = getModInfo(dir)
            if info and tostring(info:getWorkshopID()) == tostring(itemID) then
                local name = info:getName()
                local known = false
                for _, n in ipairs(names) do
                    if n == name then known = true end
                end
                if not known then table.insert(names, name) end
            end
        end
    end)
    if #names == 0 then return nil end
    return table.concat(names, ", ")
end

local function onWorkshopItems(flag, arg1, arg2)
    if flag ~= "Error" then return end
    local now = Link.now()
    if now - Join.at > WINDOW_SECONDS then Join.failed = {} end
    Join.at = now
    table.insert(Join.failed, { id = tostring(arg1 or "?"), why = tostring(arg2 or "") })
end

-- ------------------------------------------------------------------ the notice

local Notice = ISPanel:derive("StudioLinkJoinNotice")
local W = 560

function Notice:new(x, y, h)
    local o = ISPanel.new(self, x, y, W, h)
    o.backgroundColor = { r = 0.06, g = 0.05, b = 0.06, a = 0.95 }
    o.borderColor = { r = 0.97, g = 0.7, b = 0.3, a = 0.9 }
    o.moveWithMouse = true
    return o
end

--- What to do for one reason the game gave.
local function adviceFor(why)
    if why == "VersionMismatch" then return text("joinVersion") end
    if why == "ItemNotDownloaded" or why == "DownloadItemFalse" then return text("joinDownload") end
    if why == "ItemNotSubscribed" or why == "SubscribeItemFalse" then return text("joinSubscribe") end
    return text("joinOther", why)
end

function Notice:createChildren()
    local x = 10
    local function button(label, onClick)
        local bw = getTextManager():MeasureStringX(UIFont.Small, label) + 18
        local b = ISButton:new(x, self.height - 32, bw, 22, label, self, onClick)
        b:initialise()
        b:instantiate()
        b:setFont(UIFont.Small)
        b.borderColor = { r = 0.5, g = 0.5, b = 0.55, a = 0.9 }
        self:addChild(b)
        x = x + bw + 6
    end
    if #self.items > 0 then
        button(text("joinCopyLinks"), function(panel)
            local lines = {}
            for _, item in ipairs(panel.items) do
                table.insert(lines, (item.name or item.id) .. ": https://steamcommunity.com/sharedfiles/filedetails/?id=" .. item.id)
            end
            pcall(function() Clipboard.setClipboard(table.concat(lines, "\n")) end)
            panel.said = text("joinCopied")
            panel.saidAt = Link.now()
        end)
    end
    if Link.appInstalled() and Link.appAlive() then
        button(text("errSeeStudio"), function(panel)
            Link.requestShow("whats-new")
            panel.said = text("modsAppShown")
            panel.saidAt = Link.now()
        end)
    end
    button(text("errClose"), function(panel) Join.close() end)
end

local function wrap(s, width)
    local tm = getTextManager()
    local lines, line = {}, ""
    for word in string.gmatch(s, "%S+") do
        local try = line == "" and word or (line .. " " .. word)
        if tm:MeasureStringX(UIFont.Small, try) > width and line ~= "" then
            table.insert(lines, line)
            line = word
        else
            line = try
        end
    end
    if line ~= "" then table.insert(lines, line) end
    return lines
end

function Notice:layout()
    local w = W - 20
    local out = {}
    for _, item in ipairs(self.items) do
        table.insert(out, { (item.name or text("joinUnknownMod")) .. "  (Workshop " .. item.id .. ")", 1, 0.85, 0.5 })
        for _, l in ipairs(wrap(adviceFor(item.why), w - 10)) do table.insert(out, { l, 0.8, 0.8, 0.85, 10 }) end
    end
    if self.build then
        for _, l in ipairs(wrap(text("joinBuild", self.build[1], self.build[2]), w)) do table.insert(out, { l, 0.85, 0.85, 0.9 }) end
    end
    return out
end

function Notice:prerender()
    ISPanel.prerender(self)
    self:drawText(text("joinTitle"), 10, 8, 0.97, 0.75, 0.4, 1, UIFont.Medium)
    local fh = getTextManager():getFontHeight(UIFont.Small) + 2
    local y = 10 + getTextManager():getFontHeight(UIFont.Medium) + 6
    for _, l in ipairs(self:layout()) do
        self:drawText(l[1], 10 + (l[5] or 0), y, l[2], l[3], l[4], 1, UIFont.Small)
        y = y + fh
    end
    local said = self.said and self.saidAt and Link.now() - self.saidAt < 6 and self.said
    if said then self:drawText(said, 10, self.height - 50, 0.45, 0.9, 0.65, 1, UIFont.Small) end
end

function Join.close()
    if Join.panel then
        Join.panel:setVisible(false)
        Join.panel:removeFromUIManager()
        Join.panel = nil
    end
end

--- The notice for what failed: the items with their reasons, and/or the game build.
function Join.show(items, build)
    Join.close()
    local probe = Notice:new(0, 0, 100)
    probe.items, probe.build = items, build
    local lines = #probe:layout()
    local fh = getTextManager():getFontHeight(UIFont.Small) + 2
    local h = 10 + getTextManager():getFontHeight(UIFont.Medium) + 6 + lines * fh + 64
    local sw, sh = getCore():getScreenWidth(), getCore():getScreenHeight()
    local panel = Notice:new(math.floor((sw - W) / 2), math.max(10, math.floor(sh * 0.12)), h)
    panel.items, panel.build = items, build
    panel:initialise()
    panel:instantiate()
    panel:addToUIManager()
    panel:setAlwaysOnTop(true)
    Join.panel = panel
end

local function onConnectFailed(message, detail)
    local ok, err = pcall(function()
        local now = Link.now()
        local items = {}
        if now - Join.at <= WINDOW_SECONDS then
            for _, f in ipairs(Join.failed) do
                table.insert(items, { id = f.id, why = f.why, name = modsOfItem(f.id) })
            end
        end
        Join.failed = {}
        -- "Client version (42.22.2) does not match server version (42.21.1)"
        local build = nil
        local mine, theirs = string.match(tostring(message or ""), "%(([%d%.]+)%).-%(([%d%.]+)%)")
        if mine and theirs and string.find(tostring(message), "version", 1, true) then
            build = { mine, theirs }
        end
        if #items > 0 or build then Join.show(items, build) end
    end)
    if not ok and not Join.failedOnce then
        Join.failedOnce = true
        print("[StudioLink] the join notice failed: " .. tostring(err))
    end
end

if Events.OnServerWorkshopItems then Events.OnServerWorkshopItems.Add(onWorkshopItems) end
if Events.OnConnectFailed then Events.OnConnectFailed.Add(onConnectFailed) end
if Events.OnGameStart then Events.OnGameStart.Add(function() Join.close() end) end
