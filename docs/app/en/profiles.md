# Profiles

A profile is a named mod list: which mods belong to it, which of them are on, and their load order. Here you make profiles, switch between them with one click, share them and delete them. Most other screens stay locked until a profile is active.

## What you see

- At the top, how many profiles you have saved and the **New profile** button, which opens a window that walks you through making one in three steps.
- One card per saved profile, the active one first. The active one has a green frame, the **Active** tag and the **See its mods** button, which opens the Mod List; the others say **Inactive** and have **Activate (1-Click)**.
- Each card shows its description, how many mods are on (**Active Mods**), the **Total** of mods in its order, and the build: **Build 41/42** when you set it by hand, or **Played on ...** with the build it was last played on from the Studio.

## Creating a profile

Press **New profile**: a window opens with three steps (**Name**, **Which mods**, **Check**), each with an explanation at the top. **Next** and **Back** move between them; Esc or the X closes it without making anything.

1. Type a **Profile Name** (required) and, if you like, a **Description (Optional)**.
2. Under **Which mods**, choose what the profile starts with:
   - **What is on now**: the mods switched on right now.
   - **Everything installed**: every mod the Studio found.
   - **Pick them**: a list with a search box and **All** / **None** buttons; tick the mods you want.
   - **Nothing yet**: an empty profile. The Studio asks you to confirm, because switching to it turns every mod off. While a profile is active, a **Find mods in Browse Mods** button takes you there to subscribe first.
3. Under **Check** you see the name and how many mods start switched on. Press **Create and switch on**.

The new profile is created and activated at once, and keeps the load order your list already has. If the Studio has not found any mods yet (game or Zomboid folder not set in App Settings), it refuses, so a profile does not switch all your mods off by mistake.

## Switching profiles

Press **Activate (1-Click)** on a card. The Studio then:

- writes that profile's mods into the list the game reads (`Zomboid/mods/default.txt`);
- marks it active and opens the Mod List with its order;
- installs the PZ Mod Studio Link mod and keeps it on in every profile, so the game's Mods screen shows the profile.

If a mod of the profile needs a Java agent in the game's launch file (Zombie Buddy, for example), a window first shows what goes out of or back into that file. Press **Switch on ...** to go ahead. If the game is running, that part waits until the game closes. A copy of the launch file is kept first.

## Isolate this profile

**Isolate this profile** keeps apart, for that profile:

- its single-player **saves**;
- the **game's options**: graphics, sound and keys;
- the **mods' settings**: what each mod saves in the Zomboid folder (its options, its settings, its data).

With the profile on the game uses only its own, and with another profile that one's (or the shared ones, when the other is not isolated). When you isolate it you choose which shared saves it takes along; the rest stay shared. The options and the mods' settings start as a **copy** of the ones you have now, so the profile starts just as before and the other profiles keep theirs.

The **mods themselves are the same for every profile**: they are not copied or moved, and there is nothing to install again. A mod (or part of a mod) that is on in one profile and off in another works as always. Servers' saves (`Saves/Multiplayer`), the game's logs, servers, and the app's and the Link's files are not separated either.

Switching profiles the app puts everything in its place with a rename, nothing copied, always with the game closed. What belongs to the other profiles waits in `Zomboid/PZModStudio_ProfileSaves`. If the PC goes off in the middle of a switch, the app finishes it the next time it opens.

**Stop isolating** puts its saves with the shared ones and brings the shared options and mod settings back; the profile's own are kept in `Zomboid/PZModStudio_Backups/ProfileSettings`, nothing overwritten. Deleting the profile does the same, so a save or a setting is never lost.

## Editing, sharing and deleting

- The pencil button (**Rename this profile**) opens **Edit Mod Profile**: change the name, the description and the **Game build**. Leave the build on **As last played** or set **Build 41** / **Build 42**. With a build set, the Studio asks before launching this profile on another build, even before it has been played. Press **Save Changes**.
- The share button saves the profile to a `.pzpack` file you can send to anyone. It holds only the mods that are on, in order, with their Workshop ids. Whoever opens it can subscribe to what they are missing.
- The trash button deletes the profile after a confirmation. If some of its Workshop items are used by no other profile, the confirmation is a window that lists them (**Only this profile uses**), ticked, with **Tick all** / **Tick none**. **Delete and unsubscribe** deletes the profile and unsubscribes from the ticked items (Steam must be running); **Delete profile only** keeps them. Items another profile uses, and local mods, are never touched. If the profile uses patch packages from the Mod Merger (`Z_PZModStudio_...`), a second question asks whether to delete them too; **Cancel** keeps them.

## Good to know

- A deleted profile is not lost: its file is moved to `Zomboid/PZModStudio_Backups/profiles`.
- Deleting the active profile leaves no profile active, and the game's Mods screen stops showing a profile.
- Changes you make in the Mod List (switching mods, reordering) are saved into the active profile right away.
- Changes accepted in the game's own Mods screen also come back into the active profile.
- A mod switched on becomes one of the profile's mods. Mods that belong to no profile are offered in the Mod List with **Add to the profile**.
- Saves never cross major builds (41 and 42), which is why a profile remembers its build.
