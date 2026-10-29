import React, { useEffect, useState } from 'react';
import { KeptVersion, ModInfo } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { Archive, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';

interface KeptVersionsProps {
  paths: StudioPathsUI;
  /** The mod whose kept versions are shown; null = closed. */
  mod: ModInfo | null;
  onClose: () => void;
  /** A kept version was put back or a copy removed: the list on disk changed. */
  onModsChanged: () => void;
  toast: (message: string, ms?: number) => void;
}

const sizeOf = (bytes: number) => (bytes >= 1 << 20 ? `${(bytes / (1 << 20)).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`);

/**
 * The versions of one mod the Studio kept: each a copy of the folder as it was that day. Putting
 * one back makes it a local copy the game reads instead of the Workshop one; the kept copy stays.
 */
export const KeptVersions: React.FC<KeptVersionsProps> = ({ paths, mod, onClose, onModsChanged, toast }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision();
  const [all, setAll] = useState<KeptVersion[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const reload = async () => setAll(await TauriService.listKeptVersions(paths));
  useEffect(() => {
    if (!mod) return;
    setAll(null);
    void reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mod?.mod_id, changeRev]);

  if (!mod) return null;
  const versions = (all ?? []).filter((v) => v.mod_id === mod.mod_id);

  const keepNow = async () => {
    setBusy('keep');
    try {
      const kept = await TauriService.keepModVersion(paths, mod.mod_id);
      toast(t('dialogs.kept.keptNow', { name: kept.mod_name, when: kept.kept_at }));
      await reload();
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(null);
    }
  };

  const restore = async (v: KeptVersion) => {
    if (
      !window.confirm(t('dialogs.kept.confirmUse', { name: v.mod_name, when: v.kept_at, version: v.version ? t('dialogs.kept.modversion', { version: v.version }) : '' }))
    )
      return;
    setBusy(v.path);
    try {
      const back = await TauriService.restoreKeptVersion(paths, v.path);
      toast(t('dialogs.kept.restored', { name: back.mod_name, when: back.kept_at }), 6000);
      onModsChanged();
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(null);
    }
  };

  const remove = async (v: KeptVersion) => {
    if (!window.confirm(t('dialogs.kept.confirmDelete', { name: v.mod_name, when: v.kept_at }))) return;
    setBusy(v.path);
    try {
      await TauriService.deleteKeptVersion(paths, v.path);
      await reload();
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-6" onClick={onClose}>
      <div className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl w-full max-w-xl p-5 space-y-4" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
              <Archive className="w-4 h-4 text-cyan-400" />
              <span>{t('dialogs.kept.title', { name: mod.name })}</span>
            </h3>
            <p className="text-[11px] text-slate-400 mt-0.5">
              {t('dialogs.kept.blurb')}
            </p>
          </div>
          <button onClick={onClose} className="text-slate-500 hover:text-slate-300 p-1 cursor-pointer">
            <X className="w-4 h-4" aria-hidden="true" />
          </button>
        </div>

        <button
          onClick={() => void keepNow()}
          disabled={busy !== null}
          className="w-full py-1.5 rounded-lg bg-cyan-950/70 hover:bg-cyan-900/70 border border-cyan-800 text-cyan-200 text-[11px] font-bold transition cursor-pointer disabled:opacity-50"
          title={t('dialogs.kept.keepTitle')}
        >
          {busy === 'keep' ? t('dialogs.kept.keeping') : t('dialogs.kept.keepToday', { version: mod.version ? t('dialogs.kept.modversion', { version: mod.version }) : '' })}
        </button>

        {all === null ? (
          <p className="text-xs text-slate-400 py-4 text-center">{t('dialogs.kept.reading')}</p>
        ) : versions.length === 0 ? (
          <p className="text-xs text-slate-400 py-4 text-center">{t('dialogs.kept.none')}</p>
        ) : (
          <div className="max-h-80 overflow-y-auto divide-y divide-slate-800 rounded-xl border border-slate-800">
            {versions.map((v) => (
              <div key={v.path} className="p-3 flex items-start justify-between gap-3 hover:bg-slate-950/60 transition">
                <div className="min-w-0">
                  <div className="text-xs font-mono text-slate-200">{v.kept_at}</div>
                  <div className="text-[11px] text-slate-400">
                    {v.version ? `modversion ${v.version} · ` : ''}
                    {sizeOf(v.bytes)}
                  </div>
                  <div className="text-[10px] text-slate-500 font-mono truncate" title={v.path}>
                    {v.path}
                  </div>
                </div>
                <div className="flex items-center gap-1.5 shrink-0">
                  <button
                    onClick={() => void restore(v)}
                    disabled={busy !== null}
                    className="px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 cursor-pointer transition disabled:opacity-50"
                  >
                    {busy === v.path ? t('dialogs.kept.working') : t('dialogs.kept.use')}
                  </button>
                  <button
                    onClick={() => void remove(v)}
                    disabled={busy !== null}
                    className="px-2 py-1.5 rounded-lg text-[11px] text-slate-400 hover:text-red-300 border border-slate-800 hover:border-red-900 cursor-pointer transition disabled:opacity-50"
                    title={t('dialogs.kept.deleteTitle')}
                  >
                    {t('dialogs.kept.delete')}
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
