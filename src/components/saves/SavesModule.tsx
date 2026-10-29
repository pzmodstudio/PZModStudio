import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import { Archive, Car, CheckCircle2, ChevronDown, ChevronRight, FolderOpen, Globe, HardDrive, Loader2, Pin, PinOff, RefreshCw, RotateCcw, ShieldAlert, Trash2, User } from 'lucide-react';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { AppInstance, SaveBackupEntry, SaveMaps, SaveMissingMod, SaveSummary, SavesOverview, SessionDiff, SaveInside } from '../../types';
import { formatBytes } from '../browse/common';
import i18next from 'i18next';

interface SavesModuleProps {
  paths: StudioPathsUI;
  /** Explore Mods on one Workshop item / on a Workshop search. */
  onOpenWorkshopItem?: (workshopId: string) => void;
  onSearchWorkshop?: (text: string) => void;
}

const when = (secs: number) => (secs ? new Date(secs * 1000).toLocaleString(i18next.language) : '');
/** A mod list entry as a plain id (`<workshop id>/<Mod>` -> `Mod`). */
const plainId = (id: string) => id.split('/').pop() ?? id;
/** A multiplayer save's folder (`<ip>_<port>_<hash>`) as the server's address. */
const serverOf = (world: string) => {
  const m = world.match(/^(.+)_(\d+)_[0-9a-f]{16,}$/i);
  return m ? `${m[1]}:${m[2]}` : world;
};
const SHOW_ALL_KEY = 'pz_saves_show_all';

/**
 * The player's saves and their backups. The Studio backs a save up when the player leaves it,
 * before it loads with changed mods and before its mod list is changed (the Link mod asks); here
 * the player sees them, makes one, keeps one, or puts one back. Every backup listed was read back
 * after it was written, and the room on the disk is always in view.
 */
