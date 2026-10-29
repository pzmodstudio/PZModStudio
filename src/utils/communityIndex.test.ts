import { describe, expect, it } from 'vitest';
import { draftRegistry, knownIssues, proposeUrl, registryFileName, withoutUserName } from './communityIndex';
import { CatalogModSummary, ModErrorStat, ModInfo } from '../types';

const mod = {
  mod_id: '2858201402/dailystatisticsandgains',
  name: 'Daily Statistics and Gains',
  author: 'Someone',
  workshop_id: '2858201402',
  dependencies: ['ModOptions'],
  incompatible: ['OtherStats'],
  enabled: true,
  load_order_index: 3,
} as unknown as ModInfo;

describe('draftRegistry', () => {
  it('fills the registry from mod.info and the build it runs on here', () => {
    const r = draftRegistry(mod, undefined, '42.21');
    expect(r.id).toBe(mod.mod_id);
    expect(r.workshop_id).toBe('2858201402');
    expect(r.builds).toEqual(['42']);
    expect(r.requires).toEqual(['ModOptions']);
    expect(r.incompatible[0]).toMatchObject({ id: 'OtherStats', status: 'verified' });
    expect(r.issues).toEqual([]);
  });

  it('claims no build for a mod this build cannot load', () => {
    expect(draftRegistry({ ...mod, loadable_on_installed_build: false }, undefined, '42.21').builds).toEqual([]);
  });
});

describe('knownIssues', () => {
  it('is empty when the Studio saw nothing of the mod', () => {
    expect(knownIssues(undefined, undefined, '42.21')).toEqual([]);
    expect(knownIssues(undefined, { mod_name: 'X', exceptions: 0, warnings: 0, last_exception: null, last_warning: null }, '42.21')).toEqual([]);
  });

  it("turns the last session's counts and a compatible copy into verified issues on the installed build", () => {
    const stat: ModErrorStat = { mod_name: 'X', exceptions: 2, warnings: 1, last_exception: 'ERROR: x.lua:3', last_warning: 'WARN: y' };
    const catalog = { origin: { kind: 'compat', workshop_id: null, source_dir: '', changes: ['require ISUI/ISWorldMap -> ISUI/Maps/ISWorldMap'] } } as unknown as CatalogModSummary;
    const issues = knownIssues(catalog, stat, '42.21');
    expect(issues.map((i) => i.title)).toEqual(['Threw 2 errors in the last session', '1 warning in the last session (the game carried on)', 'Needed a compatible copy on this build']);
    expect(issues[0]).toMatchObject({ log: 'ERROR: x.lua:3', build: '42.21', status: 'verified', fix: { kind: 'none' } });
    expect(issues[2]).toMatchObject({ fix: { kind: 'compatible_copy' }, note: 'The copy changed: require ISUI/ISWorldMap -> ISUI/Maps/ISWorldMap' });
    expect(draftRegistry(mod, undefined, '42.21', issues).issues).toHaveLength(3);
  });
});

describe('proposeUrl', () => {
  it('opens the new-file page with the file name and the JSON filled in', () => {
    const url = proposeUrl(draftRegistry(mod, undefined, '42.21'));
    expect(url).toContain('/new/main/community/index?filename=2858201402__dailystatisticsandgains.json&value=');
    expect(decodeURIComponent(url.split('&value=')[1])).toContain('"id": "2858201402/dailystatisticsandgains"');
    expect(registryFileName('Plain')).toBe('Plain.json');
  });
});

describe('withoutUserName', () => {
  it('keeps the Windows user name out of a shared log line', () => {
    expect(withoutUserName('ERROR at C:\\Users\\Javier Pérez\\Zomboid\\mods\\X\\a.lua line 3')).toBe('ERROR at C:\\Users\\<user>\\Zomboid\\mods\\X\\a.lua line 3');
    expect(withoutUserName('at C:/Users/bob/Zomboid/x.lua')).toBe('at C:/Users/<user>/Zomboid/x.lua');
    expect(withoutUserName(null)).toBeUndefined();
  });
});
