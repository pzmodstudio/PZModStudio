import React, { useEffect, useMemo, useRef } from 'react';
import { VfsConflict, ChangeRegion, ModChangeSummary } from '../../types';
import { Info } from 'lucide-react';
import i18next from 'i18next';
import { tHtml } from '../../i18n/html';

/** Lines shown around the point of interest; big vanilla files have tens of thousands. */
const VIEW_WINDOW = 200;

interface LinedCodeSnippetProps {
  content: string;
  /** Line to scroll to and mark (1-based). */
  targetLine: number;
  /** 1-based inclusive line ranges to highlight as this side's changes. */
  highlightRanges?: Array<[number, number]>;
  /** Ranges to mark as collisions (shown in amber). */
  collisionRanges?: Array<[number, number]>;
}

const inRanges = (n: number, ranges?: Array<[number, number]>) => !!ranges && ranges.some(([a, b]) => n >= a && n <= Math.max(a, b));

/** Read-only code view. Module-level and memoized so switching conflicts does not rebuild it,
 *  and windowed so huge files only render ~200 lines. */
export const LinedCodeSnippet = React.memo(function LinedCodeSnippet({ content, targetLine, highlightRanges, collisionRanges }: LinedCodeSnippetProps) {
  const lines = useMemo(() => content.split('\n'), [content]);
  const targetRef = useRef<HTMLDivElement>(null);
  const from = Math.max(0, Math.min(targetLine - 1 - VIEW_WINDOW / 2, lines.length - VIEW_WINDOW));
  const to = Math.min(lines.length, from + VIEW_WINDOW);

  useEffect(() => {
    targetRef.current?.scrollIntoView({ block: 'center' });
  }, [targetLine, content]);

  return (
    <div className="flex-1 flex flex-col overflow-hidden rounded-b-lg">
      {lines.length > VIEW_WINDOW && (
        <div className="text-[10px] font-mono text-slate-500 bg-slate-900 px-2 py-0.5 border-b border-slate-800 shrink-0">
          {i18next.t('merger.view.window', { from: from + 1, to, total: lines.length.toLocaleString() })}
        </div>
      )}
      <div className="flex-1 overflow-auto text-xs font-mono bg-slate-950 select-text">
        {lines.slice(from, to).map((line, i) => {
          const n = from + i + 1;
          const isTarget = n === targetLine;
          const isCollision = inRanges(n, collisionRanges);
          const isChange = inRanges(n, highlightRanges);
          const style = isCollision
            ? 'bg-amber-500/25 text-amber-100 border-amber-400'
            : isChange
            ? 'bg-emerald-950/50 text-emerald-200 border-emerald-500/80'
            : 'text-slate-300 border-transparent';
          return (
            <div key={n} ref={isTarget ? targetRef : null} className={`flex leading-6 h-6 border-l-4 ${style} ${isTarget ? 'ring-1 ring-inset ring-amber-400/70' : ''}`}>
              <span className="w-12 shrink-0 text-right pr-2 text-[11px] text-slate-500 select-none">{n}</span>
              <span className="px-2 whitespace-pre text-[12px]">{line || ' '}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
});

const regionLabel = (r: ChangeRegion) =>
  r.kind === 'added'
    ? i18next.t('merger.view.adds', { count: r.mod_end - r.mod_start + 1, line: r.base_start })
    : r.kind === 'removed'
    ? i18next.t('merger.view.removes', { range: `${r.base_start}${r.base_end > r.base_start ? `–${r.base_end}` : ''}` })
    : i18next.t('merger.view.changes', { range: `${r.base_start}${r.base_end > r.base_start ? `–${r.base_end}` : ''}` });

/** Plain summary: how the merge was built, what each mod changes, where they collide. */
export const ConflictSummaryCard: React.FC<{ conflict: VfsConflict; onFocus: (baseLine: number) => void }> = ({ conflict, onFocus }) => {
  const changes = conflict.mod_changes ?? [];
  const collisions = conflict.collisions ?? [];
  const lastLoaded = conflict.competing_mods[conflict.competing_mods.length - 1];
  return (
    <div className="bg-slate-950/90 border border-slate-800 rounded-xl p-3 mb-2 shrink-0 space-y-2 text-[11px] max-h-[45%] overflow-y-auto">
      <div className="flex items-start gap-2 text-slate-200">
        <Info className="w-4 h-4 text-cyan-400 shrink-0 mt-0.5" />
        <span>
          <span dangerouslySetInnerHTML={{ __html: tHtml(
            conflict.file_type === 'TRANSLATION' ? 'merger.view.translationIntro' : conflict.file_type === 'SCRIPT_DEFINITION' ? 'merger.view.definitionIntro' : 'merger.view.fileIntro',
            { count: conflict.competing_mods.length, last: lastLoaded?.mod_name ?? '' }
          ) }} />
          {conflict.strategy}
        </span>
      </div>
      {collisions.length > 0 && (
        <div className="space-y-1">
          <div className="text-amber-300 font-bold">{i18next.t('merger.view.realConflicts', { count: collisions.length })}</div>
          <div className="flex flex-wrap gap-1.5">
            {collisions.map((c, i) => (
              <button key={i} type="button" onClick={() => onFocus(c.base_start)}
                className="px-2 py-0.5 rounded border border-amber-500/50 bg-amber-950/40 text-amber-200 hover:bg-amber-900/60 cursor-pointer font-mono">
                L{c.base_start}{c.base_end > c.base_start ? `–${c.base_end}` : ''}: {c.mods.join(' vs ')}
              </button>
            ))}
          </div>
        </div>
      )}
      {changes.length > 0 && (
        <div className="space-y-1">
          <div className="text-slate-400 font-bold">{i18next.t('merger.view.whatEachChanges')}</div>
          {changes.map((m) => (
            <div key={m.mod_id} className="flex flex-wrap items-center gap-1.5">
              <span className="font-bold text-slate-200 mr-1">{m.mod_name}</span>
              <span className="font-mono text-emerald-300">+{m.lines_added}</span>
              <span className="font-mono text-red-300">−{m.lines_removed}</span>
              <span className="text-slate-500">{i18next.t('merger.view.inPlaces', { count: m.regions.length })}</span>
              {m.regions.slice(0, 12).map((r, i) => (
                <button key={i} type="button" onClick={() => onFocus(r.base_start)}
                  className="px-1.5 py-0.5 rounded border border-slate-700 bg-slate-900 text-slate-300 hover:bg-slate-800 cursor-pointer font-mono text-[10px]">
                  {regionLabel(r)}
                </button>
              ))}
              {m.regions.length > 12 && <span className="text-slate-500">{i18next.t('merger.view.more', { count: m.regions.length - 12 })}</span>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

/** The line in a mod's own file that corresponds to vanilla line `baseLine`. */
export const modLineFor = (summary: ModChangeSummary | undefined, baseLine: number): number => {
  if (!summary) return baseLine;
  let offset = 0;
  for (const r of summary.regions) {
    if (baseLine < r.base_start) break;
    if (baseLine <= Math.max(r.base_start, r.base_end) || r.kind === 'added' && baseLine === r.base_start) return r.mod_start;
    offset += (r.mod_end - r.mod_start + 1) - (r.base_end - r.base_start + 1);
  }
  return Math.max(1, baseLine + offset);
};
