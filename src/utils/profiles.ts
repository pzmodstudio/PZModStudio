import { AppInstance, STUDIO_LINK_MOD_ID } from '../types';
import { TauriService } from '../services/tauri';

/**
 * The profile as it is on disk now. A profile held in a component's state goes stale as soon
 * as a mod is toggled, the order changes or the game's Mods screen writes the list; sending
 * that copy back whole would undo those changes. Every edit starts from this one.
 */
export async function freshProfile(userZomboidDir: string, profile: AppInstance): Promise<AppInstance> {
  try {
    const list: AppInstance[] = await TauriService.listInstances(userZomboidDir);
    return list.find((p) => p.id === profile.id) ?? profile;
  } catch {
    return profile;
  }
}

/**
 * Puts a mod in a profile from outside it (a Browse card, a Mod List row): switched on, at the
 * end of its order, once. Returns the profile as saved, or null when the mod was already in it.
 */
export async function addModToProfile(userZomboidDir: string, profile: AppInstance, modId: string): Promise<AppInstance | null> {
  const current = await freshProfile(userZomboidDir, profile);
  if (current.active_mod_ids.includes(modId)) return null;
  const updated: AppInstance = {
    ...current,
    active_mod_ids: [...current.active_mod_ids, modId],
    load_order: current.load_order.includes(modId) ? current.load_order : [...current.load_order, modId],
  };
  await TauriService.updateInstance(userZomboidDir, updated);
  return updated;
}

/**
 * What a profile's card counts, the way the Mod List counts it with that profile on: its mods that
 * are on, and all its mods, on and off (its members). Each mod id counts, so a Workshop item with
 * sub-mods counts once per sub-mod in the profile; `workshopItems` says how many items they come
 * from. The Studio Link mod is left out, as the Mod List leaves it out, and so are the mods that
 * are off and no longer installed. Not its saved order, which also remembers every mod installed
 * when the profile was made.
 *
 * `installed`: mod id -> Workshop id (null for a mod of Zomboid/mods); empty while not known.
 */
export function profileModCounts(
  profile: Pick<AppInstance, 'active_mod_ids' | 'members'>,
  installed: Map<string, string | null> = new Map()
): { active: number; total: number; workshopItems: number } {
  const plain = (id: string) => id.trim().replace(BACKSLASH, '');
  const counted = (id: string) => id !== '' && id !== STUDIO_LINK_MOD_ID;
  const on = new Set(profile.active_mod_ids.map(plain).filter(counted));
  const all = new Set(on);
  for (const id of (profile.members ?? []).map(plain)) {
    if (counted(id) && (installed.size === 0 || installed.has(id))) all.add(id);
  }
  const items = new Set<string>();
  for (const id of all) {
    const ws = installed.get(id);
    if (ws && ws !== '9999999999') items.add(ws);
  }
  return { active: on.size, total: all.size, workshopItems: items.size };
}

/** The leading backslash Build 42 puts before a mod id in its lists. */
const BACKSLASH = /^\\/;
