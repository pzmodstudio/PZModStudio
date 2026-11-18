# PZ Mod Studio

PZ Mod Studio is a free mod manager for Project Zomboid on Windows and Linux. It keeps your mods in profiles, backs up your saves, checks your mod list the way the game reads it, and helps you fix what breaks. Press "?" on any screen for its own help page.

The first time you open the Studio a **Welcome** tour shows: a card at the bottom right that walks through every screen with the real screen behind it and its button lit up in the sidebar, with what the Studio already knows about your install (the folders it found, your profiles, the problems in your list). Go on with **Next** or the arrow keys; the X closes it. To see it again, open help ("?") and click **Show the welcome tour again**.

## The screens

The sidebar lists the screens under **Studio Modules**. Screens with a lock need an active profile first: create or activate one in Profiles.

- **Profiles**: named mod lists; create them, switch between them with one click, share them as `.pzpack` files.
- **Saves**: your saves and their backups; make one, keep one, or restore one.
- **Monitor Center**: launch the game and read its log, with one card per error and the mod it came from.
- **Mod List**: switch mods on and off and set the load order, with filters, groups, notes and undo.
- **What's new**: your Workshop mods updated since you last played, with their change notes and a warning when an update may break saves.
- **Diagnostics**: what the app knows about your mods (checked the way the game reads them: missing or switched-off requirements, wrong order, incompatible mods, mods your build cannot load, with fixes), the game (memory, CPU, start-up, frames, disk) and the server running on your PC, with the Doctor on the front page.
- **Browse Mods**: search the Steam Workshop, subscribe, keep your own lists, and install mods downloaded from elsewhere.
- **Maps**: every map mod's cells over the game's world, and where two map mods fight over the same cells.
- **Mod Merger**: when two mods change the same file, combine them into a patch package that loads last.
- **Developer**: for mod makers: check a mod's Lua and script files before the game does, and start the game with only that mod on. It shows only with **Developer features** on in App Settings.
- **Servers**: host and manage a dedicated server and keep its mods in step with yours.
- **App Settings**: where your game and mods live, which warnings you see, and other options.

## Starting the game

The green button at the top right starts Project Zomboid with your mod list. The arrow beside it opens the menu:

- **Launch modes**: with debugging (`-debug`, the Monitor Center follows the log live) or the plain game, each fullscreen or windowed.
- **Where to**: **Main menu**, as always, or **Custom…**, which opens **Custom launch** to go straight somewhere:
  - **A save**: your single-player saves, the active profile's first. The game opens and, at the main menu, the Studio Link mod loads that save, with the same notices and backup as **Continue**. Then comes the game's black "click to start" screen.
  - **A server**: your kept servers. Joins like **Join** in Servers, with the kept password and your username; the game loads the server's mods.
  - **How** picks the launch mode. **Pick** sets it as the button's place; **Pick and start** also starts the game. **Back to the main menu** clears it.

With a place picked, the button names it (**Play: (save)** or **Join: (server)**) and remembers it, like the mode, while **Remember how the game starts** is on in Settings.

## The Studio Link mod in the game

PZ Mod Studio Link (`Z_StudioLink`) is a small mod for Build 42 that the Studio installs and keeps on in every profile. It is the game's side of the app. Some of it works on its own; the rest needs the app.

### Its own Mods screen

The main menu's **MODS** button opens the Link's Mods screen instead of the game's. On its own it offers:

- every installed mod with a search box (name, mod id, Workshop id, author) and filters (**All**, **On**, **Off**, **Favourites**, **Updated**, **Problems**, **Maps**);
- each mod's details: what it requires and what needs it, its builds, its Workshop page;
- turning mods on and off together with what they require, several at once with Ctrl or Shift;
- the load order: sorted by it, moved up and down, or dragged, never above what a mod requires;
- groups by author or category, each mod's options (**Mod options**), the **Map order**, and **Undo** / **Redo**;
- **Shared files**: when two mods that are on ship the same Lua file, the game runs only one copy, the one of the mod that loads later. The mod whose copy is used is marked **Overrides**, the one whose copy is not used **Overridden**; the summary button **N share files** lists them (its tooltip explains the marks) and each mod's details name the files and the other mod. Checked against the mods the game loaded, a few seconds after the main menu appears; only Lua files are looked at;
- **Reload mods** has the game read its mod folders again, so a Workshop item you subscribed to (or that updated) while the game runs shows up without restarting; the game reloads its Lua, as after **Accept**;
- **Server lines** copies the mods that are on as a dedicated server's `Mods=`, `WorkshopItems=` and `Map=` lines, in load order (each mod after what it requires, the maps in the map order), to paste over those lines of the server's `.ini` in `Zomboid/Server`; mods with no Workshop item are named on a comment line, to copy into the server's own mods folder;
- **Accept** saves to the list the game reads, as the game's own screen does. **The game's screen** opens the original one.

