import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

const releaseDir = path.join(rootDir, 'src-tauri', 'target', 'release');

// Files to copy directly to repository root
const appExeSrc = path.join(releaseDir, 'pz-mod-studio.exe');
const appExeDest = path.join(rootDir, 'Project-Zomboid-Mod-Studio.exe');

const mcpExeSrc = path.join(releaseDir, 'pz-mcp-server.exe');
const mcpExeDest = path.join(rootDir, 'pz-mcp-server.exe');

if (fs.existsSync(appExeSrc)) {
  try {
    fs.copyFileSync(appExeSrc, appExeDest);
    console.log(`[Packaging] Copied ${appExeDest}`);
  } catch (err) {
    if (err.code === 'EBUSY') {
      console.warn(`[Packaging] Warning: ${appExeDest} is currently open/in use. Close the app to update the root executable.`);
    } else {
      console.error(`[Packaging] Error copying ${appExeDest}:`, err);
      process.exitCode = 1;
    }
  }
} else {
  console.error(`[Packaging] Error: ${appExeSrc} not found.`);
  process.exitCode = 1;
}

if (fs.existsSync(mcpExeSrc)) {
  try {
    fs.copyFileSync(mcpExeSrc, mcpExeDest);
    console.log(`[Packaging] Copied ${mcpExeDest}`);
  } catch (err) {
    if (err.code === 'EBUSY') {
      console.warn(`[Packaging] Warning: ${mcpExeDest} is currently in use.`);
    } else {
      console.error(`[Packaging] Error copying ${mcpExeDest}:`, err);
      process.exitCode = 1;
    }
  }
} else {
  console.error(`[Packaging] Error: ${mcpExeSrc} not found. Build it with: cargo build --release --bin pz-mcp-server`);
  process.exitCode = 1;
}

const readmeContent = `PZ Mod Studio - portable for Windows
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
`;

fs.writeFileSync(path.join(rootDir, 'README_PORTABLE.txt'), readmeContent, 'utf-8');
const oldReadme = path.join(rootDir, 'LEEME_PORTABLE.txt');
if (fs.existsSync(oldReadme)) {
  fs.unlinkSync(oldReadme);
}
console.log(`[Packaging] Generated ${path.join(rootDir, 'README_PORTABLE.txt')}`);
console.log(`[Packaging] ✅ Portable executables created successfully in root directory: ${rootDir}`);
