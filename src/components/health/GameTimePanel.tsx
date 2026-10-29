import React, { useCallback, useEffect, useState } from 'react';
import { Gauge, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import { ModCost, ModInfo, ProfilerStatus } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { RichText } from '../common/RichText';

interface GameTimePanelProps {
  paths: StudioPathsUI;
  mods: ModInfo[];
  /** The mod list changed on disk (the profiler went in or out). */
  onModsChanged: () => void;
}

/** Mods shown before "Show all". */
const FIRST_ROWS = 12;
/** A single call this long is a hitch the player sees. */
const HITCH_MS = 100;

/**
 * Which mod costs the game the most time while it runs, from the Studio's own in-game profiler
 * (mods/PZModStudio_Profiler): every event handler of every mod, clocked by the game, summed
 * per mod. Switching it on puts the profiler first in the mod list; the numbers arrive while
 * the game runs and stay after it closes.
 */
export const GameTimePanel: React.FC<GameTimePanelProps> = ({ paths, mods, onModsChanged }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision(['mods']);
  const [status, setStatus] = useState<ProfilerStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [showAll, setShowAll] = useState(false);

  const load = useCallback(async () => {
    setStatus(await TauriService.profilerStatus(paths));
  }, [paths]);

  useEffect(() => {
    void load();
    // The profiler writes every 5 s while the game runs.
    const timer = window.setInterval(() => void load(), 5000);
    return () => window.clearInterval(timer);
  }, [load, changeRev]);

  const toggle = async () => {
    if (!status) return;
    setBusy(true);
    try {
      setStatus(await TauriService.setProfiler(paths, !status.on));
      onModsChanged();
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(false);
    }
  };

  const report = status?.report ?? null;
  const window_s = report ? report.window_ms / 1000 : 0;
  const share = (ms: number) => (report && report.window_ms > 0 ? (ms / report.window_ms) * 100 : 0);
  const nameOf = (m: ModCost) => {
    if (!m.mod_id) return t('gameTime.theGame');
    return mods.find((x) => x.mod_id === m.mod_id)?.name ?? m.name ?? m.mod_id;
  };
  const rows = report ? (showAll ? report.mods : report.mods.slice(0, FIRST_ROWS)) : [];
  const live = report !== null && report.age_secs < 15;

  return (
    <section className="flex flex-col gap-3 p-5 rounded-xl border border-slate-800 bg-slate-900/60">
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-start gap-3">
          <Gauge className="w-5 h-5 text-sky-400 mt-0.5" aria-hidden="true" />
          <div>
            <h2 className="text-sm font-bold text-slate-100">{t('gameTime.title')}</h2>
            <RichText text={t('gameTime.blurb')} foldAt={260} className="text-xs text-slate-400 max-w-2xl" />
          </div>
        </div>
        <button
          type="button"
          onClick={() => void toggle()}
          disabled={busy || !status}
          className={`flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-bold shrink-0 cursor-pointer disabled:opacity-60 ${
            status?.on ? 'bg-slate-800 hover:bg-slate-700 text-slate-200' : 'bg-sky-600 hover:bg-sky-500 text-white'
          }`}
        >
          {busy && <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />}
          {status?.on ? t('gameTime.stop') : t('gameTime.start')}
        </button>
      </div>

      {status?.on && (
        <div className="text-xs text-sky-200 rounded-lg border border-sky-900/60 bg-sky-950/30 px-3 py-2">
          {status.saves.length > 0 ? t('gameTime.onWithSave', { save: status.saves.join(', ') }) : t('gameTime.onNoSave')}
        </div>
      )}

      {report && report.mods.length > 0 && (
        <>
          <div className="flex flex-wrap gap-2 text-[11px] font-mono">
            <span className={`px-2 py-0.5 rounded border ${live ? 'border-emerald-700 text-emerald-300' : 'border-slate-700 text-slate-400'}`}>
              {live ? t('gameTime.live') : t('gameTime.lastRun')}
            </span>
            <span className="px-2 py-0.5 rounded border border-slate-700 text-slate-300">
              {report.in_game ? t('gameTime.inSave', { min: (window_s / 60).toFixed(1) }) : t('gameTime.atMenu', { min: (window_s / 60).toFixed(1) })}
            </span>
            <span className={`px-2 py-0.5 rounded border ${share(report.total_ms) >= 10 ? 'border-amber-600 text-amber-300' : 'border-slate-700 text-slate-300'}`}>
              {t('gameTime.total', { pct: share(report.total_ms).toFixed(1) })}
            </span>
          </div>

          <div className="flex flex-col gap-1">
            {rows.map((m) => {
              const pct = share(m.ms);
              const top = report.mods[0] ? share(report.mods[0].ms) : 0;
              const key = m.mod_id ?? 'game';
              return (
                <details key={key} className="rounded-lg border border-slate-800 bg-slate-950/50">
                  <summary className="flex items-center gap-3 px-3 py-1.5 cursor-pointer select-none text-xs">
                    <span className="w-48 truncate text-slate-100 font-semibold" title={m.mod_id ?? ''}>
                      {nameOf(m)}
                    </span>
                    <span className="flex-1 h-1.5 rounded bg-slate-800 overflow-hidden">
                      <span
                        className={`block h-full ${pct >= 5 ? 'bg-amber-500' : 'bg-sky-500'}`}
                        style={{ width: `${top > 0 ? Math.max(1, (pct / top) * 100) : 0}%` }}
                      />
                    </span>
                    <span className="w-16 text-right font-mono text-slate-200">{pct.toFixed(pct < 1 ? 2 : 1)} %</span>
                    <span className="w-24 text-right font-mono text-slate-400">{t('gameTime.perSecond', { ms: (m.ms / Math.max(window_s, 1)).toFixed(1) })}</span>
                    <span className={`w-28 text-right font-mono ${m.worst >= HITCH_MS ? 'text-amber-300' : 'text-slate-500'}`}>
                      {t('gameTime.worst', { ms: m.worst })}
                    </span>
                  </summary>
                  <div className="px-3 pb-2 pt-1 flex flex-col gap-0.5">
                    {m.via_package && <div className="text-[11px] text-slate-400">{t('gameTime.viaPackage')}</div>}
                    {m.handlers.map((h) => (
                      <div key={`${h.event}|${h.file}|${h.line}`} className="flex items-center gap-3 text-[11px] font-mono text-slate-400">
                        <span className="w-36 truncate text-sky-300">{h.event}</span>
                        <span className="flex-1 truncate" title={`${h.file}:${h.line}`}>
                          {h.file}:{h.line}
                        </span>
                        <span className="w-20 text-right">{h.ms} ms</span>
                        <span className="w-28 text-right">{t('gameTime.calls', { count: h.calls })}</span>
                        <span className={`w-24 text-right ${h.worst >= HITCH_MS ? 'text-amber-300' : ''}`}>{t('gameTime.worst', { ms: h.worst })}</span>
                      </div>
                    ))}
                  </div>
                </details>
              );
            })}
          </div>
          {report.mods.length > FIRST_ROWS && (
            <button type="button" onClick={() => setShowAll(!showAll)} className="self-start text-[11px] text-sky-300 hover:text-sky-200 cursor-pointer">
              {showAll ? t('gameTime.showFewer') : t('gameTime.showAll', { count: report.mods.length })}
            </button>
          )}
          <RichText text={t('gameTime.how')} foldAt={200} className="text-[11px] text-slate-500" />
        </>
      )}

      {status && !report && status.on && <div className="text-xs text-slate-400">{t('gameTime.waiting')}</div>}
    </section>
  );
};
