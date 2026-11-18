import { describe, expect, it } from 'vitest';
import { certaintyOf } from './certainty';

describe('how sure a finding is', () => {
  it("the game's rules and files are rules", () => {
    for (const code of ['missing_dependency', 'dependency_not_enabled', 'load_after_order', 'file_conflict', 'not_for_build', 'incompatible']) expect(certaintyOf({ code })).toBe('rule');
  });
  it('what the log showed is seen', () => {
    expect(certaintyOf({ code: 'floods_console' })).toBe('seen');
    expect(certaintyOf({ code: 'community_known_issue', params: { seenKey: 'verified_on' } })).toBe('seen');
    expect(certaintyOf({ code: 'function_overridden', params: { checkedKey: 'fo_seen', certain: 'load' } })).toBe('seen');
  });
  it('a function cut made while the files load is a rule; one made later stays a suspicion', () => {
    expect(certaintyOf({ code: 'function_overridden', params: { certain: 'load' } })).toBe('rule');
    expect(certaintyOf({ code: 'function_overridden', params: { certain: '' } })).toBe('suspicion');
  });
  it('code reading and reports are suspicions', () => {
    for (const code of ['api_missing', 'function_overridden', 'security_lua', 'java_patch_overlap']) expect(certaintyOf({ code })).toBe('suspicion');
    expect(certaintyOf({ code: 'community_known_issue', params: { seenKey: 'reported' } })).toBe('suspicion');
    expect(certaintyOf({ code: 'community_incompatible', params: { seenKey: 'reported_on' } })).toBe('suspicion');
    expect(certaintyOf({ code: 'community_incompatible', params: { seenKey: 'verified' } })).toBe('rule');
  });
});
