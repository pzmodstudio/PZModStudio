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

  it('says why an isolated profile cannot move its files now', () => {
    const why =
      'the saves, game options and mod settings of isolated profiles are moved only while no Project Zomboid game or server runs, so nothing is moved while one of them has the files open.';
    expect(backendSays(`Close Project Zomboid first (process 812): ${why}`)).toMatch(/^Cerrá Project Zomboid primero \(proceso 812\)/);
    expect(backendSays(`The Project Zomboid server "Knox" is running (process 77). Stop it first (Servers screen, or its console): ${why}`)).toMatch(
      /^El servidor de Project Zomboid "Knox" está en marcha \(proceso 77\)/,
    );
    expect(backendSays(`A Project Zomboid server is running (process 77). Stop it first (Servers screen, or its console): ${why}`)).toMatch(
      /^Hay un servidor de Project Zomboid en marcha \(proceso 77\)/,
    );
    expect(backendSays('Cannot move C:\\Zomboid\\Saves\\Sandbox\\W to D:\\x\\W: Access is denied.')).toBe(
      'No se pudo mover C:\\Zomboid\\Saves\\Sandbox\\W a D:\\x\\W: Access is denied.',
    );
    expect(backendSays('Cannot move the copy away: busy')).toBe('Cannot move the copy away: busy');
    expect(backendSays('Cannot move MyMod away (is the game or a server using it?): busy')).toBe(
      'Cannot move MyMod away (is the game or a server using it?): busy',
    );
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
