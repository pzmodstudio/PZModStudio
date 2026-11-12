import React, { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Cpu, FileText, FolderOpen, Loader2, MemoryStick, Play, RotateCcw, Server, Square, User, Users, Wand2 } from 'lucide-react';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, reportBackendError } from '../../services/tauri';
import { MpAutomation, MpSecondCheck, MpTestPrepareReport, MpTestProcess, MpTestStatus } from '../../types';

const AUTO_KEY = 'pz_mp_test_automation';
const loadAutomation = (): MpAutomation => {
  try {
    const saved = JSON.parse(localStorage.getItem(AUTO_KEY) ?? 'null') as MpAutomation | null;
    if (saved && typeof saved.login === 'boolean') return { ...saved, spawn: saved.spawn ?? true, admin_all: saved.admin_all ?? true };
  } catch {
    // the defaults
  }
  return { login: true, start_click: true, character: 'random', spawn: true, admin_all: true };
};

interface MpTestPanelProps {
  paths: StudioPathsUI;
  /** The server config the test runs (Zomboid/Server/<server>.ini). */
  server: string;
}

/** How a process is doing, in words and colour. */
const STATE_STYLE: Record<string, string> = {
  starting: 'border-amber-600/60 text-amber-200 bg-amber-950/40',
  started: 'border-emerald-600/60 text-emerald-200 bg-emerald-950/40',
  menu: 'border-slate-600 text-slate-300 bg-slate-900',
  connecting: 'border-amber-600/60 text-amber-200 bg-amber-950/40',
  in_game: 'border-emerald-600/60 text-emerald-200 bg-emerald-950/40',
};

