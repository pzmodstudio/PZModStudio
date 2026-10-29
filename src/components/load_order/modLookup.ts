import { ModInfo } from '../../types';

/** A mod id the way `require=` values are compared: trimmed, lowercased, no quotes or leading slashes. */
export const normalizeModId = (id: string): string => {
  return id
    .trim()
    .toLowerCase()
    .replace(/^[\\/]+/, '')
    .replace(/[\\'"]/g, '')
    .trim();
};

/**
 * The installed mod a `require=` entry means. Exact id, Workshop id or name first; then the same
 * letters and digits with punctuation ignored; then an id the entry is a prefix of. Among several
 * matches the one switched on wins.
 */
export const findInstalledDependencyInMods = (reqRaw: string, mods: ModInfo[]): ModInfo | undefined => {
  if (!reqRaw || !reqRaw.trim()) return undefined;
  const cleanReq = reqRaw.trim();
  const normReq = normalizeModId(cleanReq);
  const alphaReq = normReq.replace(/[^a-z0-9]/g, '');

  // 1. Pass 1: Exact match on Mod ID, Workshop ID, or exact Mod Name FIRST!
  const exactMatch = mods.find((m) => {
    const normM = normalizeModId(m.mod_id);
    const normName = normalizeModId(m.name);
    if (normM === normReq) return true;
    if (m.workshop_id && m.workshop_id.trim() === cleanReq) return true;
    if (normName === normReq) return true;
    return false;
  });
  if (exactMatch) return exactMatch;

  // 2. Pass 2: Full Alphanumeric Match (ignores punctuation/dashes/brackets)
  const alphaMatches = mods.filter((m) => {
    const alphaM = normalizeModId(m.mod_id).replace(/[^a-z0-9]/g, '');
    const alphaName = normalizeModId(m.name).replace(/[^a-z0-9]/g, '');
    return (alphaM.length > 0 && alphaM === alphaReq) || (alphaName.length > 0 && alphaName === alphaReq);
  });
  if (alphaMatches.length === 1) return alphaMatches[0];
  if (alphaMatches.length > 1) {
    const enabled = alphaMatches.find((c) => c.enabled);
    if (enabled) return enabled;
    return alphaMatches[0];
  }

  // 3. Pass 3: Prefix match (only if no exact or full alphanumeric match exists)
  const prefixMatches = mods.filter((m) => {
    const normM = normalizeModId(m.mod_id);
    return normM.startsWith(normReq + '_') || normM.startsWith(normReq + '-');
  });
  if (prefixMatches.length > 0) {
    const enabled = prefixMatches.find((c) => c.enabled);
    if (enabled) return enabled;
    return prefixMatches[0];
  }

  return undefined;
};
