import { invoke } from '@tauri-apps/api/core';
import { cached, clearRemoteCache, prefetch } from './cache';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import i18next from 'i18next';
import { ChangeScope, noteChange } from './changes';
import { StudioPathsUI } from '../components/settings/SettingsModule';
import { AppInstance, MissingModsReport, VfsConflict,ModInfo, TranslatedErrorCard,DedicatedServerStatus, ConnectedPlayer, ServerQuickSettings, PZServerConfig, ModHealth, SortResult, NexusSearchResult, NexusSort, InstallResult, ModOrigin, WorkshopItem, WorkshopLookup, WorkshopSearchResult, CompatResult, AutoMergeResult, NexusCategory, NexusModDetails, WorkshopItemDetails, WorkshopBrowseQuery, SubscribeOutcome, BisectSession, HealthIssue, ServerLines, ServerModReport, ModListVersion, PolyfillSection, ShimPatch, KeptVersion, ModErrorStat, UserRule, CommunitySummary, CommunityIndex, CommunityLists, SharedList, ProposeResult, LoadTimes, BootTimes, GameMemory, ProfilerStatus, ModList, ModListItem, ModValidation, DraftStatus, DraftReport, ModApiReport, WorkshopItemMeta, UploadPreview, PublishOutcome, PublishProgress, ChangedFile, DevReloadEvent, DiscordCommandsStatus, DiscordSettings, DiscordView, GameBot, ModChangelog, NewMod, CreatedMod, MapTools, MpReport, MpModReport, UrlInstallPlan, SandboxPresetInfo, PresetSandbox, ModFile, RequireAnswer, SavesOverview, SaveBackupOutcome, SaveBackupEntry, RequiredChain, SessionDiff, SaveMissingMod, SaveMaps, SaveMapsFix, SaveInside, ErrorHistoryView, PackageFileState, Translation, SmoothFramesReport, DiskReport, GameLaunchMode, WhatsNew, MapCells, MapImage, WindowPrefs, ModMeta, PublicServer, ServerRules, SavedServer, AppUpdateStatus } from '../types';

export interface LuaSyntaxResult {
  is_valid: boolean;
  error_message?: string;
}

export interface MergeChunkResult {
  merged_text: string;
  has_conflict: boolean;
  conflict_line_start?: number;
  conflict_line_end?: number;
}

export interface ModListDataRaw {
  active_mods: string[];
  raw_ini_content: string;
}

export interface DependencyAnalysisRaw {
  sorted_mod_ids: string[];
  missing_dependencies: string[];
  has_circular_dependency: boolean;
  circular_mod_ids: string[];
  duplicate_mod_ids: string[];
  incompatible_pairs: [string, string][];
}

export interface SandboxLogPayload {
  line: string;
  is_error: boolean;
}

export interface MasterPatchResultUI {
  success: boolean;
  patch_mod_dir: string;
  files_written: number;
  polyfill_shim_written: boolean;
}

export interface MasterPatchStatusInfoUI {
  is_packaged: boolean;
  created_at?: string;
  packaged_mods: string[];
  merged_files: string[];
  missing_active_mods: string[];
}

export interface MergedPackageInfoUI {
  folder_name: string;
  display_name: string;
  mod_id: string;
  description?: string;
  is_packaged: boolean;
  is_active_in_modlist: boolean;
  is_companion_bridge: boolean;
  /** The Studio Link mod (also a locked system mod). */
  is_studio_link?: boolean;
  created_at?: string;
  packaged_mods: string[];
  merged_files: string[];
  /** Only on an import: what is missing here and which copies differ from the sender's. */
  version_check?: MissingModsReport | null;
  /** The profile that was active when it was last published. */
  built_for_profile?: string | null;
}

const STORAGE_KEY_PATHS = 'pz_mod_studio_paths_profile';

/** Window event fired when a backend operation fails; App shows it to the user. */
export const BACKEND_ERROR_EVENT = 'pzms-backend-error';
/** Something the Studio did on its own that the player should know about (a green toast). */
export const NOTICE_EVENT = 'pzms-notice';
export const reportNotice = (text: string): void => {
  window.dispatchEvent(new CustomEvent<string>(NOTICE_EVENT, { detail: text }));
};

export interface BackendErrorDetail {
  action: string;
  message: string;
}

const errorMessage = (err: unknown): string =>
  typeof err === 'string' ? err : err instanceof Error ? err.message : JSON.stringify(err);

/**
 * What Rust writes in English (errors, and a few notes such as the end of a bisect hunt), in the
 * chosen language: the pattern of each known text, its values as named groups, and its key under
 * `backend.say`. The first match wins, so the more specific patterns come first.
 */
const SAID: [RegExp, string][] = [
  // Bisect
  [/^There is nothing to halve: (?<on>\d+) of the (?<total>\d+) switched-on mods/, 'bisectNothingToHalve'],
  [/^There is no hunt in progress to answer/, 'bisectNoHuntAnswer'],
  [/^There is no hunt in progress to stop/, 'bisectNoHuntStop'],
  [/^(?<mod>.+?) on its own does not do it\. Two mods together/s, 'bisectPair'],
  [/^No mod under suspicion is doing it/, 'bisectNoSuspect'],
  // Developer tools and the folders they may open
  [/^That folder is outside your game and Zomboid folders/, 'outsideFolders'],
  [/^That file is outside your game and Zomboid folders/, 'outsideFoldersFile'],
  [/^That folder has no mod\.info/, 'noModInfo'],
  [/^Windows could not open that file: (?<why>.+)$/s, 'cantOpenFile'],
  [/^Windows could not open that folder: (?<why>.+)$/s, 'cantOpenFolder'],
  [/^Nothing to reload\./, 'nothingToReload'],
  [/^The id is the folder and the list entry/, 'badModId'],
  [/^The mod needs a name/, 'modNeedsName'],
  [/^There is already a folder (?<id>.+) in Zomboid\/mods/, 'folderExists'],
  [/^Only TileZed\.exe and WorldEd\.exe/, 'onlyMapTools'],
  // Servers
  [/^Invalid server name '(?<name>[^']*)'/, 'badServerName'],
  [/^Invalid server name\.$/, 'badServerNameShort'],
  [/^A server named '(?<name>.+)' already exists/, 'serverExists'],
  [/^The server '(?<name>.+)' is running: stop it before deleting it/, 'serverRunningDelete'],
  [/^Invalid Zomboid directory/, 'badZomboidDir'],
  [/^Failed to write server log snapshot: (?<why>.+)$/s, 'snapshotFailed'],
  [/^Install directory not found: (?<what>.*)$/s, 'noInstallDir'],
  [/^The admin password cannot contain any of/, 'badAdminPassword'],
  [/^A server is already running \(PID (?<pid>\d+)\)/, 'serverAlreadyRunning'],
  [/^Could not find java\.exe, ProjectZomboidServer\.bat/, 'noServerLauncher'],
  [/^Failed to spawn dedicated server: (?<why>.+)$/s, 'serverSpawnFailed'],
  [/^Could not stop server process (?<pid>\d+): (?<why>.+)$/s, 'serverStopFailed'],
  [/^Failed to open log file '(?<what>.+)': (?<why>.+)$/s, 'logOpenFailed'],
  [/^Server \.ini file not found/, 'serverIniMissing'],
  [/^RCON refused the password/, 'rconPassword'],
  [/^RCON: (?<why>.+)$/s, 'rcon'],
  // Browse: links, downloads, archives, Steam and Nexus
  [/^Only http and https links can be installed from/, 'onlyHttp'],
  [/^That is a Steam Workshop link/, 'steamLink'],
  [/^Nexus Mods does not allow downloading without/, 'nexusLink'],
  [/^That link does not point at a \.zip/, 'notZipLink'],
  [/^GitHub is rate-limiting this machine/, 'githubRateLimit'],
  [/^GitHub could not be reached about (?<repo>\S+): (?<why>.+)$/s, 'githubUnreachable'],
  [/^GitHub has no repository (?<repo>\S+?)\./, 'githubNoRepo'],
  [/^Unexpected answer from (?<who>GitHub|Nexus Mods|Steam|the Steam Workshop|the Steam helper)\b/, 'unexpectedAnswer'],
  [/^Cannot create a download folder: (?<why>.+)$/s, 'noDownloadFolder'],
  [/^The download failed: (?<why>.+)$/s, 'downloadFailed'],
  [/^That file is larger than (?<mb>\d+) MB/, 'tooLarge'],
  [/^What came back is not a zip file/, 'notAZip'],
  [/^Cannot save the download: (?<why>.+)$/s, 'cannotSaveDownload'],
  [/^Not a readable zip archive/, 'badZip'],
  [/^Not a readable 7z archive/, 'bad7z'],
  [/^The archive has (?:\d+ entries \(limit (?<limit>\d+)\)|more than (?<limit2>\d+) entries)/, 'tooManyEntries'],
  [/^Damaged archive entry/, 'damagedEntry'],
  [/^Refusing unsafe path in archive: (?<what>.+)$/s, 'unsafePath'],
  [/^The archive unpacks to more than 4 GB/, 'tooBigUnpacked'],
  [/^Archive entry (?<what>.+) is larger than it claims/, 'entryLies'],
  [/^No Project Zomboid mod found: there is no mod\.info/, 'noModFound'],
  [/^Unusable mod folder name: (?<what>.+)$/s, 'badFolderName'],
  [/^The source folder is inside Zomboid\/mods already/, 'sourceInside'],
  [/^Cannot move the old (?<what>.+?) aside: (?<why>.+)$/s, 'cannotMoveAside'],
  [/^Copying (?<what>.+?) failed: (?<why>.+)$/s, 'copyFailed'],
  [/^Nexus Mods rejected the (?<what>.+?): (?<why>.+)$/s, 'nexusRejected'],
  [/^Nexus Mods has no mod with this id/, 'nexusNoMod'],
  [/^The Steam Workshop page could not be read/, 'workshopPageUnreadable'],
  [/^Steam rejected the Web API key/, 'badApiKey'],
  [/^Steam has no public item with this id/, 'noWorkshopItem'],
  [/^Steam refused the request/, 'steamRefused'],
  [/^The package is in use and could not be replaced/, 'packageInUse'],
  // Files and folders, last: other texts start the same way
  [/^(?<what>.+) is not there any more\.$/s, 'notThereAnyMore'],
  [/^(?<what>.+) is not there\.$/s, 'notThere'],
  [/^(?<what>.+) is not a folder\.$/s, 'notAFolder'],
  [/^(?<name>TileZed\.exe|WorldEd\.exe) would not start: (?<why>.+)$/s, 'wouldNotStart'],
  [/^Cannot write (?<what>.+?): (?<why>.+)$/s, 'cannotWrite'],
  [/^Cannot read (?<what>.+?): (?<why>.+)$/s, 'cannotRead'],
  [/^Cannot open (?<what>.+?): (?<why>.+)$/s, 'cannotOpen'],
  [/^Cannot back up (?<what>.+?): (?<why>.+)$/s, 'cannotBackUp'],
  [/^Cannot delete (?<what>.+?): (?<why>.+)$/s, 'cannotDelete'],
];

/** A text the backend wrote, in the chosen language when it is a known one; else as it came. */
export function backendSays(text: string): string {
  if (i18next.resolvedLanguage === 'en') return text;
  for (const [pattern, key] of SAID) {
    const match = pattern.exec(text);
    if (match) {
      const values: Record<string, string> = { ...(match.groups ?? {}) };
      if (values.limit2) values.limit = values.limit2;
      return i18next.t(`backend.say.${key}`, values);
    }
  }
  return text;
}

