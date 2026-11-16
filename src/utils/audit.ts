import type { HealthIssue } from '../types';
import { issueStateOf } from './issueState';
import { certaintyOf } from './certainty';

/**
 * The list mod by mod: one verdict each, from the findings Mod Health already has, the most
 * pressing first.
 * - `broken`: it does not load, or lacks what it needs to (a finding that breaks the game, about it);
 * - `choose`: it and another rule each other out or one cuts the other off (incompatible pairs,
 *   function cut-offs, alternatives of one item, two mods' same-line changes): one is to be picked;
 * - `unneeded`: the game, or another mod that is on, does it now (the index says so);
 * - `check`: only suspicions about it (an API name the build may lack, what its code could do);
 * - `keep`: nothing found.
 * A finding counts for its own mod and for the mods it names (an incompatible pair is both mods'
 * question), but "broken" only for its own mod: a missing dependency is the requiring mod's.
 */
export type AuditVerdict = 'broken' | 'choose' | 'unneeded' | 'check' | 'keep';

export interface AuditRow {
  mod_id: string;
  verdict: AuditVerdict;
  /** The findings behind the verdict, most pressing first. */
  reasons: HealthIssue[];
}

const CHOOSE = new Set(['incompatible', 'community_incompatible', 'function_overridden', 'file_conflict', 'file_conflict_stale', 'two_mod_managers']);
const UNNEEDED = new Set(['community_unneeded', 'community_successor']);
const RANK: Record<AuditVerdict, number> = { broken: 0, choose: 1, unneeded: 2, check: 3, keep: 4 };

const verdictOf = (issue: HealthIssue, own: boolean): AuditVerdict | null => {
  const state = issueStateOf(issue);
  if (state === 'handled') return null;
  if (UNNEEDED.has(issue.code)) return own ? 'unneeded' : null;
  if (CHOOSE.has(issue.code)) return 'choose';
  if (state === 'act') return own ? 'broken' : null;
  if (certaintyOf(issue) === 'suspicion') return own ? 'check' : null;
  return null;
};

export const auditList = (activeIds: string[], issues: HealthIssue[]): AuditRow[] => {
  const rows = new Map<string, AuditRow>(activeIds.map((id) => [id, { mod_id: id, verdict: 'keep', reasons: [] }]));
  for (const issue of issues) {
    const involved = [issue.mod_id, ...issue.related].filter((id): id is string => !!id);
    for (const id of involved) {
      const row = rows.get(id);
      if (!row) continue;
      const v = verdictOf(issue, id === issue.mod_id);
      if (!v) continue;
      if (!row.reasons.includes(issue)) row.reasons.push(issue);
      if (RANK[v] < RANK[row.verdict]) row.verdict = v;
    }
  }
  for (const row of rows.values()) row.reasons.sort((a, b) => (issueStateOf(a) === 'act' ? 0 : 1) - (issueStateOf(b) === 'act' ? 0 : 1));
  return [...rows.values()].sort((a, b) => RANK[a.verdict] - RANK[b.verdict]);
};
