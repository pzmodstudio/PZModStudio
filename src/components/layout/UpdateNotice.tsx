import React, { useEffect, useState } from 'react';
import { Download, Loader2, RefreshCw, X, Sparkles } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { TauriService } from '../../services/tauri';
import { AppUpdateStatus } from '../../types';

const RELEASES = 'https://github.com/pzmodstudio/PZModStudio/releases/latest';
/** While the app stays open it asks again this often (the backend trusts an answer as long). */
const RECHECK_MS = 6 * 60 * 60 * 1000;

/**
 * "Update now": downloads the release, checks its signed checksums and the zip's SHA-256, puts
 * the new app in place of this one and starts it. A failure says why and leaves everything as it
 * was; the release page stays one click away.
 */
export const UpdateNowButton: React.FC<{ version: string; compact?: boolean }> = ({ version, compact }) => {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = async () => {
    if (!window.confirm(t('update.confirm', { latest: version }))) return;
    setBusy(true);
    setError(null);
    try {
      await TauriService.selfUpdate(version);
      // The app closes itself once the new one has started.
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  };
  return (
    <>
      <button
        type="button"
        onClick={() => void run()}
        disabled={busy}
        title={t('update.nowTitle')}
        className={`flex items-center gap-1 ${compact ? 'px-2 py-1 text-[11px]' : 'px-2.5 py-0.5'} rounded bg-emerald-600 hover:bg-emerald-500 text-white font-semibold cursor-pointer disabled:opacity-60`}
      >
        {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <RefreshCw className="w-3.5 h-3.5" aria-hidden="true" />}
        {busy ? t('update.working') : t('update.now')}
      </button>
      {error && <span className="text-[11px] text-amber-300 max-w-md">{t('update.failed', { error })}</span>}
    </>
  );
};

/**
 * A bar under the header when a newer PZ Mod Studio is out. The check runs at start and every
 * six hours while the app is open (the backend asks GitHub at most that often, and not at all
 * when it is off in Settings). "Update now" puts the new version in place; Download opens the
 * release page; "Not now" hides it until the next version.
 */
export const UpdateNotice: React.FC<{ userZomboidDir: string }> = ({ userZomboidDir }) => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<AppUpdateStatus | null>(null);

  useEffect(() => {
    if (!userZomboidDir) return;
    let alive = true;
    const ask = () =>
      void TauriService.appUpdateStatus(userZomboidDir).then((s) => {
        if (alive && s) setStatus(s);
      });
    ask();
    const timer = window.setInterval(ask, RECHECK_MS);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [userZomboidDir]);

  if (!status || !status.enabled || !status.newer || status.dismissed || !status.latest) return null;
  const latest = status.latest;

  return (
    <div role="status" className="flex flex-wrap items-center gap-3 px-4 py-1.5 bg-emerald-950/70 border-b border-emerald-800/60 text-xs text-emerald-100">
      <Sparkles className="w-3.5 h-3.5 text-emerald-400 shrink-0" aria-hidden="true" />
      <span className="font-semibold">{t('update.available', { latest })}</span>
      <span className="text-emerald-300/70">({t('update.current', { current: status.current })})</span>
      <span className="ml-auto" />
      {status.self_update && <UpdateNowButton version={latest} />}
      <button
        type="button"
        onClick={() => TauriService.openExternalUrl(status.url ?? RELEASES)}
        className="flex items-center gap-1 px-2.5 py-0.5 rounded text-emerald-200 hover:bg-emerald-900/60 cursor-pointer"
      >
        <Download className="w-3.5 h-3.5" aria-hidden="true" /> {t('update.download')}
      </button>
      <button
        type="button"
        title={t('update.notNowTitle')}
        onClick={() => {
          void TauriService.dismissAppUpdate(userZomboidDir, latest);
          setStatus({ ...status, dismissed: true });
        }}
        className="flex items-center gap-1 px-2 py-0.5 rounded text-emerald-300 hover:bg-emerald-900/60 cursor-pointer"
      >
        <X className="w-3.5 h-3.5" aria-hidden="true" /> {t('update.notNow')}
      </button>
    </div>
  );
};
