# Saves

Your Project Zomboid saves and their backups. The Studio backs a save up on its own at the moments a save is most at risk; here you see those backups, make one yourself, keep one for good, or put one back.

## Each profile's saves

Every profile has its own saves. A save belongs to the profile that was active when you last played it with the Studio open; a save played without it belongs to a profile only if that profile's mods are exactly the save's. A save is never moved to another profile.

- This screen shows the active profile's saves. **Show all** adds the other profiles' saves and those with no profile, each with a **Profile: <name>** or **No profile** chip.
- A server's save shows as its address (`ip:port`) with an **Online** chip, and belongs to the profile you joined that server with from **Servers** (the Link does not run on a server, so the Studio goes by its own list of servers).
- In the game, with the Studio Link mod: the **Load** screen lists the active profile's saves, with **Show all** beside the search box, and **Continue** loads the newest save of the active profile (it is hidden when the profile has none). Without the Studio, the game lists every save as usual.

## What you see

- At the top, **Open backups folder** opens the folder where backups go (`Zomboid/PZModStudio_SaveBackups`), and the round arrow refreshes the screen.
- A bar with the room on the disk: how much is free, the disk's size, and how much the backups use. It turns amber with **Little room left** when your biggest save may not fit twice more.
- One row per save: its world name, then the game mode, size, number of files, number of mods and when it last changed.
- When the save's mods changed since its last session, an amber line says so: **Since its last session (...)**, with how many mods were added, taken out, updated, or that the order changed. Hover it to see the mod names.
- When the save's mod list names mods that are not installed, a red line says so: **Won't load: N mods in its list are not installed**, with a button per mod. Click one to open it in **Browse Mods** (the Workshop item when the list names it, otherwise a search for its id). Servers' saves keep no list of their own: the game downloads the server's mods when you join.
- On the right of each row, its state:
  - **Backed up as it is**: the newest backup matches the save.
  - **Changed since the backup of ...**: the save changed after its last backup.
  - **No backup**: there is none yet.
  - **Backing up...** while a backup runs.

## When the Studio makes a backup

- **When you left it**: when you go back to the main menu, or the game closes while you are in the save.
- **Before it loaded with changed mods**: the Studio Link mod asks for it when you load a save whose mods are not the ones of its last session, or were updated since.
- **Before its mod list was changed**: when the save's mod list is about to be changed in the game.
- **Before a restore**: the save as it was, right before you restore an older backup.

A save that has not changed since its newest backup is not zipped again.

## Making a backup yourself

Press **Back up now** on the row. The backup is a zip that is read back after it is written; checked backups show **checked**. If the save has not changed, no new zip is made: the newest backup is marked kept instead. The button is greyed out when the disk does not have room for the save plus some spare space.

## The backups of a save

Click the arrow at the left of a row to open its backups. Each one shows when it was made, why (**made by you**, **when you left it**, ...), the zip size, the save size, files and mods. For each backup:

- The pin keeps it: a kept backup is never removed to make room. Press it again to make it an automatic backup again.
- **Restore** puts that backup back as the save, after a confirmation.
- The trash button (**Remove this backup**) deletes it, after a confirmation. This cannot be undone.

## Restoring a save

1. Close Project Zomboid. A restore is refused while the game runs.
2. Open the save's backups and press **Restore** on the one you want.
3. Confirm. The Studio checks the backup, backs up the save as it is now (so you can undo), unpacks the backup next to the save and only then swaps them.

## Good to know

- The newest 5 automatic backups of each save are kept; older ones go when a new one is made. Backups you make or pin stay until you remove them.
- Automatic backups need PZ Mod Studio open while you play, and the Studio Link mod on in the game (it tells the app when you enter and leave a save).
- A server's save is backed up when the game closes with the Studio open, if it changed during that session.
- With the app closed, the Link mod warns before loading that the save cannot be backed up, and lets you go on without a backup.
- The Link mod can also play a save on a copy (`<world>_copy`), which the app makes; the original stays as it is.
