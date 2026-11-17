import React, { useEffect, useMemo, useRef, useState } from 'react';
import { PageGuide } from '../common/PageGuide';
import { Activity, ArrowRight, ClipboardList, CheckCircle2, ChevronDown, ChevronRight, Film, Gamepad2, GitMerge, HardDrive, HeartPulse, ListChecks, Loader2, MemoryStick, MessageSquare, RefreshCw, Rocket, Server, ShieldCheck, Stethoscope, Timer, Wand2, X } from 'lucide-react';
import { AutoMergeResult, CompatResult, HealthFix, HealthIssue, HealthReport, IssueState, ModInfo, CatalogModSummary } from '../../types';
import { STATE_ORDER, STATE_STYLE, UNDO_ACTIONS, issueStateOf, scopeOfIssue, SCOPE_CHIP } from '../../utils/issueState';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, reportBackendError } from '../../services/tauri';
import { loadCommunityIndex } from '../../services/communityIndex';
import { applyFix } from '../../utils/healthFix';
import { healthVerdict, inDoFirstOrder } from '../../utils/healthVerdict';
import type { HealthChanges } from '../../utils/healthChanges';
import { HealthListTarget, fixActsOn, listDifference, serverListIds } from '../../utils/healthList';
import { useListHealth } from '../../hooks/useListHealth';
import { DiagSection, DiagnosticsHome, GameLivePanel, ServerLivePanel } from './DiagnosticsPanels';
import { ProfilePhotos } from './ProfilePhotos';
import { AuditPanel } from './AuditPanel';
import { pickCollectionMods } from '../../utils/collectionPick';
import { certaintyOf } from '../../utils/certainty';
import { useTranslation } from 'react-i18next';
import i18next from 'i18next';
import { issueText } from '../../i18n/backendText';
import { RichText } from '../common/RichText';
import { tHtml } from '../../i18n/html';
import { GameTimePanel } from './GameTimePanel';
import { FramesPanel } from './FramesPanel';
import { openRequiredItem } from '../../utils/workshopRequirement';
import { BootTimePanel } from './BootTimePanel';
import { MemoryPanel } from './MemoryPanel';
import { DiskPanel } from './DiskPanel';

interface HealthModuleProps {
  paths: StudioPathsUI;
  mods: ModInfo[];
  /** Latest report from useModHealth (shared with the Mod List and the sidebar badge). */
  report: HealthReport | null;
  checking: boolean;
  /** The step of the check under way (useModHealth): said in words above the page. */
  checkStage?: 'quick' | 'full' | 'scripts' | null;
  onRecheck: () => void;
  /** When the profile's last full check ended (ms), and what changed since (null: nothing known). */
  checkedAt?: number | null;
  changes?: HealthChanges | null;
  /** Replaces the whole list (order + enabled flags) and persists it. */
  onApplyModList: (mods: ModInfo[]) => void;
  /** Mods were added/removed on disk (compatible copies, patch packages): rescan the list. */
  onModsChanged: () => void;
  /** Opens the Mod Merger on this conflicting file. */
  onOpenMerger: (relativePath: string) => void;
  /** Rebuild a patch package for the list as it is now (the Merger's Update patch). */
  onUpdatePatch?: (packageFolderName: string) => Promise<boolean>;
  /** Opens a Workshop item / a Workshop search in Explore Mods. */
  onOpenInBrowser?: (workshopId: string) => void;
  onSearchInBrowser?: (text: string) => void;
  /** The issue a Mod List chip pointed at: scrolled to and lit up once. */
  focusIssue?: HealthIssue | null;
  onFocusHandled?: () => void;
  /** The active profile's name, for the list selector. */
  profileName?: string | null;
  /** The active profile's id, for its photos. */
  profileId?: string | null;
  /** The installed mods as the game sees them (when Steam last updated each), for the photos. */
  catalog?: CatalogModSummary[];
  /** A tab another screen sends the player to (the header's Doctor and meter, the Doctor's steps). */
  focusTab?: HealthTab | null;
  onFocusTabHandled?: () => void;
  /** The Doctor, shown on the front page. */
  doctor?: React.ReactNode;
  /** Opens the Servers screen (to start a server). */
  onGoServers?: () => void;
}

/** Fixes handled here (not by rewriting the mod list). */
const ACTION_FIXES = new Set<HealthFix['action']>([
  'subscribe_workshop', 'open_workshop_page', 'make_compatible_copy', 'keep_version', 'remove_compatible_copy', 'merge_files', 'open_in_merger',
  'enable_shim_patch', 'remove_from_package', 'dismiss_known_issue', 'adopt_workshop_copy', 'recopy_workshop', 'update_package',
]);

/** Fixes the "fix all" button may apply without asking: they only enable or reorder. */
const SAFE_FIX_ACTIONS = new Set<HealthFix['action']>(['enable_mod', 'move_before']);

/**
 * Issues whose fix is a matter of taste, not of the game's rules: two mods defining the same item
 * is not broken, and which version you want is nobody's call but yours. Their fix stays on its own
 * row and is kept out of the one-click button, which is for what the game itself would complain
 * about.
 */
const YOUR_CALL_CODES = new Set(['script_definition_conflict', 'script_definition_stale', 'script_definition_settled', 'translation_conflict', 'translation_stale', 'translation_settled', 'package_built_for_other_list']);


/**
 * The screen in tabs. The first one is "what to do now": what breaks the game, then what is worth
 * doing, and folded below them what the Studio already fixed and what is only worth knowing. Every
 * row on every tab says one of those four states (`issueStateOf`), the same words the Mod List
 * uses. Conflicts (which mod's file or item wins) and Security keep tabs of their own.
 */
export type HealthTab = 'home' | 'problems' | 'conflicts' | 'security' | 'audit' | 'modtime' | 'live' | 'memory' | 'boot' | 'frames' | 'disk' | 'server';
/** The tabs that are lists of findings. */
type ListTab = 'problems' | 'conflicts' | 'security';
const isListTab = (tab: HealthTab): tab is ListTab => tab === 'problems' || tab === 'conflicts' || tab === 'security';
/**
 * Diagnóstico in parts: what it knows about the mods, about the game and how it runs, about the
 * server and how it runs. The front page (home, the Doctor) sits above the three.
 */
const SECTION_TABS: Record<DiagSection, HealthTab[]> = {
  mods: ['problems', 'conflicts', 'security', 'audit', 'modtime'],
  game: ['live', 'memory', 'boot', 'frames', 'disk'],
  server: ['server'],
};
const SECTIONS: DiagSection[] = ['mods', 'game', 'server'];
const sectionOf = (tab: HealthTab): DiagSection | 'home' => (tab === 'home' ? 'home' : SECTIONS.find((sec) => SECTION_TABS[sec].includes(tab)) ?? 'mods');
const SECTION_ICON: Record<DiagSection | 'home', React.ElementType> = { home: Stethoscope, mods: HeartPulse, game: Gamepad2, server: Server };
/** The tab last open in each part, so a part opens where it was left. */
const LAST_TAB_KEY = 'pz_diag_last_';
const TAB_ICON: Record<HealthTab, React.ElementType> = {
  home: Stethoscope,
  problems: ListChecks,
  conflicts: GitMerge,
  security: ShieldCheck,
  audit: ClipboardList,
  modtime: Timer,
  live: Activity,
  memory: MemoryStick,
  boot: Rocket,
  frames: Film,
  disk: HardDrive,
  server: Server,
};
/** Rows of one code that are the same thing for several mods: one card with one button. */
const GROUP_CODES = new Set(['compatible_copy_unneeded', 'compatible_copy', 'workshop_copy_by_hand', 'workshop_copy_stale', 'workshop_copy_kept']);
/** What a button changes, said before it is pressed: the key under health.effect. */
const effectKey = (fix: HealthFix): string => `health.effect.${fix.action}`;
/** The sections the player folded, by tab and section. */
const COLLAPSED_KEY = 'pz_health_collapsed';
/** Two mods giving the same file, item, recipe or text: the game uses one, which one is your call. */
const CONFLICT_CODE = /^(file_conflict|script_definition|translation|definition)/;

const fixLabel = (fix: HealthFix): string => {
  const t = i18next.t;
  switch (fix.action) {
    case 'enable_mod':
      return t('health.fix.enable', { id: fix.mod_id });
    case 'disable_mod':
      return t('health.fix.disable', { id: fix.mod_id });
    case 'move_before':
      return t('health.fix.moveBefore', { id: fix.mod_id, before: fix.before });
    case 'remove_from_list':
      return t('health.fix.remove', { entry: fix.entry });
    case 'replace_in_list':
      return t('health.fix.replace', { id: fix.mod_id, entry: fix.entry });
    case 'subscribe_workshop':
      return t('health.fix.searchWorkshop', { hint: fix.hint });
    case 'open_workshop_page':
      return t('health.fix.openWorkshop');
    case 'make_compatible_copy':
      return t('health.fix.makeCopy');
    case 'keep_version':
      return t('health.fix.keepVersion');
    case 'remove_compatible_copy':
      return t('health.fix.removeCopy');
    case 'merge_files':
      return t('health.fix.mergeAuto');
    case 'open_in_merger':
      return t('health.fix.openMerger');
    case 'enable_shim_patch':
      return t('health.fix.shimOn', { name: fix.name });
    case 'remove_from_package':
      return t('health.fix.removeFromPackage');
    case 'dismiss_known_issue':
      return t('health.fix.dismissKnown');
    case 'adopt_workshop_copy':
      return t('health.fix.adoptCopy');
    case 'recopy_workshop':
      return t('health.fix.recopy');
    case 'update_package':
      return t('health.fix.updatePackage');
  }
};

/** A row of a section: one issue, or several of one code shown as one card. */
type SectionItem = { kind: 'one'; issue: HealthIssue } | { kind: 'group'; code: string; issues: HealthIssue[] };
/**
 * Which rows are one card: copies across mods by code; a mod's known issues by mod (the index
 * lists one row per broken require, with the same fix).
 */
