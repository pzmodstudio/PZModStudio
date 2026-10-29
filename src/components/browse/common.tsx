import React from 'react';
import { ChevronLeft, ChevronRight, Copy, ExternalLink, Plus, Trash2 } from 'lucide-react';
import { WorkshopInstallState, WorkshopItem } from '../../types';
import type { MenuItem } from '../common/ContextMenu';
import type { KeepActions } from '../common/keep';
import { TauriService } from '../../services/tauri';

export const PAGE_SIZE = 24;
export const SCROLL_ID = 'browse-scroll';

export const formatCount = (n: number) => (n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n >= 1000 ? `${(n / 1000).toFixed(1)}k` : `${n}`);
export const formatIsoDate = (iso: string | null) => (iso ? new Date(iso).toLocaleDateString() : '');
export const formatUnixDate = (secs: number | null | undefined) => (secs ? new Date(secs * 1000).toLocaleDateString() : '');
export const formatBytes = (n: number | null | undefined) => {
  if (!n) return '';
  if (n >= 1024 ** 3) return `${(n / 1024 ** 3).toFixed(1)} GB`;
  if (n >= 1024 ** 2) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  if (n >= 1024) return `${Math.round(n / 1024)} KB`;
  return `${n} B`;
};

export const scrollBrowseToTop = () => document.getElementById(SCROLL_ID)?.scrollTo({ top: 0 });

import i18next from 'i18next';

export const Chip: React.FC<{ className: string; title?: string; children: React.ReactNode }> = ({ className, title, children }) => (
  <span className={`text-[10px] font-mono px-1.5 py-0.5 rounded border shrink-0 whitespace-nowrap ${className}`} title={title}>{children}</span>
);

export const BuildChips: React.FC<{ builds: string[]; source: string }> = ({ builds, source }) =>
  builds.length === 0 ? (
    <Chip className="border-slate-700 text-slate-500" title={i18next.t('browse.chips.buildUnknownTitle', { source })}>{i18next.t('browse.chips.buildUnknown')}</Chip>
  ) : (
    <>
      {builds.map((b) => (
        <Chip key={b} className={b === '42' ? 'border-emerald-600/60 text-emerald-300' : 'border-amber-600/60 text-amber-300'} title={i18next.t('browse.chips.buildTitle', { build: b, source })}>
          B{b}
        </Chip>
      ))}
    </>
  );

export const InstalledChip: React.FC = () => <Chip className="border-emerald-500/60 text-emerald-300 bg-emerald-950/40">{i18next.t('browse.chips.installed')}</Chip>;

export const UpdateChips: React.FC<{ state: WorkshopInstallState | undefined | null }> = ({ state }) => (
  <>
    {state?.update_pending && (
      <Chip className="border-amber-500/60 text-amber-300 bg-amber-950/40" title={i18next.t('browse.chips.updateWaitingTitle')}>{i18next.t('browse.chips.updateWaiting')}</Chip>
    )}
    {state?.updated_since_last_play && !state.update_pending && (
      <Chip className="border-sky-500/60 text-sky-300 bg-sky-950/40" title={i18next.t('browse.chips.newSinceTitle')}>{i18next.t('browse.chips.newSince')}</Chip>
    )}
  </>
);

/** What a card shows in its hover panel, as the Workshop does: the description and when it was posted. */
export interface CardHover {
  title: string;
  text: string;
  /** "Posted: …", already worded. */
  date?: string | null;
  tags?: string[];
}

/** The Workshop's hover panel for a card: its description, when it was posted, its tags. */
export const cardHover = (item: WorkshopItem): CardHover => ({
  title: item.title,
  text: item.summary,
  date: item.time_created
    ? i18next.t('browse.card.posted', { date: formatUnixDate(item.time_created) })
    : item.time_updated
    ? i18next.t('browse.card.updated', { date: formatUnixDate(item.time_updated) })
    : null,
  tags: item.tags,
});

/**
 * A Workshop mod's right-click menu, the same wherever its card is (Browse, a collection):
 * subscribe or unsubscribe, its page, its link and id, and favorites / lists / profiles.
 */
