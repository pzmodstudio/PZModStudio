import { AppInstance, ModInfo, profileMembers, STUDIO_LINK_MOD_ID } from '../types';

/** The Studio's own mods (the Link, the bridge, its packages, the profiler): never offered. */
const isStudioMod = (id: string): boolean =>
  id === STUDIO_LINK_MOD_ID || id.startsWith('Z_PZModStudio_') || id === 'Z_ClaudeAgent' || id === 'PZModStudio_Profiler';

/** One Workshop item and the mods it brings, as found when a profile is deleted. */
export interface WorkshopItemOfProfile {
  workshop_id: string;
  /** The names of its mods, for the dialog. */
  mod_names: string[];
  /** The other profiles that use any mod of this item; empty when only the deleted profile did. */
  used_by: string[];
}

export interface ProfileCleanupPlan {
  /** Workshop items no other profile uses: these can go without touching anything else. */
  exclusive: WorkshopItemOfProfile[];
  /** Workshop items another profile also uses: these stay, whatever is chosen. */
  shared: WorkshopItemOfProfile[];
  /** Mods of the profile that have no Workshop item (a local folder): nothing to unsubscribe. */
  local_mod_names: string[];
}

/**
 * What deleting a profile could unsubscribe. A Workshop item is "exclusive" when none of its
 * mods belongs to any other profile (members, on or off); one shared mod keeps the whole item,
 * because Steam subscribes to items, not to mods. Mods switched on in the profile and mods
 * only kept in it count the same: both are the profile's.
 */
export function planProfileCleanup(profile: AppInstance, others: AppInstance[], mods: ModInfo[]): ProfileCleanupPlan {
  const byId = new Map(mods.map((m) => [m.mod_id, m]));
  const mine = new Set(profileMembers(profile));
  // Workshop item -> profiles (other than this one) that use a mod of it.
  const usedBy = new Map<string, Set<string>>();
  for (const other of others) {
    if (other.id === profile.id) continue;
    for (const id of profileMembers(other)) {
      const wid = byId.get(id)?.workshop_id;
      if (!wid) continue;
      if (!usedBy.has(wid)) usedBy.set(wid, new Set());
      usedBy.get(wid)!.add(other.name);
    }
  }
  // A Workshop item of the profile holds every installed mod of that item, including the ones
  // the profile never switched on: they go together.
  const items = new Map<string, WorkshopItemOfProfile>();
  const local: string[] = [];
  for (const id of mine) {
    const mod = byId.get(id);
    if (!mod || mod.not_installed || isStudioMod(id)) continue;
    if (!mod.workshop_id) {
      local.push(mod.name);
      continue;
    }
    if (!items.has(mod.workshop_id)) {
      const names = mods.filter((m) => m.workshop_id === mod.workshop_id).map((m) => m.name);
      items.set(mod.workshop_id, {
        workshop_id: mod.workshop_id,
        mod_names: [...new Set(names)].sort((a, b) => a.localeCompare(b)),
        used_by: [...(usedBy.get(mod.workshop_id) ?? [])].sort((a, b) => a.localeCompare(b)),
      });
    }
  }
  const all = [...items.values()].sort((a, b) => a.mod_names[0].localeCompare(b.mod_names[0]));
  return {
    exclusive: all.filter((i) => i.used_by.length === 0),
    shared: all.filter((i) => i.used_by.length > 0),
    local_mod_names: local.sort((a, b) => a.localeCompare(b)),
  };
}
