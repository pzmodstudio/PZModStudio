-- =============================================================================
-- Claude Agent â€” lets an outside agent drive Project Zomboid everywhere:
-- main menu, new-game screens, sandbox options, loading and in-game.
--
-- Protocol (files live in <user>/Zomboid/Lua/):
--   claude_cmd.lua  agent writes Lua; first line "--id:<n>". Body may `return` a value.
--   claude_out.txt  mod writes "id:<n>\nok|err\n<serialized result>" after running it.
--   claude_log.txt  CA.log(...) lines, appended (cleared at boot).
-- The command runs once per new id. Polled from OnFETick (menus), OnTickEvenPaused
-- (in game, also while paused) and OnPreUIDraw (fallback for any other state).
-- =============================================================================

CA = CA or {}
CA.CMD = "claude_cmd.lua"
CA.OUT = "claude_out.txt"
CA.LOG = "claude_log.txt"
CA.lastId = CA.lastId -- kept across a hot reload, or the reload command would run again
CA.lastPollMs = 0
CA.lastCmdMs = CA.lastCmdMs or 0
CA.tasks = CA.tasks or {}

local function now() return getTimeInMillis() end
-- note: IsoMovingObject:DistTo is not euclidean (about 1.4x on diagonals); helpers use getDistanceSq

