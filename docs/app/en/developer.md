# Developer

The Developer screen is for people who make mods. You can create a new mod, check every file of a mod before the game does, start the game with only that mod on, reload its Lua while the game runs, check it for multiplayer and against the installed build, write translation drafts, publish it to the Steam Workshop and start the map editors.

## What you see

The screen only appears in the sidebar when **Developer features** is on in **App Settings**, and, like the other screens, once a profile is active. At the top, **The mod you are working on** holds the mod every tool works on, with **Open its folder** and **Start the game with only this mod**. Below it there is one tab per tool: **Check files**, **Live reload**, **Multiplayer**, **Against the build**, **Translations**, **Workshop**, **Maps** and **New mod**. A tab shows a small badge when it has something to say (problems found, watching, the multiplayer verdict). The Studio remembers the last tab you opened. Every tab but **Workshop**, **Maps** and **New mod** needs a mod picked first.

## Making a new mod

1. Open the **New mod** tab.
2. Fill in **Name**, **Id** (the folder and the list entry: letters, digits, `_` and `-`; it follows the name until you change it), **Author** and **Description**.
3. Tick **Build 41 too** if the mod should also work on Build 41.
4. Press **Make it**.

The Studio creates the mod in `Zomboid/mods` the way Build 42 reads it: `42/mod.info`, the `42/media/lua/client`, `server` and `shared` folders, `42/media/scripts`, `common/media/lua`, a poster to replace and one Lua file that prints the mod's name when the game loads it. It refuses an id that is not a plain name or a folder that already exists. **Open the folder** opens it in Windows Explorer, and the new mod is picked as the one you are working on. Switch it on in the Mod List when it is ready.

## Picking the mod you work on

Under **The mod you are working on**, choose a mod in the list. The mods are grouped by where they come from, the ones you work on first: **Your Workshop uploads (Zomboid/Workshop)**, **Folders in Zomboid/mods**, **Installed from Nexus, a file or a link**, the copies the Studio adapted to your build, **The Studio's own mods**, and **Workshop subscriptions** last, for a look. Under the list you see the mod's id, version, Workshop number and folder. **Open its folder** opens the mod's folder. The Studio remembers your choice.

## Checking every file

In the **Check files** tab, press **Check every file**. Every `.lua` file goes through the same Lua parser the Mod Merger uses, and every `media/scripts/*.txt` through a brace check (the game silently loses a whole script file at its first unbalanced block). The result appears right under the button, with how many Lua and script files were checked. It says **Every file reads**, or lists the problems by file with line and column. Click a problem to open the file at that line in VS Code when it is installed, or else in the program Windows uses for that file.

This only checks syntax. What the game does with the files is shown in the Monitor.

## Starting the game with only this mod

**Start the game with only this mod**, at the top of the screen, launches the game in Debug (Windowed) with only this mod, what it requires and the Studio's bridge on. Your mod list is set aside, and a banner says so. It comes back when the game closes, or at once with **Put my list back now**. The list is also kept in **Earlier lists** in the Mod List. The button is off while such a launch is running.

## Reloading in the game while it runs

This lets you change a Lua file and see it in the game without restarting.

1. Start the game with the bridge mod (`Z_PZModStudio_Bridge`) on.
2. Open the **Live reload** tab and turn on the switch on the right. It says **Watching (mod name)**.
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
- **Files that `require` it**: the mod's other files that load the changed file with `require` are reloaded right after it, so they get its new code.

The log says what was kept, for example "1 handler(s), 1 table(s) kept".

## Multiplayer check

In the **Multiplayer** tab, **Check this mod for multiplayer** looks at the mod's client and shared Lua for calls that change the world, a vehicle, an animal or the map directly, where the game's own code asks the server instead. Each finding shows the file, the line and the game's handler that does it on the server. A call inside an `if isClient()` branch counts as handled. Click a finding to open it in your editor. It checks only those calls: a mod with nothing listed can still misbehave in multiplayer in ways this cannot see.

## Against the installed build

In the **Against the build** tab, **Check the names this build has** looks up every name the mod's Lua calls in the game you have installed: its Java classes and methods, its global functions, its own Lua and the events it triggers, plus what this mod and your other active mods define. What is left is a call that would fail at that line: a function Build 42 dropped, a renamed method, an event that no longer fires, a typo. The findings are grouped as global functions, methods and events, each with its file and line, and a "did you mean" when a known name is close. Click one to open it in your editor. A call the file also names without calling (`if foo then`) counts as guarded and is left out.

The first check reads the game's files and takes a few seconds; after that it is kept per build. It goes by name only: a name built while the game runs is not seen, and a mod that ships Java may define names Lua cannot show (the result says so). Open the line to judge.

## Translation drafts

The **Translations** tab shows the language the mod is written in, how many texts it has, and its languages in groups: the ones with texts missing, the complete ones, and folders of the mod that are not game languages. Tick the languages you want and press **Write drafts for…**: the Studio writes a draft of every text each language lacks, translated by the same service that translates mod descriptions (**Translation** in **App Settings**), into the mod's own `Translate/<LANG>/` folder. What is already there is left as it is. **Open the Translate folder** opens it. A draft is a start, not a translation: read it over before publishing.

## Publishing to the Workshop

The **Workshop** tab uploads a mod to the Steam Workshop through the Steam client running on this PC. It uses the same files as the game's own uploader, in `Zomboid/Workshop/<item>/`: `workshop.txt`, `preview.png` and `Contents/mods/<mod>/`.

1. Pick an item in the list. For a mod of your own in `Zomboid/mods` that has no item yet, press **Prepare (mod) for the Workshop**: it makes the item folder with a copy of the mod, a `workshop.txt` from its `mod.info` and its poster as the preview.
2. Fill in **Title**, **Visibility** (**Public**, **Friends only**, **Private** or **Unlisted (by link)**), **Tags (separated by ;)** and **Description** (Steam takes up to 8000 characters), and press **Save workshop.txt**.
3. **Look at the files** shows what would be sent and what changed since the last upload from the Studio. A `.workshopignore` in the item folder (one pattern per line, such as `*.psd` or `.git/`) keeps files out.
4. Write a change note if you want, and press **Publish on the Workshop** (or **Upload the update** for an item already published). You are asked first. Steam must be running and signed in to the account that owns the item.

What is sent is what `workshop.txt` says: an edit made on the Steam page is replaced by the next upload, the way the game does it. If Steam keeps a new item hidden until you accept its Workshop legal agreement, the result says so, with a link. **See the Workshop page** opens the item in your browser. The Studio does not upload someone else's mod: when the item ships a mod you are subscribed to, it says so and the button stays off. The bin button next to the list takes the item folder out of `Zomboid/Workshop` (to the Recycle Bin); nothing changes on Steam.

## Map tools

In the **Maps** tab, TileZed draws the tiles of each cell (buildings, rooms, furniture, ground); WorldEd lays cells on the world grid and writes the map files the game reads. They come from The Indie Stone as one zip you unpack anywhere. The Studio looks for them in the usual places (your user folder, Documents, Desktop, Downloads, the C: drive) and in the folder you type in the box under each one. When it finds one, **Start TileZed** or **Start WorldEd** starts it from its own folder. A map mod's cells go in `media/maps/<name>` of the mod.

## Good to know

- Reloading needs the game running with the bridge mod on; the other tools do not need the game.
- Reloading in the game works without restarting; a constant you changed in a table keeps its old value until a full Reset Lua.
- Every tool but **Workshop**, **Maps** and **New mod** needs a mod picked whose folder the Studio knows.
