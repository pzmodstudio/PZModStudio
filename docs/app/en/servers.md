# Servers

The Servers screen has three tabs at the top: **Public servers** and **My servers**, to find a multiplayer server and join it, and **Host a server**, to run your own dedicated server from this PC. Under **Host a server**, a bar says whether your server is on, with **See how it runs** (opens Diagnostics > Server: memory, CPU, whether it has started and what its log says), **Share Invite** and the button that reads the server files again.

## Join a server: finding one

Under **Public servers**, press **Find servers**. The Studio asks Steam for every Project Zomboid server, the way the game's own browser does, through the Steam client running on this PC (no key needed; Steam shows you as in Project Zomboid while it asks). Servers appear as they answer; all of them take about a minute and a half. **Stop** ends the search, **Search again** starts over.

Narrow the list with the box (**Name or map**) and the filters: **Only my build**, **With players**, **Not full**, **No password**, **Modded and vanilla** / **Modded** / **Vanilla**, and sort by **Most players**, **Lowest ping** or **Name**. Right-click a server for a menu: **See what it says**, **Join**, **Star it**, **Copy the address**, making a profile with its mods and, for a server in **My servers**, **Forget this server**.

**My servers** holds the servers you star (**Starred**) and the ones you joined from here (**Joined from here**). To add a private or a friend's server, type its address under **Add a server by its address** (`ip:port`; port 16261 if left out), an optional name and the password, and press **Add**. The password is kept in Windows' credential store, never in a file.

## Join a server: joining

1. Click a server. The panel on the right shows its build, whether it is open or whitelist only, its last wipe and its mods. Each mod is coloured: **Installed and on**, **Installed**, or **The game downloads it**.
2. If the server runs another build than yours, a warning says so: the game will not let you in until both match (Steam beta branch).
3. Type the server password if it has one, and tick **Remember the server password** to keep it. A kept one shows as a green line (**Server password kept in Windows**) instead of the field: the Studio fills it in when you join. **Change** opens the field for a new one and **Forget** deletes it. Your account password goes in the game's connect window.
4. Close the game if it is open, and press **Join**.

The game starts on this server's connect window with the server password filled in, and your username too when you typed it in **Your username on this server** (the Studio keeps it; the Studio Link mod fills it in). There you type your account password and press **Connect**, which stays off without a username. A new username makes a new account on that server. The game downloads the server's mods and uses them only there: your own mod list stays as it is; the profile you join with gets them (below).

With the Studio Link mod, the server also goes into the game's **Favorites** (the Favorites tab of its Multiplayer screen), with your username under it, as if you had added it with **Add Server** and **Add New Account**. So if you disconnect, it is there to join again without closing the game. It is never added twice, and your account password is not kept; the server password is, the way the game keeps it.

## Making a profile for a server

A server only announces its first few mods (Steam cuts the list). After you join it once from here, the Studio knows the full list, and the button becomes **Make a profile with these N mods**. Before that, **Make a profile with the N mods it announces** uses the partial list. The profile is named "Server: (name)" and keeps the server's order. Mods not installed now are pointed out before the profile is made.

To play with it, activate it in Profiles before joining. Each time you join from here, once the game has loaded the server's mods, they go into the profile active when you joined: on, in the server's order, with the ones the game had you download. Its other mods stay in it, switched off, except those that run only on your side (ZombieBuddy and its client-only Java mods, such as Viewpoint): they stay as you had them. The server's pane says which profile that is.

## Host a server: the server list

On the left, **Server Configs (.ini)** lists the server configurations in `Zomboid/Server`, with their mod and Workshop counts and **Running** for the one that is up. Type a name and press **Create** to make a new one, or **⚡ Create default configuration (servertest.ini)** when there is none. The bin deletes a configuration from disk (you are asked first). Pick one to see its tabs. The top banner shows **ONLINE** or **OFFLINE**, **Share Invite** (copies an invite text with the name, port, max players, password and mod count; you fill in your public IP and forward the port) and a button to reload the server files.

## Host & Launcher

