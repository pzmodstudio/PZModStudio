import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowRight, Gamepad2, HeartPulse, Info, Loader2, Server } from 'lucide-react';
import { BootTimes, GameMemory, HealthReport, MpTestProcess, MpTestStatus } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { ResourceMeter } from '../common/ResourceMeter';
import { ProcessCard } from '../server/MpTestPanel';
import { issueStateOf } from '../../utils/issueState';

export type DiagSection = 'mods' | 'game' | 'server';

/**
 * Every game and server process on this PC (the same reading as the multiplayer test's cards),
 * asked again every `everyMs` while the page shows it. null until the first answer.
 */
export function useProcesses(paths: StudioPathsUI, everyMs = 2000): MpTestStatus | null {
  const [status, setStatus] = useState<MpTestStatus | null>(null);
  useEffect(() => {
    let cancelled = false;
    const read = () =>
      TauriService.mpTestStatus(paths)
        .then((s) => !cancelled && setStatus(s))
        .catch(() => undefined);
    void read();
    const id = window.setInterval(read, everyMs);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [paths, everyMs]);
  return status;
}

/** The game's facts that do not change while it runs: its memory limit, out-of-memory sessions, last start. */
function useGameFacts(paths: StudioPathsUI) {
  const [memory, setMemory] = useState<GameMemory | null>(null);
  const [boots, setBoots] = useState<BootTimes[] | null>(null);
  useEffect(() => {
    let cancelled = false;
    void TauriService.gameMemory(paths).then((m) => !cancelled && setMemory(m));
    void TauriService.bootHistory(paths).then((b) => !cancelled && setBoots(b)).catch(() => !cancelled && setBoots([]));
    return () => {
      cancelled = true;
    };
  }, [paths]);
  return { memory, boots };
}

const secs = (ms: number) => `${(ms / 1000).toFixed(0)} s`;
const gb = (mb: number) => `${(mb / 1024).toFixed(1)} GB`;

/** What the game screen a process is on says in one word. */
const gameWhere = (p: MpTestProcess, t: (k: string, o?: Record<string, unknown>) => string) =>
  p.screen && p.screen !== 'launching' && p.screen !== 'menu' ? t(`servers.mpTest.screen.${p.screen}`, { defaultValue: p.screen }) : t(`servers.mpTest.state.${p.state}`, { defaultValue: p.state });

/**
 * Diagnóstico's front page: one card per part (mods, the game, the server), each with how it is in
 * a line and a way in, and under them the Doctor, "what is wrong?", step by step.
 */
export const DiagnosticsHome: React.FC<{
  paths: StudioPathsUI;
  report: HealthReport | null;
  checking: boolean;
  onOpen: (section: DiagSection) => void;
  doctor?: React.ReactNode;
}> = ({ paths, report, checking, onOpen, doctor }) => {
  const { t } = useTranslation();
  const procs = useProcesses(paths, 4000);
  const { memory, boots } = useGameFacts(paths);
  const games = procs?.processes.filter((p) => p.role === 'player') ?? [];
  const servers = procs?.processes.filter((p) => p.role === 'server') ?? [];

  const act = report?.issues.filter((i) => issueStateOf(i) === 'act').length ?? 0;
  const recommended = report?.issues.filter((i) => issueStateOf(i) === 'recommended').length ?? 0;
  const modsLine = !report
    ? checking
      ? t('diag.home.mods.checking')
      : t('diag.home.mods.notYet')
    : act > 0
      ? t('diag.home.mods.act', { count: act })
      : recommended > 0
        ? t('diag.home.mods.recommended', { count: recommended })
        : t('diag.home.mods.ok', { count: report.active_count });
  const modsTone = act > 0 ? 'red' : recommended > 0 ? 'amber' : report ? 'green' : 'slate';

  const oom = memory?.out_of_memory.length ?? 0;
  const lastBoot = boots?.[0];
  const gameLine = games.length > 0 ? t('diag.home.game.running', { where: gameWhere(games[0], t), ram: gb(games[0].ram_mb) }) : t('diag.home.game.closed');
  const gameSub = [
    lastBoot ? t('diag.home.game.lastBoot', { time: secs(lastBoot.total_ms) }) : null,
    oom > 0 ? t('diag.home.game.oom', { count: oom }) : null,
    memory?.current_mb ? t('diag.home.game.limit', { gb: gb(memory.current_mb) }) : null,
  ]
    .filter(Boolean)
    .join(' · ');
  const gameTone = oom > 0 ? 'amber' : games.length > 0 ? 'green' : 'slate';

  const serverLine =
    servers.length > 0
      ? t('diag.home.server.running', { name: servers[0].name, state: t(`servers.mpTest.state.${servers[0].state}`, { defaultValue: servers[0].state }), ram: gb(servers[0].ram_mb) })
      : t('diag.home.server.off');
  const serverTone = servers.length > 0 ? (servers[0].errors > 0 ? 'amber' : 'green') : 'slate';

  const TONE: Record<string, string> = {
    red: 'border-red-700/60 bg-red-950/20',
    amber: 'border-amber-700/50 bg-amber-950/15',
    green: 'border-emerald-800/60 bg-emerald-950/15',
    slate: 'border-slate-800 bg-slate-900/60',
  };
  const DOT: Record<string, string> = { red: 'bg-red-400', amber: 'bg-amber-400', green: 'bg-emerald-400', slate: 'bg-slate-500' };
  const card = (section: DiagSection, Icon: React.ElementType, line: string, sub: string, tone: string) => (
    <button
      key={section}
      type="button"
      onClick={() => onOpen(section)}
      className={`group text-left rounded-xl border p-4 flex flex-col gap-2 cursor-pointer hover:brightness-125 transition ${TONE[tone]}`}
    >
      <div className="flex items-center gap-2">
        <Icon className="w-4 h-4 text-slate-300" aria-hidden="true" />
        <span className="text-sm font-bold text-slate-100">{t(`diag.sections.${section}`)}</span>
        <span className={`ml-auto w-2 h-2 rounded-full ${DOT[tone]}`} />
      </div>
      <div className="text-[12.5px] text-slate-200 leading-snug">{line}</div>
      {sub && <div className="text-[11px] text-slate-400 leading-snug">{sub}</div>}
      <div className="mt-auto flex items-center gap-1 text-[11px] text-sky-300 group-hover:text-sky-200">
        {t('diag.home.see')} <ArrowRight className="w-3 h-3" aria-hidden="true" />
      </div>
    </button>
  );

  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-3 md:grid-cols-3">
        {card('mods', HeartPulse, modsLine, t('diag.home.mods.sub'), modsTone)}
        {card('game', Gamepad2, gameLine, gameSub, gameTone)}
        {card('server', Server, serverLine, servers.length > 0 ? t('diag.home.server.sub', { errors: servers[0].errors }) : t('diag.home.server.offSub'), serverTone)}
      </div>
      {doctor}
    </div>
  );
};

