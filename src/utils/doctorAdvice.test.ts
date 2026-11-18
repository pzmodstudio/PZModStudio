import { describe, expect, it } from 'vitest';
import type { TFunction } from 'i18next';
import { crashAdvice, onsetAdvice } from './doctorAdvice';
import { BrainJvmCrash, BrainOnset } from '../types';

/** The key said, with its values: what the advice chose to say. */
const t = ((key: string, v?: Record<string, unknown>) => `${key}${v ? ` ${JSON.stringify(v)}` : ''}`) as unknown as TFunction;
const name = (id: string) => id;
const crash = (at: number, kind: BrainJvmCrash['kind'], role: BrainJvmCrash['role'] = 'client', frame = ''): BrainJvmCrash => ({ file: `hs_err_${at}.log`, at, elapsed: 60, role, kind, reason: '', frame });

describe('doctorAdvice', () => {
  it('says the PC ran out of memory when most crashes were that, and that processes were open together', () => {
    const a = crashAdvice([crash(1000, 'out_of_memory'), crash(1010, 'out_of_memory', 'server'), crash(5000, 'fatal', 'client', 'KERNELBASE.dll')], t);
    expect(a).toContain('doctorChat.advice.memory');
    expect(a).toContain('"memory":2');
    expect(a).toContain('doctorChat.advice.together');
    expect(crashAdvice([crash(1, 'fatal', 'client', 'ZNetJNI64.dll')], t)).toContain('doctorChat.advice.net');
    expect(crashAdvice([], t)).toBeNull();
  });

  it('names what an error began with: the mod\'s own update, the game\'s, or mods added then', () => {
    const onset = (changes: BrainOnset['changes'], more = 0): BrainOnset => ({ first_bad: 2, last_good: 1, changes, more });
    expect(onsetAdvice(onset([{ kind: 'updated', mod_id: 'A', from: '1', to: '2' }]), 'A', name, t)).toContain('updatedSelf');
    expect(onsetAdvice(onset([{ kind: 'build', mod_id: '', from: '42.21', to: '42.22' }]), 'A', name, t)).toContain('advice.build');
    expect(onsetAdvice(onset([{ kind: 'added', mod_id: 'B', from: '', to: '' }]), 'A', name, t)).toContain('advice.added');
    // many changes at once: no single thing to point at
    expect(onsetAdvice(onset([{ kind: 'added', mod_id: 'B', from: '', to: '' }], 40), 'A', name, t)).toBeNull();
    expect(onsetAdvice(null, 'A', name, t, 'java_mod_turned_off')).toContain('switchedOff');
  });
});
