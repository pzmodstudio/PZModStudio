# Diagnostics

Diagnostics brings together what the app knows about your mods, the game and the server. Four buttons at the top: **Overview**, **Mods**, **Game** and **Server**. It always opens on Overview; each part remembers the tab you last looked at.

## Overview and the Doctor

- Three cards, one per part, each saying how it is in one line. **Mods**: something that breaks the game, something worth fixing, or all in order. **Game**: open or closed, how much memory it uses, the last start, the memory limit and whether a session ran out of memory. **Server**: whether one runs on this PC and how it is doing. Click a card to open that part.
- Below them, **the Doctor**: it asks what is wrong (**The game crashes or will not start**, **A mod does not work**, **It is slow**, **Something else**) and takes you step by step with what the app already knows: what the mod check found, the Monitor Center's errors and the times the game ran out of memory. When a step sends you elsewhere, coming back finds the same step. **Start over** goes back to the question.
- The **Doctor** button at the very top opens this page from any screen.

## Mods

Checks your active mods the way the game reads them, and lists every problem it finds with a button to fix it. It looks at what your game build can load, missing or switched-off requirements, the load order, incompatible mods, files two mods both change, and mods that filled the console last time you played.

- At the top: your **Game build** and how many mods are active.
- **Studio notes** (for example **Studio notes: 22 mods**): the load-order rules, incompatible pairs and known issues the app carries inside. When you marked warnings with **No longer happens**, a button beside it says how many and shows them all again.
- **Check again** runs the check again. It also runs by itself when your list changes.
- Five tabs; the first three say how many rows ask for something:
  - **What to do now**: what breaks the game or will (missing or switched-off requirements, load order, mods your build cannot load, incompatible pairs). **Fix N problem(s) automatically** is here. Folded below: what the Studio already fixed and what is only good to know.
  - **Conflicts**: two mods giving the same file, item, recipe or text; the game uses one, and which one is your call. **Merge N files automatically** is here.
  - **Security**: what a mod could do beyond modding (below).
  - **Audit**: your list mod by mod, each under one verdict: **Broken**, **Choose**, **Unneeded**, **Check** or **Keep**. Click a mod to see why, and **Go** to go to the warning.
  - **Time per mod**: which mods slow the game down while you play (below).
- Each row shows the mod, what is wrong, a short code (like `missing_dependency`) and its fix buttons on the right. **Copy for an assistant** copies the row with a question, to paste into any chat assistant; nothing is sent from the Studio. A problem chip in the Mod List opens the tab that has it.

If nothing is wrong, you see **No problems found in your N active mods.**

### What to do with each row

Each row says one of four things, the same as in the Mod List:

- **You need to act**: the game breaks or does not load the mod until you do something. Start here.
- **Worth doing**: it works, but it is worth fixing (wrong order, an update not downloaded yet, a mod filling the console). The button says what it changes.
- **Good to know**: nothing to do, just so you know (two mods define the same thing, mods updated since you last played).
- **Already fixed by the Studio**: leave it alone, this is how it works (a compatible copy, a file in your package). Each can be undone if you need to.

Each section can be folded by clicking its title.

### The problems it finds

