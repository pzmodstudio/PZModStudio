import React from 'react';
import { CatalogModSummary } from '../../types';
import { TauriService } from '../../services/tauri';
import i18next from 'i18next';
import { Tip, TipBody, TipTone } from './Tip';
import { RichText } from './RichText';

export type SourceKind = 'steam' | 'nexus' | 'manual' | 'local' | 'upload' | 'studio' | 'compat';

const STYLE: Record<SourceKind, string> = {
  steam: 'border-sky-600/60 text-sky-300 bg-sky-950/40',
  nexus: 'border-orange-600/60 text-orange-300 bg-orange-950/40',
  manual: 'border-violet-600/60 text-violet-300 bg-violet-950/40',
  local: 'border-slate-600 text-slate-300 bg-slate-900',
  upload: 'border-teal-600/60 text-teal-300 bg-teal-950/40',
  studio: 'border-emerald-600/60 text-emerald-300 bg-emerald-950/40',
  compat: 'border-amber-600/60 text-amber-300 bg-amber-950/40',
};

const TONE: Record<SourceKind, TipTone> = {
  steam: 'sky',
  nexus: 'orange',
  manual: 'violet',
  local: 'slate',
  upload: 'teal',
  studio: 'emerald',
  compat: 'amber',
};

export function sourceOf(modId: string, entry: CatalogModSummary | undefined): { kind: SourceKind; title: string; url?: string; changes?: string[] } {
  // Studio packages, the Live Bridge and the Claude Agent mod are all installed by the Studio.
  if (modId.startsWith('Z_PZModStudio_') || modId === 'Z_ClaudeAgent') return { kind: 'studio', title: i18next.t('source.studioTitle') };
  if (!entry) return { kind: 'local', title: i18next.t('source.localTitle') };
  switch (entry.source.kind) {
    case 'workshop':
      return {
        kind: 'steam',
        title: i18next.t('source.workshopTitle', { id: entry.source.workshop_id }),
        url: `https://steamcommunity.com/sharedfiles/filedetails/?id=${entry.source.workshop_id}`,
      };
    case 'workshop_upload':
      return { kind: 'upload', title: i18next.t('source.uploadTitle', { folder: entry.source.item_folder }) };
    case 'local':
      if (entry.origin?.kind === 'nexus') {
        return { kind: 'nexus', title: i18next.t('source.nexusTitle', { id: entry.origin.mod_id }), url: entry.origin.page_url };
      }
      if (entry.origin?.kind === 'compat') {
        return {
          kind: 'compat',
          title: i18next.t('source.compatHead'),
          changes: entry.origin.changes,
          url: entry.origin.workshop_id ? `https://steamcommunity.com/sharedfiles/filedetails/?id=${entry.origin.workshop_id}` : undefined,
        };
      }
      if (entry.origin?.kind === 'url') {
        return {
          kind: 'manual',
          title: i18next.t('source.urlTitle', { source: entry.origin.source }),
          url: entry.origin.page_url ?? entry.origin.url,
        };
      }
      if (entry.origin?.kind === 'manual') {
        return { kind: 'manual', title: entry.origin.file_name ? i18next.t('source.manualFromTitle', { file: entry.origin.file_name }) : i18next.t('source.manualTitle') };
      }
      return { kind: 'local', title: i18next.t('source.outsideTitle') };
  }
}

/** What the source tag's tip says: where the mod comes from and, for a compatible copy, what was changed and why it wins. */
const SourceTip: React.FC<{ src: ReturnType<typeof sourceOf>; modId: string; entry: CatalogModSummary | undefined }> = ({ src, modId, entry }) => (
  <TipBody
    title={src.title}
    tone={TONE[src.kind]}
    action={src.url ? i18next.t('source.openPageAction') : undefined}
    footnote={src.kind === 'compat' ? <RichText text={i18next.t('source.compatWins', { id: modId })} mods={[modId]} /> : undefined}
  >
    {src.changes && src.changes.length > 0 ? (
      <div className="flex flex-col gap-1">
        <div className="text-[10.5px] uppercase tracking-wider font-bold text-slate-500">{i18next.t('source.compatChanges')}</div>
        {src.changes.map((c, i) => (
          <RichText key={i} text={c} foldAt={220} />
        ))}
      </div>
    ) : entry?.root_dir ? (
      <RichText text={i18next.t('source.folder', { path: entry.root_dir })} />
    ) : null}
  </TipBody>
);

/** Small tag saying where a mod comes from; clicking it opens its page when there is one. */
export const SourceBadge: React.FC<{ modId: string; entry: CatalogModSummary | undefined }> = ({ modId, entry }) => {
  const src = sourceOf(modId, entry);
  const label = i18next.t(`source.${src.kind}`);
  const className = `text-[10px] font-mono font-bold px-1.5 py-0.5 rounded border shrink-0 ${STYLE[src.kind]}`;
  const tip = <SourceTip src={src} modId={modId} entry={entry} />;
  if (!src.url) {
    return (
      <Tip content={tip}>
        <span className={className}>{label}</span>
      </Tip>
    );
  }
  return (
    <Tip content={tip}>
      <button
        type="button"
        className={`${className} cursor-pointer hover:brightness-125`}
        onClick={(e) => {
          e.stopPropagation();
          TauriService.openExternalUrl(src.url!);
        }}
      >
        {label}
      </button>
    </Tip>
  );
};
