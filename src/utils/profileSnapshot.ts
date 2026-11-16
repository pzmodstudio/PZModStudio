import type { CatalogModSummary, HealthIssue, ModInfo } from '../types';
import { issueStateOf } from './issueState';

/**
 * A photo of a profile at one moment: its mods (on or off, in order, and which version of each:
 * its mod.info version and when Steam last updated it), the game build, and what Mod Health found.
 * Compared with a later one it answers "what changed since?": mods in or out, switched, moved,
 * updated by Steam, and findings that appeared or went away. The way to tell what a Workshop
 * update or a game update broke, and the yardstick of the stress test.
 */
export interface SnapshotMod {
  id: string;
  name: string;
  on: boolean;
  /** Place among the switched-on mods (the order the game loads them), or null when off. */
  position: number | null;
  version: string | null;
  /** When Steam last updated it (unix seconds); null for a local mod. */
  updated: number | null;
}

export interface SnapshotFinding {
  /** What identifies it across checks: code, mod, and the message's first words. */
  key: string;
  code: string;
  state: string;
  mod_id: string | null;
  text: string;
}

export interface ProfileSnapshot {
  version: 1;
  taken_at: number;
  profile_id: string;
  profile_name: string;
  label: string;
  game_version: string | null;
  mods: SnapshotMod[];
  findings: SnapshotFinding[];
}

const findingKey = (i: { code: string; mod_id: string | null; message: string }) => `${i.code}|${i.mod_id ?? ''}|${i.message.slice(0, 160)}`;

export const takeSnapshot = (
  profile: { id: string; name: string },
  label: string,
  mods: ModInfo[],
  catalog: CatalogModSummary[],
  issues: HealthIssue[],
  gameVersion: string | null,
  now: number = Math.floor(Date.now() / 1000)
): ProfileSnapshot => {
  const updated = new Map<string, number | null>(catalog.map((c) => [c.id, c.workshop_state?.installed_time ?? null]));
  let position = 0;
  return {
    version: 1,
    taken_at: now,
    profile_id: profile.id,
    profile_name: profile.name,
    label,
    game_version: gameVersion,
    mods: mods.map((m) => ({
      id: m.mod_id,
      name: m.name,
      on: m.enabled,
      position: m.enabled ? position++ : null,
      version: m.version ?? null,
      updated: updated.get(m.mod_id) ?? null,
    })),
    findings: issues.map((i) => ({ key: findingKey(i), code: i.code, state: issueStateOf(i), mod_id: i.mod_id, text: i.message })),
  };
};

export interface SnapshotDiff {
  game: { before: string | null; after: string | null } | null;
  added: SnapshotMod[];
  removed: SnapshotMod[];
  switchedOn: SnapshotMod[];
  switchedOff: SnapshotMod[];
  /** On in both, at another place in the order. */
  moved: { mod: SnapshotMod; from: number; to: number }[];
  /** Steam updated it, or its mod.info version changed. */
  updated: { mod: SnapshotMod; fromVersion: string | null; toVersion: string | null }[];
  newFindings: SnapshotFinding[];
  goneFindings: SnapshotFinding[];
}

/** What changed from `before` to `after` (two photos of one profile, or a photo and now). */
export const compareSnapshots = (before: ProfileSnapshot, after: ProfileSnapshot): SnapshotDiff => {
  const was = new Map(before.mods.map((m) => [m.id, m]));
  const is = new Map(after.mods.map((m) => [m.id, m]));
  const diff: SnapshotDiff = {
    game: before.game_version !== after.game_version ? { before: before.game_version, after: after.game_version } : null,
    added: after.mods.filter((m) => !was.has(m.id)),
    removed: before.mods.filter((m) => !is.has(m.id)),
    switchedOn: [],
    switchedOff: [],
    moved: [],
    updated: [],
    newFindings: [],
    goneFindings: [],
  };
  // A move counts only against the mods both photos have on, so one mod switched on or off
  // does not read as every mod after it moving.
  const bothOn = new Set(after.mods.filter((m) => m.on && was.get(m.id)?.on).map((m) => m.id));
  const rank = (mods: SnapshotMod[]) => new Map(mods.filter((m) => m.on && bothOn.has(m.id)).map((m, i) => [m.id, i]));
  const rankBefore = rank(before.mods);
  const rankAfter = rank(after.mods);
  for (const m of after.mods) {
    const old = was.get(m.id);
    if (!old) continue;
    if (m.on && !old.on) diff.switchedOn.push(m);
    if (!m.on && old.on) diff.switchedOff.push(m);
    const from = rankBefore.get(m.id);
    const to = rankAfter.get(m.id);
    if (from !== undefined && to !== undefined && from !== to) diff.moved.push({ mod: m, from: from + 1, to: to + 1 });
    if ((m.updated !== null && old.updated !== null && m.updated !== old.updated) || (m.version ?? '') !== (old.version ?? '')) {
      diff.updated.push({ mod: m, fromVersion: old.version, toVersion: m.version });
    }
  }
  const keysBefore = new Set(before.findings.map((f) => f.key));
  const keysAfter = new Set(after.findings.map((f) => f.key));
  diff.newFindings = after.findings.filter((f) => !keysBefore.has(f.key));
  diff.goneFindings = before.findings.filter((f) => !keysAfter.has(f.key));
  return diff;
};

export const diffIsEmpty = (d: SnapshotDiff) =>
  !d.game && [d.added, d.removed, d.switchedOn, d.switchedOff, d.moved, d.updated, d.newFindings, d.goneFindings].every((l) => l.length === 0);
