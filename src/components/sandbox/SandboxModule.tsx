import React, { useState, useEffect, useMemo, useRef } from 'react';
import { TranslatedErrorCard, LogFileInfoUI, ModInfo } from '../../types';
import { DiagnosticsPanel } from './DiagnosticsPanel';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, explainError } from '../../services/tauri';
import { clearLiveConsole, liveConsoleLines, startLiveConsole, subscribeLiveConsole } from '../../services/liveConsole';
import { useTranslation } from 'react-i18next';
import i18next from 'i18next';

const liveConsoleHeader = () => [i18next.t('monitor.console.header1'), i18next.t('monitor.console.header2')];
const serverConsoleHeader = () => [
  i18next.t('monitor.console.serverHeader1'),
  i18next.t('monitor.console.serverHeader2'),
];
import {
  FolderX,
  Activity,
  Filter,
  Trash2,
  FolderOpen,
  Plus,
  X,
  Search,
  Copy,
  Check,
  FileText,
  ChevronDown,
  Save,
} from 'lucide-react';

interface SandboxModuleProps {
  paths: StudioPathsUI;
  /** The installed mods, for the hunt: its suspects and their requirements. */
  mods: ModInfo[];
  /** Re-reads the Mod List after the hunt has written a trial to disk. */
  onRefreshMods: () => void;
  errorCards: TranslatedErrorCard[];
  /** The Build 42 shim travels inside a merge package, so repairs start in the Merger. */
  onOpenMerger: () => void;
  /** Switches a mod off by name, for when the game blames it for an error. */
  onSwitchOffMod?: (modName: string) => void;
  /**
   * Opens the Merger on this mod's files to patch one; with a file name from the log, that file.
   * The mod stays on: the package loads last and the game reads the patched copy.
   */
  onPatchMod?: (modId: string, fileHint?: string | null, fix?: { from: string; to: string } | null) => void;
  onClearErrorCards?: () => void;
  /** Removes one card, once what it reported has been fixed. */
  onDismissErrorCard?: (cardId: string) => void;
  onGoToSettings: () => void;
}

interface LogTab {
  id: string;
  title: string;
  isLive: boolean;
  filePath?: string;
  lines: string[];
}

type LogSeverityFilter = 'ALL' | 'ERRORS' | 'LUA' | 'JAVA' | 'BRIDGE';

const isTranslationSpamLine = (line: string): boolean => {
  const lower = line.toLowerCase();
  return (
    lower.includes('translator') ||
    lower.includes('translation') ||
    lower.includes('missing') ||
    lower.includes('language') ||
    lower.includes('igui_') ||
    lower.includes('itemname_') ||
    lower.includes('contextmenu_') ||
    lower.includes('sandbox_') ||
    lower.includes('tooltip_') ||
    lower.includes('recipe_') ||
    lower.includes('ui_modexporter') ||
    lower.includes('text_')
  );
};

