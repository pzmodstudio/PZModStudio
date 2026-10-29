import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ClipboardCopy, FileText, Stethoscope } from 'lucide-react';
import { StudioPathsUI } from './SettingsModule';
import { TauriService } from '../../services/tauri';

/**
 * Everything a bug report needs, copied in one click with the Windows user name and the PC's
 * name masked; and the Studio's own log, for what went wrong in it.
 */
export const DiagnosticsCard: React.FC<{ paths: StudioPathsUI }> = ({ paths }) => {
  const { t } = useTranslation();
  const [state, setState] = useState<'idle' | 'copied' | 'failed'>('idle');
  const [preview, setPreview] = useState<string | null>(null);

  const copy = async () => {
    const text = await TauriService.diagnosticsReport(paths);
    if (!text) {
      setState('failed');
      return;
    }
    setPreview(text);
    try {
      await navigator.clipboard.writeText(text);
      setState('copied');
    } catch {
      setState('failed');
    }
    setTimeout(() => setState('idle'), 3000);
  };

  return (
    <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-3">
      <div className="space-y-1">
        <div className="text-xs font-bold text-slate-200 flex items-center gap-2">
          <Stethoscope className="w-3.5 h-3.5 text-sky-400" aria-hidden="true" /> {t('settings.diagnostics.title')}
        </div>
        <p className="text-[11px] text-slate-400 max-w-2xl">{t('settings.diagnostics.blurb')}</p>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <button type="button" onClick={() => void copy()} className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-sky-800/70 hover:bg-sky-700 text-sky-50 cursor-pointer">
          <ClipboardCopy className="w-3.5 h-3.5" /> {state === 'copied' ? t('settings.diagnostics.copied') : t('settings.diagnostics.copy')}
        </button>
        <button type="button" onClick={() => void TauriService.openStudioLogFolder()} className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer">
          <FileText className="w-3.5 h-3.5" /> {t('settings.diagnostics.openLog')}
        </button>
        {state === 'failed' && <span className="text-[11px] text-red-300">{t('settings.diagnostics.failed')}</span>}
      </div>
      {preview && (
        <pre className="text-[10px] text-slate-400 bg-slate-950 border border-slate-800 rounded-lg p-3 max-h-64 overflow-auto whitespace-pre-wrap">{preview}</pre>
      )}
    </div>
  );
};
