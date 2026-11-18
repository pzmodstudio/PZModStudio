// Publishes the current version as a GitHub release, from this PC (no Actions): packs the portable
// zip from the root exes, adds the Linux files of this version from dist-local/linux, writes and
// signs SHA256SUMS.txt, then creates the release v<version> (or replaces the files of the one that
// exists) and uploads them.
//   node scripts/publish-release.cjs [--repo owner/name] [--notes "text"]
// The token is read from the file named by GH_TOKEN_FILE (default ~/.pz-github-token) and the
// signing key from RELEASE_SIGNING_KEY_FILE (default ~/pz-release-signing-key-2026.pem).
// Neither is printed or written anywhere.
const { execFileSync } = require('child_process');
const crypto = require('crypto');
const fs = require('fs');
const os = require('os');
const path = require('path');

const root = path.resolve(__dirname, '..');
const arg = (name, fallback) => { const i = process.argv.indexOf(name); return i > -1 ? process.argv[i + 1] : fallback; };
const repo = arg('--repo', 'pzmodstudio/PZModStudio');
const version = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version;
const tag = 'v' + version;
const read = (p) => fs.readFileSync(p, 'utf8').trim();
const token = read(process.env.GH_TOKEN_FILE || path.join(os.homedir(), '.pz-github-token'));
const keyPem = fs.readFileSync(process.env.RELEASE_SIGNING_KEY_FILE || path.join(os.homedir(), 'pz-release-signing-key-2026.pem'), 'utf8');

const out = path.join(root, 'dist-local', 'publish');
fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });

// 1. The portable zip: the two exes, the readme and the licence, as the release workflow made it.
const stage = path.join(root, 'dist-local', 'PZ-Mod-Studio-Portable');
fs.rmSync(stage, { recursive: true, force: true });
fs.mkdirSync(stage, { recursive: true });
for (const f of ['Project-Zomboid-Mod-Studio.exe', 'pz-mcp-server.exe', 'README_PORTABLE.txt', 'LICENSE']) fs.copyFileSync(path.join(root, f), path.join(stage, f));
const zipName = `PZ-Mod-Studio-Portable-${tag}.zip`;
execFileSync('powershell', ['-NoProfile', '-Command', `Compress-Archive -Path '${stage}\\*' -DestinationPath '${path.join(out, zipName)}' -Force`], { stdio: 'inherit' });

// 1b. The Linux files of this version (AppImage, .deb, console server), when scripts/build-linux.cjs
// made them in dist-local/linux; --no-linux leaves them out, --linux requires them.
const linuxDir = path.join(root, 'dist-local', 'linux');
const linuxFiles = process.argv.includes('--no-linux') || !fs.existsSync(linuxDir) ? [] : fs.readdirSync(linuxDir).filter((n) => n.includes(`-${tag}-linux`));
if (process.argv.includes('--linux') && linuxFiles.length < 3) { console.error(`The Linux files of ${tag} are not in ${linuxDir}: run node scripts/build-linux.cjs first.`); process.exit(1); }
for (const n of linuxFiles) fs.copyFileSync(path.join(linuxDir, n), path.join(out, n));
const appImage = linuxFiles.find((n) => n.endsWith('.AppImage'));
const deb = linuxFiles.find((n) => n.endsWith('.deb'));

// 2. The sums and the project's signature of them.
const sha = (f) => crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const sums = fs.readdirSync(out).sort().map((n) => `${sha(path.join(out, n))}  ${n}`).join('\n') + '\n';
fs.writeFileSync(path.join(out, 'SHA256SUMS.txt'), sums);
const key = crypto.createPrivateKey(keyPem);
fs.writeFileSync(path.join(out, 'SHA256SUMS.txt.sig'), crypto.sign(null, fs.readFileSync(path.join(out, 'SHA256SUMS.txt')), key).toString('base64') + '\n');
execFileSync('node', [path.join(__dirname, 'verify-release.cjs'), out], { stdio: 'inherit' });

// 3. The release.
const api = async (method, url, body, headers = {}) => {
  const res = await fetch(url, { method, headers: { Authorization: 'Bearer ' + token, Accept: 'application/vnd.github+json', 'User-Agent': 'pz-mod-studio-release', ...headers }, body });
  const text = await res.text();
  let json = null; try { json = JSON.parse(text); } catch (e) { /* not JSON */ }
  return { status: res.status, json, text };
};
const linuxNote = appImage ? `\n\n**Linux:** \`${appImage}\` (make it executable and run it) or \`${deb}\` (\`sudo apt install ./${deb}\`). The console MCP server for AI agents is in the .deb as \`/usr/bin/pz-mcp-server\`, and on its own as the \`-linux-x86_64\` file.` : '';
const notes = arg('--notes', `**Portable (Windows):** download \`${zipName}\`, extract it anywhere and run \`Project-Zomboid-Mod-Studio.exe\`.\nIt also contains \`pz-mcp-server.exe\` for AI agents (see AGENTS.md).${linuxNote}\n\nCheck a download with \`SHA256SUMS.txt\` and \`SHA256SUMS.txt.sig\` (signed with \`keys/release-signing.pub.pem\`; \`node scripts/verify-release.cjs <folder>\`).`);

(async () => {
  let rel = await api('GET', `https://api.github.com/repos/${repo}/releases/tags/${tag}`);
  if (rel.status === 200 && process.argv.includes('--recreate')) {
    // The tag is made from the branch head when the release is created, so a release made before the
    // sources were pushed has the tag on older code: drop both and make them again.
    await api('DELETE', `https://api.github.com/repos/${repo}/releases/${rel.json.id}`);
    await api('DELETE', `https://api.github.com/repos/${repo}/git/refs/tags/${tag}`);
    rel = { status: 404 };
  }
  if (rel.status === 404) {
    rel = await api('POST', `https://api.github.com/repos/${repo}/releases`, JSON.stringify({ tag_name: tag, target_commitish: 'main', name: 'PZ Mod Studio ' + tag, body: notes, draft: false, prerelease: false }));
    if (rel.status !== 201) { console.error('Could not create the release:', rel.status, rel.json && rel.json.message); process.exit(1); }
    console.log('Created release ' + tag);
  } else if (rel.status === 200) {
    await api('PATCH', `https://api.github.com/repos/${repo}/releases/${rel.json.id}`, JSON.stringify({ body: notes, name: 'PZ Mod Studio ' + tag }));
    for (const a of rel.json.assets || []) await api('DELETE', `https://api.github.com/repos/${repo}/releases/assets/${a.id}`);
    console.log('Updated release ' + tag + ' (old files removed)');
  } else { console.error('Could not read the release:', rel.status, rel.json && rel.json.message); process.exit(1); }

  for (const name of fs.readdirSync(out)) {
    const data = fs.readFileSync(path.join(out, name));
    const type = name.endsWith('.zip') ? 'application/zip' : /\.(txt|sig)$/.test(name) ? 'text/plain' : 'application/octet-stream';
    const up = await api('POST', `https://uploads.github.com/repos/${repo}/releases/${rel.json.id}/assets?name=${encodeURIComponent(name)}`, data, { 'Content-Type': type, 'Content-Length': String(data.length) });
    if (up.status !== 201) { console.error('Upload of ' + name + ' failed:', up.status, up.json && up.json.message); process.exit(1); }
    console.log('Uploaded ' + name + ' (' + (data.length / 1048576).toFixed(1) + ' MB)');
  }
  console.log(`https://github.com/${repo}/releases/tag/${tag}`);
})();
