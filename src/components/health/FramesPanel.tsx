import React, { useCallback, useEffect, useState } from 'react';
import { Activity, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ModInfo, ProfileReport, SmoothFramesReport } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';

interface FramesPanelProps {
  paths: StudioPathsUI;
  mods: ModInfo[];
}

/** Mods shown before "Show all". */
const FIRST_ROWS = 12;
/** The Workshop item of Viewpoint - Smooth Frames, which writes the numbers. */
const SMOOTH_FRAMES_ITEM = '3816784011';

/**
 * Frame times and per-mod Lua costs of the current session, read from what Viewpoint's Smooth
 * Frames mod writes to the console once a minute: nothing to switch on, nothing added to the
 * list. Below the Studio's own profiler, which measures every mod but has to be put in the list.
 */
export const FramesPanel: React.FC<FramesPanelProps> = ({ paths, mods }) => {
  const { t } = useTranslation();
  const [report, setReport] = useState<SmoothFramesReport | null | undefined>(undefined);
  /** The Studio's own profiler, for the frames it counts when Smooth Frames is not there. */
  const [own, setOwn] = useState<ProfileReport | null>(null);
  const [showAll, setShowAll] = useState(false);

  const load = useCallback(async () => {
    const [smooth, status] = await Promise.all([TauriService.smoothFrames(paths), TauriService.profilerStatus(paths)]);
    setReport(smooth);
    setOwn(status.report);
  }, [paths]);

  useEffect(() => {
    void load();
    // The mod writes once a minute while the game runs.
    const timer = window.setInterval(() => void load(), 30000);
    return () => window.clearInterval(timer);
  }, [load]);

  const nameOf = (id: string) => (id === 'vanilla' ? t('frames.theGame') : mods.find((m) => m.mod_id === id)?.name ?? id);
  const hasMod = mods.some((m) => m.workshop_id === SMOOTH_FRAMES_ITEM || m.mod_id === 'ViewpointSmoothFrames');
  const rows = report ? (showAll ? report.mods : report.mods.slice(0, FIRST_ROWS)) : [];
  const ms = (n: number) => n.toFixed(n >= 10 ? 0 : 1);

  return (
    <section className="flex flex-col gap-3 p-5 rounded-xl border border-slate-800 bg-slate-900/60">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
            <Activity className="w-4 h-4 text-sky-400" aria-hidden="true" />
            {t('frames.title')}
          </h3>
          <p className="text-[11px] text-slate-400 mt-1 max-w-3xl">{t('frames.blurb')}</p>
        </div>
        <button type="button" onClick={() => void load()} className="text-[11px] text-slate-400 hover:text-slate-200 cursor-pointer">
          {t('frames.refresh')}
        </button>
      </div>

      {report === undefined ? (
        <div className="flex items-center gap-2 text-[11px] text-slate-500"><Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />{t('frames.reading')}</div>
      ) : report === null ? (
        own?.frames && own.frames.frames > 0 ? (
          <>
            {/* The profiler's own count: the same numbers, without Smooth Frames. */}
            <div className="flex flex-wrap gap-x-5 gap-y-1 text-[11px] font-mono text-slate-300">
              <span>{t('frames.ownFrames', { count: own.frames.frames, min: Math.round(own.window_ms / 60000) })}</span>
              <span>{t('frames.fps', { avg: own.frames.fps_avg.toFixed(1), low: own.frames.low1_fps.toFixed(1) })}</span>
              <span className={own.frames.over100 > 0 ? 'text-amber-300' : ''}>{t('frames.hitches', { over25: own.frames.over25, over50: own.frames.over50, over100: own.frames.over100 })}</span>
              <span>{t('frames.percentiles', { p50: own.frames.p50, p95: own.frames.p95, p99: own.frames.p99, max: own.frames.max_ms })}</span>
              <span>{t('frames.ownChunks', { chunks: own.frames.chunks, squares: own.frames.squares })}</span>
            </div>
            {own.mods.some((m) => m.worst >= 25) && (
              <table className="w-full text-[11px]">
                <thead>
                  <tr className="text-left text-[10px] uppercase tracking-wider text-slate-500 font-mono">
                    <th className="py-1 pr-2 font-normal">{t('frames.col.mod')}</th>
                    <th className="py-1 pr-2 font-normal text-right" title={t('frames.col.worstTitle')}>{t('frames.col.worst')}</th>
                    <th className="py-1 font-normal">{t('frames.col.where')}</th>
                  </tr>
                </thead>
                <tbody>
                  {[...own.mods].filter((m) => m.worst >= 25).sort((a, b) => b.worst - a.worst).slice(0, FIRST_ROWS).map((m) => {
                    const h = [...m.handlers].sort((a, b) => b.worst - a.worst)[0];
                    return (
                      <tr key={m.mod_id ?? 'game'} className="border-t border-slate-800/80">
                        <td className="py-1.5 pr-2 text-slate-200">{m.mod_id ? mods.find((x) => x.mod_id === m.mod_id)?.name ?? m.name : t('frames.theGame')}</td>
                        <td className="py-1.5 pr-2 text-right font-mono text-amber-300">{m.worst}</td>
                        <td className="py-1.5 font-mono text-[10px] text-slate-400">{h ? `${h.event} · ${h.file.split('/').pop()}:${h.line}` : ''}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            )}
            <p className="text-[10px] text-slate-500">{t('frames.ownNote')}</p>
          </>
        ) : (
          <p className="text-[11px] text-slate-500">{hasMod ? t('frames.nothingYet') : own ? t('frames.needsPlay') : t('frames.needsMod')}</p>
        )
      ) : (
        <>
          <div className="flex flex-wrap gap-x-5 gap-y-1 text-[11px] font-mono text-slate-300">
            <span title={t('frames.minutesTitle')}>{t('frames.minutes', { count: report.minutes })}</span>
            <span>{t('frames.fps', { avg: report.fps_avg.toFixed(1), low: report.low1_min.toFixed(1) })}</span>
            <span className={report.over100 > 0 ? 'text-amber-300' : ''}>{t('frames.hitches', { over25: report.over25, over50: report.over50, over100: report.over100 })}</span>
            <span>{t('frames.lua', { ms: report.lua_ms_per_s_avg.toFixed(0), pct: (report.lua_ms_per_s_avg / 10).toFixed(0) })}</span>
            <span>{t('frames.chunks', { done: report.chunks_done, deferred: report.chunks_deferred })}</span>
            <span>{t('frames.gc', { ms: Math.round(report.gc_ms / Math.max(1, report.minutes)) })}</span>
          </div>

          {report.mods.length > 0 && (
            <table className="w-full text-[11px]">
              <thead>
                <tr className="text-left text-[10px] uppercase tracking-wider text-slate-500 font-mono">
                  <th className="py-1 pr-2 font-normal">{t('frames.col.mod')}</th>
                  <th className="py-1 pr-2 font-normal text-right" title={t('frames.col.avgTitle')}>{t('frames.col.avg')}</th>
                  <th className="py-1 pr-2 font-normal text-right" title={t('frames.col.maxTitle')}>{t('frames.col.max')}</th>
                  <th className="py-1 pr-2 font-normal text-right" title={t('frames.col.worstTitle')}>{t('frames.col.worst')}</th>
                  <th className="py-1 pr-2 font-normal text-right" title={t('frames.col.hitchesTitle')}>{t('frames.col.hitches')}</th>
                  <th className="py-1 font-normal">{t('frames.col.where')}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((m) => {
                  const top = m.top[0];
                  return (
                    <tr key={m.mod_id} className="border-t border-slate-800/80 align-top">
                      <td className="py-1.5 pr-2 text-slate-200">
                        {nameOf(m.mod_id)}
                        <span className="block text-[10px] text-slate-500 font-mono">{t('frames.inWindows', { n: m.windows, total: report.minutes })}</span>
                      </td>
                      <td className="py-1.5 pr-2 text-right font-mono text-slate-100">{ms(m.ms_per_s_avg)}</td>
                      <td className="py-1.5 pr-2 text-right font-mono text-slate-400">{ms(m.ms_per_s_max)}</td>
                      <td className={`py-1.5 pr-2 text-right font-mono ${m.worst_frame_ms >= 25 ? 'text-amber-300' : 'text-slate-400'}`}>{ms(m.worst_frame_ms)}</td>
                      <td className={`py-1.5 pr-2 text-right font-mono ${m.hitches > 0 ? 'text-amber-300' : 'text-slate-500'}`}>{m.hitches}</td>
                      <td className="py-1.5 font-mono text-[10px] text-slate-400">
                        {top && (
                          <span title={m.top.map((h) => `${h.event} ${ms(h.ms_per_s)} ms/s · ${h.file}:${h.line} ${h.func}`).join('\n')}>
                            {top.event} · {top.file.split('/').pop()}:{top.line}{top.func ? ` ${top.func}` : ''}
                          </span>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
          {report.mods.length > FIRST_ROWS && (
            <button type="button" onClick={() => setShowAll((v) => !v)} className="self-start text-[11px] text-sky-300 hover:text-sky-200 cursor-pointer">
              {showAll ? t('frames.showLess') : t('frames.showAll', { count: report.mods.length })}
            </button>
          )}
          <p className="text-[10px] text-slate-500">{t('frames.note')}</p>
        </>
      )}
    </section>
  );
};
