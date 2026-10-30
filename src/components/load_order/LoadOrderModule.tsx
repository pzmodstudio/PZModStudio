import { openRequiredItem } from '../../utils/workshopRequirement';
import { freshProfile } from '../../utils/profiles';
import React, { useState, useRef, useMemo, useEffect } from 'react';
import { ModInfo, HealthIssue, CatalogModSummary, AppInstance, ModErrorStat, profileMembers, STUDIO_LINK_MOD_ID, ModMeta } from '../../types';
import { ModMetaDialog } from './ModMetaDialog';
import { dependentsOf, enableWithRequirements } from '../../utils/planner';
import { sourceOf } from '../common/SourceBadge';
import { MenuItem, useContextMenu } from '../common/ContextMenu';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { findInstalledDependencyInMods, normalizeModId } from './modLookup';
import { ModPresets } from './ModPresets';
import { ModListHistory } from './ModListHistory';
import { ModInspector } from './ModInspector';
import { ModRowBody } from './ModRow';
import { MOD_FILTERS, MOD_SOURCES, ModFilterId, issuesByModOf, modTagsOf, tagLabel } from './modFacets';
import { useKeepActions } from '../common/keep';
import { KeptVersions } from './KeptVersions';
import { LoadRules } from './LoadRules';
import { ShareState, draftRegistry, knownIssues, markShared, proposeUrl, shareStateOf, sharedFacts } from '../../utils/communityIndex';
import { useCommunityIndex } from '../../services/communityIndex';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import {
  AlertTriangle,
  Archive,
  ArrowDownCircle,
  ArrowUpCircle,
  BookOpen,
  Check,
  CheckSquare,
  ChevronsDown,
  ChevronsUp,
  Copy,
  EyeOff,
  FolderOpen,
  Filter,
  FolderX,
  Globe,
  ListOrdered,
  MapPin,
  Package,
  PackagePlus,
  RefreshCw,
  Search,
  ShieldCheck,
  Square,
  ChevronDown,
  ChevronRight,
  StickyNote,
  Tags,
  Wand2,
  X,
  Undo2,
  Redo2,
  UserMinus,
  Trash2,
} from 'lucide-react';
import { tHtml } from '../../i18n/html';

interface LoadOrderModuleProps {
  paths: StudioPathsUI;
  mods: ModInfo[];
  onReorder: (newOrder: ModInfo[]) => void;
  onToggleMod: (modId: string) => void;
  onRefreshMods: () => void;
  onGoToSettings: () => void;
  onLoadMockups: (mockMods: ModInfo[]) => void;
  onApplyPresetLoadOrder?: (presetLoadOrder: string[], activeIds: string[]) => void;
  openedPackageFolder?: string | null;
  /** Issues from the shared health check (useModHealth); null until the first check. */
  healthIssues?: HealthIssue[] | null;
  /** Installed mods as the game sees them (source, origin), from useModHealth. */
  catalogById?: Map<string, CatalogModSummary>;
  /** What each mod did to the last session's log, by the name the game writes. */
  errorStats?: Map<string, ModErrorStat>;
  /** Mod id -> how long its files took to load last session, in ms (the game's DebugLog). */
  loadTimes?: Map<string, number>;
  /** Share of the game's time per mod (%), from the last profiler measurement. */
  gameShares?: Map<string, number>;
  /** The game build installed now, named in a rule proposed to the community. */
  installedBuild?: string | null;
  /** Opens this Workshop item's page in Browse Mods (where the Workshop link lives). */
  onOpenInBrowser?: (workshopId: string) => void;
  /** Searches the Workshop in Explore Mods (a mod that is not installed has no item to open). */
  onSearchInBrowser?: (text: string) => void;
  /** Opens the Mod Merger on this conflicting file (a row chip's fix). */
  onOpenMerger?: (relativePath: string) => void;
  /** Opens Mod Health, for a fix a row cannot apply by itself. */
  onGoToHealth?: (issue?: HealthIssue) => void;
  /** The player's load-order rules changed: the health check reads them again. */
  onRulesChanged?: () => void;
  /** Every saved profile, so the list can show which of them a mod belongs to. */
  profiles?: AppInstance[];
  activeProfileId?: string | null;
  /** After "Add to a profile…": the profiles are read again. */
  onProfilesChanged?: () => void;
  /** Another screen asked to see this mod: it is selected, scrolled to and highlighted. */
  focusModId?: string | null;
  onFocusHandled?: () => void;
}

const PACKAGE_COLOR_PALETTES = [
  { border: 'border-l-cyan-400', bgMatch: 'bg-cyan-950/25 hover:bg-cyan-950/40 border-cyan-900/40', badge: 'bg-cyan-950/90 text-cyan-300 border-cyan-700/60' },
  { border: 'border-l-emerald-400', bgMatch: 'bg-emerald-950/25 hover:bg-emerald-950/40 border-emerald-900/40', badge: 'bg-emerald-950/90 text-emerald-300 border-emerald-700/60' },
  { border: 'border-l-amber-400', bgMatch: 'bg-amber-950/25 hover:bg-amber-950/40 border-amber-900/40', badge: 'bg-amber-950/90 text-amber-300 border-amber-700/60' },
  { border: 'border-l-purple-400', bgMatch: 'bg-purple-950/25 hover:bg-purple-950/40 border-purple-900/40', badge: 'bg-purple-950/90 text-purple-300 border-purple-700/60' },
  { border: 'border-l-pink-400', bgMatch: 'bg-pink-950/25 hover:bg-pink-950/40 border-pink-900/40', badge: 'bg-pink-950/90 text-pink-300 border-pink-700/60' },
  { border: 'border-l-indigo-400', bgMatch: 'bg-indigo-950/25 hover:bg-indigo-950/40 border-indigo-900/40', badge: 'bg-indigo-950/90 text-indigo-300 border-indigo-700/60' },
  { border: 'border-l-rose-400', bgMatch: 'bg-rose-950/25 hover:bg-rose-950/40 border-rose-900/40', badge: 'bg-rose-950/90 text-rose-300 border-rose-700/60' },
  { border: 'border-l-teal-400', bgMatch: 'bg-teal-950/25 hover:bg-teal-950/40 border-teal-900/40', badge: 'bg-teal-950/90 text-teal-300 border-teal-700/60' },
];

/** The questions worth asking of a long mod list. */
/**
 * A row's border colour and background. Every row has the same border widths (1px, 4px on the
 * left), whatever its state, so the switches and names of all rows sit on the same line; states
 * change colours and rings only.
 */
function rowFrame(row: {
  isHighlighted: boolean;
  isSelected: boolean;
  isConflictPartner: boolean;
  isOpenedPackage: boolean;
  worstIssue: 'error' | 'warning' | null;
  packageColor: { border: string; bgMatch: string } | null;
  enabled: boolean;
}): string {
  const border = row.isHighlighted
    ? 'border-cyan-400'
    : row.worstIssue === 'error'
    ? 'border-red-500/80'
    : row.isConflictPartner || row.worstIssue === 'warning' || row.isOpenedPackage
    ? 'border-amber-500/70'
    : row.isSelected
    ? 'border-emerald-500/60'
    : 'border-slate-800';
  const left = row.packageColor ? row.packageColor.border : '';
  const background = row.isHighlighted
    ? 'bg-cyan-950/90 ring-2 ring-cyan-400/60 shadow-lg shadow-cyan-900/50 animate-pulse'
    : row.isSelected
    ? 'bg-slate-800/90 ring-1 ring-emerald-500/40 shadow-lg'
    : row.isConflictPartner
    ? 'bg-amber-950/80 ring-2 ring-amber-400/50 animate-pulse'
    : row.worstIssue === 'error'
    ? 'bg-red-950/40 hover:bg-red-950/55'
    : row.worstIssue === 'warning'
    ? 'bg-amber-950/30 hover:bg-amber-950/45'
    : row.packageColor
    ? row.packageColor.bgMatch.split(' ').filter((c) => !c.startsWith('border-')).join(' ')
    : row.isOpenedPackage
    ? 'bg-amber-950/20 hover:bg-amber-950/30'
    : row.enabled
    ? 'bg-slate-900/90 hover:bg-slate-800/80'
    : 'bg-slate-950/40 opacity-60 hover:opacity-100';
  return `${border} ${left} ${background}`;
}

/** How the list is shown (only the view: the load order stays as it is). */
type SortBy = 'order' | 'name' | 'newest' | 'oldest';
const SORTS: SortBy[] = ['order', 'name', 'newest', 'oldest'];

