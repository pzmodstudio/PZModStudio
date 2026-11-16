import React, { useEffect, useState } from 'react';
import { Cpu, MemoryStick } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { TauriService } from '../../services/tauri';
import { StudioPathsUI } from '../settings/SettingsModule';
import { GameResources } from '../../types';

/** GB with one decimal, the way the meter says it. */
const gb = (mb: number) => (mb / 1024).toFixed(1);

/**
 * How tight memory is, by what the PC has left: amber under a fifth free, red under a tenth.
 * Not the process against its Java limit: the process holds far more than its Java heap (the JVM,
 * textures, map data; 11.6 GB beside a 6 GB limit, measured), so that comparison says nothing.
 */
export const meterTone = (pcFree: number, pcTotal: number): 'ok' | 'high' | 'full' => {
  if (pcTotal <= 0) return 'ok';
  const share = pcFree / pcTotal;
  return share < 0.1 ? 'full' : share < 0.2 ? 'high' : 'ok';
};
const TONE_TEXT = { ok: 'text-emerald-300', high: 'text-amber-300', full: 'text-red-300' } as const;
const TONE_BAR = { ok: 'bg-emerald-500', high: 'bg-amber-500', full: 'bg-red-500' } as const;

/**
 * What the game (and the Studio's server) take from the PC, read every few seconds while one runs:
 * memory against its Java limit, CPU, and what the PC has left. `compact`: one line for the top
 * bar, hidden while nothing runs; otherwise the fuller version for Mod Health's Performance tab.
 */
export const ResourceMeter: React.FC<{ paths: StudioPathsUI; compact?: boolean; onOpen?: () => void }> = ({ paths, compact = false, onOpen }) => {
  const { t } = useTranslation();
  const [res, setRes] = useState<GameResources | null>(null);
  useEffect(() => {
    if (!paths.is_valid) return;
    let cancelled = false;
    let timer: number | undefined;
    const tick = async () => {
      const r = await TauriService.gameResources(paths);
      if (cancelled) return;
      setRes(r);
      // Quicker while something runs; a slow look otherwise, to notice the game starting.
      timer = window.setTimeout(tick, r && r.processes.length > 0 ? 3000 : 8000);
    };
    void tick();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [paths]);

  const procs = res?.processes ?? [];
  const label = (kind: string, n: number) => (kind === 'server' ? t('meter.server') : procs.filter((p) => p.kind === 'game').length > 1 ? t('meter.gameN', { n }) : t('meter.game'));
  const pcTitle = res ? t('meter.pcFree', { free: gb(res.pc_free_mb), total: gb(res.pc_total_mb) }) : '';

  if (compact) {
    if (procs.length === 0) return null;
    let gameN = 0;
    return (
      <button
        type="button"
        onClick={onOpen}
        title={`${t('meter.title')}\n${pcTitle}`}
        className="flex items-center gap-2.5 px-3 py-1 rounded-full bg-slate-950 border border-slate-800 hover:border-sky-600/60 cursor-pointer font-mono text-[11px]"
      >
        {procs.map((p) => {
          if (p.kind === 'game') gameN += 1;
          const tone = res ? meterTone(res.pc_free_mb, res.pc_total_mb) : 'ok';
          return (
            <span key={p.pid} className="flex items-center gap-1">
              <span className="text-slate-400">{label(p.kind, gameN)}</span>
              <span className={`font-bold ${TONE_TEXT[tone]}`}>{gb(p.ram_mb)} GB</span>
              {p.cpu_percent != null && <span className="text-slate-400">· {Math.round(p.cpu_percent)}%</span>}
            </span>
          );
        })}
      </button>
    );
  }

  let gameN = 0;
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/60 p-3 flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2 text-xs">
        <span className="font-bold text-slate-200">{t('meter.title')}</span>
        {res && <span className={`text-[11px] ${TONE_TEXT[meterTone(res.pc_free_mb, res.pc_total_mb)]}`}>{pcTitle}</span>}
      </div>
      {procs.length === 0 ? (
        <div className="text-[11px] text-slate-400">{t('meter.nothingRunning')}</div>
      ) : (
        procs.map((p) => {
          if (p.kind === 'game') gameN += 1;
          const tone = res ? meterTone(res.pc_free_mb, res.pc_total_mb) : 'ok';
          const pct = res && res.pc_total_mb > 0 ? Math.min(100, Math.round((p.ram_mb / res.pc_total_mb) * 100)) : null;
          return (
            <div key={p.pid} className="flex flex-col gap-1">
              <div className="flex flex-wrap items-center gap-3 text-[11px]">
                <span className="font-bold text-slate-200 w-24">{label(p.kind, gameN)}</span>
                <span className={`flex items-center gap-1 ${TONE_TEXT[tone]}`}>
                  <MemoryStick className="w-3.5 h-3.5" aria-hidden="true" />
                  {t('meter.ram', { used: gb(p.ram_mb) })}{p.ram_max_mb ? <span className="text-slate-400">{t('meter.javaLimit', { max: gb(p.ram_max_mb) })}</span> : null}
                </span>
                <span className="flex items-center gap-1 text-slate-300">
                  <Cpu className="w-3.5 h-3.5" aria-hidden="true" />
                  {p.cpu_percent != null ? t('meter.cpu', { pct: Math.round(p.cpu_percent) }) : t('meter.cpuReading')}
                </span>
                <span className="text-slate-500 font-mono">PID {p.pid}</span>
              </div>
              {pct != null && (
                <div className="h-1.5 rounded bg-slate-800 overflow-hidden">
                  <div className={`h-full ${TONE_BAR[tone]} transition-[width] duration-700`} style={{ width: `${pct}%` }} />
                </div>
              )}
            </div>
          );
        })
      )}
      {res && procs.length > 0 && meterTone(res.pc_free_mb, res.pc_total_mb) !== 'ok' && (
        <div className={`text-[10.5px] ${TONE_TEXT[meterTone(res.pc_free_mb, res.pc_total_mb)]}`}>
          {t(meterTone(res.pc_free_mb, res.pc_total_mb) === 'full' ? 'meter.nearFull' : 'meter.high')}
        </div>
      )}
    </div>
  );
};
