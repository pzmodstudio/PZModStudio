import { ModInfo } from '../types';
import { TauriService } from '../services/tauri';

/**
 * The Workshop item that provides a mod another mod requires, or null when it cannot be told.
 *
 * A requirement names a mod id (`require=VisibleDoorDamagebyYurik`), and the Workshop's search
 * knows titles, not mod ids: searching for the id gives back thousands of unrelated items. The
 * requiring mod's own Workshop page lists its Required items, though. Those already installed are
 * not it; when one is left, that is the one. With several left, the one whose description names
 * the mod id is.
 */
export async function findRequiredItem(requiringModId: string | null | undefined, missingModId: string, mods: ModInfo[]): Promise<string | null> {
  const requiring = mods.find((m) => m.mod_id === requiringModId);
  if (!requiring?.workshop_id) return null;
  try {
    const details = await TauriService.workshopItemDetails(requiring.workshop_id);
    const installed = new Set(mods.map((m) => m.workshop_id).filter(Boolean));
    const left = details.required_items.filter((r) => !installed.has(r.workshop_id));
    if (left.length === 1) return left[0].workshop_id;
    const wanted = missingModId.toLowerCase();
    for (const candidate of left) {
      const d = await TauriService.workshopItemDetails(candidate.workshop_id);
      if (d.description.toLowerCase().includes(wanted)) return candidate.workshop_id;
    }
  } catch {
    // Offline, or the page could not be read: the text search is what is left.
  }
  return null;
}

/** Opens the Workshop item that provides `missingModId`, or searches for it when there is none to tell. */
export async function openRequiredItem(
  requiringModId: string | null | undefined,
  missingModId: string,
  mods: ModInfo[],
  openItem: (workshopId: string) => void,
  search: (text: string) => void
): Promise<void> {
  const found = await findRequiredItem(requiringModId, missingModId, mods);
  if (found) openItem(found);
  else search(missingModId);
}
