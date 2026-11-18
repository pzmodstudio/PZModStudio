import type { ActiveTab, GameLaunchMode, HealthIssue, IssueState } from '../types';
import { issueStateOf } from './issueState';

/**
 * Simple or Complete: how much of the Studio a player sees. Simple shows what playing with mods
 * needs (a home with Play, the mods, profiles, Explore, saves, and only the findings that need
 * something done); Complete is the whole app. Nothing is removed in Simple: every screen is one
 * switch away. Kept on this PC with the other screen preferences (localStorage).
 */
export type UiMode = 'simple' | 'advanced';

export const UI_MODE_KEY = 'pz_ui_mode';
/** Sent on window when the mode changes; `detail` is the new mode. */
export const UI_MODE_EVENT = 'pzms-ui-mode-changed';

/** Keys that do not tell an install that was used before (written before anything else, or ours). */
const NOT_A_SIGN_OF_USE = new Set([UI_MODE_KEY, 'pz_language']);

/**
 * The mode of a PC that never chose one: Complete when the Studio was already used here (it left
 * its preferences behind, so the player keeps the app they know), Simple on a new install.
 */
export const defaultUiMode = (storageKeys: string[]): UiMode =>
  storageKeys.some((k) => (k.startsWith('pz_') || k.startsWith('pzms_')) && !NOT_A_SIGN_OF_USE.has(k)) ? 'advanced' : 'simple';

const storageKeys = (storage: Storage): string[] => {
  const out: string[] = [];
  for (let n = 0; n < storage.length; n++) {
    const key = storage.key(n);
    if (key) out.push(key);
  }
  return out;
};

/**
 * The mode now. The first time on a PC it is decided from what is already kept and written down
 * at once, so the preferences the app writes later never turn an install into "used before".
 * Called in main.tsx before the app renders.
 */
export const readUiMode = (storage: Storage | null = typeof localStorage === 'undefined' ? null : localStorage): UiMode => {
  if (!storage) return 'advanced';
  try {
    const saved = storage.getItem(UI_MODE_KEY);
    if (saved === 'simple' || saved === 'advanced') return saved;
    const mode = defaultUiMode(storageKeys(storage));
    storage.setItem(UI_MODE_KEY, mode);
    return mode;
  } catch {
    return 'advanced';
  }
};

export const setUiMode = (mode: UiMode) => {
  try {
    localStorage.setItem(UI_MODE_KEY, mode);
  } catch {
    // the session keeps it
  }
  window.dispatchEvent(new CustomEvent<UiMode>(UI_MODE_EVENT, { detail: mode }));
};

/** The four launch modes the header offers (the others are older names kept for saved values). */
export const LAUNCH_MODE_IDS: GameLaunchMode[] = ['DEBUG_FULLSCREEN', 'DEBUG_WINDOWED', 'NORMAL_FULLSCREEN', 'NORMAL_WINDOWED'];

/**
 * The launch mode of a player who never chose one: a normal game in Simple (a beginner has no use
 * for -debug, which also stops the game on any Lua error), Debug in Complete as before.
 */
export const defaultLaunchMode = (mode: UiMode): GameLaunchMode => (mode === 'simple' ? 'NORMAL_FULLSCREEN' : 'DEBUG_FULLSCREEN');

/** The mode the player last picked, when they want it remembered (Settings; on by default); null when none. */
export const chosenLaunchMode = (storage: Storage | null = typeof localStorage === 'undefined' ? null : localStorage): GameLaunchMode | null => {
  if (!storage) return null;
  try {
    if (storage.getItem('pz_remember_launch_mode') === 'false') return null;
    const saved = storage.getItem('pz_last_launch_mode') as GameLaunchMode | null;
    return saved && LAUNCH_MODE_IDS.includes(saved) ? saved : null;
  } catch {
    return null;
  }
};

/** The mode Play starts the game in: the player's own choice, else the default of the Simple / Complete mode. */
export const launchModeFor = (mode: UiMode, storage?: Storage | null): GameLaunchMode =>
  chosenLaunchMode(storage) ?? defaultLaunchMode(mode);

/** The screens Simple shows, in the sidebar's order. Home is Simple's own front page. */
export const SIMPLE_TABS: ActiveTab[] = ['HOME', 'PROFILES', 'MOD_LIST', 'BROWSE', 'SAVES', 'HEALTH', 'SETTINGS'];

export const tabShown = (tab: ActiveTab, mode: UiMode): boolean => (mode === 'simple' ? SIMPLE_TABS.includes(tab) : tab !== 'HOME');

/**
 * Where the player lands after switching modes: the same screen when the new mode shows it, else
 * the front page of that mode (Home in Simple when a profile is active, Profiles otherwise).
 */
export const tabAfterSwitch = (tab: ActiveTab, mode: UiMode, hasProfile: boolean): ActiveTab => {
  if (tabShown(tab, mode)) return tab === 'HOME' && !hasProfile ? 'PROFILES' : tab;
  return mode === 'simple' && hasProfile ? 'HOME' : 'PROFILES';
};

/**
 * Diagnostics in Simple: only what needs something done (the "act" rows), plus the one row
 * another screen pointed at, so a chip never leads to an empty page. The rest is counted:
 * `optional` (worth doing, the game works without it) and `nothingToDo` (good to know, or already
 * fixed by the Studio).
 */
export const simpleFindings = (issues: HealthIssue[], keepKey: string | null, keyOf: (i: HealthIssue) => string) => {
  const shown: HealthIssue[] = [];
  const counts: Record<IssueState, number> = { act: 0, recommended: 0, info: 0, handled: 0 };
  for (const issue of issues) {
    const state = issueStateOf(issue);
    if (state === 'act' || (keepKey !== null && keyOf(issue) === keepKey)) shown.push(issue);
    else counts[state] += 1;
  }
  return { shown, optional: counts.recommended, nothingToDo: counts.info + counts.handled };
};
