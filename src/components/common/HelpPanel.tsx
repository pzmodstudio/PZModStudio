import React, { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SHOW_WELCOME_EVENT } from './WelcomeTour';
import i18next from 'i18next';
import { HelpCircle, X } from 'lucide-react';
import { ActiveTab } from '../../types';

/**
 * The help of each screen: `docs/app/<lang>/<page>.md`, written for the player, bundled into the
 * app. The "?" button (or F1) opens the page of the screen in front, in the app's language.
 */
const PAGES = import.meta.glob('../../../docs/app/*/*.md', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

const PAGE_OF_TAB: Record<ActiveTab, string> = {
  PROFILES: 'profiles',
  SAVES: 'saves',
  MONITOR: 'monitor',
  MOD_LIST: 'mod-list',
  WHATS_NEW: 'whats-new',
  HEALTH: 'health',
  BROWSE: 'browse',
  MAPS: 'maps',
  MERGER: 'merger',
  DEVELOPER: 'developer',
  SERVERS: 'servers',
  SETTINGS: 'settings',
};

/** A page's text in this language, else in English. */
export function helpPage(page: string, language: string): string | null {
  const lang = language.startsWith('es') ? 'es' : 'en';
  // (a byte-order mark an editor left at the start would hide the title)
  const find = (l: string) => Object.entries(PAGES).find(([path]) => path.endsWith(`/${l}/${page}.md`))?.[1]?.replace(/^﻿/, '') ?? null;
  return find(lang) ?? find('en');
}

/** `**bold**` and `` `code` `` inside a line, as React nodes (nothing is read as HTML). */
function inline(text: string): React.ReactNode[] {
  const out: React.ReactNode[] = [];
  const re = /(\*\*[^*]+\*\*|`[^`]+`)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const token = m[0];
    out.push(
      token.startsWith('**') ? (
        <b key={m.index} className="text-slate-100">{token.slice(2, -2)}</b>
      ) : (
        <code key={m.index} className="px-1 rounded bg-slate-800 text-emerald-300 text-[0.92em]">{token.slice(1, -1)}</code>
      )
    );
    last = m.index + token.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

/** The small part of Markdown the help pages use: headings, paragraphs, lists. */
export const MiniMarkdown: React.FC<{ text: string }> = ({ text }) => {
  const blocks = useMemo(() => {
    const out: React.ReactNode[] = [];
    let para: string[] = [];
    let list: { ordered: boolean; items: string[] } | null = null;
    const flush = () => {
      if (para.length) {
        out.push(<p key={out.length} className="text-[12.5px] leading-relaxed text-slate-300">{inline(para.join(' '))}</p>);
        para = [];
      }
      if (list) {
        const items = list.items.map((it, i) => <li key={i}>{inline(it)}</li>);
        out.push(
          list.ordered ? (
            <ol key={out.length} className="list-decimal pl-5 space-y-1 text-[12.5px] text-slate-300">{items}</ol>
          ) : (
            <ul key={out.length} className="list-disc pl-5 space-y-1 text-[12.5px] text-slate-300">{items}</ul>
          )
        );
        list = null;
      }
    };
    for (const raw of text.replace(/\r/g, '').split('\n')) {
      const line = raw.trimEnd();
      const heading = line.match(/^(#{1,3})\s+(.*)$/);
      const bullet = line.match(/^\s*[-*]\s+(.*)$/);
      const numbered = line.match(/^\s*\d+[.)]\s+(.*)$/);
      if (heading) {
        flush();
        const level = heading[1].length;
        const cls = level === 1 ? 'text-lg font-bold text-slate-100' : level === 2 ? 'text-sm font-bold text-emerald-300 pt-2' : 'text-[13px] font-bold text-slate-200 pt-1';
        out.push(<div key={out.length} className={cls}>{inline(heading[2])}</div>);
      } else if (bullet || numbered) {
        const ordered = !bullet;
        if (para.length || (list && list.ordered !== ordered)) flush();
        if (!list) list = { ordered, items: [] };
        list.items.push((bullet ?? numbered)![1]);
      } else if (line.trim() === '') {
        flush();
      } else if (list && /^\s{2,}/.test(raw)) {
        // A list item that goes on over the next line.
        list.items[list.items.length - 1] += ' ' + line.trim();
      } else {
        if (list) flush();
        para.push(line.trim());
      }
    }
    flush();
    return out;
  }, [text]);
  return <div className="space-y-2.5">{blocks}</div>;
};

/** The "?" button in the corner and the page it opens over the right side. */
export const HelpButton: React.FC<{ tab: ActiveTab }> = ({ tab }) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [page, setPage] = useState<string>(PAGE_OF_TAB[tab]);

  useEffect(() => {
    setPage(PAGE_OF_TAB[tab]);
  }, [tab]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'F1') {
        e.preventDefault();
        setOpen((o) => !o);
      } else if (e.key === 'Escape') {
        setOpen(false);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const text = helpPage(page, i18next.language) ?? t('help.missing');

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="fixed bottom-4 right-4 z-40 w-9 h-9 rounded-full bg-slate-800 hover:bg-emerald-700 border border-slate-600 text-slate-200 flex items-center justify-center shadow-lg cursor-pointer"
        title={t('help.button')}
        aria-label={t('help.button')}
      >
        <HelpCircle className="w-5 h-5" aria-hidden="true" />
      </button>
      {open && (
        <aside className="fixed top-14 right-0 bottom-0 z-50 w-full max-w-[520px] bg-slate-950/98 border-l border-slate-700 shadow-2xl flex flex-col" role="dialog" aria-label={t('help.title')}>
          <div className="flex items-center justify-between gap-2 px-4 py-2.5 border-b border-slate-800">
            <div className="flex items-center gap-2 text-xs font-bold text-slate-300">
              <HelpCircle className="w-4 h-4 text-emerald-400" aria-hidden="true" /> {t('help.title')}
              {page !== 'index' && (
                <button type="button" onClick={() => setPage('index')} className="ml-2 text-[11px] font-normal text-slate-400 hover:text-slate-200 underline cursor-pointer">
                  {t('help.allScreens')}
                </button>
              )}
              {page === 'index' && PAGE_OF_TAB[tab] !== 'index' && (
                <button type="button" onClick={() => setPage(PAGE_OF_TAB[tab])} className="ml-2 text-[11px] font-normal text-slate-400 hover:text-slate-200 underline cursor-pointer">
                  {t('help.thisScreen')}
                </button>
              )}
            </div>
            <button
              type="button"
              onClick={() => {
                setOpen(false);
                window.dispatchEvent(new Event(SHOW_WELCOME_EVENT));
              }}
              className="ml-auto text-[11px] text-emerald-400 hover:text-emerald-300 underline cursor-pointer"
            >
              {t('welcome.again')}
            </button>
            <button type="button" onClick={() => setOpen(false)} className="p-1 rounded hover:bg-slate-800 text-slate-400 cursor-pointer" aria-label={t('help.close')}>
              <X className="w-4 h-4" aria-hidden="true" />
            </button>
          </div>
          <div className="flex-1 overflow-y-auto px-5 py-4">
            <MiniMarkdown text={text} />
          </div>
        </aside>
      )}
    </>
  );
};
