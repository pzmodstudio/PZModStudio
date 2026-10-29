import { describe, expect, it } from 'vitest';
import { planProfileCleanup } from './profileCleanup';
import { AppInstance, ModInfo } from '../types';

const mod = (mod_id: string, workshop_id?: string, extra: Partial<ModInfo> = {}): ModInfo => ({
  mod_id,
  name: mod_id,
  workshop_id,
  dependencies: [],
  enabled: false,
  load_order_index: 0,
  ...extra,
});

const profile = (id: string, active: string[], members?: string[]): AppInstance => ({
  id,
  name: id,
  created_at: '',
  is_active: false,
  active_mod_ids: active,
  load_order: active,
  members: members ?? null,
});

const mods: ModInfo[] = [
  mod('A', '100'),
  mod('B', '200'),
  mod('B2', '200'), // same Workshop item as B
  mod('C', '300'),
  mod('L'), // local folder
  mod('G', '400', { not_installed: true }),
  mod('Z_StudioLink', '3824256253'),
  mod('Z_PZModStudio_MergedPatch'),
];

describe('planProfileCleanup', () => {
  it('an item no other profile uses is exclusive; a shared one stays, naming who keeps it', () => {
    const plan = planProfileCleanup(
      profile('heavy', ['A', 'B', 'C', 'L']),
      [profile('light', ['C']), profile('coop', ['C'])],
      mods
    );
    expect(plan.exclusive.map((i) => i.workshop_id)).toEqual(['100', '200']);
    expect(plan.shared).toEqual([{ workshop_id: '300', mod_names: ['C'], used_by: ['coop', 'light'] }]);
    expect(plan.local_mod_names).toEqual(['L']);
  });

  it('one shared mod keeps the whole Workshop item, since Steam subscribes to items', () => {
    const plan = planProfileCleanup(profile('heavy', ['B']), [profile('light', ['B2'])], mods);
    expect(plan.exclusive).toEqual([]);
    expect(plan.shared[0]).toMatchObject({ workshop_id: '200', mod_names: ['B', 'B2'], used_by: ['light'] });
  });

  it('a mod kept in another profile but switched off there still keeps the item', () => {
    const plan = planProfileCleanup(profile('heavy', ['A']), [profile('light', [], ['A'])], mods);
    expect(plan.exclusive).toEqual([]);
    expect(plan.shared.map((i) => i.workshop_id)).toEqual(['100']);
  });

  it('the deleted profile itself in the list of others does not count as sharing', () => {
    const me = profile('heavy', ['A']);
    const plan = planProfileCleanup(me, [me], mods);
    expect(plan.exclusive.map((i) => i.workshop_id)).toEqual(['100']);
  });

  it("the Studio's own mods are never offered: the Link is in every profile, the packages are local", () => {
    const plan = planProfileCleanup(profile('only', ['Z_StudioLink', 'Z_PZModStudio_MergedPatch', 'A']), [], mods);
    expect(plan.exclusive.map((i) => i.workshop_id)).toEqual(['100']);
    expect(plan.local_mod_names).toEqual([]);
  });

  it('a mod that is no longer installed is left out: nothing to unsubscribe', () => {
    const plan = planProfileCleanup(profile('heavy', ['G']), [], mods);
    expect(plan).toEqual({ exclusive: [], shared: [], local_mod_names: [] });
  });
});
