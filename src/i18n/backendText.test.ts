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

  it('says a finding learned on this PC with the Monitor card it came from, in the locale', () => {
    const params = {
      mod: 'Viewpoint - Light Fix', seen: '7', on: '11', last: '2026-11-16', what: 'Viewpoint - Light Fix switched itself off', evidence: '',
      cardCode: 'java_mod_turned_off', cardParams: JSON.stringify({ mod: 'Viewpoint - Light Fix', tag: 'ViewpointHeadlightPlacement', what: 'Disabled' }),
    };
    const text = issueText({ code: 'learned_issue', params, message: 'english' });
    expect(text).toContain('Aprendido en esta PC: Viewpoint - Light Fix dio el mismo error en 7 de 11 partidas tuyas');
    expect(text).not.toContain('switched itself off');
    // card values that do not parse leave the English title
    expect(issueText({ code: 'learned_issue', params: { ...params, cardParams: '{' }, message: 'english' })).toContain('Aprendido en esta PC');
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

  it('says a Lua failure of a mod in plain words, with where and the words of the game', () => {
    const card = {
      code: 'mod_named_exception',
      params: { mod: 'Radio Voices', variant: 'lua', luaKey: 'lua_index', luaLongKey: 'lua_index_long', name: 'getId', place: 'RadioVoices.lua:1463', placeKey: 'lua_place', given: '', givenKey: 'lua_given_value', words: 'java.lang.RuntimeException: attempted index: getId of non-table: null' },
      title: 'english', explanation: 'english', recommended_action: 'english',
    };
    expect(cardText(card, 'title')).toBe('Radio Voices: usó getId de algo que no existe');
    expect(cardText(card, 'explanation')).toBe(
      'Radio Voices pidió getId de algo que no existe (nil), en RadioVoices.lua:1463. Lo que estaba haciendo en ese momento se cortó; el juego siguió. Lo que dijo el juego: java.lang.RuntimeException: attempted index: getId of non-table: null',
    );
    const call = { ...card, params: { ...card.params, luaKey: 'lua_no_impl', luaLongKey: 'lua_no_impl_long', name: 'getWeaponType', given: 'ComboItem', givenKey: 'lua_given', place: '', placeKey: '' } };
    expect(cardText(call, 'explanation')).toContain('Radio Voices llamó a getWeaponType del juego con un ComboItem, que esa función no acepta: o el mod le pasó algo equivocado, o la función cambió en una actualización del juego. Lo que');
  });

  it('says ZombieBuddy could not start a Java mod, naming the mod', () => {
    const card = { code: 'java_mod_error', params: { mod: 'Viewpoint', variant: 'zb_main' }, title: 'english', explanation: 'english', recommended_action: 'english' };
    expect(cardText(card, 'title')).toBe('ZombieBuddy no pudo correr el código de arranque de Viewpoint');
    expect(cardText(card, 'action')).toContain('En un servidor dedicado no hay nada que hacer.');
  });

  it('says a call the build does not have by name, with where, how many more and what the log adds', () => {
    const params = {
      mod: 'TV Remote', count: '2', names: ':Is(, getFoo(', moreNames: '', moreNamesKey: '', file: '42/media/lua/server/RemoteSpawn.lua', line: '353',
      more: '2', moreKey: 'and_more_places', variant: '', exceptions: '20', tailKey: 'api_tail_errors',
    };
    expect(issueText({ code: 'api_missing', params, message: 'english' })).toBe(
      'TV Remote llama a 2 cosas que esta versión del juego no tiene, por nombre: :Is(, getFoo( (42/media/lua/server/RemoteSpawn.lua:353 y 2 lugares más). Si ese código se ejecuta, falla ahí; un mod que falla en cada cuadro llena el log. La revisión va solo por nombre: un nombre armado mientras el juego corre no se ve. El log de la última sesión tiene 20 errores de este mod: el Monitor los muestra. Qué hacer: apagá el mod, o dejalo y esperá que su autor lo actualice; el Studio no puede arreglarlo.',
    );
    const one = { ...params, count: '7', names: ':a(, :b(, :c(, :d(, :e(', moreNames: '2', moreNamesKey: 'api_more_names', more: '', moreKey: '', variant: 'one', tailKey: 'api_tail_java' };
    expect(issueText({ code: 'api_missing', params: one, message: 'english' })).toBe(
      'TV Remote llama a algo que esta versión del juego no tiene, por nombre: :a(, :b(, :c(, :d(, :e( y 2 más (42/media/lua/server/RemoteSpawn.lua:353). Si ese código se ejecuta, falla ahí; un mod que falla en cada cuadro llena el log. La revisión va solo por nombre: un nombre armado mientras el juego corre no se ve. Este mod trae Java, que puede darle a Lua nombres que esta revisión no ve: puede que no sea nada. No hay nada que hacer salvo que veas fallar este mod en el juego.',
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
