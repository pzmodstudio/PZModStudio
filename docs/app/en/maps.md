# Maps

Maps shows the game's world and every installed map mod on one picture, so you can see which parts of Knox Country a map replaces and where two map mods fight over the same ground. It also lists the map mods that add no land of their own, and warns about tile clashes.

## What you see

- The real map of the game's world, with each map mod's cells drawn on top in its own colour.
- On the right, the list of map mods that add cells.
- Above the map, a warning for each pair of map mods that share cells or tile numbers.
- Below, **Other map mods**: spawn points, map lists and tile packs.

The world is cut into cells; one cell is 256 x 256 tiles.

## Moving around the map

- Turn the mouse **wheel** to zoom in and out. It zooms on the point under the pointer.
- **Hold the left button and drag** to move the map.
- **Double-click** the map, or press the button in its top-right corner (**See the whole map**), to see it all again.
- Rest the pointer on a cell to see its number, whether it is part of the game's world, and which maps cover it.

The picture is put together from the map images the game and the mods ship. While that happens you see **Putting the map together…**. A map mod that ships no picture still shows its cells as coloured squares.

## What the colours mean

- **The game's world**: the base map, with a faint grid over it.
- **Each map mod's colour**: its cells. The small square next to its name in the list shows the colour. A map mod's cells over the game's world replace what was there.
- **Red**: cells two map mods share. There, one of them replaces the other in your game, unless one is built on top of the other.

## The list of map mods

Each row shows the map's name, the mod it comes from, how many cells it has, how many are over the game's world (or **outside the game's world**), which maps it **builds on**, and **off** if the mod is switched off.

- Untick a row's box to hide that map from the picture. This only changes the view, not your mod list.
- Press the list icon on a row (**See it in the Mod List**) to jump to that mod in the Mod List, highlighted.
- Tick **Only map mods on in my list** to show only the maps you have switched on.
- The refresh button at the top reads the maps again. They are also read again when your mod list changes.

## Clashes

- A **red** warning means two map mods share cells and both are on: in your next game one replaces the other there. Under it, each mod has a button to see it in the Mod List and a **Turn off** button that switches it off (after asking).
- An **amber** warning means they would clash if both were on.
- A tile definition warning means two mods use the same tile definition number. If both are on, their tiles overwrite each other and you get wrong walls, floors or furniture. Keep only one on.

## Other map mods

These mods are about maps but add no cells: spawn points, a map list for another map, or the tiles maps are built from. Each one is tagged **spawn points / map list** or **tiles**. They need the map or mod they are for to be on. Click one to see it in the Mod List.

## Good to know

- Two maps that share cells do not clash when one names the other in its `map.info` (`lots=`); the list shows that as **builds on**.
- The map pictures are made once and kept in `Zomboid/PZModStudio_Cache/maps/`, so the screen opens faster next time.
- Nothing on this screen changes your game except **Turn off**.
