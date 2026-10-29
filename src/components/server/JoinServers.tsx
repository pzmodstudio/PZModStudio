import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import { ChevronLeft, ChevronRight, Copy, Globe, Info, Lock, Play, RefreshCw, Search, Square, Star, Trash2, UserPlus, Users, Wifi, X } from 'lucide-react';
import { useContextMenu, MenuItem } from '../common/ContextMenu';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, explainError } from '../../services/tauri';
import { AppInstance, ModInfo, PublicServer, SavedServer, ServerRules } from '../../types';

interface JoinServersProps {
  paths: StudioPathsUI;
  mods: ModInfo[];
  profiles: AppInstance[];
  /** "42.21" when known. */
  installedBuild: string | null;
  /** Makes a profile with these mods, in this order; `announcedOf` when they are only the first
   * ones the server announces out of that many. */
  onMakeProfile: (name: string, modIds: string[], announcedOf?: number, server?: { address: string; name: string }) => Promise<void>;
}

/** What is selected: a server from the public list, or one of the player's own. */
interface Picked {
  name: string;
  address: string;
  queryAddress: string;
  listed?: PublicServer;
}

/** Rows per page of the public list. */
const PAGE = 50;
const bare = (id: string) => id.replace(/^\\+/, '').trim();
const majorMinor = (v: string) => v.split('.').slice(0, 2).join('.');

/**
 * Finding a server and joining it: Steam's public list (through the running Steam client, as the
 * game's own browser does), the player's own servers (private ones by address), what a server
 * says about itself, and a profile with the mods it really loaded the last time it was joined.
 */
