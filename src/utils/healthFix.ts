import { HealthFix, ModInfo } from '../types';

const isStudioDraftPackage = (m: ModInfo) =>
  m.mod_id.startsWith('Z_PZModStudio_') && m.mod_id !== 'Z_PZModStudio_Bridge' && m.is_packaged === false;

/**
 * Applies one of the health check's fixes to a copy of the list -- switch a mod on or off, move
 * it before another, take an entry out -- and returns the new list, or null when the fix does
 * not apply (already in place, mod not there, a draft package that cannot be switched on).
 * Fixes that act on disk or open a page are not list fixes and come back null too.
 */
export function applyFix(mods: ModInfo[], fix: HealthFix): ModInfo[] | null {
  switch (fix.action) {
    case 'enable_mod':
    case 'disable_mod': {
      const enable = fix.action === 'enable_mod';
      const target = mods.find((m) => m.mod_id === fix.mod_id);
      if (!target || target.enabled === enable || (enable && isStudioDraftPackage(target))) return null;
      return mods.map((m) => (m.mod_id === fix.mod_id ? { ...m, enabled: enable } : m));
    }
    case 'remove_from_list': {
      const next = mods.filter((m) => m.mod_id !== fix.entry);
      return next.length === mods.length ? null : next;
    }
    case 'move_before': {
      const from = mods.findIndex((m) => m.mod_id === fix.mod_id);
      const anchor = mods.findIndex((m) => m.mod_id === fix.before);
      if (from < 0 || anchor < 0 || from < anchor) return null;
      const next = [...mods];
      const [moved] = next.splice(from, 1);
      next.splice(anchor, 0, moved);
      return next;
    }
    default:
      return null;
  }
}
