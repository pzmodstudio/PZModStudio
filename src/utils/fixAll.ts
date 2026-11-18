import type { HealthFix, HealthIssue } from '../types';

/** Fixes the "fix all" button may apply without asking: they only enable or reorder. */
export const SAFE_FIX_ACTIONS = new Set<HealthFix['action']>(['enable_mod', 'move_before']);

/**
 * Issues whose fix is a matter of taste, not of the game's rules: two mods defining the same item
 * is not broken, and which version you want is nobody's call but yours. Their fix stays on its own
 * row and is kept out of the one-click button, which is for what the game itself would complain
 * about.
 */
export const YOUR_CALL_CODES = new Set([
  'script_definition_conflict',
  'script_definition_stale',
  'script_definition_settled',
  'translation_conflict',
  'translation_stale',
  'translation_settled',
  'package_built_for_other_list',
]);

export interface SafeFixPlan {
  /** The rows the button fixes: what it counts. */
  issues: HealthIssue[];
  /** Their safe fixes, in the order the rows come. */
  fixes: HealthFix[];
  /** How many mods it switches on and how many it moves: each mod once, however many rows ask for it. */
  enabling: number;
  moving: number;
}

/**
 * What "Fix N problems automatically" does: every safe fix (switch on, move before) of the rows it
 * may touch. `among` limits it to the rows on screen, so the button never counts or applies what
 * the player cannot see (Simple hides the optional rows).
 */
export const safeFixPlan = (allIssues: HealthIssue[], among?: (issue: HealthIssue) => boolean): SafeFixPlan => {
  const issues = allIssues.filter(
    (i) => !YOUR_CALL_CODES.has(i.code) && i.fixes.some((f) => SAFE_FIX_ACTIONS.has(f.action)) && (!among || among(i))
  );
  const fixes = issues.flatMap((i) => i.fixes.filter((f) => SAFE_FIX_ACTIONS.has(f.action)));
  return {
    issues,
    fixes,
    // Two mods needing the same library switch it on once: the question counts mods, not rows.
    enabling: new Set(fixes.flatMap((f) => (f.action === 'enable_mod' ? [f.mod_id] : []))).size,
    moving: new Set(fixes.flatMap((f) => (f.action === 'move_before' ? [f.mod_id] : []))).size,
  };
};
