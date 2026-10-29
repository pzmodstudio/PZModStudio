import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import { ListOrdered, Map as MapIcon, Maximize, Power, RefreshCw, TriangleAlert } from 'lucide-react';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { MapCells, MapImage } from '../../types';

interface MapsModuleProps {
  paths: StudioPathsUI;
  /** The mods on now, joined: when it changes the maps are read again. */
  activeKey: string;
  /** Opens the Mod List on this mod, highlighted. */
  onShowMod: (modId: string) => void;
  /** Turns this mod off in the list (asks first). */
  onTurnOff: (modId: string) => void;
}

const key = (c: [number, number]) => `${c[0]},${c[1]}`;
/** A colour per map, far apart on the wheel, never red (red is for cells two maps share). */
const colour = (i: number, alpha = 0.85) => `hsla(${45 + ((i * 137.5) % 255)}, 70%, 55%, ${alpha})`;
const VANILLA = 'rgba(71, 85, 105, 0.9)';
const CLASH = 'rgba(239, 68, 68, 1)';
const SQUARES_PER_CELL = 256;
/** The tick "only mods on in my list", kept between visits. */
const ONLY_ACTIVE_KEY = 'pz_maps_only_active';

/** Pixels per cell, the farthest and the closest the wheel goes. */
const MIN_SCALE = 2;
const MAX_SCALE = 64;
const VIEW_HEIGHT = 560;
/** How much of the view the maps always cover while panning (or all of them, when smaller). */
const KEEP_IN_VIEW = 0.5;

/** Where the grid is on the canvas: a cell (x, y) is drawn at ((x - x0) * scale + ox, (y - y0) * scale + oy). */
interface View {
  scale: number;
  ox: number;
  oy: number;
}

/**
 * Every cell of the world the game ships and each installed map mod covers, on one grid over the
 * real map: which maps replace parts of the game's world, and where two maps fight over the same
 * cells. The wheel zooms where the pointer is, dragging pans. Below, the mods about maps with no
 * cells of their own (spawn points, tiles).
 */
