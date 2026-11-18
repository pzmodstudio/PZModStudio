import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Sparkles, X } from 'lucide-react';
import { dismissSimpleOffer, setUiMode, shouldOfferSimple } from '../../utils/uiMode';

/**
 * Once, for a player who used the Studio before Simple existed (they were put in Complete without
 * choosing): a line under the header offering to try Simple. It never blocks anything; either
 * answer is kept and it does not come back. Switching is always one click at the foot of the
 * sidebar, so trying costs nothing.
 */
export const SimpleOffer: React.FC = () => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(() => shouldOfferSimple());
  if (!open) return null;
  return (
    <div role="status" className="flex flex-wrap items-center gap-3 px-4 py-2 bg-sky-950/60 border-b border-sky-800/60 text-xs text-sky-100">
      <Sparkles className="w-4 h-4 text-sky-300 shrink-0" aria-hidden="true" />
      <div className="flex-1 min-w-[260px]">
        <span className="font-semibold">{t('simple.offer.title')}</span>{' '}
        <span className="text-sky-200/80">{t('simple.offer.body')}</span>
      </div>
      <button
        type="button"
        onClick={() => {
          setUiMode('simple');
          setOpen(false);
        }}
        className="px-3 py-1 rounded-md font-bold bg-sky-600 hover:bg-sky-500 text-white cursor-pointer"
      >
        {t('simple.offer.try')}
      </button>
      <button
        type="button"
        onClick={() => {
          dismissSimpleOffer();
          setOpen(false);
        }}
        title={t('simple.offer.noTitle')}
        className="flex items-center gap-1 px-2 py-1 rounded-md text-sky-200 hover:bg-sky-900/60 cursor-pointer"
      >
        <X className="w-3.5 h-3.5" aria-hidden="true" /> {t('simple.offer.no')}
      </button>
    </div>
  );
};
