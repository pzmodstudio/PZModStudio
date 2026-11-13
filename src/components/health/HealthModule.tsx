import React, { useEffect, useMemo, useRef, useState } from 'react';
import { PageGuide } from '../common/PageGuide';
import { ArrowRight, CheckCircle2, ChevronDown, ChevronRight, Gauge, GitMerge, HardDrive, HeartPulse, ListChecks, Loader2, MessageSquare, RefreshCw, ShieldCheck, Wand2, X } from 'lucide-react';
import { AutoMergeResult, CompatResult, HealthFix, HealthIssue, HealthReport, IssueState, ModInfo } from '../../types';
import { STATE_ORDER, STATE_STYLE, UNDO_ACTIONS, issueStateOf, scopeOfIssue, SCOPE_CHIP } from '../../utils/issueState';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, reportBackendError } from '../../services/tauri';
import { loadCommunityIndex } from '../../services/communityIndex';
import { applyFix } from '../../utils/healthFix';
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
  onRecheck: () => void;
  /** Replaces the whole list (order + enabled flags) and persists it. */
  onApplyModList: (mods: ModInfo[]) => void;
  /** Mods were added/removed on disk (compatible copies, patch packages): rescan the list. */
  onModsChanged: () => void;
  /** Opens the Mod Merger on this conflicting file. */
  onOpenMerger: (relativePath: string) => void;
  /** Opens a Workshop item / a Workshop search in Explore Mods. */
  onOpenInBrowser?: (workshopId: string) => void;
  onSearchInBrowser?: (text: string) => void;
  /** The issue a Mod List chip pointed at: scrolled to and lit up once. */
  focusIssue?: HealthIssue | null;
  onFocusHandled?: () => void;
}

