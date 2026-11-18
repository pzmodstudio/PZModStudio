import { describe, expect, it } from 'vitest';
import { asksTheGuide, followUpOf, intentChoices, splitQuestions, workshopAskOf, intentOf, modByName, modsNamedIn, orderOf, sinceOf, superlativeOf, suggestGroups, withoutModNames } from './doctorChat';
import { ModInfo } from '../types';

const mod = (mod_id: string, name: string, deps: string[] = []): ModInfo => ({ mod_id, name, dependencies: deps, enabled: true }) as unknown as ModInfo;
const mods = [
  mod('Viewpoint', 'Project Viewpoint'),
  mod('ViewpointLight', 'Viewpoint - Light Fix', ['Viewpoint']),
  mod('ViewpointDoor', 'Viewpoint - Door Fix', ['Viewpoint']),
  mod('ViewpointFences', 'Viewpoint Fences', ['Viewpoint']),
  mod('RC_RealisticColdMod', 'Realistic Temperature'),
];

describe('doctorChat', () => {
  it('reads the question asked, with or without accents, in Spanish or English', () => {
    expect(intentOf('¿Qué cambió?')).toBe('changed');
    expect(intentOf('que cambio')).toBe('changed');
    expect(intentOf('why did it close?')).toBe('crash');
    expect(intentOf('¿Por qué se cerró el juego?')).toBe('crash');
    expect(intentOf('¿funcionaron los arreglos?')).toBe('fixes');
    expect(intentOf('sugerí grupos')).toBe('groups');
    expect(intentOf('anotá lo aprendido')).toBe('notes');
    expect(intentOf('Viewpoint')).toBeNull();
    // a mod's name is not a question: 'Light Fix' does not ask about fixes
    expect(intentOf(withoutModNames('Viewpoint - Light Fix', modsNamedIn('Viewpoint - Light Fix', mods)))).toBeNull();
    expect(intentOf(withoutModNames('what changed for Viewpoint - Light Fix?', modsNamedIn('what changed for Viewpoint - Light Fix?', mods)))).toBe('changed');
  });

  it('finds the mods a text names, the longest name first', () => {
    expect(modsNamedIn('historial de Viewpoint - Light Fix', mods).map((m) => m.mod_id)).toEqual(['ViewpointLight']);
    expect(modsNamedIn('how is realistic temperature doing', mods).map((m) => m.mod_id)).toEqual(['RC_RealisticColdMod']);
    expect(modByName('light fix', mods).map((m) => m.mod_id)).toEqual(['ViewpointLight']);
    // an exact id wins over the names holding it
    expect(modByName('viewpoint', mods).map((m) => m.mod_id)).toEqual(['Viewpoint']);
  });

  it('reads orders on groups and the Studio notes, keeping their case', () => {
    expect(orderOf('agrupá Viewpoint - Light Fix, Viewpoint - Door Fix en Arreglos VP')).toEqual({ kind: 'add', mods: ['Viewpoint - Light Fix', 'Viewpoint - Door Fix'], group: 'Arreglos VP' });
    expect(orderOf('group A and B in Maps')).toEqual({ kind: 'add', mods: ['A', 'B'], group: 'Maps' });
    expect(orderOf('sacá Light Fix del grupo Arreglos VP')).toEqual({ kind: 'remove', mods: ['Light Fix'], group: 'Arreglos VP' });
    expect(orderOf('renombrá el grupo Mapas a Mundo')).toEqual({ kind: 'rename', group: 'Mapas', to: 'Mundo' });
    expect(orderOf('borrá el grupo Mapas')).toEqual({ kind: 'delete', group: 'Mapas' });
    expect(orderOf('nota en Light Fix: se apaga con Viewpoint 0.1.7')).toEqual({ kind: 'note', mod: 'Light Fix', text: 'se apaga con Viewpoint 0.1.7' });
    expect(orderOf('borrá la nota del Studio de Light Fix')).toEqual({ kind: 'noteRemove', mod: 'Light Fix' });
    expect(orderOf('¿qué cambió?')).toBeNull();
  });

  it('suggests a group for three or more mods sharing a name, leaving grouped ones out', () => {
    const s = suggestGroups(mods, new Set(), mods);
    expect(s[0].group).toBe('Viewpoint');
    expect(s[0].mods.map((m) => m.mod_id)).toContain('ViewpointLight');
    expect(suggestGroups(mods, new Set(['Viewpoint', 'ViewpointLight', 'ViewpointDoor', 'ViewpointFences']), mods)).toEqual([]);
  });
});