export const JoinServers: React.FC<JoinServersProps> = ({ paths, mods, profiles, installedBuild, onMakeProfile }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision();
  const [source, setSource] = useState<'public' | 'mine'>('public');
  const [list, setList] = useState<PublicServer[]>([]);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [text, setText] = useState('');
  const [sameBuild, setSameBuild] = useState(true);
  const [hideFull, setHideFull] = useState(false);
  const [hideEmpty, setHideEmpty] = useState(false);
  const [hideLocked, setHideLocked] = useState(false);
  const [kind, setKind] = useState<'all' | 'modded' | 'vanilla'>('all');
  const [sort, setSort] = useState<'players' | 'ping' | 'name'>('players');
  const [saved, setSaved] = useState<SavedServer[]>([]);
  const [picked, setPicked] = useState<Picked | null>(null);
  const [rules, setRules] = useState<ServerRules | null>(null);
  const [rulesError, setRulesError] = useState<string | null>(null);
  const [asking, setAsking] = useState(false);
  const [password, setPassword] = useState('');
  const [username, setUsername] = useState('');
  const [remember, setRemember] = useState(false);
  /** A kept server password is being replaced: the field shows instead of the "kept" line. */
  const [changingPw, setChangingPw] = useState(false);
  const [joining, setJoining] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [direct, setDirect] = useState({ name: '', address: '', password: '' });
  const [page, setPage] = useState(0);
  const asked = useRef(0);
  const showMenu = useContextMenu();

  const loadSaved = async () => setSaved(await TauriService.savedServers(paths));
  useEffect(() => {
    void loadSaved();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths, changeRev]);

  useEffect(() => {
    let off: (() => void) | undefined;
    void TauriService.listenPublicServers((batch) => setList((prev) => {
      const seen = new Set(prev.map((s) => s.address));
      return [...prev, ...batch.filter((s) => !seen.has(s.address))];
    })).then((u) => { off = u; });
    return () => {
      off?.();
      void TauriService.stopServerSearch();
    };
  }, []);

  const search = async () => {
    setList([]);
    setSearchError(null);
    setSearching(true);
    try {
      await TauriService.publicServers();
    } catch (err) {
      setSearchError(explainError(err));
    } finally {
      setSearching(false);
    }
  };

  const build = installedBuild ? majorMinor(installedBuild) : null;
  const filtered = useMemo(() => {
    const q = text.trim().toLowerCase();
    const out = list.filter((s) =>
      (!q || s.name.toLowerCase().includes(q) || s.map.toLowerCase().includes(q)) &&
      (!sameBuild || !build || !s.version || majorMinor(s.version) === build) &&
      (!hideFull || s.players < s.max_players) &&
      (!hideEmpty || s.players > 0) &&
      (!hideLocked || s.password !== true) &&
      (kind === 'all' || s.tags.split(';').includes(kind)));
    out.sort((a, b) => sort === 'players' ? b.players - a.players : sort === 'ping' ? (a.ping_ms ?? 9999) - (b.ping_ms ?? 9999) : a.name.localeCompare(b.name));
    return out;
  }, [list, text, sameBuild, build, hideFull, hideEmpty, hideLocked, kind, sort]);
  // A new filter or order starts on the first page; more servers answering only add pages.
  useEffect(() => setPage(0), [text, sameBuild, hideFull, hideEmpty, hideLocked, kind, sort]);
  const pages = Math.max(1, Math.ceil(filtered.length / PAGE));
  const shownPage = Math.min(page, pages - 1);

  const pick = async (p: Picked) => {
    setPicked(p);
    setRules(null);
    setRulesError(null);
    setNotice(null);
    setPassword('');
    setUsername(savedOf(p.address)?.username ?? '');
    setRemember(false);
    setChangingPw(false);
    const ticket = ++asked.current;
    setAsking(true);
    try {
      const r = await TauriService.serverRules(p.queryAddress);
      if (ticket === asked.current) setRules(r);
    } catch (err) {
      if (ticket === asked.current) setRulesError(explainError(err));
    } finally {
      if (ticket === asked.current) setAsking(false);
    }
  };

  const savedOf = (address: string) => saved.find((s) => s.address.toLowerCase() === address.toLowerCase());
  const keep = async (p: Picked, starred: boolean, pw: string | null) => {
    const old = savedOf(p.address);
    const server: SavedServer = {
      name: old?.name || p.name,
      address: p.address,
      note: old?.note ?? '',
      profile_id: old?.profile_id ?? null,
      has_password: old?.has_password ?? false,
      starred,
      last_joined: old?.last_joined ?? null,
      mods: old?.mods ?? [],
      items: old?.items ?? [],
      mods_seen: old?.mods_seen ?? null,
      username: old?.username ?? '',
      made_profile_id: old?.made_profile_id ?? null,
    };
    try {
      setSaved(await TauriService.saveServer(paths, server, pw));
    } catch (err) {
      setNotice(explainError(err));
    }
  };

  const joinNow = async (target: Picked) => {
    setJoining(true);
    setNotice(null);
    const typed = picked?.address === target.address ? password : '';
    const account = picked?.address === target.address ? username.trim() : '';
    try {
      if (remember && typed) await keep(target, savedOf(target.address)?.starred ?? true, typed);
      await TauriService.joinServer(paths, target.address, target.name, typed || null, account || null);
      setNotice(t('join.started', { name: target.name }));
      void loadSaved();
    } catch (err) {
      setNotice(explainError(err));
    } finally {
      setJoining(false);
    }
  };
  const join = () => (picked ? joinNow(picked) : Promise.resolve());

  const makeProfileFor = async (p: Picked) => {
    const kept = savedOf(p.address);
    if (kept && kept.mods.length > 0) {
      await onMakeProfile(t('join.profileName', { name: p.name }), kept.mods.map(bare), undefined, p);
      return;
    }
    let r = picked?.address === p.address ? rules : null;
    if (!r) {
      try {
        r = await TauriService.serverRules(p.queryAddress);
      } catch (err) {
        setNotice(t('join.noAnswer', { error: explainError(err) }));
        return;
      }
    }
    if (r.mods.length === 0) {
      setNotice(t('join.noModsToProfile'));
      return;
    }
    const total = r.mod_count ?? r.mods.length;
    await onMakeProfile(t('join.profileName', { name: p.name }), r.mods.map(bare), total > r.mods.length ? total : undefined, p);
  };

  const copyAddress = (address: string) => {
    void navigator.clipboard?.writeText(address);
    setNotice(t('join.copied', { address }));
  };

  /** Right-click on a server, in either list. */
  const menuFor = (p: Picked): MenuItem[] => {
    const kept = savedOf(p.address);
    const items: MenuItem[] = [
      { label: t('join.menuDetails'), icon: Info, onSelect: () => void pick(p) },
      { label: t('join.join'), icon: Play, onSelect: () => { void pick(p); void joinNow(p); } },
      { label: kept?.starred ? t('join.unstar') : t('join.star'), icon: Star, onSelect: () => void keep(p, !(kept?.starred ?? false), null), separatorBefore: true },
      { label: t('join.menuCopy'), icon: Copy, onSelect: () => copyAddress(p.address) },
      { label: kept && kept.mods.length > 0 ? t('join.makeProfile', { count: kept.mods.length }) : t('join.menuProfileAnnounced'), icon: UserPlus, onSelect: () => void makeProfileFor(p), separatorBefore: true },
    ];
    if (kept) items.push({ label: t('join.forget'), icon: Trash2, danger: true, separatorBefore: true, onSelect: () => void TauriService.forgetServer(paths, p.address).then(setSaved) });
    return items;
  };

  /** The profile played on the server when joining from here: the server's mods go into it. */
  const activeProfile = profiles.find((p) => p.is_active);
  /**
   * The profile made from the picked server, while it still exists. One made before servers
   * kept it is the profile the server was joined with, when that one was made from a server.
   */
  const madeProfile = (s: SavedServer | undefined) => {
    if (!s) return undefined;
    if (s.made_profile_id) return profiles.find((p) => p.id === s.made_profile_id);
    const joinedWith = profiles.find((p) => p.id === s.profile_id);
    return joinedWith && joinedWith.description === t('join.profileDescription') ? joinedWith : undefined;
  };
  useEffect(() => {
    const kept = picked ? savedOf(picked.address) : undefined;
    const found = madeProfile(kept);
    if (kept && found && !kept.made_profile_id) void TauriService.setServerMadeProfile(paths, kept.address, kept.name, found.id).then((list) => list.length > 0 && setSaved(list));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [picked?.address, saved, profiles]);

  const byId = useMemo(() => new Map(mods.map((m) => [bare(m.mod_id), m])), [mods]);
  const pickedSaved = picked ? savedOf(picked.address) : undefined;
  const fullMods = pickedSaved && pickedSaved.mods.length > 0 ? pickedSaved.mods : null;
  const shownMods = fullMods ?? rules?.mods ?? [];
  const missing = shownMods.filter((id) => !byId.has(bare(id))).length;
  const buildDiffers = !!(build && rules?.version && majorMinor(rules.version) !== build);
  /** Page buttons and the count, drawn above and below the public list. */
  const pager = (edge: string) => (
    <div className={`flex flex-wrap items-center gap-1 px-3 py-1.5 bg-slate-900/60 text-[11px] text-slate-400 border-slate-800 ${edge}`}>
      <button onClick={() => setPage(Math.max(0, shownPage - 1))} disabled={shownPage === 0} aria-label={t('join.prevPage')} className="p-1 rounded hover:bg-slate-800 disabled:opacity-30 cursor-pointer"><ChevronLeft className="w-3.5 h-3.5" /></button>
      {Array.from({ length: pages }, (_, i) => i)
        .filter((i) => i === 0 || i === pages - 1 || Math.abs(i - shownPage) <= 2)
        .map((i, n, all) => (
          <React.Fragment key={i}>
            {n > 0 && i - all[n - 1] > 1 && <span className="px-1">…</span>}
            <button onClick={() => setPage(i)} className={`min-w-[26px] px-1.5 py-0.5 rounded cursor-pointer ${i === shownPage ? 'bg-emerald-700/50 text-emerald-100' : 'hover:bg-slate-800'}`}>{i + 1}</button>
          </React.Fragment>
        ))}
      <button onClick={() => setPage(Math.min(pages - 1, shownPage + 1))} disabled={shownPage >= pages - 1} aria-label={t('join.nextPage')} className="p-1 rounded hover:bg-slate-800 disabled:opacity-30 cursor-pointer"><ChevronRight className="w-3.5 h-3.5" /></button>
      <span className="ml-2">{t('join.pageOf', { from: shownPage * PAGE + 1, to: Math.min(filtered.length, shownPage * PAGE + PAGE), count: filtered.length })}</span>
    </div>
  );
  const starred = saved.filter((s) => s.starred);
  const recent = saved.filter((s) => !s.starred);

  const row = (s: SavedServer) => (
    <button key={s.address} onClick={() => void pick({ name: s.name, address: s.address, queryAddress: s.address })} onContextMenu={(e) => showMenu(e, menuFor({ name: s.name, address: s.address, queryAddress: s.address }))} className={`w-full text-left rounded-md border px-3 py-2 cursor-pointer ${picked?.address === s.address ? 'border-emerald-600 bg-emerald-950/30' : 'border-slate-800 bg-slate-900/60 hover:bg-slate-900'}`}>
      <div className="flex items-center gap-2 text-xs text-slate-100">
        {s.starred && <Star className="w-3 h-3 text-amber-300 fill-amber-300" />}
        <span className="truncate">{s.name}</span>
        {s.has_password && <Lock className="w-3 h-3 text-slate-400" />}
      </div>
      <div className="text-[10px] text-slate-500 font-mono">{s.address}{s.last_joined ? ` · ${t('join.lastJoined', { date: new Date(s.last_joined * 1000).toLocaleString() })}` : ''}{s.mods.length > 0 ? ` · ${t('join.modsKnown', { count: s.mods.length })}` : ''}</div>
    </button>
  );

  return (
    <div className="flex flex-col lg:flex-row gap-4 items-start">
      <div className="flex-[3] min-w-0 w-full space-y-3">
        <div className="flex items-center gap-1 p-1 rounded-lg bg-slate-900 border border-slate-800 w-fit">
          {(['public', 'mine'] as const).map((k) => (
            <button key={k} onClick={() => setSource(k)} className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs cursor-pointer ${source === k ? 'bg-emerald-700/40 text-emerald-100' : 'text-slate-400 hover:text-slate-200'}`}>
              {k === 'public' ? <Globe className="w-3.5 h-3.5" /> : <Star className="w-3.5 h-3.5" />}
              {t(`join.source_${k}`)}
              {k === 'mine' && saved.length > 0 && <span className="text-[10px] text-slate-500">{saved.length}</span>}
            </button>
          ))}
        </div>

        {source === 'public' && (
          <>
            <div className="flex flex-wrap items-center gap-2">
              {searching ? (
                <button onClick={() => void TauriService.stopServerSearch()} className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-slate-800 text-xs text-slate-200 hover:bg-slate-700 cursor-pointer">
                  <Square className="w-3.5 h-3.5" />{t('join.stop')}
                </button>
              ) : (
                <button onClick={() => void search()} className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-emerald-700 text-xs text-white hover:bg-emerald-600 cursor-pointer">
                  <Search className="w-3.5 h-3.5" />{list.length > 0 ? t('join.searchAgain') : t('join.search')}
                </button>
              )}
              <span className="text-[11px] text-slate-400 flex items-center gap-1">
                {searching && <RefreshCw className="w-3 h-3 animate-spin" />}
                {list.length > 0 && t('join.found', { count: list.length })}
                {searching && ` ${t('join.stillAnswering')}`}
              </span>
            </div>
            {searchError && <div className="rounded-md border border-amber-700/60 bg-amber-950/30 px-3 py-2 text-xs text-amber-200">{searchError}</div>}
            {list.length === 0 && !searching && !searchError && <p className="text-[11px] text-slate-500 max-w-2xl">{t('join.searchBlurb')}</p>}
            <div className="flex flex-wrap items-center gap-3 text-[11px] text-slate-300">
              <div className="relative">
                <input value={text} onChange={(e) => setText(e.target.value)} onKeyDown={(e) => { if (e.key === 'Escape') setText(''); }} placeholder={t('join.filterPlaceholder')} className="pl-2 pr-7 py-1 rounded-md bg-slate-950 border border-slate-700 text-xs text-slate-100 w-56" />
                {text && (
                  <button onClick={() => setText('')} title={t('join.clearSearch')} aria-label={t('join.clearSearch')} className="absolute right-1 top-1/2 -translate-y-1/2 p-0.5 rounded text-slate-400 hover:text-slate-100 hover:bg-slate-800 cursor-pointer"><X className="w-3.5 h-3.5" /></button>
                )}
            </div>
            {build && <label className="flex items-center gap-1 cursor-pointer"><input type="checkbox" checked={sameBuild} onChange={(e) => setSameBuild(e.target.checked)} className="accent-emerald-500" />{t('join.sameBuild', { build })}</label>}
            <label className="flex items-center gap-1 cursor-pointer"><input type="checkbox" checked={hideEmpty} onChange={(e) => setHideEmpty(e.target.checked)} className="accent-emerald-500" />{t('join.hideEmpty')}</label>
            <label className="flex items-center gap-1 cursor-pointer"><input type="checkbox" checked={hideFull} onChange={(e) => setHideFull(e.target.checked)} className="accent-emerald-500" />{t('join.hideFull')}</label>
            <label className="flex items-center gap-1 cursor-pointer"><input type="checkbox" checked={hideLocked} onChange={(e) => setHideLocked(e.target.checked)} className="accent-emerald-500" />{t('join.hideLocked')}</label>
            <select value={kind} onChange={(e) => setKind(e.target.value as typeof kind)} className="px-1.5 py-1 rounded-md bg-slate-950 border border-slate-700 text-xs">
              <option value="all">{t('join.kind_all')}</option>
              <option value="modded">{t('join.kind_modded')}</option>
              <option value="vanilla">{t('join.kind_vanilla')}</option>
            </select>
            <select value={sort} onChange={(e) => setSort(e.target.value as typeof sort)} className="px-1.5 py-1 rounded-md bg-slate-950 border border-slate-700 text-xs">
              <option value="players">{t('join.sort_players')}</option>
              <option value="ping">{t('join.sort_ping')}</option>
              <option value="name">{t('join.sort_name')}</option>
            </select>
            </div>
            {list.length > 0 && (
              <>
                <div className="rounded-lg border border-slate-800 overflow-hidden">
                  {pages > 1 && pager('border-b')}
                  <table className="w-full text-xs">
                    <thead className="bg-slate-900 text-slate-400 text-[10px] uppercase tracking-wide">
                      <tr><th className="text-left px-3 py-1.5">{t('join.col_name')}</th><th className="px-2">{t('join.col_players')}</th><th className="px-2">{t('join.col_ping')}</th><th className="px-2">{t('join.col_build')}</th></tr>
                    </thead>
                    <tbody>
                      {filtered.slice(shownPage * PAGE, shownPage * PAGE + PAGE).map((s) => (
                        <tr key={s.address} onClick={() => void pick({ name: s.name, address: s.address, queryAddress: s.query_address, listed: s })} onContextMenu={(e) => showMenu(e, menuFor({ name: s.name, address: s.address, queryAddress: s.query_address, listed: s }))} className={`cursor-pointer border-t border-slate-800/70 ${picked?.address === s.address ? 'bg-emerald-950/40' : 'hover:bg-slate-900/80'}`}>
                          <td className="px-3 py-1.5 max-w-0 w-full">
                            <div className="flex items-center gap-1.5 min-w-0">
                              {s.password && <Lock className="w-3 h-3 shrink-0 text-slate-400" />}
                              <span className="truncate text-slate-100">{s.name || s.address}</span>
                              {s.tags.split(';').includes('modded') && <span className="shrink-0 rounded px-1 text-[9px] bg-violet-900/60 text-violet-200">{t('join.modded')}</span>}
                              {s.tags.split(';').includes('pvp') && <span className="shrink-0 rounded px-1 text-[9px] bg-rose-900/50 text-rose-200">PvP</span>}
                            </div>
                          </td>
                          <td className="px-2 text-center text-slate-300 whitespace-nowrap">{s.players}/{s.max_players}</td>
                          <td className="px-2 text-center text-slate-400">{s.ping_ms ?? '—'}</td>
                          <td className="px-2 text-center text-slate-400 font-mono">{s.version || '—'}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  {pages > 1 && pager('border-t')}
                  {filtered.length === 0 && <div className="px-3 py-4 text-center text-[11px] text-slate-500">{t('join.noneMatch')}</div>}
                </div>
              </>
            )}
          </>
        )}

        {source === 'mine' && (
          <div className="space-y-3">
            <form onSubmit={(e) => { e.preventDefault(); if (!direct.address.trim()) return; const p = { name: direct.name.trim() || direct.address.trim(), address: direct.address.trim(), queryAddress: direct.address.trim() }; void keep(p, true, direct.password || null).then(() => { setDirect({ name: '', address: '', password: '' }); void pick(p); }); }} className="rounded-lg border border-slate-800 bg-slate-900/60 p-3 space-y-2">
              <div className="text-xs font-semibold text-slate-200 flex items-center gap-1.5"><UserPlus className="w-3.5 h-3.5" />{t('join.addTitle')}</div>
              <div className="flex flex-wrap gap-2">
                <input value={direct.address} onChange={(e) => setDirect({ ...direct, address: e.target.value })} placeholder={t('join.addressPlaceholder')} className="px-2 py-1 rounded-md bg-slate-950 border border-slate-700 text-xs text-slate-100 w-52 font-mono" />
                <input value={direct.name} onChange={(e) => setDirect({ ...direct, name: e.target.value })} placeholder={t('join.namePlaceholder')} className="px-2 py-1 rounded-md bg-slate-950 border border-slate-700 text-xs text-slate-100 w-44" />
                <input type="password" value={direct.password} onChange={(e) => setDirect({ ...direct, password: e.target.value })} placeholder={t('join.passwordPlaceholder')} autoComplete="off" className="px-2 py-1 rounded-md bg-slate-950 border border-slate-700 text-xs text-slate-100 w-44" />
                <button type="submit" className="px-3 py-1 rounded-md bg-emerald-700 text-xs text-white hover:bg-emerald-600 cursor-pointer">{t('join.add')}</button>
              </div>
              <p className="text-[10px] text-slate-500">{t('join.addBlurb')}</p>
            </form>
            {starred.length > 0 && <div className="space-y-1.5"><div className="text-[11px] font-semibold text-slate-300">{t('join.starred')}</div>{starred.map(row)}</div>}
            {recent.length > 0 && <div className="space-y-1.5"><div className="text-[11px] font-semibold text-slate-300">{t('join.recent')}</div>{recent.map(row)}</div>}
            {saved.length === 0 && <p className="text-[11px] text-slate-500">{t('join.noneSaved')}</p>}
          </div>
        )}
      </div>

      <div className="flex-[2] min-w-[300px] w-full lg:sticky lg:top-0">
        {!picked ? (
          <div className="rounded-lg border border-dashed border-slate-800 p-6 text-center text-[11px] text-slate-500">{t('join.pickOne')}</div>
        ) : (
          <div className="rounded-lg border border-slate-800 bg-slate-900/70 p-4 space-y-3">
            <div>
              <div className="flex items-start gap-2">
                <h3 className="text-sm font-bold text-slate-100 break-words flex-1">{picked.name}</h3>
                <button onClick={() => void keep(picked, !(pickedSaved?.starred ?? false), null)} title={pickedSaved?.starred ? t('join.unstar') : t('join.star')} className="p-1 rounded hover:bg-slate-800 cursor-pointer">
                  <Star className={`w-4 h-4 ${pickedSaved?.starred ? 'text-amber-300 fill-amber-300' : 'text-slate-400'}`} />
                </button>
                {pickedSaved && (
                  <button onClick={() => void TauriService.forgetServer(paths, picked.address).then(setSaved)} title={t('join.forget')} className="p-1 rounded hover:bg-slate-800 cursor-pointer"><Trash2 className="w-4 h-4 text-slate-400" /></button>
                )}
              </div>
              <div className="text-[11px] text-slate-400 font-mono">{picked.address}</div>
              <div className="flex flex-wrap gap-3 mt-1 text-[11px] text-slate-300">
                {picked.listed && <span className="flex items-center gap-1"><Users className="w-3 h-3" />{picked.listed.players}/{picked.listed.max_players}</span>}
                {rules && <span className="flex items-center gap-1"><Wifi className="w-3 h-3" />{rules.ping_ms} ms</span>}
                {rules?.version && <span className={buildDiffers ? 'text-amber-300' : ''}>{t('join.build', { build: rules.version })}</span>}
                {rules && <span>{rules.open ? t('join.open') : t('join.whitelist')}</span>}
                {rules?.pvp != null && <span>{rules.pvp ? 'PvP' : 'PvE'}</span>}
                {rules?.last_wipe ? <span>{t('join.lastWipe', { date: new Date(rules.last_wipe * 1000).toLocaleDateString() })}</span> : null}
              </div>
            </div>

            {asking && <div className="text-[11px] text-slate-500 flex items-center gap-1"><RefreshCw className="w-3 h-3 animate-spin" />{t('join.asking')}</div>}
            {rulesError && <div className="text-[11px] text-amber-300">{t('join.noAnswer', { error: rulesError })}</div>}
            {buildDiffers && <div className="rounded-md border border-amber-700/60 bg-amber-950/30 px-2.5 py-1.5 text-[11px] text-amber-200">{t('join.buildDiffers', { server: rules?.version, mine: installedBuild })}</div>}
            {rules?.description && <div className="max-h-40 overflow-y-auto whitespace-pre-wrap text-[11px] text-slate-300 bg-slate-950/60 rounded-md p-2">{rules.description}</div>}

            {(shownMods.length > 0 || (rules?.mod_count ?? 0) > 0) && (
              <div className="space-y-1.5">
                <div className="text-[11px] font-semibold text-slate-200">
                  {fullMods ? t('join.modsFull', { count: fullMods.length }) : t('join.modsPartial', { shown: shownMods.length, count: rules?.mod_count ?? shownMods.length })}
                  {missing > 0 && <span className="ml-1 font-normal text-slate-400">· {t('join.missing', { count: missing })}</span>}
                </div>
                {!fullMods && <p className="text-[10px] text-slate-500">{t('join.partialBlurb')}</p>}
                <div className="flex flex-wrap gap-1 max-h-36 overflow-y-auto">
                  {shownMods.map((id) => {
                    const m = byId.get(bare(id));
                    return <span key={id} title={m ? m.name : t('join.notInstalled')} className={`rounded px-1.5 py-0.5 text-[10px] font-mono ${!m ? 'bg-slate-800 text-slate-400' : m.enabled ? 'bg-emerald-900/50 text-emerald-200' : 'bg-sky-900/40 text-sky-200'}`}>{bare(id)}</span>;
                  })}
                </div>
                <div className="flex gap-3 text-[10px] text-slate-500">
                  <span><span className="inline-block w-2 h-2 rounded-sm bg-emerald-700 mr-1" />{t('join.legendOn')}</span>
                  <span><span className="inline-block w-2 h-2 rounded-sm bg-sky-800 mr-1" />{t('join.legendInstalled')}</span>
                  <span><span className="inline-block w-2 h-2 rounded-sm bg-slate-700 mr-1" />{t('join.legendDownload')}</span>
                </div>
              </div>
            )}

            <div className="space-y-2 pt-1 border-t border-slate-800">
              <input value={username} onChange={(e) => setUsername(e.target.value)} placeholder={t('join.usernamePlaceholder')} title={t('join.usernameTitle')} autoComplete="off" className="w-full px-2 py-1 rounded-md bg-slate-950 border border-slate-700 text-xs text-slate-100" />
              {pickedSaved?.has_password && !changingPw ? (
                <div className="flex flex-wrap items-center gap-2 rounded-md border border-emerald-800/60 bg-emerald-950/30 px-2.5 py-1.5 text-[11px] text-emerald-200">
                  <Lock className="w-3.5 h-3.5 shrink-0" />
                  <span className="flex-1 min-w-0">{t('join.passwordSaved')}</span>
                  <button type="button" onClick={() => { setChangingPw(true); setRemember(true); setPassword(''); }} className="px-2 py-0.5 rounded border border-emerald-700/70 hover:bg-emerald-900/50 cursor-pointer">
                    {t('join.passwordChange')}
                  </button>
                  <button
                    type="button"
                    onClick={() => { if (picked && window.confirm(t('join.passwordForgetConfirm', { name: picked.name }))) void keep(picked, pickedSaved.starred, ''); }}
                    className="px-2 py-0.5 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer"
                  >
                    {t('join.passwordForget')}
                  </button>
                </div>
              ) : (
                <div className="flex flex-wrap items-center gap-2">
                  <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder={changingPw ? t('join.passwordNew') : t('join.passwordPlaceholder')} autoComplete="off" className="px-2 py-1 rounded-md bg-slate-950 border border-slate-700 text-xs text-slate-100 w-48" />
                  <label className="flex items-center gap-1 text-[11px] text-slate-300 cursor-pointer"><input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} className="accent-emerald-500" />{t('join.remember')}</label>
                  {changingPw && (
                    <button type="button" onClick={() => { setChangingPw(false); setPassword(''); setRemember(false); }} className="text-[11px] text-slate-400 hover:text-slate-200 underline cursor-pointer">
                      {t('join.passwordKeepOld')}
                    </button>
                  )}
                </div>
              )}
              <button onClick={() => void join()} disabled={joining} className="w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-md bg-emerald-700 text-sm font-semibold text-white hover:bg-emerald-600 disabled:opacity-50 cursor-pointer">
                <Play className="w-4 h-4" />{t('join.join')}
              </button>
              <p className="text-[10px] text-slate-500">{t('join.joinBlurb')}</p>
              {madeProfile(pickedSaved) && (
                <p className="text-[11px] text-sky-200 bg-sky-950/40 border border-sky-800/60 rounded-md px-2.5 py-1.5">
                  {madeProfile(pickedSaved)!.is_active
                    ? t('join.madeProfileActive', { name: madeProfile(pickedSaved)!.name })
                    : t('join.madeProfileInactive', { name: madeProfile(pickedSaved)!.name })}
                </p>
              )}
              {/* Servers opens only with a profile active (the sidebar locks it otherwise). */}
              {activeProfile && (
                <p className="text-[11px] text-emerald-300/90 bg-emerald-950/30 border border-emerald-900/60 rounded-md px-2.5 py-1.5">{t('join.intoProfile', { name: activeProfile.name })}</p>
              )}
              {fullMods && (
                <button onClick={() => void onMakeProfile(t('join.profileName', { name: picked.name }), fullMods.map(bare), undefined, picked)} className="w-full flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-md border border-slate-700 text-xs text-slate-200 hover:bg-slate-800 cursor-pointer">
                  <UserPlus className="w-3.5 h-3.5" />{t('join.makeProfile', { count: fullMods.length })}
                </button>
              )}
              {!fullMods && shownMods.length > 0 && (
                <>
                  <button onClick={() => void makeProfileFor(picked)} className="w-full flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-md border border-slate-700 text-xs text-slate-200 hover:bg-slate-800 cursor-pointer">
                    <UserPlus className="w-3.5 h-3.5" />{t('join.makeProfileAnnounced', { count: shownMods.length })}
                  </button>
                  <p className="text-[10px] text-slate-500">{t('join.profileAfterJoin')}</p>
                </>
              )}
              {notice && <div className="text-[11px] text-slate-200 bg-slate-800/70 rounded-md px-2.5 py-1.5">{notice}</div>}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
