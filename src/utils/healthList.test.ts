import { describe, expect, it } from 'vitest';
import { fixActsOn, listDifference, serverListIds } from './healthList';

describe("a server's list in Mod Health", () => {
  it("reads Mods= as the game does: backslashes off, order kept, once each", () => {
    expect(serverListIds(['\\ZombieBuddy', 'Viewpoint', ' \\Viewpoint ', '', 'KnoxLives'])).toEqual(['ZombieBuddy', 'Viewpoint', 'KnoxLives']);
  });

  it('offers fixes to files on any list, and fixes to the list only on the profile', () => {
    const profile = { kind: 'profile' } as const;
    const server = { kind: 'server', name: 'knoxtest' } as const;
    expect(fixActsOn('move_before', profile)).toBe(true);
    expect(fixActsOn('move_before', server)).toBe(false);
    expect(fixActsOn('enable_mod', server)).toBe(false);
    expect(fixActsOn('make_compatible_copy', server)).toBe(true);
    expect(fixActsOn('merge_files', server)).toBe(false);
    expect(fixActsOn('remove_from_package', server)).toBe(false);
    expect(fixActsOn('merge_files', profile)).toBe(true);
  });

  it('says what only the profile has and what only the server has', () => {
    expect(listDifference(['A', 'B', 'C'], ['B', 'D'])).toEqual({ onlyProfile: ['A', 'C'], onlyServer: ['D'] });
    expect(listDifference(['A'], ['A'])).toEqual({ onlyProfile: [], onlyServer: [] });
  });
});