/**
 * Backend errors are written for the log, not for a player. This turns the ones we know about
 * into something a person can act on, and leaves anything else as it came.
 */
export function explainError(err: unknown): string {
  const raw = String(err ?? '').replace(/^Error:\s*/i, '').trim();
  const said = backendSays(raw);
  if (said !== raw) return said;
  const known: [RegExp, string][] = [
    [/Zomboid folder not found/i, 'noZomboid'],
    [/ProjectZomboid64\.exe not found/i, 'noExe'],
    [/steam_api64\.dll was not found/i, 'notPzFolder'],
    [/Steam is not running/i, 'steamNotRunning'],
    [/Steam Workshop busy/i, 'steamBusy'],
    [/could not be reached|os error 10051|dns error/i, 'noConnection'],
    // GitHub's own limit is said above: this one is Steam's.
    [/Steam is rate-limiting|rate-limit/i, 'rateLimit'],
    [/Access is denied|os error 5/i, 'accessDenied'],
    [/os error 2|No such file/i, 'gone'],
  ];
  const match = known.find(([pattern]) => pattern.test(raw));
  if (match) return i18next.t(`backend.explain.${match[1]}`);
  // Nothing recognised: show it, but without the Rust decoration.
  return raw.replace(/^[A-Za-z_:]+Error\(|\)$/g, '') || i18next.t('backend.somethingWrong');
}

/** Logs a failed backend call and tells the UI about it, so failures never look like success. */
export const reportBackendError = (action: string, err: unknown): void => {
  console.error(`${action} failed:`, err);
  // Into the Studio's own log too, so a bug report has it.
  void invoke('log_from_window_cmd', { level: 'ERROR', text: `${action} failed: ${errorMessage(err)}` }).catch(() => undefined);
  window.dispatchEvent(
    new CustomEvent<BackendErrorDetail>(BACKEND_ERROR_EVENT, { detail: { action, message: errorMessage(err) } })
  );
};

/**
 * Service layer wrapping all native Tauri Rust backend IPC calls
 */
