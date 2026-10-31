import React, { useEffect, useMemo, useRef, useState } from 'react';
import { AlertTriangle, CheckCircle2, ChevronDown, ChevronRight, Gauge, GitMerge, HardDrive, HeartPulse, Info, Loader2, MessageSquare, RefreshCw, ShieldCheck, Wand2, XCircle } from 'lucide-react';
import { AutoMergeResult, CompatResult, HealthFix, HealthIssue, HealthReport, HealthSeverity, ModInfo } from '../../types';
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
  'enable_shim_patch', 'remove_from_package', 'dismiss_known_issue',
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
 * The screen in tabs, so each kind of row is read on its own: what breaks or will break the game,
 * which mod's file or item wins where two mods give the same one (your call), what is only worth
 * knowing, and what the mods cost while the game runs.
 */
type HealthTab = 'problems' | 'conflicts' | 'info' | 'security' | 'performance' | 'disk';
const TABS: HealthTab[] = ['problems', 'conflicts', 'info', 'security', 'performance', 'disk'];
const TAB_ICON: Record<HealthTab, React.ElementType> = { problems: XCircle, conflicts: GitMerge, info: Info, security: ShieldCheck, performance: Gauge, disk: HardDrive };
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
  }
};

/** Where a conflict stands: the package settles it, did but a mod changed since, or does not. */
type ConflictState = 'open' | 'stale' | 'settled';
const CONFLICT_STATES: ConflictState[] = ['open', 'stale', 'settled'];
const conflictStateOf = (code: string): ConflictState =>
  code.endsWith('_settled') ? 'settled' : code.endsWith('_stale') || code === 'package_built_for_other_list' ? 'stale' : 'open';
const CONFLICT_STYLE: Record<ConflictState, { icon: React.ElementType; row: string; iconColor: string; tag: string }> = {
  open: { icon: GitMerge, row: 'border-l-4 border-orange-500/70 bg-orange-950/20', iconColor: 'text-orange-400', tag: 'border-orange-600/70 text-orange-300 bg-orange-950/50' },
  stale: { icon: AlertTriangle, row: 'border-l-4 border-amber-500/70 bg-amber-950/20', iconColor: 'text-amber-400', tag: 'border-amber-600/70 text-amber-300 bg-amber-950/50' },
  settled: { icon: CheckCircle2, row: 'border-l-4 border-emerald-600/70 bg-emerald-950/15', iconColor: 'text-emerald-400', tag: 'border-emerald-700/70 text-emerald-300 bg-emerald-950/50' },
};

