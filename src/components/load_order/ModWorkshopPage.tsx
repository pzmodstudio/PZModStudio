import React, { useEffect, useState } from 'react';
import { Heart, Loader2, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ModInfo, WorkshopItemDetails } from '../../types';
import { TauriService } from '../../services/tauri';
import { Comments, Description, Gallery } from '../browse/ModDetail';
import { formatBytes, formatCount, formatUnixDate } from '../browse/common';

interface ModWorkshopPageProps {
  /** The inspector's tab: the page, or its comment thread. */
  view: 'page' | 'comments';
  workshopId: string;
  mods: ModInfo[];
  /** The mod.info description, shown when Steam does not answer. */
  fallback?: string;
  onJumpToMod: (modId: string) => void;
  onOpenInBrowser?: (workshopId: string) => void;
}

/**
 * The selected mod's Workshop page inside the Mod List, so its page need not be opened: pictures
 * and videos, the numbers (subscribers, favourites, size, dates), categories, required items (an
 * installed one jumps to its row) and the description with "Translate"; or, on the Comments tab,
 * the comment thread. Steam is asked through the same cached calls as Explore Mods.
 */
export const ModWorkshopPage: React.FC<ModWorkshopPageProps> = ({ view, workshopId, mods, fallback, onJumpToMod, onOpenInBrowser }) => {
  const { t } = useTranslation();
  const [data, setData] = useState<WorkshopItemDetails | null>(null);
  const [state, setState] = useState<'loading' | 'ready' | 'failed'>('loading');
  useEffect(() => {
    let cancelled = false;
    setData(null);
    setState('loading');
    TauriService.workshopItemDetails(workshopId)
      .then((d) => {
        if (cancelled) return;
        setData(d);
        setState('ready');
      })
      .catch(() => !cancelled && setState('failed'));
    return () => {
      cancelled = true;
    };
  }, [workshopId]);

  const item = data?.item;
  const images = [...(item?.preview_url ? [item.preview_url] : []), ...(data?.screenshots ?? [])];
  const categories = item?.tags.filter((tag) => !/^Build \d+$/.test(tag)) ?? [];
  const installedOf = (id: string) => mods.find((m) => m.workshop_id === id);
  const stat = (label: string, value: React.ReactNode) =>
    value ? (
      <div className="flex flex-col gap-0.5 px-2.5 py-1.5 rounded-lg bg-slate-950/60 border border-slate-800 min-w-0">
        <span className="text-[10px] uppercase tracking-wider text-slate-500">{label}</span>
        <span className="text-[12px] text-slate-200 truncate">{value}</span>
      </div>
    ) : null;

  return (
    <div className="flex flex-col gap-3">
      {view === 'comments' ? (
        <Comments key={workshopId} id={workshopId} />
      ) : (
        <>
          {state === 'loading' && (
            <span className="flex items-center gap-1.5 text-[11px] text-slate-500">
              <Loader2 className="w-3.5 h-3.5 animate-spin" /> {t('browse.detail.loadingWorkshop')}
            </span>
          )}
          {state === 'failed' && <p className="text-[11px] text-amber-300/90">{t('inspector.workshop.failed')}</p>}
          {images.length > 0 || (data?.videos.length ?? 0) > 0 ? <Gallery images={images} videos={data?.videos} /> : null}
          {item && (
            <div className="grid grid-cols-3 gap-1.5">
              {stat(t('browse.detail.subscribers'), <span className="inline-flex items-center gap-1"><Users className="w-3 h-3" />{formatCount(item.subscriptions)}</span>)}
              {stat(t('browse.detail.favorites'), <span className="inline-flex items-center gap-1"><Heart className="w-3 h-3" />{formatCount(item.favorited)}</span>)}
              {stat(t('browse.detail.size'), formatBytes(item.file_size))}
              {stat(t('browse.detail.created'), formatUnixDate(data?.time_created))}
              {stat(t('browse.detail.updated'), formatUnixDate(item.time_updated))}
              {stat(t('inspector.workshop.author'), data?.author)}
            </div>
          )}
          {categories.length > 0 && (
            <div className="flex flex-wrap gap-1">
              {categories.map((c) => (
                <span key={c} className="text-[10.5px] px-1.5 py-0.5 rounded border border-slate-700 text-slate-300">{c}</span>
              ))}
            </div>
          )}
          {data && data.required_items.length > 0 && (
            <div className="flex flex-col gap-1">
              <span className="text-[10px] uppercase tracking-wider font-bold text-slate-500">{t('inspector.workshop.required', { count: data.required_items.length })}</span>
              <div className="flex flex-wrap gap-1">
                {data.required_items.map((r) => {
                  const have = installedOf(r.workshop_id);
                  return (
                    <button
                      key={r.workshop_id}
                      type="button"
                      onClick={() => (have ? onJumpToMod(have.mod_id) : onOpenInBrowser?.(r.workshop_id))}
                      title={have ? t('inspector.workshop.requiredHave') : t('inspector.workshop.requiredMissing')}
                      className={`flex items-center gap-1.5 px-2 py-1 rounded-md border text-[11px] cursor-pointer transition ${
                        have ? 'border-emerald-800 text-emerald-200 bg-emerald-950/40 hover:border-emerald-500' : 'border-amber-700 text-amber-200 bg-amber-950/40 hover:border-amber-400'
                      }`}
                    >
                      <span className={`w-1.5 h-1.5 rounded-full ${have ? 'bg-emerald-400' : 'bg-amber-400'}`} />
                      {r.title || r.workshop_id}
                    </button>
                  );
                })}
              </div>
            </div>
          )}
          {(data?.description || state !== 'loading') && (
            <div className="text-xs">
              <Description source={data?.description} fallback={fallback || item?.summary} title={item?.title} />
            </div>
          )}
        </>
      )}
    </div>
  );
};