- **Memory (-Xmx)**: 4, 8, 12 or 16 GB for the server.
- **Admin Password**: sets the password of the `admin` account when the server starts. The Studio keeps it in Windows Credential Manager (never in a file), so an assistant connected to the Studio can start this server again without being given it.
- **Auth & Network**: **Steam Enabled** or **NoSteam (LAN)**.
- **Launch Dedicated Server (name)** starts it in its own console window. While it runs, **Stop Dedicated Server** stops the server the Studio started (never other Java programs), and **Restart** stops and starts it again at once (to warn the players first, use the **Restarts** tab).

**Server program.** A server runs from the game's own files unless you choose otherwise. The **Server program** box offers the official **Project Zomboid Dedicated Server** instead: a separate program Steam gives for free (app 380870), which can be on another branch than your game and is updated apart from it.

1. Pick a **Dedicated server folder**: an empty or new folder on a disk with room (not the game's folder, not `Zomboid`). A dedicated server already on this PC (from Steam's Library > Tools, or an earlier SteamCMD) is found and offered.
2. Choose the **Branch**: **Stable (public)** or **Unstable (beta)**. It has to match the game of whoever joins; the box says which branch your game is on and starts on that one. A branch Steam keeps behind a password cannot be installed: the box says so.
3. Press **Install**. The Studio uses SteamCMD, Valve's command-line Steam. If it is not on this PC it asks first, then downloads it from Valve's own address (about 3 MB), checks that Valve signed it, and lets it update itself. Then SteamCMD downloads the server from Steam (several GB): the box shows each step with its percentage and gigabytes (about 7 GB for 42.22), and the folder's size once it ends. **Stop** ends it; pressing Install again carries on where it was.
4. Once it is installed, choose **The dedicated server**. Servers start from that folder from then on, with the same Zomboid folder (your server configs, saves and `Zomboid/mods`). **The game's files** goes back to the usual way. Change it with the server stopped.

**Update** runs SteamCMD again on that folder and branch (switching branch when you changed it). On a complete install it only fetches what changed and is done in seconds ("Already up to date" when nothing changed); after an install that was cut short, or a check of the files that was stopped, it checks every file, which takes many minutes. The box also says the game version the server starts as and whether it is your game's: a game only joins a server of its own version.

