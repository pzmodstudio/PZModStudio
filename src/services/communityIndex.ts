import { useEffect, useState } from 'react';
import { CommunityIndex, CommunityRegistry } from '../types';
import { TauriService } from './tauri';
import { StudioPathsUI } from '../components/settings/SettingsModule';

/** One copy per app session: the index is small and changes once a day at most. */
let cached: Promise<CommunityIndex | null> | null = null;

export function loadCommunityIndex(paths: StudioPathsUI, force = false): Promise<CommunityIndex | null> {
  if (!cached || force) cached = TauriService.communityIndex(paths);
  return cached;
}

/** The community index, once loaded; null until then or when none is at hand. */
export function useCommunityIndex(paths: StudioPathsUI): CommunityIndex | null {
  const [index, setIndex] = useState<CommunityIndex | null>(null);
  useEffect(() => {
    let live = true;
    void loadCommunityIndex(paths).then((i) => live && setIndex(i));
    return () => {
      live = false;
    };
  }, [paths.user_zomboid_dir]); // eslint-disable-line react-hooks/exhaustive-deps
  return index;
}

/** The registries that are about one Workshop item, by its id or by the mods installed from it. */
export function registriesForItem(index: CommunityIndex | null, workshopId: string, installedModIds: string[]): CommunityRegistry[] {
  if (!index) return [];
  const seen = new Set<string>();
  const out: CommunityRegistry[] = [];
  for (const r of Object.values(index.mods)) {
    if ((r.workshop_id === workshopId || installedModIds.includes(r.id)) && !seen.has(r.id)) {
      seen.add(r.id);
      out.push(r);
    }
  }
  return out;
}
