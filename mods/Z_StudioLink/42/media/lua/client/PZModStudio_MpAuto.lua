-- PZ Mod Studio Link: what the Studio's multiplayer test does by itself inside the game, and
-- where each game is, for the Studio to watch.
--
-- The test leaves its choices in Zomboid/Lua/PZModStudio_MpAuto.txt of this game's own folder:
--   character=random | first | manual
--     random: random traits, a random look
--     first:  what each screen shows first (the default profession, no traits, the default look)
--     manual: nothing is pressed; the player builds the character
--   spawn=1 | 0     take the first spawn place the server offers
--   name=<a surname for a character left without one>
-- This game's place goes to PZModStudio_MpState.txt (state=menu, spawn_select,
-- character_profession, character_look, in_game), which the Studio reads every two seconds.
-- After it connects a game runs the server's mods, so this file runs there only when the
-- server's list has the Studio Link (the Studio's test adds it).

local CFG = "PZModStudio_MpAuto.txt"
local STATE = "PZModStudio_MpState.txt"

local function readKeys(file)
    local reader = getFileReader(file, false)
    if not reader then return nil end
    local t = {}
    local line = reader:readLine()
    while line do
        local k, v = string.match(line, "^([%w_]+)=(.*)$")
        if k then t[k] = v end
        line = reader:readLine()
    end
    reader:close()
    return t
end

local lastState = nil
local function say(state)
    if state == lastState then return end
    lastState = state
    local writer = getFileWriter(STATE, true, false)
    if writer then
        writer:write("state=" .. state .. "\n")
        writer:close()
    end
end

local cfg = nil
local shown = {}
-- Acts on a screen once it has been up half a second (it builds over a few frames), and again
-- every five seconds while it stays up (a press the screen ignored).
local function due(key)
    shown[key] = (shown[key] or 0) + 1
    local n = shown[key]
    return n == 30 or (n > 30 and (n - 30) % 300 == 0)
end

local function auto()
    if cfg == nil then cfg = readKeys(CFG) or {} end
    return cfg.character == "random" or cfg.character == "first"
end

local function tick()
    local main = MainScreen and MainScreen.instance
    local spawn = MapSpawnSelect and MapSpawnSelect.instance
    if spawn and spawn:isVisible() then
        say("spawn_select")
        auto()
        if cfg.spawn == "1" and due("spawn") then spawn:clickNext() end
        return
    end
    shown.spawn = nil
    local prof = main and main.charCreationProfession
    if prof and prof:isVisible() then
        say("character_profession")
        if auto() and due("prof") then
            if cfg.character == "random" then prof:randomizeTraits() end
            prof:onOptionMouseDown({ internal = "NEXT" }, 0, 0)
        end
        return
    end
    shown.prof = nil
    local look = main and main.charCreationMain
    if look and look:isVisible() then
        say("character_look")
        if auto() and due("look") then
            if cfg.character == "random" then look:onRandomCharacter() end
            if look.forenameEntry and look.forenameEntry:getText() == "" then look.forenameEntry:setText("Test") end
            if look.surnameEntry and look.surnameEntry:getText() == "" then look.surnameEntry:setText(cfg.name or "Player") end
            look:onOptionMouseDown({ internal = "NEXT" }, 0, 0)
        end
        return
    end
    shown.look = nil
    if main and main:isVisible() then say("menu") end
end

if Events.OnFETick then
    Events.OnFETick.Add(tick)
end
if Events.OnGameStart then
    Events.OnGameStart.Add(function() say("in_game") end)
end
