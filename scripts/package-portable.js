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

const readmeContent = `=============================================================================
  🧟 PROJECT ZOMBOID MOD STUDIO — PORTABLE EDITION (STANDALONE) 🧟
=============================================================================

Welcome to Project Zomboid Mod Studio!

This version is 100% PORTABLE: no prior installation or configuration required.

-----------------------------------------------------------------------------
🚀 USAGE INSTRUCTIONS:
-----------------------------------------------------------------------------
1. Run the executable file directly:
   👉 "Project-Zomboid-Mod-Studio.exe"

2. It will not open extra console (CMD) windows and features a built-in
   initial cinematic splash screen.

3. On launch, the application will automatically detect your Project Zomboid,
   Steam Workshop, and mod profile directories.

-----------------------------------------------------------------------------
🤖 AI AGENT INTEGRATION (MCP SERVER):
-----------------------------------------------------------------------------
If you use Claude Desktop, Antigravity, Cursor, or Windsurf, the included
"pz-mcp-server.exe" binary allows your AI assistant to control the game,
inspect live logs, sort load orders, and generate compatibility patches.

-----------------------------------------------------------------------------
System Requirements:
- Windows 10 / Windows 11 (64-bit)
- WebView2 Runtime (built-in by default on Windows 10/11)
=============================================================================
`;

fs.writeFileSync(path.join(rootDir, 'README_PORTABLE.txt'), readmeContent, 'utf-8');
const oldReadme = path.join(rootDir, 'LEEME_PORTABLE.txt');
if (fs.existsSync(oldReadme)) {
  fs.unlinkSync(oldReadme);
}
console.log(`[Packaging] Generated ${path.join(rootDir, 'README_PORTABLE.txt')}`);
console.log(`[Packaging] ✅ Portable executables created successfully in root directory: ${rootDir}`);
