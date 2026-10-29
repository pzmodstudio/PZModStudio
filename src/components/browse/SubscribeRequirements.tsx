import React, { useMemo, useState } from 'react';
import { CheckCircle2, PackagePlus, RefreshCw, TriangleAlert, X } from 'lucide-react';
import i18next from 'i18next';
import { RequiredChain } from '../../types';
import { formatBytes } from './common';

interface SubscribeRequirementsProps {
  /** Titles of what the player asked to subscribe to. */
  titles: string[];
  chain: RequiredChain;
  /** Workshop items already on this PC. */
  installed: Set<string>;
  /** Subscribe to what was asked for plus these. */
  onSubscribe: (extra: string[]) => void;
  /** Read the chain again (some pages could not be read). */
  onRetry: () => void;
  onCancel: () => void;
}

/**
 * What a subscription needs too, before it is made: every item in the chain of "Required items",
 * the ones already installed shown as such, the ones Steam no longer has flagged, each missing
 * one ticked so it comes in the same subscription.
 */
export const SubscribeRequirements: React.FC<SubscribeRequirementsProps> = ({ titles, chain, installed, onSubscribe, onRetry, onCancel }) => {
  const missing = useMemo(() => chain.items.filter((n) => n.found && !installed.has(n.workshop_id)), [chain, installed]);
  const [picked, setPicked] = useState<Set<string>>(() => new Set(missing.map((n) => n.workshop_id)));
  const size = missing.filter((n) => picked.has(n.workshop_id)).reduce((a, n) => a + n.file_size, 0);
  const toggle = (id: string) =>
    setPicked((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const name = titles.length === 1 ? titles[0] : i18next.t('browse.requirements.theseMods', { count: titles.length });

  return (
    // z-[60]: above the mod detail window (z-50), which stays open while this asks and otherwise paints over it.
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-[60] flex items-center justify-center p-6" onClick={onCancel}>
      <div role="dialog" aria-modal="true" aria-label={i18next.t('browse.requirements.title', { name })} className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl w-full max-w-lg p-5 space-y-4" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2"><PackagePlus className="w-4 h-4 text-sky-400" />{i18next.t('browse.requirements.title', { name })}</h3>
            <p className="text-[11px] text-slate-400 mt-1">{i18next.t('browse.requirements.blurb')}</p>
          </div>
          <button type="button" onClick={onCancel} className="text-slate-500 hover:text-slate-300 p-1 cursor-pointer" aria-label={i18next.t('preview.cancel')}>
            <X className="w-4 h-4" />
          </button>
        </div>

        <ul className="space-y-1 max-h-[50vh] overflow-y-auto pr-1">
          {chain.items.map((n) => {
            const have = installed.has(n.workshop_id);
            return (
              <li key={n.workshop_id} className={`flex items-center gap-3 rounded-lg border px-2 py-1.5 ${have ? 'border-slate-800 opacity-70' : n.found ? 'border-slate-700' : 'border-amber-700/60'}`} style={{ marginLeft: `${Math.min(n.depth - 1, 3) * 14}px` }}>
                {have ? (
                  <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" aria-label={i18next.t('browse.requirements.installed')} />
                ) : n.found ? (
                  <input type="checkbox" checked={picked.has(n.workshop_id)} onChange={() => toggle(n.workshop_id)} className="accent-emerald-500 cursor-pointer" aria-label={n.title} />
                ) : (
                  <TriangleAlert className="w-4 h-4 text-amber-400 shrink-0" />
                )}
                {n.preview_url ? <img src={n.preview_url} alt="" className="w-9 h-9 rounded object-cover shrink-0" /> : <div className="w-9 h-9 rounded bg-slate-800 shrink-0" />}
                <div className="min-w-0 flex-1">
                  <div className="text-xs text-slate-100 truncate">{n.title || n.workshop_id}</div>
                  <div className="text-[10px] text-slate-500 truncate">
                    {have
                      ? i18next.t('browse.requirements.installed')
                      : !n.found
                        ? i18next.t('browse.requirements.gone')
                        : i18next.t('browse.requirements.neededBy', { names: n.required_by.join(', ') })}
                    {n.found && n.file_size > 0 && !have ? ` · ${formatBytes(n.file_size)}` : ''}
                  </div>
                </div>
              </li>
            );
          })}
        </ul>

        {(chain.unread.length > 0 || chain.cut_short) && (
          <div className="flex items-center gap-2 text-[11px] text-amber-200 bg-amber-950/40 border border-amber-800/50 rounded-md px-3 py-2">
            <TriangleAlert className="w-3.5 h-3.5 shrink-0" />
            <span className="flex-1">{chain.unread.length > 0 ? i18next.t('browse.requirements.unread', { count: chain.unread.length }) : i18next.t('browse.requirements.cutShort')}</span>
            {chain.unread.length > 0 && (
              <button type="button" onClick={onRetry} className="flex items-center gap-1 px-2 py-1 rounded border border-amber-700/60 hover:bg-amber-900/40 cursor-pointer">
                <RefreshCw className="w-3 h-3" />{i18next.t('browse.requirements.retry')}
              </button>
            )}
          </div>
        )}

        <div className="flex flex-wrap items-center justify-end gap-2 pt-1">
          <button type="button" onClick={onCancel} className="px-3 py-1.5 rounded-md text-xs text-slate-300 hover:bg-slate-800 cursor-pointer">{i18next.t('preview.cancel')}</button>
          <button type="button" onClick={() => onSubscribe([])} className="px-3 py-1.5 rounded-md border border-slate-700 text-xs text-slate-200 hover:bg-slate-800 cursor-pointer">
            {i18next.t('browse.requirements.onlyThis', { name })}
          </button>
          <button type="button" onClick={() => onSubscribe([...picked])} autoFocus className="px-3 py-1.5 rounded-md bg-emerald-700/80 hover:bg-emerald-600 text-xs font-semibold text-emerald-50 cursor-pointer">
            {picked.size > 0
              ? i18next.t('browse.requirements.subscribeAll', { count: titles.length + picked.size, size: size > 0 ? `(${formatBytes(size)})` : '' }).trim()
              : i18next.t('browse.requirements.subscribeAsked', { count: titles.length })}
          </button>
        </div>
      </div>
    </div>
  );
};
