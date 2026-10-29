-- PZ Mod Studio Link: its own Mods screen, opened by the main menu's MODS button.
--
-- On its own (no app): every installed mod with search (name, id, Workshop id, author) and
-- filters, favourites (the same ones the game's screen keeps), its details (poster, description,
-- what it requires and what needs it), turning mods on and off with what they require, the load
-- order (sorted by it, moved up and down or dragged), several mods at once (Ctrl / Shift),
-- grouping by author or category, each mod's options page (Mod Options), a summary on top that
-- filters, and Accept / Back like the game's screen. What it
-- changes goes to mods/default.txt exactly as the game's screen writes it.
-- Also on its own: the map order (the game's own Map Order screen, on the list being edited),
-- undo / redo (Ctrl+Z, Ctrl+Y), and the list / details split dragged wider or narrower.
-- With PZ Mod Studio: the profile's mods first, Mod Health's warnings on each mod, the mods
-- updated since the last session, the mods installed but in no profile (added in one click),
-- and the player's own notes and groups (made in the app; here they are shown, and a group's
-- heading selects its mods to turn them on or off together). While the app runs, what it changes
-- reaches the game on the main menu (a profile switched, mods turned on or off, a fix package:
-- see Link.readAppList), "Sync" takes it on demand, and "Open PZ Mod Studio" brings it to the front. With
-- the app closed the list is everything installed, as without it.
-- The game's own screen is one button away ("The game's screen"), and another mod manager that
-- owns the Mods screen (see PZModStudio_Link.lua) keeps it: then this screen never opens.

require "PZModStudio_Link"
require "ISUI/ISPanelJoypad"
require "ISUI/ISButton"
require "ISUI/ISTextEntryBox"
require "ISUI/ISScrollingListBox"
require "ISUI/ISRichTextPanel"
require "OptionScreens/MainScreen"
require "OptionScreens/ModSelector/MapOrderUI"
require "ISUI/ISTickBox"
require "ISUI/ISComboBox"
require "RadioCom/ISUIRadio/ISSliderPanel"

-- The base classes, held here: the game reloads its Lua going into a save, and a list left over
-- from the menu still gets every mouse-up; looked up as globals then, they were already gone.
local ListBox, Panel, PanelJoypad = ISScrollingListBox, ISPanel, ISPanelJoypad

local Link = PZModStudioLink
local text = Link.text
local plainId = Link.plainId

local function fontH(font) return getTextManager():getFontHeight(font) end
local function measure(font, s) return getTextManager():MeasureStringX(font, s) end
local WORKSHOP_URL = "https://steamcommunity.com/sharedfiles/filedetails/?id="
local FAVOURITES_FILE = "pz_modlist_settings.cfg"
local UPDATED_FILE = "PZModStudio_LinkUpdated.txt"
local FILTERS = { "all", "on", "off", "favourites", "updated", "problems", "maps", "libraries", "unloadable" }
-- Shown only with PZ Mod Studio: it is what tells.
local APP_FILTERS = { updated = true, libraries = true }
local MODS_FILE = "PZModStudio_LinkMods.txt"
local META_FILE = "PZModStudio_LinkMeta.txt"
local VIEW_FILE = "PZModStudio_LinkView.txt"
-- reopen=1: the game reloaded its Lua for mods synced from PZ Mod Studio while this screen was
-- open, and opens it again.
local REOPEN_FILE = "PZModStudio_LinkReopen.txt"

-- The app's colours (Tailwind slate and emerald), as 0..1.
local C = {
    bg = { 0.008, 0.024, 0.09 },
    panel = { 0.059, 0.09, 0.165 },
    border = { 0.2, 0.255, 0.333 },
    accent = { 0.063, 0.725, 0.506 },
    accentDark = { 0.016, 0.47, 0.34 },
    text = { 0.945, 0.96, 0.976 },
    muted = { 0.58, 0.64, 0.72 },
    -- "Support the author": the app's pink.
    support = { 0.96, 0.6, 0.78 },
    warn = { 0.98, 0.75, 0.14 },
    error = { 0.97, 0.44, 0.44 },
    map = { 0.65, 0.55, 0.98 },
    info = { 0.35, 0.65, 0.98 },
}

local function styleButton(b, primary)
    b.backgroundColor = primary and { r = C.accentDark[1], g = C.accentDark[2], b = C.accentDark[3], a = 1 } or { r = C.panel[1], g = C.panel[2], b = C.panel[3], a = 1 }
    b.backgroundColorMouseOver = primary and { r = C.accent[1], g = C.accent[2], b = C.accent[3], a = 0.8 } or { r = 0.12, g = 0.16, b = 0.23, a = 1 }
    b.borderColor = { r = C.border[1], g = C.border[2], b = C.border[3], a = 1 }
end

local Screen = ISPanelJoypad:derive("StudioLinkModsScreen")
Link.ModsScreen = Screen

-- ------------------------------------------------------------------ data

--- The plain ids of a Java list of ids (nil or not there: none).
local function idsOf(get)
    local out = {}
    local ok, list = pcall(get)
    if ok and list then
        for i = 0, list:size() - 1 do table.insert(out, plainId(tostring(list:get(i)))) end
    end
    return out
end

--- Every installed mod the game can see, once each (the first folder of an id wins, as in the
--- game's screen), with what the screen needs of it.
local function readMods()
    local list, byId = {}, {}
    for _, dir in ipairs(getModDirectoryTable()) do
        local info = getModInfo(dir)
        if info and not byId[plainId(info:getId())] then
            local id = plainId(info:getId())
            local requires = idsOf(function() return info:getRequire() end)
            local mod = {
                id = id,
                gameId = info:getId(),
                info = info,
                dir = tostring(dir),
                name = info:getName() or id,
                author = info:getAuthor() or "",
                requires = requires,
                loadAfter = idsOf(function() return info:getLoadAfter() end),
                loadBefore = idsOf(function() return info:getLoadBefore() end),
                incompatible = idsOf(function() return info:getIncompatible() end),
                available = info:isAvailable(),
                map = Link.isMapMod(info:getId()),
                workshop = tostring(info:getWorkshopID() or ""),
            }
            byId[id] = mod
            table.insert(list, mod)
        end
    end
    table.sort(list, function(a, b) return string.lower(a.name) < string.lower(b.name) end)
    return list, byId
end

--- The active list as the game has it: ids in load order, as written (B42 ids may start with \).
local function readActive()
    local order = {}
    local mods = ActiveMods.getById("default"):getMods()
    for i = 0, mods:size() - 1 do table.insert(order, mods:get(i)) end
    return order
end

--- The game's favourites (its own Mods screen keeps them in pz_modlist_settings.cfg, first line
--- "!fav!:id;id;"), and the rest of that file, kept as it is when they are written back.
local function readFavourites()
    local favs, others = {}, {}
    local file = getFileReader(FAVOURITES_FILE, true)
    if not file then return favs, others end
    local line = file:readLine()
    local first = true
    while line ~= nil do
        if first and string.sub(line, 1, 6) == "!fav!:" then
            for id in string.gmatch(string.sub(line, 7), "[^;]+") do favs[plainId(id)] = true end
        elseif line ~= "" then
            table.insert(others, line)
        end
        first = false
        line = file:readLine()
    end
    file:close()
    return favs, others
end

local function writeFavourites(favs)
    local _, others = readFavourites()
    local ids = {}
    for id, on in pairs(favs) do
        if on then table.insert(ids, id .. ";") end
    end
    local file = getFileWriter(FAVOURITES_FILE, true, false)
    if not file then return end
    file:write("!fav!:" .. table.concat(ids) .. "\n")
    for _, l in ipairs(others) do file:write(l .. "\n") end
    file:close()
end

--- What the app says was updated since the last session (and what Steam has waiting).
local function readUpdated()
    local updated = {}
    local file = getFileReader(UPDATED_FILE, false)
    if not file then return updated end
    local line = file:readLine()
    while line ~= nil do
        local key, id = string.match(line, "^(%a+)=(.+)$")
        if key == "updated" or key == "waiting" then updated[plainId(id)] = key end
        line = file:readLine()
    end
    file:close()
    return updated
end

local function noFacts()
    return { origin = {}, library = {}, tags = {}, origins = {}, tagNames = {}, support = {} }
end

--- What the app knows of each mod for the filters (origin, library, tags) and its words for them:
--- origin=<kind>\t<label>, tag=<id>\t<label>, mod=<id>\t<origin>\t<1 for a library>\t<tag;tag>,
--- support=<id>\t<name>\t<url> (where to support its author).
local function readLinkMods()
    local facts = noFacts()
    local file = getFileReader(MODS_FILE, false)
    if not file then return facts end
    local line = file:readLine()
    while line ~= nil do
        local key, rest = string.match(line, "^(%a+)=(.*)$")
        if key == "origin" or key == "tag" then
            local id, label = string.match(rest, "^([^\t]+)\t(.*)$")
            if id then table.insert(key == "origin" and facts.origins or facts.tagNames, { id = id, label = label }) end
        elseif key == "support" then
            local id, name, url = string.match(rest, "^([^\t]+)\t([^\t]*)\t(https://%S+)$")
            if id then
                id = plainId(id)
                facts.support[id] = facts.support[id] or {}
                table.insert(facts.support[id], { name = name ~= "" and name or url, url = url })
            end
        elseif key == "mod" then
            local id, origin, lib, tags = string.match(rest, "^([^\t]+)\t([^\t]*)\t([^\t]*)\t?(.*)$")
            if id then
                id = plainId(id)
                facts.origin[id] = origin
                if lib == "1" then facts.library[id] = true end
                local set = {}
                for tag in string.gmatch(tags or "", "[^;]+") do set[tag] = true end
                facts.tags[id] = set
            end
        end
        line = file:readLine()
    end
    file:close()
    return facts
end

--- The player's notes and groups, as the app wrote them: notes[id] = text, groups = { {name, ids} }.
local function readMeta()
    local notes, groups, groupNotes = {}, {}, {}
    local file = getFileReader(META_FILE, false)
    if not file then return notes, groups, groupNotes end
    local line = file:readLine()
    while line ~= nil do
        local key, rest = string.match(line, "^(%a+)=(.*)$")
        if key == "note" then
            local id, note = string.match(rest, "^([^\t]+)\t(.*)$")
            if id then notes[plainId(id)] = note end
        elseif key == "groupnote" then
            local name, note = string.match(rest, "^([^\t]+)\t(.*)$")
            if name then groupNotes[name] = note end
        elseif key == "group" then
            local name, ids = string.match(rest, "^([^\t]+)\t(.*)$")
            if name then
                local g = { name = name, ids = {} }
                for id in string.gmatch(ids, "[^;]+") do table.insert(g.ids, plainId(id)) end
                table.insert(groups, g)
            end
        end
        line = file:readLine()
    end
    file:close()
    return notes, groups, groupNotes
end

--- How wide the list is (a share of the screen), as the player last left it.
local function readRatio()
    local ratio = 0.52
    local file = getFileReader(VIEW_FILE, false)
    if file then
        local line = file:readLine()
        while line ~= nil do
            local v = string.match(line, "^ratio=([%d%.]+)")
            if v then ratio = tonumber(v) or ratio end
            line = file:readLine()
        end
        file:close()
    end
    return math.max(0.3, math.min(0.75, ratio))
end

local function writeRatio(ratio)
    local file = getFileWriter(VIEW_FILE, true, false)
    if not file then return end
    file:write("ratio=" .. string.format("%.3f", ratio) .. "\n")
    file:close()
end

-- ------------------------------------------------------------------ the list

local ModList = ISScrollingListBox:derive("StudioLinkModList")

function ModList:new(x, y, w, h, screen)
    local o = ListBox.new(self, x, y, w, h)
    o.screen = screen
    o.font = screen.listFont
    o.fontHgt = fontH(o.font)
    o.itemheight = o.fontHgt + (screen.big and 14 or 10)
    o.drawBorder = true
    o.borderColor = { r = C.border[1], g = C.border[2], b = C.border[3], a = 1 }
    o.backgroundColor = { r = C.panel[1], g = C.panel[2], b = C.panel[3], a = 0.95 }
    o.tickTexture = getTexture("media/ui/inventoryPanes/Tickbox_Tick.png")
    o.starOn = getTexture("media/ui/inventoryPanes/FavouriteYes.png")
    o.starOff = getTexture("media/ui/inventoryPanes/FavouriteNo.png")
    return o
end

-- The columns a click lands in: the star, the tick, the rest (shows the mod).
local STAR_RIGHT, TICK_RIGHT, NAME_X = 26, 52, 60

