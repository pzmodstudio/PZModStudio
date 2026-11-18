import { describe, expect, it } from 'vitest';
import type { HealthFix, HealthIssue, IssueState } from '../types';
import { safeFixPlan } from './fixAll';
import { simpleFindings } from './uiMode';

const issue = (code: string, state: IssueState, fixes: HealthFix[]): HealthIssue => ({
  severity: 'info',
  state,
  code,
  mod_id: code,
  message: code,
  fixes,
  related: [],
});

const needsOff = issue('dependency_not_enabled', 'act', [{ action: 'enable_mod', mod_id: 'lib' }]);
const tooEarly = issue('load_after_order', 'recommended', [{ action: 'move_before', mod_id: 'a', before: 'b' }]);
const yourCall = issue('script_definition_conflict', 'recommended', [{ action: 'move_before', mod_id: 'c', before: 'd' }]);
const offOnly = issue('incompatible', 'act', [{ action: 'disable_mod', mod_id: 'e' }]);
const all = [needsOff, tooEarly, yourCall, offOnly];

describe('Fix N problems automatically', () => {
  it('in Complete takes every safe fix, never a matter of taste nor switching a mod off', () => {
    const plan = safeFixPlan(all);
    expect(plan.issues).toEqual([needsOff, tooEarly]);
    expect(plan.enabling).toBe(1);
    expect(plan.moving).toBe(1);
  });

  it('in Simple counts and applies only the rows Simple shows', () => {
    const shown = new Set(simpleFindings(all, null, (i) => i.code).shown);
    const plan = safeFixPlan(all, (i) => shown.has(i));
    expect(plan.issues).toEqual([needsOff]);
    expect(plan.fixes).toEqual([{ action: 'enable_mod', mod_id: 'lib' }]);
    expect(plan.moving).toBe(0);
  });

  it('counts a library two mods need once', () => {
    const a = issue('dependency_not_enabled', 'act', [{ action: 'enable_mod', mod_id: 'lib' }, { action: 'move_before', mod_id: 'lib', before: 'a' }]);
    const b = issue('dependency_not_enabled', 'act', [{ action: 'enable_mod', mod_id: 'lib' }, { action: 'move_before', mod_id: 'lib', before: 'b' }]);
    const plan = safeFixPlan([a, b]);
    expect(plan.issues).toHaveLength(2);
    expect(plan.enabling).toBe(1);
    expect(plan.moving).toBe(1);
  });

  it('has nothing to do when no shown row has a safe fix', () => {
    const plan = safeFixPlan(all, (i) => i === offOnly);
    expect(plan.issues).toEqual([]);
    expect(plan.fixes).toEqual([]);
  });
});
