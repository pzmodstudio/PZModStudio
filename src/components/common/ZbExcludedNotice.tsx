import React from 'react';
import { useTranslation } from 'react-i18next';
import { ExcludedMod } from '../../types';

/**
 * Why a game or server stopped at boot, when its log says: a Java mod ZombieBuddy left out (its
 * signature did not verify) and the addons that failed without it. Said, never acted on: the
 * player decides (Mod Health has the button).
 */
export const ZbExcludedNotice: React.FC<{ excluded: ExcludedMod[]; place?: 'game' | 'server' }> = ({ excluded, place = 'server' }) => {
  const { t } = useTranslation();
  if (excluded.length === 0) return null;
  return (
    <>
      {excluded.map((x) => (
        <div key={x.mod_id} className="rounded-lg border border-red-800/70 bg-red-950/30 p-3 text-[11px] text-red-100 leading-relaxed">
          {t(x.kind === 'signature' ? 'servers.mpTest.zbExcluded.signature' : 'servers.mpTest.zbExcluded.other', { mod: x.mod_id })}{' '}
          {x.addons.length > 0 && t(x.fatal ? (place === 'game' ? 'servers.mpTest.zbExcluded.fatalGame' : 'servers.mpTest.zbExcluded.fatal') : 'servers.mpTest.zbExcluded.addons', { addons: x.addons.join(', ') })}{' '}
          {t('servers.mpTest.zbExcluded.what')}
        </div>
      ))}
    </>
  );
};