function ModList:doDrawItem(y, item, alt)
    local h = self.itemheight
    if y + self:getYScroll() + h < 0 or y + self:getYScroll() >= self.height then return y + h end
    local mod = item.item
    local screen = self.screen
    -- A group's heading: the fold arrow, its name and count, "Select all" on the right.
    if mod.header then
        local shown = screen.groupView and screen.groupView.name == mod.header
        self:drawRect(0, y, self.width, h, 0.95, shown and C.accentDark[1] or C.bg[1], shown and C.accentDark[2] or C.bg[2], shown and C.accentDark[3] or C.bg[3])
        local ty = y + (h - fontH(UIFont.Small)) / 2
        self:drawText(screen.collapsed[mod.header] and ">" or "v", 10, ty, C.muted[1], C.muted[2], C.muted[3], 1, UIFont.Small)
        local label = mod.header .. "  (" .. tostring(mod.onCount) .. "/" .. tostring(mod.count) .. ")"
        self:drawText(label, 28, ty, C.accent[1], C.accent[2], C.accent[3], 1, UIFont.Small)
        if screen.groupNotes[mod.header] then
            local nx = 28 + measure(UIFont.Small, label) + 12
            self:drawText(screen.groupNotes[mod.header], nx, ty, 0.55, 0.8, 1, 1, UIFont.Small)
        end
        local sel = text("modsSelectAll")
        local sw = measure(UIFont.Small, sel) + 16
        self:drawRectBorder(self.width - sw - 18, y + 4, sw, h - 8, 1, C.border[1], C.border[2], C.border[3])
        self:drawText(sel, self.width - sw - 10, ty, C.text[1], C.text[2], C.text[3], 1, UIFont.Small)
        self:drawRect(0, y + h - 1, self.width, 1, 0.8, C.border[1], C.border[2], C.border[3])
        return y + h
    end
    -- Where a dragged mod would land.
    if self.dragging and self.dropRow == item.index then
        self:drawRect(0, y, self.width, 3, 1, C.accent[1], C.accent[2], C.accent[3])
    end
    if screen.multi[mod.id] then
        self:drawRect(0, y, self.width, h, 0.35, C.accentDark[1], C.accentDark[2], C.accentDark[3])
    end
    if self.selected == item.index then
        self:drawRect(0, y, self.width, h, 0.55, C.accentDark[1], C.accentDark[2], C.accentDark[3])
    elseif self.mouseoverselected == item.index and self:isMouseOver() then
        self:drawRect(0, y, self.width, h, 0.07, 1, 1, 1)
    end
    local on = screen.on[mod.id]
    local box = math.min(18, h - 8)
    local by = y + (h - box) / 2
    -- Favourite star.
    local star = screen.favs[mod.id] and self.starOn or self.starOff
    if star then self:drawTextureScaled(star, 6, by, box, box, screen.favs[mod.id] and 1 or 0.35, 1, 1, 1) end
    -- The tick: on, off, or cannot be on (not for this build).
    local bx = 30
    if mod.available then
        if on then self:drawRect(bx, by, box, box, 0.9, C.accentDark[1], C.accentDark[2], C.accentDark[3]) end
        self:drawRectBorder(bx, by, box, box, 1, on and C.accent[1] or C.muted[1], on and C.accent[2] or C.muted[2], on and C.accent[3] or C.muted[3])
        if on and self.tickTexture then self:drawTextureScaled(self.tickTexture, bx, by, box, box, 1, 1, 1, 1) end
    else
        self:drawRectBorder(bx, by, box, box, 0.4, C.muted[1], C.muted[2], C.muted[3])
    end
    local col = mod.available and (on and C.text or C.muted) or { 0.4, 0.42, 0.46 }
    local ty = y + (h - self.fontHgt) / 2
    self:drawText(mod.name, NAME_X, ty, col[1], col[2], col[3], 1, self.font)
    -- On the right: marks, then the load position.
    local rx = self.width - 16
    local small = UIFont.Small
    local sty = y + (h - fontH(small)) / 2
    local function tag(label, c)
        local w = measure(small, label)
        rx = rx - w
        self:drawText(label, rx, sty, c[1], c[2], c[3], 1, small)
        rx = rx - 12
    end
    if on and screen.position[mod.id] then tag("#" .. tostring(screen.position[mod.id]), C.muted) end
    if screen.changed[mod.id] then tag("*", C.accent) end
    if mod.map then tag(text("modsMap"), C.map) end
    if screen.notes[mod.id] then tag(text("modsNoteTag"), C.info) end
    if screen.updated[mod.id] == "updated" then tag(text("modsUpdated"), C.info) end
    if screen.updated[mod.id] == "waiting" then tag(text("modsWaiting"), C.warn) end
    if not mod.available then tag(text("modsNotForBuild"), C.muted) end
    if screen.missingReq[mod.id] then tag(text("modsNeedsMore"), C.warn) end
    local health = screen.health[mod.id]
    if health then tag("!", health.severity == "error" and C.error or C.warn) end
    self:drawRect(0, y + h - 1, self.width, 1, 0.6, C.border[1], C.border[2], C.border[3])
    return y + h
end

function ModList:onMouseDown(x, y)
    if #self.items == 0 then return end
    local row = self:rowAt(x, y)
    if row < 1 or row > #self.items then return end
    local mod = self.items[row].item
    local screen = self.screen
    if mod.header then
        getSoundManager():playUISound("UISelectListItem")
        self.selected = row
        local sel = measure(UIFont.Small, text("modsSelectAll")) + 16
        if x < 26 then
            -- The arrow folds or unfolds the group.
            screen.collapsed[mod.header] = not screen.collapsed[mod.header] or nil
            screen:refreshList()
        elseif x > self.width - sel - 22 then
            -- "Select all": every mod of the group, to turn them on or off together.
            screen.multi = {}
            for _, m in ipairs(mod.mods) do screen.multi[m.id] = true end
            screen.groupView = nil
            screen:select(nil, mod.header)
        else
            -- The title: the group itself on the right (its note, its mods, what to do with them).
            screen.multi = {}
            screen.groupView = { name = mod.header, mods = mod.mods }
            screen:select(nil)
        end
        return
    end
    getSoundManager():playUISound("UISelectListItem")
    -- Ctrl adds or takes one mod, Shift takes the rows from the last one clicked: several mods.
    if isCtrlKeyDown() or isShiftKeyDown() then
        if isShiftKeyDown() and self.anchor then
            screen.multi = {}
            local a, b = math.min(self.anchor, row), math.max(self.anchor, row)
            for i = a, b do
                local m = self.items[i] and self.items[i].item
                if m and not m.header then screen.multi[m.id] = true end
            end
        else
            if screen.selectedMod and not screen.selectedMod.header then screen.multi[screen.selectedMod.id] = true end
            screen.multi[mod.id] = not screen.multi[mod.id] or nil
            self.anchor = row
        end
        self.selected = row
        screen:select(mod)
        return
    end
    screen.multi = {}
    screen.groupView = nil
    self.anchor = row
    self.selected = row
    -- Shown first (the list keeps the shown mod selected when it is redrawn).
    screen:select(mod)
    if x < STAR_RIGHT then
        screen:toggleFavourite(mod)
    elseif x < TICK_RIGHT then
        screen:toggle(mod)
    else
        -- Held and moved, it is dragged to another place of the load order.
        self.dragFrom = row
        self.dragStartY = y
        self.dragging = false
    end
end

function ModList:onMouseMove(dx, dy)
    ListBox.onMouseMove(self, dx, dy)
    if self.dragFrom then
        local my = self:getMouseY()
        if not self.dragging and math.abs(my - self.dragStartY) > 6 then self.dragging = true end
        if self.dragging then self.dropRow = self:rowAt(self:getMouseX(), my) end
    end
end

function ModList:onMouseUp(x, y)
    ListBox.onMouseUp(self, x, y)
    if self.dragging and self.dropRow and self.items[self.dragFrom] and self.items[self.dropRow] then
        self.screen:dropAt(self.items[self.dragFrom].item, self.items[self.dropRow].item)
    end
    self.dragFrom, self.dragging, self.dropRow = nil, false, nil
end

function ModList:onMouseUpOutside(x, y)
    ListBox.onMouseUpOutside(self, x, y)
    self.dragFrom, self.dragging, self.dropRow = nil, false, nil
end

function ModList:onMouseDoubleClick(x, y)
    local row = self:rowAt(x, y)
    if row >= 1 and row <= #self.items and x >= TICK_RIGHT and not self.items[row].item.header then self.screen:toggle(self.items[row].item) end
end

-- The handle between the list and the details: dragged, the list gets wider or narrower.
local Divider = ISPanel:derive("StudioLinkModsDivider")

function Divider:new(x, y, h, screen)
    local o = Panel.new(self, x, y, 14, h)
    o.screen = screen
    o.background = false
    return o
end

function Divider:prerender()
    local c = (self.dragging or self:isMouseOver()) and C.accent or C.border
    self:drawRect(6, 0, 2, self.height, 1, c[1], c[2], c[3])
end

-- Pressed, the screen follows the pointer every frame while the button is down (the pointer
-- soon leaves this narrow strip, and its own mouse events with it).
function Divider:onMouseDown(x, y)
    self.dragging = true
    self.screen.resizing = true
    return true
end

-- ------------------------------------------------------------------ the screen

function Screen:new(x, y, w, h)
    local o = PanelJoypad.new(self, x, y, w, h)
    o.backgroundColor = { r = C.bg[1], g = C.bg[2], b = C.bg[3], a = 0.97 }
    o.borderColor = { r = C.border[1], g = C.border[2], b = C.border[3], a = 1 }
    o.filter = "all"
    o.sort = "name"
    o.group = "none"
    o.multi = {}
    o.notes, o.groups, o.groupNotes = {}, {}, {}
    o.collapsed = {}
    o.history, o.future = {}, {}
    o.ratio = readRatio()
    o.search = ""
    -- One of the player's groups, an origin, a tag (nil: all of them).
    o.groupPick, o.originPick, o.tagPick = nil, nil, nil
    o.facts = noFacts()
    o.on, o.changed, o.position, o.missingReq, o.health, o.updated, o.favs = {}, {}, {}, {}, {}, {}, {}
    -- Big screens get bigger rows and text.
    o.big = h >= 900
    o.listFont = o.big and UIFont.Medium or UIFont.Small
    -- Below what other mods draw in the top-left corner (ZombieBuddy lists its Java mods there).
    o.top = 48
    return o
end

function Screen:readState()
    self.mods, self.byId = readMods()
    self.order = readActive()
    self.startOrder = {}
    for i, id in ipairs(self.order) do self.startOrder[i] = plainId(id) end
    self.on = {}
    for _, id in ipairs(self.order) do self.on[plainId(id)] = true end
    self.startOn = {}
    for id, _ in pairs(self.on) do self.startOn[id] = true end
    self.favs = readFavourites()
    -- With the app: its profile (only while it runs: closed, the profile may no longer be what
    -- it holds, so the list is everything installed), Mod Health's warnings and what was updated.
    self.link = Link.appInstalled() and Link.appAlive() and Link.read() or nil
    self.health = self.link and Link.readHealth() or {}
    self.updated = Link.appInstalled() and readUpdated() or {}
    if Link.appInstalled() then self.notes, self.groups, self.groupNotes = readMeta() end
    self.facts = Link.appInstalled() and readLinkMods() or noFacts()
    self.onlyProfile = self.link ~= nil
    self:recount()
end

--- What follows from the working list: positions, changes, requirements not met.
function Screen:recount()
    self.position, self.changed, self.missingReq = {}, {}, {}
    for i, id in ipairs(self.order) do self.position[plainId(id)] = i end
    self.changes = 0
    for _, mod in ipairs(self.mods) do
        if (self.on[mod.id] or false) ~= (self.startOn[mod.id] or false) then
            self.changed[mod.id] = true
            self.changes = self.changes + 1
        end
        if self.on[mod.id] then
            for _, req in ipairs(mod.requires) do
                if not self.on[req] then self.missingReq[mod.id] = true end
            end
        end
    end
    -- A new load order counts as one change.
    self.orderChanged = false
    local kept = {}
    for _, id in ipairs(self.startOrder) do
        if self.on[id] then table.insert(kept, id) end
    end
    local now = {}
    for _, id in ipairs(self.order) do
        if self.startOn[plainId(id)] then table.insert(now, plainId(id)) end
    end
    for i = 1, #kept do
        if kept[i] ~= now[i] then self.orderChanged = true end
    end
    if self.orderChanged then self.changes = self.changes + 1 end
    if self.mapOrderChanged then self.changes = self.changes + 1 end
    -- The summary on top.
    self.counts = { problems = 0, updated = 0, favourites = 0 }
    for _, mod in ipairs(self.mods) do
        if self.health[mod.id] or self.missingReq[mod.id] or (self.on[mod.id] and not mod.available) then self.counts.problems = self.counts.problems + 1 end
        if self.updated[mod.id] then self.counts.updated = self.counts.updated + 1 end
        if self.favs[mod.id] then self.counts.favourites = self.counts.favourites + 1 end
    end
end

