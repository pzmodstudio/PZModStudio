import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from '../locales/en.json';
import es from '../locales/es.json';

/**
 * The app's languages. English is the source; a language is one JSON under src/locales, and a
 * missing key falls back to English, so a half-translated language still works. The choice is
 * kept in localStorage; the first start follows the system's language.
 */
export const LANGUAGES: { code: string; name: string }[] = [
  { code: 'en', name: 'English' },
  { code: 'es', name: 'Español' },
];

const STORAGE_KEY = 'pz_language';

function startingLanguage(): string {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved && LANGUAGES.some((l) => l.code === saved)) return saved;
  } catch {
    // no storage: the system's language
  }
  const system = (navigator.language || 'en').slice(0, 2).toLowerCase();
  return LANGUAGES.some((l) => l.code === system) ? system : 'en';
}

void i18next.use(initReactI18next).init({
  resources: { en: { translation: en }, es: { translation: es } },
  lng: startingLanguage(),
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
  returnEmptyString: false,
});

export function setLanguage(code: string) {
  try {
    localStorage.setItem(STORAGE_KEY, code);
  } catch {
    // nothing to keep it in; the session has it
  }
  void i18next.changeLanguage(code);
}

export const currentLanguage = () => i18next.language;

export default i18next;
