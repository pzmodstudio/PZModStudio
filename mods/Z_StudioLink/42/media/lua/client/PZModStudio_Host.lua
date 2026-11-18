-- PZ Mod Studio Link: before hosting, the server's own mod list against the mods that are on.
--
-- HOST starts a server with the server settings' own Mods= list, not the mods switched on in the
-- Mods screen. A player who turned a mod on for this world and hosts finds it missing in game.
-- When Start is pressed on the host screen, the Link reads the chosen settings' Mods= and, if it
-- lacks mods that are on now, or names mods this PC does not have, asks first: host anyway, or go
-- back (Edit Settings changes the list). A settings name that does not exist yet is skipped: the
-- game makes it from the mods that are on.

require "PZModStudio_Link"

local Link = PZModStudioLink
local text = Link.text

local Host = {}
Link.Host = Host

local function plain(id)
    local s = tostring(id)
    s = string.match(s, "^%s*(.-)%s*$") or s
    if string.sub(s, 1, 1) == "\\" then s = string.sub(s, 2) end
    return s
end

--- The mods switched on now, by id (the Studio's own left out).
local function activeIds()
    local out = {}
    pcall(function()
        local mods = getActivatedMods()
        for i = 0, mods:size() - 1 do
            local id = plain(mods:get(i))
            if id ~= "" and id ~= "Z_StudioLink" and id ~= "PZModStudio_Profiler" and string.sub(id, 1, 14) ~= "Z_PZModStudio_" then table.insert(out, id) end
        end
    end)
    return out
end

local function installed(id)
    local ok, found = pcall(function() return getModInfoByID(id) ~= nil end)
    return ok and found
end

local function nameOf(id)
    local ok, name = pcall(function()
        local info = getModInfoByID(id)
        return info and info:getName() or nil
    end)
    return (ok and name) or id
end

--- What the settings' Mods= lacks and what it names that is not installed: two lists of names.
function Host.compare(settingsName)
    local ok, modsString = pcall(function()
        if getServerSettingsManager():isValidNewName(settingsName) then return nil end
        local settings = ServerSettings.new(settingsName)
        if not settings:isValid() then return nil end
        settings:loadFiles()
        return settings:getServerOptions():getOptionByName("Mods"):getValue()
    end)
    if not ok or not modsString then return nil end
    local server = {}
    local serverList = {}
    for raw in string.gmatch(modsString, "[^;]+") do
        local id = plain(raw)
        if id ~= "" then
            server[id] = true
            table.insert(serverList, id)
        end
    end
    local missing, notInstalled = {}, {}
    for _, id in ipairs(activeIds()) do
        if not server[id] then table.insert(missing, nameOf(id)) end
    end
    for _, id in ipairs(serverList) do
        if not installed(id) and id ~= "Z_StudioLink" then table.insert(notInstalled, id) end
    end
    return missing, notInstalled
end

local function listed(names)
    local shown = {}
    for i = 1, math.min(#names, 8) do table.insert(shown, names[i]) end
    local s = table.concat(shown, ", ")
    if #names > 8 then s = s .. " " .. text("hostAndMore", #names - 8) end
    return s
end

local original = CoopOptionsScreen and CoopOptionsScreen.onStartButtonDown
if original then
    function CoopOptionsScreen:onStartButtonDown(button, x, y)
        if self.studioLinkAsked or self.softreset then
            self.studioLinkAsked = false
            return original(self, button, x, y)
        end
        local okCompare, missing, notInstalled = pcall(Host.compare, CoopConnection and CoopConnection.servername or "")
        if not okCompare or not missing or (#missing == 0 and #notInstalled == 0) then
            return original(self, button, x, y)
        end
        local lines = { text("hostTitle", tostring(CoopConnection.servername)) }
        if #missing > 0 then table.insert(lines, text("hostMissing", #missing, listed(missing))) end
        if #notInstalled > 0 then table.insert(lines, text("hostNotInstalled", #notInstalled, listed(notInstalled))) end
        table.insert(lines, text("hostWhat"))
        local screenW, screenH = getCore():getScreenWidth(), getCore():getScreenHeight()
        local modal = ISModalDialog:new(0, 0, 1, 1, table.concat(lines, "\n\n"), true, self, function(screen, answer)
            if answer.internal == "YES" then
                screen.studioLinkAsked = true
                screen:onStartButtonDown(button, x, y)
            end
        end)
        modal:setX((screenW - modal.width) / 2)
        modal:setY((screenH - modal.height) / 2)
        modal.backgroundColor.a = 0.95
        modal:initialise()
        modal:setCapture(true)
        modal:setAlwaysOnTop(true)
        modal:addToUIManager()
    end
end
