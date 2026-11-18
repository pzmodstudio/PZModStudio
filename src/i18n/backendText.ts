import i18next from 'i18next';
import { onsetText } from '../utils/brain';
import type { BrainOnset } from '../types';

/**
 * What Rust writes, in the chosen language. Rust keeps its English text and sends beside it a
 * code and the values the text is made of; a code this file's locale knows is said in the
 * locale, anything else stays in English.
 *
 * - The key is `backendText.<area>.<code>`, or `<code>_<variant>` when `params.variant` is set,
 *   or `params.template` itself (several codes sharing one text).
 * - A param `fooKey` names a phrase under `backendText.fragments` that becomes `{{foo}}`; empty
 *   means `{{foo}}` is empty (or keeps its own value when Rust sent one). Phrases may use other
 *   phrases, one level deep. A phrase may use the value it replaces (`moreKey` naming a phrase
 *   that says "and {{more}} more"): it is always given the number Rust sent, not its own words.
 * - A param `fooVs` is a list (joined by U+001F) that becomes `{{foo}}`, joined by "against".
 * - A param `fooList` is a list of codes (comma-separated); each code's fragment, joined by commas, becomes `{{foo}}`.
 */
export function backendText(area: string, code: string | undefined, params: Record<string, string> | undefined, english: string, part?: string, inEnglishToo = false): string {
  // English is Rust's own text; `inEnglishToo` says it from the locale anyway (other values than Rust's).
  if (!code || (i18next.resolvedLanguage === 'en' && !inEnglishToo)) return english;
  const p = params ?? {};
  const base = p.template ?? (p.variant ? `${code}_${p.variant}` : code);
  const key = `backendText.${area}.${base}${part ? `.${part}` : ''}`;
  if (!i18next.exists(key)) return english;
  const values: Record<string, string> = { ...p };
  for (const [k, v] of Object.entries(p)) {
    if (k.endsWith('Vs')) values[k.slice(0, -2)] = v.split('\u001f').join(i18next.t('backendText.fragments.vs'));
    // `fooList`: codes, comma-separated, each said by its fragment, joined and deduplicated.
    if (k.endsWith('List')) {
      const said = v.split(',').filter(Boolean).map((c) => (i18next.exists(`backendText.fragments.${c}`) ? i18next.t(`backendText.fragments.${c}`) : c));
      values[k.slice(0, -4)] = [...new Set(said)].join(', ');
    }
  }
  const phrases = (from: Record<string, string>) => {
    const out: Record<string, string> = {};
    for (const [k, v] of Object.entries(p)) {
      if (!k.endsWith('Key') || k.length <= 3) continue;
      const name = k.slice(0, -3);
      if (v) out[name] = i18next.t(`backendText.fragments.${v}`, { ...from, [name]: p[name] ?? '' });
      else if (from[name] === undefined) out[name] = '';
    }
    return out;
  };
  // Twice, so a phrase can hold another ("1 mod is on ... (A, B, and 3 more)").
  Object.assign(values, phrases(values));
  Object.assign(values, phrases(values));
  return i18next.t(key, values);
}

/** A Mod Health or server issue's message. */
export const issueText = (issue: { code: string; params?: Record<string, string>; message: string }) => {
  // What this PC learned: the error is the Monitor card's title, said the way the Monitor says it.
  if (issue.code === 'learned_issue' && issue.params?.cardCode) {
    let cardParams: Record<string, string> = {};
    try {
      cardParams = JSON.parse(issue.params.cardParams || '{}');
    } catch {
      cardParams = {};
    }
    const what = backendText('monitor', issue.params.cardCode, cardParams, issue.params.what ?? '', 'title');
    const text = backendText('health', issue.code, { ...issue.params, what }, issue.message);
    // When it began and what was different then (the brain), with the names it carries.
    let onset = '';
    try {
      const o = issue.params.onsetJson ? (JSON.parse(issue.params.onsetJson) as { onset: BrainOnset; names: Record<string, string> }) : null;
      if (o) onset = onsetText(o.onset, (id) => o.names?.[id] ?? id, i18next.t);
    } catch {
      onset = '';
    }
    return onset ? `${text}. ${onset}` : text;
  }
  return backendText('health', issue.code, issue.params, issue.message);
};

/**
 * A finding said for a beginner (Simple): mods by their names only, never by their ids. Rust writes
 * a mod as "Name (id)" or as a bare id; each such value becomes the name the Mod List shows. Said
 * in the chosen language, English included (the locale's English is Rust's text with the same
 * values), so the ids go in both.
 */
export const plainIssueText = (
  issue: { code: string; params?: Record<string, string>; message: string },
  nameOf: (id: string) => string,
) => {
  if (!issue.params || issue.code === 'learned_issue') return issueText(issue);
  const named = (value: string) => {
    const bare = nameOf(value);
    if (bare !== value) return bare;
    // "Name (id)": the id in brackets is a known mod, so the name alone says it.
    const m = /^(.*\S) \(([^()]+)\)$/.exec(value);
    return m && nameOf(m[2]) !== m[2] ? m[1] : value;
  };
  const params: Record<string, string> = {};
  for (const [k, v] of Object.entries(issue.params)) params[k] = typeof v === 'string' ? named(v) : v;
  // A few findings have shorter words of their own for a beginner (no version ranges, no files).
  const own = [`simple.issueText.${params.variant ? `${issue.code}_${params.variant}` : issue.code}`, `simple.issueText.${issue.code}`].find((k) => i18next.exists(k));
  if (own) return i18next.t(own, params);
  return backendText('health', issue.code, params, issue.message, undefined, true);
};

/** A Monitor card's title, explanation or recommended action. */
export const cardText = (
  card: { code?: string; params?: Record<string, string>; title: string; explanation: string; recommended_action: string },
  part: 'title' | 'explanation' | 'action',
) =>
  backendText('monitor', card.code, card.params, part === 'title' ? card.title : part === 'explanation' ? card.explanation : card.recommended_action, part);

/** A validation problem the Studio found itself (the parser's own words stay as written). */
export const problemText = (p: { code?: string; message: string }) => backendText('validate', p.code, undefined, p.message);
