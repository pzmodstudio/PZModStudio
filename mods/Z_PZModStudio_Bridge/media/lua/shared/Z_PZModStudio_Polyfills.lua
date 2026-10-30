-- ============================================================================
-- Z_PZModStudio_Polyfills.lua (PZ Mod Studio Build 41/42+ Universal Runtime Shim)
-- Architecture: Safe Hooking via Events.OnGameBoot, FluidContainer Proxy, Translator Protection & Metatable Proxies
-- ============================================================================

if not Z_PZModStudio_Polyfills then Z_PZModStudio_Polyfills = {} end
local Poly = Z_PZModStudio_Polyfills

-- ----------------------------------------------------------------------------
-- 1. Translator Protection & Fallback Translations
-- ----------------------------------------------------------------------------
local FALLBACK_TRANSLATIONS = {
    ["IGUI_ItemCat_GunMag"] = "Magazine",
    ["IGUI_ItemCat_WeaponPart"] = "Weapon Part",
    ["IGUI_ItemCat_Firearm"] = "Firearm",
    ["IGUI_ItemCat_Ammo"] = "Ammo",
    ["IGUI_ItemCat_Melee"] = "Melee Weapon",
    ["IGUI_ItemCat_Explosive"] = "Explosive",
    ["IGUI_ItemCat_Accessory"] = "Accessory",
}

local function isLikelyPlainText(str, numArgs)
    if type(str) ~= "string" then return true end
    if str == "" then return true end
    if numArgs > 0 then return false end
    -- Translation keys in PZ never contain spaces. Strings containing spaces or % without format args
    -- are pre-formatted text; passing them to PZ Java Translator triggers reportMissingArgumentsFromPastAbuse
    -- and crashes with IllegalFormatConversionException (e.g. "% GRATIS" -> "% G" float format specifier).
    if string.find(str, "%s") or string.find(str, "%%") then
        return true
    end
    return false
end

function Poly.safeGetText(str, ...)
    if not str then return "" end
    if type(str) ~= "string" then return tostring(str) end
    if FALLBACK_TRANSLATIONS[str] then
        return FALLBACK_TRANSLATIONS[str]
    end
    if isLikelyPlainText(str, select("#", ...)) then
        return str
    end
    if zombie and zombie.core and zombie.core.Translator and zombie.core.Translator.getText then
        local status, res = pcall(zombie.core.Translator.getText, str, ...)
        if status and res ~= nil and res ~= str then return res end
    end
    if FALLBACK_TRANSLATIONS[str] then return FALLBACK_TRANSLATIONS[str] end
    return tostring(str)
end

if _G and _G.getText then
    local orig_gt = _G.getText
    _G.getText = function(str, ...)
        if not str then return "" end
        if type(str) ~= "string" then return tostring(str) end
        if FALLBACK_TRANSLATIONS[str] then return FALLBACK_TRANSLATIONS[str] end
        if isLikelyPlainText(str, select("#", ...)) then
            return str
        end
        local status, result = pcall(orig_gt, str, ...)
        if status and result ~= nil and result ~= str then return result end
        if FALLBACK_TRANSLATIONS[str] then return FALLBACK_TRANSLATIONS[str] end
        return tostring(str)
    end
end

if zombie and zombie.core and zombie.core.Translator and zombie.core.Translator.getText then
    local orig_tr_gt = zombie.core.Translator.getText
    zombie.core.Translator.getText = function(str, ...)
        if not str then return "" end
        if type(str) ~= "string" then return tostring(str) end
        if FALLBACK_TRANSLATIONS[str] then return FALLBACK_TRANSLATIONS[str] end
        if isLikelyPlainText(str, select("#", ...)) then
            return str
        end
        local status, result = pcall(orig_tr_gt, str, ...)
        if status and result ~= nil and result ~= str then return result end
        if FALLBACK_TRANSLATIONS[str] then return FALLBACK_TRANSLATIONS[str] end
        return tostring(str)
    end
end

-- ----------------------------------------------------------------------------
-- 2. ProceduralDistributions Metatable Protection
-- ----------------------------------------------------------------------------
if ProceduralDistributions then
    if not ProceduralDistributions.list then ProceduralDistributions.list = {} end
    local p_meta = getmetatable(ProceduralDistributions.list) or {}
    local orig_index = p_meta.__index
    p_meta.__index = function(t, k)
        local val = rawget(t, k)
        if val then
            if type(val) == "table" and not val.junk then val.junk = { items = {} } end
            return val
        end
        if type(orig_index) == "function" then
            val = orig_index(t, k)
        elseif type(orig_index) == "table" then
            val = orig_index[k]
        end
        if not val then
            val = { items = {}, junk = { items = {} } }
            rawset(t, k, val)
        elseif type(val) == "table" and not val.junk then
            val.junk = { items = {} }
        end
        return val
    end
    setmetatable(ProceduralDistributions.list, p_meta)
    for k, v in pairs(ProceduralDistributions.list) do
        if type(v) == "table" and not v.junk then v.junk = { items = {} } end
    end
end

-- ----------------------------------------------------------------------------
-- 3. Java List to Lua Table Converter Utility
-- ----------------------------------------------------------------------------
function Poly.toLuaTable(javaListOrTable)
    if not javaListOrTable then return {} end
    if type(javaListOrTable) == "table" then return javaListOrTable end
    local tbl = {}
    if javaListOrTable.size and javaListOrTable.get then
        local sz = javaListOrTable:size()
        for i = 0, sz - 1 do
            local item = javaListOrTable:get(i)
            table.insert(tbl, item)
        end
    end
    return tbl
end

-- ----------------------------------------------------------------------------
-- 4. Anti-Feedback Loop Guard on Fluid / Water Events
-- ----------------------------------------------------------------------------
-- 4. Event Wrapping Helper with Bidirectional Add/Remove Mapping
-- ----------------------------------------------------------------------------
local Events = Events or triggerEvent
local function wrapEventAddRemove(eventObj, wrapperBuilder)
    if not eventObj or eventObj._pzms_wrapped then return end
    eventObj._pzms_wrapped = true
    local orig_add = eventObj.Add
    local orig_remove = eventObj.Remove
    local wrapperMap = setmetatable({}, { __mode = "k" })
    
    if orig_add then
        eventObj.Add = function(func)
            if type(func) ~= "function" then return end
            local wrapper = wrapperBuilder(func)
            wrapperMap[func] = wrapper
            -- The Studio's profiler times the wrapper; this tells it whose handler is inside.
            if PZStudioInnerOf then PZStudioInnerOf[wrapper] = func end
            orig_add(wrapper)
        end
    end
    
    if orig_remove then
        eventObj.Remove = function(func)
            if type(func) ~= "function" then return end
            local wrapper = wrapperMap[func]
            if wrapper then
                orig_remove(wrapper)
                wrapperMap[func] = nil
            else
                orig_remove(func)
            end
        end
    end
end

-- Anti-Feedback Loop Guard on Fluid / Water Events
if Events and Events.OnWaterAmountChange then
    local in_water_event = false
    wrapEventAddRemove(Events.OnWaterAmountChange, function(func)
        return function(...)
            if in_water_event then return end
            in_water_event = true
            local status, err = pcall(func, ...)
            in_water_event = false
            if not status and err then
                print("[PZ Mod Studio Polyfill] Handled safe early-bailout in OnWaterAmountChange: " .. tostring(err))
            end
        end
    end)
end

-- ----------------------------------------------------------------------------
-- 4.1 OnEquipPrimary / OnEquipSecondary Bridge (B41 (player, weapon) -> B42 (player))
-- ----------------------------------------------------------------------------
if Events and Events.OnEquipPrimary then
    wrapEventAddRemove(Events.OnEquipPrimary, function(func)
        return function(player, weapon, ...)
            local realWeapon = weapon
            if realWeapon == nil and player and player.getPrimaryHandItem then
                realWeapon = player:getPrimaryHandItem()
            end
            local status, err = pcall(func, player, realWeapon, ...)
            if not status and err then
                print("[PZ Mod Studio Polyfill] Handled safe OnEquipPrimary listener error: " .. tostring(err))
            end
        end
    end)
end

if Events and Events.OnEquipSecondary then
    wrapEventAddRemove(Events.OnEquipSecondary, function(func)
        return function(player, weapon, ...)
            local realWeapon = weapon
            if realWeapon == nil and player and player.getSecondaryHandItem then
                realWeapon = player:getSecondaryHandItem()
            end
            local status, err = pcall(func, player, realWeapon, ...)
            if not status and err then
                print("[PZ Mod Studio Polyfill] Handled safe OnEquipSecondary listener error: " .. tostring(err))
            end
        end
    end)
end

-- OnClothingUpdated and OnPlayerUpdate are not wrapped. They were, in a pcall meant to keep a
-- handler failing every frame out of the console, but in B42 an error inside pcall is logged
-- all the same, stack and all (checked 2026-10-22: 365 lines in 6 s either way). The game
-- already runs each handler on its own and logs its failure with the mod's stack; the wrapper
-- only added a call per handler per frame, and a Studio frame to every stack.

-- ----------------------------------------------------------------------------
-- 4.2 OnMainMenuEnter Protection
-- ----------------------------------------------------------------------------
if Events and Events.OnMainMenuEnter then
    wrapEventAddRemove(Events.OnMainMenuEnter, function(func)
        return function(...)
            local status, err = pcall(func, ...)
            if not status and err then
                print("[PZ Mod Studio Polyfill] Protected OnMainMenuEnter from mod crash: " .. tostring(err))
            end
        end
    end)
end

