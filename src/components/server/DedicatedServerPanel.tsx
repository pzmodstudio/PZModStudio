import React, { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Check, Download, FolderOpen, HardDrive, Loader2, RefreshCw, X } from 'lucide-react';
import { DedicatedView, StudioPathsUI } from '../../types';
import { TauriService, explainError } from '../../services/tauri';
import { isWindows } from '../../utils/platform';

const gb = (bytes?: number | null) => (bytes ? `${(bytes / 1024 ** 3).toFixed(bytes > 10 * 1024 ** 3 ? 0 : 1)} GB` : '');
const mb = (bytes?: number | null) => (bytes ? `${(bytes / 1024 ** 2).toFixed(1)} MB` : '');

/**
 * Which program starts the Studio's servers: the game's own files (default), or the official
 * dedicated server installed and updated with SteamCMD in a folder the player picks.
 */
export const DedicatedServerPanel: React.FC<{ paths: StudioPathsUI; serverRunning: boolean }> = ({ paths, serverRunning }) => {
  const { t } = useTranslation();
  const [view, setView] = useState<DedicatedView | null>(null);
  const [dir, setDir] = useState('');
  const [branch, setBranch] = useState('public');
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);
  const [askDownload, setAskDownload] = useState(false);
  const [open, setOpen] = useState(false);

  const load = useCallback(async () => {
    const v = await TauriService.dedicatedServerView();
    setView(v);
    return v;
  }, []);

  useEffect(() => {
    void load().then((v) => {
      setDir(v.config.dir || v.installs[0]?.dir || v.default_dir);
      setBranch(v.config.branch || v.game_branch || 'public');
      if (v.config.source === 'dedicated' || v.progress?.running) setOpen(true);
    });
  }, [load]);

  // What is on disk changes outside this screen (an install from another window or an agent,
  // SteamCMD updating itself): looked at again whenever the window comes back to the front.
  useEffect(() => {
    const again = () => {
      if (document.visibilityState === 'visible') void load();
    };
    window.addEventListener('focus', again);
    document.addEventListener('visibilitychange', again);
    return () => {
      window.removeEventListener('focus', again);
      document.removeEventListener('visibilitychange', again);
    };
  }, [load]);

  const running = Boolean(view?.progress?.running);
  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(() => void load(), 1000);
    return () => window.clearInterval(id);
  }, [running, load]);

  if (!view) return null;

  /** "code: English text" from the backend, in the player's language. */
  const explain = (err: unknown) => {
    const text = String(err);
    const m = text.match(/^(steamcmd_missing|bad_folder|bad_branch|server_running|install_running|not_installed|download_failed|signature|bad_steamcmd): (.*)$/s);
    return m ? t(`servers.dedicated.errors.${m[1]}`, { detail: m[2] }) : explainError(err);
  };

  const pick = async () => {
    const chosen = await TauriService.pickFolder(dir || undefined);
    if (chosen) setDir(chosen);
  };

  const install = async (allowDownload: boolean) => {
    setBusy(true);
    setNote(null);
    try {
      // Every file is checked only for a first install or one cut short; an update of a complete
      // one fetches what changed (seconds instead of many minutes for ~7 GB).
      const validate = !chosen?.ready || Boolean(chosen?.incomplete) || Boolean(chosen?.check_due);
      await TauriService.dedicatedServerInstall(paths, { dir, branch, validate, allow_download: allowDownload });
      setAskDownload(false);
      await load();
    } catch (err) {
      // Valve's download is the Windows program: on Linux SteamCMD comes from the distribution.
      // Asked only when SteamCMD is really not there now (the screen may have looked before it was).
      const fresh = String(err).startsWith('steamcmd_missing') ? await load().catch(() => null) : null;
      if (fresh?.steamcmd && !allowDownload) {
        // It is there now: the same install again, no question.
        try {
          await TauriService.dedicatedServerInstall(paths, { dir, branch, validate: !chosen?.ready || Boolean(chosen?.incomplete) || Boolean(chosen?.check_due), allow_download: false });
          await load();
        } catch (again) {
          setNote({ ok: false, text: explain(again) });
        }
      } else if (String(err).startsWith('steamcmd_missing') && isWindows) setAskDownload(true);
      else if (String(err).startsWith('steamcmd_missing')) setNote({ ok: false, text: t('servers.dedicated.errors.steamcmd_missing_linux') });
      else setNote({ ok: false, text: explain(err) });
    } finally {
      setBusy(false);
    }
  };

  const useSource = async (source: 'game' | 'dedicated') => {
    setBusy(true);
    setNote(null);
    try {
      await TauriService.dedicatedServerSet({ source, dir, branch });
      await load();
      setNote({ ok: true, text: source === 'dedicated' ? t('servers.dedicated.usingDedicated') : t('servers.dedicated.usingGame') });
    } catch (err) {
      setNote({ ok: false, text: explain(err) });
    } finally {
      setBusy(false);
    }
  };

  const chosen = view.installs.find((i) => i.dir.toLowerCase() === dir.toLowerCase()) ?? (view.chosen?.dir.toLowerCase() === dir.toLowerCase() ? view.chosen : null);
  const p = view.progress;
  const usingDedicated = Boolean(view.starts_from);
  const branchLabel = (b?: string | null) => (b === 'unstable' ? t('servers.dedicated.branchUnstable') : b === 'public' || !b ? t('servers.dedicated.branchPublic') : b);

  return (
    <div className="p-4 bg-slate-950 rounded-xl border border-slate-800 space-y-3">
      <button type="button" onClick={() => setOpen(!open)} className="w-full flex items-center justify-between gap-2 text-xs font-bold text-slate-300 cursor-pointer">
        <span className="flex items-center gap-1.5">
          <HardDrive className="w-4 h-4 text-purple-400" aria-hidden="true" />
          {t('servers.dedicated.title')}
        </span>
        <span className={`text-[11px] font-mono ${usingDedicated ? 'text-emerald-300' : 'text-slate-400'}`}>
          {usingDedicated ? t('servers.dedicated.fromDedicated') : t('servers.dedicated.fromGame')}
        </span>
      </button>
      {open && (
        <div className="space-y-3">
          <p className="text-[11px] text-slate-400">{t('servers.dedicated.intro')}</p>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
            <button
              type="button"
              disabled={busy || serverRunning}
              onClick={() => void useSource('game')}
              className={`p-2.5 rounded-lg text-left text-[11px] border cursor-pointer disabled:cursor-not-allowed ${!usingDedicated ? 'bg-purple-950/60 border-purple-500 text-purple-100' : 'bg-slate-900 border-slate-800 text-slate-300'}`}
            >
              <div className="font-bold">{t('servers.dedicated.sourceGame')}</div>
              <div className="text-slate-400">{t('servers.dedicated.sourceGameHelp')}</div>
            </button>
            <button
              type="button"
              disabled={busy || serverRunning || !chosen?.ready}
              onClick={() => void useSource('dedicated')}
              className={`p-2.5 rounded-lg text-left text-[11px] border cursor-pointer disabled:cursor-not-allowed disabled:opacity-60 ${usingDedicated ? 'bg-purple-950/60 border-purple-500 text-purple-100' : 'bg-slate-900 border-slate-800 text-slate-300'}`}
            >
              <div className="font-bold">{t('servers.dedicated.sourceDedicated')}</div>
              <div className="text-slate-400">{chosen?.ready ? t('servers.dedicated.sourceDedicatedHelp') : t('servers.dedicated.installFirst')}</div>
            </button>
          </div>
          {serverRunning && <p className="text-[11px] text-amber-300/90">{t('servers.dedicated.stopFirst')}</p>}

          <div className="space-y-1.5">
            <div className="text-[11px] font-bold text-slate-300">{t('servers.dedicated.folder')}</div>
            <div className="flex gap-2">
              <input
                value={dir}
                onChange={(e) => setDir(e.target.value)}
                aria-label={t('servers.dedicated.folder')}
                className="flex-1 bg-slate-900 border border-slate-800 rounded-lg px-2.5 py-1.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-purple-500"
              />
              <button type="button" onClick={() => void pick()} className="px-2.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer" title={t('servers.dedicated.pick')}>
                <FolderOpen className="w-3.5 h-3.5" aria-hidden="true" />
              </button>
            </div>
            {chosen ? (
              <p className="text-[11px] text-emerald-300/90">
                {t('servers.dedicated.found', { build: chosen.build_id ?? '?', branch: branchLabel(chosen.branch), size: gb(chosen.size_on_disk) || '?' })}
                {chosen.incomplete && <span className="text-amber-300"> {t('servers.dedicated.incomplete')}</span>}
                {chosen.version && <span> {t('servers.dedicated.version', { version: chosen.version })}</span>}
                {chosen.check_due && !chosen.incomplete && <span className="text-amber-300"> {t('servers.dedicated.checkDue')}</span>}
              </p>
            ) : (
              <p className="text-[11px] text-slate-500">{t('servers.dedicated.notThere')}</p>
            )}
            {chosen?.version && view.game_version && (
              <p className={`text-[11px] ${chosen.version === view.game_version ? 'text-slate-400' : 'text-amber-300/90'}`}>
                {chosen.version === view.game_version
                  ? t('servers.dedicated.versionMatches', { game: view.game_version })
                  : t('servers.dedicated.versionMismatch', { game: view.game_version, version: chosen.version })}
              </p>
            )}
            {view.installs.filter((i) => i.dir.toLowerCase() !== dir.toLowerCase()).map((i) => (
              <button key={i.dir} type="button" onClick={() => setDir(i.dir)} className="block text-[11px] text-sky-300 hover:underline cursor-pointer font-mono">
                {t('servers.dedicated.alsoFound', { dir: i.dir })}
              </button>
            ))}
          </div>

          <label className="flex items-center gap-2 text-[11px] text-slate-300">
            <span className="font-bold">{t('servers.dedicated.branch')}</span>
            <select value={branch} onChange={(e) => setBranch(e.target.value)} className="bg-slate-900 border border-slate-800 rounded-lg px-2 py-1 text-xs text-slate-200">
              <option value="public">{t('servers.dedicated.branchPublic')}</option>
              <option value="unstable">{t('servers.dedicated.branchUnstable')}</option>
            </select>
          </label>
          <p className="text-[11px] text-slate-500">
            {t('servers.dedicated.branchHelp')}
            {view.game_branch && <> {t('servers.dedicated.branchGame', { branch: branchLabel(view.game_branch) })}</>}
          </p>
          {view.game_branch && view.game_branch !== branch && <p className="text-[11px] text-amber-300/90">{t('servers.dedicated.branchMismatch')}</p>}

          <div className="text-[11px] text-slate-400">
            {view.steamcmd ? t('servers.dedicated.steamcmdFound', { path: view.steamcmd }) : t('servers.dedicated.steamcmdMissing')}
          </div>

          {askDownload && !view.steamcmd && (
            <div className="p-3 rounded-lg bg-amber-950/40 border border-amber-700/60 text-[11px] text-amber-100 space-y-2">
              <p>{t('servers.dedicated.askDownload', { url: view.steamcmd_url })}</p>
              <div className="flex gap-2">
                <button type="button" disabled={busy} onClick={() => void install(true)} className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-bold bg-amber-600 hover:bg-amber-500 text-white cursor-pointer disabled:opacity-50">
                  <Download className="w-3.5 h-3.5" aria-hidden="true" />
                  {t('servers.dedicated.downloadAndInstall')}
                </button>
                <button type="button" onClick={() => setAskDownload(false)} className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer">
                  {t('servers.dedicated.notNow')}
                </button>
              </div>
            </div>
          )}

          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              disabled={busy || running || !dir.trim()}
              onClick={() => void install(false)}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-purple-600 hover:bg-purple-500 text-white cursor-pointer disabled:opacity-50"
            >
              {chosen ? <RefreshCw className="w-3.5 h-3.5" aria-hidden="true" /> : <Download className="w-3.5 h-3.5" aria-hidden="true" />}
              {chosen ? t('servers.dedicated.update') : t('servers.dedicated.install')}
            </button>
            {running && (
              <button type="button" onClick={() => void TauriService.dedicatedServerCancel().then(() => load())} className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer">
                <X className="w-3.5 h-3.5" aria-hidden="true" />
                {t('servers.dedicated.cancel')}
              </button>
            )}
            <span className="text-[11px] text-slate-500">{t('servers.dedicated.size')}</span>
          </div>

          {p && (p.running || p.finished_at) && (
            <div className="p-3 rounded-lg bg-slate-900 border border-slate-800 space-y-1.5 text-[11px]">
              <div className="flex items-center gap-2 text-slate-200 font-bold">
                {p.running ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : p.phase === 'done' ? <Check className="w-3.5 h-3.5 text-emerald-400" aria-hidden="true" /> : <X className="w-3.5 h-3.5 text-red-400" aria-hidden="true" />}
                <span>{p.phase === 'done' && p.up_to_date ? t('servers.dedicated.phase.upToDate') : t(`servers.dedicated.phase.${p.phase}`, { defaultValue: p.phase })}</span>
                {p.percent != null && <span className="font-mono text-purple-300">{p.percent.toFixed(1)}%</span>}
                {p.total_bytes ? <span className="font-mono text-slate-400">{p.phase === 'download_steamcmd' || p.phase === 'steamcmd_update' ? `${mb(p.done_bytes)} / ${mb(p.total_bytes)}` : `${gb(p.done_bytes)} / ${gb(p.total_bytes)}`}</span> : null}
              </div>
              {p.percent != null && (
                <div className="h-1.5 rounded-full bg-slate-800 overflow-hidden">
                  <div className="h-full bg-purple-500 transition-all" style={{ width: `${Math.min(100, p.percent)}%` }} />
                </div>
              )}
              {/* SteamCMD sets the files' full size before it fills them: while its own bytes are shown, the folder's size says nothing. */}
              {p.disk_bytes && !(p.running && p.total_bytes) ? (
                <div className="text-slate-400">{t(p.running ? 'servers.dedicated.onDisk' : 'servers.dedicated.folderSize', { size: gb(p.disk_bytes) || mb(p.disk_bytes) })}</div>
              ) : null}
              {p.line && <div className="font-mono text-slate-500 break-all">{p.line}</div>}
              {p.error && <div className="text-red-300">{explain(p.error)}</div>}
              {p.command && <div className="font-mono text-slate-600 break-all" title={t('servers.dedicated.commandTitle')}>{p.command}</div>}
            </div>
          )}
          {note && <p className={`text-[11px] ${note.ok ? 'text-emerald-300' : 'text-red-300'}`}>{note.text}</p>}
        </div>
      )}
    </div>
  );
};