const groupKey = (i: HealthIssue): string | null =>
  // Java patches of several mods on one class: one card for the shared methods, one for the rest.
  i.code === 'java_patch_overlap' ? `${i.code}|${i.params?.variant === 'class' ? 'class' : 'method'}`
  // A copy a server keeps in use says so on its own row.
  : GROUP_CODES.has(i.code) && !(i.code.startsWith('compatible_copy') && i.params?.variant === 'server') ? i.code : i.code === 'community_known_issue' && i.mod_id ? `${i.code}|${i.mod_id}` : null;
const groupItems = (issues: HealthIssue[]): SectionItem[] => {
  const groups = new Map<string, HealthIssue[]>();
  issues.forEach((i) => {
    const key = groupKey(i);
    if (key) groups.set(key, [...(groups.get(key) ?? []), i]);
  });
  const placed = new Set<string>();
  const out: SectionItem[] = [];
  issues.forEach((i) => {
    const key = groupKey(i);
    const group = key ? groups.get(key) : undefined;
    if (key && group && group.length > 1) {
      if (!placed.has(key)) out.push({ kind: 'group', code: i.code, issues: group });
      placed.add(key);
    } else {
      out.push({ kind: 'one', issue: i });
    }
  });
  return out;
};

export const HealthModule: React.FC<HealthModuleProps> = ({ paths, mods, report: profileReport, checking: profileChecking, checkStage: profileStage = null, onRecheck: profileRecheck, checkedAt: profileCheckedAt = null, changes: profileChanges = null, onApplyModList, onModsChanged, onOpenMerger, onUpdatePatch, onOpenInBrowser, onSearchInBrowser, focusIssue = null, onFocusHandled, profileName = null, profileId = null, catalog = [], focusTab = null, onFocusTabHandled, doctor, onGoServers }) => {
  const { t } = useTranslation();
  // Which list is checked: the profile's (the app's own check, shared with the Mod List), or a
  // server's Mods= (checked here, on its own). Everything below reads report / checking.
  const [servers, setServers] = useState<{ name: string; mods: string[]; file_path: string }[]>([]);
  const loadServers = () => {
    if (!paths.user_zomboid_dir) return;
    TauriService.listServerConfigs(paths.user_zomboid_dir)
      .then((list) => setServers((list as { name: string; mods: string[]; file_path: string }[]).map((s) => ({ name: s.name, mods: s.mods, file_path: s.file_path }))))
      .catch(() => setServers([]));
  };
  useEffect(loadServers, [paths.user_zomboid_dir]);
  const [listTarget, setListTarget] = useState<HealthListTarget>({ kind: 'profile' });
  const serverIds = useMemo(() => {
    if (listTarget.kind !== 'server') return null;
    const server = servers.find((s) => s.name === listTarget.name);
    return server ? serverListIds(server.mods) : null;
  }, [listTarget, servers]);
  const serverHealth = useListHealth(paths, serverIds);
  const onServer = listTarget.kind === 'server';
  const report = onServer ? serverHealth.report : profileReport;
  const checking = onServer ? serverHealth.checking : profileChecking;
  const checkStage = onServer ? (serverHealth.checking ? 'full' : null) : profileStage;
  const onRecheck = onServer ? () => void serverHealth.recheck() : profileRecheck;
  /** On a server: how its list differs from the profile's switched-on mods. */
  const profileOn = useMemo(() => mods.filter((m) => m.enabled).map((m) => m.mod_id), [mods]);
  const serverDiff = useMemo(() => (serverIds ? listDifference(profileOn, serverIds) : null), [profileOn, serverIds]);
  const [serverBusy, setServerBusy] = useState(false);
  /**
   * Writes a list into the server's .ini (Mods=, and its WorkshopItems= and Map= from the installed
   * mods), as Servers' own button does; the .ini is backed up first. Then the server's list is read
   * again, and the check follows it.
   */
  const writeServerList = async (ids: string[], said: string) => {
    const server = listTarget.kind === 'server' ? servers.find((x) => x.name === listTarget.name) : undefined;
    if (!server) return;
    setServerBusy(true);
    try {
      const written = await TauriService.syncClientToServer(paths, server.file_path, ids);
      say(`${said} ${t('health.list.written', { name: server.name, count: written.mods.length })}`);
      loadServers();
    } catch (err) {
      reportBackendError("Writing the server's list", err);
    } finally {
      setServerBusy(false);
    }
  };
  const writeProfileToServer = () => {
    if (!serverDiff || listTarget.kind !== 'server') return;
    if (!confirm(t('health.list.writeConfirm', { name: listTarget.name, server: serverIds?.length ?? 0, profile: profileOn.length, profileName: profileName ?? '-' }))) return;
    void writeServerList(profileOn, '');
  };
  const takeOffServer = (modId: string) => {
    if (!serverIds || listTarget.kind !== 'server') return;
    if (!confirm(t('health.list.takeOffConfirm', { mod: nameOf(modId), name: listTarget.name }))) return;
    void writeServerList(serverIds.filter((id) => id !== modId), t('health.list.takenOff', { mod: nameOf(modId) }));
  };

  /**
   * Alternatives of one Workshop item switched on together (a profile made from a collection
   * before the Studio picked one of each): the ones to switch off, keeping one per group as a
   * new collection profile would (utils/collectionPick).
   */
  const variantsOn = useMemo(() => {
    const on = mods.filter((m) => m.enabled && m.workshop_id);
    const items = [...new Set(on.map((m) => m.workshop_id as string))];
    return pickCollectionMods(items, on);
  }, [mods]);
  const keepOneVariant = () => {
    if (!confirm(t('health.variants.confirm', { count: variantsOn.off.length, list: variantsOn.choices.map((c) => t('app.collection.alternativeLine', { picked: c.picked, left: c.left.join(', ') })).join('\n') }))) return;
    const off = new Set(variantsOn.off);
    onApplyModList(mods.map((m) => (off.has(m.mod_id) ? { ...m, enabled: false } : m)));
    say(t('health.variants.done', { count: off.size }));
  };

  // A Mod List chip points at the profile's issue: its list comes back in front.
  useEffect(() => {
    if (focusIssue) setListTarget({ kind: 'profile' });
  }, [focusIssue]);
  const [litKey, setLitKey] = useState<string | null>(null);
  const issueKey = (i: HealthIssue) => `${i.code}|${i.mod_id ?? ''}|${i.message}`;
  // Opening this screen checks again, and the check lands in steps (the quick one, the full one,
  // the script conflicts), each moving the issues around. Scrolling on every step made the page
  // jump, and waiting for the last one took seconds: it scrolls once, as soon as the issue is on
  // the page (the report shown already has it), and waits for the check only when it is not there yet.
  const scrolledFor = useRef<HealthIssue | null>(null);
  /** The verdict's "Go": the same way to an issue as a Mod List chip. */
  const [ownFocus, setOwnFocus] = useState<HealthIssue | null>(null);
  const target = focusIssue ?? ownFocus;
  const focusDone = () => {
    setOwnFocus(null);
    onFocusHandled?.();
  };
  const unlight = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(unlight.current), []);
  useEffect(() => {
    if (!target) {
      scrolledFor.current = null;
      return;
    }
    if (!report || scrolledFor.current === target) return;
    const settle = window.setTimeout(() => {
      const key = issueKey(target);
      const el =
        document.querySelector<HTMLElement>(`[data-issue-key="${CSS.escape(key)}"]`) ??
        document.querySelector<HTMLElement>(`[data-issue-code="${CSS.escape(`${target.code}|${target.mod_id ?? ''}`)}"]`);
      if (!el) {
        // Not on the page yet: the check under way may bring it. Gone once it is done.
        if (checking) return;
        scrolledFor.current = target;
        focusDone();
        return;
      }
      scrolledFor.current = target;
      el.scrollIntoView({ block: 'center', behavior: 'smooth' });
      setLitKey(el.dataset.issueKey ?? key);
      window.clearTimeout(unlight.current);
      unlight.current = window.setTimeout(() => {
        setLitKey(null);
        focusDone();
      }, 4000);
    }, 250);
    return () => window.clearTimeout(settle);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target, report, checking]);
  const [compatBusy, setCompatBusy] = useState(false);
  const [compatResult, setCompatResult] = useState<CompatResult | null>(null);
  const [mergeBusy, setMergeBusy] = useState(false);
  const [mergeResult, setMergeResult] = useState<AutoMergeResult | null>(null);
  // A fix that started no check (cancelled, failed) lets its row go after a while.
  const pendingCount = useRef(0);
  useEffect(() => {
    const tick = window.setInterval(() => {
      if (pendingCount.current === 0) return;
      const now = Date.now();
      setPending((prev) => {
        const next = new Map(Array.from(prev).filter(([, v]) => (v.saw ? now - v.at < 60000 : now - v.at < 8000 || compatBusy || mergeBusy)));
        return next.size === prev.size ? prev : next;
      });
    }, 1000);
    return () => window.clearInterval(tick);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [compatBusy, mergeBusy]);

  const mergeFiles = async (relativePaths: string[]) => {
    if (relativePaths.length === 0) return;
    setMergeBusy(true);
    try {
      const result = await TauriService.autoMergeConflicts(paths, relativePaths);
      setMergeResult(result);
      onModsChanged(); // the package may be new or re-enabled
      onRecheck();
    } catch (err) {
      reportBackendError('Merging conflicting files', err);
    } finally {
      setMergeBusy(false);
    }
  };

  const makeCompatibleCopy = async (modId: string) => {
    setCompatBusy(true);
    try {
      let result: CompatResult;
      try {
        result = await TauriService.makeCompatibleCopy(paths, modId, false);
      } catch (err) {
        const text = String(err);
        if (text.includes('earlier compatible copy')) {
          if (!confirm(t('health.freshCopy', { mod: modId }))) return;
        } else if (text.includes('as a copy of the same mod')) {
          // A copy made by hand or for a server: the compatible copy is that mod, adapted.
          if (!confirm(t('health.replaceSameModCopy', { mod: nameOf(modId) }))) return;
        } else {
          throw err;
        }
        result = await TauriService.makeCompatibleCopy(paths, modId, true);
      }
      setCompatResult(result);
      onModsChanged();
      onRecheck(); // same mod id, so the list length may not change: check again explicitly
    } catch (err) {
      reportBackendError('Making a compatible copy', err);
    } finally {
      setCompatBusy(false);
    }
  };

  const removeCompatibleCopy = async (folder: string) => {
    if (!confirm(t('health.removeCopyConfirm', { folder }))) return;
    try {
      await TauriService.removeCompatibleCopy(paths, folder);
      setCompatResult(null);
      say(t('health.copyRemovedDone', { folder }));
      onModsChanged();
      onRecheck();
    } catch (err) {
      reportBackendError('Removing the compatible copy', err);
    }
  };

  /** Copies of Workshop mods made by hand in Zomboid/mods: the Studio keeps them (a marker each). */
  const adoptCopies = async (folders: string[]) => {
    let kept = 0;
    for (const folder of folders) {
      try {
        await TauriService.adoptWorkshopCopy(paths, folder);
        kept += 1;
      } catch (err) {
        reportBackendError('Keeping the copy', err);
      }
    }
    if (kept > 0) say(t('health.copiesAdoptedDone', { count: kept }));
    onRecheck();
  };

  /** The Studio's copies a Workshop update outgrew, copied again. */
  const recopyAll = async (modIds: string[]) => {
    setCompatBusy(true);
    let copied = 0;
    try {
      for (const id of modIds) {
        try {
          await TauriService.recopyWorkshop(paths, id);
          copied += 1;
        } catch (err) {
          reportBackendError('Copying the mod again', err);
        }
      }
    } finally {
      setCompatBusy(false);
    }
    if (copied > 0) say(t('health.recopiedDone', { count: copied }));
    onModsChanged();
    onRecheck();
  };

  /** Several copies the originals no longer need, in one go (one question for all). */
  const removeCompatibleCopies = async (folders: string[]) => {
    if (!confirm(t('health.removeCopiesConfirm', { count: folders.length, folders: folders.join(', ') }))) return;
    let removed = 0;
    for (const folder of folders) {
      try {
        await TauriService.removeCompatibleCopy(paths, folder);
        removed += 1;
      } catch (err) {
        reportBackendError('Removing the compatible copy', err);
      }
    }
    setCompatResult(null);
    if (removed > 0) say(t('health.copiesRemovedDone', { count: removed }));
    onModsChanged();
    onRecheck();
  };

  /** Takes a mod file's copy out of the package; the game reads the mod's own file again. */
  const removeFromPackage = async (pkg: string, relativePath: string) => {
    if (!confirm(t('health.removeFromPackageConfirm', { file: relativePath, package: pkg.replace(/^Z_PZModStudio_/, '') }))) return;
    try {
      await TauriService.removeFileFromPackage(paths, pkg, relativePath);
      onModsChanged();
      onRecheck();
    } catch (err) {
      reportBackendError('Removing the file from the package', err);
    }
  };

  /**
   * What the last button did, in words, and that nothing more is needed. A load-order change is
   * invisible from this screen -- the row stays, because two mods still define the same thing --
   * so without this the app looks like it did nothing. It stays until closed or the next button:
   * it went away by itself after six seconds, before it was read.
   */
  const [done, setDone] = useState<string | null>(null);
  const say = (what: string) => setDone(what);
  /**
   * The rows whose fix was just pressed: the button gives way to "Done, checking..." until the
   * check that follows brings the new report, so it is not pressed again while the page catches up
   * (the full check takes seconds on a long list). `saw`: a check has started since.
   */
  const [pending, setPending] = useState<Map<string, { saw: boolean; at: number }>>(new Map());
  const markPending = (issues: HealthIssue[]) =>
    setPending((prev) => {
      const next = new Map(prev);
      issues.forEach((i) => next.set(issueKey(i), { saw: checking, at: Date.now() }));
      return next;
    });
  const isPending = (i: HealthIssue) => pending.has(issueKey(i));
  pendingCount.current = pending.size;
  useEffect(() => {
    if (!checking || pending.size === 0) return;
    setPending((prev) => new Map(Array.from(prev, ([k, v]) => [k, { ...v, saw: true }] as const)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [checking]);
  useEffect(() => {
    // The first full report after a check started answers for them: fixed (gone), or still there.
    // A quick one borrows the slow findings of before, the one just fixed among them.
    if (pending.size === 0 || report?.partial) return;
    setPending((prev) => new Map(Array.from(prev).filter(([, v]) => !v.saw)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [report]);
  /**
   * "Is this up to date?": when the last full check ended, and what changed since that a new one
   * would read (opening Mod Health does not check by itself: a full check reads every mod's files).
   */
  const [nowTick, setNowTick] = useState(Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNowTick(Date.now()), 30000);
    return () => window.clearInterval(timer);
  }, []);
  const lastAt = onServer ? null : profileCheckedAt;
  const checkedWhen = lastAt
    ? `${new Date(lastAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })} (${
        nowTick - lastAt < 60000 ? t('health.lastCheck.justNow') : t('health.lastCheck.minutesAgo', { count: Math.round((nowTick - lastAt) / 60000) })
      })`
    : '';
  const changes = onServer ? null : profileChanges;
  const changesSaid = changes
    ? [
        changes.list && t('health.lastCheck.list'),
        changes.updated.length > 0 && t('health.lastCheck.updated', { count: changes.updated.length }),
        changes.build && t('health.lastCheck.build'),
        changes.played && t('health.lastCheck.played'),
      ]
        .filter(Boolean)
        .join(', ')
    : '';
  const pendingChip = (
    <span className="flex items-center gap-1.5 px-2 py-1 rounded text-[11px] font-bold text-sky-200 border border-sky-800 bg-sky-950/60 shrink-0">
      <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" />
      {t('health.pendingFix')}
    </span>
  );
  /**
   * Whose problem it is, as a small chip: the mod's (same for everyone), this list's, this PC's,
   * or the player's own mod (one they are making).
   */
  const scopeChip = (code: string, ownMod = false) => {
    const scope = scopeOfIssue({ code, own_mod: ownMod });
    return scope ? (
      <span className={`shrink-0 text-[9px] font-bold uppercase tracking-wide px-1.5 py-px rounded border ${SCOPE_CHIP[scope]}`} title={t(`health.scope.${scope}.title`)}>
        {t(`health.scope.${scope}.word`)}
      </span>
    ) : null;
  };
  /** How sure the finding is (seen in the log / the game's rule / a suspicion), as a small chip. */
  const certaintyChip = (issue: HealthIssue) => {
    const c = certaintyOf(issue);
    const tone = c === 'seen' ? 'border-red-700/70 text-red-300' : c === 'rule' ? 'border-slate-600 text-slate-300' : 'border-violet-700/60 text-violet-300';
    return (
      <span className={`shrink-0 text-[9px] font-bold uppercase tracking-wide px-1.5 py-px rounded border ${tone}`} title={t(`health.certainty.${c}.title`)}>
        {t(`health.certainty.${c}.word`)}
      </span>
    );
  };
  /** Copies the note for a mod's author: what works on this build, as a suggestion with the example. */
  const copyAuthorSuggestion = async (modId: string) => {
    try {
      await navigator.clipboard.writeText(await TauriService.compatAuthorSuggestion(paths, modId));
      setDone(t('health.authorNote.copied'));
    } catch (err) {
      setDone(t('health.authorNote.failed', { reason: String(err) }));
    }
  };

  const handleFix = (fix: HealthFix, issue?: HealthIssue) => {
    // What changes something gets its row marked; opening a page or the Merger does not.
    const opensOnly = fix.action === 'open_in_merger' || fix.action === 'open_workshop_page' || fix.action === 'subscribe_workshop' || fix.action === 'keep_version';
    if (!opensOnly && fix.action !== 'disable_mod' && issue) markPending([issue]);
    if (fix.action === 'merge_files') {
      mergeFiles(fix.relative_paths);
      return;
    }
    if (fix.action === 'open_in_merger') {
      onOpenMerger(fix.relative_path);
      return;
    }
    if (fix.action === 'make_compatible_copy') {
      makeCompatibleCopy(fix.mod_id);
      return;
    }
    if (fix.action === 'remove_compatible_copy') {
      removeCompatibleCopy(fix.folder);
      return;
    }
    if (fix.action === 'remove_from_package') {
      void removeFromPackage(fix.package, fix.relative_path);
      return;
    }
    if (fix.action === 'adopt_workshop_copy') {
      void adoptCopies([fix.folder]);
      return;
    }
    if (fix.action === 'recopy_workshop') {
      void recopyAll([fix.mod_id]);
      return;
    }
    if (fix.action === 'update_package') {
      // The Merger's Update patch: rescan for the list as it is now and publish (what waits for a
      // choice stays out, and the Merger says which).
      void (async () => {
        const ok = onUpdatePatch ? await onUpdatePatch(fix.package) : false;
        say(ok ? t('health.packageUpdated', { name: nameOf(fix.package) }) : t('health.packageUpdateFailed'));
        onRecheck();
      })();
      return;
    }
    if (fix.action === 'dismiss_known_issue') {
      void (async () => {
        try {
          await TauriService.dismissIndexIssue(paths, fix.mod_id, fix.title);
          void loadCommunityIndex(paths, true);
          say(t('health.dismissedDone', { name: nameOf(fix.mod_id) }));
          onRecheck();
        } catch {
          // Already shown by TauriService.
        }
      })();
      return;
    }
    if (fix.action === 'enable_shim_patch') {
      void (async () => {
        try {
          const written = await TauriService.setShimPatch(paths.user_zomboid_dir, fix.name, true);
          say(t('health.shimOnDone', { name: fix.name, count: written }));
          onRecheck();
        } catch {
          // Already shown by TauriService.
        }
      })();
      return;
    }
    if (fix.action === 'keep_version') {
      void (async () => {
        try {
          const kept = await TauriService.keepModVersion(paths, fix.mod_id);
          say(t('health.keptDone', { name: kept.mod_name, when: kept.kept_at }));
        } catch {
          // Already shown by TauriService.
        }
      })();
      return;
    }
    if (fix.action === 'open_workshop_page') {
      if (onOpenInBrowser) onOpenInBrowser(fix.workshop_id);
      else TauriService.openExternalUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${fix.workshop_id}`);
      return;
    }
    if (fix.action === 'subscribe_workshop') {
      // The item that provides it, from the requiring mod's Required items; a search otherwise.
      void openRequiredItem(
        issue?.mod_id,
        fix.hint,
        mods,
        (id) => (onOpenInBrowser ? onOpenInBrowser(id) : TauriService.openExternalUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${id}`)),
        (text) => (onSearchInBrowser ? onSearchInBrowser(text) : TauriService.openExternalUrl(`https://steamcommunity.com/workshop/browse/?appid=108600&searchtext=${encodeURIComponent(text)}`))
      );
      return;
    }
    if (fix.action === 'disable_mod') {
      const name = mods.find((m) => m.mod_id === fix.mod_id)?.name ?? fix.mod_id;
      if (!confirm(t('health.confirmDisable', { name }))) return;
      if (issue) markPending([issue]);
    }
    const next = applyFix(mods, fix);
    if (!next) return;
    onApplyModList(next);
    if (fix.action === 'move_before') {
      const moved = nameOf(fix.mod_id);
      const anchor = nameOf(fix.before);
      say(issue && YOUR_CALL_CODES.has(issue.code) ? t('health.nowUsesVersion', { anchor, moved }) : t('health.nowLoadsBefore', { moved, anchor }));
    } else if (fix.action === 'enable_mod') {
      say(t('health.switchedOn', { name: nameOf(fix.mod_id) }));
    } else if (fix.action === 'disable_mod') {
      say(t('health.switchedOff', { name: nameOf(fix.mod_id) }));
    } else if (fix.action === 'remove_from_list') {
      say(t('health.outOfList', { entry: fix.entry }));
    } else if (fix.action === 'replace_in_list') {
      say(t('health.replaced', { name: nameOf(fix.mod_id), entry: fix.entry }));
    }
  };

  const handleFixes = (list: HealthFix[], issues: HealthIssue[] = []) => {
    markPending(issues);
    let next = mods;
    let applied = 0;
    for (const fix of list) {
      const after = applyFix(next, fix);
      if (after) {
        next = after;
        applied += 1;
      }
    }
    if (next === mods) {
      say(t('health.nothingChanged'));
      return;
    }
    onApplyModList(next);
    say(t('health.changesMade', { count: applied }));
  };

  /** The name the Mod List shows for a mod id. */
  const nameOf = (id: string) => mods.find((m) => m.mod_id === id)?.name ?? id;

  /** One warning, with what to ask, for any chat assistant (pasted; no key needed). */
  const copyForAssistant = async (issue: HealthIssue) => {
    const describe = (id: string) => {
      const m = mods.find((x) => x.mod_id === id);
      if (!m) return id;
      const bits = [`${m.name} (id ${m.mod_id}`];
      if (m.version) bits.push(`version ${m.version}`);
      if (m.workshop_id) bits.push(`Workshop ${m.workshop_id}`);
      return bits.join(', ') + ')';
    };
    const lines = [
      t('assistant.healthAsk'),
      '',
      `Project Zomboid ${report?.game_version ?? ''}, ${report?.active_count ?? '?'} mods on`.trim(),
      `Mod Health: ${issue.severity} ${issue.code}`,
      issue.mod_id ? `Mod: ${describe(issue.mod_id)}` : '',
      issue.related.length ? `Also involved: ${issue.related.map(describe).join('; ')}` : '',
      '',
      issue.message,
      issueText(issue) !== issue.message ? `(${issueText(issue)})` : '',
      issue.fixes.length ? `\nPZ Mod Studio offers: ${issue.fixes.map((f) => f.action).join(', ')}` : '',
    ].filter((l, i, all) => l !== '' || (i > 0 && all[i - 1] !== ''));
    await navigator.clipboard.writeText(lines.join('\n'));
    say(t('assistant.copied'));
  };
  /** Names and ids a message may mention, set in bold. */
  const modWords = useMemo(() => mods.flatMap((m) => [m.name, m.mod_id]), [mods]);

  const safeFixes = useMemo(() => {
    if (!report) return [] as HealthFix[];
    return report.issues
      .filter((i) => !YOUR_CALL_CODES.has(i.code))
      .flatMap((i) => i.fixes.filter((f) => SAFE_FIX_ACTIONS.has(f.action)));
  }, [report]);

  const handleFixAllSafe = () => {
    const enabling = safeFixes.filter((f) => f.action === 'enable_mod').length;
    const moving = safeFixes.filter((f) => f.action === 'move_before').length;
    const parts = [enabling > 0 && t('health.willEnable', { count: enabling }), moving > 0 && t('health.willMove', { count: moving })].filter(Boolean);
    if (!confirm(t('health.confirmFixAll', { what: parts.join(t('health.and')) }))) return;
    handleFixes(
      safeFixes,
      report ? report.issues.filter((i) => !YOUR_CALL_CODES.has(i.code) && i.fixes.some((f) => SAFE_FIX_ACTIONS.has(f.action))) : []
    );
  };

  // Kept apart from the list fixes: merging rewrites the mod list on its own (enables the package).
  const mergeablePaths = useMemo(() => {
    if (!report) return [] as string[];
    const all = report.issues.flatMap((i) => i.fixes.flatMap((f) => (f.action === 'merge_files' ? f.relative_paths : [])));
    return Array.from(new Set(all));
  }, [report]);
  const safeIssueCount =
    report?.issues.filter(
      (i) => !YOUR_CALL_CODES.has(i.code) && i.fixes.some((f) => SAFE_FIX_ACTIONS.has(f.action))
    ).length ?? 0;

  const tabOf = (i: HealthIssue): ListTab =>
    i.code.startsWith('security_') ? 'security' : CONFLICT_CODE.test(i.code) || YOUR_CALL_CODES.has(i.code) ? 'conflicts' : 'problems';
  const byTab = useMemo(() => {
    const out: Record<ListTab, HealthIssue[]> = { problems: [], conflicts: [], security: [] };
    report?.issues.forEach((i) => out[tabOf(i)].push(i));
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [report]);
  // The front page (the Doctor) on entering, unless another screen sent the player to a tab.
  const [tab, setTab] = useState<HealthTab>(() => focusTab ?? 'home');
  const section = sectionOf(tab);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>(() => {
    try {
      return JSON.parse(localStorage.getItem(COLLAPSED_KEY) ?? '{}') as Record<string, boolean>;
    } catch {
      return {};
    }
  });
  /** What the Studio already fixed and what is only worth knowing start folded: nothing to do there. */
  const isCollapsed = (key: string, state: IssueState) => collapsed[key] ?? (state === 'handled' || state === 'info');
  const toggleSection = (key: string, state: IssueState) => {
    setCollapsed((prev) => {
      const next = { ...prev, [key]: !(prev[key] ?? (state === 'handled' || state === 'info')) };
      try {
        localStorage.setItem(COLLAPSED_KEY, JSON.stringify(next));
      } catch {
        // only a convenience
      }
      return next;
    });
  };
  const pickTab = (next: HealthTab) => {
    setTab(next);
    const sec = sectionOf(next);
    if (sec === 'home') return;
    try {
      localStorage.setItem(LAST_TAB_KEY + sec, next);
    } catch {
      // only a convenience
    }
  };
  /** A part opens on the tab it was left on. */
  const openSection = (sec: DiagSection | 'home') => {
    if (sec === 'home') return pickTab('home');
    let last: HealthTab | null = null;
    try {
      last = localStorage.getItem(LAST_TAB_KEY + sec) as HealthTab | null;
    } catch {
      // no storage: the part's first tab
    }
    pickTab(last && SECTION_TABS[sec].includes(last) ? last : SECTION_TABS[sec][0]);
  };
  useEffect(() => {
    if (!focusTab) return;
    pickTab(focusTab);
    onFocusTabHandled?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusTab]);
  // A Mod List chip points at one issue: its tab first, and its section open, so the row is there
  // to scroll to (not saved: the player's own folding stays as it was).
  useEffect(() => {
    if (!target) return;
    const focusTab = tabOf(target);
    setTab(focusTab);
    setCollapsed((prev) => ({ ...prev, [`${focusTab}:${issueStateOf(target)}`]: false }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target]);
  /** The whole list in one line, and the one thing to do now. */
  const verdict = useMemo(() => (report ? healthVerdict(report.issues) : null), [report]);
  /** What wants something done on a tab: the rows that act or are worth acting on. */
  const toDo = (issues: HealthIssue[]) => issues.filter((i) => ['act', 'recommended'].includes(issueStateOf(i)));
  const actCount = (issues: HealthIssue[]) => issues.filter((i) => issueStateOf(i) === 'act').length;
  const tabCount: Partial<Record<HealthTab, number>> = {
    problems: toDo(byTab.problems).length,
    conflicts: toDo(byTab.conflicts).length,
    security: toDo(byTab.security).length,
  };

  /**
   * The rows of the tab in front, in sections by state, most pressing first: act, worth doing,
   * worth knowing, already fixed by the Studio. The same four on every tab.
   */
  const sections = useMemo(() => {
    if (!isListTab(tab)) return [];
    const by: Record<IssueState, HealthIssue[]> = { act: [], recommended: [], info: [], handled: [] };
    byTab[tab].forEach((i) => by[issueStateOf(i)].push(i));
    // Inside a section, what can be done with a button and is sure first; a suspicion with nothing to press last.
    return STATE_ORDER.map((state) => {
      const issues = inDoFirstOrder(by[state]);
      return { key: state, state, issues, items: groupItems(issues) };
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [byTab, tab, i18next.language]);

  /** The fixes a row offers, as buttons; "Enable X" + "Move X before Y" are one button doing both. */
  const fixesOf = (issue: HealthIssue) => {
    // On a server's list, only what acts on files: its Mods= is changed in Servers.
    const applicable = issue.fixes.filter((f) => fixActsOn(f.action, listTarget) && (ACTION_FIXES.has(f.action) || applyFix(mods, f) !== null));
    // A Java mod ZombieBuddy left out goes off with the mods that require it: one button for all.
    const offWithUsers = issue.code === 'zb_excluded' && applicable.length > 1 && applicable.every((f) => f.action === 'disable_mod');
    const enableThenMove = offWithUsers || (applicable.length === 2 && applicable[0].action === 'enable_mod' && applicable[1].action === 'move_before');
    return { applicable, enableThenMove, fixes: enableThenMove ? [applicable[0]] : applicable };
  };
  const labelOf = (issue: HealthIssue, fix: HealthFix) =>
    YOUR_CALL_CODES.has(issue.code) && fix.action === 'move_before' ? t('health.useVersionInstead', { name: nameOf(fix.before) })
      : issue.code === 'two_mod_managers' && fix.action === 'disable_mod' ? t('health.fix.useStudioScreen')
      : fix.action === 'replace_in_list' && nameOf(fix.mod_id) !== fix.mod_id ? t('health.fix.replaceNamed', { name: nameOf(fix.mod_id) })
      : issue.code === 'zb_excluded' && fix.action === 'disable_mod' && issue.fixes.length > 1 ? t('health.fix.offWithUsers', { name: nameOf(fix.mod_id), count: issue.fixes.length - 1 })
      : fixLabel(fix);

  /**
   * One row: the state in words, the mod, what is going on, what the first button changes (said
   * before it is pressed) and the buttons. On a row the Studio already fixed, a button only undoes
   * what it did: quiet, and called so.
   */
  const renderIssue = (issue: HealthIssue, state: IssueState, key: string) => {
    const style = STATE_STYLE[state];
    const Icon = style.icon;
    const { applicable, enableThenMove, fixes } = fixesOf(issue);
    const undoOnly = state === 'handled' || state === 'info';
    // On a mod the player is making, "switch it off" is never the thing to do first: it stays, quiet.
    const primary = fixes.find((f) => !(undoOnly && UNDO_ACTIONS.has(f.action)) && !(issue.own_mod && f.action === 'disable_mod'));
    return (
      <div
        key={key}
        data-issue-key={issueKey(issue)}
        data-issue-code={`${issue.code}|${issue.mod_id ?? ''}`}
        className={`flex items-start gap-3 p-3 rounded-lg border ${style.card} ${litKey === issueKey(issue) ? 'ring-2 ring-cyan-400 shadow-lg shadow-cyan-900/50' : ''} ${isPending(issue) ? 'opacity-60' : ''}`}
      >
        <Icon className={`w-4 h-4 mt-0.5 shrink-0 ${style.iconColor}`} aria-hidden="true" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 min-w-0">
            <span className={`shrink-0 text-[10px] font-bold uppercase tracking-wide ${style.label}`}>{t(`health.state.${state}.word`)}</span>
            {scopeChip(issue.code, issue.own_mod)}
            {certaintyChip(issue)}
            {issue.mod_id && (
              <span className="text-[12.5px] font-bold text-slate-100 truncate">
                {nameOf(issue.mod_id)}
                {nameOf(issue.mod_id) !== issue.mod_id && <span className="ml-1.5 text-[10px] font-mono font-normal text-slate-500">{issue.mod_id}</span>}
              </span>
            )}
          </div>
          <RichText text={issueText(issue)} mods={modWords} foldAt={320} className="text-xs text-slate-300 leading-relaxed" />
          {primary && (
            <div className="flex items-start gap-1.5 mt-1 text-[11px] text-slate-400">
              <ArrowRight className="w-3 h-3 mt-0.5 shrink-0" aria-hidden="true" />
              <span>{t(effectKey(primary))}</span>
            </div>
          )}
          <div className="flex items-center gap-2 mt-0.5">
            <span className="text-[10px] font-mono text-slate-600">{issue.code}</span>
            <button
              type="button"
              onClick={() => void copyForAssistant(issue)}
              className="flex items-center gap-1 text-[10px] text-slate-500 hover:text-slate-200 cursor-pointer"
              title={t('assistant.copyTitle')}
            >
              <MessageSquare className="w-3 h-3" aria-hidden="true" /> {t('assistant.copy')}
            </button>
          </div>
          {issue.details && issue.details.length > 0 && (
            <details className="mt-1.5">
              <summary className="text-[11px] text-sky-300 cursor-pointer select-none">{t('health.byFile', { count: issue.details.length })}</summary>
              <div className="mt-1.5 flex flex-col gap-1.5">
                {issue.details.map((d) => (
                  <div key={`${d.mod_id}:${d.file}`} className="rounded border border-slate-800 bg-slate-950/60 px-2 py-1.5">
                    <div className="text-[11px] text-slate-300">
                      <b className="text-slate-100">{d.mod_name}</b> <span className="font-mono text-slate-400">{d.file}</span>
                    </div>
                    <div className="flex flex-wrap gap-1 mt-1">
                      {d.declares.map((thing) => (
                        <button
                          key={thing.definition}
                          type="button"
                          onClick={() => onOpenMerger(thing.merged_path)}
                          title={`${thing.definition}: ${thing.state === 'merged' ? t('health.thingMerged') : thing.state === 'stale' ? t('health.thingStale') : t('health.thingPending')}${t('health.thingClick')}`}
                          className={`px-1.5 py-0.5 rounded text-[10px] font-mono border cursor-pointer ${
                            thing.state === 'merged'
                              ? 'border-emerald-800 text-emerald-300 bg-emerald-950/40'
                              : thing.state === 'stale'
                              ? 'border-amber-800 text-amber-300 bg-amber-950/40'
                              : 'border-slate-700 text-slate-300 hover:bg-slate-800'
                          }`}
                        >
                          {thing.definition}
                        </button>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </details>
          )}
        </div>
        {onServer && issue.code === 'missing_mod' && issue.mod_id && (
          <button
            type="button"
            onClick={() => takeOffServer(issue.mod_id!)}
            disabled={serverBusy}
            className="shrink-0 px-2 py-1 rounded text-[11px] font-bold bg-sky-700 hover:bg-sky-600 text-white border border-sky-600 cursor-pointer disabled:opacity-60"
          >
            {t('health.list.takeOff')}
          </button>
        )}
        {isPending(issue) ? (
          pendingChip
        ) : fixes.length > 0 && (
          <div className="flex flex-wrap justify-end gap-1.5 shrink-0 max-w-[45%]">
            {fixes.map((fix, i) => {
              const quiet = undoOnly && UNDO_ACTIONS.has(fix.action);
              return (
                <button
                  key={i}
                  type="button"
                  onClick={() => (enableThenMove ? handleFixes(applicable, [issue]) : handleFix(fix, issue))}
                  disabled={fix.action === 'merge_files' && mergeBusy}
                  title={t(effectKey(fix))}
                  className={
                    quiet
                      ? 'px-2 py-1 rounded text-[11px] text-slate-400 hover:text-slate-200 border border-slate-700/70 hover:bg-slate-800/60 cursor-pointer'
                      : fix === primary && (state === 'act' || state === 'recommended')
                      ? 'px-2 py-1 rounded text-[11px] font-bold bg-sky-700 hover:bg-sky-600 text-white border border-sky-600 cursor-pointer'
                      : 'px-2 py-1 rounded text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-100 border border-slate-700 cursor-pointer'
                  }
                >
                  {quiet ? t('health.undoPrefix', { what: labelOf(issue, fix) }) : labelOf(issue, fix)}
                </button>
              );
            })}
          </div>
        )}
      </div>
    );
  };

  /**
   * Several rows of one code that are the same thing for several mods: one card with their names
   * and one button for all (copies the originals no longer need), or the list with an undo each
   * (copies the Studio keeps running).
   */
  const renderGroup = (code: string, issues: HealthIssue[], state: IssueState, key: string) => {
    const style = STATE_STYLE[state];
    const Icon = style.icon;
    if (code === 'community_known_issue') {
      // One mod's known issues: what each one is, with its own "no longer happens", and the fix
      // they share once (a compatible copy fixes them all).
      const shared: HealthFix[] = [];
      issues.forEach((i) =>
        fixesOf(i).fixes.forEach((f) => {
          if (f.action !== 'dismiss_known_issue' && !shared.some((s) => JSON.stringify(s) === JSON.stringify(f))) shared.push(f);
        })
      );
      const modId = issues[0].mod_id ?? '';
      return (
        <div
          key={key}
          className={`flex items-start gap-3 p-3 rounded-lg border ${style.card} ${issues.some((i) => litKey === issueKey(i)) ? 'ring-2 ring-cyan-400' : ''}`}
        >
          <Icon className={`w-4 h-4 mt-0.5 shrink-0 ${style.iconColor}`} aria-hidden="true" />
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 min-w-0">
              <span className={`shrink-0 text-[10px] font-bold uppercase tracking-wide ${style.label}`}>{t(`health.state.${state}.word`)}</span>
              {scopeChip(code, issues.every((i) => i.own_mod))}
              <span className="text-[12.5px] font-bold text-slate-100 truncate">{nameOf(modId)}</span>
            </div>
            <div className="text-xs text-slate-300">{t('health.group.community_known_issue.title', { count: issues.length })}</div>
            <ul className="mt-1 flex flex-col gap-1">
              {issues.map((i, n) => {
                const dismiss = i.fixes.find((f) => f.action === 'dismiss_known_issue');
                return (
                  <li key={n} data-issue-key={issueKey(i)} data-issue-code={`${i.code}|${i.mod_id ?? ''}`} className="flex items-start justify-between gap-2 rounded border border-slate-800/80 bg-slate-950/40 px-2 py-1">
                    <RichText text={issueText(i)} mods={modWords} foldAt={200} className="text-[11px] text-slate-300 leading-relaxed min-w-0" />
                    {isPending(i) ? (
                      pendingChip
                    ) : dismiss && (
                      <button
                        type="button"
                        onClick={() => handleFix(dismiss, i)}
                        title={t(effectKey(dismiss))}
                        className="px-2 py-0.5 rounded text-[10px] text-slate-400 hover:text-slate-200 border border-slate-700/70 hover:bg-slate-800/60 cursor-pointer shrink-0"
                      >
                        {fixLabel(dismiss)}
                      </button>
                    )}
                  </li>
                );
              })}
            </ul>
            {shared[0] && (
              <div className="flex items-start gap-1.5 mt-1 text-[11px] text-slate-400">
                <ArrowRight className="w-3 h-3 mt-0.5 shrink-0" aria-hidden="true" />
                <span>{t(effectKey(shared[0]))}</span>
              </div>
            )}
          </div>
          {issues.some(isPending) ? (
            pendingChip
          ) : shared.length > 0 && (
            <div className="flex flex-wrap justify-end gap-1.5 shrink-0 max-w-[45%]">
              {shared.map((fix, n) => (
                <button
                  key={n}
                  type="button"
                  onClick={() => handleFix(fix, issues[0])}
                  title={t(effectKey(fix))}
                  className={
                    n === 0
                      ? 'px-2 py-1 rounded text-[11px] font-bold bg-sky-700 hover:bg-sky-600 text-white border border-sky-600 cursor-pointer'
                      : 'px-2 py-1 rounded text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-100 border border-slate-700 cursor-pointer'
                  }
                >
                  {labelOf(issues[0], fix)}
                </button>
              ))}
            </div>
          )}
        </div>
      );
    }
    if (code === 'java_patch_overlap') {
      // Where the Java of several mods changes the same class of the game: what each row says,
      // folded, under one sentence that says what an overlap is and is not.
      const kind = issues[0].params?.variant === 'class' ? 'class' : 'method';
      return (
        <div key={key} className={`flex items-start gap-3 p-3 rounded-lg border ${style.card} ${issues.some((i) => litKey === issueKey(i)) ? 'ring-2 ring-cyan-400' : ''}`}>
          <Icon className={`w-4 h-4 mt-0.5 shrink-0 ${style.iconColor}`} aria-hidden="true" />
          <div className="flex-1 min-w-0">
            <span className={`text-[10px] font-bold uppercase tracking-wide ${style.label}`}>{t(`health.state.${state}.word`)}</span> {scopeChip(code)}
            <div className="text-[12.5px] font-bold text-slate-100">{t(`health.group.java_patch_overlap.${kind}Title`, { count: issues.length })}</div>
            <div className="text-xs text-slate-300 leading-relaxed">{t(`health.group.java_patch_overlap.${kind}Body`)}</div>
            <details className="mt-1.5">
              <summary className="text-[11px] text-sky-300 hover:text-sky-200 cursor-pointer select-none">{t('health.group.java_patch_overlap.show', { count: issues.length })}</summary>
              <ul className="mt-1.5 flex flex-col gap-1">
                {issues.map((i, n) => (
                  <li key={n} data-issue-key={issueKey(i)} data-issue-code={`${i.code}|${i.mod_id ?? ''}`} className="rounded border border-slate-800/80 bg-slate-950/40 px-2 py-1">
                    <div className="text-[11px] font-mono text-slate-200 break-all">
                      {i.params?.class}
                      {i.params?.methods ? <span className="text-slate-400"> · {i.params.methods}</span> : null}
                    </div>
                    <div className="text-[11px] text-slate-400 leading-relaxed">
                      {kind === 'class' ? i.params?.detail : i.params?.mods}
                      {i.params?.replaceKey ? t('backendText.fragments.java_patch_replace') : ''}
                      {i.params?.byCodeKey ? t('backendText.fragments.java_patch_by_code') : ''}
                    </div>
                  </li>
                ))}
              </ul>
            </details>
          </div>
        </div>
      );
    }
    // The one action the whole card takes, on every mod in it: the copies the originals no longer
    // need go, the copies made by hand are kept by the Studio, the outgrown ones copied again.
    const bulkAction: HealthFix['action'] | null =
      code === 'compatible_copy_unneeded' ? 'remove_compatible_copy' : code === 'workshop_copy_by_hand' ? 'adopt_workshop_copy' : code === 'workshop_copy_stale' ? 'recopy_workshop' : null;
    const bulkFixes = bulkAction ? issues.flatMap((i) => i.fixes.filter((f) => f.action === bulkAction)) : [];
    const folders = bulkFixes.flatMap((f) => (f.action === 'remove_compatible_copy' || f.action === 'adopt_workshop_copy' ? [f.folder] : []));
    const runBulk = () => {
      markPending(issues);
      if (bulkAction === 'remove_compatible_copy') void removeCompatibleCopies(folders);
      else if (bulkAction === 'adopt_workshop_copy') void adoptCopies(folders);
      else if (bulkAction === 'recopy_workshop') void recopyAll(bulkFixes.flatMap((f) => (f.action === 'recopy_workshop' ? [f.mod_id] : [])));
    };
    const firstFix = bulkFixes[0];
    const bulk = bulkAction !== null;
    return (
      <div
        key={key}
        data-issue-key={bulk ? issueKey(issues[0]) : undefined}
        className={`flex items-start gap-3 p-3 rounded-lg border ${style.card} ${bulk && issues.some((i) => litKey === issueKey(i)) ? 'ring-2 ring-cyan-400' : ''}`}
      >
        <Icon className={`w-4 h-4 mt-0.5 shrink-0 ${style.iconColor}`} aria-hidden="true" />
        <div className="flex-1 min-w-0">
          <span className={`text-[10px] font-bold uppercase tracking-wide ${style.label}`}>{t(`health.state.${state}.word`)}</span> {scopeChip(code)}
          <div className="text-[12.5px] font-bold text-slate-100">{t(`health.group.${code}.title`, { count: issues.length })}</div>
          <div className="text-xs text-slate-300 leading-relaxed">{t(`health.group.${code}.body`)}</div>
          {bulk ? (
            <>
              <div className="text-xs text-slate-200 mt-1">
                {issues.slice(0, 15).map((i) => nameOf(i.mod_id ?? '')).join(', ')}
                {issues.length > 15 && <span className="text-slate-400">{t('health.group.andMore', { count: issues.length - 15 })}</span>}
              </div>
              {(() => {
                const servers = Array.from(new Set(issues.flatMap((i) => (i.params?.servers ? i.params.servers.split(', ') : []))));
                return servers.length > 0 ? <div className="text-[11px] text-amber-200/90 mt-1">{t('health.group.usedByServers', { servers: servers.join(', ') })}</div> : null;
              })()}
            </>
          ) : (
            <ul className="mt-1.5 flex flex-col gap-1">
              {issues.map((i, n) => {
                const fix = i.fixes.find((f) => f.action === 'remove_compatible_copy');
                return (
                  <li
                    key={`${i.mod_id}-${n}`}
                    data-issue-key={issueKey(i)}
                    data-issue-code={`${i.code}|${i.mod_id ?? ''}`}
                    className={`flex items-center justify-between gap-2 text-xs text-slate-200 rounded px-1 ${litKey === issueKey(i) ? 'ring-2 ring-cyan-400' : ''}`}
                  >
                    <span className="truncate">{nameOf(i.mod_id ?? '')}</span>
                    {i.code === 'compatible_copy' && i.mod_id && !i.own_mod && (
                      <button
                        type="button"
                        onClick={() => void copyAuthorSuggestion(i.mod_id!)}
                        title={t('health.authorNote.title')}
                        className="ml-auto px-2 py-0.5 rounded text-[10px] text-sky-300 hover:text-sky-100 border border-sky-800/70 hover:bg-sky-900/40 cursor-pointer shrink-0"
                      >
                        {t('health.authorNote.button')}
                      </button>
                    )}
                    {isPending(i) ? (
                      pendingChip
                    ) : fix && (
                      <button
                        type="button"
                        onClick={() => handleFix(fix, i)}
                        title={t(effectKey(fix))}
                        className="px-2 py-0.5 rounded text-[10px] text-slate-400 hover:text-slate-200 border border-slate-700/70 hover:bg-slate-800/60 cursor-pointer shrink-0"
                      >
                        {t('health.undoPrefix', { what: fixLabel(fix) })}
                      </button>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
          {bulk && firstFix && (
            <div className="flex items-start gap-1.5 mt-1 text-[11px] text-slate-400">
              <ArrowRight className="w-3 h-3 mt-0.5 shrink-0" aria-hidden="true" />
              <span>{t(effectKey(firstFix))}</span>
            </div>
          )}
        </div>
        {bulk && bulkFixes.length > 0 && (
          <button
            type="button"
            onClick={runBulk}
            disabled={compatBusy}
            className="px-2 py-1 rounded text-[11px] font-bold bg-sky-700 hover:bg-sky-600 text-white border border-sky-600 cursor-pointer shrink-0 disabled:opacity-60"
          >
            {t(`health.group.${code}.button`, { count: bulkFixes.length })}
          </button>
        )}
      </div>
    );
  };

  if (!paths.is_valid) {
    return (
      <div className="flex-1 flex items-center justify-center text-sm text-slate-400 p-6">
        {t('health.configureFirst')}
      </div>
    );
  }

  /** The tabs of the part in front (none on the front page). */
  const renderSubTabs = () => (
  <div role="tablist" className="flex items-end gap-1 border-b border-slate-800">
    {(section === 'home' ? [] : SECTION_TABS[section]).map((id) => {
      const Icon = TAB_ICON[id];
      const count = tabCount[id];
      const active = tab === id;
      const issuesHere = id === 'problems' || id === 'conflicts' || id === 'security' ? byTab[id] : [];
      const badge = actCount(issuesHere) > 0 ? STATE_STYLE.act.badge : STATE_STYLE.recommended.badge;
      return (
        <button
          key={id}
          type="button"
          role="tab"
          aria-selected={active}
          onClick={() => pickTab(id)}
          className={`flex items-center gap-2 px-4 py-2 text-xs font-bold rounded-t-lg border border-b-0 -mb-px cursor-pointer ${
            active ? 'bg-slate-900 border-slate-700 text-slate-100' : 'border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <Icon className="w-3.5 h-3.5" aria-hidden="true" />
          {t(`health.tabs.${id}`)}
          {!!count && <span className={`px-1.5 rounded-full text-[10px] font-mono ${badge}`}>{count}</span>}
        </button>
      );
    })}
  </div>
  );

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-y-auto p-6 gap-4 font-sans">
      <div className="rounded-xl border border-slate-800 bg-slate-900/60 overflow-hidden">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-2.5">
          <HeartPulse className="w-5 h-5 text-emerald-400 shrink-0" aria-hidden="true" />
          <h1 className="text-base font-bold text-slate-100" title={t('health.blurb')}>{t('health.title')}</h1>
              {report && (
                <div className="flex flex-wrap gap-1.5 text-[11px] font-mono">
                  <span className="px-2 py-0.5 rounded border border-slate-700 text-slate-300">
                    {t('health.gameBuild', { build: report.game_version ?? t('health.unknown') })}
                  </span>
                  <span className="px-2 py-0.5 rounded border border-slate-700 text-slate-300">{t('health.activeCount', { count: report.active_count })}</span>
                  {report.community && report.community.mods > 0 && (
                    <span
                      className="px-2 py-0.5 rounded border border-slate-700 text-slate-300"
                      title={t('health.communityTitle', { rules: report.community.rules, pairs: report.community.pairs, mods: report.community.mods, updated: report.community.index_updated ? t('health.indexUpdated', { date: report.community.index_updated }) : '' })}
                    >
                      {t('health.community', { rules: report.community.rules + report.community.pairs, mods: report.community.mods })}
                    </span>
                  )}
                  {!!report.community?.dismissed && (
                    <button
                      type="button"
                      onClick={() => {
                        void (async () => {
                          try {
                            const n = await TauriService.restoreIndexIssues(paths);
                            void loadCommunityIndex(paths, true);
                            say(t('health.dismissedRestored', { count: n }));
                            onRecheck();
                          } catch {
                            // Already shown by TauriService.
                          }
                        })();
                      }}
                      title={t('health.dismissedTitle')}
                      className="px-2 py-0.5 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer"
                    >
                      {t('health.dismissedCount', { count: report.community.dismissed })}
                    </button>
                  )}
                </div>
              )}

          <div className="ml-auto flex items-center gap-2 min-w-0">
            <span role="status" className={`text-[11px] truncate ${checking ? 'text-sky-200' : changesSaid ? 'text-amber-200' : 'text-slate-400'}`} title={checking ? t(`health.checking.${checkStage ?? 'full'}`) : undefined}>
              {checking
                ? t(`health.checking.short.${checkStage ?? 'full'}`)
                : changesSaid
                  ? t('health.lastCheck.changed', { when: checkedWhen, what: changesSaid })
                  : checkedWhen
                    ? t('health.lastCheck.same', { when: checkedWhen })
                    : null}
            </span>
            <button
              type="button"
              onClick={onRecheck}
              disabled={checking}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold cursor-pointer disabled:opacity-60 shrink-0 ${
                changesSaid && !checking ? 'bg-sky-700 hover:bg-sky-600 text-white' : 'bg-slate-800 hover:bg-slate-700 text-slate-200'
              }`}
            >
              {checking ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <RefreshCw className="w-3.5 h-3.5" aria-hidden="true" />}
              {checking ? t('health.checking.button') : t('health.checkAgain')}
            </button>
          </div>
        </div>
        <div className="h-0.5 bg-transparent overflow-hidden" aria-hidden="true">
          {checking && <div className="h-full w-1/3 rounded-full bg-sky-400 animate-indeterminate" />}
        </div>
      </div>

      {/* The parts: front page, mods, the game, the server */}
      <div role="tablist" className="flex flex-wrap items-center gap-1.5">
        {(['home', ...SECTIONS] as const).map((sec) => {
          const Icon = SECTION_ICON[sec];
          const active = section === sec;
          const todo = sec === 'mods' ? toDo(report?.issues ?? []).length : 0;
          return (
            <button
              key={sec}
              type="button"
              role="tab"
              aria-selected={active}
              onClick={() => openSection(sec)}
              className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-bold border cursor-pointer transition ${
                active ? 'bg-sky-900/40 border-sky-600 text-sky-100' : 'border-slate-800 bg-slate-900/50 text-slate-400 hover:text-slate-200 hover:border-slate-600'
              }`}
            >
              <Icon className="w-4 h-4" aria-hidden="true" />
              {t(`diag.sections.${sec}`)}
              {todo > 0 && <span className={`px-1.5 rounded-full text-[10px] font-mono ${actCount(report?.issues ?? []) > 0 ? STATE_STYLE.act.badge : STATE_STYLE.recommended.badge}`}>{todo}</span>}
            </button>
          );
        })}
      </div>

      {section === 'home' && <DiagnosticsHome paths={paths} report={report} checking={checking} onOpen={openSection} doctor={doctor} />}

      {section === 'mods' && (
        <>
      {servers.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5 text-[11px]">
          <span className="text-slate-400 mr-1">{t('health.list.checking')}</span>
          {[{ kind: 'profile' } as HealthListTarget, ...servers.map((s) => ({ kind: 'server', name: s.name }) as HealthListTarget)].map((target) => {
            const on = target.kind === listTarget.kind && (target.kind === 'profile' || (listTarget.kind === 'server' && listTarget.name === target.name));
            return (
              <button
                key={target.kind === 'profile' ? '-profile' : target.name}
                type="button"
                onClick={() => setListTarget(target)}
                className={`px-2.5 py-1 rounded-full border cursor-pointer ${on ? 'bg-sky-800 border-sky-600 text-white font-bold' : 'border-slate-700 text-slate-300 hover:bg-slate-800'}`}
              >
                {target.kind === 'profile' ? t('health.list.profile', { name: profileName ?? '-' }) : t('health.list.server', { name: target.name })}
              </button>
            );
          })}
        </div>
      )}
      {onServer && (
        <div className="rounded-lg border border-slate-700 bg-slate-900/60 px-3 py-2 text-[11px] text-slate-300 leading-relaxed">
          {t('health.list.serverNote', { name: listTarget.name, count: serverIds?.length ?? 0 })}
          {serverDiff && (
            <div className="mt-2 flex flex-wrap items-start gap-2">
              <div className="flex-1 min-w-[240px]">
                {serverDiff.onlyProfile.length === 0 && serverDiff.onlyServer.length === 0 ? (
                  <span className="text-emerald-300">{t('health.list.same', { profileName: profileName ?? '-' })}</span>
                ) : (
                  <>
                    <div className="text-amber-200">{t('health.list.differs', { profileName: profileName ?? '-', onlyProfile: serverDiff.onlyProfile.length, onlyServer: serverDiff.onlyServer.length })}</div>
                    {serverDiff.onlyServer.length > 0 && (
                      <div className="text-slate-400 mt-0.5">{t('health.list.onlyServer', { names: serverDiff.onlyServer.slice(0, 10).map(nameOf).join(', ') + (serverDiff.onlyServer.length > 10 ? '…' : '') })}</div>
                    )}
                    {serverDiff.onlyProfile.length > 0 && (
                      <div className="text-slate-400 mt-0.5">{t('health.list.onlyProfile', { names: serverDiff.onlyProfile.slice(0, 10).map(nameOf).join(', ') + (serverDiff.onlyProfile.length > 10 ? '…' : '') })}</div>
                    )}
                  </>
                )}
              </div>
              {(serverDiff.onlyProfile.length > 0 || serverDiff.onlyServer.length > 0) && (
                <button
                  type="button"
                  onClick={writeProfileToServer}
                  disabled={serverBusy}
                  className="shrink-0 flex items-center gap-1.5 px-2.5 py-1 rounded text-[11px] font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer disabled:opacity-60"
                >
                  {serverBusy && <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" />}
                  {t('health.list.writeProfile', { profileName: profileName ?? '-' })}
                </button>
              )}
            </div>
          )}
        </div>
      )}


      {verdict && (
        <div
          role="status"
          className={`rounded-lg border px-4 py-2 ${
            verdict.level === 'act' ? STATE_STYLE.act.card : verdict.level === 'recommended' ? STATE_STYLE.recommended.card : STATE_STYLE.handled.card
          }`}
        >
          {(() => {
            const Icon = verdict.level === 'ok' ? CheckCircle2 : STATE_STYLE[verdict.level].icon;
            const color = verdict.level === 'ok' ? STATE_STYLE.handled.iconColor : STATE_STYLE[verdict.level].iconColor;
            const { act, recommended, info, handled } = verdict.counts;
            return (
              <>
                <div className="flex items-center gap-2">
                  <Icon className={`w-5 h-5 shrink-0 ${color}`} aria-hidden="true" />
                  <span className="text-sm font-bold text-slate-100">{t(`health.verdict.${verdict.level}`, { count: verdict.level === 'act' ? act : recommended })}</span>
                </div>
                <div className="mt-0.5 ml-7 text-[11px] text-slate-400">
                  {[
                    act > 0 && t('health.verdict.countAct', { count: act }),
                    recommended > 0 && t('health.verdict.countRecommended', { count: recommended }),
                    info > 0 && t('health.verdict.countInfo', { count: info }),
                    handled > 0 && t('health.verdict.countHandled', { count: handled }),
                  ]
                    .filter(Boolean)
                    .join(' · ') || t('health.verdict.nothingFound')}
                </div>
                {(report?.incomplete ?? []).length > 0 && (
                  <div className="mt-1 ml-7 flex items-center gap-1.5 text-[11px] text-sky-200">
                    <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" />
                    {t('health.verdict.incomplete')}
                  </div>
                )}
                {verdict.next && (
                  <div className="mt-2 ml-7 flex items-start gap-2">
                    <ArrowRight className="w-3.5 h-3.5 mt-0.5 shrink-0 text-slate-300" aria-hidden="true" />
                    <div className="flex-1 min-w-0 text-xs text-slate-200">
                      <span className="font-bold">{t('health.verdict.next')}</span>{' '}
                      {verdict.next.mod_id && <b className="text-slate-100">{nameOf(verdict.next.mod_id)}: </b>}
                      <span className="text-slate-300 line-clamp-1" title={issueText(verdict.next)}>{issueText(verdict.next)}</span>
                    </div>
                    <button
                      type="button"
                      onClick={() => setOwnFocus(verdict.next)}
                      className="shrink-0 px-2.5 py-1 rounded text-[11px] font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer"
                    >
                      {t('health.verdict.go')}
                    </button>
                  </div>
                )}
              </>
            );
          })()}
        </div>
      )}

      {!onServer && variantsOn.off.length > 0 && (
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-amber-800/60 bg-amber-950/20 px-3 py-2 text-[11px] text-amber-100">
          <span className="flex-1 min-w-[240px]">{t('health.variants.line', { count: variantsOn.off.length, items: variantsOn.choices.length })}</span>
          <button type="button" onClick={keepOneVariant} className="px-2.5 py-1 rounded font-bold bg-amber-700 hover:bg-amber-600 text-white cursor-pointer">
            {t('health.variants.button')}
          </button>
        </div>
      )}

      {!onServer && profileId && (
        <ProfilePhotos paths={paths} profile={{ id: profileId, name: profileName ?? profileId }} mods={mods} catalog={catalog} report={report} nameOf={nameOf} />
      )}

      <PageGuide page="health" />

      {renderSubTabs()}

      {isListTab(tab) && (
        <div className="flex items-center justify-between gap-3">
          <p className="text-xs text-slate-400 max-w-3xl">{t(`health.tabs.${tab}Intro`)}</p>
          {tab === 'problems' && !onServer && safeFixes.length > 0 && (
            <button
              type="button"
              onClick={handleFixAllSafe}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-bold bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer shrink-0"
              title={t('health.fixAllTitle')}
            >
              <Wand2 className="w-3.5 h-3.5" aria-hidden="true" />
              {t('health.fixAll', { count: safeIssueCount })}
            </button>
          )}
          {tab === 'conflicts' && mergeablePaths.length > 1 && (
            <button
              type="button"
              onClick={() => mergeFiles(mergeablePaths)}
              disabled={mergeBusy}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-bold bg-indigo-600 hover:bg-indigo-500 text-white cursor-pointer disabled:opacity-60 shrink-0"
              title={t('health.mergeAllTitle')}
            >
              <Wand2 className="w-3.5 h-3.5" aria-hidden="true" />
              {t('health.mergeAll', { count: mergeablePaths.length })}
            </button>
          )}
        </div>
      )}

      {compatBusy && (
        <div className="flex items-center gap-2 p-3 rounded-lg border border-slate-800 bg-slate-900/60 text-xs text-slate-300">
          <Loader2 className="w-4 h-4 animate-spin" aria-hidden="true" /> {t('health.makingCopy')}
        </div>
      )}
      {mergeBusy && (
        <div className="flex items-center gap-2 p-3 rounded-lg border border-slate-800 bg-slate-900/60 text-xs text-slate-300">
          <Loader2 className="w-4 h-4 animate-spin" aria-hidden="true" /> {t('health.merging')}
        </div>
      )}
      {mergeResult && (
        <div className="flex flex-col gap-2 p-4 rounded-xl border border-indigo-800/60 bg-indigo-950/20 text-xs">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-indigo-200 font-bold">
              <CheckCircle2 className="w-4 h-4 text-indigo-300" aria-hidden="true" />
              {t('health.mergedInto', { count: mergeResult.merged.length, package: mergeResult.package_folder })}
            </div>
            <button type="button" onClick={() => setMergeResult(null)} className="text-slate-500 hover:text-slate-300 cursor-pointer text-[11px]">{t('health.close')}</button>
          </div>
          {mergeResult.merged.length > 0 && (
            <ul className="list-disc pl-5 text-slate-300 space-y-0.5 font-mono">
              {mergeResult.merged.map((p) => <li key={p}>{p}</li>)}
            </ul>
          )}
          {mergeResult.skipped.length > 0 && (
            <>
              <div className="text-amber-300 font-bold mt-1">{t('health.needDecision')}</div>
              <ul className="list-disc pl-5 text-amber-200/90 space-y-0.5 font-mono">
                {mergeResult.skipped.map((p) => (
                  <li key={p}>
                    {p}{' '}
                    <button type="button" onClick={() => onOpenMerger(p)} className="underline text-amber-100 cursor-pointer">{t('health.openInMerger')}</button>
                  </li>
                ))}
              </ul>
            </>
          )}
          <div className="text-slate-400">
            {t('health.packageWins')}
          </div>
        </div>
      )}
      {compatResult && (
        <div className="flex flex-col gap-2 p-4 rounded-xl border border-emerald-800/60 bg-emerald-950/20 text-xs">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-emerald-200 font-bold">
              <CheckCircle2 className="w-4 h-4 text-emerald-400" aria-hidden="true" />
              {t('health.copyReady', { mod: compatResult.mod_id, folder: compatResult.folder })}
            </div>
            <button type="button" onClick={() => setCompatResult(null)} className="text-slate-500 hover:text-slate-300 cursor-pointer text-[11px]">{t('health.close')}</button>
          </div>
          <ul className="list-disc pl-5 text-slate-300 space-y-0.5">
            {compatResult.changes.map((c, i) => <li key={i}><RichText text={c} foldAt={240} /></li>)}
          </ul>
          {compatResult.warnings.length > 0 ? (
            <>
              <div className="text-amber-300 font-bold mt-1">{t('health.mayBreak')}</div>
              <ul className="list-disc pl-5 text-amber-200/90 space-y-0.5">
                {compatResult.warnings.map((w, i) => <li key={i}><RichText text={w} foldAt={240} /></li>)}
              </ul>
            </>
          ) : (
            <div className="text-slate-400">{t('health.noLeftovers')}</div>
          )}
          <div className="text-slate-400" dangerouslySetInnerHTML={{ __html: tHtml('health.startFromStudio') }} />
        </div>
      )}

      {done && (
        <div className="flex items-start gap-2 px-4 py-3 rounded-lg border border-emerald-700/70 bg-emerald-950/40 text-emerald-100 text-xs">
          <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" aria-hidden="true" />
          <div className="flex-1 min-w-0">
            <div className="font-bold">{done}</div>
            <div className="text-emerald-300/80">{t('health.nothingMore')}</div>
          </div>
          <button type="button" onClick={() => setDone(null)} className="text-emerald-400/70 hover:text-emerald-200 cursor-pointer" aria-label={t('health.close')}>
            <X className="w-3.5 h-3.5" aria-hidden="true" />
          </button>
        </div>
      )}

      {/* On the first tab, an "in order" verdict above already says it: the same green line twice reads as noise. */}
      {report && isListTab(tab) && toDo(byTab[tab]).length === 0 && !(tab === 'problems' && verdict?.level === 'ok') && (
        <div className="flex items-center gap-3 p-5 rounded-xl border border-emerald-800/60 bg-emerald-950/30 text-emerald-200 text-sm">
          <CheckCircle2 className="w-5 h-5 text-emerald-400" aria-hidden="true" />
          {tab === 'problems' ? t('health.noProblems', { count: report.active_count }) : t(`health.tabs.${tab}Empty`)}
        </div>
      )}

      {sections.map((sec) => {
        if (sec.issues.length === 0) return null;
        const style = STATE_STYLE[sec.state];
        const StateIcon = style.icon;
        const key = `${tab}:${sec.state}`;
        const folded = isCollapsed(key, sec.state);
        return (
          <section key={sec.key} className="flex flex-col gap-2">
            <button
              type="button"
              onClick={() => toggleSection(key, sec.state)}
              aria-expanded={!folded}
              className={`flex items-start gap-2 text-left px-3 py-2 rounded-lg border cursor-pointer ${style.card} hover:brightness-125`}
            >
              {folded ? <ChevronRight className="w-4 h-4 mt-0.5 text-slate-400 shrink-0" aria-hidden="true" /> : <ChevronDown className="w-4 h-4 mt-0.5 text-slate-400 shrink-0" aria-hidden="true" />}
              <StateIcon className={`w-4 h-4 mt-0.5 shrink-0 ${style.iconColor}`} aria-hidden="true" />
              <span className="flex flex-col min-w-0">
                <span className={`text-[13px] font-bold ${style.label}`}>
                  {t(`health.state.${sec.state}.title`)} ({sec.issues.length})
                </span>
                <span className="text-[11px] text-slate-400">{t(`health.state.${sec.state}.hint`)}</span>
              </span>
            </button>
            {!folded &&
              sec.items.map((item, idx) =>
                item.kind === 'group' ? (
                  renderGroup(item.code, item.issues, sec.state, `${item.code}-${idx}`)
                ) : (
                  renderIssue(item.issue, sec.state, `${item.issue.code}-${item.issue.mod_id}-${idx}`)
                )
              )}
          </section>
        );
      })}
      {tab === 'modtime' && <GameTimePanel paths={paths} mods={mods} onModsChanged={onModsChanged} />}
      {tab === 'audit' && report && <AuditPanel mods={mods} issues={report.issues} nameOf={nameOf} onGo={(issue) => setOwnFocus(issue)} />}
        </>
      )}

      {section === 'game' && renderSubTabs()}
      {tab === 'live' && <GameLivePanel paths={paths} />}
      {tab === 'memory' && <MemoryPanel paths={paths} />}
      {tab === 'boot' && <BootTimePanel paths={paths} />}
      {tab === 'frames' && <FramesPanel paths={paths} mods={mods} />}
      {tab === 'disk' && <DiskPanel paths={paths} onModsChanged={onModsChanged} onOpenInBrowser={onOpenInBrowser} />}
      {tab === 'server' && <ServerLivePanel paths={paths} onGoServers={onGoServers} />}
    </div>
  );
};