- `missing_mod` (Not installed): a mod in your list is not on disk. If an installed mod has almost the same id, it offers to switch that one on instead.
- `not_for_build`, `outside_version_range`, `invalid_mod_info`: the game will not load this mod on your build, or its `mod.info` is broken.
- `missing_dependency`: the mod requires another that is not installed.
- `dependency_not_enabled`: the required mod is installed but off.
- `dependency_after`, `load_after_order`, `load_before_order`: two mods are in the wrong order.
- `dependency_unavailable`: a required mod cannot load itself.
- `incompatible`: the mod says it does not work with another mod you have on.
- `duplicate_entry`: the same mod is in the list twice.
- `two_mod_managers`: another mod manager that draws the game's Mods screen is on next to Studio Link.
- `workshop_update_pending`: Steam has a newer version that is not downloaded yet. Servers reject players whose mod versions differ.
- `file_conflict`: two mods ship the same file and the game only uses the last one.
- `function_overridden` (**Cuts another mod off**): a mod replaces a function of the game, or of another mod, after another mod changed it, so that change never runs. When many functions of one mod are cut off by another (two versions of the same mod on at once), it is one row. With Studio Link on it is checked in the running game: the row says **Seen in the running game**, or `function_seen_other` (**Another version in the game**) shows when the game has another mod's version.
- `java_link_broken` (**Java older than this build**): the mod's Java uses classes, methods or fields the installed game no longer has (an update changed them). That code fails when it runs. If the Monitor shows it failing, look for an update of the mod or turn it off; if it works, the mod checks first.
- `tiledef_clash`: two mods file their tiles under the same `tiledef` number with different sheets, so some tiles show as the other mod's or go missing. Keep one on, or ask an author to change the number. `file_conflict` rows also say whether the log of the last session (with this same list) confirms which copy the game used.
- `game_folder_changed` (**The game's folder changed**): while the game ran, files of its own folder changed, where the game never writes. The app compares the game's folder, the Zomboid folder (without saves or logs) and ZombieBuddy's when the game opens and when it closes, only while the app is open.
- `script_definition_conflict` and `translation_conflict`: two mods define the same item, recipe or name differently; the one that loads last wins. Rows ending in `_stale` mean you merged it before, but a mod changed since; `_settled` means your package already settles it.
- `floods_console` (shown in the Mod List as **Fills the console**): in your last session this mod wrote more than 1,000 lines of errors to the console. That slows the game and hides other mods' errors. Look at them in the Monitor Center, or switch the mod off.
- `user_rule_order`: one of your own load-order rules is broken.
- `community_rule_order`, `community_incompatible`, `community_known_issue`, `community_successor`: what the Studio's rules and mod index say about your mods. A known issue has **No longer happens**: it is not shown again on this PC.
- `learned_issue`: an error a mod gave in 3 or more of your sessions with the same version (see "What this PC learns").
- `package_built_for_other_list`: your Studio package was built with a different mod list than this profile.
- `package_file_stale`: your package carries an old copy of a mod's file, and the mod has changed that file since. The game reads the old copy.
- `package_file_differs`, `package_file_orphan`: your package carries a file that only one mod in the list ships, or that no mod in the list ships. Take it out if you no longer need it.
- `security_executable`, `security_script`, `security_java`, `security_lua`: what a mod could do beyond modding, on the **Security** tab (below).
- `compatible_copy`, `shadowed_copy`, `updated_since_last_play`: notes about which copy of a mod is used and what changed.

Rows about the same definition can be opened with **By file** to see each mod's file. Click a definition there to open it in the Merger.

### The fix buttons

- **Enable X** or **Disable X**: switches a mod on or off. Switching off asks first; the mod stays installed.
- **Move X before Y**: changes the load order. On a "your call" row (two mods define the same thing) it reads **Use Y's version instead**.
- **Remove X from the list**: takes an entry out of the list.
- **Search Workshop for "X"** and **Open Workshop page**: open the Steam Workshop in your browser.
- **Make compatible copy**: makes an adapted copy of a Workshop mod in `Zomboid/mods` for your build. The result shows what it changed and what may still break. Start the game from the Studio so it uses the copy. **Back to the original** removes the copy.
- **Keep the version you have**: saves the mod as it is today, before a Workshop update lands. You can put it back from the Mod List (right-click, Kept versions).
- **Merge automatically** and **Open in Merger**: settle a file or definition conflict in your patch package, which loads last.
- **Take out of the package**: takes a file out of your patch package, after asking; the game reads the mod's own file again on its next launch.
- **Switch the shim's X patch on**: turns on a patch of the Build 42 shim that a known issue needs.

A green line confirms each change, with **Saved to your mod list.**

### Fixing many at once

- **Fix N problem(s) automatically** switches on missing requirements and moves mods earlier where needed. It never switches anything off and skips the "your call" rows. It asks first, and the Mod List can undo it.
- **Merge N files automatically** merges every conflicting file that has no overlapping changes into your patch package. Files where both mods change the same lines are listed for you to decide in the Merger.

### The Studio's rules and index

The Studio carries load-order rules, pairs of mods that do not go together, and an index of known issues with their fix. They come with each version of the app; nothing is downloaded, and nothing is sent. They show up as `community_*` rows. A known issue is left out by itself when the mod's own files show it is gone (an update fixed the file it was about), or when the mod changed after the issue was seen and its log line no longer appears in your session. **No longer happens** leaves one out by hand.

### What this PC learns

Besides the index it ships with, each app learns from its own PC. In the background it reads the game and server session logs the game keeps (`Zomboid/Logs`) and notes which mods were on, in which version, and which errors the game blamed on them (the same ones the Monitor shows). It keeps this in `Zomboid/PZModStudio_Learned.json` and sends nothing.

When a mod gives the same error in 3 or more sessions, with the same version and on the installed build, it shows in Diagnostics as **Learned on this PC** (`learned_issue`), with the count as its evidence: "in 8 of 14 of your sessions". When the app's index already speaks of that problem, only the index's row shows. A Java patch the log blames on several mods at once is not learned, as it cannot tell whose it is. When the mod updates, the count starts from zero; when its last 5 sessions went without it (fixed, or a fix that worked), it is no longer shown. **No longer happens** takes it out by hand. The Diagnostics front page says how many sessions it learned from.


### The Doctor and the brain

The Diagnostics front page has **Ask the Doctor**, a chat that answers with no AI: it tells what you ask by its words and builds the answer from what this PC learned and from the app's tools (the same as the MCP server's). Each answer says which tool it came from. Ask it what it knows, why the game closed, what changed when an error began, which mods fail, whether loading is slower, whether fixes worked, what the last game update did, or name a mod to see its history.

