import React from 'react';
import { useTranslation } from 'react-i18next';
import { Heart } from 'lucide-react';
import { AuthorLink } from '../../types';
import { TauriService } from '../../services/tauri';

/** The changelog's links, then the Workshop description's, each address once. */
export function mergeLinks(first: AuthorLink[], more: AuthorLink[] = []): AuthorLink[] {
  const out = [...first];
  for (const l of more) if (!out.some((o) => o.url.toLowerCase() === l.url.toLowerCase())) out.push(l);
  return out;
}

/** "Support the author": the links a mod's author put in its changelog or its Workshop
 *  description (Ko-fi, Patreon...), opened in the browser. */
export const SupportLinks: React.FC<{ links: AuthorLink[]; more?: AuthorLink[] }> = ({ links: first, more }) => {
  const { t } = useTranslation();
  const links = mergeLinks(first, more);
  if (links.length === 0) return null;
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <span className="flex items-center gap-1 text-[10px] uppercase tracking-wider text-pink-300/80 font-bold">
        <Heart className="w-3 h-3" aria-hidden="true" /> {t('support.title')}
      </span>
      {links.map((l) => (
        <button
          key={l.url}
          type="button"
          onClick={() => void TauriService.openExternalUrl(l.url)}
          title={l.url}
          className="px-2 py-0.5 rounded border border-pink-800/60 text-[11px] text-pink-200 hover:bg-pink-950/50 cursor-pointer"
        >
          {l.name}
        </button>
      ))}
    </div>
  );
};