With the app, the same screen also shows only the profile's mods (**Profile's mods** / **All installed**), Diagnostics's warnings on each mod, mods updated since your last session (in their details, their author's newest change notes and whether the update may break a save), your notes and groups, and a button to add new mods that are in no profile. **Sync with PZ Mod Studio** takes in what the app changed, and **Open PZ Mod Studio** brings the app to the front.

If another mod manager that owns the Mods screen is on, the Link asks which screen you want and leaves it to the other one if you say so.

### The card on the main menu

When the app has run on this PC, a card on the main menu shows whether the app is open or closed, the active profile and its number of mods, Diagnostics's errors and warnings, and **Continue: (save)** with what changed in its mods since its last session. If the app was never installed, the card instead invites you to get it, with **Download** (copies the download page's address) and **Hide**.

### The error notice

When the game records a Lua error, a notice names the mod it came from (and which mod called it), with the last message. **Copy report** puts the error on the clipboard to send to the mod's author, **Show in Mods** opens the Mods screen on that mod, and **See in the Studio** opens the Monitor Center on its card. The notice stays quiet when another mod that already shows these errors is on. **All (N)**, and **Lua errors (N)** on the pause menu, open every error of the session, each kind once with how many times it came, its mod and file:line: a click copies one, **Copy all** copies them all with the game build, the mode and how many mods are on. A report names each mod with its id, version and Workshop item.

Every error also goes to `Zomboid/Lua/PZModStudio_LinkErrorLog.txt` within a second of happening, so a crash, a disconnect or going back to the menu does not lose it: each kind once in full (mod, file:line, message, stack) and its repeats counted, under a heading with the date, the build and whether it was the menu or which save, with a line where the game went back to the main menu or the server dropped it. The file keeps about the last 3000 lines. **Error log** in the list opens its folder and copies its path.

For mod authors: `PZModStudioLink.registerReport("YourModId", fn)` (or `PZModStudioLinkReports["YourModId"] = fn` from Lua that runs before the Link's) adds your own information to the reports. `fn(err)` returns a string, a list of lines or a table of name = value pairs; it runs only when a report is made, inside `pcall`. Its text goes into **Copy all**, and into **Copy report** and the error log for an error of your mod. Other mods' debug reports, the ones mods already register for error tools, go into the same places under the name each mod gave them.

The game's own red **ERROR** counter in the bottom-right corner is drawn by the game's Java, which Lua cannot reach: the Link cannot hide it.

### Joining a server

When the game drops the connection because a Workshop mod differs from the server's, the Link names the mod (the game gives only a number), says what to do for the case (the server updates when it restarts, or Steam has to download the new version) and offers **Copy Workshop links**. When the game build differs from the server's, it says so with both builds.

When hosting (**HOST**, **Start**), the game loads that server settings' own mod list, not the Mods screen's. If that list lacks mods you have on, or names mods this PC does not have, the Link asks first: **Yes** hosts anyway, **No** goes back (**Edit Settings** changes its mods).

### Functions two mods change

When Diagnostics sees two mods redefine the same function, the Link asks the game, a few seconds after the main menu and after a save loads, which file the function that stayed comes from. Diagnostics shows it as **Seen in the running game**.

### With and without the app

- Without the app: the Link's Mods screen (with **Server lines**), the error notice and its error log (without **See in the Studio**), and the invitation card.
- With a profile active, even while the app is closed: the Mods screen shows the profile as the app last left it; on the load screen each save is marked by whether its mods match the profile; a save that uses mods outside the profile does not load; a save with an empty mod list asks first.
- Only while the app is open: backups of a save before it loads with changed mods and when you leave it, playing a save on a copy, adding new mods to the profile, **Sync**, **Open PZ Mod Studio** and **See in the Studio**.
- Notes, groups, profiles and Diagnostics are made in the app; the game only shows them.
