import React, { useEffect, useMemo, useState } from 'react';
import { ExternalLink, FolderPlus, Globe, Pencil, Plus, Star, Trash2, Users, X } from 'lucide-react';
import { CommunityLists, shareList } from './CommunityLists';
import { CatalogModSummary, ModList, WorkshopInstallState, WorkshopItem } from '../../types';
import { TauriService, reportBackendError } from '../../services/tauri';
import { FAVORITES, itemKey, useModLists } from '../../services/modLists';
import { MenuItem, useContextMenu } from '../common/ContextMenu';
import { StudioPathsUI } from '../settings/SettingsModule';
import { CardGrid, Chip, InstalledChip, ModCard, UpdateChips, formatUnixDate } from './common';
import { DetailTarget } from './ModDetail';
import i18next from 'i18next';

interface ListsTabProps {
  paths: StudioPathsUI;
  catalog: CatalogModSummary[];
  installedByItem: Map<string, string[]>;
  stateByItem: Map<string, WorkshopInstallState>;
  onOpen: (t: DetailTarget) => void;
  steamReady: boolean;
  onSubscribe: (ids: string[], subscribe: boolean) => Promise<void>;
  /** Makes a profile holding the installed mods of the list's Workshop items. */
  onMakeProfile?: (name: string, workshopIds: string[]) => void;
}

/**
 * The player's lists: Favorites and the ones they made. What a Workshop collection is, but
 * theirs, offline, and able to hold a mod from anywhere. A list can be subscribed to at once
 * and made into a profile.
 */