/** Fixes handled here (not by rewriting the mod list). */
const ACTION_FIXES = new Set<HealthFix['action']>([
  'subscribe_workshop', 'open_workshop_page', 'make_compatible_copy', 'keep_version', 'remove_compatible_copy', 'merge_files', 'open_in_merger',
  'enable_shim_patch', 'remove_from_package', 'dismiss_known_issue', 'adopt_workshop_copy', 'recopy_workshop',
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
type HealthTab = 'problems' | 'conflicts' | 'security' | 'performance' | 'disk';
const TABS: HealthTab[] = ['problems', 'conflicts', 'security', 'performance', 'disk'];
const TAB_ICON: Record<HealthTab, React.ElementType> = { problems: ListChecks, conflicts: GitMerge, security: ShieldCheck, performance: Gauge, disk: HardDrive };
/** Rows of one code that are the same thing for several mods: one card with one button. */
const GROUP_CODES = new Set(['compatible_copy_unneeded', 'compatible_copy', 'workshop_copy_by_hand', 'workshop_copy_stale', 'workshop_copy_kept']);
/** What a button changes, said before it is pressed: the key under health.effect. */
const effectKey = (fix: HealthFix): string => `health.effect.${fix.action}`;
const TAB_KEY = 'pz_health_tab';
/** Performance has four panels, each a screen of its own: one at a time. */
type PerfTab = 'memory' | 'boot' | 'time' | 'frames';
const PERF_TABS: PerfTab[] = ['memory', 'boot', 'time', 'frames'];
const PERF_KEY = 'pz_health_perf_tab';
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

export const HealthModule: React.FC<HealthModuleProps> = ({ paths, mods, report, checking, onRecheck, onApplyModList, onModsChanged, onOpenMerger, onOpenInBrowser, onSearchInBrowser, focusIssue = null, onFocusHandled }) => {
  const { t } = useTranslation();
  const [litKey, setLitKey] = useState<string | null>(null);
  const issueKey = (i: HealthIssue) => `${i.code}|${i.mod_id ?? ''}|${i.message}`;
  // Opening this screen checks again, and the check lands in two steps (the rules, then the
  // script conflicts), each moving the issues around. Scrolling on every step made the page jump;
  // so it waits until the check is done and the page has kept still a moment, then scrolls once.
  const scrolledFor = useRef<HealthIssue | null>(null);
  const unlight = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(unlight.current), []);
  useEffect(() => {
    if (!focusIssue) {
      scrolledFor.current = null;
      return;
    }
    if (!report || checking || scrolledFor.current === focusIssue) return;
    const settle = window.setTimeout(() => {
      const key = issueKey(focusIssue);
      const el =
        document.querySelector<HTMLElement>(`[data-issue-key="${CSS.escape(key)}"]`) ??
        document.querySelector<HTMLElement>(`[data-issue-code="${CSS.escape(`${focusIssue.code}|${focusIssue.mod_id ?? ''}`)}"]`);
      scrolledFor.current = focusIssue;
      if (!el) {
        onFocusHandled?.();
        return;
      }
      el.scrollIntoView({ block: 'center', behavior: 'smooth' });
      setLitKey(el.dataset.issueKey ?? key);
      window.clearTimeout(unlight.current);
      unlight.current = window.setTimeout(() => {
        setLitKey(null);
        onFocusHandled?.();
      }, 4000);
    }, 250);
    return () => window.clearTimeout(settle);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusIssue, report, checking]);
  const [compatBusy, setCompatBusy] = useState(false);
  const [compatResult, setCompatResult] = useState<CompatResult | null>(null);
  const [mergeBusy, setMergeBusy] = useState(false);
  const [mergeResult, setMergeResult] = useState<AutoMergeResult | null>(null);

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

  const handleFixes = (list: HealthFix[]) => {
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
    handleFixes(safeFixes);
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

  const tabOf = (i: HealthIssue): Exclude<HealthTab, 'performance' | 'disk'> =>
    i.code.startsWith('security_') ? 'security' : CONFLICT_CODE.test(i.code) || YOUR_CALL_CODES.has(i.code) ? 'conflicts' : 'problems';
  const byTab = useMemo(() => {
    const out: Record<Exclude<HealthTab, 'performance' | 'disk'>, HealthIssue[]> = { problems: [], conflicts: [], security: [] };
    report?.issues.forEach((i) => out[tabOf(i)].push(i));
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [report]);
  const [tab, setTab] = useState<HealthTab>(() => {
    try {
      const saved = localStorage.getItem(TAB_KEY) as HealthTab | null;
      if (saved && TABS.includes(saved)) return saved;
    } catch {
      // no storage: the first tab
    }
    return 'problems';
  });
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
  const [perfTab, setPerfTab] = useState<PerfTab>(() => {
    try {
      const saved = localStorage.getItem(PERF_KEY) as PerfTab | null;
      return saved && PERF_TABS.includes(saved) ? saved : 'memory';
    } catch {
      return 'memory';
    }
  });
  const pickPerfTab = (next: PerfTab) => {
    setPerfTab(next);
    try {
      localStorage.setItem(PERF_KEY, next);
    } catch {
      // only a convenience
    }
  };
  const pickTab = (next: HealthTab) => {
    setTab(next);
    try {
      localStorage.setItem(TAB_KEY, next);
    } catch {
      // only a convenience
    }
  };
  // A Mod List chip points at one issue: its tab first, and its section open, so the row is there
  // to scroll to (not saved: the player's own folding stays as it was).
  useEffect(() => {
    if (!focusIssue) return;
    const focusTab = tabOf(focusIssue);
    setTab(focusTab);
    setCollapsed((prev) => ({ ...prev, [`${focusTab}:${issueStateOf(focusIssue)}`]: false }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusIssue]);
  /** What wants something done on a tab: the rows that act or are worth acting on. */
  const toDo = (issues: HealthIssue[]) => issues.filter((i) => ['act', 'recommended'].includes(issueStateOf(i)));
  const actCount = (issues: HealthIssue[]) => issues.filter((i) => issueStateOf(i) === 'act').length;
  const tabCount: Record<HealthTab, number | null> = {
    problems: toDo(byTab.problems).length,
    conflicts: toDo(byTab.conflicts).length,
    security: toDo(byTab.security).length,
    performance: null,
    disk: null,
  };

  /**
   * The rows of the tab in front, in sections by state, most pressing first: act, worth doing,
   * worth knowing, already fixed by the Studio. The same four on every tab.
   */
  const sections = useMemo(() => {
    if (tab === 'performance' || tab === 'disk') return [];
    const by: Record<IssueState, HealthIssue[]> = { act: [], recommended: [], info: [], handled: [] };
    byTab[tab].forEach((i) => by[issueStateOf(i)].push(i));
    return STATE_ORDER.map((state) => ({ key: state, state, issues: by[state], items: groupItems(by[state]) }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [byTab, tab, i18next.language]);

  /** The fixes a row offers, as buttons; "Enable X" + "Move X before Y" are one button doing both. */
  const fixesOf = (issue: HealthIssue) => {
    const applicable = issue.fixes.filter((f) => ACTION_FIXES.has(f.action) || applyFix(mods, f) !== null);
    const enableThenMove = applicable.length === 2 && applicable[0].action === 'enable_mod' && applicable[1].action === 'move_before';
    return { applicable, enableThenMove, fixes: enableThenMove ? [applicable[0]] : applicable };
  };
  const labelOf = (issue: HealthIssue, fix: HealthFix) =>
    YOUR_CALL_CODES.has(issue.code) && fix.action === 'move_before' ? t('health.useVersionInstead', { name: nameOf(fix.before) })
      : issue.code === 'two_mod_managers' && fix.action === 'disable_mod' ? t('health.fix.useStudioScreen')
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
        className={`flex items-start gap-3 p-3 rounded-lg border ${style.card} ${litKey === issueKey(issue) ? 'ring-2 ring-cyan-400 shadow-lg shadow-cyan-900/50' : ''}`}
      >
        <Icon className={`w-4 h-4 mt-0.5 shrink-0 ${style.iconColor}`} aria-hidden="true" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 min-w-0">
            <span className={`shrink-0 text-[10px] font-bold uppercase tracking-wide ${style.label}`}>{t(`health.state.${state}.word`)}</span>
            {scopeChip(issue.code, issue.own_mod)}
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
        {fixes.length > 0 && (
          <div className="flex flex-wrap justify-end gap-1.5 shrink-0 max-w-[45%]">
            {fixes.map((fix, i) => {
              const quiet = undoOnly && UNDO_ACTIONS.has(fix.action);
              return (
                <button
                  key={i}
                  type="button"
                  onClick={() => (enableThenMove ? handleFixes(applicable) : handleFix(fix, issue))}
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
                    {dismiss && (
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
          {shared.length > 0 && (
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
                    {fix && (
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

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-y-auto p-6 gap-4 font-sans">
      <div className="flex items-start justify-between gap-4 p-5 rounded-xl border border-slate-800 bg-slate-900/60">
        <div className="flex items-start gap-3">
          <HeartPulse className="w-6 h-6 text-emerald-400 mt-0.5" aria-hidden="true" />
          <div>
            <h1 className="text-lg font-bold text-slate-100">{t('health.title')}</h1>
            <p className="text-xs text-slate-400 max-w-2xl">
              {t('health.blurb')}
            </p>
            {report && (
              <div className="flex flex-wrap gap-2 mt-3 text-[11px] font-mono">
                <span className="px-2 py-0.5 rounded border border-slate-700 text-slate-300">
                  {t('health.gameBuild', { build: report.game_version ?? t('health.unknown') })}
                </span>
                <span className="px-2 py-0.5 rounded border border-slate-700 text-slate-300">{t('health.activeCount', { count: report.active_count })}</span>
                {report.community && (
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
          </div>
        </div>
        <button
          type="button"
          onClick={onRecheck}
          disabled={checking}
          className="flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-60 shrink-0"
        >
          {checking ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <RefreshCw className="w-3.5 h-3.5" aria-hidden="true" />}
          {t('health.checkAgain')}
        </button>
      </div>

      <PageGuide page="health" />

      <div role="tablist" className="flex items-end gap-1 border-b border-slate-800">
        {TABS.map((id) => {
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
              {count !== null && count > 0 && <span className={`px-1.5 rounded-full text-[10px] font-mono ${badge}`}>{count}</span>}
            </button>
          );
        })}
      </div>

      {tab !== 'performance' && tab !== 'disk' && (
        <div className="flex items-center justify-between gap-3">
          <p className="text-xs text-slate-400 max-w-3xl">{t(`health.tabs.${tab}Intro`)}</p>
          {tab === 'problems' && safeFixes.length > 0 && (
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

      {report && tab !== 'performance' && tab !== 'disk' && toDo(byTab[tab]).length === 0 && (
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
      {tab === 'performance' && (
        <div role="tablist" className="flex flex-wrap items-center gap-1">
          {PERF_TABS.map((id) => (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={perfTab === id}
              onClick={() => pickPerfTab(id)}
              className={`px-3 py-1.5 text-xs font-bold rounded-md border cursor-pointer ${
                perfTab === id ? 'bg-slate-800 border-slate-600 text-slate-100' : 'border-slate-800 text-slate-400 hover:text-slate-200 hover:bg-slate-900'
              }`}
            >
              {t(`health.perfTabs.${id}`)}
            </button>
          ))}
        </div>
      )}
      {tab === 'performance' && perfTab === 'memory' && <MemoryPanel paths={paths} />}
      {tab === 'performance' && perfTab === 'boot' && <BootTimePanel paths={paths} />}
      {tab === 'performance' && perfTab === 'time' && <GameTimePanel paths={paths} mods={mods} onModsChanged={onModsChanged} />}
      {tab === 'performance' && perfTab === 'frames' && <FramesPanel paths={paths} mods={mods} />}
      {tab === 'disk' && <DiskPanel paths={paths} onModsChanged={onModsChanged} onOpenInBrowser={onOpenInBrowser} />}
    </div>
  );
};
