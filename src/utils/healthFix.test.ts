import { describe, expect, it } from 'vitest';
import { applyFix } from './healthFix';
import { ModInfo } from '../types';

const mod = (mod_id: string, enabled = true, extra: Partial<ModInfo> = {}): ModInfo =>
  ({ mod_id, name: mod_id, enabled, dependencies: [], load_order_index: 0, ...extra }) as unknown as ModInfo;

describe('applyFix', () => {
  const list = [mod('A'), mod('B', false), mod('C')];

  it('switches a mod on and off, and says nothing when it already is', () => {
    expect(applyFix(list, { action: 'enable_mod', mod_id: 'B' })?.find((m) => m.mod_id === 'B')?.enabled).toBe(true);
    expect(applyFix(list, { action: 'enable_mod', mod_id: 'A' })).toBeNull();
    expect(applyFix(list, { action: 'disable_mod', mod_id: 'A' })?.find((m) => m.mod_id === 'A')?.enabled).toBe(false);
    expect(applyFix(list, { action: 'enable_mod', mod_id: 'nope' })).toBeNull();
  });

  it('never switches on a draft package: the game cannot read one', () => {
    const draft = mod('Z_PZModStudio_Test', false, { is_packaged: false });
    expect(applyFix([...list, draft], { action: 'enable_mod', mod_id: 'Z_PZModStudio_Test' })).toBeNull();
  });

  it('moves a mod before the one it must precede, and leaves a list already in order alone', () => {
    const moved = applyFix(list, { action: 'move_before', mod_id: 'C', before: 'A' });
    expect(moved?.map((m) => m.mod_id)).toEqual(['C', 'A', 'B']);
    expect(applyFix(list, { action: 'move_before', mod_id: 'A', before: 'C' })).toBeNull();
    expect(applyFix(list, { action: 'move_before', mod_id: 'A', before: 'nope' })).toBeNull();
  });

  it('removes an entry, and the list is untouched by what it does not hold', () => {
    expect(applyFix(list, { action: 'remove_from_list', entry: 'B' })?.map((m) => m.mod_id)).toEqual(['A', 'C']);
    expect(applyFix(list, { action: 'remove_from_list', entry: 'nope' })).toBeNull();
  });

  it('is not for fixes that act on disk or open a page', () => {
    expect(applyFix(list, { action: 'subscribe_workshop', hint: 'x' })).toBeNull();
    expect(applyFix(list, { action: 'open_in_merger', relative_path: 'media/x.lua' })).toBeNull();
  });

  it('returns a new list and leaves the given one as it was', () => {
    const before = list.map((m) => ({ ...m }));
    applyFix(list, { action: 'disable_mod', mod_id: 'A' });
    expect(list).toEqual(before);
  });
});
