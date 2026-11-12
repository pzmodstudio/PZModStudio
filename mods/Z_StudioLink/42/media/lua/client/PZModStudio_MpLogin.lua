-- PZ Mod Studio Link: the account of a multiplayer test on this PC, filled in for you.
--
-- The Studio's multiplayer test starts the game with +connect 127.0.0.1:<port>. The game's
-- connect window is meant to fill the account from the saved server list, but it compares the
-- saved port (a number) with the port it was given (text), so with +connect it never does. The
-- Studio leaves the account in Zomboid/Lua/PZModStudio_MpLogin.txt (this game's own folder);
-- once the window shows that address it is filled in and, when asked, connected. The file is
-- emptied as soon as it is read, so the password does not stay in it.

local FILE = "PZModStudio_MpLogin.txt"
local done = false

local function readLogin()
    local reader = getFileReader(FILE, false)
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

local function forget()
    local writer = getFileWriter(FILE, true, false)
    if writer then
        writer:write("")
        writer:close()
    end
end

local function fill()
    if done then return end
    local popup = ServerConnectPopup and ServerConnectPopup.instance
    if not popup or not popup:isVisible() then return end
    done = true
    local login = readLogin()
    if not login or not login.ip or login.ip == "" then return end
    if tostring(popup.ip) ~= login.ip or tostring(popup.port) ~= login.port then return end
    forget()
    if popup.usernameEntry:getText() ~= "" then return end
    popup.usernameEntry:setText(login.user or "")
    popup.passwordEntry:setText(login.password or "")
    if login.connect == "1" and login.user and login.user ~= "" then
        popup:onOptionMouseDown({ internal = "CONNECT" }, 0, 0)
    end
end

if Events.OnFETick then
    Events.OnFETick.Add(fill)
end
