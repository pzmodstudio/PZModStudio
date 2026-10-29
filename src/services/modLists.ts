import { useCallback, useEffect, useState } from 'react';
import { ModList, ModListItem } from '../types';
import { TauriService } from './tauri';

/**
 * The player's lists (Favorites first), shared by every screen that shows a star: one copy per
 * app session, every change written through the backend and announced to every subscriber.
 */
let cache: ModList[] | null = null;
let cacheDir = '';
const listeners = new Set<() => void>();

const announce = () => listeners.forEach((fn) => fn());

export const FAVORITES = 'favorites';

/** What makes two entries the same: the Workshop item, else the Nexus mod, else the mod id. */
export const itemKey = (item: { workshop_id?: string | null; nexus_id?: number | null; mod_id?: string | null }) =>
  item.workshop_id ? `w:${item.workshop_id}` : item.nexus_id ? `n:${item.nexus_id}` : item.mod_id ? `m:${item.mod_id}` : '';

async function load(userZomboidDir: string, force = false): Promise<ModList[]> {
  if (!cache || cacheDir !== userZomboidDir || force) {
    cache = await TauriService.modLists(userZomboidDir);
    cacheDir = userZomboidDir;
    announce();
  }
  return cache;
}

function apply(updated: ModList) {
  if (!cache) return;
  cache = cache.map((l) => (l.id === updated.id ? updated : l));
  announce();
}

/** The installed mods the last sync saw, so a star changed here reaches the game at once. */
let installedForSync: { mod_id: string; workshop_id: string | null; name: string }[] | null = null;

/**
 * Favorites and the game's stars (its Mods screen and the Studio Link's) made one: run when the
 * mods are scanned, when the app comes back to the front, every few seconds, and after a star.
 */
export async function syncFavouritesWithGame(userZomboidDir: string, installed: { mod_id: string; workshop_id: string | null; name: string }[]) {
  installedForSync = installed;
  const lists = await TauriService.syncGameFavourites(userZomboidDir, installed);
  if (!lists) return;
  const same = cache && cacheDir === userZomboidDir && JSON.stringify(cache) === JSON.stringify(lists);
  if (same) return;
  cache = lists;
  cacheDir = userZomboidDir;
  announce();
}

export function useModLists(userZomboidDir: string) {
  const [lists, setLists] = useState<ModList[]>(cache ?? []);
  useEffect(() => {
    const refresh = () => setLists(cache ?? []);
    listeners.add(refresh);
    if (userZomboidDir) void load(userZomboidDir).then(refresh);
    return () => {
      listeners.delete(refresh);
    };
  }, [userZomboidDir]);

  const favorites = lists.find((l) => l.id === FAVORITES);
  const favoriteKeys = new Set((favorites?.items ?? []).map(itemKey));

  const isFavorite = useCallback((item: { workshop_id?: string | null; nexus_id?: number | null; mod_id?: string | null }) => favoriteKeys.has(itemKey(item)), [favoriteKeys]);

  const addTo = useCallback(
    async (listId: string, item: ModListItem) => apply(await TauriService.addToModList(userZomboidDir, listId, item)),
    [userZomboidDir]
  );
  const removeFrom = useCallback(
    async (listId: string, key: string) => apply(await TauriService.removeFromModList(userZomboidDir, listId, key)),
    [userZomboidDir]
  );
  /** Stars or unstars: in Favorites, out of Favorites. */
  const toggleFavorite = useCallback(
    async (item: ModListItem) => {
      if (favoriteKeys.has(itemKey(item))) await removeFrom(FAVORITES, itemKey(item));
      else await addTo(FAVORITES, item);
      if (installedForSync) await syncFavouritesWithGame(userZomboidDir, installedForSync);
    },
    [favoriteKeys, addTo, removeFrom]
  );
  const create = useCallback(
    async (name: string) => {
      const list = await TauriService.createModList(userZomboidDir, name);
      cache = [...(cache ?? []), list];
      announce();
      return list;
    },
    [userZomboidDir]
  );
  const rename = useCallback(async (listId: string, name: string) => apply(await TauriService.renameModList(userZomboidDir, listId, name)), [userZomboidDir]);
  const remove = useCallback(
    async (listId: string) => {
      cache = await TauriService.deleteModList(userZomboidDir, listId);
      announce();
    },
    [userZomboidDir]
  );

  return { lists, favorites, isFavorite, toggleFavorite, addTo, removeFrom, create, rename, remove, reload: () => load(userZomboidDir, true) };
}
