import React, { useEffect, useState } from 'react';
import { ArrowLeft, Check, Download, ExternalLink, FileArchive, Heart, Languages, Layers, Loader2, Play, Plus, Trash2, Users, X } from 'lucide-react';
import { AuthorLink, NexusMod, NexusModDetails, Translation, WorkshopInstallState, WorkshopItem, WorkshopItemDetails, WorkshopLookup } from '../../types';
import { TRANSLATION_SERVICES, appLanguage, languageName, scriptLanguage } from '../../utils/language';
import { SupportLinks } from '../common/SupportLinks';
import { TauriService, reportBackendError } from '../../services/tauri';
import { StudioPathsUI } from '../settings/SettingsModule';
import { BBCode } from './BBCode';
import i18next from 'i18next';
import { CommunityIndexSection } from './CommunityIndexSection';
import { KeepActions, StarButton } from '../common/keep';
import { registriesForItem, useCommunityIndex } from '../../services/communityIndex';
import { BuildChips, CardGrid, Chip, InstalledChip, ModCard, UpdateChips, cardHover, formatBytes, formatCount, formatIsoDate, formatUnixDate, workshopItemMenu } from './common';
import { useContextMenu } from '../common/ContextMenu';
import { tHtml } from '../../i18n/html';

export type DetailTarget =
  | { kind: 'workshop'; id: string; preview?: WorkshopItem }
  | { kind: 'collection'; id: string; preview?: WorkshopItem }
  | { kind: 'nexus'; modId: number; preview?: NexusMod };

interface ModDetailProps {
  target: DetailTarget;
  paths: StudioPathsUI;
  /** True when the Steam client can take subscriptions right now. */
  steamReady: boolean;
  /** Subscribes or unsubscribes through Steam; resolves once Steam has taken it. */
  onSubscribe: (ids: string[], subscribe: boolean) => Promise<void>;
  canGoBack: boolean;
  onBack: () => void;
  onClose: () => void;
  onOpen: (target: DetailTarget) => void;
  /** Workshop id -> installed mod ids. */
  installedByItem: Map<string, string[]>;
  stateByItem: Map<string, WorkshopInstallState>;
  /** Nexus mod id -> installed mod ids. */
  installedByNexusId: Map<number, string[]>;
  onNexusDownload: (m: NexusMod) => void;
  /** Makes a profile holding the mods installed from a collection's items. */
  onMakeProfile?: (name: string, workshopIds: string[]) => void;
  /** The game build installed, so the index's lines read as "reported on 42.19" when it differs. */
  installedBuild?: string | null;
  /** Favorite, lists, profiles: the star in the header. */
  keep?: KeepActions;
}

const Section: React.FC<{ title: string; children: React.ReactNode }> = ({ title, children }) => (
  <section className="flex flex-col gap-1.5">
    <h3 className="text-[10px] font-bold uppercase tracking-wider text-slate-500 font-mono">{title}</h3>
    {children}
  </section>
);

const InfoRow: React.FC<{ label: string; value: React.ReactNode }> = ({ label, value }) =>
  value ? (
    <div className="flex justify-between gap-3 text-[11px]">
      <span className="text-slate-500">{label}</span>
      <span className="text-slate-200 text-right">{value}</span>
    </div>
  ) : null;

/**
 * Big image plus a strip of thumbnails. A video thumbnail plays the video in the big box, in
 * YouTube's cookie-free embed; a link under it opens YouTube in the browser, for the videos
 * whose author does not allow embedding (YouTube then says so in the box).
 */
