import i18next from 'i18next';

const ESCAPES: Record<string, string> = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };

export function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ESCAPES[c]);
}

/**
 * A translation meant for `dangerouslySetInnerHTML`: the locale's own markup (`<b>`, `<code>`)
 * is kept and every value put into it is escaped. i18next is set up with `escapeValue: false`
 * for React text, and the values here are mod names from `mod.info`, Workshop titles, Nexus
 * names and fields of imported packs, any of which can carry markup.
 */
export function tHtml(key: string, vars?: Record<string, unknown>): string {
  if (!vars) return i18next.t(key) as string;
  const safe: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(vars)) {
    safe[k] = typeof v === 'string' ? escapeHtml(v) : Array.isArray(v) ? v.map((x) => (typeof x === 'string' ? escapeHtml(x) : x)) : v;
  }
  return i18next.t(key, safe) as string;
}
