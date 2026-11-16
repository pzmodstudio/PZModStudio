import type { HealthInputs } from '../types';

/**
 * What changed since Mod Health's last full check, from what the check depends on (the game's
 * build, its log, each active mod's stamp), compared without reading any mod file. Opening Mod
 * Health no longer checks by itself (a full check reads every file of every mod: seconds on a
 * long list, minutes on a PC short of memory): it says this, and the button does the rest.
 */
export interface HealthChanges {
  /** Mods switched on, off or moved since. */
  list: boolean;
  /** Active mods whose files changed since (a Workshop update, an edit to a local mod). */
  updated: string[];
  /** The game was updated since. */
  build: boolean;
  /** The game wrote its log since (a session played: "seen in the log" findings may differ). */
  played: boolean;
}

export const healthChanges = (then: HealthInputs, now: HealthInputs): HealthChanges => {
  const before = Object.keys(then.mods);
  const after = Object.keys(now.mods);
  const sameSet = before.length === after.length && after.every((id) => id in then.mods);
  return {
    list: !sameSet,
    updated: after.filter((id) => id in then.mods && then.mods[id] !== now.mods[id]),
    build: (then.build ?? '') !== (now.build ?? ''),
    played: (now.log_written ?? 0) > (then.log_written ?? 0),
  };
};

export const anyChange = (c: HealthChanges | null): boolean => !!c && (c.list || c.updated.length > 0 || c.build || c.played);
