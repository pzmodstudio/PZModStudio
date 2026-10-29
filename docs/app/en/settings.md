# App Settings

App Settings is where you tell the Studio where your game and mods live, and choose how the Studio behaves: language, window, warnings, how it starts, and a few optional extras. Every change is saved as soon as you make it.

## What you see

The screen has tabs at the top right:

- **Directory Paths**: folders and every general preference (most of this page).
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

Under **System Notifications and Warnings** you can silence or bring back two messages:

- **Warning when launching game without active merge package**: the warning before launching when no merge package is active.
- **Auto-Sort Dependencies Explanatory Message**: the explanation shown when you sort dependencies in the Mod List.

Each has **Silence Warning** / **Restore Warning** (or **Silence Notification** / **Restore Notification**).

## How the Studio starts and its window

- **What should open when the Studio starts?**: **Profiles Screen (Start)** (recommended) shows the profile manager; **Open Last Profile Directly** loads the last active profile and opens the Mod List.
- **Open the window as I left it**: same size, place and monitor, maximized or fullscreen if it was. If that monitor is gone, it opens on the main one.
- **The close button (X)**: **Closes the Studio** quits it (the game keeps running if it is open). **Keeps it in the tray, next to the clock** hides the window and the Studio keeps watching the game (backups when you exit, the profile in the game's Mods screen). Click the tray icon to open it again; its menu has **Quit**.

## Language

Pick **English** or **Español** under **Language**. A line missing in a language shows in English. The game's own messages (errors, warnings, the Monitor's log lines) stay as the game writes them.

## Game start and the game's Mods screen

- **Remember how the game starts**: on, the Launch button starts on the mode you used last time (Debug, Windowed, Normal...). Off, it always starts on Debug Mode (Fullscreen), the recommended one.
- **The game's Mods screen, with another mod manager on**: only one mod manager can draw the game's Mods screen. When another one is on (DMM, PZMM...), choose **Ask me in the game**, **PZ Mod Studio's (asks again if the other one is turned on)** or **The other manager's**. Save backups and the check before a save loads stay with PZ Mod Studio either way.

## Community files and GitHub token

- **Download the community rules and mod index**: once a day the Studio fetches load-order rules, incompatible pairs and a registry per mod from its own GitHub repository. Auto-Sort, Mod Health, the Mod List and Browse Mods use them. Off, the last copy is kept and nothing is fetched. Nothing is ever sent. Its two buttons open the index's web page and its community lists.
- **Check for new versions of PZ Mod Studio**: at start and every 6 hours while it is open, the Studio asks GitHub which is the latest release. When there is a newer one, a bar under the header says so. **Update now** downloads the new version, checks the project signed it (the signature of `SHA256SUMS.txt` and the zip's SHA-256), puts it in place of yours and restarts the Studio; if anything does not match, nothing changes. **Download** opens the release page in your browser, and **Not now** hides the notice until the next version. When the Studio is installed in a folder it cannot write to (Program Files, for every user), update it with the installer from the page. Nothing is installed unless you ask, and nothing is sent. **Check now** asks at once. Off, GitHub is not asked.
- **GitHub, for "Propose" in one click**: optional. Connected, the "Propose to the community" buttons (rules, a mod's registry, a list) open the pull request for you. Without it, they open GitHub's page with everything filled in. Press **Connect with GitHub**: a code appears (already copied) and GitHub's page opens; type the code there and approve **PZ Mod Studio**, and the Studio is connected. Or press **Make a token**, make one on GitHub, paste it and press **Check and save**. The token is checked with GitHub and kept in Windows Credential Manager on this PC only; **Remove** deletes it.

## Diagnostics

**Diagnostics** collects what a bug report needs: the Studio's and Windows' versions, the folders, the game build, the active profile, the Studio Link, Zombie Buddy, whether Steam is limiting requests, and the end of the Studio's log. **Copy diagnostics** copies it; your Windows user name and PC name are replaced with `<user>` and `<pc>`. Look before you paste it anywhere. **Open the logs folder** opens the folder with the Studio's own logs.

## Developer features

**Developer features** shows the Developer screen in the sidebar, the **MCP Agent Server** tab (which lets an AI assistant drive the Studio) and the companion mod that runs commands inside the game. A player does not need it; leave it off unless you are building with it. The choice is remembered on this PC.

## Build 42 fixes

This tab explains the Build 42 compatibility shim: one file the Studio writes into every merge package it builds, which patches calls that Build 41 mods make to things Build 42 renamed or removed. Parts 1 to 6 always apply while a merge package is active. Part 7 patches particular mods, each with an **ON** / **OFF** switch. A switch is written into every Studio package on disk at once; no publish is needed, and the game reads it on its next launch.

## MCP Agent Server

Only with **Developer features** on. Pick your AI client, press **Copy MCP JSON** and paste it into that client's configuration file; step-by-step notes for each client are shown below. Do not run `pz-mcp-server.exe` by double-clicking it: a blank black window is normal, and your AI client starts and stops it by itself.

## Good to know

- Folder, language, window and warning choices apply right away; nothing needs a restart except the shim switches, which the game reads on its next launch.
- The GitHub token never leaves this PC except to talk to GitHub.
