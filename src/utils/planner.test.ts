import { describe, expect, it } from 'vitest';
import { applyPlan, dependentsOf, enableWithRequirements, planIsSimple, planToggle } from './planner';
import { ModInfo } from '../types';

const mod = (mod_id: string, enabled: boolean, dependencies: string[] = [], extra: Partial<ModInfo> = {}) =>
  ({ mod_id, name: mod_id.toUpperCase(), enabled, dependencies, ...extra }) as unknown as ModInfo;

describe('switching a mod on', () => {
  it('brings what it needs, all the way down, and puts it before', () => {
    const mods = [mod('App', false, ['Lib']), mod('Lib', false, ['Core']), mod('Core', false), mod('Other', true)];
    const plan = planToggle(mods, 'App')!;
    expect(plan.enable).toBe(true);
    expect(plan.turnOn.map((s) => [s.mod_id, s.because])).toEqual([['Lib', 'APP'], ['Core', 'LIB']]);
    expect(planIsSimple(plan)).toBe(false);
    const after = applyPlan(mods, plan, true);
    expect(after.filter((m) => m.enabled).map((m) => m.mod_id)).toEqual(['Core', 'Lib', 'App', 'Other']);
  });

  it('says what is not installed and what the build cannot load, and only this mod changes when asked', () => {
    const mods = [mod('App', false, ['Gone', 'Old']), mod('Old', false, [], { loadable_on_installed_build: false })];
    const plan = planToggle(mods, 'App')!;
    expect(plan.missing).toEqual([{ id: 'Gone', because: 'APP' }]);
    expect(plan.blocked.map((s) => s.mod_id)).toEqual(['Old']);
    expect(plan.turnOn).toEqual([]);
    const only = applyPlan(mods, plan, false);
    expect(only.map((m) => [m.mod_id, m.enabled])).toEqual([['App', true], ['Old', false]]);
  });

  it('moves a requirement that is on but loads after the mod', () => {
    const mods = [mod('App', false, ['Lib']), mod('Lib', true)];
    const plan = planToggle(mods, 'App')!;
    expect(plan.turnOn).toEqual([]);
    expect(plan.moved.map((s) => s.mod_id)).toEqual(['Lib']);
    expect(applyPlan(mods, plan, true).map((m) => m.mod_id)).toEqual(['Lib', 'App']);
  });

  it('asks nothing when nothing else is involved', () => {
    const mods = [mod('Lib', true), mod('App', false, ['Lib'])];
    expect(planIsSimple(planToggle(mods, 'App')!)).toBe(true);
  });
});

describe('mods that arrived from a subscription', () => {
  it('go on with what they need, requirements first', () => {
    const mods = [mod('Old', true), mod('New', false, ['Frame']), mod('Frame', false)];
    const after = enableWithRequirements(mods, ['New', 'Frame']);
    expect(after.filter((m) => m.enabled).map((m) => m.mod_id)).toEqual(['Old', 'Frame', 'New']);
  });
});

describe('switching a mod off', () => {
  it('names what is on and needs it, through other mods too', () => {
    const mods = [mod('Core', true), mod('Lib', true, ['Core']), mod('App', true, ['Lib']), mod('Free', true)];
    const plan = planToggle(mods, 'Core')!;
    expect(plan.turnOff.map((s) => [s.mod_id, s.because])).toEqual([['Lib', 'CORE'], ['App', 'LIB']]);
    expect(applyPlan(mods, plan, true).filter((m) => m.enabled).map((m) => m.mod_id)).toEqual(['Free']);
    expect(applyPlan(mods, plan, false).filter((m) => m.enabled).map((m) => m.mod_id)).toEqual(['Lib', 'App', 'Free']);
  });
});

describe('what needs the mods about to go', () => {
  it('names the installed mods whose require= points at them, the ones that are on first', () => {
    const mods = [mod('Lib', true), mod('Glass', false), mod('Off', false, ['Lib']), mod('On', true, ['Glass']), mod('Other', true, ['Core']), mod('Gone', true, ['Lib'], { not_installed: true })];
    expect(dependentsOf(mods, ['Lib', 'Glass']).map((d) => [d.mod_id, d.enabled, d.because])).toEqual([['On', true, 'GLASS'], ['Off', false, 'LIB']]);
    expect(dependentsOf(mods, ['Other'])).toEqual([]);
  });
});
