# Monitor Center

The Monitor shows the game's log as it is written and turns each error into a card that says which mod the game blamed and what you can do about it. It follows `console.txt` whether you started the game from the Studio or from Steam.

## What you see

- On the left, the log viewer, with one tab per log.
- On the right, **ERRORS AND WARNINGS**: the error cards, and above them the **Find the mod that does it** panel.
- At the top, **See how the game runs** opens Diagnostics > Game > Live: memory, CPU and which screen the game is on.
- Above the log, while the game runs (Full view), the **Live** panel: see below.
- At the top, **📁 Logs Folder** opens the folder with `console.txt` and the `Logs` subfolder in Windows Explorer.
- Next to it, the spam filter button. While it reads **Spam Filter: ACTIVE** it hides the lines about missing translations; press it to switch to **Filter: SHOW ALL**.

If the game folder is not set yet, the screen only shows **Configure Directory Paths in Settings**.

## Live (while the game runs)

In the Full view, while the game is open, a **Live** panel sits above the log. Its title line sums it up (FPS, Java memory, CPU, Lua errors); a click folds it. It is read every 2 seconds:

- **FPS, last minutes**: a line of the last 10 minutes (move the mouse over it to read a moment); the dashed line is 30 FPS. Under it the average frame and the longest frame of the last 10 seconds, the memory Java uses against its limit, the whole process's memory with what the PC has free, and the CPU. A second line follows the memory.
- **Lua time now, by mod**: the 5 mods whose Lua took the most time over the last 10 seconds, in milliseconds per second of play. It comes from the Studio profiler. When the profiler is off nothing is timed, and the panel says so with **Turn it on for the next start**: the profiler goes first in the mod list (and in the save you played last) and measures from the next time you start the game.
- **Lua errors this session**: how many, and which mods the game blamed, as the Studio Link counts them in the game; under it the newest error cards, marked **new** when they came while you watched.
- A last line says whether the Link's overlay shows in the game and what its own code costs.

FPS, Java memory and errors come from the Studio Link (version 1.20 or newer), once you are in a game. The panel goes away when the game closes.

### The overlay in the game

