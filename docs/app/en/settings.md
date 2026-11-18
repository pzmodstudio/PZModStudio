# App Settings

App Settings is where you tell the Studio where your game and mods live, and choose how the Studio behaves: language, window, warnings, how it starts, and a few optional extras. Every change is saved as soon as you make it.

## Simple and Complete

The card at the top, **How much of the Studio to show**, picks one of two views. The same switch is always at the foot of the left bar, under **View**. Changing it never deletes or switches off anything: it only shows or hides screens, and you can go back and forth whenever you like.

- **Simple** shows what you need to play with mods:
  - **Home**: the profile in use, **Play**, **Continue** (your last game with this profile, once you have one) and the check-up in one line: green when there is nothing to do, red with **See what to do** when something needs fixing first.
  - **Profiles**, **Mod List**, **Browse Mods**, **Saves** and **Diagnostics**.
  - The **Mod List** lets you switch mods on and off, order them (by hand or with **Auto-Sort**), search, and filter by on, off and with a problem. Each mod is marked in red, in plain words, only for what needs fixing, with its button beside it. Groups, notes, presets, the other filters and load times stay in Complete.
  - **Diagnostics** opens straight on what you need to act on, each with one button. Optional suggestions (the game works without them) and what is only good to know or already fixed are counted under the list, not shown; **Show everything (Complete)** shows them. **Fix N problem(s) automatically** counts and fixes only the rows of the list you see: the hidden suggestions are left alone.
  - Here, Settings keeps the **Directory Paths**, **Alerts** and **The app** tabs.
