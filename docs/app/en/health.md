# Mod Health

Mod Health checks your active mods the way the game reads them, and lists every problem it finds with a button to fix it. It looks at what your game build can load, missing or switched-off requirements, the load order, incompatible mods, files two mods both change, and mods that filled the console last time you played.

## What you see

- At the top: your **Game build** and how many mods are active.
- The **community** button (for example **community: 40 rules, 120 mods ↻**): what the shared community files add to the check. Click it to download them now. **Community index** beside it opens the index's web page: every mod's known issues and their fixes.
- **Check again** runs the check again. It also runs by itself when your list changes.
- Five tabs, each with how many rows it has:
  - **Problems**: what breaks the game or will (missing or switched-off requirements, load order, mods your build cannot load, incompatible pairs). **Fix N problem(s) automatically** is here.
  - **Conflicts**: two mods giving the same file, item, recipe or text; the game uses one, and which one is your call. **Merge N files automatically** is here.
  - **Good to know**: nothing to fix (ignored copies, mods updated since you last played, community notes).
  - **Performance**: game time per mod (below).
  - **Disk**: how much each mod takes and which ones can go (below).
- Each row shows the mod, what is wrong, a short code (like `missing_dependency`) and its fix buttons on the right. The screen remembers the last tab; a problem chip in the Mod List opens the tab that has it.

If nothing is wrong, you see **No problems found in your N active mods.**

## Severities

- **Errors**: the game will not load the mod, or it loads without something it needs. Fix these first.
- **Warnings**: the game loads, but something is likely to go wrong: wrong order, a file only one mod gets to use, an update not downloaded yet, a mod filling the console.
- **Info**: worth knowing, nothing is broken: two mods define the same thing, a compatible copy is in use, mods updated since you last played.

## The problems it finds

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
- `script_definition_conflict` and `translation_conflict`: two mods define the same item, recipe or name differently; the one that loads last wins. Rows ending in `_stale` mean you merged it before, but a mod changed since; `_settled` means your package already settles it.
- `floods_console` (shown in the Mod List as **Fills the console**): in your last session this mod wrote more than 1,000 lines of errors to the console. That slows the game and hides other mods' errors. Look at them in the Monitor Center, or switch the mod off.
- `user_rule_order`: one of your own load-order rules is broken.
- `community_rule_order`, `community_incompatible`, `community_known_issue`, `community_successor`: what the community files say about your mods.
- `package_built_for_other_list`: your Studio package was built with a different mod list than this profile.
- `compatible_copy`, `shadowed_copy`, `updated_since_last_play`: notes about which copy of a mod is used and what changed.

Rows about the same definition can be opened with **By file** to see each mod's file. Click a definition there to open it in the Merger.

## The fix buttons

- **Enable X** or **Disable X**: switches a mod on or off. Switching off asks first; the mod stays installed.
- **Move X before Y**: changes the load order. On a "your call" row (two mods define the same thing) it reads **Use Y's version instead**.
- **Remove X from the list**: takes an entry out of the list.
- **Search Workshop for "X"** and **Open Workshop page**: open the Steam Workshop in your browser.
- **Make compatible copy**: makes an adapted copy of a Workshop mod in `Zomboid/mods` for your build. The result shows what it changed and what may still break. Start the game from the Studio so it uses the copy. **Remove compatible copy** goes back to the original.
- **Keep the version you have**: saves the mod as it is today, before a Workshop update lands. You can put it back from the Mod List (right-click, Kept versions).
- **Merge automatically** and **Open in Merger**: settle a file or definition conflict in your patch package, which loads last.
- **Switch the shim's X patch on**: turns on a patch of the Build 42 shim that a known issue needs.

A green line confirms each change, with **Saved to your mod list.**

## Fixing many at once

- **Fix N problem(s) automatically** switches on missing requirements and moves mods earlier where needed. It never switches anything off and skips the "your call" rows. It asks first, and the Mod List can undo it.
- **Merge N files automatically** merges every conflicting file that has no overlapping changes into your patch package. Files where both mods change the same lines are listed for you to decide in the Merger.

## Community rules and index

The Studio downloads shared rules once a day from its own repository: load-order rules, pairs of mods that do not go together, and an index of known issues with their fix. They show up as `community_*` rows. You can turn the daily download off in App Settings; then the last copy is used.

## Game memory

In the **Performance** tab: how much memory the game's Java may use (`-Xmx` in `ProjectZomboid64.json`), out of the PC's, with a suggestion for your active mods and a warning when a recent session ran out of memory. Pick a value (2 GB up to half the PC's memory) and **Apply** with the game closed; it takes effect on the next start, and a copy of the file before the change goes to `PZModStudio_Backups`.

Each profile keeps its own value: switching a profile on gives the game its memory. A profile without one uses the last value chosen. If a game update puts its own value back, the Studio restores yours the next time it starts the game.

## Game time per mod

In the **Performance** tab: which mods slow the game down while you play. **Measure** puts a small Studio mod (PZ Mod Studio Profiler) first in your mod list and in the save you played last; start the game (restart it if it was open) and play a few minutes as usual. Every event handler of every mod is timed by the game and added up per mod: the share of real time it took, milliseconds per second, and its longest single call (a long one is a freeze you see). Open a row for the handlers that cost the most, with their file and line. **Stop measuring** takes the profiler out of the list and the save again; the last numbers stay.

The windows, bars and timed actions of a mod count too (as Class:render, Class:update); each function counts only its own time. The profiler adds a little time of its own to every handler, so stop it when you are done.

## Disk space

On the **Disk** tab: how much each installed mod takes, counted by what goes as one (a Workshop item with all its mods, a folder in `Zomboid/mods`). Measuring takes a couple of seconds.

- **Nothing uses it** (amber): no profile has it, on or off; no save names it in its list; no kept server loaded it; it is not in the game's active list.
- **Off everywhere** (light blue): some profile keeps it, switched off, and no save or server uses it.
- **Leftover**: a Workshop folder of an item you are no longer subscribed to; Steam sometimes leaves them.
- **Your upload**: a mod of yours in `Zomboid/Workshop`. Shown, never offered for removal.

Tick the ones you want gone. A Workshop item goes with **Unsubscribe** (Steam must be running, and deletes the files itself). A local folder or a leftover goes with **Send to the Recycle Bin**: it can be restored from the Recycle Bin. Each one asks first, with the names. Nothing goes to the Recycle Bin while the game is open.

Below, **Studio and game folders**: how much save backups, kept mod versions, the cache and the logs take, with a button to open each folder. Nothing is deleted from there.

## Good to know

- Changes here are saved to the active profile's mod list.
- Merged files and shim patches take effect the next time the game starts.
- "Fills the console" is measured from the last session's `console.txt`, so it updates after you play.
- Clicking a problem chip in the Mod List opens this screen on that row.