export const workshopItemMenu = (
  item: WorkshopItem,
  installedIds: string[],
  steamReady: boolean,
  onSubscribe: (ids: string[], subscribe: boolean) => Promise<void>,
  keep?: KeepActions
): MenuItem[] => {
  const installed = installedIds.length > 0;
  return [
    {
      label: installed ? i18next.t('browse.workshop.unsubscribe') : i18next.t('browse.workshop.subscribe'),
      icon: installed ? Trash2 : Plus,
      onSelect: () => void onSubscribe([item.workshop_id], !installed).catch(() => undefined),
      disabled: !steamReady,
      disabledReason: i18next.t('browse.workshop.steamNotRunning'),
    },
    { label: i18next.t('browse.workshop.openPage'), icon: ExternalLink, separatorBefore: true,
      onSelect: () => TauriService.openExternalUrl(item.page_url) },
    { label: i18next.t('browse.workshop.copyLink'), icon: Copy, onSelect: () => navigator.clipboard.writeText(item.page_url) },
    { label: i18next.t('browse.workshop.copyId'), icon: Copy, onSelect: () => navigator.clipboard.writeText(item.workshop_id) },
    ...(keep ? keep.menuItems({ workshop_id: item.workshop_id, name: item.title, installedModIds: installedIds }) : []),
  ];
};

const HOVER_DELAY_MS = 350;
const HOVER_WIDTH = 340;

/**
 * The panel beside a card while the pointer rests on it. It is placed against the window, to the
 * right of the card or to its left when there is no room, and never takes the pointer, so moving
 * on to the next card just moves the panel.
 */
const HoverPanel: React.FC<{ rect: DOMRect; hover: CardHover }> = ({ rect, hover }) => {
  const gap = 8;
  const right = rect.right + gap + HOVER_WIDTH <= window.innerWidth;
  const left = right ? rect.right + gap : Math.max(gap, rect.left - gap - HOVER_WIDTH);
  const top = Math.max(gap, Math.min(rect.top, window.innerHeight - 260));
  return (
    <div
      role="tooltip"
      className="fixed z-50 pointer-events-none rounded-lg border border-slate-600 bg-slate-800/95 shadow-2xl p-3 space-y-2 animate-fade-in"
      style={{ left, top, width: HOVER_WIDTH }}
    >
      <div className="text-sm font-bold text-slate-100 leading-snug">{hover.title}</div>
      {hover.text && <p className="text-xs text-slate-300 leading-relaxed line-clamp-[8]">{hover.text}</p>}
      {hover.date && <div className="text-[11px] text-slate-400">{hover.date}</div>}
      {hover.tags && hover.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {hover.tags.map((tag) => (
            <span key={tag} className="text-[10px] px-1.5 py-0.5 rounded bg-slate-700/80 text-slate-200">{tag}</span>
          ))}
        </div>
      )}
    </div>
  );
};

/** A compact mod card; the whole card opens the detail page. */
export const ModCard: React.FC<{
  image: string | null;
  title: string;
  subtitle?: React.ReactNode;
  chips?: React.ReactNode;
  stats?: React.ReactNode;
  summary?: string | null;
  onOpen?: () => void;
  /** Right-click menu for this card. */
  onContextMenu?: (e: React.MouseEvent) => void;
  /** The description panel shown while the pointer rests on the card. */
  hover?: CardHover | null;
}> = ({ image, title, subtitle, chips, stats, summary, onOpen, onContextMenu, hover }) => {
  const [hoverRect, setHoverRect] = React.useState<DOMRect | null>(null);
  const timer = React.useRef<number | null>(null);
  const clear = () => {
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = null;
    setHoverRect(null);
  };
  React.useEffect(() => clear, []);
  return (
  <button
    type="button"
    onClick={(e) => {
      clear();
      onOpen?.();
      e.currentTarget.blur();
    }}
    onContextMenu={(e) => {
      clear();
      onContextMenu?.(e);
    }}
    onMouseEnter={(e) => {
      if (!hover) return;
      const el = e.currentTarget;
      timer.current = window.setTimeout(() => setHoverRect(el.getBoundingClientRect()), HOVER_DELAY_MS);
    }}
    onMouseLeave={clear}
    onWheel={() => hoverRect && clear()}
    disabled={!onOpen}
    className="group text-left flex flex-col rounded-lg border border-slate-800 bg-slate-900/60 overflow-hidden hover:border-slate-600 hover:bg-slate-900 transition cursor-pointer disabled:cursor-default"
  >
    <div className="relative aspect-[16/9] bg-slate-950 overflow-hidden">
      {image ? (
        <img src={image} alt="" loading="lazy" className="w-full h-full object-cover group-hover:scale-[1.03] transition-transform" />
      ) : (
        <div className="w-full h-full flex items-center justify-center text-slate-700 text-[11px]">{i18next.t('browse.chips.noImage')}</div>
      )}
      {chips && <div className="absolute top-1.5 left-1.5 right-1.5 flex flex-wrap gap-1 [&>span]:bg-slate-950/85">{chips}</div>}
    </div>
    <div className="flex flex-col gap-1 p-2.5 flex-1 min-w-0 w-full">
      <h3 className="text-xs font-bold text-slate-100 leading-snug line-clamp-2">{title}</h3>
      {subtitle && <div className="text-[10px] text-slate-500 truncate">{subtitle}</div>}
      {summary && <p className="text-[11px] text-slate-400 line-clamp-2">{summary}</p>}
      {stats && <div className="flex items-center gap-3 text-[10px] font-mono text-slate-500 mt-auto pt-1">{stats}</div>}
    </div>
    {hover && hoverRect && <HoverPanel rect={hoverRect} hover={hover} />}
  </button>
  );
};

