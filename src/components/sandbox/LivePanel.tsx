import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Activity, ChevronDown, ChevronRight, Gauge, Loader2 } from 'lucide-react';
import { LivePoint, LiveSnapshot, ModInfo, TranslatedErrorCard } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';

interface LivePanelProps {
  paths: StudioPathsUI;
  mods: ModInfo[];
  errorCards: TranslatedErrorCard[];
  /** The mod list changed on disk (the profiler went in). */
  onModsChanged: () => void;
}

const OPEN_KEY = 'pz_monitor_live_open';
const EVERY_MS = 2000;
const TOP = 5;
const gb = (mb: number) => (mb / 1024).toFixed(1);

/**
 * One measure over the last minutes, as a thin line with a hover read-out. One series, one axis:
 * memory gets its own chart, never a second scale on this one.
 */
const LineChart: React.FC<{
  points: LivePoint[];
  value: (p: LivePoint) => number | null;
  format: (v: number) => string;
  title: string;
  floor?: number;
  reference?: { at: number; label: string };
}> = ({ points, value, format, title, floor = 0, reference }) => {
  const { t } = useTranslation();
  const [hover, setHover] = useState<number | null>(null);
  const W = 320;
  const H = 70;
  const values = points.map(value);
  const known = values.filter((v): v is number => v != null);
  if (points.length < 2 || known.length === 0) {
    return <div className="h-[86px] flex items-center text-[11px] text-slate-500">{t('monitor.live.chartWaiting')}</div>;
  }
  const t0 = points[0].t;
  const t1 = points[points.length - 1].t;
  const span = Math.max(1, t1 - t0);
  const max = Math.max(floor, ...known) * 1.1 || 1;
  const x = (i: number) => ((points[i].t - t0) / span) * W;
  const y = (v: number) => H - (v / max) * H;
  // Unknown moments break the line instead of joining across them.
  const segments: string[] = [];
  let current: string[] = [];
  values.forEach((v, i) => {
    if (v == null) {
      if (current.length > 1) segments.push(current.join(' '));
      current = [];
    } else current.push(`${x(i).toFixed(1)},${y(v).toFixed(1)}`);
  });
  if (current.length > 1) segments.push(current.join(' '));
  const minutes = Math.max(1, Math.round(span / 60000));
  const shown = hover != null ? values[hover] : values[values.length - 1];
  const onMove = (e: React.MouseEvent<SVGSVGElement>) => {
    const box = e.currentTarget.getBoundingClientRect();
    const at = t0 + ((e.clientX - box.left) / box.width) * span;
    let best = 0;
    points.forEach((p, i) => {
      if (Math.abs(p.t - at) < Math.abs(points[best].t - at)) best = i;
    });
    setHover(best);
  };
  return (
    <div className="flex flex-col gap-0.5">
      <div className="flex items-baseline justify-between text-[11px]">
        <span className="text-slate-400">{title}</span>
        <span className="font-mono text-slate-200">
          {shown != null ? format(shown) : '-'}
          {hover != null && <span className="text-slate-500"> · {new Date(points[hover].t).toLocaleTimeString()}</span>}
        </span>
      </div>
      <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" className="w-full h-[60px] overflow-visible cursor-crosshair" onMouseMove={onMove} onMouseLeave={() => setHover(null)} role="img" aria-label={title}>
        <line x1={0} y1={H} x2={W} y2={H} stroke="currentColor" className="text-slate-700" strokeWidth={1} vectorEffect="non-scaling-stroke" />
        {reference && reference.at < max && (
          <line x1={0} y1={y(reference.at)} x2={W} y2={y(reference.at)} stroke="currentColor" className="text-slate-600" strokeDasharray="3 3" strokeWidth={1} vectorEffect="non-scaling-stroke" />
        )}
        {segments.map((s, i) => (
          <polyline key={i} points={s} fill="none" stroke="currentColor" className="text-sky-400" strokeWidth={2} strokeLinejoin="round" strokeLinecap="round" vectorEffect="non-scaling-stroke" />
        ))}
        {hover != null && <line x1={x(hover)} y1={0} x2={x(hover)} y2={H} stroke="currentColor" className="text-slate-400" strokeWidth={1} vectorEffect="non-scaling-stroke" />}
      </svg>
      <div className="flex justify-between text-[10px] text-slate-500">
        <span>{t('monitor.live.minutesAgo', { count: minutes })}</span>
        {reference && reference.at < max && <span>{reference.label}</span>}
        <span>{t('monitor.live.now')}</span>
      </div>
    </div>
  );
};

