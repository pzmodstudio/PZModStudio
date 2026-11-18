-- PZ Mod Studio Link: which mod's version of a function the game really has.
--
-- When two mods redefine the same function, the Studio works out from their files which one the
-- game ends up with. Here the game is asked: the app lists those functions in
-- Lua/PZModStudio_LinkProbe.txt (`list=<key>`, one `fn=<Table.name>` per function), and the Link
-- writes, for each, the file and first line of the function that is there now:
--   Lua/PZModStudio_LinkProbeMenu.txt  a few seconds after the main menu came up;
--   Lua/PZModStudio_LinkProbeGame.txt  a few seconds into a loaded save (handlers of OnGameStart
--                                      and the like have run by then).
-- Each starts with `list=<key>` (the app's, echoed) and `at=<unix seconds>`, then one line per
-- function: `fn=<name>\t<file>\t<line>`, or `fn=<name>\t-\t0` when it is not a Lua function (gone,
-- or the game's Java). Only reads names: nothing in the game is changed.

require "PZModStudio_Link"

local Link = PZModStudioLink

local PROBE_FILE = "PZModStudio_LinkProbe.txt"
local MENU_FILE = "PZModStudio_LinkProbeMenu.txt"
local GAME_FILE = "PZModStudio_LinkProbeGame.txt"
local DELAY_SECONDS = 4
local MAX_NAMES = 400

local Check = {}
Link.Check = Check

local function readProbe()
    local file = getFileReader(PROBE_FILE, false)
    if not file then return nil end
    local probe = { key = "", names = {} }
    local line = file:readLine()
    while line ~= nil do
        local key, value = string.match(line, "^(%a+)=(.*)$")
        if key == "list" then
            probe.key = value
        elseif key == "fn" and value ~= "" and #probe.names < MAX_NAMES then
            table.insert(probe.names, value)
        end
        line = file:readLine()
    end
    file:close()
    return probe
end

--- The value at a dotted name (`ISFoo.bar`), or nil.
local function lookUp(name)
    local value = _G
    for part in string.gmatch(name, "[^%.]+") do
        if type(value) ~= "table" then return nil end
        value = value[part]
        if value == nil then return nil end
    end
    return value
end

--- Where the function at a name was written: its file and first line, or nil.
local function placeOf(name)
    local okLook, fn = pcall(lookUp, name)
    if not okLook then return nil end
    if type(fn) ~= "function" then return nil end
    -- Only a Lua function has a file: asked of one of the game's Java functions, the game writes
    -- an error in its log even inside pcall. Made a string, a Lua function reads "closure 0x...",
    -- one of the game's Java "function 0x...".
    local okText, said = pcall(tostring, fn)
    if not okText or not string.find(tostring(said), "^closure") then return nil end
    local okFile, file = pcall(getFilenameOfClosure, fn)
    if not okFile or not file or file == "" then return nil end
    local okLine, line = pcall(getFirstLineOfClosure, fn)
    return tostring(file), (okLine and tonumber(line)) or 0
end

local function clean(s)
    return (string.gsub(tostring(s), "[\t\r\n]", " "))
end

--- Looks at every function the app listed and writes what was found.
function Check.run(fileName)
    local probe = readProbe()
    if not probe or #probe.names == 0 then return false end
    local out = getFileWriter(fileName, true, false)
    if not out then return false end
    out:write("list=" .. clean(probe.key) .. "\n")
    out:write("at=" .. tostring(Link.now()) .. "\n")
    for _, name in ipairs(probe.names) do
        local ok, file, line = pcall(placeOf, name)
        if ok and file then
            out:write("fn=" .. clean(name) .. "\t" .. clean(file) .. "\t" .. tostring(line) .. "\n")
        else
            out:write("fn=" .. clean(name) .. "\t-\t0\n")
        end
    end
    out:close()
    return true
end

-- A few seconds after the menu or a save came up, once: what the handlers of those events
-- define has run by then.
local due = nil
local function tick()
    if not due or Link.now() < due.at then return end
    local fileName = due.file
    due = nil
    local ok, err = pcall(Check.run, fileName)
    if not ok and not Check.failed then
        Check.failed = true
        print("[StudioLink] the function check failed: " .. tostring(err))
    end
end

Events.OnMainMenuEnter.Add(function() due = { at = Link.now() + DELAY_SECONDS, file = MENU_FILE } end)
Events.OnGameStart.Add(function() due = { at = Link.now() + DELAY_SECONDS, file = GAME_FILE } end)
if Events.OnFETick then Events.OnFETick.Add(tick) end
if Events.OnTickEvenPaused then Events.OnTickEvenPaused.Add(tick) elseif Events.OnTick then Events.OnTick.Add(tick) end