export const MapsModule: React.FC<MapsModuleProps> = ({ paths, activeKey, onShowMod, onTurnOff }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision();
  const [data, setData] = useState<MapCells | null>(null);
  const [loading, setLoading] = useState(false);
  const [onlyActive, setOnlyActive] = useState<boolean>(() => {
    try {
      return localStorage.getItem(ONLY_ACTIVE_KEY) === '1';
    } catch {
      return false;
    }
  });
  const pickOnlyActive = (next: boolean) => {
    setOnlyActive(next);
    try {
      localStorage.setItem(ONLY_ACTIVE_KEY, next ? '1' : '0');
    } catch {
      // only a convenience
    }
  };
  // The wheel over the map scrolls the page like anywhere else until the map is clicked; from
  // then on, until the pointer leaves it, the wheel zooms the map.
  const [armed, setArmed] = useState(false);
  const armedRef = useRef(false);
  const arm = (on: boolean) => {
    armedRef.current = on;
    setArmed(on);
  };
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  const [hover, setHover] = useState<{ x: number; y: number; px: number; py: number } | null>(null);
  const [images, setImages] = useState<{ img: MapImage; el: HTMLImageElement }[]>([]);
  const [imagesLoading, setImagesLoading] = useState(false);
  const [width, setWidth] = useState(800);
  const [view, setView] = useState<View | null>(null);
  const [dragging, setDragging] = useState(false);
  const canvas = useRef<HTMLCanvasElement | null>(null);
  const box = useRef<HTMLDivElement | null>(null);
  const drag = useRef<{ x: number; y: number; ox: number; oy: number } | null>(null);

  const load = async () => {
    if (!paths.user_zomboid_dir) return;
    setLoading(true);
    setData(await TauriService.mapCells(paths));
    setLoading(false);
  };
  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths, activeKey, changeRev]);

  const maps = useMemo(() => (data?.maps ?? []).map((m, i) => ({ ...m, idx: i })).filter((m) => !onlyActive || m.active), [data, onlyActive]);
  const shown = useMemo(() => maps.filter((m) => !hidden.has(`${m.mod_id}/${m.folder}`)), [maps, hidden]);
  const clashes = useMemo(() => (data?.clashes ?? []).filter((c) => !onlyActive || c.both_active), [data, onlyActive]);
  const related = (data?.related ?? []).filter((r) => !onlyActive || r.active);
  const tiledefClashes = (data?.tiledef_clashes ?? []).filter((c) => !onlyActive || c.both_active);

  // Who is on each cell, for the drawing and the hover.
  const byCell = useMemo(() => {
    const out = new Map<string, string[]>();
    for (const m of shown) for (const c of m.cells) out.set(key(c), [...(out.get(key(c)) ?? []), m.title]);
    return out;
  }, [shown]);
  const vanillaSet = useMemo(() => new Set((data?.vanilla ?? []).map(key)), [data]);
  const clashSet = useMemo(() => new Set(clashes.flatMap((c) => c.cells.map(key))), [clashes]);

  // The world plus every map mod, so a map does not jump when another is hidden.
  const bounds = useMemo(() => {
    const all = [...(data?.vanilla ?? []), ...(data?.maps ?? []).flatMap((m) => m.cells)];
    if (all.length === 0) return null;
    const xs = all.map((c) => c[0]);
    const ys = all.map((c) => c[1]);
    return { x0: Math.min(...xs), y0: Math.min(...ys), x1: Math.max(...xs), y1: Math.max(...ys) };
  }, [data]);

  const fit = useCallback((): View | null => {
    if (!bounds) return null;
    const cw = bounds.x1 - bounds.x0 + 1;
    const ch = bounds.y1 - bounds.y0 + 1;
    const scale = Math.max(MIN_SCALE, Math.min((width - 16) / cw, (VIEW_HEIGHT - 16) / ch));
    return { scale, ox: (width - cw * scale) / 2, oy: (VIEW_HEIGHT - ch * scale) / 2 };
  }, [bounds, width]);
  useEffect(() => {
    if (!view) setView(fit());
  }, [fit, view]);

  /** The view moved or zoomed, kept so some of the maps is always on screen. */
  const clamp = useCallback(
    (v: View): View => {
      if (!bounds) return v;
      const w = (bounds.x1 - bounds.x0 + 1) * v.scale;
      const h = (bounds.y1 - bounds.y0 + 1) * v.scale;
      const keepX = Math.min(width * KEEP_IN_VIEW, w);
      const keepY = Math.min(VIEW_HEIGHT * KEEP_IN_VIEW, h);
      return { ...v, ox: Math.max(keepX - w, Math.min(width - keepX, v.ox)), oy: Math.max(keepY - h, Math.min(VIEW_HEIGHT - keepY, v.oy)) };
    },
    [bounds, width],
  );

  // The canvas is as wide as its box.
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(Math.max(200, Math.floor(el.clientWidth))));
    ro.observe(el);
    return () => ro.disconnect();
  }, [data]);

  const scale = view?.scale ?? 8;
  // The images come at 16 px per cell, or 32 when zoomed in past 16.
  const imagePx = scale > 16 ? 32 : 16;

  useEffect(() => {
    if (!data || !paths.user_zomboid_dir) return;
    let gone = false;
    const urls: string[] = [];
    setImagesLoading(true);
    void (async () => {
      const list = await TauriService.mapImages(paths, imagePx);
      const loaded: { img: MapImage; el: HTMLImageElement }[] = [];
      for (const img of list) {
        const url = await TauriService.mapImageUrl(paths, img.id);
        if (!url || gone) continue;
        urls.push(url);
        const el = new Image();
        el.src = url;
        try {
          await el.decode();
          loaded.push({ img, el });
        } catch {
          // a broken image is left out; the grid still draws
        }
      }
      if (!gone) {
        setImages(loaded);
        setImagesLoading(false);
      }
    })();
    return () => {
      gone = true;
      urls.forEach((u) => URL.revokeObjectURL(u));
    };
  }, [data, paths, imagePx]);

  useEffect(() => {
    const el = canvas.current;
    if (!el || !bounds || !data || !view) return;
    const dpr = window.devicePixelRatio || 1;
    el.width = Math.round(width * dpr);
    el.height = Math.round(VIEW_HEIGHT * dpr);
    el.style.width = `${width}px`;
    el.style.height = `${VIEW_HEIGHT}px`;
    const g = el.getContext('2d');
    if (!g) return;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.fillStyle = '#020617';
    g.fillRect(0, 0, width, VIEW_HEIGHT);
    const s = view.scale;
    const gap = s > 4 ? 1 : 0;
    const at = (c: [number, number]) => [(c[0] - bounds.x0) * s + view.ox, (c[1] - bounds.y0) * s + view.oy] as const;
    const visible = (x: number, y: number) => x > -s && y > -s && x < width && y < VIEW_HEIGHT;
    const cell = (c: [number, number], fill: string) => {
      const [x, y] = at(c);
      if (!visible(x, y)) return;
      g.fillStyle = fill;
      g.fillRect(x, y, s - gap, s - gap);
    };
    const withMap = images.length > 0;
    if (withMap) {
      g.imageSmoothingEnabled = s < 32;
      for (const { img, el: im } of images) {
        const shownMod = img.folder === 'Muldraugh, KY' || shown.some((m) => m.folder === img.folder);
        if (!shownMod) continue;
        const x = (img.bounds[0] / SQUARES_PER_CELL - bounds.x0) * s + view.ox;
        const y = (img.bounds[1] / SQUARES_PER_CELL - bounds.y0) * s + view.oy;
        g.drawImage(im, x, y, ((img.bounds[2] - img.bounds[0]) / SQUARES_PER_CELL) * s, ((img.bounds[3] - img.bounds[1]) / SQUARES_PER_CELL) * s);
      }
      // The game's world as a faint grid over its own picture.
      if (s >= 6) {
        g.strokeStyle = 'rgba(15, 23, 42, 0.35)';
        g.lineWidth = 1;
        for (const c of data.vanilla) {
          const [x, y] = at(c);
          if (visible(x, y)) g.strokeRect(x + 0.5, y + 0.5, s - 1, s - 1);
        }
      }
      for (const m of shown) for (const c of m.cells) cell(c, colour(m.idx, 0.45));
      for (const c of clashes) for (const x of c.cells) cell(x, 'rgba(239, 68, 68, 0.7)');
    } else {
      for (const c of data.vanilla) cell(c, VANILLA);
      for (const m of shown) for (const c of m.cells) cell(c, colour(m.idx));
      for (const c of clashes) for (const x of c.cells) cell(x, CLASH);
    }
  }, [data, shown, clashes, bounds, view, width, images]);

  // The wheel zooms on the point under the pointer (not the page: the listener is not passive).
  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (!armedRef.current) return; // the page scrolls
      e.preventDefault();
      const r = el.getBoundingClientRect();
      const px = e.clientX - r.left;
      const py = e.clientY - r.top;
      setView((v) => {
        if (!v) return v;
        const next = Math.max(MIN_SCALE, Math.min(MAX_SCALE, v.scale * Math.pow(1.0015, -e.deltaY)));
        const k = next / v.scale;
        return clamp({ scale: next, ox: px - (px - v.ox) * k, oy: py - (py - v.oy) * k });
      });
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  }, [data, clamp]);

  const onPointerDown = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (e.button !== 0 || !view) return;
    arm(true);
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { x: e.clientX, y: e.clientY, ox: view.ox, oy: view.oy };
    setDragging(true);
  };
  const onPointerUp = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
    drag.current = null;
    setDragging(false);
  };
  const onPointerMove = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (!bounds || !view) return;
    const d = drag.current;
    if (d) {
      setView(clamp({ ...view, ox: d.ox + e.clientX - d.x, oy: d.oy + e.clientY - d.y }));
      setHover(null);
      return;
    }
    const r = e.currentTarget.getBoundingClientRect();
    const px = e.clientX - r.left;
    const py = e.clientY - r.top;
    const x = Math.floor((px - view.ox) / view.scale) + bounds.x0;
    const y = Math.floor((py - view.oy) / view.scale) + bounds.y0;
    setHover({ x, y, px, py });
  };
  const hoverMaps = hover ? byCell.get(`${hover.x},${hover.y}`) ?? [] : [];
  const hoverVanilla = hover ? vanillaSet.has(`${hover.x},${hover.y}`) : false;

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-y-auto p-6 space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-bold text-slate-100 flex items-center gap-2"><MapIcon className="w-5 h-5 text-emerald-400" />{t('maps.title')}</h2>
          <p className="text-xs text-slate-400 mt-1 max-w-3xl">{t('maps.blurb')}</p>
        </div>
        <button onClick={() => void load()} disabled={loading} aria-label={t('maps.refresh')} className="flex items-center gap-1 px-2.5 py-1.5 rounded-md border border-slate-700 text-xs text-slate-300 hover:bg-slate-800 disabled:opacity-50 cursor-pointer">
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      <div className="flex flex-wrap items-center gap-4">
        <label className="flex items-center gap-1.5 text-xs text-slate-300 cursor-pointer w-fit">
          <input type="checkbox" checked={onlyActive} onChange={(e) => pickOnlyActive(e.target.checked)} className="accent-emerald-500" />
          {t('maps.onlyActive')}
        </label>
        <span className="text-[11px] text-slate-500">{t('maps.howToMove')}</span>
        {imagesLoading && <span className="text-[11px] text-slate-500 flex items-center gap-1"><RefreshCw className="w-3 h-3 animate-spin" />{t('maps.loadingImage')}</span>}
      </div>

      {(clashes.length > 0 || tiledefClashes.length > 0) && (
        <div className="space-y-1.5">
          {clashes.map((c) => (
            <div key={`${c.a}|${c.b}`} className={`rounded-md border px-3 py-2 text-xs ${c.both_active ? 'border-red-700/70 bg-red-950/40 text-red-200' : 'border-amber-700/60 bg-amber-950/30 text-amber-200'}`}>
              <div className="flex items-start gap-2">
                <TriangleAlert className="w-4 h-4 shrink-0 mt-0.5" />
                <span>{c.both_active ? t('maps.clashNow', { a: c.a, b: c.b, count: c.cells.length }) : t('maps.clashIf', { a: c.a, b: c.b, count: c.cells.length })}</span>
              </div>
              <ModButtons mods={[[c.mod_a, c.a, (data?.maps ?? []).some((m) => m.mod_id === c.mod_a && m.active)], [c.mod_b, c.b, (data?.maps ?? []).some((m) => m.mod_id === c.mod_b && m.active)]]} offer={c.both_active} onShowMod={onShowMod} onTurnOff={onTurnOff} />
            </div>
          ))}
          {tiledefClashes.map((c) => {
            const names = c.mods.map((m) => `${m[1]}${m[3] ? '' : ` (${t('maps.off')})`}`).join(', ');
            return (
              <div key={`tiledef-${c.number}`} className={`rounded-md border px-3 py-2 text-xs ${c.both_active ? 'border-red-700/70 bg-red-950/40 text-red-200' : 'border-amber-700/60 bg-amber-950/30 text-amber-200'}`}>
                <div className="flex items-start gap-2">
                  <TriangleAlert className="w-4 h-4 shrink-0 mt-0.5" />
                  <span>{c.both_active ? t('maps.tiledefNow', { number: c.number, mods: names }) : t('maps.tiledefIf', { number: c.number, mods: names })}</span>
                </div>
                <ModButtons mods={c.mods.map((m) => [m[0], m[1], m[3]] as [string, string, boolean])} offer={c.both_active} onShowMod={onShowMod} onTurnOff={onTurnOff} />
              </div>
            );
          })}
        </div>
      )}

      {data && data.maps.length === 0 && <div className="p-8 text-center text-slate-500 text-xs">{t('maps.none')}</div>}

      {data && (
        <div className="flex flex-col xl:flex-row gap-4 items-start">
          <div className="flex-[3] min-w-0 w-full">
            <div ref={box} className="relative rounded-lg border border-slate-800 bg-slate-950 overflow-hidden">
              <canvas
                ref={canvas}
                onPointerDown={onPointerDown}
                onPointerMove={onPointerMove}
                onPointerUp={onPointerUp}
                onPointerCancel={onPointerUp}
                onPointerLeave={() => {
                  setHover(null);
                  arm(false);
                }}
                onDoubleClick={() => setView(fit())}
                className={`block touch-none select-none ${dragging ? 'cursor-grabbing' : 'cursor-grab'}`}
                aria-label={t('maps.gridLabel')}
              />
              <button onClick={() => setView(fit())} aria-label={t('maps.fit')} title={t('maps.fit')} className="absolute top-2 right-2 p-1.5 rounded-md border border-slate-700 bg-slate-900/90 text-slate-300 hover:bg-slate-800 cursor-pointer">
                <Maximize className="w-3.5 h-3.5" />
              </button>
              {hover && !armed && (
                <div className="absolute bottom-2 left-2 pointer-events-none rounded-md border border-slate-700 bg-slate-900/90 px-2 py-1 text-[10px] text-slate-400">{t('maps.clickToZoom')}</div>
              )}
              {hover && !dragging && (hoverVanilla || hoverMaps.length > 0) && (
                <div className="absolute pointer-events-none rounded-md border border-slate-700 bg-slate-900/95 px-2 py-1 text-[11px] text-slate-200 shadow-lg" style={{ left: Math.min(hover.px + 18, width - 180), top: hover.py + 8 }}>
                  <div className="font-mono text-slate-400">{t('maps.cell', { x: hover.x, y: hover.y })}</div>
                  {hoverVanilla && <div className="text-slate-400">{t('maps.vanillaCell')}</div>}
                  {hoverMaps.map((name) => <div key={name}>{name}</div>)}
                  {clashSet.has(`${hover.x},${hover.y}`) && <div className="text-red-300">{t('maps.sharedCell')}</div>}
                </div>
              )}
            </div>
            <div className="flex flex-wrap gap-3 mt-2 text-[10px] text-slate-400">
              <span className="flex items-center gap-1"><span className="inline-block w-3 h-3 rounded-sm border border-slate-600" />{t('maps.legendVanilla')}</span>
              <span className="flex items-center gap-1"><span className="inline-block w-3 h-3 rounded-sm" style={{ background: CLASH }} />{t('maps.legendClash')}</span>
            </div>
          </div>

          <div className="flex-1 min-w-[260px] space-y-1.5">
            {maps.map((m) => {
              const id = `${m.mod_id}/${m.folder}`;
              const off = hidden.has(id);
              return (
                <label key={id} className={`flex items-start gap-2 rounded-md border border-slate-800 bg-slate-900/70 px-3 py-2 cursor-pointer ${off ? 'opacity-50' : ''}`}>
                  <input type="checkbox" checked={!off} onChange={() => setHidden((prev) => { const n = new Set(prev); if (n.has(id)) n.delete(id); else n.add(id); return n; })} className="mt-0.5 accent-emerald-500" />
                  <span className="inline-block w-3 h-3 mt-0.5 rounded-sm shrink-0" style={{ background: colour(m.idx) }} />
                  <span className="min-w-0">
                    <span className="block text-xs text-slate-100 truncate">{m.title}</span>
                    <span className="block text-[10px] text-slate-500 truncate">{m.mod_name}</span>
                    <span className="block text-[10px] text-slate-400">
                      {t('maps.cells', { count: m.cells.length })}
                      {' · '}
                      {m.over_vanilla > 0 ? t('maps.overVanilla', { count: m.over_vanilla }) : t('maps.outsideVanilla')}
                      {m.lots.length > 0 ? ` · ${t('maps.buildsOn', { maps: m.lots.join(', ') })}` : ''}
                      {!m.active ? ` · ${t('maps.off')}` : ''}
                    </span>
                  </span>
                  <button onClick={(e) => { e.preventDefault(); onShowMod(m.mod_id); }} title={t('maps.showInList')} aria-label={t('maps.showInList')} className="ml-auto shrink-0 p-1 rounded text-slate-400 hover:text-slate-100 hover:bg-slate-800 cursor-pointer">
                    <ListOrdered className="w-3.5 h-3.5" />
                  </button>
                </label>
              );
            })}
          </div>
        </div>
      )}

      {data && (
        <div className="space-y-2">
          <h3 className="text-sm font-semibold text-slate-200">{t('maps.related', { count: related.length })}</h3>
          <p className="text-[11px] text-slate-400 max-w-3xl">{t('maps.relatedBlurb')}</p>
          {related.length === 0 ? (
            <div className="text-xs text-slate-500">{t('maps.noRelated')}</div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 2xl:grid-cols-3 gap-1.5">
              {related.map((r) => (
                <button key={r.mod_id} onClick={() => onShowMod(r.mod_id)} title={t('maps.showInList')} className={`text-left rounded-md border border-slate-800 bg-slate-900/70 px-3 py-2 hover:border-emerald-700/70 hover:bg-slate-900 cursor-pointer ${r.active ? '' : 'opacity-60'}`}>
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="text-xs text-slate-100 truncate">{r.mod_name}</span>
                    {r.kinds.map((k) => (
                      <span key={k} className={`shrink-0 rounded px-1.5 py-0.5 text-[9px] uppercase tracking-wide ${k === 'spawn' ? 'bg-sky-900/60 text-sky-200' : 'bg-violet-900/60 text-violet-200'}`}>{t(`maps.kind_${k}`)}</span>
                    ))}
                    {!r.active && <span className="shrink-0 text-[10px] text-slate-500">{t('maps.off')}</span>}
                  </div>
                  <div className="text-[10px] text-slate-500 font-mono truncate" title={r.detail.join('\n')}>{r.mod_id} · {r.detail.join(' · ')}</div>
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

/** Under a clash: each mod, to see it in the Mod List, and to turn it off when both are on. */
const ModButtons: React.FC<{ mods: [string, string, boolean][]; offer: boolean; onShowMod: (id: string) => void; onTurnOff: (id: string) => void }> = ({ mods, offer, onShowMod, onTurnOff }) => {
  const { t } = useTranslation();
  return (
    <div className="flex flex-wrap gap-1.5 mt-2 ml-6">
      {mods.map(([id, name, on]) => (
        <span key={id} className="inline-flex rounded-md border border-slate-700/80 bg-slate-950/60 overflow-hidden">
          <button onClick={() => onShowMod(id)} title={t('maps.showInList')} className="flex items-center gap-1 px-2 py-1 text-[11px] text-slate-200 hover:bg-slate-800 cursor-pointer">
            <ListOrdered className="w-3 h-3" />
            {name}
          </button>
          {offer && on && (
            <button onClick={() => onTurnOff(id)} className="flex items-center gap-1 px-2 py-1 text-[11px] text-red-200 border-l border-slate-700/80 hover:bg-red-900/50 cursor-pointer">
              <Power className="w-3 h-3" />
              {t('maps.turnOff')}
            </button>
          )}
        </span>
      ))}
    </div>
  );
};
