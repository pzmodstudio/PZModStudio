# Mod List

Your installed mods: which ones are on and the order the game loads them in. Every change here is saved at once into the active profile and into `Zomboid/mods/default.txt`, the list the game reads.

## What you see

- The header counts Workshop items, mods, mods **active**, and map, library and script mods. Red and amber counters (**Dependencies missing**, **Dependencies off**, **Incompatibility**, **List Order**) appear when there is a problem; click one to jump through those mods.
- The toolbar: search, filters, groups, and the **Enable All** / **Disable All** buttons.
- The list. The green bar on top (**LOADS FIRST**) is where libraries go; the amber bar at the bottom (**LOADS LAST**) is where a mod wins any file two mods share.
- On the right, the inspector for the selected mod: its details, what it requires, and warnings with buttons (**Move After ...**, **Activate Library**, **Deactivate**).

## Support the author

The inspector shows **Support the author** with the links its author put in the mod's changelog or in its Workshop description (Ko-fi, Patreon, Buy Me a Coffee...), opened in your browser, and, when the mod ships a changelog, **Latest in its changelog**, which opens its newest entry. The same buttons are on a Workshop item's page in Browse, installed or not.

## The # column

The number on each row is the mod's place among the mods that are **on**: the load order the game reads, and the same number the game shows. A mod that is off shows **—** and the **OFF** tag, because it has no place in the load order. Off mods keep their spot in the list, so switching one on usually puts it back where it was.

## Turning mods on and off

- Click the square on a row, or right-click and choose **Switch on** / **Switch off**.
- When a mod needs other mods, a window lists what comes on with it (and what goes off with it). Choose **Switch on all N**, or **Only ...** for that one mod alone.
- **Enable All** and **Disable All** act only on the rows shown now (after search and filters), and ask first.
- **Undo** / **Redo** (the arrows next to them, or Ctrl+Z / Ctrl+Y) step back and forward through the changes made here: moves, drags, switches, sorting. Each profile keeps its own, also after closing the app.
- Ctrl+click picks several mods, Shift+click picks a range, Esc lets them go. Right-click on the picked ones: switch them on or off, take them out of the profile, or unsubscribe from their Workshop items.
- An open draft package from the Mod Merger cannot be switched on until it is packaged.

## Changing the load order

- Drag a row by its number and drop it above or below another row. Hidden rows in between are respected.
- The arrow buttons on the right: move to top, up one, down one, move to bottom. Up and down swap with the next mod in the full list, which may be a mod that is off.
- **Pos #**: type a number and press **Go** or Enter. The mod goes to that place among the mods that are on.
- **Auto-Sort Dependencies**: puts each mod after what it requires and its `loadModAfter` mods, before its `loadModBefore` mods, and after the mods your own rules say. Everything else keeps your order; PZ Mod Studio packages go last. A window says how many mods moved, with **Undo**.

## Finding mods

- The search box matches the name, the mod id, the Workshop id and your notes.
- The filter buttons: **All**, **On**, **Off**, **Favourites**, **Updated**, **Problems**, **Maps**, **Libraries**, **Can't load**. Each shows its count.
- Below them: **Sort: load order**, **Sort: name**, **Sort: newest installs** or **Sort: oldest installs** (by when each mod was installed: when its folder was made; for a Workshop one, the first download, which updates do not change; each row shows the date and mods not installed go last), the **Groups** menu, one of your groups, the source (**Steam Workshop**, **Your Workshop uploads**, **Installed by hand**, **Compatible copies**...), a tag (errors or warnings last session, slow to load, heavy on the game, a Mod Health problem) and **Profile's mods** / **All installed**.
- The game's Mods screen (Studio Link mod) has the same filters in the same order. Updated, Libraries, the group, the source and the tag need PZ Mod Studio; without it the game says so where they would be.
- Dragging works sorted by name too: the mod goes to the place of the one you drop it on.
- **clear filters** shows every mod again.

## Groups and notes

- The **Groups** menu lays the list out in folding sections: **Groups: none**, **Groups: author**, **Groups: category** or **Groups: mine**. Each heading shows how many are on, with **All on** / **All off**. In **Groups: mine**, a note button edits the group's note.
- Right-click a mod and choose **Groups…** to tick the groups it is in, create a new one (**Add to it**), rename or delete one. A group left empty goes away. Groups never change the load order.
- Picking one of your groups shows **Turn the group on** (with what its mods require) and **Turn the group off**.
- Right-click and choose **Add a note…** to write a note on a mod. A note icon shows on its row.
- Each profile has its own groups: a group belongs to the profile you made it in. Notes are about a mod and the same in every profile.
- Notes and groups are yours only. The game shows them (the active profile's groups) on the Studio Link mod's Mods screen.

## The profile's mods

With a profile active, the list shows that profile's mods. The **Profile's mods** button switches to **All installed**, which lists every installed mod and tags each with the profiles it is in. Installed mods that are in no profile (subscribed from the Steam website, for example) get a blue bar with **Add to the profile** (they join switched off), **Show them**, or the X to hide it.

## The right-click menu

**Switch on/off**, **Add to the profile** / **Take out of the profile** (taken out also switches it off), **Move to the top**, **Move to the bottom**, **Open in Mod Browser** (**Search for it in Mod Browser** for a mod that is not installed), **Open its folder**, **Add a note…** (**Edit the note…** once it has one), **Groups…**, **Load-order rules…**, **Keep this version**, **Kept versions…**, **Favorite**, **Add to a list…**, **Add to a profile…**, **Copy the mod id** and **Unsubscribe from its Workshop item** (Steam deletes its files; other mods in the same item go too).

- **Load-order rules…**: say a mod must load after another (things you found by playing). Auto-Sort follows the rule and Mod Health warns when the list breaks it. **Share this rule** offers it to the community on GitHub.
- **Keep this version** copies the mod as it is today to `Zomboid/PZModStudio_Snapshots`. **Kept versions…** lists the copies: **Use this version** puts one back as a local copy the game reads instead of the Workshop one.

## Row tags

Tags on a row come from the last session and Mod Health: errors or warnings the game logged for the mod, how long it took to load (from 250 ms), its share of the game's time when you measured it in Mod Health (from 1 %, amber from 5 %), and problems such as **Needs a mod that is off** or **Loads too early**. Click a tag to go to the related mod or to Mod Health; the wrench button next to it applies the fix.

## Earlier lists and presets

- **Earlier lists** is undo for the list. Each time the Studio wrote it, the previous one was kept. Pick one and press **Go back to this**; your current list is kept too.
- The **Presets** button saves the list as a named preset, loads one, or exports and imports `.pzpack` files. An imported pack lists the mods you lack, with **Subscribe to all**.

## Good to know

- **Refresh** scans the mod folders again after you subscribe or unsubscribe.
- The PZ Mod Studio Link mod stays on in every profile and is hidden from the list while a profile is active.
- While the game runs, changes made here reach it on the main menu (through the Studio Link mod), or with **Sync with PZ Mod Studio** on its Mods screen.
