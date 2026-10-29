import React, { useEffect, useState } from 'react';
import { AlertTriangle, CheckCircle2, X } from 'lucide-react';
import { BACKEND_ERROR_EVENT, BackendErrorDetail, NOTICE_EVENT, explainError } from '../../services/tauri';
import { useTranslation } from 'react-i18next';

interface ToastItem extends BackendErrorDetail {
  id: number;
  /** A notice, not an error: `message` is the whole text. */
  notice?: boolean;
}

const AUTO_DISMISS_MS = 12000;
const MAX_VISIBLE = 4;

/** Shows every failed backend operation reported through TauriService (reportBackendError). */
export const BackendErrorToasts: React.FC = () => {
  const { t } = useTranslation();
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  useEffect(() => {
    let nextId = 0;
    const timers = new Set<ReturnType<typeof setTimeout>>();
    const onError = (e: Event) => {
      const detail = (e as CustomEvent<BackendErrorDetail>).detail;
      const id = nextId++;
      setToasts((prev) => [...prev, { ...detail, id }].slice(-MAX_VISIBLE));
      const timer = setTimeout(() => {
        timers.delete(timer);
        setToasts((prev) => prev.filter((t) => t.id !== id));
      }, AUTO_DISMISS_MS);
      timers.add(timer);
    };
    const onNotice = (e: Event) => {
      const id = nextId++;
      setToasts((prev) => [...prev, { action: '', message: (e as CustomEvent<string>).detail, id, notice: true }].slice(-MAX_VISIBLE));
      const timer = setTimeout(() => {
        timers.delete(timer);
        setToasts((prev) => prev.filter((t) => t.id !== id));
      }, AUTO_DISMISS_MS);
      timers.add(timer);
    };
    window.addEventListener(BACKEND_ERROR_EVENT, onError);
    window.addEventListener(NOTICE_EVENT, onNotice);
    return () => {
      window.removeEventListener(BACKEND_ERROR_EVENT, onError);
      window.removeEventListener(NOTICE_EVENT, onNotice);
      timers.forEach(clearTimeout);
    };
  }, []);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-[100] flex flex-col gap-2 w-96 max-w-[calc(100vw-2rem)]" role="status" aria-live="assertive">
      {toasts.map((item) => item.notice ? (
        <div key={item.id} role="status" className="flex items-start gap-3 p-3 rounded-lg border border-emerald-500/40 bg-emerald-950/90 text-emerald-100 shadow-lg">
          <CheckCircle2 className="w-4 h-4 mt-0.5 shrink-0 text-emerald-400" aria-hidden="true" />
          <div className="flex-1 min-w-0 text-[11px] break-words">{item.message}</div>
          <button
            type="button"
            onClick={() => setToasts((prev) => prev.filter((x) => x.id !== item.id))}
            className="p-0.5 rounded text-emerald-300 hover:text-white hover:bg-emerald-800/60 cursor-pointer"
            aria-label={t('backend.dismiss')}
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      ) : (
        <div key={item.id} role="alert" className="flex items-start gap-3 p-3 rounded-lg border border-red-500/40 bg-red-950/90 text-red-100 shadow-lg">
          <AlertTriangle className="w-4 h-4 mt-0.5 shrink-0 text-red-400" aria-hidden="true" />
          <div className="flex-1 min-w-0">
            <div className="text-xs font-bold">{t('backend.failed', { action: t(`backend.actions.${item.action}`, { defaultValue: item.action }) })}</div>
            <div className="text-[11px] text-red-200/90 break-words">{explainError(item.message)}</div>
          </div>
          <button
            type="button"
            onClick={() => setToasts((prev) => prev.filter((x) => x.id !== item.id))}
            className="p-0.5 rounded text-red-300 hover:text-white hover:bg-red-800/60 cursor-pointer"
            aria-label={t('backend.dismiss')}
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      ))}
    </div>
  );
};
