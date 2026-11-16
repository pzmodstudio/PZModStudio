import type { HealthIssue, TranslatedErrorCard } from '../types';
import { issueStateOf } from './issueState';

/**
 * The Doctor's next step for "the game crashes or will not start", from what the Studio knows,
 * the surest first, each step tried once:
 * 1. something in the list breaks the game (a Mod Health finding to act on): fix that first;
 * 2. the last session's log names a mod in an exception: that mod, switched off to try;
 * 3. the game ran out of memory (its log, or Windows): the memory;
 * 4. nothing explains it: say so, and offer the hunt (halving the list) and a report to hand on.
 * Never a guess beyond what the findings and the log say.
 */
export type DoctorStep =
  | { kind: 'health'; id: string; issue: HealthIssue; count: number }
  | { kind: 'blamed'; id: string; mod: string; card: TranslatedErrorCard }
  | { kind: 'memory'; id: string; outOfMemory: number; windowsLow: number }
  | { kind: 'unknown'; id: string };

export interface DoctorInput {
  issues: HealthIssue[];
  /** The game's cards of the last session (the Monitor's). */
  cards: TranslatedErrorCard[];
  outOfMemory: number;
  windowsLow: number;
  /** Steps already tried (their ids): each is offered once. */
  tried: string[];
}

export const doctorStep = (input: DoctorInput): DoctorStep => {
  const tried = new Set(input.tried);
  const act = input.issues.filter((i) => issueStateOf(i) === 'act');
  if (act.length > 0 && !tried.has('health')) return { kind: 'health', id: 'health', issue: act[0], count: act.length };
  for (const card of input.cards) {
    if (card.log || card.severity !== 'EXCEPTION' || !card.mod_name || card.own_mod) continue;
    const id = `blamed:${card.mod_name}`;
    if (!tried.has(id)) return { kind: 'blamed', id, mod: card.mod_name, card };
  }
  if ((input.outOfMemory > 0 || input.windowsLow > 0) && !tried.has('memory')) {
    return { kind: 'memory', id: 'memory', outOfMemory: input.outOfMemory, windowsLow: input.windowsLow };
  }
  return { kind: 'unknown', id: 'unknown' };
};
