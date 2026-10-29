# Monitor Center

The Monitor shows the game's log as it is written and turns each error into a card that says which mod the game blamed and what you can do about it. It follows `console.txt` whether you started the game from the Studio or from Steam.

## What you see

- On the left, the log viewer, with one tab per log.
- On the right, **DIAGNOSTICS**: the error cards, and above them the **Find the mod that does it** panel.
- At the top, **📁 Logs Folder** opens the folder with `console.txt` and the `Logs` subfolder in Windows Explorer.
- Next to it, the spam filter button. While it reads **Spam Filter: ACTIVE** it hides the lines about missing translations; press it to switch to **Filter: SHOW ALL**.

If the game folder is not set yet, the screen only shows **Configure Directory Paths in Settings**.

## The log viewer

Two tabs are always there:

- **console.txt (Client)**: the live game log. The Studio starts reading at the end of the file, so you see what happens from the moment the Studio is open. When a new game session starts, the file starts over and so does the tab.
- **Dedicated Server (Live)**: the log of the dedicated server, while one you started in **Servers** is running.

To read other logs, press **Load Log**. The menu lists the logs found in your Zomboid folder (with their size); pick one to open it in a new tab. **📂 Browse another file (.txt)...** opens any text file. These tabs can be closed with their X; the two live tabs cannot.

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

A card can show:

- **The mod's name**, when the game named it on the error line or in the stack trace under it.
- **called from ...**: the mods whose code called the one that failed, read from the game's own stack. The fault can be in either one.
- **Log Snippet**: the line itself.
- For a failed `require`, where that module really is now, when the Studio finds it.
- What to do next, or a note that the Build 42 shim already covers it.

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

1. Open the panel and press **Start halving the list**, then confirm.
2. Half of your suspects are switched on. Mods that another mod requires stay on in every round, so nothing breaks for that reason.
3. Press **Launch the game**, play the way you did when it happened, and close it.
4. Answer **It still happens** or **It is gone**. The list is halved again. The badge shows the round and how many launches are left (about 8 for a hundred mods).
5. When one mod is left, play once more to confirm it.

At the end the panel names the mod, shows any lines the log has about it, and offers **Patch a file of it**, **Switch it off** or **Keep it on**. Your original list is put back whichever way the hunt ends. **Stop and put my list back** ends it early.

If no single mod is to blame, the panel says why: it may take two mods together, or be one of the required mods, or not be caused by a mod at all.

## Good to know

- The hunt writes your Mod List itself. Do not switch mods on or off in the Mod List while it runs, or the next answer means nothing.
- A hunt survives closing the Studio; it is saved in `Zomboid/PZModStudio_Bisect.json`.
- **Launch the game** in the hunt starts the game normally, without debug mode.
- Fixes published into a package take effect the next time the game starts.
