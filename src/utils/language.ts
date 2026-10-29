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
  if (kana > 0 && kana * 10 >= han) return 'ja';
  if (hangul > 0) return 'ko';
  if (han > 0) return 'zh';
  if (cyr >= thai && cyr >= arabic) return 'ru';
  if (thai >= arabic) return 'th';
  return 'ar';
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
