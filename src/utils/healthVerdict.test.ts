import { describe, expect, it } from 'vitest';
import type { HealthIssue, IssueState } from '../types';
import { healthVerdict } from './healthVerdict';

const issue = (code: string, state?: IssueState, severity: HealthIssue['severity'] = 'info'): HealthIssue => ({
  severity,
  state,
  code,
  mod_id: code,
  message: code,
  fixes: [],
  related: [],
});

describe('the list in one word', () => {
  it('is ok with nothing found, or only things to know and things already fixed', () => {
    expect(healthVerdict([]).level).toBe('ok');
    const v = healthVerdict([issue('a', 'info'), issue('b', 'handled')]);
    expect(v.level).toBe('ok');
    expect(v.next).toBeNull();
    expect(v.counts).toEqual({ act: 0, recommended: 0, info: 1, handled: 1 });
  });

  it('asks to act when anything breaks, and names the first such finding as the thing to do now', () => {
    const v = healthVerdict([issue('tip', 'recommended'), issue('broken', 'act'), issue('second', 'act')]);
    expect(v.level).toBe('act');
    expect(v.next?.code).toBe('broken');
    expect(v.counts.act).toBe(2);
  });

  it('only recommends when nothing breaks, and names the first recommendation', () => {
    const v = healthVerdict([issue('know', 'info'), issue('tip', 'recommended')]);
    expect(v.level).toBe('recommended');
    expect(v.next?.code).toBe('tip');
  });

  it('puts a sure finding with a button first, before a suspicion with nothing to press', () => {
    const suspicion = issue('function_overridden', 'recommended');
    const sureNoButton = issue('file_conflict', 'recommended');
    const sureButton = { ...issue('workshop_copy_stale', 'recommended'), fixes: [{ action: 'make_compatible_copy' } as HealthIssue['fixes'][number]] };
    expect(healthVerdict([suspicion, sureNoButton, sureButton]).next?.code).toBe('workshop_copy_stale');
    expect(healthVerdict([suspicion, sureNoButton]).next?.code).toBe('file_conflict');
    expect(healthVerdict([suspicion]).next?.code).toBe('function_overridden');
  });

  it('reads an older finding without a state by its severity', () => {
    expect(healthVerdict([issue('old', undefined, 'error')]).level).toBe('act');
    expect(healthVerdict([issue('old', undefined, 'warning')]).level).toBe('recommended');
  });
});
