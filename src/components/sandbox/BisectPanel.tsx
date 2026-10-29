import React, { useEffect, useState } from 'react';
import { ModInfo, BisectSession, TranslatedErrorCard } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, backendSays, explainError } from '../../services/tauri';
import { Crosshair, Play, ThumbsDown, ThumbsUp, Undo2, ChevronDown, ChevronRight, Search, Wrench } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { cardText } from '../../i18n/backendText';
import { tHtml } from '../../i18n/html';

interface BisectPanelProps {
  paths: StudioPathsUI;
  mods: ModInfo[];
  /** The Mod List has to be re-read after every trial: the hunt writes the list itself. */
  onRefreshMods: () => void;
  /** The Monitor's cards, to show what the log says about the mod the hunt ends on. */
  cards?: TranslatedErrorCard[];
  /** Puts a term in the console search, so the lines about that mod are in front of you. */
  onInspect?: (term: string) => void;
  /** Opens the Merger on that mod's files, or on the file the log named. */
  onPatchMod?: (modId: string, fileHint?: string | null) => void;
}

/**
 * The hunt for the mod that breaks the game when the log names nobody: switch half the list off,
 * play, and keep halving whichever half still does it. Everything that another mod requires stays
 * on in every trial, so a trial never fails for a reason the hunt invented.
 */
