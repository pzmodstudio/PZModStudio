// The release tag must be the version every manifest says, so what is published is what it claims
// to be. node scripts/check-release-version.cjs v1.2.0
const fs = require('fs');
const path = require('path');

const root = path.join(__dirname, '..');
const tag = (process.argv[2] || '').trim();
const wanted = tag.replace(/^v/, '');
if (!/^\d+\.\d+\.\d+([-+][\w.]+)?$/.test(wanted)) {
  console.error(`"${tag}" is not a version tag (vMAJOR.MINOR.PATCH).`);
  process.exit(1);
}

const found = {
  'package.json': JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version,
  'src-tauri/tauri.conf.json': JSON.parse(fs.readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8')).version,
  'src-tauri/Cargo.toml': (fs.readFileSync(path.join(root, 'src-tauri', 'Cargo.toml'), 'utf8').match(/^version\s*=\s*"([^"]+)"/m) || [])[1],
};
const wrong = Object.entries(found).filter(([, v]) => v !== wanted);
for (const [file, v] of Object.entries(found)) console.log(`${file}: ${v}`);
if (wrong.length > 0) {
  console.error(`The tag says ${wanted}, but ${wrong.map(([f, v]) => `${f} says ${v}`).join(', ')}.`);
  process.exit(1);
}
console.log(`All manifests say ${wanted}.`);
