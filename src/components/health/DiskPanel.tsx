import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ExternalLink, FolderOpen, HardDrive, Loader2, RefreshCw, Trash2, Unplug } from 'lucide-react';
import i18next from 'i18next';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import { DiskReport, DiskUnit } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { formatBytes } from '../browse/common';

interface DiskPanelProps {
  paths: StudioPathsUI;
  /** Mods went away from the disk: rescan the list. */
  onModsChanged: () => void;
  onOpenInBrowser?: (workshopId: string) => void;
}

const SHOW_ALL_KEY = 'pz_disk_show_all';

/**
 * The last measurement, kept while the app runs: measuring walks every mod folder and the game's
 * (seconds), so coming back to this tab shows what was measured, with its time, and measures
 * again only on the button or once the mods changed.
 */
let lastMeasured: { key: string; report: DiskReport; at: number } | null = null;
const pathsKey = (paths: StudioPathsUI) => `${paths.pz_install_dir}|${paths.workshop_dir}|${paths.user_zomboid_dir}`;

/** What the Studio's own folders in Zomboid hold, by name. */
const FOLDER_KEYS: Record<string, string> = {
  game: 'game',
  workshop: 'workshop',
  mods: 'mods',
  saves: 'saves',
  PZModStudio_SaveBackups: 'saveBackups',
  PZModStudio_Snapshots: 'snapshots',
  PZModStudio_Backups: 'listBackups',
  PZModStudio_Cache: 'cache',
  PZModStudio_Staging: 'staging',
  PZModStudio_Logs: 'studioLogs',
  Logs: 'gameLogs',
};

/** A unit the Studio can give back: a Workshop item by unsubscribing, a folder to the Recycle Bin. */
const removable = (u: DiskUnit) => (u.unused || u.idle) && (u.kind === 'workshop' || u.kind === 'local' || u.kind === 'leftover');

/**
 * Disk use: every installed mod's size, and the ones no profile, save, kept server or the game's
 * active list uses. Those can go: a Workshop item by unsubscribing (Steam removes its files),
 * a local folder or a leftover Workshop folder to the Recycle Bin.
 */