export const LoadOrderModule: React.FC<LoadOrderModuleProps> = ({
  paths,
  mods,
  onReorder,
  onToggleMod,
  onRefreshMods,
  onGoToSettings,
  onApplyPresetLoadOrder,
  openedPackageFolder,
  healthIssues = null,
  catalogById,
  errorStats,
  loadTimes,
  gameShares,
  installedBuild = null,
  onOpenInBrowser,
  onSearchInBrowser,
  onOpenMerger,
  onGoToHealth,
  onRulesChanged,
  profiles = [],
  activeProfileId = null,
  onProfilesChanged,
  focusModId = null,
  onFocusHandled,
}) => {
  const showContextMenu = useContextMenu();
  const [selectedModId, setSelectedModId] = useState<string | null>(mods.length > 0 ? mods[0].mod_id : null);
  const [highlightedModId, setHighlightedModId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const { t } = useTranslation();
  const changeRev = useChangeRevision();
  const [assigningModId, setAssigningModId] = useState<string | null>(null);
  const [targetPosInput, setTargetPosInput] = useState<string>('');
  const [isRefreshing, setIsRefreshing] = useState<boolean>(false);
  const [saveToast, setSaveToast] = useState<string | null>(null);
  const [showAutoSortNotice, setShowAutoSortNotice] = useState<boolean>(false);
  const [dontShowNoticeChecked, setDontShowNoticeChecked] = useState<boolean>(false);


  const modRowRefs = useRef<Record<string, HTMLDivElement | null>>({});
  /** The mod whose kept versions are open in the dialog. */
  const [keptVersionsOf, setKeptVersionsOf] = useState<ModInfo | null>(null);
  /** The mod whose load-order rules are open in the dialog. */
  const [rulesOf, setRulesOf] = useState<ModInfo | null>(null);

  /**
   * Undo / redo: what the list was (order and switches) before each change made here. Restored
   * onto the mods as they are now, so a mod installed since is kept, at the end.
   */
  type Snap = { id: string; on: boolean }[];
  const history = useRef<Snap[]>([]);
  const future = useRef<Snap[]>([]);
  const [, setHistoryTick] = useState(0);
  const snapOf = (list: ModInfo[]): Snap => list.map((m) => ({ id: m.mod_id, on: m.enabled }));
  const sameSnap = (a: Snap, b: Snap) => a.length === b.length && a.every((x, i) => x.id === b[i].id && x.on === b[i].on);
  const remember = () => {
    const now = snapOf(mods);
    const last = history.current[history.current.length - 1];
    if (!last || !sameSnap(last, now)) {
      history.current.push(now);
      if (history.current.length > 50) history.current.shift();
    }
    future.current = [];
    saveHistory();
    setHistoryTick((n) => n + 1);
  };
  const restore = (snap: Snap) => {
    const byId = new Map(mods.map((m) => [m.mod_id, m]));
    const out = snap.filter((x) => byId.has(x.id)).map((x) => ({ ...byId.get(x.id)!, enabled: x.on }));
    const kept = new Set(snap.map((x) => x.id));
    for (const m of mods) if (!kept.has(m.mod_id)) out.push(m);
    onReorder(out);
  };
  /** One step back (or forward): a step that changed nothing (a toggle cancelled) is skipped. */
  const step = (from: React.MutableRefObject<Snap[]>, to: React.MutableRefObject<Snap[]>) => {
    const now = snapOf(mods);
    let target: Snap | undefined;
    while ((target = from.current.pop()) && sameSnap(target, now)) {
      // the same as now: try the one before
    }
    if (target) {
      to.current.push(now);
      restore(target);
    }
    saveHistory();
    setHistoryTick((n) => n + 1);
  };
  const undo = () => step(history, future);
  const redo = () => step(future, history);
  // Each profile keeps its own history, across restarts (kept in this browser's storage; the
  // previous lists on disk are in "Previous lists").
  const historyKey = `pzms_list_history_${activeProfileId ?? 'none'}`;
  useEffect(() => {
    try {
      const kept = JSON.parse(localStorage.getItem(historyKey) ?? 'null') as { past: Snap[]; future: Snap[] } | null;
      history.current = kept?.past ?? [];
      future.current = kept?.future ?? [];
    } catch {
      history.current = [];
      future.current = [];
    }
    setHistoryTick((n) => n + 1);
  }, [historyKey]);
  const saveHistory = () => {
    try {
      localStorage.setItem(historyKey, JSON.stringify({ past: history.current, future: future.current }));
    } catch {
      // A convenience: without storage it lasts this session.
    }
  };

  const triggerReorder = (newOrder: ModInfo[]) => {
    remember();
    onReorder(newOrder);
  };

  const triggerToggleMod = (modId: string) => {
    remember();
    onToggleMod(modId);
  };

  /** The green note at the top right, gone again after a moment. */
  const showToast = (message: string, ms = 3000) => {
    setSaveToast(message);
    setTimeout(() => setSaveToast(null), ms);
  };

  const handleRefresh = async () => {
    setIsRefreshing(true);
    await onRefreshMods();
    setTimeout(() => setIsRefreshing(false), 500);
  };


  const { multiModPackageMap, workshopColorMap } = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const m of mods) {
      if (m.workshop_id && m.workshop_id !== '9999999999' && !m.mod_id.startsWith('Z_PZModStudio_')) {
        counts[m.workshop_id] = (counts[m.workshop_id] || 0) + 1;
      }
    }


    const multiMap: Record<string, boolean> = {};
    const colorMap: Record<string, typeof PACKAGE_COLOR_PALETTES[0]> = {};
    let colorIdx = 0;

    for (const [wId, count] of Object.entries(counts)) {
      if (count > 1) {
        multiMap[wId] = true;
        colorMap[wId] = PACKAGE_COLOR_PALETTES[colorIdx % PACKAGE_COLOR_PALETTES.length];
        colorIdx++;
      }
    }

    return { multiModPackageMap: multiMap, workshopColorMap: colorMap };
  }, [mods]);

  // Dependency, order and compatibility problems come from the Rust health check (game-exact
  // rules: case-sensitive ids, require/loadModAfter/loadModBefore/incompatible from mod.info).
  // The maps below keep the shapes this screen's badges and buttons already use.
  // Position of each mod in the full list (rows use it instead of scanning the list each time).
  const modIndexById = useMemo(() => new Map(mods.map((m, idx) => [m.mod_id, idx])), [mods]);

  /** The health check's issues by mod, for the row chips; an incompatible pair is on both rows. */
  const issuesByMod = useMemo(() => issuesByModOf(healthIssues), [healthIssues]);

  const {
    activeConflictsMap,
    missingActiveDependenciesMap,
    missingUninstalledDependenciesMap,
    loadOrderViolationsMap,
  } = useMemo(() => {
    const conflicts: Record<string, { conflictingModId: string; conflictingModName: string }[]> = {};
    const disabledDeps: Record<string, ModInfo[]> = {};
    const uninstalledDeps: Record<string, string[]> = {};
    const violations: Record<string, { requiredModId: string; requiredModName: string; requiredModIndex: number }[]> = {};
    const byId = new Map(mods.map((m, idx) => [m.mod_id, { mod: m, idx }]));
    const nameOf = (id: string) => byId.get(id)?.mod.name ?? id;
    const addViolation = (modId: string, requiredId: string) => {
      const list = (violations[modId] ??= []);
      if (!list.some((v) => v.requiredModId === requiredId)) {
        list.push({ requiredModId: requiredId, requiredModName: nameOf(requiredId), requiredModIndex: byId.get(requiredId)?.idx ?? -1 });
      }
    };

    for (const issue of healthIssues ?? []) {
      const modId = issue.mod_id;
      if (!modId) continue;
      switch (issue.code) {
        case 'incompatible':
          for (const other of issue.related) {
            (conflicts[modId] ??= []).push({ conflictingModId: other, conflictingModName: nameOf(other) });
            (conflicts[other] ??= []).push({ conflictingModId: modId, conflictingModName: nameOf(modId) });
          }
          break;
        case 'dependency_not_enabled':
          disabledDeps[modId] = issue.related.map((id) => byId.get(id)?.mod).filter((m): m is ModInfo => !!m);
          break;
        case 'missing_dependency':
          (uninstalledDeps[modId] ??= []).push(...issue.related);
          break;
        case 'dependency_after':
        case 'load_after_order':
          issue.related.forEach((req) => addViolation(modId, req));
          break;
        case 'load_before_order':
          // modId must load before the related mod: the related mod is the one out of place.
          issue.related.forEach((other) => addViolation(other, modId));
          break;
      }
    }
    return {
      activeConflictsMap: conflicts,
      missingActiveDependenciesMap: disabledDeps,
      missingUninstalledDependenciesMap: uninstalledDeps,
      loadOrderViolationsMap: violations,
    };
  }, [healthIssues, mods]);

  const [orderViolationCycleIdx, setOrderViolationCycleIdx] = useState(0);
  const [conflictCycleIdx, setConflictCycleIdx] = useState(0);
  const [missingDepCycleIdx, setMissingDepCycleIdx] = useState(0);
  const [missingUninstalledCycleIdx, setMissingUninstalledCycleIdx] = useState(0);

  const orderViolationModIds = useMemo(() => Object.keys(loadOrderViolationsMap), [loadOrderViolationsMap]);
  const conflictModIds = useMemo(() => Object.keys(activeConflictsMap), [activeConflictsMap]);
  const missingDepModIds = useMemo(() => Object.keys(missingActiveDependenciesMap), [missingActiveDependenciesMap]);
  const missingUninstalledModIds = useMemo(() => Object.keys(missingUninstalledDependenciesMap), [missingUninstalledDependenciesMap]);

  const handleCycleOrderViolations = () => {
    if (orderViolationModIds.length === 0) return;
    const nextIdx = (orderViolationCycleIdx + 1) % orderViolationModIds.length;
    setOrderViolationCycleIdx(nextIdx);
    handleJumpToMod(orderViolationModIds[nextIdx]);
  };

  const handleCycleConflicts = () => {
    if (conflictModIds.length === 0) return;
    const nextIdx = (conflictCycleIdx + 1) % conflictModIds.length;
    setConflictCycleIdx(nextIdx);
    handleJumpToMod(conflictModIds[nextIdx]);
  };

  const handleCycleMissingDeps = () => {
    if (missingDepModIds.length === 0) return;
    const nextIdx = (missingDepCycleIdx + 1) % missingDepModIds.length;
    setMissingDepCycleIdx(nextIdx);
    handleJumpToMod(missingDepModIds[nextIdx]);
  };

  const handleCycleMissingUninstalled = () => {
    if (missingUninstalledModIds.length === 0) return;
    const nextIdx = (missingUninstalledCycleIdx + 1) % missingUninstalledModIds.length;
    setMissingUninstalledCycleIdx(nextIdx);
    handleJumpToMod(missingUninstalledModIds[nextIdx]);
  };

  const handleFixLoadOrderViolation = (modId: string, requiredModId: string) => {
    const currentIdx = mods.findIndex((m) => m.mod_id === modId);
    const reqMod = findInstalledDependencyInMods(requiredModId, mods);
    if (currentIdx === -1 || !reqMod) return;

    const requiredIdx = mods.findIndex((m) => m.mod_id === reqMod.mod_id);
    if (requiredIdx === -1) return;

    // Move requiredModId (library/framework) BEFORE modId so it loads first!
    const newOrder = [...mods];
    const [movedReq] = newOrder.splice(requiredIdx, 1);
    const newCurrentIdx = newOrder.findIndex((m) => m.mod_id === modId);
    newOrder.splice(newCurrentIdx, 0, movedReq);

    triggerReorder(newOrder);
    handleJumpToMod(modId);
  };

  const [sourceFilter, setSourceFilter] = useState<string>('all');
  /** What you are looking for in a list of a hundred mods. See MOD_FILTERS. */
  const [stateFilter, setStateFilter] = useState<ModFilterId>('all');
  /**
   * With a profile active the list is about that profile, so the rest of the library is out of
   * the way until asked for. "Show everything" then labels each mod with the profiles using it,
   * which is how you add one to the profile you are in.
   */
  const [showEveryMod, setShowEveryMod] = useState<boolean>(false);

  const activeProfile = profiles.find((p) => p.id === activeProfileId) ?? null;
  const keep = useKeepActions(paths, profiles, onProfilesChanged, showToast);
  const inActiveProfile = useMemo(
    () => new Set(activeProfile ? profileMembers(activeProfile) : []),
    [activeProfile]
  );
  /**
   * The mods the list is about: with a profile active and "everything" off, that profile's mods
   * (and anything switched on); otherwise every installed mod. The header's numbers and the
   * filters' counts are about these, so an empty profile says it is empty. The Studio Link mod is
   * the Studio's own plumbing, on in every profile, and is left out while a profile is active,
   * as the game's Mods screen leaves it out.
   */
  const scopeMods = useMemo(() => {
    if (!activeProfile) return mods;
    return mods.filter(
      (m) => m.mod_id !== STUDIO_LINK_MOD_ID && (showEveryMod || inActiveProfile.has(m.mod_id) || m.enabled)
    );
  }, [mods, activeProfile, showEveryMod, inActiveProfile]);
  /** The mod in the inspector: the one picked, else the first the list is about (none in an empty profile). */
  const selectedMod = scopeMods.find((m) => m.mod_id === selectedModId) || scopeMods[0] || null;
  /** Workshop items installed in all, whichever profile: what Steam calls your subscribed items. */
  const libraryWorkshopItems = useMemo(
    () => new Set(mods.filter((m) => m.workshop_id && m.workshop_id !== '9999999999' && !m.mod_id.startsWith('Z_PZModStudio_')).map((m) => m.workshop_id)).size,
    [mods]
  );
  const headerStats = useMemo(() => {
    const workshop = new Set<string>();
    let mapCount = 0;
    let libCount = 0;
    let scriptCount = 0;
    let active = 0;
    for (const m of scopeMods) {
      if (m.workshop_id && m.workshop_id !== '9999999999' && !m.mod_id.startsWith('Z_PZModStudio_')) workshop.add(m.workshop_id);
      if (m.enabled) active++;
      if (m.is_map_mod) mapCount++;
      else if (m.is_library) libCount++;
      else scriptCount++;
    }
    return { workshop: workshop.size, total: scopeMods.length, active, mapCount, libCount, scriptCount };
  }, [scopeMods]);
  /** mod id -> the profiles it belongs to, on or off. */
  const profilesByMod = useMemo(() => {
    const map = new Map<string, string[]>();
    for (const p of profiles) {
      for (const id of profileMembers(p)) {
        map.set(id, [...(map.get(id) ?? []), p.name]);
      }
    }
    return map;
  }, [profiles]);
  /**
   * Installed mods no profile holds, on or off: usually subscribed from the Steam website, which
   * the Studio never sees happen. While a profile is active they are out of sight, so the list
   * says they are there and offers to add them (switched off). The game's Mods screen offers the
   * same through the Studio Link mod.
   */
  const orphanMods = useMemo(
    () =>
      activeProfile
        ? mods.filter(
            (m) =>
              !m.not_installed &&
              !m.enabled &&
              m.mod_id !== STUDIO_LINK_MOD_ID &&
              !m.mod_id.startsWith('Z_PZModStudio_') &&
              !profilesByMod.has(m.mod_id)
          )
        : [],
    [mods, activeProfile, profilesByMod]
  );
  const orphanKey = orphanMods.map((m) => m.mod_id).sort().join('|');
  const [dismissedOrphans, setDismissedOrphans] = useState<string>('');
  const addOrphansToProfile = async () => {
    if (!activeProfile || orphanMods.length === 0) return;
    const current = await freshProfile(paths.user_zomboid_dir || '', activeProfile);
    const members = profileMembers(current);
    const added = orphanMods.map((m) => m.mod_id).filter((id) => !members.includes(id));
    try {
      await TauriService.setProfileMembers(paths.user_zomboid_dir || '', activeProfile.id, [...members, ...added]);
      showToast(t('modList.orphans.added', { count: added.length, name: activeProfile.name }));
      onProfilesChanged?.();
      onRefreshMods();
    } catch {
      // Already shown by TauriService.
    }
  };
  /**
   * Whether a mod belongs in each filter. The answers come from what the game and the health
   * check already told us, so a filter never claims more than the Studio knows.
   */
  const matchesFilter = (m: ModInfo, id: ModFilterId): boolean => {
    const catalog = catalogById?.get(m.mod_id);
    switch (id) {
      case 'all':
        return true;
      case 'enabled':
        return m.enabled;
      case 'favourites':
        return keep.isFavorite({ workshop_id: m.workshop_id ?? null, mod_id: m.workshop_id ? null : m.mod_id, name: m.name, installedModIds: [m.mod_id] });
      case 'disabled':
        return !m.enabled;
      case 'problems':
        return Boolean(
          m.not_installed ||
            missingActiveDependenciesMap[m.mod_id] ||
            missingUninstalledDependenciesMap[m.mod_id] ||
            loadOrderViolationsMap[m.mod_id] ||
            activeConflictsMap[m.mod_id]
        );
      case 'maps':
        return Boolean(m.is_map_mod);
      case 'libraries':
        return Boolean(m.is_library);
      case 'unloadable':
        return (
          m.loadable_on_installed_build === false ||
          (catalog ? catalog.loadability.status !== 'loadable' : false)
        );
      case 'updated':
        return Boolean(
          catalog?.workshop_state?.updated_since_last_play || catalog?.workshop_state?.update_pending
        );
    }
  };

  /** How many mods each filter would show, so an empty filter is visibly empty before you click. */
  const filterCounts = useMemo(() => {
    const counts = {} as Record<ModFilterId, number>;
    for (const id of MOD_FILTERS) counts[id] = 0;
    for (const m of scopeMods) {
      for (const id of MOD_FILTERS) if (matchesFilter(m, id)) counts[id] += 1;
    }
    return counts;
  }, [
    scopeMods,
    catalogById,
    missingActiveDependenciesMap,
    missingUninstalledDependenciesMap,
    loadOrderViolationsMap,
    activeConflictsMap,
    keep,
  ]);

  /**
   * The tags each row shows (what the last session said, its loading time, the health check's
   * chips), so the list can be narrowed to one of them. A tag is the same word as the chip.
   */
  const [tagFilter, setTagFilter] = useState<string>('all');
  // The player's notes and groups (kept by the app; the game's Link shows them).
  const [meta, setMeta] = useState<ModMeta>({ notes: {}, groups: [] });
  const [groupFilter, setGroupFilter] = useState<string>('all');
  // Grouped like the game's Mods screen: none, author, category, the player's own groups.
  const [groupBy, setGroupByState] = useState<'none' | 'author' | 'category' | 'mine'>(() => {
    try {
      const v = localStorage.getItem('pzms_group_by');
      return v === 'author' || v === 'category' || v === 'mine' ? v : 'none';
    } catch {
      return 'none';
    }
  });
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  useEffect(() => {
    try {
      setCollapsed(new Set(JSON.parse(localStorage.getItem(`pzms_group_collapsed_${groupBy}`) ?? '[]') as string[]));
    } catch {
      setCollapsed(new Set());
    }
  }, [groupBy]);
  const setGroupBy = (v: 'none' | 'author' | 'category' | 'mine') => {
    setGroupByState(v);
    try { localStorage.setItem('pzms_group_by', v); } catch { /* per-viewer convenience */ }
  };
  const [groupNoteOf, setGroupNoteOf] = useState<{ name: string; note: string } | null>(null);
  const [metaOf, setMetaOf] = useState<{ mod: ModInfo; tab: 'note' | 'groups' } | null>(null);
  useEffect(() => {
    if (!paths.user_zomboid_dir) return;
    // Groups are each profile's own: another profile, its groups.
    void TauriService.modMeta(paths).then(setMeta);
    setGroupFilter('all');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths.user_zomboid_dir, activeProfileId, changeRev]);
  const groupsByMod = useMemo(() => {
    const out = new Map<string, string[]>();
    for (const g of meta.groups) for (const id of g.mods) out.set(id, [...(out.get(id) ?? []), g.name]);
    return out;
  }, [meta]);
  /** Turns every mod of a group on (with what they require) or off. */
  const setGroupOn = (name: string, on: boolean) => {
    const group = meta.groups.find((g) => g.name === name);
    if (!group) return;
    const ids = new Set(group.mods);
    const next = on ? enableWithRequirements(mods, group.mods) : mods.map((m) => (ids.has(m.mod_id) && m.mod_id !== STUDIO_LINK_MOD_ID ? { ...m, enabled: false } : m));
    triggerReorder(next);
    showToast(on ? t('modList.groups.turnedOn', { name }) : t('modList.groups.turnedOff', { name }));
  };
  // What each mod has for the community index, against what is in it and what this player sent.
  const communityIndex = useCommunityIndex(paths);
  const [shared, setShared] = useState(sharedFacts);
  const shareStates = useMemo(() => {
    const map = new Map<string, ShareState>();
    for (const m of mods) {
      const state = shareStateOf(m.mod_id, knownIssues(catalogById?.get(m.mod_id), errorStats?.get(m.name), installedBuild), communityIndex, shared);
      if (state) map.set(m.mod_id, state);
    }
    return map;
  }, [mods, catalogById, errorStats, installedBuild, communityIndex, shared]);
  const modTags = useMemo(() => {
    const tags = modTagsOf(mods, errorStats, loadTimes, catalogById, issuesByMod, gameShares);
    for (const [id, state] of shareStates) tags.set(id, [...(tags.get(id) ?? []), state === 'shared' ? 'shared' : 'share']);
    return tags;
  }, [mods, errorStats, loadTimes, catalogById, issuesByMod, gameShares, shareStates]);
  /** The origins and groups the list's mods have, with how many (as the game's pickers count). */
  const sourceCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const m of scopeMods) {
      if (m.not_installed) continue;
      const kind = sourceOf(m.mod_id, catalogById?.get(m.mod_id)).kind;
      counts.set(kind, (counts.get(kind) ?? 0) + 1);
    }
    return counts;
  }, [scopeMods, catalogById]);
  const groupCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const m of scopeMods) for (const g of groupsByMod.get(m.mod_id) ?? []) counts.set(g, (counts.get(g) ?? 0) + 1);
    return counts;
  }, [scopeMods, groupsByMod]);
  const tagOptions = useMemo(() => {
    const counts = new Map<string, number>();
    for (const m of scopeMods) for (const tag of modTags.get(m.mod_id) ?? []) counts.set(tag, (counts.get(tag) ?? 0) + 1);
    return [...counts.entries()].map(([id, count]) => ({ id, count, label: tagLabel(id) })).sort((a, b) => a.label.localeCompare(b.label));
  }, [modTags, scopeMods, t]);

  const [sortBy, setSortByState] = useState<SortBy>(() => {
    try {
      const saved = localStorage.getItem('pzms_sort_by') as SortBy | null;
      return saved && SORTS.includes(saved) ? saved : 'order';
    } catch {
      return 'order';
    }
  });
  const setSortBy = (v: SortBy) => {
    setSortByState(v);
    try { localStorage.setItem('pzms_sort_by', v); } catch { /* per-viewer convenience */ }
  };
  const filteredMods = useMemo(() => {
    const query = searchQuery.toLowerCase().trim();
    const shown = scopeMods.filter((m) => {
      if (sourceFilter !== 'all' && (m.not_installed || sourceOf(m.mod_id, catalogById?.get(m.mod_id)).kind !== sourceFilter)) return false;
      if (!matchesFilter(m, stateFilter)) return false;
      if (tagFilter !== 'all' && !(modTags.get(m.mod_id) ?? []).includes(tagFilter)) return false;
      if (groupFilter !== 'all' && !(groupsByMod.get(m.mod_id) ?? []).includes(groupFilter)) return false;
      if (!query) return true;
      return (
        m.name.toLowerCase().includes(query) ||
        m.mod_id.toLowerCase().includes(query) ||
        (m.workshop_id && m.workshop_id.toLowerCase().includes(query)) ||
        (meta.notes[m.mod_id] ?? '').toLowerCase().includes(query)
      );
    });
    const byName = (x: ModInfo, y: ModInfo) => x.name.localeCompare(y.name, undefined, { sensitivity: 'base' });
    if (sortBy === 'name') return [...shown].sort(byName);
    if (sortBy === 'newest' || sortBy === 'oldest') {
      // Mods with no date (not installed) go last either way.
      const sign = sortBy === 'newest' ? -1 : 1;
      return [...shown].sort((x, y) => {
        const a = x.installed_at ?? null;
        const b = y.installed_at ?? null;
        if (a === null || b === null) return a === b ? byName(x, y) : a === null ? 1 : -1;
        return a === b ? byName(x, y) : sign * (a - b);
      });
    }
    return shown;
  }, [
    sortBy,
    groupFilter,
    groupsByMod,
    meta,
    scopeMods,
    searchQuery,
    sourceFilter,
    stateFilter,
    tagFilter,
    modTags,
    catalogById,
    activeProfile,
    showEveryMod,
    inActiveProfile,
    filterCounts,
  ]);

  /**
   * Dragging a mod to where it should load. The position is read from the full list, not the
   * filtered one, so a mod dropped between two visible rows lands between those two mods even
   * when the rows in between are hidden.
   */
  const [dragModId, setDragModId] = useState<string | null>(null);
  const [dropTarget, setDropTarget] = useState<{ modId: string; after: boolean } | null>(null);

  const handleDropOnMod = (targetModId: string, after: boolean) => {
    const dragged = dragModId;
    setDragModId(null);
    setDropTarget(null);
    if (!dragged || dragged === targetModId) return;

    const from = mods.findIndex((m) => m.mod_id === dragged);
    if (from === -1) return;
    const next = [...mods];
    const [moved] = next.splice(from, 1);
    const at = next.findIndex((m) => m.mod_id === targetModId);
    if (at === -1) return;
    next.splice(after ? at + 1 : at, 0, moved);
    triggerReorder(next);
    setHighlightedModId(dragged);
    setTimeout(() => setHighlightedModId(null), 1200);
  };

  const moveUp = (index: number) => {
    if (index <= 0) return;
    const newOrder = [...mods];
    const temp = newOrder[index];
    newOrder[index] = newOrder[index - 1];
    newOrder[index - 1] = temp;
    triggerReorder(newOrder);
  };

  const moveDown = (index: number) => {
    if (index >= mods.length - 1) return;
    const newOrder = [...mods];
    const temp = newOrder[index];
    newOrder[index] = newOrder[index + 1];
    newOrder[index + 1] = temp;
    triggerReorder(newOrder);
  };

  const moveToTop = (index: number) => {
    if (index <= 0) return;
    const newOrder = [...mods];
    const item = newOrder.splice(index, 1)[0];
    newOrder.unshift(item);
    triggerReorder(newOrder);
  };

  const moveToBottom = (index: number) => {
    if (index >= mods.length - 1) return;
    const newOrder = [...mods];
    const item = newOrder.splice(index, 1)[0];
    newOrder.push(item);
    triggerReorder(newOrder);
  };

  /** What right-clicking a mod offers: the things people reach for most. */
  /**
   * Puts a mod in the active profile or takes it out (taken out, it is switched off too). The
   * game's Mods screen shows the same set the next time it opens.
   */
  const setInProfile = async (mod: ModInfo, inside: boolean) => {
    if (!activeProfile) return;
    const current = await freshProfile(paths.user_zomboid_dir || '', activeProfile);
    const members = profileMembers(current).filter((id) => id !== mod.mod_id);
    if (inside) members.push(mod.mod_id);
    try {
      await TauriService.setProfileMembers(paths.user_zomboid_dir || '', activeProfile.id, members);
      showToast(inside
        ? t('modList.menu.addedToProfile', { mod: mod.name, name: activeProfile.name })
        : t('modList.menu.removedFromProfile', { mod: mod.name, name: activeProfile.name }));
      onProfilesChanged?.();
      onRefreshMods();
    } catch {
      // Already shown by TauriService.
    }
  };

  const modRowMenu = (mod: ModInfo, index: number): MenuItem[] => [
    {
      label: mod.enabled ? t('modList.menu.switchOff') : t('modList.menu.switchOn'),
      icon: mod.enabled ? EyeOff : Check,
      onSelect: () => triggerToggleMod(mod.mod_id),
    },
    ...(activeProfile
      ? [{
          label: inActiveProfile.has(mod.mod_id)
            ? t('modList.menu.removeFromProfile', { name: activeProfile.name })
            : t('modList.menu.addToProfile', { name: activeProfile.name }),
          icon: inActiveProfile.has(mod.mod_id) ? FolderX : Check,
          onSelect: () => void setInProfile(mod, !inActiveProfile.has(mod.mod_id)),
          disabled: mod.mod_id === STUDIO_LINK_MOD_ID,
          disabledReason: t('modList.menu.linkAlwaysOn'),
        } as MenuItem]
      : []),
    { label: t('modList.menu.toTop'), icon: ChevronsUp, onSelect: () => moveToTop(index), disabled: index === 0,
      disabledReason: t('modList.menu.alreadyFirst') },
    { label: t('modList.menu.toBottom'), icon: ChevronsDown, onSelect: () => moveToBottom(index),
      disabled: index === mods.length - 1, disabledReason: t('modList.menu.alreadyLast') },
    // A mod that is not installed has no Workshop item to open: look for it by name instead.
    !mod.workshop_id && mod.not_installed
      ? {
          label: t('modList.menu.searchBrowser'),
          icon: Search,
          separatorBefore: true,
          onSelect: () => onSearchInBrowser?.(mod.name || mod.mod_id),
          disabled: !onSearchInBrowser,
          disabledReason: t('modList.menu.notWorkshop'),
        }
      : {
          label: t('modList.menu.openBrowser'),
          icon: BookOpen,
          separatorBefore: true,
          onSelect: () => mod.workshop_id && onOpenInBrowser?.(mod.workshop_id),
          disabled: !mod.workshop_id || !onOpenInBrowser,
          disabledReason: t('modList.menu.notWorkshop'),
        },
    {
      label: t('modList.menu.openFolder'),
      icon: FolderOpen,
      onSelect: () => TauriService.openModFolder(paths, catalogById?.get(mod.mod_id)?.root_dir ?? ''),
      disabled: !catalogById?.get(mod.mod_id)?.root_dir,
      disabledReason: t('modList.menu.noFolder'),
    },
    {
      label: meta.notes[mod.mod_id] ? t('modList.menu.editNote') : t('modList.menu.addNote'),
      icon: StickyNote,
      separatorBefore: true,
      onSelect: () => setMetaOf({ mod, tab: 'note' }),
    },
    {
      label: t('modList.menu.groups'),
      icon: Tags,
      onSelect: () => setMetaOf({ mod, tab: 'groups' }),
    },
    {
      label: t('modList.menu.rules'),
      icon: ListOrdered,
      separatorBefore: true,
      onSelect: () => setRulesOf(mod),
    },
    {
      label: t('modList.menu.keepVersion'),
      icon: Archive,
      separatorBefore: true,
      onSelect: () => {
        void TauriService.keepModVersion(paths, mod.mod_id).then((kept) =>
          showToast(t('modList.menu.kept', { name: kept.mod_name, when: kept.kept_at }))
        );
      },
      disabled: !catalogById?.get(mod.mod_id)?.root_dir,
      disabledReason: t('modList.menu.noFolder'),
    },
    {
      label: t('modList.menu.keptVersions'),
      icon: Archive,
      onSelect: () => setKeptVersionsOf(mod),
    },
    ...(mod.mod_id.startsWith('Z_PZModStudio_') ? [] : keep.menuItems({ workshop_id: mod.workshop_id ?? null, mod_id: mod.workshop_id ? null : mod.mod_id, name: mod.name, installedModIds: [mod.mod_id] })),
    {
      label: t('modList.menu.share'),
      icon: Globe,
      onSelect: () => {
        void (async () => {
          const registry = draftRegistry(mod, catalogById?.get(mod.mod_id), installedBuild, knownIssues(catalogById?.get(mod.mod_id), errorStats?.get(mod.name), installedBuild));
          if (await TauriService.githubTokenStatus()) {
            // With a token: the Studio forks, writes the file on a branch and opens the pull request.
            // That is public at once, so the person sees what goes out first.
            if (!confirm(t('modList.menu.prConfirm', { name: mod.name, json: JSON.stringify(registry, null, 2).slice(0, 1500) }))) return;
            showToast(t('modList.menu.openingPr', { name: mod.name }), 20000);
            try {
              const r = await TauriService.proposeRegistry(mod.mod_id, JSON.stringify(registry, null, 2));
              setShared(markShared(mod.mod_id, registry.issues, r.pr_url));
              showToast(t('modList.menu.prOpened', { url: r.pr_url }), 10000);
              TauriService.openExternalUrl(r.pr_url);
            } catch {
              // Already shown by TauriService.
            }
            return;
          }
          // Without one: GitHub's new-file page, with the registry the Studio can fill in already
          // there; the person adds what they tested and GitHub opens the pull request. Nothing is sent from here.
          void navigator.clipboard.writeText(JSON.stringify(registry, null, 2));
          showToast(t('modList.menu.openingGithub', { name: mod.name }), 8000);
          TauriService.openExternalUrl(proposeUrl(registry));
        })();
      },
      // Everything seen is in the index or in a pull request already: another one would say it twice.
      disabled: mod.mod_id.startsWith('Z_PZModStudio_') || knownIssues(catalogById?.get(mod.mod_id), errorStats?.get(mod.name), installedBuild).length === 0 || shareStates.get(mod.mod_id) === 'shared',
      disabledReason: mod.mod_id.startsWith('Z_PZModStudio_')
        ? t('modList.menu.notCommunity')
        : shareStates.get(mod.mod_id) === 'shared'
        ? t('modList.menu.alreadyShared', { when: shared[mod.mod_id]?.at ? ` (${shared[mod.mod_id].at})` : '' })
        : t('modList.menu.nothingSeen'),
      title: t('modList.menu.shareTitle', { facts: knownIssues(catalogById?.get(mod.mod_id), errorStats?.get(mod.name), installedBuild).map((f) => f.title.toLowerCase()).join('; ') }),
    },
    {
      label: t('modList.menu.copyId'),
      icon: Copy,
      separatorBefore: true,
      onSelect: () => navigator.clipboard.writeText(mod.mod_id),
    },
    ...unsubscribeItem(mod),
  ];

  /**
   * "Unsubscribe" for one mod: the Workshop item it came from, with the other mods that item
   * holds named (they go with it). Not for local folders or the Studio's own packages.
   */
  const unsubscribeItem = (mod: ModInfo): MenuItem[] => {
    if (mod.mod_id.startsWith('Z_PZModStudio_')) return [];
    const item = mod.workshop_id && mod.workshop_id !== '9999999999' && mod.workshop_id !== '0' ? mod.workshop_id : null;
    const siblings = item ? mods.filter((m) => m.workshop_id === item && m.mod_id !== mod.mod_id) : [];
    return [
      {
        label: t('modList.menu.unsubscribe'),
        icon: Trash2,
        danger: true,
        separatorBefore: true,
        disabled: !item,
        disabledReason: t('modList.menu.notWorkshop'),
        title: siblings.length > 0
          ? t('modList.menu.unsubscribeWithTitle', { count: siblings.length, mods: siblings.map((m) => m.name).join(', ') })
          : t('modList.menu.unsubscribeTitle'),
        onSelect: () => item && void unsubscribe(mods.filter((m) => m.workshop_id === item), [item]),
      },
    ];
  };

  /** Its place among the mods that are on (1-based): the number the game shows. */
  const activeRanks = useMemo(() => {
    const out = new Map<string, number>();
    let n = 0;
    for (const m of mods) if (m.enabled) out.set(m.mod_id, ++n);
    return out;
  }, [mods]);

  /** "Pos #": the mod goes to that place among the mods that are on (the game's number). */
  const handleAssignPosition = (modId: string, targetPos: number) => {
    const currentIndex = mods.findIndex((m) => m.mod_id === modId);
    if (currentIndex === -1) return;
    if (activeRanks.get(modId) === targetPos) {
      setAssigningModId(null);
      return;
    }

    const newOrder = [...mods];
    const [movedMod] = newOrder.splice(currentIndex, 1);
    const onIndexes = newOrder.map((m, i) => (m.enabled ? i : -1)).filter((i) => i >= 0);
    const n = Math.max(1, targetPos);
    // Before the mod that is n-th on now, or after the last one that is on.
    const targetIndex = n <= onIndexes.length ? onIndexes[n - 1] : (onIndexes.length > 0 ? onIndexes[onIndexes.length - 1] + 1 : 0);
    newOrder.splice(targetIndex, 0, movedMod);

    triggerReorder(newOrder);
    setAssigningModId(null);
    setTargetPosInput('');
  };

  const handleToggleSingleMod = (modId: string) => {
    if (openedPackageFolder && modId === openedPackageFolder) {
      return;
    }

    const targetMod = mods.find((m) => m.mod_id === modId);
    if (!targetMod) return;

    const targetWillBeEnabled = !targetMod.enabled;

    if (targetWillBeEnabled && targetMod.workshop_id) {
      const updated = mods.map((m) => {
        if (m.mod_id === modId) {
          return { ...m, enabled: true };
        }

        // Variants in the same Workshop item that declare each other incompatible (exact ids,
        // backslashes removed, like the game) cannot run together: switch the other one off.
        if (m.enabled && m.workshop_id === targetMod.workshop_id && m.mod_id !== modId) {
          const clean = (ids?: string[]) => (ids ?? []).map((i) => i.replace(/\\/g, '').trim());
          if (clean(targetMod.incompatible).includes(m.mod_id) || clean(m.incompatible).includes(targetMod.mod_id)) {
            return { ...m, enabled: false };
          }
        }
        return m;
      });
      triggerReorder(updated);
    } else {
      triggerToggleMod(modId);
    }
  };

  /**
   * Several mods at once: Ctrl+click adds or takes one, Shift+click takes the rows from the last
   * one clicked; right-click on them acts on all. Esc lets them go.
   */
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const anchor = useRef<string | null>(null);
  const pickRow = (e: React.MouseEvent, id: string) => {
    if (e.ctrlKey || e.metaKey) {
      setPicked((prev) => {
        const next = new Set(prev.size > 0 ? prev : selectedModId ? [selectedModId] : []);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      });
      anchor.current = id;
    } else if (e.shiftKey && anchor.current) {
      const ids = filteredMods.map((m) => m.mod_id);
      const a = ids.indexOf(anchor.current);
      const b = ids.indexOf(id);
      if (a >= 0 && b >= 0) setPicked(new Set(ids.slice(Math.min(a, b), Math.max(a, b) + 1)));
    } else {
      setPicked(new Set());
      anchor.current = id;
    }
    setSelectedModId(id);
  };
  useEffect(() => setPicked(new Set()), [activeProfileId]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLElement && (['INPUT', 'TEXTAREA', 'SELECT'].includes(e.target.tagName) || e.target.isContentEditable);
      if (typing) return;
      if (e.key === 'Escape') setPicked(new Set());
      const ctrl = e.ctrlKey || e.metaKey;
      if (ctrl && !e.shiftKey && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        undo();
      } else if (ctrl && (e.key.toLowerCase() === 'y' || (e.shiftKey && e.key.toLowerCase() === 'z'))) {
        e.preventDefault();
        redo();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  /** Right-click on several picked mods: what can be done to all of them. */
  const pickedMenu = (list: ModInfo[]): MenuItem[] => {
    const ids = new Set(list.map((m) => m.mod_id));
    const off = list.filter((m) => !m.enabled);
    const on = list.filter((m) => m.enabled && !(activeProfile && m.mod_id === STUDIO_LINK_MOD_ID));
    const items = [...new Set(list.map((m) => m.workshop_id).filter((w): w is string => !!w && w !== '9999999999' && w !== '0'))];
    const out: MenuItem[] = [
      {
        label: t('modList.picked.switchOn', { count: off.length }),
        icon: Check,
        disabled: off.length === 0,
        disabledReason: t('modList.picked.allOn'),
        onSelect: () => triggerReorder(enableWithRequirements(mods, off.map((m) => m.mod_id))),
      },
      {
        label: t('modList.picked.switchOff', { count: on.length }),
        icon: EyeOff,
        disabled: on.length === 0,
        disabledReason: t('modList.picked.allOff'),
        onSelect: () => triggerReorder(mods.map((m) => (ids.has(m.mod_id) && !(activeProfile && m.mod_id === STUDIO_LINK_MOD_ID) ? { ...m, enabled: false } : m))),
      },
    ];
    if (activeProfile) {
      out.push({
        label: t('modList.picked.removeFromProfile', { count: list.length, name: activeProfile.name }),
        icon: UserMinus,
        separatorBefore: true,
        onSelect: () => void takeOutOfProfile(list),
      });
    }
    out.push({
      label: t('modList.picked.unsubscribe', { count: items.length }),
      icon: Trash2,
      danger: true,
      separatorBefore: !activeProfile,
      disabled: items.length === 0,
      disabledReason: t('modList.picked.noneFromWorkshop'),
      onSelect: () => void unsubscribe(list, items),
    });
    out.push({ label: t('modList.picked.clear'), icon: X, separatorBefore: true, onSelect: () => setPicked(new Set()) });
    return out;
  };
  const takeOutOfProfile = async (list: ModInfo[]) => {
    if (!activeProfile) return;
    const leaving = new Set(list.map((m) => m.mod_id).filter((id) => id !== STUDIO_LINK_MOD_ID));
    const current = await freshProfile(paths.user_zomboid_dir || '', activeProfile);
    try {
      await TauriService.setProfileMembers(paths.user_zomboid_dir || '', activeProfile.id, profileMembers(current).filter((id) => !leaving.has(id)));
      showToast(t('modList.picked.removed', { count: leaving.size, name: activeProfile.name }));
      setPicked(new Set());
      onProfilesChanged?.();
      onRefreshMods();
    } catch {
      // Already shown by TauriService.
    }
  };
  /** Unsubscribes from the Workshop items of these mods (a Workshop item can hold several). */
  const unsubscribe = async (list: ModInfo[], items: string[]) => {
    const names = list.filter((m) => m.workshop_id && items.includes(m.workshop_id)).map((m) => `- ${m.name}`);
    const shown = names.slice(0, 15).join('\n') + (names.length > 15 ? `\n${t('modList.picked.andMore', { count: names.length - 15 })}` : '');
    // What else breaks: the installed mods whose `require=` names one of the mods going.
    const needers = dependentsOf(mods, list.filter((m) => m.workshop_id && items.includes(m.workshop_id)).map((m) => m.mod_id));
    const on = needers.filter((d) => d.enabled).length;
    const needed = needers.length === 0 ? '' : t('modList.picked.unsubscribeNeededBy', {
      mods: needers.slice(0, 15).map((d) => `- ${d.name}${d.enabled ? t('modList.picked.unsubscribeNeederOn') : ''} (${d.because})`).join('\n')
        + (needers.length > 15 ? `\n${t('modList.picked.andMore', { count: needers.length - 15 })}` : ''),
    }) + (on > 0 ? t('modList.picked.unsubscribeNeededByOn', { count: on }) : '');
    if (!confirm(t('modList.picked.unsubscribeConfirm', { count: items.length, mods: shown }) + needed)) return;
    try {
      const outcome = await TauriService.workshopSubscribe(paths, items, false);
      showToast(
        outcome.failed.length > 0
          ? t('modList.picked.unsubscribedPartly', { done: outcome.done.length, count: items.length, reason: outcome.failed[0][1] })
          : t('modList.picked.unsubscribed', { count: outcome.done.length }),
        6000
      );
      setPicked(new Set());
      onRefreshMods();
    } catch (err) {
      showToast(t('modList.picked.unsubscribeFailed', { reason: String(err) }), 6000);
    }
  };

  const handleEnableAllWithoutDiscrimination = (enable: boolean) => {
    // Only the rows on screen: with a profile, a filter or a search, "every mod" is what is shown,
    // not the whole library (switched on, a mod joins the profile for good).
    const shown = new Set(filteredMods.map((m) => m.mod_id));
    const changing = mods.filter((m) => shown.has(m.mod_id) && m.enabled !== enable).length;
    if (changing === 0) return;
    if (!confirm(t(enable ? 'modList.enableAllConfirm' : 'modList.disableAllConfirm', { count: changing }))) return;
    const isPackage = (id: string) => id.startsWith('Z_PZModStudio_') && !id.includes('Carrier') && id !== 'Z_PZModStudio_Bridge';
    const updated = mods.map((m) => {
      if (!shown.has(m.mod_id)) return m;
      if (!enable) {
        // With a profile active the Studio Link mod stays on: it is how the game shows the profile.
        return activeProfile && m.mod_id === STUDIO_LINK_MOD_ID ? m : { ...m, enabled: false };
      }
      // Packages keep their own rules: a draft never goes on, and only one goes on at a time.
      if (isPackage(m.mod_id)) return m;
      return { ...m, enabled: true };
    });
    triggerReorder(updated);
  };

  // Game-exact sort in Rust: moves only what require / loadModAfter / loadModBefore force and keeps
  // the user's order everywhere else (Studio packages last). The previous order can be restored.
  const [orderBeforeSort, setOrderBeforeSort] = useState<ModInfo[] | null>(null);
  const [lastSortMoved, setLastSortMoved] = useState<number>(0);
  const handleAutoSortDependencies = async () => {
    let result;
    try {
      result = await TauriService.sortLoadOrder(paths, mods.map((m) => m.mod_id));
    } catch {
      return; // already shown by TauriService
    }
    const byId = new Map(mods.map((m) => [m.mod_id, m]));
    const sorted = result.order.map((id) => byId.get(id)).filter((m): m is ModInfo => !!m);
    if (sorted.length !== mods.length) return; // list changed meanwhile; do nothing
    setLastSortMoved(result.moved.length);
    if (result.moved.length > 0) {
      setOrderBeforeSort(mods);
      triggerReorder(sorted);
    }
    const isSilenced = localStorage.getItem('pz_hide_autosort_notice') === 'true';
    if (!isSilenced) {
      setShowAutoSortNotice(true);
    }
  };

  const handleUndoSort = () => {
    if (orderBeforeSort) {
      triggerReorder(orderBeforeSort);
      setOrderBeforeSort(null);
    }
  };

  const findInstalledDependency = (reqRaw: string): ModInfo | undefined => {
    return findInstalledDependencyInMods(reqRaw, mods);
  };

  /**
   * Helper to jump to any mod in the list: clears search query, selects it, highlights row with cyan glow, and scrolls smoothly to it!
   */
  const handleJumpToMod = (targetModId: string) => {
    const targetMod = mods.find((m) => m.mod_id === targetModId || normalizeModId(m.mod_id) === normalizeModId(targetModId));
    const finalId = targetMod ? targetMod.mod_id : targetModId;

    setSearchQuery(''); // Clear search query so target mod is guaranteed visible in DOM
    setSelectedModId(finalId);
    setHighlightedModId(finalId);

    setTimeout(() => {
      const targetElem = modRowRefs.current[finalId];
      if (targetElem) {
        targetElem.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    }, 60);

    setTimeout(() => {
      setHighlightedModId(null);
    }, 3000);
  };

  // A mod another screen sent here (Maps): every filter off so its row is there, then the jump.
  useEffect(() => {
    if (!focusModId || mods.length === 0) return;
    setStateFilter('all');
    handleJumpToMod(focusModId);
    onFocusHandled?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusModId, mods.length]);

  const handleJumpToDependency = (depModIdRaw: string) => {
    const targetMod = findInstalledDependency(depModIdRaw);

    if (!targetMod) {
      // The item that provides it, from the selected mod's Required items; a search otherwise.
      void openRequiredItem(
        selectedMod?.mod_id,
        normalizeModId(depModIdRaw),
        mods,
        (id) => (onOpenInBrowser ? onOpenInBrowser(id) : TauriService.openExternalUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${id}`)),
        (text) => (onSearchInBrowser ? onSearchInBrowser(text) : TauriService.openExternalUrl(`https://steamcommunity.com/workshop/browse/?appid=108600&searchtext=${encodeURIComponent(text)}`))
      );
      return;
    }

    handleJumpToMod(targetMod.mod_id);
  };

  // State 1: Invalid paths guard
  if (!paths.is_valid || !paths.mod_list_ini_path) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-6 bg-slate-950 text-slate-200 font-sans">
        <div className="max-w-md w-full bg-slate-900/80 border border-amber-500/40 rounded-2xl p-6 text-center space-y-4 shadow-xl">
          <div className="w-12 h-12 rounded-xl bg-amber-500/10 text-amber-400 border border-amber-500/30 flex items-center justify-center mx-auto">
            <FolderX className="w-6 h-6" />
          </div>
          <div>
            <h3 className="text-base font-bold text-slate-100">{t('modList.empty.noPathsTitle')}</h3>
            <p className="text-xs text-slate-400 mt-1 leading-relaxed">{t('modList.empty.noPathsBody')}</p>
          </div>
          <button
            onClick={onGoToSettings}
            className="w-full flex items-center justify-center gap-2 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white font-medium text-xs py-2.5 rounded-lg shadow transition cursor-pointer"
          >
            {t('modList.empty.noPathsButton')}
          </button>
        </div>
      </div>
    );
  }

  // State 2: Valid paths, but 0 active mods in ModListData.ini
  if (mods.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-6 bg-slate-950 text-slate-200 font-sans">
        <div className="max-w-lg w-full bg-slate-900/80 border border-emerald-500/30 rounded-2xl p-8 text-center space-y-5 shadow-xl font-sans">
          <div className="w-16 h-16 rounded-2xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 flex items-center justify-center mx-auto shadow-inner">
            <ShieldCheck className="w-8 h-8" />
          </div>
          <div>
            <h3 className="text-lg font-bold text-slate-100">{t('modList.empty.noModsTitle')}</h3>
            <p className="text-xs text-slate-400 mt-1.5 leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('modList.empty.noModsBody') }} />
          </div>

          <div className="pt-2 flex justify-center">
            <button
              onClick={handleRefresh}
              className="flex items-center justify-center gap-2 bg-emerald-950 hover:bg-emerald-900 text-emerald-300 text-xs font-medium px-5 py-2.5 rounded-lg border border-emerald-800 transition cursor-pointer shadow"
            >
              <RefreshCw className={`w-3.5 h-3.5 text-emerald-400 ${isRefreshing ? 'animate-spin' : ''}`} />
              {t('modList.empty.refresh')}
            </button>
          </div>
        </div>
      </div>
    );
  }

  const anyFilter = stateFilter !== 'all' || sourceFilter !== 'all' || tagFilter !== 'all' || groupFilter !== 'all' || searchQuery !== '';
  const clearFilters = () => {
    setStateFilter('all');
    setSourceFilter('all');
    setTagFilter('all');
    setGroupFilter('all');
    setSearchQuery('');
  };
  /** One of the problem counters of the toolbar: click to walk through those mods. */
  const cycleButton = (count: number, index: number, onClick: () => void, label: string, title: string, tone: 'red' | 'amber') =>
    count > 0 && (
      <button
        onClick={onClick}
        className={`flex items-center gap-1 px-1.5 py-0.5 rounded-md border text-[10.5px] font-bold transition cursor-pointer ${
          tone === 'red'
            ? 'text-red-300 bg-red-500/10 hover:bg-red-500/20 border-red-500/40'
            : 'text-amber-300 bg-amber-500/10 hover:bg-amber-500/20 border-amber-500/40'
        }`}
        title={title}
      >
        <AlertTriangle className="w-3 h-3 shrink-0" />
        <span>{label}</span>
        <span className="font-mono text-[10px] opacity-80">
          {index + 1}/{count}
        </span>
      </button>
    );
  const selectClass = (active: boolean) =>
    `bg-slate-950 border rounded-md px-2 py-1 text-xs cursor-pointer outline-none focus:border-cyan-500/80 ${
      active ? 'border-cyan-600/70 text-cyan-200' : 'border-slate-800 text-slate-300'
    }`;

  /** One row of the list (the same in the flat list and inside a group). */
  const renderRow = (mod: ModInfo) => {
                // The row's colour follows the worst thing the health check found for it.
                const rowIssues = issuesByMod.get(mod.mod_id) ?? [];
                const worstIssue = rowIssues.some((i) => i.severity === 'error') ? 'error' : rowIssues.some((i) => i.severity === 'warning') ? 'warning' : null;
                const originalIndex = modIndexById.get(mod.mod_id) ?? -1;
                const isSelected = mod.mod_id === selectedModId;
                const isHighlighted = mod.mod_id === highlightedModId;
                const isAssigningThis = assigningModId === mod.mod_id;

                const isMultiPackage = (!mod.mod_id.startsWith("Z_PZModStudio_") && mod.workshop_id) ? multiModPackageMap[mod.workshop_id] : false;
                const packageColor = (!mod.mod_id.startsWith("Z_PZModStudio_") && mod.workshop_id) ? workshopColorMap[mod.workshop_id] : null;


                const isConflictPartner =
                  selectedMod &&
                  activeConflictsMap[selectedMod.mod_id] &&
                  activeConflictsMap[selectedMod.mod_id].some((c) => c.conflictingModId === mod.mod_id);

                const isSyntheticFusion = mod.mod_id.startsWith("Z_PZModStudio_") && !mod.mod_id.includes("Carrier");
                const isBridge = mod.mod_id === "Z_PZModStudio_Bridge";
                const isOpenedPackage = Boolean(isSyntheticFusion && !isBridge && mod.is_packaged === false);
                const isClosedPackage = Boolean(isSyntheticFusion && !isBridge && mod.is_packaged === true);

                const isDragging = dragModId === mod.mod_id;
                const dropEdge = dropTarget?.modId === mod.mod_id ? dropTarget.after : null;

                return (
                  <div
                    key={mod.mod_id}
                    ref={(el) => {
                      modRowRefs.current[mod.mod_id] = el;
                    }}
                    draggable={!isAssigningThis}
                    onDragStart={(e) => {
                      setDragModId(mod.mod_id);
                      e.dataTransfer.effectAllowed = 'move';
                      // Firefox refuses to start a drag without data on it.
                      e.dataTransfer.setData('text/plain', mod.mod_id);
                    }}
                    onDragEnd={() => {
                      setDragModId(null);
                      setDropTarget(null);
                    }}
                    onDragOver={(e) => {
                      if (!dragModId || dragModId === mod.mod_id) return;
                      e.preventDefault();
                      e.dataTransfer.dropEffect = 'move';
                      const box = e.currentTarget.getBoundingClientRect();
                      const after = e.clientY > box.top + box.height / 2;
                      if (dropTarget?.modId !== mod.mod_id || dropTarget.after !== after) {
                        setDropTarget({ modId: mod.mod_id, after });
                      }
                    }}
                    onDrop={(e) => {
                      e.preventDefault();
                      const box = e.currentTarget.getBoundingClientRect();
                      handleDropOnMod(mod.mod_id, e.clientY > box.top + box.height / 2);
                    }}
                    onMouseDown={(e) => {
                      // Shift+click picks rows, not text.
                      if (e.shiftKey) e.preventDefault();
                    }}
                    onClick={(e) => pickRow(e, mod.mod_id)}
                    onContextMenu={(e) => {
                      if (picked.size > 1 && picked.has(mod.mod_id)) {
                        showContextMenu(e, pickedMenu(mods.filter((m) => picked.has(m.mod_id))));
                        return;
                      }
                      setPicked(new Set());
                      setSelectedModId(mod.mod_id);
                      showContextMenu(e, modRowMenu(mod, originalIndex));
                    }}
                    className={`flex gap-2 px-2 py-1.5 items-center rounded-lg text-xs cursor-pointer transition border border-l-4 ${picked.has(mod.mod_id) ? 'ring-1 ring-emerald-400/70 bg-emerald-950/40' : ''} ${rowFrame({
                      isHighlighted,
                      isSelected: isSelected || picked.has(mod.mod_id),
                      isConflictPartner: Boolean(isConflictPartner),
                      isOpenedPackage,
                      worstIssue,
                      packageColor: isMultiPackage ? packageColor : null,
                      enabled: mod.enabled,
                    })} ${isDragging ? 'opacity-40' : ''} ${
                      dropEdge === false
                        ? 'shadow-[inset_0_2px_0_0_#34d399]'
                        : dropEdge === true
                        ? 'shadow-[inset_0_-2px_0_0_#34d399]'
                        : ''
                    }`}
                  >
                    <ModRowBody
                      paths={paths}
                      mod={mod}
                      originalIndex={originalIndex}
                      showInstalled={sortBy === 'newest' || sortBy === 'oldest'}
                      activeRank={activeRanks.get(mod.mod_id)}
                      mods={mods}
                      isOpenedPackage={isOpenedPackage}
                      isClosedPackage={isClosedPackage}
                      isBridge={isBridge}
                      catalogById={catalogById}
                      errorStats={errorStats}
                      shareState={shareStates.get(mod.mod_id)}
                      sharedAt={shared[mod.mod_id]?.at}
                      loadTimes={loadTimes}
                      gameShares={gameShares}
                      isFavorite={keep.isFavorite({ workshop_id: mod.workshop_id ?? null, mod_id: mod.workshop_id ? null : mod.mod_id, name: mod.name, installedModIds: [mod.mod_id] })}
                      note={meta.notes[mod.mod_id]}
                      groups={groupsByMod.get(mod.mod_id)}
                      showEveryMod={showEveryMod}
                      activeProfile={activeProfile}
                      profilesByMod={profilesByMod}
                      inActiveProfile={inActiveProfile}
                      rowIssues={rowIssues}
                      isAssigningThis={isAssigningThis}
                      targetPosInput={targetPosInput}
                      setTargetPosInput={setTargetPosInput}
                      setAssigningModId={setAssigningModId}
                      handleToggleSingleMod={handleToggleSingleMod}
                      showToast={showToast}
                      triggerReorder={triggerReorder}
                      handleJumpToMod={handleJumpToMod}
                      onOpenMerger={onOpenMerger}
                      onGoToHealth={onGoToHealth}
                      onOpenInBrowser={onOpenInBrowser}
                      onSearchInBrowser={onSearchInBrowser}
                      moveToTop={moveToTop}
                      moveUp={moveUp}
                      moveDown={moveDown}
                      moveToBottom={moveToBottom}
                      handleAssignPosition={handleAssignPosition}
                    />
                  </div>
                );
  };

  /** The list in groups (as in the game's Mods screen): each heading folds its group. */
  const groupKeys = (mod: ModInfo): string[] => {
    if (groupBy === 'author') return [mod.author?.trim() || t('modList.groupBy.noAuthor')];
    if (groupBy === 'category') {
      const c = mod.category || (mod.is_map_mod ? 'map' : '');
      return [c ? t(`modList.groupBy.category_${c}`, { defaultValue: c }) : t('modList.groupBy.noCategory')];
    }
    const mine = groupsByMod.get(mod.mod_id) ?? [];
    return mine.length > 0 ? mine : [t('modList.groupBy.noGroup')];
  };
  const toggleCollapsed = (key: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      try { localStorage.setItem(`pzms_group_collapsed_${groupBy}`, JSON.stringify([...next])); } catch { /* per-viewer convenience */ }
      return next;
    });
  const setManyOn = (list: ModInfo[], on: boolean) => {
    const ids = new Set(list.map((m) => m.mod_id));
    const next = on ? enableWithRequirements(mods, [...ids]) : mods.map((m) => (ids.has(m.mod_id) && m.mod_id !== STUDIO_LINK_MOD_ID ? { ...m, enabled: false } : m));
    triggerReorder(next);
  };
  const renderGrouped = () => {
    const groups = new Map<string, ModInfo[]>();
    for (const mod of filteredMods) for (const key of groupKeys(mod)) groups.set(key, [...(groups.get(key) ?? []), mod]);
    const last = new Set([t('modList.groupBy.noAuthor'), t('modList.groupBy.noCategory'), t('modList.groupBy.noGroup')]);
    const keys = [...groups.keys()].sort((a, b) => (last.has(a) === last.has(b) ? a.localeCompare(b, undefined, { sensitivity: 'base' }) : last.has(a) ? 1 : -1));
    return keys.map((key) => {
      const list = groups.get(key) ?? [];
      const open = !collapsed.has(key);
      const onCount = list.filter((m) => m.enabled).length;
      const mine = groupBy === 'mine' ? meta.groups.find((g) => g.name === key) : undefined;
      return (
        <div key={key} className="space-y-1">
          <div className="flex items-center gap-2 px-2 py-1.5 rounded-lg bg-slate-950/80 border border-slate-800 sticky top-0 z-10">
            <button onClick={() => toggleCollapsed(key)} aria-expanded={open} className="flex items-center gap-1.5 min-w-0 flex-1 text-left cursor-pointer">
              {open ? <ChevronDown className="w-3.5 h-3.5 text-slate-400 shrink-0" /> : <ChevronRight className="w-3.5 h-3.5 text-slate-400 shrink-0" />}
              <span className="text-xs font-semibold text-emerald-300 truncate">{key}</span>
              <span className="text-[10px] text-slate-500 shrink-0">{t('modList.groupBy.count', { on: onCount, count: list.length })}</span>
              {mine?.note && <span className="text-[10px] text-sky-300 truncate" title={mine.note}>· {mine.note}</span>}
            </button>
            {mine && (
              <button onClick={() => setGroupNoteOf({ name: mine.name, note: mine.note ?? '' })} title={t('modList.groupBy.editNote')} className="p-1 rounded text-slate-400 hover:text-sky-300 hover:bg-slate-800 cursor-pointer">
                <StickyNote className="w-3.5 h-3.5" />
              </button>
            )}
            <button onClick={() => setManyOn(list, true)} className="text-[10px] px-1.5 py-0.5 rounded border border-emerald-800/70 text-emerald-300 hover:bg-emerald-950/50 cursor-pointer">{t('modList.groupBy.allOn')}</button>
            <button onClick={() => setManyOn(list, false)} className="text-[10px] px-1.5 py-0.5 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer">{t('modList.groupBy.allOff')}</button>
          </div>
          {open && list.map(renderRow)}
        </div>
      );
    });
  };

  return (
    <div className="flex-1 flex flex-col bg-slate-950 text-slate-100 overflow-hidden p-6 font-sans">
      {/* Module Header: what the list is and its numbers on the left, what you do with the whole list on the right. */}
      <div className="flex items-start justify-between gap-4 mb-3">
        <div className="min-w-0">
          <h2 className="text-lg font-bold text-slate-100 flex items-center gap-2 min-w-0">
            <ListOrdered className="w-5 h-5 text-emerald-400 shrink-0" />
            <span className="truncate">{t('modList.title')}</span>
            <code className="text-[11px] font-mono font-normal text-slate-500 truncate">Zomboid/mods/default.txt</code>
          </h2>
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 mt-1 text-[11px] text-slate-400">
            {headerStats.workshop !== libraryWorkshopItems ? (
              <span title={t('modList.workshopInProfileTitle', { shown: headerStats.workshop, total: libraryWorkshopItems, name: activeProfile?.name ?? '' })}>
                <b className="text-cyan-400">{headerStats.workshop}</b> {t('modList.statWorkshopOf', { total: libraryWorkshopItems })}
              </span>
            ) : (
              <span title={t('modList.subscriptions')}>
                <b className="text-cyan-400">{headerStats.workshop}</b> {t('modList.statWorkshop')}
              </span>
            )}
            <span className="text-slate-700">|</span>
            <span>
              <b className="text-slate-200">{headerStats.total}</b> {t('modList.statMods')}
            </span>
            <span className="text-slate-700">|</span>
            <span>
              <b className="text-emerald-400">{headerStats.active}</b> {t('modList.statActive')}
            </span>
            <span className="text-slate-700">|</span>
            <span className="flex items-center gap-1" title={t('modList.mapTitle')}>
              <MapPin className="w-3 h-3 text-amber-400" /> <b className="text-amber-400">{headerStats.mapCount}</b> {t('modList.map')}
            </span>
            <span className="flex items-center gap-1" title={t('modList.libraryTitle')}>
              <Package className="w-3 h-3 text-purple-400" /> <b className="text-purple-400">{headerStats.libCount}</b> {t('modList.library')}
            </span>
            <span className="flex items-center gap-1" title={t('modList.scriptTitle')}>
              <ListOrdered className="w-3 h-3 text-cyan-400" /> <b className="text-cyan-400">{headerStats.scriptCount}</b> {t('modList.script')}
            </span>
            {cycleButton(missingUninstalledModIds.length, missingUninstalledCycleIdx, handleCycleMissingUninstalled, t('modList.missing'), t('modList.cycleMissing'), 'red')}
            {cycleButton(missingDepModIds.length, missingDepCycleIdx, handleCycleMissingDeps, t('modList.depsOff'), t('modList.cycleOff'), 'red')}
            {cycleButton(conflictModIds.length, conflictCycleIdx, handleCycleConflicts, t('modList.incompatibility'), t('modList.cycleIncompatible'), 'amber')}
            {cycleButton(orderViolationModIds.length, orderViolationCycleIdx, handleCycleOrderViolations, t('modList.listOrder'), t('modList.cycleOrder'), 'amber')}
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <ModPresets
            paths={paths}
            mods={mods}
            onApplyPresetLoadOrder={onApplyPresetLoadOrder}
            onReorder={triggerReorder}
            onOpenInBrowser={onOpenInBrowser}
            toast={showToast}
          />
          <ModListHistory paths={paths} mods={mods} onRefreshMods={onRefreshMods} toast={showToast} />
          <button
            onClick={handleRefresh}
            disabled={isRefreshing}
            className="p-2 bg-slate-900 hover:bg-slate-800 text-slate-200 border border-slate-800 rounded-lg transition cursor-pointer shrink-0"
            title={`${t('modList.refresh')}: ${t('modList.refreshTitle')}`}
            aria-label={t('modList.refresh')}
          >
            <RefreshCw className={`w-3.5 h-3.5 text-cyan-400 ${isRefreshing ? 'animate-spin' : ''}`} />
          </button>
          <button
            onClick={handleAutoSortDependencies}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white text-xs font-bold rounded-lg shadow transition cursor-pointer"
          >
            <Wand2 className="w-4 h-4" />
            {t('modList.autoSort')}
          </button>
          <KeptVersions paths={paths} mod={keptVersionsOf} onClose={() => setKeptVersionsOf(null)} onModsChanged={onRefreshMods} toast={showToast} />
          {keep.node}
          <LoadRules paths={paths} mod={rulesOf} mods={mods} installedBuild={installedBuild} onClose={() => setRulesOf(null)} onRulesChanged={() => onRulesChanged?.()} toast={showToast} />
          <ModMetaDialog paths={paths} open={metaOf} meta={meta} onChange={setMeta} onClose={() => setMetaOf(null)} />
          {groupNoteOf && (
            <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-6" onClick={() => setGroupNoteOf(null)}>
              <div className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl w-full max-w-md p-5 space-y-3" onClick={(e) => e.stopPropagation()}>
                <h3 className="text-sm font-bold text-slate-100">{t('modList.groupBy.noteTitle', { name: groupNoteOf.name })}</h3>
                <textarea autoFocus value={groupNoteOf.note} onChange={(e) => setGroupNoteOf({ ...groupNoteOf, note: e.target.value })} rows={4} maxLength={500} className="w-full px-2.5 py-2 rounded-lg bg-slate-950 border border-slate-700 text-xs text-slate-100 resize-none" />
                <div className="flex justify-end gap-2">
                  <button onClick={() => setGroupNoteOf(null)} className="px-3 py-1.5 rounded-md border border-slate-700 text-xs text-slate-300 hover:bg-slate-800 cursor-pointer">{t('dialogs.meta.close')}</button>
                  <button onClick={() => void TauriService.setModGroupNote(paths, groupNoteOf.name, groupNoteOf.note).then((m) => { setMeta(m); setGroupNoteOf(null); })} className="px-3 py-1.5 rounded-md bg-emerald-700 text-xs text-white hover:bg-emerald-600 cursor-pointer">{t('dialogs.meta.saveNote')}</button>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Toolbar: find and filter on the left, the problems to walk through and the switches for everything on the right. */}
      <div className="flex flex-wrap items-center gap-2 mb-3 px-2 py-1.5 rounded-xl border border-slate-800 bg-slate-900/60">
        <div className="relative flex items-center">
          <Search className="w-3.5 h-3.5 text-slate-500 absolute left-2.5 pointer-events-none" />
          <input
            type="text"
            placeholder={t('modList.searchPlaceholder')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-52 bg-slate-950 border border-slate-800 focus:border-cyan-500/80 rounded-md pl-8 pr-7 py-1 text-xs text-slate-200 placeholder-slate-500 transition outline-none"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery('')}
              className="absolute right-1.5 text-slate-500 hover:text-slate-300 p-0.5 cursor-pointer"
              title={t('modList.clearSearch')}
            >
              <X className="w-3 h-3" />
            </button>
          )}
        </div>

        <Filter className="w-3.5 h-3.5 text-slate-500 ml-1" aria-hidden="true" />
        <div role="group" aria-label={t('modList.filterLabel')} title={t('modList.filterTitle')} className="flex flex-wrap items-center rounded-md border border-slate-800 bg-slate-950 overflow-hidden text-xs">
          {MOD_FILTERS.map((id, i) => (
            <React.Fragment key={id}>
              {i > 0 && <span className="w-px self-stretch bg-slate-800" />}
              <button
                type="button"
                onClick={() => setStateFilter(id)}
                aria-pressed={stateFilter === id}
                title={t(`modList.filters.${id}.hint`)}
                className={`px-2 py-1 transition cursor-pointer ${stateFilter === id ? 'bg-emerald-900/60 text-emerald-200' : 'text-slate-300 hover:bg-slate-800'} ${id !== 'all' && (filterCounts[id] ?? 0) === 0 && stateFilter !== id ? 'opacity-50' : ''}`}
              >
                {t(`modList.filterButtons.${id}`)}
                {id !== 'all' && <span className="ml-1 text-[10px] text-slate-500">{filterCounts[id] ?? 0}</span>}
              </button>
            </React.Fragment>
          ))}
        </div>
        {anyFilter && (
          <button onClick={clearFilters} className="text-[11px] text-cyan-400 hover:text-cyan-300 underline cursor-pointer" title={t('modList.clearFiltersTitle')}>
            {t('modList.clearFilters')}
          </button>
        )}
        <div className="basis-full h-0" aria-hidden="true" />
        <select
          value={sortBy}
          onChange={(e) => setSortBy(e.target.value as SortBy)}
          aria-label={t('modList.sortBy.label')}
          title={t('modList.sortBy.title')}
          className={selectClass(sortBy !== 'order')}
        >
          <option value="order">{t('modList.sortBy.order')}</option>
          <option value="name">{t('modList.sortBy.name')}</option>
          <option value="newest">{t('modList.sortBy.newest')}</option>
          <option value="oldest">{t('modList.sortBy.oldest')}</option>
        </select>
        <select
          value={groupBy}
          onChange={(e) => setGroupBy(e.target.value as 'none' | 'author' | 'category' | 'mine')}
          aria-label={t('modList.groupBy.label')}
          title={t('modList.groupBy.title')}
          className={selectClass(groupBy !== 'none')}
        >
          <option value="none">{t('modList.groupBy.none')}</option>
          <option value="author">{t('modList.groupBy.author')}</option>
          <option value="category">{t('modList.groupBy.category')}</option>
          <option value="mine">{t('modList.groupBy.mine')}</option>
        </select>
        <select
          value={groupFilter}
          onChange={(e) => setGroupFilter(e.target.value)}
          disabled={groupCounts.size === 0 && groupFilter === 'all'}
          aria-label={t('modList.groups.filterLabel')}
          title={t('modList.groups.filterTitle')}
          className={`${selectClass(groupFilter !== 'all')} disabled:opacity-50`}
        >
          <option value="all">{t('modList.groups.all')}</option>
          {meta.groups
            .filter((g) => (groupCounts.get(g.name) ?? 0) > 0 || g.name === groupFilter)
            .map((g) => (
              <option key={g.name} value={g.name}>{g.name} ({groupCounts.get(g.name) ?? 0})</option>
            ))}
        </select>
        {groupFilter !== 'all' && (
          <>
            <button onClick={() => setGroupOn(groupFilter, true)} className="text-[11px] px-2 py-0.5 rounded border border-emerald-700/60 text-emerald-300 hover:bg-emerald-950/50 cursor-pointer">{t('modList.groups.turnOn')}</button>
            <button onClick={() => setGroupOn(groupFilter, false)} className="text-[11px] px-2 py-0.5 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer">{t('modList.groups.turnOff')}</button>
          </>
        )}
        <select
          value={sourceFilter}
          onChange={(e) => setSourceFilter(e.target.value)}
          aria-label={t('modList.sourceLabel')}
          title={t('modList.sourceTitle')}
          className={selectClass(sourceFilter !== 'all')}
        >
          <option value="all">{t('modList.sources.all')}</option>
          {MOD_SOURCES.filter((id) => (sourceCounts.get(id) ?? 0) > 0 || id === sourceFilter).map((id) => (
            <option key={id} value={id}>{t(`modList.sources.${id}`)} ({sourceCounts.get(id) ?? 0})</option>
          ))}
        </select>
        <select
          value={tagFilter}
          onChange={(e) => setTagFilter(e.target.value)}
          aria-label={t('modList.tagLabel')}
          title={t('modList.tagTitle')}
          className={selectClass(tagFilter !== 'all')}
        >
          <option value="all">{t('modList.tags.all')}</option>
          {tagFilter !== 'all' && !tagOptions.some((tag) => tag.id === tagFilter) && <option value={tagFilter}>{tagFilter} (0)</option>}
          {tagOptions.map((tag) => (
            <option key={tag.id} value={tag.id}>
              {tag.label} ({tag.count})
            </option>
          ))}
        </select>
        {activeProfile && (
          <button
            type="button"
            onClick={() => setShowEveryMod((v) => !v)}
            aria-pressed={showEveryMod}
            title={showEveryMod ? t('modList.onlyProfileTitle', { name: activeProfile.name }) : t('modList.everythingTitle')}
            className={`px-2 py-1 rounded-md text-xs font-medium border cursor-pointer transition ${
              showEveryMod
                ? 'bg-slate-800 text-slate-200 border-slate-700'
                : 'bg-emerald-950/60 text-emerald-300 border-emerald-800/70'
            }`}
          >
            {showEveryMod ? t('modList.showingEverything') : t('modList.onlyProfile', { name: activeProfile.name })}
          </button>
        )}

        <div className="flex-1" />


        {picked.size > 1 && (
          <span className="flex items-center gap-1.5 text-[11px] text-emerald-300">
            {t('modList.picked.count', { count: picked.size })}
            <button onClick={() => setPicked(new Set())} title={t('modList.picked.clear')} className="p-0.5 rounded text-slate-400 hover:text-slate-100 hover:bg-slate-800 cursor-pointer"><X className="w-3 h-3" /></button>
          </span>
        )}
        <div className="flex items-center rounded-md border border-slate-800 bg-slate-950 overflow-hidden text-xs">
          <button onClick={undo} disabled={history.current.length === 0} title={t('modList.undoTitle')} aria-label={t('modList.undoTitle')} className="px-2 py-1 text-slate-300 hover:bg-slate-800 disabled:opacity-30 cursor-pointer">
            <Undo2 className="w-3.5 h-3.5" />
          </button>
          <span className="w-px self-stretch bg-slate-800" />
          <button onClick={redo} disabled={future.current.length === 0} title={t('modList.redoTitle')} aria-label={t('modList.redoTitle')} className="px-2 py-1 text-slate-300 hover:bg-slate-800 disabled:opacity-30 cursor-pointer">
            <Redo2 className="w-3.5 h-3.5" />
          </button>
        </div>
        <div className="flex items-center rounded-md border border-slate-800 bg-slate-950 overflow-hidden text-xs">
          <button
            onClick={() => handleEnableAllWithoutDiscrimination(true)}
            className="flex items-center gap-1 px-2 py-1 hover:bg-slate-800 text-emerald-400 font-medium transition cursor-pointer"
            title={t('modList.enableAllTitle')}
          >
            <CheckSquare className="w-3.5 h-3.5" />
            <span className="hidden 2xl:inline">{t('modList.enableAll')}</span>
          </button>
          <span className="w-px self-stretch bg-slate-800" />
          <button
            onClick={() => handleEnableAllWithoutDiscrimination(false)}
            className="flex items-center gap-1 px-2 py-1 hover:bg-slate-800 text-slate-400 font-medium transition cursor-pointer"
            title={t('modList.disableAllTitle')}
          >
            <Square className="w-3.5 h-3.5 text-slate-500" />
            <span className="hidden 2xl:inline">{t('modList.disableAll')}</span>
          </button>
        </div>
      </div>

      {/* Save Notification Toast */}
      {saveToast && (
        <div className="fixed top-16 right-8 bg-emerald-950/90 border border-emerald-500/80 text-emerald-200 px-4 py-2 rounded-xl text-xs font-bold shadow-2xl z-50 flex items-center gap-2 animate-fade-in">
          <Check className="w-4 h-4 text-emerald-400" />
          <span>{saveToast}</span>
        </div>
      )}

      {/* Main 2-Column Grid: Left Column Reorderable List / Right Column Mod Inspector */}
      <div className="flex-1 grid grid-cols-12 gap-6 min-h-0">
        {/* Left Column (7 cols): Reorderable Mod List */}
        <div className="col-span-12 lg:col-span-7 bg-slate-900/80 border border-slate-800 rounded-xl flex flex-col overflow-hidden shadow">
          <div className="flex gap-2 pl-5 pr-4 py-2 bg-slate-900 text-slate-500 font-bold text-[10.5px] uppercase tracking-wider border-b border-slate-800 items-center">
            <div className="w-9 shrink-0 text-right">#</div>
            <div className="w-8 shrink-0 text-center">{t('modList.colActive')}</div>
            <div className="flex-1 min-w-0 pl-8">
              {t('modList.colName')}
              {filteredMods.length !== scopeMods.length && (
                <span className="ml-2 normal-case tracking-normal font-normal text-cyan-400">
                  {t('modList.showingCount', { shown: filteredMods.length, total: scopeMods.length })}
                </span>
              )}
            </div>
            <div className="min-w-[150px] text-right">{t('modList.colActions')}</div>
          </div>

          {activeProfile && orphanMods.length > 0 && dismissedOrphans !== orphanKey && (
            <div className="bg-sky-950/40 border-b border-sky-800/50 px-4 py-2 flex items-center gap-2 text-xs text-sky-200">
              <PackagePlus className="w-4 h-4 shrink-0 text-sky-400" />
              <span
                className="flex-1 min-w-0"
                title={orphanMods.map((m) => m.name).join('\n')}
              >
                {t('modList.orphans.text', { count: orphanMods.length, name: activeProfile.name })}
              </span>
              <button
                onClick={() => void addOrphansToProfile()}
                className="px-2 py-1 rounded bg-sky-700/60 hover:bg-sky-600/70 text-sky-50 font-medium cursor-pointer"
              >
                {t('modList.orphans.add', { count: orphanMods.length })}
              </button>
              {!showEveryMod && (
                <button
                  onClick={() => setShowEveryMod(true)}
                  className="px-2 py-1 rounded hover:bg-sky-900/60 text-sky-300 cursor-pointer"
                >
                  {t('modList.orphans.show')}
                </button>
              )}
              <button
                onClick={() => setDismissedOrphans(orphanKey)}
                className="p-1 rounded hover:bg-sky-900/60 text-sky-400 cursor-pointer"
                title={t('modList.orphans.dismiss')}
                aria-label={t('modList.orphans.dismiss')}
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
          )}

          <div className="bg-emerald-950/40 border-b border-emerald-800/40 px-4 py-1.5 flex items-center justify-between text-[11px] font-mono text-emerald-400">
            <span className="flex items-center gap-1.5 font-bold">
              <ArrowUpCircle className="w-3.5 h-3.5" />
              {t('modList.loadsFirst')}
            </span>
            <span className="text-[10px] text-emerald-500/80">{t('modList.position', { n: 1 })}</span>
          </div>

          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            {filteredMods.length === 0 ? (
              <div className="p-8 text-center text-slate-500 text-xs">
                {activeProfile && !showEveryMod && scopeMods.length === 0 ? (
                  <>
                    {t('modList.emptyProfile', { name: activeProfile.name })}
                    <button
                      onClick={() => setShowEveryMod(true)}
                      className="ml-2 underline text-emerald-400 hover:text-emerald-300 cursor-pointer"
                    >
                      {t('modList.emptyProfileShowAll')}
                    </button>
                  </>
                ) : searchQuery ? (
                  <>{t('modList.noMatch')} "<b className="text-slate-400">{searchQuery}</b>"</>
                ) : (
                  <>{t('modList.noneFiltered')}</>
                )}
                {anyFilter && (
                  <button
                    onClick={clearFilters}
                    className="ml-2 underline text-emerald-400 hover:text-emerald-300 cursor-pointer"
                  >
                    {t('modList.clearFilters')}
                  </button>
                )}
              </div>
            ) : (
              groupBy === 'none' ? filteredMods.map(renderRow) : renderGrouped()
            )}
          </div>

          <div className="bg-amber-950/40 border-t border-amber-800/40 px-4 py-1.5 flex items-center justify-between text-[11px] font-mono text-amber-400">
            <span className="flex items-center gap-1.5 font-bold">
              <ArrowDownCircle className="w-3.5 h-3.5" />
              {t('modList.loadsLast')}
            </span>
            <span className="text-[10px] text-amber-500/80">{t('modList.position', { n: Math.max(activeRanks.size, 1) })}</span>
          </div>
        </div>

        {/* Right Column (5 cols): the selected mod, its problems and what it needs. */}
        <ModInspector
          paths={paths}
          mod={selectedMod}
          mods={mods}
          position={selectedMod ? (activeRanks.get(selectedMod.mod_id) ?? 0) - 1 : -1}
          problems={
            selectedMod
              ? {
                  // Places among the mods that are on, as on the rows and in the game.
                  orderViolation: (() => {
                    const v = loadOrderViolationsMap[selectedMod.mod_id]?.[0];
                    return v ? { ...v, requiredModIndex: (activeRanks.get(v.requiredModId) ?? 0) - 1 } : undefined;
                  })(),
                  disabledDependency: missingActiveDependenciesMap[selectedMod.mod_id]?.[0],
                  incompatibility: activeConflictsMap[selectedMod.mod_id]?.[0],
                }
              : {}
          }
          packageBadgeClass={
            selectedMod?.workshop_id && multiModPackageMap[selectedMod.workshop_id]
              ? workshopColorMap[selectedMod.workshop_id]?.badge
              : undefined
          }
          onJumpToMod={handleJumpToMod}
          onJumpToDependency={handleJumpToDependency}
          onFixLoadOrder={handleFixLoadOrderViolation}
          onToggleMod={handleToggleSingleMod}
          onOpenInBrowser={onOpenInBrowser}
        />
      </div>

      {/* In-App Auto-Sort Notice Modal */}
      {showAutoSortNotice && (
        <div className="fixed inset-0 bg-slate-950/80 backdrop-blur-md flex items-center justify-center p-4 z-50 animate-in fade-in duration-200">
          <div className="bg-slate-900 border border-emerald-500/50 rounded-2xl p-6 max-w-md w-full space-y-4 shadow-2xl">
            <div className="flex items-center gap-3 border-b border-slate-800 pb-3">
              <div className="w-10 h-10 rounded-xl bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shrink-0">
                <Wand2 className="w-5 h-5" />
              </div>
              <div>
                <h3 className="text-base font-bold text-slate-100">{t('modList.sortDone')}</h3>
                <p className="text-xs text-slate-400">
                  {lastSortMoved === 0 ? t('modList.sortNothing') : t('modList.sortMoved', { count: lastSortMoved })}
                </p>
              </div>
            </div>

            <ul className="text-xs text-slate-300 space-y-2 bg-slate-950/60 p-3.5 rounded-xl border border-slate-800 font-sans leading-relaxed">
              <li dangerouslySetInnerHTML={{ __html: tHtml('modList.sortRule1') }} />
              <li>{t('modList.sortRule2')}</li>
              <li className="text-amber-300">{t('modList.sortRule3')}</li>
            </ul>

            <div className="flex items-center justify-between pt-2 border-t border-slate-800 gap-4">
              <label className="flex items-center gap-2 text-xs text-slate-400 hover:text-slate-200 cursor-pointer select-none">
                <input
                  type="checkbox"
                  checked={dontShowNoticeChecked}
                  onChange={(e) => setDontShowNoticeChecked(e.target.checked)}
                  className="rounded bg-slate-950 border-slate-700 text-emerald-500 focus-visible:outline-2 focus-visible:outline-emerald-400 focus-visible:outline-offset-2 w-4 h-4 cursor-pointer"
                />
                <span>{t('modList.dontShowAgain')}</span>
              </label>

              {orderBeforeSort && (
                <button
                  onClick={() => {
                    handleUndoSort();
                    setShowAutoSortNotice(false);
                  }}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs font-bold transition cursor-pointer shrink-0"
                >
                  {t('modList.undo')}
                </button>
              )}
              <button
                onClick={() => {
                  if (dontShowNoticeChecked) {
                    localStorage.setItem('pz_hide_autosort_notice', 'true');
                  }
                  setShowAutoSortNotice(false);
                }}
                className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-xs font-bold shadow transition cursor-pointer shrink-0"
              >
                {t('modList.understood')}
              </button>
            </div>
          </div>
        </div>
      )}

    </div>
  );
};
