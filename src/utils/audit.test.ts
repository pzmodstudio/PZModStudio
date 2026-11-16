import { describe, expect, it } from 'vitest';
import type { HealthIssue, IssueState } from '../types';
import { auditList } from './audit';

const issue = (code: string, mod_id: string, state: IssueState, related: string[] = []): HealthIssue => ({
  severity: state === 'act' ? 'error' : 'warning',
  state,
  code,
  mod_id,
  message: code,
  fixes: [],
  related,
});

describe('the list, mod by mod', () => {
  it('gives each mod its most pressing verdict, and keeps the rest', () => {
    const rows = auditList(
      ['Broken', 'A', 'B', 'Old', 'Odd', 'Fine'],
      [
        issue('missing_dependency', 'Broken', 'act', ['Lib']),
        issue('incompatible', 'A', 'act', ['B']),
        issue('community_unneeded', 'Old', 'recommended'),
        issue('api_missing', 'Odd', 'info'),
        issue('compatible_copy', 'Fine', 'handled'),
      ]
    );
    const by = Object.fromEntries(rows.map((r) => [r.mod_id, r.verdict]));
    expect(by).toEqual({ Broken: 'broken', A: 'choose', B: 'choose', Old: 'unneeded', Odd: 'check', Fine: 'keep' });
    expect(rows.map((r) => r.verdict)).toEqual(['broken', 'choose', 'choose', 'unneeded', 'check', 'keep']);
  });

  it('a mod cut off by another is the choice of both; a missing dependency breaks only the mod that needs it', () => {
    const rows = auditList(['Winner', 'Cut', 'Needs', 'Lib'], [issue('function_overridden', 'Winner', 'recommended', ['Cut']), issue('dependency_after', 'Needs', 'act', ['Lib'])]);
    const by = Object.fromEntries(rows.map((r) => [r.mod_id, r.verdict]));
    expect(by).toEqual({ Winner: 'choose', Cut: 'choose', Needs: 'broken', Lib: 'keep' });
  });
});
