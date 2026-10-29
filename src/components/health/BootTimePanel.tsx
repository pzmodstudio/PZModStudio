import React, { useEffect, useMemo, useState } from 'react';
import { Timer } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import { BootTimes } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';

/** The stages in the order the game runs them, with their bar colour. */
const PHASES: { id: string; color: string }[] = [
  { id: 'start', color: 'bg-slate-500' },
  { id: 'mods', color: 'bg-sky-500' },
  { id: 'translations', color: 'bg-cyan-400' },
  { id: 'scripts', color: 'bg-teal-500' },
  { id: 'clothing', color: 'bg-emerald-500' },
  { id: 'lua', color: 'bg-indigo-500' },
  { id: 'textures', color: 'bg-violet-400' },
  { id: 'models', color: 'bg-amber-500' },
  { id: 'menu', color: 'bg-rose-400' },
];
/** A stage this much slower than usual (and by at least SLOWER_MS) is pointed out. */
const SLOWER_RATIO = 1.5;
const SLOWER_MS = 3000;

const seconds = (ms: number) => (ms / 1000).toFixed(ms < 10000 ? 1 : 0);
const median = (values: number[]) => {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
};

/**
 * How long the game took to reach the main menu, stage by stage, from the DebugLog every launch
 * leaves (Zomboid/Logs). A start is set beside the usual one: the median of the other starts with
 * about as many mods, so one slow start (the disk busy, the PC doing something else) does not
 * move what "usual" means.
 */
export const BootTimePanel: React.FC<{ paths: StudioPathsUI }> = ({ paths }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision(['mods']);
  const [boots, setBoots] = useState<BootTimes[] | null>(null);
  const [picked, setPicked] = useState(0);

  useEffect(() => {
    void TauriService.bootHistory(paths).then(setBoots);
  }, [paths, changeRev]);

  const shown = boots?.[picked] ?? null;
  // The other starts with about as many mods (within 10 %).
  const peers = useMemo(() => {
    if (!boots || !shown) return [];
    return boots.filter((b, i) => i !== picked && Math.abs(b.mods - shown.mods) <= Math.max(2, shown.mods * 0.1));
  }, [boots, shown, picked]);
  const usual = (id: string) => median(peers.map((b) => b.phases.find((p) => p.id === id)?.millis ?? 0));
  const usualTotal = median(peers.map((b) => b.total_ms));
  const comparable = peers.length >= 2;

  if (!boots) return null;
  return (
    <section className="flex flex-col gap-3 p-5 rounded-xl border border-slate-800 bg-slate-900/60">
      <div className="flex items-start gap-3">
        <Timer className="w-5 h-5 text-sky-400 mt-0.5" aria-hidden="true" />
        <div>
          <h2 className="text-sm font-bold text-slate-100">{t('bootTime.title')}</h2>
          <p className="text-xs text-slate-400 max-w-2xl">{t('bootTime.blurb')}</p>
        </div>
      </div>

      {!shown ? (
        <div className="text-xs text-slate-400">{t('bootTime.none')}</div>
      ) : (
        <>
          <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-xs">
            <span className="text-lg font-bold text-slate-100">{t('bootTime.total', { s: seconds(shown.total_ms) })}</span>
            <span className="text-slate-400">{t('bootTime.when', { when: shown.session.slice(0, 14), mods: shown.mods })}</span>
            {comparable && (
              <span className={shown.total_ms > usualTotal * 1.2 ? 'text-amber-300' : 'text-slate-400'}>
                {t('bootTime.usually', { s: seconds(usualTotal), count: peers.length })}
              </span>
            )}
          </div>

          <div className="flex h-3 rounded overflow-hidden bg-slate-800">
            {PHASES.map(({ id, color }) => {
              const ms = shown.phases.find((p) => p.id === id)?.millis ?? 0;
              if (ms <= 0) return null;
              return <span key={id} className={color} style={{ width: `${(ms / shown.total_ms) * 100}%` }} title={`${t(`bootTime.phase.${id}`)}: ${seconds(ms)} s`} />;
            })}
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-x-6 gap-y-1">
            {PHASES.map(({ id, color }) => {
              const ms = shown.phases.find((p) => p.id === id)?.millis;
              if (ms === undefined) return null;
              const before = usual(id);
              const slower = comparable && ms > before * SLOWER_RATIO && ms - before >= SLOWER_MS;
              return (
                <div key={id} className="flex items-center gap-2 text-[11px]" title={t(`bootTime.about.${id}`)}>
                  <span className={`w-2.5 h-2.5 rounded-sm shrink-0 ${color}`} />
                  <span className="flex-1 truncate text-slate-300">{t(`bootTime.phase.${id}`)}</span>
                  <span className={`font-mono ${slower ? 'text-amber-300 font-bold' : 'text-slate-200'}`}>{seconds(ms)} s</span>
                  {comparable && <span className="w-16 text-right font-mono text-slate-500">{t('bootTime.was', { s: seconds(before) })}</span>}
                </div>
              );
            })}
          </div>
          {comparable && shown.phases.some((p) => p.millis > usual(p.id) * SLOWER_RATIO && p.millis - usual(p.id) >= SLOWER_MS) && (
            <div className="text-[11px] text-amber-200/90">{t('bootTime.slowerNote')}</div>
          )}

          {boots.length > 1 && (
            <details className="text-[11px]">
              <summary className="cursor-pointer select-none text-sky-300">{t('bootTime.earlier', { count: boots.length })}</summary>
              <div className="mt-1.5 flex flex-col gap-0.5">
                {boots.map((b, i) => (
                  <button
                    key={b.session}
                    type="button"
                    onClick={() => setPicked(i)}
                    className={`flex items-center gap-3 px-2 py-1 rounded text-left cursor-pointer ${i === picked ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:bg-slate-800/60'}`}
                  >
                    <span className="w-32 font-mono">{b.session.slice(0, 14)}</span>
                    <span className="w-16 text-right font-mono">{seconds(b.total_ms)} s</span>
                    <span>{t('bootTime.modsCount', { count: b.mods })}</span>
                  </button>
                ))}
              </div>
            </details>
          )}
        </>
      )}
    </section>
  );
};