Java mods with ZombieBuddy need ZombieBuddy's `zbNative.dll` and `ZombieBuddy.jar` in that folder too: copy them from the game's folder (the **Copy ZombieBuddy's files to the dedicated server** button in **Mods & Sync** does it) (ZombieBuddy puts them there when the game starts with it). Until they are there, the server starts without its agent and **Mods & Sync** says so.

**Java mods (ZombieBuddy).** Build 42 mods with Java code need ZombieBuddy, and ZombieBuddy needs its agent on the server's command line. When the server's `Mods=` has ZombieBuddy (written `\ZombieBuddy` too), the Studio starts it with `-agentlib:zbNative=policy=deny-new`, and a note on this tab says so. *deny-new* means the server loads only the jars this PC already approved in ZombieBuddy's window (they are kept in `.zombie_buddy/mod_approvals.json` in your user folder) and refuses the rest without asking: a server has no window, and the question would stop it for good. **Mods & Sync** says which jars are not approved yet.

## Restarts

PZ Mod Studio can restart the server for you, warning the players in the game first. A restart sends the warnings over RCON (`servermsg`), then `save`, then `quit` (the server saves the world as it quits; it is closed by force only if it does not go within 90 seconds), and starts it again the same way it was started (same memory, Steam or NoSteam, program).

- **Scheduled restarts**: at times of day (this PC's clock) and/or **Every** 2 to 24 hours from when the server started. A time less than 30 minutes after the server started is skipped.
- **Warn the players before**: 60, 30, 15, 10, 5 and 1 minutes, the ones you tick (30, 15, 5 and 1 at first). The messages are in the app's language.
- **Restart when a Workshop mod of this server is updated**: every 15 minutes the Studio asks Steam (its public Web API) when each item of the server's `WorkshopItems=` was last updated. One updated after the server started gets a restart with the same warnings, and the server downloads the update as it starts.
- **Start it again if it stops by itself** (on at first): after a crash, if it had come up (a server that fails while it starts would fail again, so it is not started again). At most 3 times in 10 minutes; after that it stays off and the tab says so in red. Starting it by hand counts again from zero. A stop you asked for (Stop, Restart, an assistant, Discord) or an admin's `quit` in the game is never taken for a crash.

The top of the tab says what is coming: the next scheduled restart, or the one counting down with when and why. **Restart now (warn players)** starts a countdown of the minutes you choose (**no warning (now)** restarts at once); **Call it off** stops it and tells the players. **Last restarts** lists what happened and why: Scheduled, Mod update, Asked for, After a crash, Left off: too many crashes, Stopped by itself, Stopped while the app was closed.

**Only while the app is open.** The restarts are done by PZ Mod Studio. The server keeps running if you close the app, but then nothing restarts it, warns anyone or starts it after a crash. The schedule is saved (`Zomboid/PZModStudio_ServerSchedule.json`) and goes on when you open the app again; a server that stopped while the app was closed is noted, not started again.

## Mods & Sync

This compares your active mods with what the server's `.ini` has. A server does not read your mod list: it needs three lines in its `.ini`, `Mods=`, `WorkshopItems=` and `Map=`, and all three must agree or the server quietly plays without the mods. The Studio checks them and lists what is wrong (a mod never downloaded, an item never loaded, a map ignored).

**Write my list into this server (Mods, WorkshopItems and Map)** writes all three lines from your active mods, in load order, and keeps every other setting as it was. The previous file is saved in `Zomboid/PZModStudio_Backups`.

A PZ Mod Studio package (`Z_PZModStudio_*`) that is on goes into `Mods=` too, but it is a local mod, not a Workshop item: the server on this PC has it, players do not get it by downloading, and the game does not let them join without it. Share the package with them, or switch it off before you write the list.

The same check reads what the server will do with **Java mods**: a jar ZombieBuddy has not approved on this PC yet (new, or changed since: a mod update or a rebuild is a new jar) is refused by the server. Start the game once with that mod on, allow it in ZombieBuddy's window, and then start the server. A jar you told ZombieBuddy to refuse, and Java mods in a list without ZombieBuddy (only their Lua loads), are pointed out too.

With **NoSteam (LAN)** chosen on **Host & Launcher**, it also lists the mods that are only in the Steam Workshop folder. A server started without Steam reads only `Zomboid/mods`: it reports those "not found", and drops every mod that requires them too. **Copy N Workshop mod(s) into Zomboid/mods** copies each one's Workshop folder there. A copy, not a link: with some Workshop mods linked into `Zomboid/mods` (a directory junction) a NoSteam server crashes while it creates the world, so a link to a Workshop folder is flagged too and the same button replaces it with a copy. A copy also wins over the Workshop item in your own game. When the Workshop item is updated after the Studio copied it, the copy is flagged as outdated and the button copies it again. A folder of the same name the Studio did not put there (your own copy) is never replaced.

**Does this list work in multiplayer?** (**Check**) gives one verdict for the list and one per mod. It reads what each mod's description says about multiplayer, and lists calls that a mod's client code makes directly where the game routes them through the server; in multiplayer they change only that player's world, or are refused.

## Players

**Connected Players** lists who is online, read from the server's own log, with role, Steam id, where they entered and whether they died (the game's logs carry no ping or health). While the server runs, the Studio talks to it over RCON (the `RCONPassword` and `RCONPort` of its `.ini`; a server the Studio starts without a password gets one):

- **Kick** on a player's row disconnects them; they can join again. You can give a reason, which they see.
- **Ban** bans them from this server, with an optional reason. To take a ban back, use the server console: `unbanuser "name"`.
- The box at the top sends a message to everyone in the game (**Send to all**, or Enter); it shows on their screen.

The server's answer appears under the box.

## Quick .ini

Edit the main settings and press **Save Settings to .ini**: **Public Server Name**, **Server Password (Join)** (blank for an open server), **Admin Password (-adminpassword)**, **Max Players**, **Default Port**, **Public Description**, **PVP Combat** and **Pause When Empty**.

## All options

Every server and sandbox option, as the game shows them: by page, with its help, its default and your mods' sandbox options. **Server** and **Sandbox** switch lists, the search finds an option by name or help, and **Only the ones not at their default** shows what you changed. An option's arrow puts it back to its default. **Save** writes only what you changed, both files backed up first (to `PZModStudio_Backups/server_settings`), and refuses while that server runs. The Studio Link notes the list of options the first time you open the game to the main menu (and again when the build, the language or your mods change).

## Discord

A Discord webhook lets the server post to one channel: when it is up, stopped, crashed or back after a restart, who joins and leaves, and when its Workshop mods were updated and it needs a restart. No one is pinged.

1. In Discord, open the channel's settings, Integrations, Webhooks, New Webhook, and copy its URL.
2. Paste it under **Webhook address** and press **Save**. It is kept in Windows Credential Manager and never shown again.
3. Press **Send a test** and look at the channel.
4. Tick **Post this server's news to the channel** and choose the events: **Up, stopped and crashed**, **Players joining and leaving**, **Restarts**, **Workshop mods waiting for a restart**.

**Forget it** removes the address from this PC. Each server has its own webhook.

### Chat with the game: the server's own bot

Project Zomboid's dedicated server has a Discord bot of its own. The Studio sets it up in the server's `.ini`; it lives inside the server, so it works whenever the server runs, even with the Studio closed.

1. In the Discord Developer Portal: **New Application**, then **Bot**: **Reset Token** and copy it, and turn on **Message Content Intent**.
2. In **Chat with the game**, paste the **Bot token**, write the **Chat channel** (its name as it is in Discord, without `#`) and, if you want one, the **Log channel**, tick **Turn the bot on** and press **Save in the .ini**.
3. Press **Invite the bot to your Discord**, pick your Discord server, and start (or restart) the game server.

The in-game general chat then goes to that channel, and what is written there goes into the game. The game reads the token from the `.ini`, so it is written there: whoever has that file can use the bot. If the `.ini` has a command channel, the screen warns you: whatever anyone writes there runs as an admin console command; **Clear it** unless that channel is only yours.

A **status message that edits itself** (on by default) keeps one message in the channel saying whether the server is up, since when, and who is in, instead of posting a new one each time.

### Commands from Discord (while the Studio is open)

Off unless you turn them on. With the bot above set up, write a **Commands channel** and the **Role that may use them** (names as they are in Discord) and tick **Turn the commands on**. The Studio reads that channel every 5 seconds and answers there; members without the role are told they cannot.

- `!status` and `!players`: whether the server is up and who is in.
- `!restart [minutes]` (1 to 60, 5 if not said): the same warned restart as the **Restarts** tab: players are warned in the game with the warnings set there (those that fit in the time), the channel is told too, and it shows in the Restarts history; then the server saves, stops and starts again. `!restart cancel` calls off any restart counting down, a scheduled one too.
- `!save` saves the world; `!msg <text>` says something to everyone in the game.
- `!help` lists them. They also work in Spanish (`!estado`, `!jugadores`, `!reiniciar`, `!guardar`, `!mensaje`).

They talk to the server over RCON: a server the game made has no RCON password, so the Studio gives it a random one the next time it starts it (the `.ini` is backed up first); that also lets **Stop** save before quitting. They work only while PZ Mod Studio is open, and they never run a console command typed by hand. Use a channel only admins see: the chat channel also goes into the game.

## Good to know

- Posting to Discord only happens while PZ Mod Studio is open and hosting the server.
- Launching a server needs the game folder set in **App Settings**.
- Joining starts the game: close it first.
- To join your own server from this PC, use its LAN address (for example `192.168.0.10:16261`): a Steam client cannot join `127.0.0.1` or this PC's own public IP. A server started with **NoSteam (LAN)** needs a game started with `-nosteam` (there `127.0.0.1` works), and a game started with `-debug` may only join as an admin.
- Assistants connected to the Studio (MCP) can do the same: `server_start`, `server_status`, `server_stop`, `server_join`, `server_schedule`, `server_restart` and `dedicated_server`.
