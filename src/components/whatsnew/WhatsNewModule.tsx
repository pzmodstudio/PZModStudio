import { SupportLinks } from '../common/SupportLinks';
import React, { useCallback, useEffect, useMemo, useState, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import i18next from 'i18next';
import { Archive, ExternalLink, Loader2, Newspaper, RefreshCw } from 'lucide-react';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { AuthorLink, UpdatedItem, WhatsNew } from '../../types';

interface WhatsNewModuleProps {
  paths: StudioPathsUI;
}

type Period = 'last' | 'week' | 'month';

const day = (secs: number | null | undefined) => (secs ? new Date(secs * 1000).toLocaleDateString(i18next.language) : '');
const when = (secs: number | null | undefined) => (secs ? new Date(secs * 1000).toLocaleString(i18next.language, { dateStyle: 'medium', timeStyle: 'short' }) : '');

/**
 * What changed in your Workshop mods since you last played, with each update's change notes, so
 * a save is not loaded blind into an update that breaks it. An update still waiting in Steam can
 * be headed off: the version on disk can be kept first.
 */
export const WhatsNewModule: React.FC<WhatsNewModuleProps> = ({ paths }) => {
  const { t } = useTranslation();
  // Only the mods change what is new: a note or a setting saved elsewhere must not reload this.
  const changeRev = useChangeRevision(['mods']);
  const [period, setPeriod] = useState<Period>('last');
  const [onlyActive, setOnlyActive] = useState(true);
  const [data, setData] = useState<WhatsNew | null>(null);
  const [reading, setReading] = useState(false);
  const [notice, setNotice] = useState<{ tone: 'ok' | 'error'; text: string } | null>(null);
  const [kept, setKept] = useState<Set<string>>(new Set());

  const since = useMemo(() => {
    const now = Math.floor(Date.now() / 1000);
    return period === 'week' ? now - 7 * 86400 : period === 'month' ? now - 30 * 86400 : null;
  }, [period]);

  /** The newest load wins: an older one that finishes later must not put its list back. */
  const loadSeq = useRef(0);
  const hasData = useRef(false);
  hasData.current = data !== null;
  const load = useCallback(async () => {
    if (!paths.user_zomboid_dir) return;
    const seq = ++loadSeq.current;
    // The list first (offline, no notes), but only when nothing is on screen yet: on a reload
    // what is shown stays, notes included, until the full read replaces it.
    if (!hasData.current) {
      const quick = await TauriService.whatsNew(paths, false, since);
      if (seq !== loadSeq.current) return;
      if (quick) setData(quick);
    }
    setReading(true);
    const full = await TauriService.whatsNew(paths, true, since);
    if (seq !== loadSeq.current) return;
    if (full) setData(full);
    setReading(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths.user_zomboid_dir, paths.workshop_dir, paths.pz_install_dir, since]);

  useEffect(() => {
    void load();
  }, [load, changeRev]);

  /** Workshop id -> the support links in its Workshop description (one request for all). */
  const [support, setSupport] = useState<Map<string, AuthorLink[]>>(new Map());
  const workshopIds = useMemo(() => (data?.items ?? []).map((i) => i.workshop_id).filter((id) => /^\d+$/.test(id)).sort().join(','), [data]);
  useEffect(() => {
    if (!workshopIds) return;
    let cancelled = false;
    TauriService.workshopDetails(workshopIds.split(','))
      .then((found) => !cancelled && setSupport(new Map(found.filter((i) => i.support?.length).map((i) => [i.workshop_id, i.support!]))))
      .catch(() => undefined); // offline: the changelog's links still show
    return () => {
      cancelled = true;
    };
  }, [workshopIds]);

  const items = (data?.items ?? []).filter((i) => !onlyActive || i.active);
  const unread = items.filter((i) => i.notes_error === 'later' || i.notes_error === 'busy').length;

  /** A local mod's changelog entries are read: it leaves the list until a new one comes. */
  const markRead = async (item: UpdatedItem) => {
    try {
      for (const id of item.mod_ids) await TauriService.changelogMarkRead(paths, id);
      setData((d) => (d ? { ...d, items: d.items.filter((x) => x.workshop_id !== item.workshop_id) } : d));
    } catch (err) {
      setNotice({ tone: 'error', text: String(err) });
    }
  };

  const keep = async (item: UpdatedItem) => {
    setNotice(null);
    try {
      for (const id of item.mod_ids) await TauriService.keepModVersion(paths, id);
      setKept((prev) => new Set(prev).add(item.workshop_id));
      setNotice({ tone: 'ok', text: t('whatsNew.keptDone', { name: item.title }) });
    } catch (err) {
      setNotice({ tone: 'error', text: String(err) });
    }
  };

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-y-auto p-6 space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-bold text-slate-100 flex items-center gap-2"><Newspaper className="w-5 h-5 text-emerald-400" />{t('whatsNew.title')}</h2>
          <p className="text-xs text-slate-400 mt-1 max-w-3xl">{t('whatsNew.blurb')}</p>
        </div>
        <button onClick={() => void load()} disabled={reading} className="flex items-center gap-1 px-2.5 py-1.5 rounded-md border border-slate-700 text-xs text-slate-300 hover:bg-slate-800 disabled:opacity-50 cursor-pointer" aria-label={t('whatsNew.refresh')}>
          <RefreshCw className={`w-3.5 h-3.5 ${reading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      <div className="flex flex-wrap items-center gap-3 text-xs">
        <div role="tablist" className="flex rounded-md border border-slate-700 overflow-hidden">
          {(['last', 'week', 'month'] as Period[]).map((p) => (
            <button key={p} role="tab" aria-selected={period === p} onClick={() => setPeriod(p)}
              className={`px-3 py-1.5 cursor-pointer ${period === p ? 'bg-emerald-800/60 text-emerald-50' : 'text-slate-300 hover:bg-slate-800'}`}>
              {/* With the time: a session earlier today read as "since today" when only the date showed */}
              {p === 'last' ? t('whatsNew.sinceLast', { date: when(data?.last_played) || '?' }) : t(`whatsNew.period.${p}`)}
            </button>
          ))}
        </div>
        <label className="flex items-center gap-1.5 text-slate-300 cursor-pointer">
          <input type="checkbox" checked={onlyActive} onChange={(e) => setOnlyActive(e.target.checked)} className="accent-emerald-500" />
          {t('whatsNew.onlyActive')}
        </label>
        <span className="text-slate-500">{t('whatsNew.count', { count: items.length })}</span>
        {reading && <span className="flex items-center gap-1 text-sky-300"><Loader2 className="w-3.5 h-3.5 animate-spin" />{t('whatsNew.readingNotes')}</span>}
        {!reading && unread > 0 && (
          <button onClick={() => void load()} className="px-2 py-1 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer">{t('whatsNew.readMore', { count: unread })}</button>
        )}
      </div>

      {notice && (
        <div className={`rounded-md border px-3 py-2 text-xs ${notice.tone === 'ok' ? 'border-emerald-700/60 bg-emerald-950/40 text-emerald-200' : 'border-red-700/60 bg-red-950/40 text-red-200'}`}>{notice.text}</div>
      )}

      {data && items.length === 0 && (
        <div className="p-8 text-center text-slate-500 text-xs space-y-2">
          <div>{period === 'last' && data.last_played ? t('whatsNew.emptySinceLast', { when: when(data.last_played) }) : t('whatsNew.empty')}</div>
          {period === 'last' && (
            <button onClick={() => setPeriod('week')} className="px-2 py-1 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer">{t('whatsNew.period.week')}</button>
          )}
        </div>
      )}

      <div className="space-y-3">
        {items.map((item) => {
          return (
            <div key={item.workshop_id} className="rounded-lg border bg-slate-900/70 p-4 border-slate-800">
              <div className="flex items-start gap-3">
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-semibold text-slate-100">{item.title}</div>
                  <div className="flex flex-wrap items-center gap-1.5 mt-1 text-[10px]">
                    {item.pending ? (
                      <span className="px-1.5 py-0.5 rounded border border-amber-600/60 text-amber-300">{t('whatsNew.pending')}</span>
                    ) : (
                      <span className="px-1.5 py-0.5 rounded border border-slate-700 text-slate-300">{t('whatsNew.updatedOn', { date: day(item.installed_time) })}</span>
                    )}
                    {item.local && <span className="px-1.5 py-0.5 rounded border border-sky-700/60 text-sky-300">{t('whatsNew.localMod')}</span>}
                    {item.read_in_game && <span className="px-1.5 py-0.5 rounded border border-slate-700 text-slate-400" title={t('whatsNew.readInGameTitle')}>{t('whatsNew.readInGame')}</span>}
                    {/* one chip per profile that has it on; the profile on now in green */}
                    {item.profiles.map((p) => (
                      <span key={p.name} className={`px-1.5 py-0.5 rounded border ${p.current ? 'border-emerald-700/60 text-emerald-300' : 'border-slate-700 text-slate-400'}`} title={p.current ? t('whatsNew.profileCurrentTitle') : t('whatsNew.profileTitle')}>
                        {p.name}
                      </span>
                    ))}
                    {item.profiles.length === 0 && <span className="px-1.5 py-0.5 rounded border border-slate-700 text-slate-500">{t('whatsNew.inNoProfile')}</span>}
                  </div>
                </div>
                <div className="flex flex-col items-end gap-1.5 shrink-0">
                  {item.local ? (
                    <button onClick={() => void markRead(item)}
                      className="flex items-center gap-1 px-2 py-1 rounded border border-slate-700 text-[11px] text-slate-300 hover:bg-slate-800 cursor-pointer">
                      {t('whatsNew.markRead')}
                    </button>
                  ) : (
                    <button onClick={() => TauriService.openExternalUrl(`https://steamcommunity.com/sharedfiles/filedetails/changelog/${item.workshop_id}`)}
                      className="flex items-center gap-1 px-2 py-1 rounded border border-slate-700 text-[11px] text-slate-300 hover:bg-slate-800 cursor-pointer">
                      <ExternalLink className="w-3 h-3" />{t('whatsNew.allNotes')}
                    </button>
                  )}
                  {item.pending && (
                    <button onClick={() => void keep(item)} disabled={kept.has(item.workshop_id)} title={t('whatsNew.keepTitle')}
                      className="flex items-center gap-1 px-2 py-1 rounded bg-sky-800/60 hover:bg-sky-700/70 text-[11px] text-sky-50 disabled:opacity-50 cursor-pointer">
                      <Archive className="w-3 h-3" />{kept.has(item.workshop_id) ? t('whatsNew.kept') : t('whatsNew.keep')}
                    </button>
                  )}
                </div>
              </div>
              <div className="mt-3 space-y-2">
                {item.notes.map((n) => (
                  <div key={n.at} className="border-l-2 border-slate-700 pl-3">
                    <div className="text-[10px] text-slate-500">{when(n.at)}</div>
                    <div className="text-xs text-slate-300 whitespace-pre-wrap break-words line-clamp-6">{n.text || t('whatsNew.noText')}</div>
                  </div>
                ))}
                {item.changelog.length > 0 && (
                  <div className="space-y-1.5">
                    <div className="text-[10px] uppercase tracking-wider text-slate-500 font-bold">{t('whatsNew.ownChangelog', { file: item.changelog_file ?? '' })}</div>
                    {item.changelog.map((e, k) => (
                      <div key={`${e.title}-${k}`} className="border-l-2 border-sky-800/70 pl-3">
                        {e.title && <div className="text-[10px] text-sky-300/80">{e.title}</div>}
                        <div className="text-xs text-slate-300 whitespace-pre-wrap break-words line-clamp-6">{e.text || t('whatsNew.noText')}</div>
                      </div>
                    ))}
                  </div>
                )}
                <SupportLinks links={item.support} more={support.get(item.workshop_id)} />
                {item.notes.length === 0 && !item.local && (
                  <div className="text-[11px] text-slate-500">
                    {item.notes_error === 'busy'
                      ? t('whatsNew.notesBusy')
                      : item.notes_error === 'later'
                        ? t('whatsNew.notesLater')
                        : item.notes_error
                          ? t('whatsNew.notesError', { error: item.notes_error })
                          : reading ? t('whatsNew.readingNotes') : t('whatsNew.noNotes')}
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
