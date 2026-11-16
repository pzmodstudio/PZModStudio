/**
 * Which mods of a Workshop collection go in switched on. A Workshop item can hold several mods:
 * mostly a main mod and add-ons that require it (extra textures, themed versions), which add up
 * and all go on; but sometimes alternatives of one thing (Drag Bodies Faster 25/50/60/70/80, a
 * version of a mod per game build, a normal and a "bright" look), which the mods themselves
 * declare with `incompatible=`. Switching every mod on put both of each pair on (26 "incompatible"
 * findings in a 319-item collection). Of each group of mods of one item that rule each other
 * out, one goes on — the first that loads on the installed build, in the item's order — and the
 * rest go in switched off, named, for the player to choose.
 */
export interface PickMod {
  mod_id: string;
  name: string;
  workshop_id?: string;
  incompatible?: string[];
  loadable_on_installed_build?: boolean | null;
}

export interface CollectionPick {
  /** Switched on, in the collection's order. */
  on: string[];
  /** In the profile, switched off: the alternatives not picked. */
  off: string[];
  /** For telling the player: per item, the one picked and the ones left off. */
  choices: { picked: string; left: string[] }[];
}

const bare = (id: string) => id.trim().replace(/^\\+/, '');

export const pickCollectionMods = (workshopIds: string[], mods: PickMod[]): CollectionPick => {
  const byItem = new Map<string, PickMod[]>();
  for (const m of mods) if (m.workshop_id) byItem.set(m.workshop_id, [...(byItem.get(m.workshop_id) ?? []), m]);
  const on: string[] = [];
  const off: string[] = [];
  const choices: CollectionPick['choices'] = [];
  for (const item of workshopIds) {
    const inItem = byItem.get(item) ?? [];
    // Two mods rule each other out when either names the other.
    const clash = (a: PickMod, b: PickMod) =>
      (a.incompatible ?? []).some((x) => bare(x) === b.mod_id) || (b.incompatible ?? []).some((x) => bare(x) === a.mod_id);
    // Loadable ones first (an alternative for another build is the one to leave), else the item's order.
    const ordered = [...inItem].sort((a, b) => Number(b.loadable_on_installed_build !== false) - Number(a.loadable_on_installed_build !== false));
    const picked: PickMod[] = [];
    const left = new Map<string, string[]>();
    for (const m of ordered) {
      const against = picked.find((p) => clash(p, m));
      if (against) {
        off.push(m.mod_id);
        left.set(against.mod_id, [...(left.get(against.mod_id) ?? []), m.name]);
      } else {
        picked.push(m);
      }
    }
    // Back in the item's own order for the list.
    for (const m of inItem) if (picked.includes(m)) on.push(m.mod_id);
    for (const [id, names] of left) choices.push({ picked: inItem.find((m) => m.mod_id === id)?.name ?? id, left: names });
  }
  return { on, off, choices };
};