/**
 * Monitor's Live panel (Full mode), while the game runs: FPS over the last minutes, memory and CPU,
 * the mods whose Lua takes the most time now (from the Studio's profiler) and the session's errors
 * as cards come. The same snapshot as the MCP tool game_live and the Studio Link's overlay.
 */
export const LivePanel: React.FC<LivePanelProps> = ({ paths, mods, errorCards, onModsChanged }) => {
  const { t } = useTranslation();
  const [snap, setSnap] = useState<LiveSnapshot | null>(null);
  const [open, setOpen] = useState<boolean>(() => localStorage.getItem(OPEN_KEY) !== 'false');
  const [busy, setBusy] = useState(false);
  // The cards there when the game was first seen running are not "new".
  const seenCards = useRef<Set<string> | null>(null);

  useEffect(() => {
    if (!paths.is_valid) return;
    let cancelled = false;
    let timer: number | undefined;
    const read = async () => {
      const s = await TauriService.gameLive(paths);
      if (cancelled) return;
      setSnap(s);
      timer = window.setTimeout(read, s?.running ? EVERY_MS : 6000);
    };
    void read();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [paths]);

  const running = !!snap?.running;
  useEffect(() => {
    if (!running) seenCards.current = null;
    else if (!seenCards.current) seenCards.current = new Set(errorCards.map((c) => c.id));
  }, [running, errorCards]);

  const exceptions = useMemo(() => errorCards.filter((c) => c.severity === 'EXCEPTION'), [errorCards]);
  const newest = exceptions.slice(-4).reverse();

  if (!snap || !snap.running) return null;

  const toggleOpen = () => {
    const next = !open;
    setOpen(next);
    localStorage.setItem(OPEN_KEY, String(next));
  };
  const turnOn = async () => {
    setBusy(true);
    try {
      await TauriService.setProfiler(paths, true);
      setSnap((s) => (s ? { ...s, profiler_on: true } : s));
      onModsChanged();
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(false);
    }
  };

  const link = snap.link;
  const prof = snap.profiler && snap.profiler.in_game ? snap.profiler : null;
  const fps = link && link.fps > 0 ? link.fps : prof?.fps ?? null;
  const frameMs = link && link.fps > 0 ? link.frame_ms : prof?.frame_ms ?? null;
  const worst = link ? link.worst_ms : prof?.worst_ms ?? null;
  const nameOf = (id: string | null, name: string) => (id ? mods.find((m) => m.mod_id === id)?.name ?? (name || id) : t('monitor.live.theGame'));
  const topMods = prof?.mods.slice(0, TOP) ?? [];
  const topShare = topMods[0]?.ms_per_s ?? 0;

  return (
    <section className="rounded-xl border border-emerald-900/60 bg-slate-900/60 mb-3 shrink-0">
      <button type="button" onClick={toggleOpen} className="w-full flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2 cursor-pointer text-left">
        {open ? <ChevronDown className="w-4 h-4 text-slate-400" aria-hidden="true" /> : <ChevronRight className="w-4 h-4 text-slate-400" aria-hidden="true" />}
        <Activity className="w-4 h-4 text-emerald-400" aria-hidden="true" />
        <span className="text-sm font-bold text-slate-100">{t('monitor.live.title')}</span>
        <span className="px-1.5 py-0.5 rounded border border-emerald-700 text-emerald-300 text-[10px] font-bold uppercase tracking-wide">{t('monitor.live.badge')}</span>
        <span className="text-[11.5px] font-mono text-slate-300">
          {[
            fps != null ? t('monitor.live.sumFps', { fps: Math.round(fps) }) : null,
            link?.heap_mb != null && snap.ram_max_mb ? t('monitor.live.sumHeap', { used: gb(link.heap_mb), max: gb(snap.ram_max_mb) }) : snap.ram_mb != null ? t('monitor.live.sumRam', { gb: gb(snap.ram_mb) }) : null,
            snap.cpu_percent != null ? t('monitor.live.sumCpu', { pct: Math.round(snap.cpu_percent) }) : null,
            link ? t('monitor.live.sumErrors', { count: link.errors }) : null,
          ]
            .filter(Boolean)
            .join('  ·  ')}
        </span>
      </button>

      {open && (
        <div className="grid gap-4 px-4 pb-3 lg:grid-cols-3">
          {/* Frames and memory */}
          <div className="flex flex-col gap-2 min-w-0">
            <LineChart
              points={snap.history}
              value={(p) => p.fps}
              format={(v) => t('monitor.live.fpsValue', { fps: Math.round(v) })}
              title={t('monitor.live.fpsTitle')}
              floor={60}
              reference={{ at: 30, label: t('monitor.live.fps30') }}
            />
            <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-[11.5px]">
              <dt className="text-slate-500">{t('monitor.live.frame')}</dt>
              <dd className="font-mono text-slate-200">{frameMs != null && fps != null ? t('monitor.live.frameValue', { ms: frameMs.toFixed(1), worst: worst ?? 0 }) : '-'}</dd>
              <dt className="text-slate-500">{t('monitor.live.heap')}</dt>
              <dd className="font-mono text-slate-200">
                {link?.heap_mb != null ? (snap.ram_max_mb ? t('monitor.live.heapValue', { used: gb(link.heap_mb), max: gb(snap.ram_max_mb) }) : t('monitor.live.heapNoLimit', { used: gb(link.heap_mb) })) : t('monitor.live.heapUnknown')}
              </dd>
              <dt className="text-slate-500">{t('monitor.live.process')}</dt>
              <dd className="font-mono text-slate-200">{snap.ram_mb != null ? t('monitor.live.processValue', { gb: gb(snap.ram_mb), free: gb(snap.pc_free_mb) }) : '-'}</dd>
              <dt className="text-slate-500">{t('monitor.live.cpu')}</dt>
              <dd className="font-mono text-slate-200">{snap.cpu_percent != null ? `${snap.cpu_percent.toFixed(0)} %` : '-'}</dd>
            </dl>
            <LineChart points={snap.history} value={(p) => (p.heap_mb ?? p.ram_mb ?? null)} format={(v) => `${gb(v)} GB`} title={snap.history.some((p) => p.heap_mb != null) ? t('monitor.live.heapTitle') : t('monitor.live.ramTitle')} />
          </div>

          {/* Mods' Lua time now */}
          <div className="flex flex-col gap-2 min-w-0">
            <div className="flex items-center gap-2 text-[11px]">
              <Gauge className="w-3.5 h-3.5 text-sky-400" aria-hidden="true" />
              <span className="font-bold text-slate-200">{t('monitor.live.luaTitle')}</span>
              {prof && <span className="ml-auto font-mono text-slate-400">{t('monitor.live.luaTotal', { ms: prof.total_ms_per_s.toFixed(1), pct: prof.share_percent.toFixed(1) })}</span>}
            </div>
            {prof ? (
              topMods.length > 0 ? (
                <div className="flex flex-col gap-1">
                  {topMods.map((m) => (
                    <div key={m.mod_id ?? 'game'} className="flex items-center gap-2 text-[11.5px]" title={t('monitor.live.luaRowTip', { calls: m.calls, worst: m.worst })}>
                      <span className="w-40 truncate text-slate-100">{nameOf(m.mod_id, m.name)}</span>
                      <span className="flex-1 h-1.5 rounded bg-slate-800 overflow-hidden">
                        <span className={`block h-full rounded ${m.ms_per_s >= 5 ? 'bg-amber-500' : 'bg-sky-500'}`} style={{ width: `${topShare > 0 ? Math.max(2, (m.ms_per_s / topShare) * 100) : 0}%` }} />
                      </span>
                      <span className="w-20 text-right font-mono text-slate-200">{t('monitor.live.msPerS', { ms: m.ms_per_s.toFixed(1) })}</span>
                    </div>
                  ))}
                  <div className="text-[10.5px] text-slate-500">{t('monitor.live.luaHow', { cost: prof.cost_ms_per_s.toFixed(2), handlers: prof.handlers })}</div>
                </div>
              ) : (
                <div className="text-[11px] text-slate-500">{t('monitor.live.luaNone')}</div>
              )
            ) : (
              <div className="rounded-lg border border-slate-800 bg-slate-950/50 px-3 py-2 flex flex-col gap-2 text-[11.5px]">
                <span className="text-slate-300">{snap.profiler_on ? t('monitor.live.profilerNext') : t('monitor.live.profilerOff')}</span>
                {!snap.profiler_on && (
                  <button
                    type="button"
                    onClick={() => void turnOn()}
                    disabled={busy}
                    className="self-start flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-sky-600 hover:bg-sky-500 text-white cursor-pointer disabled:opacity-60"
                  >
                    {busy && <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />}
                    {t('monitor.live.profilerTurnOn')}
                  </button>
                )}
              </div>
            )}
          </div>

          {/* Errors as they come */}
          <div className="flex flex-col gap-2 min-w-0">
            <div className="text-[11px] font-bold text-slate-200">{t('monitor.live.errorsTitle')}</div>
            {link ? (
              link.errors > 0 ? (
                <div className="flex flex-col gap-0.5 text-[11.5px]">
                  <span className="text-red-300 font-bold">{t('monitor.live.errorsCount', { count: link.errors })}</span>
                  {link.error_mods.slice(0, 3).map((g) => (
                    <span key={g.name} className="text-slate-300 truncate">
                      {g.name || t('monitor.live.theGame')} <span className="font-mono text-slate-500">×{g.count}</span>
                    </span>
                  ))}
                </div>
              ) : (
                <span className="text-[11.5px] text-emerald-300">{t('monitor.live.errorsNone')}</span>
              )
            ) : (
              <span className="text-[11px] text-slate-500">{t('monitor.live.noLink')}</span>
            )}
            {newest.length > 0 && (
              <div className="flex flex-col gap-1">
                {newest.map((c) => {
                  const fresh = seenCards.current != null && !seenCards.current.has(c.id);
                  return (
                    <div key={c.id} className={`rounded-md border px-2 py-1 text-[11px] ${fresh ? 'border-red-700/70 bg-red-950/30' : 'border-slate-800 bg-slate-950/40'}`}>
                      <div className="flex items-center gap-1.5">
                        {fresh && <span className="px-1 rounded bg-red-800 text-red-100 text-[9.5px] font-bold uppercase">{t('monitor.live.newCard')}</span>}
                        <span className="truncate text-slate-100">{c.title}</span>
                      </div>
                      {c.mod_name && <div className="truncate text-slate-400">{c.mod_name}</div>}
                    </div>
                  );
                })}
              </div>
            )}
            {link && (
              <div className="text-[10.5px] text-slate-500 mt-auto">
                {link.overlay ? t('monitor.live.overlayOn', { cost: link.cost_ms_per_s.toFixed(2) }) : t('monitor.live.overlayOff', { cost: link.cost_ms_per_s.toFixed(2) })}
              </div>
            )}
          </div>
        </div>
      )}
    </section>
  );
};
