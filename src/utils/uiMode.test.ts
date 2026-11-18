import { describe, expect, it } from 'vitest';
import type { ActiveTab, HealthIssue, IssueState } from '../types';
import { SIMPLE_TABS, UI_MODE_KEY, defaultUiMode, launchModeFor, readUiMode, simpleFindings, tabAfterSwitch, tabShown } from './uiMode';

/** A Storage over a Map, for the first-run decision. */
const memoryStorage = (entries: Record<string, string> = {}): Storage => {
  const map = new Map(Object.entries(entries));
  return {
    get length() {
      return map.size;
    },
    key: (n: number) => [...map.keys()][n] ?? null,
    getItem: (k: string) => map.get(k) ?? null,
    setItem: (k: string, v: string) => void map.set(k, v),
    removeItem: (k: string) => void map.delete(k),
    clear: () => map.clear(),
  };
};

const issue = (code: string, state: IssueState): HealthIssue => ({
  severity: 'info',
  state,
  code,
  mod_id: code,
  message: code,
  fixes: [],
  related: [],
});
const keyOf = (i: HealthIssue) => i.code;

describe('which mode a PC starts in', () => {
  it('is Simple on a new install and Complete where the Studio was used before', () => {
    expect(defaultUiMode([])).toBe('simple');
    // The language is kept before anything else and says nothing about earlier use.
    expect(defaultUiMode(['pz_language', 'something_else'])).toBe('simple');
    expect(defaultUiMode(['pz_welcome_done'])).toBe('advanced');
    expect(defaultUiMode(['pzms_sort_by'])).toBe('advanced');
  });

  it('is decided once and written down, so later preferences do not change it', () => {
    const fresh = memoryStorage();
    expect(readUiMode(fresh)).toBe('simple');
    expect(fresh.getItem(UI_MODE_KEY)).toBe('simple');
    fresh.setItem('pz_welcome_done', '1');
    expect(readUiMode(fresh)).toBe('simple');

    const used = memoryStorage({ pz_show_advanced: 'false' });
    expect(readUiMode(used)).toBe('advanced');
    expect(used.getItem(UI_MODE_KEY)).toBe('advanced');
  });

  it('keeps what the player chose', () => {
    expect(readUiMode(memoryStorage({ [UI_MODE_KEY]: 'simple', pz_welcome_done: '1' }))).toBe('simple');
    expect(readUiMode(memoryStorage({ [UI_MODE_KEY]: 'advanced' }))).toBe('advanced');
    // A broken value is decided again.
    expect(readUiMode(memoryStorage({ [UI_MODE_KEY]: 'weird' }))).toBe('simple');
  });
});

describe('the launch mode Play uses', () => {
  it('starts a normal game in Simple and Debug in Complete when the player never chose', () => {
    expect(launchModeFor('simple', memoryStorage())).toBe('NORMAL_FULLSCREEN');
    expect(launchModeFor('advanced', memoryStorage())).toBe('DEBUG_FULLSCREEN');
  });

  it('keeps the mode the player picked, in either view', () => {
    const picked = memoryStorage({ pz_last_launch_mode: 'DEBUG_WINDOWED' });
    expect(launchModeFor('simple', picked)).toBe('DEBUG_WINDOWED');
    expect(launchModeFor('advanced', picked)).toBe('DEBUG_WINDOWED');
  });

  it('falls back to the default when remembering is off or the value is unknown', () => {
    expect(launchModeFor('simple', memoryStorage({ pz_remember_launch_mode: 'false', pz_last_launch_mode: 'DEBUG_WINDOWED' }))).toBe('NORMAL_FULLSCREEN');
    expect(launchModeFor('simple', memoryStorage({ pz_last_launch_mode: 'MONITORED' }))).toBe('NORMAL_FULLSCREEN');
  });
});

describe('the screens each mode shows', () => {
  it('Simple shows the player screens and hides the advanced ones', () => {
    for (const tab of ['HOME', 'PROFILES', 'MOD_LIST', 'BROWSE', 'SAVES', 'HEALTH', 'SETTINGS'] as ActiveTab[]) expect(tabShown(tab, 'simple')).toBe(true);
    for (const tab of ['MERGER', 'DEVELOPER', 'MONITOR', 'SERVERS', 'MAPS', 'WHATS_NEW'] as ActiveTab[]) expect(tabShown(tab, 'simple')).toBe(false);
  });

  it('Complete is every screen of the app as it was, without Simple’s Home', () => {
    const all: ActiveTab[] = ['MOD_LIST', 'HEALTH', 'BROWSE', 'SERVERS', 'PROFILES', 'SAVES', 'WHATS_NEW', 'MAPS', 'MERGER', 'MONITOR', 'DEVELOPER', 'SETTINGS'];
    for (const tab of all) expect(tabShown(tab, 'advanced')).toBe(true);
    expect(tabShown('HOME', 'advanced')).toBe(false);
    expect(SIMPLE_TABS.every((tab) => tab === 'HOME' || all.includes(tab))).toBe(true);
  });

  it('switching keeps the screen in front when the new mode has it, else goes to its front page', () => {
    expect(tabAfterSwitch('MOD_LIST', 'simple', true)).toBe('MOD_LIST');
    expect(tabAfterSwitch('MERGER', 'simple', true)).toBe('HOME');
    expect(tabAfterSwitch('MERGER', 'simple', false)).toBe('PROFILES');
    expect(tabAfterSwitch('HOME', 'advanced', true)).toBe('PROFILES');
    expect(tabAfterSwitch('HOME', 'simple', false)).toBe('PROFILES');
    expect(tabAfterSwitch('SERVERS', 'advanced', true)).toBe('SERVERS');
  });
});

describe('Diagnostics in Simple', () => {
  const issues = [issue('broken', 'act'), issue('tip', 'recommended'), issue('know', 'info'), issue('fixed', 'handled'), issue('also', 'act')];

  it('shows only what needs acting on and counts the rest', () => {
    const out = simpleFindings(issues, null, keyOf);
    expect(out.shown.map((i) => i.code)).toEqual(['broken', 'also']);
    expect(out.optional).toBe(1);
    expect(out.nothingToDo).toBe(2);
  });

  it('keeps the row another screen pointed at, so its chip never leads to an empty page', () => {
    const out = simpleFindings(issues, 'tip', keyOf);
    expect(out.shown.map((i) => i.code)).toEqual(['broken', 'tip', 'also']);
    expect(out.optional).toBe(0);
  });

  it('says nothing to act on when nothing breaks', () => {
    const out = simpleFindings([issue('tip', 'recommended')], null, keyOf);
    expect(out.shown).toEqual([]);
    expect(out.optional).toBe(1);
  });
});