export const SavesModule: React.FC<SavesModuleProps> = ({ paths, onOpenWorkshopItem, onSearchWorkshop }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision();
  const [overview, setOverview] = useState<SavesOverview | null>(null);
  const [diffs, setDiffs] = useState<Record<string, SessionDiff>>({});
  const [missing, setMissing] = useState<Record<string, SaveMissingMod[]>>({});
  const [maps, setMaps] = useState<Record<string, SaveMaps>>({});
  /** What each save holds (characters, vehicles), read when asked; 'loading' meanwhile. */
  const [inside, setInside] = useState<Record<string, SaveInside | 'loading' | { error: string }>>({});
  const [insideOpen, setInsideOpen] = useState<Record<string, boolean>>({});
  const toggleInside = async (save: SaveSummary) => {
    const open = !insideOpen[save.id];
    setInsideOpen((prev) => ({ ...prev, [save.id]: open }));
    if (!open || (inside[save.id] && inside[save.id] !== 'loading')) return;
    setInside((prev) => ({ ...prev, [save.id]: 'loading' }));
    try {
      const r = await TauriService.saveInside(paths, save.id);
      setInside((prev) => ({ ...prev, [save.id]: r }));
    } catch (err) {
      setInside((prev) => ({ ...prev, [save.id]: { error: String(err) } }));
    }
  };
  /** "Base.CarNormal" -> "CarNormal"; "" -> the unknown label. */
  const scriptName = (script: string) => (script ? script.split('.').pop() ?? script : t('saves.inside.unknownVehicle'));
  const [profiles, setProfiles] = useState<AppInstance[]>([]);
  /** Every save, not only the active profile's (remembered). */
  const [showAll, setShowAll] = useState<boolean>(() => {
    try {
      return localStorage.getItem(SHOW_ALL_KEY) === '1';
    } catch {
      return false;
    }
  });
  const toggleShowAll = () => {
    const next = !showAll;
    setShowAll(next);
    try {
      localStorage.setItem(SHOW_ALL_KEY, next ? '1' : '0');
    } catch {
      // only a convenience
    }
  };
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState<string | null>(null);
  const [working, setWorking] = useState<string | null>(null);
  const [notice, setNotice] = useState<{ tone: 'ok' | 'error'; text: string } | null>(null);

  const refresh = useCallback(async () => {
    if (!paths.user_zomboid_dir) return;
    setLoading(true);
    const [next, sessions, list, absent, unloaded] = await Promise.all([
      TauriService.savesOverview(paths),
      TauriService.saveSessionDiffs(paths),
      TauriService.listInstances(paths.user_zomboid_dir).catch(() => [] as AppInstance[]),
      TauriService.saveMissingMods(paths),
      TauriService.saveMissingMaps(paths),
    ]);
    if (next) setOverview(next);
    setDiffs(sessions);
    setMissing(absent);
    setMaps(unloaded);
    setProfiles(list);
    setLoading(false);
  }, [paths]);

  useEffect(() => {
    void refresh();
    const unlisten: Array<() => void> = [];
    void TauriService.listenSaveBackupsChanged(() => void refresh()).then((u) => unlisten.push(u));
    void TauriService.listenLinkGameState(() => void refresh()).then((u) => unlisten.push(u));
    return () => unlisten.forEach((u) => u());
  }, [refresh, changeRev]);

  /**
   * The profile a save was played with. The Studio notes the active profile each time a save is
   * played while it runs; a save played without it is matched only to a profile whose mods are
   * exactly the save's. Anything less than that names no profile.
   */
  const profileOf = (save: SaveSummary): { id: string; name: string } | null => {
    const noted = diffs[save.id];
    if (noted?.profile) {
      const name = profiles.find((p) => p.id === noted.profile)?.name ?? noted.profile_name ?? noted.profile;
      return { id: noted.profile, name };
    }
    const mods = new Set(save.mods.map(plainId));
    if (mods.size === 0) return null;
    const same = profiles.find((p) => p.active_mod_ids.length === mods.size && p.active_mod_ids.every((id) => mods.has(plainId(id))));
    return same ? { id: same.id, name: same.name } : null;
  };
  /**
   * Each profile has its own saves: the active profile's are shown, and "Show all" adds the
   * other profiles' and the saves no profile is known for. With no profile active, all are.
   */
  const activeProfile = profiles.find((p) => p.is_active) ?? null;
  const allSaves = overview?.saves ?? [];
  const shownSaves = !activeProfile || showAll ? allSaves : allSaves.filter((s) => profileOf(s)?.id === activeProfile.id);
  const hiddenCount = allSaves.length - shownSaves.length;

  const free = overview?.free_bytes ?? null;
  const biggest = useMemo(() => Math.max(0, ...(overview?.saves ?? []).map((s) => s.bytes)), [overview]);
  /** Short of room: the biggest save could not be backed up twice more. */
  const lowSpace = free !== null && biggest > 0 && free < biggest * 2 + 256 * 1024 * 1024;

  const run = async (key: string, job: () => Promise<string>) => {
    setWorking(key);
    setNotice(null);
    try {
      setNotice({ tone: 'ok', text: await job() });
    } catch (err) {
      setNotice({ tone: 'error', text: String(err) });
    }
    setWorking(null);
    void refresh();
  };

  const backUp = (save: SaveSummary) =>
    run(`backup:${save.id}`, async () => {
      const out = await TauriService.backupSave(paths, save.id);
      return out.unchanged ? t('saves.unchangedKept', { world: save.world }) : t('saves.backedUp', { world: save.world, size: formatBytes(out.entry.zip_bytes) });
    });

  const restore = (save: SaveSummary, b: SaveBackupEntry) => {
    if (!window.confirm(t('saves.restoreConfirm', { world: save.world, when: when(b.at) }))) return;
    void run(`restore:${save.id}:${b.file}`, async () => {
      await TauriService.restoreSaveBackup(paths, save.id, b.file);
      return t('saves.restored', { world: save.world, when: when(b.at) });
    });
  };

  const remove = (save: SaveSummary, b: SaveBackupEntry) => {
    if (!window.confirm(t('saves.deleteConfirm', { when: when(b.at), world: save.world }))) return;
    void run(`delete:${save.id}:${b.file}`, async () => {
      await TauriService.deleteSaveBackup(paths, save.id, b.file);
      return t('saves.deleted', { when: when(b.at) });
    });
  };

  /** The maps the save's own mods bring, put in front of its list (the game never does it). */
  const addMaps = (save: SaveSummary) => {
    const r = maps[save.id];
    if (!r) return;
    if (!window.confirm(t('saves.maps.confirm', { count: r.missing.length, world: save.world, chunks: r.empty_chunks }))) return;
    void run(`maps:${save.id}`, async () => {
      const out = await TauriService.saveAddMaps(paths, save.id);
      return t('saves.maps.added', { world: save.world, count: out.added.length, chunks: out.moved_chunks, backup: out.backup });
    });
  };

  const toggleKept = (save: SaveSummary, b: SaveBackupEntry) =>
    run(`kept:${save.id}:${b.file}`, async () => {
      await TauriService.setSaveBackupKept(paths, save.id, b.file, !b.kept);
      return b.kept ? t('saves.unkept', { when: when(b.at) }) : t('saves.kept', { when: when(b.at) });
    });

  const status = (save: SaveSummary) => {
    if (save.busy || working === `backup:${save.id}`)
      return <span className="flex items-center gap-1 text-sky-300"><Loader2 className="w-3.5 h-3.5 animate-spin" />{t('saves.status.busy')}</span>;
    if (save.backed_up) return <span className="flex items-center gap-1 text-emerald-300"><CheckCircle2 className="w-3.5 h-3.5" />{t('saves.status.backedUp')}</span>;
    if (save.backups.length > 0) return <span className="flex items-center gap-1 text-amber-300"><ShieldAlert className="w-3.5 h-3.5" />{t('saves.status.changed', { when: when(save.backups[0].at) })}</span>;
    return <span className="flex items-center gap-1 text-red-300"><ShieldAlert className="w-3.5 h-3.5" />{t('saves.status.none')}</span>;
  };

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-y-auto p-6 space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-bold text-slate-100 flex items-center gap-2"><Archive className="w-5 h-5 text-emerald-400" />{t('saves.title')}</h2>
          <p className="text-xs text-slate-400 mt-1 max-w-3xl">{t('saves.blurb', { keep: overview?.keep_automatic ?? 5 })}</p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <button onClick={() => void TauriService.openBackupsFolder(paths)} className="flex items-center gap-1 px-2.5 py-1.5 rounded-md border border-slate-700 text-xs text-slate-300 hover:bg-slate-800 cursor-pointer">
            <FolderOpen className="w-3.5 h-3.5" />{t('saves.openFolder')}
          </button>
          <button onClick={() => void refresh()} className="flex items-center gap-1 px-2.5 py-1.5 rounded-md border border-slate-700 text-xs text-slate-300 hover:bg-slate-800 cursor-pointer" aria-label={t('saves.refresh')}>
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
          </button>
        </div>
      </div>

      {/* The room on the disk, always in view. */}
      <div className={`rounded-lg border px-4 py-3 flex items-center gap-3 text-xs ${lowSpace ? 'border-amber-600/60 bg-amber-950/30 text-amber-200' : 'border-slate-800 bg-slate-900/60 text-slate-300'}`}>
        <HardDrive className="w-4 h-4 shrink-0" />
        <div className="flex-1 min-w-0">
          {free !== null
            ? t('saves.space', { free: formatBytes(free), total: formatBytes(overview?.total_bytes ?? 0), used: formatBytes(overview?.backups_bytes ?? 0) || '0 B' })
            : t('saves.spaceUnknown', { used: formatBytes(overview?.backups_bytes ?? 0) || '0 B' })}
          {lowSpace && <div className="mt-0.5 font-semibold">{t('saves.lowSpace', { size: formatBytes(biggest) })}</div>}
          <div className="text-[10px] text-slate-500 font-mono truncate mt-0.5" title={overview?.backup_dir}>{overview?.backup_dir}</div>
        </div>
      </div>

      {notice && (
        <div className={`rounded-md border px-3 py-2 text-xs ${notice.tone === 'ok' ? 'border-emerald-700/60 bg-emerald-950/40 text-emerald-200' : 'border-red-700/60 bg-red-950/40 text-red-200'}`}>{notice.text}</div>
      )}

      {activeProfile && allSaves.length > 0 && (
        <div className="flex items-center justify-between gap-3 text-xs">
          <span className="text-slate-400">
            {showAll ? t('saves.showingAll', { count: allSaves.length }) : t('saves.showingProfile', { name: activeProfile.name, count: shownSaves.length })}
          </span>
          {(showAll || hiddenCount > 0) && (
            <button onClick={toggleShowAll} className="px-2.5 py-1.5 rounded-md border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer">
              {showAll ? t('saves.onlyProfile', { name: activeProfile.name }) : t('saves.showAll', { count: hiddenCount })}
            </button>
          )}
        </div>
      )}

      {overview && allSaves.length === 0 && (
        <div className="p-8 text-center text-slate-500 text-xs">{t('saves.empty')}</div>
      )}
      {allSaves.length > 0 && shownSaves.length === 0 && activeProfile && (
        <div className="p-8 text-center text-slate-500 text-xs">{t('saves.noneForProfile', { name: activeProfile.name })}</div>
      )}

      <div className="space-y-2">
        {shownSaves.map((save) => {
          const expanded = open === save.id;
          const roomFor = free === null || free > save.bytes + 256 * 1024 * 1024;
          return (
            <div key={save.id} className="rounded-lg border border-slate-800 bg-slate-900/70">
              <div className="flex items-center gap-3 px-4 py-3">
                <button onClick={() => setOpen(expanded ? null : save.id)} className="text-slate-500 hover:text-slate-300 cursor-pointer" aria-label={t('saves.showBackups')}>
                  {expanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
                </button>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="text-sm font-semibold text-slate-100 truncate">{save.mode === 'Multiplayer' ? serverOf(save.world) : save.world}</span>
                    {save.mode === 'Multiplayer' && (
                      <span className="shrink-0 text-[10px] px-1.5 py-0.5 rounded border border-violet-700/60 text-violet-300">{t('saves.online')}</span>
                    )}
                    {save.made_on && (
                      <span className="shrink-0 text-[10px] px-1.5 py-0.5 rounded border border-slate-700 text-slate-400 font-mono" title={t('saves.madeOnTitle', { version: save.world_version })}>
                        {t('saves.madeOn', { build: save.made_on })}
                      </span>
                    )}
                    {(() => {
                      const profile = profileOf(save);
                      if (profile) {
                        return (
                          <span className="shrink-0 text-[10px] px-1.5 py-0.5 rounded border border-sky-700/60 text-sky-300" title={t('saves.profileTitle')}>
                            {t('saves.profile', { name: profile.name })}
                          </span>
                        );
                      }
                      return showAll ? (
                        <span className="shrink-0 text-[10px] px-1.5 py-0.5 rounded border border-slate-700 text-slate-400" title={t('saves.noProfileTitle')}>
                          {t('saves.noProfile')}
                        </span>
                      ) : null;
                    })()}
                  </div>
                  <div className="text-[11px] text-slate-500">
                    {t('saves.line', { mode: save.mode, size: formatBytes(save.bytes), files: save.files.toLocaleString(), mods: save.mods.length, when: when(save.modified) })}
                  </div>
                  {(() => {
                    const d = diffs[save.id];
                    if (!d || (d.added.length === 0 && d.removed.length === 0 && d.updated.length === 0 && !d.reordered)) return null;
                    const parts = [
                      d.added.length > 0 && t('saves.session.added', { count: d.added.length }),
                      d.removed.length > 0 && t('saves.session.removed', { count: d.removed.length }),
                      d.updated.length > 0 && t('saves.session.updated', { count: d.updated.length }),
                      d.reordered && t('saves.session.reordered'),
                    ].filter(Boolean).join(' · ');
                    const detail = [
                      d.removed.length > 0 && `${t('saves.session.removedList')}: ${d.removed.join(', ')}`,
                      d.added.length > 0 && `${t('saves.session.addedList')}: ${d.added.join(', ')}`,
                      d.updated.length > 0 && `${t('saves.session.updatedList')}: ${d.updated.join(', ')}`,
                    ].filter(Boolean).join('\n');
                    return (
                      <div className="text-[11px] text-amber-300 mt-0.5" title={detail}>
                        {t('saves.session.line', { when: when(d.at), what: parts })}
                      </div>
                    );
                  })()}
                  {(missing[save.id]?.length ?? 0) > 0 && (
                    <div className="mt-1 text-[11px] text-rose-300">
                      <span className="inline-flex items-center gap-1" title={t('saves.missing.title')}>
                        <ShieldAlert size={12} />
                        {t('saves.missing.line', { count: missing[save.id].length })}
                      </span>
                      <div className="flex flex-wrap gap-1 mt-1">
                        {missing[save.id].map((m) => (
                          <button
                            key={`${m.workshop_id ?? ''}/${m.id}`}
                            onClick={() => (m.workshop_id ? onOpenWorkshopItem?.(m.workshop_id) : onSearchWorkshop?.(m.id))}
                            disabled={m.workshop_id ? !onOpenWorkshopItem : !onSearchWorkshop}
                            title={m.workshop_id ? t('saves.missing.openTitle', { id: m.workshop_id }) : t('saves.missing.searchTitle')}
                            className="px-1.5 py-0.5 rounded border border-rose-800/60 text-rose-200 hover:bg-rose-900/40 disabled:cursor-default cursor-pointer"
                          >
                            {m.id}
                          </button>
                        ))}
                      </div>
                    </div>
                  )}
                  {(maps[save.id]?.missing.length ?? 0) > 0 && (() => {
                    const r = maps[save.id];
                    const over = r.missing.reduce((n, m) => n + m.over_listed, 0);
                    return (
                      <div className="mt-1 text-[11px] text-amber-300">
                        <span className="inline-flex items-center gap-1" title={t('saves.maps.title')}>
                          <Globe size={12} />
                          {t('saves.maps.line', { count: r.missing.length })}
                        </span>
                        <div className="flex flex-wrap items-center gap-1 mt-1">
                          {r.missing.map((m) => (
                            <span key={m.folder} title={t('saves.maps.chipTitle', { folder: m.folder, mod: m.mod_name, cells: m.cells, chunks: m.empty_chunks })} className="px-1.5 py-0.5 rounded border border-amber-800/60 text-amber-200">
                              {t('saves.maps.chip', { folder: m.folder, mod: m.mod_name })}
                            </span>
                          ))}
                          <button
                            onClick={() => addMaps(save)}
                            disabled={working !== null || save.busy}
                            title={t('saves.maps.addTitle')}
                            className="px-2 py-0.5 rounded-md bg-amber-700/60 hover:bg-amber-600/70 text-amber-50 font-medium disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
                          >
                            {t('saves.maps.add')}
                          </button>
                        </div>
                        {over > 0 && <div className="text-[10px] text-slate-500 mt-0.5">{t('saves.maps.overListed', { count: over })}</div>}
                      </div>
                    );
                  })()}
                  <div className="mt-1 text-[11px]">
                    <button
                      onClick={() => void toggleInside(save)}
                      className="inline-flex items-center gap-1 text-slate-400 hover:text-slate-200 cursor-pointer"
                      title={t('saves.inside.title')}
                    >
                      {insideOpen[save.id] ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                      <User size={12} />
                      {t('saves.inside.toggle')}
                    </button>
                    {insideOpen[save.id] && (() => {
                      const r = inside[save.id];
                      if (!r || r === 'loading') return <div className="flex items-center gap-1 text-slate-500 mt-1"><Loader2 size={12} className="animate-spin" />{t('saves.inside.reading')}</div>;
                      if ('error' in r) return <div className="text-red-300 mt-1">{r.error}</div>;
                      return (
                        <div className="mt-1 space-y-1.5 text-slate-300">
                          {r.players.length === 0 && <div className="text-slate-500">{t('saves.inside.noPlayers')}</div>}
                          {r.players.map((pl, i) => (
                            <div key={i}>
                              <span className="inline-flex items-center gap-1 text-slate-200"><User size={12} />{pl.name || t('saves.inside.noName')}</span>
                              <span className="ml-2 font-mono text-slate-400">{Math.round(pl.x)}, {Math.round(pl.y)}, {Math.round(pl.z)}</span>
                              {pl.dead && <span className="ml-2 text-red-300">{t('saves.inside.dead')}</span>}
                              {pl.items.length > 0 && (
                                <div className="text-[10px] text-slate-500 mt-0.5" title={pl.items.map((it) => `${it.item_type} ×${it.count}`).join('\n')}>
                                  {t('saves.inside.items', { count: pl.items.reduce((n, it) => n + it.count, 0), kinds: pl.items.length })}: {pl.items.slice(0, 12).map((it) => `${it.item_type.split('.').pop()}${it.count > 1 ? ` ×${it.count}` : ''}`).join(', ')}{pl.items.length > 12 ? ', …' : ''}
                                </div>
                              )}
                            </div>
                          ))}
                          {r.vehicles.length > 0 && (
                            <div>
                              <span className="inline-flex items-center gap-1 text-slate-200"><Car size={12} />{t('saves.inside.vehicles', { count: r.vehicles.length })}</span>
                              <div className="mt-0.5 space-y-0.5">
                                {r.vehicles.map((v) => (
                                  <div key={v.id} className="text-[10px] text-slate-400" title={v.items.map((it) => `${it.item_type} ×${it.count}`).join('\n') || undefined}>
                                    <span className="text-slate-300">{scriptName(v.script)}</span>
                                    <span className="font-mono ml-2">{Math.round(v.x)}, {Math.round(v.y)}</span>
                                    {v.distance !== null && <span className="ml-2">{t('saves.inside.away', { squares: Math.round(v.distance) })}</span>}
                                    {v.items.length > 0 && <span className="ml-2 text-slate-500">{t('saves.inside.vehicleItems', { count: v.items.reduce((n, it) => n + it.count, 0) })}</span>}
                                  </div>
                                ))}
                              </div>
                            </div>
                          )}
                          <div className="text-[10px] text-slate-600">{t('saves.inside.note')}</div>
                        </div>
                      );
                    })()}
                  </div>
                </div>
                <div className="text-[11px] shrink-0">{status(save)}</div>
                <button
                  onClick={() => void backUp(save)}
                  disabled={working !== null || save.busy || !roomFor}
                  title={!roomFor ? t('saves.noRoom', { size: formatBytes(save.bytes) }) : undefined}
                  className="shrink-0 px-2.5 py-1.5 rounded-md bg-emerald-700/70 hover:bg-emerald-600/80 text-emerald-50 text-xs font-medium disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
                >
                  {t('saves.backUpNow')}
                </button>
              </div>
              {expanded && (
                <div className="border-t border-slate-800 px-4 py-2 space-y-1">
                  {save.backups.length === 0 && <div className="text-xs text-slate-500 py-2">{t('saves.noBackups')}</div>}
                  {save.backups.map((b) => (
                    <div key={b.file} className="flex items-center gap-3 text-xs py-1.5">
                      <div className="flex-1 min-w-0">
                        <span className="text-slate-200">{when(b.at)}</span>
                        <span className="ml-2 text-slate-400">{t(`saves.reason.${b.reason}`, { defaultValue: b.reason })}</span>
                        {b.kept && <span className="ml-2 text-[10px] px-1.5 py-0.5 rounded border border-sky-700/60 text-sky-300">{t('saves.keptChip')}</span>}
                        <div className="text-[10px] text-slate-500">
                          {t('saves.backupLine', { zip: formatBytes(b.zip_bytes), size: formatBytes(b.save_bytes), files: b.files.toLocaleString(), mods: b.mods.length })}
                          {b.verified && <span className="ml-2 text-emerald-400">✓ {t('saves.verified')}</span>}
                        </div>
                      </div>
                      <button onClick={() => void toggleKept(save, b)} disabled={working !== null} title={b.kept ? t('saves.unkeepTitle') : t('saves.keepTitle', { keep: overview?.keep_automatic ?? 5 })} className="p-1.5 rounded hover:bg-slate-800 text-slate-400 hover:text-sky-300 disabled:opacity-40 cursor-pointer">
                        {b.kept ? <PinOff className="w-3.5 h-3.5" /> : <Pin className="w-3.5 h-3.5" />}
                      </button>
                      <button onClick={() => restore(save, b)} disabled={working !== null} className="flex items-center gap-1 px-2 py-1 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 disabled:opacity-40 cursor-pointer">
                        <RotateCcw className="w-3.5 h-3.5" />{t('saves.restore')}
                      </button>
                      <button onClick={() => remove(save, b)} disabled={working !== null} className="p-1.5 rounded hover:bg-red-950/60 text-slate-500 hover:text-red-300 disabled:opacity-40 cursor-pointer" aria-label={t('saves.delete')} title={t('saves.delete')}>
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