- **Complete** is the whole Studio: every screen in the sidebar (Monitor Center, What's new, Maps, Mod Merger, Servers, Developer), every Diagnostics tab and every Settings tab.

A new install starts in **Simple**, with a three-step welcome: where your game was found, make or choose a profile, play. If the Studio was already used on this PC, it starts in **Complete**, as it always looked, and the first time a line at the top offers to try Simple: **Try Simple** or **No, thanks**. It shows once, and never if you already chose a view. The choice is remembered on this PC.

A link from another screen can still open a screen Simple hides (the Doctor's steps, for example); it shows in the sidebar while it is open.

## What you see

The screen has tabs at the top, side by side:

- **Directory Paths**: where the game, the Workshop, your Zomboid folder and the mod list are.
- **Alerts**: which warnings the Studio shows you.
- **The app**: how the Studio starts, its window and close button, the language, remembering how the game starts, the game's Mods screen, checking for new versions and translation.
- **Advanced**: the app's diagnostics, developer features and, with them, the Java mods you allowed in ZombieBuddy.
- **Build 42 fixes**: what the Build 42 compatibility shim covers, with a switch per mod it patches.
- **MCP Agent Server**: only shown with **Developer features** on.

## Folders

Press **Auto-Detect Paths** to let the Studio find everything by itself. To set a folder by hand, press **Browse...** next to it. A **Saved.** note confirms the change.

- **Project Zomboid Installation Directory**: the folder with `ProjectZomboid64.exe`. It shows **Found the game** when the folder is right, and **The Studio cannot find the game here** when it is not. Steam, GOG and manual installs all work.
- **Steam Workshop Folder (Optional if not using Steam)**: where Steam downloads Workshop mods (`workshop/content/108600`). Leave it empty if you play on GOG or put mods in `Zomboid/mods` by hand.
- **User Zomboid Directory (Data and Local Mods Folder)**: your `C:\Users\<User>\Zomboid` folder, with logs, saves, profiles and local mods. It is required.
- **Mod list file (Zomboid/mods/default.txt)**: the file the game reads to know which mods are on and in what order. It is found automatically; **Browse...** here picks a file, not a folder.

Without a valid game folder, screens such as the Mod Merger ask you to come here first.

## Warnings

Under **System Notifications and Warnings** you can silence or bring back three messages:

- **Warning at launch about unmerged mod conflicts**: the warning before launching when your active mods share files that no active merge package merges (or merges from older versions of the mods).
- **Notice at launch about mods that define the same things**: the notice before launching when two active mods define the same item, recipe or text differently and no package decides which stays. Nothing breaks: the one loading last wins.
- **Auto-Sort Dependencies Explanatory Message**: the explanation shown when you sort dependencies in the Mod List.

Each has **Silence Warning** / **Restore Warning** (or **Silence Notification** / **Restore Notification**).

## How the Studio starts and its window

- **What should open when the Studio starts?**: **Profiles Screen (Start)** (recommended) shows the profile manager; **Open Last Profile Directly** loads the last active profile and opens the Mod List.
- **Open the window as I left it**: same size, place and monitor, maximized or fullscreen if it was. If that monitor is gone, it opens on the main one.
- **The close button (X)**: **Closes the Studio** quits it (the game keeps running if it is open). **Keeps it in the tray, next to the clock** hides the window and the Studio keeps watching the game (backups when you exit, the profile in the game's Mods screen). Click the tray icon to open it again; its menu has **Quit**.

## Language

Pick the app's language under **Language**: English, Español, Português (Brasil), Français, Deutsch, Русский or 简体中文. A line missing in a language shows in English. The game's own messages (errors, warnings, the Monitor's log lines) stay as the game writes them.

## Game start and the game's Mods screen

- **Remember how the game starts**: on, the Launch button starts on the mode you used last time (Debug, Windowed, Normal...). Off, it always starts on Debug Mode (Fullscreen), the recommended one.
- **The game's Mods screen, with another mod manager on**: only one mod manager can draw the game's Mods screen. When another one is on, choose **Ask me in the game**, **PZ Mod Studio's (asks again if the other one is turned on)** or **The other manager's**. Save backups and the check before a save loads stay with PZ Mod Studio either way.

## App updates

- **Check for new versions of PZ Mod Studio**: at start and every 6 hours while it is open, the Studio asks GitHub which is the latest release. When there is a newer one, a bar under the header says so. **Update now** downloads the new version, checks the project signed it (the signature of `SHA256SUMS.txt` and the zip's SHA-256), puts it in place of yours and restarts the Studio; if anything does not match, nothing changes. **Download** opens the release page in your browser, and **Not now** hides the notice until the next version. When the Studio is installed in a folder it cannot write to (Program Files, for every user), update it with the installer from the page. Nothing is installed unless you ask, and nothing is sent. **Check now** asks at once. When GitHub does not answer, the Studio asks its itch.io page instead; then the notice offers only the download page, not **Update now**. Off, GitHub is not asked.

## Earlier versions of mods

- **Keep earlier versions of mods** (the app asks the first time, saying how much space it would take and on which drive; nothing is kept before you answer): Steam keeps no old versions of a Workshop mod. The Studio keeps a copy of the installed version of every Workshop mod a profile uses, in `Zomboid/PZModStudio_Versions`, before Steam replaces it; the Mod List's **Versions…** puts one back. **Up to** sets the space it may take (10 GB at first; a limit you chose stays as you chose it); each file is kept once, so a version after an update costs only what changed. Past the limit, the oldest earlier versions go first, then copies of mods not in the active profile; versions kept by hand stay. Whatever the limit, the Studio never fills the drive: a round always leaves 15% of the drive's size free, and when that is what holds the store back the card says **Limited by free space on D:** (with the limit that applies for now). **Folder** shows where the copies are and how much they take; **Choose folder…** moves them, for example to another drive: every file is copied and checked against the original, and only then is the old folder removed (on the same drive it is just renamed). A folder that already has other files gets a `PZModStudio_Versions` folder inside it. Moving is refused while the game runs or a round is running, and for the root of a drive, Steam's Workshop folder, the game's folder, `Zomboid/mods` and `Zomboid/Saves`. The line below says how much is used and how many of your Workshop mods are covered; mods left without a copy for lack of room are named. **Keep now** runs a round at once (it also runs by itself a little after the Studio starts and whenever Steam updates something). Nothing is done while the game runs, and the Workshop folder is only read.

## Diagnostics

**Diagnostics** collects what a bug report needs: the Studio's and Windows' versions, the folders, the game build, the active profile, the Studio Link, Zombie Buddy, whether Steam is limiting requests, and the end of the Studio's log. **Copy diagnostics** copies it; your Windows user name and PC name are replaced with `<user>` and `<pc>`. Look before you paste it anywhere. **Open the logs folder** opens the folder with the Studio's own logs.

## Translation

Browse Mods has a Translate button on a mod's page and one for the summaries of a page of results; the Developer screen's translation drafts use the same service. Without a key, the free Google Translate service is used (it is unofficial and may stop one day). With a DeepL API key (free up to 500,000 characters a month), DeepL is used first: press **Get a DeepL key**, paste it and press **Check and save**. The key is kept in Windows Credential Manager on this PC only; **Remove** deletes it. Translations already made are kept on this PC, so a text is not sent again; **Delete them** clears them.

## Developer features

**Developer features** shows the Developer screen in the sidebar, the **MCP Agent Server** tab (which lets an AI assistant drive the Studio) and the companion mod that runs commands inside the game. A player does not need it; leave it off unless you are building with it. The choice is remembered on this PC.

## Build 42 fixes

This tab explains the Build 42 compatibility shim: one file the Studio writes into every merge package it builds, which patches calls that Build 41 mods make to things Build 42 renamed or removed. Parts 1 to 6 always apply while a merge package is active. Part 7 patches particular mods, each with an **ON** / **OFF** switch. A switch is written into every Studio package on disk at once; no publish is needed, and the game reads it on its next launch.

## MCP Agent Server

Only with **Developer features** on. Pick your AI client, press **Copy MCP JSON** and paste it into that client's configuration file; step-by-step notes for each client are shown below. Do not run `pz-mcp-server.exe` by double-clicking it: a blank black window is normal, and your AI client starts and stops it by itself.

## Good to know

- Folder, language, window and warning choices apply right away; nothing needs a restart except the shim switches, which the game reads on its next launch.

## On Linux

The Linux version (AppImage or .deb) finds Steam in its usual folders (`~/.local/share/Steam`, `~/.steam`, the Flatpak's) and the Zomboid folder in your home folder. Some things work only on Windows: updating the app in place (on Linux **Download** opens the release page), moving and capturing the game's window for an AI assistant (park, screenshots, clicks, keys), the Windows out-of-memory log, subscribing through the Steam client, and downloading SteamCMD (install it from your distribution). Saved keys and passwords are kept in a private file, `~/.config/PZModStudio/secrets.json`, instead of a credential store. For an AI client, the console server is `/usr/bin/pz-mcp-server` with the .deb; the AppImage serves MCP itself with `--mcp`.
