/**
 * Remembers what the Workshop and Nexus answered, so moving around the app does not wait for the
 * same request twice. Everything here is public listing data that changes slowly; anything the
 * user changes (installing, merging) clears it through `clearRemoteCache`.
 */

interface Entry {
  value: unknown;
  /** When it stops being used (ms since epoch). */
  expires: number;
}

/** Long enough that going back and forth is instant, short enough to notice a new mod. */
export const CACHE_TTL_MS = 5 * 60 * 1000;
/** Pages of search results are big; keep the most recent ones only. */
const MAX_ENTRIES = 120;

const entries = new Map<string, Entry>();
/** Requests already on their way, so two callers asking at once make one request. */
const inFlight = new Map<string, Promise<unknown>>();

/** Runs `load` unless the same key answered recently; concurrent callers share one request. */
export async function cached<T>(key: string, load: () => Promise<T>, ttlMs = CACHE_TTL_MS): Promise<T> {
  const hit = entries.get(key);
  if (hit && hit.expires > Date.now()) {
    // Keep it as the most recently used.
    entries.delete(key);
    entries.set(key, hit);
    return hit.value as T;
  }
  const running = inFlight.get(key);
  if (running) return running as Promise<T>;

  const request = load()
    .then((value) => {
      entries.set(key, { value, expires: Date.now() + ttlMs });
      if (entries.size > MAX_ENTRIES) {
        // Oldest first: Map keeps insertion order.
        const oldest = entries.keys().next();
        if (!oldest.done) entries.delete(oldest.value);
      }
      return value;
    })
    .finally(() => inFlight.delete(key));

  inFlight.set(key, request);
  return request;
}

/** Warms the cache without waiting for it (used to make "next page" feel instant). */
export function prefetch(key: string, load: () => Promise<unknown>): void {
  if (entries.has(key) || inFlight.has(key)) return;
  void cached(key, load).catch(() => undefined);
}

/**
 * Forgets what was remembered. With a prefix, only those keys (for example "workshop:" after
 * installing something).
 */
export function clearRemoteCache(prefix?: string): void {
  if (!prefix) {
    entries.clear();
    return;
  }
  for (const key of [...entries.keys()]) {
    if (key.startsWith(prefix)) entries.delete(key);
  }
}
