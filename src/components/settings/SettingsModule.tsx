import React, { useEffect, useState } from 'react';
import { DiagnosticsCard } from './DiagnosticsCard';
import { FolderOpen, CheckCircle, AlertTriangle, RefreshCw, HardDrive, Wrench, Bell, Bot, Copy, Check, Terminal, Radio, Code2, Layers, Info, Lightbulb, ExternalLink, Loader2, Languages } from 'lucide-react';
import { AppUpdateStatus, PolyfillSection, ShimPatch } from '../../types';
import { TauriService } from '../../services/tauri';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import { LANGUAGES, setLanguage } from '../../i18n';
import { tHtml } from '../../i18n/html';
import { UpdateNowButton } from '../layout/UpdateNotice';

export interface StudioPathsUI {
  pz_install_dir: string;
  workshop_dir: string;
  user_zomboid_dir: string;
  mod_list_ini_path: string;
  is_valid: boolean;
}

interface SettingsModuleProps {
  paths: StudioPathsUI;
  /** What the Build 42 shim covers; shown as a list, not as switches. */
  onSavePaths: (updatedPaths: StudioPathsUI) => void;
  onAutoDetect: () => void;
}

/** The setup guide of each MCP client: its key under settings.mcp.guides, how many steps, and the tip's colour. */
const MCP_GUIDES: Record<string, { key: string; steps: number; tip?: 'cyan' | 'emerald' }> = {
  CLAUDE: { key: 'claude', steps: 4, tip: 'cyan' },
  CURSOR: { key: 'cursor', steps: 4, tip: 'cyan' },
  VSCODE_LOCAL: { key: 'vscode', steps: 4, tip: 'emerald' },
  ANTIGRAVITY: { key: 'antigravity', steps: 3, tip: 'cyan' },
  OPENAI_CODEX: { key: 'openai', steps: 3 },
};

