import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  AlertTriangle, CheckCircle2, Copy, Download, ExternalLink, FileArchive, FolderOpen, Globe, Heart, KeyRound, Layers, Library, Link2, Loader2, Languages, Plus, Search, Trash2, Users, X,
} from 'lucide-react';
import {
  CatalogModSummary, InstallResult, ModOrigin, NexusCategory, NexusMod, NexusSort, WorkshopInstallState, WorkshopItem, WorkshopLookup, WorkshopSort,
  AppInstance,
  RequiredChain,
} from '../../types';
import { SubscribeRequirements } from './SubscribeRequirements';
import { appLanguage } from '../../utils/language';
import { KeepActions, useKeepActions } from '../common/keep';
import { ListsTab } from './ListsTab';
import i18next from 'i18next';
import { useTranslation } from 'react-i18next';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, explainError, reportBackendError } from '../../services/tauri';
import {
  BuildChips, CardGrid, CategoryBar, cardHover, workshopItemMenu, Chip, InstalledChip, ModCard, PAGE_SIZE, Pagination, SCROLL_ID, Toolbar, UpdateChips, formatCount, formatIsoDate, formatUnixDate, scrollBrowseToTop, selectClass,
} from './common';
import { DetailTarget, ModDetail } from './ModDetail';
import { useContextMenu } from '../common/ContextMenu';
import { tHtml } from '../../i18n/html';

interface BrowseModuleProps {
  paths: StudioPathsUI;
  /** Workshop item to open the detail view on (set when the Mod List sends the user here). */
  focusWorkshopId?: string | null;
  onFocusHandled?: () => void;
  /** Text to search the Workshop for (set when Mod Health or the Mod List look for a missing mod). */
  focusSearch?: string | null;
  onSearchHandled?: () => void;
  /** Installed mods as the game sees them (from useModHealth). */
  catalog: CatalogModSummary[];
  /** Called after mods were installed, so the mod list is rescanned. */
  onInstalled: () => void;
  /** Workshop items Steam accepted a subscription to: they land later, when Steam has downloaded them. */
  onSubscribed?: (workshopIds: string[]) => void;
  /** Makes a profile from a collection: the Studio's mod list and profiles live above here. */
  onMakeProfile?: (name: string, workshopIds: string[]) => void;
  /** The player's profiles, for "Add to a profile…", and what to call after one changed. */
  profiles?: AppInstance[];
  onProfilesChanged?: () => void;
  /** The game build installed, for the community index's lines on the detail page. */
  installedBuild?: string | null;
}

type SourceTab = 'workshop' | 'nexus' | 'mine' | 'lists';

const CONFLICT_PREFIX = 'INSTALL_CONFLICT:';

/** The Workshop's categories for Project Zomboid (same list as workshop::CATEGORIES). */
const WORKSHOP_CATEGORIES = [
  'Animals', 'Audio', 'Balance', 'Building', 'Clothing/Armor', 'Farming', 'Food', 'Framework', 'Hardmode', 'Interface', 'Items',
  'Language/Translation', 'Literature', 'Map', 'Military', 'Misc', 'Models', 'Multiplayer', 'Pop Culture', 'QoL', 'Realistic',
  'Silly/Fun', 'Skills', 'Textures', 'Traits', 'Vehicles', 'Weapons', 'WIP',
].map((name) => ({ name }));

const pageCountOf = (total: number) => Math.max(1, Math.ceil(total / PAGE_SIZE));

/** Pasted Workshop links / ids instead of search words. */
const looksLikeWorkshopLinks = (text: string) => /steamcommunity\.com|CommunityFilePage/i.test(text) || /^[\d\s,;]{4,}$/.test(text.trim());

// ---------------------------------------------------------------------------------------------
// Installing a downloaded mod
// ---------------------------------------------------------------------------------------------

function useInstaller(paths: StudioPathsUI, pendingOrigin: (ModOrigin & { kind: 'nexus' }) | null, onDone: () => void) {
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<InstallResult | null>(null);

  const install = async (source: string) => {
    setBusy(true);
    setResult(null);
    const origin = pendingOrigin ?? undefined;
    try {
      let res: InstallResult;
      try {
        res = await TauriService.installMods(source, paths, false, origin);
      } catch (err) {
        const text = String(err);
        if (!text.startsWith(CONFLICT_PREFIX)) throw err;
        const { existing_folders } = JSON.parse(text.slice(CONFLICT_PREFIX.length)) as { existing_folders: string[] };
        if (!confirm(i18next.t('browse.alreadyInstalled', { folders: existing_folders.join(', ') }))) return;
        res = await TauriService.installMods(source, paths, true, origin);
      }
      setResult(res);
      TauriService.clearRemoteCache('workshop:');
      onDone();
    } catch (err) {
      reportBackendError('Installing the mod', err);
    } finally {
      setBusy(false);
    }
  };

  return {
    busy,
    result,
    clearResult: () => setResult(null),
    pickZip: async () => {
      const file = await TauriService.pickOpenFile(i18next.t('browse.archiveFilter'), 'zip,7z');
      if (file) await install(file);
    },
    pickFolder: async () => {
      const folder = await TauriService.pickFolder();
      if (folder) await install(folder);
    },
    /**
     * From a link: a direct .zip, or a GitHub repository or release. What it will download is
     * worked out and shown first, because a link does not say what is behind it.
     */
    fromUrl: async (url: string): Promise<boolean> => {
      setBusy(true);
      setResult(null);
      try {
        const plan = await TauriService.resolveUrlInstall(url);
        const size = plan.size ? ` (${(plan.size / (1024 * 1024)).toFixed(1)} MB)` : '';
        if (!confirm(i18next.t('browse.downloadConfirm', { file: plan.file_name, size, source: plan.source, url: plan.download_url }))) {
          return false;
        }
        let res: InstallResult;
        try {
          res = await TauriService.installFromUrl(paths, url, false);
        } catch (err) {
          const text = String(err);
          if (!text.startsWith(CONFLICT_PREFIX)) throw err;
          const { existing_folders } = JSON.parse(text.slice(CONFLICT_PREFIX.length)) as { existing_folders: string[] };
          if (!confirm(i18next.t('browse.alreadyInstalled', { folders: existing_folders.join(', ') }))) return false;
          res = await TauriService.installFromUrl(paths, url, true);
        }
        setResult(res);
        TauriService.clearRemoteCache('workshop:');
        onDone();
        return true;
      } catch (err) {
        reportBackendError('Installing from that link', err);
        return false;
      } finally {
        setBusy(false);
      }
    },
  };
}

const InstallResultBanner: React.FC<{ result: InstallResult; onClose: () => void }> = ({ result, onClose }) => (
  <div className="flex items-start gap-3 px-3 py-2 rounded-lg border border-emerald-800/60 bg-emerald-950/20 text-xs">
    <div className="flex-1 flex flex-col gap-1">
      {result.installed.map((m) => (
        <div key={m.destination} className="flex items-center gap-2 text-emerald-300">
          <CheckCircle2 className="w-3.5 h-3.5 shrink-0" aria-hidden="true" />
          <span>
            {m.replaced ? i18next.t('browse.replaced') : i18next.t('browse.installed')} <b>{m.name || m.folder}</b>
            {m.mod_id && <span className="font-mono text-emerald-400/70"> ({m.mod_id})</span>}{i18next.t('browse.enableInList')}
          </span>
        </div>
      ))}
      {result.warnings.map((w, i) => (
        <div key={i} className="flex items-start gap-2 text-amber-300">
          <AlertTriangle className="w-3.5 h-3.5 shrink-0 mt-0.5" aria-hidden="true" />
          <span>{w}</span>
        </div>
      ))}
    </div>
    <button type="button" onClick={onClose} className="p-0.5 rounded hover:bg-emerald-900/50 text-slate-400 cursor-pointer" aria-label={i18next.t('browse.close')}>
      <X className="w-3.5 h-3.5" />
    </button>
  </div>
);