const gb = (mb: number) => (mb / 1024).toFixed(1);
/** Seconds as m:ss (or h:mm:ss). */
const since = (s: number) => {
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(Math.floor(s % 60)).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${m}:${ss}`;
};

/**
 * A multiplayer test on this PC: the server without Steam and one or two games joined to it,
 * the second with a Zomboid folder of its own, and a card per process with what it uses, where
 * its files are and how far it got.
 */
export const MpTestPanel: React.FC<MpTestPanelProps> = ({ paths, server }) => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<MpTestStatus | null>(null);
  const [players, setPlayers] = useState(2);
  const [memoryGb, setMemoryGb] = useState(4);
  const [busy, setBusy] = useState<'prepare' | 'start' | 'stop' | null>(null);
  const [prepared, setPrepared] = useState<MpTestPrepareReport | null>(null);
  /** Whether player 2's folder is up to date for this server (checked on opening and after getting it ready). */
  const [secondCheck, setSecondCheck] = useState<MpSecondCheck | null>(null);
  const [restarting, setRestarting] = useState<number | null>(null);
  const checkSecond = useCallback(async () => {
    try {
      setSecondCheck(await TauriService.mpTestSecondCheck(paths, server));
    } catch {
      setSecondCheck(null);
    }
  }, [paths, server]);
  useEffect(() => {
    void checkSecond();
  }, [checkSecond]);
  const restart = async (slot: number) => {
    setRestarting(slot);
    try {
      await TauriService.mpTestRestart(paths, slot);
      void refresh();
    } catch (err) {
      reportBackendError('Starting the game again', err);
    } finally {
      setRestarting(null);
    }
  };
  /** What the test does by itself; kept for next time. */
  const [automation, setAutomationState] = useState<MpAutomation>(loadAutomation);
  const setAutomation = (next: MpAutomation) => {
    setAutomationState(next);
    try {
      localStorage.setItem(AUTO_KEY, JSON.stringify(next));
    } catch {
      // only a convenience
    }
  };

  const refresh = useCallback(async () => {
    try {
      setStatus(await TauriService.mpTestStatus(paths));
    } catch {
      // the next poll tries again
    }
  }, [paths]);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(), 5000);
    return () => window.clearInterval(id);
  }, [refresh]);

  const prepare = async () => {
    setBusy('prepare');
    try {
      setPrepared(await TauriService.mpTestPrepare(paths, server));
      void refresh();
      void checkSecond();
    } catch (err) {
      reportBackendError('Getting player 2 ready', err);
    } finally {
      setBusy(null);
    }
  };

  const start = async () => {
    setBusy('start');
    try {
      await TauriService.mpTestStart(paths, server, memoryGb, players, automation);
      void refresh();
    } catch (err) {
      reportBackendError('Starting the multiplayer test', err);
    } finally {
      setBusy(null);
    }
  };

  const stop = async () => {
    if (!confirm(t('servers.mpTest.stopConfirm'))) return;
    setBusy('stop');
    try {
      await TauriService.mpTestStop(paths);
      void refresh();
    } catch (err) {
      reportBackendError('Stopping the multiplayer test', err);
    } finally {
      setBusy(null);
    }
  };

  const open = (pid: number, log: boolean) => void TauriService.mpTestOpen(paths, pid, log).catch((err) => reportBackendError('Opening', err));

  const running = (status?.processes.length ?? 0) > 0;
  const needMb = memoryGb * 1024 + players * (status?.game_heap_mb ?? 6144);
  const short = status && !running && status.pc_free_mb > 0 && needMb > status.pc_free_mb;
  const needsPrepare = players === 2 && status && !status.second_dir_ready;

  const card = (p: MpTestProcess) => {
    // The process holds more than its Java heap (the JVM's own memory, native code), so the bar is
    // measured against the heap plus a quarter: red only when it really runs out.
    const pct = p.ram_max_mb ? Math.min(100, Math.round((p.ram_mb / (p.ram_max_mb * 1.25)) * 100)) : null;
    const Icon = p.role === 'server' ? Server : User;
    return (
      <div key={p.pid} className="rounded-xl border border-slate-800 bg-slate-900/60 p-3 flex flex-col gap-2">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0">
            <Icon className="w-4 h-4 text-purple-300 shrink-0" aria-hidden="true" />
            <span className="text-sm font-bold text-slate-100 truncate">
              {p.role === 'server' ? t('servers.mpTest.server', { name: p.name }) : t('servers.mpTest.player', { n: p.name })}
            </span>
            <span className="text-[10px] font-mono text-slate-500">PID {p.pid}</span>
          </div>
          <span className={`text-[10px] font-bold px-2 py-0.5 rounded border ${STATE_STYLE[p.screen === 'in_game' ? 'in_game' : p.state] ?? STATE_STYLE.menu}`}>
            {p.screen && p.screen !== 'launching' && p.screen !== 'menu' ? t(`servers.mpTest.screen.${p.screen}`, { defaultValue: p.screen }) : t(`servers.mpTest.state.${p.state}`, { defaultValue: p.state })}
          </span>
        </div>
        <div className="grid grid-cols-2 gap-2 text-[11px] text-slate-300">
          <div className="flex items-center gap-1.5">
            <MemoryStick className="w-3.5 h-3.5 text-slate-400" aria-hidden="true" />
            {p.ram_max_mb ? t('servers.mpTest.ramOf', { used: gb(p.ram_mb), max: gb(p.ram_max_mb) }) : t('servers.mpTest.ram', { used: gb(p.ram_mb) })}
          </div>
          <div className="flex items-center gap-1.5">
            <Cpu className="w-3.5 h-3.5 text-slate-400" aria-hidden="true" />
            {t('servers.mpTest.cpu', { pct: p.cpu_percent.toFixed(0) })}
          </div>
        </div>
        {pct !== null && (
          <div className="h-1.5 rounded bg-slate-800 overflow-hidden">
            <div className={`h-full ${pct > 90 ? 'bg-red-500' : pct > 75 ? 'bg-amber-500' : 'bg-emerald-500'}`} style={{ width: `${pct}%` }} />
          </div>
        )}
        <div className="rounded border border-slate-800 bg-slate-950/60 px-2 py-1 text-[11px]">
          <div className="text-slate-500">{t('servers.mpTest.nowDoing', { time: since(p.uptime_s) })}</div>
          <div className="font-mono text-slate-300 break-all line-clamp-2">{p.step ?? t('servers.mpTest.noLogYet')}</div>
        </div>
        <div className="text-[11px] text-slate-400 break-all">
          <span className="text-slate-500">{t('servers.mpTest.folder')}</span> {p.zomboid_dir}
        </div>
        {p.role === 'player' && <div className="text-[11px] text-amber-200/90">{t('servers.mpTest.usesServerList')}</div>}
        <div className="flex flex-wrap items-center gap-2">
          <button type="button" onClick={() => open(p.pid, false)} className="flex items-center gap-1 px-2 py-1 rounded text-[11px] bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 cursor-pointer">
            <FolderOpen className="w-3 h-3" aria-hidden="true" /> {t('servers.mpTest.openFolder')}
          </button>
          <button type="button" onClick={() => open(p.pid, true)} className="flex items-center gap-1 px-2 py-1 rounded text-[11px] bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 cursor-pointer">
            <FileText className="w-3 h-3" aria-hidden="true" /> {t('servers.mpTest.openLog')}
          </button>
          <span className={`text-[11px] ${p.errors > 0 ? 'text-red-300' : 'text-slate-500'}`}>{t('servers.mpTest.errors', { count: p.errors })}</span>
          {p.connect && <span className="text-[11px] font-mono text-slate-500">→ {p.connect}</span>}
          {p.role === 'player' && (p.name === '1' || p.name === '2') && (
            <button
              type="button"
              onClick={() => void restart(Number(p.name))}
              disabled={restarting !== null}
              title={t('servers.mpTest.restartTitle')}
              className="ml-auto flex items-center gap-1 px-2 py-1 rounded text-[11px] bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 cursor-pointer disabled:opacity-60"
            >
              {restarting === Number(p.name) ? <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" /> : <RotateCcw className="w-3 h-3" aria-hidden="true" />}
              {t('servers.mpTest.restart')}
            </button>
          )}
        </div>
      </div>
    );
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="rounded-xl border border-sky-800/60 bg-sky-950/20 p-3 text-xs text-slate-300 leading-relaxed flex flex-col gap-1">
        <div className="flex items-center gap-1.5 font-bold text-sky-200">
          <Users className="w-4 h-4" aria-hidden="true" /> {t('servers.mpTest.title')}
        </div>
        <div>{t('servers.mpTest.what')}</div>
        <div className="text-slate-400">{t('servers.mpTest.touches', { dir: status?.second_dir ?? 'Zomboid2' })}</div>
      </div>

      <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-3 flex flex-col gap-2 text-xs text-slate-300">
        <div className="font-bold text-slate-200">{t('servers.mpTest.auto.title')}</div>
        <label className="flex items-start gap-2 cursor-pointer">
          <input type="checkbox" checked={automation.login} disabled={running} onChange={(e) => setAutomation({ ...automation, login: e.target.checked })} className="mt-0.5" />
          <span>{t('servers.mpTest.auto.login')}</span>
        </label>
        <label className="flex items-start gap-2 cursor-pointer">
          <input type="checkbox" checked={automation.admin_all} disabled={running} onChange={(e) => setAutomation({ ...automation, admin_all: e.target.checked })} className="mt-0.5" />
          <span>{t('servers.mpTest.auto.adminAll')}</span>
        </label>
        <label className="flex items-start gap-2 cursor-pointer">
          <input type="checkbox" checked={automation.spawn} disabled={running} onChange={(e) => setAutomation({ ...automation, spawn: e.target.checked })} className="mt-0.5" />
          <span>{t('servers.mpTest.auto.spawn')}</span>
        </label>
        <label className="flex items-start gap-2 cursor-pointer">
          <input type="checkbox" checked={automation.character !== 'manual'} disabled={running} onChange={(e) => setAutomation({ ...automation, character: e.target.checked ? 'random' : 'manual' })} className="mt-0.5" />
          <span className="flex flex-wrap items-center gap-2">
            {t('servers.mpTest.auto.character')}
            {automation.character !== 'manual' && (
              <select value={automation.character} disabled={running} onChange={(e) => setAutomation({ ...automation, character: e.target.value })} className="px-1.5 py-0.5 rounded bg-slate-900 border border-slate-700 text-xs text-slate-100">
                <option value="random">{t('servers.mpTest.auto.random')}</option>
                <option value="first">{t('servers.mpTest.auto.first')}</option>
              </select>
            )}
          </span>
        </label>
        <label className="flex items-start gap-2 cursor-pointer">
          <input type="checkbox" checked={automation.start_click} disabled={running} onChange={(e) => setAutomation({ ...automation, start_click: e.target.checked })} className="mt-0.5" />
          <span>{t('servers.mpTest.auto.startClick')}</span>
        </label>
        <div className="text-[11px] text-slate-500">{t('servers.mpTest.auto.note')}</div>
      </div>

      <div className="flex flex-wrap items-end gap-3">
        <label className="flex flex-col gap-1 text-[11px] text-slate-400">
          {t('servers.mpTest.players')}
          <select value={players} onChange={(e) => setPlayers(Number(e.target.value))} disabled={running} className="px-2 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-sm text-slate-100">
            <option value={1}>1</option>
            <option value={2}>2</option>
          </select>
        </label>
        <label className="flex flex-col gap-1 text-[11px] text-slate-400">
          {t('servers.mpTest.serverMemory')}
          <select value={memoryGb} onChange={(e) => setMemoryGb(Number(e.target.value))} disabled={running} className="px-2 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-sm text-slate-100">
            {[2, 3, 4, 6, 8, 12].map((g) => (
              <option key={g} value={g}>{g} GB</option>
            ))}
          </select>
        </label>
        {players === 2 && (
          <button type="button" onClick={() => void prepare()} disabled={busy !== null || running} className="flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-100 border border-slate-700 cursor-pointer disabled:opacity-60">
            {busy === 'prepare' ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Wand2 className="w-3.5 h-3.5" aria-hidden="true" />}
            {secondCheck?.ready
              ? t('servers.mpTest.secondReady')
              : secondCheck && status?.second_dir_ready
              ? t('servers.mpTest.secondOutdated', { count: secondCheck.missing.length + secondCheck.outdated.length + (secondCheck.link_outdated ? 1 : 0) })
              : t('servers.mpTest.prepare')}
          </button>
        )}
        {!running ? (
          <button type="button" onClick={() => void start()} disabled={busy !== null || !!needsPrepare} title={needsPrepare ? t('servers.mpTest.prepareFirst') : undefined} className="flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-bold bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer disabled:opacity-60">
            {busy === 'start' ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Play className="w-3.5 h-3.5" aria-hidden="true" />}
            {t('servers.mpTest.start', { count: players })}
          </button>
        ) : (
          <button type="button" onClick={() => void stop()} disabled={busy !== null} className="flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-bold bg-red-700 hover:bg-red-600 text-white cursor-pointer disabled:opacity-60">
            {busy === 'stop' ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Square className="w-3.5 h-3.5" aria-hidden="true" />}
            {t('servers.mpTest.stop')}
          </button>
        )}
      </div>

      {status && (
        <div className={`text-[11px] ${short ? 'text-red-300' : 'text-slate-400'}`}>
          {t('servers.mpTest.pcMemory', { free: gb(status.pc_free_mb), total: gb(status.pc_total_mb) })}
          {!running && ' ' + t('servers.mpTest.needs', { need: gb(needMb) })}
          {short && ' ' + t('servers.mpTest.notEnough')}
        </div>
      )}
      {needsPrepare && <div className="text-[11px] text-amber-200">{t('servers.mpTest.prepareFirst')}</div>}
      {prepared && (
        <div className="rounded-lg border border-emerald-800/60 bg-emerald-950/20 p-3 text-[11px] text-emerald-100">
          {t('servers.mpTest.prepared', { copied: prepared.copied.length, linked: prepared.linked.length, kept: prepared.kept.length })}
          {prepared.missing.length > 0 && <div className="text-amber-200 mt-1">{t('servers.mpTest.missing', { mods: prepared.missing.join(', ') })}</div>}
        </div>
      )}
      {status?.phase && status.phase !== 'stopped' && (
        <div className="text-[11px] text-sky-200">{t(`servers.mpTest.phase.${status.phase.split(':')[0]}`, { defaultValue: status.phase })}</div>
      )}

      {running ? (
        <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">{status!.processes.map(card)}</div>
      ) : (
        <div className="text-xs text-slate-500">{t('servers.mpTest.nothingRunning')}</div>
      )}
    </div>
  );
};
