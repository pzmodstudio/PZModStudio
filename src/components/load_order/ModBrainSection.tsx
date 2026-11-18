import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Loader2 } from 'lucide-react';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { BrainModHistory } from '../../types';
import { day, fixText, onsetText } from '../../utils/brain';
import { cardText } from '../../i18n/backendText';

/**
 * One mod's history on this PC, from the brain (rules::brain::mod_history): the sessions it was
 * on, the versions seen, each error the game blamed on it with when it began and what changed
 * then, and the fixes applied to it with what the sessions after them said.
 */
export const ModBrainSection: React.FC<{ paths: StudioPathsUI; modId: string; name: (id: string) => string }> = ({ paths, modId, name }) => {
  const { t } = useTranslation();
  const [h, setH] = useState<BrainModHistory | null | undefined>(undefined);
  useEffect(() => {
    let live = true;
    setH(undefined);
    void TauriService.brainModHistory(paths, modId).then((x) => live && setH(x));
    return () => {
      live = false;
    };
  }, [paths, modId]);

  if (h === undefined) {
    return (
      <div className="flex items-center gap-2 text-[11.5px] text-slate-400">
        <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />
        {t('brain.history.loading')}
      </div>
    );
  }
  if (!h || h.sessions === 0) return <div className="text-[11.5px] text-slate-500">{t('brain.history.none')}</div>;
  return (
    <div className="space-y-2 text-[11.5px] text-slate-300 leading-relaxed">
      <div>
        {t('brain.history.sessions', { count: h.sessions, first: day(h.first), last: day(h.last) })}{' '}
        {h.place && t('brain.history.place', { place: h.place[0], of: h.place[1] })}
      </div>
      {h.versions.length > 0 && (
        <div className="text-slate-400">
          {t('brain.history.versions', { list: h.versions.map((v) => t('brain.history.version', { version: v.version, sessions: v.sessions })).join(', ') })}
        </div>
      )}
      {(h.failed_to_start > 0 || h.out_of_memory > 0 || (h.crashed ?? 0) > 0) && (
        <div className="text-amber-300/90">{t('brain.history.bad', { failed: h.failed_to_start, oom: h.out_of_memory, crashed: h.crashed ?? 0 })}</div>
      )}
      {h.errors.length > 0 && (
        <div className="space-y-1.5">
          <div className="text-[10.5px] font-bold uppercase tracking-wide text-slate-500">{t('brain.history.errors')}</div>
          {h.errors.slice(0, 6).map((e) => (
            <div key={e.key} className={`rounded border px-2 py-1.5 ${e.stopped ? 'border-slate-800 bg-slate-950/30 text-slate-400' : 'border-amber-800/50 bg-amber-950/15'}`}>
              <div className="font-semibold text-slate-200">
                {cardText({ code: e.code, params: e.params, title: e.title, explanation: '', recommended_action: '' }, 'title')}
                {e.stopped && <span className="ml-1.5 text-[10px] font-normal text-emerald-300">· {t('brain.history.stopped')}</span>}
              </div>
              <div className="text-slate-400">
                {t('brain.history.error', { seen: e.seen, on: e.on, first: day(e.first), last: day(e.last) })}
                {e.file ? ` · ${e.file}${e.line ? `:${e.line}` : ''}` : ''}
              </div>
              {e.onset && <div className="mt-0.5 text-slate-400">{onsetText(e.onset, name, t)}</div>}
            </div>
          ))}
        </div>
      )}
      {h.fixes.length > 0 && (
        <div className="space-y-1">
          <div className="text-[10.5px] font-bold uppercase tracking-wide text-slate-500">{t('brain.history.fixes')}</div>
          {h.fixes.map((f, i) => (
            <div key={i} className={f.verdict === 'worked' ? 'text-emerald-300' : f.verdict === 'not_worked' ? 'text-amber-300' : 'text-slate-400'}>
              {fixText(f, name, t)}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
