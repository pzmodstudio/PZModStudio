/**
 * Updates the player marked as seen in What's New: the sidebar's count leaves them out, the page
 * still shows them (dimmed, "Seen"). Kept per update, not per mod: the next update of the same
 * mod (or one now waiting in Steam) is new again. Only on this PC (localStorage).
 */
const KEY = 'pzms.whatsNew.seen';
export const SEEN_EVENT = 'pzms-whatsnew-seen';

export const updateKey = (item: { workshop_id: string; installed_time: number | null; latest_time: number | null; pending: boolean }) =>
  `${item.workshop_id}@${item.pending ? `p${item.latest_time ?? 0}` : item.installed_time ?? 0}`;

export const readSeen = (): Set<string> => {
  try {
    const raw = localStorage.getItem(KEY);
    return new Set(raw ? (JSON.parse(raw) as string[]) : []);
  } catch {
    return new Set();
  }
};

export const markSeen = (keys: string[]) => {
  const seen = readSeen();
  keys.forEach((k) => seen.add(k));
  try {
    // The newest 2000 are plenty: older ones belong to updates long replaced.
    localStorage.setItem(KEY, JSON.stringify(Array.from(seen).slice(-2000)));
  } catch {
    // only a convenience
  }
  window.dispatchEvent(new Event(SEEN_EVENT));
};

export const unmarkSeen = (key: string) => {
  const seen = readSeen();
  seen.delete(key);
  try {
    localStorage.setItem(KEY, JSON.stringify(Array.from(seen)));
  } catch {
    // only a convenience
  }
  window.dispatchEvent(new Event(SEEN_EVENT));
};