export const TauriService = {
  /**
   * Opens any external web URL in the user's default system browser
   */
  openExternalUrl: async (url: string): Promise<void> => {
    try {
      await openUrl(url);
      return;
    } catch (pluginErr) {
      console.warn('Tauri openUrl plugin fallback:', pluginErr);
    }

    try {
      await invoke('open_external_url_cmd', { url });
      return;
    } catch (ipcErr) {
      console.warn('Tauri open_external_url_cmd fallback:', ipcErr);
    }

    try {
      window.open(url, '_blank');
    } catch (winErr) {
      console.error('Failed to open external URL:', winErr);
    }
  },

  /**
   * Loads saved directory paths profile from LocalStorage or Rust auto-detection
   */
  loadSavedPathsProfile: async (): Promise<StudioPathsUI> => {
    try {
      const cached = localStorage.getItem(STORAGE_KEY_PATHS);
      if (cached) {
        const parsed = JSON.parse(cached) as StudioPathsUI;
        if (parsed.pz_install_dir) {
          // Re-validate cached paths with Rust
          return await invoke<StudioPathsUI>('set_and_validate_paths', { customPaths: parsed });
        }
      }
    } catch (err) {
      console.warn('Failed to load cached paths profile:', err);
    }
    // Fallback to Rust auto-detection
    return await TauriService.getAutoPaths();
  },

  /**
   * Saves directory paths profile persistently to LocalStorage
   */
  savePathsProfile: async (paths: StudioPathsUI): Promise<StudioPathsUI> => {
    const validated = await TauriService.validatePaths(paths);
    localStorage.setItem(STORAGE_KEY_PATHS, JSON.stringify(validated));
    return validated;
  },

  /**
   * Auto-detects Project Zomboid installation and user directories
   */
  getAutoPaths: async (): Promise<StudioPathsUI> => {
    try {
      return await invoke<StudioPathsUI>('get_auto_paths');
    } catch (err) {
      console.warn('Fallback to default paths:', err);
      return {
        pz_install_dir: '',
        workshop_dir: '',
        user_zomboid_dir: '',
        mod_list_ini_path: '',
        is_valid: false,
      };
    }
  },

  /**
   * Validates custom or manually entered directory paths
   */
  validatePaths: async (customPaths: StudioPathsUI): Promise<StudioPathsUI> => {
    try {
      return await invoke<StudioPathsUI>('set_and_validate_paths', { customPaths });
    } catch (err) {
      console.error('Failed to validate paths:', err);
      return customPaths;
    }
  },

  /**
   * Opens native Windows Explorer folder picker dialog
   */
  pickFolder: async (defaultPath?: string): Promise<string | null> => {
    try {
      return await invoke<string | null>('pick_folder_cmd', { defaultPath });
    } catch (err) {
      console.error('Folder picker error:', err);
      return null;
    }
  },

  /**
   * Scans Virtual File System for real conflicts between active mods
   */
  scanConflicts: async (paths: StudioPathsUI, packageFolder?: string | null): Promise<VfsConflict[]> => {
    try {
      const rawConflicts = await invoke<any[]>('scan_conflicts_cmd', { paths, packageFolder: packageFolder ?? null });
      if (!rawConflicts || rawConflicts.length === 0) {
        return [];
      }
      return rawConflicts.map((c) => ({
        id: c.id,
        relative_path: c.relative_path,
        file_type: c.file_type,
        start_line: c.start_line ?? 1,
        end_line: c.end_line ?? 10,
        conflict_line: c.conflict_line ?? 1,
        total_file_lines: c.total_file_lines ?? c.base_content.split('\n').length ?? 1,
        base_content: c.base_content,
        merged_output: c.merged_output || c.base_content,
        auto_ast_output: c.auto_ast_output || c.merged_output || c.base_content,
        is_identical_noise: c.is_identical_noise ?? false,
        competing_mods: c.competing_files.map((f: any) => ({
          mod_id: f.mod_id,
          mod_name: f.mod_name,
          absolute_path: f.absolute_path,
          content: f.content,
        })),
        status: (c.status as any) || (c.is_identical_noise ? ('AUTO_MERGED' as const) : ('MANUAL_CONFLICT' as const)),
        mod_changes: c.mod_changes ?? [],
        collisions: c.collisions ?? [],
        strategy: c.strategy ?? '',
      }));
    } catch (err) {
      console.warn('Scan conflicts error:', err);
      return [];
    }
  },

  /**
   * Scans all subscribed Workshop & local mods, ordering active ones first according to ModListData.ini
   */
  scanAllInstalledMods: async (paths: StudioPathsUI): Promise<ModInfo[]> => {
    try {
      const rawMods = await invoke<any[]>('scan_all_installed_mods_cmd', { paths });
      if (!rawMods || rawMods.length === 0) {
        return [];
      }
      return rawMods.map((m, idx) => ({
        mod_id: m.id,
        name: m.name || m.id,
        description: m.description,
        workshop_id: m.workshop_id,
        author: m.author,
        version: m.version,
        pzversion: m.pzversion,
        url: m.url,
        dependencies: m.require || [],
        load_mod_after: m.load_mod_after || [],
        incompatible: m.incompatible || [],
        enabled: m.enabled ?? false,
        icon_path: m.icon_path,
        poster_url: m.poster_url,
        is_library: m.is_library ?? false,
        is_map_mod: m.is_map_mod ?? false,
        is_packaged: m.is_packaged,
        loadable_on_installed_build: m.loadable_on_installed_build,
        not_installed: m.not_installed ?? false,
        installed_at: m.installed_at ?? null,
        load_order_index: idx + 1,
      }));
    } catch (err) {
      reportBackendError('Scanning installed mods', err);
      return [];
    }
  },

  /**
   * Read-only health check of the active load order (game rules: build, dependencies, order,
   * incompatibilities). Throws on backend failure so callers can show it.
   */
  getModHealth: async (paths: StudioPathsUI, active: string[]): Promise<ModHealth> => {
    return await invoke<ModHealth>('get_mod_health_cmd', { paths, active });
  },

  /** Project Zomboid mods on Nexus Mods (public API). Throws on failure. */
  nexusSearch: async (query: string, category: string | null, sort: NexusSort, offset: number, count: number): Promise<NexusSearchResult> => {
    return await cached(`nexus:search:${sort}:${category ?? ''}:${offset}:${count}:${query}`, () =>
      invoke<NexusSearchResult>('nexus_search_cmd', { query: query || null, category: category || null, sort, offset, count })
    );
  },

  /** Project Zomboid categories on Nexus with mod counts. Throws on failure. */
  nexusCategories: async (): Promise<NexusCategory[]> => {
    return await cached('nexus:categories', () => invoke<NexusCategory[]>('nexus_categories_cmd'));
  },

  /** Description, requirements and files of one Nexus mod. Throws on failure. */
  nexusModDetails: async (modId: number): Promise<NexusModDetails> => {
    return await cached(`nexus:mod:${modId}`, () => invoke<NexusModDetails>('nexus_mod_details_cmd', { modId }));
  },

  /** Full description, screenshots and required items of one Workshop item (no key). Throws on failure. */
  workshopItemDetails: async (id: string): Promise<WorkshopItemDetails> => {
    return await cached(`workshop:item:${id}`, () => invoke<WorkshopItemDetails>('workshop_item_details_cmd', { id }));
  },

  /**
   * Installs the mod(s) in a downloaded .zip or an extracted folder into Zomboid/mods.
   * Rejects with a message starting with INSTALL_CONFLICT: (JSON {existing_folders}) when
   * folders already exist and replaceExisting is false.
   */
  /** Forgets cached Workshop/Nexus answers (after installing, or on a manual refresh). */
  clearRemoteCache,
  /** Warms a cache key in the background. */
  prefetch,

  installMods: async (source: string, paths: StudioPathsUI, replaceExisting: boolean, origin?: ModOrigin): Promise<InstallResult> => {
    return await invoke<InstallResult>('install_mods_cmd', { source, paths, replaceExisting, origin: origin ?? null });
  },

  /** Steam Workshop details for item ids (no key needed). Throws on failure. */
  workshopDetails: async (ids: string[]): Promise<WorkshopItem[]> => {
    return await cached(`workshop:details:${[...ids].sort().join(',')}`, () => invoke<WorkshopItem[]>('workshop_details_cmd', { ids }));
  },

  /** Copies a mod the installed build won't load into Zomboid/mods and adapts it. Throws with the reason. */
  makeCompatibleCopy: async (paths: StudioPathsUI, modId: string, replaceExisting: boolean): Promise<CompatResult> => {
    return await invoke<CompatResult>('make_compatible_copy_cmd', { paths, modId, replaceExisting });
  },

  /** Merges the given conflicting files into the active patch package (safe cases only). Throws with the reason. */
  autoMergeConflicts: async (paths: StudioPathsUI, relativePaths: string[]): Promise<AutoMergeResult> => {
    return await invoke<AutoMergeResult>('auto_merge_conflicts_cmd', { paths, relativePaths });
  },

  /** Removes a compatible copy (moved to Zomboid/PZModStudio_Backups/mods). */
  removeCompatibleCopy: async (paths: StudioPathsUI, folder: string): Promise<void> => {
    await invoke('remove_compatible_copy_cmd', { paths, folder });
  },

  /** Keeps a copy of the mod as installed today, under Zomboid/PZModStudio_Snapshots. */
  keepModVersion: async (paths: StudioPathsUI, modId: string): Promise<KeptVersion> => {
    try {
      return await invoke<KeptVersion>('keep_mod_version_cmd', { paths, modId });
    } catch (err) {
      reportBackendError('Keeping a copy of the mod', err);
      throw err;
    }
  },

  /** Fetches the community rules and the mod index now; returns what came. */
  refreshCommunity: async (paths: StudioPathsUI): Promise<CommunitySummary> => {
    try {
      return await invoke<CommunitySummary>('refresh_community_cmd', { paths });
    } catch (err) {
      reportBackendError('Fetching the community rules and index', err);
      throw err;
    }
  },

  /** The community mod index at hand (cached; fetched when a day old), or null when none. */
  communityIndex: async (paths: StudioPathsUI): Promise<CommunityIndex | null> => {
    try {
      return await invoke<CommunityIndex | null>('community_index_cmd', { paths });
    } catch {
      return null;
    }
  },

  /** Kick, ban or message the players of a server this Studio runs, over RCON; the server's answer. */
  serverPlayerAction: async (paths: StudioPathsUI, server: string, action: 'kick' | 'ban' | 'message', player: string | null, text: string): Promise<string> => {
    // The players screen shows a failure itself, beside the buttons.
    return await invoke<string>('server_player_action_cmd', { paths, server, action, player, text });
  },

  /** Whether a newer PZ Mod Studio is out (GitHub, once a day while the check is on). */
  appUpdateStatus: async (userZomboidDir: string, force = false): Promise<AppUpdateStatus | null> => {
    try {
      return await invoke<AppUpdateStatus>('app_update_status_cmd', { userZomboidDir, force });
    } catch {
      return null;
    }
  },

  setAppUpdateCheck: async (userZomboidDir: string, enabled: boolean): Promise<AppUpdateStatus> => {
    try {
      return await invoke<AppUpdateStatus>('set_app_update_check_cmd', { userZomboidDir, enabled });
    } catch (err) {
      reportBackendError('Switching the update check', err);
      throw err;
    }
  },

  /** "Not now" on the notice: quiet until the next version. */
  /** Downloads, checks and puts the release `version` in place; the app then restarts itself. */
  selfUpdate: async (version: string): Promise<void> => {
    await invoke('self_update_cmd', { version });
  },
  dismissAppUpdate: async (userZomboidDir: string, version: string): Promise<void> => {
    try {
      await invoke('dismiss_app_update_cmd', { userZomboidDir, version });
    } catch (err) {
      reportBackendError('Hiding the update notice', err);
    }
  },

  /** Whether the Studio fetches the community rules and index (Settings). */
  communityDownloads: async (userZomboidDir: string): Promise<boolean> => {
    try {
      return await invoke<boolean>('community_downloads_cmd', { userZomboidDir });
    } catch {
      return true;
    }
  },

  setCommunityDownloads: async (userZomboidDir: string, enabled: boolean): Promise<void> => {
    try {
      await invoke('set_community_downloads_cmd', { userZomboidDir, enabled });
    } catch (err) {
      reportBackendError('Switching the community downloads', err);
      throw err;
    }
  },

  /** The player's own load-order rules. */
  listLoadRules: async (paths: StudioPathsUI): Promise<UserRule[]> => {
    try {
      return await invoke<UserRule[]>('list_load_rules_cmd', { paths });
    } catch {
      return [];
    }
  },

  addLoadRule: async (paths: StudioPathsUI, modId: string, after: string, note: string | null): Promise<UserRule[]> => {
    try {
      return await invoke<UserRule[]>('add_load_rule_cmd', { paths, modId, after, note });
    } catch (err) {
      reportBackendError('Adding the load-order rule', err);
      throw err;
    }
  },

  removeLoadRule: async (paths: StudioPathsUI, modId: string, after: string): Promise<UserRule[]> => {
    try {
      return await invoke<UserRule[]>('remove_load_rule_cmd', { paths, modId, after });
    } catch (err) {
      reportBackendError('Removing the load-order rule', err);
      throw err;
    }
  },

  /** What each mod did to the last session's log, from console.txt as it is now. */
  /** Every Lua and script file of a mod checked; throws with the reason. */
  /** The player's own Workshop items (Zomboid/Workshop/<item>). */
  workshopUploads: async (paths: StudioPathsUI): Promise<WorkshopItemMeta[]> => {
    return await invoke<WorkshopItemMeta[]>('workshop_uploads_cmd', { paths });
  },
  workshopUploadSaveMeta: async (paths: StudioPathsUI, folder: string, title: string, description: string, tags: string[], visibility: string): Promise<WorkshopItemMeta> => {
    return await invoke<WorkshopItemMeta>('workshop_upload_save_meta_cmd', { paths, folder, title, description, tags, visibility });
  },
  /** Makes a Workshop item folder for a mod that has none. */
  workshopUploadPrepare: async (paths: StudioPathsUI, rootDir: string): Promise<WorkshopItemMeta> => {
    return await invoke<WorkshopItemMeta>('workshop_upload_prepare_cmd', { paths, rootDir });
  },
  /** Takes an item folder out of Zomboid/Workshop (to the Recycle Bin). Nothing on Steam changes. */
  workshopUploadRemove: async (paths: StudioPathsUI, folder: string): Promise<void> => {
    return await invoke('workshop_upload_remove_cmd', { paths, folder });
  },
  /** The files an upload would send, with what changed since the last upload from here. */
  workshopUploadPreview: async (paths: StudioPathsUI, folder: string): Promise<UploadPreview> => {
    return await invoke<UploadPreview>('workshop_upload_preview_cmd', { paths, folder });
  },
  /** Publishes the item through the running Steam client (minutes for a big mod). */
  workshopPublish: async (paths: StudioPathsUI, folder: string, changeNote: string): Promise<PublishOutcome> => {
    return await invoke<PublishOutcome>('workshop_publish_cmd', { paths, folder, changeNote });
  },
  listenWorkshopUploadProgress: async (callback: (p: PublishProgress) => void): Promise<UnlistenFn> => {
    return await listen<PublishProgress>('workshop-upload-progress', (event) => callback(event.payload));
  },
  /** The names a mod's Lua calls that the installed build does not have, by file and line. */
  apiLintMod: async (paths: StudioPathsUI, modId: string, rootDir: string): Promise<ModApiReport> => {
    return await invoke<ModApiReport>('api_lint_mod_cmd', { paths, modId, rootDir });
  },
  /** A mod's translation languages: the source language and how much of it each language has. */
  translationDraftsStatus: async (rootDir: string): Promise<DraftStatus> => {
    return await invoke<DraftStatus>('translation_drafts_status_cmd', { rootDir });
  },
  /** Writes the missing keys of those languages, translated from the mod's source language. */
  writeTranslationDrafts: async (rootDir: string, langs: string[]): Promise<DraftReport> => {
    return await invoke<DraftReport>('write_translation_drafts_cmd', { rootDir, langs });
  },
  validateMod: async (paths: StudioPathsUI, rootDir: string): Promise<ModValidation> => {
    try {
      return await invoke<ModValidation>('validate_mod_cmd', { paths, rootDir });
    } catch (err) {
      reportBackendError('Checking the mod', err);
      throw err;
    }
  },

  /** Opens a file at a line in VS Code or the file's own program; resolves to which. */
  openInEditor: async (paths: StudioPathsUI, file: string, line: number | null, col: number | null): Promise<string> => {
    try {
      return await invoke<string>('open_in_editor_cmd', { paths, file, line, col });
    } catch (err) {
      reportBackendError('Opening the file', err);
      throw err;
    }
  },

  /** The multiplayer check of one mod folder; throws with the reason. */
  mpCheckMod: async (paths: StudioPathsUI, rootDir: string): Promise<MpReport> => {
    try {
      return await invoke<MpReport>('mp_check_mod_cmd', { paths, rootDir });
    } catch (err) {
      reportBackendError('Checking the mod for multiplayer', err);
      throw err;
    }
  },

  /** The multiplayer check of several mods by id (the active list, usually). */
  mpCheckMods: async (paths: StudioPathsUI, modIds: string[]): Promise<MpModReport[]> => {
    try {
      return await invoke<MpModReport[]>('mp_check_mods_cmd', { paths, modIds });
    } catch {
      return [];
    }
  },

  /** TileZed and WorldEd, looked for in the named folder and the usual places. */
  mapTools: async (configured: string[]): Promise<MapTools> => {
    try {
      return await invoke<MapTools>('map_tools_cmd', { configured });
    } catch {
      return { tilezed: null, worlded: null, searched: [] };
    }
  },

  launchMapTool: async (exe: string): Promise<void> => {
    try {
      await invoke('launch_map_tool_cmd', { exe });
    } catch (err) {
      reportBackendError('Starting the map tool', err);
      throw err;
    }
  },

  /** Makes a new mod under Zomboid/mods; throws with the reason. */
  createMod: async (userZomboidDir: string, newMod: NewMod): Promise<CreatedMod> => {
    try {
      return await invoke<CreatedMod>('create_mod_cmd', { userZomboidDir, newMod });
    } catch (err) {
      reportBackendError('Making the mod', err);
      throw err;
    }
  },

  /** The mod's Lua files changed after a moment (unix ms); all for 0. */
  changedLuaFiles: async (rootDir: string, sinceMs: number): Promise<ChangedFile[]> => {
    try {
      return await invoke<ChangedFile[]>('changed_lua_files_cmd', { rootDir, sinceMs });
    } catch {
      return [];
    }
  },

  /** Watches a mod's folder: each Lua file saved is checked and reloaded in the running game. */
  devWatchStart: async (paths: StudioPathsUI, rootDir: string): Promise<boolean> => {
    try {
      await invoke('dev_watch_start_cmd', { paths, rootDir });
      return true;
    } catch (err) {
      reportBackendError('Watching the mod', err);
      return false;
    }
  },

  devWatchStop: async (paths: StudioPathsUI): Promise<void> => {
    try {
      await invoke('dev_watch_stop_cmd', { paths });
    } catch {
      // Nothing to stop.
    }
  },

  /** Every Lua file of the mod, reloaded now; the stages come as `dev-reload` events. */
  devReloadAll: async (paths: StudioPathsUI, rootDir: string): Promise<void> => {
    try {
      await invoke('dev_reload_all_cmd', { paths, rootDir });
    } catch (err) {
      reportBackendError('Reloading Lua in the game', err);
    }
  },

  listenDevReload: async (callback: (e: DevReloadEvent) => void): Promise<UnlistenFn> => {
    return await listen<DevReloadEvent>('dev-reload', (event) => callback(event.payload));
  },

  /** The game's pid when ProjectZomboid64.exe is running, else null. */
  gameStatus: async (): Promise<number | null> => {
    try {
      return await invoke<number | null>('game_status_cmd');
    } catch {
      return null;
    }
  },

  /** The player's lists of mods, Favorites first. */
  modLists: async (userZomboidDir: string): Promise<ModList[]> => {
    try {
      return await invoke<ModList[]>('mod_lists_cmd', { userZomboidDir });
    } catch {
      return [];
    }
  },

  createModList: async (userZomboidDir: string, name: string): Promise<ModList> => {
    try {
      return await invoke<ModList>('create_mod_list_cmd', { userZomboidDir, name });
    } catch (err) {
      reportBackendError('Making a list', err);
      throw err;
    }
  },

  renameModList: async (userZomboidDir: string, listId: string, name: string): Promise<ModList> => {
    try {
      return await invoke<ModList>('rename_mod_list_cmd', { userZomboidDir, listId, name });
    } catch (err) {
      reportBackendError('Renaming a list', err);
      throw err;
    }
  },

  deleteModList: async (userZomboidDir: string, listId: string): Promise<ModList[]> => {
    try {
      return await invoke<ModList[]>('delete_mod_list_cmd', { userZomboidDir, listId });
    } catch (err) {
      reportBackendError('Deleting a list', err);
      throw err;
    }
  },

  addToModList: async (userZomboidDir: string, listId: string, item: ModListItem): Promise<ModList> => {
    try {
      return await invoke<ModList>('add_to_mod_list_cmd', { userZomboidDir, listId, item });
    } catch (err) {
      reportBackendError('Adding to a list', err);
      throw err;
    }
  },

  removeFromModList: async (userZomboidDir: string, listId: string, key: string): Promise<ModList> => {
    try {
      return await invoke<ModList>('remove_from_mod_list_cmd', { userZomboidDir, listId, key });
    } catch (err) {
      reportBackendError('Taking out of a list', err);
      throw err;
    }
  },

  /**
   * Favorites and the game's own stars made one list: a star put or taken off on either side is
   * the same on the other. Returns every list, as mod_lists_cmd.
   */
  syncGameFavourites: async (userZomboidDir: string, installed: { mod_id: string; workshop_id: string | null; name: string }[]): Promise<ModList[] | null> => {
    try {
      return await invoke<ModList[]>('sync_game_favourites_cmd', { userZomboidDir, installed });
    } catch {
      // Tried again on the next round; the app's own stars are untouched.
      return null;
    }
  },

  /** How long each mod's files took to load in the last session (the game's DebugLog); empty when there is none. */
  modLoadTimes: async (paths: StudioPathsUI): Promise<LoadTimes> => {
    try {
      return await invoke<LoadTimes>('mod_load_times_cmd', { paths });
    } catch {
      return { log_file: null, session: null, mods: [] };
    }
  },

  /** The memory the game may use, the PC's, and what the Studio suggests. */
  gameMemory: async (paths: StudioPathsUI): Promise<GameMemory | null> => {
    try {
      return await invoke<GameMemory>('game_memory_cmd', { paths });
    } catch {
      return null;
    }
  },

  /** Gives the game this much memory (MB) from its next start. Throws with the reason. */
  setGameMemory: async (paths: StudioPathsUI, mb: number): Promise<GameMemory> => {
    return await invoke<GameMemory>('set_game_memory_cmd', { paths, mb });
  },

  /** The game's last starts, stage by stage, newest first. */
  bootHistory: async (paths: StudioPathsUI): Promise<BootTimes[]> => {
    try {
      return await invoke<BootTimes[]>('boot_history_cmd', { paths });
    } catch {
      return [];
    }
  },

  /** Whether the in-game profiler is on and what it measured last. */
  /** Frame times and per-mod Lua costs of the session, from Smooth Frames' console lines; null without them. */
  smoothFrames: async (paths: StudioPathsUI): Promise<SmoothFramesReport | null> => {
    try {
      return await invoke<SmoothFramesReport | null>('smooth_frames_cmd', { paths });
    } catch {
      return null;
    }
  },

  profilerStatus: async (paths: StudioPathsUI): Promise<ProfilerStatus> => {
    try {
      return await invoke<ProfilerStatus>('profiler_status_cmd', { paths });
    } catch {
      return { on: false, saves: [], report: null };
    }
  },

  /** Puts the profiler first in the mod list (and the last save's), or takes it out. */
  setProfiler: async (paths: StudioPathsUI, on: boolean): Promise<ProfilerStatus> => {
    try {
      return await invoke<ProfilerStatus>('profiler_set_cmd', { paths, on });
    } catch (e) {
      reportBackendError('Switching the in-game profiler', e);
      throw e;
    }
  },

  modErrorStats: async (paths: StudioPathsUI): Promise<ModErrorStat[]> => {
    try {
      return await invoke<ModErrorStat[]>('mod_error_stats_cmd', { paths });
    } catch {
      return [];
    }
  },

  /**
   * Mod Health's warnings for the Studio Link mod, which marks those mods in the game's Mods
   * screen: [severity, mod id, text in the player's language] each.
   */
  /** Each mod's origin, library flag and tags for the game's Mods screen filters. */
  writeLinkMods: async (paths: StudioPathsUI, text: string): Promise<void> => {
    try {
      await invoke('write_link_mods_cmd', { paths, text });
    } catch {
      // The game's extra filters are a convenience; the app has them all.
    }
  },
  writeLinkHealth: async (paths: StudioPathsUI, lines: [string, string, string][]): Promise<void> => {
    try {
      await invoke('write_link_health_cmd', { paths, lines });
    } catch {
      // The marks are a convenience; the Mod List says the same.
    }
  },

  /** The active profile was just played: the Studio notes the installed build on it. */
  markProfilePlayed: async (userZomboidDir: string, instanceId: string): Promise<string | null> => {
    try {
      return await invoke<string | null>('mark_profile_played_cmd', { userZomboidDir, instanceId });
    } catch {
      return null;
    }
  },

  listKeptVersions: async (paths: StudioPathsUI): Promise<KeptVersion[]> => {
    try {
      return await invoke<KeptVersion[]>('list_kept_versions_cmd', { paths });
    } catch {
      return [];
    }
  },

  /** Puts a kept version back as a local copy in Zomboid/mods that wins over the Workshop one. */
  restoreKeptVersion: async (paths: StudioPathsUI, path: string): Promise<KeptVersion> => {
    try {
      return await invoke<KeptVersion>('restore_kept_version_cmd', { paths, path });
    } catch (err) {
      reportBackendError('Putting the kept version back', err);
      throw err;
    }
  },

  deleteKeptVersion: async (paths: StudioPathsUI, path: string): Promise<void> => {
    try {
      await invoke('delete_kept_version_cmd', { paths, path });
    } catch (err) {
      reportBackendError('Deleting the kept version', err);
      throw err;
    }
  },

  /** Asks Steam for its server list; servers arrive through listenPublicServers while they
   * answer. Resolves with how many were found. */
  publicServers: async (): Promise<number> => {
    return await invoke<number>('public_servers_cmd');
  },

  listenPublicServers: async (callback: (batch: PublicServer[]) => void): Promise<UnlistenFn> => {
    return await listen<PublicServer[]>('public-servers', (e) => callback(e.payload));
  },

  stopServerSearch: async (): Promise<void> => {
    await invoke('stop_server_search_cmd');
  },

  /** Asks one server about itself. Throws with the reason when it does not answer. */
  serverRules: async (address: string): Promise<ServerRules> => {
    return await invoke<ServerRules>('server_rules_cmd', { address });
  },

  savedServers: async (paths: StudioPathsUI): Promise<SavedServer[]> => {
    try {
      return await invoke<SavedServer[]>('saved_servers_cmd', { paths });
    } catch (err) {
      reportBackendError('Reading your servers', err);
      return [];
    }
  },

  /** `password`: null leaves the kept one, '' forgets it. */
  saveServer: async (paths: StudioPathsUI, server: SavedServer, password: string | null): Promise<SavedServer[]> => {
    return await invoke<SavedServer[]>('save_server_cmd', { paths, server, password });
  },

  forgetServer: async (paths: StudioPathsUI, address: string): Promise<SavedServer[]> => {
    return await invoke<SavedServer[]>('forget_server_cmd', { paths, address });
  },

  /** Starts the game joining this server. Throws when the game is already running. */
  /** Keeps the profile just made from a server with it, for its pane to say so. */
  setServerMadeProfile: async (paths: StudioPathsUI, address: string, name: string, profileId: string): Promise<SavedServer[]> => {
    try {
      return await invoke<SavedServer[]>('set_server_made_profile_cmd', { paths, address, name, profileId });
    } catch {
      return [];
    }
  },

  /** testMode: the launcher's mode (Custom launch); none is the plain game. */
  joinServer: async (paths: StudioPathsUI, address: string, name: string, password: string | null, username: string | null, testMode?: GameLaunchMode): Promise<number> => {
    return await invoke<number>('join_server_cmd', { paths, address, name, password, username, testMode: testMode ?? null });
  },
  /** The save the next launch loads (the Studio Link loads it at the main menu); null clears it. */
  setLaunchSave: async (paths: StudioPathsUI, save: string | null): Promise<void> => {
    await invoke('set_launch_save_cmd', { paths, save });
  },

  /** Which Mods screen the game uses when another mod manager is on: [choice, that manager]. */
  linkScreenChoice: async (paths: StudioPathsUI): Promise<[string, string]> => {
    try {
      return await invoke<[string, string]>('link_screen_choice_cmd', { paths });
    } catch {
      return ['ask', ''];
    }
  },

  setLinkScreenChoice: async (paths: StudioPathsUI, screen: 'ask' | 'studio' | 'other'): Promise<void> => {
    try {
      await invoke('set_link_screen_choice_cmd', { paths, screen });
    } catch (err) {
      reportBackendError('Saving the Mods screen choice', err);
    }
  },

  /** The player's notes on mods and groups of mods (the game's Link shows them too). */
  modMeta: async (paths: StudioPathsUI): Promise<ModMeta> => {
    try {
      return await invoke<ModMeta>('mod_meta_cmd', { paths });
    } catch {
      return { notes: {}, groups: [] };
    }
  },
  setModNote: async (paths: StudioPathsUI, modId: string, note: string): Promise<ModMeta> => {
    return await invoke<ModMeta>('set_mod_note_cmd', { paths, modId, note });
  },
  setInModGroup: async (paths: StudioPathsUI, group: string, modIds: string[], inside: boolean): Promise<ModMeta> => {
    return await invoke<ModMeta>('set_in_mod_group_cmd', { paths, group, modIds, inside });
  },
  renameModGroup: async (paths: StudioPathsUI, from: string, to: string): Promise<ModMeta> => {
    return await invoke<ModMeta>('rename_mod_group_cmd', { paths, from, to });
  },
  setModGroupNote: async (paths: StudioPathsUI, name: string, note: string): Promise<ModMeta> => {
    return await invoke<ModMeta>('set_mod_group_note_cmd', { paths, name, note });
  },
  deleteModGroup: async (paths: StudioPathsUI, name: string): Promise<ModMeta> => {
    return await invoke<ModMeta>('delete_mod_group_cmd', { paths, name });
  },

  /** How the Studio's window opens and what its close button does. */
  windowPrefs: async (): Promise<WindowPrefs | null> => {
    try {
      return await invoke<WindowPrefs>('window_prefs_cmd');
    } catch {
      return null;
    }
  },

  /** Saves them, with the tray menu's words in the Studio's language. */
  setWindowPrefs: async (prefs: Pick<WindowPrefs, 'remember_window' | 'close_to_tray'>): Promise<void> => {
    await invoke('set_window_prefs_cmd', {
      prefs: { ...prefs, tray_open: i18next.t('settings.trayOpen'), tray_quit: i18next.t('settings.trayQuit'), tray_tip: 'PZ Mod Studio' },
    });
  },

  /** Whether a Steam Web API key is saved (the key is never returned). */
  /** Mod texts (Steam BBCode) in `target` ("es"); each text goes out once, then comes from disk. */
  translateTexts: async (texts: string[], target: string): Promise<Translation> => {
    return await invoke<Translation>('translate_texts_cmd', { texts, target });
  },
  deeplKeyStatus: async (): Promise<boolean> => {
    try {
      return await invoke<boolean>('deepl_key_status_cmd');
    } catch {
      return false;
    }
  },
  /** Checks the key with DeepL and keeps it in Windows Credential Manager. */
  setDeeplKey: async (key: string): Promise<void> => {
    await invoke('set_deepl_key_cmd', { key });
  },
  deleteDeeplKey: async (): Promise<void> => {
    await invoke('delete_deepl_key_cmd');
  },
  /** How many translations are kept on disk, and how big they are. */
  translationCacheInfo: async (): Promise<{ count: number; bytes: number }> => {
    try {
      return await invoke<{ count: number; bytes: number }>('translation_cache_info_cmd');
    } catch {
      return { count: 0, bytes: 0 };
    }
  },
  /** Deletes every kept translation; returns how many went. */
  clearTranslationCache: async (): Promise<number> => {
    return await invoke<number>('clear_translation_cache_cmd');
  },

  steamKeyStatus: async (): Promise<boolean> => {
    return await invoke<boolean>('steam_key_status_cmd');
  },

  /** Verifies and saves the key in Windows Credential Manager. Throws with the reason on failure. */
  setSteamKey: async (key: string): Promise<void> => {
    await invoke('set_steam_key_cmd', { key });
  },

  deleteSteamKey: async (): Promise<void> => {
    await invoke('delete_steam_key_cmd');
  },

  /** Whether a GitHub token is saved on this PC (the token never comes back). */
  githubTokenStatus: async (): Promise<boolean> => {
    try {
      return await invoke<boolean>('github_token_status_cmd');
    } catch {
      return false;
    }
  },

  /** Checks the token with GitHub and saves it; resolves to the account's login. Throws with the reason. */
  setGithubToken: async (token: string): Promise<string> => {
    return await invoke<string>('set_github_token_cmd', { token });
  },

  deleteGithubToken: async (): Promise<void> => {
    await invoke('delete_github_token_cmd');
  },

  /** "Connect with GitHub": a code to type at github.com/login/device. */
  githubDeviceStart: async (): Promise<{ user_code: string; verification_uri: string; expires_in: number }> => {
    return await invoke('github_device_start_cmd');
  },

  /** Waits until the code is approved on GitHub; the token is kept; returns the login. */
  githubDeviceWait: async (): Promise<string> => {
    return await invoke<string>('github_device_wait_cmd');
  },

  githubDeviceCancel: async (): Promise<void> => {
    await invoke('github_device_cancel_cmd');
  },

  communityLists: async (paths: StudioPathsUI, refresh = false): Promise<CommunityLists | null> => {
    try {
      return await invoke<CommunityLists>('community_lists_cmd', { paths, refresh });
    } catch (err) {
      reportBackendError('Loading the community lists', err);
      return null;
    }
  },

  copyCommunityList: async (paths: StudioPathsUI, listId: string): Promise<ModList> => {
    return await invoke<ModList>('copy_community_list_cmd', { paths, listId });
  },

  draftSharedList: async (paths: StudioPathsUI, listId: string, author: string, description: string, build: string): Promise<SharedList> => {
    return await invoke<SharedList>('draft_shared_list_cmd', { paths, listId, author, description, build });
  },

  proposeSharedList: async (list: SharedList): Promise<ProposeResult> => {
    return await invoke<ProposeResult>('propose_shared_list_cmd', { list });
  },

  /** With the saved token: forks, writes this player's own report on the mod and opens the pull request. */
  proposeReport: async (modId: string, content: string): Promise<ProposeResult> => {
    try {
      return await invoke<ProposeResult>('propose_report_cmd', { modId, content });
    } catch (err) {
      reportBackendError('Proposing to the community index', err);
      throw err;
    }
  },
  /** With the saved token: forks, writes the registry on a branch and opens the pull request. */
  proposeRegistry: async (modId: string, content: string): Promise<ProposeResult> => {
    try {
      return await invoke<ProposeResult>('propose_registry_cmd', { modId, content });
    } catch (err) {
      reportBackendError('Proposing to the community index', err);
      throw err;
    }
  },

  /** With the saved token: adds the rule to community/rules.json on a branch and opens the pull request. */
  proposeRule: async (rule: { mod_id: string; after: string; note: string; source: string }): Promise<ProposeResult> => {
    try {
      return await invoke<ProposeResult>('propose_rule_cmd', { rule });
    } catch (err) {
      reportBackendError('Proposing a community rule', err);
      throw err;
    }
  },

  /** Workshop search with the saved key. Throws on failure (no key, rejected key, offline). */
  workshopSearch: async (query: WorkshopBrowseQuery): Promise<WorkshopSearchResult> => {
    return await cached(TauriService.workshopSearchKey(query), () =>
      invoke<WorkshopSearchResult>('workshop_search_cmd', {
        query: { perPage: 30, ...query, text: query.text || null, category: query.category || null, build: query.build || null },
      })
    );
  },

  /** The cache key for a search, so a page can be warmed before the user asks for it. */
  workshopSearchKey: (q: WorkshopBrowseQuery): string =>
    `workshop:search:${q.collections ? 'c' : 'm'}:${q.sort ?? ''}:${q.days ?? ''}:${q.build ?? ''}:${q.category ?? ''}:${q.page}:${q.text ?? ''}`,

  /**
   * Subscribes or unsubscribes through the Steam client that is already signed in (no API key).
   * Steam downloads or removes the files itself. Throws with the reason.
   */
  /** What subscribing to these items needs too: their "Required items", all the way down. */
  workshopRequirements: async (ids: string[]): Promise<RequiredChain | null> => {
    try {
      return await invoke<RequiredChain>('workshop_requirements_cmd', { ids });
    } catch (err) {
      reportBackendError('Reading what the mod needs', err);
      return null;
    }
  },

  workshopSubscribe: async (paths: StudioPathsUI, ids: string[], subscribe: boolean): Promise<SubscribeOutcome> => {
    // The Workshop's own data (titles, descriptions, collections) does not change when you
    // subscribe, and whether you have a mod comes from the disk, not from here. Keeping what was
    // cached makes Back instant; clearing it made every Back ask Steam again, and a refused or
    // slow answer then read as "no connection".
    return await invoke<SubscribeOutcome>('workshop_subscribe_cmd', { paths, ids, subscribe });
  },

  /** Whether subscribing from here works right now (Steam running and signed in). */
  steamClientReady: async (paths: StudioPathsUI): Promise<boolean> => {
    try {
      return await invoke<boolean>('steam_client_ready_cmd', { paths });
    } catch {
      return false;
    }
  },

  /** A mod's own poster/icon on disk, as a data URL the webview can show. Throws with the reason. */
  readModImage: async (paths: StudioPathsUI, path: string): Promise<string> => {
    return await invoke<string>('read_mod_image_cmd', { paths, path });
  },

  /** Looks up pasted Workshop links / ids / a collection link. Throws on failure. */
  workshopLookup: async (input: string): Promise<WorkshopLookup> => {
    // Pasted text is looked up fresh; only the two helpers below are cached.
    return await invoke<WorkshopLookup>('workshop_lookup_cmd', { input });
  },

  /**
   * Game-exact load-order sort: moves only what require/loadModAfter/loadModBefore force,
   * keeps everything else in place, Studio packages last.
   */
  sortLoadOrder: async (paths: StudioPathsUI, order: string[]): Promise<SortResult> => {
    try {
      return await invoke<SortResult>('sort_load_order_cmd', { paths, order });
    } catch (err) {
      reportBackendError('Sorting the load order', err);
      throw err;
    }
  },

  /**
   * Validates Lua AST syntax using Rust full_moon parser
   */
  validateLuaSyntax: async (code: string): Promise<LuaSyntaxResult> => {
    try {
      return await invoke<LuaSyntaxResult>('validate_lua_syntax_cmd', { code });
    } catch (err) {
      return { is_valid: false, error_message: String(err) };
    }
  },

  /**
   * Performs 3-way AST merge on Lua code
   */
  threeWayMergeLua: async (base: string, targetA: string, targetB: string): Promise<MergeChunkResult> => {
    try {
      return await invoke<MergeChunkResult>('three_way_merge_lua_cmd', { base, targetA, targetB });
    } catch (err) {
      return { merged_text: base, has_conflict: true };
    }
  },

  /**
   * Reads ModListData.ini file
   */
  readModListIni: async (iniPath: string): Promise<ModListDataRaw> => {
    try {
      return await invoke<ModListDataRaw>('read_mod_list_ini_cmd', { iniPath });
    } catch (err) {
      return { active_mods: [], raw_ini_content: '' };
    }
  },

  /**
   * Writes active mod load order back to ModListData.ini
   */
  writeModListIni: async (iniPath: string, activeMods: string[]): Promise<void> => {
    try {
      await invoke('write_mod_list_ini_cmd', { iniPath, activeMods });
    } catch (err) {
      reportBackendError('Saving the mod list', err);
      throw err;
    }
  },

  /**
   * Generates the synthetic master patch mod under Zomboid/mods/Z_PZModStudio_MergedPatch
   */
  /**
   * One more file into a package, published in place: everything the package holds is published
   * again with it, and a packaged package stays packaged -- no unpacking, nothing leaves the game.
   */
  publishFileIntoPackage: async (
    paths: StudioPathsUI,
    packageFolderName: string,
    relativePath: string,
    content: string
  ): Promise<MasterPatchResultUI> => {
    try {
      return await invoke<MasterPatchResultUI>('publish_file_into_package_cmd', {
        userZomboidDir: paths.user_zomboid_dir,
        modListIniPath: paths.mod_list_ini_path,
        packageFolderName,
        relativePath,
        content,
      });
    } catch (err) {
      reportBackendError('Publishing the patched file', err);
      throw err;
    }
  },

  generateMasterPatch: async (req: {
    workshop_dir?: string;
    pz_install_dir?: string;
    user_zomboid_dir: string;
    mod_list_ini_path: string;
    merged_files: { relative_path: string; content: string }[];
    package_folder_name?: string;
  }): Promise<MasterPatchResultUI> => {
    try {
      return await invoke<MasterPatchResultUI>('generate_master_patch_cmd', { req });
    } catch (err) {
      reportBackendError('Generating the merge package', err);
      throw err;
    }
  },

  /**
   * Safely removes Z_PZModStudio_MergedPatch synthetic patch files from disk
   */
  cleanMasterPatch: async (req: {
    workshop_dir?: string;
    pz_install_dir?: string;
    user_zomboid_dir: string;
    mod_list_ini_path: string;
    merged_files?: { relative_path: string; content: string }[];
    package_folder_name?: string;
  }): Promise<boolean> => {
    try {
      return await invoke<boolean>('clean_master_patch_cmd', {
        req: {
          ...req,
          merged_files: req.merged_files || [],
        },
      });
    } catch (err) {
      reportBackendError('Unpacking the merge package', err);
      return false;
    }
  },

  /**
   * Reads current packaging status and metadata for Z_PZModStudio_MergedPatch
   */
  getMasterPatchStatus: async (userZomboidDir: string, modListIniPath: string, packageFolderName?: string): Promise<MasterPatchStatusInfoUI> => {
    try {
      return await invoke<MasterPatchStatusInfoUI>('get_master_patch_status_cmd', {
        userZomboidDir,
        modListIniPath,
        packageFolderName: packageFolderName || null,
      });
    } catch (err) {
      console.error('Failed to get master patch status:', err);
      return {
        is_packaged: false,
        packaged_mods: [],
        merged_files: [],
        missing_active_mods: [],
      };
    }
  },

  /**
   * Lists all existing merged packages under Zomboid/mods/Z_PZModStudio_*
   */
  listMergedPackages: async (userZomboidDir: string, modListIniPath: string): Promise<MergedPackageInfoUI[]> => {
    try {
      return await invoke<MergedPackageInfoUI[]>('list_merged_packages_cmd', { userZomboidDir, modListIniPath });
    } catch (err) {
      console.error('Failed to list merged packages:', err);
      return [];
    }
  },

  /**
   * Creates a new named merged package (e.g. "Mix Mods" -> Z_PZModStudio_MixMods)
   */
  createMergedPackage: async (userZomboidDir: string, modListIniPath: string, name: string, description?: string): Promise<MergedPackageInfoUI | null> => {
    try {
      return await invoke<MergedPackageInfoUI>('create_merged_package_cmd', { userZomboidDir, modListIniPath, name, description });
    } catch (err) {
      reportBackendError('Creating the package', err);
      return null;
    }
  },

  /**
   * Renames an existing merged package
   */
  renameMergedPackage: async (userZomboidDir: string, modListIniPath: string, oldFolder: string, newName: string, description?: string): Promise<MergedPackageInfoUI | null> => {
    try {
      return await invoke<MergedPackageInfoUI>('rename_merged_package_cmd', { userZomboidDir, modListIniPath, oldFolder, newName, description });
    } catch (err) {
      reportBackendError('Renaming the package', err);
      return null;
    }
  },

  /**
   * Toggles a merged package presence (active/hidden) in ModListData.ini / default.txt
   */
  togglePackageInModlist: async (
    userZomboidDir: string,
    modListIniPath: string,
    folderName: string,
    enabled: boolean
  ): Promise<boolean> => {
    try {
      return await invoke<boolean>('toggle_package_in_modlist_cmd', {
        userZomboidDir,
        modListIniPath,
        folderName,
        enabled,
      });
    } catch (err) {
      reportBackendError('Updating the package in the mod list', err);
      return false;
    }
  },


  /**
   * Deletes a merged package directory from disk
   */
  deleteMergedPackage: async (userZomboidDir: string, modListIniPath: string, folderName: string): Promise<boolean> => {
    try {
      return await invoke<boolean>('delete_merged_package_cmd', {
        userZomboidDir,
        modListIniPath,
        folderName,
      });
    } catch (err) {
      reportBackendError('Deleting the package', err);
      return false;
    }
  },

  saveDraftResolution: async (
    userZomboidDir: string,
    packageFolderName: string,
    relativePath: string,
    resolvedContent: string,
    status: string
  ): Promise<boolean> => {
    try {
      return await invoke<boolean>('save_draft_resolution_cmd', {
        userZomboidDir,
        packageFolderName,
        relativePath,
        resolvedContent,
        status,
      });
    } catch (err) {
      reportBackendError('Saving the draft', err);
      return false;
    }
  },

  clearDraftResolutions: async (
    userZomboidDir: string,
    packageFolderName: string
  ): Promise<boolean> => {
    try {
      return await invoke<boolean>('clear_draft_resolutions_cmd', {
        userZomboidDir,
        packageFolderName,
      });
    } catch (err) {
      reportBackendError('Clearing drafts', err);
      return false;
    }
  },
  exportMergedPackage: async (
    userZomboidDir: string,
    packageFolderName: string,
    targetFilePath: string
  ): Promise<boolean> => {
    try {
      return await invoke<boolean>('export_merged_package_cmd', {
        userZomboidDir,
        packageFolderName,
        targetFilePath,
      });
    } catch (err) {
      reportBackendError('Exporting the package', err);
      throw err;
    }
  },

  importMergedPackage: async (
    userZomboidDir: string,
    modListIniPath: string,
    sourceFilePath: string
  ): Promise<MergedPackageInfoUI> => {
    try {
      return await invoke<MergedPackageInfoUI>('import_merged_package_cmd', {
        userZomboidDir,
        modListIniPath,
        sourceFilePath,
      });
    } catch (err) {
      reportBackendError('Importing the package', err);
      throw err;
    }
  },

  /**
   * Things two or more active mods declare: the same item or recipe in differently named files,
   * which collide nowhere on disk. Asked for separately because it reads every script.
   */
  scriptConflicts: async (paths: StudioPathsUI, active: string[]): Promise<HealthIssue[]> => {
    try {
      return await invoke<HealthIssue[]>('script_conflicts_cmd', { paths, active });
    } catch {
      // Nothing is broken if this cannot be read; the rest of the health check stands on its own.
      return [];
    }
  },

  /** What a link offers, without downloading it. Throws with the reason it cannot be used. */
  resolveUrlInstall: async (url: string): Promise<UrlInstallPlan> => {
    return await invoke<UrlInstallPlan>('resolve_url_install_cmd', { url });
  },

  /** Downloads a link and installs the mod(s) in it. Throws (CONFLICT_PREFIX when folders exist). */
  installFromUrl: async (paths: StudioPathsUI, url: string, replaceExisting: boolean): Promise<InstallResult> => {
    return await invoke<InstallResult>('install_from_url_cmd', { paths, url, replaceExisting });
  },

  /** What the Build 42 shim covers, read from the shim that ships. */
  polyfillSections: async (): Promise<PolyfillSection[]> => {
    try {
      return await invoke<PolyfillSection[]>('polyfill_sections_cmd');
    } catch {
      return [];
    }
  },

  /** The shim's per-mod patches with their switches. */
  shimPatches: async (userZomboidDir: string): Promise<ShimPatch[]> => {
    try {
      return await invoke<ShimPatch[]>('shim_patches_cmd', { userZomboidDir });
    } catch {
      return [];
    }
  },

  /** Switches one per-mod patch; returns how many Studio packages on disk got the switches. */
  setShimPatch: async (userZomboidDir: string, name: string, enabled: boolean): Promise<number> => {
    try {
      return await invoke<number>('set_shim_patch_cmd', { userZomboidDir, name, enabled });
    } catch (err) {
      reportBackendError('Switching a shim patch', err);
      throw err;
    }
  },

  /** Every earlier version of the mod list the Studio kept, newest first. */
  modListHistory: async (paths: StudioPathsUI): Promise<ModListVersion[]> => {
    try {
      return await invoke<ModListVersion[]>('mod_list_history_cmd', { paths });
    } catch {
      // No history is not an error: there may simply be no saved versions yet.
      return [];
    }
  },

  /** Puts one back and returns the mods it switches on. Throws with the reason. */
  restoreModListVersion: async (paths: StudioPathsUI, backupFile: string): Promise<string[]> => {
    return await invoke<string[]>('restore_mod_list_version_cmd', { paths, backupFile });
  },

  /** The hunt for a culprit mod that the player left running, if there is one. */
  bisectState: async (paths: StudioPathsUI): Promise<BisectSession | null> => {
    try {
      return await invoke<BisectSession | null>('bisect_state_cmd', { paths });
    } catch {
      // A hunt that cannot be read is simply no hunt; the Monitor offers to start one.
      return null;
    }
  },

  /** Starts halving the list. The mods switched on now are the suspects. Throws with the reason. */
  bisectStart: async (paths: StudioPathsUI, mods: ModInfo[]): Promise<BisectSession> => {
    return await invoke<BisectSession>('bisect_start_cmd', {
      paths,
      mods: mods.map((m) => ({
        mod_id: m.mod_id,
        dependencies: m.dependencies ?? [],
        enabled: m.enabled,
      })),
    });
  },

  /** Answers the trial and switches the next one on. Throws with the reason. */
  bisectAnswer: async (paths: StudioPathsUI, stillHappens: boolean): Promise<BisectSession> => {
    return await invoke<BisectSession>('bisect_answer_cmd', { paths, stillHappens });
  },

  /** Ends the hunt and puts the player's list back, optionally without one mod. */
  bisectStop: async (paths: StudioPathsUI, switchOff: string | null): Promise<string[]> => {
    return await invoke<string[]>('bisect_stop_cmd', { paths, switchOff });
  },

  /**
   * Spawns isolated sandbox test run
   */
  launchSandbox: async (config: { pz_install_dir: string; user_zomboid_dir: string; test_mode: string }): Promise<number> => {
    try {
      return await invoke<number>('launch_sandbox_cmd', { config });
    } catch (err) {
      reportBackendError('Launching the game', err);
      throw err;
    }
  },

  /**
   * Listens to realtime log streaming events from Rust sandbox
   */
  listenSandboxLogs: async (callback: (payload: SandboxLogPayload) => void): Promise<UnlistenFn> => {
    return await listen<SandboxLogPayload>('sandbox-log', (event) => {
      callback(event.payload);
    });
  },

  /**
   * Listens to translated crash error cards from Rust log watcher
   */
  /**
   * Listens to translated crash error cards from Rust log watcher
   */
  /** The game wrote a fresh console.txt: a new session started, and the last one's cards are stale. */
  listenSandboxSessionStarted: async (callback: () => void): Promise<UnlistenFn> => {
    return await listen('sandbox-session-started', () => callback());
  },

  listenSandboxErrorCards: async (callback: (card: TranslatedErrorCard) => void): Promise<UnlistenFn> => {
    return await listen<TranslatedErrorCard>('sandbox-error-card', (event) => {
      callback(event.payload);
    });
  },



  // ==========================================
  // Presets & File Dialogs (.pzpack, .pzmerge)
  // ==========================================
  pickSaveFile: async (defaultName?: string, filterName?: string, filterExt?: string): Promise<string | null> => {
    try {
      return await invoke<string | null>('pick_save_file_cmd', {
        defaultName,
        filterName,
        filterExt,
      });
    } catch (err) {
      return null;
    }
  },

  pickOpenFile: async (filterName?: string, filterExt?: string): Promise<string | null> => {
    try {
      return await invoke<string | null>('pick_open_file_cmd', {
        filterName,
        filterExt,
      });
    } catch (err) {
      return null;
    }
  },

  /**
   * One Markdown file describing an error — the log line, the build, the mod, the code — so it
   * can be handed to someone, or to an AI agent, away from this machine. Throws with the reason.
   */
  buildErrorReport: async (
    paths: StudioPathsUI,
    request: {
      rawError: string;
      title: string;
      explanation: string;
      modName?: string | null;
      sourceFile?: string | null;
      logContext: string[];
    }
  ): Promise<string> => {
    return await invoke<string>('error_report_cmd', { paths, request });
  },

  /** Writes text where the user chose. Throws with the reason. */
  saveTextFile: async (path: string, contents: string): Promise<void> => {
    await invoke('save_text_file_cmd', { path, contents });
  },

  /** Where this Studio's MCP server is, for the config snippet. */
  mcpServerPath: async (): Promise<string> => {
    try {
      return await invoke<string>('mcp_server_path_cmd');
    } catch {
      return 'pz-mcp-server.exe';
    }
  },

  /** Shows a mod's folder in Explorer. Throws with the reason. */
  openModFolder: async (paths: StudioPathsUI, folder: string): Promise<void> => {
    try {
      await invoke('open_mod_folder_cmd', { paths, folder });
    } catch (err) {
      reportBackendError('Opening the folder', err);
    }
  },

  openPackageFolder: async (userZomboidDir: string, packageFolderName: string): Promise<boolean> => {
    try {
      return await invoke<boolean>('open_package_folder_cmd', {
        userZomboidDir,
        packageFolderName,
      });
    } catch (err) {
      console.error('Failed to open package folder in explorer:', err);
      return false;
    }
  },

  openLogsFolder: async (userZomboidDir: string): Promise<boolean> => {
    try {
      return await invoke<boolean>('open_logs_folder_cmd', { userZomboidDir });
    } catch (err) {
      console.error('Failed to open logs folder in explorer:', err);
      return false;
    }
  },

  listAvailableLogFiles: async (userZomboidDir: string): Promise<any[]> => {
    try {
      return await invoke<any[]>('list_available_log_files_cmd', { userZomboidDir });
    } catch (err) {
      console.error('Failed to list available log files:', err);
      return [];
    }
  },

  readLogFile: async (filePath: string, maxLines?: number): Promise<string[]> => {
    try {
      return await invoke<string[]>('read_log_file_cmd', { filePath, maxLines });
    } catch (err) {
      console.error('Failed to read log file:', err);
      return [];
    }
  },

  /**
   * Writes a .pzpack. Each mod's version and the publish time of the copy on disk are filled in
   * by the backend from this install, so the pack records what the list was really tested with.
   */
  exportPresetFile: async (
    paths: StudioPathsUI,
    preset: any,
    filePath: string,
    sandboxPreset?: string | null
  ): Promise<any> => {
    return await invoke('export_preset_file', { paths, preset, filePath, sandboxPreset: sandboxPreset ?? null });
  },

  /**
   * For each `require("...") failed`, where that module really is. Answered in one call: the
   * index covers every Lua file of the game and the active mods.
   */
  resolveFailedRequires: async (paths: StudioPathsUI, wanted: string[]): Promise<RequireAnswer[]> => {
    try {
      return await invoke<RequireAnswer[]>('resolve_failed_requires_cmd', { paths, wanted });
    } catch {
      return [];
    }
  },

  /** The text files of one installed mod, to pick one to patch. */
  modFiles: async (paths: StudioPathsUI, modId: string): Promise<ModFile[]> => {
    return await invoke<ModFile[]>('mod_files_cmd', { paths, modId });
  },

  /** The files of a mod that `require` this module: where a known path fix has to go. */
  modFilesRequiring: async (paths: StudioPathsUI, modId: string, module: string): Promise<ModFile[]> => {
    return await invoke<ModFile[]>('mod_files_requiring_cmd', { paths, modId, module });
  },

  /** Every file of every active mod that `require`s this module; the log does not always say which. */
  activeModsFilesRequiring: async (
    paths: StudioPathsUI,
    module: string
  ): Promise<Array<{ mod_id: string; mod_name: string; file: ModFile }>> => {
    return await invoke('active_mods_files_requiring_cmd', { paths, module });
  },

  /** One file out of a package, published in place: the game reads the mod's own copy again. */
  /** The active package's copies of mod files beside the mod's own: which are out of date. */
  packageFileStates: async (paths: StudioPathsUI, active: string[]): Promise<PackageFileState[]> => {
    try {
      return await invoke<PackageFileState[]>('package_file_states_cmd', { paths, active });
    } catch (err) {
      reportBackendError('Checking the package copies', err);
      return [];
    }
  },

  removeFileFromPackage: async (
    paths: StudioPathsUI,
    packageFolderName: string,
    file: string
  ): Promise<MasterPatchResultUI> => {
    try {
      return await invoke<MasterPatchResultUI>('remove_file_from_package_cmd', {
        userZomboidDir: paths.user_zomboid_dir,
        modListIniPath: paths.mod_list_ini_path,
        packageFolderName,
        file,
      });
    } catch (err) {
      reportBackendError('Removing the file from the package', err);
      throw err;
    }
  },

  /** Opens one mod's file as something the Merger can edit and save into the package. */
  modFileToPatch: async (paths: StudioPathsUI, modId: string, relativePath: string): Promise<VfsConflict> => {
    const raw = await invoke<any>('mod_file_to_patch_cmd', { paths, modId, relativePath });
    return {
      ...raw,
      competing_mods: raw.competing_files.map((f: any) => ({
        mod_id: f.mod_id,
        mod_name: f.mod_name,
        absolute_path: f.absolute_path,
        content: f.content,
      })),
      status: raw.status || 'MANUAL_CONFLICT',
    } as VfsConflict;
  },

  /** The player's own sandbox presets, so one can be put into a pack. */
  sandboxPresets: async (userZomboidDir: string): Promise<SandboxPresetInfo[]> => {
    try {
      return await invoke<SandboxPresetInfo[]>('sandbox_presets_cmd', { userZomboidDir });
    } catch {
      return [];
    }
  },

  /** Saves a pack's sandbox settings, never over one of the player's own. Returns the name. */
  writeSandboxPreset: async (userZomboidDir: string, sandbox: PresetSandbox): Promise<string> => {
    return await invoke<string>('write_sandbox_preset_cmd', { userZomboidDir, sandbox });
  },

  importPresetFile: async (filePath: string): Promise<any> => {
    return await invoke('import_preset_file', { filePath });
  },

  checkMissingPresetMods: async (preset: any, userZomboidDir: string, workshopDir: string): Promise<any> => {
    return await invoke('check_missing_preset_mods', { preset, userZomboidDir, workshopDir });
  },

  // ==========================================
  // Server Manager & Dedicated Sync
  // ==========================================
  listServerConfigs: async (userZomboidDir: string): Promise<any[]> => {
    return await invoke('list_server_configs', { userZomboidDir });
  },

  /**
   * Writes this mod list into a server's .ini: Mods=, WorkshopItems= and Map=. The Workshop ids
   * and map names come from the installed mods, so the caller does not have to work them out.
   * Returns what was written; throws with the reason.
   */
  syncClientToServer: async (
    paths: StudioPathsUI,
    filePath: string,
    activeModIds: string[]
  ): Promise<ServerLines> => {
    return await invoke<ServerLines>('sync_client_to_server', { paths, filePath, activeModIds });
  },

  /** Everything in a server's .ini that will not do what the admin meant. */
  checkServerMods: async (paths: StudioPathsUI, filePath: string): Promise<ServerModReport | null> => {
    try {
      return await invoke<ServerModReport>('check_server_mods', { paths, filePath });
    } catch {
      // An .ini that cannot be read is reported by the list itself; no need to shout twice.
      return null;
    }
  },

  createNewServerConfig: async (
    userZomboidDir: string,
    serverName: string
  ): Promise<PZServerConfig> => {
    return await invoke('create_new_server_config', { userZomboidDir, serverName });
  },

  deleteServerConfig: async (
    userZomboidDir: string,
    filePath: string,
    serverName?: string
  ): Promise<boolean> => {
    return await invoke('delete_server_config', { userZomboidDir, filePath, serverName });
  },

  launchDedicatedServer: async (
    pzInstallDir: string,
    userZomboidDir: string,
    serverName: string,
    memoryGb?: number,
    nosteam?: boolean,
    adminPassword?: string
  ): Promise<number> => {
    return await invoke('launch_dedicated_server', {
      pzInstallDir,
      userZomboidDir,
      serverName,
      memoryGb,
      nosteam,
      adminPassword,
    });
  },

  stopDedicatedServer: async (userZomboidDir: string, pid?: number): Promise<boolean> => {
    return await invoke('stop_dedicated_server', { userZomboidDir, pid });
  },

  getDedicatedServerStatus: async (userZomboidDir: string): Promise<DedicatedServerStatus> => {
    return await invoke('get_dedicated_server_status', { userZomboidDir });
  },

  getDedicatedServerLogs: async (userZomboidDir: string, maxLines?: number): Promise<string[]> => {
    return await invoke('get_dedicated_server_logs', { userZomboidDir, maxLines });
  },

  saveServerLogSnapshot: async (
    userZomboidDir: string,
    serverName: string,
    customLines?: string[]
  ): Promise<string> => {
    return await invoke('save_server_log_snapshot', {
      userZomboidDir,
      serverName,
      customLines,
    });
  },

  getConnectedPlayers: async (userZomboidDir: string): Promise<ConnectedPlayer[]> => {
    return await invoke('get_connected_players', { userZomboidDir });
  },

  sendServerCommand: async (
    userZomboidDir: string,
    action: string,
    target?: string,
    message?: string,
    reason?: string
  ): Promise<boolean> => {
    return await invoke('send_server_command', {
      userZomboidDir,
      action,
      target,
      message,
      reason,
    });
  },

  discordView: async (paths: StudioPathsUI, server: string): Promise<DiscordView | null> => {
    try {
      return await invoke<DiscordView>('discord_view_cmd', { paths, server });
    } catch {
      return null;
    }
  },

  /** `webhook`: a new address to store, "" to forget the stored one, undefined to keep it. */
  discordSave: async (paths: StudioPathsUI, server: string, settings: DiscordSettings, webhook?: string): Promise<DiscordView> => {
    return await invoke<DiscordView>('discord_save_cmd', { paths, server, settings, webhook: webhook ?? null });
  },

  gameBot: async (paths: StudioPathsUI, filePath: string): Promise<GameBot | null> => {
    try {
      return await invoke<GameBot>('game_bot_cmd', { paths, filePath });
    } catch {
      return null;
    }
  },

  saveGameBot: async (
    paths: StudioPathsUI,
    filePath: string,
    v: { enabled: boolean; token: string | null; chatChannel: string; logChannel: string; clearCommandChannel: boolean }
  ): Promise<GameBot> => {
    return await invoke<GameBot>('save_game_bot_cmd', { paths, filePath, ...v });
  },

  discordCommandsStatus: async (): Promise<DiscordCommandsStatus | null> => {
    try {
      return await invoke<DiscordCommandsStatus | null>('discord_commands_status_cmd');
    } catch {
      return null;
    }
  },

  discordTest: async (paths: StudioPathsUI, server: string): Promise<void> => {
    await invoke('discord_test_cmd', { paths, server });
  },

  getServerQuickSettings: async (filePath: string): Promise<ServerQuickSettings> => {
    return await invoke('get_server_quick_settings', { filePath });
  },

  saveServerQuickSettings: async (filePath: string, settings: ServerQuickSettings): Promise<boolean> => {
    return await invoke('save_server_quick_settings', { filePath, settings });
  },

  // ==========================================
  // Modrinth-Style Instance & Profile Manager
  // ==========================================
  listInstances: async (userZomboidDir: string): Promise<any[]> => {
    return await invoke('list_instances', { userZomboidDir });
  },

  createInstance: async (
    userZomboidDir: string,
    name: string,
    description: string | undefined,
    activeModIds: string[],
    loadOrder: string[]
  ): Promise<any> => {
    return await invoke('create_instance', { userZomboidDir, name, description, activeModIds, loadOrder });
  },

  /**
   * What switching this list on would change in the game's launch file (Java agents such as
   * ZombieBuddy's), without changing it; and whether the game is running (then it waits).
   */
  javaAgentsPlan: async (paths: StudioPathsUI, activeModIds: string[]): Promise<{ removed: string[]; restored: string[]; removed_args: string[]; restored_args: string[]; file: string; gameRunning: boolean } | null> => {
    try {
      const [plan, gameRunning] = await invoke<[{ removed: string[]; restored: string[]; removed_args: string[]; restored_args: string[]; file: string }, boolean]>('java_agents_plan_cmd', { paths, activeModIds });
      return { ...plan, gameRunning };
    } catch {
      return null;
    }
  },

  activateInstance: async (userZomboidDir: string, instanceId: string): Promise<void> => {
    return await invoke('activate_instance', { userZomboidDir, instanceId });
  },

  deleteInstance: async (userZomboidDir: string, instanceId: string): Promise<void> => {
    return await invoke('delete_instance', { userZomboidDir, instanceId });
  },

  updateInstance: async (userZomboidDir: string, instance: any): Promise<any> => {
    return await invoke('update_instance', { userZomboidDir, instance });
  },

  /** Which mods belong to a profile, on or off; one taken out is switched off too. */
  setProfileMembers: async (userZomboidDir: string, instanceId: string, members: string[]): Promise<AppInstance> => {
    try {
      return await invoke<AppInstance>('set_profile_members', { userZomboidDir, instanceId, members });
    } catch (err) {
      reportBackendError('Changing the mods of the profile', err);
      throw err;
    }
  },

  /** The game's world and every installed map mod's cells, and where two of them clash. */
  mapCells: async (paths: StudioPathsUI): Promise<MapCells | null> => {
    try {
      return await invoke<MapCells>('map_cells_cmd', { paths });
    } catch (err) {
      reportBackendError('Reading the map mods', err);
      return null;
    }
  },

  /** The real map images (the game's world and map mods with one) at about `pxPerCell` per cell. */
  mapImages: async (paths: StudioPathsUI, pxPerCell: number): Promise<MapImage[]> => {
    try {
      return await invoke<MapImage[]>('map_images_cmd', { paths, pxPerCell });
    } catch (err) {
      reportBackendError('Drawing the real maps', err);
      return [];
    }
  },

  /** The PNG of one map image, as a URL the page can draw (revoke it when done). */
  mapImageUrl: async (paths: StudioPathsUI, id: string): Promise<string | null> => {
    try {
      const bytes = await invoke<ArrayBuffer>('map_image_bytes_cmd', { paths, id });
      return URL.createObjectURL(new Blob([bytes], { type: 'image/png' }));
    } catch (err) {
      reportBackendError('Drawing the real maps', err);
      return null;
    }
  },

  /** Everything a bug report needs, with the user name and the PC's name masked. */
  diagnosticsReport: async (paths: StudioPathsUI): Promise<string | null> => {
    try {
      return await invoke<string>('diagnostics_report_cmd', { paths });
    } catch (err) {
      reportBackendError('Gathering the diagnostics', err);
      return null;
    }
  },

  openStudioLogFolder: async (): Promise<void> => {
    try {
      await invoke('open_studio_log_folder_cmd');
    } catch (err) {
      reportBackendError('Opening the logs folder', err);
    }
  },

  /** A background job of the Studio panicked: the text says where. */
  listenStudioPanic: async (callback: (text: string) => void): Promise<UnlistenFn> => {
    return await listen<string>('studio-panic', (event) => callback(event.payload));
  },

  /** Every save, its backups, and the room on the disk. */
  savesOverview: async (paths: StudioPathsUI): Promise<SavesOverview | null> => {
    try {
      return await invoke<SavesOverview>('saves_overview_cmd', { paths });
    } catch (err) {
      reportBackendError('Reading the saves', err);
      return null;
    }
  },

  /**
   * Workshop mods updated since the last session (or `since`), or waiting in Steam. With
   * `withNotes` their change notes are read too (cached); without, nothing leaves the PC.
   */
  /** A mod's own changelog and its author's support links, if it ships one. */
  modChangelog: async (paths: StudioPathsUI, modId: string): Promise<ModChangelog | null> => {
    try {
      return await invoke<ModChangelog | null>('mod_changelog_cmd', { paths, modId });
    } catch {
      return null;
    }
  },

  changelogMarkRead: async (paths: StudioPathsUI, modId: string): Promise<void> => {
    await invoke('changelog_mark_read_cmd', { paths, modId });
  },

  whatsNew: async (paths: StudioPathsUI, withNotes: boolean, since: number | null): Promise<WhatsNew | null> => {
    try {
      return await invoke<WhatsNew>('whats_new_cmd', { paths, withNotes, since });
    } catch (err) {
      reportBackendError("Reading what's new", err);
      return null;
    }
  },

  /** For each save with a recorded session: mods added, removed and updated since. */
  diskReport: async (paths: StudioPathsUI): Promise<DiskReport> => {
    return await invoke<DiskReport>('disk_report_cmd', { paths });
  },
  /** An unused local mod folder or leftover Workshop folder, to the Recycle Bin. */
  diskRecycle: async (paths: StudioPathsUI, folder: string): Promise<void> => {
    await invoke('disk_recycle_cmd', { paths, folder });
  },
  saveMissingMods: async (paths: StudioPathsUI): Promise<Record<string, SaveMissingMod[]>> => {
    try {
      return await invoke<Record<string, SaveMissingMod[]>>('save_missing_mods_cmd', { paths });
    } catch {
      return {};
    }
  },
  /** For each save that misses a map its own mods bring: what it loads and what it misses. */
  saveMissingMaps: async (paths: StudioPathsUI): Promise<Record<string, SaveMaps>> => {
    try {
      return await invoke<Record<string, SaveMaps>>('save_missing_maps_cmd', { paths });
    } catch {
      return {};
    }
  },
  /** For each console line: seen in an earlier session or not, in how many, since when. */
  errorHistory: async (paths: StudioPathsUI, lines: string[]): Promise<ErrorHistoryView | null> => {
    try {
      return await invoke<ErrorHistoryView>('error_history_cmd', { paths, lines });
    } catch {
      return null;
    }
  },
  /** The characters and vehicles of a save, with positions and the item types they carry. Throws with the reason. */
  saveInside: async (paths: StudioPathsUI, saveId: string): Promise<SaveInside> => {
    return await invoke<SaveInside>('save_inside_cmd', { paths, saveId });
  },
  /** Gives a save the maps its mods bring, after a backup. Throws with the reason (the game running, no room). */
  saveAddMaps: async (paths: StudioPathsUI, saveId: string): Promise<SaveMapsFix> => {
    return await invoke<SaveMapsFix>('save_add_maps_cmd', { paths, saveId });
  },
  saveSessionDiffs: async (paths: StudioPathsUI): Promise<Record<string, SessionDiff>> => {
    try {
      return await invoke<Record<string, SessionDiff>>('save_session_diffs_cmd', { paths });
    } catch {
      return {};
    }
  },

  /** A backup the player asked for (kept until removed). Throws with the reason when it fails. */
  backupSave: async (paths: StudioPathsUI, saveId: string): Promise<SaveBackupOutcome> => {
    return await invoke<SaveBackupOutcome>('backup_save_cmd', { paths, saveId });
  },

  /** Puts a backup back as the save; the save as it is is backed up first. */
  restoreSaveBackup: async (paths: StudioPathsUI, saveId: string, file: string): Promise<SaveBackupOutcome | null> => {
    return await invoke<SaveBackupOutcome | null>('restore_save_backup_cmd', { paths, saveId, file });
  },

  deleteSaveBackup: async (paths: StudioPathsUI, saveId: string, file: string): Promise<void> => {
    return await invoke('delete_save_backup_cmd', { paths, saveId, file });
  },

  setSaveBackupKept: async (paths: StudioPathsUI, saveId: string, file: string, kept: boolean): Promise<SaveBackupEntry> => {
    return await invoke<SaveBackupEntry>('set_save_backup_kept_cmd', { paths, saveId, file, kept });
  },

  openBackupsFolder: async (paths: StudioPathsUI): Promise<void> => {
    try {
      await invoke('open_backups_folder_cmd', { paths });
    } catch (err) {
      reportBackendError('Opening the backups folder', err);
    }
  },

  /** A save was backed up (or a backup made, restored or removed) by the Studio. */
  listenSaveBackupsChanged: async (callback: (saveId: string) => void): Promise<UnlistenFn> => {
    return await listen<string>('save-backups-changed', (event) => callback(event.payload));
  },

  /** A launch-file change left for later (the game was running) was made now. */
  listenJavaAgentsSynced: async (callback: (changes: { removed: string[]; restored: string[] }) => void): Promise<UnlistenFn> => {
    return await listen<{ removed: string[]; restored: string[] }>('java-agents-synced', (event) => callback(event.payload));
  },

  /** The game entered the menu or a save (the Link mod's game file). */
  listenLinkGameState: async (callback: () => void): Promise<UnlistenFn> => {
    return await listen('link-game-state', () => callback());
  },

  /** The game's Mods screen changed the active profile (mods switched on or off, or reordered). */
  listenProfileChangedInGame: async (callback: (profile: AppInstance) => void): Promise<UnlistenFn> => {
    return await listen<AppInstance>('profile-changed-in-game', (event) => callback(event.payload));
  },

  /** The game asked to see one of the Studio's screens ("monitor", from its error notice). */
  listenStudioNavigate: async (callback: (screen: string) => void): Promise<UnlistenFn> => {
    return await listen<string>('studio-navigate', (event) => callback(event.payload));
  },

  saveMasterLoadOrder: async (
    userZomboidDir: string,
    loadOrder: string[],
    activeModIds: string[]
  ): Promise<void> => {
    return await invoke('save_master_load_order', { userZomboidDir, loadOrder, activeModIds });
  },
};