export const ListsTab: React.FC<ListsTabProps> = ({ paths, catalog, installedByItem, stateByItem, onOpen, steamReady, onSubscribe, onMakeProfile }) => {
  const lists = useModLists(paths.user_zomboid_dir);
  const showContextMenu = useContextMenu();
  const [selectedId, setSelectedId] = useState(FAVORITES);
  const [newName, setNewName] = useState('');
  const [subscribing, setSubscribing] = useState(false);
  const [details, setDetails] = useState<Map<string, WorkshopItem>>(new Map());
  const [loading, setLoading] = useState(false);
  const [mode, setMode] = useState<'mine' | 'community'>('mine');
  const [shareNote, setShareNote] = useState<{ ok: boolean; text: string } | null>(null);
  const [sharing, setSharing] = useState(false);

  const list: ModList | undefined = lists.lists.find((l) => l.id === selectedId) ?? lists.lists[0];
  const workshopIds = useMemo(() => (list?.items ?? []).map((i) => i.workshop_id).filter((id): id is string => !!id), [list]);
  const idsKey = workshopIds.slice().sort().join(',');

  useEffect(() => {
    if (!idsKey) return;
    let cancelled = false;
    setLoading(true);
    TauriService.workshopDetails(idsKey.split(','))
      .then((items) => !cancelled && setDetails((prev) => new Map([...prev, ...items.map((i) => [i.workshop_id, i] as const)])))
      .catch((err) => !cancelled && reportBackendError('Loading Workshop details', err))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [idsKey]);

  const modeSwitch = (
    <div className="flex gap-1 p-0.5 rounded-lg bg-slate-900 border border-slate-800 self-start">
      {(['mine', 'community'] as const).map((m) => (
        <button key={m} type="button" onClick={() => setMode(m)}
          className={`flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-bold cursor-pointer ${mode === m ? 'bg-slate-700 text-slate-100' : 'text-slate-400 hover:text-slate-200'}`}>
          {m === 'community' ? <Users className="w-3 h-3" aria-hidden="true" /> : <Star className="w-3 h-3" aria-hidden="true" />}
          {i18next.t(`browse.lists.mode.${m}`)}
        </button>
      ))}
    </div>
  );

  if (mode === 'community') {
    return (
      <div className="flex flex-col gap-3">
        {modeSwitch}
        <CommunityLists
          paths={paths}
          installedWorkshopIds={new Set(installedByItem.keys())}
          onCopied={(id) => {
            void lists.reload().then(() => {
              setSelectedId(id);
              setMode('mine');
            });
          }}
        />
      </div>
    );
  }

  if (!list) return null;
  const catalogById = new Map(catalog.map((m) => [m.id, m]));
  const missing = workshopIds.filter((id) => !installedByItem.has(id));

  const itemMenu = (e: React.MouseEvent, key: string, name: string, extra: MenuItem[] = []) =>
    showContextMenu(e, [
      ...extra,
      {
        label: i18next.t('browse.lists.takeOut', { list: list.name }),
        icon: X,
        danger: true,
        separatorBefore: extra.length > 0,
        onSelect: () => void lists.removeFrom(list.id, key).catch(() => undefined),
        title: i18next.t('browse.lists.takeOutTitle', { name }),
      },
    ]);

  const rename = async () => {
    const name = window.prompt(i18next.t('browse.lists.renamePrompt', { name: list.name }), list.name);
    if (name && name.trim() && name.trim() !== list.name) await lists.rename(list.id, name.trim()).catch(() => undefined);
  };
  const remove = async () => {
    if (!window.confirm(i18next.t('browse.lists.deleteConfirm', { name: list.name }))) return;
    await lists.remove(list.id).catch(() => undefined);
    setSelectedId(FAVORITES);
  };

  return (
    <div className="flex flex-col gap-3">
      {modeSwitch}
      <div className="flex flex-wrap items-center gap-2">
        {lists.lists.map((l) => (
          <button
            key={l.id}
            type="button"
            onClick={() => setSelectedId(l.id)}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold border cursor-pointer transition ${
              l.id === list.id ? 'bg-amber-950/60 text-amber-200 border-amber-700' : 'bg-slate-900 text-slate-400 border-slate-800 hover:text-slate-200'
            }`}
          >
            {l.id === FAVORITES && <Star className="w-3 h-3" aria-hidden="true" />}
            {l.name}
            <span className="font-mono text-[10px] opacity-70">{l.items.length}</span>
          </button>
        ))}
        <form
          className="flex gap-1.5"
          onSubmit={(e) => {
            e.preventDefault();
            if (!newName.trim()) return;
            void lists.create(newName.trim()).then((l) => {
              setNewName('');
              setSelectedId(l.id);
            }).catch(() => undefined);
          }}
        >
          <input
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder={i18next.t('browse.lists.newPlaceholder')}
            aria-label={i18next.t('browse.lists.newLabel')}
            className="w-36 px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-800 text-xs text-slate-200 focus:outline-none focus:border-emerald-500"
          />
          <button type="submit" disabled={!newName.trim()} className="px-2.5 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50" title={i18next.t('browse.lists.newTitle')}>
            <Plus className="w-3.5 h-3.5" aria-hidden="true" />
          </button>
        </form>
      </div>

      <div className="flex flex-wrap items-center gap-2 text-[11px]">
        <span className="text-slate-400">
          {i18next.t('browse.lists.count', { count: list.items.length })}
          {missing.length > 0 && <span className="text-amber-300">{i18next.t('browse.lists.notInstalled', { count: missing.length })}</span>}
        </span>
        <div className="flex-1" />
        {missing.length > 0 && (
          <button
            type="button"
            disabled={!steamReady || subscribing}
            onClick={() => {
              setSubscribing(true);
              onSubscribe(missing, true).catch(() => {}).finally(() => setSubscribing(false));
            }}
            title={steamReady ? i18next.t('browse.lists.subscribeMissingTitle', { count: missing.length }) : i18next.t('browse.lists.steamNotRunning')}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer disabled:opacity-50"
          >
            <Plus className="w-3 h-3" aria-hidden="true" /> {i18next.t('browse.lists.subscribeMissing')}
          </button>
        )}
        {onMakeProfile && workshopIds.length > 0 && (
          <button
            type="button"
            onClick={() => onMakeProfile(list.name, workshopIds)}
            title={i18next.t('browse.lists.makeProfileTitle')}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-emerald-700 hover:bg-emerald-600 text-white cursor-pointer"
          >
            <FolderPlus className="w-3 h-3" aria-hidden="true" /> {i18next.t('browse.lists.makeProfile')}
          </button>
        )}
        {list.items.length > 0 && (
          <button
            type="button"
            disabled={sharing}
            onClick={() => {
              setSharing(true);
              setShareNote(null);
              void shareList(paths, list.id, list.name).then(setShareNote).finally(() => setSharing(false));
            }}
            title={i18next.t('browse.lists.share.title')}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50"
          >
            <Globe className="w-3 h-3" aria-hidden="true" /> {i18next.t('browse.lists.share.button')}
          </button>
        )}
        <button type="button" onClick={() => void rename()} className="p-1.5 rounded-lg border border-slate-800 text-slate-400 hover:text-slate-200 cursor-pointer" title={i18next.t('browse.lists.rename')}>
          <Pencil className="w-3.5 h-3.5" aria-hidden="true" />
        </button>
        {list.id !== FAVORITES && (
          <button type="button" onClick={() => void remove()} className="p-1.5 rounded-lg border border-slate-800 text-slate-400 hover:text-red-300 hover:border-red-900 cursor-pointer" title={i18next.t('browse.lists.delete')}>
            <Trash2 className="w-3.5 h-3.5" aria-hidden="true" />
          </button>
        )}
      </div>

      {shareNote && <div className={`text-[11px] ${shareNote.ok ? 'text-emerald-300' : 'text-red-300'}`}>{shareNote.text}</div>}

      {list.items.length === 0 && (
        <p className="text-xs text-slate-500 py-6 text-center">
          {i18next.t('browse.lists.empty')}
        </p>
      )}

      <CardGrid>
        {list.items.map((entry) => {
          const key = itemKey(entry);
          if (entry.workshop_id) {
            const item = details.get(entry.workshop_id);
            const installedIds = installedByItem.get(entry.workshop_id) ?? [];
            return (
              <ModCard
                key={key}
                image={item?.preview_url ?? null}
                title={item?.title ?? entry.name}
                subtitle={item ? i18next.t('browse.lists.updated', { date: formatUnixDate(item.time_updated) }) : loading ? i18next.t('browse.lists.loading') : i18next.t('browse.lists.workshopItem', { id: entry.workshop_id })}
                chips={<>{installedIds.length > 0 ? <InstalledChip /> : <Chip className="border-amber-500/60 text-amber-300">{i18next.t('browse.lists.notInstalledChip')}</Chip>}<UpdateChips state={stateByItem.get(entry.workshop_id)} /></>}
                summary={item?.summary ?? null}
                onOpen={() => onOpen({ kind: 'workshop', id: entry.workshop_id!, preview: item })}
                onContextMenu={(e) =>
                  itemMenu(e, key, entry.name, [
                    { label: installedIds.length > 0 ? i18next.t('browse.lists.unsubscribe') : i18next.t('browse.lists.subscribe'), icon: installedIds.length > 0 ? Trash2 : Plus, onSelect: () => void onSubscribe([entry.workshop_id!], installedIds.length === 0), disabled: !steamReady, disabledReason: i18next.t('browse.lists.steamNotRunning') },
                    { label: i18next.t('browse.lists.openPage'), icon: ExternalLink, onSelect: () => TauriService.openExternalUrl(item?.page_url ?? `https://steamcommunity.com/sharedfiles/filedetails/?id=${entry.workshop_id}`) },
                  ])
                }
              />
            );
          }
          if (entry.nexus_id) {
            return (
              <ModCard
                key={key}
                image={null}
                title={entry.name}
                subtitle={<span className="font-mono">{i18next.t('browse.lists.nexusMod', { id: entry.nexus_id })}</span>}
                chips={<Chip className="border-orange-700/60 text-orange-300">{i18next.t('browse.lists.nexus')}</Chip>}
                onOpen={() => onOpen({ kind: 'nexus', modId: entry.nexus_id! })}
                onContextMenu={(e) => itemMenu(e, key, entry.name)}
              />
            );
          }
          const mod = entry.mod_id ? catalogById.get(entry.mod_id) : undefined;
          return (
            <ModCard
              key={key}
              image={null}
              title={mod?.name ?? entry.name}
              subtitle={<span className="font-mono">{entry.mod_id}</span>}
              chips={mod ? <InstalledChip /> : <Chip className="border-amber-500/60 text-amber-300">{i18next.t('browse.lists.notInstalledChip')}</Chip>}
              onContextMenu={(e) => itemMenu(e, key, entry.name)}
            />
          );
        })}
      </CardGrid>
    </div>
  );
};
