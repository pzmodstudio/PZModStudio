import React, { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import i18next from 'i18next';
import { currentLanguageCode } from '../../i18n';
import { AlertTriangle, Check, Clock, Loader2, Plus, RotateCw, X } from 'lucide-react';
import { RestartSchedule, RestartView, StudioPathsUI } from '../../types';
import { TauriService, explainError } from '../../services/tauri';

const WARN_CHOICES = [60, 30, 15, 10, 5, 1];

/**
 * A server's restarts: at times of day or every N hours, when a Workshop mod of the server is
 * updated, and after it stops by itself; the next one, the last ones and why, and a restart now
 * with warnings in the game. The app does them, so only while it is open.
 */
export const RestartsPanel: React.FC<{ paths: StudioPathsUI; server: string }> = ({ paths, server }) => {
  const { t } = useTranslation();
  const [view, setView] = useState<RestartView | null>(null);
  const [draft, setDraft] = useState<RestartSchedule | null>(null);
  const [newTime, setNewTime] = useState('06:00');
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);
  const [warnNow, setWarnNow] = useState(5);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));

  const load = useCallback(async () => {
    try {
      const v = await TauriService.serverRestartsView(paths, server);
      setView(v);
      setDraft((d) => d ?? v.schedule);
    } catch (err) {
      setNote({ ok: false, text: explainError(err) });
    }
  }, [paths, server]);

  useEffect(() => {
    setDraft(null);
    setNote(null);
    void load();
    const id = window.setInterval(() => {
      setNow(Math.floor(Date.now() / 1000));
      void load();
    }, 3000);
    return () => window.clearInterval(id);
  }, [load]);

  if (!view || !draft) return null;

  const changed = JSON.stringify(draft) !== JSON.stringify(view.schedule);
  const set = (patch: Partial<RestartSchedule>) => setDraft({ ...draft, ...patch });
  const when = (secs: number) => new Date(secs * 1000).toLocaleString(i18next.language, { weekday: 'short', hour: '2-digit', minute: '2-digit', day: 'numeric', month: 'short' });
  const inMinutes = (secs: number) => Math.max(0, Math.ceil((secs - now) / 60));

  const save = async () => {
    setBusy(true);
    try {
      const s = await TauriService.serverScheduleSave(paths, server, { ...draft, language: currentLanguageCode() });
      setDraft(s);
      setNote({ ok: true, text: t('servers.restarts.saved') });
      await load();
      setDraft(s);
    } catch (err) {
      setNote({ ok: false, text: explainError(err) });
    } finally {
      setBusy(false);
    }
  };

  const restartNow = async (cancel = false) => {
    setBusy(true);
    try {
      await TauriService.serverRestartRequest(paths, warnNow, cancel);
      setNote({ ok: true, text: cancel ? t('servers.restarts.cancelAsked') : t('servers.restarts.asked', { count: warnNow }) });
    } catch (err) {
      setNote({ ok: false, text: explainError(err) });
    } finally {
      setBusy(false);
    }
  };

  const addTime = () => {
    if (!/^\d{1,2}:\d{2}$/.test(newTime) || draft.times.includes(newTime)) return;
    set({ times: [...draft.times, newTime].sort() });
  };
  const toggleWarn = (m: number) =>
    set({ warn_minutes: draft.warn_minutes.includes(m) ? draft.warn_minutes.filter((x) => x !== m) : [...draft.warn_minutes, m].sort((a, b) => b - a) });

  const box = 'bg-slate-950/60 border border-slate-800 rounded-xl p-4 space-y-3';
  const tick = (on: boolean, onChange: () => void, label: string, help?: string) => (
    <label className="flex items-start gap-2 text-xs text-slate-200 cursor-pointer">
      <input type="checkbox" checked={on} onChange={onChange} className="mt-0.5 accent-purple-500" />
      <span>
        <span className="font-semibold">{label}</span>
        {help && <span className="block text-[11px] text-slate-400">{help}</span>}
      </span>
    </label>
  );
  const pending = view.pending;

  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <div className="text-sm font-bold text-slate-100">{t('servers.restarts.title')}</div>
        <p className="text-[11px] text-slate-400">{t('servers.restarts.intro')}</p>
        <p className="text-[11px] text-amber-300/90">{t('servers.restarts.whileOpen')}</p>
        {view.running && !view.app_watching && <p className="text-[11px] text-red-300">{t('servers.restarts.notWatching')}</p>}
      </div>

      {/* What is coming */}
      <div className={box}>
        <div className="flex flex-wrap items-center gap-3 text-xs">
          <Clock className="w-4 h-4 text-purple-400" aria-hidden="true" />
          {view.restarting_since ? (
            <span className="text-amber-200 font-bold flex items-center gap-1.5">
              <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />
              {t('servers.restarts.restarting')}
            </span>
          ) : pending ? (
            <span className="text-amber-200 font-bold">
              {t('servers.restarts.pending', { when: when(pending.at), count: inMinutes(pending.at), why: t(`servers.restarts.reason.${pending.reason}`) })}
            </span>
          ) : !view.running ? (
            <span className="text-slate-400">{t('servers.restarts.notRunning')}</span>
          ) : view.next_restart ? (
            <span className="text-slate-200">{t('servers.restarts.next', { when: when(view.next_restart) })}</span>
          ) : (
            <span className="text-slate-400">{t('servers.restarts.noneScheduled')}</span>
          )}
        </div>
        {pending?.reason === 'mod_update' && pending.detail && <p className="text-[11px] text-slate-400">{t('servers.restarts.updatedMods', { list: pending.detail })}</p>}
        {view.gave_up_at && (
          <div className="p-2.5 rounded-lg bg-red-950/40 border border-red-800/70 text-[11px] text-red-100 flex items-start gap-2">
            <AlertTriangle className="w-4 h-4 shrink-0 text-red-300" aria-hidden="true" />
            <span>{t('servers.restarts.gaveUp', { when: when(view.gave_up_at), count: view.schedule.crash_limit, minutes: view.schedule.crash_window_minutes })}</span>
          </div>
        )}
        <div className="flex flex-wrap items-center gap-2 pt-1">
          <label className="text-[11px] text-slate-300">{t('servers.restarts.warnFor')}</label>
          <select
            value={warnNow}
            onChange={(e) => setWarnNow(Number(e.target.value))}
            className="bg-slate-950 border border-slate-800 rounded-lg px-2 py-1 text-xs text-slate-200"
            aria-label={t('servers.restarts.warnFor')}
          >
            {[0, 1, 2, 5, 10, 15, 30].map((m) => (
              <option key={m} value={m}>
                {m === 0 ? t('servers.restarts.noWarning') : t('servers.restarts.minutes', { count: m })}
              </option>
            ))}
          </select>
          <button
            type="button"
            disabled={busy || !view.running || !view.app_watching || Boolean(view.restarting_since)}
            onClick={() => void restartNow(false)}
            title={t('servers.restarts.restartNowTitle')}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-amber-600 hover:bg-amber-500 text-white cursor-pointer disabled:opacity-50"
          >
            <RotateCw className="w-3.5 h-3.5" aria-hidden="true" />
            {t('servers.restarts.restartNow')}
          </button>
          {pending && (
            <button
              type="button"
              disabled={busy}
              onClick={() => void restartNow(true)}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 cursor-pointer disabled:opacity-50"
            >
              <X className="w-3.5 h-3.5" aria-hidden="true" />
              {t('servers.restarts.cancel')}
            </button>
          )}
        </div>
      </div>

      {/* The schedule */}
      <div className={box}>
        {tick(draft.enabled, () => set({ enabled: !draft.enabled }), t('servers.restarts.scheduled'), t('servers.restarts.scheduledHelp'))}
        <div className={`space-y-3 pl-6 ${draft.enabled ? '' : 'opacity-50'}`}>
          <div className="space-y-1.5">
            <div className="text-[11px] font-bold text-slate-300">{t('servers.restarts.times')}</div>
            <div className="flex flex-wrap items-center gap-1.5">
              {draft.times.map((tm) => (
                <span key={tm} className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-purple-950/70 border border-purple-700/60 text-xs font-mono text-purple-200">
                  {tm}
                  <button type="button" onClick={() => set({ times: draft.times.filter((x) => x !== tm) })} aria-label={t('servers.restarts.removeTime', { time: tm })} className="cursor-pointer text-purple-300 hover:text-white">
                    <X className="w-3 h-3" aria-hidden="true" />
                  </button>
                </span>
              ))}
              <input type="time" value={newTime} onChange={(e) => setNewTime(e.target.value)} aria-label={t('servers.restarts.newTime')} className="bg-slate-950 border border-slate-800 rounded-lg px-2 py-0.5 text-xs text-slate-200" />
              <button type="button" onClick={addTime} className="flex items-center gap-1 px-2 py-1 rounded-lg text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer">
                <Plus className="w-3 h-3" aria-hidden="true" />
                {t('servers.restarts.addTime')}
              </button>
            </div>
          </div>
          <label className="flex items-center gap-2 text-[11px] text-slate-300">
            <span className="font-bold">{t('servers.restarts.every')}</span>
            <select value={draft.every_hours} onChange={(e) => set({ every_hours: Number(e.target.value) })} className="bg-slate-950 border border-slate-800 rounded-lg px-2 py-1 text-xs text-slate-200">
              {[0, 2, 3, 4, 6, 8, 12, 24].map((h) => (
                <option key={h} value={h}>
                  {h === 0 ? t('servers.restarts.everyOff') : t('servers.restarts.hours', { count: h })}
                </option>
              ))}
            </select>
            <span className="text-slate-500">{t('servers.restarts.everyHelp')}</span>
          </label>
        </div>

        <div className="space-y-1.5">
          <div className="text-[11px] font-bold text-slate-300">{t('servers.restarts.warnings')}</div>
          <div className="flex flex-wrap gap-1.5">
            {WARN_CHOICES.map((m) => (
              <button
                key={m}
                type="button"
                onClick={() => toggleWarn(m)}
                className={`px-2 py-0.5 rounded-lg text-[11px] font-bold cursor-pointer border ${draft.warn_minutes.includes(m) ? 'bg-purple-600 border-purple-500 text-white' : 'bg-slate-900 border-slate-800 text-slate-400'}`}
              >
                {t('servers.restarts.minutes', { count: m })}
              </button>
            ))}
          </div>
          <p className="text-[11px] text-slate-500">{t('servers.restarts.warningsHelp')}</p>
        </div>

        {tick(draft.on_mod_update, () => set({ on_mod_update: !draft.on_mod_update }), t('servers.restarts.onModUpdate'), t('servers.restarts.onModUpdateHelp', { minutes: draft.mod_check_minutes }))}
        {tick(draft.on_crash, () => set({ on_crash: !draft.on_crash }), t('servers.restarts.onCrash'), t('servers.restarts.onCrashHelp', { count: draft.crash_limit, minutes: draft.crash_window_minutes }))}

        <div className="flex items-center gap-2">
          <button
            type="button"
            disabled={busy || !changed}
            onClick={() => void save()}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-purple-600 hover:bg-purple-500 text-white cursor-pointer disabled:opacity-50"
          >
            {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Check className="w-3.5 h-3.5" aria-hidden="true" />}
            {t('servers.restarts.save')}
          </button>
          {changed && <span className="text-[11px] text-amber-300">{t('servers.restarts.unsaved')}</span>}
        </div>
      </div>

      {note && <p className={`text-[11px] ${note.ok ? 'text-emerald-300' : 'text-red-300'}`}>{note.text}</p>}

      {/* What happened */}
      <div className={box}>
        <div className="text-xs font-bold text-slate-200">{t('servers.restarts.history')}</div>
        {view.history.length === 0 ? (
          <p className="text-[11px] text-slate-500">{t('servers.restarts.noHistory')}</p>
        ) : (
          <div className="divide-y divide-slate-800 max-h-64 overflow-y-auto">
            {view.history.map((e, i) => (
              <div key={i} className="py-1.5 flex items-start gap-2 text-[11px]">
                <span className={`mt-1 w-1.5 h-1.5 rounded-full shrink-0 ${e.ok ? 'bg-emerald-400' : 'bg-red-400'}`} aria-hidden="true" />
                <span className="text-slate-400 font-mono shrink-0">{when(e.at)}</span>
                <span className="text-slate-200 font-semibold shrink-0">{t(`servers.restarts.reason.${e.reason}`)}</span>
                {e.detail && <span className="text-slate-400 break-words">{e.detail}</span>}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
