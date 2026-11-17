import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Eraser, Loader2, Trash2, Users } from 'lucide-react';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, explainError } from '../../services/tauri';
import { clearRemoteCache } from '../../services/cache';
import { formatBytes } from '../browse/common';
import { Cleanup } from '../../types';

/**
 * Room the Studio can give back by itself: its own cache (only what it builds again when it needs
 * it) and the second player's Zomboid folder a multiplayer test made (to the Recycle Bin; the next
 * test makes it again).
 */
export const CleanupCard: React.FC<{ paths: StudioPathsUI }> = ({ paths }) => {
  const { t } = useTranslation();
  const [info, setInfo] = useState<Cleanup | null>(null);
  const [busy, setBusy] = useState<'cache' | 'second' | null>(null);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);
  const load = () => void TauriService.cleanupInfo(paths).then(setInfo).catch(() => setInfo(null));
  useEffect(load, [paths]);

  const clearCache = async () => {
    if (!info || !window.confirm(t('cleanup.cacheConfirm', { size: formatBytes(info.cache_bytes) || '0 B' }))) return;
    setBusy('cache');
    setNote(null);
    try {
      const freed = await TauriService.cleanupClearCache(paths);
      clearRemoteCache();
      setNote({ ok: true, text: t('cleanup.cacheDone', { size: formatBytes(freed) || '0 B' }) });
    } catch (err) {
      setNote({ ok: false, text: explainError(err) });
    } finally {
      setBusy(null);
      load();
    }
  };

  const removeSecond = async () => {
    if (!info || !window.confirm(t('cleanup.secondConfirm', { dir: info.second_dir, size: formatBytes(info.second_bytes) || '0 B' }))) return;
    setBusy('second');
    setNote(null);
    try {
      await TauriService.cleanupRemoveSecond(paths);
      setNote({ ok: true, text: t('cleanup.secondDone', { dir: info.second_dir }) });
    } catch (err) {
      setNote({ ok: false, text: explainError(err) });
    } finally {
      setBusy(null);
      load();
    }
  };

  if (!info) return null;
  const row = (icon: React.ReactNode, title: string, text: string, button: React.ReactNode) => (
    <div className="flex flex-wrap items-center gap-3 px-3 py-2.5 rounded-lg border border-slate-800 bg-slate-950/50">
      {icon}
      <div className="flex-1 min-w-[240px]">
        <div className="text-xs font-bold text-slate-200">{title}</div>
        <div className="text-[11px] text-slate-400 leading-relaxed">{text}</div>
      </div>
      {button}
    </div>
  );
  return (
    <div className="flex flex-col gap-2">
      <div className="text-[11px] font-bold uppercase tracking-wider text-slate-400">{t('cleanup.title')}</div>
      {row(
        <Eraser className="w-4 h-4 text-sky-300 shrink-0" aria-hidden="true" />,
        t('cleanup.cacheTitle', { size: formatBytes(info.cache_bytes) || '0 B' }),
        t('cleanup.cacheText'),
        <button
          type="button"
          onClick={() => void clearCache()}
          disabled={busy !== null || info.cache_bytes === 0}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-slate-700 text-xs text-slate-200 hover:bg-slate-800 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {busy === 'cache' ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Eraser className="w-3.5 h-3.5" aria-hidden="true" />}
          {t('cleanup.cacheButton')}
        </button>
      )}
      {info.second_exists &&
        row(
          <Users className="w-4 h-4 text-purple-300 shrink-0" aria-hidden="true" />,
          t('cleanup.secondTitle', { size: formatBytes(info.second_bytes) || '0 B' }),
          t('cleanup.secondText', { dir: info.second_dir }),
          <button
            type="button"
            onClick={() => void removeSecond()}
            disabled={busy !== null}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-red-800/70 text-xs text-red-200 hover:bg-red-950/50 cursor-pointer disabled:opacity-40"
          >
            {busy === 'second' ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Trash2 className="w-3.5 h-3.5" aria-hidden="true" />}
            {t('cleanup.secondButton')}
          </button>
        )}
      {note && (
        <div className={`text-xs rounded-md border px-3 py-2 ${note.ok ? 'border-emerald-700/60 bg-emerald-950/40 text-emerald-200' : 'border-red-700/60 bg-red-950/40 text-red-200'}`}>{note.text}</div>
      )}
    </div>
  );
};