-- ----------------------------------------------------------------------------
-- 4.3 Safe getOnlinePlayers() Metatable Proxy (Shields against Java IndexOutOfBoundsException)
-- ----------------------------------------------------------------------------
if _G and _G.getOnlinePlayers and not _G._pzms_online_players_wrapped then
    _G._pzms_online_players_wrapped = true
    local orig_getOnlinePlayers = _G.getOnlinePlayers
    _G.getOnlinePlayers = function(...)
        local list = orig_getOnlinePlayers(...)
        if not list then return list end
        
        local proxy = {}
        local mt = {
            __index = function(t, k)
                if k == "get" then
                    return function(self, idx)
                        if idx == nil or type(idx) ~= "number" or idx < 0 then return nil end
                        local sz = 0
                        if list.size then
                            local ok, s = pcall(function() return list:size() end)
                            if ok and type(s) == "number" then sz = s end
                        end
                        if idx >= sz then return nil end
                        local ok, res = pcall(function() return list:get(idx) end)
                        if ok then return res end
                        return nil
                    end
                elseif k == "size" then
                    return function(self)
                        if list.size then
                            local ok, s = pcall(function() return list:size() end)
                            if ok and type(s) == "number" then return s end
                        end
                        return 0
                    end
                elseif k == "isEmpty" then
                    return function(self)
                        if list.size then
                            local ok, s = pcall(function() return list:size() end)
                            if ok and type(s) == "number" then return s == 0 end
                        end
                        return true
                    end
                elseif k == "_raw" or k == "rawList" then
                    return list
                else
                    local val = list[k]
                    if type(val) == "function" then
                        return function(self, ...)
                            return val(list, ...)
                        end
                    end
                    return val
                end
            end,
            __len = function(t)
                if list.size then
                    local ok, s = pcall(function() return list:size() end)
                    if ok and type(s) == "number" then return s end
                end
                return 0
            end
        }
        setmetatable(proxy, mt)
        return proxy
    end
end


-- ----------------------------------------------------------------------------
-- 4.5 Switches for the per-mod patches of part 7, and the shared ArrayList proxy
-- Any Lua that loads before the game boots can
-- turn one off: PZMS_PATCHES = { fancy_handwork = false }. A patch also runs only while its
-- mod is in the active list, where the mod's id is known; otherwise the mod's own globals
-- are the test, and they exist only when it loaded.
-- ----------------------------------------------------------------------------
Poly.patchDefaults = {
    daily_report = true,            -- Daily Report Journal (dSAG)
    gunfighter = true,              -- GunFighter equip wrapper
    ocd_inventory = true,           -- OCDInventory refresh
    fancy_handwork_ocd = true,      -- Fancy Handwork + OCDInventory hand swap
    bandits = true,                 -- Bandits / Bandits Week One
    ai_week_one = true,             -- AIWeekOne navigation
    fancy_handwork = true,          -- Fancy Handwork multiplayer bounds
    sauced_carts = true,            -- SaucedCarts / Pushable Carts
    exhaustive_power_rework = true, -- Exhaustive Power Rework server start
}

function Poly.modActive(ids)
    if not ids or #ids == 0 then return true end
    if not getActivatedMods then return true end
    local ok, active = pcall(getActivatedMods)
    if not ok or not active then return true end
    for _, id in ipairs(ids) do
        local found, has = pcall(function() return active:contains(id) end)
        if found and has then return true end
    end
    return false
end

function Poly.patchOn(name, ids)
    local switches = _G.PZMS_PATCHES
    if type(switches) == 'table' and switches[name] == false then return false end
    if Poly.patchDefaults[name] == false then return false end
    return Poly.modActive(ids)
end

-- Java ArrayList as a Lua table that never throws on a bad index; shared by parts 5 and 6.
local function createSafeArrayListProxy(rawList)
    if rawList == nil then
        if ArrayList and ArrayList.new then
            local ok, al = pcall(ArrayList.new)
            if ok and al then rawList = al end
        end
    end
    local proxy = {}
    setmetatable(proxy, {
        __index = function(t, k)
            if k == "get" then
                return function(self, idx)
                    if not rawList or type(idx) ~= "number" then return nil end
                    local okSize, size = pcall(function() return rawList:size() end)
                    if okSize and type(size) == "number" and (idx < 0 or idx >= size) then
                        return nil
                    end
                    local okGet, val = pcall(function() return rawList:get(idx) end)
                    if okGet then return val end
                    return nil
                end
            elseif k == "size" then
                return function(self)
                    if not rawList then return 0 end
                    local okSize, size = pcall(function() return rawList:size() end)
                    if okSize and type(size) == "number" then return size end
                    return 0
                end
            elseif k == "isEmpty" then
                return function(self)
                    if not rawList then return true end
                    local okEmpty, empty = pcall(function() return rawList:isEmpty() end)
                    if okEmpty then return empty end
                    local okSize, size = pcall(function() return rawList:size() end)
                    if okSize and type(size) == "number" then return size == 0 end
                    return true
                end
            elseif k == "contains" then
                return function(self, item)
                    if not rawList or item == nil then return false end
                    local ok, res = pcall(function() return rawList:contains(item) end)
                    return ok and res == true
                end
            elseif k == "add" then
                return function(self, item)
                    if not rawList then return false end
                    local ok, res = pcall(function() return rawList:add(item) end)
                    return ok and res == true
                end
            elseif k == "clear" then
                return function(self)
                    if not rawList then return end
                    pcall(function() rawList:clear() end)
                end
            end
            if rawList and rawList[k] then
                local m = rawList[k]
                if type(m) == "function" then
                    return function(self, ...)
                        local ok, res = pcall(m, rawList, ...)
                        if ok then return res end
                        return nil
                    end
                end
                return m
            end
            return nil
        end,
        __len = function()
            if not rawList then return 0 end
            local okSize, size = pcall(function() return rawList:size() end)
            return (okSize and size) or 0
        end
    })
    return proxy
end

