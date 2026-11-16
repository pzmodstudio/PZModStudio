import { HealthIssue, IssueState } from '../types';
import { issueStateOf } from './issueState';
import { certaintyOf } from './certainty';

/**
 * The whole list in one word, from the same four states every finding has:
 * - `act`: something breaks until it is fixed (a mod that will not load, one missing what it needs);
 * - `recommended`: the game goes, and some things are worth doing;
 * - `ok`: nothing to do (findings to know about, or already fixed by the Studio, may remain).
 * It says nothing the findings do not: no finding of a kind is never read as "fine" for it.
 */
export type VerdictLevel = 'act' | 'recommended' | 'ok';

export interface HealthVerdict {
  level: VerdictLevel;
  counts: Record<IssueState, number>;
  /**
   * The one thing to do now, from the most pressing state with something to do: first a finding
   * that is sure (not a suspicion) and has a button, then a sure one, then any. A suspicion with
   * nothing to press ("keep one if something is missing") is never put first over a fix.
   */
  next: HealthIssue | null;
}

/** Lower comes first: sure with a button, sure, a suspicion with a button, a suspicion. */
export const doFirstRank = (i: HealthIssue) => (certaintyOf(i) !== 'suspicion' ? 0 : 2) + (i.fixes.length > 0 ? 0 : 1);

/** The rows of one state in the order to do them (stable: the engine's order within a rank). */
export const inDoFirstOrder = (issues: HealthIssue[]): HealthIssue[] =>
  issues.map((issue, n) => ({ issue, n })).sort((a, b) => doFirstRank(a.issue) - doFirstRank(b.issue) || a.n - b.n).map((x) => x.issue);

export const healthVerdict = (issues: HealthIssue[]): HealthVerdict => {
  const counts: Record<IssueState, number> = { act: 0, recommended: 0, info: 0, handled: 0 };
  const best: Partial<Record<IssueState, HealthIssue>> = {};
  for (const issue of issues) {
    const state = issueStateOf(issue);
    counts[state] += 1;
    const now = best[state];
    if (!now || doFirstRank(issue) < doFirstRank(now)) best[state] = issue;
  }
  const level: VerdictLevel = counts.act > 0 ? 'act' : counts.recommended > 0 ? 'recommended' : 'ok';
  return { level, counts, next: best.act ?? best.recommended ?? null };
};