--- The mods the list shows now, in the order chosen.
function Screen:visibleMods()
    local out = {}
    -- What was typed, as plain text (a "-" or "(" would mean something to Lua's find).
    local q = string.gsub(string.lower(self.search or ""), "%p", "%%%0")
    for _, mod in ipairs(self.mods) do
        local show = self:inScope(mod)
        local f = self.filter
        if show and f == "on" and not self.on[mod.id] then show = false end
        if show and f == "off" and self.on[mod.id] then show = false end
        if show and f == "favourites" and not self.favs[mod.id] then show = false end
        if show and f == "updated" and not self.updated[mod.id] then show = false end
        if show and f == "maps" and not mod.map then show = false end
        if show and f == "problems" and not (self.health[mod.id] or self.missingReq[mod.id] or (self.on[mod.id] and not mod.available)) then show = false end
        if show and f == "libraries" and not self.facts.library[mod.id] then show = false end
        if show and f == "unloadable" and mod.available then show = false end
        if show and self.groupPick and not self:inGroup(mod, self.groupPick) then show = false end
        if show and self.originPick and self.facts.origin[mod.id] ~= self.originPick then show = false end
        if show and self.tagPick and not (self.facts.tags[mod.id] or {})[self.tagPick] then show = false end
        if show and q ~= "" then
            show = string.find(string.lower(mod.name), q) ~= nil or string.find(string.lower(mod.id), q) ~= nil
                or string.find(mod.workshop, q) ~= nil or string.find(string.lower(mod.author), q) ~= nil
        end
        if show then table.insert(out, mod) end
    end
    if self.sort == "order" then
        -- Mods that are on in their load order, then the rest by name.
        table.sort(out, function(a, b)
            local pa, pb = self.position[a.id], self.position[b.id]
            if pa and pb then return pa < pb end
            if pa or pb then return pa ~= nil end
            return string.lower(a.name) < string.lower(b.name)
        end)
    end
    return out
end

--- Whether the list is about this mod at all: with a profile, its mods and whatever is on (all
--- installed with "All installed"), never the Studio Link itself (on in every profile, as the app
--- leaves it out too).
function Screen:inScope(mod)
    if not self.link then return true end
    if mod.id == Link.SELF_ID then return false end
    return not self.onlyProfile or self.link.members[mod.id] or self.on[mod.id] == true
end

function Screen:inGroup(mod, name)
    for _, g in ipairs(self.groups) do
        if g.name == name then
            for _, id in ipairs(g.ids) do
                if id == mod.id then return true end
            end
        end
    end
    return false
end

--- The group, origin and tag pickers, from what the app wrote: each lists only what some mod the
--- list is about has, with how many (as the app counts); a pick that is gone goes back to "all".
function Screen:fillPicks()
    local function fill(combo, allLabel, items, current)
        if not combo then return nil end
        combo:clear()
        combo:addOptionWithData(allLabel, nil)
        local at = 1
        for i, item in ipairs(items) do
            combo:addOptionWithData(item.label, item.id)
            if item.id == current then at = i + 1 end
        end
        combo.selected = at
        return at > 1 and current or nil
    end
    local function counted(list, has)
        local out = {}
        for _, item in ipairs(list) do
            local n = 0
            for _, mod in ipairs(self.mods) do
                if self:inScope(mod) and has(mod, item.id) then n = n + 1 end
            end
            if n > 0 then table.insert(out, { id = item.id, label = item.label .. " (" .. n .. ")" }) end
        end
        return out
    end
    local groups = {}
    for _, g in ipairs(self.groups) do table.insert(groups, { id = g.name, label = g.name }) end
    groups = counted(groups, function(mod, name) return self:inGroup(mod, name) end)
    self.groupPick = fill(self.groupCombo, text("modsAllGroups"), groups, self.groupPick)
    self.originPick = fill(self.originCombo, text("modsAllOrigins"), counted(self.facts.origins, function(mod, id) return self.facts.origin[mod.id] == id end), self.originPick)
    self.tagPick = fill(self.tagCombo, text("modsAllTags"), counted(self.facts.tagNames, function(mod, id) return (self.facts.tags[mod.id] or {})[id] end), self.tagPick)
end

--- A mod's group: its author(s), or its category (a map mod with none set is a map).
function Screen:groupOf(mod)
    if self.group == "mine" then
        local out = {}
        for _, g in ipairs(self.groups) do
            for _, id in ipairs(g.ids) do
                if id == mod.id then table.insert(out, g.name) end
            end
        end
        if #out == 0 then return text("modsNoGroup") end
        return out
    end
    if self.group == "author" then
        return mod.author ~= "" and mod.author or text("modsNoAuthor")
    end
    local category = mod.info:getCategory() or ""
    if category == "" and mod.map then category = "map" end
    if category == "" then return text("modsNoCategory") end
    local label = text("modsCategory_" .. category)
    return label ~= "modsCategory_" .. category and label or category
end

function Screen:refreshList()
    local keep = self.selectedMod
    self.list:clear()
    local mods = self:visibleMods()
    local rows = {}
    if self.group == "none" then
        rows = mods
    else
        -- Groups by name, the mods of each in the order chosen, a heading above each.
        local groups, keys = {}, {}
        for _, mod in ipairs(mods) do
            local gs = self:groupOf(mod)
            if type(gs) ~= "table" then gs = { gs } end
            -- A mod in several of the player's groups is under each of them.
            for _, g in ipairs(gs) do
                if not groups[g] then
                    groups[g] = {}
                    table.insert(keys, g)
                end
                table.insert(groups[g], mod)
            end
        end
        -- By name; "(no author)", "(in none of your groups)"... last.
        local last = { [text("modsNoAuthor")] = true, [text("modsNoCategory")] = true, [text("modsNoGroup")] = true }
        table.sort(keys, function(a, b)
            if last[a] ~= last[b] then return not last[a] end
            return string.lower(a) < string.lower(b)
        end)
        for _, g in ipairs(keys) do
            local onCount = 0
            for _, mod in ipairs(groups[g]) do
                if self.on[mod.id] then onCount = onCount + 1 end
            end
            table.insert(rows, { header = g, count = #groups[g], onCount = onCount, mods = groups[g], id = "\0" .. g })
            if not self.collapsed[g] then
                for _, mod in ipairs(groups[g]) do table.insert(rows, mod) end
            end
        end
    end
    local pick
    for i, mod in ipairs(rows) do
        self.list:addItem(mod.name or mod.header, mod)
        if keep and mod.id == keep.id then pick = i end
        if not pick and not mod.header and not keep then pick = i end
    end
    if not pick then
        for i, row in ipairs(rows) do
            if not row.header then
                pick = i
                break
            end
        end
    end
    self.list.selected = pick or 1
    if pick then
        self:select(self.list.items[pick].item)
        self.list:ensureVisible(pick)
    else
        self:select(nil)
    end
    self:updateButtons()
end

function Screen:say(message)
    self.note = message
    self.noteAt = Link.now()
end

-- ------------------------------------------------------------------ undo / redo

local function snapshot(self)
    local order, on = {}, {}
    for i, id in ipairs(self.order) do order[i] = id end
    for id, v in pairs(self.on) do on[id] = v end
    return { order = order, on = on }
end

--- Called before every change to the list: what Undo goes back to.
function Screen:checkpoint()
    table.insert(self.history, snapshot(self))
    if #self.history > 50 then table.remove(self.history, 1) end
    self.future = {}
end

function Screen:restore(state)
    self.order, self.on = state.order, state.on
    self:recount()
    self:refreshList()
end

function Screen:undo()
    if #self.history == 0 then return end
    table.insert(self.future, snapshot(self))
    self:restore(table.remove(self.history))
end

function Screen:redo()
    if #self.future == 0 then return end
    table.insert(self.history, snapshot(self))
    self:restore(table.remove(self.future))
end

-- ------------------------------------------------------------------ favourites

function Screen:toggleFavourite(mod)
    if not mod then return end
    self.favs[mod.id] = not self.favs[mod.id] or nil
    pcall(writeFavourites, self.favs)
    if self.filter == "favourites" then self:refreshList() else self:updateButtons() end
end

-- ------------------------------------------------------------------ turning mods on and off

--- The mods `mod` needs that are installed and off, all the way down, and those not installed.
function Screen:needed(mod, out, missing, seen)
    seen = seen or {}
    for _, req in ipairs(mod.requires) do
        if not seen[req] then
            seen[req] = true
            local r = self.byId[req]
            if not r then
                table.insert(missing, req)
            else
                self:needed(r, out, missing, seen)
                if not self.on[req] then table.insert(out, r) end
            end
        end
    end
end

--- The mods that are on and need `mod`, all the way up.
function Screen:dependents(mod, out, seen)
    seen = seen or {}
    for _, other in ipairs(self.mods) do
        if self.on[other.id] and not seen[other.id] then
            for _, req in ipairs(other.requires) do
                if req == mod.id then
                    seen[other.id] = true
                    table.insert(out, other)
                    self:dependents(other, out, seen)
                end
            end
        end
    end
end

function Screen:setOn(mod, on)
    if on and not self.on[mod.id] then
        self.on[mod.id] = true
        table.insert(self.order, mod.gameId)
    elseif not on and self.on[mod.id] then
        self.on[mod.id] = nil
        for i = #self.order, 1, -1 do
            if plainId(self.order[i]) == mod.id then table.remove(self.order, i) end
        end
    end
end

local function names(mods)
    local out = {}
    for i, m in ipairs(mods) do
        if i > 12 then
            table.insert(out, text("andMore", #mods - 12))
            break
        end
        table.insert(out, "- " .. (type(m) == "table" and m.name or tostring(m)))
    end
    return table.concat(out, " <LINE> ")
end

--- On: what it requires comes on with it (said first when there is any). Off: what needs it
--- goes off with it, said first.
function Screen:toggle(mod)
    if not mod then return end
    if not self.on[mod.id] then
        if not mod.available then
            Link.WaitPanel.choose(mod.name, text("modsCannotTurnOn"), 150, { { label = text("back") } })
            return
        end
        local extra, missing = {}, {}
        self:needed(mod, extra, missing)
        local function apply()
            self:checkpoint()
            for _, r in ipairs(extra) do self:setOn(r, true) end
            self:setOn(mod, true)
            self:recount()
            self:refreshList()
        end
        if #extra == 0 and #missing == 0 then return apply() end
        local body = ""
        if #extra > 0 then body = body .. text("modsAlsoOn", mod.name) .. " <LINE> " .. names(extra) .. " <LINE> <LINE> " end
        if #missing > 0 then body = body .. " <RGB:1,0.6,0.3> " .. text("modsMissingReq", mod.name) .. " <RGB:1,1,1> <LINE> " .. names(missing) .. " <LINE> " end
        local lines = #extra + #missing + 6
        Link.WaitPanel.choose(mod.name, body, 100 + lines * (fontH(UIFont.Small) + 2), {
            { label = text("back") },
            { label = text("modsTurnOn"), accept = true, onClick = apply },
        })
    else
        local deps = {}
        self:dependents(mod, deps)
        local function apply()
            self:checkpoint()
            for _, d in ipairs(deps) do self:setOn(d, false) end
            self:setOn(mod, false)
            self:recount()
            self:refreshList()
        end
        if #deps == 0 then return apply() end
        local body = text("modsAlsoOff", mod.name) .. " <LINE> " .. names(deps)
        Link.WaitPanel.choose(mod.name, body, 100 + (#deps + 4) * (fontH(UIFont.Small) + 2), {
            { label = text("back") },
            { label = text("modsTurnOff"), accept = true, onClick = apply },
        })
    end
end

-- ------------------------------------------------------------------ several mods at once

--- The mods the buttons act on when there is no single mod: the selection, or the group shown.
function Screen:actingMods()
    local many = self:multiMods()
    if #many == 0 and self.groupView then return self.groupView.mods end
    return many
end

function Screen:multiMods()
    local out = {}
    for _, mod in ipairs(self.mods) do
        if self.multi[mod.id] then table.insert(out, mod) end
    end
    return out
end

--- Turns on (when any of them is off) or off all the mods selected, with what they require or
--- what needs them, said first.
function Screen:toggleMany(mods)
    local anyOff = false
    for _, mod in ipairs(mods) do
        if not self.on[mod.id] and mod.available then anyOff = true end
    end
    local inSet = {}
    for _, mod in ipairs(mods) do inSet[mod.id] = true end
    if anyOff then
        local extra, missing, seen = {}, {}, {}
        for _, mod in ipairs(mods) do
            if not self.on[mod.id] and mod.available then
                local e, m = {}, {}
                self:needed(mod, e, m)
                for _, r in ipairs(e) do
                    if not inSet[r.id] and not seen[r.id] then
                        seen[r.id] = true
                        table.insert(extra, r)
                    end
                end
                for _, id in ipairs(m) do
                    if not seen[id] then
                        seen[id] = true
                        table.insert(missing, id)
                    end
                end
            end
        end
        local function apply()
            self:checkpoint()
            for _, r in ipairs(extra) do self:setOn(r, true) end
            for _, mod in ipairs(mods) do
                if mod.available then self:setOn(mod, true) end
            end
            self:recount()
            self:refreshList()
        end
        if #extra == 0 and #missing == 0 then return apply() end
        local body = ""
        if #extra > 0 then body = body .. text("modsAlsoOnMany") .. " <LINE> " .. names(extra) .. " <LINE> <LINE> " end
        if #missing > 0 then body = body .. " <RGB:1,0.6,0.3> " .. text("modsMissingReqMany") .. " <RGB:1,1,1> <LINE> " .. names(missing) .. " <LINE> " end
        Link.WaitPanel.choose(text("modsSelected", #mods), body, 100 + (#extra + #missing + 6) * (fontH(UIFont.Small) + 2), {
            { label = text("back") },
            { label = text("modsTurnOnAll"), accept = true, onClick = apply },
        })
    else
        local deps, seen = {}, {}
        for _, mod in ipairs(mods) do
            local d = {}
            self:dependents(mod, d)
            for _, x in ipairs(d) do
                if not inSet[x.id] and not seen[x.id] then
                    seen[x.id] = true
                    table.insert(deps, x)
                end
            end
        end
        local function apply()
            self:checkpoint()
            for _, d in ipairs(deps) do self:setOn(d, false) end
            for _, mod in ipairs(mods) do self:setOn(mod, false) end
            self:recount()
            self:refreshList()
        end
        if #deps == 0 then return apply() end
        Link.WaitPanel.choose(text("modsSelected", #mods), text("modsAlsoOffMany") .. " <LINE> " .. names(deps), 100 + (#deps + 4) * (fontH(UIFont.Small) + 2), {
            { label = text("back") },
            { label = text("modsTurnOffAll"), accept = true, onClick = apply },
        })
    end
end

function Screen:favouriteMany(mods)
    local all = true
    for _, mod in ipairs(mods) do
        if not self.favs[mod.id] then all = false end
    end
    for _, mod in ipairs(mods) do self.favs[mod.id] = (not all) or nil end
    pcall(writeFavourites, self.favs)
    self:recount()
    if self.filter == "favourites" then self:refreshList() else self:updateButtons() end
end

-- ------------------------------------------------------------------ map order

--- The game's own Map Order screen, on the list as it is here (not yet accepted): its model is
--- the game's, pointed at "currentGame" filled with this list. Its order comes back here and is
--- written with the rest on Accept.
function Screen:openMapOrder()
    local temp = ActiveMods.getById("currentGame")
    temp:copyFrom(ActiveMods.getById("default"))
    local list = temp:getMods()
    list:clear()
    for _, id in ipairs(self.order) do list:add(id) end
    if self.mapOrder then
        temp:getMapOrder():clear()
        for _, m in ipairs(self.mapOrder) do temp:getMapOrder():add(m) end
    end
    local model = setmetatable({ isNewGame = true, mapGroups = MapGroups.new() }, { __index = ModSelector.Model })
    if not model:checkMapConflicts() then return self:say(text("modsMapOrderNone")) end
    local ui = MapOrderUI:new(self.x + self.width * 0.15, self.y + self.height * 0.1, self.width * 0.7, self.height * 0.8, model)
    ui:initialise()
    ui:instantiate()
    local screen = self
    local function back()
        ui:setVisible(false)
        ui:removeFromUIManager()
        screen:setVisible(true)
    end
    ui.onOptionMouseDown = function(panel, button)
        if button.internal == "ACCEPT" then
            panel:onAccept()
            screen:checkpoint()
            screen.mapOrder = {}
            local order = temp:getMapOrder()
            for i = 0, order:size() - 1 do table.insert(screen.mapOrder, order:get(i)) end
            screen.mapOrderChanged = true
            screen:recount()
            screen:updateButtons()
            back()
        elseif button.internal == "BACK" then
            back()
        end
    end
    self:setVisible(false)
    ui:addToUIManager()
    ui:bringToTop()
end

-- ------------------------------------------------------------------ Mod Options

--- The game's options page of a mod (B42 Mod Options), when it registered one: only mods that
--- are on run their code, so only those can have it.
function Screen:hasOptions(mod)
    if not mod or not self.startOn[mod.id] then return false end
    return #self:optionsOf(mod) > 0
end

--- The Mod Options objects of a mod (the game's PZAPI.ModOptions; usually one).
function Screen:optionsOf(mod)
    local out = {}
    pcall(function()
        local api = PZAPI and PZAPI.ModOptions
        if not api then return end
        for _, o in ipairs(api.Data or {}) do
            if o.modOptionsID == mod.id or o.modOptionsID == mod.gameId or getText(o.name or "") == mod.name then table.insert(out, o) end
        end
    end)
    return out
end

--- A window over this screen with the mod's own options, saved as the game's Options screen
--- saves them (ModOptions.ini). This screen stays where it was.
local OptionsWindow = ISPanelJoypad:derive("StudioLinkModOptions")

function OptionsWindow:new(screen, mod, list)
    local w = math.min(760, screen.width - 80)
    local h = math.min(screen.height - 120, 640)
    local o = PanelJoypad.new(self, (screen.width - w) / 2, (screen.height - h) / 2, w, h)
    o.screen, o.mod, o.list = screen, mod, list
    o.backgroundColor = { r = C.panel[1], g = C.panel[2], b = C.panel[3], a = 0.99 }
    o.borderColor = { r = C.accent[1], g = C.accent[2], b = C.accent[3], a = 0.8 }
    o.controls = {}
    return o
end

function OptionsWindow:createChildren()
    local pad, bh = 16, math.max(24, fontH(UIFont.Small) + 8)
    local body = ISPanelJoypad:new(pad, 44, self.width - pad * 2, self.height - 44 - bh - 30)
    body:initialise()
    body:instantiate()
    body.background = false
    body:setScrollChildren(true)
    body:addScrollBars()
    body.onMouseWheel = function(panel, del)
        local maxScroll = math.max(0, panel:getScrollHeight() - panel.height)
        panel:setYScroll(math.max(-maxScroll, math.min(0, panel:getYScroll() - del * 40)))
        return true
    end
    -- What does not fit is cut at the window's edge and scrolled into view.
    body.prerender = function(panel)
        PanelJoypad.prerender(panel)
        panel:setStencilRect(0, 0, panel.width, panel.height)
    end
    local controls = self.controls
    body.render = function(panel)
        for _, c in ipairs(controls) do
            if c.kind == "slider" and c.valueLabel then c.valueLabel:setName(tostring(c.control:getCurrentValue())) end
        end
        PanelJoypad.render(panel)
        panel:clearStencilRect()
    end
    self:addChild(body)
    self.body = body
    local labelW = math.floor(body.width * 0.45)
    local cx, cw = labelW + 12, body.width - labelW - 30
    local y = 4
    local function label(t, yy)
        local l = ISLabel:new(0, yy, bh, t, C.text[1], C.text[2], C.text[3], 1, UIFont.Small, true)
        l:initialise()
        body:addChild(l)
    end
    for _, options in ipairs(self.list) do
        if #self.list > 1 then
            label(getText(options.name or ""), y)
            y = y + bh + 4
        end
        for _, option in ipairs(options.data) do
            local t = option.type
            local name = option.name and getText(option.name) or ""
            if t == "title" then
                local l = ISLabel:new(0, y + 6, bh, name, C.accent[1], C.accent[2], C.accent[3], 1, UIFont.Medium, true)
                l:initialise()
                body:addChild(l)
                y = y + bh + 10
            elseif t == "separator" then
                local sep = ISPanel:new(0, y + 6, body.width - 20, 1)
                sep:initialise()
                sep.backgroundColor = { r = C.border[1], g = C.border[2], b = C.border[3], a = 1 }
                body:addChild(sep)
                y = y + 14
            elseif t == "description" then
                local rich = ISRichTextPanel:new(0, y, body.width - 24, 20)
                rich:initialise()
                rich.background = false
                rich.autosetheight = true
                rich.marginLeft, rich.marginRight = 0, 0
                rich:setText(" <RGB:0.58,0.64,0.72> " .. tostring(option.text or ""))
                rich:paginate()
                body:addChild(rich)
                y = y + rich.height + 6
            elseif t == "tickbox" then
                label(name, y)
                local box = ISTickBox:new(cx, y, bh, bh, "", nil, function(_, index, selected)
                    if option.onChange then pcall(option.onChange, option, selected) end
                end)
                box:initialise()
                body:addChild(box)
                box:addOption("")
                box:setSelected(1, option.value == true)
                if option.isEnabled == false then box:disableOption("", true) end
                table.insert(self.controls, { option = option, kind = t, control = box })
                y = y + bh + 6
            elseif t == "multipletickbox" then
                label(name, y)
                local box = ISTickBox:new(cx, y, cw, bh, "", nil, function(_, index, selected)
                    if option.onChange then pcall(option.onChange, option, index, selected) end
                end)
                box:initialise()
                body:addChild(box)
                for i, v in ipairs(option.values) do
                    box:addOption(getText(v.name))
                    box:setSelected(i, v.value == true)
                end
                table.insert(self.controls, { option = option, kind = t, control = box })
                y = y + math.max(bh, box.height) + 6
            elseif t == "combobox" then
                label(name, y)
                local combo = ISComboBox:new(cx, y, math.min(cw, 300), bh, nil, function(_, c)
                    if option.onChange then pcall(option.onChange, option, c.selected) end
                end)
                combo:initialise()
                body:addChild(combo)
                for _, v in ipairs(option.values) do combo:addOption(v) end
                combo.selected = option.selected or 1
                if option.isEnabled == false then combo.disabled = true end
                table.insert(self.controls, { option = option, kind = t, control = combo })
                y = y + bh + 6
            elseif t == "textentry" then
                label(name, y)
                local entry = ISTextEntryBox:new(tostring(option.value or ""), cx, y, math.min(cw, 300), bh)
                entry:initialise()
                entry:instantiate()
                body:addChild(entry)
                table.insert(self.controls, { option = option, kind = t, control = entry })
                y = y + bh + 6
            elseif t == "slider" then
                label(name, y)
                local slider = ISSliderPanel:new(cx + 50, y, math.min(cw, 320) - 50, bh)
                slider:initialise()
                body:addChild(slider)
                slider:setValues(option.min, option.max, option.step, option.step * 10)
                slider:setCurrentValue(option.value, true)
                local valueLabel = ISLabel:new(cx, y, bh, tostring(option.value), C.text[1], C.text[2], C.text[3], 1, UIFont.Small, true)
                valueLabel:initialise()
                body:addChild(valueLabel)
                table.insert(self.controls, { option = option, kind = t, control = slider, valueLabel = valueLabel })
                y = y + bh + 6
            elseif t == "button" then
                local b = ISButton:new(cx, y, math.min(cw, 240), bh, name, nil, function()
                    if option.onclick then pcall(option.onclick, option.target, option.args[1], option.args[2], option.args[3], option.args[4]) end
                end)
                b:initialise()
                b:instantiate()
                styleButton(b, false)
                body:addChild(b)
                y = y + bh + 6
            elseif t == "keybind" or t == "colorpicker" then
                -- Shown, changed in the game's Options (a key or a colour picker of its own).
                label(name, y)
                local value = t == "keybind" and getKeyName(tonumber(option.key) or 0) or text("modsOptionColour")
                local l = ISLabel:new(cx, y, bh, value .. "   (" .. text("modsOptionInOptions") .. ")", C.muted[1], C.muted[2], C.muted[3], 1, UIFont.Small, true)
                l:initialise()
                body:addChild(l)
                y = y + bh + 6
            end
        end
        y = y + 8
    end
    body:setScrollHeight(y + 10)
    local by = self.height - bh - 14
    local save = ISButton:new(self.width - pad - 140, by, 140, bh, text("modsOptionsSave"), self, OptionsWindow.save)
    save:initialise()
    save:instantiate()
    styleButton(save, true)
    self:addChild(save)
    local cancel = ISButton:new(save.x - 130, by, 120, bh, text("back"), self, OptionsWindow.close)
    cancel:initialise()
    cancel:instantiate()
    styleButton(cancel, false)
    self:addChild(cancel)
end

function OptionsWindow:prerender()
    PanelJoypad.prerender(self)
    self:drawText(text("modsOptionsTitle", self.mod.name), 16, 12, C.text[1], C.text[2], C.text[3], 1, UIFont.Medium)
end

--- Written into the options as the game's Options screen does, then saved.
function OptionsWindow:save()
    for _, c in ipairs(self.controls) do
        local o, k = c.option, c.kind
        local ok, err = pcall(function()
            if k == "tickbox" then
                local v = c.control:isSelected(1)
                if o.onChangeApply and o.value ~= v then o:onChangeApply(v) end
                o.value = v
            elseif k == "multipletickbox" then
                for i, val in ipairs(o.values) do
                    local v = c.control:isSelected(i)
                    if o.onChangeApply and val.value ~= v then o:onChangeApply(i, v) end
                    val.value = v
                end
            elseif k == "combobox" then
                local v = c.control.selected
                if o.onChangeApply and o.selected ~= v then o:onChangeApply(v) end
                o.selected = v
            elseif k == "textentry" then
                local v = c.control:getText()
                if o.onChangeApply and o.value ~= v then o:onChangeApply(v) end
                o.value = v
            elseif k == "slider" then
                local v = c.control:getCurrentValue()
                if o.onChangeApply and o.value ~= v then o:onChangeApply(v) end
                o.value = v
            end
        end)
        if not ok then print("[StudioLink] a mod option could not be set: " .. tostring(err)) end
    end
    for _, options in ipairs(self.list) do pcall(function() options:apply() end) end
    pcall(function() PZAPI.ModOptions:save() end)
    self.screen:say(text("modsOptionsSaved", self.mod.name))
    self:close()
end

function OptionsWindow:close()
    self:setVisible(false)
    self:removeFromUIManager()
    self.screen.optionsWindow = nil
end

function OptionsWindow:onKeyRelease(key)
    if key == Keyboard.KEY_ESCAPE then self:close() end
end

--- The mod's options in a window over this screen; nothing else changes.
function Screen:openOptions()
    local mod = self.selectedMod
    if not mod then return end
    local list = self:optionsOf(mod)
    if #list == 0 then return end
    if self.optionsWindow then self.optionsWindow:close() end
    local win = OptionsWindow:new(self, mod, list)
    win:initialise()
    win:instantiate()
    win:addToUIManager()
    win:setAlwaysOnTop(true)
    self.optionsWindow = win
end

-- ------------------------------------------------------------------ load order

--- Moves a mod that is on one place up or down the load order. A mod never goes above what it
--- requires, nor below a mod that requires it: the game would load them the wrong way round.
function Screen:move(mod, step)
    if not mod or not self.on[mod.id] then return end
    local at
    for i, id in ipairs(self.order) do
        if plainId(id) == mod.id then at = i end
    end
    local to = at and at + step
    if not to or to < 1 or to > #self.order then return end
    local other = self.byId[plainId(self.order[to])]
    if other then
        if step < 0 then
            for _, req in ipairs(mod.requires) do
                if req == other.id then return self:say(text("modsCantUp", mod.name, other.name)) end
            end
        else
            for _, req in ipairs(other.requires) do
                if req == mod.id then return self:say(text("modsCantDown", mod.name, other.name)) end
            end
        end
    end
    self:checkpoint()
    self.order[at], self.order[to] = self.order[to], self.order[at]
    self.note = nil
    self:recount()
    if self.sort ~= "order" then
        self.sort = "order"
        self:say(text("modsSortedByOrder"))
    end
    self:refreshList()
end

--- A mod dropped on another row: it goes to that mod's place in the load order, but never above
--- what it requires nor below a mod that requires it (it stops at the nearest place it can go).
--- Sorted by name or in groups too: the place is the target's in the load order, which the list
--- then shows.
function Screen:dropAt(mod, target)
    if not mod or not target or target.header or mod.id == target.id then return end
    if not self.on[mod.id] or not self.on[target.id] then return self:say(text("modsDragOnlyOn")) end
    self:checkpoint()
    local from
    for i, id in ipairs(self.order) do
        if plainId(id) == mod.id then from = i end
    end
    local entry = table.remove(self.order, from)
    local to
    for i, id in ipairs(self.order) do
        if plainId(id) == target.id then to = i end
    end
    -- Dropped below a row it was above: after that row.
    if to >= from then to = to + 1 end
    local lo, hi = 1, #self.order + 1
    for i, id in ipairs(self.order) do
        local other = self.byId[plainId(id)]
        for _, req in ipairs(mod.requires) do
            if req == plainId(id) and i + 1 > lo then lo = i + 1 end
        end
        if other then
            for _, req in ipairs(other.requires) do
                if req == mod.id and i < hi then hi = i end
            end
        end
    end
    local wanted = to
    if to < lo then to = lo end
    if to > hi then to = hi end
    table.insert(self.order, to, entry)
    if to ~= wanted then
        self:say(text("modsDragLimited", mod.name))
    elseif self.sort ~= "order" then
        self:say(text("modsSortedByOrder"))
    else
        self.note = nil
    end
    self.sort = "order"
    self:recount()
    self:select(mod)
    self:refreshList()
end

-- ------------------------------------------------------------------ details

function Screen:select(mod, groupName)
    if mod and mod.header then mod = nil end
    self.selectedMod = mod
    self.selectedGroup = groupName
    if not self.desc then return end
    local many = self:multiMods()
    local view = self.groupView
    if view and #many == 0 then
        local list = {}
        for i, m in ipairs(view.mods) do
            if i > 40 then
                table.insert(list, text("andMore", #view.mods - 40))
                break
            end
            local state = self.on[m.id] and " <RGB:0.45,0.9,0.65> " .. text("modsOnShort") or " <RGB:0.58,0.64,0.72> " .. text("modsOff")
            table.insert(list, " <RGB:1,1,1> - " .. m.name .. "," .. state .. " <RGB:1,1,1> ")
        end
        local head = ""
        if self.groupNotes[view.name] then head = " <RGB:0.55,0.8,1> " .. text("modsGroupNote") .. " " .. self.groupNotes[view.name] .. " <RGB:1,1,1> <LINE> <LINE> " end
        self.desc:setText(head .. " <RGB:0.58,0.64,0.72> " .. text("modsGroupHelp") .. " <RGB:1,1,1> <LINE> <LINE> " .. table.concat(list, " <LINE> "))
        self.desc:paginate()
        self.desc:setYScroll(0)
        self.poster = nil
        self.posterPath = nil
        self:updateButtons()
        return
    end
    if #many > 1 then
        local list = {}
        for i, m in ipairs(many) do
            if i > 30 then
                table.insert(list, text("andMore", #many - 30))
                break
            end
            -- "Name, on": the rich text drops a space that stands alone before a colour.
            local state = self.on[m.id] and " <RGB:0.45,0.9,0.65> " .. text("modsOnShort") or " <RGB:0.58,0.64,0.72> " .. text("modsOff")
            table.insert(list, " <RGB:1,1,1> - " .. m.name .. "," .. state .. " <RGB:1,1,1> ")
        end
        self.desc:setText(" <RGB:0.58,0.64,0.72> " .. text("modsManyHelp") .. " <RGB:1,1,1> <LINE> <LINE> " .. table.concat(list, " <LINE> "))
        self.desc:paginate()
        self.desc:setYScroll(0)
        self.poster = nil
        self.posterPath = nil
        self:updateButtons()
        return
    end
    if not mod then
        self.desc:setText("")
        self.desc:paginate()
        self.poster = nil
        self:updateButtons()
        return
    end
    local info = mod.info
    local parts = {}
    local function line(s) table.insert(parts, s) end
    -- Where it stands and where it comes from, as the app's inspector shows it.
    local function fact(label, value, color)
        -- (<SPACE>: the rich text drops a plain space that stands before a colour)
        line(" <RGB:0.58,0.64,0.72> " .. label .. " <SPACE> <RGB:" .. (color or "0.9,0.92,0.95") .. "> " .. value .. " <RGB:1,1,1> <LINE> ")
    end
    local function nameOf(id)
        local m = self.byId[id]
        return m and m.name or id
    end
    if self.on[mod.id] then
        fact(text("modsState"), text("modsStateOn", self.position[mod.id] or "?", #self.order), "0.45,0.9,0.65")
    else
        fact(text("modsState"), text("modsStateOff"))
    end
    if mod.workshop ~= "" and mod.workshop ~= "0" and string.find(mod.dir, "108600", 1, true) then
        fact(text("modsFrom"), text("modsFromWorkshop", mod.workshop))
    else
        fact(text("modsFrom"), text("modsFromLocal"))
    end
    local support = self.facts.support[mod.id]
    if support then
        local names = {}
        for _, link in ipairs(support) do table.insert(names, link.name) end
        fact(text("modsSupport"), table.concat(names, ", "), table.concat(C.support, ","))
    end
    local okMin, vMin = pcall(function() return info:getVersionMin() end)
    local okMax, vMax = pcall(function() return info:getVersionMax() end)
    vMin = okMin and vMin and tostring(vMin) or nil
    vMax = okMax and vMax and tostring(vMax) or nil
    if vMin and vMax then
        fact(text("modsBuilds"), text("modsBuildsRange", vMin, vMax))
    elseif vMin then
        fact(text("modsBuilds"), text("modsBuildsFrom", vMin))
    elseif vMax then
        fact(text("modsBuilds"), text("modsBuildsUpTo", vMax))
    end
    local function idList(ids, warnOn)
        local out, warn = {}, false
        for _, id in ipairs(ids) do
            table.insert(out, nameOf(id))
            if warnOn and self.on[id] then warn = true end
        end
        return table.concat(out, ", "), warn
    end
    if #mod.loadAfter > 0 then fact(text("modsLoadAfter"), (idList(mod.loadAfter))) end
    if #mod.loadBefore > 0 then fact(text("modsLoadBefore"), (idList(mod.loadBefore))) end
    if #mod.incompatible > 0 then
        local names, clash = idList(mod.incompatible, self.on[mod.id])
        fact(text("modsIncompatible"), names, clash and "1,0.45,0.45" or nil)
    end
    line(" <LINE> ")
    -- The player's own note (with the app).
    if self.notes[mod.id] then line(" <RGB:0.55,0.8,1> " .. text("modsYourNote") .. " " .. self.notes[mod.id] .. " <RGB:1,1,1> <LINE> <LINE> ") end
    -- What Mod Health says (with the app).
    local health = self.health[mod.id]
    if health then
        local color = health.severity == "error" and "1,0.45,0.45" or "1,0.8,0.35"
        for _, l in ipairs(health.lines) do line(" <RGB:" .. color .. "> " .. l .. " <RGB:1,1,1> <LINE> ") end
        line(" <LINE> ")
    end
    if not mod.available then line(" <RGB:1,0.6,0.3> " .. text("modsCannotTurnOn") .. " <RGB:1,1,1> <LINE> <LINE> ") end
    if self.updated[mod.id] == "updated" then line(" <RGB:0.35,0.65,0.98> " .. text("modsUpdatedLong") .. " <RGB:1,1,1> <LINE> <LINE> ") end
    if self.updated[mod.id] == "waiting" then line(" <RGB:0.98,0.75,0.14> " .. text("modsWaitingLong") .. " <RGB:1,1,1> <LINE> <LINE> ") end
    if #mod.requires > 0 then
        line(" <RGB:0.58,0.64,0.72> " .. text("modsRequires") .. " <RGB:1,1,1> <LINE> ")
        for _, req in ipairs(mod.requires) do
            local r = self.byId[req]
            if not r then
                line(" <RGB:1,0.5,0.4> - " .. req .. " (" .. text("modsNotInstalled") .. ") <RGB:1,1,1> <LINE> ")
            elseif self.on[req] then
                line(" <RGB:0.45,0.9,0.65> - " .. r.name .. " <RGB:1,1,1> <LINE> ")
            else
                line(" <RGB:1,0.75,0.4> - " .. r.name .. " (" .. text("modsOff") .. ") <RGB:1,1,1> <LINE> ")
            end
        end
        line(" <LINE> ")
    end
    local needs = {}
    for _, other in ipairs(self.mods) do
        for _, req in ipairs(other.requires) do
            if req == mod.id then table.insert(needs, other) end
        end
    end
    if #needs > 0 then
        line(" <RGB:0.58,0.64,0.72> " .. text("modsNeededBy") .. " <RGB:1,1,1> <LINE> ")
        for i, other in ipairs(needs) do
            if i > 10 then
                line(" " .. text("andMore", #needs - 10) .. " <LINE> ")
                break
            end
            line(" <RGB:0.85,0.87,0.9> - " .. other.name .. " <RGB:1,1,1> <LINE> ")
        end
        line(" <LINE> ")
    end
    local description = info:getDescription() or ""
    if description ~= "" then line(description) end
    self.desc:setText(table.concat(parts, ""))
    self.desc:paginate()
    self.desc:setYScroll(0)
    self.poster = nil
    self.posterPath = nil
    pcall(function()
        if info:getPosterCount() > 0 then
            self.posterPath = info:getPoster(0)
            self.poster = getTexture(self.posterPath)
        end
    end)
    self:updateButtons()
end

--- The support links under their own heading ("Support the author", drawn in prerender), in the
--- pink of that line of the details, so they read as one thing: one wide button for one link,
--- side by side for more. The options button goes under them, or takes their place.
function Screen:layoutSupport(links)
    local n = math.min(#links, #self.supportButtons)
    local w = n > 0 and math.floor((self.buttonW - 6 * (n - 1)) / n) or 0
    local top = self.buttonY + self.buttonStep * 4
    local y = top + self.supportLabelH
    self.supportAt = n > 0 and { x = self.buttonX, y = top + 1 } or nil
    for i, b in ipairs(self.supportButtons) do
        local link = links[i]
        b:setVisible(i <= n)
        if i <= n then
            b.url = link.url
            b:setX(self.buttonX + (w + 6) * (i - 1))
            b:setY(y)
            b:setWidth(i == n and self.buttonW - (w + 6) * (n - 1) or w)
            b:setTitle(link.name)
            b.tooltip = text("modsSupportTip") .. " " .. link.url
        end
    end
    self.optionsButton:setY(n > 0 and y + self.buttonStep or top)
end

function Screen:updateButtons()
    if not self.toggleButton then return end
    local mod = self.selectedMod
    local many = self:actingMods()
    self.groupButton:setTitle(text("modsGroup_" .. self.group))
    self.summary.problems:setTitle(text("modsSumProblems", self.counts.problems))
    self.summary.favourites:setTitle(text("modsSumFavourites", self.counts.favourites))
    if self.summary.updated then self.summary.updated:setTitle(text("modsSumUpdated", self.counts.updated)) end
    self.undoButton:setEnable(#self.history > 0)
    self.redoButton:setEnable(#self.future > 0)
    if #many > 1 or (self.groupView and #self:multiMods() == 0) then
        local anyOff = false
        for _, m in ipairs(many) do
            if not self.on[m.id] and m.available then anyOff = true end
        end
        self.toggleButton:setVisible(true)
        self.toggleButton:setEnable(true)
        self.toggleButton:setTitle(anyOff and text("modsTurnOnAll") or text("modsTurnOffAll"))
        styleButton(self.toggleButton, anyOff)
        self.favButton:setVisible(true)
        self.favButton:setTitle(text("modsFavouriteAll"))
        self.upButton:setVisible(false)
        self.downButton:setVisible(false)
        self.workshopButton:setVisible(false)
        self.optionsButton:setVisible(false)
        for _, b in ipairs(self.supportButtons) do b:setVisible(false) end
        self.supportAt = nil
        self.acceptButton:setTitle(self.changes > 0 and text("modsAcceptN", self.changes) or text("modsAccept"))
        for key, button in pairs(self.filterButtons) do styleButton(button, key == self.filter) end
        self.sortButton:setTitle(self.sort == "order" and text("modsSortOrder") or text("modsSortName"))
        return
    end
    self.optionsButton:setVisible(self:hasOptions(mod))
    self:layoutSupport(mod and self.facts.support[mod.id] or {})
    local has = mod ~= nil
    self.toggleButton:setVisible(has)
    self.workshopButton:setVisible(has)
    self.favButton:setVisible(has)
    self.upButton:setVisible(has and self.on[mod.id] == true)
    self.downButton:setVisible(has and self.on[mod.id] == true)
    if has then
        self.toggleButton:setTitle(self.on[mod.id] and text("modsTurnOff") or text("modsTurnOn"))
        self.toggleButton:setEnable(mod.available or self.on[mod.id] == true)
        styleButton(self.toggleButton, not self.on[mod.id])
        self.workshopButton:setEnable(mod.workshop ~= "" and mod.workshop ~= "0")
        self.favButton:setTitle(self.favs[mod.id] and text("modsUnfavourite") or text("modsFavourite"))
    end
    self.acceptButton:setTitle(self.changes > 0 and text("modsAcceptN", self.changes) or text("modsAccept"))
    for key, button in pairs(self.filterButtons) do
        styleButton(button, key == self.filter)
    end
    self.sortButton:setTitle(self.sort == "order" and text("modsSortOrder") or text("modsSortName"))
    if self.profileTick then
        self.profileTick:setTitle(self.onlyProfile and text("modsOnlyProfileOn") or text("modsOnlyProfileOff"))
    end
end

-- ------------------------------------------------------------------ layout

local function button(parent, x, y, w, h, label, onClick, primary)
    local b = ISButton:new(x, y, w, h, label, parent, onClick)
    b:initialise()
    b:instantiate()
    b:setFont(UIFont.Small)
    styleButton(b, primary)
    parent:addChild(b)
    return b
end

function Screen:createChildren()
    local pad = 20
    local bh = math.max(26, fontH(UIFont.Small) + 10)
    local top = self.top + fontH(UIFont.Large) + fontH(UIFont.Small) + 16
    self.controlsY = top

    self.searchBox = ISTextEntryBox:new("", pad, top, 260, bh)
    self.searchBox.font = UIFont.Small
    self.searchBox:initialise()
    self.searchBox:instantiate()
    self.searchBox:setClearButton(true)
    self.searchBox.tooltip = text("modsSearchTip")
    -- getText() still holds the text before this key; getInternalText() has the key in it.
    self.searchBox.onTextChange = function()
        self.search = self.searchBox:getInternalText() or ""
        self:refreshList()
    end
    self:addChild(self.searchBox)

    self.filterButtons = {}
    local fx = self.searchBox:getRight() + 12
    for _, key in ipairs(FILTERS) do
        -- "Updated" and "Libraries" only with the app (it is what tells).
        if not APP_FILTERS[key] or Link.appInstalled() then
            local label = text("modsFilter_" .. key)
            local b = button(self, fx, top, measure(UIFont.Small, label) + 22, bh, label, function(screen)
                screen.filter = key
                screen:refreshList()
            end)
            self.filterButtons[key] = b
            fx = b:getRight() + 6
        end
    end
    -- The second row, in the app's order: sort, groups, one group, origin, tag, the profile.
    local top2 = top + bh + 6
    self.controlsY2 = top2
    fx = pad
    local sw = math.max(measure(UIFont.Small, text("modsSortName")), measure(UIFont.Small, text("modsSortOrder"))) + 22
    self.sortButton = button(self, fx, top2, sw, bh, text("modsSortName"), function(screen)
        screen.sort = screen.sort == "order" and "name" or "order"
        screen:refreshList()
    end)
    fx = self.sortButton:getRight() + 6
    local gw = 0
    for _, g in ipairs({ "none", "author", "category", "mine" }) do gw = math.max(gw, measure(UIFont.Small, text("modsGroup_" .. g))) end
    self.groupButton = button(self, fx, top2, gw + 22, bh, text("modsGroup_none"), function(screen)
        local cycle = { "none", "author", "category" }
        if #screen.groups > 0 then table.insert(cycle, "mine") end
        local nextGroup = cycle[1]
        for i, g in ipairs(cycle) do
            if g == screen.group then nextGroup = cycle[(i % #cycle) + 1] end
        end
        screen.group = nextGroup
        screen:refreshList()
    end)
    fx = self.groupButton:getRight() + 6
    if Link.appInstalled() then
        local function picker(width, onPick)
            local combo = ISComboBox:new(fx, top2, width, bh, nil, function(_, c)
                onPick(c:getOptionData(c.selected))
                self:refreshList()
            end)
            combo:initialise()
            self:addChild(combo)
            fx = combo:getRight() + 6
            return combo
        end
        self.groupCombo = picker(170, function(v) self.groupPick = v end)
        self.originCombo = picker(170, function(v) self.originPick = v end)
        self.tagCombo = picker(200, function(v) self.tagPick = v end)
        self:fillPicks()
    else
        -- Without the app: what it would add, where those pickers would be.
        self.moreAt = { x = fx + 6, y = top2 + math.floor((bh - fontH(UIFont.Small)) / 2) }
    end

    -- The summary, right of the title: each one shows its mods.
    self.summary = {}
    local sx = self.width - pad
    local sy = self.top
    local function summaryButton(key, label)
        local w = measure(UIFont.Small, label) + 40
        sx = sx - w
        local b = button(self, sx, sy, w, bh, label, function(screen)
            screen.filter = key
            screen:refreshList()
        end)
        sx = sx - 6
        self.summary[key] = b
    end
    summaryButton("favourites", text("modsSumFavourites", 999))
    if Link.appInstalled() then summaryButton("updated", text("modsSumUpdated", 999)) end
    summaryButton("problems", text("modsSumProblems", 999))

    if self.link then
        local w = math.max(measure(UIFont.Small, text("modsOnlyProfileOn")), measure(UIFont.Small, text("modsOnlyProfileOff"))) + 22
        self.profileTick = button(self, fx + 6, top2, w, bh, text("modsOnlyProfileOn"), function(screen)
            screen.onlyProfile = not screen.onlyProfile
            screen:fillPicks()
            screen:refreshList()
        end)
        fx = self.profileTick:getRight() + 6
        -- Installed mods in no profile at all: added to this one, off, by the app.
        local fresh = {}
        for _, mod in ipairs(self.mods) do
            if mod.id ~= Link.SELF_ID and not string.find(mod.id, Link.STUDIO_PACKAGE) and not self.link.members[mod.id] and not self.link.others[mod.id] and not self.on[mod.id] then
                table.insert(fresh, mod.id)
            end
        end
        if #fresh > 0 then
            local nl = text("newMods", #fresh)
            self.newButton = button(self, fx + 6, top2, measure(UIFont.Small, nl) + 22, bh, nl, function(screen, b)
                Link.requestAdd(fresh)
                b:setTitle(text("adding"))
                b:setEnable(false)
            end, true)
        end
    end

    local listTop = top2 + bh + 12
    local bottom = self.height - pad - bh - 14
    local listW = math.floor((self.width - pad * 3) * self.ratio)
    self.layoutTop, self.layoutBottom, self.bh = listTop, bottom, bh
    -- Above and below the list, what its ends mean (as in the app): #1 loads first, the last
    -- one loads last.
    self.barH = fontH(UIFont.Small) + 8
    self.list = ModList:new(pad, listTop + self.barH + 4, listW, bottom - listTop - (self.barH + 4) * 2, self)
    self.list:initialise()
    self.list:instantiate()
    self:addChild(self.list)

    -- Details: the poster on the left, the name and its buttons beside it, the text below.
    local dx = pad * 2 + listW
    local dw = self.width - dx - pad
    self.detailX, self.detailY, self.detailW = dx, listTop, dw
    self.posterSize = math.min(self.big and 256 or 190, math.floor(dw * 0.42))
    local bx = dx + self.posterSize + 16
    local bw = math.min(200, dw - self.posterSize - 16)
    local nameH = fontH(UIFont.Medium) * 2 + fontH(UIFont.Small) * 2 + 18
    local by = listTop + nameH
    self.toggleButton = button(self, bx, by, bw, bh, text("modsTurnOn"), function(screen)
        local many = screen:actingMods()
        if #many > 1 or screen.groupView then screen:toggleMany(many) else screen:toggle(screen.selectedMod) end
    end, true)
    self.favButton = button(self, bx, by + bh + 6, bw, bh, text("modsFavourite"), function(screen)
        local many = screen:actingMods()
        if #many > 1 or screen.groupView then screen:favouriteMany(many) else screen:toggleFavourite(screen.selectedMod) end
    end)
    local half = math.floor((bw - 6) / 2)
    self.upButton = button(self, bx, by + (bh + 6) * 2, half, bh, text("modsUp"), function(screen) screen:move(screen.selectedMod, -1) end)
    self.downButton = button(self, bx + half + 6, by + (bh + 6) * 2, bw - half - 6, bh, text("modsDown"), function(screen) screen:move(screen.selectedMod, 1) end)
    self.workshopButton = button(self, bx, by + (bh + 6) * 3, bw, bh, text("modsWorkshopPage"), function(screen)
        local mod = screen.selectedMod
        if mod then pcall(function() openUrl(WORKSHOP_URL .. mod.workshop) end) end
    end)
    -- Where to support its author (up to three links, side by side); the mod's options below.
    self.buttonX, self.buttonW, self.buttonY, self.buttonStep = bx, bw, by, bh + 6
    self.supportButtons = {}
    self.supportLabelH = fontH(UIFont.Small) + 4
    for i = 1, 3 do
        local b = button(self, bx, by + (bh + 6) * 4 + self.supportLabelH, bw, bh, "", function(screen, clicked)
            if clicked.url then pcall(function() openUrl(clicked.url) end) end
        end)
        b.borderColor = { r = C.support[1], g = C.support[2], b = C.support[3], a = 0.7 }
        b.textColor = { r = C.support[1], g = C.support[2], b = C.support[3], a = 1 }
        b.backgroundColorMouseOver = { r = 0.3, g = 0.08, b = 0.2, a = 1 }
        b.tooltip = text("modsSupportTip")
        b:setVisible(false)
        self.supportButtons[i] = b
    end
    self.optionsButton = button(self, bx, by + (bh + 6) * 5 + self.supportLabelH, bw, bh, text("modsOptions"), function(screen) screen:openOptions() end)
    local descTop = math.max(listTop + self.posterSize, self.optionsButton:getBottom()) + 14
    self.desc = ISRichTextPanel:new(dx, descTop, dw, bottom - descTop - 24)
    self.desc.background = false
    self.desc.autosetheight = false
    self.desc.clip = true
    self.desc.marginLeft = 2
    self.desc.marginRight = 10
    self.desc:initialise()
    self.desc:instantiate()
    self.desc:addScrollBars(true)
    self:addChild(self.desc)
    self.noteY = bottom - 18

    self.divider = Divider:new(dx - pad + 3, listTop, bottom - listTop, self)
    self.divider:initialise()
    self:addChild(self.divider)

    -- The bottom bar: the game's own screen, undo / redo and the map order on the left, Back
    -- and Accept on the right.
    local fy = self.height - pad - bh
    local gs = button(self, pad, fy, measure(UIFont.Small, text("modsGameScreen")) + 26, bh, text("modsGameScreen"), function(screen) screen:openGameScreen() end)
    self.undoButton = button(self, gs:getRight() + 16, fy, measure(UIFont.Small, text("modsUndo")) + 26, bh, text("modsUndo"), function(screen) screen:undo() end)
    self.redoButton = button(self, self.undoButton:getRight() + 6, fy, measure(UIFont.Small, text("modsRedo")) + 26, bh, text("modsRedo"), function(screen) screen:redo() end)
    self.mapOrderButton = button(self, self.redoButton:getRight() + 16, fy, measure(UIFont.Small, text("modsMapOrder")) + 26, bh, text("modsMapOrder"), function(screen) screen:openMapOrder() end)
    -- With PZ Mod Studio: take what it changed (or send what changed here), and open it.
    if Link.appInstalled() then
        self.syncButton = button(self, self.mapOrderButton:getRight() + 16, fy, measure(UIFont.Small, text("modsSync")) + 26, bh, text("modsSync"), function(screen) screen:sync() end)
        self.syncButton.tooltip = text("modsSyncTip")
        self.openAppButton = button(self, self.syncButton:getRight() + 6, fy, measure(UIFont.Small, text("modsOpenApp")) + 26, bh, text("modsOpenApp"), function(screen) screen:openApp() end)
    elseif not Link.inviteHidden() then
        -- Without the app (and its invitation not hidden): where to get it.
        self.getAppButton = button(self, self.mapOrderButton:getRight() + 16, fy, measure(UIFont.Small, text("modsGetApp")) + 26, bh, text("modsGetApp"), function(screen)
            screen:say(Link.copyDownloadLink() and text("modsGetAppCopied", Link.DOWNLOAD_URL) or text("modsGetAppFailed", Link.DOWNLOAD_URL))
        end)
        self.getAppButton.tooltip = text("modsGetAppTip")
    end
    self.acceptButton = button(self, self.width - pad - 200, fy, 200, bh, text("modsAccept"), function(screen) screen:accept() end, true)
    self.backButton = button(self, self.acceptButton.x - 140, fy, 130, bh, text("back"), function(screen) screen:back() end)

    self:refreshList()
end

--- The list and the details, re-placed for a new split (the divider dragged).
function Screen:setRatio(ratio)
    ratio = math.max(0.3, math.min(0.75, ratio))
    if math.abs(ratio - self.ratio) < 0.005 then
        self.divider:setX(self.detailX - 17)
        return
    end
    self.ratio = ratio
    local pad = 20
    local listW = math.floor((self.width - pad * 3) * ratio)
    -- A new list at the new width (the game's list does not take a new width well).
    local old = self.list
    self.list = ModList:new(pad, old.y, listW, old.height, self)
    self.list:initialise()
    self.list:instantiate()
    self:removeChild(old)
    self:addChild(self.list)
    self:refreshList()
    local dx = pad * 2 + listW
    local dw = self.width - dx - pad
    self.detailX, self.detailW = dx, dw
    self.divider:setX(dx - 17)
    self.posterSize = math.min(self.big and 256 or 190, math.floor(dw * 0.42))
    local bx = dx + self.posterSize + 16
    local bw = math.min(200, dw - self.posterSize - 16)
    local half = math.floor((bw - 6) / 2)
    for _, b in ipairs({ self.toggleButton, self.favButton, self.workshopButton, self.optionsButton }) do
        b:setX(bx)
        b:setWidth(bw)
    end
    self.buttonX, self.buttonW = bx, bw
    self:updateButtons()
    self.upButton:setX(bx)
    self.upButton:setWidth(half)
    self.downButton:setX(bx + half + 6)
    self.downButton:setWidth(bw - half - 6)
    local descTop = math.max(self.detailY + self.posterSize, self.buttonY + self.buttonStep * 5 + self.supportLabelH + self.bh) + 14
    self.desc:setX(dx)
    self.desc:setY(descTop)
    self.desc:setWidth(dw)
    self.desc:setHeight(self.layoutBottom - descTop - 24)
    if self.desc.vscroll then
        self.desc.vscroll:setX(dw - self.desc.vscroll.width)
        self.desc.vscroll:setHeight(self.desc.height)
    end
    self.desc:paginate()
end

--- A note at the foot of the details, wrapped to their width (its last line on the foot).
function Screen:drawNote(note)
    local lines, lineText = {}, ""
    for word in string.gmatch(note, "%S+") do
        local try = lineText == "" and word or lineText .. " " .. word
        if lineText ~= "" and measure(UIFont.Small, try) > self.detailW then
            table.insert(lines, lineText)
            lineText = word
        else
            lineText = try
        end
    end
    if lineText ~= "" then table.insert(lines, lineText) end
    local fh = fontH(UIFont.Small)
    for i, l in ipairs(lines) do
        self:drawText(l, self.detailX, self.noteY - (#lines - i) * fh, C.warn[1], C.warn[2], C.warn[3], 1, UIFont.Small)
    end
end

--- The bars above and below the list: #1 loads first, the last # loads last (the # beside each
--- mod is its place in that order, whatever order the list is sorted in).
function Screen:drawBars()
    local l = self.list
    if not l or not self.barH then return end
    local h = self.barH
    local small = UIFont.Small
    local function bar(y, label, detail, position)
        self:drawRect(l.x, y, l.width, h, 0.95, C.panel[1], C.panel[2], C.panel[3])
        self:drawRectBorder(l.x, y, l.width, h, 1, C.border[1], C.border[2], C.border[3])
        self:drawRect(l.x, y, 3, h, 1, C.accent[1], C.accent[2], C.accent[3])
        local ty = y + (h - fontH(small)) / 2
        self:drawText(label, l.x + 12, ty, C.accent[1], C.accent[2], C.accent[3], 1, small)
        local lw = measure(small, label)
        local pw = measure(small, position)
        self:drawText(position, l.x + l.width - pw - 12, ty, C.muted[1], C.muted[2], C.muted[3], 1, small)
        local room = l.width - lw - pw - 48
        if room < 60 then return end
        local d = detail
        if measure(small, d) > room then
            while string.len(d) > 1 and measure(small, d .. "...") > room do d = string.sub(d, 1, -2) end
            d = d .. "..."
        end
        self:drawText(d, l.x + 24 + lw, ty, C.muted[1], C.muted[2], C.muted[3], 1, small)
    end
    bar(l.y - h - 4, text("modsLoadsFirst"), text("modsLoadsFirstWhy"), text("modsPosition", 1))
    bar(l:getBottom() + 4, text("modsLoadsLast"), text("modsLoadsLastWhy"), text("modsPosition", math.max(#self.order, 1)))
end

function Screen:prerender()
    PanelJoypad.prerender(self)
    -- While the divider is held only its line follows the pointer; let go, the list and the
    -- details are laid out again for the new split.
    if self.resizing then
        local ratio = math.max(0.3, math.min(0.75, (self:getMouseX() - 40) / (self.width - 60)))
        if isMouseButtonDown(0) then
            self.divider:setX(40 + math.floor((self.width - 60) * ratio) - 17)
        else
            self.resizing = false
            self.divider.dragging = false
            self:setRatio(ratio)
            pcall(writeRatio, self.ratio)
        end
    end
    local pad = 20
    self:drawText(text("modsTitle"), pad, self.top, C.text[1], C.text[2], C.text[3], 1, UIFont.Large)
    -- Under the title: the profile (with the app) or what is installed (without it).
    local onCount = 0
    for _, _ in pairs(self.on) do onCount = onCount + 1 end
    local subY = self.top + fontH(UIFont.Large) + 2
    if self.link then
        local alive = Link.appAlive()
        local sub = text("modsSubProfile", self.link.name, onCount, #self.mods)
        self:drawText(sub, pad, subY, C.muted[1], C.muted[2], C.muted[3], 1, UIFont.Small)
        local status = "   " .. (alive and text("cardOpen") or text("cardClosed"))
        local c = alive and C.accent or C.muted
        self:drawText(status, pad + measure(UIFont.Small, sub), subY, c[1], c[2], c[3], 1, UIFont.Small)
    else
        local sub = text("modsSub", #self.mods, onCount)
        self:drawText(sub, pad, subY, C.muted[1], C.muted[2], C.muted[3], 1, UIFont.Small)
        if Link.appInstalled() then self:drawText("   " .. text("modsAppClosed"), pad + measure(UIFont.Small, sub), subY, C.muted[1], C.muted[2], C.muted[3], 1, UIFont.Small) end
    end
    self:drawBars()
    if not self.detailX then return end
    -- The details panel behind the poster and the text.
    self:drawRect(self.detailX - 10, self.detailY - 6, self.detailW + 20, self.noteY + 22 - self.detailY, 0.95, C.panel[1], C.panel[2], C.panel[3])
    self:drawRectBorder(self.detailX - 10, self.detailY - 6, self.detailW + 20, self.noteY + 22 - self.detailY, 1, C.border[1], C.border[2], C.border[3])
    -- A poster the game had not loaded yet on the first try is asked for again while drawing.
    if not self.poster and self.posterPath then
        local ok, tex = pcall(getTexture, self.posterPath)
        if ok then self.poster = tex end
    end
    if self.poster then
        self:drawTextureScaledAspect(self.poster, self.detailX, self.detailY, self.posterSize, self.posterSize, 1, 1, 1, 1)
    else
        self:drawRectBorder(self.detailX, self.detailY, self.posterSize, self.posterSize, 0.6, C.border[1], C.border[2], C.border[3])
    end
    local mod = self.selectedMod
    local many = #self:multiMods()
    if self.groupView and many == 0 then
        local onCount = 0
        for _, m in ipairs(self.groupView.mods) do
            if self.on[m.id] then onCount = onCount + 1 end
        end
        self:drawText(self.groupView.name, self.detailX + self.posterSize + 16, self.detailY, C.text[1], C.text[2], C.text[3], 1, UIFont.Medium)
        self:drawText(text("modsGroupCounts", onCount, #self.groupView.mods), self.detailX + self.posterSize + 16, self.detailY + fontH(UIFont.Medium) + 4, C.muted[1], C.muted[2], C.muted[3], 1, UIFont.Small)
    elseif many > 1 then
        local title = self.selectedGroup and text("modsGroupSelected", self.selectedGroup, many) or text("modsSelected", many)
        self:drawText(title, self.detailX + self.posterSize + 16, self.detailY, C.text[1], C.text[2], C.text[3], 1, UIFont.Medium)
    elseif mod then
        -- Beside the poster: the name (up to two lines), then id, Workshop, version, author.
        local nx = self.detailX + self.posterSize + 16
        local nw = self.detailW - self.posterSize - 16
        local ny = self.detailY
        local name = mod.name
        if measure(UIFont.Medium, name) > nw then
            local cut = string.len(name)
            while cut > 1 and measure(UIFont.Medium, string.sub(name, 1, cut)) > nw do cut = cut - 1 end
            self:drawText(string.sub(name, 1, cut), nx, ny, C.text[1], C.text[2], C.text[3], 1, UIFont.Medium)
            ny = ny + fontH(UIFont.Medium)
            name = string.sub(name, cut + 1)
        end
        self:drawText(name, nx, ny, C.text[1], C.text[2], C.text[3], 1, UIFont.Medium)
        ny = ny + fontH(UIFont.Medium) + 4
        local meta = mod.id
        if mod.workshop ~= "" and mod.workshop ~= "0" then meta = meta .. "   Workshop " .. mod.workshop end
        local version = mod.info:getModVersion()
        if version and version ~= "" then meta = meta .. "   v" .. version end
        self:drawText(meta, nx, ny, C.muted[1], C.muted[2], C.muted[3], 1, UIFont.Small)
        ny = ny + fontH(UIFont.Small) + 2
        if mod.author ~= "" then self:drawText(text("modsBy", mod.author), nx, ny, C.muted[1], C.muted[2], C.muted[3], 1, UIFont.Small) end
    end
    if self.moreAt then
        self:drawText(text("modsMoreWithApp"), self.moreAt.x, self.moreAt.y, C.muted[1], C.muted[2], C.muted[3], 1, UIFont.Small)
    end
    -- Last, over the details panel drawn above.
    if self.supportAt then
        self:drawText(text("modsSupport"), self.supportAt.x, self.supportAt.y, C.support[1], C.support[2], C.support[3], 1, UIFont.Small)
    end
    -- A short note (why a move was refused...), for a few seconds.
    if self.note and self.noteAt and Link.now() - self.noteAt < 8 then
        self:drawNote(self.note)
    elseif self.appChanged then
        -- PZ Mod Studio changed the mods while changes made here wait: Sync says which stay.
        self:drawNote(text("modsAppChanged"))
    end
end

-- ------------------------------------------------------------------ PZ Mod Studio

local function writeReopen(on)
    local file = getFileWriter(REOPEN_FILE, true, false)
    if not file then return end
    file:write("reopen=" .. (on and "1" or "0") .. "\n")
    file:close()
end

local function readReopen()
    local file = getFileReader(REOPEN_FILE, false)
    if not file then return false end
    local line = file:readLine()
    file:close()
    return line == "reopen=1"
end

--- Gives the game PZ Mod Studio's list, and reloads the game's Lua when mods came on or went
--- off (as Accept does). False when the game already had it.
local function takeFromApp(list, reopen)
    if not list or Link.sameAsGame(list) then return false end
    local reset = Link.takeAppList(list)
    if reset then
        pcall(writeReopen, reopen)
        if Screen.instance then Screen.instance:close() end
        MainScreen.instance.bottomPanel:setVisible(false)
        getCore():ResetLua("default", "modsChanged")
    end
    return true
end

--- What the app changed is taken as seen (its list now, its files).
local function noteAppSeen(list)
    if list then Link.appListSig = list.sig end
    Link.appSig = Link.appSignature()
end

--- The screen made again from what the game and the app hold now, as the player had it.
function Screen.reopen(note)
    local old = Screen.instance
    local keep = old and {
        filter = old.filter, sort = old.sort, group = old.group, search = old.search,
        groupPick = old.groupPick, originPick = old.originPick, tagPick = old.tagPick,
        collapsed = old.collapsed, shown = old.selectedMod and old.selectedMod.id, onlyProfile = old.onlyProfile,
    }
    Screen.open(keep)
    if note and Screen.instance then Screen.instance:say(note) end
end

--- With changes waiting here, only the app's files are read again (the list stays as edited).
function Screen:refreshAppData()
    self.link = Link.appAlive() and Link.read() or nil
    self.health = self.link and Link.readHealth() or {}
    self.updated = readUpdated()
    self.notes, self.groups, self.groupNotes = readMeta()
    self.facts = readLinkMods()
    -- The stars are the app's too: a mod starred there is starred here.
    self.favs = readFavourites()
    self:fillPicks()
    self:recount()
    self:refreshList()
end

--- Sync: what PZ Mod Studio holds becomes the game's. With changes waiting here, the player says
--- which stay: theirs (Accept, which the app takes into the profile) or the app's.
function Screen:sync()
    if not Link.appAlive() then return self:say(text("modsSyncClosed")) end
    local function pull()
        local list = Link.readAppList()
        noteAppSeen(list)
        if Screen.instance then Screen.instance.appChanged = false end
        if takeFromApp(list, true) then
            if Screen.instance then Screen.reopen(text("modsSynced")) end
        else
            Screen.reopen(text("modsSyncedSame"))
        end
    end
    if self.changes == 0 then return pull() end
    Link.WaitPanel.choose(text("modsSync"), text("modsSyncWhich", self.changes), 170, {
        { label = text("back") },
        { label = text("modsSyncTake"), onClick = pull },
        { label = text("modsSyncSend"), accept = true, onClick = function() self:accept() end },
    })
end

function Screen:openApp()
    self:say(Link.openApp() and text("modsAppShown") or text("modsAppNoOpen"))
end

--- Every few seconds on the main menu: what PZ Mod Studio changed while the game runs reaches
--- it. A new mod list becomes the game's (at once on the main menu or on this screen, unless
--- changes wait here: then Sync asks); a new profile, health, notes or updates are read again.
local function watchApp(ms)
    if ms.inGame or Link.inGame or not Link.appInstalled() then return end
    local t = Link.now()
    if Link.watchedAt and t - Link.watchedAt < 3 then return end
    Link.watchedAt = t
    local screen = Screen.instance
    -- Not under a window of its own (a mod's options, a question): the screen may be made again.
    if Link.dialogUp() or (screen and screen.optionsWindow) then return end
    local list = Link.readAppList()
    if list and Link.appListSig == nil then
        Link.appListSig = list.sig
    elseif list and list.sig ~= Link.appListSig then
        if screen and screen.changes > 0 then
            if not screen.appChanged then
                screen.appChanged = true
                if screen.syncButton then styleButton(screen.syncButton, true) end
            end
        elseif screen or ms.bottomPanel:isVisible() then
            -- (on another screen of the menu, New Game or Load, it waits until that one closes)
            noteAppSeen(list)
            if takeFromApp(list, screen ~= nil) then
                if screen and Screen.instance == screen then Screen.reopen(text("modsFromApp")) end
                return
            end
        end
    end
    local sig = Link.appSignature()
    if Link.appSig == nil then
        Link.appSig = sig
    elseif sig ~= Link.appSig then
        Link.appSig = sig
        if screen and screen.changes == 0 then
            Screen.reopen()
        elseif screen then
            screen:refreshAppData()
        end
    end
end

-- ------------------------------------------------------------------ leaving

function Screen:close()
    self:setVisible(false)
    if self.parent then self.parent:removeChild(self) end
    Screen.instance = nil
    if MainScreen.instance then
        MainScreen.instance.bottomPanel:setVisible(true)
        local joypad = JoypadState.getMainMenuJoypad()
        if joypad then
            joypad.focus = MainScreen.instance
            updateJoypadFocus(joypad)
        end
    end
end

--- Writes the working list to the game's active mods, saved and loaded as the game's screen does.
function Screen:accept()
    if self.changes == 0 then return self:close() end
    local ok, err = pcall(function()
        local active = ActiveMods.getById("default")
        local mods = active:getMods()
        mods:clear()
        -- The chosen order, with each mod after what it requires (the game's own rule).
        local placed = {}
        local function place(id)
            local plain = plainId(id)
            if placed[plain] or not self.on[plain] then return end
            placed[plain] = true
            local mod = self.byId[plain]
            if mod then
                for _, req in ipairs(mod.requires) do
                    if self.byId[req] then place(self.byId[req].gameId) end
                end
            end
            mods:add(id)
        end
        for _, id in ipairs(self.order) do place(id) end
        if self.mapOrder then
            active:getMapOrder():clear()
            for _, m in ipairs(self.mapOrder) do active:getMapOrder():add(m) end
        end
        active:checkMissingMods()
        active:checkMissingMaps()
        saveModsFile()
        ActiveMods.getById("currentGame"):copyFrom(active)
        local reset = ActiveMods.requiresResetLua(active)
        self:close()
        if reset then
            MainScreen.instance.bottomPanel:setVisible(false)
            getCore():ResetLua("default", "modsChanged")
        end
    end)
    if not ok then
        print("[StudioLink] the Mods screen could not save: " .. tostring(err))
        self:close()
    end
end

function Screen:back()
    if self.changes == 0 then return self:close() end
    Link.WaitPanel.choose(text("modsTitle"), text("modsDiscard", self.changes), 150, {
        { label = text("back") },
        { label = text("modsDiscardYes"), onClick = function() self:close() end },
        { label = text("modsAccept"), accept = true, onClick = function() self:accept() end },
    })
end

--- The game's own Mods screen, as its MODS button opens it (changes here are dropped first).
function Screen:openGameScreen()
    self:close()
    local ms = MainScreen.instance
    if not ms or not ms.modSelect then return end
    ms.bottomPanel:setVisible(false)
    ms.modSelect.model.isNewGame = false
    ms.modSelect:setVisible(true, JoypadState.getMainMenuJoypad())
    ms.modSelect.model:reloadMods()
    pcall(ModSelector.showNagPanel)
    ModSelector.instance.returnToUI = ms
end

-- ------------------------------------------------------------------ controller and keys

function Screen:onJoypadDown(button)
    if button == Joypad.AButton then
        self:toggle(self.selectedMod)
    elseif button == Joypad.BButton then
        self:back()
    elseif button == Joypad.YButton then
        self:accept()
    elseif button == Joypad.XButton then
        local shown = {}
        for _, key in ipairs(FILTERS) do
            if self.filterButtons[key] then table.insert(shown, key) end
        end
        for i, key in ipairs(shown) do
            if key == self.filter then
                self.filter = shown[(i % #shown) + 1]
                break
            end
        end
        self:refreshList()
    elseif button == Joypad.LBumper then
        self:move(self.selectedMod, -1)
    elseif button == Joypad.RBumper then
        self:move(self.selectedMod, 1)
    end
end

function Screen:selectRow()
    local row = self.list.items[self.list.selected]
    self:select(row and row.item)
end
function Screen:onJoypadDirUp()
    self.list:onJoypadDirUp()
    local row = self.list.items[self.list.selected]
    if row and row.item.header then self.list:onJoypadDirUp() end
    self:selectRow()
end
function Screen:onJoypadDirDown()
    self.list:onJoypadDirDown()
    local row = self.list.items[self.list.selected]
    if row and row.item.header then self.list:onJoypadDirDown() end
    self:selectRow()
end

function Screen:onKeyRelease(key)
    if key == Keyboard.KEY_ESCAPE then
        self:back()
    elseif key == Keyboard.KEY_Z and isCtrlKeyDown() then
        self:undo()
    elseif key == Keyboard.KEY_Y and isCtrlKeyDown() then
        self:redo()
    end
end

-- ------------------------------------------------------------------ opening it

--- This screen opens from MODS unless another mod manager owns the Mods screen.
function Screen.owns()
    return not Link.otherManager()
end

function Screen.open(keep)
    local ms = MainScreen.instance
    if not ms then return end
    if Screen.instance then Screen.instance:close() end
    local screen = Screen:new(0, 0, ms.width, ms.height)
    screen:readState()
    if keep then
        screen.filter, screen.sort, screen.group = keep.filter, keep.sort, keep.group
        screen.groupPick, screen.originPick, screen.tagPick = keep.groupPick, keep.originPick, keep.tagPick
        screen.search = keep.search or ""
        screen.collapsed = keep.collapsed or {}
        if screen.link and keep.onlyProfile ~= nil then screen.onlyProfile = keep.onlyProfile end
        if keep.shown then screen.selectedMod = screen.byId[keep.shown] end
    end
    screen:initialise()
    screen:instantiate()
    if screen.search ~= "" then screen.searchBox:setText(screen.search) end
    ms:addChild(screen)
    screen:bringToTop()
    Screen.instance = screen
    ms.bottomPanel:setVisible(false)
    local joypad = JoypadState.getMainMenuJoypad()
    if joypad then
        joypad.focus = screen
        updateJoypadFocus(joypad)
    end
end

-- The MODS item calls MainScreen.onMenuItemMouseDownMainMenu, kept on the item when the menu
-- was made: the item is pointed at this one each time the menu shows.
local gameClick = MainScreen.onMenuItemMouseDownMainMenu
local function click(item, x, y)
    if item and item.internal == "MODS" and MainScreen.instance and (MainScreen.instance.delay or 0) <= 0 then
        local ok, owns = pcall(Screen.owns)
        if ok and owns then
            getSoundManager():playUISound("UIActivateMainMenuItem")
            local opened, err = pcall(Screen.open)
            if opened then return end
            print("[StudioLink] the Mods screen could not open, the game's opens instead: " .. tostring(err))
        end
    end
    return gameClick(item, x, y)
end
MainScreen.onMenuItemMouseDownMainMenu = click

local menuPrerender = MainScreen.prerender
function MainScreen:prerender(...)
    menuPrerender(self, ...)
    if self.modsOption and self.modsOption.onMouseDown ~= click then self.modsOption.onMouseDown = click end
    if self.inGame then return end
    -- Back from a reload asked by Sync while this screen was open: it opens again.
    if not Link.reopenChecked then
        Link.reopenChecked = true
        if readReopen() then
            pcall(writeReopen, false)
            local ok, err = pcall(Screen.open)
            if ok and Screen.instance then Screen.instance:say(text("modsSynced")) end
            if not ok then print("[StudioLink] the Mods screen could not open again: " .. tostring(err)) end
        end
    end
    local ok, err = pcall(watchApp, self)
    if not ok and not Link.watchFailed then
        Link.watchFailed = true
        print("[StudioLink] following PZ Mod Studio failed: " .. tostring(err))
    end
end
