import React, { useCallback, useEffect, useState } from 'react';
import { Loader2, MemoryStick } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import i18next from 'i18next';
import { useChangeRevision } from '../../services/changes';
import { GameMemory } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';

const GB = 1024;
const gb = (mb: number) => {
  const v = mb / GB;
  return Number.isInteger(v) ? String(v) : v.toFixed(1);
};

/**
 * The memory the game's Java may use (-Xmx in ProjectZomboid64.json): what it has, the PC's,
 * what the Studio suggests for the active list, and the out-of-memory errors of the last
 * sessions. Changed from the game's next start, with the game closed.
 */
export const MemoryPanel: React.FC<{ paths: StudioPathsUI }> = ({ paths }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision(['mods', 'profiles']);
  const [mem, setMem] = useState<GameMemory | null>(null);
  const [pick, setPick] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);

  const load = useCallback(async () => {
    const next = await TauriService.gameMemory(paths);
    setMem(next);
    setPick((p) => p ?? next?.current_mb ?? next?.recommended_mb ?? null);
  }, [paths]);

  useEffect(() => {
    void load();
  }, [load, changeRev]);

  if (!mem || !mem.file) return null;

  // Whole gigabytes from 2 GB to the most offered.
  const choices: number[] = [];
  for (let mb = 2 * GB; mb <= mem.max_mb; mb += GB) choices.push(mb);
  if (mem.current_mb && !choices.includes(mem.current_mb)) choices.push(mem.current_mb);
  choices.sort((a, b) => a - b);

  const apply = async () => {
    if (pick === null) return;
    setBusy(true);
    setNotice(null);
    try {
      const next = await TauriService.setGameMemory(paths, pick);
      setMem(next);
      setNotice({ ok: true, text: t('gameMemory.done', { gb: gb(pick), name: next.profile ?? '' }) });
    } catch (err) {
      setNotice({ ok: false, text: String(err) });
    } finally {
      setBusy(false);
    }
  };

  const current = mem.current_mb;
  const ranOut = mem.out_of_memory.length > 0;
  const updateUndid = mem.chosen_mb !== null && current !== null && mem.chosen_mb !== current;

  return (
    <section className="flex flex-col gap-3 p-5 rounded-xl border border-slate-800 bg-slate-900/60">
      <div className="flex items-start gap-3">
        <MemoryStick className="w-5 h-5 text-sky-400 mt-0.5" aria-hidden="true" />
        <div>
          <h2 className="text-sm font-bold text-slate-100">{t('gameMemory.title')}</h2>
          <p className="text-xs text-slate-400 max-w-2xl">{t('gameMemory.blurb')}</p>
        </div>
      </div>

      <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-xs">
        <span className="text-lg font-bold text-slate-100">
          {current !== null ? t('gameMemory.current', { gb: gb(current) }) : t('gameMemory.noValue')}
        </span>
        {mem.total_mb !== null && <span className="text-slate-400">{t('gameMemory.pc', { gb: gb(mem.total_mb) })}</span>}
        <span className={current !== null && current < mem.recommended_mb ? 'text-amber-300' : 'text-slate-400'}>
          {t('gameMemory.recommended', { gb: gb(mem.recommended_mb), count: mem.active_mods })}
        </span>
      </div>

      {mem.profile && (
        <div className="text-xs text-slate-400">
          {mem.profile_has_own ? t('gameMemory.profileOwn', { name: mem.profile }) : t('gameMemory.profileNone', { name: mem.profile })}
        </div>
      )}

      {ranOut && (
        <div className="text-xs rounded-lg border border-red-800/60 bg-red-950/30 text-red-200 px-3 py-2">
          {t('gameMemory.ranOut', { count: mem.out_of_memory.length, logs: mem.out_of_memory.slice(0, 3).join(', ') })}
        </div>
      )}
      {mem.windows_low_memory.length > 0 && (
        /* Windows itself ran out: told as a Monitor card when it happens; here only the log. */
        <div className="text-[11px] text-slate-500" title={t('gameMemory.windowsAdvice')}>
          {t('gameMemory.windowsLog', { count: mem.windows_low_memory.length })}{' '}
          {mem.windows_low_memory
            .slice(0, 3)
            .map((e) => `${new Date(e.at * 1000).toLocaleString(i18next.language, { dateStyle: 'medium', timeStyle: 'short' })} (${t('gameMemory.windowsLogGame', { game: e.game_gb !== null ? e.game_gb.toFixed(1) : '?', charge: e.commit_charge_gb.toFixed(0), limit: e.commit_limit_gb.toFixed(0) })})`)
            .join(' · ')}
        </div>
      )}
      {updateUndid && (
        <div className="text-xs rounded-lg border border-amber-800/60 bg-amber-950/30 text-amber-200 px-3 py-2">
          {t('gameMemory.updateUndid', { chosen: gb(mem.chosen_mb as number), now: gb(current as number) })}
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2 text-xs">
        <label htmlFor="game-memory" className="text-slate-300">{t('gameMemory.give')}</label>
        <select
          id="game-memory"
          value={pick ?? ''}
          onChange={(e) => setPick(Number(e.target.value))}
          className="bg-slate-950 border border-slate-700 rounded-md px-2 py-1.5 text-slate-100"
        >
          {choices.map((mb) => (
            <option key={mb} value={mb}>
              {gb(mb)} GB{mb === mem.recommended_mb ? ` (${t('gameMemory.suggested')})` : ''}
              {mb === current ? ` · ${t('gameMemory.now')}` : ''}
            </option>
          ))}
        </select>
        <button
          type="button"
          onClick={() => void apply()}
          disabled={busy || pick === null || pick === current || mem.game_running}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-bold bg-sky-600 hover:bg-sky-500 text-white cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {busy && <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />}
          {t('gameMemory.apply')}
        </button>
        {mem.game_running && <span className="text-amber-300">{t('gameMemory.closeGame')}</span>}
      </div>

      {notice && (
        <div className={`text-xs rounded-md border px-3 py-2 ${notice.ok ? 'border-emerald-700/60 bg-emerald-950/40 text-emerald-200' : 'border-red-700/60 bg-red-950/40 text-red-200'}`}>{notice.text}</div>
      )}

      <p className="text-[11px] text-slate-500 max-w-3xl">{t('gameMemory.how', { max: gb(mem.max_mb) })}</p>
    </section>
  );
};