const SEVERITY_STYLE: Record<HealthSeverity, { icon: React.ElementType; row: string; iconColor: string }> = {
  error: { icon: XCircle, row: 'border-red-500/40 bg-red-950/30', iconColor: 'text-red-400' },
  warning: { icon: AlertTriangle, row: 'border-amber-500/40 bg-amber-950/20', iconColor: 'text-amber-400' },
  info: { icon: Info, row: 'border-slate-700 bg-slate-900/50', iconColor: 'text-slate-400' },
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
        if (!String(err).includes('earlier compatible copy')) throw err;
        if (!confirm(t('health.freshCopy', { mod: modId }))) return;
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
      onModsChanged();
      onRecheck();
    } catch (err) {
      reportBackendError('Removing the compatible copy', err);
    }
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
   * What the last button did, in words. A load-order change is invisible from this screen -- the
   * row stays, because two mods still define the same thing -- so without this the app looks
   * like it did nothing.
   */
  const [done, setDone] = useState<string | null>(null);
  const say = (what: string) => {
    setDone(what);
    window.setTimeout(() => setDone((current) => (current === what ? null : current)), 6000);
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
    i.code.startsWith('security_') ? 'security' : CONFLICT_CODE.test(i.code) || YOUR_CALL_CODES.has(i.code) ? 'conflicts' : i.severity === 'info' ? 'info' : 'problems';
  const byTab = useMemo(() => {
    const out: Record<Exclude<HealthTab, 'performance' | 'disk'>, HealthIssue[]> = { problems: [], conflicts: [], info: [], security: [] };
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
  const toggleSection = (key: string) => {
    setCollapsed((prev) => {
      const next = { ...prev, [key]: !prev[key] };
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
  // A Mod List chip points at one issue: its tab first, so the row is there to scroll to.
  useEffect(() => {
    if (focusIssue) setTab(tabOf(focusIssue));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusIssue]);
  const errorsCount = byTab.problems.filter((i) => i.severity === 'error').length;
  const tabCount: Record<HealthTab, number | null> = {
    problems: byTab.problems.length,
    // Those that want something done: the settled ones are only there to be seen.
    conflicts: byTab.conflicts.filter((i) => conflictStateOf(i.code) !== 'settled').length,
    info: byTab.info.length,
    security: byTab.security.length,
    performance: null,
    disk: null,
  };

  /**
   * The rows of the tab in front, in sections: by severity, except on Conflicts, where what
   * matters is whether the package settles each one, so they go by that (the open ones first),
   * each with its colour and a word saying so.
   */
  const sections = useMemo(() => {
    if (tab === 'performance' || tab === 'disk') return [];
    if (tab === 'conflicts') {
      const by: Record<ConflictState, HealthIssue[]> = { open: [], stale: [], settled: [] };
      byTab.conflicts.forEach((i) => by[conflictStateOf(i.code)].push(i));
      return CONFLICT_STATES.map((state) => ({
        key: state,
        title: t(`health.conflictState.${state}`),
        issues: by[state],
        style: CONFLICT_STYLE[state],
        tag: t(`health.conflictState.${state}Tag`),
      }));
    }
    const by: Record<HealthSeverity, HealthIssue[]> = { error: [], warning: [], info: [] };
    byTab[tab].forEach((i) => by[i.severity].push(i));
    return (['error', 'warning', 'info'] as HealthSeverity[]).map((sev) => ({
      key: sev,
      title: t(`health.severity.${sev}`),
      issues: by[sev],
      style: SEVERITY_STYLE[sev],
      tag: null as string | null,
    }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [byTab, tab, i18next.language]);

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

      <div role="tablist" className="flex items-end gap-1 border-b border-slate-800">
        {TABS.map((id) => {
          const Icon = TAB_ICON[id];
          const count = tabCount[id];
          const active = tab === id;
          const badge =
            id === 'problems' && errorsCount > 0
              ? 'bg-red-600 text-white'
              : id === 'problems'
              ? 'bg-amber-500 text-slate-950'
              : 'bg-slate-700 text-slate-200';
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
        <div className="flex items-center gap-2 px-4 py-2 rounded-lg border border-emerald-800/60 bg-emerald-950/30 text-emerald-200 text-xs">
          <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" aria-hidden="true" />
          <span>{done}</span>
          <span className="text-emerald-500/70">{t('health.savedToList')}</span>
        </div>
      )}

      {report && tab !== 'performance' && tab !== 'disk' && byTab[tab].length === 0 && (
        <div className="flex items-center gap-3 p-5 rounded-xl border border-emerald-800/60 bg-emerald-950/30 text-emerald-200 text-sm">
          <CheckCircle2 className="w-5 h-5 text-emerald-400" aria-hidden="true" />
          {tab === 'problems' ? t('health.noProblems', { count: report.active_count }) : t(`health.tabs.${tab}Empty`)}
        </div>
      )}

      {sections.map((sec) =>
        sec.issues.length === 0 ? null : (
          <section key={sec.key} className="flex flex-col gap-2">
            <h2 className="text-[11px] font-bold uppercase tracking-wider text-slate-500 font-mono">
              <button type="button" onClick={() => toggleSection(`${tab}:${sec.key}`)} className="flex items-center gap-1 cursor-pointer hover:text-slate-300" aria-expanded={!collapsed[`${tab}:${sec.key}`]}>
                {collapsed[`${tab}:${sec.key}`] ? <ChevronRight className="w-3.5 h-3.5" aria-hidden="true" /> : <ChevronDown className="w-3.5 h-3.5" aria-hidden="true" />}
                {sec.title} ({sec.issues.length})
              </button>
            </h2>
            {!collapsed[`${tab}:${sec.key}`] && sec.issues.map((issue, idx) => {
              const style = sec.style;
              const Icon = style.icon;
              // "Enable X" + "Move X before Y" for the same issue are shown as one button doing both.
              const applicable = issue.fixes.filter((f) => ACTION_FIXES.has(f.action) || applyFix(mods, f) !== null);
              const enableThenMove = applicable.length === 2 && applicable[0].action === 'enable_mod' && applicable[1].action === 'move_before';
              const fixes = enableThenMove ? [applicable[0]] : applicable;
              return (
                <div
                  key={`${issue.code}-${issue.mod_id}-${idx}`}
                  data-issue-key={issueKey(issue)}
                  data-issue-code={`${issue.code}|${issue.mod_id ?? ''}`}
                  className={`flex items-start gap-3 p-3 rounded-lg border ${style.row} ${litKey === issueKey(issue) ? 'ring-2 ring-cyan-400 shadow-lg shadow-cyan-900/50' : ''}`}
                >
                  <Icon className={`w-4 h-4 mt-0.5 shrink-0 ${style.iconColor}`} aria-hidden="true" />
                  <div className="flex-1 min-w-0">
                    {(issue.mod_id || sec.tag) && (
                      <div className="flex items-center gap-2 min-w-0">
                        {sec.tag && (
                          <span className={`shrink-0 text-[10px] font-bold uppercase tracking-wide px-1.5 py-0.5 rounded border ${'tag' in style ? style.tag : ''}`}>{sec.tag}</span>
                        )}
                        {issue.mod_id && (
                          <span className="text-[12.5px] font-bold text-slate-100 truncate">
                            {nameOf(issue.mod_id)}
                            {nameOf(issue.mod_id) !== issue.mod_id && <span className="ml-1.5 text-[10px] font-mono font-normal text-slate-500">{issue.mod_id}</span>}
                          </span>
                        )}
                      </div>
                    )}
                    <RichText text={issueText(issue)} mods={modWords} foldAt={320} className="text-xs text-slate-300 leading-relaxed" />
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
                        <summary className="text-[11px] text-sky-300 cursor-pointer select-none">
                          {t('health.byFile', { count: issue.details.length })}
                        </summary>
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
                      {fixes.map((fix, i) => (
                        <button
                          key={i}
                          type="button"
                          onClick={() => (enableThenMove ? handleFixes(applicable) : handleFix(fix, issue))}
                          disabled={fix.action === 'merge_files' && mergeBusy}
                          title={fix.action === 'remove_compatible_copy' ? t('health.fix.removeCopyTitle') : undefined}
                          className={
                            // Undoing something the Studio did is there if wanted, not a thing to do.
                            fix.action === 'remove_compatible_copy'
                              ? 'px-2 py-1 rounded text-[11px] text-slate-400 hover:text-slate-200 border border-slate-700/70 hover:bg-slate-800/60 cursor-pointer'
                              : 'px-2 py-1 rounded text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-100 border border-slate-700 cursor-pointer'
                          }
                        >
                          {YOUR_CALL_CODES.has(issue.code) && fix.action === 'move_before'
                            ? t('health.useVersionInstead', { name: nameOf(fix.before) })
                            : fixLabel(fix)}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </section>
        )
      )}

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
