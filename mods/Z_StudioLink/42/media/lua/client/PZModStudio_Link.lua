-- PZ Mod Studio Link: the game's Mods screen shows the profile that is active in PZ Mod Studio.
--
-- Files in Zomboid/Lua (the only folder the game's Lua can use), protocol 2:
--   PZModStudio_Link.txt          app:  profile=<name>, member=<mod id> per mod of the profile
--                                       (on or off, in load order), other=<mod id> per mod
--                                       another profile holds
--   PZModStudio_LinkHealth.txt    app:  <severity>\t<mod id>\t<text> per Mod Health warning, in
--                                       the player's language
--   PZModStudio_LinkApp.txt       app:  protocol, version, alive=<unix seconds>, every few
--                                       seconds while it runs
--   PZModStudio_LinkRequests.txt  this: <id>\t<verb>\t<argument> per request (add <mod id>;
--                                       show: the app's window to the front)
--   PZModStudio_LinkAcks.txt      app:  <id>\tok|error\t<text> per request it handled
--   PZModStudio_LinkGame.txt      this: state=menu|ingame, save, mods, on entering either
--   PZModStudio_LinkScreen.txt    both: screen=studio|other|ask, other=<mod id>: which Mods
--                                       screen the player chose when another manager is on
--   PZModStudio_LinkActive.txt    app:  a copy of mods/default.txt, made whenever it changes; a
--                                       new copy while the game runs (the app changed the mods)
--                                       becomes the game's active list
--
-- What it does with them:
-- - Mods screen: only the profile's mods ("Show all subscribed mods" brings the rest back); a
--   button for mods installed but in no profile at all (new subscriptions), which adds them to
--   the profile; a mark on each mod Mod Health has something to say about, with the text on hover.
-- - Load screen: on each save, whether its mods match the profile.
-- - Loading a save: one that uses mods outside the profile does not load; one whose mod list is
--   empty asks first, and can be given the profile's mods.
-- Turning mods on or off here needs nothing from this mod: the game writes mods/default.txt and
-- the Studio reads it. Without the Link file (no profile active in the Studio) the game's screens
-- are left as they are.

require "OptionScreens/ModSelector/ModSelector"
require "OptionScreens/ModSelector/ModSelectorModel"
require "OptionScreens/ModSelector/ModListPanel"
require "OptionScreens/ModSelector/ModListBox"
require "OptionScreens/MainScreen"
require "OptionScreens/LoadGameScreen"

local SELF_ID = "Z_StudioLink"
local FILE = "PZModStudio_Link.txt"
local HEALTH_FILE = "PZModStudio_LinkHealth.txt"
local REQUESTS_FILE = "PZModStudio_LinkRequests.txt"
local STUDIO_PACKAGE = "^Z_PZModStudio_"

local APP_FILE = "PZModStudio_LinkApp.txt"
-- hidden=1 once the player hid the invitation to get the app.
local INVITE_FILE = "PZModStudio_LinkInvite.txt"
local DOWNLOAD_URL = "https://github.com/pzmodstudio/PZModStudio/releases/latest"
local ACKS_FILE = "PZModStudio_LinkAcks.txt"
local GAME_FILE = "PZModStudio_LinkGame.txt"
-- mods/default.txt as the app last saw it: the game takes it when the app changes it.
local ACTIVE_FILE = "PZModStudio_LinkActive.txt"
local PROTOCOL = 2
-- The app writes its file every few seconds; older than this, it is closed.
local APP_ALIVE_SECONDS = 15
-- A request the app has not answered in this long is shown as not done.
local REQUEST_TIMEOUT_SECONDS = 30
-- Mod managers that own the Mods screen themselves. With one of them on, this mod leaves that
-- screen alone (two mods reshaping one list fight) and keeps only the save checks.
-- (mod.info ids: Definitive Mod Manager, PZMM's client, Mod Manager, Mod Load Order Sorter)
local OTHER_MANAGERS = { "DefinitiveModManager", "PZMMClient", "ModManager", "ModLoadOrderSorter_b42" }
local SCREEN_FILE = "PZModStudio_LinkScreen.txt"

-- The texts, in English. The game's translations (media/lua/shared/Translate/<LANG>/UI.json,
-- keys UI_StudioLink_<name>) win when they have the key. {1}, {2}... are filled in here: a "%"
-- in a translation would be read by the game as a format of its own.
local TEXT = {
    profile = "Profile: {1}",
    profileTip = "The profile active in PZ Mod Studio. Only its mods are listed.",
    appClosed = "(PZ Mod Studio closed)",
    appClosedTip = "PZ Mod Studio is not running. The list is the profile as it was when the app last wrote it; what you ask for here is done when the app opens.",
    showAll = "Show all subscribed mods",
    newMods = "+{1} new -> profile",
    newModsTip = "{1} installed mod(s) are in no profile (subscribed outside PZ Mod Studio, for example). Click to add them to \"{2}\", switched off:\n{3}",
    adding = "Adding...",
    addFailed = "Not added",
    addFailedTip = "PZ Mod Studio could not add the mods: {1}",
    noAnswer = "PZ Mod Studio did not answer. Open it and the mods are added then.",
    blocked = "This save uses mods that are not in the profile \"{1}\":",
    howTo = "Switch to the profile you played it with in PZ Mod Studio, or add these mods to this profile.",
    empty = "This save's mod list is empty, while the profile \"{1}\" has {2} mods.",
    emptyWhy = "Loaded like this, the game would run it with no mods and drop every item that came from a mod. Yes gives it the profile's mods and loads it; No goes back.",
    badgeEmpty = "no mods",
    badgeOutside = "{1} outside the profile",
    badgeOk = "profile mods",
    andMore = "...and {1} more",
    changesTitle = "What changed since its last session",
    changesWhen = "\"{1}\" was last played {2} ago. Since then:",
    changesRemoved = "Taken out of its mod list ({1})",
    changesAdded = "Added to its mod list ({1})",
    changesUpdated = "Updated on the Workshop ({1})",
    changesReordered = "The load order of its mods changed.",
    mapTag = "(map)",
    mapsOutWarning = "Map mods were taken out: what they added to the world may be gone when it loads, and the game saves it that way.",
    agoMinutes = "{1} min",
    agoHours = "{1} h",
    agoDays = "{1} days",
    back = "Back",
    playCopy = "Play on a copy",
    loadIt = "Load it",
    copyTitle = "Copying the save",
    copyBody = "PZ Mod Studio is copying \"{1}\" ({2}). The copy loads; the save itself stays as it is.",
    copyNeedsApp = "Playing on a copy needs PZ Mod Studio open: it makes the copy.",
    copyFailed = "The save could not be copied: {1}",
    backupTitle = "Backing up the save",
    backupBeforeLoad = "PZ Mod Studio is backing up \"{1}\" ({2}) before it loads: {3} A big save takes a moment.",
    backupBeforeFix = "PZ Mod Studio is backing up \"{1}\" ({2}) before its mod list is changed.",
    whyChanged = "its mods are not the ones of its last session.",
    whyUpdated = "{1} of its mods were updated since its last session.",
    whyNever = "PZ Mod Studio did not see its last session, so it cannot tell what changed.",
    noAppLoad = "PZ Mod Studio is closed, so \"{1}\" cannot be backed up before it loads. It would be, because {2}",
    noAppFix = "PZ Mod Studio is closed, so \"{1}\" cannot be backed up before its mod list is changed.",
    backupFailed = "The save could not be backed up: {1}",
    goOnAnyway = "Go on without a backup?",
    cancel = "Cancel",
    screenTitle = "Two mod managers",
    screenBody = "PZ Mod Studio and {1} both draw the game's Mods screen, and only one can. Which one do you want? <LINE> <LINE> PZ Mod Studio's: {1} is turned off in this profile and the game reloads its mods (a few seconds). <LINE> The other one: PZ Mod Studio leaves the Mods screen to {1}; save backups and the check before a save loads stay with PZ Mod Studio. <LINE> <LINE> You can change it later in PZ Mod Studio, Settings.",
    screenStudio = "PZ Mod Studio's",
    screenOther = "The other one",
    cardProfileClosed = "Profile: {1} ({2} mods), as PZ Mod Studio left it",
    inviteLine1 = "Mod profiles, a backup of each save before it loads,",
    inviteLine2 = "Mod Health, maps and servers. Free, for Windows.",
    inviteDownload = "Download",
    inviteHide = "Hide",
    modsTitle = "Mods",
    modsSub = "{1} mods installed, {2} on",
    modsSubProfile = "Profile: {1}   {2} on of {3} installed",
    modsFilter_all = "All",
    modsFilter_on = "On",
    modsFilter_off = "Off",
    modsFilter_problems = "Problems",
    modsFilter_maps = "Maps",
    modsOnlyProfileOn = "Profile's mods",
    modsOnlyProfileOff = "All installed",
    modsMap = "map",
    modsNotForBuild = "not for this build",
    modsNeedsMore = "needs more",
    modsCannotTurnOn = "The game cannot load this mod on this build (or something it requires is not installed).",
    modsAlsoOn = "{1} needs these, which are off: they come on with it.",
    modsMissingReq = "{1} also needs these, which are not installed (subscribe to them on the Workshop):",
    modsAlsoOff = "These need {1} and are on: they go off with it.",
    modsTurnOn = "Turn on",
    modsTurnOff = "Turn off",
    modsBy = "By {1}",
    modsRequires = "Requires:",
    modsNotInstalled = "not installed",
    modsOff = "off",
    modsNeededBy = "Needed by:",
    modsAccept = "Accept",
    modsAcceptN = "Accept ({1})",
    modsWorkshopPage = "Workshop page",
    modsGameScreen = "The game's screen",
    modsDiscard = "{1} change(s) not accepted yet. Keep them?",
    modsDiscardYes = "Drop them",
    modsFilter_favourites = "Favourites",
    modsFilter_updated = "Updated",
    modsUpdated = "updated",
    modsWaiting = "update waiting",
    modsUpdatedLong = "Updated on the Workshop since your last session.",
    modsWaitingLong = "Steam has a newer version of it, not downloaded yet.",
    modsCantUp = "{1} cannot go above {2}: it requires it.",
    modsCantDown = "{1} cannot go below {2}: {2} requires it.",
    modsSortedByOrder = "Sorted by load order to show the move.",
    modsSortName = "Sort: name",
    modsSortOrder = "Sort: load order",
    modsFavourite = "Add to favourites",
    modsUnfavourite = "Remove from favourites",
    modsUp = "Move up",
    modsDown = "Move down",
    modsSearchTip = "Name, mod id, Workshop id or author",
    modsDragOnlyOn = "Only mods that are on have a place in the load order.",
    modsFilter_libraries = "Libraries",
    modsFilter_unloadable = "Can't load",
    modsAllGroups = "All groups",
    modsAllOrigins = "All sources",
    modsAllTags = "All tags",
    modsMoreWithApp = "With PZ Mod Studio open there are more filters: Updated, Libraries, a group, source and tag.",
    modsSupport = "Support the author",
    modsSupportTip = "Opens in your browser:",
    modsDragLimited = "{1} stopped where it can go: never above what it requires, never below what requires it.",
    modsSelected = "{1} mods selected",
    modsManyHelp = "Ctrl+click adds or takes a mod, Shift+click takes a range. The buttons act on all of them.",
    modsOnShort = "on",
    modsTurnOnAll = "Turn all on",
    modsTurnOffAll = "Turn all off",
    modsFavouriteAll = "Favourite / unfavourite all",
    modsAlsoOnMany = "They need these, which are off: they come on with them.",
    modsMissingReqMany = "They also need these, which are not installed:",
    modsAlsoOffMany = "These need them and are on: they go off with them.",
    modsOptions = "Mod options",
    modsGroup_none = "Groups: none",
    modsGroup_author = "Groups: author",
    modsGroup_category = "Groups: category",
    modsNoAuthor = "(no author)",
    modsNoCategory = "(no category)",
    modsCategory_map = "Maps",
    modsCategory_vehicle = "Vehicles",
    modsCategory_features = "Features",
    modsCategory_modpack = "Modpacks",
    modsSumProblems = "{1} with problems",
    modsSumUpdated = "{1} updated",
    modsSumFavourites = "{1} favourites",
    modsUndo = "Undo",
    modsRedo = "Redo",
    modsMapOrder = "Map order",
    modsMapOrderNone = "No two map mods that are on share a place: there is no map order to choose.",
    modsNoteTag = "note",
    modsYourNote = "Your note:",
    modsGroup_mine = "Groups: mine",
    modsNoGroup = "(in none of your groups)",
    modsGroupSelected = "{1}: {2} mods",
    modsSelectAll = "Select all",
    modsGroupHelp = "The buttons act on every mod of the group. \"Select all\" on its heading picks them in the list.",
    modsGroupNote = "Note:",
    modsGroupCounts = "{1} on of {2}",
    modsOptionsTitle = "Options of {1}",
    modsOptionsSave = "Save",
    modsOptionsSaved = "Options of {1} saved.",
    modsOptionInOptions = "changed in the game's Options",
    modsOptionColour = "colour",
    cardOpen = "open",
    cardClosed = "closed",
    cardProfile = "Profile: {1} ({2} mods)",
    cardHealthOk = "Mod Health: no problems",
    cardHealth = "Mod Health: {1} error(s), {2} warning(s)",
    cardContinue = "Continue: {1}",
    cardNoChanges = "same mods as its last session",
    cardChanges = "since its last session: {1} updated, {2} added, {3} taken out",
    cardNever = "PZ Mod Studio has not seen it played yet",
    errTitle = "{1} new Lua error(s)",
    errIn = "in {1}",
    errCalledFrom = "called from {1}",
    errOthers = "and {1} more in {2} other mod(s)",
    errNoMod = "in the game's own code (no mod named)",
    errCopy = "Copy report",
    errCopied = "Report copied: paste it where the mod's author can see it.",
    errShowMods = "Show in Mods",
    errSeeStudio = "See in the Studio",
    errClose = "Close",
    errReportTitle = "Lua error in Project Zomboid {1}",
    errReportMod = "Mod: {1}",
    errReportCalled = "Called from: {1}",
    errReportTimes = "Seen {1} time(s) in this session.",
    inviteCopied = "Link copied: paste it in your browser.",
    modsGetApp = "Get PZ Mod Studio",
    modsGetAppTip = "Mod profiles, a backup of each save before it loads, Mod Health, maps and servers. Free, for Windows. Copies the download page's address.",
    modsGetAppCopied = "Download link copied: paste it in your browser (Ctrl+V). {1}",
    modsGetAppFailed = "The link could not be copied. Open it in your browser: {1}",
    modsState = "State:",
    modsStateOn = "On, #{1} of {2} in the load order",
    modsStateOff = "Off (no place in the load order)",
    modsFrom = "From:",
    modsFromWorkshop = "Steam Workshop (item {1})",
    modsFromLocal = "Local folder",
    modsBuilds = "Builds:",
    modsBuildsRange = "{1} to {2}",
    modsBuildsFrom = "{1} and later",
    modsBuildsUpTo = "up to {1}",
    modsLoadAfter = "Loads after:",
    modsLoadBefore = "Loads before:",
    modsIncompatible = "Incompatible with:",
    modsLoadsFirst = "LOADS FIRST",
    modsLoadsFirstWhy = "libraries other mods rely on",
    modsLoadsLast = "LOADS LAST",
    modsLoadsLastWhy = "wins any file two mods share",
    modsPosition = "Position #{1}",
    modsAppClosed = "PZ Mod Studio closed",
    modsAppChanged = "PZ Mod Studio changed the mods. Sync to choose which stay.",
    modsSync = "Sync with PZ Mod Studio",
    modsSyncTip = "Takes what PZ Mod Studio changed (profile, mods on or off, fix packages) into the game. What you accept here goes to the profile.",
    modsSyncClosed = "PZ Mod Studio is closed: open it to sync.",
    modsSynced = "Synced with PZ Mod Studio.",
    modsSyncedSame = "Already the same as PZ Mod Studio.",
    modsSyncWhich = "{1} change(s) made here are not accepted yet, and PZ Mod Studio may hold others. Which stay?",
    modsSyncTake = "PZ Mod Studio's",
    modsSyncSend = "Mine (Accept)",
    modsOpenApp = "Open PZ Mod Studio",
    modsAppNoOpen = "PZ Mod Studio is closed: open it from Windows. With its Settings, \"The close button (X)\", set to keep it in the tray, it stays running, and this button brings it up.",
    modsAppShown = "PZ Mod Studio is in front of the game.",
    modsFromApp = "PZ Mod Studio changed the mods: this is its list now.",
    savesShowAll = "Show all ({1} more)",
    savesOnlyProfile = "Only {1}'s",
}

local translated = {}

--- A text in the player's language, with {1}, {2}... filled in.
local function text(key, ...)
    local value = translated[key]
    if value == nil then
        local full = "UI_StudioLink_" .. key
        local ok, result = pcall(getText, full)
        value = (ok and type(result) == "string" and result ~= "" and result ~= full) and result or TEXT[key] or key
        translated[key] = value
    end
    local args = { ... }
    if #args == 0 then return value end
    return (string.gsub(value, "{(%d)}", function(n)
        local arg = args[tonumber(n)]
        return arg ~= nil and tostring(arg) or ""
    end))
end

--- Seconds since 1970 by the game's clock (the app writes the same clock in its file).
local function now()
    local ok, ms = pcall(getTimestampMs)
    if ok and ms then return math.floor(ms / 1000) end
    return math.floor(os.time())
end

local Link = { showAll = false, pending = {}, waiting = {}, health = {} }
PZModStudioLink = Link

-- ------------------------------------------------------------------ the Studio's files

--- The active profile as the Studio last wrote it, or nil when there is none.
function Link.read()
    local file = getFileReader(FILE, false)
    if not file then return nil end
    local data = { name = "", members = {}, order = {}, others = {} }
    local line = file:readLine()
    while line ~= nil do
        local key, value = string.match(line, "^%s*([%w_]+)%s*=%s*(.-)%s*$")
        if key == "profile" then
            data.name = value
        elseif key == "profileid" then
            data.profileid = value
        elseif key == "member" and value ~= "" then
            if not data.members[value] then table.insert(data.order, value) end
            data.members[value] = true
        elseif key == "other" and value ~= "" then
            data.others[value] = true
        end
        line = file:readLine()
    end
    file:close()
    if data.name == "" and #data.order == 0 then return nil end
    data.members[SELF_ID] = true
    -- Asked for in this session, before the Studio has written them back.
    for id, _ in pairs(Link.pending) do
        if data.members[id] then Link.pending[id] = nil else data.members[id] = true end
    end
    return data
end

--- Mod Health's warnings, per mod id: { severity = "error"|"warning"|..., lines = {...} }.
function Link.readHealth()
    local result = {}
    local file = getFileReader(HEALTH_FILE, false)
    if not file then return result end
    local line = file:readLine()
    while line ~= nil do
        local severity, id, msg = string.match(line, "^([^\t]*)\t([^\t]+)\t(.*)$")
        if id then
            local entry = result[id] or { severity = severity, lines = {} }
            if severity == "error" then entry.severity = "error" end
            table.insert(entry.lines, msg)
            result[id] = entry
        end
        line = file:readLine()
    end
    file:close()
    return result
end

--- Asks the Studio to add these mods to the active profile (switched off). Each request gets an
--- id the app answers to; until then the mods count as in the profile.
function Link.requestAdd(ids)
    if #ids == 0 then return end
    local writer = getFileWriter(REQUESTS_FILE, true, true)
    if not writer then return end
    Link.seq = (Link.seq or 0) + 1
    local id = tostring(now()) .. "-" .. tostring(Link.seq)
    for _, modId in ipairs(ids) do
        writer:write(id .. "\tadd\t" .. modId .. "\n")
        Link.pending[modId] = true
    end
    writer:close()
    Link.waiting[id] = { ids = ids, at = now() }
    Link.addError = nil
end

--- Asks the running Studio to bring its window to the front, on one of its screens ("monitor")
--- or as it is.
function Link.requestShow(screen)
    local writer = getFileWriter(REQUESTS_FILE, true, true)
    if not writer then return end
    Link.seq = (Link.seq or 0) + 1
    writer:write(tostring(now()) .. "-" .. tostring(Link.seq) .. "\tshow\t" .. (screen or "") .. "\n")
    writer:close()
end

--- PZ Mod Studio in front, asked through its file. False when it is closed: the game's Lua cannot
--- start a program (openUrl goes to the Steam overlay), so the player opens it from Windows.
function Link.openApp()
    if not Link.appAlive() then return false end
    Link.requestShow()
    return true
end

--- mods/default.txt as PZ Mod Studio last saw it (its copy in Zomboid/Lua): the mods and maps in
--- order, as written, and the whole text to tell when it changes. Nil without a whole copy.
function Link.readAppList()
    local file = getFileReader(ACTIVE_FILE, false)
    if not file then return nil end
    local list = { mods = {}, maps = {}, hasMods = false, hasMaps = false }
    local parts, block, closed = {}, nil, false
    local line = file:readLine()
    while line ~= nil do
        table.insert(parts, line)
        local trimmed = string.match(line, "^%s*(.-)%s*$")
        if trimmed == "mods" or trimmed == "maps" then
            block = trimmed
            if block == "mods" then list.hasMods = true else list.hasMaps = true end
        elseif trimmed == "}" then
            if block == "mods" then closed = true end
            block = nil
        else
            local key, value = string.match(trimmed, "^(%a+)%s*=%s*(.-),?$")
            if key == "mod" and block == "mods" and value ~= "" then table.insert(list.mods, value) end
            if key == "map" and block == "maps" and value ~= "" then table.insert(list.maps, value) end
        end
        line = file:readLine()
    end
    file:close()
    if not closed then return nil end
    list.sig = table.concat(parts, "\n")
    return list
end

--- Whether the game's active mods (and map order) are already this list.
function Link.sameAsGame(list)
    local active = ActiveMods.getById("default")
    local mods = active:getMods()
    if mods:size() ~= #list.mods then return false end
    for i, id in ipairs(list.mods) do
        if mods:get(i - 1) ~= id then return false end
    end
    if list.hasMaps then
        local maps = active:getMapOrder()
        if maps:size() ~= #list.maps then return false end
        for i, m in ipairs(list.maps) do
            if maps:get(i - 1) ~= m then return false end
        end
    end
    return true
end

--- Gives the game the list PZ Mod Studio wrote. Returns whether the game must reload its Lua
--- (mods came on or went off), as the game's own Mods screen does after Accept.
function Link.takeAppList(list)
    local active = ActiveMods.getById("default")
    local mods = active:getMods()
    mods:clear()
    for _, id in ipairs(list.mods) do mods:add(id) end
    if list.hasMaps then
        active:getMapOrder():clear()
        for _, m in ipairs(list.maps) do active:getMapOrder():add(m) end
    end
    ActiveMods.getById("currentGame"):copyFrom(active)
    return ActiveMods.requiresResetLua(active)
end

--- What the app's files say now, as one text: when it changes, the screens read them again.
function Link.appSignature()
    local parts = { Link.appAlive() and "alive" or "closed" }
    for _, name in ipairs({ FILE, HEALTH_FILE, "PZModStudio_LinkMeta.txt", "PZModStudio_LinkUpdated.txt", "PZModStudio_LinkMods.txt" }) do
        local file = getFileReader(name, false)
        if file then
            local line = file:readLine()
            while line ~= nil do
                table.insert(parts, line)
                line = file:readLine()
            end
            file:close()
        end
        table.insert(parts, "|")
    end
    return table.concat(parts, "\n")
end

--- Whether a request is still waiting for the app's answer. (The game's Lua has no `next`.)
function Link.isWaiting()
    for _ in pairs(Link.waiting) do return true end
    return false
end

--- Whether PZ Mod Studio is running: its file says so, and says it lately.
function Link.appAlive()
    local t = now()
    if Link.appCheckedAt and t - Link.appCheckedAt < 2 then return Link.appIsAlive end
    Link.appCheckedAt = t
    local alive = false
    local file = getFileReader(APP_FILE, false)
    if file then
        local line = file:readLine()
        while line ~= nil do
            local stamp = string.match(line, "^alive=(%d+)")
            if stamp then alive = math.abs(t - tonumber(stamp)) <= APP_ALIVE_SECONDS end
            line = file:readLine()
        end
        file:close()
    end
    Link.appIsAlive = alive
    return alive
end

--- Reads the app's answers to what is still waiting. Returns true when something was settled.
function Link.readAcks()
    if not Link.isWaiting() then return false end
    local settled = false
    local file = getFileReader(ACKS_FILE, false)
    if file then
        local line = file:readLine()
        while line ~= nil do
            local id, status, msg = string.match(line, "^([^\t]+)\t([^\t]+)\t?(.*)$")
            local request = id and Link.waiting[id]
            if request then
                Link.waiting[id] = nil
                settled = true
                if status ~= "ok" then
                    for _, modId in ipairs(request.ids) do Link.pending[modId] = nil end
                    Link.addError = msg ~= "" and msg or status
                end
            end
            line = file:readLine()
        end
        file:close()
    end
    -- Not answered in time: shown as not done. The request stays in the file, so the app still
    -- does it when it opens.
    local t = now()
    for id, request in pairs(Link.waiting) do
        if t - request.at > REQUEST_TIMEOUT_SECONDS then
            Link.waiting[id] = nil
            for _, modId in ipairs(request.ids) do Link.pending[modId] = nil end
            Link.addError = text("noAnswer")
            settled = true
        end
    end
    return settled
end

--- The mod managers on in this game that own the Mods screen, by id.
function Link.otherManager()
    if Link.otherManagerId ~= nil then return Link.otherManagerId or nil end
    Link.otherManagerId = false
    pcall(function()
        local active = ActiveMods.getById("default")
        for _, id in ipairs(OTHER_MANAGERS) do
            if active:isModActive(id) then
                Link.otherManagerId = id
                print("[StudioLink] " .. id .. " is on: it owns the Mods screen; only the save checks stay")
                return
            end
        end
    end)
    return Link.otherManagerId or nil
end

--- Which Mods screen the player chose when another manager is on: { screen = "studio" |
--- "other" | "ask", other = <mod id> }.
function Link.readScreenChoice()
    local choice = { screen = "ask", other = "" }
    pcall(function()
        local file = getFileReader(SCREEN_FILE, false)
        if not file then return end
        local line = file:readLine()
        while line ~= nil do
            local key, value = string.match(line, "^%s*([%w_]+)%s*=%s*(.-)%s*$")
            if key == "screen" then choice.screen = value elseif key == "other" then choice.other = value end
            line = file:readLine()
        end
        file:close()
    end)
    return choice
end

function Link.writeScreenChoice(screen, other)
    pcall(function()
        local file = getFileWriter(SCREEN_FILE, true, false)
        file:write("screen=" .. screen .. "\nother=" .. (other or "") .. "\n")
        file:close()
    end)
end

--- The Studio's screen: every other manager off in the active list, saved and loaded the way the
--- game's own Mods screen does it (the Studio reads mods/default.txt into the profile).
function Link.useStudioScreen()
    local ok, err = pcall(function()
        local active = ActiveMods.getById("default")
        for _, id in ipairs(OTHER_MANAGERS) do
            if active:isModActive(id) then active:setModActive(id, false) end
        end
        Link.writeScreenChoice("studio", "")
        saveModsFile()
        ActiveMods.getById("currentGame"):copyFrom(active)
        getCore():ResetLua("default", "modsChanged")
    end)
    if not ok then print("[StudioLink] could not turn the other manager off: " .. tostring(err)) end
end

--- In the main menu, once: another manager is on and the player has not chosen it.
function Link.askScreen()
    local other = Link.otherManager()
    if not other or Link.screenAsked then return end
    -- Without a profile from the Studio its screen has nothing to show: nothing to choose.
    if not Link.read() then return end
    local choice = Link.readScreenChoice()
    if choice.screen == "other" and choice.other == other then return end
    Link.screenAsked = true
    local name = other
    pcall(function()
        local info = getModInfoByID(other)
        if info and info:getName() then name = info:getName() end
    end)
    local lines = 9
    local height = 110 + lines * (getTextManager():getFontHeight(UIFont.Small) + 2)
    -- Link.WaitPanel: the panel is defined further down this file.
    Link.WaitPanel.choose(text("screenTitle"), text("screenBody", name), height, {
        { label = text("screenOther"), onClick = function() Link.writeScreenChoice("other", other) end },
        { label = text("screenStudio"), accept = true, onClick = Link.useStudioScreen },
    })
end

--- What the main menu card shows, read again every few seconds (the files are small).
function Link.cardData()
    local t = now()
    if Link.card and t - Link.card.at < 3 then return Link.card end
    local card = { at = t }
    pcall(function()
        local link = Link.read()
        if not link then return end
        -- Another profile made active in the app while on the menu: Continue is its save now.
        if link.profileid and link.profileid ~= "" and link.profileid ~= Link.latestFor then
            MainScreen.instance:getLatestSave()
            local screen = MainScreen.instance
            if screen and screen.latestSaveOption and not MainScreen.latestSaveWorld then
                screen.latestSaveOption:setVisible(false)
            end
        end
        card.profile = link.name
        card.size = #link.order
        card.alive = Link.appAlive()
        local errors, warnings = 0, 0
        for _, entry in pairs(Link.readHealth()) do
            if entry.severity == "error" then errors = errors + 1 elseif entry.severity == "warning" then warnings = warnings + 1 end
        end
        card.errors, card.warnings = errors, warnings
        -- The save Continue loads, and what changed in its mods since it was last played.
        local world, mode = MainScreen.latestSaveWorld, MainScreen.latestSaveGameMode
        -- The game remembers the last save by name (latestSave.ini) even after it is deleted:
        -- only one the Load screen would list counts. Looked up once per visit to the menu.
        if world and mode and Link.latestExists == nil then
            Link.latestExists = false
            for _, k in ipairs(getFullSaveDirectoryTable()) do
                local dir = string.gsub(tostring(k), "\\", "/")
                if dir == tostring(mode) .. "/" .. tostring(world) then Link.latestExists = true end
            end
        end
        if world and mode and Link.latestExists then
            card.world = world
            local saves = Link.readSaves() or {}
            local save = saves[tostring(mode) .. "/" .. tostring(world)]
            if not save or save.session == 0 then
                card.never = true
            else
                card.changes = Link.sessionChanges(save, ActiveMods.getById("default"))
            end
        end
    end)
    Link.card = card
    return card
end

--- The card in the main menu's top-right corner: the profile, whether the Studio is open, Mod
--- Health, and the save Continue would load. Only with a profile from the Studio.
function Link.drawCard(screen)
    local card = Link.cardData()
    if not card.profile then return end
    local font = UIFont.Small
    local lineH = getTextManager():getFontHeight(font) + 3
    local lines = {}
    if card.alive then
        table.insert(lines, { text("cardProfile", card.profile, card.size), 1, 1, 1 })
    else
        table.insert(lines, { text("cardProfileClosed", card.profile, card.size), 0.85, 0.85, 0.85 })
    end
    if card.errors > 0 or card.warnings > 0 then
        table.insert(lines, { text("cardHealth", card.errors, card.warnings), 1, card.errors > 0 and 0.5 or 0.8, 0.35 })
    else
        table.insert(lines, { text("cardHealthOk"), 0.55, 0.9, 0.6 })
    end
    if card.world then
        table.insert(lines, { text("cardContinue", card.world), 0.85, 0.85, 0.85 })
        if card.never then
            table.insert(lines, { "  " .. text("cardNever"), 0.6, 0.6, 0.6 })
        elseif card.changes then
            table.insert(lines, { "  " .. text("cardChanges", #card.changes.updated, #card.changes.added, #card.changes.removed), 1, 0.75, 0.35 })
        else
            table.insert(lines, { "  " .. text("cardNoChanges"), 0.6, 0.6, 0.6 })
        end
    end
    local status = card.alive and text("cardOpen") or text("cardClosed")
    local sw = getTextManager():MeasureStringX(font, status)
    -- As wide as the longest line, and never narrower than the title with the status beside it.
    local w = getTextManager():MeasureStringX(UIFont.Medium, "PZ Mod Studio") + sw + 24
    for _, l in ipairs(lines) do w = math.max(w, getTextManager():MeasureStringX(font, l[1])) end
    w = math.min(w + 24, screen.width - 40)
    local h = 30 + #lines * lineH + 8
    local x, y = screen.width - w - 20, 20
    screen:drawRect(x, y, w, h, 0.82, 0.05, 0.06, 0.08)
    screen:drawRectBorder(x, y, w, h, 0.8, 0.35, 0.75, 1.0)
    screen:drawText("PZ Mod Studio", x + 12, y + 7, 0.6, 0.85, 1, 1, UIFont.Medium)
    if card.alive then
        screen:drawText(status, x + w - sw - 12, y + 9, 0.45, 0.9, 0.55, 1, font)
    else
        screen:drawText(status, x + w - sw - 12, y + 9, 0.6, 0.6, 0.6, 1, font)
    end
    local ly = y + 30
    for _, l in ipairs(lines) do
        screen:drawText(l[1], x + 12, ly, l[2], l[3], l[4], 1, font)
        ly = ly + lineH
    end
end

--- Whether PZ Mod Studio has ever run on this PC (it writes its heartbeat file when it does).
--- Without it the Link shows nothing of the app: only the invitation to get it.
function Link.appInstalled()
    if Link.installed ~= nil then return Link.installed end
    local file = getFileReader(APP_FILE, false)
    Link.installed = file ~= nil
    if file then file:close() end
    return Link.installed
end

--- The download page's address on the clipboard, to paste in the browser (the game's openUrl
--- goes to the Steam overlay, which is no place to download a program). True when copied.
function Link.copyDownloadLink()
    return pcall(function() Clipboard.setClipboard(DOWNLOAD_URL) end)
end

function Link.inviteHidden()
    local hidden = false
    local file = getFileReader(INVITE_FILE, false)
    if file then
        local line = file:readLine()
        while line ~= nil do
            if string.match(line, "^%s*hidden%s*=%s*1") then hidden = true end
            line = file:readLine()
        end
        file:close()
    end
    return hidden
end

--- The invitation, for players who have the mod but not the app: what the app does, a button
--- that copies its download page's address (for the browser) and one to hide it for good.
local InvitePanel = ISPanel:derive("StudioLinkInvitePanel")

function InvitePanel:new(x, y, w, h)
    local o = ISPanel.new(self, x, y, w, h)
    o.backgroundColor = { r = 0.05, g = 0.06, b = 0.08, a = 0.82 }
    o.borderColor = { r = 0.35, g = 0.75, b = 1.0, a = 0.8 }
    return o
end

function InvitePanel:createChildren()
    local bh = 22
    local download = ISButton:new(12, self.height - bh - 10, 110, bh, text("inviteDownload"), self, function(panel)
        if Link.copyDownloadLink() then panel.copiedAt = now() end
    end)
    download.tooltip = DOWNLOAD_URL
    download:initialise()
    download:instantiate()
    download.borderColor = { r = 0.35, g = 0.85, b = 0.5, a = 0.9 }
    self:addChild(download)
    local hide = ISButton:new(130, self.height - bh - 10, 80, bh, text("inviteHide"), self, function(panel)
        pcall(function()
            local file = getFileWriter(INVITE_FILE, true, false)
            file:write("hidden=1\n")
            file:close()
        end)
        panel:setVisible(false)
        panel:removeFromUIManager()
        Link.invitePanel = nil
    end)
    hide:initialise()
    hide:instantiate()
    self:addChild(hide)
end

function InvitePanel:prerender()
    ISPanel.prerender(self)
    self:drawText("PZ Mod Studio", 12, 7, 0.6, 0.85, 1, 1, UIFont.Medium)
    local lineH = getTextManager():getFontHeight(UIFont.Small) + 3
    self:drawText(text("inviteLine1"), 12, 32, 0.85, 0.85, 0.85, 1, UIFont.Small)
    if self.copiedAt and now() - self.copiedAt < 8 then
        self:drawText(text("inviteCopied"), 12, 32 + lineH, 0.45, 0.9, 0.65, 1, UIFont.Small)
    else
        self:drawText(text("inviteLine2"), 12, 32 + lineH, 0.85, 0.85, 0.85, 1, UIFont.Small)
    end
end

--- Shows the invitation in the menu's top-right corner (made once per visit to the menu).
function Link.showInvite(screen)
    if Link.inviteHidden() then return end
    if Link.invitePanel then
        Link.invitePanel:setVisible(true)
        return
    end
    local font = UIFont.Small
    local w = math.max(getTextManager():MeasureStringX(font, text("inviteLine1")), getTextManager():MeasureStringX(font, text("inviteLine2"))) + 24
    w = math.max(w, 240)
    local h = 32 + 2 * (getTextManager():getFontHeight(font) + 3) + 40
    local panel = InvitePanel:new(screen.width - w - 20, 20, w, h)
    panel:initialise()
    panel:instantiate()
    panel:addToUIManager()
    Link.invitePanel = panel
end

function Link.hideInvite()
    if Link.invitePanel then Link.invitePanel:setVisible(false) end
end

-- Started from the app on one save ("Custom" launch): the app leaves `save=<mode>/<world>` and
-- when it asked; once the main menu is up and ready, that save loads the way Continue loads one
-- (so the save checks and the backup still come first). Read once per game start, then emptied,
-- and only a request from the last ten minutes counts: a leftover never loads a save later.
local LOAD_FILE = "PZModStudio_LinkLoad.txt"
Link.loadChecked = false
function Link.loadAsked(screen)
    if Link.loadChecked then return end
    if screen.delay and screen.delay > 0 then return end
    if screen.checkSavefileModal then return end
    Link.loadChecked = true
    local file = getFileReader(LOAD_FILE, false)
    if not file then return end
    local save, at
    local line = file:readLine()
    while line ~= nil do
        local key, value = string.match(line, "^(%a+)=(.*)$")
        if key == "save" then save = value elseif key == "at" then at = tonumber(value) end
        line = file:readLine()
    end
    file:close()
    local writer = getFileWriter(LOAD_FILE, true, false)
    if writer then writer:close() end
    if not save or not at or math.abs(now() - at) > 600 then return end
    local mode, world = string.match(save, "^([^/]+)/(.+)$")
    if not mode or mode == "Multiplayer" then return end
    print("[StudioLink] loading the save the app started the game on: " .. save)
    MainScreen.continueLatestSave(mode, world)
end

-- Asked from the main menu's own drawing (OnTick does not run in the menu), a moment after it
-- shows, and only in the menu (the same screen is the pause menu in a game).
local askFrames = -1
local mainPrerender = MainScreen.prerender
function MainScreen:prerender(...)
    mainPrerender(self, ...)
    -- The card, in the main menu proper (its buttons showing), not in a game's pause menu.
    if not Link.inGame and self.bottomPanel and self.bottomPanel:isVisible() then
        -- With the app on this PC: what it knows. Without it: only the invitation to get it.
        local ok, err = pcall(function()
            if Link.appInstalled() then Link.drawCard(self) else Link.showInvite(self) end
        end)
        if not ok and not Link.cardFailed then
            Link.cardFailed = true
            print("[StudioLink] the main menu card failed: " .. tostring(err))
        end
    else
        pcall(Link.hideInvite)
    end
    if not Link.inGame and self.bottomPanel and self.bottomPanel:isVisible() then
        local ok, err = pcall(Link.loadAsked, self)
        if not ok then print("[StudioLink] the save the app asked for did not load: " .. tostring(err)) end
    end
    if askFrames < 0 or Link.inGame then return end
    askFrames = askFrames + 1
    if askFrames < 30 then return end
    askFrames = -1
    local ok, err = pcall(Link.askScreen)
    if not ok then print("[StudioLink] the Mods screen question failed: " .. tostring(err)) end
end

--- Tells the app where the player is. state = "menu" or "ingame".
function Link.writeGameState(state)
    local ok, err = pcall(function()
        local writer = getFileWriter(GAME_FILE, true, false)
        if not writer then return end
        local version = ""
        pcall(function() version = getModInfoByID(SELF_ID):getModVersion() or "" end)
        writer:write("protocol=" .. PROTOCOL .. "\n")
        writer:write("link=" .. tostring(version) .. "\n")
        writer:write("state=" .. state .. "\n")
        if state == "ingame" then
            local world = getWorld()
            local name = tostring(world:getWorld() or "")
            if name ~= "" then
                writer:write("save=" .. tostring(world:getGameMode()) .. "/" .. name .. "\n")
            end
            local mods = getActivatedMods()
            for i = 0, mods:size() - 1 do
                writer:write("mod=" .. tostring(mods:get(i)) .. "\n")
            end
        end
        writer:write("at=" .. now() .. "\n")
        writer:close()
    end)
    if not ok then print("[StudioLink] could not write the game state: " .. tostring(err)) end
end

-- Named, so another mod (or a reload of this one) can take them out again.
function Link.onMainMenuEnter()
    Link.inGame = false
    Link.latestExists = nil
    Link.card = nil
    Link.installed = nil
    Link.writeGameState("menu")
    askFrames = 0
end
function Link.onGameStart()
    Link.inGame = true
    pcall(function()
        if Link.invitePanel then
            Link.invitePanel:setVisible(false)
            Link.invitePanel:removeFromUIManager()
            Link.invitePanel = nil
        end
    end)
    Link.writeGameState("ingame")
end
Events.OnMainMenuEnter.Add(Link.onMainMenuEnter)
Events.OnGameStart.Add(Link.onGameStart)

-- While a request waits, the app's answer is looked for about once a second.
local lastAckCheck = 0
local function checkAcks()
    if not Link.isWaiting() then return end
    local t = now()
    if t == lastAckCheck then return end
    lastAckCheck = t
    if Link.readAcks() and Link.panel then
        pcall(function() Link.panel:updateView() end)
    end
end
-- Looked for while the Mods screen is drawn, which is where the answer is shown.
local listPrerender = ModSelector.ModListPanel.prerender
function ModSelector.ModListPanel:prerender(...)
    local result = listPrerender(self, ...)
    pcall(checkAcks)
    return result
end

--- Installed mods that are in no profile: not this one's, not another's, not the Studio's own.
function Link.orphans(model, link)
    local result = {}
    if not model or not model.sortedMods then return result end
    for _, modData in ipairs(model.sortedMods) do
        local id = modData.modId
        if id ~= SELF_ID and not string.find(id, STUDIO_PACKAGE) and not link.members[id] and not link.others[id] and not modData.isActive then
            table.insert(result, modData)
        end
    end
    return result
end

-- ------------------------------------------------------------------ the Mods screen

-- The list: only the profile's mods. Mods switched on are always shown, so nothing active is
-- ever out of sight; this mod itself is never shown, so it cannot be switched off by accident.
local filterMods = ModSelector.Model.filterMods
function ModSelector.Model:filterMods(...)
    filterMods(self, ...)
    if self.isServerSettingsMods or Link.otherManager() then return end
    -- Anything going wrong here leaves the game's own list as it is.
    local ok, err = pcall(function()
        local link = Link.read()
        if not link then return end
        local kept = {}
        for _, modData in ipairs(self.currentMods) do
            local id = modData.modId
            if id ~= SELF_ID and (Link.showAll or link.members[id] or modData.isActive) then
                table.insert(kept, modData)
            end
        end
        table.wipe(self.currentMods)
        for _, modData in ipairs(kept) do table.insert(self.currentMods, modData) end
    end)
    if not ok then print("[StudioLink] could not filter the list: " .. tostring(err)) end
end

-- Mod Health's warnings are read when the list is read, not on every frame.
local reloadMods = ModSelector.Model.reloadMods
function ModSelector.Model:reloadMods(...)
    Link.health = Link.readHealth()
    return reloadMods(self, ...)
end

--- Lays out the Studio's controls on the row of the game's own "enabled only" tick, leaving the
--- game's Favorites button its room: the new-mods button first, the profile name only if it fits.
local function layoutRow(panel)
    local row = panel.enabledModsTickbox
    local limit = (panel.favoriteButton and panel.favoriteButton.x or panel.width) - 10
    local x = row:getRight() + 16
    if panel.studioShowAll then
        panel.studioShowAll:setX(x)
        x = panel.studioShowAll:getRight() + 12
    end
    if panel.studioNewButton and panel.studioNewButton:isVisible() then
        local button = panel.studioNewButton
        button:setX(x)
        -- Not enough room before Favorites (small screens): just "+N", the rest on hover.
        if button:getRight() > limit and panel.studioNewShort then
            button:setTitle(panel.studioNewShort)
            button:setWidth(getTextManager():MeasureStringX(UIFont.Small, panel.studioNewShort) + 16)
        end
        x = button:getRight() + 12
    end
    if panel.studioProfileLabel then
        panel.studioProfileLabel:setX(x)
        panel.studioProfileLabel:setVisible(panel.studioProfileLabel:getRight() <= limit)
    end
end

local createChildren = ModSelector.ModListPanel.createChildren
function ModSelector.ModListPanel:createChildren()
    createChildren(self)
    if Link.otherManager() then return end
    local link = Link.read()
    if not link then return end
    Link.panel = self
    local ok, err = pcall(function()
        local row = self.enabledModsTickbox
        local h = row:getHeight()
        local label = text("showAll")
        local width = h + 10 + getTextManager():MeasureStringX(UIFont.Small, label)
        local tick = ISTickBox:new(row:getRight() + 16, row.y, width, h, "", self, function(panel, _, selected)
            Link.showAll = selected and true or false
            panel:updateView()
        end)
        tick:initialise()
        tick:instantiate()
        tick:addOption(label)
        tick.selected[1] = Link.showAll
        self.filterPanel:addChild(tick)
        self.studioShowAll = tick

        local button = ISButton:new(tick:getRight() + 12, row.y, 120, h, "", self, function(panel)
            if Link.isWaiting() then return end
            local current = Link.read()
            if not current then return end
            local ids = {}
            for _, modData in ipairs(Link.orphans(panel.model, current)) do table.insert(ids, modData.modId) end
            Link.requestAdd(ids)
            panel:updateView()
        end)
        button:initialise()
        button:instantiate()
        button.borderColor = { r = 0.3, g = 0.75, b = 1.0, a = 0.8 }
        button:setVisible(false)
        self.filterPanel:addChild(button)
        self.studioNewButton = button

        local name = ISLabel:new(button:getRight() + 12, row.y, h, text("profile", link.name),
            0.55, 0.85, 1.0, 1.0, UIFont.Small, true)
        name:initialise()
        name.tooltip = text("profileTip")
        self.filterPanel:addChild(name)
        self.studioProfileLabel = name
        layoutRow(self)
    end)
    if not ok then print("[StudioLink] could not add the profile row: " .. tostring(err)) end
end

-- The profile can change while the game sits on the main menu: read it again each time.
local updateView = ModSelector.ModListPanel.updateView
function ModSelector.ModListPanel:updateView(...)
    local result = updateView(self, ...)
    if self.studioProfileLabel then
        local ok, err = pcall(function()
            local link = Link.read()
            if not link then return end
            local label = self.studioProfileLabel
            if Link.appAlive() then
                label:setName(text("profile", link.name))
                label:setColor(0.55, 0.85, 1.0)
                label.tooltip = text("profileTip")
            else
                label:setName(text("profile", link.name) .. " " .. text("appClosed"))
                label:setColor(0.6, 0.6, 0.6)
                label.tooltip = text("appClosedTip")
            end
            local orphans = Link.orphans(self.model, link)
            local button = self.studioNewButton
            local function show(title, tooltip, r, g, b)
                button:setTitle(title)
                button:setWidth(getTextManager():MeasureStringX(UIFont.Small, title) + 16)
                button.tooltip = tooltip
                button.borderColor = { r = r, g = g, b = b, a = 0.8 }
                button:setVisible(true)
            end
            if Link.isWaiting() then
                show(text("adding"), nil, 0.6, 0.6, 0.6)
            elseif Link.addError and #orphans > 0 then
                show(text("addFailed"), text("addFailedTip", Link.addError), 0.95, 0.35, 0.35)
            elseif #orphans > 0 then
                local names = {}
                for i, modData in ipairs(orphans) do
                    if i > 12 then table.insert(names, "..."); break end
                    table.insert(names, "- " .. modData.name)
                end
                self.studioNewShort = "+" .. #orphans
                show(text("newMods", #orphans), text("newModsTip", #orphans, link.name, table.concat(names, "\n")), 0.3, 0.75, 1.0)
            else
                button:setVisible(false)
            end
            layoutRow(self)
        end)
        if not ok then print("[StudioLink] could not update the profile row: " .. tostring(err)) end
    end
    return result
end

local recalcSize = ModSelector.ModListPanel.recalcSize
function ModSelector.ModListPanel:recalcSize(...)
    local result = recalcSize(self, ...)
    if self.studioProfileLabel then pcall(layoutRow, self) end
    return result
end

-- A mark on each mod Mod Health has something to say about, left of the favourite star, with
-- what it says on hover (unless the game is already saying something there).
local SEVERITY_COLOR = {
    error = { r = 0.95, g = 0.3, b = 0.3 },
    warning = { r = 1.0, g = 0.7, b = 0.2 },
}
-- The game's own sizes for a row (locals of ModListBox.lua).
local BUTTON_HGT = getTextManager():getFontHeight(UIFont.Small) + 6
local UI_BORDER_SPACING = 10
local function drawHealthMark(list, y, nextY, item)
    local entry = item.item and Link.health[item.item.modId]
    if entry then
        local color = SEVERITY_COLOR[entry.severity] or SEVERITY_COLOR.warning
        local height = nextY - y
        local size = math.max(8, math.floor(height / 3))
        local x = list:getWidth() - UI_BORDER_SPACING - BUTTON_HGT - 1 - UI_BORDER_SPACING - size
        list:drawRect(x, y + (height - size) / 2, size, size, 0.9, color.r, color.g, color.b)
        if not item.item.isIncompatible then
            item.tooltip = table.concat(entry.lines, "\n")
            item.studioTip = true
        end
    elseif item.studioTip then
        -- The warning is gone since the list was last read.
        item.studioTip = nil
        if not (item.item and item.item.isIncompatible) then item.tooltip = nil end
    end
end
local doDrawItem = ModSelector.ModListBox.doDrawItem
function ModSelector.ModListBox:doDrawItem(y, item, alt)
    local nextY = doDrawItem(self, y, item, alt)
    pcall(drawHealthMark, self, y, nextY, item)
    return nextY
end

-- ------------------------------------------------------------------ saves

--- Mods the save uses that are not in the profile.
function Link.outsideProfile(activeMods, link)
    local result = {}
    if not activeMods then return result end
    local mods = activeMods:getMods()
    for i = 0, mods:size() - 1 do
        local id = mods:get(i)
        if not link.members[id] then
            local info = getModInfoByID(id)
            table.insert(result, info and info:getName() or id)
        end
    end
    return result
end

--- How many mods the profile has besides this one.
local function profileSize(link)
    local n = 0
    for _, id in ipairs(link.order) do
        if id ~= SELF_ID then n = n + 1 end
    end
    return n
end

--- A save against the profile: "empty", "outside" (with how many), "ok", or nil when there is
--- nothing to say (no profile, or an old save without a mod list, which uses the default one).
function Link.saveStatus(activeMods, link)
    if not link or not activeMods then return nil end
    if activeMods:getMods():isEmpty() then
        return profileSize(link) > 0 and "empty" or nil
    end
    local outside = Link.outsideProfile(activeMods, link)
    if #outside > 0 then return "outside", #outside end
    return "ok"
end

-- The badge on each save of the Load screen, at the right of the row.
local function drawSaveBadge(list, y, item)
    local link = Link.listLink
    if not link then return end
    local status, count = Link.saveStatus(item.item.activeMods, link)
    if not status then return end
    local label, r, g, b
    if status == "empty" then
        label, r, g, b = text("badgeEmpty"), 0.95, 0.35, 0.35
    elseif status == "outside" then
        label, r, g, b = text("badgeOutside", count), 1.0, 0.7, 0.2
    else
        label, r, g, b = text("badgeOk"), 0.45, 0.85, 0.5
    end
    local right = list:getWidth() - 12
    if not item.item.playerAlive and list.parent and list.parent.deadTexture then
        right = list:getWidth() - 24 - list.parent.deadTexture:getWidthOrig() - 8
    end
    list:drawTextRight(label, right, y + 6, r, g, b, 0.95, UIFont.Small)
end

-- ------------------------------------------------------------------ each profile's saves
-- Every profile has its own saves. The app writes, for each save, the profile it belongs to (the
-- one active when it was last played with the app open, or one whose mods are exactly the
-- save's), and the active profile's id. The Load screen lists the active profile's saves, with a
-- button for all of them (those of other profiles and those of none), and Continue loads the
-- newest save of the active profile. Without those files (no app, an older app) all are shown.

--- The active profile's id and the app's saves, or nil when saves are not told apart.
function Link.profileSaves()
    local link = Link.read()
    if not link or not link.profileid or link.profileid == "" then return nil end
    local saves = Link.readSaves()
    if not saves then return nil end
    return link.profileid, saves, link
end

--- Points Continue at the newest save of the active profile (nothing, when it has none).
function Link.latestOfProfile()
    local profile, saves = Link.profileSaves()
    if not profile then return end
    Link.latestFor = profile
    local mode, world = MainScreen.latestSaveGameMode, MainScreen.latestSaveWorld
    if mode and world then
        local save = saves[tostring(mode) .. "/" .. tostring(world)]
        if save and save.profile == profile then return end
    end
    local best, bestAt = nil, -1
    for _, k in ipairs(getFullSaveDirectoryTable()) do
        local id = string.gsub(tostring(k), "\\", "/")
        local save = saves[id]
        if save and save.profile == profile and save.modified > bestAt then
            best, bestAt = id, save.modified
        end
    end
    if best then
        MainScreen.latestSaveGameMode, MainScreen.latestSaveWorld = string.match(best, "^([^/]+)/(.+)$")
    else
        MainScreen.latestSaveGameMode, MainScreen.latestSaveWorld = nil, nil
    end
    Link.latestExists = nil
    Link.card = nil
end

local getLatestSave = MainScreen.getLatestSave
function MainScreen:getLatestSave(...)
    local result = getLatestSave(self, ...)
    local ok, err = pcall(Link.latestOfProfile)
    if not ok then print("[StudioLink] the profile's latest save: " .. tostring(err)) end
    return result
end

--- The Load screen's list: the active profile's saves, or all of them.
function Link.filterSaveList(screen)
    local profile, saves, link = Link.profileSaves()
    screen.linkHidden = 0
    if not profile then
        if screen.linkAllButton then screen.linkAllButton:setVisible(false) end
        return
    end
    local keep = {}
    for _, row in ipairs(screen.listbox.items) do
        local save = saves[string.gsub(tostring(row.text), "\\", "/")]
        if Link.showAllSaves or (save and save.profile == profile) then
            table.insert(keep, row)
        else
            screen.listCache[row.text] = nil
            screen.linkHidden = screen.linkHidden + 1
        end
    end
    screen.listbox:clear()
    for _, row in ipairs(keep) do screen.listbox:addItem(row.text, row.item) end
    if screen.linkAllButton then
        local shown = Link.showAllSaves or screen.linkHidden > 0
        screen.linkAllButton:setVisible(shown)
        screen.linkAllButton:setTitle(Link.showAllSaves and text("savesOnlyProfile", link.name) or text("savesShowAll", screen.linkHidden))
    end
end

function Link.toggleAllSaves(screen)
    Link.showAllSaves = not Link.showAllSaves
    screen:setSaveGamesList()
end

local createLoadScreen = LoadGameScreen.create
function LoadGameScreen:create(...)
    local result = createLoadScreen(self, ...)
    local draw = self.listbox.doDrawItem
    self.listbox.doDrawItem = function(list, y, item, alt)
        local nextY = draw(list, y, item, alt)
        pcall(drawSaveBadge, list, y, item)
        return nextY
    end
    -- "Show all" beside the search box, which gives it the room.
    pcall(function()
        local w = 220
        self.searchEntry:setWidth(self.searchEntry:getWidth() - w - 10)
        self.linkAllButton = ISButton:new(self.searchEntry:getRight() + 10, self.searchEntry:getY(), w, self.searchEntry:getHeight(), "", self, Link.toggleAllSaves)
        self.linkAllButton:initialise()
        self.linkAllButton:instantiate()
        self.linkAllButton:setAnchorLeft(false)
        self.linkAllButton:setAnchorRight(true)
        self.linkAllButton:setVisible(false)
        self:addChild(self.linkAllButton)
    end)
    return result
end

-- The profile is read once when the list of saves is filled, not on every frame.
local setSaveGamesList = LoadGameScreen.setSaveGamesList
function LoadGameScreen:setSaveGamesList(...)
    Link.listLink = Link.read()
    local result = setSaveGamesList(self, ...)
    local ok, err = pcall(Link.filterSaveList, self)
    if not ok then print("[StudioLink] the profile's saves: " .. tostring(err)) end
    return result
end

-- ------------------------------------------------------------------ save backups

-- The app's view of each save (see the Studio's studio_link::SAVES_FILE): whether it is backed
-- up as it is, its size, and the mods of its last session.
local SAVES_FILE = "PZModStudio_LinkSaves.txt"
-- "Load it anyway" survives the game's Lua reset between the two save checks of one load.
local PASS_FILE = "PZModStudio_LinkPass.txt"
local PASS_SECONDS = 120

local function splitTabs(line)
    local out = {}
    local start = 1
    while true do
        local i = string.find(line, "\t", start, true)
        if not i then
            table.insert(out, string.sub(line, start))
            return out
        end
        table.insert(out, string.sub(line, start, i - 1))
        start = i + 1
    end
end

local function plainId(id)
    return (string.gsub(tostring(id), "^\\", ""))
end

local function humanBytes(n)
    n = tonumber(n) or 0
    if n >= 1024 * 1024 * 1024 then return string.format("%.1f GB", n / (1024 * 1024 * 1024)) end
    if n >= 1024 * 1024 then return string.format("%.0f MB", n / (1024 * 1024)) end
    return string.format("%.0f KB", n / 1024)
end

--- The app's saves file, by `<mode>/<world>`, or nil when the app never wrote one.
function Link.readSaves()
    local file = getFileReader(SAVES_FILE, false)
    if not file then return nil end
    local result = {}
    local line = file:readLine()
    while line ~= nil do
        local fields = splitTabs(line)
        if fields[1] == "save" and fields[2] and fields[2] ~= "" then
            local save = { backed = false, bytes = 0, session = 0, updated = 0, modified = 0, mods = {} }
            for i = 3, #fields do
                local key, value = string.match(fields[i], "^(%w+)=(.*)$")
                if key == "backed" then save.backed = value == "1"
                elseif key == "bytes" or key == "session" or key == "updated" or key == "modified" then save[key] = tonumber(value) or 0
                elseif key == "profile" then save.profile = value ~= "" and value or nil
                elseif key == "mods" then
                    for id in string.gmatch(value, "[^;]+") do table.insert(save.mods, plainId(id)) end
                elseif key == "updatedmods" then
                    save.updatedmods = {}
                    for id in string.gmatch(value, "[^;]+") do table.insert(save.updatedmods, plainId(id)) end
                end
            end
            result[fields[2]] = save
        end
        line = file:readLine()
    end
    file:close()
    return result
end

function Link.saveId(world)
    return tostring(getWorld():getGameMode()) .. "/" .. tostring(world)
end

--- Why the save should be backed up before it loads, or nil when there is no need: it is backed
--- up as it is, or it loads with the mods of its last session and none of them changed since.
function Link.backupWhy(save, activeMods)
    if not save or save.backed then return nil end
    if save.session == 0 then return text("whyNever") end
    local now = {}
    if activeMods then
        local mods = activeMods:getMods()
        for i = 0, mods:size() - 1 do table.insert(now, plainId(mods:get(i))) end
    end
    local same = #now == #save.mods
    for i = 1, #now do
        if now[i] ~= save.mods[i] then same = false end
    end
    if not same then return text("whyChanged") end
    if save.updated > 0 then return text("whyUpdated", save.updated) end
    return nil
end

function Link.markPassed(world)
    local writer = getFileWriter(PASS_FILE, true, false)
    if not writer then return end
    writer:write(tostring(world) .. "\t" .. now() .. "\n")
    writer:close()
end

function Link.isPassed(world)
    local file = getFileReader(PASS_FILE, false)
    if not file then return false end
    local line = file:readLine()
    file:close()
    if not line then return false end
    local fields = splitTabs(line)
    return fields[1] == tostring(world) and math.abs(now() - (tonumber(fields[2]) or 0)) <= PASS_SECONDS
end

--- The app's answer to one request: "ok" / "error", and its text, or nil while there is none.
function Link.findAck(id)
    local file = getFileReader(ACKS_FILE, false)
    if not file then return nil end
    local status, msg
    local line = file:readLine()
    while line ~= nil do
        local fields = splitTabs(line)
        if fields[1] == id then status, msg = fields[2], fields[3] or "" end
        line = file:readLine()
    end
    file:close()
    return status, msg
end

local function modal(body, yesNo, onAnswer)
    local w = 520
    local dialog = ISModalRichText:new(getCore():getScreenWidth() / 2 - w / 2, getCore():getScreenHeight() / 2 - 90, w, 180,
        body, yesNo, nil, function(_, button) onAnswer(button and button.internal) end)
    dialog:initialise()
    dialog:addToUIManager()
    dialog:setAlwaysOnTop(true)
    return dialog
end

--- Asks "go on without a backup?"; Yes runs `onYes`.
local function askWithoutBackup(body, onYes)
    modal(" <RGB:1,0.75,0.3> " .. body .. " <RGB:1,1,1> <LINE> <LINE> " .. text("goOnAnyway"), true, function(answer)
        if answer == "YES" then onYes() end
    end)
end

-- The Studio's own panel, for waiting on the app and for choices: a plain panel of this mod's, so
-- UI mods that restyle the game's dialogs (and draw OK as a green tick) cannot change what it
-- says. While it waits, the app's answer is looked for as it is drawn.
local WaitPanel = ISPanelJoypad:derive("StudioLinkWaitPanel")
Link.WaitPanel = WaitPanel

local function openPanel(title, body, height, choices, waiting)
    -- Fits small screens (1366x768 and less): never wider than the screen, never taller.
    local w = math.min(560, getCore():getScreenWidth() - 40)
    local h = math.min(height, getCore():getScreenHeight() - 80)
    local panel = WaitPanel:new(getCore():getScreenWidth() / 2 - w / 2, getCore():getScreenHeight() / 2 - h / 2, w, h)
    panel.title = title
    panel.body = body
    panel.choices = choices
    panel.waiting = waiting
    -- Known before the buttons are made, so they show the controller's buttons.
    panel.joypad = JoypadState.getMainMenuJoypad()
    panel:initialise()
    panel:instantiate()
    panel:addToUIManager()
    panel:setAlwaysOnTop(true)
    -- While it is up nothing else changes the screens under it (see Link.dialogUp).
    Link.dialog = panel
    -- A controller drives it, as it does the game's own dialogs.
    if panel.joypad then
        panel.joypad.focus = panel
        updateJoypadFocus(panel.joypad)
    end
    return panel
end

--- Whether one of these panels is on screen (a question waits for the player).
function Link.dialogUp()
    return Link.dialog ~= nil and Link.dialog:isVisible()
end

--- A panel that waits for the app, with Cancel.
function WaitPanel.show(title, body, onCancel)
    return openPanel(title, body, 170, { { label = text("cancel"), onClick = onCancel } }, true)
end

--- A panel with a choice: `choices` = { { label, onClick, accept }, ... }, left to right.
function WaitPanel.choose(title, body, height, choices)
    return openPanel(title, body, height, choices, false)
end

function WaitPanel:new(x, y, w, h)
    local o = ISPanelJoypad.new(self, x, y, w, h)
    o.backgroundColor = { r = 0.08, g = 0.08, b = 0.08, a = 0.97 }
    o.borderColor = { r = 0.35, g = 0.75, b = 1.0, a = 0.8 }
    o.moveWithMouse = true
    o.started = now()
    return o
end

function WaitPanel:createChildren()
    local rich = ISRichTextPanel:new(12, 12, self.width - 24, self.height - 60)
    rich:initialise()
    rich.background = false
    rich.autosetheight = false
    rich:setText(" <H2> " .. self.title .. " <LINE> <TEXT> " .. self.body)
    rich:paginate()
    self:addChild(rich)
    self.rich = rich
    local count = #self.choices
    local gap = 10
    local bw = math.min(150, math.floor((self.width - 24 - (count - 1) * gap) / count))
    local x = (self.width - (count * bw + (count - 1) * gap)) / 2
    self.buttons = {}
    for i, choice in ipairs(self.choices) do
        local button = ISButton:new(x, self.height - 36, bw, 24, choice.label, self, function(panel)
            panel:close()
            if choice.onClick then choice.onClick() end
        end)
        button:initialise()
        button:instantiate()
        if choice.accept then
            button.borderColor = { r = 0.35, g = 0.85, b = 0.5, a = 0.9 }
        end
        -- With a controller: A the main choice, B the first (Back / Cancel), X the one between.
        choice.pad = choice.accept and "A" or (i == 1 and "B" or "X")
        if self.joypad and Joypad and Joypad.Texture then
            button:setJoypadButton(Joypad.Texture[choice.pad .. "Button"])
        end
        self.buttons[choice.pad] = button
        self:addChild(button)
        x = x + bw + gap
    end
end

function WaitPanel:onJoypadDown(button)
    local pad = (button == Joypad.AButton and "A") or (button == Joypad.BButton and "B") or (button == Joypad.XButton and "X") or nil
    local target = pad and self.buttons and self.buttons[pad]
    -- Only one choice (Cancel): B and A both take it.
    if not target and self.buttons and #self.choices == 1 and (pad == "A" or pad == "B") then
        target = self.buttons[self.choices[1].pad]
    end
    if target then
        target:forceClick()
    end
end

function WaitPanel:close()
    self:setVisible(false)
    self:removeFromUIManager()
    if Link.dialog == self then Link.dialog = nil end
    if self.joypad and self.joypad.focus == self then
        self.joypad.focus = MainScreen.instance
        updateJoypadFocus(self.joypad)
    end
end

function WaitPanel:prerender()
    ISPanelJoypad.prerender(self)
    if not self.waiting then return end
    local seconds = now() - (self.started or now())
    self:drawTextRight(tostring(seconds) .. " s", self.width - 12, self.height - 32, 0.6, 0.6, 0.6, 1, UIFont.Small)
    pcall(Link.pollBackup)
end

--- Sends a request to the app and waits for its answer with a panel: `onOk(text)` when it is
--- done, `onFail(text)` when it failed. Cancel stops waiting (the app still does it).
local function askApp(verb, arg, title, body, onOk, onFail)
    local writer = getFileWriter(REQUESTS_FILE, true, true)
    if not writer then
        onFail(REQUESTS_FILE)
        return
    end
    Link.seq = (Link.seq or 0) + 1
    local id = tostring(now()) .. "-" .. tostring(Link.seq)
    writer:write(id .. "\t" .. verb .. "\t" .. arg .. "\n")
    writer:close()
    local wait = { id = id, at = now(), onOk = onOk, onFail = onFail }
    Link.backupWait = wait
    wait.dialog = WaitPanel.show(title, body, function()
        if Link.backupWait == wait then Link.backupWait = nil end
    end)
end

--- Has PZ Mod Studio back the save up, shows a dialog while it does, and runs `onDone` once the
--- backup is made and checked. Without the app, or when it fails, the player decides.
function Link.withBackup(world, reason, why, onDone)
    local saveId = Link.saveId(world)
    local saves = Link.readSaves() or {}
    local size = humanBytes(saves[saveId] and saves[saveId].bytes or 0)
    local what = reason == "before-fix" and text("backupBeforeFix", world, size) or text("backupBeforeLoad", world, size, why or "")
    if not Link.appAlive() then
        local closed = reason == "before-fix" and text("noAppFix", world) or text("noAppLoad", world, why or "")
        askWithoutBackup(closed, onDone)
        return
    end
    askApp("backup", saveId .. "\t" .. reason, text("backupTitle"), what,
        function() onDone() end,
        function(msg) askWithoutBackup(text("backupFailed", msg), onDone) end)
end

function Link.pollBackup()
    local wait = Link.backupWait
    if not wait then return end
    local t = now()
    if wait.polled == t then return end
    wait.polled = t
    local status, msg = Link.findAck(wait.id)
    if not status then return end
    Link.backupWait = nil
    wait.dialog:close()
    if status == "ok" then
        wait.onOk(msg or "")
    else
        wait.onFail(msg ~= "" and msg or status)
    end
end

-- The arguments the game's own load started with, to start it again once the backup is made.
local continueLatestSaveAux = MainScreen.continueLatestSaveAux
function MainScreen.continueLatestSaveAux(...)
    Link.continueArgs = { ... }
    return continueLatestSaveAux(...)
end

--- What changed for the save since its last session, or nil when nothing did (or the Studio
--- never saw a session of it): mods added and taken out of its list, mods updated on the
--- Workshop since, and whether the order of the others changed.
function Link.sessionChanges(save, activeMods)
    if not save or save.session == 0 then return nil end
    local now = {}
    if activeMods then
        local mods = activeMods:getMods()
        for i = 0, mods:size() - 1 do table.insert(now, plainId(mods:get(i))) end
    end
    local inNow, inThen = {}, {}
    for _, id in ipairs(now) do inNow[id] = true end
    for _, id in ipairs(save.mods) do inThen[id] = true end
    local changes = { added = {}, removed = {}, updated = {}, reordered = false }
    for _, id in ipairs(now) do if not inThen[id] then table.insert(changes.added, id) end end
    for _, id in ipairs(save.mods) do if not inNow[id] then table.insert(changes.removed, id) end end
    for _, id in ipairs(save.updatedmods or {}) do if inNow[id] then table.insert(changes.updated, id) end end
    local keptThen, keptNow = {}, {}
    for _, id in ipairs(save.mods) do if inNow[id] then table.insert(keptThen, id) end end
    for _, id in ipairs(now) do if inThen[id] then table.insert(keptNow, id) end end
    for i = 1, #keptNow do
        if keptNow[i] ~= keptThen[i] then changes.reordered = true end
    end
    if #changes.added == 0 and #changes.removed == 0 and #changes.updated == 0 and not changes.reordered then return nil end
    return changes
end

local function modName(id)
    local info = getModInfoByID(id)
    return info and info:getName() or id
end

--- Whether a mod adds map cells: taking one out of a save loses what it built there.
local function isMapMod(id)
    local ok, folders = pcall(getMapFoldersForMod, id)
    return ok and folders ~= nil and folders:size() > 0
end

local function ago(seconds)
    if seconds < 3600 then return text("agoMinutes", math.max(1, math.floor(seconds / 60))) end
    if seconds < 86400 * 2 then return text("agoHours", math.floor(seconds / 3600)) end
    return text("agoDays", math.floor(seconds / 86400))
end

--- Shows what changed since the save's last session, and lets the player load it (backed up
--- first), play on a copy, or go back.
function Link.showChanges(world, save, changes, onLoad, onCopy)
    local lines = 0
    local body = text("changesWhen", world, ago(math.max(0, now() - save.session))) .. " <LINE> <LINE> "
    local function list(head, ids, r, g, b, mark)
        if #ids == 0 then return end
        body = body .. " <RGB:" .. r .. "," .. g .. "," .. b .. "> " .. text(head, #ids) .. " <RGB:1,1,1> <LINE> <INDENT:16> "
        for i, id in ipairs(ids) do
            if i > 8 then
                body = body .. text("andMore", #ids - 8) .. " <LINE> "
                lines = lines + 1
                break
            end
            body = body .. modName(id) .. (mark and mark(id) or "") .. " <LINE> "
            lines = lines + 1
        end
        body = body .. " <INDENT:0> <LINE> "
        lines = lines + 2
    end
    local mapsOut = 0
    list("changesRemoved", changes.removed, "1", "0.45", "0.45", function(id)
        if isMapMod(id) then
            mapsOut = mapsOut + 1
            return "  " .. text("mapTag")
        end
        return ""
    end)
    list("changesAdded", changes.added, "0.5", "0.9", "0.55")
    list("changesUpdated", changes.updated, "0.55", "0.8", "1")
    if changes.reordered then
        body = body .. text("changesReordered") .. " <LINE> <LINE> "
        lines = lines + 2
    end
    if mapsOut > 0 then
        body = body .. " <RGB:1,0.6,0.3> " .. text("mapsOutWarning") .. " <RGB:1,1,1> <LINE> "
        lines = lines + 2
    end
    local height = 120 + lines * (getTextManager():getFontHeight(UIFont.Small) + 2)
    WaitPanel.choose(text("changesTitle"), body, height, {
        { label = text("back") },
        { label = text("playCopy"), onClick = onCopy },
        { label = text("loadIt"), onClick = onLoad, accept = true },
    })
end

--- Has the app copy the save, then loads the copy; the save itself stays as it is.
function Link.playOnCopy(world)
    if not Link.appAlive() then
        modal(" <RGB:1,0.75,0.3> " .. text("copyNeedsApp") .. " <RGB:1,1,1> ", false, function() end)
        return
    end
    local saveId = Link.saveId(world)
    local saves = Link.readSaves() or {}
    local size = humanBytes(saves[saveId] and saves[saveId].bytes or 0)
    local mode = tostring(getWorld():getGameMode())
    askApp("copy", saveId, text("copyTitle"), text("copyBody", world, size),
        function(copy)
            if copy == "" then return end
            Link.markPassed(copy)
            MainScreen.continueLatestSave(mode, copy)
        end,
        function(msg)
            modal(" <RGB:1,0.75,0.3> " .. text("copyFailed", msg) .. " <RGB:1,1,1> ", false, function() end)
        end)
end

--- Before a save loads: what changed since its last session is shown first (load, play on a
--- copy, or go back), and the save is backed up first when it is not backed up as it is.
--- Returns true when it can load now.
function Link.loadGate(world, saveInfo)
    if Link.isPassed(world) then return true end
    local saves = Link.readSaves()
    if not saves then return true end
    local save = saves[Link.saveId(world)]
    local activeMods = saveInfo and saveInfo.activeMods
    local changes = Link.sessionChanges(save, activeMods)
    local why = Link.backupWhy(save, activeMods)
    if not changes and not why then return true end
    local args = Link.continueArgs or {}
    local function load()
        Link.markPassed(world)
        MainScreen.continueLatestSaveAux(args[1], args[2])
    end
    local function backupThenLoad()
        if why then Link.withBackup(world, "before-load", why, load) else load() end
    end
    if changes then
        Link.showChanges(world, save, changes, backupThenLoad, function() Link.playOnCopy(world) end)
    else
        backupThenLoad()
    end
    return false
end

-- Joining from PZ Mod Studio: the game's connect window fills the account only from the game's
-- own server list, so it waited on an empty username. The app writes the server and the account
-- it keeps for it (never a password); filled in here when the window is for that server.
local JOIN_FILE = "PZModStudio_LinkJoin.txt"

-- The server joined from the app goes into the game's own Favorites (and the account under it),
-- the way the game's "Add Server" and "Add New Account" do, so it is still there on the
-- Multiplayer screen after a disconnect. Nothing is added twice; no account password is kept.
local function keepAsFavourite(ip, port, name, username, serverPassword)
    if not (Server and Server.new and getServerList and addServerToAccountList) then return end
    local wantedPort = tonumber(port)
    local function find()
        for _, s in ipairs(getServerList()) do
            if s:getIp() == ip and tonumber(s:getPort()) == wantedPort then return s end
        end
        return nil
    end
    local server = find()
    if not server then
        local s = Server.new()
        s:setName((name and name ~= "") and name or (ip .. ":" .. tostring(port)))
        s:setIp(ip)
        s:setPort(wantedPort)
        s:setServerPassword(serverPassword or "")
        addServerToAccountList(s)
        -- The game does not hand the new row's id back: read the list again for it.
        server = find()
    end
    if not server or not username or username == "" or not (Account and Account.new and addAccountToAccountList) then return end
    local accounts = server:getAccounts()
    for i = 0, accounts:size() - 1 do
        if accounts:get(i):getUserName() == username then return end
    end
    local account = Account.new()
    account:setUserName(username)
    account:setPwd("")
    account:setSavePwd(false)
    account:setAuthType(1)
    account:setLastLogonNow()
    addAccountToAccountList(server, account)
end

local setServer = ServerConnectPopup.setServer
function ServerConnectPopup:setServer(ip, port, passwordStr)
    setServer(self, ip, port, passwordStr)
    pcall(function()
        local file = getFileReader(JOIN_FILE, false)
        if not file then return end
        local address, name, username
        local line = file:readLine()
        while line ~= nil do
            local key, value = string.match(line, "^(%a+)=(.*)$")
            if key == "address" then address = value elseif key == "name" then name = value elseif key == "username" then username = value end
            line = file:readLine()
        end
        file:close()
        if not address then return end
        local wantedPort = string.match(address, ":(%d+)$") or "16261"
        if wantedPort ~= tostring(port) then return end
        local host = string.match(address, "^(.-):%d+$") or address
        if host == ip then
            pcall(keepAsFavourite, ip, port, name, username, passwordStr)
        end
        if not username or username == "" then return end
        if (self.usernameEntry:getText() or "") == "" then self.usernameEntry:setText(username) end
        if self.checkFields then self:checkFields() end
    end)
end

-- The save check: the game's own one first, then the profile, with the game's own dialog, then
-- a backup when the save loads with changed mods.
local checkSaveFile = MainScreen.checkSaveFile
function MainScreen.checkSaveFile(...)
    if not checkSaveFile(...) then return false end
    local world = getWorld():getWorld()
    local saveInfo = getSaveInfo(world)
    local link = Link.read()
    if not link then return Link.loadGate(world, saveInfo) end
    local status = Link.saveStatus(saveInfo and saveInfo.activeMods, link)

    -- A save whose own mod list is empty loads with no mods, and the game then drops every item
    -- a mod made. Asked first, the game's own way: Yes gives it the profile's mods and loads it,
    -- No goes back.
    if status == "empty" and Link.fillEmpty == world then
        -- Yes was answered: the game's own Yes comes back through here after copying the save's
        -- (empty) list again, so the profile's mods are put back before it loads.
        Link.fillEmpty = nil
        ActiveMods.getById("currentGame"):copyFrom(ActiveMods.getById("default"))
        return Link.loadGate(world, saveInfo)
    end
    Link.fillEmpty = nil
    if status == "empty" then
        Link.askedEmpty = world
        local body = " <H1> " .. getText("UI_mainscreen_ErrorLoadingSavefile") .. " <H2> <LINE> <LINE> "
        body = body .. getText("UI_mainscreen_SavefileName", world) .. " <LINE> <LINE> "
        body = body .. " <TEXT> <RED> " .. text("empty", link.name, profileSize(link)) .. " <RGB:1,1,1> <LINE> <LINE> "
        body = body .. text("emptyWhy") .. " <LINE> "
        MainScreen.displayCheckSavefileModal(body, false)
        return false
    end

    if status ~= "outside" then return Link.loadGate(world, saveInfo) end
    local outside = Link.outsideProfile(saveInfo.activeMods, link)
    local body = " <H1> " .. getText("UI_mainscreen_ErrorLoadingSavefile") .. " <H2> <LINE> <LINE> "
    body = body .. getText("UI_mainscreen_SavefileName", world) .. " <LINE> <LINE> "
    body = body .. " <TEXT> " .. text("blocked", link.name) .. " <LINE> <INDENT:20> <RED> "
    -- A whole profile's worth of names would not fit on the screen.
    for i, name in ipairs(outside) do
        if i > 15 then
            body = body .. text("andMore", #outside - 15) .. " <LINE> "
            break
        end
        body = body .. name .. " <LINE> "
    end
    body = body .. " <INDENT:0> <RGB:1,1,1> <LINE> " .. text("howTo") .. " <LINE> "
    MainScreen.displayCheckSavefileModal(body, true)
    return false
end

-- Yes in the game's save dialogs changes the save before it loads (its own Yes writes the mod
-- list into the save: missing mods taken out, or, on the empty-list question, the profile's mods
-- put in). So the save is backed up first, and then that Yes goes on.
local onCheckSavefileModalClick = MainScreen.onCheckSavefileModalClick
function MainScreen.onCheckSavefileModalClick(model, button, ...)
    local world = getWorld():getWorld()
    local fillEmpty = Link.askedEmpty == world
    Link.askedEmpty = nil
    local yes = button and button.internal == "YES"
    local args = { ... }
    local function goOn()
        if yes and fillEmpty then
            ActiveMods.getById("currentGame"):copyFrom(ActiveMods.getById("default"))
            Link.fillEmpty = world
        end
        local result = onCheckSavefileModalClick(model, button, unpack(args))
        -- No: Continue had copied the refused save's list (empty, or outside the profile) into
        -- "currentGame", the list the next new game is made with, and the game's own No only
        -- puts the default back when a Lua reset is due. A new game started from the Load
        -- screen after that (delete the last save and the game goes straight to New Game, with
        -- no Solo click to copy the default) was created with no mods. See Link.staging.
        if not yes then pcall(Link.stagingFromDefault, "no on the save dialog") end
        return result
    end
    if yes and not Link.isPassed(world) then
        local saves = Link.readSaves()
        local save = saves and saves[Link.saveId(world)]
        if save and not save.backed then
            Link.withBackup(world, "before-fix", nil, function()
                Link.markPassed(world)
                goOn()
            end)
            return
        end
    end
    return goOn()
end

-- ------------------------------------------------------------------ the new game's mod list

-- "currentGame" is the list the game writes into a new save's mods.txt when the world is made
-- (GameLoadingState.exit). The main menu's Solo click copies "default" into it; Continue and
-- the Load screen's "Choose mods" copy a save's own list into it instead, and nothing puts the
-- default back when that save is then not loaded. The one way into New Game that skips the
-- Solo click (the Load screen with its last save deleted) then makes a world with that stale
-- list: 147 mods running, a save that says "no mods", and every modded item dropped on its
-- next load. These put the default back where the game forgets to.

--- Copies the default list into the new game's list, saying why.
function Link.stagingFromDefault(why)
    local default = ActiveMods.getById("default")
    local staging = ActiveMods.getById("currentGame")
    staging:copyFrom(default)
    print("[StudioLink] the next new game gets the active list again (" .. tostring(why) .. "): " .. tostring(default:getMods():size()) .. " mods")
end

-- Deleting the last save from the Load screen opens New Game directly.
local onDeleteModalClick = LoadGameScreen.onDeleteModalClick
function LoadGameScreen:onDeleteModalClick(...)
    local result = onDeleteModalClick(self, ...)
    pcall(function()
        local ms = MainScreen.instance
        if ms and ms.soloScreen and ms.soloScreen:isVisible() then Link.stagingFromDefault("New Game after deleting the last save") end
    end)
    return result
end

-- The net under every other path: a world about to be made or loaded with an empty list while
-- the active list has mods is never meant (the Link refuses to load such a save, and its Yes
-- gives the save the profile's mods), so the active list goes in before the game writes
-- mods.txt. A loaded save keeps its own list: this only acts on an empty one.
Events.OnInitWorld.Add(function()
    pcall(function()
        if isClient() then return end
        local staging = ActiveMods.getById("currentGame")
        local default = ActiveMods.getById("default")
        if staging:getMods():isEmpty() and not default:getMods():isEmpty() then
            Link.stagingFromDefault("the world started with an empty list")
        end
    end)
end)

-- For PZModStudio_ModsScreen.lua (loaded after this file): the helpers it shares.
Link.text = text
Link.plainId = plainId
Link.isMapMod = isMapMod
Link.now = now
Link.SELF_ID = SELF_ID
Link.DOWNLOAD_URL = DOWNLOAD_URL
Link.STUDIO_PACKAGE = STUDIO_PACKAGE
