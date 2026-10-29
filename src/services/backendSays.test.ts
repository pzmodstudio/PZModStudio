import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import i18next from '../i18n';
import { backendSays, explainError } from './tauri';

describe('backendSays', () => {
  beforeAll(async () => {
    await i18next.changeLanguage('es');
  });
  afterAll(async () => {
    await i18next.changeLanguage('en');
  });

  it('says a known backend text in the locale, with its values', () => {
    expect(backendSays('A server is already running (PID 4242): stop it before starting another.')).toBe(
      'Ya hay un servidor en marcha (PID 4242): detenelo antes de iniciar otro.',
    );
    expect(
      backendSays(
        'Brita on its own does not do it. Two mods together are doing this, and halving the list cannot separate a pair. Switch your list back on and try switching off Brita together with whichever mod changes the same thing.',
      ),
    ).toContain('Brita solo no lo provoca');
    expect(backendSays('The archive has more than 20000 entries.')).toBe('El archivo tiene más de 20000 entradas.');
  });

  it('leaves an unknown text as it came', () => {
    expect(backendSays('Something nobody wrote a pattern for')).toBe('Something nobody wrote a pattern for');
  });

  it('says GitHub when GitHub is the one limiting, not Steam', () => {
    expect(explainError('GitHub is rate-limiting this machine (it allows 60 requests an hour without an account).')).toContain('GitHub');
  });

  it('keeps English as written', async () => {
    await i18next.changeLanguage('en');
    expect(backendSays('Invalid server name.')).toBe('Invalid server name.');
    await i18next.changeLanguage('es');
  });
});
