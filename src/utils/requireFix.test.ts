import { describe, expect, it } from 'vitest';
import { applyRequireFix, packageForPatch } from './requireFix';

describe('applyRequireFix', () => {
  const fix = { from: 'Vehicle/ISVehiclePartMenu', to: 'Vehicles/ISUI/ISVehiclePartMenu' };

  it('rewrites the require in every spelling the game accepts, and reports the first line changed', () => {
    const file = ['-- header', 'require "Vehicle/ISVehiclePartMenu"', "local a = require('Vehicle/ISVehiclePartMenu')", 'require( "Vehicle/ISVehiclePartMenu" )'].join('\n');
    const out = applyRequireFix(file, fix);
    expect(out?.line).toBe(2);
    expect(out?.content.split('\n')).toEqual([
      '-- header',
      'require "Vehicles/ISUI/ISVehiclePartMenu"',
      "local a = require('Vehicles/ISUI/ISVehiclePartMenu')",
      'require( "Vehicles/ISUI/ISVehiclePartMenu" )',
    ]);
  });

  it('leaves a file alone that does not ask for the module, or asks for a longer name', () => {
    expect(applyRequireFix('x = 1', fix)).toBeNull();
    expect(applyRequireFix('require "Vehicle/ISVehiclePartMenuX"', fix)).toBeNull();
    expect(applyRequireFix('require "Vehicles/ISUI/ISVehiclePartMenu"', fix)).toBeNull();
  });

  it('treats the module as text, not as a pattern', () => {
    expect(applyRequireFix('require "a.b"', { from: 'a.b', to: 'c' })?.content).toBe('require "c"');
    expect(applyRequireFix('require "aXb"', { from: 'a.b', to: 'c' })).toBeNull();
  });
});

describe('packageForPatch', () => {
  const pkg = (folder_name: string, extra: Partial<{ is_companion_bridge: boolean; is_active_in_modlist: boolean; is_packaged: boolean }> = {}) => ({
    folder_name,
    is_companion_bridge: false,
    is_active_in_modlist: false,
    is_packaged: false,
    ...extra,
  });

  it('prefers the package the game is reading, never the bridge, and falls back to the default name', () => {
    expect(packageForPatch([pkg('Z_PZModStudio_Bridge', { is_companion_bridge: true, is_active_in_modlist: true, is_packaged: true }), pkg('Z_PZModStudio_A'), pkg('Z_PZModStudio_B', { is_active_in_modlist: true, is_packaged: true })])).toBe('Z_PZModStudio_B');
    expect(packageForPatch([pkg('Z_PZModStudio_Bridge', { is_companion_bridge: true }), pkg('Z_PZModStudio_A')])).toBe('Z_PZModStudio_A');
    expect(packageForPatch([])).toBe('Z_PZModStudio_MergedPatch');
  });
});
