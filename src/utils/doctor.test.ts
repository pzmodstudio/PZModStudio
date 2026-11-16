import { describe, expect, it } from 'vitest';
import type { HealthIssue, TranslatedErrorCard } from '../types';
import { doctorStep } from './doctor';

const act: HealthIssue = { severity: 'error', state: 'act', code: 'missing_dependency', mod_id: 'A', message: 'A needs B', fixes: [], related: [] };
const card = (mod_name: string, severity: 'EXCEPTION' | 'WARNING' = 'EXCEPTION', extra: Partial<TranslatedErrorCard> = {}): TranslatedErrorCard =>
  ({ id: mod_name, raw_error: 'x', title: 't', explanation: '', recommended_action: '', mod_name, severity, ...extra }) as TranslatedErrorCard;
const none = { issues: [], cards: [], outOfMemory: 0, windowsLow: 0, tried: [] };

describe("the Doctor's next step for a game that crashes", () => {
  it('fixes what the list breaks first', () => {
    expect(doctorStep({ ...none, issues: [act], cards: [card('M')] }).kind).toBe('health');
  });

  it('then the mod an exception names, one at a time; warnings, other logs and the player\'s own mod are not blamed', () => {
    const cards = [card('Warned', 'WARNING'), card('Server', 'EXCEPTION', { log: 'server' }), card('Mine', 'EXCEPTION', { own_mod: true }), card('M1'), card('M2')];
    const first = doctorStep({ ...none, issues: [act], cards, tried: ['health'] });
    expect(first.kind === 'blamed' && first.mod).toBe('M1');
    const second = doctorStep({ ...none, cards, tried: ['health', 'blamed:M1'] });
    expect(second.kind === 'blamed' && second.mod).toBe('M2');
  });

  it('then the memory, then says it does not know', () => {
    expect(doctorStep({ ...none, windowsLow: 2 }).kind).toBe('memory');
    expect(doctorStep({ ...none, windowsLow: 2, tried: ['memory'] }).kind).toBe('unknown');
    expect(doctorStep(none).kind).toBe('unknown');
  });
});
