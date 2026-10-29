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