-- ----------------------------------------------------------------------------
-- 5. Build 41 to Build 42 bridge: APIs that moved or changed shape
-- Runs on boot and again on later ticks; each wrap below is applied once only.
-- ----------------------------------------------------------------------------
local function initB42Bridge()
    -- FluidContainer & Drainable Bridge (getUsedDelta / setUsedDelta)
    -- initB42Polyfills runs again on later ticks: each wrap below is applied once only.
    if InventoryItem and not InventoryItem._pzms_useddelta then
        InventoryItem._pzms_useddelta = true
        local old_getUsedDelta = InventoryItem.getUsedDelta
        if old_getUsedDelta then
            InventoryItem.getUsedDelta = function(self)
                local status, res = pcall(old_getUsedDelta, self)
                if status and res ~= nil then return res end
                if self and self.getFluidContainer then
                    local fc = self:getFluidContainer()
                    if fc and fc.getAmount and fc.getCapacity then
                        local cap = fc:getCapacity()
                        if cap and cap > 0 then
                            return fc:getAmount() / cap
                        end
                    end
                end
                return 0
            end
        end

        local old_setUsedDelta = InventoryItem.setUsedDelta
        if old_setUsedDelta then
            InventoryItem.setUsedDelta = function(self, val)
                local status, res = pcall(old_setUsedDelta, self, val)
                if not status and self and self.getFluidContainer then
                    local fc = self:getFluidContainer()
                    if fc and fc.setAmount and fc.getCapacity then
                        local cap = fc:getCapacity()
                        if cap and cap > 0 then
                            pcall(fc.setAmount, fc, (val or 0) * cap)
                        end
                    end
                end
                return res
            end
        end
    end

    -- ----------------------------------------------------------------------------
    -- RecipeCodeOnCreate & BuildRecipeCode Polyfills (Prevents craft action freeze)
    -- ----------------------------------------------------------------------------
    if not RecipeCodeOnCreate then
        RecipeCodeOnCreate = {}
    end
    setmetatable(RecipeCodeOnCreate, {
        __index = function(t, k)
            local noop = function(...) end
            rawset(t, k, noop)
            return noop
        end
    })
    if _G then
        _G.RecipeCodeOnCreate = RecipeCodeOnCreate
    end

    if not RecipeCodeOnCreate.openMacAndCheese then
        RecipeCodeOnCreate.openMacAndCheese = function(data, character) end
    end
    if not RecipeCodeOnCreate.openAndEat then
        RecipeCodeOnCreate.openAndEat = function(data, character) end
    end
    if not RecipeCodeOnCreate.cutChicken then
        RecipeCodeOnCreate.cutChicken = function(data, character) end
    end

    if not BuildRecipeCode then
        BuildRecipeCode = {}
    end
    if _G then
        _G.BuildRecipeCode = BuildRecipeCode
    end

    -- ItemBodyLocation Universal Resolver & Bridge
    local function resolveItemBodyLocation(location)
        if not location then return nil end
        if type(location) == "userdata" and location.getClass then
            local ok, sName = pcall(function() return location:getClass():getSimpleName() end)
            if ok and sName == "ItemBodyLocation" then
                return location
            end
        end
        if ItemBodyLocation and ItemBodyLocation.get then
            local ok, locObj = pcall(ItemBodyLocation.get, location)
            if ok and locObj and type(locObj) == "userdata" then
                return locObj
            end
        end
        if BodyLocations and BodyLocations.getGroup then
            local okG, grp = pcall(BodyLocations.getGroup, "Human")
            if okG and grp and grp.getLocation then
                local ok, locObj = pcall(grp.getLocation, grp, tostring(location))
                if ok and locObj and type(locObj) == "userdata" then
                    return locObj
                end
            end
        end
        if ResourceLocation and ResourceLocation.of then
            local okR, rLoc = pcall(ResourceLocation.of, tostring(location))
            if okR and rLoc and ItemBodyLocation and ItemBodyLocation.get then
                local ok, locObj = pcall(ItemBodyLocation.get, rLoc)
                if ok and locObj and type(locObj) == "userdata" then
                    return locObj
                end
            end
        end
        return location
    end

    if ItemBodyLocation and not ItemBodyLocation._pzms_wrapped then
        ItemBodyLocation._pzms_wrapped = true
        local old_get = ItemBodyLocation.get
        if old_get then
            ItemBodyLocation.get = function(val)
                if not val then return nil end
                local status, res = pcall(old_get, val)
                if status and res ~= nil then return res end
                if type(val) == "string" and ResourceLocation and ResourceLocation.of then
                    local ok, rLoc = pcall(ResourceLocation.of, val)
                    if ok and rLoc then
                        local ok2, res2 = pcall(old_get, rLoc)
                        if ok2 and res2 ~= nil then return res2 end
                    end
                end
                return val
            end
        end
    end

    -- SurvivorDesc getWornItem / setWornItem Protection
    if SurvivorDesc and not SurvivorDesc._pzms_wrapped then
        SurvivorDesc._pzms_wrapped = true
        local old_swi = SurvivorDesc.setWornItem
        if old_swi then
            SurvivorDesc.setWornItem = function(self, location, item)
                if not self or not location then return end
                local actualLoc = resolveItemBodyLocation(location)
                local ok, res = pcall(old_swi, self, actualLoc, item)
                if ok then return res end
            end
        end
        local old_gwi = SurvivorDesc.getWornItem
        if old_gwi then
            SurvivorDesc.getWornItem = function(self, location)
                if not self or not location then return nil end
                local actualLoc = resolveItemBodyLocation(location)
                local ok, res = pcall(old_gwi, self, actualLoc)
                if ok and res ~= nil then return res end
                local ok2, res2 = pcall(old_gwi, self, location)
                if ok2 then return res2 end
                return nil
            end
        end
    end

    -- IsoGameCharacter & IsoPlayer getWornItem / setWornItem Protection
    if IsoGameCharacter and not IsoGameCharacter._pzms_worn_wrapped then
        IsoGameCharacter._pzms_worn_wrapped = true
        local old_char_gwi = IsoGameCharacter.getWornItem
        if old_char_gwi then
            IsoGameCharacter.getWornItem = function(self, location)
                if not self or not location then return nil end
                local actualLoc = resolveItemBodyLocation(location)
                local ok, res = pcall(old_char_gwi, self, actualLoc)
                if ok and res ~= nil then return res end
                local ok2, res2 = pcall(old_char_gwi, self, location)
                if ok2 then return res2 end
                return nil
            end
        end
        local old_char_swi = IsoGameCharacter.setWornItem
        if old_char_swi then
            IsoGameCharacter.setWornItem = function(self, location, item)
                if not self or not location then return end
                local actualLoc = resolveItemBodyLocation(location)
                local ok, res = pcall(old_char_swi, self, actualLoc, item)
                if ok then return res end
            end
        end
    end

    -- WornItems getItem / setItem / contains Protection
    if WornItems and not WornItems._pzms_wrapped then
        WornItems._pzms_wrapped = true
        local old_wi_getItem = WornItems.getItem
        if old_wi_getItem then
            WornItems.getItem = function(self, location)
                if not self or not location then return nil end
                local actualLoc = resolveItemBodyLocation(location)
                local ok, res = pcall(old_wi_getItem, self, actualLoc)
                if ok and res ~= nil then return res end
                local ok2, res2 = pcall(old_wi_getItem, self, location)
                if ok2 then return res2 end
                return nil
            end
        end
        local old_wi_setItem = WornItems.setItem
        if old_wi_setItem then
            WornItems.setItem = function(self, location, item)
                if not self or not location then return end
                local actualLoc = resolveItemBodyLocation(location)
                local ok, res = pcall(old_wi_setItem, self, actualLoc, item)
                if ok then return res end
            end
        end
        local old_wi_contains = WornItems.contains
        if old_wi_contains then
            WornItems.contains = function(self, location)
                if not self or not location then return false end
                local actualLoc = resolveItemBodyLocation(location)
                local ok, res = pcall(old_wi_contains, self, actualLoc)
                if ok and res ~= nil then return res end
                return false
            end
        end
        local old_wi_getLocation = WornItems.getLocation
        if old_wi_getLocation then
            WornItems.getLocation = function(self, location)
                if not self or not location then return nil end
                local actualLoc = resolveItemBodyLocation(location)
                local ok, res = pcall(old_wi_getLocation, self, actualLoc)
                if ok and res ~= nil then return res end
                return nil
            end
        end
    end

    -- IsoGameCharacter / IsoPlayer Hand Item Shims (prevents callFrame ReturnValues crash on weapon equip)
    if IsoGameCharacter and not IsoGameCharacter._pzms_hands_wrapped then
        IsoGameCharacter._pzms_hands_wrapped = true
        local old_setPrimary = IsoGameCharacter.setPrimaryHandItem
        if old_setPrimary then
            IsoGameCharacter.setPrimaryHandItem = function(self, item)
                if not self then return end
                if self.getPrimaryHandItem and self:getPrimaryHandItem() == item and item ~= nil then
                    return
                end
                local ok, res = pcall(old_setPrimary, self, item)
                if ok then return res end
            end
        end
        local old_setSecondary = IsoGameCharacter.setSecondaryHandItem
        if old_setSecondary then
            IsoGameCharacter.setSecondaryHandItem = function(self, item)
                if not self then return end
                if self.getSecondaryHandItem and self:getSecondaryHandItem() == item and item ~= nil then
                    return
                end
                local ok, res = pcall(old_setSecondary, self, item)
                if ok then return res end
            end
        end
    end

    if IsoPlayer and not IsoPlayer._pzms_hands_wrapped then
        IsoPlayer._pzms_hands_wrapped = true
        local old_p_setPrimary = IsoPlayer.setPrimaryHandItem
        if old_p_setPrimary then
            IsoPlayer.setPrimaryHandItem = function(self, item)
                if not self then return end
                if self.getPrimaryHandItem and self:getPrimaryHandItem() == item and item ~= nil then
                    return
                end
                local ok, res = pcall(old_p_setPrimary, self, item)
                if ok then return res end
            end
        end
        local old_p_setSecondary = IsoPlayer.setSecondaryHandItem
        if old_p_setSecondary then
            IsoPlayer.setSecondaryHandItem = function(self, item)
                if not self then return end
                if self.getSecondaryHandItem and self:getSecondaryHandItem() == item and item ~= nil then
                    return
                end
                local ok, res = pcall(old_p_setSecondary, self, item)
                if ok then return res end
            end
        end
    end

    -- InventoryItem getDisplayCategory Fallback (Eliminates "Error: No category set")
    if InventoryItem and not InventoryItem._pzms_cat_wrapped then
        InventoryItem._pzms_cat_wrapped = true
        local old_gdc = InventoryItem.getDisplayCategory
        if old_gdc then
            InventoryItem.getDisplayCategory = function(self)
                local ok, res = pcall(old_gdc, self)
                if ok and res ~= nil and res ~= "" and res ~= "Error: No category set" then
                    return res
                end
                if self and self.getCategory then
                    local ok2, cat = pcall(self.getCategory, self)
                    if ok2 and cat then
                        if cat == "Weapon" then return "Weapon" end
                        if cat == "Ammo" then return "Ammo" end
                        if cat == "WeaponPart" then return "WeaponPart" end
                        return cat
                    end
                end
                return "Item"
            end
        end
    end

    -- Global InventoryItemFactory Polyfill (Required for B42 mods instantiating items via InventoryItemFactory.CreateItem)
    if not InventoryItemFactory then
        InventoryItemFactory = {}
    end
    if not InventoryItemFactory._pzms_wrapped then
        InventoryItemFactory._pzms_wrapped = true
        local orig_CreateItem = InventoryItemFactory.CreateItem
        InventoryItemFactory.CreateItem = function(itemType, ...)
            if not itemType then return nil end
            if orig_CreateItem then
                local ok, it = pcall(orig_CreateItem, itemType, ...)
                if ok and it then return it end
                if type(itemType) == "string" and not itemType:find("%.") then
                    local ok2, it2 = pcall(orig_CreateItem, "Base." .. itemType, ...)
                    if ok2 and it2 then return it2 end
                end
            end
            if instanceItem and type(itemType) == "string" then
                local ok, it = pcall(instanceItem, itemType)
                if ok and it then return it end
                if not itemType:find("%.") then
                    local ok2, it2 = pcall(instanceItem, "Base." .. itemType)
                    if ok2 and it2 then return it2 end
                end
            end
            if zombie and zombie.inventory and zombie.inventory.InventoryItemFactory and zombie.inventory.InventoryItemFactory.CreateItem then
                local ok, it = pcall(zombie.inventory.InventoryItemFactory.CreateItem, itemType)
                if ok and it then return it end
            end
            return nil
        end
    end

    -- ItemContainer getFirstEvalRecurse & getAllEvalRecurse Protection (Eliminates Java CallFrame NullPointerException during Context Menu & UI building)
    if ItemContainer and not ItemContainer._pzms_eval_wrapped then
        ItemContainer._pzms_eval_wrapped = true
        local old_getFirstEvalRecurse = ItemContainer.getFirstEvalRecurse
        ItemContainer.getFirstEvalRecurse = function(self, predicate)
            if not self or not predicate then return nil end
            if old_getFirstEvalRecurse then
                local ok, res = pcall(old_getFirstEvalRecurse, self, predicate)
                if ok and res ~= nil then return res end
            end
            local items = self:getItems()
            if items then
                local sz = items:size()
                for i = 0, sz - 1 do
                    local it = items:get(i)
                    if it then
                        local matchOk, matchRes = pcall(predicate, it)
                        if matchOk and matchRes then
                            return it
                        end
                        if it.IsInventoryContainer and it:IsInventoryContainer() then
                            local subInv = it:getInventory()
                            if subInv and subInv.getFirstEvalRecurse then
                                local found = subInv:getFirstEvalRecurse(predicate)
                                if found then return found end
                            end
                        end
                    end
                end
            end
            return nil
        end

        local old_getAllEvalRecurse = ItemContainer.getAllEvalRecurse
        if old_getAllEvalRecurse then
            ItemContainer.getAllEvalRecurse = function(self, predicate, list)
                if not self or not predicate or not list then return end
                local ok, res = pcall(old_getAllEvalRecurse, self, predicate, list)
                if ok then return res end
                local items = self:getItems()
                if items then
                    local sz = items:size()
                    for i = 0, sz - 1 do
                        local it = items:get(i)
                        if it then
                            local matchOk, matchRes = pcall(predicate, it)
                            if matchOk and matchRes then
                                list:add(it)
                            end
                            if it.IsInventoryContainer and it:IsInventoryContainer() then
                                local subInv = it:getInventory()
                                if subInv and subInv.getAllEvalRecurse then
                                    subInv:getAllEvalRecurse(predicate, list)
                                end
                            end
                        end
                    end
                end
            end
        end
    end

    -- Safe GunFighter Attachment & Tag Shims
    if not _G.safeHasTag then
        _G.safeHasTag = function(item, tag)
            if not item or not tag then return false end
            if item.hasTag then
                local ok, res = pcall(item.hasTag, item, tag)
                if ok then return res == true end
            end
            return false
        end
    end

    -- Dynamic Magazine & Caliber Bridge (Resolves 7.62x39, 5.45x39, 5.7x28, .22, etc. Brita/GunFighter custom ammunition)
    if ISInventoryPaneContextMenu and not ISInventoryPaneContextMenu._pzms_mag_wrapped then
        ISInventoryPaneContextMenu._pzms_mag_wrapped = true
        local MAG_CALIBER_MAP = {
            ["AKClip"] = {"Base.762x39Bullets", "762x39Bullets", "Base.Bullets_762x39"},
            ["762x39Belt"] = {"Base.762x39Bullets", "762x39Bullets"},
            ["762Drum"] = {"Base.762x39Bullets", "762x39Bullets"},
            ["SKSClip"] = {"Base.762x39Bullets", "762x39Bullets"},
            ["SVDClip"] = {"Base.762x54rBullets", "762x54rBullets"},
            ["MosinClip"] = {"Base.762x54rBullets", "762x54rBullets"},
            ["762x54rBelt"] = {"Base.762x54rBullets", "762x54rBullets"},
            ["545StdClip"] = {"Base.545x39Bullets", "545x39Bullets"},
            ["545Drum"] = {"Base.545x39Bullets", "545x39Bullets"},
            ["57Clip"] = {"Base.57x28Bullets", "57x28Bullets"},
            ["P90Clip"] = {"Base.57x28Bullets", "57x28Bullets"},
            ["380Clip"] = {"Base.380Bullets", "380Bullets"},
            ["380ExtClip"] = {"Base.380Bullets", "380Bullets"},
            ["M82Clip"] = {"Base.50BMGBullets", "50BMGBullets"},
            ["3006ExtClip"] = {"Base.3006Bullets", "3006Bullets"},
            ["1903Clip"] = {"Base.3006Bullets", "3006Bullets"},
            ["22Clip"] = {"Base.22Bullets", "22Bullets"},
            ["22Drum"] = {"Base.22Bullets", "22Bullets"},
            ["22ExtClip"] = {"Base.22Bullets", "22Bullets"},
            ["CP33Clip"] = {"Base.22Bullets", "22Bullets"},
            ["CP33ExtClip"] = {"Base.22Bullets", "22Bullets"},
            ["556Clip"] = {"Base.556Bullets", "Base.223Bullets", "556Bullets", "223Bullets"},
            ["556Drum"] = {"Base.556Bullets", "Base.223Bullets", "556Bullets", "223Bullets"},
            ["556Belt"] = {"Base.556Bullets", "Base.223Bullets", "556Bullets", "223Bullets"},
            ["223Clip"] = {"Base.223Bullets", "Base.556Bullets", "223Bullets", "556Bullets"},
            ["223StdClip"] = {"Base.223Bullets", "Base.556Bullets", "223Bullets", "556Bullets"},
            ["223ExtClip"] = {"Base.223Bullets", "Base.556Bullets", "223Bullets", "556Bullets"},
        }

        local function resolveRealAmmoKey(playerObj, magazine)
            if not playerObj or not magazine then return nil, 0 end
            local inv = playerObj:getInventory()
            if not inv then return nil, 0 end

            local magType = magazine:getType()
            local candidates = MAG_CALIBER_MAP[magType] or {}

            -- Try mapped candidates first
            for _, cand in ipairs(candidates) do
                local count = inv:getItemCountRecurse(cand)
                if count > 0 then
                    return cand, count
                end
            end

            -- Try native getAmmoType
            local nativeAmmo = magazine:getAmmoType()
            if nativeAmmo then
                local nativeKey = nativeAmmo
                if type(nativeAmmo) == "userdata" and nativeAmmo.getItemKey then
                    nativeKey = nativeAmmo:getItemKey()
                elseif type(nativeAmmo) == "string" then
                    nativeKey = nativeAmmo
                end
                local count = inv:getItemCountRecurse(tostring(nativeKey))
                if count > 0 then
                    return tostring(nativeKey), count
                end
            end

            -- Try matching by DisplayName string heuristic (e.g. "7.62x39")
            local disp = tostring(magazine:getDisplayName() or "")
            if disp:find("7%.62x39") then
                local count = inv:getItemCountRecurse("Base.762x39Bullets")
                if count > 0 then return "Base.762x39Bullets", count end
            elseif disp:find("5%.45x39") then
                local count = inv:getItemCountRecurse("Base.545x39Bullets")
                if count > 0 then return "Base.545x39Bullets", count end
            elseif disp:find("5%.7x28") then
                local count = inv:getItemCountRecurse("Base.57x28Bullets")
                if count > 0 then return "Base.57x28Bullets", count end
            end

            return nil, 0
        end

        local old_doMagazineMenu = ISInventoryPaneContextMenu.doMagazineMenu
        ISInventoryPaneContextMenu.doMagazineMenu = function(playerObj, magazine, context)
            if not playerObj or not magazine or not context then return end
            if magazine:getCurrentAmmoCount() < magazine:getMaxAmmo() then
                local matchedKey, ammoCount = resolveRealAmmoKey(playerObj, magazine)
                if ammoCount > magazine:getMaxAmmo() then
                    ammoCount = magazine:getMaxAmmo()
                end
                if ammoCount > magazine:getMaxAmmo() - magazine:getCurrentAmmoCount() then
                    ammoCount = magazine:getMaxAmmo() - magazine:getCurrentAmmoCount()
                end

                if ammoCount == 0 then
                    local option = context:addOption(getText("ContextMenu_NoBullets", ammoCount))
                    option.notAvailable = true
                else
                    context:addOption(getText("ContextMenu_InsertBulletsInMagazine", ammoCount), playerObj, function(p, m, c)
                        ISInventoryPaneContextMenu.transferIfNeeded(p, m)
                        local items = p:getInventory():getSomeTypeRecurse(matchedKey, c)
                        ISInventoryPaneContextMenu.transferIfNeeded(p, items)
                        if c > 0 then
                            ISTimedActionQueue.add(ISLoadBulletsInMagazine:new(p, m, c))
                        end
                    end, magazine, ammoCount)
                end
            end

            if magazine:getCurrentAmmoCount() > 0 then
                context:addOption(getText("ContextMenu_UnloadMagazine"), playerObj, ISInventoryPaneContextMenu.onUnloadBulletsFromMagazine, magazine)
            end
        end
    end

    -- Safe ISInventoryPane / UI Tables
    if ISInventoryPane and not ISInventoryPane._pzms_safe then
        ISInventoryPane._pzms_safe = true
        local meta = getmetatable(ISInventoryPane) or {}
        local orig_idx = meta.__index
        meta.__index = function(t, k)
            local val = rawget(t, k)
            if val ~= nil then return val end
            if orig_idx then
                if type(orig_idx) == "function" then return orig_idx(t, k) end
                if type(orig_idx) == "table" then return orig_idx[k] end
            end
            return nil
        end
        setmetatable(ISInventoryPane, meta)
    end

    -- Universal ScriptManager Item & Firearm Melee Sanitizer
    if ScriptManager and ScriptManager.instance then
        local sm = ScriptManager.instance
        if sm.getAllItems then
            local ok, items = pcall(sm.getAllItems, sm)
            -- The init runs again every 120 ticks; the items only change when the scripts load
            -- again, so they are walked once per count (thousands of pcalls with a big mod list,
            -- measured at about 8 ms a pass).
            local sz = ok and items and items:size() or 0
            if sz > 0 and sz ~= Poly._itemsSanitized then
                Poly._itemsSanitized = sz
                for i = 0, sz - 1 do
                    local item = items:get(i)
                    if item then
                        -- 1. Fix missing DisplayCategory / "Error: No category set"
                        local okDc, dispCat = pcall(item.getDisplayCategory, item)
                        if not okDc or not dispCat or dispCat == "" or dispCat == "Error: No category set" then
                            local okType, cat = pcall(item.getTypeString, item)
                            if okType and cat == "Weapon" then
                                local isRanged = false
                                if item.isRanged then pcall(function() isRanged = item:isRanged() end) end
                                if isRanged then
                                    pcall(item.setDisplayCategory, item, "RangedWeapon")
                                else
                                    pcall(item.setDisplayCategory, item, "Weapon")
                                end
                            else
                                local okName, name = pcall(item.getName, item)
                                if okName and name then
                                    if name:find("Round") or name:find("Bullet") or name:find("Ammo") or name:find("Shell") then
                                        pcall(item.setDisplayCategory, item, "Ammo")
                                    elseif name:find("Mag") or name:find("Clip") or name:find("Drum") then
                                        pcall(item.setDisplayCategory, item, "GunMag")
                                    end
                                end
                            end
                        end

                        -- 2. Fix Firearm Melee Swing Animation Crash (SwipeState.java NPE)
                        if item.isRanged then
                            local isRanged = false
                            pcall(function() isRanged = item:isRanged() end)
                            if isRanged then
                                local okSw, swAnim = pcall(item.getSwingAnim, item)
                                if not okSw or not swAnim or swAnim == "" or swAnim == "Bat" then
                                    local isTwoHand = false
                                    pcall(function() isTwoHand = item:isTwoHandWeapon() end)
                                    if isTwoHand then
                                        pcall(item.setSwingAnim, item, "Rifle")
                                    else
                                        pcall(item.setSwingAnim, item, "Handgun")
                                    end
                                end
                                local okHf, hfType = pcall(item.getHitFloorType, item)
                                if not okHf or not hfType or hfType == "" then
                                    pcall(item.setHitFloorType, item, "Stomp")
                                end
                            end
                        end
                    end
                end
            end
        end
    end

    -- IsoDoor / IsoWindow / IsoThumpable B42 JVM Null Pointer Interop Safety
    if IsoDoor and not IsoDoor._pzms_wrapped then
        IsoDoor._pzms_wrapped = true
        local old_toggleDoor = IsoDoor.ToggleDoor
        if old_toggleDoor then
            IsoDoor.ToggleDoor = function(self, chr)
                if not self then return end
                local safeChr = chr or (getPlayer and getPlayer()) or (getSpecificPlayer and getSpecificPlayer(0))
                if safeChr ~= nil then
                    local ok, res = pcall(old_toggleDoor, self, safeChr)
                    if ok then return res end
                end
                pcall(function()
                    if self.setOpen and self.IsOpen then
                        self:setOpen(not self:IsOpen())
                    end
                end)
            end
        end
        local old_toggleActual = IsoDoor.ToggleDoorActual
        if old_toggleActual then
            IsoDoor.ToggleDoorActual = function(self, chr, ...)
                if not self then return end
                local safeChr = chr or (getPlayer and getPlayer()) or (getSpecificPlayer and getSpecificPlayer(0))
                if safeChr ~= nil then
                    local ok, res = pcall(old_toggleActual, self, safeChr, ...)
                    if ok then return res end
                end
                pcall(function()
                    if self.setOpen and self.IsOpen then
                        self:setOpen(not self:IsOpen())
                    end
                end)
            end
        end
    end

    if IsoWindow and not IsoWindow._pzms_wrapped then
        IsoWindow._pzms_wrapped = true
        local old_toggleWindow = IsoWindow.ToggleWindow
        if old_toggleWindow then
            IsoWindow.ToggleWindow = function(self, chr)
                if not self then return end
                local safeChr = chr or (getPlayer and getPlayer()) or (getSpecificPlayer and getSpecificPlayer(0))
                if safeChr ~= nil then
                    local ok, res = pcall(old_toggleWindow, self, safeChr)
                    if ok then return res end
                end
                pcall(function()
                    if self.setOpen and self.IsOpen then
                        self:setOpen(not self:IsOpen())
                    end
                end)
            end
        end
    end

    if IsoThumpable and not IsoThumpable._pzms_wrapped then
        IsoThumpable._pzms_wrapped = true
        local old_thumpToggle = IsoThumpable.ToggleDoor
        if old_thumpToggle then
            IsoThumpable.ToggleDoor = function(self, chr)
                if not self then return end
                local safeChr = chr or (getPlayer and getPlayer()) or (getSpecificPlayer and getSpecificPlayer(0))
                if safeChr ~= nil then
                    local ok, res = pcall(old_thumpToggle, self, safeChr)
                    if ok then return res end
                end
                pcall(function()
                    if self.setOpen and self.IsOpen then
                        self:setOpen(not self:IsOpen())
                    end
                end)
            end
        end
    end

    -- ----------------------------------------------------------------------------
    -- Safe Java ArrayList & Collection Interop Proxy Helper
    -- ----------------------------------------------------------------------------

    -- ISPathFindAction.addAfter (Build 42 has it on ISBaseTimedAction only)
    if ISPathFindAction and ISBaseTimedAction then
        if not ISPathFindAction.addAfter then
            ISPathFindAction.addAfter = ISBaseTimedAction.addAfter
        end
    end
