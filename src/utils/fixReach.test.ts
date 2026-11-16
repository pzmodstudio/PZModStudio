import { describe, expect, it } from 'vitest';
import { fixReach, reachLines } from './fixReach';

const servers = [
  { name: 'knoxtest', mods: ['\\ZombieBuddy', 'KnoxLives', 'Z_PZModStudio_Knox_Lives_MergePatch_Test'] },
  { name: 'servertest', mods: ['ZombieBuddy', 'CleanUI'] },
];

describe('where a fix lands', () => {
  it('a package change reaches the profile and the servers that load the package, and names those that do not', () => {
    const r = fixReach('package', 'Z_PZModStudio_Knox_Lives_MergePatch_Test', 'Knox Lives Test', servers);
    expect(r.serversWith).toEqual(['knoxtest']);
    expect(r.serversWithout).toEqual(['servertest']);
    expect(reachLines(r).map((l) => l.key)).toEqual(['fixReach.packageMods', 'fixReach.packageProfile', 'fixReach.packageServers', 'fixReach.packageNotServers']);
  });

  it('switching a mod off is the profile only; a server that lists the mod still loads it, one that does not is not mentioned', () => {
    const r = fixReach('switch_off', '\\ZombieBuddy', 'Main', servers);
    expect(r.serversWith).toEqual(['knoxtest', 'servertest']);
    expect(r.serversWithout).toEqual([]);
    const cleanUi = fixReach('switch_off', 'KnoxLives', 'Main', servers);
    expect(cleanUi.serversWith).toEqual(['knoxtest']);
    expect(reachLines(cleanUi).map((l) => l.key)).toEqual(['fixReach.offProfile', 'fixReach.offStillServers']);
  });

  it('with no servers it says only where it went', () => {
    expect(reachLines(fixReach('package', 'Z_PZModStudio_X', 'Main', [])).map((l) => l.key)).toEqual(['fixReach.packageMods', 'fixReach.packageProfile']);
  });
});
