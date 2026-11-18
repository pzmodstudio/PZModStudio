import React, { useEffect, useState } from 'react';
import { Archive, FolderInput, FolderOpen, Loader2, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { KeptVersionsSummary } from '../../types';
import { TauriService } from '../../services/tauri';
import { useChangeRevision } from '../../services/changes';
import type { StudioPathsUI } from './SettingsModule';

const GB = 1024 * 1024 * 1024;
/** The caps offered, in GB. */
const CAPS = [1, 2, 5, 10, 20, 50];
/** The default cap (the backend's DEFAULT_CAP_BYTES). */
const DEFAULT_CAP = 10 * GB;

const gb = (bytes: number) => (bytes >= GB ? `${(bytes / GB).toFixed(1)} GB` : `${Math.round(bytes / (1024 * 1024))} MB`);

/**
 * Settings: the versions the Studio keeps by itself so a Workshop update can be undone. Its switch,
 * the cap, the folder (with a move to another one), the drive's room, and what the last round found.
 */
export const KeptVersionsCard: React.FC<{ paths: StudioPathsUI }> = ({ paths }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision();
  const [summary, setSummary] = useState<KeptVersionsSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const [moving, setMoving] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);
  useEffect(() => {
    if (!paths.user_zomboid_dir) return;
    void TauriService.keptVersionsSummary(paths).then(setSummary);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths.user_zomboid_dir, changeRev]);

  // Off until the player decided (the first-time question): nothing is kept before.
  const decided = summary?.decided ?? true;
  const enabled = decided && (summary?.settings.enabled ?? true);
  const cap = summary?.settings.cap_bytes ?? DEFAULT_CAP;
  const status = summary?.status;
  const guard = summary?.guard;
  const used = status?.bytes ?? 0;

  const save = async (on: boolean, capBytes: number) => {
    try {
      setSummary(await TauriService.setKeptVersionsSettings(paths, on, capBytes));
    } catch {
      // Already shown by TauriService.
    }
  };
  const keepNow = async () => {
    setBusy(true);
    try {
      setSummary(await TauriService.keptVersionsRound(paths));
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(false);
    }
  };
  const chooseFolder = async () => {
    const folder = await TauriService.pickFolder(summary?.folder);
    if (!folder) return;
    if (used > 0 && !window.confirm(t('settings.keptVersions.moveConfirm', { size: gb(used), folder }))) return;
    setMoving(true);
    setNotice(null);
    try {
      const out = await TauriService.setKeptVersionsFolder(paths, folder);
      setSummary(out.summary);
      const m = out.moved;
      if (m.unchanged) setNotice({ ok: true, text: t('settings.keptVersions.movedSame', { folder: m.to }) });
      else if (m.old_left) setNotice({ ok: false, text: t('settings.keptVersions.movedOldLeft', { folder: m.to, why: m.old_left }) });
      else setNotice({ ok: true, text: t('settings.keptVersions.moved', { folder: m.to, size: gb(m.bytes) }) });
    } catch (err) {
      setNotice({ ok: false, text: String(err) });
    } finally {
      setMoving(false);
    }
  };

  return (
    <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex items-start justify-between gap-4">
      <div className="space-y-1.5 min-w-0">
        <div className="text-xs font-bold text-slate-200 flex items-center gap-2">
          <Archive className="w-3.5 h-3.5 text-cyan-400" aria-hidden="true" /> {t('settings.keptVersions.title')}
        </div>
        <p className="text-[11px] text-slate-400 max-w-2xl">{t('settings.keptVersions.blurb')}</p>
        <div className="flex flex-wrap items-center gap-2 pt-1 text-[11px] text-slate-300">
          <label className="flex items-center gap-1.5">
            {t('settings.keptVersions.cap')}
            <select
              value={CAPS.includes(Math.round(cap / GB)) ? Math.round(cap / GB) : ''}
              onChange={(e) => void save(enabled, Number(e.target.value) * GB)}
              disabled={!enabled}
              className="px-2 py-1 rounded-lg bg-slate-950 border border-slate-800 text-xs text-slate-200 cursor-pointer disabled:opacity-50"
            >
              {!CAPS.includes(Math.round(cap / GB)) && <option value="">{gb(cap)}</option>}
              {CAPS.map((c) => (
                <option key={c} value={c}>
                  {c} GB
                </option>
              ))}
            </select>
          </label>
          <button type="button" disabled={busy || moving} onClick={() => void keepNow()}
            className="flex items-center gap-1 px-2 py-1 rounded border border-slate-700 text-[11px] text-slate-300 hover:bg-slate-800 cursor-pointer disabled:opacity-60">
            {busy ? <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" /> : <RefreshCw className="w-3 h-3" aria-hidden="true" />}
            {busy ? t('settings.keptVersions.keeping') : t('settings.keptVersions.keepNow')}
          </button>
        </div>
        {summary && (
          <div className="flex flex-wrap items-center gap-2 text-[11px] text-slate-300">
            <span className="text-slate-400">{t('settings.keptVersions.folder')}</span>
            <span className="font-mono text-slate-200 break-all">{summary.folder}</span>
            <span className="text-slate-500">
              {t('settings.keptVersions.folderUse', { used: gb(used) })}
              {!summary.folder_chosen && ` ${t('settings.keptVersions.folderDefault')}`}
            </span>
            <button type="button" disabled={busy || moving || summary.moving} onClick={() => void chooseFolder()}
              className="flex items-center gap-1 px-2 py-1 rounded border border-slate-700 text-[11px] text-slate-300 hover:bg-slate-800 cursor-pointer disabled:opacity-60">
              {moving ? <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" /> : <FolderInput className="w-3 h-3" aria-hidden="true" />}
              {moving || summary.moving ? t('settings.keptVersions.moving') : t('settings.keptVersions.chooseFolder')}
            </button>
            {used > 0 && (
              <button type="button" onClick={() => void TauriService.openModFolder(paths, summary.folder)} title={t('disk.openFolder')}
                className="p-1 rounded text-slate-400 hover:text-slate-100 hover:bg-slate-800 cursor-pointer">
                <FolderOpen className="w-3.5 h-3.5" aria-hidden="true" />
              </button>
            )}
          </div>
        )}
        {notice && <div className={`text-[11px] ${notice.ok ? 'text-emerald-300' : 'text-amber-300/90'}`}>{notice.text}</div>}
        {guard && (
          <div className={`text-[11px] ${guard.limited ? 'text-amber-300/90' : 'text-slate-500'}`}>
            {guard.limited
              ? t('settings.keptVersions.limited', { disk: guard.disk, cap: gb(guard.cap_effective), free: gb(guard.free_bytes), reserve: gb(guard.reserve_bytes) })
              : t('settings.keptVersions.drive', { disk: guard.disk, free: gb(guard.free_bytes), reserve: gb(guard.reserve_bytes) })}
          </div>
        )}
        {status ? (
          <div className="text-[11px] text-slate-400 space-y-0.5">
            <div>
              {t('settings.keptVersions.status', {
                used: gb(status.bytes),
                cap: gb(status.limited_by_disk && status.cap_effective ? status.cap_effective : cap),
                kept: status.items_kept,
                items: status.items_used,
                earlier: status.earlier_versions,
              })}
              {status.limited_by_disk && ` ${t('settings.keptVersions.limitedRound', { disk: status.limited_by_disk })}`}
            </div>
            {status.no_room.length > 0 && (
              <div className="text-amber-300/90" title={status.no_room.join(', ')}>
                {t('settings.keptVersions.noRoom', { count: status.no_room.length, names: status.no_room.slice(0, 4).join(', ') })}
              </div>
            )}
            {status.errors.length > 0 && <div className="text-amber-300/90">{status.errors[0]}</div>}
          </div>
        ) : (
          enabled ? <div className="text-[11px] text-slate-500">{t('settings.keptVersions.notYet')}</div> : !decided && <div className="text-[11px] text-slate-500">{t('settings.keptVersions.notDecided')}</div>
        )}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={enabled}
        aria-label={t('settings.keptVersions.title')}
        onClick={() => void save(!enabled, cap)}
        className={`w-10 h-5 shrink-0 rounded-full p-0.5 flex items-center transition cursor-pointer ${enabled ? 'bg-emerald-500 justify-end' : 'bg-slate-700 justify-start'}`}
      >
        <span className="w-4 h-4 rounded-full bg-white shadow-md" />
      </button>
    </div>
  );
};
