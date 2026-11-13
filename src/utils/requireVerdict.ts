import type { ModuleCandidate, RequireAnswer } from '../types';

/** What the Monitor needs to know of a mod to tell whether two mods have to do with each other. */
export interface ModTies {
  mod_id: string;
  name: string;
  dependencies: string[];
  load_mod_after?: string[];
}

/** A mod id as `require=` writes it (Build 42 puts a backslash in front). */
const bareId = (id: string) => id.trim().replace(/^\\/, '');

/**
 * Whether the mod that asked has anything to do with the mod that owns a module of that name:
 * the module is the game's, or the mod's own, or one of the two names the other in its
 * `require=` or `loadModAfter=`. The lookup goes by file name only, so without one of these the
 * module found may be another one that happens to be called the same.
 */
export function related(candidate: ModuleCandidate, asking: ModTies | undefined, mods: ModTies[]): boolean {
  if (candidate.owner === 'the game') return true;
  if (!asking) return false;
  const owner = mods.find((m) => (candidate.owner_id ? m.mod_id === candidate.owner_id : m.name === candidate.owner));
  if (!owner) return false;
  if (owner.mod_id === asking.mod_id) return true;
  const names = (m: ModTies) => [...m.dependencies, ...(m.load_mod_after ?? [])].map(bareId);
  return names(asking).includes(owner.mod_id) || names(owner).includes(asking.mod_id);
}

/**
 * The candidate a card speaks of: the one at exactly the path asked for when there is one, else
 * the first that has to do with the mod that asked, else the first of all (and then `tied` is
 * false: it may be a different module with the same name, and no fix is offered from it).
 */
export function pickCandidate(
  answer: RequireAnswer | undefined,
  asking: ModTies | undefined,
  mods: ModTies[],
): { best: ModuleCandidate; tied: boolean } | null {
  const first = answer?.candidates[0];
  if (!answer || !first) return null;
  if (first.same_path) return { best: first, tied: true };
  const tied = answer.candidates.find((c) => !c.same_path && related(c, asking, mods));
  return tied ? { best: tied, tied: true } : { best: first, tied: false };
}

/**
 * What a failed `require` came to, from what the log and the installed mods show:
 * - `samePath`: the module is at exactly that path (asked from the wrong side, or too early).
 * - `gameMovedNoise` / `gameMovedFailed`: the game has it at another path; the file did not, or
 *   did, fail further down the log.
 * - `moved`: a mod tied to the one that asked has it at another path: the path can be fixed.
 * - `movedUnrelated`: only a mod with no tie to it has a module of that name: maybe another one.
 * - `optional`: nowhere among the game and the installed mods, the game carried on (the line is
 *   a warning), the line names the file that asked and no error from that file follows in the
 *   log: it reads as a link to a mod that is not there, with nothing to do.
 * - `optionalOff`: the same, and a mod that is installed and switched off has it at that path.
 * - `nowhere`: nowhere, and the log does not show that it went well.
 */
export type RequireVerdict = 'samePath' | 'gameMovedNoise' | 'gameMovedFailed' | 'moved' | 'movedUnrelated' | 'optional' | 'optionalOff' | 'nowhere';

export function requireVerdict(args: {
  answer: RequireAnswer;
  picked: { best: ModuleCandidate; tied: boolean } | null;
  /** The game wrote the line as a warning: it noticed and went on. */
  carriedOn: boolean;
  /**
   * The line names the file that asked and the log in front holds the line, so "no error from
   * that file follows" can be read off it. Without it nothing is said about what followed.
   */
  fileInLog: boolean;
  /** A later line of the log names the same file with an error. */
  fileFailedLater: boolean;
}): RequireVerdict {
  const { answer, picked, carriedOn, fileInLog, fileFailedLater } = args;
  if (!picked) {
    if (!carriedOn || !fileInLog || fileFailedLater) return 'nowhere';
    return (answer.installed_off?.length ?? 0) > 0 ? 'optionalOff' : 'optional';
  }
  if (picked.best.same_path) return 'samePath';
  if (picked.best.owner === 'the game') return fileFailedLater ? 'gameMovedFailed' : 'gameMovedNoise';
  return picked.tied ? 'moved' : 'movedUnrelated';
}

/** The fix the Studio can make by itself: a path that moved, in the game or in a mod tied to this one. */
export function knownRequireFix(wanted: string | null, picked: { best: ModuleCandidate; tied: boolean } | null): { from: string; to: string } | null {
  return wanted && picked && picked.tied && !picked.best.same_path ? { from: wanted, to: picked.best.module_path } : null;
}
