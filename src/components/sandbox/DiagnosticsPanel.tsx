import React, { useEffect, useMemo, useState } from 'react';
import { TranslatedErrorCard, ModInfo, RequireAnswer, PackageFileState, ErrorHistoryView, KeyHistory } from '../../types';
import i18next from 'i18next';
import { BisectPanel } from './BisectPanel';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, explainError, reportBackendError } from '../../services/tauri';
import { applyRequireFix, packageForPatch } from '../../utils/requireFix';
import { errorShape } from '../../utils/errorShape';
import { AlertCircle, AlertTriangle, Check, CheckCircle, Copy, Download, MessageSquare, Search, Trash2, Wrench, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { cardText } from '../../i18n/backendText';

/** The game names a Studio package when it read the package's patched copy of a mod's file. */
const STUDIO_PACKAGE_PREFIX = 'PZ Mod Studio Patch: ';

interface DiagnosticsPanelProps {
  paths: StudioPathsUI;
  /** The installed mods: to find a mod the log named, and for the hunt. */
  mods: ModInfo[];
  errorCards: TranslatedErrorCard[];
  /** The lines of the log in front of the user, for counts and for the stack under an error. */
  logLines: string[];
  /** The live console's lines, which go into a report as the context around the error. */
  reportLines: string[];
  /** Re-reads the Mod List after a fix has written to disk. */
  onRefreshMods: () => void;
  /** The Build 42 shim travels inside a merge package, so repairs start in the Merger. */
  onOpenMerger: () => void;
  /** Switches a mod off by name, for when the game blames it for an error. */
  onSwitchOffMod?: (modName: string) => void;
  /** Opens the Merger on this mod's files to patch one; with a file name from the log, that file. */
  onPatchMod?: (modId: string, fileHint?: string | null, fix?: { from: string; to: string } | null) => void;
  onClearErrorCards?: () => void;
  /** Removes one card, once what it reported has been fixed. */
  onDismissErrorCard?: (cardId: string) => void;
  /** Puts a term in the console search on the left. */
  onInspect: (term: string) => void;
  /** A short note for the console's status badge ("Saved to ..."). */
  onStatus: (message: string) => void;
}

/**
 * The right column of the Monitor: what the game threw and what it warned about, one card each,
 * with what can be done from here -- a known require fix published into the package, a copy taken
 * out of it, a file opened in the Merger, a mod switched off -- and the hunt for what the log
 * cannot name.
 */
export const DiagnosticsPanel: React.FC<DiagnosticsPanelProps> = ({
  paths,
  mods,
  errorCards,
  logLines,
  reportLines,
  onRefreshMods,
  onOpenMerger,
  onSwitchOffMod,
  onPatchMod,
  onClearErrorCards,
  onDismissErrorCard,
  onInspect,
  onStatus,
}) => {
  const { t } = useTranslation();
  const [copiedCardId, setCopiedCardId] = useState<string | null>(null);

  /**
   * Gathers everything about one error into a file. The log the Monitor already holds goes in as
   * context, so whoever reads it can see what led up to the error.
   */
  const buildReportFor = async (card: TranslatedErrorCard): Promise<string | null> => {
    const lines = reportLines;
    const at = lines.findIndex((l) => l === card.raw_error);
    const from = at >= 0 ? Math.max(0, at - 15) : Math.max(0, lines.length - 20);
    const to = at >= 0 ? Math.min(lines.length, at + 10) : lines.length;
    try {
      return await TauriService.buildErrorReport(paths, {
        rawError: card.raw_error,
        title: card.title,
        explanation: card.explanation,
        modName: card.mod_name ?? null,
        sourceFile: card.source_file ?? null,
        logContext: lines.slice(from, to),
      });
    } catch (err) {
      reportBackendError(t('monitor.errors.writingReport'), err);
      return null;
    }
  };

  const handleSaveReport = async (card: TranslatedErrorCard) => {
    const report = await buildReportFor(card);
    if (!report) return;
    const slug = (card.mod_name ?? 'error').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    const path = await TauriService.pickSaveFile(`pz-error-${slug || 'report'}.md`, t('monitor.cards.mdFilter'), 'md');
    if (!path) return;
    try {
      await TauriService.saveTextFile(path, report);
      onStatus(t('monitor.cards.savedTo', { path }));
    } catch (err) {
      reportBackendError(t('monitor.errors.savingReport'), err);
    }
  };

  const handleCopyReport = async (card: TranslatedErrorCard) => {
    const report = await buildReportFor(card);
    if (!report) return;
    await navigator.clipboard.writeText(report);
    onStatus(t('monitor.cards.reportCopied'));
  };

  /** The report with what to ask first: pasted into any chat assistant, no key needed. */
  const handleCopyForAssistant = async (card: TranslatedErrorCard) => {
    const report = await buildReportFor(card);
    if (!report) return;
    await navigator.clipboard.writeText(`${t('assistant.monitorAsk', { mod: card.mod_name ?? t('assistant.unknownMod') })}

${report}`);
    onStatus(t('assistant.copied'));
  };

  const handleInspectInConsole = (card: TranslatedErrorCard) => {
    let term = '';
    if (card.source_file) {
      term = card.source_file.split(/[/|\\]/).pop() || '';
    } else if (card.raw_error) {
      const match = card.raw_error.match(/([a-zA-Z0-9_]+\.lua|[a-zA-Z0-9_]+Exception|[a-zA-Z0-9_]+State|isLocalPlayer|ToggleDoor|BWORoomPrograms)/);
      term = match ? match[1] : card.raw_error.slice(0, 25);
    }
    onInspect(term);
  };

  const handleCopyCardSnippet = (cardId: string, snippet: string) => {
    navigator.clipboard.writeText(snippet);
    setCopiedCardId(cardId);
    setTimeout(() => setCopiedCardId(null), 2000);
  };

  /**
   * How many error lines of each shape the log in front of you holds, so a card can say whether
   * it happened once or six hundred times. Counted from the log itself rather than remembered as
   * cards arrive, so it is the same for a live session and a log loaded from disk.
   */
  const errorCounts = useMemo(() => {
    const counts = new Map<string, number>();
    let total = 0;
    for (const line of logLines) {
      if (!line.startsWith('ERROR')) continue;
      total += 1;
      const shape = errorShape(line);
      counts.set(shape, (counts.get(shape) ?? 0) + 1);
    }
    return { counts, total, distinct: counts.size };
  }, [logLines]);

  const timesSeen = (card: TranslatedErrorCard) => errorCounts.counts.get(errorShape(card.raw_error)) ?? 0;

  /**
   * What threw and what merely complained, kept apart: seven `require` warnings under a red
   * "Action Required" made a list that played fine look broken. The level the game wrote the
   * line at is the measure, and the cards that threw come first.
   */
  const isWarning = (card: TranslatedErrorCard) => card.severity === 'WARNING';

  /** The log names a mod by its display name; the Merger needs its id. */
  const modIdNamed = (name: string | null | undefined): string | null => {
    if (!name) return null;
    const hit = mods.find((m) => m.name === name || m.mod_id === name);
    return hit ? hit.mod_id : null;
  };
  /** `ZombiesAmputation.lua` from a card's source file, for the Merger to find in the mod. */
  const fileNamed = (card: TranslatedErrorCard): string | null =>
    card.source_file ? card.source_file.split(/[/\\]/).pop() || null : null;
  const exceptionCards = useMemo(() => errorCards.filter((c) => !isWarning(c)), [errorCards]);
  const warningCards = useMemo(() => errorCards.filter(isWarning), [errorCards]);
  const orderedCards = useMemo(() => [...exceptionCards, ...warningCards], [exceptionCards, warningCards]);

  /**
   * `require("X") failed` is the commonest complaint in a Build 42 log and says nothing you can
   * act on by itself. The module is usually still there under another path, sometimes in another
   * mod, sometimes nowhere at all -- and which of the three it is decides what to do about it.
   * Looked up for every such card at once, since the index costs the same for one or twenty.
   */
  const [requireAnswers, setRequireAnswers] = useState<Record<string, RequireAnswer>>({});

  const moduleWanted = (card: TranslatedErrorCard): string | null => {
    const found = card.raw_error.match(/require\(\s*["']([^"']+)["']\s*\)/);
    return found ? found[1] : null;
  };
  /**
   * "Fix the path" from the card, with no trip to the Merger: the file is read from the mod, the
   * require rewritten, and the result published into the package in place -- the package stays
   * packaged, nothing is unpacked. What the card then says is what happened, and what is left
   * (relaunch the game) is the only step the Studio cannot take.
   */
  const [fixNotes, setFixNotes] = useState<Record<string, string>>({});
  /** Whether each card's error was seen in earlier sessions, from the Studio's error history. */
  const [history, setHistory] = useState<{ view: ErrorHistoryView; byId: Map<string, KeyHistory> } | null>(null);
  const [onlyNew, setOnlyNew] = useState(false);
  const cardsKey = errorCards.map((c) => c.id).join('|');
  useEffect(() => {
    if (errorCards.length === 0 || !paths.user_zomboid_dir) {
      setHistory(null);
      return;
    }
    let live = true;
    const timer = window.setTimeout(() => {
      void TauriService.errorHistory(paths, errorCards.map((c) => c.raw_error)).then((view) => {
        if (!live || !view) return;
        const byId = new Map<string, KeyHistory>();
        errorCards.forEach((c, i) => {
          if (view.keys[i]) byId.set(c.id, view.keys[i]);
        });
        setHistory({ view, byId });
      });
    }, 400);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cardsKey, paths.user_zomboid_dir]);
  const historyOf = (card: TranslatedErrorCard): KeyHistory | undefined => history?.byId.get(card.id);
  const newCount = errorCards.filter((c) => historyOf(c)?.new).length;
  const dayOf = (secs: number | null) => (secs ? new Date(secs * 1000).toLocaleDateString(i18next.language) : '');
  const shownCards = onlyNew && history ? orderedCards.filter((c) => historyOf(c)?.new) : orderedCards;
  /** The package's copies beside the mods' own files, read once a card names a package. */
  const [packageFiles, setPackageFiles] = useState<PackageFileState[] | null>(null);
  const anyPackageCard = errorCards.some((c) => (c.mod_name ?? '').startsWith(STUDIO_PACKAGE_PREFIX));
  useEffect(() => {
    if (!anyPackageCard || !paths.user_zomboid_dir) return;
    let live = true;
    void TauriService.packageFileStates(paths, mods.filter((m) => m.enabled).map((m) => m.mod_id)).then((states) => {
      if (live) setPackageFiles(states);
    });
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [anyPackageCard, paths.user_zomboid_dir, mods]);
  /** The package's copy a card is about, when the package holds a file of that name. */
  const packageStateOf = (card: TranslatedErrorCard): PackageFileState | undefined => {
    const file = card.source_file?.split(/[/\\]/).pop()?.toLowerCase();
    if (!file || !packageFiles) return undefined;
    return packageFiles.find((f) => f.relative_path.toLowerCase().endsWith('/' + file));
  };
  const [fixing, setFixing] = useState<string | null>(null);
  const [fixedBanner, setFixedBanner] = useState<string | null>(null);
  const handleFixPath = async (card: TranslatedErrorCard, fix: { from: string; to: string }) => {
    setFixing(card.id);
    try {
      // The files that ask for the module, in every active mod. The log names the file on some
      // lines and on others (`KahluaThread.call > require(...)`) nobody -- and then the mod the
      // card names is a guess from the lines above, which is wrong when two mods' files load one
      // after the other. The module is the one thing the line says for certain.
      const asking = await TauriService.activeModsFilesRequiring(paths, fix.from);
      if (asking.length === 0) {
        setFixNotes((prev) => ({
          ...prev,
          [card.id]: t('monitor.fixes.noFileAsks', { module: fix.from }),
        }));
        return;
      }
      const packages = await TauriService.listMergedPackages(paths.user_zomboid_dir || '', paths.mod_list_ini_path || '');
      const folder = packageForPatch(packages);
      const display = packages.find((p) => p.folder_name === folder)?.display_name ?? folder;
      const done: string[] = [];
      for (const { mod_id, mod_name, file } of asking) {
        const opened = await TauriService.modFileToPatch(paths, mod_id, file.relative_path);
        const fixed = applyRequireFix(opened.merged_output || opened.base_content, fix);
        if (!fixed) continue;
        await TauriService.publishFileIntoPackage(paths, folder, opened.relative_path, fixed.content);
        done.push(t('monitor.fixes.doneItem', { file: file.relative_path.split('/').pop(), mod: mod_name, line: fixed.line }));
      }
      if (done.length === 0) {
        setFixNotes((prev) => ({ ...prev, [card.id]: t('monitor.fixes.nothingToRewrite', { module: fix.from }) }));
        return;
      }
      // Done: the card goes, and one line above the list says what happened and what is left.
      setFixedBanner(
        t(done.length === 1 ? 'monitor.fixes.pathFixedOne' : 'monitor.fixes.pathFixedMany', {
          files: done.join(', '),
          to: fix.to,
          package: display,
        })
      );
      onDismissErrorCard?.(card.id);
      onRefreshMods();
    } catch (err) {
      setFixNotes((prev) => ({ ...prev, [card.id]: explainError(err) }));
    } finally {
      setFixing(null);
    }
  };

  /**
   * Takes the package's copy of a mod file out, from the card. A copy that changed nothing only
   * freezes the file and gets the package blamed for the mod's own lines; a copy that did change
   * something is the player's, so they are asked first.
   */
  const handleRemoveCopy = async (card: TranslatedErrorCard) => {
    const file = fileNamed(card);
    const display = (card.mod_name ?? '').slice(STUDIO_PACKAGE_PREFIX.length);
    if (!file || !display) return;
    if (
      !window.confirm(t('monitor.fixes.removeCopyConfirm', { file, package: display }))
    )
      return;
    setFixing(card.id);
    try {
      const packages = await TauriService.listMergedPackages(paths.user_zomboid_dir || '', paths.mod_list_ini_path || '');
      const folder = packages.find((p) => p.display_name === display)?.folder_name ?? `Z_PZModStudio_${display.replace(/ /g, '_')}`;
      const result = await TauriService.removeFileFromPackage(paths, folder, file);
      setFixedBanner(t('monitor.fixes.copyRemoved', { file, package: display, count: result.files_written }));
      onDismissErrorCard?.(card.id);
      onRefreshMods();
    } catch (err) {
      setFixNotes((prev) => ({ ...prev, [card.id]: explainError(err) }));
    } finally {
      setFixing(null);
    }
  };

  /**
   * A card the Build 42 shim answers. The shim rides in every published package, so what to do
   * depends on the packages: one published and on means the shim is already in the game (the
   * line is from a session without it, or the shim does not cover it); one open as a draft means
   * the game is without it until it is published; none means one has to be made in the Merger.
   */
  const [shimNotes, setShimNotes] = useState<Record<string, string>>({});
  const handleShimCard = async (card: TranslatedErrorCard) => {
    let packages: Awaited<ReturnType<typeof TauriService.listMergedPackages>> = [];
    try {
      packages = await TauriService.listMergedPackages(paths.user_zomboid_dir || '', paths.mod_list_ini_path || '');
    } catch {
      // Without the list, the Merger is still the place to look.
    }
    const own = packages.filter((p) => !p.is_companion_bridge);
    const live = own.find((p) => p.is_packaged && p.is_active_in_modlist);
    const draft = own.find((p) => !p.is_packaged);
    if (live) {
      setShimNotes((prev) => ({ ...prev, [card.id]: t('monitor.fixes.shimLive', { package: live.display_name }) }));
      return;
    }
    if (draft) {
      setShimNotes((prev) => ({ ...prev, [card.id]: t('monitor.fixes.shimDraft', { package: draft.display_name }) }));
    }
    onOpenMerger();
  };

  /** A `require` the Studio found at another path: the fix is known before the file is opened. */
  const knownFix = (card: TranslatedErrorCard): { from: string; to: string } | null => {
    const wanted = moduleWanted(card);
    const best = wanted ? requireAnswers[wanted]?.candidates[0] : undefined;
    return wanted && best && !best.same_path ? { from: wanted, to: best.module_path } : null;
  };

  useEffect(() => {
    const wanted = Array.from(
      new Set(errorCards.map(moduleWanted).filter((w): w is string => !!w))
    ).filter((w) => !(w in requireAnswers));
    if (wanted.length === 0 || !paths.is_valid) return;
    let alive = true;
    void (async () => {
      const answers = await TauriService.resolveFailedRequires(paths, wanted);
      if (!alive) return;
      setRequireAnswers((current) => {
        const next = { ...current };
        for (const a of answers) next[a.wanted] = a;
        return next;
      });
    })();
    return () => {
      alive = false;
    };
  }, [errorCards, paths.is_valid]);

  /**
   * The mod a card belongs to when the error line itself does not say.
   *
   * The game writes `ERROR ... Exception thrown` first and the stack trace after it, and it is
   * the frames that carry `Lua((MOD:Name))`. The backend only sees one line at a time, so it
   * cannot look ahead; here the whole log is in front of us, so the frames just below the error
   * are read for the name.
   */
  const modFromStack = (card: TranslatedErrorCard): string | null => {
    // A mod's translated text, or a line in one of the game's own files, says whose it is (or
    // that it is nobody's): a mod named on a line nearby is someone else.
    if (card.code === 'translator_text' || card.code === 'save_item_missing' || card.code === 'connection_lost' || card.code === 'windows_low_memory' || card.raw_error.includes('Lua(Vanilla)')) return null;
    const at = logLines.findIndex((l) => l.includes(card.raw_error.trim()));
    if (at < 0) return null;
    for (const line of logLines.slice(at, at + 30)) {
      const found = line.match(/Lua\(\(MOD:(.+?)\)\)/);
      if (found) return found[1];
    }
    return null;
  };

  return (
    <div className="lg:col-span-5 bg-slate-900/80 border border-slate-800 rounded-xl flex flex-col overflow-hidden shadow p-4 space-y-3 min-h-0">
      <div className="flex items-center justify-between border-b border-slate-800 pb-2.5">
        <div>
          <h3 className="text-xs font-bold text-slate-200 flex items-center gap-2">
            <span>{t('monitor.cards.heading')}</span>
            {exceptionCards.length > 0 && (
              <span
                className="text-[10px] px-1.5 py-0.5 rounded bg-red-500/20 text-red-400 border border-red-500/30 font-mono"
                title={t('monitor.cards.exceptionsBadgeTitle')}
              >
                {t('monitor.cards.exceptionsBadge', { count: exceptionCards.length })}
              </span>
            )}
            {warningCards.length > 0 && (
              <span
                className="text-[10px] px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-300 border border-amber-500/30 font-mono"
                title={t('monitor.cards.warningsBadgeTitle')}
              >
                {t('monitor.cards.warningsBadge', { count: warningCards.length })}
              </span>
            )}
          </h3>
          <p className="text-[11px] text-slate-400 mt-0.5">
            {exceptionCards.length > 0
              ? t('monitor.cards.summaryExceptions', {
                  total: errorCounts.total.toLocaleString(),
                  distinct: errorCounts.distinct,
                })
              : warningCards.length > 0
              ? t('monitor.cards.summaryNothingThrew') +
                (errorCounts.total > 0
                  ? t('monitor.cards.summaryAlsoErrorLines', {
                      total: errorCounts.total.toLocaleString(),
                      distinct: errorCounts.distinct,
                    })
                  : '')
              : errorCounts.total > 0
              ? t('monitor.cards.summaryErrorLinesOnly', {
                  total: errorCounts.total.toLocaleString(),
                  distinct: errorCounts.distinct,
                })
              : t('monitor.cards.summaryIdle')}
          </p>
          {history && history.view.earlier_sessions > 0 && errorCards.length > 0 && (
            <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px]">
              <span className={newCount > 0 ? 'text-emerald-300' : 'text-slate-500'}>
                {t('monitor.cards.newSummary', { count: newCount, sessions: history.view.earlier_sessions })}
              </span>
              <label className="flex items-center gap-1 text-slate-400 cursor-pointer">
                <input type="checkbox" checked={onlyNew} onChange={(e) => setOnlyNew(e.target.checked)} className="accent-emerald-500" />
                {t('monitor.cards.onlyNew')}
              </label>
            </div>
          )}
        </div>

        {errorCards.length > 0 ? (
          <button
            onClick={onClearErrorCards}
            className="text-[10px] text-slate-400 hover:text-slate-200 font-mono bg-slate-900 hover:bg-slate-800 border border-slate-800 px-2 py-1 rounded transition cursor-pointer flex items-center gap-1"
            title={t('monitor.cards.discardTitle')}
          >
            <Trash2 className="w-3 h-3 text-slate-400" />
            <span>{t('monitor.cards.discard')}</span>
          </button>
        ) : (
          <span className="text-[10px] text-emerald-400 flex items-center gap-1 font-mono">
            <CheckCircle className="w-3.5 h-3.5" /> {t('monitor.cards.nothingThrew')}
          </span>
        )}
      </div>

      {fixedBanner && (
        <div className="flex items-start gap-2 px-3 py-2 rounded-lg bg-emerald-950/50 border border-emerald-700/60 text-[11px] text-emerald-200 leading-relaxed shrink-0">
          <CheckCircle className="w-3.5 h-3.5 text-emerald-400 shrink-0 mt-0.5" aria-hidden="true" />
          <span className="flex-1">{fixedBanner}</span>
          <button
            type="button"
            onClick={() => setFixedBanner(null)}
            className="text-emerald-400 hover:text-emerald-200 cursor-pointer"
            title={t('monitor.cards.dismiss')}
          >
            <X className="w-3.5 h-3.5" aria-hidden="true" />
          </button>
        </div>
      )}

      {/* For what the log cannot name: halve the list until one mod is left. */}
      <BisectPanel
        paths={paths}
        mods={mods}
        onRefreshMods={onRefreshMods}
        cards={errorCards}
        onInspect={onInspect}
        onPatchMod={onPatchMod}
      />

      <div className="flex-1 overflow-y-auto space-y-3 pr-1 min-h-0">
        {errorCards.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-center p-6 text-slate-500 space-y-2.5">
            <div className="w-12 h-12 rounded-xl bg-emerald-950/40 border border-emerald-800/40 flex items-center justify-center text-emerald-400">
              <CheckCircle className="w-6 h-6" />
            </div>
            <div className="space-y-1">
              <p className="text-xs font-bold text-slate-300">{t('monitor.cards.emptyTitle')}</p>
              <p className="text-[11px] text-slate-400 max-w-xs leading-relaxed">{t('monitor.cards.emptyBody')}</p>
            </div>
          </div>
        ) : (
          shownCards.map((card) => (
            <div
              key={card.id}
              className={
                isWarning(card)
                  ? 'bg-slate-950 border border-amber-500/30 rounded-xl p-4 space-y-3 shadow-md hover:border-amber-500/60 transition'
                  : 'bg-slate-950 border border-red-500/40 rounded-xl p-4 space-y-3 shadow-md hover:border-red-500/70 transition'
              }
            >
              <div className="flex items-start gap-2.5">
                {isWarning(card) ? (
                  <AlertTriangle className="w-5 h-5 text-amber-400 shrink-0 mt-0.5" />
                ) : (
                  <AlertCircle className="w-5 h-5 text-red-400 shrink-0 mt-0.5" />
                )}
                <div>
                  <div className="flex items-start justify-between gap-2">
                    <h4 className="text-xs font-bold text-slate-100">
                      {cardText(card, 'title')}
                      {isWarning(card) && (
                        <span
                          className="ml-2 align-middle text-[10px] font-mono font-normal px-1.5 py-0.5 rounded border border-amber-700/60 bg-amber-950/40 text-amber-300"
                          title={t('monitor.cards.warningChipTitle')}
                        >
                          {t('monitor.cards.warningChip')}
                        </span>
                      )}
                    </h4>
                    {timesSeen(card) > 1 && (
                      <span
                        className="shrink-0 text-[10px] font-mono px-1.5 py-0.5 rounded border border-red-800/70 bg-red-950/50 text-red-300"
                        title={t('monitor.cards.timesSeenTitle', {
                          count: timesSeen(card),
                          share:
                            errorCounts.total > 0
                              ? t('monitor.cards.timesSeenShare', {
                                  pct: Math.round((timesSeen(card) / errorCounts.total) * 100),
                                })
                              : '',
                        })}
                      >
                        {timesSeen(card).toLocaleString()}×
                      </span>
                    )}
                  </div>
                  {(() => {
                    const h = historyOf(card);
                    if (!h || !history || history.view.earlier_sessions === 0) return null;
                    return h.new ? (
                      <span className="ml-1.5 text-[10px] px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-300 border border-emerald-500/30 font-mono" title={t('monitor.cards.newTitle', { sessions: history.view.earlier_sessions })}>
                        {t('monitor.cards.newChip')}
                      </span>
                    ) : (
                      <span className="ml-1.5 text-[10px] px-1.5 py-0.5 rounded bg-slate-800 text-slate-400 border border-slate-700 font-mono" title={t('monitor.cards.seenTitle', { count: h.sessions_seen, sessions: history.view.earlier_sessions, first: dayOf(h.first_seen), last: dayOf(h.last_seen) })}>
                        {t('monitor.cards.seenChip', { count: h.sessions_seen, first: dayOf(h.first_seen) })}
                      </span>
                    );
                  })()}
                  {(card.mod_name ?? modFromStack(card)) && (
                    <div className="mt-1 flex flex-wrap items-center gap-1.5">
                      <span
                        className="text-[10px] font-mono px-1.5 py-0.5 rounded border border-amber-700/60 text-amber-300"
                        title={
                          card.mod_name ? t('monitor.cards.modOnLine') : t('monitor.cards.modInStack')
                        }
                      >
                        {card.mod_name ?? modFromStack(card)}
                      </span>
                      {(card.called_from?.length ?? 0) > 0 && (
                        /* The stack the game printed before the error: who called into it. */
                        <span
                          className="text-[10px] font-mono px-1.5 py-0.5 rounded border border-slate-700 text-slate-300"
                          title={t('monitor.cards.calledFromTitle', { mod: card.mod_name ?? '', mods: card.called_from!.join(', ') })}
                        >
                          {t('monitor.cards.calledFrom', { mods: card.called_from!.join(', ') })}
                        </span>
                      )}
                      {(card.mod_name ?? '').startsWith(STUDIO_PACKAGE_PREFIX) ? (
                        /* The package's patched copy of a mod file. Taking it out is done from
                           here; the game then reads the mod's own file again. */
                        fileNamed(card) && (
                          <button
                            type="button"
                            onClick={() => void handleRemoveCopy(card)}
                            disabled={fixing === card.id}
                            className="text-[10px] px-1.5 py-0.5 rounded border border-amber-700/60 text-amber-200 hover:bg-amber-950/40 cursor-pointer flex items-center gap-1 disabled:opacity-50"
                            title={t('monitor.fixes.removeCopyTitle', {
                              file: fileNamed(card),
                              package: card.mod_name!.slice(STUDIO_PACKAGE_PREFIX.length),
                            })}
                          >
                            <Wrench className="w-3 h-3" aria-hidden="true" />
                            {fixing === card.id
                              ? t('monitor.fixes.removing')
                              : t('monitor.fixes.removeCopy', { file: fileNamed(card) })}
                          </button>
                        )
                      ) : onPatchMod &&
                        modIdNamed(card.mod_name ?? modFromStack(card)) &&
                        /* A warning the Studio has no fix for gets no button: pressing one
                           led to saving an unchanged copy, which fixes nothing. */
                        (!isWarning(card) || knownFix(card)) && (
                        <button
                          type="button"
                          onClick={() => {
                            const modId = modIdNamed(card.mod_name ?? modFromStack(card))!;
                            const fix = knownFix(card);
                            // A known fix is done from here, whether or not the log named the
                            // file; only an exception with no known fix goes to the Merger.
                            if (fix) void handleFixPath(card, fix);
                            else onPatchMod(modId, fileNamed(card), null);
                          }}
                          disabled={fixing === card.id}
                          className="text-[10px] px-1.5 py-0.5 rounded border border-amber-700/60 text-amber-200 hover:bg-amber-950/40 cursor-pointer flex items-center gap-1 disabled:opacity-50"
                          title={
                            knownFix(card)
                              ? t('monitor.fixes.rewriteTitle', { module: knownFix(card)!.from })
                              : fileNamed(card)
                              ? t('monitor.fixes.patchFileTitle', { file: fileNamed(card) })
                              : t('monitor.fixes.patchAnyTitle')
                          }
                        >
                          <Wrench className="w-3 h-3" aria-hidden="true" />
                          {fixing === card.id
                            ? t('monitor.fixes.fixing')
                            : knownFix(card)
                            ? fileNamed(card)
                              ? t('monitor.fixes.fixPathIn', { file: fileNamed(card) })
                              : t('monitor.fixes.fixPath')
                            : fileNamed(card)
                            ? t('monitor.fixes.patchFile', { file: fileNamed(card) })
                            : t('monitor.fixes.patchAFile')}
                        </button>
                      )}
                      {(() => {
                        /* Why the copy failed, when the package can tell: the mod changed the file after the copy. */
                        const state = packageStateOf(card);
                        const text = state?.stale
                          ? t('monitor.fixes.packageCopyStale', { mod: state.mod_name ?? '', modDate: state.mod_date ?? '', copyDate: state.copy_date })
                          : state?.orphan
                          ? t('monitor.fixes.packageCopyOrphan', { copyDate: state.copy_date })
                          : null;
                        return text ? (
                          <div className="basis-full text-[10px] text-amber-200 bg-amber-950/30 border border-amber-800/40 rounded px-2 py-1 leading-relaxed">
                            {text}
                          </div>
                        ) : null;
                      })()}
                      {fixNotes[card.id] && (
                        <div className="basis-full text-[10px] text-emerald-300 bg-emerald-950/40 border border-emerald-800/50 rounded px-2 py-1 leading-relaxed">
                          {fixNotes[card.id]}
                        </div>
                      )}
                      {onSwitchOffMod && !(card.mod_name ?? '').startsWith(STUDIO_PACKAGE_PREFIX) && card.code !== 'ui_draw_reload' && (
                        <button
                          type="button"
                          onClick={() => onSwitchOffMod((card.mod_name ?? modFromStack(card))!)}
                          className="text-[10px] px-1.5 py-0.5 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer"
                          title={t('monitor.fixes.switchOffTitle', { mod: card.mod_name ?? modFromStack(card) })}
                        >
                          {t('monitor.fixes.switchOff')}
                        </button>
                      )}
                    </div>
                  )}
                  <p className="text-[11px] text-slate-400 mt-1 leading-relaxed">
                    {cardText(card, 'explanation')}
                  </p>
                </div>
              </div>

              <div className="bg-slate-900 p-2.5 rounded-lg font-mono text-[11px] text-red-300 border border-slate-800 select-text">
                <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">{t('monitor.cards.logSnippet')}</div>
                <div className="truncate">{card.raw_error}</div>
              </div>

              {/* Where the module it asked for actually is. */}
              {(() => {
                const wanted = moduleWanted(card);
                const answer = wanted ? requireAnswers[wanted] : undefined;
                if (!answer) return null;
                const best = answer.candidates[0];
                return (
                  <div className="px-2.5 py-2 rounded-lg bg-slate-900/80 border border-slate-800 text-[11px] text-slate-300 leading-relaxed space-y-1">
                    <div>{answer.summary}</div>
                    {best && !best.same_path && (
                      <div className="font-mono text-[10px] text-emerald-300">
                        {answer.wanted} &rarr; {best.module_path}
                      </div>
                    )}
                  </div>
                );
              })()}

              {card.polyfill_rule_id_suggestion ? (
                <div className="space-y-1.5">
                  <button
                    onClick={() => void handleShimCard(card)}
                    className="w-full flex items-center justify-center gap-2 py-2 px-3 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 text-xs font-bold transition cursor-pointer"
                    title={t('monitor.fixes.openMergerTitle')}
                  >
                    <Wrench className="w-3.5 h-3.5" aria-hidden="true" />
                    <span>{t('monitor.fixes.openMerger')}</span>
                  </button>
                  {shimNotes[card.id] && (
                    <div className="text-[11px] text-sky-200 bg-sky-950/40 border border-sky-800/50 rounded-lg px-2.5 py-1.5 leading-relaxed">
                      {shimNotes[card.id]}
                    </div>
                  )}
                </div>
              ) : card.mod_name || card.code === 'translator_text' || card.code === 'save_item_missing' || card.code === 'connection_lost' || card.code === 'windows_low_memory' ? (
                /* The game named a mod, or it is a mod's text: say what to do, not what the shim covers. */
                <div className="px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-800 text-[11px] text-slate-400">
                  {cardText(card, 'action')}
                </div>
              ) : (
                <div className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-emerald-950/40 border border-emerald-800/40 text-[11px] text-emerald-300">
                  <CheckCircle className="w-3.5 h-3.5 text-emerald-400 shrink-0" aria-hidden="true" />
                  <span>{t('monitor.fixes.shimCovers')}</span>
                </div>
              )}

              <div className="flex flex-wrap items-center gap-2 pt-0.5">
                <button
                  onClick={() => handleSaveReport(card)}
                  className="flex-1 flex items-center justify-center gap-1.5 py-1.5 px-2.5 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-300 hover:text-white text-[11px] font-medium transition cursor-pointer"
                  title={t('monitor.cards.saveReportTitle')}
                >
                  <Download className="w-3.5 h-3.5" aria-hidden="true" />
                  <span>{t('monitor.cards.saveReport')}</span>
                </button>
                <button
                  onClick={() => handleCopyReport(card)}
                  className="flex items-center justify-center gap-1.5 py-1.5 px-2.5 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-300 hover:text-white text-[11px] font-medium transition cursor-pointer"
                  title={t('monitor.cards.copyReportTitle')}
                >
                  <Copy className="w-3.5 h-3.5" aria-hidden="true" />
                  <span>{t('monitor.cards.copyReport')}</span>
                </button>
                <button
                  onClick={() => handleCopyForAssistant(card)}
                  className="flex items-center justify-center gap-1.5 py-1.5 px-2.5 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-300 hover:text-white text-[11px] font-medium transition cursor-pointer"
                  title={t('assistant.copyTitle')}
                >
                  <MessageSquare className="w-3.5 h-3.5" aria-hidden="true" />
                  <span>{t('assistant.copy')}</span>
                </button>
              </div>

              <div className="flex flex-wrap items-center gap-2 pt-0.5">
                <button
                  onClick={() => handleInspectInConsole(card)}
                  className="flex-1 flex items-center justify-center gap-1.5 py-1.5 px-2.5 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-300 hover:text-white text-[11px] font-medium transition cursor-pointer"
                  title={t('monitor.cards.inspectTitle')}
                >
                  <Search className="w-3 h-3 text-cyan-400" />
                  <span>{t('monitor.cards.inspect')}</span>
                </button>
                <button
                  onClick={() => handleCopyCardSnippet(card.id, card.raw_error)}
                  className="flex items-center justify-center gap-1.5 py-1.5 px-3 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-400 hover:text-slate-200 text-[11px] font-medium transition cursor-pointer"
                  title={t('monitor.cards.copyLineTitle')}
                >
                  {copiedCardId === card.id ? (
                    <>
                      <Check className="w-3 h-3 text-emerald-400" />
                      <span className="text-emerald-400">{t('monitor.cards.copied')}</span>
                    </>
                  ) : (
                    <>
                      <Copy className="w-3 h-3 text-slate-400" />
                      <span>{t('monitor.cards.copyLine')}</span>
                    </>
                  )}
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};
