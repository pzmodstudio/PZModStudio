import type { ActiveTab, GameLaunchMode, HealthFix, HealthIssue, IssueState } from '../types';
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

/** Set once the player picked a mode themselves (the sidebar switch, Settings, the offer). */
export const UI_MODE_CHOSEN_KEY = 'pz_ui_mode_chosen';
/** Set once the offer to try Simple was answered (either way): it never shows again. */
export const SIMPLE_OFFER_KEY = 'pz_simple_offer_done';

/** Keys that do not tell an install that was used before (written before anything else, or ours). */
const NOT_A_SIGN_OF_USE = new Set([UI_MODE_KEY, UI_MODE_CHOSEN_KEY, SIMPLE_OFFER_KEY, 'pz_language']);

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

/**
 * The player switches modes. Picking one counts as having seen the offer to try Simple: a player
 * who already went to Simple, or chose Complete on purpose, is never asked.
 */
export const setUiMode = (mode: UiMode) => {
  try {
    localStorage.setItem(UI_MODE_KEY, mode);
    localStorage.setItem(UI_MODE_CHOSEN_KEY, '1');
    localStorage.setItem(SIMPLE_OFFER_KEY, '1');
  } catch {
    // the session keeps it
  }
  window.dispatchEvent(new CustomEvent<UiMode>(UI_MODE_EVENT, { detail: mode }));
};

/**
 * Whether to offer Simple once to a player who used the Studio before Simple existed: they were
 * put in Complete without choosing, never picked a mode, and never answered the offer.
 */
export const shouldOfferSimple = (storage: Storage | null = typeof localStorage === 'undefined' ? null : localStorage): boolean => {
  if (!storage) return false;
  try {
    return storage.getItem(UI_MODE_KEY) === 'advanced' && storage.getItem(UI_MODE_CHOSEN_KEY) === null && storage.getItem(SIMPLE_OFFER_KEY) === null;
  } catch {
    return false;
  }
};

/** "No, thanks": the offer never shows again, and the mode stays as it is. */
export const dismissSimpleOffer = (storage: Storage | null = typeof localStorage === 'undefined' ? null : localStorage) => {
  try {
    storage?.setItem(SIMPLE_OFFER_KEY, '1');
  } catch {
    // shown again next start: no harm
  }
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

/**
 * The launch mode as Simple has it: Simple offers no Debug modes, so a Debug mode picked in
 * Complete starts the same kind of game (full screen or window) without -debug there. The pick
 * itself is kept: Complete starts it again.
 */
export const simpleLaunchMode = (launch: GameLaunchMode): GameLaunchMode =>
  ['DEBUG_WINDOWED', 'WINDOWED', 'NORMAL_WINDOWED', 'NORMAL'].includes(launch) ? 'NORMAL_WINDOWED' : 'NORMAL_FULLSCREEN';

/** The mode Play starts the game in: the player's own choice, else the default of the Simple / Complete mode. */
export const launchModeFor = (mode: UiMode, storage?: Storage | null): GameLaunchMode => {
  const launch = chosenLaunchMode(storage) ?? defaultLaunchMode(mode);
  return mode === 'simple' ? simpleLaunchMode(launch) : launch;
};

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

/**
 * The findings a Mod List row or the mod's panel shows in Simple: only what needs something done,
 * the same rows Diagnostics shows there. The optional and nothing-to-do ones are in Complete.
 */
export const simpleRowIssues = (issues: HealthIssue[]): HealthIssue[] => issues.filter((i) => issueStateOf(i) === 'act');

/** Fixes that open a screen only Complete has (the Merger): Simple sends the player to Diagnostics instead. */
const COMPLETE_ONLY_FIXES = new Set<HealthFix['action']>(['open_in_merger']);
export const fixShownIn = (mode: UiMode, fix: HealthFix): boolean => mode === 'advanced' || !COMPLETE_ONLY_FIXES.has(fix.action);