describe('doctorChat, understanding more', () => {
  it('forgives typos and other ways of saying it', () => {
    expect(intentOf('se me crasheo el juego')).toBe('crash');
    expect(intentOf('se tildo todo')).toBe('crash');
    expect(intentOf('anda re lentoo')).toBe('slow');
    expect(intentOf('el juego no arranca')).toBe('crash');
    expect(intentOf('qe canbio')).toBe('changed');
  });

  it('reads a time window', () => {
    const now = new Date(2026, 10, 17, 18, 0, 0);
    const day = (d: Date) => Math.floor(d.getTime() / 1000);
    expect(sinceOf('¿qué cambió desde ayer?', now)).toBe(day(new Date(2026, 10, 16)));
    expect(sinceOf('caídas de esta semana', now)).toBe(day(new Date(2026, 10, 10)));
    expect(sinceOf('since 12/11', now)).toBe(day(new Date(2026, 10, 12)));
    expect(sinceOf('últimos 3 días', now)).toBe(day(new Date(2026, 10, 14)));
    expect(sinceOf('qué cambió', now)).toBeNull();
  });

  it('follows the last answer', () => {
    expect(followUpOf('apagalo')).toBe('disable');
    expect(followUpOf('¿y desde cuándo?')).toBe('why');
    expect(followUpOf('anotalo')).toBe('note');
    expect(followUpOf('mostrámelo en la lista')).toBe('show');
    expect(followUpOf('¿por qué se cierra el juego cuando cargo una partida grande?')).toBeNull();
  });

  it('reads which one is the most', () => {
    expect(superlativeOf('¿cuál tarda más en cargar?')).toBe('slowest');
    expect(superlativeOf('which mod gives the most errors?')).toBe('most_errors');
    expect(superlativeOf('qué cambió')).toBeNull();
  });

  it('finds a mod a typo away', () => {
    expect(modByName('Realistc Temprature', mods).map((m) => m.mod_id)).toEqual(['RC_RealisticColdMod']);
  });

  it('offers a choice when two questions weigh the same', () => {
    expect(intentChoices('error de carga')).toEqual(expect.arrayContaining(['errors', 'slow']));
    expect(intentChoices('se cerro')).toEqual([]);
  });
});

describe('doctorChat, the new questions', () => {
  it('reads the culprit, the hunt, the week and undo', () => {
    expect(intentOf('¿qué mod hace que se caiga?')).toBe('culprit');
    expect(intentOf('buscá el culpable por mitades')).toBe('hunt');
    expect(intentOf('¿qué pasó esta semana?')).toBe('week');
    expect(intentOf('deshacé eso')).toBe('undo');
  });

  it('sends questions about the app to the guide and splits two questions', () => {
    expect(asksTheGuide('¿Cómo hago un perfil?')).toBe(true);
    expect(asksTheGuide('what is a compatible copy')).toBe(true);
    expect(asksTheGuide('apagalo')).toBe(false);
    expect(splitQuestions('¿qué cambió y qué mods fallan?')).toEqual(['¿qué cambió', 'qué mods fallan']);
    expect(splitQuestions('what changed? which mods fail?')).toEqual(['what changed', 'which mods fail']);
    expect(splitQuestions('agrupá A y B en Mapas')).toEqual(['agrupá A y B en Mapas']);
  });

  it('reads a question to the Workshop', () => {
    expect(workshopAskOf('¿hay versión nueva?')).toBe('update');
    expect(workshopAskOf('¿qué dicen en los comentarios?')).toBe('comments');
    expect(workshopAskOf('¿qué cambió?')).toBeNull();
  });
});

