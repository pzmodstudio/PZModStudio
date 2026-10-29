import React, { useState } from 'react';
import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface ChooseOption {
  id: string;
  label: string;
  hint?: string;
  /** Greyed out, with the reason: already there, not possible. */
  disabledReason?: string;
}

interface ChooseDialogProps {
  title: string;
  /** One line under the title saying what picking does. */
  blurb?: string;
  options: ChooseOption[];
  onPick: (id: string) => void;
  onClose: () => void;
  /** A "new …" row at the end, with a name box: for lists. */
  allowNew?: { placeholder: string; onCreate: (name: string) => Promise<void> | void };
}

/** A short list of things to pick one of: a profile, a list. */
export const ChooseDialog: React.FC<ChooseDialogProps> = ({ title, blurb, options, onPick, onClose, allowNew }) => {
  const { t } = useTranslation();
  const [newName, setNewName] = useState('');
  const [busy, setBusy] = useState(false);
  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-6" onClick={onClose}>
      <div className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl w-full max-w-md p-5 space-y-3" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-bold text-slate-100">{title}</h3>
            {blurb && <p className="text-[11px] text-slate-400 mt-0.5">{blurb}</p>}
          </div>
          <button type="button" onClick={onClose} className="text-slate-500 hover:text-slate-300 p-1 cursor-pointer" aria-label={t('dialogs.keep.close')}>
            <X className="w-4 h-4" aria-hidden="true" />
          </button>
        </div>
        {options.length === 0 && !allowNew && <p className="text-xs text-slate-400 py-2 text-center">{t('dialogs.keep.nothingToPick')}</p>}
        {options.length > 0 && (
          <div className="rounded-xl border border-slate-800 divide-y divide-slate-900 max-h-72 overflow-y-auto">
            {options.map((o) => (
              <button
                key={o.id}
                type="button"
                disabled={!!o.disabledReason}
                title={o.disabledReason}
                onClick={() => onPick(o.id)}
                className="w-full text-left px-3 py-2 text-xs text-slate-200 hover:bg-slate-950/70 cursor-pointer flex items-center justify-between gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <span className="truncate">{o.label}</span>
                {o.hint && <span className="font-mono text-[10px] text-slate-500 shrink-0">{o.hint}</span>}
              </button>
            ))}
          </div>
        )}
        {allowNew && (
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              if (!newName.trim()) return;
              setBusy(true);
              void Promise.resolve(allowNew.onCreate(newName.trim()))
                .then(() => setNewName(''))
                .finally(() => setBusy(false));
            }}
          >
            <input
              type="text"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={allowNew.placeholder}
              className="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500"
            />
            <button type="submit" disabled={busy || !newName.trim()} className="px-3 py-1.5 rounded-lg text-xs font-bold bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer disabled:opacity-50">
              {t('dialogs.keep.newList')}
            </button>
          </form>
        )}
      </div>
    </div>
  );
};
