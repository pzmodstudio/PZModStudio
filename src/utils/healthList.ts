import { HealthFix } from '../types';

/**
 * Which list Mod Health checks: the active profile's (what single player loads), or a server's
 * `Mods=` (what the server, and every game joined to it, loads). The engine is the same; what
 * changes is the list it reads and which fixes can act on it from here.
 */
export type HealthListTarget = { kind: 'profile' } | { kind: 'server'; name: string };

/**
 * Fixes that rewrite the list itself (switch on/off, move, take out, replace). They act on the
 * profile's list; a server's `Mods=` is changed in Servers, so on a server's check they are not
 * offered. Fixes to files (a compatible copy, the patch package, a merge) live in Zomboid/mods and
 * reach every list, so they stay.
 */
export const LIST_FIX_ACTIONS = new Set<HealthFix['action']>(['enable_mod', 'disable_mod', 'move_before', 'remove_from_list', 'replace_in_list']);
/**
 * Fixes that rewrite the patch package from the profile's list (merge, take a file out, switch a
 * compatibility fix): the package is built for the profile, so on a server's check, whose list may
 * differ, they could take out something single player still needs. Not offered there either.
 */
export const PACKAGE_FIX_ACTIONS = new Set<HealthFix['action']>(['merge_files', 'remove_from_package', 'enable_shim_patch', 'update_package']);

export const fixActsOn = (action: HealthFix['action'], target: HealthListTarget): boolean =>
  target.kind === 'profile' || (!LIST_FIX_ACTIONS.has(action) && !PACKAGE_FIX_ACTIONS.has(action));

/**
 * A server's `Mods=` as the ids the check reads: B42 writes `\id`, the order is kept, an id
 * listed twice counts once (the game loads it once).
 */
export const serverListIds = (mods: string[]): string[] => {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of mods) {
    const id = raw.trim().replace(/^\\+/, '');
    if (!id || seen.has(id)) continue;
    seen.add(id);
    out.push(id);
  }
  return out;
};

/** How a server's list differs from the profile's switched-on mods, in the profile's order and the server's. */
export const listDifference = (profileOn: string[], serverIds: string[]): { onlyProfile: string[]; onlyServer: string[] } => {
  const server = new Set(serverIds);
  const profile = new Set(profileOn);
  return { onlyProfile: profileOn.filter((id) => !server.has(id)), onlyServer: serverIds.filter((id) => !profile.has(id)) };
};
