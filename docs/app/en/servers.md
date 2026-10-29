# Servers

The Servers screen has two sides: **Join a server**, to find a multiplayer server and play on it, and **Host a server**, to run your own dedicated server from this PC. Switch between them with the two buttons at the top.

## Join a server: finding one

Under **Public servers**, press **Find servers**. The Studio asks Steam for every Project Zomboid server, the way the game's own browser does, through the Steam client running on this PC (no key needed; Steam shows you as in Project Zomboid while it asks). Servers appear as they answer; all of them take about a minute and a half. **Stop** ends the search, **Search again** starts over.

Narrow the list with the box (**Name or map**) and the filters: **Only my build**, **With players**, **Not full**, **No password**, **Modded and vanilla** / **Modded** / **Vanilla**, and sort by **Most players**, **Lowest ping** or **Name**. Right-click a server for a menu: **See what it says**, **Join**, **Star it**, **Copy the address** and making a profile with its mods.

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
- **Admin Password**: sets the password of the `admin` account when the server starts.
- **Auth & Network**: **Steam Enabled** or **NoSteam (LAN)**.
- **Launch Dedicated Server (name)** starts it in its own console window. While it runs, **Stop Dedicated Server** stops the server the Studio started (never other Java programs), and **Restart** stops and starts it again.

## Mods & Sync

This compares your active mods with what the server's `.ini` has. A server does not read your mod list: it needs three lines in its `.ini`, `Mods=`, `WorkshopItems=` and `Map=`, and all three must agree or the server quietly plays without the mods. The Studio checks them and lists what is wrong (a mod never downloaded, an item never loaded, a map ignored).

**Write my list into this server (Mods, WorkshopItems and Map)** writes all three lines from your active mods, in load order, and keeps every other setting as it was. The previous file is saved in `Zomboid/PZModStudio_Backups`.

A PZ Mod Studio package (`Z_PZModStudio_*`) that is on goes into `Mods=` too, but it is a local mod, not a Workshop item: the server on this PC has it, players do not get it by downloading, and the game does not let them join without it. Share the package with them, or switch it off before you write the list.

**Multiplayer check of the active mods** (**Check**) lists calls that a mod's client code makes directly where the game routes them through the server; in multiplayer they change only that player's world, or are refused.

## Players

**Connected Players** lists who is online, read from the server's own log, with role, ping, health and position. While the server runs, the Studio talks to it over RCON (the `RCONPassword` and `RCONPort` of its `.ini`; a server the Studio starts without a password gets one):

- **Kick** on a player's row disconnects them; they can join again. You can give a reason, which they see.
- **Ban** bans them from this server, with an optional reason. To take a ban back, use the server console: `unbanuser "name"`.
- The box at the top sends a message to everyone in the game (**Send to all**, or Enter); it shows on their screen.

The server's answer appears under the box.

## Quick .ini

Edit the main settings and press **Save Settings to .ini**: **Public Server Name**, **Server Password (Join)** (blank for an open server), **Admin Password (-adminpassword)**, **Max Players**, **Default Port**, **Public Description**, **PVP Combat** and **Pause When Empty**.

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
- `!restart [minutes]` (1 to 60, 5 if not said): players are warned in the game then, 5 and 1 minute before; then the server is stopped the usual way (it saves) and started again. `!restart cancel` stops it.
- `!save` saves the world; `!msg <text>` says something to everyone in the game.
- `!help` lists them. They also work in Spanish (`!estado`, `!jugadores`, `!reiniciar`, `!guardar`, `!mensaje`).

They talk to the server over RCON: a server the game made has no RCON password, so the Studio gives it a random one the next time it starts it (the `.ini` is backed up first); that also lets **Stop** save before quitting. They work only while PZ Mod Studio is open, and they never run a console command typed by hand. Use a channel only admins see: the chat channel also goes into the game.

## Good to know

- Posting to Discord only happens while PZ Mod Studio is open and hosting the server.
- Launching a server needs the game folder set in **App Settings**.
- Joining starts the game: close it first.
