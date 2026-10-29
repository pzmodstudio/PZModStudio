import React, { useState } from 'react';
import { AlertTriangle, Check, Download, X } from 'lucide-react';
import i18next from 'i18next';
import { MissingModsReport } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, explainError } from '../../services/tauri';
import { tHtml } from '../../i18n/html';

/** Whether an imported package's check has anything to say. */
export const importCheckHasNews = (r: MissingModsReport | null | undefined): r is MissingModsReport =>
  !!r &&
  (r.missing_mods.length > 0 ||
    (r.drift?.length ?? 0) > 0 ||
    (!!r.pack_game_version && !!r.your_game_version && r.pack_game_version !== r.your_game_version));

interface ImportCheckProps {
  packageName: string;
  report: MissingModsReport;
  paths: StudioPathsUI;
  onClose: () => void;
}

/**
 * After a .pzmerge import: the package's files are the sender's copies of the mods it merges, so
 * here they only fit the same copies. Says which mods are missing (with one-click subscribing),
 * which copies differ, and whether the game build differs.
 */
export const ImportCheck: React.FC<ImportCheckProps> = ({ packageName, report, paths, onClose }) => {
  const t = i18next.t.bind(i18next);
  const workshopIds = [...new Set(report.missing_mods.map((m) => m.workshop_id).filter((id): id is string => !!id))];
  const [subscribe, setSubscribe] = useState<{ kind: 'idle' } | { kind: 'working' } | { kind: 'done'; text: string; ok: boolean }>({ kind: 'idle' });
  const buildDiffers = !!report.pack_game_version && !!report.your_game_version && report.pack_game_version !== report.your_game_version;
  const drift = report.drift ?? [];

  const subscribeAll = async () => {
    setSubscribe({ kind: 'working' });
    try {
      const outcome = await TauriService.workshopSubscribe(paths, workshopIds, true);
      setSubscribe(
        outcome.failed.length > 0
          ? { kind: 'done', ok: false, text: t('presets.subscribeFailed', { count: outcome.failed.length, reason: outcome.failed[0][1] }) }
          : { kind: 'done', ok: true, text: t('merger.importCheck.subscribed', { count: outcome.done.length }) }
      );
    } catch (err) {
      setSubscribe({ kind: 'done', ok: false, text: explainError(err) });
    }
  };

  return (
    <div className="fixed inset-0 bg-slate-950/80 backdrop-blur-sm flex items-center justify-center p-4 z-50">
      <div className="bg-slate-900 border border-amber-500/40 rounded-2xl p-6 max-w-lg w-full space-y-4 shadow-2xl">
        <div className="flex items-center justify-between border-b border-slate-800 pb-3">
          <div className="flex items-center gap-2.5">
            <AlertTriangle className="w-5 h-5 text-amber-400" />
            <h3 className="text-base font-bold text-slate-100">{t('merger.importCheck.title', { name: packageName })}</h3>
          </div>
          <button onClick={onClose} className="text-slate-500 hover:text-slate-300 p-1" aria-label={t('merger.importCheck.close')}>
            <X className="w-4 h-4" aria-hidden="true" />
          </button>
        </div>

        <p className="text-xs text-slate-300 leading-relaxed">{t('merger.importCheck.why')}</p>

        {buildDiffers && (
          <div
            className="p-3 bg-slate-950/60 border border-slate-700 rounded-xl text-[11px] text-slate-300 leading-relaxed"
            dangerouslySetInnerHTML={{ __html: tHtml('merger.importCheck.build', { pack: report.pack_game_version, yours: report.your_game_version }) }}
          />
        )}

        {report.missing_mods.length > 0 && (
          <div className="p-3 bg-amber-950/50 border border-amber-600/60 rounded-xl space-y-2">
            <div className="text-[11px] font-bold text-amber-300">{t('merger.importCheck.missing', { count: report.missing_mods.length })}</div>
            <div className="max-h-32 overflow-y-auto space-y-1 pr-1">
              {report.missing_mods.map((m) => (
                <div key={m.mod_id} className="flex items-center justify-between gap-2 p-1.5 bg-slate-950/80 rounded border border-slate-800 text-[11px]">
                  <span className="text-slate-200 truncate">{m.name}</span>
                  <span className="font-mono text-[10px] text-slate-500 shrink-0">{m.workshop_id ? `#${m.workshop_id}` : m.mod_id}</span>
                </div>
              ))}
            </div>
            {workshopIds.length > 0 && (
              <div className="space-y-1">
                <button
                  onClick={subscribeAll}
                  disabled={subscribe.kind === 'working' || (subscribe.kind === 'done' && subscribe.ok)}
                  className="flex items-center gap-1.5 px-3 py-1.5 bg-cyan-700 hover:bg-cyan-600 disabled:opacity-50 text-white rounded-lg text-[11px] font-bold shadow transition cursor-pointer"
                  title={t('presets.subscribeAllTitle')}
                >
                  <Download className="w-3.5 h-3.5" />
                  <span>{subscribe.kind === 'working' ? t('presets.subscribing') : t('presets.subscribeAll', { count: workshopIds.length })}</span>
                </button>
                {subscribe.kind === 'done' && (
                  <div className={`text-[11px] ${subscribe.ok ? 'text-emerald-300' : 'text-rose-300'}`}>{subscribe.text}</div>
                )}
              </div>
            )}
          </div>
        )}

        {drift.length > 0 && (
          <div className="p-3 bg-slate-950/60 border border-slate-700 rounded-xl space-y-2">
            <div className="text-[11px] font-bold text-slate-200">{t('merger.importCheck.drift', { count: drift.length })}</div>
            <div className="max-h-32 overflow-y-auto space-y-1 pr-1">
              {drift.map((d) => (
                <div key={d.mod_id} className="flex items-center justify-between gap-2 p-1.5 bg-slate-950/80 rounded border border-slate-800 text-[11px]">
                  <span className="text-slate-200 truncate">{d.name}</span>
                  <span className="font-mono text-[10px] text-slate-400 shrink-0">
                    {d.direction === 'older' ? t('presets.older') : d.direction === 'newer' ? t('presets.newer') : t('presets.differs')}
                    {d.pack_version || d.your_version ? ` (${d.pack_version ?? '?'} → ${d.your_version ?? '?'})` : ''}
                  </span>
                </div>
              ))}
            </div>
            <p className="text-[10px] text-slate-400 leading-relaxed">{t('merger.importCheck.driftWhat')}</p>
          </div>
        )}

        <div className="flex justify-end pt-2 border-t border-slate-800">
          <button
            onClick={onClose}
            className="px-5 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs font-bold transition cursor-pointer flex items-center gap-1.5"
          >
            <Check className="w-3.5 h-3.5 text-emerald-400" />
            <span>{t('merger.importCheck.ok')}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
