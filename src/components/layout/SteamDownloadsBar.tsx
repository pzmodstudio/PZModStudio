import React, { useEffect, useRef, useState } from 'react';
import { CheckCircle2, Download, PackageOpen, X, AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { TauriService } from '../../services/tauri';
import { noteChange } from '../../services/changes';
import { StudioPathsUI, SteamDownloads } from '../../types';

const ACTIVE_POLL_MS = 2000;
const IDLE_POLL_MS = 6000;
const DONE_SHOWN_MS = 10000;

const mb = (bytes: number) => (bytes / 1048576).toFixed(bytes < 10 * 1048576 ? 1 : 0);

/**
 * What Steam is downloading for the game, on every screen: the items, the phase and, while it
 * downloads, how much of it is on disk. When a job ends the mods are read again (so the Mod List
 * shows what arrived) and the bar says so for a few seconds. Nothing shows while Steam is idle.
 */
export const SteamDownloadsBar: React.FC<{ paths: StudioPathsUI }> = ({ paths }) => {
  const { t } = useTranslation();
  const [state, setState] = useState<SteamDownloads | null>(null);
  const [done, setDone] = useState<SteamDownloads['finished']>(null);
  const [names, setNames] = useState<Map<string, string>>(new Map());
  const lastFinished = useRef<string | null | undefined>(undefined);

  useEffect(() => {
    if (!paths.is_valid) return;
    let cancelled = false;
    let timer: number | undefined;
    const poll = async () => {
      const s = await TauriService.steamDownloads(paths);
      if (cancelled) return;
      if (s) {
        setState(s);
        const at = s.finished?.at ?? null;
        // The first read only learns where the log stands: a job that ended before the app opened is old news.
        if (lastFinished.current !== undefined && at && at !== lastFinished.current && s.finished) {
          setDone(s.finished);
          noteChange('mods');
          window.setTimeout(() => setDone((d) => (d?.at === at ? null : d)), DONE_SHOWN_MS);
        }
        lastFinished.current = at;
      }
      timer = window.setTimeout(poll, s && s.phase !== 'idle' ? ACTIVE_POLL_MS : IDLE_POLL_MS);
    };
    void poll();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [paths]);

  // Steam gives ids; the Workshop gives their titles.
  const ids = [...(state?.items ?? []), ...(done?.items ?? [])];
  const missing = ids.filter((id) => !names.has(id));
  const missingKey = missing.join(',');
  useEffect(() => {
    if (!missingKey) return;
    TauriService.workshopDetails(missingKey.split(','))
      .then((items) => setNames((prev) => new Map([...prev, ...items.map((i) => [i.workshop_id, i.title] as [string, string])])))
      .catch(() => setNames((prev) => new Map([...prev, ...missingKey.split(',').map((id) => [id, id] as [string, string])])));
  }, [missingKey]);
  const label = (items: string[]) => {
    const first = items.length ? names.get(items[0]) ?? items[0] : t('steamDownloads.someItem');
    return items.length > 1 ? t('steamDownloads.andMore', { name: first, count: items.length - 1 }) : first;
  };

  const active = state && state.phase !== 'idle' ? state : null;
  if (!active && !done) return null;

  if (!active && done) {
    return (
      <div className="fixed bottom-4 left-1/2 -translate-x-1/2 z-[90] w-[26rem] max-w-[calc(100vw-2rem)]" role="status" aria-live="polite">
        <div className={`flex items-center gap-3 px-3 py-2 rounded-lg border shadow-lg ${done.ok ? 'border-emerald-600/50 bg-emerald-950/95 text-emerald-100' : 'border-amber-600/50 bg-amber-950/95 text-amber-100'}`}>
          {done.ok ? <CheckCircle2 className="w-4 h-4 shrink-0 text-emerald-400" /> : <AlertTriangle className="w-4 h-4 shrink-0 text-amber-400" />}
          <div className="flex-1 min-w-0 text-[11px] truncate">
            {done.ok ? t('steamDownloads.done', { name: label(done.items) }) : t('steamDownloads.failed', { name: label(done.items), result: done.result })}
          </div>
          <button type="button" onClick={() => setDone(null)} className="text-slate-400 hover:text-slate-200" aria-label={t('backend.dismiss')}>
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    );
  }

  const s = active!;
  const pct = s.total_bytes && s.done_bytes != null ? Math.min(100, Math.round((s.done_bytes / s.total_bytes) * 100)) : null;
  const shownPct = s.phase === 'installing' ? 100 : pct;
  const Icon = s.phase === 'installing' ? PackageOpen : Download;
  return (
    <div className="fixed bottom-4 left-1/2 -translate-x-1/2 z-[90] w-[26rem] max-w-[calc(100vw-2rem)]" role="status" aria-live="polite">
      <div className="px-3 py-2 rounded-lg border border-sky-700/50 bg-slate-900/95 shadow-lg text-slate-100">
        <div className="flex items-center gap-2 text-[11px]">
          <Icon className="w-4 h-4 shrink-0 text-sky-400" />
          <span className="font-bold text-sky-300 shrink-0">{t(`steamDownloads.phase.${s.phase}`)}</span>
          <span className="flex-1 min-w-0 truncate">{label(s.items)}</span>
          {shownPct != null && <span className="font-mono text-sky-200 shrink-0">{shownPct}%</span>}
        </div>
        <div className="mt-1.5 h-1.5 rounded bg-slate-800 overflow-hidden">
          {shownPct != null ? (
            <div className="h-full bg-sky-500 transition-[width] duration-700" style={{ width: `${shownPct}%` }} />
          ) : (
            <div className="h-full w-1/3 bg-sky-500/70 animate-pulse" />
          )}
        </div>
        {s.phase === 'downloading' && s.total_bytes != null && (
          <div className="mt-1 text-[10px] font-mono text-slate-400">
            {s.done_bytes != null ? `${mb(s.done_bytes)} / ${mb(s.total_bytes)} MB` : `${mb(s.total_bytes)} MB`}
            {s.rate_mbps ? ` · ${s.rate_mbps.toFixed(0)} Mbps` : ''}
          </div>
        )}
      </div>
    </div>
  );
};
