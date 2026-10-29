# Developer

The Developer screen is for people who make mods. You can create a new mod, check every file of a mod before the game does, start the game with only that mod on, reload its Lua while the game runs, check it for multiplayer and start the map editors.

## What you see

The screen only appears in the sidebar when **Developer features** is on in **App Settings**, and, like the other screens, once a profile is active. From top to bottom it has these cards: **A new mod**, **The mod you are working on**, **Reload in the game while it runs**, **Multiplayer check** and **Map tools**. The results of a file check appear at the bottom.

## Making a new mod

1. Press **New mod…**.
2. Fill in **Name**, **Id** (the folder and the list entry: letters, digits, `_` and `-`; it follows the name until you change it), **Author** and **Description**.
3. Tick **Build 41 too** if the mod should also work on Build 41.
4. Press **Make it**.

The Studio creates the mod in `Zomboid/mods` the way Build 42 reads it: `42/mod.info`, the `42/media/lua/client`, `server` and `shared` folders, `42/media/scripts`, `common/media/lua`, a poster to replace and one Lua file that prints the mod's name when the game loads it. It refuses an id that is not a plain name or a folder that already exists. **Open the folder** opens it in Windows Explorer, and the new mod is picked as the one you are working on. Switch it on in the Mod List when it is ready.

## Picking the mod you work on

Under **The mod you are working on**, choose a mod in the list. Your own mods (**In Zomboid/mods (yours)**) come first; Workshop mods follow, for a look. **Open its folder** opens the mod's folder. The Studio remembers your choice.

## Checking every file

Press **Check every file**. Every `.lua` file goes through the same Lua parser the Mod Merger uses, and every `media/scripts/*.txt` through a brace check (the game silently loses a whole script file at its first unbalanced block). The result says **Every file reads**, or lists the problems by file with line and column. Click a problem to open the file at that line in VS Code when it is installed, or else in the program Windows uses for that file.

This only checks syntax. What the game does with the files is shown in the Monitor.

## Starting the game with only this mod

**Start the game with only this mod** launches the game in Debug (Windowed) with only this mod, what it requires and the Studio's bridge on. Your mod list is set aside, and a banner says so. It comes back when the game closes, or at once with **Put my list back now**. The list is also kept in **Earlier lists** in the Mod List. The button is off while such a launch is running.

## Reloading in the game while it runs

This lets you change a Lua file and see it in the game without restarting.

1. Start the game with the bridge mod (`Z_PZModStudio_Bridge`) on.
2. Turn on the switch on the right of **Reload in the game while it runs**. It says **Watching (mod name)**.
3. Save a Lua file of the mod. The Studio sees it the moment it is saved, checks it with the parser and sends it to the game, which reloads it on the main menu, in game or paused.

**Reload every Lua of this mod now** sends all of the mod's Lua files at once.

The log below the buttons shows each step:

- **changed:** a file was sent to the game.
- **not sent, it does not parse:** the file has a syntax error, with its line, column and message. It is not sent, so the game never runs broken code.
- The game's answer for each file, in green when it worked and red when it failed.
- The times: detected, sent, applied in the game, and back in the Studio, in milliseconds.
- **The game is not running** or **No answer from the game in 6 s** when there was nobody to reload into (check that the bridge mod is on).

When a file is reloaded, its top-level code runs again, and the bridge keeps what that would break:

- **Event handlers**: each one the file adds runs once, a handler you add for the first time starts working, and one you take out of the file stops.
- **Tables**: when the file makes a global table again (`MyMod = {}`, `MyPanel = ISPanel:derive(...)`), the old table is kept and gets the new functions, so open windows and saved references run the new code with their state. A value the old table already had keeps its old value (the bridge cannot tell your state from a constant you changed): for a changed constant, use a full Reset Lua.
- **Your own set-up**: `if PZStudio then PZStudio.onReload(function(file) ... end) end` runs after each reload of that file, for what `OnGameStart` did and does not do again.

The log says what was kept, for example "1 handler(s), 1 table(s) kept".

## Multiplayer check

**Check this mod for multiplayer** looks at the mod's client and shared Lua for calls that change the world, a vehicle, an animal or the map directly, where the game's own code asks the server instead. Each finding shows the file, the line and the game's handler that does it on the server. A call inside an `if isClient()` branch counts as handled. Click a finding to open it in your editor. It checks only those calls: a mod with nothing listed can still misbehave in multiplayer in ways this cannot see.

## Map tools

TileZed draws the tiles of each cell (buildings, rooms, furniture, ground); WorldEd lays cells on the world grid and writes the map files the game reads. They come from The Indie Stone as one zip you unpack anywhere. The Studio looks for them in the usual places (your user folder, Documents, Desktop, Downloads, the C: drive) and in the folder you type in the box under each one. When it finds one, **Start TileZed** or **Start WorldEd** starts it from its own folder. A map mod's cells go in `media/maps/<name>` of the mod.

## Good to know

- Reloading needs the game running with the bridge mod on; the other tools do not need the game.
- Reloading in the game works without restarting; a constant you changed in a table keeps its old value until a full Reset Lua, and a `local` table that other files got through `require` keeps the old functions.
- The check, the reload and the multiplayer check need a mod picked whose folder the Studio knows.
