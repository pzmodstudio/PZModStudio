import React, { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react';

/**
 * The app's own right-click menu.
 *
 * A webview comes with a browser menu (Reload, Inspect, Back) that has nothing to do with a mod
 * manager, so it is turned off everywhere except in text fields, where cut/copy/paste is the
 * point. Anywhere the app has something useful to offer, it offers that instead.
 */

export interface MenuItem {
  label: string;
  onSelect: () => void;
  icon?: React.ElementType;
  /** Shown greyed out, with the reason on hover. */
  disabled?: boolean;
  disabledReason?: string;
  /** A line on hover saying what the item does, when the label alone is not enough. */
  title?: string;
  /** Draws a line above this item. */
  separatorBefore?: boolean;
  /** Red, for anything that removes or undoes something. */
  danger?: boolean;
}

type ShowMenu = (event: React.MouseEvent, items: MenuItem[]) => void;

const ContextMenuContext = createContext<ShowMenu>(() => undefined);

/** `showContextMenu(event, items)` — call it from an element's `onContextMenu`. */
export const useContextMenu = () => useContext(ContextMenuContext);

const isEditable = (target: EventTarget | null): boolean => {
  const el = target as HTMLElement | null;
  if (!el || !el.closest) return false;
  return Boolean(el.closest('input, textarea, [contenteditable="true"]'));
};

interface OpenMenu {
  x: number;
  y: number;
  items: MenuItem[];
}

export const ContextMenuProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const showMenu = useCallback<ShowMenu>((event, items) => {
    if (items.length === 0) return;
    event.preventDefault();
    event.stopPropagation();
    setMenu({ x: event.clientX, y: event.clientY, items });
  }, []);

  // The browser's own menu stays only where typing happens.
  useEffect(() => {
    const onContextMenu = (e: MouseEvent) => {
      if (!isEditable(e.target)) e.preventDefault();
    };
    document.addEventListener('contextmenu', onContextMenu);
    return () => document.removeEventListener('contextmenu', onContextMenu);
  }, []);

  // Anything else that happens closes it: a click, Escape, a scroll, a resize.
  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    window.addEventListener('click', close);
    window.addEventListener('resize', close);
    window.addEventListener('blur', close);
    window.addEventListener('keydown', onKey);
    document.addEventListener('scroll', close, true);
    return () => {
      window.removeEventListener('click', close);
      window.removeEventListener('resize', close);
      window.removeEventListener('blur', close);
      window.removeEventListener('keydown', onKey);
      document.removeEventListener('scroll', close, true);
    };
  }, [menu]);

  // Keep it on screen: near the right or bottom edge it opens the other way.
  useEffect(() => {
    const el = menuRef.current;
    if (!el || !menu) return;
    const { width, height } = el.getBoundingClientRect();
    const x = Math.min(menu.x, window.innerWidth - width - 8);
    const y = Math.min(menu.y, window.innerHeight - height - 8);
    el.style.left = `${Math.max(8, x)}px`;
    el.style.top = `${Math.max(8, y)}px`;
  }, [menu]);

  return (
    <ContextMenuContext.Provider value={showMenu}>
      {children}
      {menu && (
        <div
          ref={menuRef}
          role="menu"
          className="fixed z-[100] min-w-[200px] py-1 rounded-lg border border-slate-700 bg-slate-900 shadow-2xl text-xs animate-fade-in"
          style={{ left: menu.x, top: menu.y }}
          onClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
        >
          {menu.items.map((item, i) => {
            const Icon = item.icon;
            return (
              <React.Fragment key={`${item.label}-${i}`}>
                {item.separatorBefore && <div className="my-1 border-t border-slate-800" />}
                <button
                  type="button"
                  role="menuitem"
                  disabled={item.disabled}
                  title={item.disabled ? item.disabledReason : item.title}
                  onClick={() => {
                    setMenu(null);
                    item.onSelect();
                  }}
                  className={`w-full flex items-center gap-2 px-3 py-1.5 text-left transition cursor-pointer disabled:cursor-default disabled:text-slate-600 ${
                    item.danger ? 'text-rose-300 hover:bg-rose-950/60' : 'text-slate-200 hover:bg-slate-800'
                  }`}
                >
                  {Icon && <Icon className="w-3.5 h-3.5 shrink-0 opacity-80" aria-hidden="true" />}
                  <span className="truncate">{item.label}</span>
                </button>
              </React.Fragment>
            );
          })}
        </div>
      )}
    </ContextMenuContext.Provider>
  );
};
