import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Archive, X } from 'lucide-react';
import { KeptVersionsSummary } from '../../types';
import { TauriService } from '../../services/tauri';
import type { StudioPathsUI } from '../settings/SettingsModule';

const GB = 1024 * 1024 * 1024;
const size = (bytes: number) => (bytes >= GB ? `${(bytes / GB).toFixed(1)} GB` : `${Math.max(1, Math.round(bytes / (1024 * 1024)))} MB`);

/**
 * Once, on a PC that has not decided: keep earlier versions of the Workshop mods? Says how much it
 * would use and on which drive before anything is written; nothing is kept until the player
 * answers (yes here, yes in a folder they pick, or no). Settings > The app changes it later.
 */
export const KeptVersionsOffer: React.FC<{ paths: StudioPathsUI }> = ({ paths }) => {
  const { t } = useTranslation();
  const [summary, setSummary] = useState<KeptVersionsSummary | null>(null);
  const [estimate, setEstimate] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!paths.is_valid || !paths.user_zomboid_dir) return;
    let alive = true;
    void TauriService.keptVersionsSummary(paths).then((s) => {
      if (!alive || !s || s.decided) return;
      setSummary(s);
      void TauriService.keptVersionsEstimate(paths).then((e) => alive && setEstimate(e));
    });
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths.is_valid, paths.user_zomboid_dir]);

  if (!summary || summary.decided) return null;
  const cap = summary.guard?.cap_effective || summary.settings.cap_bytes;
  const about = estimate && estimate > 0 ? Math.min(estimate, cap) : cap;
  const disk = summary.guard?.disk ?? '';

  const answer = async (on: boolean) => {
    setBusy(true);
    try {
      setSummary(await TauriService.setKeptVersionsSettings(paths, on, summary.settings.cap_bytes));
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(false);
    }
  };
  const elsewhere = async () => {
    const folder = await TauriService.pickFolder(summary.folder);
    if (!folder) return;
    setBusy(true);
    try {
      await TauriService.setKeptVersionsFolder(paths, folder);
      setSummary(await TauriService.setKeptVersionsSettings(paths, true, summary.settings.cap_bytes));
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(false);
    }
  };

  return (
    <div role="status" className="flex flex-wrap items-center gap-3 px-4 py-2 bg-cyan-950/50 border-b border-cyan-800/60 text-xs text-cyan-50">
      <Archive className="w-4 h-4 text-cyan-300 shrink-0" aria-hidden="true" />
      <div className="flex-1 min-w-[260px]">
        <span className="font-semibold">{t('keptOffer.title')}</span>{' '}
        <span className="text-cyan-100/80">
          {t('keptOffer.body', { size: size(about), disk, free: size(summary.guard?.free_bytes ?? 0), cap: size(cap) })}
        </span>
      </div>
      <button type="button" disabled={busy} onClick={() => void answer(true)}
        className="px-3 py-1 rounded-md font-bold bg-cyan-600 hover:bg-cyan-500 text-white cursor-pointer disabled:opacity-60">
        {t('keptOffer.yes', { disk })}
      </button>
      <button type="button" disabled={busy} onClick={() => void elsewhere()}
        className="px-3 py-1 rounded-md font-bold border border-cyan-700 text-cyan-100 hover:bg-cyan-900/60 cursor-pointer disabled:opacity-60">
        {t('keptOffer.elsewhere')}
      </button>
      <button type="button" disabled={busy} onClick={() => void answer(false)} title={t('keptOffer.noTitle')}
        className="flex items-center gap-1 px-2 py-1 rounded-md text-cyan-200 hover:bg-cyan-900/60 cursor-pointer disabled:opacity-60">
        <X className="w-3.5 h-3.5" aria-hidden="true" /> {t('keptOffer.no')}
      </button>
    </div>
  );
};