/**
 * The game as it runs now: the PC's memory meter, and a card per game open (memory against its
 * Java limit, CPU, the screen it is on, what its log says). Closed: what is known from last time.
 */
export const GameLivePanel: React.FC<{ paths: StudioPathsUI }> = ({ paths }) => {
  const { t } = useTranslation();
  const procs = useProcesses(paths);
  const { memory, boots } = useGameFacts(paths);
  const games = procs?.processes.filter((p) => p.role === 'player') ?? [];
  return (
    <div className="flex flex-col gap-3">
      <ResourceMeter paths={paths} />
      {!procs && (
        <span className="flex items-center gap-1.5 text-[11px] text-slate-500">
          <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> {t('diag.live.reading')}
        </span>
      )}
      {procs && games.length === 0 && (
        <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-4 flex flex-col gap-2 text-xs">
          <div className="font-bold text-slate-200">{t('diag.live.gameClosed')}</div>
          <div className="text-slate-400">{t('diag.live.gameClosedHint')}</div>
          <dl className="grid grid-cols-[180px_1fr] gap-x-3 gap-y-1 text-[11.5px] mt-1">
            {boots?.[0] && (
              <>
                <dt className="text-slate-500">{t('diag.live.lastBoot')}</dt>
                <dd className="text-slate-200">{t('diag.live.lastBootValue', { time: secs(boots[0].total_ms), mods: boots[0].mods })}</dd>
              </>
            )}
            {memory?.current_mb != null && (
              <>
                <dt className="text-slate-500">{t('diag.live.limit')}</dt>
                <dd className="text-slate-200">{gb(memory.current_mb)}</dd>
              </>
            )}
            {memory && (
              <>
                <dt className="text-slate-500">{t('diag.live.oom')}</dt>
                <dd className={memory.out_of_memory.length > 0 ? 'text-amber-300 font-bold' : 'text-slate-200'}>
                  {memory.out_of_memory.length > 0 ? t('diag.live.oomCount', { count: memory.out_of_memory.length }) : t('diag.live.none')}
                </dd>
                <dt className="text-slate-500">{t('diag.live.windowsLow')}</dt>
                <dd className={memory.windows_low_memory.length > 0 ? 'text-amber-300 font-bold' : 'text-slate-200'}>
                  {memory.windows_low_memory.length > 0 ? t('diag.live.windowsLowCount', { count: memory.windows_low_memory.length }) : t('diag.live.none')}
                </dd>
              </>
            )}
          </dl>
        </div>
      )}
      {games.length > 0 && (
        <div className="grid gap-3 lg:grid-cols-2">
          {games.map((p) => (
            <ProcessCard key={p.pid} paths={paths} p={p} testPlayer={false} />
          ))}
        </div>
      )}
    </div>
  );
};

