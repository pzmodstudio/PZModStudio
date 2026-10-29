# Profiles

A profile is a named mod list: which mods belong to it, which of them are on, and their load order. Here you make profiles, switch between them with one click, share them and delete them. Most other screens stay locked until a profile is active.

## What you see

- On the left, the **Create New Profile** form.
- On the right, one card per saved profile. The active one has a green frame and the **Active** tag; the others say **Inactive**.
- Each card shows its description, how many mods are on (**Active Mods**), the **Total** of mods in its order, and the build: **Build 41/42** when you set it by hand, or **Played on ...** with the build it was last played on from the Studio.
- The counter at the top right says how many profiles you have saved.

## Creating a profile

1. Type a **Profile Name** (required) and, if you like, a **Description (Optional)**.
2. Under **Which mods**, choose what the profile starts with:
   - **What is on now**: the mods switched on right now.
   - **Everything installed**: every mod the Studio found.
   - **Pick them**: a list with a search box and **All** / **None** buttons; tick the mods you want.
   - **Nothing yet**: an empty profile. The Studio asks you to confirm, because switching to it turns every mod off. A **Find mods in Browse Mods** button takes you there to subscribe first.
3. Press **Create New Profile**.

The new profile is created and activated at once, and keeps the load order your list already has. If the Studio has not found any mods yet (game or Zomboid folder not set in App Settings), it refuses, so a profile does not switch all your mods off by mistake.

## Switching profiles

Press **Activate (1-Click)** on a card. The Studio then:

- writes that profile's mods into the list the game reads (`Zomboid/mods/default.txt`);
- marks it active and opens the Mod List with its order;
- installs the PZ Mod Studio Link mod and keeps it on in every profile, so the game's Mods screen shows the profile.

If a mod of the profile needs a Java agent in the game's launch file (Zombie Buddy, for example), a window first shows what goes out of or back into that file. Press **Switch on ...** to go ahead. If the game is running, that part waits until the game closes. A copy of the launch file is kept first.

## Editing, sharing and deleting

- The pencil button (**Rename this profile**) opens **Edit Mod Profile**: change the name, the description and the **Game build**. Leave the build on **As last played** or set **Build 41** / **Build 42**. With a build set, the Studio asks before launching this profile on another build, even before it has been played. Press **Save Changes**.
- The share button saves the profile to a `.pzpack` file you can send to anyone. It holds only the mods that are on, in order, with their Workshop ids. Whoever opens it can subscribe to what they are missing.
- The trash button deletes the profile after a confirmation. If the profile uses patch packages from the Mod Merger (`Z_PZModStudio_...`), a second question asks whether to delete them too; **Cancel** keeps them.

## Good to know

- A deleted profile is not lost: its file is moved to `Zomboid/PZModStudio_Backups/profiles`.
- Deleting the active profile leaves no profile active, and the game's Mods screen stops showing a profile.
- Changes you make in the Mod List (switching mods, reordering) are saved into the active profile right away.
- Changes accepted in the game's own Mods screen also come back into the active profile.
- A mod switched on becomes one of the profile's mods. Mods that belong to no profile are offered in the Mod List with **Add to the profile**.
- Saves never cross major builds (41 and 42), which is why a profile remembers its build.