end

-- ----------------------------------------------------------------------------
-- 6. Vanilla and multiplayer safety: guards on the game's own code
-- ----------------------------------------------------------------------------
local function initVanillaGuards()
    -- ----------------------------------------------------------------------------
    -- ISHandcraftAction MP Null-Safety & Containers Guard
    -- ----------------------------------------------------------------------------
    if ISHandcraftAction and not ISHandcraftAction._pzms_wrapped then
        ISHandcraftAction._pzms_wrapped = true

        local function ensureJavaArrayList(contList, character)
            if contList and type(contList) == "userdata" and contList.size and contList.addAll then
                return contList
            end
            local list = ArrayList and ArrayList.new and ArrayList.new() or nil
            if list then
                if character and character.getInventory then
                    list:add(character:getInventory())
                end
                return list
            end
            return contList
        end

        local orig_handcraft_start = ISHandcraftAction.start
        if orig_handcraft_start then
            ISHandcraftAction.start = function(self)
                self.containers = ensureJavaArrayList(self.containers, self.character)
                return orig_handcraft_start(self)
            end
        end

        local orig_handcraft_serverStart = ISHandcraftAction.serverStart
        if orig_handcraft_serverStart then
            ISHandcraftAction.serverStart = function(self)
                self.containers = ensureJavaArrayList(self.containers, self.character)
                return orig_handcraft_serverStart(self)
            end
        end

        local orig_handcraft_update = ISHandcraftAction.update
        if orig_handcraft_update then
            ISHandcraftAction.update = function(self)
                local ok, err = pcall(orig_handcraft_update, self)
                if not ok and err then
                    print("[PZModStudio_Polyfills] Handcraft update error caught: " .. tostring(err))
                end
            end
        end
    end

    if BaseCraftingLogic and BaseCraftingLogic.setContainers and not BaseCraftingLogic._pzms_setcontainers then
        BaseCraftingLogic._pzms_setcontainers = true
        local orig_bcl_setContainers = BaseCraftingLogic.setContainers
        BaseCraftingLogic.setContainers = function(self, list)
            local safeList = list
            if not safeList or type(safeList) ~= "userdata" or not safeList.size then
                local al = ArrayList and ArrayList.new and ArrayList.new() or nil
                if al then
                    if self.player and self.player.getInventory then
                        al:add(self.player:getInventory())
                    elseif self.character and self.character.getInventory then
                        al:add(self.character:getInventory())
                    end
                    safeList = al
                end
            end
            local ok, res = pcall(orig_bcl_setContainers, self, safeList)
            if ok then return res end
            return false
        end
    end
    -- ----------------------------------------------------------------------------
    -- ISBaseTimedAction MP Network Shield (multiplayer clients only)
    -- Intercepts timed actions right before serialization in LuaTimedActionNew
    -- Prevents ContainerID.findObject crashes caused by bags, floor containers, or onCompleteArgs
    -- Single player serializes nothing, so vanilla stays as it is there. The action itself is
    -- left in the arguments: vanilla passes it (onRestPathFound) and needs it back.
    -- ----------------------------------------------------------------------------
    if isClient and isClient() and ISBaseTimedAction and not ISBaseTimedAction._pzms_shielded then
        ISBaseTimedAction._pzms_shielded = true

        local orig_bta_setOnComplete = ISBaseTimedAction.setOnComplete
        if orig_bta_setOnComplete then
            ISBaseTimedAction.setOnComplete = function(self, func, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8)
                self.onCompleteFunc = func
                local function sanitizeArg(v)
                    if v and type(v) == "userdata" then
                        if v.size and v.addAll then return nil end
                        if v.getType and v.AddItem then return nil end
                    end
                    return v
                end
                self.onCompleteArgs = {
                    sanitizeArg(arg1),
                    sanitizeArg(arg2),
                    sanitizeArg(arg3),
                    sanitizeArg(arg4),
                    sanitizeArg(arg5),
                    sanitizeArg(arg6),
                    sanitizeArg(arg7),
                    sanitizeArg(arg8)
                }
            end
        end

        local orig_bta_begin = ISBaseTimedAction.begin
        if orig_bta_begin then
            ISBaseTimedAction.begin = function(self)
                if self.containersIn and type(self.containersIn) == "userdata" and self.containersIn.size then
                    self.containersIn = nil
                end
                if self.containers and type(self.containers) == "userdata" and self.containers.size then
                    if self.recipe then
                        self.containers = nil
                    end
                end
                if self.container and type(self.container) == "userdata" and self.container.getContainingItem and self.container:getContainingItem() ~= nil then
                    if self.character and self.character.getInventory then
                        self.container = self.character:getInventory()
                    else
                        self.container = nil
                    end
                end
                if self.onCompleteArgs and type(self.onCompleteArgs) == "table" then
                    for k, v in pairs(self.onCompleteArgs) do
                        if v and type(v) == "userdata" then
                            if v.size and v.addAll then
                                self.onCompleteArgs[k] = nil
                            elseif v.getType and v.AddItem then
                                self.onCompleteArgs[k] = nil
                            end
                        end
                    end
                end
                return orig_bta_begin(self)
            end
        end
    end

    -- ----------------------------------------------------------------------------
    -- ISCraftAction MP Network Crash Prevention & Execution Guard
    -- Fixes: "Make sheet", "Rip clothing", "Open can/box/packet", etc. in Multiplayer
    -- Prevents: NullPointerException at ContainerID.findObject when serializing floor containers
    -- ----------------------------------------------------------------------------
    if ISCraftAction then
        ISCraftAction._pzms_wrapped = true

        function ISCraftAction:new(character, item, recipe, container, containersIn)
            local o = ISBaseTimedAction.new(self, character)
            o.character = character
            o.item = item
            o.recipe = recipe
            -- CRITICAL: Always use character's main inventory to prevent ContainerID.java:435 crash on server
            o.container = character and character.getInventory and character:getInventory() or nil
            o.containersIn = nil
            o.containers = nil

            o.stopOnWalk = recipe and recipe.isStopOnWalk and recipe:isStopOnWalk() or false
            o.stopOnRun = recipe and recipe.isStopOnRun and recipe:isStopOnRun() or false
            o.stopOnAim = true
            o.maxTime = o:getDuration()
            o.jobType = recipe and recipe.getName and recipe:getName() or "Craft"
            o.forceProgressBar = true
            return o
        end

        function ISCraftAction:setOnComplete(func, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8)
            self.onCompleteFunc = func
            -- Avoid storing action self-references or container ArrayLists in onCompleteArgs
            self.onCompleteArgs = { true, arg2, arg3, nil, nil, arg6, nil }
        end

        function ISCraftAction:isValid()
            if self.character and self.character.getInventory and self.item and self.item.getID then
                local inv = self.character:getInventory()
                if inv then
                    local fresh = inv:getItemById(self.item:getID())
                    if fresh then
                        self.item = fresh
                    end
                end
            end
            if self.item and self.item.getContainer and self.item:getContainer() == nil then
                self.item = nil
            end
            if not self.recipe or not self.character then
                return false
            end
            local ok, res = pcall(function()
                return RecipeManager.IsRecipeValid(self.recipe, self.character, self.item, self.containers) and not (self.character.isDriving and self.character:isDriving())
            end)
            if ok then return res end
            return not (self.character.isDriving and self.character:isDriving())
        end

        function ISCraftAction:start()
            if self.character and self.character.getInventory and self.item and self.item.getID then
                local inv = self.character:getInventory()
                if inv then
                    local fresh = inv:getItemById(self.item:getID())
                    if fresh then
                        self.item = fresh
                    end
                end
            end
            if showDebugInfoInChat and self.recipe and self.recipe.getOriginalname then
                showDebugInfoInChat("CRAFT \'" .. tostring(self.recipe:getOriginalname()) .. "\'")
            end
            if self.recipe and self.recipe.getSound and self.recipe:getSound() and self.character and self.character.playSound then
                self.craftSound = self.character:playSound(self.recipe:getSound())
            end
            if self.item then
                if self.recipe and self.recipe.getName then
                    self.item:setJobType(self.recipe:getName())
                end
                self.item:setJobDelta(0.0)
            end
            if self.recipe and (self.recipe.getProp1 or self.recipe.getProp2) then
                local prop1 = self.recipe.getProp1 and self.recipe:getProp1()
                local prop2 = self.recipe.getProp2 and self.recipe:getProp2()
                if prop1 or prop2 then
                    self:setOverrideHandModels(self:getPropItemOrModel(prop1), self:getPropItemOrModel(prop2))
                end
            end
            if self.recipe and self.recipe.getAnimNode and self.recipe:getAnimNode() then
                self:setActionAnim(self.recipe:getAnimNode())
            else
                self:setActionAnim(CharacterActionAnims.Craft)
            end
            if self.item and self.item.getContainer and self.item:getContainer() == nil then
                self.item = nil
            end
        end

        function ISCraftAction:stop()
            if self.craftSound and self.character and self.character.getEmitter and self.character:getEmitter():isPlaying(self.craftSound) then
                self.character:stopOrTriggerSound(self.craftSound)
            end
            if self.item ~= nil then
                self.item:setJobDelta(0.0)
            end
            ISBaseTimedAction.stop(self)
        end

        function ISCraftAction:perform()
            if self.craftSound and self.character and self.character.getEmitter and self.character:getEmitter():isPlaying(self.craftSound) then
                self.character:stopOrTriggerSound(self.craftSound)
            end
            if self.item then
                self.item:setJobDelta(0.0)
            end

            if ISInventoryPage and ISInventoryPage.dirtyUI then
                ISInventoryPage.dirtyUI()
            end

            if self.onCompleteFunc then
                local playerObj = self.character
                local recipe = self.recipe
                local selectedItemType = self.item and self.item.getFullType and self.item:getFullType() or nil
                pcall(function()
                    self.onCompleteFunc(self, recipe, playerObj, playerObj and playerObj:getInventory(), nil, selectedItemType, nil)
                end)
            end

            ISBaseTimedAction.perform(self)
        end

        function ISCraftAction:complete()
            if not self.character or not self.recipe then
                return false
            end

            -- Re-resolve character inventory & item
            local inv = self.character.getInventory and self.character:getInventory()
            if inv and self.item and self.item.getID then
                local fresh = inv:getItemById(self.item:getID())
                if fresh then
                    self.item = fresh
                end
            end

            self.container = inv

            -- Execute PerformMakeItem directly on character inventory
            local ok, list = pcall(function()
                return RecipeManager.PerformMakeItem(self.recipe, self.item, self.character, nil)
            end)

            if not ok or not list then
                print("[PZModStudio_Polyfills] RecipeManager.PerformMakeItem error: " .. tostring(list))
                return false
            end

            if list and list.size then
                local sz = list:size()
                for i = 0, sz - 1 do
                    local outItem = list:get(i)
                    if outItem then
                        if Actions and Actions.addOrDropItem then
                            Actions.addOrDropItem(self.character, outItem)
                        elseif inv then
                            inv:AddItem(outItem)
                            if sendAddItemToContainer then
                                sendAddItemToContainer(inv, outItem)
                            end
                        end
                    end
                end
            end

            return true
        end
    end

    -- ----------------------------------------------------------------------------
    -- ISInventoryPaneContextMenu.OnCraft Safe Item Transfer Pre-check
    -- ----------------------------------------------------------------------------
    if ISInventoryPaneContextMenu and not ISInventoryPaneContextMenu._pzms_craft_wrapped then
        ISInventoryPaneContextMenu._pzms_craft_wrapped = true
        local orig_ipc_onCraft = ISInventoryPaneContextMenu.OnCraft
        if orig_ipc_onCraft then
            ISInventoryPaneContextMenu.OnCraft = function(selectedItem, recipe, player, all)
                if selectedItem and recipe and recipe.isCanBeDoneFromFloor and not recipe:isCanBeDoneFromFloor() then
                    local playerObj = type(player) == "number" and getSpecificPlayer(player) or player
                    if playerObj and selectedItem.getContainer and selectedItem:getContainer() ~= playerObj:getInventory() then
                        if ISInventoryPaneContextMenu.transferIfNeeded then
                            ISInventoryPaneContextMenu.transferIfNeeded(playerObj, selectedItem)
                        end
                    end
                end
                return orig_ipc_onCraft(selectedItem, recipe, player, all)
            end
        end
    end

    -- ----------------------------------------------------------------------------
    -- ISVehicleDashboard Null-Safety Guard (Prevents null pointer when entering/switching seat)
    -- ----------------------------------------------------------------------------
    if ISVehicleDashboard and not ISVehicleDashboard._pzms_wrapped then
        ISVehicleDashboard._pzms_wrapped = true
        local orig_onSwitchSeat = ISVehicleDashboard.onSwitchVehicleSeat
        if orig_onSwitchSeat then
            ISVehicleDashboard.onSwitchVehicleSeat = function(character)
                if character and instanceof(character, 'IsoPlayer') and character:isLocalPlayer() then
                    local vehicle = character:getVehicle()
                    local db = getPlayerVehicleDashboard and getPlayerVehicleDashboard(character:getPlayerNum())
                    if db and db.setVehicle then
                        if vehicle and vehicle:isDriver(character) then
                            db:setVehicle(vehicle)
                        else
                            db:setVehicle(nil)
                        end
                    end
                end
            end
        end
        local orig_onEnterVehicle = ISVehicleDashboard.onEnterVehicle
        if orig_onEnterVehicle then
            ISVehicleDashboard.onEnterVehicle = function(character)
                if character and instanceof(character, 'IsoPlayer') and character:isLocalPlayer() then
                    local vehicle = character:getVehicle()
                    if vehicle and vehicle:isDriver(character) then
                        local db = getPlayerVehicleDashboard and getPlayerVehicleDashboard(character:getPlayerNum())
                        if db and db.setVehicle then
                            db:setVehicle(vehicle)
                        end
                    end
                end
            end
        end
        local orig_onExitVehicle = ISVehicleDashboard.onExitVehicle
        if orig_onExitVehicle then
            ISVehicleDashboard.onExitVehicle = function(character)
                if character and instanceof(character, 'IsoPlayer') and character:isLocalPlayer() then
                    local db = getPlayerVehicleDashboard and getPlayerVehicleDashboard(character:getPlayerNum())
                    if db and db.setVehicle then
                        db:setVehicle(nil)
                    end
                end
            end
        end
    end

    -- IsoPlayer / IsoGameCharacter Attack & Melee Swing Safety (prevents SwipeState / Melee gun swing crash)
    if IsoPlayer and not IsoPlayer._pzms_attack_wrapped then
        IsoPlayer._pzms_attack_wrapped = true
        local old_attack = IsoPlayer.Attack
        if old_attack then
            IsoPlayer.Attack = function(self, ...)
                local ok, res = pcall(old_attack, self, ...)
                if ok then return res end
                print("[PZ Mod Studio Polyfill] Handled safe IsoPlayer:Attack exception: " .. tostring(res))
            end
        end
    end

    -- VehiclePart getDeviceData Safety Shim
    if VehiclePart and VehiclePart.getDeviceData and not VehiclePart._pzms_devicedata then
        VehiclePart._pzms_devicedata = true
        local old_gdd = VehiclePart.getDeviceData
        VehiclePart.getDeviceData = function(self)
            local status, res = pcall(old_gdd, self)
            if status and res ~= nil then return res end
            return nil
        end
    end

    -- ----------------------------------------------------------------------------
    -- Dedicated Server & Multiplayer Null-Pointer Shield for getOnlinePlayers
    -- ----------------------------------------------------------------------------
    if _G and _G.getOnlinePlayers and not _G.__pzms_getOnlinePlayers_wrapped then
        _G.__pzms_getOnlinePlayers_wrapped = true
        local orig_getOnlinePlayers = _G.getOnlinePlayers
        _G.getOnlinePlayers = function(...)
            if isServer and isServer() then
                if not (zombie and zombie.network and zombie.network.GameServer and zombie.network.GameServer.udpEngine) then
                    return createSafeArrayListProxy(nil)
                end
            end
            local ok, res = pcall(orig_getOnlinePlayers, ...)
            if ok and res ~= nil then
                return createSafeArrayListProxy(res)
            end
            return createSafeArrayListProxy(nil)
        end
    end

    -- ----------------------------------------------------------------------------
    -- Multiplayer & Bounds Safety Shield for IsoPlayer.getPlayers
    -- ----------------------------------------------------------------------------
    if IsoPlayer and IsoPlayer.getPlayers and not IsoPlayer.__pzms_getPlayers_wrapped then
        IsoPlayer.__pzms_getPlayers_wrapped = true
        local orig_getPlayers = IsoPlayer.getPlayers
        IsoPlayer.getPlayers = function(...)
            local ok, res = pcall(orig_getPlayers, ...)
            if ok and res ~= nil then
                return createSafeArrayListProxy(res)
            end
            return createSafeArrayListProxy(nil)
        end
    end

    -- ----------------------------------------------------------------------------
    -- Global Vehicle Spawn Guard for MP Clients (Suppresses Unauthorized AddVehicle Commands)
    -- ----------------------------------------------------------------------------
    if isClient and isClient() then
        if _G.addVehicle and not _G.__pzms_addveh_wrapped then
            _G.__pzms_addveh_wrapped = true
            local orig_addVehicle = _G.addVehicle
            _G.addVehicle = function(...)
                local p = getSpecificPlayer and getSpecificPlayer(0)
                local acc = (p and p.getAccessLevel and tostring(p:getAccessLevel()):lower()) or "none"
                if acc == "none" or acc == "" or acc == "nil" then
                    return nil
                end
                return orig_addVehicle(...)
            end
        end

        if _G.addVehicleDebug and not _G.__pzms_addvehdeb_wrapped then
            _G.__pzms_addvehdeb_wrapped = true
            local orig_addVehicleDebug = _G.addVehicleDebug
            _G.addVehicleDebug = function(...)
                local p = getSpecificPlayer and getSpecificPlayer(0)
                local acc = (p and p.getAccessLevel and tostring(p:getAccessLevel()):lower()) or "none"
                if acc == "none" or acc == "" or acc == "nil" then
                    return nil
                end
                return orig_addVehicleDebug(...)
            end
        end
    end

    -- ----------------------------------------------------------------------------
    -- Core:getKey(nil): a key binding asked for with no name (42.21)
    -- ----------------------------------------------------------------------------
    -- Java throws a NullPointerException ("keybindID" is null) and logs it, then hands Lua nil;
    -- pcall does not keep it out of the log. A mod that walks its bindings on every key press
    -- with a header among them (Fancy Handwork's "[FancyHandwork]") wrote hundreds of them a
    -- session. Same answer, nil, without asking Java. An unknown name already answers 0.
    local okCore, coreClass = pcall(function() return Core.class end)
    local coreIndex = okCore and coreClass and __classmetatables and __classmetatables[coreClass]
    coreIndex = coreIndex and coreIndex.__index
    if type(coreIndex) == "table" and coreIndex.getKey and not coreIndex.__pzms_getKey then
        local orig_getKey = coreIndex.getKey
        coreIndex.getKey = function(self, name, ...)
            if name == nil then return nil end
            return orig_getKey(self, name, ...)
        end
        coreIndex.__pzms_getKey = true
    end

end

-- ----------------------------------------------------------------------------
-- 7. Per-mod patches: each runs only while its mod is active, and each can be switched off
-- ----------------------------------------------------------------------------
local function initModPatches()
    if Poly.patchOn("daily_report", { "2858201402/dailystatisticsandgains" }) then
    -- ReportWindow:new Protection for Daily Report Journal (prevents nil operand division)
    if ReportWindow and ReportWindow.new and not ReportWindow._pzms_wrapped then
        ReportWindow._pzms_wrapped = true
        local old_rep_new = ReportWindow.new
        ReportWindow.new = function(self, x, y, width, height, player)
            if type(x) ~= "number" and x ~= nil then
                player = x
                x = nil
                y = nil
                width = nil
                height = nil
            end
            width = tonumber(width) or 500
            height = tonumber(height) or 500
            local sw = (getCore() and getCore():getScreenWidth()) or 1920
            local sh = (getCore() and getCore():getScreenHeight()) or 1080
            x = tonumber(x) or (sw / 2 - (width / 2))
            y = tonumber(y) or (sh / 2 - (height / 2))
            local status, res = pcall(old_rep_new, self, x, y, width, height, player)
            if status and res then return res end
            local o = ISCollapsableWindow:new(x, y, width, height)
            setmetatable(o, self)
            self.__index = self
            o.borderColor = {r=0.4, g=0.4, b=0.4, a=1}
            o.backgroundColor = {r=0, g=0, b=0, a=0.8}
            o.width = width
            o.height = height
            o.player = player or (getSpecificPlayer and getSpecificPlayer(0))
            o:setResizable(false)
            o.selectedFaction = nil
            o.moveWithMouse = true
            ReportWindow.instance = o
            o:setTitle("Daily Report Journal")
            return o
        end
    end

    end

    if Poly.patchOn("gunfighter", nil) then
    -- ISEquipWeaponAction / GunFighter perform wrapper (prevents JVM null callframe crash on equip)
    if ISEquipWeaponAction and not ISEquipWeaponAction._pzms_perform_wrapped then
        ISEquipWeaponAction._pzms_perform_wrapped = true
        local old_perform = ISEquipWeaponAction.perform
        if old_perform then
            ISEquipWeaponAction.perform = function(self)
                local ok, err = pcall(old_perform, self)
                if not ok and err then
                    print("[PZ Mod Studio Polyfill] Handled ISEquipWeaponAction:perform error safely: " .. tostring(err))
                    if self and self.character and self.item then
                        if self.primary then
                            pcall(self.character.setPrimaryHandItem, self.character, self.item)
                        else
                            pcall(self.character.setSecondaryHandItem, self.character, self.item)
                        end
                    end
                end
            end
        end
    end

    end

    if Poly.patchOn("ocd_inventory", { "OCDInventory" }) then
    -- OCDInventory Async Refresh Wrapper (prevents mid-callframe UI re-layout during OnEquipPrimary)
    if OCDInventory and OCDInventory.InventoryObserver and not OCDInventory.InventoryObserver._pzms_wrapped then
        OCDInventory.InventoryObserver._pzms_wrapped = true
        local old_requestRefresh = OCDInventory.InventoryObserver.requestRefresh
        if old_requestRefresh then
            OCDInventory.InventoryObserver.requestRefresh = function(playerNum, reason)
                if reason == "equipPrimary" or reason == "equipSecondary" or reason == "clothingUpdated" then
                    pcall(function()
                        if OCDInventory.RefreshCoordinator and OCDInventory.RefreshCoordinator.schedule then
                            OCDInventory.RefreshCoordinator.schedule(playerNum, reason)
                        end
                    end)
                else
                    pcall(old_requestRefresh, playerNum, reason)
                end
            end
        end
    end

    end

    if Poly.patchOn("fancy_handwork_ocd", { "FancyHandworkB42_19", "OCDInventory" }) then
    -- ISEquipWeaponAction Safety Guard (prevents hand-swap action crashes between Fancy Handwork & OCDInventory)
    if ISEquipWeaponAction and not ISEquipWeaponAction._pzms_complete_wrapped then
        ISEquipWeaponAction._pzms_complete_wrapped = true
        local orig_complete = ISEquipWeaponAction.complete
        if orig_complete then
            ISEquipWeaponAction.complete = function(self)
                local ok, res = pcall(orig_complete, self)
                if not ok then
                    pcall(function()
                        if self.item and self.character then
                            if self.twoHands or (self.item.isRequiresEquippedBothHands and self.item:isRequiresEquippedBothHands()) then
                                self.character:setPrimaryHandItem(self.item)
                                self.character:setSecondaryHandItem(self.item)
                            elseif self.primary then
                                self.character:setPrimaryHandItem(self.item)
                            else
                                self.character:setSecondaryHandItem(self.item)
                            end
                        end
                    end)
                    return true
                end
                return res
            end
        end
    end

    end

    if Poly.patchOn("bandits", nil) then
    -- Bandits Week One Room Programs & Nil Room Safety Guard (BWORoomPrograms.lua:1005)
    if BWORoomPrograms and not BWORoomPrograms._pzms_wrapped then
        BWORoomPrograms._pzms_wrapped = true
        local old_restaurant = BWORoomPrograms.restaurant
        if old_restaurant then
            BWORoomPrograms.restaurant = function(bandit, def)
                local status, res = pcall(old_restaurant, bandit, def)
                if status and res ~= nil then return res end
                return {}
            end
        end
        for progName, progFunc in pairs(BWORoomPrograms) do
            if type(progFunc) == "function" and progName ~= "restaurant" then
                local rawProg = progFunc
                BWORoomPrograms[progName] = function(bandit, def)
                    local ok, res = pcall(rawProg, bandit, def)
                    if ok and res ~= nil then return res end
                    return {}
                end
            end
        end
    end

    end

    if Poly.patchOn("bandits", nil) then
    -- Bandits & BWO Program Task Return Safety Guard (Prevents __len on nil operand)
    if BanditPrograms then
        local bwoFns = {"Events", "Symptoms", "Talk", "FollowRoad"}
        for _, fn in ipairs(bwoFns) do
            if type(BanditPrograms[fn]) == "function" and not BanditPrograms["__pzms_guarded_" .. fn] then
                BanditPrograms["__pzms_guarded_" .. fn] = true
                local orig = BanditPrograms[fn]
                BanditPrograms[fn] = function(...)
                    local ok, res = pcall(orig, ...)
                    if ok and type(res) == "table" then
                        return res
                    end
                    return {}
                end
            end
        end
    end

    end

    if Poly.patchOn("bandits", nil) then
    -- Bandits UpdateItemsToSpawnAtDeath Safety Guard (Bandit.lua:714 / BWOSquareLoader.lua:763)
    if Bandit and Bandit.UpdateItemsToSpawnAtDeath and not Bandit.__pzms_death_guarded then
        Bandit.__pzms_death_guarded = true
        local orig_update_items = Bandit.UpdateItemsToSpawnAtDeath
        Bandit.UpdateItemsToSpawnAtDeath = function(zombie, brain)
            if not zombie then return end
            if not brain and BanditBrain and BanditBrain.Get then
                brain = BanditBrain.Get(zombie)
            end
            if not brain or type(brain) ~= "table" or not brain.weapons then
                return
            end
            return orig_update_items(zombie, brain)
        end
    end

    end

    if Poly.patchOn("ai_week_one", nil) then
    -- AIWeekOne AINavigation Window / Navigation Safety Protection
    if AINavigation and not AINavigation._pzms_wrapped then
        AINavigation._pzms_wrapped = true
        local old_recover = AINavigation.recover
        if old_recover then
            AINavigation.recover = function(zombie, brain, ...)
                local ok, res = pcall(old_recover, zombie, brain, ...)
                if ok then return res end
                return false
            end
        end
    end

    end

    if Poly.patchOn("daily_report", { "2858201402/dailystatisticsandgains" }) then
    -- ----------------------------------------------------------------------------
    -- Daily Report Journal (dSAG / dailyStatisticsAndGains) Safety Guard
    -- ----------------------------------------------------------------------------
    local function guardDSAGTable(tbl)
        if not tbl or tbl._pzms_wrapped then return end
        tbl._pzms_wrapped = true
        local methods = {
            "onNewDay", "OnNewDay", "onPlayerDeath", "OnPlayerDeath",
            "onDataReloaded", "OnDataReloaded", "analyzeStats", "switchTab",
            "calculateStats", "updateStats", "render", "prerender"
        }
        for _, mName in ipairs(methods) do
            if type(tbl[mName]) == "function" then
                local rawFn = tbl[mName]
                tbl[mName] = function(...)
                    local ok, res = pcall(rawFn, ...)
                    if ok then return res end
                    return nil
                end
            end
        end
    end

    if dSAG then guardDSAGTable(dSAG) end
    if dSAG_Window then guardDSAGTable(dSAG_Window) end
    if dailyStatisticsAndGains then guardDSAGTable(dailyStatisticsAndGains) end
    if DailyReportJournal then guardDSAGTable(DailyReportJournal) end

    end

    if Poly.patchOn("fancy_handwork", { "FancyHandworkB42_19" }) then
    -- ----------------------------------------------------------------------------
    -- Fancy Handwork B42 Multiplayer & Java Bounds Safety Guard
    -- ----------------------------------------------------------------------------
    local function guardFancyTable(tbl)
        if not tbl or tbl.__pzms_fh_guarded then return end
        tbl.__pzms_fh_guarded = true
        for k, v in pairs(tbl) do
            if type(v) == "function" then
                local origFn = v
                tbl[k] = function(...)
                    local ok, res = pcall(origFn, ...)
                    if ok then return res end
                    return nil
                end
            end
        end
    end

    if FancyHandwork then guardFancyTable(FancyHandwork) end
    if aFancyHandwork then guardFancyTable(aFancyHandwork) end
    if FHandwork then guardFancyTable(FHandwork) end
    if _G.fancyMP and not _G.__pzms_fancyMP_guarded then
        _G.__pzms_fancyMP_guarded = true
        local orig_fancyMP = _G.fancyMP
        _G.fancyMP = function(...)
            local ok, res = pcall(orig_fancyMP, ...)
            if ok then return res end
            return nil
        end
    end

    end

    if Poly.patchOn("sauced_carts", { "SaucedCarts" }) then
    -- ----------------------------------------------------------------------------
    -- SaucedCarts / Pushable Carts Multiplayer & Attach Interop Guard
    -- ----------------------------------------------------------------------------
    local function guardSaucedCartsTable(tbl)
        if not tbl or tbl._pzms_wrapped then return end
        tbl._pzms_wrapped = true
        for k, v in pairs(tbl) do
            if type(v) == "function" then
                local rawFn = v
                tbl[k] = function(...)
                    local ok, res = pcall(rawFn, ...)
                    if ok then return res end
                    return nil
                end
            end
        end
    end

    if SaucedCarts then guardSaucedCartsTable(SaucedCarts) end
    if PushableCarts then guardSaucedCartsTable(PushableCarts) end

    end

    if Poly.patchOn("exhaustive_power_rework", { "ExhaustivePowerRework", "ExhaustivePowerReworkZB" }) then
    -- ----------------------------------------------------------------------------
    -- Exhaustive Power Rework (EPR) Dedicated Server Startup Guard
    -- ----------------------------------------------------------------------------
    if Grid and type(Grid.Reconcile) == "function" and not Grid.__pzms_patched then
        Grid.__pzms_patched = true
        local old_grid_reconcile = Grid.Reconcile
        Grid.Reconcile = function(...)
            if isServer and isServer() and not (zombie and zombie.network and zombie.network.GameServer and zombie.network.GameServer.udpEngine) then
                return
            end
            return old_grid_reconcile(...)
        end
    end

    end

    if Poly.patchOn("bandits", nil) then
    -- ----------------------------------------------------------------------------
    -- Bandits Week One (BWO) Comprehensive Multiplayer & Audio Safety Guards
    -- ----------------------------------------------------------------------------
    if BWOVariants and not BWOVariants.__pzms_patched then
        BWOVariants.__pzms_patched = true
        local fallback_variant = BWOVariants["original"] or {
            playerIsHostile = false,
            schedule = {},
            inhabitants = {},
            streets = {},
        }
        setmetatable(BWOVariants, {
            __index = function(t, k)
                if k == nil or rawget(t, k) == nil then
                    return fallback_variant
                end
                return rawget(t, k)
            end
        })
    end

    if _G.GetBWOModData and not _G.__pzms_gmd_patched then
        _G.__pzms_gmd_patched = true
        local orig_GetBWOModData = _G.GetBWOModData
        _G.GetBWOModData = function()
            local data = orig_GetBWOModData()
            if type(data) ~= "table" then
                data = {}
                if _G.BWOGlobalData then _G.BWOGlobalData = data end
            end
            if not data.Variant then
                if SandboxVars and SandboxVars.BanditsWeekOne and SandboxVars.BanditsWeekOne.Variant then
                    data.Variant = SandboxVars.BanditsWeekOne.Variant
                else
                    data.Variant = "original"
                end
            end
            return data
        end
    end

    if BWOScheduler and BWOScheduler.generateSchedule and not BWOScheduler.__pzms_patched then
        BWOScheduler.__pzms_patched = true
        local orig_generateSchedule = BWOScheduler.generateSchedule
        BWOScheduler.generateSchedule = function(...)
            local gmd = GetBWOModData and GetBWOModData()
            if gmd and not gmd.Variant then
                if SandboxVars and SandboxVars.BanditsWeekOne and SandboxVars.BanditsWeekOne.Variant then
                    gmd.Variant = SandboxVars.BanditsWeekOne.Variant
                else
                    gmd.Variant = "original"
                end
            end
            local ok, res = pcall(orig_generateSchedule, ...)
            if ok and res ~= nil then return res end
            if BWOVariants and BWOVariants["original"] and BWOVariants["original"].schedule then
                return BWOVariants["original"].schedule
            end
            return {}
        end
    end

    if BWOMusic and not BWOMusic.__pzms_patched then
        BWOMusic.__pzms_patched = true
        if BWOMusic.Process then
            local orig_bwo_process = BWOMusic.Process
            BWOMusic.Process = function(...)
                if not BWOMusic.origReturnVolume then
                    local sm = getSoundManager and getSoundManager()
                    BWOMusic.origReturnVolume = (sm and sm.getMusicVolume and sm:getMusicVolume()) or 0.5
                end
                if not BWOMusic.emitter then return end
                pcall(orig_bwo_process, ...)
            end
        end

        if BWOMusic.Play then
            local orig_bwo_play = BWOMusic.Play
            BWOMusic.Play = function(...)
                pcall(orig_bwo_play, ...)
            end
        end
    end

    if BWOPopControl and not BWOPopControl.__pzms_patched then
        BWOPopControl.__pzms_patched = true
        if BWOPopControl.StreetsSpawn then
            local orig_streets_spawn = BWOPopControl.StreetsSpawn
            BWOPopControl.StreetsSpawn = function(cnt)
                local gmd = GetBWOModData and GetBWOModData()
                if gmd and not gmd.Variant then gmd.Variant = "original" end
                pcall(orig_streets_spawn, cnt)
            end
        end
        if BWOPopControl.UpdateCivs then
            local orig_update_civs = BWOPopControl.UpdateCivs
            BWOPopControl.UpdateCivs = function(...)
                local gmd = GetBWOModData and GetBWOModData()
                if gmd and not gmd.Variant then gmd.Variant = "original" end
                pcall(orig_update_civs, ...)
            end
        end
        if isClient and isClient() then
            BWOPopControl.Zombie = function() end
        end
    end

    if BWOCompatibility and not BWOCompatibility.__pzms_patched then
        BWOCompatibility.__pzms_patched = true
        local orig_get_vars = BWOCompatibility.GetSandboxOptionVars
        BWOCompatibility.GetSandboxOptionVars = function(square)
            local ok, vars = pcall(orig_get_vars, square)
            if ok and type(vars) == "table" then
                for _, var in pairs(vars) do
                    if var and type(var) == "table" then
                        if var[2] == nil or type(var[2]) ~= "number" then var[2] = tonumber(var[2]) or 1.0 end
                        if var[3] == nil or type(var[3]) ~= "number" then var[3] = tonumber(var[3]) or var[2] or 1.0 end
                    end
                end
                return vars
            end
            return {}
        end
    end

    end

end

local function initB42Polyfills()
    initB42Bridge()
    initVanillaGuards()
    initModPatches()
end


initB42Polyfills()

if Events then
    if Events.OnGameBoot then Events.OnGameBoot.Add(initB42Polyfills) end
    if Events.OnGameStart then Events.OnGameStart.Add(initB42Polyfills) end
    if Events.OnInitGlobalModData then Events.OnInitGlobalModData.Add(initB42Polyfills) end
    if Events.OnTick then
        local pzms_ticks = 0
        Events.OnTick.Add(function()
            pzms_ticks = pzms_ticks + 1
            if pzms_ticks <= 60 or pzms_ticks % 120 == 0 then
                initB42Polyfills()
            end
        end)
    end
end
