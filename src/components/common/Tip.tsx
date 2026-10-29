import React, { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { MousePointerClick } from 'lucide-react';

/** How long the pointer rests on something before its tip opens, and how long the tip waits for the pointer to come over. */
const OPEN_DELAY = 300;
const CLOSE_GRACE = 160;

interface TipProps {
  /** What the tip says: plain text or a TipBody. Nothing = no tip. */
  content: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}

/**
 * A tooltip that can hold formatting -- names in bold, paths in monospace, the click's action on
 * its own line -- where the browser's `title` only takes plain text. It opens below the element
 * (above when there is no room), stays open while the pointer is over it so a "show more" in it
 * can be clicked, and also opens on keyboard focus.
 */
export const Tip: React.FC<TipProps> = ({ content, children, className = '' }) => {
  const anchor = useRef<HTMLSpanElement>(null);
  const box = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);
  const timer = useRef<number | undefined>(undefined);

  const later = (fn: () => void, ms: number) => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(fn, ms);
  };
  const show = () => later(() => setOpen(true), OPEN_DELAY);
  const hide = () => later(() => setOpen(false), CLOSE_GRACE);
  const stay = () => window.clearTimeout(timer.current);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  const place = useCallback(() => {
    const a = anchor.current?.getBoundingClientRect();
    const b = box.current?.getBoundingClientRect();
    if (!a || !b) return;
    const margin = 8;
    const left = Math.max(margin, Math.min(a.left, window.innerWidth - b.width - margin));
    const below = a.bottom + 6;
    const top = below + b.height + margin > window.innerHeight ? Math.max(margin, a.top - b.height - 6) : below;
    setPos({ left, top });
  }, []);
  useLayoutEffect(() => {
    if (open) place();
    else setPos(null);
  }, [open, place]);

  if (content === null || content === undefined || content === '' || content === false) return <>{children}</>;
  return (
    <span
      ref={anchor}
      className={`inline-flex ${className}`}
      onMouseEnter={show}
      onMouseLeave={hide}
      onFocus={show}
      onBlur={hide}
      onMouseDown={() => {
        window.clearTimeout(timer.current);
        setOpen(false);
      }}
    >
      {children}
      {open &&
        createPortal(
          <div
            ref={box}
            role="tooltip"
            onMouseEnter={stay}
            onMouseLeave={hide}
            // The tip lives in the anchor's React tree: keep its clicks from reaching the row or
            // closing the tip itself.
            onClick={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
            style={{ left: pos?.left ?? -9999, top: pos?.top ?? -9999 }}
            className="fixed z-[100] max-w-[440px] rounded-lg border border-slate-700 bg-slate-950/95 backdrop-blur px-3 py-2 text-[11.5px] leading-relaxed text-slate-300 shadow-2xl shadow-black/60 normal-case tracking-normal font-sans font-normal text-left whitespace-normal"
          >
            {content}
          </div>,
          document.body
        )}
    </span>
  );
};

const TONE: Record<string, string> = {
  red: 'text-red-300',
  amber: 'text-amber-300',
  emerald: 'text-emerald-300',
  sky: 'text-sky-300',
  cyan: 'text-cyan-300',
  slate: 'text-slate-100',
  violet: 'text-violet-300',
  orange: 'text-orange-300',
  teal: 'text-teal-300',
};

export type TipTone = keyof typeof TONE;

/** The usual shape of a tip: a coloured title, the explanation, and what a click does. */
export const TipBody: React.FC<{ title?: React.ReactNode; tone?: TipTone; children?: React.ReactNode; action?: React.ReactNode; footnote?: React.ReactNode }> = ({
  title,
  tone = 'slate',
  children,
  action,
  footnote,
}) => (
  <div className="flex flex-col gap-1.5">
    {title && <div className={`font-bold text-[12px] ${TONE[tone]}`}>{title}</div>}
    {children && <div className="text-slate-300">{children}</div>}
    {footnote && <div className="text-[10.5px] text-slate-500 border-t border-slate-800 pt-1.5">{footnote}</div>}
    {action && (
      <div className="flex items-center gap-1.5 text-[11px] font-semibold text-cyan-300 border-t border-slate-800 pt-1.5">
        <MousePointerClick className="w-3.5 h-3.5 shrink-0" aria-hidden="true" />
        <span>{action}</span>
      </div>
    )}
  </div>
);
