import { describe, expect, it } from 'vitest';
import en from '../locales/en.json';
import es from '../locales/es.json';

/** Every key, dotted, with its text. */
const flatten = (obj: unknown, prefix = ''): Record<string, string> => {
  const out: Record<string, string> = {};
  if (obj && typeof obj === 'object') {
    for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
      const key = prefix ? `${prefix}.${k}` : k;
      if (typeof v === 'string') out[key] = v;
      else Object.assign(out, flatten(v, key));
    }
  }
  return out;
};

describe('locales', () => {
  const enKeys = flatten(en);
  const esKeys = flatten(es);

  it('Spanish has no key English lacks (English is the source)', () => {
    const extra = Object.keys(esKeys).filter((k) => !(k in enKeys));
    expect(extra).toEqual([]);
  });

  it('every English line has its Spanish, so nothing falls back by accident', () => {
    const missing = Object.keys(enKeys).filter((k) => !(k in esKeys));
    expect(missing).toEqual([]);
  });

  it('a line keeps the same placeholders in both languages', () => {
    const vars = (s: string) => (s.match(/\{\{\w+\}\}/g) ?? []).sort();
    const off = Object.keys(enKeys).filter((k) => k in esKeys && vars(enKeys[k]).join() !== vars(esKeys[k]).join());
    expect(off).toEqual([]);
  });
});

// The other languages: whole, with the plural forms each language has, and the same placeholders.
const OTHERS = import.meta.glob('../locales/*.json', { eager: true, import: 'default' }) as Record<string, unknown>;
const PLURAL = /_(zero|one|two|few|many|other)$/;
for (const [path, data] of Object.entries(OTHERS)) {
  const code = path.replace(/^.*\/([^/]+)\.json$/, '$1');
  if (code === 'en' || code === 'es') continue;
  describe(`locale ${code}`, () => {
    const enKeys = flatten(en);
    const keys = flatten(data);
    const base = (k: string) => k.replace(PLURAL, '');
    // a plural family is a key English has with both "_one" and "_other" (a key may just end in
    // "_other" by name: "function_seen_other" is another version seen)
    const enBases = new Set(Object.keys(enKeys).filter((k) => k.endsWith('_other') && `${base(k)}_one` in enKeys).map(base));
    const isPlural = (k: string) => PLURAL.test(k) && enBases.has(base(k));
    const vars = (s: string) => [...new Set(s.match(/\{\{\w+\}\}/g) ?? [])].sort().join();

    it('has no key English lacks', () => {
      const extra = Object.keys(keys).filter((k) => !(k in enKeys) && !isPlural(k));
      expect(extra).toEqual([]);
    });

    it('has every English line and every plural form the language uses', () => {
      const categories = new Intl.PluralRules(code).resolvedOptions().pluralCategories;
      const missing = Object.keys(enKeys).filter((k) => !isPlural(k) && !(k in keys));
      for (const b of enBases) for (const c of categories) if (!(`${b}_${c}` in keys)) missing.push(`${b}_${c}`);
      expect(missing).toEqual([]);
    });

    it('keeps the placeholders of each line', () => {
      const off = Object.keys(keys).filter((k) => {
        const source = enKeys[k] ?? (isPlural(k) ? enKeys[`${base(k)}_other`] : undefined);
        if (source === undefined) return false;
        // a plural form may say the number where English's "one" does not (Russian's "one" is 21 too)
        if (isPlural(k)) return vars(source.replace(/\{\{count\}\}/g, '')) !== vars(keys[k].replace(/\{\{count\}\}/g, ''));
        return vars(source) !== vars(keys[k]);
      });
      expect(off).toEqual([]);
    });
  });
}
