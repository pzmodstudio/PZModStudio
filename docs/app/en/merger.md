# Mod Merger

The Mod Merger fixes files that two or more of your active mods both change, and lets you patch a bug in one mod's own file. Your decisions go into a package: a small mod of its own that loads last, so the game reads your version of those files.

## What a package is

- A package is a mod folder in `Zomboid/mods` whose name starts with `Z_PZModStudio_` (the default one is `Z_PZModStudio_MergedPatch`).
- When you publish it, the Studio writes your merged files into it (for Build 41 and in its `42` folder for Build 42), adds a small compatibility file, and puts the package **last** in the game's mod list. The mod that loads last is the one whose copy of a file the game uses, so your merged copy wins.
- A package has two states. **🟢 PACKAGED / CLOSED** means it is published and the game reads it. **🟡 OPEN (Draft)** means you are editing it; while it is a draft it is taken out of the game's mod list, so the game does not read it until you publish again.
- Each package belongs to the profiles that have it among their mods. A package you create while a profile is active is added to that profile.
- A package remembers the profile that was active when it was last published (**built with** on its card, in amber when another profile is active now): its merges settle that profile's list. With a profile that has no package of its own, **Package for "name"** in the top bar makes one named after it.

## What you see

When no package is open, you see the **Merge Packages** list. Each card shows the package name, its files, its state, the profiles it belongs to and these buttons:

- **🔍 Open / Enter Package** opens it in the workspace and scans your active mods for conflicts.
- **Visible in ModList** / **Hidden in ModList** turns the package on or off in the game's mod list (`default.txt`).
- The pencil edits its name and description; the bin deletes it from disk (you are asked first). Profiles that held it keep the new name after a rename.
- The download icon saves it as a `.pzmerge` file to share; the folder icon opens its folder in Windows Explorer.
- **Other profiles' packages** shows packages that belong to other profiles; **Only "name"** goes back to the active profile's.

The top bar has **Load Package (.pzmerge)** to import a package someone sent you, and **➕ New Package** to create one (a name, an optional description, then **Create Package**). The Studio's own mods (the Live IPC Bridge and the Studio Link) appear marked **System**: you can see their files but not edit, rename or delete them.

## Working inside a package

The top bar shows **← Back to List**, a **Package:** selector to jump to another package, the state and visibility, and these buttons:

- **Patch a mod file**: opens one mod's own file to fix it (see below).
- **⚡ Merge all and publish**: merges every conflicting file with the automatic engine and publishes the package at once: it is written to `Zomboid/mods` and put last in your mod list, and the game reads it on its next launch. Review the files afterwards.
- **↩️ Unmerge All**: throws away every saved merge in this package. A copy is kept in `Zomboid/PZModStudio_Backups`, and the files already published stay until you publish again.
- **All** / **No Duplicates**: hides files where the mods ship identical copies, so only real collisions are listed.
- Folder, export and rescan icons.
- **📦 Package & Publish**: the final step. It writes every file's current decision into the package and puts the package last in the game's mod list.

On the left, **VFS Conflicts** lists each colliding file with its kind (Lua, **DEFINITION** for item and recipe scripts, **WORDS** for translations) and its state: **Needs Review**, **Auto-Merged** or **Resolved**.

## Deciding what the game reads for one file

1. Click a file in the list. The top half shows the vanilla file, each mod's copy and a summary of what each mod changes compared to the base game; real conflicts (the same lines changed differently) are marked. Click a change to scroll every panel to it.
2. Under **What should the game read?**, pick one answer:
   - **Keep both** (for Lua and other files): every mod's changes together. Where two change the same lines, the one that loads last wins and the other is kept in a comment.
   - **Merge them** (for definitions and translations): what can hold both values keeps both; where only one fits, what the game does today stays and the other is left in a comment.
   - **Only (mod name)**: that mod's version. For a definition, the other mods' settings that this one does not touch still apply.
   - **Vanilla**: the game's own file, as if no mod had touched it (not offered when the base game has no such file).
3. Or edit the code in the editor below; it then shows **Your own edit**. **Append** and **Remove** on a mod's panel add or take out that mod's whole copy, and **Load Base** puts the vanilla file in the editor.
4. Press **✓ Save Merge of this File**. The file becomes **Resolved**.
5. When every file is done, press **📦 Package & Publish**.

**Restore AST Merge** puts the automatic merge back in the editor, and **Unmerge this File** forgets the saved decision for that file. Drag the bar between the panels to resize them.

## Patching a bug in one mod

Use **Patch a mod file** when one mod has a bug and there is nothing to merge. Pick the mod, then the Lua or script file, edit it and press **✓ Save Merge of this File**. Your copy goes in the package and the game reads it instead of the mod's, for that file only; the mod stays on and the rest of it is untouched. Saving a copy with no changes is refused, because it would only freeze the file at today's version.

The Monitor and Mod Health can send you here directly: the right package opens and the file the error named opens in the editor, sometimes with the fix already applied.

## Packaged packages and publishing in place

A packaged package is read-only: its files stay hidden behind a summary until you press **🔓 Unmerge / Open to Edit** (or **Unpack to Edit**) and confirm. Unpacking takes the published files out of the game until you publish again.

A mod's own file opened to patch is the exception: in a packaged package, its save **publishes in place**. Everything the package holds is published again with your file beside it, the package stays packaged, and the game reads the new file on its next launch. When Mod Health or the Monitor sends you to one file of a packaged package, that file is shown without unpacking it.

## Sharing packages

**Load Package (.pzmerge)** imports a package file. If it was made with mods you do not have, or with different copies of them, a **Before you publish** window lists them, with **Subscribe to all** for the missing Workshop mods. The safest fix is for both of you to have the newest copies, then rescan and merge again before publishing.

## Good to know

- The game reads a package when it starts: after publishing, relaunch the game if it is running.
- If the game folder is not found, the screen asks you to set the paths in **App Settings** first.
- When you leave a file with unsaved edits, you are asked whether to leave them behind.
- A package you delete is removed from disk; the mods it merged are not touched.