describe('doctorChat, in every language of the app', () => {
  const now = new Date(2026, 10, 17, 18, 0, 0);
  const day = (d: Date) => Math.floor(d.getTime() / 1000);

  it('reads the questions in Portuguese', () => {
    expect(intentOf('o que mudou?')).toBe('changed');
    expect(intentOf('por que o jogo fechou?')).toBe('crash');
    expect(intentOf('o jogo travou de novo')).toBe('crash');
    expect(intentOf('está muito lento')).toBe('slow');
    expect(intentOf('qual mod causa o problema?')).toBe('culprit');
    expect(intentOf('os consertos funcionaram?')).toBe('fixes');
    expect(intentOf('o que aconteceu esta semana?')).toBe('week');
    expect(asksTheGuide('como faço um perfil?')).toBe(true);
    expect(sinceOf('o que mudou desde ontem?', now)).toBe(day(new Date(2026, 10, 16)));
    expect(followUpOf('desliga ele')).toBe('disable');
    expect(followUpOf('e desde quando?')).toBe('why');
    expect(superlativeOf('qual demora mais para carregar?')).toBe('slowest');
    expect(workshopAskOf('tem versão nova?')).toBe('update');
    expect(splitQuestions('o que mudou e quais mods falham?')).toEqual(['o que mudou', 'quais mods falham?'.replace('?', '')]);
    expect(orderOf('coloque A, B no grupo Mapas')).toEqual({ kind: 'add', mods: ['A', 'B'], group: 'Mapas' });
  });

  it('reads the questions in French', () => {
    expect(intentOf("qu'est-ce qui a changé ?")).toBe('changed');
    expect(intentOf('le jeu a planté')).toBe('crash');
    expect(intentOf('le jeu ne démarre pas')).toBe('crash');
    expect(intentOf("c'est très lent")).toBe('slow');
    expect(intentOf('quel mod cause ça ?')).toBe('culprit');
    expect(intentOf('les erreurs')).toBe('errors');
    expect(asksTheGuide("comment créer un profil ?")).toBe(true);
    expect(asksTheGuide("qu'est-ce qu'une copie compatible ?")).toBe(true);
    expect(sinceOf("qu'est-ce qui a changé depuis hier ?", now)).toBe(day(new Date(2026, 10, 16)));
    expect(sinceOf('les plantages de cette semaine', now)).toBe(day(new Date(2026, 10, 10)));
    expect(followUpOf('désactive-le')).toBe('disable');
    expect(followUpOf('et pourquoi ?')).toBe('why');
    expect(superlativeOf('quel mod a le plus d’erreurs ?')).toBe('most_errors');
    expect(orderOf('renomme le groupe Cartes en Monde')).toEqual({ kind: 'rename', group: 'Cartes', to: 'Monde' });
  });

  it('reads the questions in German', () => {
    expect(intentOf('was hat sich geändert?')).toBe('changed');
    expect(intentOf('das Spiel ist abgestürzt')).toBe('crash');
    expect(intentOf('es ist sehr langsam')).toBe('slow');
    expect(intentOf('welche Mod verursacht das?')).toBe('culprit');
    expect(intentOf('haben die Reparaturen funktioniert? wurde es behoben?')).toBe('fixes');
    expect(asksTheGuide('wie erstelle ich ein Profil?')).toBe(true);
    expect(sinceOf('was hat sich seit gestern geändert?', now)).toBe(day(new Date(2026, 10, 16)));
    expect(sinceOf('seit 12.11.', now)).toBe(day(new Date(2026, 10, 12)));
    expect(followUpOf('schalte es aus')).toBe('disable');
    expect(followUpOf('und seit wann?')).toBe('why');
    expect(superlativeOf('welche Mod lädt am längsten?')).toBe('slowest');
    expect(orderOf('lösche die Gruppe Karten')).toEqual({ kind: 'delete', group: 'Karten' });
    expect(orderOf('benenne die Gruppe Karten in Welt um')).toEqual({ kind: 'rename', group: 'Karten', to: 'Welt' });
  });

  it('reads the questions in Russian', () => {
    expect(intentOf('что изменилось?')).toBe('changed');
    expect(intentOf('игра вылетает')).toBe('crash');
    expect(intentOf('почему игра закрылась?')).toBe('crash');
    expect(intentOf('очень медленно грузится')).toBe('slow');
    expect(intentOf('какой мод вызывает ошибки?')).toBe('culprit');
    expect(intentOf('какие ошибки?')).toBe('errors');
    expect(asksTheGuide('как сделать профиль?')).toBe(true);
    expect(sinceOf('что изменилось со вчера? вчера', now)).toBe(day(new Date(2026, 10, 16)));
    expect(sinceOf('за последние 3 дня', now)).toBe(day(new Date(2026, 10, 14)));
    expect(followUpOf('выключи его')).toBe('disable');
    expect(followUpOf('и с каких пор?')).toBe('why');
    expect(orderOf('добавь A, B в группу Карты')).toEqual({ kind: 'add', mods: ['A', 'B'], group: 'Карты' });
    expect(orderOf('убери A из группы Карты')).toEqual({ kind: 'remove', mods: ['A'], group: 'Карты' });
  });

  it('reads the questions in Chinese', () => {
    expect(intentOf('改变了什么？')).toBe('changed');
    expect(intentOf('游戏崩溃了')).toBe('crash');
    expect(intentOf('游戏很卡顿')).toBe('slow');
    expect(intentOf('哪个模组导致的？')).toBe('culprit');
    expect(intentOf('有什么错误')).toBe('errors');
    expect(asksTheGuide('怎么创建配置方案？')).toBe(true);
    expect(sinceOf('昨天以来改变了什么', now)).toBe(day(new Date(2026, 10, 16)));
    expect(sinceOf('最近3天', now)).toBe(day(new Date(2026, 10, 14)));
    expect(followUpOf('关掉它')).toBe('disable');
    expect(followUpOf('为什么？')).toBe('why');
    expect(superlativeOf('哪个模组加载最慢？')).toBe('slowest');
    expect(splitQuestions('改变了什么？哪些模组出错？')).toEqual(['改变了什么', '哪些模组出错']);
  });

  it('reads the Studio notes and Chinese orders', () => {
    expect(orderOf('nota em Light Fix: desliga com Viewpoint')).toEqual({ kind: 'note', mod: 'Light Fix', text: 'desliga com Viewpoint' });
    expect(orderOf('note sur Light Fix : à revoir')).toEqual({ kind: 'note', mod: 'Light Fix', text: 'à revoir' });
    expect(orderOf('Notiz zu Light Fix: später prüfen')).toEqual({ kind: 'note', mod: 'Light Fix', text: 'später prüfen' });
    expect(orderOf('заметка к Light Fix: проверить')).toEqual({ kind: 'note', mod: 'Light Fix', text: 'проверить' });
    expect(orderOf('备注 Light Fix：以后再看')).toEqual({ kind: 'note', mod: 'Light Fix', text: '以后再看' });
    expect(orderOf('lösche die Studio-Notiz von Light Fix')).toEqual({ kind: 'noteRemove', mod: 'Light Fix' });
    expect(orderOf('supprime la note du Studio de Light Fix')).toEqual({ kind: 'noteRemove', mod: 'Light Fix' });
    expect(orderOf('把 A、B 加到 地图 分组')).toEqual({ kind: 'add', mods: ['A', 'B'], group: '地图' });
    expect(orderOf('把 A 从 地图 分组移出')).toEqual({ kind: 'remove', mods: ['A'], group: '地图' });
    expect(orderOf('把 地图 分组改名为 世界')).toEqual({ kind: 'rename', group: '地图', to: '世界' });
    expect(orderOf('删除 地图 分组')).toEqual({ kind: 'delete', group: '地图' });
    expect(orderOf('改变了什么？')).toBeNull();
  });

  it('keeps Spanish and English as they were', () => {
    expect(intentOf('no abre el juego')).toBe('crash');
    expect(followUpOf('no abre el juego')).toBeNull();
    expect(followUpOf('¿qué hay de nuevo hier?')).toBeNull();
    expect(orderOf('group No Lag, Better Sorting in Maps')).toEqual({ kind: 'add', mods: ['No Lag', 'Better Sorting'], group: 'Maps' });
  });
});
