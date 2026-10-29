import React, { useEffect, useState } from 'react';
import { Copy, ExternalLink, Loader2, RefreshCw, Users } from 'lucide-react';
import i18next from 'i18next';
import { CommunityLists as CommunityListsData, SharedList } from '../../types';
import { TauriService, reportBackendError } from '../../services/tauri';
import { StudioPathsUI } from '../settings/SettingsModule';
import { COMMUNITY_REPO, COMMUNITY_SITE } from '../../utils/communityIndex';

/**
 * Other players' lists, from the community repository (fetched once a day). Copying one makes it
 * the player's own list, where Subscribe all and Make a profile already work; nothing is installed
 * by copying.
 */
export const CommunityLists: React.FC<{ paths: StudioPathsUI; installedWorkshopIds: Set<string>; onCopied: (listId: string) => void }> = ({ paths, installedWorkshopIds, onCopied }) => {
  const [data, setData] = useState<CommunityListsData | null>(null);
  const [loading, setLoading] = useState(true);
  const [copying, setCopying] = useState<string | null>(null);
  const [filter, setFilter] = useState('');

  const load = (refresh: boolean) => {
    setLoading(true);
    void TauriService.communityLists(paths, refresh).then((d) => {
      setData(d);
      setLoading(false);
    });
  };
  useEffect(() => load(false), [paths.user_zomboid_dir]); // eslint-disable-line react-hooks/exhaustive-deps

  const copy = async (list: SharedList) => {
    setCopying(list.id);
    try {
      const mine = await TauriService.copyCommunityList(paths, list.id);
      onCopied(mine.id);
    } catch (err) {
      reportBackendError('Copying a community list', err);
    } finally {
      setCopying(null);
    }
  };

  const q = filter.trim().toLowerCase();
  const shown = (data?.lists ?? []).filter((l) => !q || l.name.toLowerCase().includes(q) || l.author.toLowerCase().includes(q) || l.description.toLowerCase().includes(q) || l.items.some((i) => i.name.toLowerCase().includes(q)));

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <p className="text-[11px] text-slate-400 flex-1 min-w-[240px]">{i18next.t('browse.lists.community.intro')}</p>
        <input
          type="search"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder={i18next.t('browse.lists.community.filter')}
          aria-label={i18next.t('browse.lists.community.filter')}
          className="w-48 px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-800 text-xs text-slate-200 focus:outline-none focus:border-emerald-500"
        />
        <button type="button" onClick={() => load(true)} disabled={loading} title={i18next.t('browse.lists.community.refresh')}
          className="p-1.5 rounded-lg border border-slate-800 text-slate-400 hover:text-slate-200 cursor-pointer disabled:opacity-50">
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} aria-hidden="true" />
        </button>
        <button type="button" onClick={() => TauriService.openExternalUrl(`${COMMUNITY_SITE}#lists`)} title={i18next.t('browse.lists.community.allTitle')}
          className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-slate-800 text-[11px] font-bold text-slate-300 hover:text-slate-100 cursor-pointer">
          <ExternalLink className="w-3 h-3" aria-hidden="true" /> {i18next.t('browse.lists.community.all')}
        </button>
      </div>

      {!loading && shown.length === 0 && (
        <p className="text-xs text-slate-500 py-6 text-center">{data?.lists.length ? i18next.t('browse.lists.community.noMatch') : i18next.t('browse.lists.community.none')}</p>
      )}

      <div className="grid md:grid-cols-2 gap-3">
        {shown.map((list) => {
          const workshop = list.items.filter((i) => i.workshop_id);
          const have = workshop.filter((i) => installedWorkshopIds.has(i.workshop_id!)).length;
          return (
            <div key={list.id} className="bg-slate-900/60 border border-slate-800 rounded-xl p-3 flex flex-col gap-2">
              <div className="flex items-start gap-2">
                <Users className="w-4 h-4 text-emerald-300 shrink-0 mt-0.5" aria-hidden="true" />
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-bold text-slate-100 truncate">{list.name}</div>
                  <div className="text-[11px] text-slate-500">
                    {i18next.t('browse.lists.community.byline', { author: list.author, count: list.items.length })}
                    {list.build && ` · B${list.build}`}
                    {list.updated && ` · ${list.updated}`}
                  </div>
                </div>
              </div>
              {list.description && <p className="text-[11px] text-slate-300 whitespace-pre-line">{list.description}</p>}
              <p className="text-[10.5px] text-slate-500 line-clamp-2">{list.items.map((i) => i.name).join(' · ')}</p>
              {workshop.length > 0 && <p className="text-[10.5px] text-slate-400">{i18next.t('browse.lists.community.have', { have, count: workshop.length })}</p>}
              <div className="flex gap-2">
                <button type="button" disabled={copying !== null} onClick={() => void copy(list)}
                  className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-emerald-700 hover:bg-emerald-600 text-white cursor-pointer disabled:opacity-50">
                  {copying === list.id ? <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" /> : <Copy className="w-3 h-3" aria-hidden="true" />}
                  {i18next.t('browse.lists.community.copy')}
                </button>
                <button type="button" onClick={() => TauriService.openExternalUrl(`${COMMUNITY_REPO}/blob/main/community/lists/${list.id}.json`)}
                  className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer">
                  <ExternalLink className="w-3 h-3" aria-hidden="true" /> {i18next.t('browse.lists.community.source')}
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

/**
 * Shares one of the player's lists: with a GitHub token the Studio opens the pull request after
 * showing what goes out; without one, GitHub's new-file page with the list filled in. Returns a
 * line to show, or null when the player stopped.
 */
export async function shareList(paths: StudioPathsUI, listId: string, listName: string): Promise<{ ok: boolean; text: string } | null> {
  let saved = '';
  try {
    saved = localStorage.getItem('pzms.shareAuthor') ?? '';
  } catch {
    /* no storage */
  }
  const author = window.prompt(i18next.t('browse.lists.share.author'), saved);
  if (author === null || !author.trim()) return null;
  try {
    localStorage.setItem('pzms.shareAuthor', author.trim());
  } catch {
    /* no storage */
  }
  const description = window.prompt(i18next.t('browse.lists.share.description', { name: listName }), '');
  if (description === null) return null;
  let draft: SharedList;
  try {
    draft = await TauriService.draftSharedList(paths, listId, author, description, '42');
  } catch (err) {
    return { ok: false, text: String(err) };
  }
  const json = JSON.stringify(draft, null, 2);
  if (await TauriService.githubTokenStatus()) {
    if (!window.confirm(i18next.t('browse.lists.share.prConfirm', { name: draft.name, json: json.slice(0, 1500) }))) return null;
    try {
      const r = await TauriService.proposeSharedList(draft);
      TauriService.openExternalUrl(r.pr_url);
      return { ok: true, text: i18next.t('browse.lists.share.prOpened', { url: r.pr_url }) };
    } catch (err) {
      const text = String(err);
      return { ok: false, text: text.startsWith('taken:') ? i18next.t('browse.lists.share.taken') : text };
    }
  }
  void navigator.clipboard.writeText(json + '\n');
  TauriService.openExternalUrl(`${COMMUNITY_REPO}/new/main/community/lists?filename=${encodeURIComponent(draft.id + '.json')}&value=${encodeURIComponent(json + '\n')}`);
  return { ok: true, text: i18next.t('browse.lists.share.githubOpened') };
}
