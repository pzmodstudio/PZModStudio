import i18next from 'i18next';

/**
 * What Rust writes, in the chosen language. Rust keeps its English text and sends beside it a
 * code and the values the text is made of; a code this file's locale knows is said in the
 * locale, anything else stays in English.
 *
 * - The key is `backendText.<area>.<code>`, or `<code>_<variant>` when `params.variant` is set,
 *   or `params.template` itself (several codes sharing one text).
 * - A param `fooKey` names a phrase under `backendText.fragments` that becomes `{{foo}}`; empty
 *   means `{{foo}}` is empty (or keeps its own value when Rust sent one). Phrases may use other
 *   phrases, one level deep.
 * - A param `fooVs` is a list (joined by U+001F) that becomes `{{foo}}`, joined by "against".
 * - A param `fooList` is a list of codes (comma-separated); each code's fragment, joined by commas, becomes `{{foo}}`.
 */
export function backendText(area: string, code: string | undefined, params: Record<string, string> | undefined, english: string, part?: string): string {
  if (!code || i18next.resolvedLanguage === 'en') return english;
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
      if (v) out[name] = i18next.t(`backendText.fragments.${v}`, from);
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
export const issueText = (issue: { code: string; params?: Record<string, string>; message: string }) =>
  backendText('health', issue.code, issue.params, issue.message);

/** A Monitor card's title, explanation or recommended action. */
export const cardText = (
  card: { code?: string; params?: Record<string, string>; title: string; explanation: string; recommended_action: string },
  part: 'title' | 'explanation' | 'action',
) =>
  backendText('monitor', card.code, card.params, part === 'title' ? card.title : part === 'explanation' ? card.explanation : card.recommended_action, part);

/** A validation problem the Studio found itself (the parser's own words stay as written). */
export const problemText = (p: { code?: string; message: string }) => backendText('validate', p.code, undefined, p.message);
