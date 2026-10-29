import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import i18next from './index';
import { backendText, issueText } from './backendText';

describe('backendText', () => {
  beforeAll(async () => {
    await i18next.changeLanguage('es');
  });
  afterAll(async () => {
    await i18next.changeLanguage('en');
  });

  it('says a known code in the locale, with its values', () => {
    expect(issueText({ code: 'dependency_after', params: { mod: 'A', dep: 'B' }, message: 'A requires B, but B loads after it' })).toBe(
      'A requiere B, pero B carga después',
    );
  });

  it('picks the variant and fills phrases, including one inside another', () => {
    const params = {
      package: 'P', sep: '',
      missingN: '6', missingList: 'A, B, C, D', missingMoreN: '2', missingMoreKey: 'more_missing', missingKey: 'pkg_missing_other',
      extraN: '0', extraList: '', extraMoreN: '0', extraMoreKey: '', extraKey: '',
    };
    const text = issueText({ code: 'package_built_for_other_list', params, message: 'english' });
    expect(text).toContain('P se armó con otra lista: 6 mods están activados en este perfil y el paquete nunca los vio (A, B, C, D, y 2 más). Un paquete');
  });

  it('joins a list with the locale word and keeps a raw value when its phrase is empty', () => {
    const text = backendText('health', 'translation_conflict', {
      template: 'translation', losers: 'A', winner: 'B', lang: 'ES', category: 'ItemName',
      what: 'ItemName_Axe', whatKey: '', n: '1', named: 'ItemName_Axe', ellipsis: '',
      exName: 'ItemName_Axe', exValuesVs: '"Hacha" (A)\u001f"Hachuela" (B)', exampleKey: 'tr_example', settledKey: '',
    }, 'english');
    expect(text).toBe(
      'A y B nombran ItemName_Axe de forma distinta en ES (ItemName). ItemName_Axe es "Hacha" (A) contra "Hachuela" (B). El juego lee las traducciones de todos los mods y se queda con la última cargada, así que hoy se ven las palabras de B.',
    );
  });

  it('keeps the English text for a code the locale does not know, and in English', async () => {
    expect(issueText({ code: 'something_new', params: { mod: 'A' }, message: 'english' })).toBe('english');
    await i18next.changeLanguage('en');
    expect(issueText({ code: 'dependency_after', params: { mod: 'A', dep: 'B' }, message: 'rust words' })).toBe('rust words');
    await i18next.changeLanguage('es');
  });
});