/**
 * The server as it runs now on this PC: a card per server process (memory, CPU, started or not,
 * what its log says) and, during a multiplayer test, its games. A server joined elsewhere cannot be
 * measured from here, and the page says so.
 */
export const ServerLivePanel: React.FC<{ paths: StudioPathsUI; onGoServers?: () => void }> = ({ paths, onGoServers }) => {
  const { t } = useTranslation();
  const procs = useProcesses(paths);
  const servers = procs?.processes.filter((p) => p.role === 'server') ?? [];
  const testRunning = !!procs?.phase && procs.phase !== 'stopped' && servers.length > 0;
  const testGames = testRunning ? procs!.processes.filter((p) => p.role === 'player') : [];
  return (
    <div className="flex flex-col gap-3">
      {!procs && (
        <span className="flex items-center gap-1.5 text-[11px] text-slate-500">
          <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> {t('diag.live.reading')}
        </span>
      )}
      {procs && servers.length === 0 && (
        <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-4 flex flex-col gap-2 text-xs">
          <div className="font-bold text-slate-200">{t('diag.server.none')}</div>
          <div className="text-slate-400">{t('diag.server.noneHint')}</div>
          {onGoServers && (
            <button type="button" onClick={onGoServers} className="self-start mt-1 flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer">
              <Server className="w-3.5 h-3.5" aria-hidden="true" /> {t('diag.server.goServers')}
            </button>
          )}
        </div>
      )}
      {servers.length > 0 && (
        <div className="grid gap-3 lg:grid-cols-2">
          {servers.map((p) => (
            <ProcessCard key={p.pid} paths={paths} p={p} />
          ))}
        </div>
      )}
      {testGames.length > 0 && (
        <>
          <div className="text-[11px] font-bold uppercase tracking-wider text-slate-400 mt-1">{t('diag.server.testGames')}</div>
          <div className="grid gap-3 lg:grid-cols-2">
            {testGames.map((p) => (
              <ProcessCard key={p.pid} paths={paths} p={p} />
            ))}
          </div>
        </>
      )}
      <div className="flex items-start gap-2 text-[11px] text-slate-500">
        <Info className="w-3.5 h-3.5 mt-0.5 shrink-0" aria-hidden="true" />
        {t('diag.server.remoteNote')}
      </div>
    </div>
  );
};
