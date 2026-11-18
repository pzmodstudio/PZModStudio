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
