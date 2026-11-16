import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';

// The backend: a delete Windows refuses, and who holds the file.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === 'who_holds_cmd') return [{ pid: 4242, exe: 'java.exe', role: 'server', label: 'knoxtest' }];
    if (cmd === 'remove_local_mod_cmd') throw 'Cannot move C:\\Users\\x\\Zomboid\\mods\\A: Access is denied. (os error 5)';
    throw 'Something else went wrong';
  }),
}));

import i18next from '../i18n';
import { TauriService, explainError } from './tauri';

describe('a file Windows would not let go', () => {
  beforeAll(async () => {
    await i18next.changeLanguage('es');
  });
  afterAll(async () => {
    await i18next.changeLanguage('en');
  });

  it('names who has it open, readable as it came and said in the language', async () => {
    const paths = { pz_install_dir: '', user_zomboid_dir: '', workshop_dir: '' } as never;
    const err = await TauriService.removeLocalMod(paths, 'A').catch((e: unknown) => e);
    expect(String(err)).toContain('(open in: java.exe, PID 4242)');
    expect(explainError(err)).toBe(
      'Windows no lo permitió: el archivo lo tiene abierto el servidor "knoxtest". Cerralo (o esperá a que termine) y volvé a intentarlo.',
    );
  });

  it('leaves any other error as it was', async () => {
    const paths = { pz_install_dir: '', user_zomboid_dir: '', workshop_dir: '' } as never;
    const err = await TauriService.mpTestStatus(paths).catch((e: unknown) => e);
    expect(err).toBe('Something else went wrong');
  });
});
