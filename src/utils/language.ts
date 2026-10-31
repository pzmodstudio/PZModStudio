import i18next from 'i18next';

/** The app's language as a two-letter code: what translations are asked in. */
export function appLanguage(): string {
  return (i18next.resolvedLanguage || i18next.language || 'en').slice(0, 2).toLowerCase();
}

/**
 * The language a text's script gives away (Cyrillic, kana, hangul, Han, Thai, Arabic), or null
 * for the Latin alphabet and for texts where Latin letters dominate. Mirrors the Rust side.
 */
export function scriptLanguage(text: string): string | null {
  let cyr = 0, kana = 0, hangul = 0, han = 0, thai = 0, arabic = 0, latin = 0;
  for (const ch of text) {
    const c = ch.codePointAt(0) ?? 0;
    if (c >= 0x0400 && c <= 0x04ff) cyr++;
    else if (c >= 0x3040 && c <= 0x30ff) kana++;
    else if ((c >= 0xac00 && c <= 0xd7af) || (c >= 0x1100 && c <= 0x11ff)) hangul++;
    else if ((c >= 0x4e00 && c <= 0x9fff) || (c >= 0x3400 && c <= 0x4dbf)) han++;
    else if (c >= 0x0e00 && c <= 0x0e7f) thai++;
    else if (c >= 0x0600 && c <= 0x06ff) arabic++;
    else if ((c >= 0x41 && c <= 0x5a) || (c >= 0x61 && c <= 0x7a)) latin++;
  }
  const other = cyr + kana + hangul + han + thai + arabic;
  if (other === 0 || other * 4 < latin) return null;
  // The script with most letters wins: a Russian description that quotes one Chinese name is
  // Russian. Japanese mixes kana and Han, so they count together when there is kana enough.
  const japanese = kana > 0 && kana * 10 >= han ? kana + han : 0;
  const counts: [string, number][] = [['ja', japanese], ['ko', hangul], ['zh', japanese ? 0 : han], ['ru', cyr], ['th', thai], ['ar', arabic]];
  let best: [string, number] = ['', 0];
  for (const c of counts) if (c[1] > best[1]) best = c;
  return best[0] || null;
}

/** "ruso" for "ru", in the app's language; the code itself when the browser has no name for it. */
export function languageName(code: string): string {
  if (!code) return '';
  try {
    return new Intl.DisplayNames([appLanguage()], { type: 'language' }).of(code) ?? code;
  } catch {
    return code;
  }
}

export const TRANSLATION_SERVICES: Record<string, string> = { google: 'Google Translate', deepl: 'DeepL', mymemory: 'MyMemory' };

/** A failed translation in words: Google's "too many requests" gets its own explanation. */
export function translateErrorText(error: string): string {
  return /Google answered 429/.test(error)
    ? i18next.t('browse.detail.translateLimited')
    : i18next.t('browse.detail.translateFailed', { reason: error });
}
