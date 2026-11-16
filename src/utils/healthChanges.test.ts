import { describe, expect, it } from 'vitest';
import { anyChange, healthChanges } from './healthChanges';

const inputs = (mods: Record<string, string>, build = '1', log_written = 100) => ({ build, log_written, mods });

describe('what changed since the last check', () => {
  it('says nothing when nothing the check reads changed', () => {
    const c = healthChanges(inputs({ A: 'w1', B: 'l5' }), inputs({ A: 'w1', B: 'l5' }));
    expect(anyChange(c)).toBe(false);
  });

  it('names the mods updated since, the list changed, the game updated and a session played', () => {
    const c = healthChanges(inputs({ A: 'w1', B: 'l5' }), inputs({ A: 'w2', B: 'l5', C: 'w9' }, '2', 200));
    expect(c).toEqual({ list: true, updated: ['A'], build: true, played: true });
    expect(anyChange(c)).toBe(true);
  });
});