export const formatLogDisplayName = (file: LogFileInfoUI): string => {
  if (file.is_active_console || file.file_name.toLowerCase().startsWith('console.txt')) {
    if (file.modified_timestamp) {
      const d = new Date(file.modified_timestamp * 1000);
      const pad = (n: number) => n.toString().padStart(2, '0');
      const dateStr = `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
      return `console.txt (${dateStr})`;
    }
    return i18next.t('monitor.console.onDisk');
  }
  return file.file_name;
};

/** The order the player dragged the live console tabs into. */
const TAB_ORDER_KEY = 'pz_monitor_tab_order';

export const SandboxModule: React.FC<SandboxModuleProps> = ({
  paths,
  mods,
  onRefreshMods,
  errorCards,
  onOpenMerger,
  onSwitchOffMod,
  onPatchMod,
  onClearErrorCards,
  onDismissErrorCard,
  onGoToSettings,
}) => {
  const { t } = useTranslation();
  // The companion bridge is a developer feature; its log filter follows the same switch.
  const [showAdvanced, setShowAdvanced] = useState<boolean>(() => localStorage.getItem('pz_show_advanced') === 'true');
  useEffect(() => {
    const onChange = (e: Event) => setShowAdvanced(Boolean((e as CustomEvent).detail));
    window.addEventListener('pzms-advanced-changed', onChange);
    return () => window.removeEventListener('pzms-advanced-changed', onChange);
  }, []);

  const [filterSpam, setFilterSpam] = useState<boolean>(true);
  // ZombieBuddy's own lines ([ZB] patching..., Transformed..., Found patch...): a third of a
  // session's log with many Java mods. Hidden unless asked for; the choice is remembered.
  const [hideZb, setHideZb] = useState<boolean>(() => localStorage.getItem('pz_monitor_hide_zb') !== 'false');
  const toggleHideZb = () => setHideZb((v) => {
    localStorage.setItem('pz_monitor_hide_zb', String(!v));
    return !v;
  });
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [severityFilter, setSeverityFilter] = useState<LogSeverityFilter>('ALL');
  const [copied, setCopied] = useState<boolean>(false);
  const [isAddMenuOpen, setIsAddMenuOpen] = useState<boolean>(false);
  const [availableLogs, setAvailableLogs] = useState<LogFileInfoUI[]>([]);
  const addMenuRef = useRef<HTMLDivElement>(null);
  const terminalContainerRef = useRef<HTMLDivElement>(null);

  // Tabs state with permanent Client and Dedicated Server tabs
  const [tabs, setTabs] = useState<LogTab[]>([
    {
      id: 'live_console',
      title: t('monitor.console.clientTab'),
      isLive: true,
      // What the app has streamed so far: leaving this tab and coming back loses nothing.
      lines: [...liveConsoleHeader(), ...liveConsoleLines()],
    },
    {
      id: 'live_server',
      title: t('monitor.console.serverTab'),
      isLive: true,
      lines: serverConsoleHeader(),
    },
    {
      // The second game of a multiplayer test on this PC (its own Zomboid2 folder).
      id: 'live_player2',
      title: t('monitor.console.player2Tab'),
      isLive: true,
      lines: [t('monitor.console.player2Empty')],
    },
  ]);
  useEffect(() => {
    try {
      const order = JSON.parse(localStorage.getItem(TAB_ORDER_KEY) ?? '[]') as string[];
      if (order.length > 0) {
        setTabs((prev) => [...prev].sort((a, b) => (order.indexOf(a.id) + 1 || 99) - (order.indexOf(b.id) + 1 || 99)));
      }
    } catch {
      // the default order
    }
  }, []);
  const [activeTabId, setActiveTabId] = useState<string>('live_console');
  // Which of the three live consoles has something running behind it: a live tab shows only
  // then. With nothing running the bar holds just the logs opened by hand.
  const [running, setRunning] = useState<Record<string, boolean>>({ live_console: false, live_server: false, live_player2: false });
  const pollCount = useRef(0);
  const [saveStatus, setSaveStatus] = useState<string | null>(null);
  const [clearedAnchors, setClearedAnchors] = useState<Record<string, string>>({});

  // Close dropdown on outside click
  useEffect(() => {
    const handleOutside = (e: MouseEvent) => {
      if (addMenuRef.current && !addMenuRef.current.contains(e.target as Node)) {
        setIsAddMenuOpen(false);
      }
    };
    if (isAddMenuOpen) document.addEventListener('mousedown', handleOutside);
    return () => document.removeEventListener('mousedown', handleOutside);
  }, [isAddMenuOpen]);

  // Dedicated Server live log polling (only streams when server is running)
  useEffect(() => {
    if (!paths.user_zomboid_dir) return;

    let isMounted = true;
    const fetchServerLogs = async () => {
      try {
        const srvStatus = await TauriService.getDedicatedServerStatus(paths.user_zomboid_dir);
        if (!isMounted) return;

        // The tab says which server and whether it runs with Steam or without.
        const serverTitle = srvStatus.is_running && srvStatus.server_name
          ? t('monitor.console.serverTabNamed', { name: srvStatus.server_name, mode: srvStatus.nosteam ? 'NoSteam' : 'Steam' })
          : t('monitor.console.serverTab');
        setTabs((prev) => prev.map((tab) => (tab.id === 'live_server' && tab.title !== serverTitle ? { ...tab, title: serverTitle } : tab)));
        // The games on this PC, asked every third round (it lists the processes).
        let gameOne: boolean | null = null;
        if (pollCount.current++ % 3 === 0) {
          const mp = await TauriService.mpTestStatus(paths).catch(() => null);
          if (!isMounted) return;
          if (mp) {
            const players = mp.processes.filter((p) => p.role === 'player');
            gameOne = players.some((p) => p.name !== '2');
            const next = { live_console: gameOne, live_server: srvStatus.is_running || mp.processes.some((p) => p.role === 'server'), live_player2: players.some((p) => p.name === '2') };
            setRunning((prev) => (prev.live_console === next.live_console && prev.live_server === next.live_server && prev.live_player2 === next.live_player2 ? prev : next));
          }
        }
        // Game 1's console.txt while the live stream has nothing (the app did not start that game).
        const headerOnly = liveConsoleLines().length === 0 && gameOne !== false;
        if (headerOnly) {
          const p1 = await TauriService.mpTestPlayer2Log(paths, 1500, 1).catch(() => [] as string[]);
          if (p1.length > 0 && isMounted) {
            const cleanedP1 = p1.map((l) => l.replace(/^\[\d{2}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2}\.\d{3}\]\s*/, ''));
            setTabs((prev) => prev.map((tab) => (tab.id === 'live_console' ? { ...tab, lines: [...liveConsoleHeader(), t('monitor.console.fromFile'), ...cleanedP1] } : tab)));
          }
        }
        // Player 2's console, when the multiplayer test has made its folder.
        const p2 = await TauriService.mpTestPlayer2Log(paths, 1500).catch(() => [] as string[]);
        if (p2.length > 0 && isMounted) {
          const cleanedP2 = p2.map((l) => l.replace(/^\[\d{2}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2}\.\d{3}\]\s*/, ''));
          setTabs((prev) => prev.map((tab) => (tab.id === 'live_player2' ? { ...tab, lines: cleanedP2 } : tab)));
        }
        if (srvStatus.is_running) {
          const srvLines = await TauriService.getDedicatedServerLogs(paths.user_zomboid_dir, 1500);
          if (srvLines && srvLines.length > 0 && isMounted) {
            const cleaned = srvLines.map((l) =>
              l.replace(/^\[\d{2}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2}\.\d{3}\]\s*/, '')
            );
            setTabs((prev) =>
              prev.map((t) => (t.id === 'live_server' ? { ...t, lines: cleaned } : t))
            );
          }
        } else {
          setTabs((prev) =>
            prev.map((t) =>
              t.id === 'live_server' && t.lines.length > 2
                ? {
                    ...t,
                    lines: serverConsoleHeader(),
                  }
                : t
            )
          );
        }
      } catch (err) {
        console.warn('Dedicated server log poll error:', err);
      }
    };

    fetchServerLogs();
    const interval = setInterval(fetchServerLogs, 2000);
    return () => {
      isMounted = false;
      clearInterval(interval);
    };
  }, [paths.user_zomboid_dir]);

  // The live tab mirrors the app-wide console store, which keeps streaming while this tab is away.
  useEffect(() => {
    startLiveConsole();
    return subscribeLiveConsole(() => {
      setTabs((prev) =>
        prev.map((t) =>
          t.id === 'live_console' ? { ...t, lines: [...liveConsoleHeader(), ...liveConsoleLines()] } : t
        )
      );
    });
  }, []);

  // Fetch available log files when opening dropdown
  const handleOpenAddMenu = async () => {
    setIsAddMenuOpen((prev) => !prev);
    if (!isAddMenuOpen && paths.user_zomboid_dir) {
      const files = await TauriService.listAvailableLogFiles(paths.user_zomboid_dir);
      setAvailableLogs(files);
    }
  };

  const handleOpenLogFile = async (logFile: LogFileInfoUI) => {
    setIsAddMenuOpen(false);
    // Check if already open
    const existing = tabs.find((t) => t.filePath === logFile.absolute_path);
    if (existing) {
      setActiveTabId(existing.id);
      return;
    }

    const displayName = formatLogDisplayName(logFile);
    const lines = await TauriService.readLogFile(logFile.absolute_path, 2000);
    const newTab: LogTab = {
      id: `log_${Date.now()}`,
      title: displayName,
      isLive: false,
      filePath: logFile.absolute_path,
      lines: lines.length > 0 ? lines : [t('monitor.console.emptyLog')],
    };

    setTabs((prev) => [...prev, newTab]);
    setActiveTabId(newTab.id);
  };

  const handlePickCustomLog = async () => {
    setIsAddMenuOpen(false);
    const filePath = await TauriService.pickOpenFile(t('monitor.logs.logFileFilter'), 'txt');
    if (!filePath) return;

    const fileName = filePath.split(/[/\\]/).pop() || t('monitor.logs.externalLog');
    const lines = await TauriService.readLogFile(filePath, 2000);
    const newTab: LogTab = {
      id: `custom_${Date.now()}`,
      title: fileName,
      isLive: false,
      filePath,
      lines: lines.length > 0 ? lines : [t('monitor.console.emptyLog')],
    };

    setTabs((prev) => [...prev, newTab]);
    setActiveTabId(newTab.id);
  };

  const isPermanentTab = (tabId: string) => tabId === 'live_console' || tabId === 'live_server' || tabId === 'live_player2';

  /** Tabs are reordered by dragging one onto another; the order of the three live ones is kept. */
  const [draggedTab, setDraggedTab] = useState<string | null>(null);
  const moveTab = (from: string, to: string) => {
    if (from === to) return;
    setTabs((prev) => {
      const list = [...prev];
      const i = list.findIndex((x) => x.id === from);
      const j = list.findIndex((x) => x.id === to);
      if (i < 0 || j < 0) return prev;
      const [moved] = list.splice(i, 1);
      list.splice(j, 0, moved);
      try {
        localStorage.setItem(TAB_ORDER_KEY, JSON.stringify(list.filter((x) => isPermanentTab(x.id)).map((x) => x.id)));
      } catch {
        // only a convenience
      }
      return list;
    });
  };

  const handleCloseTab = (tabId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (isPermanentTab(tabId)) return;
    const remaining = tabs.filter((t) => t.id !== tabId);
    setTabs(remaining);
    if (activeTabId === tabId) {
      setActiveTabId(remaining[remaining.length - 1].id);
    }
  };

  const handleSaveSnapshot = async () => {
    if (!paths.user_zomboid_dir) return;
    try {
      const serverName = activeTab.id === 'live_server' ? 'servertest' : 'console_snapshot';
      const savedPath = await TauriService.saveServerLogSnapshot(
        paths.user_zomboid_dir,
        serverName,
        activeTab.lines
      );
      const filename = savedPath.split(/[/\\]/).pop() || savedPath;
      setSaveStatus(filename);
      setTimeout(() => setSaveStatus(null), 3500);
      const files = await TauriService.listAvailableLogFiles(paths.user_zomboid_dir);
      setAvailableLogs(files);
    } catch (err: any) {
      alert(t('monitor.logs.saveSnapshotError', { error: explainError(err) }));
    }
  };

  const shownTabs = useMemo(() => tabs.filter((tab) => !isPermanentTab(tab.id) || running[tab.id]), [tabs, running]);
  /** Nothing running and no log opened: one line saying so, in place of a console. */
  const idleTab = useMemo<LogTab>(() => ({ id: 'idle', title: '', isLive: false, lines: [t('monitor.console.nothingRunning')] }), [t]);
  const activeTab = shownTabs.find((tab) => tab.id === activeTabId) || shownTabs[0] || idleTab;
  /** Opens console.txt, the log of the last game session, as a tab of its own. */
  const openLastSession = async () => {
    if (!paths.user_zomboid_dir) return;
    const files = await TauriService.listAvailableLogFiles(paths.user_zomboid_dir);
    const last = files.find((l) => l.is_active_console) ?? files[0];
    if (last) await handleOpenLogFile(last);
  };

  // Filter lines dynamically
  const filteredLines = useMemo(() => {
    if (!activeTab) return [];
    const raw = activeTab.lines;
    // Cleared tabs show what came after the last line on screen at the time. A count would not
    // do: the live tabs keep a rolling window of 1500 lines, so once full their length never
    // grows past the count and the tab stayed empty for good. An anchor that has scrolled out
    // of the window means everything on screen is new.
    const anchor = clearedAnchors[activeTab.id];
    const at = anchor === undefined ? -1 : raw.lastIndexOf(anchor);
    let list = at >= 0 ? raw.slice(at + 1) : raw;

    // 1. Spam filter
    if (filterSpam) {
      list = list.filter((line) => !isTranslationSpamLine(line));
    }

    // ZombieBuddy's routine lines; one that reports trouble stays.
    if (hideZb) {
      list = list.filter((line) => !line.includes('[ZB]') || /^(ERROR|WARN)|Exception|Error:/.test(line));
    }

    // 2. Severity filter
    if (severityFilter === 'ERRORS') {
      list = list.filter((l) =>
        /exception|error|stack trace|crash|nullpointer|failed|kahlua/i.test(l)
      );
    } else if (severityFilter === 'LUA') {
      list = list.filter((l) => /lua|function|require|script/i.test(l));
    } else if (severityFilter === 'JAVA') {
      list = list.filter((l) => /java|\.java|jvm|kahlua|zombie\./i.test(l));
    } else if (severityFilter === 'BRIDGE') {
      list = list.filter((l) => /pzmodstudio|bridge|companion/i.test(l));
    }

    // 3. Search query
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter((l) => l.toLowerCase().includes(q));
    }

    return list;
  }, [activeTab, filterSpam, hideZb, severityFilter, searchQuery, clearedAnchors]);

  const handleClearActiveTab = () => {
    if (!activeTab) return;
    // The game's console is a shared store: empty it, so the header and new lines show. When the
    // tab shows the game's console.txt instead (a game the app did not start), that is read again
    // every two seconds: the anchor below keeps what was cleared off the screen.
    if (activeTab.id === 'live_console') {
      clearLiveConsole();
    }
    if (activeTab.lines.length === 0) return;
    setClearedAnchors((prev) => ({
      ...prev,
      [activeTab.id]: activeTab.lines[activeTab.lines.length - 1],
    }));
  };

  const handleCopyLogs = () => {
    const text = filteredLines.join('\n');
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Auto-scroll terminal to bottom when new lines arrive in live mode
  useEffect(() => {
    if (activeTab?.isLive && terminalContainerRef.current) {
      terminalContainerRef.current.scrollTop = terminalContainerRef.current.scrollHeight;
    }
  }, [filteredLines.length, activeTabId]);

  if (!paths.is_valid) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-6 bg-slate-950 text-slate-200 font-sans">
        <div className="max-w-md w-full bg-slate-900/80 border border-amber-500/40 rounded-2xl p-6 text-center space-y-4 shadow-xl">
          <div className="w-12 h-12 rounded-xl bg-amber-500/10 text-amber-400 border border-amber-500/30 flex items-center justify-center mx-auto">
            <FolderX className="w-6 h-6" />
          </div>
          <div>
            <h3 className="text-base font-bold text-slate-100">{t('monitor.launch.setupTitle')}</h3>
            <p className="text-xs text-slate-400 mt-1 leading-relaxed">{t('monitor.launch.setupBody')}</p>
          </div>
          <button
            onClick={onGoToSettings}
            className="w-full flex items-center justify-center gap-2 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white font-medium text-xs py-2.5 rounded-lg shadow transition cursor-pointer"
          >
            {t('monitor.launch.setupButton')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col bg-slate-950 text-slate-100 overflow-hidden p-6 font-sans min-h-0">
      {/* Module Header & Controls */}
      <div className="flex items-center justify-between border-b border-slate-800 pb-4 mb-3 shrink-0">
        <div>
          <h2 className="text-lg font-bold text-slate-100 flex items-center gap-2">
            <Activity className="w-5 h-5 text-emerald-400" />
            <span>{t('monitor.console.title')}</span>
          </h2>
          <p className="text-xs text-slate-400 mt-0.5">{t('monitor.console.subtitle')}</p>
        </div>

        <div className="flex items-center gap-2.5">
          {/* Open Real Logs Folder */}
          <button
            onClick={() => TauriService.openLogsFolder(paths.user_zomboid_dir)}
            className="px-3 py-1.5 text-xs font-medium bg-slate-900 hover:bg-slate-800 text-slate-200 border border-slate-700 rounded-lg transition cursor-pointer flex items-center gap-1.5 shadow"
            title={t('monitor.console.logsFolderTitle')}
          >
            <FolderOpen className="w-3.5 h-3.5 text-amber-400" />
            <span>{t('monitor.console.logsFolder')}</span>
          </button>

          {/* Spam Filter Toggle */}
          <button
            onClick={() => setFilterSpam(!filterSpam)}
            className={`px-3 py-1.5 text-xs font-bold rounded-lg border transition cursor-pointer flex items-center gap-1.5 shadow ${
              filterSpam
                ? 'bg-emerald-950/80 text-emerald-300 border-emerald-700 shadow-emerald-950/40'
                : 'bg-slate-900 text-slate-400 border-slate-700 hover:text-slate-200'
            }`}
            title={t('monitor.console.spamFilterTitle')}
          >
            <Filter className={`w-3.5 h-3.5 ${filterSpam ? 'text-emerald-400' : 'text-slate-500'}`} />
            <span>{filterSpam ? t('monitor.console.spamActive') : t('monitor.console.spamShowAll')}</span>
          </button>
          <label
            className="px-3 py-1.5 text-xs font-bold rounded-lg border border-slate-700 bg-slate-900 text-slate-300 flex items-center gap-1.5 cursor-pointer select-none"
            title={t('monitor.console.hideZbTitle')}
          >
            <input type="checkbox" checked={hideZb} onChange={toggleHideZb} className="accent-emerald-500" />
            <span>{t('monitor.console.hideZb')}</span>
          </label>
        </div>
      </div>

      {/* Main Grid: Console Terminal Watcher (Left) vs Actionable Cards (Right) */}
      <div className="flex-1 grid grid-cols-1 lg:grid-cols-12 gap-5 min-h-0 overflow-hidden">
        {/* Left Column (7 cols): Realtime & Historical Multi-Tab Log Inspector */}
        <div className="lg:col-span-7 bg-slate-900/80 border border-slate-800 rounded-xl flex flex-col shadow relative min-h-0 overflow-hidden">
          {/* Top Tabs Bar */}
          <div className="bg-slate-950 border-b border-slate-800 flex items-center justify-between px-2 pt-1.5 shrink-0 gap-2 relative z-30">
            {/* Tabs List (Only the tabs scroll horizontally if there are many) */}
            <div className="flex items-center gap-1 overflow-x-auto no-scrollbar flex-1 min-w-0">
              {!shownTabs.some((tab) => tab.isLive) && (
                <button
                  type="button"
                  onClick={() => void openLastSession()}
                  title={t('monitor.console.openLastSessionTitle')}
                  className="px-3 py-1.5 mb-1 rounded-lg text-xs font-bold text-sky-200 border border-sky-800/70 hover:bg-sky-900/40 cursor-pointer shrink-0"
                >
                  {t('monitor.console.openLastSession')}
                </button>
              )}
              {shownTabs.map((tab) => {
                const isActive = tab.id === activeTab.id;
                return (
                  <button
                    key={tab.id}
                    draggable
                    onDragStart={(e) => {
                      setDraggedTab(tab.id);
                      e.dataTransfer.effectAllowed = 'move';
                    }}
                    onDragOver={(e) => {
                      if (draggedTab) e.preventDefault();
                    }}
                    onDrop={(e) => {
                      e.preventDefault();
                      if (draggedTab) moveTab(draggedTab, tab.id);
                      setDraggedTab(null);
                    }}
                    onDragEnd={() => setDraggedTab(null)}
                    onClick={() => setActiveTabId(tab.id)}
                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-t-lg text-xs font-medium border-t border-x transition cursor-pointer shrink-0 max-w-[200px] truncate ${
                      isActive
                        ? 'bg-slate-900 text-emerald-400 border-slate-700 font-bold shadow'
                        : 'bg-slate-950 text-slate-400 hover:text-slate-200 border-transparent hover:bg-slate-900/50'
                    }`}
                  >
                    <span className="truncate">{tab.title}</span>
                    {!isPermanentTab(tab.id) && (
                      <span
                        onClick={(e) => handleCloseTab(tab.id, e)}
                        className="text-slate-500 hover:text-red-400 rounded-full p-0.5"
                        title={t('monitor.console.closeTab')}
                      >
                        <X className="w-3 h-3" />
                      </span>
                    )}
                  </button>
                );
              })}
            </div>

            {/* Fixed Right Actions (Add Log Button + Menu + Save Snapshot + Copy + Clear) */}
            <div className="flex items-center gap-1.5 pb-1 shrink-0 relative">
              {saveStatus && (
                <span className="text-[10px] font-mono text-emerald-400 bg-emerald-950/80 px-2 py-1 rounded border border-emerald-700/60 animate-fade-in truncate max-w-[130px]" title={t('monitor.logs.savedTitle', { name: saveStatus })}>
                  {t('monitor.logs.saved')}
                </span>
              )}

              {/* Add New Tab Dropdown Button */}
              <div className="relative" ref={addMenuRef}>
                <button
                  onClick={handleOpenAddMenu}
                  className="flex items-center gap-1 px-2.5 py-1 text-xs font-medium text-slate-300 hover:text-white bg-slate-900 hover:bg-slate-800 border border-slate-700 rounded-md transition cursor-pointer"
                  title={t('monitor.logs.loadLogTitle')}
                >
                  <Plus className="w-3.5 h-3.5 text-emerald-400" />
                  <span>{t('monitor.logs.loadLog')}</span>
                  <ChevronDown className="w-3 h-3 text-slate-400" />
                </button>

                {isAddMenuOpen && (
                  <div className="absolute right-0 top-full mt-1.5 w-72 bg-slate-900 border border-slate-700 rounded-xl shadow-2xl z-50 p-2 space-y-1 animate-fade-in text-xs select-none">
                    <div className="px-2 py-1 text-[10px] font-bold text-slate-400 uppercase tracking-wider font-mono">
                      {t('monitor.logs.availableLogs')}
                    </div>

                    <div className="max-h-48 overflow-y-auto space-y-0.5 pr-1">
                      {availableLogs.length === 0 ? (
                        <div className="p-2 text-slate-500 text-center">{t('monitor.logs.searchingLogs')}</div>
                      ) : (
                        availableLogs.map((item) => (
                          <button
                            key={item.absolute_path}
                            onClick={() => handleOpenLogFile(item)}
                            className="w-full flex items-center justify-between p-2 rounded-lg hover:bg-slate-800 text-slate-200 text-left transition cursor-pointer group"
                          >
                            <div className="flex items-center gap-2 truncate">
                              <FileText className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
                              <span className="truncate">{formatLogDisplayName(item)}</span>
                            </div>
                            <span className="text-[10px] text-slate-500 font-mono shrink-0 ml-2">
                              {(item.size_bytes / 1024).toFixed(0)} KB
                            </span>
                          </button>
                        ))
                      )}
                    </div>

                    <div className="border-t border-slate-800 pt-1">
                      <button
                        onClick={handlePickCustomLog}
                        className="w-full flex items-center gap-2 p-2 rounded-lg hover:bg-emerald-950/60 text-emerald-300 font-medium transition cursor-pointer text-left"
                      >
                        <FolderOpen className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                        <span>{t('monitor.logs.browseFile')}</span>
                      </button>
                    </div>
                  </div>
                )}
              </div>

              {/* Save Snapshot to Zomboid/Logs */}
              <button
                onClick={handleSaveSnapshot}
                className="p-1.5 text-slate-400 hover:text-emerald-400 bg-slate-900 hover:bg-slate-800 border border-slate-800 rounded transition cursor-pointer"
                title={t('monitor.logs.saveSnapshotTitle')}
              >
                <Save className="w-3.5 h-3.5 text-emerald-400" />
              </button>

              {/* Copy logs */}
              <button
                onClick={handleCopyLogs}
                className="p-1.5 text-slate-400 hover:text-slate-200 bg-slate-900 hover:bg-slate-800 border border-slate-800 rounded transition cursor-pointer"
                title={t('monitor.console.copyTitle')}
              >
                {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
              </button>

              {/* Clear logs */}
              <button
                onClick={handleClearActiveTab}
                className="p-1.5 text-slate-400 hover:text-red-400 bg-slate-900 hover:bg-slate-800 border border-slate-800 rounded transition cursor-pointer"
                title={t('monitor.console.clearTitle')}
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          {/* Search & Severity Filters Bar */}
          <div className="bg-slate-900 px-3 py-2 border-b border-slate-800 flex flex-wrap items-center justify-between gap-2 text-xs shrink-0">
            {/* Search Input */}
            <div className="relative flex-1 min-w-[180px]">
              <Search className="w-3.5 h-3.5 text-slate-500 absolute left-2.5 top-1/2 -translate-y-1/2" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={t('monitor.console.searchPlaceholder')}
                className="w-full bg-slate-950 border border-slate-700 rounded-lg pl-8 pr-7 py-1 text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-emerald-500 transition"
              />
              {searchQuery && (
                <button
                  onClick={() => setSearchQuery('')}
                  className="absolute right-2 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300 text-xs"
                >
                  ✕
                </button>
              )}
            </div>

            {/* Severity Filter Chips */}
            <div className="flex items-center gap-1 font-mono text-[11px]">
              <button
                onClick={() => setSeverityFilter('ALL')}
                className={`px-2 py-0.5 rounded transition cursor-pointer ${
                  severityFilter === 'ALL'
                    ? 'bg-slate-700 text-slate-100 font-bold'
                    : 'text-slate-400 hover:text-slate-200'
                }`}
              >
                {t('monitor.console.filterAll')}
              </button>
              <button
                onClick={() => setSeverityFilter('ERRORS')}
                className={`px-2 py-0.5 rounded transition cursor-pointer ${
                  severityFilter === 'ERRORS'
                    ? 'bg-red-950 text-red-300 border border-red-800 font-bold'
                    : 'text-slate-400 hover:text-red-400'
                }`}
              >
                {t('monitor.console.filterErrors')}
              </button>
              <button
                onClick={() => setSeverityFilter('LUA')}
                className={`px-2 py-0.5 rounded transition cursor-pointer ${
                  severityFilter === 'LUA'
                    ? 'bg-blue-950 text-blue-300 border border-blue-800 font-bold'
                    : 'text-slate-400 hover:text-blue-400'
                }`}
              >
                {t('monitor.console.filterLua')}
              </button>
              <button
                onClick={() => setSeverityFilter('JAVA')}
                className={`px-2 py-0.5 rounded transition cursor-pointer ${
                  severityFilter === 'JAVA'
                    ? 'bg-amber-950 text-amber-300 border border-amber-800 font-bold'
                    : 'text-slate-400 hover:text-amber-400'
                }`}
              >
                {t('monitor.console.filterJava')}
              </button>
              {showAdvanced && (
              <button
                onClick={() => setSeverityFilter('BRIDGE')}
                className={`px-2 py-0.5 rounded transition cursor-pointer ${
                  severityFilter === 'BRIDGE'
                    ? 'bg-purple-950 text-purple-300 border border-purple-800 font-bold'
                    : 'text-slate-400 hover:text-purple-400'
                }`}
              >
                {t('monitor.console.filterBridge')}
              </button>
              )}
            </div>
          </div>

          {/* Log Lines Terminal Area */}
          <div ref={terminalContainerRef} className="flex-1 p-3 font-mono text-xs bg-slate-950 overflow-y-auto space-y-1 select-text min-h-0">
            {filteredLines.length === 0 ? (
              <div className="p-6 text-center text-slate-500 space-y-1">
                <Search className="w-6 h-6 mx-auto text-slate-600 mb-1" />
                <p>{t('monitor.console.noLines')}</p>
                {searchQuery && (
                  <p className="text-[11px] text-slate-600">
                    {t('monitor.console.tryChangingSearch', { query: searchQuery })}
                  </p>
                )}
              </div>
            ) : (
              filteredLines.map((log, idx) => {
                const isErr = /exception|error|stack trace|crash|nullpointer|failed/i.test(log);
                const isWarn = /warning|warn/i.test(log);
                const isBridge = /pzmodstudio|bridge/i.test(log);

                return (
                  <div
                    key={idx}
                    className={`leading-relaxed px-2 py-0.5 rounded text-[11px] ${
                      isErr
                        ? 'text-red-400 font-bold bg-red-950/30 border-l-2 border-red-500'
                        : isWarn
                        ? 'text-amber-300 bg-amber-950/20 border-l-2 border-amber-500'
                        : isBridge
                        ? 'text-purple-300 bg-purple-950/20 border-l-2 border-purple-500'
                        : 'text-slate-300 hover:bg-slate-900/60'
                    }`}
                  >
                    {log}
                  </div>
                );
              })
            )}
          </div>

          {/* Terminal Footer Bar */}
          <div className="bg-slate-900 px-3 py-1.5 border-t border-slate-800 flex items-center justify-between text-[11px] text-slate-400 font-mono shrink-0">
            <span>{t('monitor.console.linesCount', { shown: filteredLines.length, total: activeTab.lines.length })}</span>
            {searchQuery && (
              <span className="text-emerald-400">{t('monitor.console.activeFilter', { query: searchQuery })}</span>
            )}
            {activeTab.filePath && (
              <span className="truncate max-w-xs text-slate-500" title={activeTab.filePath}>
                {activeTab.filePath}
              </span>
            )}
          </div>
        </div>

        {/* Right Column (5 cols): what the game threw and warned about, with what can be done. */}
        <DiagnosticsPanel
          paths={paths}
          mods={mods}
          errorCards={errorCards}
          logLines={activeTab?.lines ?? []}
          reportLines={(shownTabs.find((tab) => tab.isLive) ?? activeTab).lines}
          onRefreshMods={onRefreshMods}
          onOpenMerger={onOpenMerger}
          onSwitchOffMod={onSwitchOffMod}
          onPatchMod={onPatchMod}
          onClearErrorCards={onClearErrorCards}
          onDismissErrorCard={onDismissErrorCard}
          onInspect={(term) => {
            setSearchQuery(term);
            setSeverityFilter('ALL');
          }}
          onStatus={(message) => {
            setSaveStatus(message);
            setTimeout(() => setSaveStatus(null), 4000);
          }}
        />
      </div>
    </div>
  );
};