-- ---------------------------------------------------------------- serialize
function CA.ser(v, depth, seen)
    depth = depth or 4
    seen = seen or {}
    local t = type(v)
    if t == "string" then return '"' .. v:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n') .. '"' end
    if t == "number" or t == "boolean" or t == "nil" then return tostring(v) end
    if t == "table" then
        if seen[v] then return '"<cycle>"' end
        if depth <= 0 then return '"<table>"' end
        seen[v] = true
        local parts, n = {}, 0
        -- a list only if every key is 1..#v (mixed tables keep their named fields)
        local count = 0
        for _ in pairs(v) do count = count + 1 end
        local isArr = #v > 0 and count == #v
        if isArr then
            for i = 1, #v do
                n = n + 1
                if n > 400 then parts[#parts + 1] = '"..."'; break end
                parts[#parts + 1] = CA.ser(v[i], depth - 1, seen)
            end
            seen[v] = nil
            return "[" .. table.concat(parts, ",") .. "]"
        end
        for k, val in pairs(v) do
            if type(val) ~= "function" then
                n = n + 1
                if n > 400 then parts[#parts + 1] = '"...":0'; break end
                parts[#parts + 1] = CA.ser(tostring(k), 1) .. ":" .. CA.ser(val, depth - 1, seen)
            end
        end
        seen[v] = nil
        return "{" .. table.concat(parts, ",") .. "}"
    end
    -- Java objects / userdata / functions
    local ok, s = pcall(tostring, v)
    return CA.ser(ok and s or ("<" .. t .. ">"), 1)
end

-- ---------------------------------------------------------------- file io
local function readAll(name)
    local r = getFileReader(name, false)
    if not r then return nil end
    local lines = {}
    local line = r:readLine()
    while line do
        lines[#lines + 1] = line
        line = r:readLine()
    end
    r:close()
    return table.concat(lines, "\n")
end

local function writeAll(name, text, append)
    local w = getFileWriter(name, true, append == true)
    if not w then return end
    w:write(text)
    w:close()
end

function CA.log(...)
    local parts = {}
    for i = 1, select("#", ...) do
        local a = select(i, ...)
        parts[#parts + 1] = type(a) == "string" and a or CA.ser(a, 3)
    end
    writeAll(CA.LOG, tostring(now()) .. " " .. table.concat(parts, " ") .. "\n", true)
end

-- ---------------------------------------------------------------- ui helpers
local function elInfo(el, depth)
    local info = {}
    pcall(function()
        info.type = el.Type
        info.x = math.floor(el:getAbsoluteX()); info.y = math.floor(el:getAbsoluteY())
        info.w = math.floor(el:getWidth()); info.h = math.floor(el:getHeight())
        info.vis = el:getIsVisible()
        local label = el.title or el.name or el.internal or el.text
        if type(label) == "string" and label ~= "" then info.label = label end
        if el.internal then info.internal = tostring(el.internal) end
        if el.enable == false then info.disabled = true end
        if el.selected ~= nil and type(el.selected) == "table" then info.selected = el.selected end
        if el.options and type(el.options) == "table" and el.Type == "ISComboBox" then
            local opts = {}
            for i, o in ipairs(el.options) do opts[i] = type(o) == "table" and (o.text or tostring(o)) or tostring(o) end
            info.options = opts
        end
    end)
    return info
end

-- Visit every visible UI element (depth-first). fn(el, depth) returning true stops.
function CA.walk(fn, includeHidden)
    local roots = UIManager.getUI()
    local seen = {}
    local function visit(el, depth)
        if not el then return false end
        local okVis, vis = pcall(function() return el:getIsVisible() end)
        if not includeHidden and okVis and not vis then return false end
        if seen[el] then return false end
        seen[el] = true
        if fn(el, depth) then return true end
        if el.children then
            for _, c in pairs(el.children) do
                if visit(c, depth + 1) then return true end
            end
        end
        -- some screens (e.g. CoopCharacterCreation) add panels on the Java side only
        local jo = el.javaObject
        if jo and jo.getControls then
            local ok, ctl = pcall(function() return jo:getControls() end)
            if ok and ctl then
                for i = 0, ctl:size() - 1 do
                    local t = ctl:get(i):getTable()
                    if t and visit(t, depth + 1) then return true end
                end
            end
        end
        return false
    end
    for i = 0, roots:size() - 1 do
        local j = roots:get(i)
        local t = j and j.getTable and j:getTable()
        if t and visit(t, 0) then return end
    end
end

-- Flat list of visible elements that carry a label or are interactive.
function CA.ui(filter)
    local out = {}
    CA.walk(function(el, depth)
        local info = elInfo(el, depth)
        info.d = depth
        local interesting = info.label or (info.type and (info.type:find("Button") or info.type:find("Tick") or info.type:find("Combo") or info.type:find("TextEntry") or info.type:find("Slider") or info.type:find("ScrollingListBox")))
        if interesting then
            if not filter or (info.label and info.label:lower():find(filter:lower(), 1, true)) or (info.type and info.type:find(filter, 1, true)) then
                out[#out + 1] = info
            end
        end
        return #out >= 300
    end)
    return out
end

function CA.find(text, typeName)
    local found = nil
    local lt = text and text:lower()
    CA.walk(function(el)
        local label = el.title or el.name or el.internal or el.text
        local okLabel = (not lt) or (type(label) == "string" and label:lower() == lt)
        local okType = (not typeName) or el.Type == typeName
        if okLabel and okType then found = el; return true end
        return false
    end)
    if found or not lt then return found end
    -- second pass: substring match
    CA.walk(function(el)
        local label = el.title or el.name or el.internal or el.text
        if type(label) == "string" and label:lower():find(lt, 1, true) and ((not typeName) or el.Type == typeName) then
            found = el; return true
        end
        return false
    end)
    return found
end

-- Click a button / menu label by visible text (exact first, then substring).
function CA.click(text, typeName)
    local el = CA.find(text, typeName)
    if not el then return "not found: " .. tostring(text) end
    if el.forceClick then el:forceClick(); return "clicked " .. tostring(el.Type) .. " " .. tostring(el.title or el.internal) end
    if el.onMouseDown then el:onMouseDown(1, 1); if el.onMouseUp then pcall(el.onMouseUp, el, 1, 1) end; return "mousedown " .. tostring(el.Type) end
    if el.onclick then el.onclick(el.target, el); return "onclick " .. tostring(el.Type) end
    return "no click handler on " .. tostring(el.Type)
end

-- Answer the front-most modal dialog. answer = "YES" | "NO" | "OK". Works when UI mods
-- hide the vanilla buttons (forceClick skips hidden buttons).
function CA.answer(answer)
    answer = (answer or "YES"):upper()
    local dlg = nil
    CA.walk(function(el)
        if (el.yes or el.ok) and el.Type ~= "ISButton" and ISModalDialog then dlg = el end
        return false
    end)
    if not dlg then return "no dialog" end
    local btn = { internal = answer, player = dlg.player, parent = dlg }
    if dlg.onClick then dlg:onClick(btn) else ISModalDialog.onClick(dlg, btn) end
    return "answered " .. answer .. " on " .. tostring(dlg.Type)
end

-- ---------------------------------------------------------------- state
function CA.state()
    local s = { ms = now() }
    s.ingame = getPlayer() ~= nil
    pcall(function() s.mainScreenVisible = MainScreen.instance and MainScreen.instance:getIsVisible() end)
    pcall(function() s.inGameMenu = MainScreen.instance and MainScreen.instance.inGame end)
    pcall(function() s.paused = isGamePaused() end)
    pcall(function() s.speed = getGameSpeed() end)
    pcall(function()
        local gt = getGameTime()
        if gt then s.time = string.format("%d-%02d-%02d %02d:%02d", gt:getYear(), gt:getMonth() + 1, gt:getDay() + 1, gt:getHour(), gt:getMinutes()) end
    end)
    pcall(function()
        local p = getPlayer()
        if p then s.player = { x = p:getX(), y = p:getY(), z = p:getZ(), dead = p:isDead() } end
    end)
    return s
end

-- ---------------------------------------------------------------- research helpers
-- Run fn, return its value or "ERR <msg>" instead of failing the whole command.
function CA.try(fn, ...)
    local ok, res = pcall(fn, ...)
    if ok then return res end
    return "ERR " .. tostring(res)
end

-- Public Java methods of an object (debug reflection), optional substring filter.
function CA.methods(obj, filter)
    local out = {}
    local n = getNumClassFunctions(obj)
    for i = 0, n - 1 do
        local name = tostring(getClassFunction(obj, i))
        if not filter or name:lower():find(filter:lower(), 1, true) then out[#out + 1] = name end
    end
    table.sort(out)
    return out
end

-- Java fields with values (debug reflection), optional substring filter.
function CA.fields(obj, filter)
    local out = {}
    local n = getNumClassFields(obj)
    for i = 0, n - 1 do
        local f = getClassField(obj, i)
        local name = tostring(f)
        if not filter or name:lower():find(filter:lower(), 1, true) then
            local ok, v = pcall(getClassFieldVal, obj, f)
            out[name:match("([%w_]+)$") or name] = ok and tostring(v) or "?"
        end
    end
    return out
end

function CA.stats(chr)
    chr = chr or getPlayer()
    local out = {}
    local st = chr:getStats()
    for _, k in ipairs({ "HUNGER", "THIRST", "FATIGUE", "ENDURANCE", "STRESS", "PANIC", "BOREDOM", "UNHAPPINESS",
        "PAIN", "SICKNESS", "FOOD_SICKNESS", "INTOXICATION", "ANGER", "DISCOMFORT", "IDLENESS", "MORALE", "SANITY",
        "TEMPERATURE", "WETNESS", "FITNESS", "ZOMBIE_INFECTION", "ZOMBIE_FEVER", "NICOTINE_WITHDRAWAL", "POISON" }) do
        local ok, v = pcall(function() return st:get(CharacterStat[k]) end)
        if ok and v ~= nil then out[k] = math.floor(v * 1000 + 0.5) / 1000 end
    end
    pcall(function() out.HEALTH = chr:getBodyDamage():getOverallBodyHealth() end)
    return out
end

-- 0 = pause, 1 = normal, 2 = x5, 3 = x20, 4 = x40 ("Wait"). Goes through the speed
-- widget's buttons: SetCurrentGameSpeed alone only changes the widget, not the clock.
-- The game itself refuses fast forward while zombies are near or the player is busy.
function CA.speed(n)
    local sc = UIManager.getSpeedControls()
    if not sc then return "no speed controls" end
    local names = { [0] = "Pause", [1] = "Play", [2] = "Fast Forward x 1", [3] = "Fast Forward x 2", [4] = "Wait" }
    if n == 0 then
        if sc:getCurrentGameSpeed() ~= 0 then sc:ButtonClicked("Pause") end
    else
        -- "Play" also drops back to x1 from fast forward (only pressing it when paused left x20 running)
        if n == 1 or sc:getCurrentGameSpeed() == 0 then sc:ButtonClicked("Play") end
        if n > 1 then sc:ButtonClicked(names[n]) end
    end
    return "speed " .. tostring(n) .. " x" .. tostring(getGameTime():getTrueMultiplier())
end

-- Queue a walk to a square (pathfinding through doors and around walls).
function CA.walkTo(x, y, z)
    local p = getPlayer()
    local sq = getCell():getGridSquare(x, y, z or 0)
    if not sq then return "square not loaded" end
    ISTimedActionQueue.clear(p)
    ISTimedActionQueue.add(ISWalkToTimedAction:new(p, sq))
    return "walking to " .. x .. "," .. y .. "," .. (z or 0)
end

-- Zombies near the player with distance, state and whether they target the player.
function CA.zombies(radius)
    radius = radius or 30
    local p = getPlayer()
    local out = {}
    local zl = getCell():getZombieList()
    for i = 0, zl:size() - 1 do
        local z = zl:get(i)
        local d = z:getDistanceSq(p) ^ 0.5
        if d <= radius then
            local e = { d = math.floor(d * 10) / 10, x = math.floor(z:getX()), y = math.floor(z:getY()), z = math.floor(z:getZ()) }
            pcall(function() e.target = z:getTarget() == p end)
            pcall(function() e.state = tostring(z:getCurrentState()):match("([%w_]+)@") or tostring(z:getCurrentState()) end)
            pcall(function() e.canSee = z:CanSee(p) end)
            pcall(function() e.crawl = z:isCrawling() end)
            pcall(function()
                local zb = z:getCurrentSquare():getBuilding()
                local pb = p:getCurrentSquare():getBuilding()
                e.inside = zb ~= nil
                e.sameBuilding = zb ~= nil and zb == pb
            end)
            out[#out + 1] = e
        end
    end
    table.sort(out, function(a, b) return a.d < b.d end)
    return out
end

-- ---------------------------------------------------------------- play actions
-- These queue the same timed actions a player's clicks queue (animations, time, noise).

-- Find an item by display name (substring) in containers within `radius` squares.
function CA.findItem(name, radius)
    radius = radius or 6
    local p = getPlayer()
    local cell = getCell()
    local px, py, pz = math.floor(p:getX()), math.floor(p:getY()), math.floor(p:getZ())
    local lname = name:lower()
    for x = px - radius, px + radius do
        for y = py - radius, py + radius do
            local sq = cell:getGridSquare(x, y, pz)
            if sq then
                local objs = sq:getObjects()
                for k = 0, objs:size() - 1 do
                    local o = objs:get(k)
                    for c = 0, o:getContainerCount() - 1 do
                        local cont = o:getContainerByIndex(c)
                        local items = cont:getItems()
                        for i = 0, items:size() - 1 do
                            local it = items:get(i)
                            if it:getDisplayName():lower():find(lname, 1, true) or it:getFullType():lower():find(lname, 1, true) then return it, cont, sq end
                        end
                    end
                end
            end
        end
    end
    return nil
end

-- Walk to the container, then move the item to the main inventory.
function CA.take(name, radius)
    local p = getPlayer()
    local it, cont, sq = CA.findItem(name, radius)
    if not it then return "no " .. name .. " nearby" end
    -- furniture squares are not walkable: walk next to it, as the game's own loot clicks do
    luautils.walkAdj(p, sq, true)
    ISTimedActionQueue.add(ISInventoryTransferAction:new(p, it, cont, p:getInventory()))
    return "taking " .. it:getDisplayName() .. " from " .. cont:getType()
end

function CA.invItem(name)
    local items = getPlayer():getInventory():getItems()
    local lname = name:lower()
    for i = 0, items:size() - 1 do
        local it = items:get(i)
        if it:getDisplayName():lower():find(lname, 1, true) or it:getFullType():lower():find(lname, 1, true) then return it end
    end
end

function CA.equip(name, twoHands)
    local p = getPlayer()
    local it = CA.invItem(name)
    if not it then return "not carrying " .. name end
    ISTimedActionQueue.add(ISEquipWeaponAction:new(p, it, 50, true, twoHands or it:isTwoHandWeapon()))
    return "equipping " .. it:getDisplayName()
end

function CA.eat(name)
    local p = getPlayer()
    local it = CA.invItem(name)
    if not it then return "not carrying " .. name end
    -- 42.21: some snacks are ComboItem packs that must be opened first; ISEatFoodAction breaks on them
    if not instanceof(it, "Food") then return it:getFullType() .. " is not Food (" .. tostring(it:getClass()) .. "), open it first" end
    ISTimedActionQueue.add(ISEatFoodAction:new(p, it, 1))
    return "eating " .. it:getDisplayName()
end

function CA.inventory()
    local out = {}
    local items = getPlayer():getInventory():getItems()
    for i = 0, items:size() - 1 do out[#out + 1] = items:get(i):getDisplayName() end
    return out
end

-- Run a craft recipe the way an inventory right-click does (e.g. "RipClothing" on a shirt).
function CA.craft(recipeName, itemName)
    local p = getPlayer()
    local recipe = getScriptManager():getCraftRecipe(recipeName)
    if not recipe then return "no recipe " .. recipeName end
    local item = itemName and CA.invItem(itemName) or nil
    local logic = HandcraftLogic.new(p, nil, nil)
    logic:setContainers(ISInventoryPaneContextMenu.getContainers(p))
    if item then logic:setRecipeFromContextClick(recipe, item) else logic:setRecipe(recipe) end
    if not logic:canPerformCurrentRecipe() then return "cannot do " .. recipeName .. " now" end
    ISTimedActionQueue.add(ISHandcraftAction.FromLogic(logic))
    return "crafting " .. recipeName .. (item and (" from " .. item:getDisplayName()) or "")
end

-- Call a no-arg method if it exists (B41/B42 renamed several BodyPart methods).
local function has(obj, m)
    local ok, v = pcall(function() return obj[m](obj) end)
    return ok and v
end

-- Wounds per body part.
function CA.wounds()
    local out = {}
    local parts = getPlayer():getBodyDamage():getBodyParts()
    for i = 0, parts:size() - 1 do
        local b = parts:get(i)
        local w = {}
        if has(b, "bleeding") then w.bleeding = true end
        if has(b, "bitten") then w.bite = true end
        if has(b, "scratched") then w.scratch = true end
        if has(b, "isCut") or (has(b, "getCutTime") or 0) > 0 then w.laceration = true end
        if has(b, "isDeepWounded") then w.deep = true end
        if has(b, "bandaged") then w.bandaged = true end
        local hp = has(b, "getHealth")
        if type(hp) == "number" and hp < 100 then w.hp = math.floor(hp) end
        local any = false
        for _ in pairs(w) do any = true end
        if any then out[tostring(b:getType())] = w end
    end
    return out
end

-- Bandage every bleeding or open wound with rags/bandages from the inventory.
function CA.bandageAll()
    local p = getPlayer()
    local parts = p:getBodyDamage():getBodyParts()
    local inv = p:getInventory()
    local queued = {}
    local used = {}
    for i = 0, parts:size() - 1 do
        local b = parts:get(i)
        if not has(b, "bandaged") and (has(b, "bleeding") or has(b, "scratched") or has(b, "isCut") or (has(b, "getCutTime") or 0) > 0 or has(b, "bitten") or has(b, "isDeepWounded")) then
            local band = nil
            local items = inv:getItems()
            for k = 0, items:size() - 1 do
                local it = items:get(k)
                if it:isCanBandage() and not used[it] then band = it; break end
            end
            if not band then queued[#queued + 1] = "no bandage for " .. tostring(b:getType()); break end
            used[band] = true
            ISTimedActionQueue.add(ISApplyBandage:new(p, p, band, b, true))
            queued[#queued + 1] = tostring(b:getType()) .. " <- " .. band:getDisplayName()
        end
    end
    return queued
end

-- ---------------------------------------------------------------- moodles

-- How to clear each vanilla moodle (B42.21 MoodleType constants). Level 1..4, 4 = worst.
CA.MOODLE_FIX = {
    HUNGRY = "eat: CA.findFood / CA.eat",
    THIRST = "drink: CA.drink (sink, toilet tank, rain barrel)",
    TIRED = "sleep: CA.sleep (bed); coffee/caffeine pills short term",
    ENDURANCE = "stop running/fighting; sit or stand still",
    PAIN = "remove the cause (soiled clothes, wounds -> CA.bandageAll); painkillers; sleep",
    INJURED = "CA.wounds then CA.bandageAll; disinfect deep wounds",
    BLEEDING = "CA.bandageAll NOW",
    UNHAPPY = "read magazines/books, eat good food, wash; avoid stale/burnt food",
    BORED = "read, listen to radio/TV, change activity",
    STRESS = "leave danger; cigarettes (smoker), alcohol, read",
    PANIC = "get away from zombies (CA.flee), lose line of sight",
    SICK = "rest, eat light, avoid rotten food; antibiotics if infected wound",
    HAS_A_COLD = "stay warm and dry, sleep, eat",
    WET = "go indoors, towel / dry clothes, change wet clothes",
    HYPOTHERMIA = "warm clothes, indoors, heat source",
    HYPERTHERMIA = "remove layers, shade, drink",
    WINDCHILL = "get out of the wind / wear a jacket",
    HEAVY_LOAD = "drop items or use a bag",
    DRUNK = "wait it out, don't fight",
    UNCOMFORTABLE = "change dirty/wet/bloody clothes; wash (Nature's Call grime/soilage)",
    NOXIOUS_SMELL = "move away from corpses/rot; clear the room",
    ANGRY = "wait; avoid stressors",
    CANT_SPRINT = "rest legs",
    ZOMBIE = "infected: nothing cures it in vanilla",
}

-- Active vanilla moodles {name = level}, MoodleFramework moodles with level ~= 0,
-- and a fix hint per moodle, worst first.
function CA.moodles(chr)
    chr = chr or getPlayer()
    local m = chr:getMoodles()
    local out = { list = {} }
    for k, fix in pairs(CA.MOODLE_FIX) do
        local ok, lv = pcall(function() return m:getMoodleLevel(MoodleType[k]) end)
        if ok and lv and lv > 0 then out.list[#out.list + 1] = { name = k, level = lv, fix = fix } end
    end
    if MF and MF.createdMoodleTypes then
        for name, _ in pairs(MF.createdMoodleTypes) do
            local ok, mo = pcall(MF.getMoodle, name)
            if ok and mo and mo:getLevel() ~= 0 then
                out.list[#out.list + 1] = { name = name, level = mo:getLevel(), value = mo:getValue(), fix = "MoodleFramework moodle" }
            end
        end
    end
    pcall(function() out.naturesCall = CA.natureCall(chr) end)
    table.sort(out.list, function(a, b) return math.abs(a.level) > math.abs(b.level) end)
    return out
end

-- Nature's Call needs (player modData.NaturesCall). Fill 0..120: moodle 1 at 70, 2 at 95,
-- 3 over 100 (pain + accidents). Body soilage 0..100 gives rash pain up to 25 (urine) + 45 (feces).
function CA.natureCall(chr)
    chr = chr or getPlayer()
    local nc = chr:getModData().NaturesCall
    if not nc then return nil end
    local r = function(v) return v and math.floor(v + 0.5) end
    local out = { bladder = r(nc.bladderFill), bowel = r(nc.bowelFill), control = nc.control,
        wipe = nc.wipeRequired and nc.wipeKind or nil, moodles = {}, soiledWorn = {} }
    if nc.bodySoilage then out.bodyUrine, out.bodyFeces = r(nc.bodySoilage.urine), r(nc.bodySoilage.feces) end
    if nc.moodles then
        for k, v in pairs(nc.moodles) do if type(v) == "number" and v > 0 then out.moodles[k] = v end end
    end
    out.grime = r(chr:getModData().tabas_BodyGrime)
    local w = chr:getWornItems()
    for i = 0, w:size() - 1 do
        local it = w:get(i):getItem()
        local s = it:getModData().NaturesCall
        s = s and s.soilage
        if s and ((s.urine or 0) > 0 or (s.feces or 0) > 0) then
            out.soiledWorn[#out.soiledWorn + 1] = it:getFullType() .. " u=" .. r(s.urine or 0) .. " f=" .. r(s.feces or 0)
        end
    end
    local fix = {}
    if (nc.bladderFill or 0) >= 70 then fix[#fix + 1] = "CA.relieve('urine')" end
    if (nc.bowelFill or 0) >= 70 then fix[#fix + 1] = "CA.relieve('feces')" end
    if #out.soiledWorn > 0 then fix[#fix + 1] = "CA.undress(soiled) + CA.wear(clean)" end
    if (out.bodyUrine or 0) > 0 or (out.bodyFeces or 0) > 0 or (out.grime or 0) >= 45 then fix[#fix + 1] = "CA.washBody()" end
    out.fix = fix
    return out
end

-- Nearest Nature's Call target ("toilet"/"sink") within radius, as the mod's own detector sees it.
function CA.ncTarget(kind, radius)
    local found = nil
    CA.eachObject(radius or 50, function(o, sq)
        local ok, t = pcall(NaturesCall.ActionCore.getTargetType, sq)
        if ok and t == kind then found = sq; return true end
    end)
    return found
end

-- Pee or poop ("urine"/"feces") at the nearest toilet; outside on the ground when there is none
-- within `radius` (costs 10/15 unhappiness). Wipes automatically when supplies are carried.
function CA.relieve(kind, radius)
    local p = getPlayer()
    kind = kind or "urine"
    local sq = CA.ncTarget("toilet", radius)
    if sq then
        luautils.walk(p, NaturesCall.ToiletUtils.getUseSquare(p, sq))
        ISTimedActionQueue.add(NaturesCallEmptyAction:new(p, kind, "toilet", sq, true, false, nil))
        return kind .. " at toilet " .. sq:getX() .. "," .. sq:getY()
    end
    ISTimedActionQueue.add(NaturesCallEmptyAction:new(p, kind, "ground", p:getCurrentSquare(), true, true, nil))
    return kind .. " on the ground here"
end

-- Wash the body at the nearest sink (clears Nature's Call body soilage, which causes rash pain).
function CA.washBody(radius)
    local p = getPlayer()
    local src = nil
    CA.eachObject(radius or 20, function(o)
        if o:hasWater() then src = o; return true end
    end)
    if not src then return "no water nearby" end
    luautils.walkAdjObject(p, src, true, true)
    ISTimedActionQueue.add(NaturesCallCleanBodyAction:new(p, "wash", src, nil, 3))
    local ok, water = pcall(function() return src:getFluidAmount() end)
    return "washing at " .. src:getSquare():getX() .. "," .. src:getSquare():getY() .. " water=" .. tostring(ok and water or "?")
end

-- Take off a worn item by name/type substring (also drops Nature's Call stain overlays with it).
function CA.undress(name)
    local p = getPlayer()
    local w = p:getWornItems()
    local lname = name:lower()
    local done = {}
    for i = w:size() - 1, 0, -1 do
        local it = w:get(i):getItem()
        if it:getFullType():lower():find(lname, 1, true) or it:getDisplayName():lower():find(lname, 1, true) then
            ISTimedActionQueue.add(ISUnequipAction:new(p, it, 50))
            done[#done + 1] = it:getFullType()
        end
    end
    return #done > 0 and done or ("not wearing " .. name)
end

-- Put on a clothing item carried in the inventory.
function CA.wear(name)
    local p = getPlayer()
    local it = CA.invItem(name)
    if not it then return "not carrying " .. name end
    ISTimedActionQueue.add(ISWearClothing:new(p, it, 50))
    return "wearing " .. it:getFullType()
end

-- ---------------------------------------------------------------- survival helpers

-- Drink from the nearest water source (sink, toilet, well...) within `radius`.
function CA.drink(radius)
    local p = getPlayer()
    local src = nil
    CA.eachObject(radius or 10, function(o)
        if o:hasWater() then src = o; return true end
    end)
    if not src then return "no water nearby" end
    if not luautils.walkAdjObject(p, src, true, true) then return "cannot reach water" end
    ISTimedActionQueue.add(ISTakeWaterAction:new(p, nil, src, src:isTaintedWater()))
    return "drinking at " .. src:getSquare():getX() .. "," .. src:getSquare():getY() .. (src:isTaintedWater() and " (tainted)" or "")
end

-- The door or window object on square x,y (z defaults to the player's level).
function CA.doorAt(x, y, z)
    local sq = getCell():getGridSquare(x, y, z or math.floor(getPlayer():getZ()))
    if not sq then return nil end
    local objs = sq:getObjects()
    for k = 0, objs:size() - 1 do
        local o = objs:get(k)
        if instanceof(o, "IsoDoor") or instanceof(o, "IsoWindow") then return o end
    end
end

-- Walk to a door/window and open (want=true) or close (want=false) it, like a click.
function CA.door(x, y, want)
    local p = getPlayer()
    local d = CA.doorAt(x, y)
    if not d then return "no door/window at " .. x .. "," .. y end
    if want == nil then want = not d:IsOpen() end
    if d:IsOpen() == want then return "already " .. (want and "open" or "closed") end
    if d:isLocked() and want then return "locked" end
    luautils.walkAdjWindowOrDoor(p, d:getSquare(), d)
    if instanceof(d, "IsoDoor") then
        ISTimedActionQueue.add(ISOpenCloseDoor:new(p, d))
    else
        ISTimedActionQueue.add(ISOpenCloseWindow:new(p, d))
    end
    return (want and "opening " or "closing ") .. (instanceof(d, "IsoDoor") and "door" or "window")
end

-- Buildings with a square within `radius` (sampled every 3 squares), nearest first.
function CA.buildings(radius)
    radius = radius or 60
    local p = getPlayer()
    local cell = getCell()
    local px, py = math.floor(p:getX()), math.floor(p:getY())
    local seen, out = {}, {}
    for x = px - radius, px + radius, 3 do
        for y = py - radius, py + radius, 3 do
            local sq = cell:getGridSquare(x, y, 0)
            local b = sq and sq:getBuilding()
            if b then
                local def = b:getDef()
                local key = def:getX() .. "," .. def:getY()
                if not seen[key] then
                    seen[key] = true
                    local cx, cy = def:getX() + def:getW() / 2, def:getY() + def:getH() / 2
                    local rooms = {}
                    for i = 0, def:getRooms():size() - 1 do rooms[#rooms + 1] = def:getRooms():get(i):getName() end
                    out[#out + 1] = { x = def:getX(), y = def:getY(), w = def:getW(), h = def:getH(),
                        d = math.floor(math.sqrt((cx - px) ^ 2 + (cy - py) ^ 2)), rooms = rooms }
                end
            end
        end
    end
    table.sort(out, function(a, b) return a.d < b.d end)
    return out
end

-- Visit every object within `radius` on the player's level, nearest rings first;
-- fn(obj, sq) returning true stops.
function CA.eachObject(radius, fn, z)
    local p = getPlayer()
    local cell = getCell()
    local px, py = math.floor(p:getX()), math.floor(p:getY())
    z = z or math.floor(p:getZ())
    for r = 0, radius do
        for x = px - r, px + r do
            for y = py - r, py + r do
                if math.abs(x - px) == r or math.abs(y - py) == r then
                    local sq = cell:getGridSquare(x, y, z)
                    if sq then
                        local objs = sq:getObjects()
                        for k = 0, objs:size() - 1 do
                            if fn(objs:get(k), sq) then return end
                        end
                    end
                end
            end
        end
    end
end

-- Walk into a building (nearest free square of one of its rooms) given a CA.buildings() entry.
function CA.enter(b)
    local p = getPlayer()
    local cell = getCell()
    local best, bd = nil, 1e9
    for x = b.x, b.x + b.w do
        for y = b.y, b.y + b.h do
            local sq = cell:getGridSquare(x, y, 0)
            if sq and sq:getRoom() and sq:isFree(false) then
                local d = (x - p:getX()) ^ 2 + (y - p:getY()) ^ 2
                if d < bd then best, bd = sq, d end
            end
        end
    end
    if not best then return "no free room square" end
    ISTimedActionQueue.clear(p)
    ISTimedActionQueue.add(ISWalkToTimedAction:new(p, best))
    return "entering via " .. best:getX() .. "," .. best:getY() .. " (" .. best:getRoom():getName() .. ")"
end

-- Food that can be eaten as is (no can opener), in containers within `radius`.
function CA.findFood(radius)
    local out = {}
    CA.eachObject(radius or 10, function(o, sq)
        for c = 0, o:getContainerCount() - 1 do
            local cont = o:getContainerByIndex(c)
            local items = cont:getItems()
            for i = 0, items:size() - 1 do
                local it = items:get(i)
                if instanceof(it, "Food") and not it:isRotten() and (it:getHungerChange() or 0) < 0 then
                    local ft = it:getFullType()
                    local canned = ft:find("Canned") or ft:find("Tin")
                    if not canned then
                        out[#out + 1] = { name = it:getDisplayName(), hunger = it:getHungerChange(), cont = cont:getType(), x = sq:getX(), y = sq:getY() }
                    end
                end
            end
        end
    end)
    return out
end

-- Sleep in the nearest bed within `radius`, or on the floor. Mirrors the bed context menu
-- (it refuses when zombies are visible, panic is up, or pain is high).
function CA.sleep(radius)
    local p = getPlayer()
    local bed = nil
    CA.eachObject(radius or 15, function(o)
        local props = o:getProperties()
        if props and props:has("BedType") then bed = o; return true end
    end)
    if not bed then
        ISWorldObjectContextMenu.onSleepWalkToComplete(p:getPlayerNum(), nil)
        return "sleeping on the floor, asleep=" .. tostring(p:isAsleep())
    end
    local adjacent = AdjacentFreeTileFinder.Find(bed:getSquare(), p)
    if AdjacentFreeTileFinder.isTileOrAdjacent(p:getCurrentSquare(), bed:getSquare()) or not adjacent then
        ISWorldObjectContextMenu.onSleepWalkToComplete(p:getPlayerNum(), bed)
        return "sleeping in " .. tostring(bed:getProperties():get("BedType")) .. ", asleep=" .. tostring(p:isAsleep())
    end
    local action = ISWalkToTimedAction:new(p, adjacent)
    action:setOnComplete(ISWorldObjectContextMenu.onSleepWalkToComplete, p:getPlayerNum(), bed)
    ISTimedActionQueue.add(action)
    return "walking to " .. tostring(bed:getProperties():get("BedType")) .. " at " .. bed:getSquare():getX() .. "," .. bed:getSquare():getY()
end

-- Long walk in hops (pathfinding only reaches loaded squares). Stops when there, or when
-- a zombie within `alert` squares targets the player. Progress in CA.travelLog.
function CA.travel(tx, ty, alert, hop)
    alert, hop = alert or 10, hop or 30
    local p = getPlayer()
    local log = { from = { math.floor(p:getX()), math.floor(p:getY()) }, to = { tx, ty }, hops = 0, start = CA.state().time }
    CA.travelLog = log
    CA.task("travel", 500, function()
        local dx, dy = tx - p:getX(), ty - p:getY()
        local dist = math.sqrt(dx * dx + dy * dy)
        log.left = math.floor(dist)
        for _, z in ipairs(CA.zombies(alert)) do
            -- a zombie thumping a door it cannot see past is noise, not a threat yet
            if z.target and z.state ~= "ThumpState" and not (z.inside and not z.sameBuilding) then
                ISTimedActionQueue.clear(p)
                log.result = string.format("zombie on me at %.1f", z.d)
                log.at = CA.state().time
                return "stop"
            end
        end
        if dist < 3 then log.result = "arrived"; log.at = CA.state().time; return "stop" end
        if CA.busy().actions > 0 then return end
        local step = math.min(hop, dist)
        local hx, hy = p:getX() + dx / dist * step, p:getY() + dy / dist * step
        local cell = getCell()
        for r = 0, 6 do
            for ox = -r, r do
                for oy = -r, r do
                    local sq = cell:getGridSquare(math.floor(hx + ox), math.floor(hy + oy), 0)
                    if sq and sq:isFree(false) then
                        log.hops = log.hops + 1
                        ISTimedActionQueue.add(ISWalkToTimedAction:new(p, sq))
                        return
                    end
                end
            end
        end
        log.result = "no free square ahead"
        return "stop"
    end)
    return "travel to " .. tx .. "," .. ty
end

-- What the player is doing right now.
function CA.busy()
    local p = getPlayer()
    local q = ISTimedActionQueue.getTimedActionQueue(p)
    local cur = q and q.queue and q.queue[1]
    return { actions = q and q.queue and #q.queue or 0, current = cur and cur.Type or nil, moving = p:isPlayerMoving() }
end

-- Sprint `dist` squares straight away from the centroid of nearby zombies.
function CA.flee(dist)
    dist = dist or 10
    local p = getPlayer()
    local zs = CA.zombies(8)
    local cx, cy, n = 0, 0, 0
    for _, z in ipairs(zs) do cx, cy, n = cx + z.x, cy + z.y, n + 1 end
    if n == 0 then return "nothing to flee" end
    cx, cy = cx / n, cy / n
    local dx, dy = p:getX() - cx, p:getY() - cy
    local len = math.max(0.1, math.sqrt(dx * dx + dy * dy))
    local tx, ty = math.floor(p:getX() + dx / len * dist), math.floor(p:getY() + dy / len * dist)
    -- search outward for a walkable free square near the target point
    local cell = getCell()
    for r = 0, 4 do
        for ox = -r, r do
            for oy = -r, r do
                local sq = cell:getGridSquare(tx + ox, ty + oy, p:getZ())
                if sq and sq:isFree(false) then
                    ISTimedActionQueue.clear(p)
                    p:setSprinting(true)
                    p:setRunning(true)
                    ISTimedActionQueue.add(ISWalkToTimedAction:new(p, sq))
                    return "fleeing to " .. sq:getX() .. "," .. sq:getY()
                end
            end
        end
    end
    return "no free square to flee to"
end

-- ---------------------------------------------------------------- melee bot
-- Engage zombies within `radius` one at a time with the equipped weapon.
-- Stops when none are left, health drops below `minHealth`, or `crowd` zombies are close.
-- Writes a running record to CA.fightLog (read with `return CA.fightLog`).
function CA.fight(radius, minHealth, crowd)
    radius, minHealth, crowd = radius or 25, minHealth or 60, crowd or 3
    local p = getPlayer()
    local log = { start = CA.state().time, swings = 0, kills = 0, events = {} }
    CA.fightLog = log
    local weapon = p:getPrimaryHandItem()
    local reach = weapon and weapon.getMaxRange and weapon:getMaxRange() or 1.0
    log.weapon = weapon and weapon:getDisplayName() or "hands"
    log.reach = reach
    local target, lastWalk, startKills = nil, 0, p:getZombieKills()
    local function ev(s) log.events[#log.events + 1] = CA.state().time .. " " .. s end
    CA.task("fight", 150, function()
        if p:isDead() then ev("player died"); log.result = "dead"; return "stop" end
        local hp = p:getBodyDamage():getOverallBodyHealth()
        log.health = hp
        log.kills = p:getZombieKills() - startKills
        if hp < minHealth then ev("health " .. math.floor(hp) .. ", disengaging"); log.result = "low health"; p:setIsAiming(false); CA.flee(12); return "stop" end
        local close, nearest, nd = 0, nil, 1e9
        local zl = getCell():getZombieList()
        for i = 0, zl:size() - 1 do
            local z = zl:get(i)
            if not z:isDead() and z:getZ() == p:getZ() then
                local d = z:getDistanceSq(p) ^ 0.5
                if d < 3 then close = close + 1 end
                if d < nd and d <= radius then nearest, nd = z, d end
            end
        end
        local endurance = p:getStats():get(CharacterStat.ENDURANCE)
        log.endurance = endurance
        if p:isBeingGrappled() and not log.grappled then log.grappled = true; ev("grabbed") end
        if close >= crowd or endurance < 0.25 then
            ev(close .. " close, endurance " .. string.format("%.2f", endurance) .. ", fleeing")
            log.result = "fled"
            p:setIsAiming(false)
            CA.flee(10)
            return "stop"
        end
        if not nearest then ev("no zombies left within " .. radius); log.result = "clear"; p:setIsAiming(false); return "stop" end
        if nearest ~= target then target = nearest; ev(string.format("target at %.1f", nd)) end
        if p:isPerformingAttackAnimation() then return end
        if close >= 2 and nd < 1.2 then
            -- two at arm's length: shove the nearest to make room instead of swinging
            ISTimedActionQueue.clear(p)
            p:faceThisObject(nearest)
            p:setIsAiming(true)
            p:setDoShove(true)
            p:DoAttack(0)
            log.shoves = (log.shoves or 0) + 1
            return
        end
        p:setDoShove(false)
        if nd > reach * 0.9 then
            p:setIsAiming(false)
            if getTimeInMillis() - lastWalk > 700 then
                lastWalk = getTimeInMillis()
                ISTimedActionQueue.clear(p)
                ISTimedActionQueue.add(ISWalkToTimedAction:new(p, nearest:getCurrentSquare()))
            end
        else
            ISTimedActionQueue.clear(p)
            p:faceThisObject(nearest)
            p:setIsAiming(true)
            if not p:isAttackStarted() then
                p:DoAttack(0)
                log.swings = log.swings + 1
            end
        end
    end)
    return "fighting with " .. log.weapon .. " reach " .. tostring(reach)
end

-- Repeating task: fn() runs every `ms` milliseconds until it returns "stop" or CA.untask(name).
function CA.task(name, ms, fn) CA.tasks[name] = { ms = ms, fn = fn, last = 0 } return "task " .. name end
function CA.untask(name) CA.tasks[name] = nil return "untask " .. name end

-- ---------------------------------------------------------------- poll
local function runTasks()
    local t = now()
    for name, task in pairs(CA.tasks) do
        if t - task.last >= task.ms then
            task.last = t
            local ok, res = pcall(task.fn)
            if not ok then CA.log("task " .. name .. " error: " .. tostring(res)); CA.tasks[name] = nil
            elseif res == "stop" then CA.tasks[name] = nil end
        end
    end
end

-- On a multiplayer server only admin/staff may run commands (the same rule as the bridge).
function CA.refusedInMP()
    if not (isClient and isClient()) then return false end
    local player = getPlayer and getPlayer()
    if not player then return true end
    local ok, level = pcall(function() return player:getAccessLevel() end)
    level = ok and level and tostring(level):lower() or "none"
    return level == "none" or level == "" or level == "player"
end

local function hasTasks()
    for _ in pairs(CA.tasks) do return true end
    return false
end

function CA.poll()
    local t = now()
    -- Ten reads a second while an agent is talking (a command in the last minute, or a task
    -- running), two a second otherwise. Each read opens the command file, and a slow disk or an
    -- antivirus can hold that for 30 ms: a visible hitch when it happens ten times a second for
    -- the whole session, measured on a real list (Smooth Frames: worst frame 34 ms).
    -- (Kahlua has no `next`: a pairs loop says whether a task is running.)
    local every = (t - CA.lastCmdMs < 60000 or hasTasks()) and 100 or 500
    if t - CA.lastPollMs < every then return end
    CA.lastPollMs = t
    -- the debug toolstrip turns break-on-error back on when it is created
    if CA.debugQuiet ~= false and isDebugEnabled() and UIManager.isShowLuaDebuggerOnError and UIManager.isShowLuaDebuggerOnError() then UIManager.setShowLuaDebuggerOnError(false) end
    runTasks()
    local src = readAll(CA.CMD)
    if not src or src == "" then return end
    local id = src:match("^%-%-id:(%S+)")
    if not id or id == CA.lastId then return end
    CA.lastId = id
    CA.lastCmdMs = t
    local fn, err = loadstring(src)
    local status, result
    if CA.refusedInMP() then
        -- the command file can be written by any local program: on a server it would be a cheat
        status, result = "err", "refused: Claude Agent commands only run in single player or for admin/staff"
    elseif not fn then
        status, result = "err", "syntax: " .. tostring(err)
    else
        local ok, res = pcall(fn)
        status = ok and "ok" or "err"
        result = ok and CA.ser(res, 6) or tostring(res)
    end
    writeAll(CA.OUT, "id:" .. id .. "\n" .. status .. "\n" .. tostring(result))
end

-- CA.poll is wrapped so a hot reload (sending this file as a command) swaps the code
-- without adding the event handlers twice.
if not CA.registered then
    CA.registered = true
    -- under -debug every Lua error (even inside pcall) opens the debugger and freezes the
    -- game; an agent probing APIs hits those all the time, so keep the debugger closed
    if isDebugEnabled() and UIManager.setShowLuaDebuggerOnError then UIManager.setShowLuaDebuggerOnError(false) end
    writeAll(CA.LOG, "boot " .. tostring(now()) .. "\n", false)
    -- the command left over from the last session already ran then: never run it again at boot
    local left = readAll(CA.CMD)
    CA.lastId = left and left:match("^%-%-id:(%S+)") or nil
    local function pollProxy() CA.poll() end
    if Events.OnFETick then Events.OnFETick.Add(pollProxy) end
    if Events.OnTickEvenPaused then Events.OnTickEvenPaused.Add(pollProxy) end
    if Events.OnPreUIDraw then Events.OnPreUIDraw.Add(pollProxy) end
end
print("[ClaudeAgent] ready")
