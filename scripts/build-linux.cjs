#!/usr/bin/env node
/**
 * Builds the Linux release (AppImage, .deb and the pz-mcp-server console binary) from this Windows
 * PC, inside WSL. The repository is copied to the distro's own disk first (building on /mnt is
 * slow), so nothing in this checkout changes but the output folder.
 *
 *   node scripts/build-linux.cjs [--out <folder>] [--distro Ubuntu-24.04] [--skip-tests] [--setup]
 *
 * --out        where the finished files go (default: dist-local/linux, which git ignores)
 * --distro     the WSL distro (default Ubuntu-24.04); commands run in it as root
 * --skip-tests do not run `cargo test --lib` before building
 * --setup      first install what the build needs in the distro: Tauri's Linux packages (apt),
 *              rustup (stable) and Node.js 22 (NodeSource). Downloads from the internet: only
 *              when asked.
 *
 * The distro keeps its copy in /root/pzms (node_modules and the Rust target are kept between
 * builds, so a second build takes a few minutes).
 */
const { spawnSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

const root = path.resolve(__dirname, '..');
const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const value = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
};
const distro = value('--distro', 'Ubuntu-24.04');
const outDir = path.resolve(value('--out', path.join(root, 'dist-local', 'linux')));
const version = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version;

/** C:\a\b -> /mnt/c/a/b */
const wslPath = (p) => {
  const abs = path.resolve(p);
  const m = abs.match(/^([A-Za-z]):[\\/](.*)$/);
  if (!m) throw new Error(`Not a Windows drive path: ${abs}`);
  return `/mnt/${m[1].toLowerCase()}/${m[2].replace(/\\/g, '/')}`;
};

/** Runs a bash script inside the distro as root, its output shown as it comes. */
const runInWsl = (script, what) => {
  const file = path.join(os.tmpdir(), `pzms-build-linux-${process.pid}.sh`);
  fs.writeFileSync(file, `set -euo pipefail\n${script}\n`.replace(/\r\n/g, '\n'));
  console.log(`\n[build-linux] ${what}`);
  const r = spawnSync('wsl', ['-d', distro, '-u', 'root', '--', 'bash', '-l', wslPath(file)], { stdio: 'inherit' });
  fs.rmSync(file, { force: true });
  if (r.status !== 0) {
    console.error(`[build-linux] Failed: ${what} (exit ${r.status}).`);
    process.exit(r.status || 1);
  }
};

if (process.platform !== 'win32') {
  console.error('[build-linux] Run this on Windows with WSL (on Linux itself: npm ci && npm run tauri build -- --bundles appimage,deb).');
  process.exit(1);
}

if (flag('--setup')) {
  runInWsl(
    `export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y -q build-essential curl wget file pkg-config rsync libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev librsvg2-dev libayatana-appindicator3-dev libxdo-dev
[ -x /root/.cargo/bin/cargo ] || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
command -v node >/dev/null && node -e 'process.exit(+process.versions.node.split(".")[0] < 22)' || { curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && apt-get install -y -q nodejs; }`,
    'Installing the build tools in the distro'
  );
}

const repo = wslPath(root);
runInWsl(
  `export PATH=/root/.cargo/bin:/usr/local/bin:/usr/bin:/bin
for tool in cargo node npm rsync pkg-config; do command -v $tool >/dev/null || { echo "$tool is missing in ${distro}: run again with --setup"; exit 1; }; done
pkg-config --exists webkit2gtk-4.1 || { echo "webkit2gtk-4.1 is missing in ${distro}: run again with --setup"; exit 1; }
mkdir -p /root/pzms
rsync -a --delete --exclude node_modules --exclude src-tauri/target --exclude .git --exclude dist --exclude dist-local --exclude publish-repo --exclude private --exclude '*.exe' "${repo}/" /root/pzms/
cd /root/pzms
npm ci --no-audit --no-fund
${flag('--skip-tests') ? '' : '(cd src-tauri && cargo test --lib --quiet)'}
npm run tauri build -- --bundles appimage,deb`,
  `Building ${version} in ${distro}`
);

fs.mkdirSync(outDir, { recursive: true });
const outWsl = wslPath(outDir);
const bundle = '/root/pzms/src-tauri/target/release';
runInWsl(
  `cp "${bundle}/bundle/appimage/Project Zomboid Mod Studio_${version}_amd64.AppImage" "${outWsl}/PZ-Mod-Studio-v${version}-linux-x86_64.AppImage"
cp "${bundle}/bundle/deb/Project Zomboid Mod Studio_${version}_amd64.deb" "${outWsl}/PZ-Mod-Studio-v${version}-linux-amd64.deb"
cp "${bundle}/pz-mcp-server" "${outWsl}/pz-mcp-server-v${version}-linux-x86_64"
cd "${outWsl}" && ls -l *v${version}-linux* && sha256sum *v${version}-linux*`,
  `Copying the bundles to ${outDir}`
);
console.log(`\n[build-linux] Done: ${outDir}`);
