import React from 'react';
import { ArrowDown, FolderOpen, ListOrdered, Monitor, Server, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';

/**
 * Where things live, in one picture: one place for mod files (and the fixes to them), one list per
 * way of playing (the profile's, each server's), and who loads which list. Every tooltip about
 * copies, packages, profiles, servers and player 2 explains a piece of this; here it is whole.
 * Folded by default under a one-line question.
 */
const Box: React.FC<{ icon: React.ElementType; title: string; text: string; tone: string }> = ({ icon: Icon, title, text, tone }) => (
  <div className={`flex-1 min-w-[150px] rounded-lg border px-2.5 py-2 ${tone}`}>
    <div className="flex items-center gap-1.5 text-[11px] font-bold text-slate-100">
      <Icon className="w-3.5 h-3.5 shrink-0" aria-hidden="true" />
      {title}
    </div>
    <div className="mt-0.5 text-[10.5px] text-slate-300 leading-snug">{text}</div>
  </div>
);

const Down: React.FC<{ note: string }> = ({ note }) => (
  <div className="flex items-center gap-2 pl-3 text-[10px] text-slate-400">
    <ArrowDown className="w-3.5 h-3.5 text-slate-500 shrink-0" aria-hidden="true" />
    {note}
  </div>
);

export const WhereThingsLive: React.FC<{ className?: string }> = ({ className = '' }) => {
  const { t } = useTranslation();
  return (
    <details className={`rounded-lg border border-slate-800 bg-slate-950/40 px-3 py-2 ${className}`}>
      <summary className="text-[11px] text-sky-300 hover:text-sky-200 cursor-pointer select-none">{t('where.question')}</summary>
      <div className="mt-2 flex flex-col gap-1.5">
        <Box icon={FolderOpen} title={t('where.mods.title')} text={t('where.mods.text')} tone="border-violet-800/70 bg-violet-950/20" />
        <Down note={t('where.toLists')} />
        <div className="flex flex-wrap gap-2">
          <Box icon={ListOrdered} title={t('where.profile.title')} text={t('where.profile.text')} tone="border-sky-800/70 bg-sky-950/20" />
          <Box icon={ListOrdered} title={t('where.server.title')} text={t('where.server.text')} tone="border-amber-800/70 bg-amber-950/20" />
        </div>
        <Down note={t('where.toGames')} />
        <div className="flex flex-wrap gap-2">
          <Box icon={Monitor} title={t('where.single.title')} text={t('where.single.text')} tone="border-slate-700 bg-slate-900/60" />
          <Box icon={Server} title={t('where.hosted.title')} text={t('where.hosted.text')} tone="border-slate-700 bg-slate-900/60" />
          <Box icon={Users} title={t('where.player2.title')} text={t('where.player2.text')} tone="border-slate-700 bg-slate-900/60" />
        </div>
      </div>
    </details>
  );
};
