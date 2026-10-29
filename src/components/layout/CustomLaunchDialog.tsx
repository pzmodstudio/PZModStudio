import React, { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import i18next from 'i18next';
import { Archive, Loader2, Play, Server, X } from 'lucide-react';
import { GameLaunchMode, LaunchTarget, SavedServer, SaveSummary, SessionDiff } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';

interface CustomLaunchDialogProps {
  paths: StudioPathsUI;
  /** The active profile's id: its saves come first. */
  activeProfileId?: string | null;
  initial: LaunchTarget;
  modes: GameLaunchMode[];
  mode: GameLaunchMode;
  onCancel: () => void;
  /** start: launch now; otherwise only remember it for the main button. */
  onConfirm: (target: LaunchTarget, mode: GameLaunchMode, start: boolean) => void;
}

const when = (secs: number | null | undefined) => (secs ? new Date(secs * 1000).toLocaleString(i18next.language) : '');

/**
 * "Custom" launch: where the game goes once it starts (the main menu, one save, one server) and
 * in which mode. A save is loaded by the Studio Link at the main menu; a server is joined the way
 * Servers > Join does (kept password, username, Favorites).
 */
export const CustomLaunchDialog: React.FC<CustomLaunchDialogProps> = ({ paths, activeProfileId, initial, modes, mode, onCancel, onConfirm }) => {
  const { t } = useTranslation();
  const [tab, setTab] = useState<'save' | 'server'>(initial.kind === 'server' ? 'server' : 'save');
  const [picked, setPicked] = useState<LaunchTarget>(initial);
  const [pickedMode, setPickedMode] = useState<GameLaunchMode>(mode);
  const [saves, setSaves] = useState<SaveSummary[] | null>(null);
  const [diffs, setDiffs] = useState<Record<string, SessionDiff>>({});
  const [servers, setServers] = useState<SavedServer[] | null>(null);

  useEffect(() => {
    void Promise.all([TauriService.savesOverview(paths), TauriService.saveSessionDiffs(paths), TauriService.savedServers(paths).catch(() => [] as SavedServer[])]).then(
      ([overview, sessions, list]) => {
        setSaves(overview?.saves ?? []);
        setDiffs(sessions);
        setServers(list);
      }
    );
  }, [paths]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && onCancel();
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onCancel]);

  /** Single-player saves, the active profile's first, then the newest. */
  const ownSaves = useMemo(() => {
    const list = (saves ?? []).filter((s) => s.mode !== 'Multiplayer');
    const mine = (s: SaveSummary) => (activeProfileId && diffs[s.id]?.profile === activeProfileId ? 0 : 1);
    return [...list].sort((a, b) => mine(a) - mine(b) || b.modified - a.modified);
  }, [saves, diffs, activeProfileId]);
  const sortedServers = useMemo(
    () => [...(servers ?? [])].sort((a, b) => Number(b.starred) - Number(a.starred) || (b.last_joined ?? 0) - (a.last_joined ?? 0)),
    [servers]
  );

  const isPicked = (target: LaunchTarget) =>
    (target.kind === 'save' && picked.kind === 'save' && picked.id === target.id) ||
    (target.kind === 'server' && picked.kind === 'server' && picked.address === target.address);

  const row = (key: string, target: LaunchTarget, title: string, sub: string, chip?: string) => (
    <button
      key={key}
      type="button"
      onClick={() => setPicked(target)}
      onDoubleClick={() => onConfirm(target, pickedMode, true)}
      className={`w-full text-left px-3 py-2 rounded-lg border flex items-center gap-2 cursor-pointer ${
        isPicked(target) ? 'border-emerald-600 bg-emerald-950/40' : 'border-slate-800 hover:bg-slate-800/60'
      }`}
    >
      <div className="flex-1 min-w-0">
        <div className="text-xs font-semibold text-slate-100 truncate">{title}</div>
        <div className="text-[10px] text-slate-500 truncate">{sub}</div>
      </div>
      {chip && <span className="shrink-0 text-[10px] px-1.5 py-0.5 rounded border border-sky-700/60 text-sky-300">{chip}</span>}
    </button>
  );

  const loading = saves === null || servers === null;
  const canStart = picked.kind !== 'menu';

  return (
    <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/60" onMouseDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div role="dialog" aria-label={t('launch.title')} className="w-[560px] max-w-[calc(100vw-2rem)] max-h-[calc(100vh-4rem)] flex flex-col rounded-xl border border-slate-700 bg-slate-950 shadow-2xl">
        <div className="flex items-center justify-between px-4 py-3 border-b border-slate-800">
          <div>
            <h2 className="text-sm font-bold text-slate-100">{t('launch.title')}</h2>
            <p className="text-[11px] text-slate-400">{t('launch.blurb')}</p>
          </div>
          <button type="button" onClick={onCancel} className="p-1 rounded hover:bg-slate-800 text-slate-400 cursor-pointer" aria-label={t('launch.cancel')}>
            <X className="w-4 h-4" aria-hidden="true" />
          </button>
        </div>

        <div className="flex gap-1 px-4 pt-3">
          {(['save', 'server'] as const).map((id) => (
            <button
              key={id}
              type="button"
              onClick={() => setTab(id)}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-semibold cursor-pointer ${tab === id ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'}`}
            >
              {id === 'save' ? <Archive className="w-3.5 h-3.5" aria-hidden="true" /> : <Server className="w-3.5 h-3.5" aria-hidden="true" />}
              {t(`launch.tab.${id}`)}
            </button>
          ))}
        </div>

        <div className="flex-1 overflow-y-auto px-4 py-3 flex flex-col gap-1.5 min-h-[200px]">
          {loading && (
            <div className="flex items-center gap-2 text-xs text-slate-400">
              <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> {t('launch.loading')}
            </div>
          )}
          {!loading && tab === 'save' && (ownSaves.length === 0
            ? <p className="text-xs text-slate-400">{t('launch.noSaves')}</p>
            : ownSaves.map((s) => {
                const profile = diffs[s.id]?.profile;
                return row(
                  s.id,
                  { kind: 'save', id: s.id, label: s.world },
                  s.world,
                  t('launch.saveLine', { mode: s.mode, when: when(s.modified) }),
                  profile && profile === activeProfileId ? t('launch.thisProfile') : undefined
                );
              }))}
          {!loading && tab === 'server' && (sortedServers.length === 0
            ? <p className="text-xs text-slate-400">{t('launch.noServers')}</p>
            : sortedServers.map((s) =>
                row(
                  s.address,
                  { kind: 'server', address: s.address, name: s.name },
                  s.name,
                  s.last_joined ? t('launch.serverLine', { address: s.address, when: when(s.last_joined) }) : s.address,
                  s.starred ? '★' : undefined
                )
              ))}
          {!loading && tab === 'save' && <p className="text-[10px] text-slate-500 mt-1">{t('launch.saveHow')}</p>}
          {!loading && tab === 'server' && <p className="text-[10px] text-slate-500 mt-1">{t('launch.serverHow')}</p>}
        </div>

        <div className="px-4 py-2 border-t border-slate-800 flex flex-wrap items-center gap-1.5">
          <span className="text-[11px] text-slate-400 mr-1">{t('launch.how')}</span>
          {modes.map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => setPickedMode(m)}
              className={`px-2 py-1 rounded-md border text-[11px] cursor-pointer ${pickedMode === m ? 'border-emerald-600 text-emerald-200 bg-emerald-950/40' : 'border-slate-700 text-slate-300 hover:bg-slate-800'}`}
            >
              {t(`header.modes.${m}.title`)}
            </button>
          ))}
        </div>

        <div className="px-4 py-3 border-t border-slate-800 flex items-center gap-2">
          <button
            type="button"
            onClick={() => onConfirm({ kind: 'menu' }, pickedMode, false)}
            className="text-[11px] text-slate-400 hover:text-slate-200 underline cursor-pointer"
          >
            {t('launch.backToMenu')}
          </button>
          <span className="flex-1" />
          <button type="button" onClick={onCancel} className="px-3 py-1.5 rounded-md text-xs text-slate-300 hover:bg-slate-800 cursor-pointer">
            {t('launch.cancel')}
          </button>
          <button
            type="button"
            disabled={!canStart}
            onClick={() => onConfirm(picked, pickedMode, false)}
            className="px-3 py-1.5 rounded-md text-xs border border-emerald-700 text-emerald-200 hover:bg-emerald-950/50 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {t('launch.keep')}
          </button>
          <button
            type="button"
            disabled={!canStart}
            onClick={() => onConfirm(picked, pickedMode, true)}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-bold bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <Play className="w-3.5 h-3.5 fill-current" aria-hidden="true" /> {t('launch.start')}
          </button>
        </div>
      </div>
    </div>
  );
};
