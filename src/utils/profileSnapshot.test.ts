import { describe, expect, it } from 'vitest';
import type { CatalogModSummary, HealthIssue, ModInfo } from '../types';
import { compareSnapshots, diffIsEmpty, takeSnapshot } from './profileSnapshot';

const mod = (mod_id: string, enabled: boolean, version?: string): ModInfo => ({ mod_id, name: mod_id, enabled, version, dependencies: [], load_order_index: 0 });
const cat = (id: string, installed_time: number | null): CatalogModSummary =>
  ({ id, workshop_state: installed_time === null ? null : { installed_time, latest_time: null, size: null, update_pending: false, updated_since_last_play: false } }) as unknown as CatalogModSummary;
const issue = (code: string, mod_id: string, message = code): HealthIssue => ({ severity: 'warning', state: 'recommended', code, mod_id, message, fixes: [], related: [] });
const profile = { id: 'p1', name: 'Test' };

describe('a photo of a profile, compared', () => {
  it('nothing changed reads as nothing', () => {
    const mods = [mod('A', true), mod('B', false)];
    const a = takeSnapshot(profile, 'one', mods, [cat('A', 100)], [issue('x', 'A')], '42.22', 1);
    const b = takeSnapshot(profile, 'two', mods, [cat('A', 100)], [issue('x', 'A')], '42.22', 2);
    expect(diffIsEmpty(compareSnapshots(a, b))).toBe(true);
  });

  it('says what came in, went, switched, updated, and which findings appeared or went', () => {
    const a = takeSnapshot(profile, 'before', [mod('A', true, '1.0'), mod('B', true), mod('C', false), mod('Gone', true)], [cat('A', 100)], [issue('old', 'B')], '42.21', 1);
    const b = takeSnapshot(profile, 'after', [mod('A', true, '1.1'), mod('B', false), mod('C', true), mod('New', true)], [cat('A', 200)], [issue('fresh', 'A')], '42.22', 2);
    const d = compareSnapshots(a, b);
    expect(d.game).toEqual({ before: '42.21', after: '42.22' });
    expect(d.added.map((m) => m.id)).toEqual(['New']);
    expect(d.removed.map((m) => m.id)).toEqual(['Gone']);
    expect(d.switchedOn.map((m) => m.id)).toEqual(['C']);
    expect(d.switchedOff.map((m) => m.id)).toEqual(['B']);
    expect(d.updated.map((u) => [u.mod.id, u.fromVersion, u.toVersion])).toEqual([['A', '1.0', '1.1']]);
    expect(d.newFindings.map((f) => f.code)).toEqual(['fresh']);
    expect(d.goneFindings.map((f) => f.code)).toEqual(['old']);
  });

  it('a mod switched on does not make every mod after it count as moved; a real swap does', () => {
    const a = takeSnapshot(profile, '', [mod('A', true), mod('B', true), mod('C', true)], [], [], null, 1);
    const inserted = takeSnapshot(profile, '', [mod('X', true), mod('A', true), mod('B', true), mod('C', true)], [], [], null, 2);
    expect(compareSnapshots(a, inserted).moved).toEqual([]);
    const swapped = takeSnapshot(profile, '', [mod('B', true), mod('A', true), mod('C', true)], [], [], null, 3);
    expect(compareSnapshots(a, swapped).moved.map((m) => [m.mod.id, m.from, m.to])).toEqual([['B', 2, 1], ['A', 1, 2]]);
  });
});
