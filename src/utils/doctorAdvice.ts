import type { TFunction } from 'i18next';
import { BrainFixOutcome, BrainJvmCrash, BrainOnset } from '../types';

/**
 * The Doctor's advice: from the facts the brain has, the one thing worth trying, said with its
 * evidence. Each rule needs its facts to say anything; none guesses past them.
 */

/** Crashes: most of them because the PC ran out of memory, or in the game's own network library. */
export const crashAdvice = (crashes: BrainJvmCrash[], t: TFunction): string | null => {
  if (crashes.length === 0) return null;
  const memory = crashes.filter((c) => c.kind === 'out_of_memory');
  if (memory.length >= Math.max(2, crashes.length * 0.6)) {
    // two of the game's processes falling the same minute: they were open together
    const together = memory.some((c) => memory.some((o) => o !== c && Math.abs(o.at - c.at) <= 60)) || memory.some((c) => c.role === 'server');
    return t('doctorChat.advice.memory', { memory: memory.length, count: crashes.length, together: together ? t('doctorChat.advice.together') : '' });
  }
  if (crashes.some((c) => c.kind === 'fatal' && /znetjni/i.test(c.frame))) return t('doctorChat.advice.net');
  return null;
};

/** When an error began: the mod's own update, the game's, or mods added right then. */
export const onsetAdvice = (onset: BrainOnset | null | undefined, modId: string, name: (id: string) => string, t: TFunction, code?: string): string | null => {
  if (code === 'java_mod_turned_off') return t('doctorChat.advice.switchedOff', { mod: name(modId) });
  if (!onset || !onset.last_good) return null;
  const build = onset.changes.find((c) => c.kind === 'build');
  if (onset.changes.some((c) => c.kind === 'updated' && c.mod_id === modId)) return t('doctorChat.advice.updatedSelf', { mod: name(modId) });
  if (build) return t('doctorChat.advice.build', { from: build.from, to: build.to });
  const added = onset.changes.filter((c) => c.kind === 'added');
  if (added.length > 0 && added.length <= 3 && onset.changes.length === added.length && onset.more === 0) {
    const mods = added.map((c) => name(c.mod_id)).join(', ');
    return t('doctorChat.advice.added', { mods, count: added.length });
  }
  return null;
};

/** Loading got slower when mods were added. */
export const loadAdvice = (onset: BrainOnset | null | undefined, name: (id: string) => string, t: TFunction): string | null => {
  const added = onset?.changes.filter((c) => c.kind === 'added') ?? [];
  if (added.length === 0 || added.length > 4) return null;
  return t('doctorChat.advice.loadAdded', { mods: added.map((c) => name(c.mod_id)).join(', '), count: added.length });
};

/** A fix that was not enough. */
export const fixAdvice = (fixes: BrainFixOutcome[], t: TFunction): string | null =>
  fixes.some((f) => f.verdict === 'not_worked' && f.fix.action !== 'disable_mod') ? t('doctorChat.advice.fixNot') : null;
