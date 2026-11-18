/**
 * How sure a finding is, in three words a player can weigh:
 * - `seen`: the game wrote it in its log this session (a mod flooding the console), or the Studio
 *   Link saw it in the running game;
 * - `rule`: it follows from the game's own rules and the files on disk (a missing dependency, an
 *   order mod.info asks for, two files at one path, a mod for another build): the game will act so;
 * - `suspicion`: read from the mods' code or from others' reports, and can be wrong (an API name
 *   the build may lack, a function one mod seems to cut off, what code could do, a report about a pair).
 * The index's entries: a known issue verified in the game counts as seen, a verified pair or order
 * as a rule, anything only reported as a suspicion.
 *
 * Calibrated 2026-11-18 on three collections (340, 389 and 449 mods) against the mods' files, the
 * game's decompiled loader and its logs:
 * - the mod.info findings (requirements, version limits, incompatible pairs, load-after) matched an
 *   independent reading every time once the game's own load order was used (it loads each mod's
 *   requirements first, and those that are off too): rules;
 * - two mods' files at one path: every one real (306 of 306), the winner right in that order: a rule;
 * - a function cut off: made while the files load, every cut checked in the code was real (the
 *   backend marks them `certain: load`), so a rule; one made in a handler or install(), or in a file
 *   that may stop early, depends on what runs: a suspicion;
 * - an API name the build lacks: the name was really missing in about 98 of 100 after the fixes,
 *   but whether that code ever runs is not known: a suspicion.
 */
export type Certainty = 'seen' | 'rule' | 'suspicion';

const SEEN = new Set(['floods_console', 'zb_excluded', 'learned_issue', 'function_seen_other', 'game_folder_changed']);
const SUSPICION = new Set(['api_missing', 'function_overridden', 'java_patch_overlap', 'two_mod_managers']);

export const certaintyOf = (issue: { code: string; params?: Record<string, string> }): Certainty => {
  const { code } = issue;
  if (SEEN.has(code)) return 'seen';
  // A function the Studio Link looked at in the running game: what it saw there.
  if (code === 'function_overridden' && issue.params?.checkedKey) return 'seen';
  // A cut made while the files load follows from the load order alone.
  if (code === 'function_overridden' && issue.params?.certain === 'load') return 'rule';
  // Two mods' copies of one file: the game's log of its last session said which it took.
  if (code === 'file_conflict' && issue.params?.checkedKey) return 'seen';
  // The index's entries say whether someone verified them in the game, or only reported them.
  const verified = issue.params?.seenKey?.startsWith('verified');
  if (code === 'community_known_issue') return verified ? 'seen' : 'suspicion';
  if (code === 'community_incompatible' || code === 'community_rule_order') return verified ? 'rule' : 'suspicion';
  if (code.startsWith('security_') || SUSPICION.has(code)) return 'suspicion';
  return 'rule';
};
