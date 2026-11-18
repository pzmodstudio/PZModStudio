-- PZ Mod Studio Link: the game's own list of server and sandbox options, for the app's editor.
--
-- The game knows every server option (ServerOptions) and builds the pages of its sandbox screen in
-- Lua (ServerSettingsScreen.getSandboxSettingsTable): each option with its name, help, type, the
-- words of each choice and its default, and the sandbox options of the mods that are on. A few seconds after the
-- main menu comes up, the Link writes that list to Lua/PZModStudio_LinkSettings.txt, in the
-- player's language, when the build, the language or the mods changed since it last did:
--   key=<build>|<language>|<mods>
--   page=<server|sandbox>\t<page name>
--   opt=<server|sandbox>\t<name>\t<type>\t<default>\t<min>\t<max>\t<label>\t<help>
--   val=<name>\t<number>\t<words>          (the choices of an enum option, from 1)
-- Only reads: nothing of the game changes.

require "PZModStudio_Link"

local Link = PZModStudioLink

local FILE = "PZModStudio_LinkSettings.txt"
local DELAY_SECONDS = 6

local Settings = {}
Link.Settings = Settings

local function clean(s)
    if s == nil then return "" end
    s = tostring(s)
    s = string.gsub(s, "<LINE>", " ")
    s = string.gsub(s, "[\t\r\n]", " ")
    s = string.gsub(s, "%s+", " ")
    return s
end

--- What the file says it was written for: build, language and the mods that were on.
local function currentKey()
    local build, lang, mods = "?", "?", ""
    pcall(function() build = tostring(getCore():getVersion()) end)
    pcall(function() lang = tostring(Translator.getLanguage()) end)
    pcall(function()
        local list = getActivatedMods()
        local ids = {}
        for i = 0, list:size() - 1 do table.insert(ids, tostring(list:get(i))) end
        table.sort(ids)
        mods = table.concat(ids, ",")
    end)
    return build .. "|" .. lang .. "|" .. mods
end

local function writtenKey()
    local file = getFileReader(FILE, false)
    if not file then return nil end
    local first = file:readLine()
    file:close()
    return first and string.match(first, "^key=(.*)$") or nil
end

local function numberOrEmpty(option, method)
    local ok, v = pcall(function() return option[method](option) end)
    if ok and v ~= nil then return tostring(v) end
    return ""
end

--- One option: its line, and its choices when it has some.
local function optionLines(kind, option, label, out)
    local name = option:getName()
    local typ = tostring(option:getType())
    local default = ""
    pcall(function() default = tostring(option:getDefaultValue()) end)
    local tooltip = ""
    pcall(function() tooltip = option:getTooltip() end)
    -- Only numbers have a range: asked of another option, the game would log an error.
    local numeric = typ == "integer" or typ == "double"
    local min = numeric and numberOrEmpty(option, "getMin") or ""
    local max = numeric and numberOrEmpty(option, "getMax") or ""
    table.insert(out, "opt=" .. kind .. "\t" .. clean(name) .. "\t" .. typ .. "\t" .. clean(default) .. "\t" .. min .. "\t" .. max .. "\t" .. clean(label) .. "\t" .. clean(tooltip))
    if typ == "enum" then
        local n = 0
        pcall(function() n = option:getNumValues() end)
        for k = 1, n do
            local words = tostring(k)
            if name == "StartYear" then
                pcall(function() words = tostring(getSandboxOptions():getFirstYear() + k - 1) end)
            elseif name ~= "StartDay" then
                pcall(function() words = option:getValueTranslationByIndex(k) end)
            end
            table.insert(out, "val=" .. clean(name) .. "\t" .. tostring(k) .. "\t" .. clean(words))
        end
    end
end

--- Writes the list when what it was written for changed. True when it wrote.
function Settings.export(force)
    local key = currentKey()
    if not force and writtenKey() == key then return false end
    local out = { "key=" .. key }
    -- The server's options, in the game's own order (its screen groups them in pages of its own).
    local server = ServerOptions and ServerOptions.new() or nil
    if server then
        table.insert(out, "page=server\t" .. clean(Link.text("settingsServerPage")))
        local n = 0
        pcall(function() n = server:getNumOptions() end)
        for i = 0, n - 1 do
            local option = nil
            pcall(function()
                option = server:getOptionByIndex(i)
                if option and option.asConfigOption then option = option:asConfigOption() end
            end)
            if option then
                local name = option:getName()
                local label = name
                local ok, t = pcall(getTextOrNull, "UI_ServerOption_" .. name)
                if ok and t then label = t end
                pcall(optionLines, "server", option, label, out)
            end
        end
    end
    -- The sandbox's, by the pages of the game's own screen, the mods' pages included.
    local pages = nil
    pcall(function() pages = ServerSettingsScreen.getSandboxSettingsTable() end)
    local sandbox = getSandboxOptions()
    for _, page in ipairs(pages or {}) do
        table.insert(out, "page=sandbox\t" .. clean(page.name or page.title))
        for _, setting in ipairs(page.settings or {}) do
            local option = nil
            pcall(function() option = sandbox:getOptionByName(setting.name) end)
            if option then pcall(optionLines, "sandbox", option, setting.translatedName or setting.name, out) end
        end
    end
    local writer = getFileWriter(FILE, true, false)
    if not writer then return false end
    for _, line in ipairs(out) do writer:write(line .. "\n") end
    writer:close()
    return true
end

local due = nil
local function tick()
    if not due or Link.now() < due then return end
    due = nil
    local ok, err = pcall(Settings.export, false)
    if not ok and not Settings.failed then
        Settings.failed = true
        print("[StudioLink] the settings list failed: " .. tostring(err))
    end
end

Events.OnMainMenuEnter.Add(function() due = Link.now() + DELAY_SECONDS end)
if Events.OnFETick then Events.OnFETick.Add(tick) end
