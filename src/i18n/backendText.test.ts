import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import i18next from './index';
import { backendText, cardText, issueText } from './backendText';

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

  it('says the texts the game could not format as one card, with the keys, how many more and whose they are', () => {
    const card = {
      code: 'translator_text',
      params: { variant: 'many', count: '13', keys: 'Sandbox_A, Sandbox_B', more: '11', moreKey: 'and_more', mod: 'Neat Lockpicking', whoseKey: 'text_whose_mod_many' },
      title: 'english', explanation: 'english', recommended_action: 'english',
    };
    expect(cardText(card, 'title')).toBe('13 textos tienen un % que el juego no puede formatear');
    expect(cardText(card, 'explanation')).toContain('Los textos: Sandbox_A, Sandbox_B y 11 más. Están en los archivos de traducción de Neat Lockpicking.');
    const one = { ...card, params: { variant: '', count: '1', keys: 'Sandbox_A', more: '', moreKey: '', mod: '', whoseKey: 'text_whose_none_one' } };
    expect(cardText(one, 'explanation')).toContain('El texto: Sandbox_A. El Studio no lo encontró en los textos de los mods que tenés activados; puede ser del propio juego.');
  });

  it('says a call the build does not have by name, with where, how many more and what the log adds', () => {
    const params = {
      mod: 'TV Remote', count: '2', names: ':Is(, getFoo(', moreNames: '', moreNamesKey: '', file: '42/media/lua/server/RemoteSpawn.lua', line: '353',
      more: '2', moreKey: 'and_more_places', variant: '', exceptions: '20', tailKey: 'api_tail_errors',
    };
    expect(issueText({ code: 'api_missing', params, message: 'english' })).toBe(
      'TV Remote llama a 2 cosas que esta versión del juego no tiene, por nombre: :Is(, getFoo( (42/media/lua/server/RemoteSpawn.lua:353 y 2 lugares más). Si ese código se ejecuta, falla ahí; un mod que falla en cada cuadro llena el log. La revisión va solo por nombre: un nombre armado mientras el juego corre no se ve. El log de la última sesión tiene 20 errores de este mod: el Monitor los muestra.',
    );
    const one = { ...params, count: '7', names: ':a(, :b(, :c(, :d(, :e(', moreNames: '2', moreNamesKey: 'api_more_names', more: '', moreKey: '', variant: 'one', tailKey: 'api_tail_java' };
    expect(issueText({ code: 'api_missing', params: one, message: 'english' })).toBe(
      'TV Remote llama a algo que esta versión del juego no tiene, por nombre: :a(, :b(, :c(, :d(, :e( y 2 más (42/media/lua/server/RemoteSpawn.lua:353). Si ese código se ejecuta, falla ahí; un mod que falla en cada cuadro llena el log. La revisión va solo por nombre: un nombre armado mientras el juego corre no se ve. Este mod trae Java, que puede darle a Lua nombres que esta revisión no ve: puede que no sea nada.',
    );
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
