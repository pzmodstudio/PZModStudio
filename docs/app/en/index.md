# PZ Mod Studio

PZ Mod Studio is a free mod manager for Project Zomboid on Windows. It keeps your mods in profiles, backs up your saves, checks your mod list the way the game reads it, and helps you fix what breaks. Press "?" on any screen for its own help page.

The first time you open the Studio a **Welcome** tour shows: a card at the bottom right that walks through every screen with the real screen behind it and its button lit up in the sidebar, with what the Studio already knows about your install (the folders it found, your profiles, the problems in your list). Go on with **Next** or the arrow keys; the X closes it. To see it again, open help ("?") and click **Show the welcome tour again**.

## The screens

The sidebar lists the screens under **Studio Modules**. Screens with a lock need an active profile first: create or activate one in Profiles.

- **Profiles**: named mod lists; create them, switch between them with one click, share them as `.pzpack` files.
- **Saves**: your saves and their backups; make one, keep one, or restore one.
- **Monitor Center**: launch the game and read its log, with one card per error and the mod it came from.
- **Mod List**: switch mods on and off and set the load order, with filters, groups, notes and undo.
- **What's new**: your Workshop mods updated since you last played, with their change notes and a warning when an update may break saves.
- **Mod Health**: checks the active list like the game does: missing or disabled requirements, wrong order, incompatible mods, mods your build cannot load, with fixes.
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
- **Accept** saves to the list the game reads, as the game's own screen does. **The game's screen** opens the original one.

With the app, the same screen also shows only the profile's mods (**Profile's mods** / **All installed**), Mod Health's warnings on each mod, mods updated since your last session, your notes and groups, and a button to add new mods that are in no profile. **Sync with PZ Mod Studio** takes in what the app changed, and **Open PZ Mod Studio** brings the app to the front.

If another mod manager that owns the Mods screen is on, the Link asks which screen you want and leaves it to the other one if you say so.

### The card on the main menu

When the app has run on this PC, a card on the main menu shows whether the app is open or closed, the active profile and its number of mods, Mod Health's errors and warnings, and **Continue: (save)** with what changed in its mods since its last session. If the app was never installed, the card instead invites you to get it, with **Download** (copies the download page's address) and **Hide**.

### The error notice

When the game records a Lua error, a notice names the mod it came from (and which mod called it), with the last message. **Copy report** puts the error on the clipboard to send to the mod's author, **Show in Mods** opens the Mods screen on that mod, and **See in the Studio** opens the Monitor Center on its card. The notice stays quiet when Error Magnifier is on.

### With and without the app

- Without the app: the Link's Mods screen, the error notice (without **See in the Studio**), and the invitation card.
- With a profile active, even while the app is closed: the Mods screen shows the profile as the app last left it; on the load screen each save is marked by whether its mods match the profile; a save that uses mods outside the profile does not load; a save with an empty mod list asks first.
- Only while the app is open: backups of a save before it loads with changed mods and when you leave it, playing a save on a copy, adding new mods to the profile, **Sync**, **Open PZ Mod Studio** and **See in the Studio**.
- Notes, groups, profiles and Mod Health are made in the app; the game only shows them.
