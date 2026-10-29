// Puts the Studio Link mod together for the game's own Workshop upload screen:
//   <Zomboid>/Workshop/Z_StudioLink/{workshop.txt, preview.png, Contents/mods/Z_StudioLink/...}
// Then in the game: main menu > Workshop > Create and update items > Z_StudioLink > upload.
// After the first upload the game writes the item's id= into that workshop.txt: copy it back to
// workshop/Z_StudioLink/workshop.txt so later updates go to the same item.
// Usage: node scripts/prepare-link-workshop.cjs [path to Zomboid]
const fs = require('fs');
const path = require('path');
const os = require('os');

const root = path.resolve(__dirname, '..');
const zomboid = process.argv[2] || path.join(os.homedir(), 'Zomboid');
const target = path.join(zomboid, 'Workshop', 'Z_StudioLink');
const source = path.join(root, 'workshop', 'Z_StudioLink');

if (!fs.existsSync(zomboid)) {
  console.error(`No Zomboid folder at ${zomboid}`);
  process.exit(1);
}
// An id the game wrote after an upload is kept: it is the Workshop item updated next time.
let keptId = '';
const oldTxt = path.join(target, 'workshop.txt');
if (fs.existsSync(oldTxt)) {
  const m = fs.readFileSync(oldTxt, 'utf8').match(/^id=(\d+)/m);
  if (m) keptId = m[1];
}
fs.rmSync(path.join(target, 'Contents'), { recursive: true, force: true });
fs.mkdirSync(path.join(target, 'Contents', 'mods'), { recursive: true });
fs.cpSync(path.join(root, 'mods', 'Z_StudioLink'), path.join(target, 'Contents', 'mods', 'Z_StudioLink'), { recursive: true });
let txt = fs.readFileSync(path.join(source, 'workshop.txt'), 'utf8');
const id = (txt.match(/^id=(\d+)/m) || [])[1] || keptId;
if (id && !/^id=/m.test(txt)) txt = txt.replace(/^version=1\r?\n/, `version=1\nid=${id}\n`);
fs.writeFileSync(path.join(target, 'workshop.txt'), txt);
fs.copyFileSync(path.join(source, 'preview.png'), path.join(target, 'preview.png'));
const version = (fs.readFileSync(path.join(root, 'mods', 'Z_StudioLink', '42', 'mod.info'), 'utf8').match(/^modversion=(.*)$/m) || [])[1];
console.log(`Ready: ${target} (Link ${version}${id ? `, Workshop item ${id}` : ', first upload'})`);
