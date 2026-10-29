#!/usr/bin/env node
/**
 * Players' lists of mods: one file per list in community/lists/<id>.json, built into
 * community/lists.json, which PZ Mod Studio downloads once a day (Browse > My lists > Community).
 *
 *   node scripts/community-lists.js check   -- every list is well formed (what a pull request must pass)
 *   node scripts/community-lists.js build   -- check, then write community/lists.json
 *
 * The same rules as the Studio's own check (src-tauri/src/rules/shared_lists.rs).
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(__dirname, '..', 'community');
const DIR = path.join(ROOT, 'lists');
const OUT = path.join(ROOT, 'lists.json');
const MAX_ITEMS = 300;
const ID = /^[a-z0-9](?:[a-z0-9-]{1,58})[a-z0-9]$/;
const DATE = /^\d{4}-\d{2}-\d{2}$/;

function problemsOf(list, fileName) {
  const p = [];
  const str = (v, max) => typeof v === 'string' && v.trim().length > 0 && [...v].length <= max;
  if (!(typeof list.id === 'string' && ID.test(list.id))) p.push('id: 3 to 60 lower-case letters, digits and -');
  else if (fileName !== `${list.id}.json`) p.push(`file name must be ${list.id}.json`);
  if (!str(list.name, 80)) p.push('name: 1 to 80 characters');
  if (!str(list.author, 60)) p.push('author: 1 to 60 characters');
  if (list.description !== undefined && !(typeof list.description === 'string' && [...list.description].length <= 500)) p.push('description: at most 500 characters');
  if (list.build !== undefined && list.build !== '' && list.build !== '41' && list.build !== '42') p.push('build: "41" or "42"');
  if (list.updated !== undefined && !(typeof list.updated === 'string' && DATE.test(list.updated))) p.push('updated: YYYY-MM-DD');
  if (!Array.isArray(list.items) || list.items.length === 0 || list.items.length > MAX_ITEMS) p.push(`items: 1 to ${MAX_ITEMS}`);
  else list.items.forEach((item, i) => {
    if (item.workshop_id !== undefined && !/^\d+$/.test(item.workshop_id)) p.push(`items[${i}]: workshop_id is the number in the Workshop link`);
    if (item.nexus_id !== undefined && !Number.isInteger(item.nexus_id)) p.push(`items[${i}]: nexus_id is a number`);
    if (item.workshop_id === undefined && item.nexus_id === undefined && !str(item.mod_id, 200)) p.push(`items[${i}]: a Workshop id, a Nexus id or a mod id`);
    if (!str(item.name, 200)) p.push(`items[${i}]: name is missing`);
  });
  return p;
}

function main() {
  const mode = process.argv[2];
  if (mode !== 'check' && mode !== 'build') {
    console.error('usage: node scripts/community-lists.js check|build');
    process.exit(2);
  }
  const lists = [];
  const errors = [];
  for (const fileName of fs.readdirSync(DIR).sort()) {
    if (!fileName.endsWith('.json')) continue;
    let list;
    try {
      list = JSON.parse(fs.readFileSync(path.join(DIR, fileName), 'utf8'));
    } catch (e) {
      errors.push(`${fileName}: not JSON (${e.message})`);
      continue;
    }
    for (const problem of problemsOf(list, fileName)) errors.push(`${fileName}: ${problem}`);
    lists.push(list);
  }
  if (errors.length) {
    for (const e of errors) console.error(`  ${e}`);
    console.error(`${errors.length} problem(s) in community/lists.`);
    process.exit(1);
  }
  console.log(`${lists.length} list(s) well formed.`);
  if (mode === 'build') {
    const updated = lists.map((l) => l.updated).filter(Boolean).sort().pop() ?? null;
    fs.writeFileSync(OUT, JSON.stringify({ version: 1, updated, lists }, null, 2) + '\n');
    console.log(`Wrote ${path.relative(process.cwd(), OUT)}.`);
  }
}

main();
