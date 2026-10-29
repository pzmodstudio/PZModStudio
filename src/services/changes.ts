import { useEffect, useState } from 'react';

/**
 * What a change touched: `mods` (the installed mods or the game's active list: a subscribe, a
 * package made or deleted, a restored list), `profiles` (a profile made, switched, edited or
 * deleted), `other` (anything else a screen keeps a list of: rules, notes, servers, backups).
 */
export type ChangeScope = 'mods' | 'profiles' | 'other';

type Listener = (scope: ChangeScope) => void;
const listeners = new Set<Listener>();

/**
 * Says that something was written. Every backend call that changes something says it (see the
 * end of services/tauri.ts), so a screen never has to be left and opened again to show what a
 * button just did: the screens read their lists again on the revision below, and App reads the
 * mods and the profiles again for the scopes that touch them.
 */
export function noteChange(scope: ChangeScope = 'other'): void {
  for (const l of [...listeners]) {
    try {
      l(scope);
    } catch (err) {
      console.error('A change listener failed:', err);
    }
  }
}

export function onChange(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * A number that goes up a moment after any change (several in a row count once). Put it in a
 * load effect's dependencies and the screen reads its list again right after the change. With
 * `scopes`, only a change in one of those scopes counts: a screen whose list only the mods
 * touch does not read it again every time a note or a setting is saved.
 */
export function useChangeRevision(scopes?: ChangeScope[]): number {
  const [rev, setRev] = useState(0);
  const key = scopes?.join(',') ?? '';
  useEffect(() => {
    let timer: number | undefined;
    const wanted = key ? new Set(key.split(',')) : null;
    const off = onChange((scope) => {
      if (wanted && !wanted.has(scope)) return;
      window.clearTimeout(timer);
      timer = window.setTimeout(() => setRev((r) => r + 1), 250);
    });
    return () => {
      off();
      window.clearTimeout(timer);
    };
  }, [key]);
  return rev;
}
