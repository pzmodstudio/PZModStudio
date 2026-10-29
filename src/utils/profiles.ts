import { AppInstance } from '../types';
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
