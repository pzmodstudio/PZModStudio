import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import i18next from '../../i18n';
import { labelFor } from './RowHealth';

describe('the short name of a finding', () => {
  beforeAll(async () => {
    await i18next.changeLanguage('es');
  });
  afterAll(async () => {
    await i18next.changeLanguage('en');
  });

  it('is said in plain words in Simple, as the Studio says it in Complete', () => {
    expect(labelFor({ code: 'invalid_mod_info' })).toBe('mod.info roto');
    expect(labelFor({ code: 'invalid_mod_info' }, 'simple')).toBe('El juego no puede leerlo');
    expect(labelFor({ code: 'zb_excluded' }, 'simple')).not.toMatch(/ZombieBuddy/);
    expect(labelFor({ code: 'not_for_build' }, 'simple')).not.toMatch(/build/);
  });

  it('keeps the usual name in Simple when it is already plain', () => {
    expect(labelFor({ code: 'missing_dependency' }, 'simple')).toBe(labelFor({ code: 'missing_dependency' }));
  });

  it('has plain words in English too', async () => {
    await i18next.changeLanguage('en');
    expect(labelFor({ code: 'zb_excluded' }, 'simple')).toBe('Part of the mod did not load');
    await i18next.changeLanguage('es');
  });
});
