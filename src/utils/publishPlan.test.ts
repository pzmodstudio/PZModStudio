import { describe, expect, it } from 'vitest';
import type { VfsConflict } from '../types';
import { publishPlan } from './publishPlan';

const c = (relative_path: string, status: VfsConflict['status'], auto_mergeable: boolean, merged = `merged ${relative_path}`): VfsConflict => ({
  id: relative_path,
  relative_path,
  file_type: 'LUA',
  base_content: 'base',
  competing_mods: [],
  merged_output: merged,
  status,
  auto_mergeable,
});

describe('what a publish puts in the package', () => {
  it('on Update patch, merges again what merges by itself and asks again where a mod changed a decided file', () => {
    const fresh = (p: string, status: VfsConflict['status'], auto: boolean): VfsConflict => ({ ...c(p, status, auto, 'kept from last time'), auto_ast_output: 'merged from the mods now' });
    const plan = publishPlan([fresh('auto.lua', 'RESOLVED', true), fresh('decided.lua', 'RESOLVED', false), fresh('changed.lua', 'RESOLVED', false)], { stalePaths: new Set(['changed.lua']) });
    expect(plan.ship).toEqual([
      { relative_path: 'auto.lua', content: 'merged from the mods now' },
      { relative_path: 'decided.lua', content: 'kept from last time' },
    ]);
    expect(plan.waiting.map((w) => w.relative_path)).toEqual(['changed.lua']);
  });

  it('on Update patch, a file where the player kept one mod stays that mod, even if it could merge', () => {
    const chosen: VfsConflict = { ...c('chosen.lua', 'RESOLVED', true, 'mod A file'), auto_ast_output: 'merged', competing_mods: [{ mod_id: 'A', mod_name: 'A', absolute_path: '', content: 'mod A file' }] };
    expect(publishPlan([chosen], { stalePaths: new Set() }).ship).toEqual([{ relative_path: 'chosen.lua', content: 'mod A file' }]);
  });

  it('ships what the player decided and what merges without a decision, and holds the rest', () => {
    const plan = publishPlan([
      c('decided.lua', 'RESOLVED', false, 'only mod A'),
      c('separate-lines.lua', 'MANUAL_CONFLICT', true),
      c('same-file-twice.lua', 'AUTO_MERGED', false),
      c('same-lines.lua', 'MANUAL_CONFLICT', false),
    ]);
    expect(plan.ship).toEqual([
      { relative_path: 'decided.lua', content: 'only mod A' },
      { relative_path: 'separate-lines.lua', content: 'merged separate-lines.lua' },
      { relative_path: 'same-file-twice.lua', content: 'merged same-file-twice.lua' },
    ]);
    expect(plan.waiting.map((w) => w.relative_path)).toEqual(['same-lines.lua']);
  });
});
