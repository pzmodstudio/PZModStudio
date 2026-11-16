/**
 * Where a fix lands, and who it reaches. There is one place for fixes to files (Zomboid/mods: a
 * compatible copy, the Studio's patch package) and one list per player of what loads (the active
 * profile's for single player, each server's `Mods=`; a game joined to a server loads the
 * server's). So:
 * - a change to the package reaches every list that loads the package: the active profile (the
 *   Studio puts the package in it when it publishes) and each server whose `Mods=` has it; a
 *   server without it plays without the fix;
 * - switching a mod off changes the active profile only: a server listing the mod still loads it.
 * Said on the card, so "fixed" never reads as "fixed everywhere" when it is not.
 */
export type FixKind = 'package' | 'switch_off';

export interface ServerList {
  name: string;
  mods: string[];
}

export interface FixReach {
  kind: FixKind;
  /** The active profile's name: the list single player loads. */
  profile: string | null;
  /** Servers that load it (the package, or the mod still on there). */
  serversWith: string[];
  /** Servers that do not (the package missing from their Mods=: they play without the fix). */
  serversWithout: string[];
}

/** B42 writes `\id` in Mods=; the Studio's lists use the bare id. */
const bare = (id: string) => id.trim().replace(/^\\+/, '');

export const fixReach = (kind: FixKind, subject: string, profile: string | null, servers: ServerList[]): FixReach => {
  const id = bare(subject);
  const has = (s: ServerList) => s.mods.some((m) => bare(m) === id);
  const serversWith = servers.filter(has).map((s) => s.name);
  // For a mod switched off, servers that never listed it are not concerned: nothing to say about them.
  const serversWithout = kind === 'package' ? servers.filter((s) => !has(s)).map((s) => s.name) : [];
  return { kind, profile, serversWith, serversWithout };
};

/** The sentence parts, as i18n keys and their values, in the order they are said. */
export const reachLines = (r: FixReach): { key: string; values: Record<string, string | number> }[] => {
  const lines: { key: string; values: Record<string, string | number> }[] = [];
  const list = (names: string[]) => ({ names: names.join(', '), count: names.length });
  if (r.kind === 'package') {
    lines.push({ key: 'fixReach.packageMods', values: {} });
    if (r.profile) lines.push({ key: 'fixReach.packageProfile', values: { profile: r.profile } });
    if (r.serversWith.length) lines.push({ key: 'fixReach.packageServers', values: list(r.serversWith) });
    if (r.serversWithout.length) lines.push({ key: 'fixReach.packageNotServers', values: list(r.serversWithout) });
  } else {
    lines.push({ key: 'fixReach.offProfile', values: { profile: r.profile ?? '-' } });
    if (r.serversWith.length) lines.push({ key: 'fixReach.offStillServers', values: list(r.serversWith) });
  }
  return lines;
};
