import React, { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

/**
 * Plain messages (Rust's, the locale's) made easier to read: mod names in bold, paths and file
 * names in monospace, quoted text and arrows picked out, a "Head: a -> b, c -> d" run turned into
 * a list, line breaks kept, and a long text folded behind "show more".
 */

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

const PATTERNS = [
  // C:\Users\me\Zomboid\mods\X, D:\Apps\Steam\steamapps\workshop\content\108600\123\mods\X
  String.raw`[A-Za-z]:\\[^\s,;:'"()<>]+`,
  // Zomboid/mods, media/lua/client/X.lua, ISUI/ISVehicleMechanics
  String.raw`(?:[\w.-]+/)+[\w.-]*[\w-]`,
  // mod.info, ClaudeAgent.lua, default.txt
  String.raw`\b[\w-]+\.(?:lua|txt|info|json|xml|png|ogg|bank|ini)\b`,
  // "ItemType =", 'x'
  String.raw`"[^"\n]{1,90}"`,
  // [verified on 42.21]
  String.raw`\[[^\]\n]{1,40}\]`,
  String.raw`->|→`,
  // -modfolders mods,workshop,steam, -debug
  String.raw`(?<![\w-])-[a-z]\w+(?: \w+(?:,\w+)+)?`,
];

type Kind = 'path' | 'quote' | 'tag' | 'arrow' | 'mod';

const classify = (token: string, mods: Set<string>): Kind => {
  if (mods.has(token)) return 'mod';
  if (token === '->' || token === '→') return 'arrow';
  if (token.startsWith('"')) return 'quote';
  if (token.startsWith('[')) return 'tag';
  return 'path';
};

const STYLE: Record<Kind, string> = {
  path: 'font-mono text-[0.92em] text-sky-300 bg-slate-800/70 rounded px-1 break-all',
  quote: 'font-mono text-[0.92em] text-amber-200',
  tag: 'text-[0.9em] font-mono text-emerald-400/90',
  arrow: 'text-cyan-400 font-bold px-0.5',
  mod: 'font-bold text-slate-50',
};

function inline(text: string, re: RegExp, mods: Set<string>, keyBase: string): React.ReactNode[] {
  const out: React.ReactNode[] = [];
  let last = 0;
  let n = 0;
  for (const m of text.matchAll(re)) {
    const at = m.index ?? 0;
    if (at > last) out.push(text.slice(last, at));
    const token = m[0];
    const kind = classify(token, mods);
    out.push(
      <span key={`${keyBase}-${n++}`} className={STYLE[kind]}>
        {kind === 'arrow' ? '→' : token}
      </span>
    );
    last = at + token.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

/** "Head: a -> b, c -> d." as the head and its pairs, or null when the line is not one. */
function arrowList(line: string): { head: string; items: string[]; tail: string } | null {
  const colon = line.indexOf(': ');
  if (colon < 0) return null;
  const rest = line.slice(colon + 2);
  const end = rest.match(/^(.*?)(\.?\s*)$/s);
  const body = end ? end[1] : rest;
  const items = body.split(/,\s+/);
  if (items.length < 2 || !items.every((i) => i.includes('->'))) return null;
  return { head: line.slice(0, colon + 1), items, tail: '' };
}

/** "First. Second? Third" -> sentences, keeping their stops; paths' dots do not split. */
const sentences = (text: string) => text.match(/[^.!?\n]+(?:[.!?]+(?=\s|$)|$)\s*/g) ?? [text];

interface RichTextProps {
  text: string;
  /** Mod names and ids to set in bold. */
  mods?: Iterable<string>;
  /** Fold what follows this many characters behind "show more"; 0 = never. */
  foldAt?: number;
  className?: string;
}

export const RichText: React.FC<RichTextProps> = ({ text, mods, foldAt = 0, className = '' }) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const modSet = useMemo(() => new Set([...(mods ?? [])].filter((m) => m && m.length >= 3)), [mods]);
  const re = useMemo(() => {
    const names = [...modSet].sort((a, b) => b.length - a.length).map(escape);
    // A name counts only as a whole word, so "UI" inside "CleanUI" stays plain.
    const named = names.length ? [`(?<![\\w])(?:${names.join('|')})(?![\\w])`] : [];
    return new RegExp([...named, ...PATTERNS].join('|'), 'g');
  }, [modSet]);

  // Where to fold: at the end of the sentence that crosses foldAt, if enough is left to hide.
  const { shown, hidden } = useMemo(() => {
    if (!foldAt || text.length <= foldAt * 1.3) return { shown: text, hidden: '' };
    let cut = 0;
    for (const s of sentences(text)) {
      cut += s.length;
      if (cut >= foldAt) break;
    }
    return cut >= text.length - 20 ? { shown: text, hidden: '' } : { shown: text.slice(0, cut).trimEnd(), hidden: text.slice(cut).trimStart() };
  }, [text, foldAt]);

  const render = (part: string, key: string) =>
    part.split('\n').map((line, i) => {
      if (!line.trim()) return <div key={`${key}-${i}`} className="h-1.5" />;
      const list = arrowList(line);
      if (list) {
        return (
          <div key={`${key}-${i}`}>
            {inline(list.head, re, modSet, `${key}-${i}h`)}
            <ul className="mt-0.5 ml-1 space-y-0.5">
              {list.items.map((item, j) => (
                <li key={j} className="flex gap-1.5">
                  <span className="text-slate-600 select-none">-</span>
                  <span className="min-w-0">{inline(item, re, modSet, `${key}-${i}-${j}`)}</span>
                </li>
              ))}
            </ul>
          </div>
        );
      }
      return <div key={`${key}-${i}`}>{inline(line, re, modSet, `${key}-${i}`)}</div>;
    });

  return (
    <div className={`break-words ${className}`}>
      {render(shown, 's')}
      {hidden && open && render(hidden, 'h')}
      {hidden && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            setOpen((v) => !v);
          }}
          className="mt-0.5 text-[0.92em] font-semibold text-cyan-400 hover:text-cyan-300 cursor-pointer"
        >
          {open ? t('common.showLess') : t('common.showMore')}
        </button>
      )}
    </div>
  );
};
