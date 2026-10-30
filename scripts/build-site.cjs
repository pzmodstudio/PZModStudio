// Fills site/ with the data its pages read, so the folder can be uploaded as it is (no git needed
// at the host): releases.json from the git tags, the community index beside the pages. Run: node scripts/build-site.cjs
const { execFileSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const site = path.join(root, 'site');
const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();

const tags = git('for-each-ref', '--sort=-creatordate', '--format=%(refname:short)|%(creatordate:short)', 'refs/tags')
  .split('\n').filter((l) => /^v\d/.test(l)).map((l) => { const [tag, date] = l.split('|'); return { tag, date }; });

// What the page lists: what a player can notice. Work on the site, the repository, the scripts and the
// packaging stays out, and a line that names another mod is said without the name.
const INTERNAL = /^(Site:|Site redesign|New captures|Repo moved|Ignore |Download from itch|Merge |Version \d|Bump |Docs?:|Tidy|Community index|Releases:|itch.io page)/;
const REWRITE = [
  [/^Site: one card per app screen.*/, 'Help: every page checked against the screens and brought up to date; the Workshop remove button and Mod Health\'s "Take out of the package" no longer show a raw text key'],
  [/^Workshop publish: read the upload.*/i, 'Developer: publishing a mod to the Steam Workshop from the Studio now says when Steam has finished'],
  [/ from Smooth Frames' console lines/i, " from the game's console lines"],
  [/Smooth Frames/gi, 'the frame report']
];
const subjects = (range) => git('log', '--format=%s', range).split(/\r?\n/)
  .filter(Boolean).map((s) => REWRITE.reduce((t, [re, to]) => t.replace(re, to), s))
  .filter((s) => !INTERNAL.test(s));

const releases = [];
const head = subjects(tags[0].tag + '..HEAD');
if (head.length) releases.push({ version: null, date: git('log', '-1', '--format=%ad', '--date=short'), notes: head });
tags.slice(0, 12).forEach((t, i) => {
  const prev = tags[i + 1];
  releases.push({ version: t.tag.replace(/^v/, ''), date: t.date, notes: subjects(prev ? prev.tag + '..' + t.tag : t.tag) });
});

const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
fs.writeFileSync(path.join(site, 'releases.json'), JSON.stringify({ latest: pkg.version, count: tags.length, built: new Date().toISOString().slice(0, 10), releases }, null, 1));

for (const f of ['index.json', 'rules.json', 'lists.json']) fs.copyFileSync(path.join(root, 'community', f), path.join(site, f));


console.log('site/ ready: version ' + pkg.version + ', ' + releases.length + ' release entries');
