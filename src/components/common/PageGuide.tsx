import React, { useState } from 'react';
import { ChevronDown, Compass, HardDrive, HelpCircle, Route, Target, UserCheck } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface PageGuideProps {
  /** The page: its texts are under `guide.<page>` (what, who, touches, and optionally flow). */
  page: 'health' | 'merger' | 'developer';
  className?: string;
}

const KEY = (page: string) => `pz_guide_folded_${page}`;

/**
 * What a page is for, in four short lines anyone can read: what it does, whether you need it,
 * what it changes on disk, and how you get here from the rest of the app. Open the first time;
 * "Got it" folds it to one line, kept folded on this PC.
 */
export const PageGuide: React.FC<PageGuideProps> = ({ page, className = '' }) => {
  const { t, i18n } = useTranslation();
  const [folded, setFolded] = useState<boolean>(() => {
    try {
      return localStorage.getItem(KEY(page)) === '1';
    } catch {
      return false;
    }
  });
  const fold = (next: boolean) => {
    setFolded(next);
    try {
      localStorage.setItem(KEY(page), next ? '1' : '0');
    } catch {
      // only a convenience
    }
  };

  if (folded) {
    return (
      <button
        type="button"
        onClick={() => fold(false)}
        className={`flex items-center gap-1.5 text-[11px] text-sky-300 hover:text-sky-200 cursor-pointer self-start ${className}`}
      >
        <HelpCircle className="w-3.5 h-3.5" aria-hidden="true" />
        {t('guide.whatIsThis')}
      </button>
    );
  }

  const rows: { icon: React.ElementType; label: string; text: string }[] = [
    { icon: Target, label: t('guide.what'), text: t(`guide.${page}.what`) },
    { icon: UserCheck, label: t('guide.who'), text: t(`guide.${page}.who`) },
    { icon: HardDrive, label: t('guide.touches'), text: t(`guide.${page}.touches`) },
  ];
  if (i18n.exists(`guide.${page}.flow`)) rows.push({ icon: Route, label: t('guide.flow'), text: t(`guide.${page}.flow`) });

  return (
    <div className={`rounded-xl border border-sky-800/60 bg-sky-950/20 p-3 flex flex-col gap-2 ${className}`}>
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 text-[12px] font-bold text-sky-200">
          <Compass className="w-4 h-4 text-sky-300" aria-hidden="true" />
          {t('guide.title')}
        </div>
        <button
          type="button"
          onClick={() => fold(true)}
          className="flex items-center gap-1 px-2 py-0.5 rounded text-[11px] text-sky-200 border border-sky-800 hover:bg-sky-900/50 cursor-pointer"
        >
          <ChevronDown className="w-3 h-3 rotate-180" aria-hidden="true" />
          {t('guide.gotIt')}
        </button>
      </div>
      <dl className="grid gap-1.5 sm:grid-cols-[150px_minmax(0,1fr)] text-xs">
        {rows.map(({ icon: Icon, label, text }) => (
          <React.Fragment key={label}>
            <dt className="flex items-center gap-1.5 font-bold text-slate-200">
              <Icon className="w-3.5 h-3.5 text-sky-300 shrink-0" aria-hidden="true" />
              {label}
            </dt>
            <dd className="text-slate-300 leading-relaxed">{text}</dd>
          </React.Fragment>
        ))}
      </dl>
    </div>
  );
};
