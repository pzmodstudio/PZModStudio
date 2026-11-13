import { describe, expect, it } from 'vitest';
import type { ModuleCandidate, RequireAnswer } from '../types';
import { knownRequireFix, pickCandidate, related, requireVerdict, type ModTies } from './requireVerdict';

const candidate = (owner: string, owner_id: string, module_path: string, same_path = false): ModuleCandidate => ({ owner, owner_id, module_path, phase: 'client', same_path });
const answer = (wanted: string, candidates: ModuleCandidate[], installed_off: ModuleCandidate[] = []): RequireAnswer => ({ wanted, candidates, summary: '', installed_off });
const mod = (mod_id: string, name: string, dependencies: string[] = [], load_mod_after: string[] = []): ModTies => ({ mod_id, name, dependencies, load_mod_after });

const moodles = mod('MoodlesInLua', 'Moodles in lua');
const svu = mod('SVUCore', 'Standardized Vehicle Upgrades 3 - Core', ['\\tsarslib']);
const tsar = mod('tsarslib', "Tsar's Common Library");
const stranger = mod('Stranger', 'A Stranger');
const addon = mod('MoodlesAddon', 'Moodles Addon', ['MoodlesInLua']);
const mods = [moodles, svu, tsar, stranger, addon];

describe('a failed require of a module that is nowhere', () => {
  const nowhere = answer('LSMoodleManager', []);

  it('reads as an optional link only when the game carried on and the file did not fail later', () => {
    expect(requireVerdict({ answer: nowhere, picked: null, carriedOn: true, fileInLog: true, fileFailedLater: false })).toBe('optional');
  });

  it('says only that it is nowhere when an error from that file follows', () => {
    expect(requireVerdict({ answer: nowhere, picked: null, carriedOn: true, fileInLog: true, fileFailedLater: true })).toBe('nowhere');
  });

  it('says only that it is nowhere when the log cannot show what followed (no file on the line, or the line is not in the log in front)', () => {
    expect(requireVerdict({ answer: nowhere, picked: null, carriedOn: true, fileInLog: false, fileFailedLater: false })).toBe('nowhere');
  });

  it('says only that it is nowhere when the line is not a warning', () => {
    expect(requireVerdict({ answer: nowhere, picked: null, carriedOn: false, fileInLog: true, fileFailedLater: false })).toBe('nowhere');
  });

  it('names the mod that is installed and switched off when it has the module at that path', () => {
    const off = answer('LSMoodleManager', [], [candidate('Life Sim', 'Life', 'LSMoodleManager', true)]);
    expect(requireVerdict({ answer: off, picked: null, carriedOn: true, fileInLog: true, fileFailedLater: false })).toBe('optionalOff');
    expect(requireVerdict({ answer: off, picked: null, carriedOn: true, fileInLog: true, fileFailedLater: true })).toBe('nowhere');
  });

  it('offers no fix', () => {
    expect(knownRequireFix('LSMoodleManager', pickCandidate(nowhere, moodles, mods))).toBeNull();
  });
});

describe('a module of the same name in another mod', () => {
  it('has to do with the mod that asked when one of the two requires the other', () => {
    expect(related(candidate("Tsar's Common Library", 'tsarslib', 'Tuning2/ATATuning2'), svu, mods)).toBe(true);
    expect(related(candidate('Moodles in lua', 'MoodlesInLua', 'X/Manager'), addon, mods)).toBe(true);
    expect(related(candidate('Moodles Addon', 'MoodlesAddon', 'X/Manager'), moodles, mods)).toBe(true);
    expect(related(candidate('A Stranger', 'Stranger', 'Other/ATA2Tuning2'), svu, mods)).toBe(false);
  });

  it('is the game\'s or the mod\'s own: always tied', () => {
    expect(related(candidate('the game', '', 'Vehicles/Vehicles'), stranger, mods)).toBe(true);
    expect(related(candidate('A Stranger', 'Stranger', 'Mine/Util'), stranger, mods)).toBe(true);
  });

  it('gets no automatic fix when the only one is in an unrelated mod, and says it may be another module', () => {
    const a = answer('Tuning2/ATA2Tuning2', [candidate('A Stranger', 'Stranger', 'Other/ATA2Tuning2')]);
    const picked = pickCandidate(a, svu, mods);
    expect(picked?.tied).toBe(false);
    expect(knownRequireFix(a.wanted, picked)).toBeNull();
    expect(requireVerdict({ answer: a, picked, carriedOn: true, fileInLog: true, fileFailedLater: false })).toBe('movedUnrelated');
  });

  it('prefers the one in a mod tied to the asker over an unrelated one found first', () => {
    const a = answer('Tuning/ATATuning2', [candidate('A Stranger', 'Stranger', 'Other/ATATuning2'), candidate("Tsar's Common Library", 'tsarslib', 'Tuning2/ATATuning2')]);
    const picked = pickCandidate(a, svu, mods);
    expect(picked?.best.owner_id).toBe('tsarslib');
    expect(knownRequireFix(a.wanted, picked)).toEqual({ from: 'Tuning/ATATuning2', to: 'Tuning2/ATATuning2' });
    expect(requireVerdict({ answer: a, picked, carriedOn: true, fileInLog: true, fileFailedLater: false })).toBe('moved');
  });

  it('keeps the game\'s own moved module fixable, noise or not', () => {
    const a = answer('Vehicle/Vehicles', [candidate('the game', '', 'Vehicles/Vehicles')]);
    const picked = pickCandidate(a, undefined, mods);
    expect(knownRequireFix(a.wanted, picked)).toEqual({ from: 'Vehicle/Vehicles', to: 'Vehicles/Vehicles' });
    expect(requireVerdict({ answer: a, picked, carriedOn: true, fileInLog: true, fileFailedLater: false })).toBe('gameMovedNoise');
    expect(requireVerdict({ answer: a, picked, carriedOn: true, fileInLog: true, fileFailedLater: true })).toBe('gameMovedFailed');
  });

  it('a module at exactly the path asked for is about where it was asked from, with no path to fix', () => {
    const a = answer('TOC/Controllers/ItemsController', [candidate('A Stranger', 'Stranger', 'TOC/Controllers/ItemsController', true)]);
    const picked = pickCandidate(a, svu, mods);
    expect(requireVerdict({ answer: a, picked, carriedOn: true, fileInLog: true, fileFailedLater: false })).toBe('samePath');
    expect(knownRequireFix(a.wanted, picked)).toBeNull();
  });
});
