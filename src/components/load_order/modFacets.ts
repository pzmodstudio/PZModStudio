import i18next from 'i18next';
import { AuthorLink, CatalogModSummary, HealthIssue, ModErrorStat, ModInfo } from '../../types';
import { sourceOf } from '../common/SourceBadge';
import { HEAVY_SHARE, SLOW_LOAD_MS } from './ModRow';
import { labelFor, rowIssueCodes } from './RowHealth';

/**
 * What the Mod List can be narrowed to, in the order both the app and the game's Mods screen
 * show them (the Studio Link mod has the same buttons).
 */
export const MOD_FILTERS = ['all', 'enabled', 'disabled', 'favourites', 'updated', 'problems', 'maps', 'libraries', 'unloadable'] as const;
export type ModFilterId = (typeof MOD_FILTERS)[number];

/** Where a mod came from, in the order the origin filter lists them. */
export const MOD_SOURCES = ['steam', 'upload', 'nexus', 'manual', 'local', 'studio', 'compat'] as const;

/** The health check's issues by mod, for the row chips; an incompatible pair is on both rows. */
export function issuesByModOf(issues: HealthIssue[] | null | undefined): Map<string, HealthIssue[]> {
  const map = new Map<string, HealthIssue[]>();
  const add = (id: string, issue: HealthIssue) => map.set(id, [...(map.get(id) ?? []), issue]);
  for (const issue of issues ?? []) {
    if (issue.mod_id) add(issue.mod_id, issue);
    if (issue.code === 'incompatible') issue.related.forEach((other) => add(other, issue));
  }
  return map;
}

/**
 * The tags each row shows (what the last session said, its loading time, the health check's
 * chips), so the list can be narrowed to one of them. A tag is the same word as the chip.
 */
export function modTagsOf(
  mods: ModInfo[],
  errorStats: Map<string, ModErrorStat> | undefined,
  loadTimes: Map<string, number> | undefined,
  catalogById: Map<string, CatalogModSummary> | undefined,
  issuesByMod: Map<string, HealthIssue[]>,
  gameShares?: Map<string, number>
): Map<string, string[]> {
  const map = new Map<string, string[]>();
  for (const m of mods) {
    const tags: string[] = [];
    const stat = errorStats?.get(m.name);
    if (stat && stat.exceptions > 0) tags.push('errors');
    else if (stat && stat.warnings > 0) tags.push('warnings');
    if ((loadTimes?.get(m.mod_id) ?? 0) >= SLOW_LOAD_MS) tags.push('slow');
    if ((gameShares?.get(m.mod_id) ?? 0) >= HEAVY_SHARE) tags.push('heavy');
    const compat = sourceOf(m.mod_id, catalogById?.get(m.mod_id)).kind === 'compat';
    for (const code of rowIssueCodes(issuesByMod.get(m.mod_id) ?? [])) {
      if (compat && code === 'shadowed_copy') continue;
      if (!tags.includes(`health:${code}`)) tags.push(`health:${code}`);
    }
    map.set(m.mod_id, tags);
  }
  return map;
}

export const tagLabel = (tag: string): string =>
  tag.startsWith('health:') ? labelFor({ code: tag.slice(7) }) : i18next.t(`modList.tags.${tag}`, { min: SLOW_LOAD_MS, pct: HEAVY_SHARE });

/**
 * What the game's Mods screen cannot tell by itself, for the Studio Link's filters: each mod's
 * origin, whether it is a library and its tags, with the words in the app's language.
 *   origin=<kind>\t<label>
 *   tag=<id>\t<label>
 *   mod=<id>\t<origin>\t<1 if a library>\t<tag;tag>
 *   support=<id>\t<name>\t<url>   (where to support its author, as the inspector shows)
 */
export function linkModsText(
  mods: ModInfo[],
  catalogById: Map<string, CatalogModSummary> | undefined,
  tags: Map<string, string[]>,
  support: Map<string, AuthorLink[]> = new Map()
): string {
  const clean = (s: string) => s.replace(/[\t\r\n]/g, ' ');
  const lines: string[] = MOD_SOURCES.map((kind) => `origin=${kind}\t${clean(i18next.t(`modList.sources.${kind}`))}`);
  const seen = new Set<string>();
  for (const list of tags.values()) for (const tag of list) seen.add(tag);
  for (const tag of [...seen].sort()) lines.push(`tag=${tag}\t${clean(tagLabel(tag))}`);
  for (const m of mods) {
    const kind = m.not_installed ? '' : sourceOf(m.mod_id, catalogById?.get(m.mod_id)).kind;
    lines.push(`mod=${m.mod_id}\t${kind}\t${m.is_library ? 1 : 0}\t${(tags.get(m.mod_id) ?? []).join(';')}`);
    for (const link of support.get(m.mod_id) ?? []) {
      if (link.url.startsWith('https://')) lines.push(`support=${m.mod_id}\t${clean(link.name)}\t${clean(link.url)}`);
    }
  }
  return lines.join('\n') + '\n';
}