/** "12 KB" / "1.3 MB". */
const formatKb = (bytes: number): string => (bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`);

export const SettingsModule: React.FC<SettingsModuleProps> = ({
  paths,
  onSavePaths,
  onAutoDetect,
}) => {
  /** What the shim covers, as the shim heads its own parts. */
  const [shimSections, setShimSections] = useState<PolyfillSection[]>([]);
  const shimLines = shimSections.reduce((n, s) => n + s.lines, 0);
  /** The per-mod patches of part 7, each with its switch. */
  const [shimPatches, setShimPatches] = useState<ShimPatch[]>([]);
  const [patchNote, setPatchNote] = useState<string | null>(null);
  const changeRev = useChangeRevision();
  useEffect(() => {
    if (!paths.user_zomboid_dir) return;
    void (async () => setShimPatches(await TauriService.shimPatches(paths.user_zomboid_dir)))();
  }, [paths.user_zomboid_dir, changeRev]);
  const togglePatch = async (patch: ShimPatch) => {
    try {
      const written = await TauriService.setShimPatch(paths.user_zomboid_dir, patch.name, !patch.enabled);
      setShimPatches((list) => list.map((p) => (p.name === patch.name ? { ...p, enabled: !patch.enabled } : p)));
      setPatchNote(t('settings.patchWritten', { label: patch.label, state: patch.enabled ? t('settings.stateOff') : t('settings.stateOn'), count: written }));
      setTimeout(() => setPatchNote(null), 6000);
    } catch {
      // Already shown by TauriService.
    }
  };

  useEffect(() => {
    void (async () => setShimSections(await TauriService.polyfillSections()))();
  }, []);
  const [activeSubTab, setActiveSubTab] = useState<'PATHS' | 'POLYFILLS' | 'MCP'>('PATHS');
  /**
   * The MCP server, the Claude Agent and the in-game bridge are for people building things on
   * top of the Studio. A player has no use for them and they only raise questions, so they stay
   * out of sight until someone turns them on. Remembered on this PC.
   */
  const [showAdvanced, setShowAdvanced] = useState<boolean>(() => localStorage.getItem('pz_show_advanced') === 'true');

  const toggleAdvanced = (on: boolean) => {
    setShowAdvanced(on);
    localStorage.setItem('pz_show_advanced', String(on));
    window.dispatchEvent(new CustomEvent('pzms-advanced-changed', { detail: on }));
    if (!on && activeSubTab === 'MCP') setActiveSubTab('PATHS');
  };
  const [selectedMcpClient, setSelectedMcpClient] = useState<'ANTIGRAVITY' | 'CLAUDE' | 'CURSOR' | 'VSCODE_LOCAL' | 'OPENAI_CODEX'>('ANTIGRAVITY');
  const [copiedMcpConfig, setCopiedMcpConfig] = useState<boolean>(false);
  const [formData, setFormData] = useState<StudioPathsUI>(paths);

  React.useEffect(() => {
    setFormData(paths);
  }, [paths]);

  const [savedSuccess, setSavedSuccess] = useState<boolean>(false);
  const [noticeSilenced, setNoticeSilenced] = useState<boolean>(
    localStorage.getItem('pz_hide_autosort_notice') === 'true'
  );
  const [noMergeWarningSilenced, setNoMergeWarningSilenced] = useState<boolean>(
    localStorage.getItem('pz_skip_no_merge_warning') === 'true'
  );
  const [definitionWarningSilenced, setDefinitionWarningSilenced] = useState<boolean>(
    localStorage.getItem('pz_skip_definition_warning') === 'true'
  );
  const { t, i18n } = useTranslation();
  /** Whether the launch button starts on the mode used last time (on) or always on Debug (off). */
  const [rememberLaunchMode, setRememberLaunchMode] = useState<boolean>(
    localStorage.getItem('pz_remember_launch_mode') !== 'false'
  );
  const toggleRememberLaunchMode = (on: boolean) => {
    setRememberLaunchMode(on);
    localStorage.setItem('pz_remember_launch_mode', String(on));
  };
  // The check for a newer PZ Mod Studio: its switch and "Check now".
  const [appUpdate, setAppUpdate] = useState<AppUpdateStatus | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  useEffect(() => {
    void TauriService.appUpdateStatus(paths.user_zomboid_dir).then(setAppUpdate);
  }, [paths.user_zomboid_dir, changeRev]);
  const toggleAppUpdate = async (on: boolean) => {
    try {
      setAppUpdate(await TauriService.setAppUpdateCheck(paths.user_zomboid_dir, on));
    } catch {
      // Already shown by TauriService.
    }
  };
  const checkUpdateNow = async () => {
    setCheckingUpdate(true);
    setAppUpdate(await TauriService.appUpdateStatus(paths.user_zomboid_dir, true));
    setCheckingUpdate(false);
  };
  /** A DeepL API key, kept in Windows Credential Manager; translations use it before the free services. */
  const [deeplSaved, setDeeplSaved] = useState(false);
  const [deeplInput, setDeeplInput] = useState('');
  const [deeplBusy, setDeeplBusy] = useState(false);
  const [deeplError, setDeeplError] = useState<string | null>(null);
  useEffect(() => {
    void TauriService.deeplKeyStatus().then(setDeeplSaved);
  }, []);
  const saveDeeplKey = async () => {
    setDeeplBusy(true);
    setDeeplError(null);
    try {
      await TauriService.setDeeplKey(deeplInput);
      setDeeplSaved(true);
      setDeeplInput('');
    } catch (err) {
      setDeeplError(String(err));
    } finally {
      setDeeplBusy(false);
    }
  };
  /** The translations kept on disk, and a way to start over. */
  const [translationCache, setTranslationCache] = useState<{ count: number; bytes: number }>({ count: 0, bytes: 0 });
  useEffect(() => {
    void TauriService.translationCacheInfo().then(setTranslationCache);
  }, []);
  const clearTranslationCache = async () => {
    if (!confirm(t('settings.translate.clearConfirm', { count: translationCache.count }))) return;
    try {
      await TauriService.clearTranslationCache();
      setTranslationCache({ count: 0, bytes: 0 });
    } catch (err) {
      setDeeplError(String(err));
    }
  };
  const removeDeeplKey = async () => {
    if (!confirm(t('settings.translate.removeConfirm'))) return;
    try {
      await TauriService.deleteDeeplKey();
      setDeeplSaved(false);
    } catch (err) {
      setDeeplError(String(err));
    }
  };
  // Which Mods screen the game uses when another mod manager is on.
  const [screenChoice, setScreenChoice] = useState<string>('ask');
  useEffect(() => {
    if (!paths.user_zomboid_dir) return;
    void TauriService.linkScreenChoice(paths).then(([c]) => setScreenChoice(c));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths.user_zomboid_dir, changeRev]);

  // The window: opened as it was left, and what the close button does.
  const [rememberWindow, setRememberWindow] = useState(true);
  const [closeToTray, setCloseToTray] = useState(false);
  useEffect(() => {
    void TauriService.windowPrefs().then((p) => {
      if (!p) return;
      setRememberWindow(p.remember_window);
      setCloseToTray(p.close_to_tray);
    });
  }, []);
  const saveWindowPrefs = (remember_window: boolean, close_to_tray: boolean) => {
    setRememberWindow(remember_window);
    setCloseToTray(close_to_tray);
    void TauriService.setWindowPrefs({ remember_window, close_to_tray });
  };

  const [startupBehavior, setStartupBehavior] = useState<'SHOW_STARTUP_SCREEN' | 'OPEN_LAST_PROFILE'>(() => {
    return (localStorage.getItem('pz_startup_behavior') as any) || 'SHOW_STARTUP_SCREEN';
  });

  const handleSetStartupBehavior = (behavior: 'SHOW_STARTUP_SCREEN' | 'OPEN_LAST_PROFILE') => {
    setStartupBehavior(behavior);
    localStorage.setItem('pz_startup_behavior', behavior);
  };

  const handleRestoreNotice = () => {
    localStorage.removeItem('pz_hide_autosort_notice');
    setNoticeSilenced(false);
  };

  const handleRestoreNoMergeWarning = () => {
    localStorage.removeItem('pz_skip_no_merge_warning');
    setNoMergeWarningSilenced(false);
  };

  const handleSilenceNoMergeWarning = () => {
    localStorage.setItem('pz_skip_no_merge_warning', 'true');
    setNoMergeWarningSilenced(true);
  };

  const handleRestoreDefinitionWarning = () => {
    localStorage.removeItem('pz_skip_definition_warning');
    setDefinitionWarningSilenced(false);
  };

  const handleSilenceDefinitionWarning = () => {
    localStorage.setItem('pz_skip_definition_warning', 'true');
    setDefinitionWarningSilenced(true);
  };

  const handlePickFolder = async (field: keyof StudioPathsUI, currentVal: string) => {
    const selectedFolder = await TauriService.pickFolder(currentVal);
    if (selectedFolder) {
      const updated = {
        ...formData,
        [field]: selectedFolder,
      };
      setFormData(updated);
      onSavePaths(updated);
      setSavedSuccess(true);
      setTimeout(() => setSavedSuccess(false), 3000);
    }
  };

  /** The mod list is a file, so offer a file picker rather than a folder one. */
  const handlePickModListFile = async () => {
    const chosen = await TauriService.pickOpenFile(t('settings.modListFilter'), 'txt');
    if (!chosen) return;
    const updated = { ...formData, mod_list_ini_path: chosen };
    setFormData(updated);
    onSavePaths(updated);
    setSavedSuccess(true);
    setTimeout(() => setSavedSuccess(false), 3000);
  };

  /** The real path on this PC, so the snippet can be pasted as it is. */
  const [mcpBinaryPath, setMcpBinaryPath] = useState<string>('pz-mcp-server.exe');
  useEffect(() => {
    TauriService.mcpServerPath().then(setMcpBinaryPath).catch(() => undefined);
  }, []);

  const getMcpConfigSnippet = () => {
    const binaryPath = mcpBinaryPath;
    if (selectedMcpClient === 'CLAUDE') {
      return JSON.stringify(
        {
          mcpServers: {
            'pz-mod-studio': {
              command: binaryPath,
              args: [],
            },
          },
        },
        null,
        2
      );
    } else if (selectedMcpClient === 'CURSOR') {
      return JSON.stringify(
        {
          mcpServers: {
            'pz-mod-studio': {
              command: binaryPath,
              args: [],
            },
          },
        },
        null,
        2
      );
    } else if (selectedMcpClient === 'VSCODE_LOCAL') {
      return JSON.stringify(
        {
          mcpServers: {
            'pz-mod-studio': {
              command: binaryPath,
              args: [],
              disabled: false,
              autoApprove: [],
            },
          },
        },
        null,
        2
      );
    } else if (selectedMcpClient === 'OPENAI_CODEX') {
      return JSON.stringify(
        {
          mcpServers: {
            'pz-mod-studio': {
              command: binaryPath,
              args: [],
              description: 'Project Zomboid Mod Studio MCP Server (Build 41/42)',
            },
          },
        },
        null,
        2
      );
    } else {
      // ANTIGRAVITY / GEMINI CLI
      return JSON.stringify(
        {
          mcpServers: {
            'pz-mod-studio': {
              command: binaryPath,
              args: [],
              env: {},
            },
          },
        },
        null,
        2
      );
    }
  };

  const handleCopyMcpConfig = () => {
    navigator.clipboard.writeText(getMcpConfigSnippet());
    setCopiedMcpConfig(true);
    setTimeout(() => setCopiedMcpConfig(false), 2500);
  };

  return (
    <div className="flex-1 flex flex-col bg-slate-950 text-slate-100 overflow-y-auto p-6 font-sans">
      <div className="max-w-6xl w-full space-y-6">
        {/* Settings Header & Sub-Tab Navigation */}
        <div className="flex items-center justify-between border-b border-slate-800 pb-4">
          <div>
            <h2 className="text-xl font-bold text-slate-100 flex items-center gap-2">
              <HardDrive className="w-5 h-5 text-emerald-400" />
              {t('settings.title')}
            </h2>
            <p className="text-xs text-slate-400 mt-1">
              {t('settings.blurb')}
            </p>
          </div>

          {/* Sub-Tab Selector Buttons */}
          <div className="bg-slate-900 border border-slate-800 rounded-lg p-1 flex gap-1 text-xs">
            <button
              onClick={() => setActiveSubTab('PATHS')}
              className={`px-3 py-1.5 rounded-md font-medium transition cursor-pointer flex items-center gap-1.5 ${
                activeSubTab === 'PATHS'
                  ? 'bg-slate-800 text-emerald-400 font-bold border border-slate-700 shadow-sm'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              <HardDrive className="w-3.5 h-3.5" />
              {t('settings.tabPaths')}
            </button>
            <button
              onClick={() => setActiveSubTab('POLYFILLS')}
              className={`px-3 py-1.5 rounded-md font-medium transition cursor-pointer flex items-center gap-1.5 ${
                activeSubTab === 'POLYFILLS'
                  ? 'bg-slate-800 text-emerald-400 font-bold border border-slate-700 shadow-sm'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              <Wrench className="w-3.5 h-3.5" />
              {t('settings.tabShim')}
            </button>
            {showAdvanced && (
            <button
              onClick={() => setActiveSubTab('MCP')}
              className={`px-3 py-1.5 rounded-md font-medium transition cursor-pointer flex items-center gap-1.5 ${
                activeSubTab === 'MCP'
                  ? 'bg-slate-800 text-cyan-400 font-bold border border-slate-700 shadow-sm'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              <Bot className="w-3.5 h-3.5 text-cyan-400" />
              {t('settings.tabMcp')}
            </button>
            )}
          </div>
        </div>

        {savedSuccess && (
          <div className="p-3 bg-emerald-950/80 border border-emerald-500/40 rounded-lg text-xs text-emerald-300 flex items-center gap-2">
            <CheckCircle className="w-4 h-4 text-emerald-400" />
            {t('settings.saved')}
          </div>
        )}

        {/* SUB-TAB 1: Directory Paths */}
        {activeSubTab === 'PATHS' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <span className="text-xs font-bold text-slate-400 uppercase tracking-wider">
                {t('settings.folders')}
              </span>

              <div className="flex items-center gap-2">
                <button
                  onClick={onAutoDetect}
                  className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg border border-slate-700 transition cursor-pointer"
                >
                  <RefreshCw className="w-3.5 h-3.5 text-cyan-400" />
                  {t('settings.autoDetect')}
                </button>
              </div>
            </div>

            {/* Card 1: PZ Installation Path */}
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-2">
              <div className="flex items-center justify-between">
                <label className="text-xs font-bold text-slate-200">
                  {t('settings.gameDir')}
                </label>
                {/* The backend is what knows whether the folder is really a game install. */}
                {!formData.pz_install_dir ? (
                  <span className="text-[10px] text-amber-400 flex items-center gap-1">
                    <AlertTriangle className="w-3 h-3" aria-hidden="true" /> {t('settings.notSet')}
                  </span>
                ) : paths.is_valid ? (
                  <span className="text-[10px] text-emerald-400 flex items-center gap-1">
                    <CheckCircle className="w-3 h-3" aria-hidden="true" /> {t('settings.foundGame')}
                  </span>
                ) : (
                  <span className="text-[10px] text-amber-400 flex items-center gap-1">
                    <AlertTriangle className="w-3 h-3" aria-hidden="true" /> {t('settings.notFoundHere')}
                  </span>
                )}
              </div>
              <p className="text-[11px] text-slate-400" dangerouslySetInnerHTML={{ __html: tHtml('settings.gameDirBlurb') }} />
              <div className="flex gap-2">
                <input
                  type="text"
                  readOnly
                  value={formData.pz_install_dir}
                  className="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-xs font-mono text-slate-300 cursor-not-allowed"
                />
                <button
                  onClick={() => handlePickFolder('pz_install_dir', formData.pz_install_dir)}
                  className="flex items-center gap-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium rounded-lg border border-slate-700 transition cursor-pointer"
                >
                  <FolderOpen className="w-4 h-4 text-emerald-400" />
                  {t('settings.browse')}
                </button>
              </div>
            </div>

            {/* Card 2: Steam Workshop / External Mods Path */}
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-2">
              <div className="flex items-center justify-between">
                <label className="text-xs font-bold text-slate-200">
                  {t('settings.workshopDir')}
                </label>
                {formData.workshop_dir ? (
                  <span className="text-[10px] text-slate-400 font-mono">{t('settings.steamAppId')}</span>
                ) : (
                  <span className="text-[10px] text-cyan-300 font-mono bg-cyan-950/60 px-2 py-0.5 rounded border border-cyan-800/50">
                    {t('settings.optionalNoSteam')}
                  </span>
                )}
              </div>
              <p className="text-[11px] text-slate-400" dangerouslySetInnerHTML={{ __html: tHtml('settings.workshopDirBlurb') }} />
              <div className="flex gap-2">
                <input
                  type="text"
                  readOnly
                  value={formData.workshop_dir}
                  placeholder={t('settings.workshopPlaceholder')}
                  className="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-xs font-mono text-slate-300 cursor-not-allowed"
                />
                <button
                  onClick={() => handlePickFolder('workshop_dir', formData.workshop_dir)}
                  className="flex items-center gap-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium rounded-lg border border-slate-700 transition cursor-pointer"
                >
                  <FolderOpen className="w-4 h-4 text-emerald-400" />
                  {t('settings.browse')}
                </button>
              </div>
            </div>

            {/* Card 3: User Zomboid Directory */}
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-2">
              <div className="flex items-center justify-between">
                <label className="text-xs font-bold text-slate-200">
                  {t('settings.userDir')}
                </label>
                {formData.user_zomboid_dir ? (
                  <span className="text-[10px] text-emerald-400 flex items-center gap-1">
                    <CheckCircle className="w-3 h-3" /> {t('settings.universal')}
                  </span>
                ) : (
                  <span className="text-[10px] text-amber-400 flex items-center gap-1">
                    <AlertTriangle className="w-3 h-3" /> {t('settings.required')}
                  </span>
                )}
              </div>
              <p className="text-[11px] text-slate-400" dangerouslySetInnerHTML={{ __html: tHtml('settings.userDirBlurb') }} />
              <div className="flex gap-2">
                <input
                  type="text"
                  readOnly
                  value={formData.user_zomboid_dir}
                  className="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-xs font-mono text-slate-300 cursor-not-allowed"
                />
                <button
                  onClick={() => handlePickFolder('user_zomboid_dir', formData.user_zomboid_dir)}
                  className="flex items-center gap-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium rounded-lg border border-slate-700 transition cursor-pointer"
                >
                  <FolderOpen className="w-4 h-4 text-emerald-400" />
                  {t('settings.browse')}
                </button>
              </div>
            </div>

            {/* Card 4: ModListData.ini Path */}
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-2">
              <label className="text-xs font-bold text-slate-200">
                {t('settings.modListFile')}
              </label>
              <p className="text-[11px] text-slate-400" dangerouslySetInnerHTML={{ __html: tHtml('settings.modListBlurb') }} />
              <div className="flex gap-2">
                <input
                  type="text"
                  readOnly
                  value={formData.mod_list_ini_path}
                  className="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-xs font-mono text-slate-300 cursor-not-allowed"
                />
                <button
                  onClick={() => handlePickModListFile()}
                  className="flex items-center gap-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium rounded-lg border border-slate-700 transition cursor-pointer"
                >
                  <FolderOpen className="w-4 h-4 text-emerald-400" />
                  {t('settings.browse')}
                </button>
              </div>
            </div>

            {/* Card 5: Unified App Notification & Warning Preferences */}
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-4">
              <div className="flex items-center justify-between border-b border-slate-800/80 pb-3">
                <label className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                  <Bell className="w-4 h-4 text-emerald-400" />
                  {t('settings.notifications')}
                </label>
                <span className="text-[10px] font-bold font-mono px-2 py-0.5 rounded bg-slate-950 text-slate-400 border border-slate-800">
                  {t('settings.uiPrefs')}
                </span>
              </div>
              <p className="text-[11px] text-slate-400">
                {t('settings.chooseWarnings')}
              </p>

              <div className="space-y-3">
                {/* 1. Warning before launching with no merge package */}
                <div className="p-3.5 bg-slate-950/60 border border-slate-800 rounded-xl flex flex-col md:flex-row md:items-center justify-between gap-3">
                  <div className="space-y-1 max-w-xl">
                    <div className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                      <AlertTriangle className="w-3.5 h-3.5 text-amber-400" />
                      <span>{t('settings.noMergeWarning')}</span>
                    </div>
                    <p className="text-[11px] text-slate-400 leading-relaxed">
                      {t('settings.noMergeWarningBlurb')}
                    </p>
                  </div>

                  <div className="shrink-0 flex items-center gap-2">
                    {noMergeWarningSilenced ? (
                      <button
                        onClick={handleRestoreNoMergeWarning}
                        className="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-xs font-bold transition cursor-pointer shadow"
                      >
                        {t('settings.restoreWarning')}
                      </button>
                    ) : (
                      <button
                        onClick={handleSilenceNoMergeWarning}
                        className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-xs font-medium border border-slate-700 transition cursor-pointer"
                      >
                        {t('settings.silenceWarning')}
                      </button>
                    )}
                  </div>
                </div>

                {/* 1b. Notice before launching with definitions two mods disagree on */}
                <div className="p-3.5 bg-slate-950/60 border border-slate-800 rounded-xl flex flex-col md:flex-row md:items-center justify-between gap-3">
                  <div className="space-y-1 max-w-xl">
                    <div className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                      <Info className="w-3.5 h-3.5 text-sky-400" />
                      <span>{t('settings.definitionWarning')}</span>
                    </div>
                    <p className="text-[11px] text-slate-400 leading-relaxed">
                      {t('settings.definitionWarningBlurb')}
                    </p>
                  </div>

                  <div className="shrink-0 flex items-center gap-2">
                    {definitionWarningSilenced ? (
                      <button
                        onClick={handleRestoreDefinitionWarning}
                        className="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-xs font-bold transition cursor-pointer shadow"
                      >
                        {t('settings.restoreWarning')}
                      </button>
                    ) : (
                      <button
                        onClick={handleSilenceDefinitionWarning}
                        className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-xs font-medium border border-slate-700 transition cursor-pointer"
                      >
                        {t('settings.silenceWarning')}
                      </button>
                    )}
                  </div>
                </div>

                {/* 2. The auto-sort explanation */}
                <div className="p-3.5 bg-slate-950/60 border border-slate-800 rounded-xl flex flex-col md:flex-row md:items-center justify-between gap-3">
                  <div className="space-y-1 max-w-xl">
                    <div className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                      <RefreshCw className="w-3.5 h-3.5 text-cyan-400" />
                      <span>{t('settings.sortNotice')}</span>
                    </div>
                    <p className="text-[11px] text-slate-400 leading-relaxed">
                      {t('settings.sortNoticeBlurb')}
                    </p>
                  </div>

                  <div className="shrink-0 flex items-center gap-2">
                    {noticeSilenced ? (
                      <button
                        onClick={handleRestoreNotice}
                        className="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-xs font-bold transition cursor-pointer shadow"
                      >
                        {t('settings.restoreNotification')}
                      </button>
                    ) : (
                      <button
                        onClick={() => {
                          localStorage.setItem('pz_hide_autosort_notice', 'true');
                          setNoticeSilenced(true);
                        }}
                        className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-xs font-medium border border-slate-700 transition cursor-pointer"
                      >
                        {t('settings.silenceNotification')}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            </div>

            {/* Card 6: Startup Screen Behavior */}
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-3">
              <div className="flex items-center justify-between">
                <label className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                  <Layers className="w-4 h-4 text-emerald-400" />
                  {t('settings.startup')}
                </label>
                <span className="text-[10px] font-bold font-mono px-2 py-0.5 rounded bg-slate-950 text-emerald-300 border border-slate-800">
                  {startupBehavior === 'SHOW_STARTUP_SCREEN' ? t('settings.profilesScreen') : t('settings.lastProfile')}
                </span>
              </div>
              <p className="text-[11px] text-slate-400">
                {t('settings.startupQuestion')}
              </p>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
                <div
                  onClick={() => handleSetStartupBehavior('SHOW_STARTUP_SCREEN')}
                  className={`p-3.5 rounded-xl border transition cursor-pointer flex items-start gap-3 ${
                    startupBehavior === 'SHOW_STARTUP_SCREEN'
                      ? 'bg-emerald-950/40 border-emerald-500 text-slate-100 ring-1 ring-emerald-500/50'
                      : 'bg-slate-950/60 border-slate-800 text-slate-400 hover:border-slate-700 hover:text-slate-200'
                  }`}
                >
                  <input
                    type="radio"
                    name="startupBehavior"
                    checked={startupBehavior === 'SHOW_STARTUP_SCREEN'}
                    onChange={() => handleSetStartupBehavior('SHOW_STARTUP_SCREEN')}
                    className="mt-1 text-emerald-500 focus-visible:outline-2 focus-visible:outline-emerald-400 focus-visible:outline-offset-2 cursor-pointer"
                  />
                  <div className="space-y-1 text-left">
                    <div className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                      <span>{t('settings.profilesStart')}</span>
                      <span className="text-[10px] bg-emerald-950 text-emerald-300 border border-emerald-700/60 px-1.5 py-0.5 rounded font-mono">{t('settings.recommended')}</span>
                    </div>
                    <p className="text-[11px] text-slate-400 leading-relaxed">
                      {t('settings.profilesStartBlurb')}
                    </p>
                  </div>
                </div>

                <div
                  onClick={() => handleSetStartupBehavior('OPEN_LAST_PROFILE')}
                  className={`p-3.5 rounded-xl border transition cursor-pointer flex items-start gap-3 ${
                    startupBehavior === 'OPEN_LAST_PROFILE'
                      ? 'bg-emerald-950/40 border-emerald-500 text-slate-100 ring-1 ring-emerald-500/50'
                      : 'bg-slate-950/60 border-slate-800 text-slate-400 hover:border-slate-700 hover:text-slate-200'
                  }`}
                >
                  <input
                    type="radio"
                    name="startupBehavior"
                    checked={startupBehavior === 'OPEN_LAST_PROFILE'}
                    onChange={() => handleSetStartupBehavior('OPEN_LAST_PROFILE')}
                    className="mt-1 text-emerald-500 focus-visible:outline-2 focus-visible:outline-emerald-400 focus-visible:outline-offset-2 cursor-pointer"
                  />
                  <div className="space-y-1 text-left">
                    <div className="text-xs font-bold text-slate-200">
                      {t('settings.openLast')}
                    </div>
                    <p className="text-[11px] text-slate-400 leading-relaxed">
                      {t('settings.openLastBlurb')}
                    </p>
                  </div>
                </div>
              </div>

              <div className="flex items-start justify-between gap-4 pt-2 border-t border-slate-800">
                <div className="space-y-1">
                  <div className="text-xs font-bold text-slate-200">{t('settings.rememberWindow')}</div>
                  <p className="text-[11px] text-slate-400 max-w-2xl">{t('settings.rememberWindowBlurb')}</p>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-checked={rememberWindow}
                  aria-label={t('settings.rememberWindow')}
                  onClick={() => saveWindowPrefs(!rememberWindow, closeToTray)}
                  className={`w-10 h-5 shrink-0 rounded-full p-0.5 flex items-center transition cursor-pointer ${rememberWindow ? 'bg-emerald-500 justify-end' : 'bg-slate-700 justify-start'}`}
                >
                  <span className="w-4 h-4 rounded-full bg-white shadow-md" />
                </button>
              </div>

              <div className="space-y-2 pt-2 border-t border-slate-800">
                <div className="text-xs font-bold text-slate-200">{t('settings.closeButton')}</div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                  {([false, true] as const).map((tray) => (
                    <label key={String(tray)} className={`p-3 rounded-xl border cursor-pointer flex items-start gap-3 ${closeToTray === tray ? 'bg-emerald-950/40 border-emerald-500 ring-1 ring-emerald-500/50' : 'bg-slate-950/60 border-slate-800 hover:border-slate-700'}`}>
                      <input type="radio" name="closeButton" checked={closeToTray === tray} onChange={() => saveWindowPrefs(rememberWindow, tray)} className="mt-1 accent-emerald-500 cursor-pointer" />
                      <div className="space-y-1">
                        <div className="text-xs font-bold text-slate-200">{tray ? t('settings.closeToTray') : t('settings.closeQuits')}</div>
                        <p className="text-[11px] text-slate-400 leading-relaxed">{tray ? t('settings.closeToTrayBlurb') : t('settings.closeQuitsBlurb')}</p>
                      </div>
                    </label>
                  ))}
                </div>
              </div>
            </div>
          </div>
        )}

        {/* SUB-TAB 2: what the Build 42 compatibility shim covers */}
        {activeSubTab === 'PATHS' && (
          <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex items-start justify-between gap-4">
            <div className="space-y-1">
              <div className="text-xs font-bold text-slate-200">{t('settings.language.title')}</div>
              <p className="text-[11px] text-slate-400 max-w-2xl">{t('settings.language.blurb')}</p>
              <p className="text-[11px] text-slate-500 max-w-2xl">{t('settings.language.contribute')}</p>
            </div>
            <select
              value={i18n.language.slice(0, 2)}
              onChange={(e) => setLanguage(e.target.value)}
              aria-label={t('settings.language.title')}
              className="shrink-0 px-2.5 py-1.5 rounded-lg bg-slate-950 border border-slate-800 text-xs text-slate-200 cursor-pointer"
            >
              {LANGUAGES.map((l) => <option key={l.code} value={l.code}>{l.name}</option>)}
            </select>
          </div>
        )}

        {activeSubTab === 'PATHS' && (
          <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex items-start justify-between gap-4">
            <div className="space-y-1">
              <div className="text-xs font-bold text-slate-200">{t('settings.rememberMode')}</div>
              <p className="text-[11px] text-slate-400 max-w-2xl">
                {t('settings.rememberModeBlurb')}
              </p>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={rememberLaunchMode}
              aria-label={t('settings.rememberMode')}
              onClick={() => toggleRememberLaunchMode(!rememberLaunchMode)}
              className={`w-10 h-5 shrink-0 rounded-full p-0.5 flex items-center transition cursor-pointer ${
                rememberLaunchMode ? 'bg-emerald-500 justify-end' : 'bg-slate-700 justify-start'
              }`}
            >
              <span className="w-4 h-4 rounded-full bg-white shadow-md" />
            </button>
          </div>
        )}

        {activeSubTab === 'PATHS' && (
          <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex items-start justify-between gap-4">
            <div className="space-y-1">
              <div className="text-xs font-bold text-slate-200">{t('settings.modsScreen')}</div>
              <p className="text-[11px] text-slate-400 max-w-2xl">{t('settings.modsScreenBlurb')}</p>
            </div>
            <select
              value={screenChoice}
              onChange={(e) => {
                const v = e.target.value as 'ask' | 'studio' | 'other';
                setScreenChoice(v);
                void TauriService.setLinkScreenChoice(paths, v);
              }}
              aria-label={t('settings.modsScreen')}
              className="shrink-0 px-2.5 py-1.5 rounded-lg bg-slate-950 border border-slate-800 text-xs text-slate-200 cursor-pointer"
            >
              <option value="ask">{t('settings.modsScreenAsk')}</option>
              <option value="studio">{t('settings.modsScreenStudio')}</option>
              <option value="other">{t('settings.modsScreenOther')}</option>
            </select>
          </div>
        )}

        {activeSubTab === 'PATHS' && (
          <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex items-start justify-between gap-4">
            <div className="space-y-1">
              <div className="text-xs font-bold text-slate-200">{t('settings.updateCheck')}</div>
              <p className="text-[11px] text-slate-400 max-w-2xl">{t('settings.updateCheckBlurb')}</p>
              <div className="flex flex-wrap items-center gap-2 pt-1">
                <button type="button" disabled={checkingUpdate} onClick={() => void checkUpdateNow()}
                  className="flex items-center gap-1 px-2 py-1 rounded border border-slate-700 text-[11px] text-slate-300 hover:bg-slate-800 cursor-pointer disabled:opacity-60">
                  {checkingUpdate
                    ? <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" />
                    : <RefreshCw className="w-3 h-3" aria-hidden="true" />}
                  {checkingUpdate ? t('settings.updateChecking') : t('settings.updateCheckNow')}
                </button>
                {appUpdate && !checkingUpdate && (
                  appUpdate.error && !appUpdate.latest ? (
                    <span className="text-[11px] text-amber-400">{t('settings.updateFailed', { error: appUpdate.error })}</span>
                  ) : appUpdate.newer && appUpdate.latest ? (
                    <>
                      <span className="text-[11px] text-emerald-300">{t('settings.updateAvailable', { latest: appUpdate.latest, current: appUpdate.current })}</span>
                      <UpdateNowButton version={appUpdate.latest} compact />
                      <button type="button" onClick={() => appUpdate.url && TauriService.openExternalUrl(appUpdate.url)}
                        className="flex items-center gap-1 px-2 py-1 rounded border border-emerald-700 hover:bg-emerald-900/50 text-[11px] text-emerald-200 font-semibold cursor-pointer">
                        <ExternalLink className="w-3 h-3" aria-hidden="true" /> {t('settings.updateDownload')}
                      </button>
                    </>
                  ) : appUpdate.latest ? (
                    <span className="text-[11px] text-slate-400">{t('settings.updateUpToDate', { current: appUpdate.current })}</span>
                  ) : null
                )}
              </div>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={appUpdate?.enabled ?? true}
              aria-label={t('settings.updateCheck')}
              onClick={() => void toggleAppUpdate(!(appUpdate?.enabled ?? true))}
              className={`w-10 h-5 shrink-0 rounded-full p-0.5 flex items-center transition cursor-pointer ${
                appUpdate?.enabled ?? true ? 'bg-emerald-500 justify-end' : 'bg-slate-700 justify-start'
              }`}
            >
              <span className="w-4 h-4 rounded-full bg-white shadow-md" />
            </button>
          </div>
        )}

        {activeSubTab === 'PATHS' && <DiagnosticsCard paths={paths} />}

        {activeSubTab === 'PATHS' && (
          <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-3">
            <div className="space-y-1">
              <div className="text-xs font-bold text-slate-200 flex items-center gap-2">
                <Languages className="w-3.5 h-3.5 text-sky-400" aria-hidden="true" /> {t('settings.translate.title')}
              </div>
              <p className="text-[11px] text-slate-400 max-w-2xl">{t('settings.translate.blurb')}</p>
            </div>
            {deeplSaved ? (
              <div className="flex flex-wrap items-center gap-2 text-[11px]">
                <span className="flex items-center gap-1.5 text-emerald-300">
                  <CheckCircle className="w-3.5 h-3.5" aria-hidden="true" /> {t('settings.translate.saved')}
                </span>
                <button type="button" onClick={() => void removeDeeplKey()}
                  className="px-2.5 py-1 rounded-lg text-[11px] text-slate-300 hover:text-red-300 border border-slate-800 hover:border-red-900 cursor-pointer transition">
                  {t('settings.remove')}
                </button>
              </div>
            ) : (
              <form className="flex flex-wrap gap-2" onSubmit={(e) => { e.preventDefault(); void saveDeeplKey(); }}>
                <button type="button" onClick={() => TauriService.openExternalUrl('https://www.deepl.com/pro-api')}
                  className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer">
                  <ExternalLink className="w-3 h-3" aria-hidden="true" /> {t('settings.translate.get')}
                </button>
                <input type="password" autoComplete="off" spellCheck={false} value={deeplInput} onChange={(e) => setDeeplInput(e.target.value)}
                  placeholder={t('settings.translate.paste')} aria-label={t('settings.translate.label')}
                  className="flex-1 min-w-[220px] px-2.5 py-1.5 rounded-lg bg-slate-950 border border-slate-800 focus:border-sky-700 text-[11px] text-slate-100 outline-none placeholder:text-slate-600" />
                <button type="submit" disabled={deeplBusy || !deeplInput.trim()}
                  className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer disabled:opacity-50">
                  {deeplBusy && <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" />} {t('settings.checkAndSave')}
                </button>
              </form>
            )}
            <div className="flex flex-wrap items-center gap-2 text-[11px] text-slate-400">
              <span>{t('settings.translate.cached', { count: translationCache.count, size: formatKb(translationCache.bytes) })}</span>
              {translationCache.count > 0 && (
                <button type="button" onClick={() => void clearTranslationCache()}
                  className="px-2.5 py-1 rounded-lg text-[11px] text-slate-300 hover:text-red-300 border border-slate-800 hover:border-red-900 cursor-pointer transition">
                  {t('settings.translate.clear')}
                </button>
              )}
            </div>
            {deeplError && <p className="text-[11px] text-red-300">{deeplError}</p>}
          </div>
        )}

        {activeSubTab === 'PATHS' && (
          <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex items-start justify-between gap-4">
            <div className="space-y-1">
              <div className="text-xs font-bold text-slate-200">{t('settings.developer')}</div>
              <p className="text-[11px] text-slate-400 max-w-2xl">
                {t('settings.developerBlurb')}
              </p>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={showAdvanced}
              aria-label={t('settings.developer')}
              onClick={() => toggleAdvanced(!showAdvanced)}
              className={`w-10 h-5 shrink-0 rounded-full p-0.5 flex items-center transition cursor-pointer ${
                showAdvanced ? 'bg-emerald-500 justify-end' : 'bg-slate-700 justify-start'
              }`}
            >
              <span className="w-4 h-4 rounded-full bg-white shadow-md" />
            </button>
          </div>
        )}

        {activeSubTab === 'POLYFILLS' && (
          <div className="space-y-4">
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-3">
              <div className="flex items-start gap-3">
                <div className="w-9 h-9 rounded-lg bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center shrink-0">
                  <Wrench className="w-4 h-4 text-emerald-400" aria-hidden="true" />
                </div>
                <div className="space-y-1">
                  <h3 className="text-xs font-bold text-slate-100">{t('settings.shimTitle')}</h3>
                  <p className="text-[11px] text-slate-400 leading-relaxed max-w-3xl">
                    {t('settings.shimBlurb')}
                  </p>
                </div>
              </div>

              <div className="pt-3 border-t border-slate-800">
                <div className="flex items-baseline justify-between mb-2">
                  <div className="text-[10px] font-bold uppercase tracking-wider text-slate-500">{t('settings.whatItCovers')}</div>
                  <div className="text-[10px] text-slate-500 font-mono">
                    {t('settings.readFromShim')}{shimLines > 0 ? t('settings.linesOfLua', { count: shimLines }) : ''}
                  </div>
                </div>
                <ul className="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-1.5">
                  {shimSections.map((section) => (
                    <li key={section.number} className="flex items-start gap-2 text-[11px] text-slate-300">
                      <Check className="w-3 h-3 text-emerald-400 shrink-0 mt-0.5" aria-hidden="true" />
                      <span>
                        {section.title}
                        <span className="block text-[10px] text-slate-500 font-mono">
                          {t('settings.partLines', { number: section.number, lines: section.lines })}
                        </span>
                      </span>
                    </li>
                  ))}
                </ul>
                {shimSections.length === 0 && (
                  <p className="text-[11px] text-slate-500">{t('settings.shimUnreadable')}</p>
                )}
              </div>

              {/* Part 7, one switch per mod: off means the shim leaves that mod alone. */}
              {shimPatches.length > 0 && (
                <div className="pt-3 border-t border-slate-800">
                  <div className="flex items-baseline justify-between mb-2">
                    <div className="text-[10px] font-bold uppercase tracking-wider text-slate-500">{t('settings.perModPatches')}</div>
                    <div className="text-[10px] text-slate-500 font-mono">
                      {t('settings.perModNote')}
                    </div>
                  </div>
                  <ul className="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-1.5">
                    {shimPatches.map((patch) => (
                      <li key={patch.name} className="flex items-center justify-between gap-3 text-[11px] text-slate-300">
                        <span>
                          {patch.label}
                          <span className="block text-[10px] text-slate-500 font-mono">{patch.name}</span>
                        </span>
                        <button
                          type="button"
                          role="switch"
                          aria-checked={patch.enabled}
                          onClick={() => void togglePatch(patch)}
                          className={`shrink-0 px-2 py-0.5 rounded text-[10px] font-mono font-bold border cursor-pointer transition ${
                            patch.enabled
                              ? 'bg-emerald-950/80 text-emerald-300 border-emerald-700/70 hover:bg-emerald-900/80'
                              : 'bg-slate-900 text-slate-400 border-slate-700 hover:bg-slate-800'
                          }`}
                          title={patch.enabled ? t('settings.patchOnTitle') : t('settings.patchOffTitle')}
                        >
                          {patch.enabled ? t('settings.on') : t('settings.off')}
                        </button>
                      </li>
                    ))}
                  </ul>
                  {patchNote && <p className="text-[11px] text-emerald-300 mt-2">{patchNote}</p>}
                  <p className="text-[10px] text-slate-500 mt-2 leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('settings.switchNote') }} />
                </div>
              )}
            </div>
          </div>
        )}

        {/* SUB-TAB 3: Model Context Protocol (MCP) Agent Server */}
        {activeSubTab === 'MCP' && (
          <div className="space-y-6">
            {/* Header Banner */}
            <div className="bg-gradient-to-r from-slate-900 via-slate-900/90 to-cyan-950/40 border border-cyan-500/30 rounded-xl p-5 shadow-lg">
              <div className="flex items-start justify-between">
                <div className="space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="p-1.5 rounded-lg bg-cyan-950 text-cyan-400 border border-cyan-800">
                      <Bot className="w-5 h-5" />
                    </span>
                    <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
                      {t('settings.mcp.title')}
                      <span className="text-[10px] font-mono font-semibold px-2 py-0.5 bg-emerald-950 text-emerald-300 border border-emerald-800 rounded-full flex items-center gap-1">
                        <Radio className="w-2.5 h-2.5 text-emerald-400 animate-pulse" />
                        {t('settings.mcp.ready')}
                      </span>
                    </h3>
                  </div>
                  <p className="text-xs text-slate-300 max-w-3xl leading-relaxed">
                    {t('settings.mcp.blurb')}
                  </p>
                </div>
              </div>
            </div>

            {/* Quick Setup / Client Configuration */}
            <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-5 space-y-4">
              <div className="flex flex-col md:flex-row md:items-center justify-between gap-3">
                <div>
                  <h4 className="text-xs font-bold text-slate-200 uppercase tracking-wider flex items-center gap-1.5">
                    <Terminal className="w-4 h-4 text-cyan-400" />
                    {t('settings.mcp.generator')}
                  </h4>
                  <p className="text-[11px] text-slate-400 mt-0.5">
                    {t('settings.mcp.generatorBlurb')}
                  </p>
                </div>

                {/* Client Selector Pills */}
                <div className="bg-slate-950 border border-slate-800 rounded-lg p-1 flex flex-wrap gap-1 text-[11px]">
                  <button
                    onClick={() => setSelectedMcpClient('ANTIGRAVITY')}
                    className={`px-2.5 py-1 rounded font-medium transition cursor-pointer ${
                      selectedMcpClient === 'ANTIGRAVITY'
                        ? 'bg-cyan-950 text-cyan-300 font-bold border border-cyan-800'
                        : 'text-slate-400 hover:text-slate-200'
                    }`}
                  >
                    Antigravity / Gemini
                  </button>
                  <button
                    onClick={() => setSelectedMcpClient('CLAUDE')}
                    className={`px-2.5 py-1 rounded font-medium transition cursor-pointer ${
                      selectedMcpClient === 'CLAUDE'
                        ? 'bg-cyan-950 text-cyan-300 font-bold border border-cyan-800'
                        : 'text-slate-400 hover:text-slate-200'
                    }`}
                  >
                    Claude Desktop
                  </button>
                  <button
                    onClick={() => setSelectedMcpClient('CURSOR')}
                    className={`px-2.5 py-1 rounded font-medium transition cursor-pointer ${
                      selectedMcpClient === 'CURSOR'
                        ? 'bg-cyan-950 text-cyan-300 font-bold border border-cyan-800'
                        : 'text-slate-400 hover:text-slate-200'
                    }`}
                  >
                    Cursor / Windsurf
                  </button>
                  <button
                    onClick={() => setSelectedMcpClient('VSCODE_LOCAL')}
                    className={`px-2.5 py-1 rounded font-medium transition cursor-pointer ${
                      selectedMcpClient === 'VSCODE_LOCAL'
                        ? 'bg-cyan-950 text-cyan-300 font-bold border border-cyan-800'
                        : 'text-slate-400 hover:text-slate-200'
                    }`}
                  >
                    VS Code (Roo / Cline / Ollama Local)
                  </button>
                  <button
                    onClick={() => setSelectedMcpClient('OPENAI_CODEX')}
                    className={`px-2.5 py-1 rounded font-medium transition cursor-pointer ${
                      selectedMcpClient === 'OPENAI_CODEX'
                        ? 'bg-cyan-950 text-cyan-300 font-bold border border-cyan-800'
                        : 'text-slate-400 hover:text-slate-200'
                    }`}
                  >
                    OpenAI / Codex / ChatGPT
                  </button>
                </div>
              </div>

              {/* JSON Code Box */}
              <div className="relative bg-slate-950 border border-slate-800 rounded-lg p-4 font-mono text-xs text-slate-300 overflow-x-auto">
                <button
                  onClick={handleCopyMcpConfig}
                  className={`absolute top-3 right-3 px-3 py-1.5 rounded-md text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer shadow ${
                    copiedMcpConfig
                      ? 'bg-emerald-600 text-white'
                      : 'bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700'
                  }`}
                >
                  {copiedMcpConfig ? (
                    <>
                      <Check className="w-3.5 h-3.5" />
                      {t('settings.mcp.copied')}
                    </>
                  ) : (
                    <>
                      <Copy className="w-3.5 h-3.5" />
                      {t('settings.mcp.copy')}
                    </>
                  )}
                </button>
                <pre>{getMcpConfigSnippet()}</pre>
              </div>

              {/* Step-by-Step Instructions per Client */}
              {(() => {
                const guide = MCP_GUIDES[selectedMcpClient];
                if (!guide) return null;
                const base = `settings.mcp.guides.${guide.key}`;
                return (
                  <div className="bg-slate-950/70 border border-slate-800 rounded-lg p-4 space-y-2.5 text-xs">
                    <div className="flex items-center gap-2 text-cyan-300 font-bold">
                      <Info className="w-4 h-4 text-cyan-400" />
                      <span>{t(`${base}.title`)}</span>
                    </div>
                    <ol className="list-decimal list-inside space-y-1.5 text-slate-300 leading-relaxed pl-1 text-[11px] [&_code]:bg-slate-900 [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-cyan-300 [&_code]:font-mono [&_code]:text-[10px] [&_kbd]:px-1 [&_kbd]:py-0.5 [&_kbd]:rounded [&_kbd]:bg-slate-800 [&_kbd]:font-mono [&_kbd]:text-[10px] [&_kbd]:text-slate-200">
                      {Array.from({ length: guide.steps }, (_, i) => (
                        <li key={i} dangerouslySetInnerHTML={{ __html: tHtml(`${base}.s${i + 1}`, { path: mcpBinaryPath }) }} />
                      ))}
                    </ol>
                    {guide.tip && (
                      <div
                        className={`p-2 rounded text-[11px] ${guide.tip === 'emerald' ? 'bg-emerald-950/30 border border-emerald-900/60 text-emerald-200' : 'bg-cyan-950/30 border border-cyan-900/60 text-cyan-200'}`}
                        dangerouslySetInnerHTML={{ __html: tHtml(`${base}.tip`) }}
                      />
                    )}
                  </div>
                );
              })()}

              {/* FAQ / How Stdio Works Card */}
              <div className="p-3.5 bg-amber-950/20 border border-amber-500/30 rounded-lg text-xs text-amber-200 space-y-1.5">
                <div className="flex items-center gap-1.5 font-bold text-amber-300">
                  <Lightbulb className="w-4 h-4 text-amber-400 shrink-0" />
                  <span>{t('settings.mcp.important')}</span>
                </div>
                <p className="text-[11px] text-amber-300/80 leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('settings.mcp.importantBlurb') }} />
              </div>

              <div className="text-[11px] text-slate-400 flex flex-wrap items-center gap-2 pt-1 border-t border-slate-800/60">
                <span className="font-semibold text-slate-300">{t('settings.mcp.executable')}</span>
                <code className="bg-slate-950 px-2 py-0.5 rounded border border-slate-800 text-cyan-300 font-mono text-[10px]">
                  &lt;PZ-Mod-Studio-Folder&gt;\PZ-Mod-Studio-Portable\pz-mcp-server.exe
                </code>
                <span>{t('settings.mcp.orFlag')}</span>
                <code className="bg-slate-950 px-2 py-0.5 rounded border border-slate-800 text-cyan-300 font-mono text-[10px]">
                  Project-Zomboid-Mod-Studio.exe --mcp
                </code>
              </div>
            </div>

            {/* The tool and resource catalogue */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
              {/* Exposed Tools (Span 2 cols) */}
              <div className="lg:col-span-2 bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-3">
                <div className="flex items-center justify-between pb-2 border-b border-slate-800">
                  <h4 className="text-xs font-bold text-slate-200 uppercase tracking-wider flex items-center gap-1.5">
                    <Code2 className="w-4 h-4 text-emerald-400" />
                    MCP Tools Catalog (34 tools)
                  </h4>
                  <span className="text-[10px] font-mono text-emerald-400 font-bold bg-emerald-950/60 px-2 py-0.5 rounded border border-emerald-800">
                    tools/call
                  </span>
                </div>

                <div className="space-y-3.5 max-h-[520px] overflow-y-auto pr-1">
                  {/* Category 1: Game Control & IPC Bridge */}
                  <div className="space-y-1.5">
                    <span className="text-[10px] font-bold text-cyan-400 uppercase tracking-wider block">
                      🎮 Process Control & Live IPC Bridge (6 Tools)
                    </span>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-cyan-300 font-bold text-[11px]">get_game_status</div>
                        <div className="text-[10px] text-slate-400">Checks if ProjectZomboid64.exe is running and returns its PID.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-cyan-300 font-bold text-[11px]">launch_game</div>
                        <div className="text-[10px] text-slate-400">Launches the game with configurable debugging flags (-debug, -windowed).</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-cyan-300 font-bold text-[11px]">terminate_game</div>
                        <div className="text-[10px] text-slate-400">Closes or forces the termination of the Project Zomboid process.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-cyan-300 font-bold text-[11px]">send_game_ipc_command</div>
                        <div className="text-[10px] text-slate-400">Sends live commands (give_item, eval_lua, godmode) to the companion mod.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-cyan-300 font-bold text-[11px]">get_game_ipc_response</div>
                        <div className="text-[10px] text-slate-400">Reads the execution response emitted by the companion mod in the game.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-cyan-300 font-bold text-[11px]">install_bridge_companion_mod</div>
                        <div className="text-[10px] text-slate-400">Installs the companion mod Z_PZModStudio_Bridge in Zomboid/mods.</div>
                      </div>
                    </div>
                  </div>

                  {/* Category 2: Monitor Center & Diagnostics */}
                  <div className="space-y-1.5">
                    <span className="text-[10px] font-bold text-amber-400 uppercase tracking-wider block">
                      🩺 Monitor Center & Crash Diagnostics (4 Tools)
                    </span>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-amber-300 font-bold text-[11px]">get_monitor_logs</div>
                        <div className="text-[10px] text-slate-400">Reads and filters recent lines from console.txt (errors and stacktraces).</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-amber-300 font-bold text-[11px]">get_crash_diagnostics</div>
                        <div className="text-[10px] text-slate-400">Parses Java/Lua exceptions and generates B41/B42 diagnostics with solutions.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-amber-300 font-bold text-[11px]">list_available_logs</div>
                        <div className="text-[10px] text-slate-400">Lists all session log files on disk (console.txt and Logs/).</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-amber-300 font-bold text-[11px]">read_log_file</div>
                        <div className="text-[10px] text-slate-400">Reads any specific log file with exception filtering.</div>
                      </div>
                    </div>
                  </div>

                  {/* Category 3: Mods, Paths & Profiles */}
                  <div className="space-y-1.5">
                    <span className="text-[10px] font-bold text-emerald-400 uppercase tracking-wider block">
                      📦 Mods, Paths & Profiles (7 Tools)
                    </span>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-emerald-300 font-bold text-[11px]">get_studio_paths</div>
                        <div className="text-[10px] text-slate-400">Auto-detects game paths, Workshop and Zomboid folder.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-emerald-300 font-bold text-[11px]">list_installed_mods</div>
                        <div className="text-[10px] text-slate-400">Scans and lists all Steam Workshop and local mods.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-emerald-300 font-bold text-[11px]">sort_mod_load_order</div>
                        <div className="text-[10px] text-slate-400">Puts mods in an order that satisfies what they require.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-emerald-300 font-bold text-[11px]">scan_mod_conflicts</div>
                        <div className="text-[10px] text-slate-400">Scans VFS collisions between mods and the base game.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-emerald-300 font-bold text-[11px]">list_mod_profiles</div>
                        <div className="text-[10px] text-slate-400">Lists all saved, active mod profiles and order.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-emerald-300 font-bold text-[11px]">create_mod_profile</div>
                        <div className="text-[10px] text-slate-400">Creates a new independent mod combination profile.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-emerald-300 font-bold text-[11px]">activate_mod_profile</div>
                        <div className="text-[10px] text-slate-400">Activates a mod profile in default.txt / ModListData.ini.</div>
                      </div>
                    </div>
                  </div>

                  {/* Category 4: AST Merger & Patch Engine */}
                  <div className="space-y-1.5">
                    <span className="text-[10px] font-bold text-purple-400 uppercase tracking-wider block">
                      ⚡ AST Merge Engine & Patches (5 Tools)
                    </span>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-purple-300 font-bold text-[11px]">validate_lua_syntax</div>
                        <div className="text-[10px] text-slate-400">Validates Lua syntax using AST parser (full_moon) with line and column.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-purple-300 font-bold text-[11px]">merge_lua_scripts</div>
                        <div className="text-[10px] text-slate-400">3-way AST merge between base code and two conflicting variants.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-purple-300 font-bold text-[11px]">list_merged_packages</div>
                        <div className="text-[10px] text-slate-400">Lists all patch and merge packages (Z_PZModStudio_*).</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-purple-300 font-bold text-[11px]">get_master_patch_status</div>
                        <div className="text-[10px] text-slate-400">Queries the Master Patch status and saved resolutions.</div>
                      </div>
                      <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                        <div className="font-mono text-purple-300 font-bold text-[11px]">save_draft_resolution</div>
                        <div className="text-[10px] text-slate-400">Saves a code resolution directly into the patch package.</div>
                      </div>
                    </div>
                  </div>
                </div>

              {/* Exposed Resources (1 col) */}
              <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-4 space-y-3">
                <div className="flex items-center justify-between pb-2 border-b border-slate-800">
                  <h4 className="text-xs font-bold text-slate-200 uppercase tracking-wider flex items-center gap-1.5">
                    <Layers className="w-4 h-4 text-purple-400" />
                    MCP Resources (8 Passive URIs)
                  </h4>
                  <span className="text-[10px] font-mono text-purple-300 font-bold bg-purple-950/60 px-2 py-0.5 rounded border border-purple-800">
                    resources/read
                  </span>
                </div>

                <div className="space-y-2 max-h-[520px] overflow-y-auto pr-1 text-xs">
                  <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                    <div className="font-mono text-purple-300 font-bold text-[11px]">pz://monitor/console-log</div>
                    <div className="text-[10px] text-slate-400">Live text stream of the console.txt file.</div>
                  </div>
                  <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                    <div className="font-mono text-purple-300 font-bold text-[11px]">pz://game/status</div>
                    <div className="text-[10px] text-slate-400">Game execution status, PID and companion bridge.</div>
                  </div>
                  <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                    <div className="font-mono text-purple-300 font-bold text-[11px]">pz://diagnostics/latest-crash</div>
                    <div className="text-[10px] text-slate-400">Parsed diagnostic of the last crash and stacktraces.</div>
                  </div>
                  <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                    <div className="font-mono text-purple-300 font-bold text-[11px]">pz://mods/installed-summary</div>
                    <div className="text-[10px] text-slate-400">JSON snapshot of all installed mods and order.</div>
                  </div>
                  <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                    <div className="font-mono text-purple-300 font-bold text-[11px]">pz://profiles/list</div>
                    <div className="text-[10px] text-slate-400">Complete list of saved and active mod profiles.</div>
                  </div>
                  <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                    <div className="font-mono text-purple-300 font-bold text-[11px]">pz://conflicts/active</div>
                    <div className="text-[10px] text-slate-400">Snapshot of VFS collisions between active mods.</div>
                  </div>
                  <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                    <div className="font-mono text-purple-300 font-bold text-[11px]">pz://patches/status</div>
                    <div className="text-[10px] text-slate-400">Status and resolutions of Z_PZModStudio_MasterPatch.</div>
                  </div>
                  <div className="p-2 rounded-lg bg-slate-950 border border-slate-800 space-y-0.5">
                    <div className="font-mono text-purple-300 font-bold text-[11px]">pz://paths/config</div>
                    <div className="text-[10px] text-slate-400">Detected system paths and configuration.</div>
                  </div>
                </div>
              </div>
            </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
