#!/usr/bin/env node
/**
 * The community mod index: one registry per mod in community/index/<id>.json, built into
 * community/index.json, which PZ Mod Studio downloads once a day.
 *
 *   node scripts/community-index.js check   -- every registry is well formed (what a pull request must pass)
 *   node scripts/community-index.js build   -- check, then write community/index.json
 *
 * A registry's file name is its id with "/" written as "__" (dSAG's id is "2858201402/dailystatisticsandgains").
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(__dirname, '..', 'community');
const DIR = path.join(ROOT, 'index');
const OUT = path.join(ROOT, 'index.json');

const STATUSES = new Set(['verified', 'reported']);
const FIX_KINDS = new Set(['shim_patch', 'require_path', 'compatible_copy', 'none']);
const DATE = /^\d{4}-\d{2}-\d{2}$/;
const BUILD = /^\d{2}(\.\d+)?$/;

/** The shim's per-mod patch names, read from the shim that ships, so a fix cannot name a patch that is not there. */
function shimPatchNames() {
  const lua = fs.readFileSync(path.join(__dirname, '..', 'mods', 'Z_PZModStudio_Bridge', 'media', 'lua', 'shared', 'Z_PZModStudio_Polyfills.lua'), 'utf8');
  const names = new Set();
  let inside = false;
  for (const raw of lua.split('\n')) {
    const line = raw.trim();
    if (line.startsWith('Poly.patchDefaults = {')) { inside = true; continue; }
    if (!inside) continue;
    if (line.startsWith('}')) break;
    const m = /^(\w+)\s*=/.exec(line);
    if (m) names.add(m[1]);
  }
  return names;
}

function problemsOf(registry, fileName, patches) {
  const p = [];
  const str = (v) => typeof v === 'string' && v.trim().length > 0;
  const seen = (what, e) => {
    if (e.build !== undefined && !(str(e.build) && BUILD.test(e.build))) p.push(`${what}: build must look like "42" or "42.21"`);
    if (e.date !== undefined && !(str(e.date) && DATE.test(e.date))) p.push(`${what}: date must be YYYY-MM-DD`);
    if (e.status !== undefined && !STATUSES.has(e.status)) p.push(`${what}: status must be "verified" or "reported"`);
    if (!str(e.source) && !str(e.build)) p.push(`${what}: say where it was seen (source) or on which build`);
  };
  if (!str(registry.id)) p.push('id is missing');
  else if (fileName !== registry.id.replace(/\//g, '__') + '.json') p.push(`file name must be ${registry.id.replace(/\//g, '__')}.json`);
  if (registry.name !== undefined && !str(registry.name)) p.push('name must be text');
  if (registry.workshop_id !== undefined && !/^\d+$/.test(registry.workshop_id)) p.push('workshop_id must be the number in the Workshop URL');
  for (const key of ['authors', 'builds', 'requires']) {
    if (registry[key] !== undefined && !(Array.isArray(registry[key]) && registry[key].every(str))) p.push(`${key} must be a list of text`);
  }
  if (registry.builds) for (const b of registry.builds) if (!/^\d{2}$/.test(b)) p.push(`builds: "${b}" is not a major build ("41", "42")`);
  for (const key of ['incompatible', 'load_after']) {
    const list = registry[key] ?? [];
    if (!Array.isArray(list)) { p.push(`${key} must be a list`); continue; }
    list.forEach((rel, i) => {
      const what = `${key}[${i}]`;
      if (!str(rel.id)) p.push(`${what}: id is missing`);
      else if (rel.id === registry.id) p.push(`${what}: a mod cannot relate to itself`);
      seen(what, rel);
    });
  }
  if (registry.successor !== undefined) {
    if (!str(registry.successor.id)) p.push('successor: id is missing');
    if (registry.successor.workshop_id !== undefined && !/^\d+$/.test(registry.successor.workshop_id)) p.push('successor: workshop_id must be a number');
  }
  const issues = registry.issues ?? [];
  if (!Array.isArray(issues)) p.push('issues must be a list');
  else issues.forEach((issue, i) => {
    const what = `issues[${i}]`;
    if (!str(issue.title)) p.push(`${what}: title is missing`);
    if (issue.line !== undefined && !Number.isInteger(issue.line)) p.push(`${what}: line must be a number`);
    if (issue.file !== undefined && (!str(issue.file) || issue.file.startsWith('/') || issue.file.includes('..'))) p.push(`${what}: file is a path from the mod's root (media/lua/...)`);
    const fix = issue.fix ?? { kind: 'none' };
    if (!FIX_KINDS.has(fix.kind)) p.push(`${what}: fix.kind must be one of ${[...FIX_KINDS].join(', ')}`);
    if (fix.kind === 'shim_patch' && !patches.has(fix.name)) p.push(`${what}: the shim has no patch named "${fix.name}" (${[...patches].join(', ')})`);
    if (fix.kind === 'require_path' && !(str(fix.from) && str(fix.to))) p.push(`${what}: a require_path fix needs from and to`);
    if ((fix.kind === 'require_path' || fix.kind === 'compatible_copy') && !str(issue.file)) p.push(`${what}: say which file, so the Studio knows when it is already fixed`);
    seen(what, issue);
  });
  return p;
}

function readAll() {
  const patches = shimPatchNames();
  const mods = {};
  const errors = [];
  for (const fileName of fs.readdirSync(DIR).sort()) {
    if (!fileName.endsWith('.json')) continue;
    let registry;
    try {
      registry = JSON.parse(fs.readFileSync(path.join(DIR, fileName), 'utf8'));
    } catch (e) {
      errors.push(`${fileName}: not JSON (${e.message})`);
      continue;
    }
    for (const problem of problemsOf(registry, fileName, patches)) errors.push(`${fileName}: ${problem}`);
    if (mods[registry.id]) errors.push(`${fileName}: ${registry.id} is already filed`);
    mods[registry.id] = registry;
  }
  return { mods, errors };
}

function main() {
  const mode = process.argv[2];
  if (mode !== 'check' && mode !== 'build') {
    console.error('usage: node scripts/community-index.js check|build');
    process.exit(2);
  }
  const { mods, errors } = readAll();
  if (errors.length) {
    for (const e of errors) console.error(`  ${e}`);
    console.error(`${errors.length} problem(s) in community/index.`);
    process.exit(1);
  }
  const ids = Object.keys(mods);
  console.log(`${ids.length} registr${ids.length === 1 ? 'y' : 'ies'} well formed.`);
  if (mode === 'build') {
    const dates = ids.flatMap((id) => [...(mods[id].issues ?? []), ...(mods[id].incompatible ?? []), ...(mods[id].load_after ?? [])].map((e) => e.date).filter(Boolean));
    const index = { version: 1, updated: dates.sort().pop() ?? null, mods };
    fs.writeFileSync(OUT, JSON.stringify(index, null, 2) + '\n');
    console.log(`Wrote ${path.relative(process.cwd(), OUT)}.`);
  }
}

main();
