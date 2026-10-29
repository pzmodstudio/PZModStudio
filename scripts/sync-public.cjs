// Mirrors the app (without the private notes, binaries and workflows) into the public repo's
// working folder, commits it on main, puts site/ on the gh-pages branch, and pushes both.
//   node scripts/sync-public.cjs "commit message" [--no-push]
// The public folder is PUBLIC_REPO_DIR (default publish-repo/ inside the project, ignored by git); its git login is
// the one Git Credential Manager already holds. Nothing here reads a token.
const { execFileSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

const root = path.resolve(__dirname, '..');
const dest = process.env.PUBLIC_REPO_DIR || path.join(root, 'publish-repo');
const message = process.argv[2] || 'Update';
const push = !process.argv.includes('--no-push');
const run = (cwd, ...args) => execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'] }).trim();

// What stays private or goes elsewhere: research notes, forum posts, the exes (they are Release files),
// the workflows (no Actions here), loose error reports and scratch files.
const EXCLUDE = /^(private\/|publish-repo\/|docs\/(RESEARCH|COMPETITORS|AUDIT|ROADMAP|BRIEF)|docs\/forum\/|\.github\/(?!ISSUE_TEMPLATE\/)|dist-local\/|pz-error-|PZ MODDING AGENT GUIDE\.md$|DOCUMENTACION_SISTEMA\.md$)|\.exe$|\.exe\.old|\.old-/;

// What the community workflow used to do on every merge: the registries and lists are checked and
// index.json / lists.json rebuilt from them, so a sync never ships a stale or malformed index.
const node = (...args) => execFileSync('node', args, { cwd: root, stdio: 'inherit' });
node(path.join(__dirname, 'community-index.js'), 'build');
node(path.join(__dirname, 'community-lists.js'), 'build');
const rules = JSON.parse(fs.readFileSync(path.join(root, 'community', 'rules.json'), 'utf8'));
for (const x of rules.load_after || []) if (!x.mod_id || !x.after || x.mod_id === x.after) throw new Error('bad load_after rule ' + JSON.stringify(x));
for (const x of rules.incompatible || []) if (!x.a || !x.b || x.a === x.b) throw new Error('bad incompatible pair ' + JSON.stringify(x));
console.log('rules.json ok');

node(path.join(__dirname, 'build-site.cjs'));

const files = run(root, 'ls-files', '-co', '--exclude-standard').split('\n').filter((f) => f && !EXCLUDE.test(f) && fs.existsSync(path.join(root, f)));
const keep = new Set(files);

// Remove what the public copy has and the project no longer ships.
const walk = (dir, rel = '') => fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
  if (e.name === '.git') return [];
  const r = rel ? rel + '/' + e.name : e.name;
  return e.isDirectory() ? walk(path.join(dir, e.name), r) : [r];
});
for (const f of walk(dest)) if (!keep.has(f)) fs.rmSync(path.join(dest, f));
for (const f of files) { fs.mkdirSync(path.dirname(path.join(dest, f)), { recursive: true }); fs.copyFileSync(path.join(root, f), path.join(dest, f)); }
// docs/INDEX.md lists a document that stays private.
const indexMd = path.join(dest, 'docs', 'INDEX.md');
if (fs.existsSync(indexMd)) fs.writeFileSync(indexMd, fs.readFileSync(indexMd, 'utf8').split('\n').filter((l) => !l.includes('COMPETITORS.md')).join('\n'));

run(dest, 'add', '-A');
if (run(dest, 'status', '--porcelain')) { run(dest, 'commit', '-q', '-m', message); console.log('main: committed "' + message + '"'); } else console.log('main: nothing changed');

// gh-pages: a commit whose tree is site/ alone, built with plumbing so no branch switch is needed.
const tmpIndex = path.join(os.tmpdir(), 'pz-pages-index');
fs.rmSync(tmpIndex, { force: true });
const env = { ...process.env, GIT_INDEX_FILE: tmpIndex, GIT_WORK_TREE: path.join(dest, 'site') };
fs.writeFileSync(path.join(dest, 'site', '.nojekyll'), '');
execFileSync('git', ['add', '-A', '-f', '.'], { cwd: path.join(dest, 'site'), env: { ...env, GIT_DIR: path.join(dest, '.git') } });
const tree = execFileSync('git', ['write-tree'], { cwd: path.join(dest, 'site'), env: { ...env, GIT_DIR: path.join(dest, '.git') }, encoding: 'utf8' }).trim();
fs.rmSync(path.join(dest, 'site', '.nojekyll'));
let parent = null;
try { parent = run(dest, 'rev-parse', '--verify', '-q', 'refs/heads/gh-pages'); } catch (e) { parent = null; }
const prevTree = parent ? run(dest, 'rev-parse', parent + '^{tree}') : null;
if (tree !== prevTree) {
  const commit = run(dest, 'commit-tree', tree, ...(parent ? ['-p', parent] : []), '-m', 'Site: ' + message);
  run(dest, 'update-ref', 'refs/heads/gh-pages', commit);
  console.log('gh-pages: updated');
} else console.log('gh-pages: nothing changed');

if (push) { run(dest, 'push', 'origin', 'main', 'gh-pages'); console.log('pushed main and gh-pages'); }