// ---------------------------------------------------------------------------------------------
// Steam Workshop
// ---------------------------------------------------------------------------------------------

const workshopCard = (
  item: WorkshopItem,
  installedIds: string[],
  state: WorkshopInstallState | undefined,
  onOpen: (t: DetailTarget) => void,
  menu?: (e: React.MouseEvent, item: WorkshopItem, installed: boolean) => void,
  translated?: { title: string; summary: string }
) => (
  <ModCard
    key={item.workshop_id}
    image={item.preview_url}
    title={item.found ? translated?.title || item.title : i18next.t('browse.card.item', { id: item.workshop_id })}
    subtitle={item.found ? i18next.t('browse.card.updated', { date: formatUnixDate(item.time_updated) }) : i18next.t('browse.card.notPublic')}
    chips={<>{installedIds.length > 0 && <InstalledChip />}<UpdateChips state={state} />{item.found && <BuildChips builds={item.builds} source={i18next.t('browse.card.workshopSource')} />}</>}
    summary={translated?.summary ?? item.summary}
    hover={item.found ? cardHover(item) : null}
    stats={item.found && (
      <>
        <span className="flex items-center gap-1" title={i18next.t('browse.card.subscribers')}><Users className="w-3 h-3" aria-hidden="true" />{formatCount(item.subscriptions)}</span>
        <span className="flex items-center gap-1" title={i18next.t('browse.card.favorites')}><Heart className="w-3 h-3" aria-hidden="true" />{formatCount(item.favorited)}</span>
      </>
    )}
    onOpen={item.found ? () => onOpen({ kind: 'workshop', id: item.workshop_id, preview: item }) : undefined}
    onContextMenu={menu && item.found ? (e) => menu(e, item, installedIds.length > 0) : undefined}
  />
);

// The same four the Workshop's own Browse menu offers, in its wording.
/** A collection: its picture, how many of its mods you already have, and a way in. */
const collectionCard = (
  item: WorkshopItem,
  installedByItem: Map<string, string[]>,
  onOpen: (t: DetailTarget) => void,
  menu?: (e: React.MouseEvent, item: WorkshopItem, missingIds: string[]) => void
) => {
  const missingIds = item.children.filter((id) => !installedByItem.has(id));
  const missing = missingIds.length;
  return (
    <ModCard
      key={item.workshop_id}
      image={item.preview_url}
      title={item.title}
      subtitle={i18next.t('browse.card.modsUpdated', { count: item.children.length, date: formatUnixDate(item.time_updated) })}
      chips={
        <>
          <Chip className="border-violet-600/60 text-violet-300">{i18next.t('browse.card.collection')}</Chip>
          {item.children.length > 0 && (
            missing === 0
              ? <Chip className="border-emerald-500/60 text-emerald-300">{i18next.t('browse.card.allInstalled')}</Chip>
              : <Chip className="border-amber-500/60 text-amber-300">{i18next.t('browse.card.missing', { count: missing })}</Chip>
          )}
        </>
      }
      summary={item.summary}
      stats={
        <>
          <span className="flex items-center gap-1" title={i18next.t('browse.card.subscribers')}><Users className="w-3 h-3" aria-hidden="true" />{formatCount(item.subscriptions)}</span>
          <span className="flex items-center gap-1" title={i18next.t('browse.card.favorites')}><Heart className="w-3 h-3" aria-hidden="true" />{formatCount(item.favorited)}</span>
        </>
      }
      onOpen={() => onOpen({ kind: 'collection', id: item.workshop_id, preview: item })}
      onContextMenu={menu ? (e) => menu(e, item, missingIds) : undefined}
      hover={cardHover(item)}
    />
  );
};

const WORKSHOP_SORTS: WorkshopSort[] = ['trending', 'top_rated', 'newest', 'recently_updated'];

/** The time window the Workshop offers next to "Most Popular". */
const POPULAR_DAYS = [1, 7, 90, 365];

/** Fallback when the public Workshop page cannot be read: the user's own Steam Web API key. */
const SteamKeyForm: React.FC<{ onSaved: () => void; reason: string }> = ({ onSaved, reason }) => {
  const [keyInput, setKeyInput] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await TauriService.setSteamKey(keyInput);
      setKeyInput('');
      onSaved();
    } catch (err) {
      setError(explainError(err));
    } finally {
      setSaving(false);
    }
  };
  return (
    <div className="flex flex-col gap-2 px-3 py-2.5 rounded-lg border border-amber-900/60 bg-amber-950/20">
      <div className="flex items-start gap-2 text-[11px] text-slate-300">
        <KeyRound className="w-4 h-4 text-amber-400 shrink-0" aria-hidden="true" />
        <span>
          <b className="text-amber-200">{reason}</b><span dangerouslySetInnerHTML={{ __html: tHtml('browse.steamKey.blurb') }} />
        </span>
      </div>
      <form className="flex flex-wrap gap-2" onSubmit={(e) => { e.preventDefault(); save(); }}>
        <button type="button" onClick={() => TauriService.openExternalUrl('https://steamcommunity.com/dev/apikey')}
          className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer">
          <ExternalLink className="w-3 h-3" aria-hidden="true" /> {i18next.t('browse.steamKey.get')}
        </button>
        <input type="password" autoComplete="off" spellCheck={false} value={keyInput} onChange={(e) => setKeyInput(e.target.value)}
          placeholder={i18next.t('browse.steamKey.paste')} aria-label={i18next.t('browse.steamKey.label')}
          className="flex-1 min-w-[220px] px-2.5 py-1.5 rounded-lg bg-slate-950 border border-slate-800 focus:border-sky-700 text-[11px] text-slate-100 outline-none placeholder:text-slate-600 font-mono" />
        <button type="submit" disabled={saving || !keyInput.trim()}
          className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer disabled:opacity-50">
          {saving && <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" />} {i18next.t('browse.steamKey.save')}
        </button>
      </form>
      {error && <p className="text-[11px] text-red-300">{error}</p>}
    </div>
  );
};

type WorkshopQuery = { text: string; sort: WorkshopSort; build: string; category: string; days: number; collections: boolean };

/** The filters as the backend wants them (collections are not filtered by build or category). */
const toBrowseQuery = (q: WorkshopQuery, page: number) => ({
  text: q.text,
  sort: q.sort,
  build: q.collections ? null : q.build,
  category: q.collections ? null : q.category,
  days: q.days,
  collections: q.collections,
  page,
});

