import React from 'react';
import { ArrowDownUp, CircleMinus, CirclePlus, TriangleAlert, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface PreviewRow {
  name: string;
  /** Why it is here: "needed by X", "needs X". */
  note?: string;
}

export interface PreviewSection {
  label: string;
  tone: 'on' | 'off' | 'move' | 'warn';
  rows: PreviewRow[];
}

export interface PreviewAction {
  label: string;
  onClick: () => void;
}

interface ChangePreviewProps {
  title: string;
  blurb?: string;
  sections: PreviewSection[];
  /** The change as shown. */
  primary: PreviewAction;
  /** A smaller version of it ("Only this mod"). */
  secondary?: PreviewAction;
  onClose: () => void;
}

const TONE = {
  on: { icon: CirclePlus, head: 'text-emerald-300', dot: 'text-emerald-400' },
  off: { icon: CircleMinus, head: 'text-red-300', dot: 'text-red-400' },
  move: { icon: ArrowDownUp, head: 'text-sky-300', dot: 'text-sky-400' },
  warn: { icon: TriangleAlert, head: 'text-amber-300', dot: 'text-amber-400' },
} as const;

/**
 * Every change the Studio makes on its own initiative is shown before it is made: what goes on,
 * what goes off, what moves, and why; and what cannot be done. Nothing changes until the player
 * picks an action.
 */
export const ChangePreview: React.FC<ChangePreviewProps> = ({ title, blurb, sections, primary, secondary, onClose }) => {
  const { t } = useTranslation();
  const shown = sections.filter((s) => s.rows.length > 0);
  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-6" onClick={onClose}>
      <div role="dialog" aria-modal="true" aria-label={title} className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl w-full max-w-lg p-5 space-y-4" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-bold text-slate-100">{title}</h3>
            {blurb && <p className="text-[11px] text-slate-400 mt-1">{blurb}</p>}
          </div>
          <button type="button" onClick={onClose} className="text-slate-500 hover:text-slate-300 p-1 cursor-pointer" aria-label={t('preview.cancel')}>
            <X className="w-4 h-4" aria-hidden="true" />
          </button>
        </div>
        <div className="space-y-3 max-h-[50vh] overflow-y-auto pr-1">
          {shown.map((s) => {
            const tone = TONE[s.tone];
            const Icon = tone.icon;
            return (
              <div key={s.label}>
                <div className={`flex items-center gap-1.5 text-[11px] font-bold uppercase tracking-wide ${tone.head}`}>
                  <Icon className="w-3.5 h-3.5" aria-hidden="true" />
                  {s.label} <span className="font-mono text-slate-500">({s.rows.length})</span>
                </div>
                <ul className="mt-1 space-y-0.5">
                  {s.rows.map((r, i) => (
                    <li key={`${r.name}-${i}`} className="text-xs text-slate-200 flex gap-2">
                      <span className={tone.dot}>•</span>
                      <span className="min-w-0">
                        {r.name}
                        {r.note && <span className="text-slate-500"> — {r.note}</span>}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            );
          })}
        </div>
        <div className="flex flex-wrap justify-end gap-2 pt-1">
          <button type="button" onClick={onClose} className="px-3 py-1.5 rounded-md text-xs text-slate-300 hover:bg-slate-800 cursor-pointer">
            {t('preview.cancel')}
          </button>
          {secondary && (
            <button type="button" onClick={secondary.onClick} className="px-3 py-1.5 rounded-md border border-slate-700 text-xs text-slate-200 hover:bg-slate-800 cursor-pointer">
              {secondary.label}
            </button>
          )}
          <button type="button" onClick={primary.onClick} autoFocus className="px-3 py-1.5 rounded-md bg-emerald-700/80 hover:bg-emerald-600 text-xs font-semibold text-emerald-50 cursor-pointer">
            {primary.label}
          </button>
        </div>
      </div>
    </div>
  );
};