const Gallery: React.FC<{ images: string[]; videos?: string[] }> = ({ images, videos = [] }) => {
  const [current, setCurrent] = useState(0);
  const [playing, setPlaying] = useState<string | null>(null);
  const imagesKey = images.join('|');
  useEffect(() => {
    setCurrent(0);
    setPlaying(null);
  }, [imagesKey]);
  if (images.length === 0 && videos.length === 0) return <div className="aspect-video rounded-lg bg-slate-950 flex items-center justify-center text-slate-700 text-xs">{i18next.t('browse.chips.noImage')}</div>;
  const video = playing ?? (images.length === 0 ? videos[0] : null);
  return (
    <div className="flex flex-col gap-2">
      <div className="aspect-video rounded-lg bg-slate-950 overflow-hidden flex items-center justify-center">
        {video ? (
          <iframe
            key={video}
            src={`https://www.youtube-nocookie.com/embed/${video}?autoplay=1&rel=0`}
            title={i18next.t('browse.detail.watch')}
            allow="autoplay; fullscreen; picture-in-picture"
            allowFullScreen
            className="w-full h-full border-0"
          />
        ) : (
          <img src={images[Math.min(current, images.length - 1)]} alt="" className="max-w-full max-h-full object-contain" />
        )}
      </div>
      {video && (
        <button type="button" onClick={() => TauriService.openExternalUrl(`https://www.youtube.com/watch?v=${video}`)}
          className="self-end flex items-center gap-1 text-[11px] text-sky-300 hover:text-sky-200 cursor-pointer">
          <ExternalLink className="w-3 h-3" aria-hidden="true" />
          {i18next.t('browse.detail.watchBrowser')}
        </button>
      )}
      {(images.length > 1 || videos.length > 0) && (
        <div className="flex gap-1.5 overflow-x-auto pb-1">
          {images.map((src, i) => (
            <button key={src} type="button" onClick={() => { setCurrent(i); setPlaying(null); }} aria-label={i18next.t('browse.detail.image', { n: i + 1 })}
              className={`shrink-0 w-24 aspect-video rounded overflow-hidden border-2 cursor-pointer ${i === current && !video ? 'border-emerald-500' : 'border-transparent opacity-70 hover:opacity-100'}`}>
              <img src={src} alt="" loading="lazy" className="w-full h-full object-cover" />
            </button>
          ))}
          {videos.map((id) => (
            <button key={id} type="button" onClick={() => setPlaying(id)}
              className={`relative shrink-0 w-24 aspect-video rounded overflow-hidden border-2 cursor-pointer ${id === video ? 'border-emerald-500' : 'border-transparent opacity-80 hover:opacity-100'}`} title={i18next.t('browse.detail.watch')}>
              <img src={`https://img.youtube.com/vi/${id}/mqdefault.jpg`} alt="" loading="lazy" className="w-full h-full object-cover" />
              <Play className="absolute inset-0 m-auto w-6 h-6 text-white drop-shadow" aria-hidden="true" />
            </button>
          ))}
        </div>
      )}
    </div>
  );
};

/**
 * The mod's description, with "Translate" above it. Nothing is sent until the button is pressed;
 * once translated, the original is a click away. The button stands out when the script says the
 * text is not in a Latin alphabet (Russian, Chinese, Japanese, Korean...).
 */
