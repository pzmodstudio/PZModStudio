PZ Mod Studio - portable for Windows
====================================

The free mod manager for Project Zomboid (Build 42) that tells you which mod
broke your game. Nothing to install: this folder is the whole program.

Start
-----
Run Project-Zomboid-Mod-Studio.exe. It finds your game, Steam Workshop and
Zomboid folders by itself. Windows may show a SmartScreen notice the first
time, only because the program has no paid code-signing certificate.

The other file
--------------
pz-mcp-server.exe is a console program for AI assistants (Claude Desktop,
Cursor and others) to read your logs and mod list. A player can ignore it.

Good to know
------------
- The backup before a save loads with changed mods is made by the app, so
  keep it open (or in the tray) while you play.
- It never edits your Workshop mods; a save is only changed when you say yes,
  after a backup.
- To stop using it: delete this folder. What it made is listed in the
  README on https://github.com/pzmodstudio/PZModStudio ("How to stop using it").

Also for Linux (AppImage or .deb, beta): https://github.com/pzmodstudio/PZModStudio/releases/latest
Website: https://pzmodstudio.github.io/PZModStudio/

Needs Windows 10 or 11 (64-bit) with the WebView2 runtime (built into both).
