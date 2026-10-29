import { HelpButton } from './components/common/HelpPanel';
import React, { useState, useEffect, useCallback, useRef, lazy, Suspense } from 'react';
import { ActiveTab, VfsConflict, ModInfo, TranslatedErrorCard, GameLaunchMode, LaunchTarget, AppProfile, ModErrorStat, HealthIssue, AuthorLink, profileBuildMajor, profileMembers, STUDIO_LINK_MOD_ID } from './types';
import { DeveloperModule } from './components/dev/DeveloperModule';
import { StudioHeader } from './components/layout/StudioHeader';
import { UpdateNotice } from './components/layout/UpdateNotice';
import { StudioSidebar } from './components/layout/StudioSidebar';
import { LoadOrderModule } from './components/load_order/LoadOrderModule';
import { SandboxModule } from './components/sandbox/SandboxModule';
import { SettingsModule, StudioPathsUI } from './components/settings/SettingsModule';
import { TauriService, reportBackendError, reportNotice } from './services/tauri';
import { startLiveConsole } from './services/liveConsole';
import { errorShape } from './utils/errorShape';

import { ServerModule } from './components/server/ServerModule';
import { JoinServers } from './components/server/JoinServers';
import { InstanceModule } from './components/instances/InstanceModule';
import { ChangeScope, onChange } from './services/changes';
import { SavesModule } from './components/saves/SavesModule';
import { WhatsNewModule } from './components/whatsnew/WhatsNewModule';
import { MapsModule } from './components/maps/MapsModule';
import { ChangePreview } from './components/common/ChangePreview';
import { TogglePlan, applyPlan, enableWithRequirements, planIsSimple, planToggle } from './utils/planner';
import { SplashScreen } from './components/layout/SplashScreen';
import { SHOW_WELCOME_EVENT, WelcomeTour, welcomeSeen } from './components/common/WelcomeTour';
import { BackendErrorToasts } from './components/layout/BackendErrorToasts';
import { HealthModule } from './components/health/HealthModule';
import { useModHealth } from './hooks/useModHealth';
import { BrowseModule } from './components/browse/BrowseModule';
import { issuesByModOf, linkModsText, modTagsOf } from './components/load_order/modFacets';
import { syncFavouritesWithGame } from './services/modLists';
import { mergeLinks } from './components/common/SupportLinks';
import { ContextMenuProvider } from './components/common/ContextMenu';

// The Merger bundles the Monaco editor (~4 MB): load it only when its tab is opened.
const MergerModule = lazy(() => import('./components/merger/MergerModule').then((m) => ({ default: m.MergerModule })));
/** The list set aside by Developer > launch with only this mod, until it is put back. */
const DEV_LAUNCH_KEY = 'pzms_dev_launch';

import { Sparkles, CheckCircle2, AlertTriangle, Info } from 'lucide-react';
import i18next from 'i18next';
import { tHtml } from './i18n/html';
import { issueText } from './i18n/backendText';
import { labelFor } from './components/load_order/RowHealth';

