import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';

/**
 * The app's languages. English is the source; a language is one JSON under src/locales, and a
 * missing key falls back to English, so a half-translated language still works. The choice is
 * kept in localStorage; the first start follows the system's language.
 */
export const LANGUAGES: { code: string; name: string }[] = [
  { code: 'en', name: 'English' },
  { code: 'es', name: 'Español' },
  { code: 'pt-BR', name: 'Português (Brasil)' },
  { code: 'fr', name: 'Français' },
  { code: 'de', name: 'Deutsch' },
  { code: 'ru', name: 'Русский' },
  { code: 'zh-CN', name: '简体中文' },
];

const FILES = import.meta.glob('../locales/*.json', { eager: true, import: 'default' }) as Record<string, Record<string, unknown>>;
const resources: Record<string, { translation: Record<string, unknown> }> = {};
for (const [path, translation] of Object.entries(FILES)) {
  const code = path.replace(/^.*\/([^/]+)\.json$/, '$1');
  resources[code] = { translation };
}

/** The languages whose file is in the build (a language listed before its file lands stays hidden). */
export const AVAILABLE_LANGUAGES = LANGUAGES.filter((l) => resources[l.code]);

const STORAGE_KEY = 'pz_language';

/** A language tag ("pt-PT", "zh-Hans-CN", "de-AT") as one of the app's codes, or null. */
export function matchLanguage(tag: string | null | undefined): string | null {
  if (!tag) return null;
  const lower = tag.toLowerCase();
  const exact = AVAILABLE_LANGUAGES.find((l) => l.code.toLowerCase() === lower);
  if (exact) return exact.code;
  const base = lower.split(/[-_]/)[0];
  // Portuguese of any country reads the Brazilian one; Chinese is only Simplified here, so a
  // Traditional system (zh-TW, zh-HK, zh-Hant) stays in English.
  if (base === 'zh' && /hant|tw|hk|mo/.test(lower)) return null;
  const same = AVAILABLE_LANGUAGES.find((l) => l.code.toLowerCase().split('-')[0] === base);
  return same ? same.code : null;
}

function startingLanguage(): string {
  try {
    const saved = matchLanguage(localStorage.getItem(STORAGE_KEY));
    if (saved) return saved;
  } catch {
    // no storage: the system's language
  }
  for (const tag of navigator.languages?.length ? navigator.languages : [navigator.language]) {
    const found = matchLanguage(tag);
    if (found) return found;
  }
  return 'en';
}

void i18next.use(initReactI18next).init({
  resources,
  lng: startingLanguage(),
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
  returnEmptyString: false,
});

// The page says its language, so Windows picks the right font for Chinese characters and the
// spell checker the right dictionary.
const tagPage = (code: string) => {
  if (typeof document !== 'undefined') document.documentElement.lang = code;
};
tagPage(i18next.language);
i18next.on('languageChanged', tagPage);

export function setLanguage(code: string) {
  try {
    localStorage.setItem(STORAGE_KEY, code);
  } catch {
    // nothing to keep it in; the session has it
  }
  void i18next.changeLanguage(code);
}

export const currentLanguage = () => i18next.language;

/** The app's code for the language in use ("pt-BR", "es"...), for pickers. */
export const currentLanguageCode = () => matchLanguage(i18next.resolvedLanguage || i18next.language) ?? 'en';

export default i18next;
