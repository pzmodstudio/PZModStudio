import { describe, expect, it } from 'vitest';
import { profileModCounts } from './profiles';

describe('profileModCounts', () => {
  it('counts the mods that are on and all the mods of the profile, on and off', () => {
    expect(profileModCounts({ active_mod_ids: ['A', 'B'], members: ['A', 'B', 'C', 'D'] })).toMatchObject({ active: 2, total: 4 });
  });

  it('counts a mod that is on but missing from the members', () => {
    expect(profileModCounts({ active_mod_ids: ['A', 'B'], members: ['A'] })).toMatchObject({ active: 2, total: 2 });
  });

  it('leaves the Studio Link out, as the Mod List does', () => {
    expect(profileModCounts({ active_mod_ids: ['A', 'Z_StudioLink'], members: ['A', 'Z_StudioLink', 'C'] })).toMatchObject({ active: 1, total: 2 });
  });

  it('leaves out the mods that are off and no longer installed, never the ones that are on', () => {
    const installed = new Map<string, string | null>([['A', '1'], ['C', '2']]);
    expect(profileModCounts({ active_mod_ids: ['A', 'Gone'], members: ['A', 'C', 'Old', 'Gone'] }, installed)).toMatchObject({ active: 2, total: 3 });
  });

  it('counts each sub-mod, and the Workshop items they come from once', () => {
    const installed = new Map<string, string | null>([['Main', '111'], ['MainPatch', '111'], ['Other', '222'], ['Local', null]]);
    expect(profileModCounts({ active_mod_ids: ['Main', 'MainPatch'], members: ['Main', 'MainPatch', 'Other', 'Local'] }, installed)).toEqual({ active: 2, total: 4, workshopItems: 2 });
  });

  it('reads a B42 id with its backslash as the same mod', () => {
    expect(profileModCounts({ active_mod_ids: [String.fromCharCode(92) + 'A'], members: ['A', 'B'] })).toMatchObject({ active: 1, total: 2 });
  });
});