export const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState<ActiveTab>('PROFILES');
  const [conflicts, setConflicts] = useState<VfsConflict[]>([]);
  const [mods, setMods] = useState<ModInfo[]>([]);
  const [activeProfile, setActiveProfile] = useState<AppProfile | null>(null);
  /** Every saved profile, so the Mod List can say which of them a mod belongs to. */
  const [profiles, setProfiles] = useState<AppProfile[]>([]);
  const [openedPackageFolder, setOpenedPackageFolder] = useState<string | null>(null);
  /** Conflicting file the Merger should open on (set from Mod Health). */
  const [mergerFocusPath, setMergerFocusPath] = useState<string | null>(null);
  /** A mod (and maybe the file the log named) the Monitor sends to the Merger to patch. */
  const [mergerPatchFocus, setMergerPatchFocus] = useState<{
    modId: string;
    fileHint?: string | null;
    fix?: { from: string; to: string } | null;
  } | null>(null);
  /** Workshop item Browse Mods should open on (set from the Mod List). */
  const [browseFocusWorkshopId, setBrowseFocusWorkshopId] = useState<string | null>(null);
  const [browseFocusSearch, setBrowseFocusSearch] = useState<string | null>(null);
  const [errorCards, setErrorCards] = useState<TranslatedErrorCard[]>([]);
  const [autoMergeResultModal, setAutoMergeResultModal] = useState<{
    patch_mod_dir: string;
    total_conflicts: number;
    files_written: number;
    polyfill_shim_written: boolean;
  } | null>(null);

  // Loading Screen & Startup State
  const [isInitializing, setIsInitializing] = useState<boolean>(true);
  const [initStatusMessage, setInitStatusMessage] = useState<string>('Starting PZ Mod Studio...');
  const [initProgress, setInitProgress] = useState<number>(15);

  // Studio Directory Paths State (Persistent Profile)
  const [paths, setPaths] = useState<StudioPathsUI>({
    pz_install_dir: '',
    workshop_dir: '',
    user_zomboid_dir: '',
    mod_list_ini_path: '',
    is_valid: false,
  });

  // Game-exact health check of the active list, shared by Mod List, Mod Health and the sidebar.
  const health = useModHealth(paths, mods);

  /** Mods on in the list updated since the last session or waiting in Steam (offline count). */
  const [whatsNewCount, setWhatsNewCount] = useState(0);
  useEffect(() => {
    if (!paths.is_valid || !paths.user_zomboid_dir) return;
    void TauriService.whatsNew(paths, false, null).then((w) => setWhatsNewCount(w ? w.items.filter((i) => i.active).length : 0));
  }, [paths, mods]);

  /**
   * Mod Health's warnings go to the game too: the Studio Link mod marks those mods in the game's
   * Mods screen, with the text on hover, in the language the Studio speaks.
   */
  const healthIssues = health.report?.issues;
  useEffect(() => {
    if (!paths.is_valid || !paths.user_zomboid_dir || !healthIssues) return;
    const plain = (text: string) => text.replace(/`|\*\*/g, '').replace(/\s+/g, ' ').trim();
    const lines: [string, string, string][] = healthIssues
      .filter((i) => i.mod_id && (i.severity === 'error' || i.severity === 'warning'))
      .map((i) => [i.severity, i.mod_id as string, plain(`${labelFor(i)}: ${issueText(i)}`)]);
    void TauriService.writeLinkHealth(paths, lines);
  }, [paths, healthIssues, i18next.language]);

  /**
   * What each mod did to the last session's log, by the name the game writes. Read when the
   * paths are known and again when the game starts a new session, so the Mod List's row can say
   * "threw 413 times last session" without the Monitor open.
   */
  const [errorStats, setErrorStats] = useState<Map<string, ModErrorStat>>(new Map());
  /** Mod id -> how long its files took to load last session, read with the error stats. */
  const [loadTimes, setLoadTimes] = useState<Map<string, number>>(new Map());
  /** Each mod's share of the game's time in the last profiler measurement (%), when there is one. */
  const [gameShares, setGameShares] = useState<Map<string, number>>(new Map());

  /**
   * One list of favourites: the app's stars and the game's (its Mods screen keeps its own). Synced
   * when the mods are scanned, when the window comes back to the front and every 10 seconds, so a
   * star put in the game shows here soon after.
   */
  useEffect(() => {
    if (!paths.is_valid || !paths.user_zomboid_dir || mods.length === 0) return;
    const installed = mods
      .filter((m) => !m.not_installed)
      .map((m) => ({ mod_id: m.mod_id, workshop_id: m.workshop_id ?? null, name: m.name }));
    const run = () => void syncFavouritesWithGame(paths.user_zomboid_dir, installed);
    run();
    const timer = window.setInterval(run, 10000);
    window.addEventListener('focus', run);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener('focus', run);
    };
  }, [paths, mods]);

  /**
   * Where to support each installed mod's author, as the Mod List's inspector shows it: the links
   * in its changelog, then those in its Workshop description. Read again when the installed mods
   * change; the game's Mods screen shows them too.
   */
  const [supportByMod, setSupportByMod] = useState<Map<string, AuthorLink[]>>(new Map());
  const installedKey = mods.filter((m) => !m.not_installed).map((m) => m.mod_id).join('|');
  useEffect(() => {
    if (!paths.is_valid || !paths.user_zomboid_dir || !installedKey) return;
    let cancelled = false;
    const installed = mods.filter((m) => !m.not_installed);
    // The same ids, in the same order, as the inspector asks for: Steam's answer is shared.
    const ids = [...new Set(mods.map((m) => m.workshop_id).filter((id): id is string => !!id))].sort();
    void (async () => {
      const fromWorkshop = new Map<string, AuthorLink[]>();
      try {
        for (const item of await TauriService.workshopDetails(ids)) if (item.support?.length) fromWorkshop.set(item.workshop_id, item.support);
      } catch {
        // Offline: the changelogs' links still go.
      }
      const logs = await Promise.all(installed.map((m) => TauriService.modChangelog(paths, m.mod_id)));
      const out = new Map<string, AuthorLink[]>();
      installed.forEach((m, i) => {
        const links = mergeLinks(logs[i]?.links ?? [], m.workshop_id ? fromWorkshop.get(m.workshop_id) : undefined);
        if (links.length > 0) out.set(m.mod_id, links);
      });
      if (!cancelled) setSupportByMod(out);
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths, installedKey]);

  /**
   * The game's Mods screen has the app's filters while the app is used: each mod's origin, whether
   * it is a library and its tags, in the language the Studio speaks.
   */
  useEffect(() => {
    if (!paths.is_valid || !paths.user_zomboid_dir || mods.length === 0) return;
    const tags = modTagsOf(mods, errorStats, loadTimes, health.catalogById, issuesByModOf(healthIssues), gameShares);
    void TauriService.writeLinkMods(paths, linkModsText(mods, health.catalogById, tags, supportByMod));
  }, [paths, mods, healthIssues, health.catalogById, errorStats, loadTimes, gameShares, supportByMod, i18next.language]);
  const loadErrorStats = useCallback(async () => {
    if (!paths.is_valid) return;
    const list = await TauriService.modErrorStats(paths);
    setErrorStats(new Map(list.map((s) => [s.mod_name, s])));
    const times = await TauriService.modLoadTimes(paths);
    setLoadTimes(new Map(times.mods.map((t) => [t.mod_id, t.millis])));
    const profile = (await TauriService.profilerStatus(paths)).report;
    setGameShares(
      new Map(
        (profile && profile.window_ms > 0 ? profile.mods : []).flatMap((m) => (m.mod_id ? [[m.mod_id, (m.ms / profile!.window_ms) * 100] as [string, number]] : []))
      )
    );
  }, [paths]);
  useEffect(() => {
    void loadErrorStats();
  }, [loadErrorStats]);

  // Mod Health checks again every time it is opened, besides its own button: what the list looks
  // like now, not when the tab was last looked at.
  useEffect(() => {
    if (activeTab === 'HEALTH') void health.recheck();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTab]);
  /** The issue a row's chip sent the player to: Mod Health scrolls to it and lights it up. */
  const [healthFocus, setHealthFocus] = useState<HealthIssue | null>(null);

  /** Whether the profiles were read once (until then "none active" is not known yet). */
  const [profilesRead, setProfilesRead] = useState(false);
  /** The welcome tour: once by itself on the first run, again from Help. */
  const [welcomeOpen, setWelcomeOpen] = useState(false);
  const [tourHighlight, setTourHighlight] = useState<ActiveTab | null>(null);
  // Not waiting for the profiles: with no Zomboid folder found they are never read, and that
  // player needs the tour most.
  useEffect(() => {
    if (!isInitializing && !welcomeSeen()) setWelcomeOpen(true);
  }, [isInitializing]);
  useEffect(() => {
    const open = () => setWelcomeOpen(true);
    window.addEventListener(SHOW_WELCOME_EVENT, open);
    return () => window.removeEventListener(SHOW_WELCOME_EVENT, open);
  }, []);
  const refreshActiveProfile = useCallback(async (customUserDir?: string): Promise<AppProfile | null> => {
    const userDir = customUserDir || paths.user_zomboid_dir;
    if (userDir) {
      try {
        const list: AppProfile[] = await TauriService.listInstances(userDir);
        const active = list.find((i) => i.is_active) || null;
        setProfiles(list);
        setActiveProfile(active);
        setProfilesRead(true);
        return active;
      } catch (err) {
        console.error('Error listing profiles:', err);
        return null;
      }
    }
    return null;
  }, [paths.user_zomboid_dir]);

  // Every screen but Profiles and Settings is about a profile (the sidebar locks them without
  // one): with none active, whatever was open goes back to Profiles.
  useEffect(() => {
    if (profilesRead && !activeProfile && activeTab !== 'PROFILES' && activeTab !== 'SETTINGS') setActiveTab('PROFILES');
  }, [profilesRead, activeProfile, activeTab]);

  // Load saved profile or auto-detect paths from Rust backend on initial load
  useEffect(() => {
    const initPaths = async () => {
      try {
        setInitStatusMessage('Checking Project Zomboid and Steam Workshop paths...');
        setInitProgress(25);
        const savedProfile = await TauriService.loadSavedPathsProfile();
        setPaths(savedProfile);

        if (savedProfile.is_valid) {
          setInitStatusMessage(i18next.t('app.init.reading'));
          setInitProgress(50);
          const active = await refreshActiveProfile(savedProfile.user_zomboid_dir);

          setInitStatusMessage(i18next.t('app.init.looking'));
          setInitProgress(75);
          // Scan all subscribed Workshop & local mods on disk (preserving saved absolute order)
          const allSubscribedMods = await TauriService.scanAllInstalledMods(savedProfile);
          if (allSubscribedMods.length > 0) {
            setMods([...allSubscribedMods]);
            setInitStatusMessage(`[Workshop] ${allSubscribedMods.length} mods loaded and indexed.`);
          }
          setConflicts([]);
          setInitProgress(90);

          // Determine initial startup screen based on settings and active profile presence
          const startupBehavior = localStorage.getItem('pz_startup_behavior') || 'SHOW_STARTUP_SCREEN';
          if (!active) {
            setActiveTab('PROFILES');
          } else if (startupBehavior === 'OPEN_LAST_PROFILE') {
            setActiveTab('MOD_LIST');
          } else {
            setActiveTab('PROFILES');
          }
        } else {
          setInitStatusMessage(i18next.t('app.init.setup'));
          setInitProgress(85);
        }

        setInitStatusMessage('Ready.');
        setInitProgress(100);
      } catch (e) {
        console.error('Initialization error:', e);
      } finally {
        // Long enough for the bar to be seen full; someone who skipped the intro is waiting.
        setTimeout(() => {
          setIsInitializing(false);
        }, 150);
      }
    };
    initPaths();
    // The console keeps streaming from the start, whichever tab is in front.
    startLiveConsole();
  }, []);


  // Listen to realtime sandbox logs & error cards from Rust watcher
  useEffect(() => {
    let unlistenErrorCards: (() => void) | undefined;

    TauriService.listenSandboxErrorCards((card) => {
      setErrorCards((prev) => {
        // One card per problem: the same shape of line again is the same problem. The title
        // alone used to decide, which folded three different lines of one mod into one card.
        const shape = errorShape(card.raw_error);
        const isDuplicate = prev.some(
          (c) =>
            (c.title === card.title && errorShape(c.raw_error) === shape) ||
            (c.polyfill_rule_id_suggestion && c.polyfill_rule_id_suggestion === card.polyfill_rule_id_suggestion)
        );
        if (isDuplicate) return prev;

        return [card, ...prev];
      });
    }).then((unlisten) => {
      unlistenErrorCards = unlisten;
    });

    // A relaunch writes a fresh console.txt: the cards are this session's, so the last one's go.
    let unlistenSession: (() => void) | undefined;
    TauriService.listenSandboxSessionStarted(() => {
      setErrorCards([]);
      void loadErrorStats();
    }).then((unlisten) => {
      unlistenSession = unlisten;
    });

    return () => {
      if (unlistenErrorCards) unlistenErrorCards();
      if (unlistenSession) unlistenSession();
    };
  }, []);

  // Accepting the game's Mods screen changes the active profile: show it the way the game left it.
  const pathsRef = useRef(paths);
  pathsRef.current = paths;
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    // A background job of the Studio panicked: shown like any failure, with where it happened.
    let unlistenPanic: (() => void) | null = null;
    TauriService.listenStudioPanic((text) => reportBackendError('Studio background job', text)).then((u) => {
      unlistenPanic = u;
    });
    // A launch-file change the game kept from happening while it ran, made once it closed.
    let unlistenAgents: (() => void) | null = null;
    TauriService.listenJavaAgentsSynced((changes) => {
      if (changes.removed.length === 0 && changes.restored.length === 0) return;
      reportNotice(i18next.t('notices.javaAgentsLater', {
        on: changes.restored.join(', ') || '-',
        off: changes.removed.join(', ') || '-',
      }));
    }).then((u) => {
      unlistenAgents = u;
    });
    // The game's error notice: "See in the Studio" opens the Monitor, which has the error's card.
    let unlistenNavigate: (() => void) | null = null;
    TauriService.listenStudioNavigate((screen) => {
      if (screen === 'monitor') setActiveTab('MONITOR');
    }).then((u) => {
      unlistenNavigate = u;
    });
    TauriService.listenProfileChangedInGame(async () => {
      const current = pathsRef.current;
      if (!current.is_valid) return;
      await refreshActiveProfile(current.user_zomboid_dir);
      const scanned = await TauriService.scanAllInstalledMods(current);
      if (scanned.length > 0) setMods([...scanned]);
    }).then((u) => {
      unlisten = u;
    });
    return () => {
      if (unlisten) unlisten();
      if (unlistenAgents) unlistenAgents();
      if (unlistenPanic) unlistenPanic();
      if (unlistenNavigate) unlistenNavigate();
    };
  }, [refreshActiveProfile]);



  /** `packageFolder` is the package the Merger has open, whose saved merges should be shown. */
  const handleRescan = async (packageFolder?: string) => {
    if (paths.is_valid) {
      const scannedConflicts = await TauriService.scanConflicts(paths, packageFolder ?? openedPackageFolder);
      setConflicts(scannedConflicts);
      const allSubscribedMods = await TauriService.scanAllInstalledMods(paths);
      if (allSubscribedMods.length > 0) {
        setMods([...allSubscribedMods]);
      }
    }
  };

  const handleRefreshMods = async () => {
    if (paths.is_valid) {
      const allSubscribedMods = await TauriService.scanAllInstalledMods(paths);
      setMods([...allSubscribedMods]);
    }
  };

  // A write anywhere (services/changes.ts): the mods and the profiles are read again a moment
  // later, so every screen shows what the button just did without being opened again. Through
  // a ref, since the listener lives as long as the app and these two change with the paths.
  const refreshAfterChange = useRef<{ mods: () => Promise<void>; profiles: () => Promise<unknown> }>({ mods: async () => undefined, profiles: async () => undefined });
  refreshAfterChange.current = { mods: handleRefreshMods, profiles: refreshActiveProfile };
  useEffect(() => {
    const pending = new Set<ChangeScope>();
    let timer: number | undefined;
    return onChange((scope) => {
      pending.add(scope);
      window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        const scopes = [...pending];
        pending.clear();
        if (scopes.includes('mods')) void refreshAfterChange.current.mods();
        if (scopes.includes('profiles') || scopes.includes('mods')) void refreshAfterChange.current.profiles();
      }, 300);
    });
  }, []);

  /**
   * Workshop items subscribed from Browse that Steam is still downloading, with the profile that
   * was active then. Steam downloads in the background, so the list is read again every few
   * seconds until each item lands; its mods then join that profile, switched off (an item can
   * hold several mods, and which to switch on is the player's call). After 20 minutes the Studio
   * stops waiting; anything that lands later is in "Showing everything".
   */
  const [awaitedItems, setAwaitedItems] = useState<Map<string, { profileId: string | null; since: number }>>(new Map());
  const handleSubscribed = useCallback(
    (workshopIds: string[]) => {
      setAwaitedItems((prev) => {
        const next = new Map(prev);
        for (const id of workshopIds) next.set(id, { profileId: activeProfile?.id ?? null, since: Date.now() });
        return next;
      });
    },
    [activeProfile]
  );
  useEffect(() => {
    if (awaitedItems.size === 0 || !paths.is_valid) return;
    let cancelled = false;
    const timer = window.setInterval(async () => {
      const scanned = await TauriService.scanAllInstalledMods(paths).catch(() => null);
      if (cancelled || !scanned) return;
      const landed = new Map<string, string[]>(); // profile id -> mod ids that arrived
      const done: string[] = [];
      for (const [itemId, wait] of awaitedItems) {
        const arrived = scanned.filter((m) => m.workshop_id === itemId).map((m) => m.mod_id);
        if (arrived.length > 0) {
          done.push(itemId);
          if (wait.profileId) landed.set(wait.profileId, [...(landed.get(wait.profileId) ?? []), ...arrived]);
        } else if (Date.now() - wait.since > 20 * 60 * 1000) {
          done.push(itemId);
        }
      }
      if (done.length === 0) return;
      setMods([...scanned]);
      const fresh: AppProfile[] = await TauriService.listInstances(paths.user_zomboid_dir).catch(() => []);
      for (const [profileId, modIds] of landed) {
        const profile = fresh.find((p) => p.id === profileId);
        if (!profile) continue;
        const members = profileMembers(profile);
        const added = modIds.filter((id) => !members.includes(id));
        if (added.length > 0) {
          await TauriService.setProfileMembers(paths.user_zomboid_dir, profileId, [...members, ...added]).catch(() => null);
        }
        // In the profile that is on, what arrived goes on too, with what it needs before it.
        if (profile.is_active) {
          const updated = enableWithRequirements(scanned, modIds);
          if (updated.some((m, i) => m.enabled !== scanned[i]?.enabled || m.mod_id !== scanned[i]?.mod_id)) {
            setMods(updated);
            autoPersistProfileChanges(updated);
          }
        }
      }
      void refreshActiveProfile();
      setAwaitedItems((prev) => {
        const next = new Map(prev);
        for (const id of done) next.delete(id);
        return next;
      });
    }, 6000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [awaitedItems, paths, refreshActiveProfile]);

  /**
   * Turns a Workshop collection into a profile: the mods you have installed from its items, kept
   * in the order your list already has them, so switching to the profile is one click. Items you
   * are not subscribed to cannot go in — there is nothing on disk to switch on — so they are
   * counted and named instead, and the collection's own Subscribe button is the way to get them.
   */
  /** Which half of the Servers screen is shown: joining someone's server, or hosting one. */
  const [serversView, setServersView] = useState<'JOIN' | 'HOST'>('JOIN');

  /** A profile with the mods a server loaded, in its order. */
  /** Explore Mods, on one Workshop item / on a Workshop search. */
  const openInBrowser = (workshopId: string) => {
    setBrowseFocusWorkshopId(workshopId);
    setActiveTab('BROWSE');
  };
  const searchInBrowser = (text: string) => {
    setBrowseFocusSearch(text);
    setActiveTab('BROWSE');
  };

  const handleMakeProfileFromServer = async (name: string, modIds: string[], announcedOf?: number, server?: { address: string; name: string }) => {
    const installed = new Set(mods.map((m) => m.mod_id));
    const missing = modIds.filter((id) => !installed.has(id));
    const partial = announcedOf ? i18next.t('join.profilePartial', { count: modIds.length, total: announcedOf }) : '';
    const ok = confirm(i18next.t(announcedOf ? 'join.profileConfirmAnnounced' : 'join.profileConfirm', { name, count: modIds.length, missing: (missing.length > 0 ? i18next.t('join.profileMissing', { count: missing.length }) : '') + partial }));
    if (!ok) return;
    try {
      const made = await TauriService.createInstance(paths.user_zomboid_dir, name, i18next.t('join.profileDescription'), modIds, modIds);
      // The server's pane then says it has a profile of its own.
      if (server && made?.id) await TauriService.setServerMadeProfile(paths, server.address, server.name, made.id);
      await refreshActiveProfile();
      setActiveTab('PROFILES');
    } catch (err) {
      reportBackendError('Making the profile', err);
    }
  };

  const handleMakeProfileFromCollection = async (name: string, workshopIds: string[]) => {
    const wanted = new Set(workshopIds);
    const included = mods.filter((m) => m.workshop_id && wanted.has(m.workshop_id));
    const haveItem = new Set(included.map((m) => m.workshop_id as string));
    const missing = workshopIds.filter((id) => !haveItem.has(id));

    if (included.length === 0) {
      alert(i18next.t('app.collection.noneInstalled', { count: workshopIds.length, name }));
      return;
    }
    const ok = confirm(i18next.t('app.collection.confirm', { name, count: included.length, missing: missing.length > 0 ? i18next.t('app.collection.missing', { count: missing.length }) : '' }));
    if (!ok) return;

    try {
      const ids = included.map((m) => m.mod_id);
      await TauriService.createInstance(paths.user_zomboid_dir, name, i18next.t('app.collection.description'), ids, ids);
      await refreshActiveProfile();
      setActiveTab('PROFILES');
    } catch (err) {
      reportBackendError('Making the profile', err);
    }
  };

  // Auto-refresh mod list when switching to MOD_LIST tab to reflect external additions/unsubscriptions
  useEffect(() => {
    if (activeTab === 'MOD_LIST' && paths.is_valid) {
      handleRefreshMods();
    }
  }, [activeTab]);

  const handleLoadMockups = (mockups: VfsConflict[]) => {
    setConflicts(mockups);
  };

  const handleLoadModMockups = (mockMods: ModInfo[]) => {
    setMods(mockMods);
  };

  // Auto-Merge All conflicts into the selected package (or the default one) on disk.
  // Returns whether the package was written, so callers only report success when it was.
  const handleAutoMergeAll = async (packageFolderName?: string): Promise<boolean> => {
    if (!paths.is_valid) {
      alert(i18next.t('app.pathsFirst'));
      return false;
    }

    try {

      const mergedFiles = conflicts.map((c) => ({
        relative_path: c.relative_path,
        content: c.merged_output || c.base_content,
      }));

      const result = await TauriService.generateMasterPatch({
        workshop_dir: paths.workshop_dir,
        pz_install_dir: paths.pz_install_dir,
        user_zomboid_dir: paths.user_zomboid_dir,
        mod_list_ini_path: paths.mod_list_ini_path,
        merged_files: mergedFiles,
        package_folder_name: packageFolderName || 'Z_PZModStudio_MergedPatch',
      });
      if (!result.success) return false;

      setAutoMergeResultModal({
        patch_mod_dir: result.patch_mod_dir,
        total_conflicts: conflicts.length,
        files_written: result.files_written,
        polyfill_shim_written: result.polyfill_shim_written,
      });

      await handleRefreshMods();
      return true;
    } catch {
      // Already shown to the user by TauriService.
      return false;
    }
  };

  // Writes the merge package to disk and puts it in the mod list the game reads.
  const handleOptimizeAndResolve = async (packageFolderName?: string): Promise<boolean> => {
    const totalCount = conflicts.length;

    // Call Rust to generate synthetic Master Patch mod on disk
    if (paths.is_valid && paths.user_zomboid_dir) {
      const mergedFilesPayload = conflicts.map((c) => ({
        relative_path: c.relative_path,
        content: c.merged_output || c.base_content,
      }));


      let result;
      try {
        result = await TauriService.generateMasterPatch({
          workshop_dir: paths.workshop_dir,
          pz_install_dir: paths.pz_install_dir,
          user_zomboid_dir: paths.user_zomboid_dir,
          mod_list_ini_path: paths.mod_list_ini_path,
          merged_files: mergedFilesPayload,
          package_folder_name: packageFolderName,
        });
      } catch {
        return false; // Already shown to the user by TauriService; conflicts stay unresolved.
      }
      if (!result.success) return false;

      // Mark all conflicts as resolved
      setConflicts((prev) =>
        prev.map((c) => ({
          ...c,
          status: 'RESOLVED' as const,
        }))
      );

      // Refresh installed mods so the newly published package appears in the Mod List tab immediately
      const allSubscribedMods = await TauriService.scanAllInstalledMods(paths);
      if (allSubscribedMods.length > 0) {
        setMods([...allSubscribedMods]);
      }

      setAutoMergeResultModal({
        patch_mod_dir: result.patch_mod_dir,
        total_conflicts: totalCount,
        files_written: result.files_written,
        polyfill_shim_written: result.polyfill_shim_written,
      });
      return true;
    }
    return false;
  };

  const [pendingLaunchMode, setPendingLaunchMode] = useState<GameLaunchMode | null>(null);
  /** Where the launch going on goes (the launcher's Custom): read by executeLaunch, which a
   * warning dialog may call later. */
  const launchTargetRef = useRef<LaunchTarget>({ kind: 'menu' });
  // Read again on every launch: Settings can switch them while the app is open.
  const [skipNoMergeWarning, setSkipNoMergeWarning] = useState<boolean>(() => localStorage.getItem('pz_skip_no_merge_warning') === 'true');
  const [skipDefinitionWarning, setSkipDefinitionWarning] = useState<boolean>(() => localStorage.getItem('pz_skip_definition_warning') === 'true');

  /** Files the active mods share that the active package does not merge, or merges from older copies. */
  const unmergedFiles = (health.report?.issues ?? []).filter((i) => i.code === 'file_conflict' || i.code === 'file_conflict_stale');
  /** Items, recipes or texts two active mods define differently, with no package settling which wins
   * (or settled before a mod changed). Not broken, the last mod decides: so a softer notice. */
  const unsettledDefinitions = (health.report?.issues ?? []).filter((i) =>
    ['script_definition_conflict', 'script_definition_stale', 'translation_conflict', 'translation_stale'].includes(i.code));

  const executeLaunch = async (mode: GameLaunchMode = 'DEBUG_FULLSCREEN', onlyIds?: string[]) => {
    try {
      // A caller that has just changed the list passes it: `mods` here is still the old render's.
      const activeModIds = onlyIds ?? mods.filter((m) => m.enabled).map((m) => m.mod_id);
      await TauriService.writeModListIni(paths.mod_list_ini_path, activeModIds);

      // The save the Link loads at the main menu (Custom), or none: a request left from an
      // earlier launch must not load a save now.
      const target = launchTargetRef.current;
      try {
        await TauriService.setLaunchSave(paths, target.kind === 'save' ? target.id : null);
      } catch (err) {
        if (target.kind === 'save') {
          reportBackendError(i18next.t('header.target.saveFailed', { name: target.label }), err);
          return;
        }
      }

      await TauriService.launchSandbox({
        pz_install_dir: paths.pz_install_dir,
        user_zomboid_dir: paths.user_zomboid_dir,
        test_mode: mode,
      });

      // The profile now carries the build it was played on, for the day the game is updated.
      if (activeProfile) {
        await TauriService.markProfilePlayed(paths.user_zomboid_dir, activeProfile.id);
        void refreshActiveProfile();
      }

      if (mode.includes('DEBUG') || mode === 'MONITORED' || mode === 'WINDOWED') {
        setActiveTab('MONITOR');
      }
    } catch {
      // The failed step (saving the mod list or launching) was already shown by TauriService;
      // the game is not started with a mod list that was not saved.
    }
  };

  const handleRunSandbox = async (mode: GameLaunchMode = 'DEBUG_FULLSCREEN', target: LaunchTarget = { kind: 'menu' }) => {
    launchTargetRef.current = target;
    // A server loads its own mods: none of the list's checks apply. Joined the way Servers does
    // (kept password, username, Favorites), in the launcher's mode.
    if (target.kind === 'server') {
      try {
        await TauriService.joinServer(paths, target.address, target.name, null, null, mode);
        if (mode.includes('DEBUG')) setActiveTab('MONITOR');
      } catch (err) {
        reportBackendError(i18next.t('header.target.serverFailed', { name: target.name }), err);
      }
      return;
    }
    // A profile last played on another major build: its saves will not open on this one, and the
    // player may be about to find that out the hard way. One question before the game starts.
    const installed = health.report?.game_version ?? null;
    const profileMajor = profileBuildMajor(activeProfile);
    if (installed && profileMajor && profileMajor !== installed.split('.')[0]) {
      const why = activeProfile?.target_build
        ? i18next.t('app.build.setFor', { name: activeProfile.name, build: profileMajor })
        : i18next.t('app.build.lastPlayed', { name: activeProfile!.name, build: activeProfile!.last_played_build });
      const ok = window.confirm(i18next.t('app.build.confirm', { why, installed, profile: profileMajor, major: installed.split('.')[0] }));
      if (!ok) return;
    }

    // Check if there is an active CLOSED/PACKAGED synthetic merge package (excluding Carrier and Bridge)
    const hasActiveClosedMergePackage = mods.some(
      (m) => m.enabled && m.mod_id.startsWith('Z_PZModStudio_') && m.is_packaged === true && m.mod_id !== 'Z_PZModStudio_Bridge' && !m.mod_id.includes('Carrier')
    );

    const openDraftPackages = mods.filter(
      (m) => m.mod_id.startsWith('Z_PZModStudio_') && m.mod_id !== 'Z_PZModStudio_Bridge' && !m.mod_id.includes('Carrier') && m.is_packaged === false
    );

    // A package being edited is switched off: what it fixes is not in the game until it is published.
    if (!hasActiveClosedMergePackage && openDraftPackages.length > 0) {
      setPendingLaunchMode(mode);
      return;
    }
    // Without a package nothing is wrong by itself: the warning is for files the active mods share
    // that no package merges (or merges from older versions of the mods), as Mod Health finds them.
    // Then, more softly, things two mods define differently: one of them wins without a word.
    const skipFiles = localStorage.getItem('pz_skip_no_merge_warning') === 'true';
    const skipDefinitions = localStorage.getItem('pz_skip_definition_warning') === 'true';
    setSkipNoMergeWarning(skipFiles);
    setSkipDefinitionWarning(skipDefinitions);
    if ((!skipFiles && unmergedFiles.length > 0) || (!skipDefinitions && unsettledDefinitions.length > 0)) {
      setPendingLaunchMode(mode);
      return;
    }

    await executeLaunch(mode);
  };

  const handleResolveConflict = async (
    conflictId: string,
    resolvedCode: string,
    packageFolderName?: string,
    relativePath?: string
  ) => {
    setConflicts((prev) =>
      prev.map((c) =>
        c.id === conflictId ? { ...c, merged_output: resolvedCode, status: 'RESOLVED' as const } : c
      )
    );

    // A mod's own file opened to patch is not in this list -- only one mod ships it, so the scan
    // never listed it -- and its Save used to find nothing here and write nothing to disk. The
    // Merger says which path it is; once the package holds the decision the scan lists it too.
    const targetConflict = conflicts.find((c) => c.id === conflictId);
    const path = targetConflict?.relative_path ?? relativePath;
    if (path && paths.user_zomboid_dir) {
      await TauriService.saveDraftResolution(
        paths.user_zomboid_dir,
        packageFolderName || 'Z_PZModStudio_MergedPatch',
        path,
        resolvedCode,
        'RESOLVED'
      );
      if (!targetConflict) await handleRescan(packageFolderName);
    }
  };


  const autoPersistProfileChanges = async (updatedMods: ModInfo[]) => {
    const userDir = paths.user_zomboid_dir || '';
    const activeModIds = updatedMods.filter((m) => m.enabled).map((m) => m.mod_id);
    const fullLoadOrder = updatedMods.map((m) => m.mod_id);

    try {
      await TauriService.saveMasterLoadOrder(userDir, fullLoadOrder, activeModIds);
    } catch (err) {
      reportBackendError('Saving the load order', err);
    }
  };

  const handleReorderMods = (newOrder: ModInfo[]) => {
    setMods([...newOrder]);
    autoPersistProfileChanges(newOrder);
  };

  /** A switch that brings other mods with it, waiting for the player to see it. */
  const [pendingPlan, setPendingPlan] = useState<TogglePlan | null>(null);
  /** A mod another screen wants shown in the Mod List. */
  const [modListFocus, setModListFocus] = useState<string | null>(null);
  const applyTogglePlan = (plan: TogglePlan, withExtras: boolean) => {
    const updated = applyPlan(mods, plan, withExtras);
    setPendingPlan(null);
    setMods(updated);
    autoPersistProfileChanges(updated);
  };

  const handleToggleMod = (modId: string) => {
    const target = mods.find((m) => m.mod_id === modId);
    if (!target) return;
    // With a profile active the Studio Link mod stays on: it is how the game shows the profile.
    if (modId === STUDIO_LINK_MOD_ID && activeProfile && target.enabled) return;

    // An ordinary mod: what it needs comes on with it, what needs it is named before it goes
    // off, and anything beyond the one mod is shown first.
    if (!modId.startsWith('Z_PZModStudio_')) {
      const plan = planToggle(mods, modId);
      if (!plan) return;
      if (planIsSimple(plan)) applyTogglePlan(plan, true);
      else setPendingPlan(plan);
      return;
    }

    const isSyntheticFusionPackage =
      modId.startsWith('Z_PZModStudio_') &&
      !modId.includes('Carrier') &&
      modId !== 'Z_PZModStudio_Bridge';

    // Block activation of open / draft packages (is_packaged === false)
    if (isSyntheticFusionPackage && target.is_packaged === false) {
      return;
    }

    {
      const willEnable = !target.enabled;

      const updated = mods.map((m) => {
        if (m.mod_id === modId) {
          return { ...m, enabled: willEnable };
        }
        // If enabling a synthetic fusion package, ensure only ONE fusion package is active at a time!
        // (The Live Bridge companion mod is independent and can remain active simultaneously)
        if (
          willEnable &&
          isSyntheticFusionPackage &&
          m.mod_id.startsWith('Z_PZModStudio_') &&
          !m.mod_id.includes('Carrier') &&
          m.mod_id !== 'Z_PZModStudio_Bridge'
        ) {
          return { ...m, enabled: false };
        }
        return m;
      });

      // Persist outside the state updater: updaters may run twice (StrictMode) and must stay pure.
      setMods(updated);
      autoPersistProfileChanges(updated);
    }
  };

  const handleApplyPresetLoadOrder = (presetLoadOrder: string[], activeIds: string[]) => {
    {
      const modMap = new Map(mods.map((m) => [m.mod_id, m]));
      const activeSet = new Set(activeIds);

      const reordered: ModInfo[] = [];
      presetLoadOrder.forEach((id) => {
        const found = modMap.get(id);
        if (found) {
          reordered.push({ ...found, enabled: activeSet.has(id) });
          modMap.delete(id);
        }
      });

      // Append remaining mods
      modMap.forEach((m) => {
        reordered.push({ ...m, enabled: activeSet.has(m.mod_id) });
      });

      setMods(reordered);
      autoPersistProfileChanges(reordered);
    }
    refreshActiveProfile();
  };

  const handleClearErrorCards = () => {
    setErrorCards([]);
  };

  /**
   * Developer: the game with only one mod on. The list as it is goes aside (Earlier lists keeps
   * it too), the mod with what it requires and the Studio's bridge go on, the game starts in
   * Debug (Windowed), and the list comes back by itself once the game has closed.
   */
  // Kept in localStorage too: if the Studio is closed while the game runs, the list is put back
  // the next time it opens instead of staying at one mod.
  const [devLaunch, setDevLaunchState] = useState<{ modId: string; since: string; saved: ModInfo[] } | null>(() => {
    try {
      const raw = localStorage.getItem(DEV_LAUNCH_KEY);
      return raw ? JSON.parse(raw) : null;
    } catch {
      return null;
    }
  });
  const setDevLaunch = (value: { modId: string; since: string; saved: ModInfo[] } | null) => {
    setDevLaunchState(value);
    try {
      if (value) localStorage.setItem(DEV_LAUNCH_KEY, JSON.stringify(value));
      else localStorage.removeItem(DEV_LAUNCH_KEY);
    } catch {
      // Private storage off: the list still comes back while the Studio stays open.
    }
  };
  const devLaunchFromEarlierSession = useRef(devLaunch !== null);
  const restoreDevList = useCallback(() => {
    if (!devLaunch) return;
    devLaunchFromEarlierSession.current = false;
    const saved = devLaunch.saved;
    setDevLaunch(null);
    handleApplyPresetLoadOrder(saved.map((m) => m.mod_id), saved.filter((m) => m.enabled).map((m) => m.mod_id));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [devLaunch, mods]);
  const launchOnly = async (modId: string) => {
    const wanted = new Set<string>([modId, 'Z_PZModStudio_Bridge']);
    const byId = new Map(mods.map((m) => [m.mod_id, m]));
    const queue = [modId];
    while (queue.length) {
      for (const dep of byId.get(queue.shift()!)?.dependencies ?? []) {
        const found = mods.find((m) => m.mod_id === dep || m.mod_id.endsWith(`/${dep}`));
        if (found && !wanted.has(found.mod_id)) {
          wanted.add(found.mod_id);
          queue.push(found.mod_id);
        }
      }
    }
    setDevLaunch({ modId, since: new Date().toLocaleTimeString(), saved: mods.map((m) => ({ ...m })) });
    handleApplyPresetLoadOrder(mods.map((m) => m.mod_id), [...wanted]);
    await executeLaunch('DEBUG_WINDOWED', mods.map((m) => m.mod_id).filter((id) => wanted.has(id)));
  };
  useEffect(() => {
    if (!devLaunch || mods.length === 0) return;
    // Left from a session that closed while the game ran: once the game is not running, the
    // list comes back at the first check.
    let seenRunning = devLaunchFromEarlierSession.current;
    const t = setInterval(() => {
      void TauriService.gameStatus().then((pid) => {
        if (pid !== null) seenRunning = true;
        else if (seenRunning) restoreDevList();
      });
    }, 5000);
    return () => clearInterval(t);
  }, [devLaunch, restoreDevList]);

  const handleSavePaths = async (updatedPaths: StudioPathsUI) => {
    const saved = await TauriService.savePathsProfile(updatedPaths);
    setPaths(saved);
    if (saved.is_valid) {
      await refreshActiveProfile(saved.user_zomboid_dir);
      const allSubscribedMods = await TauriService.scanAllInstalledMods(saved);
      if (allSubscribedMods.length > 0) {
        setMods([...allSubscribedMods]);
      }
    }
  };

  const handleAutoDetect = async () => {
    const autoPaths = await TauriService.getAutoPaths();
    const saved = await TauriService.savePathsProfile(autoPaths);
    setPaths(saved);
    if (saved.is_valid) {
      await refreshActiveProfile(saved.user_zomboid_dir);
      const allSubscribedMods = await TauriService.scanAllInstalledMods(saved);
      if (allSubscribedMods.length > 0) {
        setMods([...allSubscribedMods]);
      }
    }
  };

  return (
    <ContextMenuProvider>
      <SplashScreen
        isLoading={isInitializing}
        statusMessage={initStatusMessage}
        progress={initProgress}
      />
      <div className="h-screen w-screen flex flex-col bg-slate-950 text-slate-100 overflow-hidden font-sans">
      {/* Studio Header */}
      <StudioHeader
        conflictCount={conflicts.filter((c) => c.status !== 'RESOLVED' && c.status !== 'AUTO_MERGED').length}
        resolvedConflictCount={conflicts.filter((c) => c.status === 'RESOLVED' || c.status === 'AUTO_MERGED').length}
        onNavigateToMerger={() => setActiveTab('MERGER')}
        activeProfileName={activeProfile?.name}
        installedBuild={health.report?.game_version ?? null}
        install={health.report?.install ?? null}
        profileBuild={activeProfile?.last_played_build ?? null}
        profileTarget={activeProfile?.target_build ?? null}
        onNavigateToProfiles={() => setActiveTab('PROFILES')}
        onRunSandbox={handleRunSandbox}
        paths={paths}
        activeProfileId={activeProfile?.id ?? null}
      />
      {/* A newer PZ Mod Studio on GitHub (once a day; Settings can switch it off). */}
      {!isInitializing && <UpdateNotice userZomboidDir={paths.user_zomboid_dir} />}
      {/* The help of the screen in front: "?" in the corner, or F1. */}
      <HelpButton tab={activeTab} />
      <WelcomeTour
        open={welcomeOpen}
        onClose={() => setWelcomeOpen(false)}
        paths={paths}
        activeProfileName={activeProfile?.name ?? null}
        profileCount={profiles.length}
        installedMods={mods.length}
        health={health.report}
        onShowTab={setActiveTab}
        onHighlight={setTourHighlight}
      />


      {/* Main Studio Body */}
      <div className="flex-1 flex overflow-hidden">
        {/* StudioSidebar */}
        <StudioSidebar
          activeTab={activeTab}
          setActiveTab={setActiveTab}
          hasActiveProfile={!!activeProfile}
          highlightTab={tourHighlight}
          conflictCount={conflicts.length}
          errorCardCount={errorCards.filter((c) => c.severity !== 'WARNING').length}
          healthErrorCount={health.report?.errors ?? 0}
          whatsNewCount={whatsNewCount}
        />

        {/* Tab Modules */}
        <main className="flex-1 flex flex-col min-h-0 min-w-0 overflow-hidden bg-slate-950">
          {activeTab === 'PROFILES' && (
            <div className="flex-1 flex flex-col min-h-0 overflow-y-auto p-6 font-sans">
              <InstanceModule
                paths={paths}
                mods={mods}
                onApplyInstanceLoadOrder={handleApplyPresetLoadOrder}
                onProfileDeleted={() => {
                  setActiveProfile(null);
                  setActiveTab('PROFILES');
                }}
                onProfileActivated={(inst) => {
                  setActiveProfile(inst);
                  setActiveTab('MOD_LIST');
                }}
                onGoToBrowse={activeProfile ? () => setActiveTab('BROWSE') : undefined}
                onProfilesChanged={() => void refreshActiveProfile()}
                onRefreshMods={handleRefreshMods}
              />
            </div>
          )}

          {activeTab === 'SAVES' && <SavesModule paths={paths} onOpenWorkshopItem={openInBrowser} onSearchWorkshop={searchInBrowser} />}
          {activeTab === 'WHATS_NEW' && <WhatsNewModule paths={paths} />}
          {activeTab === 'MAPS' && (
            <MapsModule
              paths={paths}
              activeKey={mods.filter((m) => m.enabled).map((m) => m.mod_id).join('|')}
              onShowMod={(modId) => {
                setModListFocus(modId);
                setActiveTab('MOD_LIST');
              }}
              onTurnOff={(modId) => {
                const target = mods.find((m) => m.mod_id === modId);
                if (!target || !target.enabled) return;
                if (!confirm(i18next.t('app.switchOff.confirm', { name: target.name }))) return;
                handleToggleMod(target.mod_id);
              }}
            />
          )}

          {activeTab === 'MOD_LIST' && (
            <LoadOrderModule
              paths={paths}
              mods={mods}
              onReorder={handleReorderMods}
              onToggleMod={handleToggleMod}
              onRefreshMods={handleRefreshMods}
              onGoToSettings={() => setActiveTab('SETTINGS')}
              onLoadMockups={handleLoadModMockups}
              onApplyPresetLoadOrder={handleApplyPresetLoadOrder}
              openedPackageFolder={openedPackageFolder}
              healthIssues={health.report?.issues ?? null}
              catalogById={health.catalogById}
              errorStats={errorStats}
              loadTimes={loadTimes}
              gameShares={gameShares}
              installedBuild={health.report?.game_version ?? null}
              onOpenMerger={(relativePath) => {
                setMergerFocusPath(relativePath);
                setActiveTab('MERGER');
              }}
              onGoToHealth={(issue) => {
                setHealthFocus(issue ?? null);
                setActiveTab('HEALTH');
              }}
              onRulesChanged={health.recheck}
              profiles={profiles}
              activeProfileId={activeProfile?.id ?? null}
              onProfilesChanged={() => void refreshActiveProfile()}
              onOpenInBrowser={openInBrowser}
              onSearchInBrowser={searchInBrowser}
              focusModId={modListFocus}
              onFocusHandled={() => setModListFocus(null)}
            />
          )}

          {activeTab === 'SERVERS' && (
            <div className="flex-1 flex flex-col min-h-0 overflow-y-auto p-6 font-sans">
              <div className="flex items-center gap-1 p-1 mb-4 rounded-lg bg-slate-900 border border-slate-800 w-fit">
                {(['JOIN', 'HOST'] as const).map((k) => (
                  <button key={k} onClick={() => setServersView(k)} className={`px-4 py-1.5 rounded-md text-xs font-semibold cursor-pointer ${serversView === k ? 'bg-emerald-700/40 text-emerald-100' : 'text-slate-400 hover:text-slate-200'}`}>
                    {k === 'JOIN' ? i18next.t('join.tabJoin') : i18next.t('join.tabHost')}
                  </button>
                ))}
              </div>
              {serversView === 'JOIN' ? (
                <JoinServers paths={paths} mods={mods} profiles={profiles} installedBuild={health.report?.game_version ?? null} onMakeProfile={handleMakeProfileFromServer} />
              ) : (
                <ServerModule
                  paths={paths}
                  mods={mods}
                />
              )}
            </div>
          )}

          {activeTab === 'BROWSE' && (
            <BrowseModule
              paths={paths}
              catalog={health.catalog}
              onInstalled={handleRefreshMods}
              onSubscribed={handleSubscribed}
              focusWorkshopId={browseFocusWorkshopId}
              onFocusHandled={() => setBrowseFocusWorkshopId(null)}
              focusSearch={browseFocusSearch}
              onSearchHandled={() => setBrowseFocusSearch(null)}
              onMakeProfile={handleMakeProfileFromCollection}
              installedBuild={health.report?.game_version ?? null}
              profiles={profiles}
              onProfilesChanged={() => void refreshActiveProfile()}
            />
          )}

          {activeTab === 'HEALTH' && (
            <HealthModule
              paths={paths}
              mods={mods}
              report={health.report}
              checking={health.checking}
              onRecheck={health.recheck}
              focusIssue={healthFocus}
              onFocusHandled={() => setHealthFocus(null)}
              onApplyModList={handleReorderMods}
              onModsChanged={handleRefreshMods}
              onOpenMerger={(relativePath) => {
                setMergerFocusPath(relativePath);
                setActiveTab('MERGER');
              }}
              onOpenInBrowser={openInBrowser}
              onSearchInBrowser={searchInBrowser}
            />
          )}

          {activeTab === 'MERGER' && (
            <Suspense fallback={<div className="flex-1 flex items-center justify-center text-xs text-slate-500">{i18next.t('app.loadingMerger')}</div>}>
            <MergerModule
              conflicts={conflicts}
              mods={mods}
              paths={paths}
              onResolveConflict={handleResolveConflict}
              onAutoMergeAll={handleAutoMergeAll}
              onOptimizeAndResolve={handleOptimizeAndResolve}
              onGoToSettings={() => setActiveTab('SETTINGS')}
              onRescan={handleRescan}
              onClearConflicts={() => setConflicts([])}
              onLoadMockups={handleLoadMockups}
              onToggleMod={handleToggleMod}
              onRefreshMods={handleRefreshMods}
              onPackageOpened={(folder) => setOpenedPackageFolder(folder)}
              onPackageClosed={() => setOpenedPackageFolder(null)}
              focusRelativePath={mergerFocusPath}
              onFocusHandled={() => setMergerFocusPath(null)}
              patchFocus={mergerPatchFocus}
              onPatchFocusHandled={() => setMergerPatchFocus(null)}
              profiles={profiles}
              activeProfileId={activeProfile?.id ?? null}
              onProfilesChanged={() => void refreshActiveProfile()}
            />
            </Suspense>
          )}

          {activeTab === 'MONITOR' && (
            <SandboxModule
              paths={paths}
              mods={mods}
              onRefreshMods={handleRefreshMods}
              errorCards={errorCards}
              onOpenMerger={() => setActiveTab('MERGER')}
              onPatchMod={(modId, fileHint, fix) => {
                setMergerPatchFocus({ modId, fileHint: fileHint ?? null, fix: fix ?? null });
                setActiveTab('MERGER');
              }}
              onSwitchOffMod={(modName) => {
                // The log gives a display name, which is what the Mod List shows too.
                const target = mods.find((m) => m.name === modName || m.mod_id === modName);
                if (!target) {
                  alert(i18next.t('app.switchOff.gone', { name: modName }));
                  return;
                }
                if (!target.enabled) {
                  alert(i18next.t('app.switchOff.already', { name: target.name }));
                  return;
                }
                if (!confirm(i18next.t('app.switchOff.confirm', { name: target.name }))) return;
                handleToggleMod(target.mod_id);
                setActiveTab('MOD_LIST');
              }}
              onClearErrorCards={handleClearErrorCards}
              onDismissErrorCard={(id) => setErrorCards((prev) => prev.filter((c) => c.id !== id))}
              onGoToSettings={() => setActiveTab('SETTINGS')}
            />
          )}

          {activeTab === 'DEVELOPER' && (
            <DeveloperModule
              paths={paths}
              mods={mods}
              catalogById={health.catalogById}
              onLaunchOnly={(modId) => void launchOnly(modId)}
              devLaunch={devLaunch ? { modId: devLaunch.modId, since: devLaunch.since } : null}
              onRestoreList={restoreDevList}
              onRefreshMods={handleRefreshMods}
            />
          )}

          {activeTab === 'SETTINGS' && (
            // Settings pads and scrolls itself, like every other screen.
            <div className="flex-1 flex flex-col min-h-0 font-sans">
              <SettingsModule
                paths={paths}
                onSavePaths={handleSavePaths}
                onAutoDetect={handleAutoDetect}
              />
            </div>
          )}
        </main>
      </div>


      {/* Sleek Native Auto-Merge Completion Modal */}
      {autoMergeResultModal && (
        <div className="fixed inset-0 bg-slate-950/80 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div className="bg-slate-900 border border-emerald-500/40 rounded-2xl p-6 max-w-lg w-full space-y-4 shadow-2xl animate-fade-in text-slate-200">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 flex items-center justify-center shrink-0">
                <Sparkles className="w-5 h-5 text-emerald-400" />
              </div>
              <div>
                <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
                  <span>{i18next.t('app.autoMerge.title')}</span>
                </h3>
                <p className="text-xs text-slate-400 mt-0.5">{i18next.t('app.autoMerge.blurb')}</p>
              </div>
            </div>

            <div className="bg-slate-950 border border-slate-800 rounded-xl p-4 space-y-2.5 text-xs font-mono">
              <div className="text-emerald-400 font-bold flex items-center gap-2">
                <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
                <span>{i18next.t('app.autoMerge.patchOnDisk')}</span>
              </div>
              <div className="p-2 bg-slate-900 rounded border border-slate-800 text-[11px] text-slate-300 break-all select-all font-mono">
                {autoMergeResultModal.patch_mod_dir}
              </div>
              <div className="grid grid-cols-2 gap-2 pt-1 text-slate-300">
                <div className="p-2 bg-slate-900/60 rounded border border-slate-800/80">
                  <span className="text-slate-400 text-[10px] block">{i18next.t('app.autoMerge.conflicts')}</span>
                  <b className="text-emerald-400 text-sm">{autoMergeResultModal.total_conflicts}</b>
                </div>
                <div className="p-2 bg-slate-900/60 rounded border border-slate-800/80">
                  <span className="text-slate-400 text-[10px] block">{i18next.t('app.autoMerge.files')}</span>
                  <b className="text-emerald-400 text-sm">{autoMergeResultModal.files_written}</b>
                </div>
              </div>
            </div>

            <div className="flex items-center justify-end pt-2">
              <button
                onClick={() => setAutoMergeResultModal(null)}
                className="px-5 py-2.5 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white text-xs font-bold rounded-lg cursor-pointer shadow-lg transition"
              >
                {i18next.t('app.autoMerge.accept')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Launch Warning Modal — an open package, unmerged shared files, or definitions two mods disagree on */}
      {pendingLaunchMode && (() => {
        const openPackages = mods.filter(
          (m) => m.mod_id.startsWith('Z_PZModStudio_') && m.mod_id !== 'Z_PZModStudio_Bridge' && !m.mod_id.includes('Carrier') && m.is_packaged === false
        );
        const isOpenPackageWarning = openPackages.length > 0 && !mods.some(
          (m) => m.enabled && m.mod_id.startsWith('Z_PZModStudio_') && m.is_packaged === true && m.mod_id !== 'Z_PZModStudio_Bridge' && !m.mod_id.includes('Carrier')
        );
        const openPackageNames = openPackages.map((p) => p.name || p.mod_id).join(', ');
        const SHOWN = 5;
        // Each part shows while it is not switched off; the box takes the tone of the worst one.
        const files = !isOpenPackageWarning && !skipNoMergeWarning ? unmergedFiles : [];
        const definitions = !isOpenPackageWarning && !skipDefinitionWarning ? unsettledDefinitions : [];
        const soft = !isOpenPackageWarning && files.length === 0;
        const definitionName = (i: HealthIssue) => i.params?.what || i.params?.named || i.mod_id || '';
        const toggle = (key: string, set: (v: boolean) => void) => (e: React.ChangeEvent<HTMLInputElement>) => {
          set(e.target.checked);
          localStorage.setItem(key, e.target.checked ? 'true' : 'false');
        };

        return (
        <div className="fixed inset-0 bg-slate-950/80 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div className={`bg-slate-900 border rounded-2xl p-6 max-w-md w-full space-y-4 shadow-2xl animate-fade-in text-slate-200 ${soft ? 'border-sky-500/50' : 'border-amber-500/50'}`}>
            <div className="flex items-start gap-3.5">
              <div className={`w-10 h-10 rounded-xl border flex items-center justify-center shrink-0 ${soft ? 'bg-sky-500/10 border-sky-500/30' : 'bg-amber-500/10 border-amber-500/30'}`}>
                {soft ? <Info className="w-5 h-5 text-sky-400" /> : <AlertTriangle className="w-5 h-5 text-amber-400" />}
              </div>
              <div className="space-y-1">
                <h3 className="text-base font-bold text-slate-100">
                  {isOpenPackageWarning ? i18next.t('app.launch.openTitle') : soft ? i18next.t('app.launch.definitionsTitle') : i18next.t('app.launch.unmergedTitle')}
                </h3>
                {isOpenPackageWarning ? (
                  <p className="text-xs text-amber-300/90 font-medium" dangerouslySetInnerHTML={{ __html: tHtml('app.launch.openLine', { names: openPackageNames }) }} />
                ) : soft ? (
                  <p className="text-xs text-sky-300/90 font-medium">{i18next.t('app.launch.definitionsLine', { count: definitions.length })}</p>
                ) : (
                  <p className="text-xs text-amber-300/90 font-medium">{i18next.t('app.launch.unmergedLine', { count: files.length })}</p>
                )}
              </div>
            </div>

            {isOpenPackageWarning && (
              <p className="text-xs text-slate-300 leading-relaxed bg-slate-950/60 p-3 rounded-xl border border-slate-800">{i18next.t('app.launch.openBody')}</p>
            )}

            {files.length > 0 && (
              <div className="text-xs text-slate-300 leading-relaxed bg-slate-950/60 p-3 rounded-xl border border-slate-800 space-y-2">
                <p>{i18next.t('app.launch.unmergedBody')}</p>
                <ul className="space-y-1">
                  {files.slice(0, SHOWN).map((i) => (
                    <li key={`${i.code}|${i.params?.path ?? i.message}`} className="text-[11px]">
                      <span className="font-mono text-amber-200 break-all">{i.params?.path}</span>
                      <span className="text-slate-500"> · {i.params?.mods}{i.code === 'file_conflict_stale' ? ` · ${i18next.t('app.launch.stale')}` : ''}</span>
                    </li>
                  ))}
                  {files.length > SHOWN && <li className="text-[11px] text-slate-500">{i18next.t('app.launch.moreFiles', { count: files.length - SHOWN })}</li>}
                </ul>
                <label className="flex items-center gap-2 pt-1 text-[11px] text-slate-400 select-none cursor-pointer">
                  <input type="checkbox" checked={skipNoMergeWarning} onChange={toggle('pz_skip_no_merge_warning', setSkipNoMergeWarning)} className="rounded border-slate-700 accent-amber-500 bg-slate-950 cursor-pointer" />
                  {i18next.t('app.launch.dontShow')}
                </label>
              </div>
            )}

            {definitions.length > 0 && (
              <div className="text-xs text-slate-300 leading-relaxed bg-slate-950/60 p-3 rounded-xl border border-slate-800 space-y-2">
                {!soft && <p className="font-semibold text-sky-300">{i18next.t('app.launch.definitionsLine', { count: definitions.length })}</p>}
                <p>{i18next.t('app.launch.definitionsBody')}</p>
                <ul className="space-y-1">
                  {definitions.slice(0, SHOWN).map((i) => (
                    <li key={`${i.code}|${i.message}`} className="text-[11px]">
                      <span className="text-sky-200">{definitionName(i)}</span>
                      <span className="text-slate-500"> · {i18next.t('app.launch.wins', { winner: i.params?.winner ?? '', losers: i.params?.losers ?? '' })}{i.code.endsWith('_stale') ? ` · ${i18next.t('app.launch.stale')}` : ''}</span>
                    </li>
                  ))}
                  {definitions.length > SHOWN && <li className="text-[11px] text-slate-500">{i18next.t('app.launch.moreFiles', { count: definitions.length - SHOWN })}</li>}
                </ul>
                <label className="flex items-center gap-2 pt-1 text-[11px] text-slate-400 select-none cursor-pointer">
                  <input type="checkbox" checked={skipDefinitionWarning} onChange={toggle('pz_skip_definition_warning', setSkipDefinitionWarning)} className="rounded border-slate-700 accent-sky-500 bg-slate-950 cursor-pointer" />
                  {i18next.t('app.launch.dontShowDefinitions')}
                </label>
              </div>
            )}

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-800">
              {soft ? (
                <button
                  onClick={() => {
                    setPendingLaunchMode(null);
                    setHealthFocus(definitions[0] ?? null);
                    setActiveTab('HEALTH');
                  }}
                  className="px-4 py-2 bg-sky-700 hover:bg-sky-600 text-white text-xs font-bold rounded-lg transition cursor-pointer shadow"
                >
                  {i18next.t('app.launch.goToHealth')}
                </button>
              ) : (
                <button
                  onClick={() => {
                    setPendingLaunchMode(null);
                    setActiveTab('MERGER');
                  }}
                  className="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white text-xs font-bold rounded-lg transition cursor-pointer shadow"
                >
                  {i18next.t('app.launch.goToMerger')}
                </button>
              )}
              <button
                onClick={() => {
                  const mode = pendingLaunchMode;
                  setPendingLaunchMode(null);
                  executeLaunch(mode);
                }}
                className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-bold rounded-lg transition cursor-pointer"
              >
                {isOpenPackageWarning ? i18next.t('app.launch.withoutPackage') : i18next.t('app.launch.anyway')}
              </button>
            </div>
          </div>
        </div>
        );
      })()}
    </div>
    {pendingPlan && (
      <ChangePreview
        title={pendingPlan.enable
          ? i18next.t('preview.onTitle', { name: pendingPlan.target.name })
          : i18next.t('preview.offTitle', { name: pendingPlan.target.name })}
        blurb={pendingPlan.enable ? i18next.t('preview.onBlurb') : i18next.t('preview.offBlurb')}
        sections={[
          { label: i18next.t('preview.turnOn'), tone: 'on', rows: pendingPlan.turnOn.map((s) => ({ name: s.name, note: i18next.t('preview.neededBy', { name: s.because }) })) },
          { label: i18next.t('preview.turnOff'), tone: 'off', rows: pendingPlan.turnOff.map((s) => ({ name: s.name, note: i18next.t('preview.needs', { name: s.because }) })) },
          { label: i18next.t('preview.moved'), tone: 'move', rows: pendingPlan.moved.map((s) => ({ name: s.name, note: i18next.t('preview.movedBefore', { name: s.because }) })) },
          { label: i18next.t('preview.missing'), tone: 'warn', rows: pendingPlan.missing.map((m) => ({ name: m.id, note: i18next.t('preview.missingNote', { name: m.because }) })) },
          { label: i18next.t('preview.blocked'), tone: 'warn', rows: pendingPlan.blocked.map((s) => ({ name: s.name, note: i18next.t('preview.blockedNote', { name: s.because }) })) },
        ]}
        primary={{
          label: pendingPlan.enable
            ? (pendingPlan.turnOn.length + pendingPlan.moved.length > 0
              ? i18next.t('preview.onAll', { count: pendingPlan.turnOn.length + 1 })
              : i18next.t('preview.onAnyway'))
            : i18next.t('preview.offAll', { count: pendingPlan.turnOff.length + 1 }),
          onClick: () => applyTogglePlan(pendingPlan, true),
        }}
        secondary={(pendingPlan.enable ? pendingPlan.turnOn.length + pendingPlan.moved.length : pendingPlan.turnOff.length) > 0
          ? { label: i18next.t('preview.onlyThis', { name: pendingPlan.target.name }), onClick: () => applyTogglePlan(pendingPlan, false) }
          : undefined}
        onClose={() => setPendingPlan(null)}
      />
    )}
    <BackendErrorToasts />
    </ContextMenuProvider>
  );
};

export default App;