const WorkshopTab: React.FC<{
  installedByItem: Map<string, string[]>;
  stateByItem: Map<string, WorkshopInstallState>;
  onOpen: (t: DetailTarget) => void;
  steamReady: boolean;
  onSubscribe: (ids: string[], subscribe: boolean) => Promise<void>;
  keep: KeepActions;
  searchFor?: string | null;
  onSearchHandled?: () => void;
}> = ({ installedByItem, stateByItem, onOpen, steamReady, onSubscribe, keep, searchFor, onSearchHandled }) => {
  const showContextMenu = useContextMenu();
  const [text, setText] = useState('');
  const [query, setQuery] = useState<WorkshopQuery>({ text: '', sort: 'trending', build: '42', category: '', days: 7, collections: false });
  const [page, setPage] = useState(1);
  const [pages, setPages] = useState(1);
  const [items, setItems] = useState<WorkshopItem[]>([]);
  const [total, setTotal] = useState(0);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [hasSavedKey, setHasSavedKey] = useState(false);
  const [lookup, setLookup] = useState<WorkshopLookup | null>(null);
  const [lookupPage, setLookupPage] = useState(1);
  /** Card summaries in the app's language, by item, once "Translate summaries" was pressed. */
  const [summaries, setSummaries] = useState<Map<string, { title: string; summary: string }>>(new Map());
  const [translatingSummaries, setTranslatingSummaries] = useState(false);
  const [translateError, setTranslateError] = useState<string | null>(null);
  const translateSummaries = async () => {
    const todo = items.filter((i) => i.found && (i.summary || i.title) && !summaries.has(i.workshop_id));
    if (todo.length === 0) return;
    setTranslatingSummaries(true);
    setTranslateError(null);
    try {
      // Title and summary of each item, in pairs.
      const res = await TauriService.translateTexts(todo.flatMap((i) => [i.title, i.summary]), appLanguage());
      setSummaries((prev) => {
        const next = new Map(prev);
        todo.forEach((i, k) => next.set(i.workshop_id, { title: res.texts[2 * k] || i.title, summary: res.texts[2 * k + 1] ?? i.summary }));
        return next;
      });
    } catch (err) {
      setTranslateError(String(err));
    } finally {
      setTranslatingSummaries(false);
    }
  };

  useEffect(() => {
    TauriService.steamKeyStatus().then(setHasSavedKey).catch(() => setHasSavedKey(false));
  }, []);

  /** Takes the saved key out of Windows Credential Manager. */
  const removeKey = async () => {
    if (!confirm(i18next.t('browse.steamKey.removeConfirm'))) return;
    try {
      await TauriService.deleteSteamKey();
      setHasSavedKey(false);
    } catch (err) {
      reportBackendError('Removing the Steam key', err);
    }
  };

  // Only the latest search may fill the list: one sent from another screen can overtake the
  // default one the tab starts with.
  const searchTicket = useRef(0);
  const runSearch = useCallback(async (q: WorkshopQuery, p: number) => {
    const ticket = ++searchTicket.current;
    setSearching(true);
    try {
      const r = await TauriService.workshopSearch(toBrowseQuery(q, p));
      if (ticket !== searchTicket.current) return;
      setTotal(r.total);
      setPages(r.pages);
      setItems(r.items);
      setPage(r.page || p);
      setSearchError(null);
      TauriService.steamKeyStatus().then(setHasSavedKey).catch(() => undefined);
      // The next page is usually where the user goes: fetch it while they read this one.
      const next = { ...toBrowseQuery(q, p + 1) };
      if (p + 1 <= r.pages) TauriService.prefetch(TauriService.workshopSearchKey(next), () => TauriService.workshopSearch(next));
    } catch (err) {
      if (ticket !== searchTicket.current) return;
      setSearchError(explainError(err));
      setItems([]);
      setTotal(0);
    } finally {
      if (ticket === searchTicket.current) setSearching(false);
    }
  }, []);

  useEffect(() => {
    if (!lookup) runSearch(query, 1);
    // Re-run when a filter changes; the text only on submit.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query]);

  const submit = async () => {
    const t = text.trim();
    if (t && looksLikeWorkshopLinks(t)) {
      setSearching(true);
      try {
        setLookup(await TauriService.workshopLookup(t));
        setLookupPage(1);
      } catch (err) {
        reportBackendError('Looking up the Workshop link', err);
      } finally {
        setSearching(false);
      }
      return;
    }
    setLookup(null);
    setQuery((q) => ({ ...q, text: t }));
  };

  // Another screen asked for a search: the text in the box, by relevance, in every category.
  useEffect(() => {
    if (!searchFor) return;
    setText(searchFor);
    setLookup(null);
    setQuery((q) => ({ ...q, text: searchFor, sort: 'relevance', category: '', collections: false }));
    onSearchHandled?.();
  }, [searchFor, onSearchHandled]);

  const setFilter = (patch: Partial<WorkshopQuery>) => {
    setLookup(null);
    setQuery((q) => ({ ...q, ...patch }));
  };

  const cardMenu = (e: React.MouseEvent, item: WorkshopItem) =>
    showContextMenu(e, workshopItemMenu(item, installedByItem.get(item.workshop_id) ?? [], steamReady, onSubscribe, keep));

  /** A collection card's menu: what is missing of it, its page, its link and id. */
  const collectionMenu = (e: React.MouseEvent, item: WorkshopItem, missingIds: string[]) =>
    showContextMenu(e, [
      {
        label: i18next.t('browse.workshop.subscribeMissing', { count: missingIds.length }),
        icon: Plus,
        onSelect: () => onSubscribe(missingIds, true),
        disabled: !steamReady || missingIds.length === 0,
        disabledReason: missingIds.length === 0 ? i18next.t('browse.card.allInstalled') : i18next.t('browse.workshop.steamNotRunning'),
      },
      { label: i18next.t('browse.workshop.showCollection'), icon: Layers, separatorBefore: true,
        onSelect: () => onOpen({ kind: 'collection', id: item.workshop_id, preview: item }) },
      { label: i18next.t('browse.workshop.openPage'), icon: ExternalLink,
        onSelect: () => TauriService.openExternalUrl(item.page_url) },
      { label: i18next.t('browse.workshop.copyLink'), icon: Copy, onSelect: () => navigator.clipboard.writeText(item.page_url) },
      { label: i18next.t('browse.workshop.copyId'), icon: Copy, onSelect: () => navigator.clipboard.writeText(item.workshop_id) },
    ]);

  const lookupMissing = lookup ? lookup.items.filter((i) => !installedByItem.has(i.workshop_id)).length : 0;
  const lookupItems = lookup ? lookup.items.slice((lookupPage - 1) * PAGE_SIZE, lookupPage * PAGE_SIZE) : [];

  return (
    <div className="flex flex-col gap-3">
      <Toolbar onSubmit={submit}>
        <div className="flex items-center gap-2 flex-1 min-w-[240px] px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-800 focus-within:border-sky-700">
          <Search className="w-3.5 h-3.5 text-slate-500" aria-hidden="true" />
          <input value={text} onChange={(e) => setText(e.target.value)} aria-label={i18next.t('browse.workshop.searchLabel')}
            placeholder={i18next.t('browse.workshop.searchPlaceholder')}
            className="flex-1 bg-transparent text-xs text-slate-100 outline-none placeholder:text-slate-600" />
          {(text || lookup) && (
            <button type="button" onClick={() => { setText(''); setLookup(null); if (query.text) setFilter({ text: '' }); }} aria-label={i18next.t('browse.workshop.clear')}
              className="text-slate-500 hover:text-slate-300 cursor-pointer"><X className="w-3.5 h-3.5" /></button>
          )}
        </div>
        <select value={query.text ? 'relevance' : query.sort} onChange={(e) => setFilter({ sort: e.target.value as WorkshopSort })} aria-label={i18next.t('browse.workshop.sort')}
          disabled={!!query.text} title={query.text ? i18next.t('browse.workshop.relevanceOnly') : undefined} className={selectClass}>
          {query.text && <option value="relevance">{i18next.t('browse.sorts.relevance')}</option>}
          {WORKSHOP_SORTS.map((id) => <option key={id} value={id}>{i18next.t(`browse.sorts.${id}`)}</option>)}
        </select>
        {query.sort === 'trending' && !query.text && (
          <select value={query.days} onChange={(e) => setFilter({ days: Number(e.target.value) })} aria-label={i18next.t('browse.workshop.popularOver')} className={selectClass}>
            {POPULAR_DAYS.map((d) => <option key={d} value={d}>{i18next.t(`browse.days.${d}`)}</option>)}
          </select>
        )}
        {!query.collections && (
          <select value={query.build} onChange={(e) => setFilter({ build: e.target.value })} aria-label={i18next.t('browse.workshop.build')} className={selectClass}>
            <option value="42">{i18next.t('browse.workshop.build42')}</option>
            <option value="41">{i18next.t('browse.workshop.build41')}</option>
            <option value="">{i18next.t('browse.workshop.anyBuild')}</option>
          </select>
        )}
        <button type="submit" disabled={searching} className="px-3 py-1.5 rounded-lg text-xs font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer disabled:opacity-60">
          {i18next.t('browse.workshop.search')}
        </button>
        <button type="button" onClick={() => TauriService.openExternalUrl('https://steamcommunity.com/app/108600/workshop/')} title={i18next.t('browse.workshop.webTitle')}
          className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs text-slate-300 bg-slate-900 border border-slate-800 hover:bg-slate-800 cursor-pointer">
          <Globe className="w-3.5 h-3.5" aria-hidden="true" /> {i18next.t('browse.workshop.web')}
        </button>
      </Toolbar>

      <div className="flex flex-wrap items-center gap-3">
        <div className="flex gap-1 p-0.5 rounded-lg bg-slate-900 border border-slate-800" role="group" aria-label={i18next.t('browse.workshop.whatToList')}>
          {[
            { id: false, label: i18next.t('browse.workshop.mods') },
            { id: true, label: i18next.t('browse.workshop.collections') },
          ].map((o) => (
            <button key={String(o.id)} type="button" onClick={() => setFilter({ collections: o.id })} aria-pressed={query.collections === o.id}
              className={`px-2.5 py-1 rounded-md text-[11px] font-bold cursor-pointer transition ${query.collections === o.id ? 'bg-sky-800/80 text-sky-100' : 'text-slate-400 hover:text-slate-200'}`}>
              {o.label}
            </button>
          ))}
        </div>
        {!query.collections && (
          <CategoryBar categories={WORKSHOP_CATEGORIES} value={query.category} onChange={(c) => setFilter({ category: c })} accent="border-sky-600 bg-sky-900/50 text-sky-200" />
        )}
      </div>
      {searchError && !lookup && (searchError === i18next.t('backend.explain.steamBusy') ? (
        // Steam itself is refusing searches: a key would not help, trying again later will.
        <div className="flex flex-wrap items-center gap-3 px-3 py-2.5 rounded-lg border border-amber-700/60 bg-amber-950/30 text-xs text-amber-100">
          <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0" aria-hidden="true" />
          <span className="flex-1 min-w-[240px]">{searchError}</span>
          <button type="button" onClick={() => runSearch(query, page)} className="px-3 py-1 rounded-md bg-amber-700 hover:bg-amber-600 text-white font-bold cursor-pointer">
            {i18next.t('browse.workshop.tryAgain')}
          </button>
        </div>
      ) : (
        <SteamKeyForm reason={searchError} onSaved={() => runSearch(query, 1)} />
      ))}
      {hasSavedKey && (
        <div className="flex flex-wrap items-center gap-2 text-[11px] text-slate-500">
          <KeyRound className="w-3 h-3" aria-hidden="true" />
          <span>{i18next.t('browse.steamKey.saved')}</span>
          <button type="button" onClick={removeKey} className="underline hover:text-slate-300 cursor-pointer">{i18next.t('browse.steamKey.removeIt')}</button>
        </div>
      )}

      {lookup ? (
        <>
          <div className="flex flex-wrap items-center gap-2 text-[11px] text-slate-300">
            {lookup.collection ? (
              <>
                <Library className="w-3.5 h-3.5 text-sky-400" aria-hidden="true" />
                <span dangerouslySetInnerHTML={{ __html: tHtml('browse.workshop.collectionLine', { title: lookup.collection.title, count: lookup.items.length }) }} />
                <span className={lookupMissing ? 'text-amber-300' : 'text-emerald-300'}>{i18next.t('browse.workshop.notInstalled', { count: lookupMissing })}</span>
                <button type="button" onClick={() => TauriService.openExternalUrl(lookup.collection!.page_url)}
                  className="px-2 py-0.5 rounded text-[11px] font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer" title={i18next.t('browse.workshop.openCollectionTitle')}>
                  {i18next.t('browse.workshop.openCollection')}
                </button>
              </>
            ) : (
              <>{i18next.t('browse.workshop.itemsNotInstalled', { count: lookup.items.length, missing: lookupMissing })}</>
            )}
          </div>
          <Pagination page={lookupPage} pageCount={pageCountOf(lookup.items.length)} onPage={setLookupPage} />
          <CardGrid>{lookupItems.map((i) => workshopCard(i, installedByItem.get(i.workshop_id) ?? [], stateByItem.get(i.workshop_id), onOpen, cardMenu))}</CardGrid>
          <Pagination page={lookupPage} pageCount={pageCountOf(lookup.items.length)} onPage={setLookupPage} />
        </>
      ) : (
        <>
          <div className="flex items-center justify-between text-[11px] font-mono text-slate-500">
            <span>
              {searching && items.length === 0
                ? i18next.t('browse.workshop.searching')
                : i18next.t('browse.workshop.results', { total: total.toLocaleString(), what: query.collections ? i18next.t('browse.workshop.collections').toLowerCase() : i18next.t('browse.workshop.mods').toLowerCase(), text: query.text ? i18next.t('browse.workshop.forText', { text: query.text }) : '', category: query.category ? i18next.t('browse.workshop.inCategory', { category: query.category }) : '' })}
              {searching && items.length > 0 && <Loader2 className="inline w-3 h-3 ml-2 animate-spin" aria-hidden="true" />}
            </span>
            <span className="flex items-center gap-3">
              {!query.collections && items.some((i) => i.found && (i.summary || i.title)) && (
                <button type="button" onClick={() => void translateSummaries()} title={i18next.t('browse.detail.translateSummariesTitle')}
                  disabled={translatingSummaries || items.every((i) => !i.found || summaries.has(i.workshop_id))}
                  className="flex items-center gap-1 text-sky-300 hover:text-sky-200 cursor-pointer disabled:opacity-40 disabled:cursor-default">
                  {translatingSummaries ? <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" /> : <Languages className="w-3 h-3" aria-hidden="true" />}
                  {i18next.t('browse.detail.translateSummaries')}
                </button>
              )}
              {translateError && <span className="text-red-300">{i18next.t('browse.detail.translateFailed', { reason: translateError })}</span>}
              <span>{i18next.t('browse.workshop.page', { page, pages: pages.toLocaleString() })}</span>
            </span>
          </div>
          <Pagination page={page} pageCount={pages} onPage={(p) => runSearch(query, p)} disabled={searching} />
          <CardGrid>
            {items.map((i) =>
              query.collections
                ? collectionCard(i, installedByItem, onOpen, collectionMenu)
                : workshopCard(i, installedByItem.get(i.workshop_id) ?? [], stateByItem.get(i.workshop_id), onOpen, cardMenu, summaries.get(i.workshop_id))
            )}
          </CardGrid>
          <Pagination page={page} pageCount={pages} onPage={(p) => runSearch(query, p)} disabled={searching} />
        </>
      )}
    </div>
  );
};

// ---------------------------------------------------------------------------------------------
// Nexus Mods
// ---------------------------------------------------------------------------------------------

const NEXUS_SORTS: NexusSort[] = ['updated', 'downloads', 'endorsements', 'newest'];

const nexusCard = (m: NexusMod, installed: boolean, onOpen: (t: DetailTarget) => void, menu?: (e: React.MouseEvent, m: NexusMod) => void) => (
  <ModCard
    key={m.mod_id}
    image={m.thumbnail_url}
    title={m.name}
    subtitle={`${i18next.t('browse.card.by', { author: m.author || m.uploader || i18next.t('browse.card.unknown') })}${m.version ? ` · v${m.version}` : ''} · ${formatIsoDate(m.updated_at)}`}
    chips={<>{installed && <InstalledChip />}<BuildChips builds={m.build_hint ? m.build_hint.split('+') : []} source={i18next.t('browse.card.nexusSource')} />{m.category && <Chip className="border-orange-700/60 text-orange-300">{m.category}</Chip>}</>}
    summary={m.summary}
    stats={
      <>
        <span className="flex items-center gap-1" title={i18next.t('browse.card.downloads')}><Download className="w-3 h-3" aria-hidden="true" />{formatCount(m.downloads)}</span>
        <span className="flex items-center gap-1" title={i18next.t('browse.card.endorsements')}><Heart className="w-3 h-3" aria-hidden="true" />{formatCount(m.endorsements)}</span>
      </>
    }
    onOpen={() => onOpen({ kind: 'nexus', modId: m.mod_id, preview: m })}
    onContextMenu={menu ? (e) => menu(e, m) : undefined}
  />
);

const NexusTab: React.FC<{ installedByNexusId: Map<number, string[]>; onOpen: (t: DetailTarget) => void; keep: KeepActions }> = ({ installedByNexusId, onOpen, keep }) => {
  const showContextMenu = useContextMenu();
  const nexusMenu = (e: React.MouseEvent, m: NexusMod) =>
    showContextMenu(e, keep.menuItems({ nexus_id: m.mod_id, name: m.name, installedModIds: installedByNexusId.get(m.mod_id) ?? [] }));
  const [text, setText] = useState('');
  const [query, setQuery] = useState({ text: '', category: '', sort: 'updated' as NexusSort });
  const [showAdult, setShowAdult] = useState(false);
  const [categories, setCategories] = useState<NexusCategory[]>([]);
  const [mods, setMods] = useState<NexusMod[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    TauriService.nexusCategories().then(setCategories).catch(() => setCategories([]));
  }, []);

  const load = useCallback(async (p: number) => {
    setLoading(true);
    try {
      const r = await TauriService.nexusSearch(query.text, query.category || null, query.sort, (p - 1) * PAGE_SIZE, PAGE_SIZE);
      setTotal(r.total);
      setMods(r.mods);
      setPage(p);
      if (p * PAGE_SIZE < r.total) {
        TauriService.prefetch(`nexus:search:${query.sort}:${query.category}:${p * PAGE_SIZE}:${PAGE_SIZE}:${query.text}`, () =>
          TauriService.nexusSearch(query.text, query.category || null, query.sort, p * PAGE_SIZE, PAGE_SIZE)
        );
      }
    } catch (err) {
      reportBackendError('Searching Nexus Mods', err);
    } finally {
      setLoading(false);
    }
  }, [query]);

  useEffect(() => {
    load(1);
  }, [load]);

  const visible = showAdult ? mods : mods.filter((m) => !m.adult_content);
  const hiddenAdult = mods.length - visible.length;

  return (
    <div className="flex flex-col gap-3">
      <Toolbar onSubmit={() => setQuery((q) => ({ ...q, text: text.trim() }))}>
        <div className="flex items-center gap-2 flex-1 min-w-[240px] px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-800 focus-within:border-orange-700">
          <Search className="w-3.5 h-3.5 text-slate-500" aria-hidden="true" />
          <input value={text} onChange={(e) => setText(e.target.value)} placeholder={i18next.t('browse.nexus.searchPlaceholder')} aria-label={i18next.t('browse.nexus.searchLabel')}
            className="flex-1 bg-transparent text-xs text-slate-100 outline-none placeholder:text-slate-600" />
          {text && (
            <button type="button" onClick={() => { setText(''); setQuery((q) => ({ ...q, text: '' })); }} aria-label={i18next.t('browse.workshop.clear')} className="text-slate-500 hover:text-slate-300 cursor-pointer">
              <X className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
        <select value={query.sort} onChange={(e) => setQuery((q) => ({ ...q, sort: e.target.value as NexusSort }))} aria-label={i18next.t('browse.workshop.sort')} className={selectClass}>
          {NEXUS_SORTS.map((id) => <option key={id} value={id}>{i18next.t(`browse.nexusSorts.${id}`)}</option>)}
        </select>
        <button type="submit" className="px-3 py-1.5 rounded-lg text-xs font-bold bg-orange-700 hover:bg-orange-600 text-white cursor-pointer">{i18next.t('browse.workshop.search')}</button>
        <label className="flex items-center gap-1.5 text-[11px] text-slate-400 cursor-pointer select-none">
          <input type="checkbox" checked={showAdult} onChange={(e) => setShowAdult(e.target.checked)} className="w-3.5 h-3.5" />
          {i18next.t('browse.nexus.adult')}
        </label>
        <button type="button" onClick={() => TauriService.openExternalUrl('https://www.nexusmods.com/games/projectzomboid/mods')} title={i18next.t('browse.nexus.webTitle')}
          className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs text-slate-300 bg-slate-900 border border-slate-800 hover:bg-slate-800 cursor-pointer">
          <Globe className="w-3.5 h-3.5" aria-hidden="true" /> {i18next.t('browse.workshop.web')}
        </button>
      </Toolbar>
      {categories.length > 0 && (
        <CategoryBar categories={categories} value={query.category} onChange={(c) => setQuery((q) => ({ ...q, category: c }))} accent="border-orange-600 bg-orange-900/50 text-orange-200" />
      )}
      <div className="flex items-center justify-between text-[11px] font-mono text-slate-500">
        <span>
          {loading && mods.length === 0 ? i18next.t('browse.nexus.loading') : i18next.t('browse.nexus.results', { total, text: query.text ? i18next.t('browse.nexus.matching', { text: query.text }) : '', category: query.category ? i18next.t('browse.workshop.inCategory', { category: query.category }) : '', adult: hiddenAdult ? i18next.t('browse.nexus.adultHidden', { count: hiddenAdult }) : '' })}
          {loading && mods.length > 0 && <Loader2 className="inline w-3 h-3 ml-2 animate-spin" aria-hidden="true" />}
        </span>
        <span>{i18next.t('browse.workshop.page', { page, pages: pageCountOf(total) })}</span>
      </div>
      <Pagination page={page} pageCount={pageCountOf(total)} onPage={load} disabled={loading} />
      <CardGrid>{visible.map((m) => nexusCard(m, installedByNexusId.has(m.mod_id), onOpen, nexusMenu))}</CardGrid>
      <Pagination page={page} pageCount={pageCountOf(total)} onPage={load} disabled={loading} />
    </div>
  );
};

// ---------------------------------------------------------------------------------------------
// My mods
// ---------------------------------------------------------------------------------------------

type MineRow =
  | { key: string; kind: 'workshop'; item: WorkshopItem; modIds: string[]; state?: WorkshopInstallState }
  | { key: string; kind: 'local'; mod: CatalogModSummary };

type MineSource = 'all' | 'workshop' | 'nexus' | 'local';
type MineStatus = 'all' | 'new' | 'pending';

const localLabel = (m: CatalogModSummary) =>
  m.origin?.kind === 'nexus' ? i18next.t('browse.mine.labels.nexus') : m.origin?.kind === 'compat' ? i18next.t('browse.mine.labels.compat') : m.source.kind === 'workshop_upload' ? i18next.t('browse.mine.labels.upload') : i18next.t('browse.mine.labels.local');

const MyModsTab: React.FC<{
  catalog: CatalogModSummary[];
  installedByItem: Map<string, string[]>;
  stateByItem: Map<string, WorkshopInstallState>;
  onOpen: (t: DetailTarget) => void;
  steamReady: boolean;
  onSubscribe: (ids: string[], subscribe: boolean) => Promise<void>;
  keep: KeepActions;
}> = ({ catalog, installedByItem, stateByItem, onOpen, steamReady, onSubscribe, keep }) => {
  const showContextMenu = useContextMenu();
  const [details, setDetails] = useState<WorkshopItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [text, setText] = useState('');
  const [source, setSource] = useState<MineSource>('all');
  const [status, setStatus] = useState<MineStatus>('all');
  const [category, setCategory] = useState('');
  const [sort, setSort] = useState<'updated' | 'name'>('updated');
  const [page, setPage] = useState(1);

  const subscribedKey = useMemo(() => [...installedByItem.keys()].sort().join(','), [installedByItem]);
  useEffect(() => {
    if (!subscribedKey) return;
    let cancelled = false;
    setLoading(true);
    TauriService.workshopDetails(subscribedKey.split(','))
      .then((items) => !cancelled && setDetails(items))
      .catch((err) => !cancelled && reportBackendError('Loading Workshop details', err))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [subscribedKey]);

  const rows = useMemo<MineRow[]>(() => {
    const byId = new Map(details.map((d) => [d.workshop_id, d]));
    const out: MineRow[] = [];
    for (const [id, modIds] of installedByItem) {
      const item = byId.get(id) ?? ({ workshop_id: id, found: true, title: modIds.join(', '), summary: '', preview_url: null, time_updated: null, subscriptions: 0, favorited: 0, file_size: 0, tags: [], builds: [], is_project_zomboid: true, banned: false, page_url: '', children: [], is_collection: false } as WorkshopItem);
      out.push({ key: `w${id}`, kind: 'workshop', item, modIds, state: stateByItem.get(id) });
    }
    for (const m of catalog) {
      if (m.source.kind !== 'workshop' && !m.shadowed_by && !m.id.startsWith('Z_PZModStudio_')) out.push({ key: `l${m.id}`, kind: 'local', mod: m });
    }
    return out;
  }, [details, installedByItem, stateByItem, catalog]);

  const counts = useMemo(() => ({
    workshop: rows.filter((r) => r.kind === 'workshop').length,
    nexus: rows.filter((r) => r.kind === 'local' && r.mod.origin?.kind === 'nexus').length,
    local: rows.filter((r) => r.kind === 'local' && r.mod.origin?.kind !== 'nexus').length,
    newSincePlay: rows.filter((r) => r.kind === 'workshop' && r.state?.updated_since_last_play).length,
    pending: rows.filter((r) => r.kind === 'workshop' && r.state?.update_pending).length,
  }), [rows]);

  const categories = useMemo(() => {
    const tally = new Map<string, number>();
    for (const r of rows) if (r.kind === 'workshop') for (const t of r.item.tags) if (!/^Build \d+$/.test(t)) tally.set(t, (tally.get(t) ?? 0) + 1);
    return [...tally].sort((a, b) => b[1] - a[1]).map(([name, count]) => ({ name, count }));
  }, [rows]);

  const filtered = useMemo(() => {
    const q = text.toLowerCase().trim();
    const list = rows.filter((r) => {
      if (source === 'workshop' && r.kind !== 'workshop') return false;
      if (source === 'nexus' && !(r.kind === 'local' && r.mod.origin?.kind === 'nexus')) return false;
      if (source === 'local' && !(r.kind === 'local' && r.mod.origin?.kind !== 'nexus')) return false;
      if (status === 'new' && !(r.kind === 'workshop' && r.state?.updated_since_last_play)) return false;
      if (status === 'pending' && !(r.kind === 'workshop' && r.state?.update_pending)) return false;
      if (category && !(r.kind === 'workshop' && r.item.tags.includes(category))) return false;
      if (!q) return true;
      const hay = r.kind === 'workshop' ? `${r.item.title} ${r.modIds.join(' ')} ${r.item.workshop_id}` : `${r.mod.name} ${r.mod.id}`;
      return hay.toLowerCase().includes(q);
    });
    const name = (r: MineRow) => (r.kind === 'workshop' ? r.item.title : r.mod.name).toLowerCase();
    const time = (r: MineRow) => (r.kind === 'workshop' ? r.item.time_updated ?? 0 : 0);
    return list.sort((a, b) => (sort === 'name' ? name(a).localeCompare(name(b)) : time(b) - time(a) || name(a).localeCompare(name(b))));
  }, [rows, text, source, status, category, sort]);

  const mineMenu = (e: React.MouseEvent, item: WorkshopItem) =>
    showContextMenu(e, [
      {
        label: i18next.t('browse.workshop.unsubscribe'),
        icon: Trash2,
        danger: true,
        onSelect: () => onSubscribe([item.workshop_id], false),
        disabled: !steamReady,
        disabledReason: i18next.t('browse.workshop.steamNotRunning'),
      },
      { label: i18next.t('browse.workshop.openPage'), icon: ExternalLink, separatorBefore: true,
        onSelect: () => TauriService.openExternalUrl(item.page_url) },
      { label: i18next.t('browse.workshop.copyId'), icon: Copy, onSelect: () => navigator.clipboard.writeText(item.workshop_id) },
      ...keep.menuItems({ workshop_id: item.workshop_id, name: item.title, installedModIds: installedByItem.get(item.workshop_id) ?? [] }),
    ]);

  useEffect(() => {
    setPage(1);
  }, [text, source, status, category, sort]);
  const pageRows = filtered.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  const statusChip = (value: MineStatus, label: string, n: number, cls: string) =>
    n > 0 && (
      <button type="button" onClick={() => setStatus(status === value ? 'all' : value)} aria-pressed={status === value}
        className={`px-2 py-0.5 rounded-full text-[11px] border cursor-pointer ${status === value ? cls : 'border-slate-800 bg-slate-900 text-slate-400 hover:text-slate-200'}`}>
        {label} <span className="font-mono">{n}</span>
      </button>
    );

  return (
    <div className="flex flex-col gap-3">
      <Toolbar onSubmit={() => undefined}>
        <div className="flex items-center gap-2 flex-1 min-w-[240px] px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-800 focus-within:border-emerald-700">
          <Search className="w-3.5 h-3.5 text-slate-500" aria-hidden="true" />
          <input value={text} onChange={(e) => setText(e.target.value)} placeholder={i18next.t('browse.mine.filterPlaceholder')} aria-label={i18next.t('browse.mine.filterLabel')}
            className="flex-1 bg-transparent text-xs text-slate-100 outline-none placeholder:text-slate-600" />
        </div>
        <select value={source} onChange={(e) => setSource(e.target.value as MineSource)} aria-label={i18next.t('browse.mine.source')} className={selectClass}>
          <option value="all">{i18next.t('browse.mine.allSources', { count: rows.length })}</option>
          <option value="workshop">{i18next.t('browse.mine.workshop', { count: counts.workshop })}</option>
          <option value="nexus">{i18next.t('browse.mine.nexus', { count: counts.nexus })}</option>
          <option value="local">{i18next.t('browse.mine.local', { count: counts.local })}</option>
        </select>
        <select value={sort} onChange={(e) => setSort(e.target.value as 'updated' | 'name')} aria-label={i18next.t('browse.workshop.sort')} className={selectClass}>
          <option value="updated">{i18next.t('browse.mine.recentlyUpdated')}</option>
          <option value="name">{i18next.t('browse.mine.name')}</option>
        </select>
        {loading && <Loader2 className="w-3.5 h-3.5 animate-spin text-slate-500" aria-label={i18next.t('browse.detail.loading')} />}
      </Toolbar>
      {(counts.newSincePlay > 0 || counts.pending > 0) && (
        <div className="flex flex-wrap items-center gap-1.5">
          {statusChip('new', i18next.t('browse.mine.newSincePlay'), counts.newSincePlay, 'border-sky-600 bg-sky-900/50 text-sky-200')}
          {statusChip('pending', i18next.t('browse.mine.updateWaiting'), counts.pending, 'border-amber-600 bg-amber-900/50 text-amber-200')}
        </div>
      )}
      {categories.length > 0 && <CategoryBar categories={categories} value={category} onChange={setCategory} accent="border-emerald-600 bg-emerald-900/50 text-emerald-200" />}
      <div className="flex items-center justify-between text-[11px] font-mono text-slate-500">
        <span>{i18next.t('browse.mine.count', { shown: filtered.length, total: rows.length })}</span>
        <span>{i18next.t('browse.workshop.page', { page, pages: pageCountOf(filtered.length) })}</span>
      </div>
      {rows.length === 0 && !loading && <p className="text-xs text-slate-500">{i18next.t('browse.mine.none')}</p>}
      <Pagination page={page} pageCount={pageCountOf(filtered.length)} onPage={setPage} />
      <CardGrid>
        {pageRows.map((r) => {
          // Every card here is already installed, so the Installed chip would say nothing.
          if (r.kind === 'workshop') return workshopCard(r.item, [], r.state, onOpen, mineMenu);
          const nexusId = r.mod.origin?.kind === 'nexus' ? r.mod.origin.mod_id : null;
          return (
            <ModCard
              key={r.key}
              image={null}
              title={r.mod.name}
              subtitle={<span className="font-mono">{r.mod.id}</span>}
              chips={<Chip className={nexusId !== null ? 'border-orange-700/60 text-orange-300' : 'border-slate-600 text-slate-300'}>{localLabel(r.mod)}</Chip>}
              summary={r.mod.loadability.status !== 'loadable' ? i18next.t('browse.mine.notLoadable') : null}
              onOpen={nexusId !== null ? () => onOpen({ kind: 'nexus', modId: nexusId }) : undefined}
              onContextMenu={(e) => showContextMenu(e, keep.menuItems({ mod_id: r.mod.id, nexus_id: nexusId, name: r.mod.name, installedModIds: [r.mod.id] }).map((i, n) => ({ ...i, separatorBefore: n === 0 ? false : i.separatorBefore })))}
            />
          );
        })}
      </CardGrid>
      <Pagination page={page} pageCount={pageCountOf(filtered.length)} onPage={setPage} />
    </div>
  );
};

// ---------------------------------------------------------------------------------------------

const TABS: { id: SourceTab; active: string }[] = [
  { id: 'workshop', active: 'bg-sky-950/80 text-sky-200 border-sky-700' },
  { id: 'nexus', active: 'bg-orange-950/80 text-orange-200 border-orange-700' },
  { id: 'mine', active: 'bg-emerald-950/80 text-emerald-200 border-emerald-700' },
  { id: 'lists', active: 'bg-amber-950/80 text-amber-200 border-amber-700' },
];

export const BrowseModule: React.FC<BrowseModuleProps> = ({ paths, catalog, onInstalled, onSubscribed, focusWorkshopId, onFocusHandled, focusSearch, onSearchHandled, onMakeProfile, installedBuild = null, profiles = [], onProfilesChanged }) => {
  const { t } = useTranslation();
  const keep = useKeepActions(paths, profiles, onProfilesChanged);
  const [tab, setTab] = useState<SourceTab>('workshop');
  // Tabs stay mounted once visited, so switching keeps their page and filters.
  const [visited, setVisited] = useState<Set<SourceTab>>(() => new Set(['workshop']));
  const [pendingOrigin, setPendingOrigin] = useState<(ModOrigin & { kind: 'nexus' }) | null>(null);
  const [detailStack, setDetailStack] = useState<DetailTarget[]>([]);
  const installer = useInstaller(paths, pendingOrigin, () => {
    setPendingOrigin(null);
    onInstalled();
  });

  const installedByItem = useMemo(() => {
    const map = new Map<string, string[]>();
    for (const m of catalog) {
      // A copy the game ignores (another item ships the same mod id) is still a subscribed, installed item.
      if (m.source.kind === 'workshop') map.set(m.source.workshop_id, [...(map.get(m.source.workshop_id) ?? []), m.id]);
    }
    return map;
  }, [catalog]);
  const stateByItem = useMemo(() => {
    const map = new Map<string, WorkshopInstallState>();
    for (const m of catalog) if (m.source.kind === 'workshop' && m.workshop_state) map.set(m.source.workshop_id, m.workshop_state);
    return map;
  }, [catalog]);
  const installedByNexusId = useMemo(() => {
    const map = new Map<number, string[]>();
    for (const m of catalog) if (m.origin?.kind === 'nexus' && !m.shadowed_by) map.set(m.origin.mod_id, [...(map.get(m.origin.mod_id) ?? []), m.id]);
    return map;
  }, [catalog]);
  const myCount = useMemo(() => installedByItem.size + catalog.filter((m) => m.source.kind !== 'workshop' && !m.shadowed_by && !m.id.startsWith('Z_PZModStudio_')).length, [catalog, installedByItem]);

  const openTab = (t: SourceTab) => {
    setTab(t);
    setVisited((v) => new Set(v).add(t));
    scrollBrowseToTop();
  };
  const openDetail = useCallback((t: DetailTarget) => setDetailStack((s) => [...s, t]), []);

  // Subscribing needs the Steam client, not a key: ask it once whether it is there.
  const [steamReady, setSteamReady] = useState(false);
  useEffect(() => {
    if (!paths.is_valid) return;
    let cancelled = false;
    const check = () => TauriService.steamClientReady(paths).then((ready) => !cancelled && setSteamReady(ready));
    void check();
    // Steam started after the Studio: ask again when the window comes back to the front.
    window.addEventListener('focus', check);
    return () => {
      cancelled = true;
      window.removeEventListener('focus', check);
    };
  }, [paths]);

  /**
   * What a subscription needs too, asked before it is made: the dialog resolves with the extra
   * items to subscribe to, or null when the player cancels.
   */
  const [requirements, setRequirements] = useState<{ chain: RequiredChain; resolve: (extra: string[] | null) => void; ids: string[] } | null>(null);
  const askRequirements = useCallback(
    async (ids: string[]): Promise<string[] | null> => {
      const chain = await TauriService.workshopRequirements(ids);
      if (!chain) return [];
      const missing = chain.items.filter((n) => !installedByItem.has(n.workshop_id));
      if (missing.length === 0 && chain.unread.length === 0) return [];
      return await new Promise<string[] | null>((resolve) => setRequirements({ chain, resolve, ids }));
    },
    [installedByItem]
  );

  const handleSubscribe = useCallback(
    async (ids: string[], subscribe: boolean) => {
      try {
        // What it needs comes in the same subscription, if the player says so.
        let extra: string[] = [];
        if (subscribe) {
          const answer = await askRequirements(ids);
          if (answer === null) throw new Error('cancelled');
          extra = answer.filter((id) => !ids.includes(id));
        }
        const all = [...ids, ...extra];
        const outcome = await TauriService.workshopSubscribe(paths, all, subscribe);
        if (outcome.failed.length > 0) {
          reportBackendError(
            subscribe ? 'Subscribing' : 'Unsubscribing',
            i18next.t('browse.subscribePartial', { failed: outcome.failed.length, total: all.length, reason: outcome.failed[0][1] })
          );
        }
        // Steam downloads in the background; pick up whatever has landed, and watch for the rest.
        onInstalled();
        if (subscribe && outcome.done.length > 0) onSubscribed?.(outcome.done);
        // Nothing went through: the button must not say it did.
        if (outcome.done.length === 0) throw new Error('none subscribed');
      } catch (err) {
        if (!(err instanceof Error && (err.message === 'none subscribed' || err.message === 'cancelled'))) {
          reportBackendError(subscribe ? 'Subscribing' : 'Unsubscribing', err);
        }
        throw err;
      }
    },
    [paths, onInstalled, onSubscribed, askRequirements]
  );

  // The Mod List asked for one item: show it on its own, not stacked on whatever was open.
  useEffect(() => {
    if (!focusWorkshopId) return;
    setDetailStack([{ kind: 'workshop', id: focusWorkshopId }]);
    onFocusHandled?.();
  }, [focusWorkshopId, onFocusHandled]);
  // A search from another screen lands on the Workshop tab's list, not on whatever was open.
  useEffect(() => {
    if (!focusSearch) return;
    setTab('workshop');
    setVisited((v) => new Set(v).add('workshop'));
    setDetailStack([]);
  }, [focusSearch]);
  const closeDetail = useCallback(() => setDetailStack([]), []);

  const handleNexusDownload = (m: NexusMod) => {
    setPendingOrigin({ kind: 'nexus', mod_id: m.mod_id, page_url: m.page_url, name: m.name });
    TauriService.openExternalUrl(`${m.page_url}?tab=files`);
  };

  return (
    <div id={SCROLL_ID} className="flex-1 flex flex-col min-h-0 overflow-y-auto font-sans">
      {requirements && (
        <SubscribeRequirements
          titles={requirements.chain.asked.length > 0 ? requirements.chain.asked.map((a) => a.title) : requirements.ids}
          chain={requirements.chain}
          installed={new Set(installedByItem.keys())}
          onSubscribe={(extra) => {
            requirements.resolve(extra);
            setRequirements(null);
          }}
          onRetry={async () => {
            const again = await TauriService.workshopRequirements(requirements.ids);
            if (again) setRequirements({ ...requirements, chain: again });
          }}
          onCancel={() => {
            requirements.resolve(null);
            setRequirements(null);
          }}
        />
      )}
      <div className="sticky top-0 z-10 flex flex-wrap items-center gap-3 px-6 py-3 border-b border-slate-800 bg-slate-950/95 backdrop-blur">
        <div className="flex items-center gap-2 mr-2">
          <Globe className="w-5 h-5 text-emerald-400" aria-hidden="true" />
          <h1 className="text-sm font-bold text-slate-100">{t('browse.title')}</h1>
        </div>
        <div role="tablist" aria-label={t('browse.sourceLabel')} className="flex gap-1.5">
          {TABS.map((tab_) => (
            <button key={tab_.id} type="button" role="tab" aria-selected={tab === tab_.id} onClick={() => openTab(tab_.id)}
              className={`px-3 py-1.5 rounded-lg text-xs font-bold border cursor-pointer transition ${tab === tab_.id ? tab_.active : 'bg-slate-900 text-slate-400 border-slate-800 hover:text-slate-200'}`}>
              {t(`browse.tabs.${tab_.id}`)}
              {tab_.id === 'mine' && <span className="ml-1.5 font-mono text-[10px] opacity-70">{myCount}</span>}
            </button>
          ))}
        </div>
        <div className="flex-1" />
        <div className="flex items-center gap-1.5" title={t('browse.installTitle')}>
          <button type="button" onClick={installer.pickZip} disabled={installer.busy || !paths.is_valid}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-bold bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer disabled:opacity-50">
            {installer.busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <FileArchive className="w-3.5 h-3.5" aria-hidden="true" />}
            {t('browse.installZip')}
          </button>
          <button type="button" onClick={installer.pickFolder} disabled={installer.busy || !paths.is_valid}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50"
            title={t('browse.folderTitle')}>
            <FolderOpen className="w-3.5 h-3.5" aria-hidden="true" />
            {t('browse.folder')}
          </button>
          <button
            type="button"
            onClick={async () => {
              const url = window.prompt(t('browse.linkPrompt'));
              if (url && url.trim()) await installer.fromUrl(url.trim());
            }}
            disabled={installer.busy || !paths.is_valid}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50"
            title={t('browse.linkTitle')}
          >
            <Link2 className="w-3.5 h-3.5" aria-hidden="true" />
            {t('browse.fromLink')}
          </button>
        </div>
      </div>

      <div className="flex flex-col gap-3 px-6 py-4">
        {pendingOrigin && (
          <div className="flex items-center gap-2 text-[11px] text-orange-200 bg-orange-950/30 border border-orange-800/50 rounded-lg px-3 py-1.5">
            <Link2 className="w-3.5 h-3.5 shrink-0" aria-hidden="true" />
            <span className="flex-1" dangerouslySetInnerHTML={{ __html: tHtml('browse.pendingNexus', { name: pendingOrigin.name }) }} />
            <button type="button" onClick={() => setPendingOrigin(null)} className="p-0.5 rounded hover:bg-orange-900/60 cursor-pointer" aria-label={t('browse.noTag')}>
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        )}
        {installer.result && <InstallResultBanner result={installer.result} onClose={installer.clearResult} />}

        <div hidden={tab !== 'workshop'}>
          {visited.has('workshop') && (
            <WorkshopTab
              installedByItem={installedByItem}
              stateByItem={stateByItem}
              onOpen={openDetail}
              steamReady={steamReady}
              onSubscribe={handleSubscribe}
              keep={keep}
              searchFor={focusSearch}
              onSearchHandled={onSearchHandled}
            />
          )}
        </div>
        <div hidden={tab !== 'nexus'}>
          {visited.has('nexus') && <NexusTab installedByNexusId={installedByNexusId} onOpen={openDetail} keep={keep} />}
        </div>
        <div hidden={tab !== 'mine'}>
          {visited.has('mine') && (
            <MyModsTab
              catalog={catalog}
              installedByItem={installedByItem}
              stateByItem={stateByItem}
              onOpen={openDetail}
              steamReady={steamReady}
              onSubscribe={handleSubscribe}
              keep={keep}
            />
          )}
        </div>
        <div hidden={tab !== 'lists'}>
          {visited.has('lists') && (
            <ListsTab
              paths={paths}
              catalog={catalog}
              installedByItem={installedByItem}
              stateByItem={stateByItem}
              onOpen={openDetail}
              steamReady={steamReady}
              onSubscribe={handleSubscribe}
              onMakeProfile={onMakeProfile}
            />
          )}
        </div>
      </div>
      {keep.node}

      {detailStack.length > 0 && (
        <ModDetail
          paths={paths}
          steamReady={steamReady}
          onSubscribe={handleSubscribe}
          target={detailStack[detailStack.length - 1]}
          canGoBack={detailStack.length > 1}
          onBack={() => setDetailStack((s) => s.slice(0, -1))}
          onClose={closeDetail}
          onOpen={openDetail}
          installedByItem={installedByItem}
          stateByItem={stateByItem}
          installedByNexusId={installedByNexusId}
          onNexusDownload={handleNexusDownload}
          onMakeProfile={onMakeProfile}
          installedBuild={installedBuild}
          keep={keep}
        />
      )}
    </div>
  );
};