export const BisectPanel: React.FC<BisectPanelProps> = ({
  paths,
  mods,
  onRefreshMods,
  cards = [],
  onInspect,
  onPatchMod,
}) => {
  const { t } = useTranslation();
  const [session, setSession] = useState<BisectSession | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showTrial, setShowTrial] = useState(false);
  const [open, setOpen] = useState(false);

  // A hunt outlives the Studio: it is written to disk after every answer.
  useEffect(() => {
    let alive = true;
    void (async () => {
      const found = await TauriService.bisectState(paths);
      if (alive && found) {
        setSession(found);
        setOpen(true);
      }
    })();
    return () => {
      alive = false;
    };
  }, [paths.user_zomboid_dir]);

  const nameOf = (id: string) => mods.find((m) => m.mod_id === id)?.name ?? id;

  const run = async (what: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await what();
    } catch (err) {
      setError(explainError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleStart = () =>
    run(async () => {
      const active = mods.filter((m) => m.enabled).length;
      const ok = window.confirm(t('monitor.bisect.startConfirm', { count: active }));
      if (!ok) return;
      const started = await TauriService.bisectStart(paths, mods);
      setSession(started);
      setShowTrial(false);
      onRefreshMods();
    });

  const handleAnswer = (stillHappens: boolean) =>
    run(async () => {
      const next = await TauriService.bisectAnswer(paths, stillHappens);
      setSession(next);
      setShowTrial(false);
      onRefreshMods();
    });

  const handleLaunch = () =>
    run(async () => {
      // Launched the way the player plays, without -debug: the point is to reproduce their game.
      await TauriService.launchSandbox({
        pz_install_dir: paths.pz_install_dir,
        user_zomboid_dir: paths.user_zomboid_dir,
        test_mode: 'NORMAL',
      });
    });

  const handleGiveUp = () =>
    run(async () => {
      await TauriService.bisectStop(paths, null);
      setSession(null);
      onRefreshMods();
    });

  const handleSwitchOffCulprit = (id: string) =>
    run(async () => {
      const active = mods.filter((m) => m.enabled).map((m) => m.mod_id).filter((m) => m !== id);
      await TauriService.writeModListIni(paths.mod_list_ini_path, active);
      setSession(null);
      onRefreshMods();
    });

  const hunting = session && session.phase !== 'Done';
  const finished = session && session.phase === 'Done';

  /**
   * What the log says about the mod the hunt ended on. The game names a mod by its display name
   * on the line; the hunt knows it by id. Usually the answer is nothing -- the hunt exists for
   * what the log cannot name -- and then it says so instead of pretending there is a line to fix.
   */
  const culpritMod = session?.culprit ? mods.find((m) => m.mod_id === session.culprit) ?? null : null;
  const culpritCards = session?.culprit
    ? cards.filter((c) => c.mod_name && (c.mod_name === culpritMod?.name || c.mod_name === session.culprit))
    : [];
  const fileNamed = (card: TranslatedErrorCard): string | null =>
    card.source_file ? card.source_file.split(/[/\\]/).pop() || null : null;

  return (
    <div
      className={`rounded-lg border shrink-0 ${
        hunting
          ? 'border-cyan-700/70 bg-cyan-950/25'
          : finished
          ? 'border-emerald-700/70 bg-emerald-950/20'
          : 'border-slate-800 bg-slate-900/60'
      }`}
    >
      <button
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left cursor-pointer"
      >
        {open ? (
          <ChevronDown className="w-3.5 h-3.5 text-slate-400 shrink-0" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-slate-400 shrink-0" />
        )}
        <Crosshair className={`w-3.5 h-3.5 shrink-0 ${hunting ? 'text-cyan-400' : 'text-slate-400'}`} />
        <span className="text-xs font-bold text-slate-200">{t('monitor.bisect.title')}</span>
        {hunting && (
          <span className="ml-auto text-[10px] font-mono px-1.5 py-0.5 rounded bg-cyan-950/80 text-cyan-300 border border-cyan-800/70">
            {t('monitor.bisect.roundBadge', { round: session!.round, left: session!.launches_left })}
          </span>
        )}
      </button>

      {open && (
        <div className="px-3 pb-3 space-y-2.5">
          {!session && (
            <>
              <p className="text-[11px] text-slate-400 leading-relaxed">{t('monitor.bisect.intro')}</p>
              <button
                onClick={handleStart}
                disabled={busy}
                className="w-full py-1.5 rounded-lg bg-cyan-950/70 hover:bg-cyan-900/70 border border-cyan-800 text-cyan-200 text-[11px] font-bold transition cursor-pointer disabled:opacity-50"
              >
                {t('monitor.bisect.start')}
              </button>
            </>
          )}

          {hunting && (
            <>
              <div className="text-[11px] text-slate-300 leading-relaxed">
                {session!.phase === 'Confirming' ? (
                  <span
                    dangerouslySetInnerHTML={{
                      __html: tHtml('monitor.bisect.confirming', { mod: nameOf(session!.trial[0]) }),
                    }}
                  />
                ) : (
                  <span
                    dangerouslySetInnerHTML={{
                      __html: tHtml('monitor.bisect.trial', {
                        trial: session!.trial.length,
                        candidates: session!.candidates.length,
                        alwaysOn: session!.always_on.length,
                      }),
                    }}
                  />
                )}
              </div>

              <p className="text-[10px] text-slate-500 leading-relaxed">{t('monitor.bisect.owns')}</p>

              <button
                onClick={() => setShowTrial((v) => !v)}
                className="text-[10px] text-slate-400 hover:text-slate-200 underline cursor-pointer"
              >
                {t(showTrial ? 'monitor.bisect.hideTrial' : 'monitor.bisect.showTrial', { count: session!.trial.length })}
              </button>
              {showTrial && (
                <div className="max-h-28 overflow-y-auto rounded border border-slate-800 bg-slate-950/70 p-1.5 space-y-0.5">
                  {session!.trial.map((id) => (
                    <div key={id} className="text-[10px] font-mono text-slate-400 truncate">
                      {nameOf(id)}
                    </div>
                  ))}
                </div>
              )}

              <button
                onClick={handleLaunch}
                disabled={busy}
                className="w-full flex items-center justify-center gap-1.5 py-1.5 rounded-lg bg-emerald-950/70 hover:bg-emerald-900/70 border border-emerald-800 text-emerald-200 text-[11px] font-bold transition cursor-pointer disabled:opacity-50"
              >
                <Play className="w-3.5 h-3.5" /> {t('monitor.bisect.launch')}
              </button>

              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleAnswer(true)}
                  disabled={busy}
                  className="flex-1 flex items-center justify-center gap-1.5 py-1.5 rounded-lg bg-red-950/60 hover:bg-red-900/60 border border-red-800/80 text-red-200 text-[11px] font-bold transition cursor-pointer disabled:opacity-50"
                >
                  <ThumbsDown className="w-3.5 h-3.5" /> {t('monitor.bisect.stillHappens')}
                </button>
                <button
                  onClick={() => handleAnswer(false)}
                  disabled={busy}
                  className="flex-1 flex items-center justify-center gap-1.5 py-1.5 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-200 text-[11px] font-bold transition cursor-pointer disabled:opacity-50"
                >
                  <ThumbsUp className="w-3.5 h-3.5" /> {t('monitor.bisect.gone')}
                </button>
              </div>

              <button
                onClick={handleGiveUp}
                disabled={busy}
                className="w-full flex items-center justify-center gap-1.5 py-1 text-[10px] text-slate-400 hover:text-slate-200 transition cursor-pointer disabled:opacity-50"
              >
                <Undo2 className="w-3 h-3" /> {t('monitor.bisect.giveUp')}
              </button>
            </>
          )}

          {finished && (
            <>
              {session!.culprit ? (
                <>
                  <div
                    className="text-[11px] text-slate-200 leading-relaxed"
                    dangerouslySetInnerHTML={{
                      __html: tHtml('monitor.bisect.culprit', {
                        mod: nameOf(session!.culprit),
                        launches: session!.history.length,
                      }),
                    }}
                  />

                  {/* What the log has on it, before deciding what to do with it. */}
                  <div className="rounded-lg border border-slate-800 bg-slate-950/70 p-2 space-y-1.5">
                    {culpritCards.length > 0 ? (
                      <>
                        <div className="text-[10px] text-slate-400">
                          {culpritCards.length === 1
                            ? t('monitor.bisect.logNamesOnce')
                            : t('monitor.bisect.logNamesTimes', { count: culpritCards.length })}
                        </div>
                        {culpritCards.map((card) => (
                          <div key={card.id} className="flex items-center gap-2">
                            <span
                              className={`shrink-0 text-[9px] font-mono px-1 py-0.5 rounded border ${
                                card.severity === 'WARNING'
                                  ? 'border-amber-700/60 text-amber-300'
                                  : 'border-red-800/70 text-red-300'
                              }`}
                            >
                              {card.severity === 'WARNING' ? t('monitor.bisect.warning') : t('monitor.bisect.exception')}
                            </span>
                            <span className="text-[10px] text-slate-300 truncate flex-1" title={card.raw_error}>
                              {cardText(card, 'explanation')}
                            </span>
                            {onPatchMod && fileNamed(card) && (
                              <button
                                type="button"
                                onClick={() => onPatchMod(session!.culprit!, fileNamed(card))}
                                className="shrink-0 text-[10px] px-1.5 py-0.5 rounded border border-amber-700/60 text-amber-200 hover:bg-amber-950/40 cursor-pointer flex items-center gap-1"
                                title={t('monitor.bisect.patchFileTitle', { file: fileNamed(card) })}
                              >
                                <Wrench className="w-3 h-3" aria-hidden="true" />
                                {t('monitor.bisect.patchFile', { file: fileNamed(card) })}
                              </button>
                            )}
                          </div>
                        ))}
                      </>
                    ) : (
                      <div className="text-[10px] text-slate-400 leading-relaxed">{t('monitor.bisect.logNeverNames')}</div>
                    )}
                    {onInspect && (
                      <button
                        type="button"
                        onClick={() => onInspect(culpritMod?.name ?? session!.culprit!)}
                        className="text-[10px] text-cyan-300 hover:text-cyan-200 underline cursor-pointer flex items-center gap-1"
                      >
                        <Search className="w-3 h-3" aria-hidden="true" />
                        {t('monitor.bisect.showLines')}
                      </button>
                    )}
                  </div>

                  <div className="flex items-center gap-2">
                    {onPatchMod && (
                      <button
                        onClick={() => onPatchMod(session!.culprit!, null)}
                        disabled={busy}
                        className="flex-1 flex items-center justify-center gap-1.5 py-1.5 rounded-lg bg-amber-950/50 hover:bg-amber-900/50 border border-amber-800/70 text-amber-200 text-[11px] font-bold transition cursor-pointer disabled:opacity-50"
                        title={t('monitor.bisect.patchAnyTitle')}
                      >
                        <Wrench className="w-3.5 h-3.5" aria-hidden="true" />
                        {t('monitor.bisect.patchAFile')}
                      </button>
                    )}
                    <button
                      onClick={() => handleSwitchOffCulprit(session!.culprit!)}
                      disabled={busy}
                      className="flex-1 py-1.5 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-200 text-[11px] font-bold transition cursor-pointer disabled:opacity-50"
                    >
                      {t('monitor.bisect.switchOff')}
                    </button>
                    <button
                      onClick={() => setSession(null)}
                      className="px-3 py-1.5 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-300 text-[11px] font-medium transition cursor-pointer"
                    >
                      {t('monitor.bisect.keepOn')}
                    </button>
                  </div>
                </>
              ) : (
                <>
                  <div className="text-[11px] text-slate-300 leading-relaxed">{backendSays(session!.note ?? '')}</div>
                  <button
                    onClick={() => setSession(null)}
                    className="w-full py-1.5 rounded-lg bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-300 text-[11px] font-medium transition cursor-pointer"
                  >
                    {t('monitor.bisect.close')}
                  </button>
                </>
              )}
            </>
          )}

          {error && (
            <div className="text-[11px] text-red-300 bg-red-950/40 border border-red-900/70 rounded p-2">
              {error}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