/**
 * The calls that change something, and what they touch. After each one resolves the screens
 * read their lists again (services/changes.ts), so no screen has to be left and opened again to
 * show what a button just did. Not here: what App writes on its own as a consequence of what
 * it already shows (the game's list on launch, the Link files, the launch target, the window),
 * which would only make the screens reload for nothing. Nor the favourites sync with the game
 * (every 10 s): services/modLists.ts tells its own screens when the lists really changed.
 */
const WRITES: Record<string, ChangeScope> = {
  // The installed mods, the packages, the game's active list.
  installMods: 'mods', installFromUrl: 'mods', makeCompatibleCopy: 'mods', removeCompatibleCopy: 'mods', autoMergeConflicts: 'mods',
  workshopSubscribe: 'mods', diskRecycle: 'mods', restoreKeptVersion: 'mods', createMod: 'mods',
  createMergedPackage: 'mods', renameMergedPackage: 'mods', deleteMergedPackage: 'mods', togglePackageInModlist: 'mods',
  importMergedPackage: 'mods', publishFileIntoPackage: 'mods', removeFileFromPackage: 'mods', generateMasterPatch: 'mods', cleanMasterPatch: 'mods',
  restoreModListVersion: 'mods', bisectStart: 'mods', bisectAnswer: 'mods', bisectStop: 'mods', setProfiler: 'mods',
  // The profiles.
  createInstance: 'profiles', activateInstance: 'profiles', deleteInstance: 'profiles', updateInstance: 'profiles', setProfileMembers: 'profiles',
  setGameMemory: 'profiles',
  // Lists a screen keeps.
  workshopUploadSaveMeta: 'other', workshopUploadPrepare: 'mods', workshopUploadRemove: 'mods', workshopPublish: 'other',
  keepModVersion: 'other', deleteKeptVersion: 'other', addLoadRule: 'other', removeLoadRule: 'other',
  createModList: 'other', renameModList: 'other', deleteModList: 'other', addToModList: 'other', removeFromModList: 'other',
  copyCommunityList: 'other', refreshCommunity: 'other', setCommunityDownloads: 'other', setAppUpdateCheck: 'other', dismissAppUpdate: 'other',
  setModNote: 'other', setInModGroup: 'other', renameModGroup: 'other', setModGroupNote: 'other', deleteModGroup: 'other',
  saveServer: 'other', forgetServer: 'other', setServerMadeProfile: 'other', setLinkScreenChoice: 'other', setShimPatch: 'other',
  setSteamKey: 'other', deleteSteamKey: 'other', setDeeplKey: 'other', deleteDeeplKey: 'other', clearTranslationCache: 'other', setGithubToken: 'other', deleteGithubToken: 'other',
  saveDraftResolution: 'other', clearDraftResolutions: 'other', writeSandboxPreset: 'other', importPresetFile: 'other',
  syncClientToServer: 'other', createNewServerConfig: 'other', deleteServerConfig: 'other', launchDedicatedServer: 'other', stopDedicatedServer: 'other',
  discordSave: 'other', saveGameBot: 'other', saveServerQuickSettings: 'other', changelogMarkRead: 'other',
  backupSave: 'other', restoreSaveBackup: 'other', deleteSaveBackup: 'other', setSaveBackupKept: 'other', saveAddMaps: 'other',
};
for (const [name, scope] of Object.entries(WRITES)) {
  const service = TauriService as unknown as Record<string, (...args: unknown[]) => Promise<unknown>>;
  const call = service[name];
  if (typeof call !== 'function') {
    console.warn(`services/tauri: no call named ${name} to note changes for`);
    continue;
  }
  service[name] = async (...args: unknown[]) => {
    const result = await call(...args);
    noteChange(scope);
    return result;
  };
}