const Description: React.FC<{ source?: string | null; fallback?: string | null; title?: string | null; onTitle?: (translated: string | null) => void }> = ({ source, fallback, title, onTitle }) => {
  const [translation, setTranslation] = useState<Translation | null>(null);
  const [showOriginal, setShowOriginal] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setTranslation(null);
    setShowOriginal(false);
    setError(null);
  }, [source]);
  // The title in the app's language goes to the header (null while the original is shown).
  useEffect(() => {
    const translated = translation && !showOriginal ? translation.texts[1]?.trim() || null : null;
    onTitle?.(translated && translated !== (title ?? '').trim() ? translated : null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [translation, showOriginal]);
  if (!source) return fallback ? <p className="text-xs text-slate-400">{fallback}</p> : null;
  const target = appLanguage();
  const script = scriptLanguage(source);
  const translate = async () => {
    setBusy(true);
    setError(null);
    try {
      setTranslation(await TauriService.translateTexts([source, title ?? ''], target));
      setShowOriginal(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };
  const fromName = translation ? languageName(translation.source || script || '') : '';
  const serviceName = translation ? TRANSLATION_SERVICES[translation.service] ?? '' : '';
  const label = translation
    ? [i18next.t('browse.detail.translated'), fromName && i18next.t('browse.detail.fromLang', { from: fromName }), serviceName].filter(Boolean).join(' \u00b7 ')
    : '';
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2 text-[11px]">
        {translation ? (
          <>
            <span className="text-slate-500">{label}</span>
            <button type="button" onClick={() => setShowOriginal((v) => !v)} className="text-sky-300 hover:text-sky-200 cursor-pointer">
              {showOriginal ? i18next.t('browse.detail.showTranslation') : i18next.t('browse.detail.showOriginal')}
            </button>
          </>
        ) : (
          <button type="button" onClick={() => void translate()} disabled={busy} title={i18next.t('browse.detail.translateTitle')}
            className={`flex items-center gap-1 px-2 py-1 rounded-lg border cursor-pointer disabled:opacity-50 ${script ? 'border-sky-700 text-sky-200 bg-sky-950/40 hover:bg-sky-950' : 'border-slate-800 text-slate-400 hover:text-slate-200'}`}>
            {busy ? <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" /> : <Languages className="w-3 h-3" aria-hidden="true" />}
            {script
              ? i18next.t('browse.detail.translateFrom', { from: languageName(script), to: languageName(target) })
              : i18next.t('browse.detail.translate', { to: languageName(target) })}
          </button>
        )}
        {error && <span className="text-red-300">{i18next.t('browse.detail.translateFailed', { reason: error })}</span>}
      </div>
      <BBCode source={translation && !showOriginal ? translation.texts[0] : source} />
    </div>
  );
};

const ActionButton: React.FC<{ onClick: () => void; className: string; title?: string; children: React.ReactNode }> = ({ onClick, className, title, children }) => (
  <button type="button" onClick={onClick} title={title}
    className={`w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg text-xs font-bold cursor-pointer ${className}`}>
    {children}
  </button>
);

/**
 * Subscribe / Unsubscribe through Steam. Only shown when the Steam client answered: without it
 * the Workshop page stays the way to subscribe.
 */
const SubscribeButton: React.FC<{
  ids: string[];
  installed: boolean;
  steamReady: boolean;
  onSubscribe: (ids: string[], subscribe: boolean) => Promise<void>;
  /** Wording for a collection ("Subscribe to all 67"). */
  allLabel?: string;
}> = ({ ids, installed, steamReady, onSubscribe, allLabel }) => {
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<'subscribed' | 'unsubscribed' | null>(null);
  if (!steamReady || ids.length === 0) return null;

  const subscribing = !installed && done !== 'subscribed';
  const run = async () => {
    setBusy(true);
    try {
      await onSubscribe(ids, subscribing);
      setDone(subscribing ? 'subscribed' : 'unsubscribed');
    } catch {
      // Already shown; the button stays as it was.
    } finally {
      setBusy(false);
    }
  };

  const label = subscribing
    ? allLabel ?? i18next.t('browse.detail.subscribe')
    : allLabel
      ? i18next.t('browse.detail.unsubscribeAll')
      : i18next.t('browse.detail.unsubscribe');
  return (
    <div className="flex flex-col gap-1">
      <ActionButton
        onClick={run}
        className={subscribing ? 'bg-emerald-600 hover:bg-emerald-500 text-white' : 'bg-slate-800 hover:bg-slate-700 text-slate-200'}
        title={subscribing ? i18next.t('browse.detail.subscribeTitle') : i18next.t('browse.detail.unsubscribeTitle')}
      >
        {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />
          : subscribing ? <Plus className="w-3.5 h-3.5" aria-hidden="true" />
          : <Trash2 className="w-3.5 h-3.5" aria-hidden="true" />}
        {busy ? i18next.t('browse.detail.askingSteam') : label}
      </ActionButton>
      {done && (
        <span className="flex items-center gap-1 text-[10px] text-emerald-300">
          <Check className="w-3 h-3" aria-hidden="true" />
          {done === 'subscribed' ? i18next.t('browse.detail.subscribed') : i18next.t('browse.detail.unsubscribed')}
        </span>
      )}
    </div>
  );
};

function useDetails<T>(load: () => Promise<T>, key: string, what: string) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let cancelled = false;
    setData(null);
    setLoading(true);
    load()
      .then((d) => !cancelled && setData(d))
      .catch((err) => !cancelled && reportBackendError(what, err))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return { data, loading };
}

/** "Support the author", from the changelog of the copy installed here and, for a Workshop item,
 *  its description. */
const InstalledSupport: React.FC<{ paths: StudioPathsUI; modId: string; more?: AuthorLink[] }> = ({ paths, modId, more }) => {
  const [links, setLinks] = useState<AuthorLink[]>([]);
  useEffect(() => {
    let cancelled = false;
    void TauriService.modChangelog(paths, modId).then((log) => {
      if (!cancelled) setLinks(log?.links ?? []);
    });
    return () => {
      cancelled = true;
    };
  }, [paths, modId]);
  return <SupportLinks links={links} more={more} />;
};

const WorkshopDetail: React.FC<ModDetailProps & { target: { kind: 'workshop' } & DetailTarget }> = ({ target, paths, installedByItem, stateByItem, onOpen, steamReady, onSubscribe, installedBuild = null, keep }) => {
  const { data, loading } = useDetails<WorkshopItemDetails>(() => TauriService.workshopItemDetails(target.id), target.id, i18next.t('browse.detail.loadingWorkshop'));
  const index = useCommunityIndex(paths);
  const item = data?.item ?? target.preview;
  const installed = installedByItem.get(target.id) ?? [];
  const images = [...(item?.preview_url ? [item.preview_url] : []), ...(data?.screenshots ?? [])];
  const missing = data?.required_items.filter((r) => !installedByItem.has(r.workshop_id)) ?? [];
  const categories = item?.tags.filter((t) => !/^Build \d+$/.test(t)) ?? [];
  /** The title in the app's language once the description was translated; the original moves under it. */
  const [translatedTitle, setTranslatedTitle] = useState<string | null>(null);
  const originalTitle = item?.title ?? i18next.t('browse.detail.workshopItem', { id: target.id });
  const by = data?.author ? i18next.t('browse.detail.by', { author: data.author }) : undefined;

  return (
    <DetailLayout
      title={translatedTitle ?? originalTitle}
      subtitle={translatedTitle ? [originalTitle, by].filter(Boolean).join(' · ') : by}
      chips={<>{keep && <StarButton keep={keep} target={{ workshop_id: target.id, name: item?.title ?? i18next.t('browse.detail.workshopItem', { id: target.id }), installedModIds: installed }} />}{installed.length > 0 && <InstalledChip />}<UpdateChips state={stateByItem.get(target.id)} />{item && <BuildChips builds={item.builds} source={i18next.t('browse.card.workshopSource')} />}</>}
      loading={loading}
      main={
        <>
          <Gallery images={images} videos={data?.videos} />
          <Description source={data?.description} fallback={item?.summary} title={item?.title} onTitle={setTranslatedTitle} />
        </>
      }
      side={
        <>
          <div className="flex flex-col gap-2">
            <SubscribeButton ids={[target.id]} installed={installed.length > 0} steamReady={steamReady} onSubscribe={onSubscribe} />
            <ActionButton onClick={() => TauriService.openExternalUrl(item?.page_url ?? `https://steamcommunity.com/sharedfiles/filedetails/?id=${target.id}`)}
              className="bg-sky-700 hover:bg-sky-600 text-white"
              title={i18next.t('browse.detail.openWorkshopTitle')}>
              <ExternalLink className="w-3.5 h-3.5" aria-hidden="true" /> {steamReady || installed.length > 0 ? i18next.t('browse.detail.openWorkshop') : i18next.t('browse.detail.openToSubscribe')}
            </ActionButton>
            {installed.length > 0 && <div className="text-[10px] font-mono text-emerald-400/80 break-all">{i18next.t('browse.detail.installedAs', { ids: installed.join(', ') })}</div>}
            {installed.length > 0
              ? <InstalledSupport paths={paths} modId={installed[0]} more={data?.item.support} />
              : <SupportLinks links={[]} more={data?.item.support} />}
          </div>
          {item && (
            <Section title={i18next.t('browse.detail.info')}>
              <InfoRow label={i18next.t('browse.detail.subscribers')} value={<span className="inline-flex items-center gap-1"><Users className="w-3 h-3" />{formatCount(item.subscriptions)}</span>} />
              <InfoRow label={i18next.t('browse.detail.favorites')} value={<span className="inline-flex items-center gap-1"><Heart className="w-3 h-3" />{formatCount(item.favorited)}</span>} />
              <InfoRow label={i18next.t('browse.detail.size')} value={formatBytes(item.file_size)} />
              <InfoRow label={i18next.t('browse.detail.created')} value={formatUnixDate(data?.time_created)} />
              <InfoRow label={i18next.t('browse.detail.updated')} value={formatUnixDate(item.time_updated)} />
              <InfoRow label={i18next.t('browse.detail.workshopId')} value={<span className="font-mono">{target.id}</span>} />
            </Section>
          )}
          {categories.length > 0 && (
            <Section title={i18next.t('browse.detail.categories')}>
              <div className="flex flex-wrap gap-1">{categories.map((t) => <Chip key={t} className="border-slate-700 text-slate-300">{t}</Chip>)}</div>
            </Section>
          )}
          {data && data.required_items.length > 0 && (
            <Section title={i18next.t('browse.detail.requiredItems', { count: data.required_items.length, missing: missing.length ? i18next.t('browse.detail.missingCount', { count: missing.length }) : '' })}>
              <div className="flex flex-col gap-1">
                {data.required_items.map((r) => {
                  const have = installedByItem.has(r.workshop_id);
                  return (
                    <button key={r.workshop_id} type="button" onClick={() => onOpen({ kind: 'workshop', id: r.workshop_id })}
                      className="flex items-center justify-between gap-2 px-2 py-1.5 rounded border border-slate-800 bg-slate-950/60 hover:border-slate-600 text-left cursor-pointer">
                      <span className="text-[11px] text-slate-200 line-clamp-2">{r.title || r.workshop_id}</span>
                      {have ? <InstalledChip /> : <Chip className="border-amber-500/60 text-amber-300">{i18next.t('browse.detail.missing')}</Chip>}
                    </button>
                  );
                })}
              </div>
            </Section>
          )}
          <CommunityIndexSection registries={registriesForItem(index, target.id, installed)} installedBuild={installedBuild} nameOf={(id) => index?.mods[id]?.name ?? id} />
          {item && !item.is_project_zomboid && <p className="text-[11px] text-amber-300">{i18next.t('browse.detail.otherGame')}</p>}
          {item?.banned && <p className="text-[11px] text-red-300">{i18next.t('browse.detail.banned')}</p>}
        </>
      }
    />
  );
};

const FILE_STYLE: Record<string, string> = {
  MAIN: 'border-emerald-600/60 text-emerald-300',
  UPDATE: 'border-sky-600/60 text-sky-300',
  OPTIONAL: 'border-violet-600/60 text-violet-300',
};

const NexusDetail: React.FC<ModDetailProps & { target: { kind: 'nexus' } & DetailTarget }> = ({ target, paths, installedByNexusId, onNexusDownload, onOpen, keep }) => {
  const { data, loading } = useDetails<NexusModDetails>(() => TauriService.nexusModDetails(target.modId), String(target.modId), i18next.t('browse.detail.loadingNexus'));
  const m = data?.mod ?? target.preview;
  const installed = installedByNexusId.get(target.modId) ?? [];
  const pageUrl = m?.page_url ?? `https://www.nexusmods.com/projectzomboid/mods/${target.modId}`;
  const image = m?.picture_url ?? m?.thumbnail_url ?? null;

  /** The title in the app's language once the description was translated; the original moves under it. */
  const [translatedTitle, setTranslatedTitle] = useState<string | null>(null);
  const originalTitle = m?.name ?? i18next.t('browse.detail.nexusMod', { id: target.modId });
  const by = m ? `${i18next.t('browse.detail.by', { author: m.author || m.uploader || i18next.t('browse.card.unknown') })}${m.version ? ` · v${m.version}` : ''}` : undefined;
  return (
    <DetailLayout
      title={translatedTitle ?? originalTitle}
      subtitle={translatedTitle ? [originalTitle, by].filter(Boolean).join(' · ') : by}
      chips={<>{keep && <StarButton keep={keep} target={{ nexus_id: target.modId, name: m?.name ?? i18next.t('browse.detail.nexusMod', { id: target.modId }), installedModIds: installed }} />}{installed.length > 0 && <InstalledChip />}{m && <BuildChips builds={m.build_hint ? m.build_hint.split('+') : []} source={i18next.t('browse.card.nexusSource')} />}{m?.category && <Chip className="border-orange-700/60 text-orange-300">{m.category}</Chip>}</>}
      loading={loading}
      main={
        <>
          <Gallery images={image ? [image] : []} />
          <Description source={data?.description} fallback={m?.summary} title={m?.name} onTitle={setTranslatedTitle} />
        </>
      }
      side={
        <>
          <div className="flex flex-col gap-2">
            {m && (
              <ActionButton onClick={() => onNexusDownload(m)} className="bg-orange-700 hover:bg-orange-600 text-white"
                title={i18next.t('browse.detail.downloadNexusTitle')}>
                <Download className="w-3.5 h-3.5" aria-hidden="true" /> {i18next.t('browse.detail.downloadNexus')}
              </ActionButton>
            )}
            <ActionButton onClick={() => TauriService.openExternalUrl(pageUrl)} className="bg-slate-800 hover:bg-slate-700 text-slate-200">
              <ExternalLink className="w-3.5 h-3.5" aria-hidden="true" /> {i18next.t('browse.detail.openNexus')}
            </ActionButton>
            {installed.length > 0 && <div className="text-[10px] font-mono text-emerald-400/80 break-all">{i18next.t('browse.detail.installedAs', { ids: installed.join(', ') })}</div>}
            {installed.length > 0 && <InstalledSupport paths={paths} modId={installed[0]} />}
            <p className="text-[10px] text-slate-500" dangerouslySetInnerHTML={{ __html: tHtml('browse.detail.afterDownload') }} />
          </div>
          {m && (
            <Section title={i18next.t('browse.detail.info')}>
              <InfoRow label={i18next.t('browse.detail.downloads')} value={formatCount(m.downloads)} />
              <InfoRow label={i18next.t('browse.detail.endorsements')} value={formatCount(m.endorsements)} />
              <InfoRow label={i18next.t('browse.detail.created')} value={formatIsoDate(m.created_at)} />
              <InfoRow label={i18next.t('browse.detail.updated')} value={formatIsoDate(m.updated_at)} />
              <InfoRow label={i18next.t('browse.detail.nexusId')} value={<span className="font-mono">{target.modId}</span>} />
            </Section>
          )}
          {data && data.requirements.length > 0 && (
            <Section title={i18next.t('browse.detail.requirements', { count: data.requirements.length })}>
              <div className="flex flex-col gap-1">
                {data.requirements.map((r, i) => {
                  const nexusId = !r.external && r.mod_id ? r.mod_id : null;
                  const have = nexusId !== null && installedByNexusId.has(nexusId);
                  return (
                    <button key={i} type="button"
                      onClick={() => (nexusId !== null ? onOpen({ kind: 'nexus', modId: nexusId }) : r.url && TauriService.openExternalUrl(r.url))}
                      className="flex flex-col gap-0.5 px-2 py-1.5 rounded border border-slate-800 bg-slate-950/60 hover:border-slate-600 text-left cursor-pointer">
                      <span className="flex items-center justify-between gap-2">
                        <span className="text-[11px] text-slate-200">{r.name}</span>
                        {have ? <InstalledChip /> : r.external ? <Chip className="border-slate-700 text-slate-400">{i18next.t('browse.detail.external')}</Chip> : null}
                      </span>
                      {r.notes && <span className="text-[10px] text-slate-500">{r.notes}</span>}
                    </button>
                  );
                })}
              </div>
            </Section>
          )}
          {data && data.files.length > 0 && (
            <Section title={i18next.t('browse.detail.files', { count: data.files.length })}>
              <div className="flex flex-col gap-1">
                {data.files.map((f, i) => (
                  <div key={i} className="flex flex-col gap-0.5 px-2 py-1.5 rounded border border-slate-800 bg-slate-950/60">
                    <div className="flex items-center justify-between gap-2">
                      <span className="flex items-center gap-1.5 text-[11px] text-slate-200 min-w-0">
                        <FileArchive className="w-3 h-3 shrink-0 text-slate-500" aria-hidden="true" />
                        <span className="truncate" title={f.name}>{f.name}</span>
                      </span>
                      <Chip className={FILE_STYLE[f.category] ?? 'border-slate-700 text-slate-400'}>{f.category.toLowerCase()}</Chip>
                    </div>
                    <div className="text-[10px] font-mono text-slate-500">
                      {[f.version && `v${f.version}`, formatBytes(f.size_bytes), formatUnixDate(f.uploaded)].filter(Boolean).join(' · ')}
                    </div>
                    {f.description && <div className="text-[10px] text-slate-400 line-clamp-3">{f.description}</div>}
                  </div>
                ))}
              </div>
            </Section>
          )}
          {m?.adult_content && <p className="text-[11px] text-amber-300">{i18next.t('browse.detail.adult')}</p>}
        </>
      }
    />
  );
};

/** A Workshop collection: what it holds and what you are missing. */
const CollectionDetail: React.FC<ModDetailProps & { target: { kind: 'collection' } & DetailTarget }> = ({ target, installedByItem, stateByItem, onOpen, steamReady, onSubscribe, onMakeProfile, keep }) => {
  const showContextMenu = useContextMenu();
  // The browse list already told us which mods the collection holds, so ask Steam once.
  const known = target.preview?.children ?? [];
  const { data, loading } = useDetails<WorkshopLookup>(
    async () =>
      known.length > 0
        ? { collection: target.preview ?? null, items: await TauriService.workshopDetails(known) }
        : TauriService.workshopLookup(target.id),
    target.id,
    i18next.t('browse.detail.loadingCollection')
  );
  const info = data?.collection ?? target.preview;
  const items = data?.items ?? [];
  const missing = items.filter((i) => !installedByItem.has(i.workshop_id));
  const pageUrl = info?.page_url ?? `https://steamcommunity.com/sharedfiles/filedetails/?id=${target.id}`;

  return (
    <DetailLayout
      title={info?.title ?? i18next.t('browse.detail.collection', { id: target.id })}
      subtitle={items.length > 0 ? i18next.t('browse.detail.modsInstalled', { count: items.length, installed: items.length - missing.length }) : undefined}
      chips={
        <>
          <Chip className="border-violet-600/60 text-violet-300">{i18next.t('browse.detail.collectionChip')}</Chip>
          {items.length > 0 && (missing.length === 0
            ? <Chip className="border-emerald-500/60 text-emerald-300">{i18next.t('browse.detail.allInstalled')}</Chip>
            : <Chip className="border-amber-500/60 text-amber-300">{i18next.t('browse.detail.missingChip', { count: missing.length })}</Chip>)}
        </>
      }
      loading={loading}
      main={
        <>
          {info?.summary && <p className="text-xs text-slate-400">{info.summary}</p>}
          {items.length === 0 && !loading && <p className="text-xs text-slate-500">{i18next.t('browse.detail.noItems')}</p>}
          <CardGrid>
            {items.map((i) => (
              <ModCard
                key={i.workshop_id}
                image={i.preview_url}
                title={i.found ? i.title : i18next.t('browse.card.item', { id: i.workshop_id })}
                subtitle={i18next.t('browse.card.updated', { date: formatUnixDate(i.time_updated) })}
                chips={
                  <>
                    {installedByItem.has(i.workshop_id)
                      ? <InstalledChip />
                      : <Chip className="border-amber-500/60 text-amber-300">{i18next.t('browse.detail.missing')}</Chip>}
                    <UpdateChips state={stateByItem.get(i.workshop_id)} />
                    <BuildChips builds={i.builds} source={i18next.t('browse.card.workshopSource')} />
                  </>
                }
                summary={i.summary}
                hover={i.found ? cardHover(i) : null}
                onOpen={() => onOpen({ kind: 'workshop', id: i.workshop_id, preview: i })}
                onContextMenu={i.found ? (e) => showContextMenu(e, workshopItemMenu(i, installedByItem.get(i.workshop_id) ?? [], steamReady, onSubscribe, keep)) : undefined}
              />
            ))}
          </CardGrid>
        </>
      }
      side={
        <>
          {missing.length > 0 && (
            <SubscribeButton ids={missing.map((i) => i.workshop_id)} installed={false} steamReady={steamReady} onSubscribe={onSubscribe}
              allLabel={i18next.t('browse.detail.subscribeMissing', { count: missing.length })} />
          )}
          {items.length > 0 && missing.length === 0 && (
            <SubscribeButton ids={items.map((i) => i.workshop_id)} installed steamReady={steamReady} onSubscribe={onSubscribe} allLabel={i18next.t('browse.detail.all')} />
          )}
          {onMakeProfile && items.length > 0 && (
            <ActionButton
              onClick={() => onMakeProfile(info?.title ?? i18next.t('browse.detail.collection', { id: target.id }), items.map((i) => i.workshop_id))}
              className="bg-violet-800 hover:bg-violet-700 text-white"
              title={i18next.t('browse.detail.makeProfileTitle')}
            >
              <Layers className="w-3.5 h-3.5" aria-hidden="true" /> {i18next.t('browse.detail.makeProfile')}
            </ActionButton>
          )}
          <ActionButton onClick={() => TauriService.openExternalUrl(pageUrl)} className="bg-sky-700 hover:bg-sky-600 text-white"
            title={i18next.t('browse.detail.openCollectionTitle')}>
            <ExternalLink className="w-3.5 h-3.5" aria-hidden="true" /> {i18next.t('browse.detail.openCollectionPage')}
          </ActionButton>
          <p className="text-[10px] text-slate-500">
            {i18next.t('browse.detail.steamDownloads')}
          </p>
          {info && (
            <Section title={i18next.t('browse.detail.info')}>
              <InfoRow label={i18next.t('browse.detail.mods')} value={items.length || info.children.length} />
              <InfoRow label={i18next.t('browse.detail.missingLabel')} value={missing.length || undefined} />
              <InfoRow label={i18next.t('browse.detail.subscribers')} value={<span className="inline-flex items-center gap-1"><Users className="w-3 h-3" />{formatCount(info.subscriptions)}</span>} />
              <InfoRow label={i18next.t('browse.detail.favorites')} value={<span className="inline-flex items-center gap-1"><Heart className="w-3 h-3" />{formatCount(info.favorited)}</span>} />
              <InfoRow label={i18next.t('browse.detail.updated')} value={formatUnixDate(info.time_updated)} />
              <InfoRow label={i18next.t('browse.detail.workshopId')} value={<span className="font-mono">{target.id}</span>} />
            </Section>
          )}
          {missing.length > 0 && (
            <Section title={i18next.t('browse.detail.missingSection', { count: missing.length })}>
              <div className="flex flex-col gap-1">
                {missing.map((i) => (
                  <button key={i.workshop_id} type="button" onClick={() => onOpen({ kind: 'workshop', id: i.workshop_id, preview: i })}
                    className="px-2 py-1.5 rounded border border-slate-800 bg-slate-950/60 hover:border-slate-600 text-left text-[11px] text-slate-200 cursor-pointer line-clamp-2">
                    {i.found ? i.title : i.workshop_id}
                  </button>
                ))}
              </div>
            </Section>
          )}
        </>
      }
    />
  );
};

const DetailLayout: React.FC<{
  title: string;
  subtitle?: string;
  chips: React.ReactNode;
  loading: boolean;
  main: React.ReactNode;
  side: React.ReactNode;
}> = ({ title, subtitle, chips, loading, main, side }) => (
  <>
    <div className="flex flex-col gap-1 min-w-0">
      <h2 className="text-base font-bold text-slate-100 leading-snug flex items-center gap-2">
        <span className="min-w-0">{title}</span>
        {loading && <Loader2 className="w-4 h-4 animate-spin text-slate-500 shrink-0" aria-label={i18next.t('browse.detail.loading')} />}
      </h2>
      <div className="flex flex-wrap items-center gap-1.5">
        {subtitle && <span className="text-[11px] text-slate-400 mr-1">{subtitle}</span>}
        {chips}
      </div>
    </div>
    <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_280px] mt-3">
      <div className="flex flex-col gap-4 min-w-0">{main}</div>
      <aside className="flex flex-col gap-4">{side}</aside>
    </div>
  </>
);

/** Full-page detail view over the Browse tab (Esc closes). */
export const ModDetail: React.FC<ModDetailProps> = (props) => {
  const { target, canGoBack, onBack, onClose } = props;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);
  const scrollRef = React.useRef<HTMLDivElement>(null);
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [target]);

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4" onClick={onClose}>
      <div role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}
        className="w-full max-w-5xl max-h-full flex flex-col rounded-xl border border-slate-700 bg-slate-900 shadow-2xl overflow-hidden">
        <div className="flex items-center justify-between gap-2 px-4 py-2 border-b border-slate-800 bg-slate-950/60">
          <div className="flex items-center gap-2">
            {canGoBack && (
              <button type="button" onClick={onBack} className="flex items-center gap-1 px-2 py-1 rounded text-[11px] text-slate-300 hover:bg-slate-800 cursor-pointer">
                <ArrowLeft className="w-3.5 h-3.5" aria-hidden="true" /> {i18next.t('browse.detail.back')}
              </button>
            )}
            <span className={`text-[10px] font-mono font-bold px-1.5 py-0.5 rounded border ${target.kind === 'nexus' ? 'border-orange-700 text-orange-300' : 'border-sky-700 text-sky-300'}`}>
              {target.kind === 'nexus' ? i18next.t('browse.detail.nexusKind') : target.kind === 'collection' ? i18next.t('browse.detail.collectionKind') : i18next.t('browse.detail.workshopKind')}
            </span>
          </div>
          <button type="button" onClick={onClose} className="p-1 rounded hover:bg-slate-800 text-slate-400 hover:text-slate-200 cursor-pointer" aria-label={i18next.t('browse.detail.close')}>
            <X className="w-4 h-4" />
          </button>
        </div>
        <div ref={scrollRef} className="overflow-y-auto p-5">
          {/* Keyed by item: a button's "subscribed" must not carry over to the next item opened. */}
          {target.kind === 'workshop' && <WorkshopDetail key={`w:${target.id}`} {...props} target={target} />}
          {target.kind === 'collection' && <CollectionDetail key={`c:${target.id}`} {...props} target={target} />}
          {target.kind === 'nexus' && <NexusDetail key={`n:${target.modId}`} {...props} target={target} />}
        </div>
      </div>
    </div>
  );
};
