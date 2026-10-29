import { describe, expect, it } from 'vitest';
import { findInstalledDependencyInMods, normalizeModId } from './modLookup';
import { ModInfo } from '../../types';

const mod = (mod_id: string, name: string, enabled = true, workshop_id?: string): ModInfo =>
  ({ mod_id, name, enabled, workshop_id, dependencies: [], load_order_index: 0 }) as unknown as ModInfo;

describe('normalizeModId', () => {
  it('reads a require= value the way the game compares it', () => {
    expect(normalizeModId('  \\ModID ')).toBe('modid');
    expect(normalizeModId('"Quoted"')).toBe('quoted');
  });
});

describe('findInstalledDependencyInMods', () => {
  const mods = [
    mod('TheOnlyCure', 'The Only Cure'),
    mod('TOC_Addon', 'TOC Addon', false),
    mod('NeatUI_Framework', 'NeatUI Framework', true, '12345'),
    mod('neat-ui-framework', 'Other copy', false),
  ];

  it('takes an exact id, Workshop id or name before anything looser', () => {
    expect(findInstalledDependencyInMods('TheOnlyCure', mods)?.mod_id).toBe('TheOnlyCure');
    expect(findInstalledDependencyInMods('12345', mods)?.mod_id).toBe('NeatUI_Framework');
    expect(findInstalledDependencyInMods('The Only Cure', mods)?.mod_id).toBe('TheOnlyCure');
  });

  it('matches on letters and digits alone when punctuation differs, preferring the mod that is on', () => {
    expect(findInstalledDependencyInMods('neatuiframework', mods)?.mod_id).toBe('NeatUI_Framework');
  });

  it('falls back to a prefix with a separator, and gives nothing for a stranger', () => {
    expect(findInstalledDependencyInMods('TOC', mods)?.mod_id).toBe('TOC_Addon');
    expect(findInstalledDependencyInMods('NotInstalled', mods)).toBeUndefined();
    expect(findInstalledDependencyInMods('   ', mods)).toBeUndefined();
  });
});