The Studio Link can show the same numbers over the game: FPS and frame time, Java memory against its limit, CPU, the 5 mods with the most Lua time in the last 10 seconds (with the profiler on) and the session's Lua errors with the mods blamed. Open it from the pause menu with **Live overlay**, or give it a key in the game's **Options > Mods > PZ Mod Studio Link** (it has none until you choose one, so it never takes a key of the game's). Drag it where you want it; it stays there and comes back in the next game. Its last line says what it costs itself, in milliseconds per second. The memory limit and the CPU are known only while PZ Mod Studio is open.

## The log viewer

Two tabs are always there:

- **console.txt (Client)**: the live game log. The Studio starts reading at the end of the file, so you see what happens from the moment the Studio is open. When a new game session starts, the file starts over and the Studio reads it from its first line. The tab keeps the latest 1,500 lines, also while you are on another screen.
- **Dedicated Server (Live)**: the log of the dedicated server, while one you started in **Servers** is running. It shows its latest 1,500 lines and is read again every 2 seconds.

To read other logs, press **Load Log**. The menu lists the logs found in your Zomboid folder (with their size); pick one to open it in a new tab. **📂 Browse another file (.txt)...** opens any text file. A loaded log shows its last 2,000 lines. These tabs can be closed with their X; the two live tabs cannot.

Under the tabs:

- The search box keeps only the lines that contain what you type.
- The chips **All**, **🚨 Errors**, **🌙 Lua** and **☕ Java** keep only lines of that kind. **⚡ Bridge** appears only with Developer features on.
- The footer shows how many lines are shown out of the total.

The small buttons at the right of the tabs:

- The disk button saves what the tab holds as a new file in `Zomboid/Logs/`.
- The copy button copies the lines you can see (after filters) to the clipboard.
- The trash button empties the viewer for this tab. It does not touch the file on disk.

Error lines are shown in red, warnings in amber.

## The error cards

A card appears when the game throws an error, or when it writes a warning and names a mod. Everything else stays in the log on the left. Cards that threw (red) come first, warnings (amber, marked **warning · the game carried on**) after them. The same problem appears once, however many times the game repeats it; the number like **600×** says how many times it is in the log, and the ones that repeat are the ones worth fixing first.

Above the cards, a badge counts the exceptions and another the warnings (each card says whether there is anything to do), with a line that sums up the log. When the Studio remembers earlier sessions, each card says **new** or **seen in N sessions since** a date, and **Only the new ones** hides the rest.

A card can show:

- **The mod's name**, when the game named it on the error line or in the stack trace under it.
- **called from ...**: the mods whose code called the one that failed, read from the game's own stack. The fault can be in either one.
- **Log Snippet**: the line itself.
- For a failed `require`, where that module really is now, when the Studio finds it.
- What to do next, or a note that the Build 42 shim already covers it.
- When the error is in a copy your Studio package holds: a note when the mod changed that file after the copy was made, or when no mod in your list ships that file any more.

When a new game session starts, the cards of the last one are cleared. **Discard** clears them by hand.

## Fixing from a card

- **Fix the path** (or **Fix the path in File.lua**): appears when a mod asks for a module by an old path and the Studio knows the new one. It rewrites the `require` in every file of the active mods that asks for it and publishes them into your Studio package. The mod is not changed and stays on. A green line says what was fixed; relaunch the game.
- **Patch File.lua** or **Patch a file of it**: opens that mod's files in the Mod Merger so you can correct one. Your copy goes into a package that loads last, so the game reads it instead of the mod's. Warnings with no known fix do not get this button.
- **Remove the copy of File.lua**: appears when the error is in a file your Studio package already patched. It takes that copy out of the package (after asking), and the game reads the mod's own file again. Any edit you made to the copy is lost.
- **Switch it off**: asks for confirmation, switches the mod off and takes you to the Mod List.
- **A merge package may fix this — open the Mod Merger**: for errors the Build 42 shim answers. The shim travels inside a published merge package; the card tells you if yours is already live or still a draft.

## Reports

- **Save report** writes a Markdown file with the error, your game build, the mod and its code, plus the log lines around it. It is ready to give to a mod author or an AI assistant.
- **Copy report** puts the same report on the clipboard.
- **🔍 Inspect in Logs** puts the file or key word of the error in the search box on the left.
- **Copy the line** copies only the error line.

## Find the mod that does it

For problems the log does not name: a freeze, a recipe that never appears, a save that will not open.

1. Open the panel. If the problem does not happen every time, tick **Does not happen every time**. If the log shows some text of the error, type it: after each test game the app reads the log and suggests the answer. Press **Start halving the list**, then confirm.
2. Part of your suspects is switched on; the ones this PC already suspects (sessions that went wrong, errors that repeat) are tried first. Mods that another mod requires stay on in every round, so nothing breaks for that reason.
3. Press **Launch the game**, play the way you did when it happened, and close it.
4. Answer **It still happens** or **It is gone**. The list is halved again. The badge shows the round and how many launches are left (about 8 for a hundred mods).
5. When one mod is left, play once more to confirm it.

At the end the panel names the mod, shows any lines the log has about it (with **Patch File.lua** when a line names a file, and **Show its lines in the console**), and offers **Patch a file of it**, **Switch it off** or **Keep it on**. Your original list is put back whichever way the hunt ends. **Stop and put my list back** ends it early.

If the last mod does not do it on its own, the hunt goes on by itself: it keeps that mod on and looks for the other mod that does it together with it, then names the pair. If it finds neither a mod nor a pair, the panel says why: one of the required mods, more than two mods, or not a mod at all.

## Good to know

- The hunt writes your Mod List itself. Do not switch mods on or off in the Mod List while it runs, or the next answer means nothing.
- A hunt survives closing the Studio; it is saved in `Zomboid/PZModStudio_Bisect.json`.
- **Launch the game** in the hunt starts the game normally, without debug mode.
- Fixes published into a package take effect the next time the game starts.
