/**
 * How sure a finding is, in three words a player can weigh:
 * - `seen`: the game wrote it in its log this session (a mod flooding the console);
 * - `rule`: it follows from the game's own rules and the files on disk (a missing dependency, an
 *   order mod.info asks for, two files at one path, a mod for another build): the game will act so;
 * - `suspicion`: read from the mods' code or from others' reports, and can be wrong (an API name
 *   the build may lack, a function one mod seems to cut off, what code could do, a report about a pair).
 * The index's entries: a known issue verified in the game counts as seen, a verified pair or order
 * as a rule, anything only reported as a suspicion.
 */
export type Certainty = 'seen' | 'rule' | 'suspicion';

const SEEN = new Set(['floods_console', 'zb_excluded']);
const SUSPICION = new Set(['api_missing', 'function_overridden', 'java_patch_overlap', 'two_mod_managers']);

export const certaintyOf = (issue: { code: string; params?: Record<string, string> }): Certainty => {
  const { code } = issue;
  if (SEEN.has(code)) return 'seen';
  // The index's entries say whether someone verified them in the game, or only reported them.
  const verified = issue.params?.seenKey?.startsWith('verified');
  if (code === 'community_known_issue') return verified ? 'seen' : 'suspicion';
  if (code === 'community_incompatible' || code === 'community_rule_order') return verified ? 'rule' : 'suspicion';
  if (code.startsWith('security_') || SUSPICION.has(code)) return 'suspicion';
  return 'rule';
};