export const CardGrid: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div className="grid gap-3" style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(210px, 1fr))' }}>{children}</div>
);

/** Page numbers: 1 … 4 5 [6] 7 8 … 40. Shown above and below the mods, so long pages need no
 * scrolling back up to turn them. */
export const Pagination: React.FC<{ page: number; pageCount: number; onPage: (page: number) => void; disabled?: boolean }> = ({ page, pageCount, onPage, disabled }) => {
  if (pageCount <= 1) return null;
  const pages: (number | 'gap')[] = [];
  const window = new Set([1, pageCount, page - 2, page - 1, page, page + 1, page + 2].filter((p) => p >= 1 && p <= pageCount));
  [...window].sort((a, b) => a - b).forEach((p, i, arr) => {
    if (i > 0 && p - arr[i - 1] > 1) pages.push('gap');
    pages.push(p);
  });
  const go = (p: number) => {
    if (p < 1 || p > pageCount || p === page) return;
    onPage(p);
    scrollBrowseToTop();
  };
  const btn = 'min-w-[30px] h-[30px] px-2 rounded-md text-[11px] font-bold font-mono border cursor-pointer disabled:opacity-40 disabled:cursor-default';
  return (
    <nav className="flex flex-wrap items-center justify-center gap-1 py-2" aria-label={i18next.t('browse.chips.pages')}>
      <button type="button" className={`${btn} border-slate-800 bg-slate-900 text-slate-300 hover:bg-slate-800`} onClick={() => go(page - 1)} disabled={disabled || page <= 1} aria-label={i18next.t('browse.chips.previous')}>
        <ChevronLeft className="w-3.5 h-3.5" />
      </button>
      {pages.map((p, i) =>
        p === 'gap' ? (
          <span key={`g${i}`} className="px-1 text-slate-600 text-xs">…</span>
        ) : (
          <button key={p} type="button" onClick={() => go(p)} disabled={disabled} aria-current={p === page ? 'page' : undefined}
            className={`${btn} ${p === page ? 'border-emerald-600 bg-emerald-900/50 text-emerald-200' : 'border-slate-800 bg-slate-900 text-slate-300 hover:bg-slate-800'}`}>
            {p}
          </button>
        )
      )}
      <button type="button" className={`${btn} border-slate-800 bg-slate-900 text-slate-300 hover:bg-slate-800`} onClick={() => go(page + 1)} disabled={disabled || page >= pageCount} aria-label={i18next.t('browse.chips.next')}>
        <ChevronRight className="w-3.5 h-3.5" />
      </button>
    </nav>
  );
};

/** One row of category chips; "All" clears the filter. */
export const CategoryBar: React.FC<{
  categories: { name: string; count?: number }[];
  value: string;
  onChange: (name: string) => void;
  accent: string;
}> = ({ categories, value, onChange, accent }) => (
  <div className="flex flex-wrap gap-1.5" role="group" aria-label={i18next.t('browse.chips.categories')}>
    {[{ name: '' }, ...categories].map((c) => {
      const active = c.name === value;
      return (
        <button key={c.name || 'all'} type="button" onClick={() => onChange(c.name)} aria-pressed={active}
          className={`px-2 py-0.5 rounded-full text-[11px] border cursor-pointer transition ${active ? accent : 'border-slate-800 bg-slate-900 text-slate-400 hover:text-slate-200 hover:border-slate-700'}`}>
          {c.name || i18next.t('browse.chips.all')}
          {'count' in c && c.count !== undefined && <span className="ml-1 font-mono text-[10px] opacity-60">{c.count}</span>}
        </button>
      );
    })}
  </div>
);

export const Toolbar: React.FC<{ children: React.ReactNode; onSubmit: () => void }> = ({ children, onSubmit }) => (
  <form className="flex flex-wrap items-center gap-2" onSubmit={(e) => { e.preventDefault(); onSubmit(); }}>
    {children}
  </form>
);

export const selectClass = 'px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-800 text-xs text-slate-200 cursor-pointer disabled:opacity-50';
