import { describe, expect, it } from 'vitest';
import { pickCollectionMods, PickMod } from './collectionPick';

const mod = (mod_id: string, workshop_id: string, incompatible: string[] = [], loadable: boolean | null = true): PickMod => ({
  mod_id,
  name: mod_id,
  workshop_id,
  incompatible,
  loadable_on_installed_build: loadable,
});

describe("a collection's mods, switched on", () => {
  it('a main mod and its add-ons all go on', () => {
    const r = pickCollectionMods(['1'], [mod('69mini', '1'), mod('69mini_MrBean', '1'), mod('69mini_ItalianJob', '1')]);
    expect(r.on).toEqual(['69mini', '69mini_MrBean', '69mini_ItalianJob']);
    expect(r.off).toEqual([]);
  });

  it('of alternatives that rule each other out, one goes on and the rest wait, named', () => {
    const all = ['DBFaster25', 'DBFaster50', 'DBFaster60'];
    const mods = all.map((id) => mod(id, '2', all.filter((x) => x !== id).map((x) => `\\${x}`)));
    const r = pickCollectionMods(['2'], mods);
    expect(r.on).toEqual(['DBFaster25']);
    expect(r.off).toEqual(['DBFaster50', 'DBFaster60']);
    expect(r.choices).toEqual([{ picked: 'DBFaster25', left: ['DBFaster50', 'DBFaster60'] }]);
  });

  it('one side naming the other is enough; the version for this build wins over one for another', () => {
    const r = pickCollectionMods(['3'], [mod('Ladders42131', '3', ['Ladders4222'], false), mod('Ladders4222', '3', [], true)]);
    expect(r.on).toEqual(['Ladders4222']);
    expect(r.off).toEqual(['Ladders42131']);
  });

  it('keeps the collection order across items, and mods of no item in it are left out', () => {
    const r = pickCollectionMods(['b', 'a'], [mod('A', 'a'), mod('B', 'b'), mod('Other', 'z')]);
    expect(r.on).toEqual(['B', 'A']);
  });
});