export const DiskPanel: React.FC<DiskPanelProps> = ({ paths, onModsChanged, onOpenInBrowser }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision(['mods']);
  const [report, setReport] = useState<DiskReport | null>(() => (lastMeasured?.key === pathsKey(paths) ? lastMeasured.report : null));
  const [measuredAt, setMeasuredAt] = useState<number | null>(() => (lastMeasured?.key === pathsKey(paths) ? lastMeasured.at : null));
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);
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

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const next = await TauriService.diskReport(paths);
      lastMeasured = { key: pathsKey(paths), report: next, at: Date.now() };
      setReport(next);
      setMeasuredAt(lastMeasured.at);
      setPicked((old) => new Set([...old].filter((f) => next.units.some((u) => u.folder === f && removable(u)))));
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [paths]);

  // On the first look, and again once the mods changed; not on every return to the tab.
  const seenRev = useRef(changeRev);
  useEffect(() => {
    if (lastMeasured?.key === pathsKey(paths) && seenRev.current === changeRev) return;
    seenRev.current = changeRev;
    void load();
  }, [load, changeRev, paths]);

  const shown = useMemo(() => (report ? report.units.filter((u) => showAll || u.unused || u.idle) : []), [report, showAll]);
  const pickedUnits = report ? report.units.filter((u) => picked.has(u.folder)) : [];
  const toUnsubscribe = pickedUnits.filter((u) => u.kind === 'workshop');
  const toRecycle = pickedUnits.filter((u) => u.kind === 'local' || u.kind === 'leftover');
  const sum = (list: DiskUnit[]) => list.reduce((n, u) => n + u.bytes, 0);
  /** The names for a confirmation, each kept-off one with the profiles that keep it. */
  const namesOf = (list: DiskUnit[]) =>
    list.map((u) => (u.idle ? `${u.name} (${t('disk.keptOffBy', { names: u.used_by.profiles_off.join(', ') })})` : u.name)).join('\n');

  const toggle = (folder: string) =>
    setPicked((old) => {
      const next = new Set(old);
      if (next.has(folder)) next.delete(folder);
      else next.add(folder);
      return next;
    });
  const pickAllUnused = () => setPicked(new Set((report?.units ?? []).filter(removable).map((u) => u.folder)));

  const unsubscribe = async () => {
    if (toUnsubscribe.length === 0) return;
    if (!window.confirm(t('disk.unsubscribeConfirm', { count: toUnsubscribe.length, size: formatBytes(sum(toUnsubscribe)), names: namesOf(toUnsubscribe) }))) return;
    setBusy(true);
    setNotice(null);
    try {
      const out = await TauriService.workshopSubscribe(paths, toUnsubscribe.map((u) => u.workshop_id as string), false);
      setNotice(
        out.failed.length === 0
          ? { ok: true, text: t('disk.unsubscribed', { count: out.done.length }) }
          : { ok: false, text: t('disk.unsubscribeFailed', { done: out.done.length, failed: out.failed.map(([id, e]) => `${id}: ${e}`).join('; ') }) }
      );
      onModsChanged();
      await load();
    } catch (err) {
      setNotice({ ok: false, text: String(err) });
    } finally {
      setBusy(false);
    }
  };

  const recycle = async () => {
    if (toRecycle.length === 0) return;
    if (!window.confirm(t('disk.recycleConfirm', { count: toRecycle.length, size: formatBytes(sum(toRecycle)), names: namesOf(toRecycle) }))) return;
    setBusy(true);
    setNotice(null);
    const failed: string[] = [];
    for (const u of toRecycle) {
      try {
        await TauriService.diskRecycle(paths, u.folder);
      } catch (err) {
        failed.push(`${u.name}: ${String(err)}`);
      }
    }
    setNotice(
      failed.length === 0
        ? { ok: true, text: t('disk.recycled', { count: toRecycle.length, size: formatBytes(sum(toRecycle)) }) }
        : { ok: false, text: t('disk.recycleFailed', { failed: failed.join('; ') }) }
    );
    onModsChanged();
    await load();
    setBusy(false);
  };

  const usedLine = (u: DiskUnit) => {
    const parts: string[] = [];
    if (u.used_by.active) parts.push(t('disk.used.active'));
    if (u.used_by.profiles.length > 0) parts.push(t('disk.used.profiles', { count: u.used_by.profiles.length, names: u.used_by.profiles.join(', ') }));
    if (u.used_by.profiles_off.length > 0) parts.push(t('disk.used.profilesOff', { count: u.used_by.profiles_off.length, names: u.used_by.profiles_off.join(', ') }));
    if (u.used_by.saves.length > 0) parts.push(t('disk.used.saves', { count: u.used_by.saves.length }));
    if (u.used_by.servers.length > 0) parts.push(t('disk.used.servers', { count: u.used_by.servers.length, names: u.used_by.servers.join(', ') }));
    return parts.join(' · ');
  };
  const usedTitle = (u: DiskUnit) =>
    [
      u.used_by.profiles.length > 0 && `${t('disk.used.profilesList')}: ${u.used_by.profiles.join(', ')}`,
      u.used_by.profiles_off.length > 0 && `${t('disk.used.profilesOffList')}: ${u.used_by.profiles_off.join(', ')}`,
      u.used_by.saves.length > 0 && `${t('disk.used.savesList')}: ${u.used_by.saves.join(', ')}`,
      u.used_by.servers.length > 0 && `${t('disk.used.serversList')}: ${u.used_by.servers.join(', ')}`,
      u.mod_ids.length > 0 && `${t('disk.modsInside')}: ${u.mod_ids.join(', ')}`,
    ]
      .filter(Boolean)
      .join('\n');

  return (
    <section className="flex flex-col gap-3 p-5 rounded-xl border border-slate-800 bg-slate-900/60">
      <div className="flex items-start gap-3">
        <HardDrive className="w-5 h-5 text-sky-400 mt-0.5" aria-hidden="true" />
        <div className="flex-1">
          <h2 className="text-sm font-bold text-slate-100">{t('disk.title')}</h2>
          <p className="text-xs text-slate-400 max-w-3xl">{t('disk.blurb')}</p>
        </div>
        <div className="flex flex-col items-end gap-1 shrink-0">
          <button
            type="button"
            onClick={() => void load()}
            disabled={loading || busy}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-slate-700 text-xs text-slate-200 hover:bg-slate-800 cursor-pointer disabled:opacity-50"
          >
            {loading ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <RefreshCw className="w-3.5 h-3.5" aria-hidden="true" />}
            {t('disk.refresh')}
          </button>
          {measuredAt !== null && !loading && (
            <span className="text-[10px] text-slate-500">{t('disk.measuredAt', { time: new Date(measuredAt).toLocaleTimeString(i18next.language, { hour: '2-digit', minute: '2-digit' }) })}</span>
          )}
        </div>
      </div>

      {error && <div className="text-xs rounded-md border border-red-700/60 bg-red-950/40 text-red-200 px-3 py-2">{error}</div>}
      {!report && loading && <div className="text-xs text-slate-400">{t('disk.measuring')}</div>}

      {report && (
        <>
          <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-xs">
            <span className="text-lg font-bold text-slate-100">{t('disk.total', { size: formatBytes(report.mods_bytes) || '0 B', count: report.units.length })}</span>
            <span className={report.unused_count > 0 ? 'text-amber-300' : 'text-slate-400'}>
              {report.unused_count > 0 ? t('disk.unusedSum', { size: formatBytes(report.unused_bytes), count: report.unused_count }) : t('disk.noneUnused')}
            </span>
            {report.idle_count > 0 && <span className="text-sky-300">{t('disk.idleSum', { size: formatBytes(report.idle_bytes) || '0 B', count: report.idle_count })}</span>}
            {report.disks.map((d) => (
              <span key={d.name} className="text-slate-400" title={t('disk.holds', { what: d.holds.map((h) => t(`disk.on.${h}`)).join(', ') })}>
                {t('disk.free', { free: formatBytes(d.free_bytes), total: formatBytes(d.total_bytes), disk: d.name })}
                {report.disks.length > 1 && <span className="text-slate-500"> ({d.holds.map((h) => t(`disk.on.${h}`)).join(', ')})</span>}
              </span>
            ))}
          </div>

          <div className="flex flex-wrap items-center gap-2 text-xs">
            <button type="button" onClick={toggleShowAll} className="px-2.5 py-1 rounded-md border border-slate-700 text-slate-200 hover:bg-slate-800 cursor-pointer">
              {showAll ? t('disk.showUnused') : t('disk.showAll', { count: report.units.length })}
            </button>
            {report.units.some(removable) && (
              <button type="button" onClick={pickAllUnused} disabled={busy} className="px-2.5 py-1 rounded-md border border-slate-700 text-slate-200 hover:bg-slate-800 cursor-pointer disabled:opacity-50">
                {t('disk.pickAll')}
              </button>
            )}
            {picked.size > 0 && (
              <button type="button" onClick={() => setPicked(new Set())} disabled={busy} className="px-2.5 py-1 rounded-md text-slate-400 hover:text-slate-200 cursor-pointer">
                {t('disk.pickNone')}
              </button>
            )}
            <span className="flex-1" />
            {toUnsubscribe.length > 0 && (
              <button
                type="button"
                onClick={() => void unsubscribe()}
                disabled={busy}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-bold bg-amber-600 hover:bg-amber-500 text-white cursor-pointer disabled:opacity-50"
              >
                {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Unplug className="w-3.5 h-3.5" aria-hidden="true" />}
                {t('disk.unsubscribe', { count: toUnsubscribe.length, size: formatBytes(sum(toUnsubscribe)) })}
              </button>
            )}
            {toRecycle.length > 0 && (
              <button
                type="button"
                onClick={() => void recycle()}
                disabled={busy}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-bold bg-red-700 hover:bg-red-600 text-white cursor-pointer disabled:opacity-50"
              >
                {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Trash2 className="w-3.5 h-3.5" aria-hidden="true" />}
                {t('disk.recycle', { count: toRecycle.length, size: formatBytes(sum(toRecycle)) })}
              </button>
            )}
          </div>

          {notice && (
            <div className={`text-xs rounded-md border px-3 py-2 ${notice.ok ? 'border-emerald-700/60 bg-emerald-950/40 text-emerald-200' : 'border-red-700/60 bg-red-950/40 text-red-200'}`}>{notice.text}</div>
          )}

          {shown.length === 0 ? (
            <div className="text-xs text-slate-400 py-4 text-center">{showAll ? t('disk.empty') : t('disk.allUsed')}</div>
          ) : (
            <ul className="flex flex-col divide-y divide-slate-800 border border-slate-800 rounded-lg">
              {shown.map((u) => (
                <li key={u.folder} className="flex items-center gap-3 px-3 py-2">
                  <input
                    type="checkbox"
                    checked={picked.has(u.folder)}
                    onChange={() => toggle(u.folder)}
                    disabled={!removable(u) || busy}
                    title={removable(u) ? undefined : u.kind === 'upload' ? t('disk.uploadTitle') : t('disk.inUseTitle')}
                    className="accent-amber-500 disabled:opacity-30"
                    aria-label={u.name}
                  />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="text-sm font-semibold text-slate-100 truncate">{u.name}</span>
                      <span className="shrink-0 text-[10px] px-1.5 py-0.5 rounded border border-slate-700 text-slate-400" title={t(`disk.kindTitle.${u.kind}`)}>
                        {t(`disk.kind.${u.kind}`)}
                      </span>
                    </div>
                    <div className="text-[11px] truncate" title={usedTitle(u)}>
                      {u.unused ? (
                        <span className="text-amber-300">{u.kind === 'leftover' ? t('disk.leftoverLine') : t('disk.unusedLine')}</span>
                      ) : u.idle ? (
                        <span className="text-sky-300">{t('disk.idleLine', { names: u.used_by.profiles_off.join(', ') })}</span>
                      ) : (
                        <span className="text-slate-500">{usedLine(u)}</span>
                      )}
                    </div>
                  </div>
                  <div className="text-right shrink-0">
                    <div className="text-sm font-mono text-slate-100">{formatBytes(u.bytes) || '0 B'}</div>
                    <div className="text-[10px] text-slate-500">{t('disk.files', { count: u.files })}</div>
                  </div>
                  {u.workshop_id && u.kind === 'workshop' && onOpenInBrowser ? (
                    <button type="button" onClick={() => onOpenInBrowser(u.workshop_id as string)} title={t('disk.openInBrowse')} className="p-1.5 rounded-md text-slate-400 hover:text-slate-100 hover:bg-slate-800 cursor-pointer">
                      <ExternalLink className="w-4 h-4" aria-hidden="true" />
                    </button>
                  ) : (
                    /* the same room, so the sizes and the folder buttons line up down the list */
                    <span className="w-7 h-7 shrink-0" aria-hidden="true" />
                  )}
                  <button type="button" onClick={() => void TauriService.openModFolder(paths, u.folder)} title={t('disk.openFolder')} className="p-1.5 rounded-md text-slate-400 hover:text-slate-100 hover:bg-slate-800 cursor-pointer">
                    <FolderOpen className="w-4 h-4" aria-hidden="true" />
                  </button>
                </li>
              ))}
            </ul>
          )}

          {report.folders.length > 0 && (
            <div className="flex flex-col gap-1">
              <h3 className="text-xs font-bold text-slate-300 mt-2">{t('disk.foldersTitle')}</h3>
              <p className="text-[11px] text-slate-500 max-w-3xl">{t('disk.foldersBlurb')}</p>
              <ul className="flex flex-col divide-y divide-slate-800 border border-slate-800 rounded-lg">
                {report.folders.map((f) => (
                  <li key={f.path} className="flex items-center gap-3 px-3 py-1.5">
                    <div className="flex-1 min-w-0">
                      <div className="text-xs text-slate-200">{FOLDER_KEYS[f.kind] ? t(`disk.folder.${FOLDER_KEYS[f.kind]}`) : f.name}</div>
                      <div className="text-[10px] font-mono text-slate-500 truncate">{f.path}</div>
                    </div>
                    <div className="text-xs font-mono text-slate-200 shrink-0">{formatBytes(f.bytes)}</div>
                    <button type="button" onClick={() => void TauriService.openModFolder(paths, f.path)} title={t('disk.openFolder')} className="p-1.5 rounded-md text-slate-400 hover:text-slate-100 hover:bg-slate-800 cursor-pointer">
                      <FolderOpen className="w-4 h-4" aria-hidden="true" />
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          )}

          <p className="text-[11px] text-slate-500 max-w-3xl">{t('disk.how')}</p>
        </>
      )}
    </section>
  );
};
