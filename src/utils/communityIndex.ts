import { CatalogModSummary, CommunityIssue, CommunityRegistry, ModErrorStat, ModInfo } from '../types';

/** The Studio's repository, where the index lives. */
export const COMMUNITY_REPO = 'https://github.com/pzmodstudio/PZModStudio';
/** The index's web page (GitHub Pages, site/index.html): every mod's known issues and fixes, searchable. */
export const COMMUNITY_SITE = 'https://pzmodstudio.github.io/PZModStudio/';

/** A registry's file name: its id, with "/" written as "__" (dSAG's id has one). */
export const registryFileName = (modId: string) => `${modId.replace(/\//g, '__')}.json`;

/**
 * What the Studio itself saw of a mod on this machine, worth telling the community: the errors
 * and warnings the Monitor counted for it in the last session, and the compatible copy it needed
 * (with what the copy changed). Each is a known issue, verified, on the installed build. Nothing
 * seen means nothing to share, and the menu says so instead of opening an empty note.
 */
/** A log line made fit to publish: the Windows (or other) user name in paths becomes `<user>`. */
export function withoutUserName(line: string | null | undefined): string | undefined {
  if (!line) return undefined;
  // A user folder name can hold spaces ("Javier Pérez"): it runs to the next separator.
  return line.replace(/([A-Za-z]:[\\/]+(?:Users|Documents and Settings)[\\/]+)[^\\/"'\r\n]+/gi, '$1<user>').replace(/(\/(?:home|Users)\/)[^/"'\r\n]+/g, '$1<user>');
}

export function knownIssues(catalog: CatalogModSummary | undefined, stat: ModErrorStat | undefined, installedBuild: string | null): CommunityIssue[] {
  const seen = { build: installedBuild ?? undefined, date: new Date().toISOString().slice(0, 10), source: "PZ Mod Studio, from the game's console.txt", status: 'verified' as const };
  const issues: CommunityIssue[] = [];
  if (stat && stat.exceptions > 0) {
    issues.push({ title: `Threw ${stat.exceptions} error${stat.exceptions === 1 ? '' : 's'} in the last session`, log: withoutUserName(stat.last_exception), fix: { kind: 'none' }, ...seen });
  }
  if (stat && stat.warnings > 0) {
    issues.push({ title: `${stat.warnings} warning${stat.warnings === 1 ? '' : 's'} in the last session (the game carried on)`, log: withoutUserName(stat.last_warning), fix: { kind: 'none' }, ...seen });
  }
  if (catalog?.origin?.kind === 'compat') {
    issues.push({
      title: 'Needed a compatible copy on this build',
      note: catalog.origin.changes.length ? `The copy changed: ${catalog.origin.changes.join('; ')}` : undefined,
      fix: { kind: 'compatible_copy' },
      ...seen,
      source: 'PZ Mod Studio, the compatible copy it made',
    });
  }
  return issues;
}

/**
 * A registry filled with what the Studio already knows about an installed mod (mod.info, where
 * it came from, the build it runs on here, and what it saw of it: `issues`), so a person
 * proposing it adds only what they tested. Every empty list is left in, as a prompt.
 */
export function draftRegistry(mod: ModInfo, catalog: CatalogModSummary | undefined, installedBuild: string | null, issues: CommunityIssue[] = []): CommunityRegistry {
  const workshopId = mod.workshop_id ?? (catalog?.source.kind === 'workshop' ? catalog.source.workshop_id : undefined);
  const major = installedBuild?.match(/^\d{2}/)?.[0];
  const builds = mod.loadable_on_installed_build !== false && major ? [major] : [];
  return {
    id: mod.mod_id,
    name: mod.name,
    ...(workshopId ? { workshop_id: workshopId } : {}),
    authors: mod.author ? [mod.author] : [],
    builds,
    requires: mod.dependencies ?? [],
    incompatible: (mod.incompatible ?? []).map((id) => ({ id, note: 'per its mod.info', status: 'verified' as const })),
    load_after: [],
    issues,
  };
}

/**
 * GitHub's "new file" page for the registry, with the path and the content filled in: the
 * person signs in, reads it over, and presses "Propose changes"; GitHub forks and opens the pull
 * request for them. Nothing is sent from the app.
 */
export function proposeUrl(registry: CommunityRegistry): string {
  const value = JSON.stringify(registry, null, 2) + '\n';
  return `${COMMUNITY_REPO}/new/main/community/index?filename=${encodeURIComponent(registryFileName(registry.id))}&value=${encodeURIComponent(value)}`;
}