It forgives typos and other ways of saying it ("it crashed", "it froze", "waht changed"), reads dates ("what changed since yesterday?", "this week", "since 12/11"), "which one takes longest to load?" or "which mod gives the most errors?", and follows the conversation: after a mod you can say "and since when?", "switch it off", "note it" or "show it". When unsure it asks "which of these?" with buttons; when it does not understand and you show it with a button what you meant, it remembers that phrase on this PC. Each answer leads with the conclusion, then one piece of advice with its evidence (for instance that almost all crashes were the PC running out of memory with the game and a server open), the details folded, and buttons to go to the Mod List or the part of Diagnostics it is about.

Also:

- **Which mod causes it?** It compares the sessions with and without each mod: "with X 3 of 7 went badly; without it, 24 of 196", and says how sure it is (fairly sure, likely, could be chance). A mod that is in nearly every session cannot be compared and is not named.
- **Hunt it by halves** from the chat: it switches half your mods off (what the others need stays on), lets you play the trial and you answer "it still happens" or "it no longer happens" until one is left. Your list is kept and comes back at the end.
- **The report**: when you open the chat after playing, it tells how your last session went (time played, how it ended, new errors, fixes that worked). "What happened this week?" does the same for a period.
- **Questions about the app** ("how do I make a profile?", "what is a compatible copy?"): it answers with the paragraph of this guide and a button to that screen.
- **Groups, profiles and the Workshop**: "VP Base" tells how that group is doing, "profile Knox Lives" whether its last sessions went well; "is there a new version of X?" and "what do the comments of X say?" read its page.
- **Several questions at once** ("what changed and which mods fail?"), **nicknames** it learns when you pick among several mods, **"undo that"** for its last change, and **memory**: next time it reminds you which mod you talked about and how it is now.

**With the game (Studio Link):**

- On the game's main menu, the PZ Mod Studio card (top right) shows what the Doctor would say first: whether the last sessions with this list went badly, whether the last one crashed, which errors repeat. The app writes it every time it checks, so the card says the last thing it knew even when the app is closed.
- The Link keeps a diary of which save you loaded and whether you went back to the menu (rather than closing the window). So each session says "save X · went back to the menu", also those played with the app closed.
- On the pause menu (Esc) there is **Mark this moment**: when something odd happens, press it. Then ask the Doctor "what happened when I marked?" and it tells you what the game's log wrote in that minute (a minute and a half before to half a minute after).

It can also change things, always with your **Do it**: make, rename or delete groups of the active profile and put mods in or take them out ("group A, B in Maps", "take A out of group Maps", "rename group Maps to World", "delete group Maps"), suggest groups for your active mods with none (those that share a name or what they require), and write or remove **Studio notes** ("note on A: text", "remove the Studio note of A", "note what you learned"). Studio notes are kept apart from yours and shown in violet with their own button to remove them; your notes it never writes, changes or removes.

What the brain tells from the sessions:

- **What changed**: when an error repeats, when it began and what was different from the last session that went without it (mods added, taken out or updated, its place in the order, the game's build). It says it as what changed then, not as the cause. It shows on the Diagnostics row and in the mod's history.
- **Fixes remembered**: each fix you apply from Diagnostics is noted, and the sessions after it say whether the mod's error stopped.
- **Before playing**: when the last sessions with nearly the same list did not even reach the menu or ran out of memory, the app says so when you press Play, with what differs from the last one that got into a save, and asks whether to play anyway.
- **A mod's history**: in the Mod List, the **On this PC** section of the right panel.
- **Slower loading** and a **game update** that made several mods fail at once.
- **Real crashes**: when Java falls over it leaves a report (`hs_err_pid…log`) in the game's folder; the brain reads it and ties it to its session ("it fell over: the PC ran out of memory" or "a fatal Java error in that library"). Closing the game with X or Alt+F4 leaves none, so that never counts as a crash.

### Security

The **Security** tab lists what a mod could do beyond modding, read from its files: program files or scripts it ships, Java that can run programs or reach the internet, and Lua that calls what the game's own scripts never call. None of it is a verdict: many mods do these things for good reasons. Most rows are Info; a Warning marks a pattern no mod has a good reason for. A Workshop mod's row has **Open Workshop page**.

### Time per mod

In **Mods** > **Time per mod**: which mods slow the game down while you play. **Measure** puts a small Studio mod (PZ Mod Studio Profiler) first in your mod list and in the save you played last; start the game (restart it if it was open) and play a few minutes as usual. Every event handler of every mod is timed by the game and added up per mod: the share of real time it took, milliseconds per second, and its longest single call (a long one is a freeze you see). Open a row for the handlers that cost the most, with their file and line. **Stop measuring** takes the profiler out of the list and the save again; the last numbers stay.

The windows, bars and timed actions of a mod count too (as Class:render, Class:update); each function counts only its own time. The profiler adds a little time of its own to every handler, so stop it when you are done.

## Game

What the app knows about the game itself, apart from the mods. Five tabs: **Live**, **Memory**, **Start-up**, **Frames** and **Disk**.

### Live

What the game uses now: at the top, the PC's free memory (the meter in the top bar leads to this tab). Below, a card per open game: memory against its Java limit, CPU, which screen it is on (menu, connecting, in game), the last line of its log, how many errors it has, and buttons to open its folder and its log. With the game closed it shows what is known from last time: the last start, the memory limit, sessions that ran out of memory and the times Windows ran out of memory.

### Memory

In **Game** > **Memory**: how much memory the game's Java may use (`-Xmx` in `ProjectZomboid64.json`), out of the PC's, with a suggestion for your active mods and a warning when a recent session ran out of memory. Pick a value (2 GB up to half the PC's memory) and **Apply** with the game closed; it takes effect on the next start, and a copy of the file before the change goes to `PZModStudio_Backups`.

Each profile keeps its own value: switching a profile on gives the game its memory. A profile without one uses the last value chosen. If a game update puts its own value back, the Studio restores yours the next time it starts the game.

When Windows itself ran out of memory with the game open in the last 14 days, it is listed here too, with what helps (a bigger page file, closing other big programs). That is not the game's Java memory, so raising the value above does not fix it.

### Start-up

In **Game** > **Start-up**: how long the game took to reach the main menu, stage by stage (mods' files, scripts, Lua, models and so on), from the log every launch leaves. Each start is set beside your usual start with about as many mods; a stage in amber took much longer than usual. Pick an earlier start to see it.

### Frames and hitches

In **Game** > **Frames**: frame times while you play (fps, frames over 25, 50 and 100 ms), chunks loaded and, per mod, the handlers that held a frame the longest, with file and line. The same profiler as **Mods** > **Time per mod** counts them, so press **Measure** there first and play a few minutes. **Read again** reads the newest numbers.

### Disk

At the top, **Free up space**: **Clear cache** deletes what the Studio keeps so it does not read or ask for the same thing again (each mod's checks, what Steam said, map pictures, translations); it builds it again by itself and never touches mods, saves, profiles or notes. After a multiplayer test, **Player 2's folder** sends the second game's `Zomboid2` folder (with copies of the server's mods) to the Recycle Bin; mods in development linked there are not touched, and the next test makes it again. Not while the test runs.

In **Game** > **Disk**: how much each installed mod takes, counted by what goes as one (a Workshop item with all its mods, a folder in `Zomboid/mods`). Measuring takes a couple of seconds.

- **Nothing uses it** (amber): no profile has it, on or off; no save names it in its list; no kept server loaded it; it is not in the game's active list.
- **Off everywhere** (light blue): some profile keeps it, switched off, and no save or server uses it.
- **Leftover**: a Workshop folder of an item you are no longer subscribed to; Steam sometimes leaves them.
- **Your upload**: a mod of yours in `Zomboid/Workshop`. Shown, never offered for removal.

Tick the ones you want gone. A Workshop item goes with **Unsubscribe from N items** (Steam must be running, and deletes the files itself). A local folder or a leftover goes with **Send N folders to the Recycle Bin**: it can be restored from the Recycle Bin. Each one asks first, with the names. Nothing goes to the Recycle Bin while the game is open.

Below, **Game and Studio folders**: how much the game, the Workshop mods, local mods, saves, save backups, kept mod versions, mod list backups, the cache, half-installed downloads and the logs take, with a button to open each folder. Nothing is deleted from there.

## Server

The server running on this PC (the one you start from Servers, or a multiplayer test's), with the same card: memory against its limit, CPU, whether it has started, the last line of its log and how many errors it has. During a multiplayer test its players show too. With no server running, a button leads to **Servers** to start one.

A server on another PC that you join cannot be measured from here: only the one running on this PC.

## Good to know

- Fixes in **Mods** are saved to the active profile's mod list.
- Merged files and shim patches take effect the next time the game starts.
- "Fills the console" is measured from the last session's `console.txt`, so it updates after you play.
- Clicking a problem chip in the Mod List opens this screen on that row.
