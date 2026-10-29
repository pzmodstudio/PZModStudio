import React, { useState, useRef, useEffect } from 'react';
import { Shield, Play, Activity, ChevronDown, Layout, Zap, Check, Layers, Crosshair, Home } from 'lucide-react';
import { GameLaunchMode, GameInstall, LaunchTarget } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { CustomLaunchDialog } from './CustomLaunchDialog';
import { useTranslation } from 'react-i18next';

interface StudioHeaderProps {
  /** Conflicts still needing a decision. */
  conflictCount: number;
  /** Conflicts already resolved or merged automatically. */
  resolvedConflictCount?: number;
  onNavigateToMerger?: () => void;
  activeProfileName?: string;
  /** The game build installed now, from Zomboid/version.txt ("42.21"). */
  installedBuild?: string | null;
  /** The build the active profile was last played on from the Studio. */
  profileBuild?: string | null;
  /** The major build the active profile is set for by hand, when it is. */
  profileTarget?: string | null;
  /** Steam's view of the install: the branch and when it last updated. */
  install?: GameInstall | null;
  onNavigateToProfiles?: () => void;
  onRunSandbox: (mode: GameLaunchMode, target: LaunchTarget) => void;
  /** For the Custom launch's lists of saves and servers. */
  paths?: StudioPathsUI;
  activeProfileId?: string | null;
}

/** The modes; their words live in the locale files under header.modes.<id>. */
const LAUNCH_MODES: { id: GameLaunchMode; badgeColor: string; category: 'DEBUG' | 'NORMAL' }[] = [
  { id: 'DEBUG_FULLSCREEN', badgeColor: 'bg-emerald-950 text-emerald-300 border-emerald-800/80', category: 'DEBUG' },
  { id: 'DEBUG_WINDOWED', badgeColor: 'bg-cyan-950 text-cyan-300 border-cyan-800/80', category: 'DEBUG' },
  { id: 'NORMAL_FULLSCREEN', badgeColor: 'bg-amber-950 text-amber-300 border-amber-800/80', category: 'NORMAL' },
  { id: 'NORMAL_WINDOWED', badgeColor: 'bg-purple-950 text-purple-300 border-purple-800/80', category: 'NORMAL' },
];

export const StudioHeader: React.FC<StudioHeaderProps> = ({
  conflictCount,
  resolvedConflictCount = 0,
  onNavigateToMerger,
  activeProfileName,
  installedBuild = null,
  profileBuild = null,
  profileTarget = null,
  install = null,
  onNavigateToProfiles,
  onRunSandbox,
  paths,
  activeProfileId = null,
}) => {
  const { t } = useTranslation();
  const modeText = (id: GameLaunchMode) => ({
    title: t(`header.modes.${id}.title`),
    badge: t(`header.modes.${id}.badge`),
    description: t(`header.modes.${id}.description`),
    buttonLabel: t(`header.modes.${id}.button`),
  });
  // The launch mode last used, when the player wants it remembered (Settings; on by default).
  // Without it every start of the Studio went back to Debug and the player changed it each time.
  const [selectedMode, setSelectedModeRaw] = useState<GameLaunchMode>(() => {
    if (localStorage.getItem('pz_remember_launch_mode') === 'false') return 'DEBUG_FULLSCREEN';
    const saved = localStorage.getItem('pz_last_launch_mode') as GameLaunchMode | null;
    return saved && LAUNCH_MODES.some((m) => m.id === saved) ? saved : 'DEBUG_FULLSCREEN';
  });
  const setSelectedMode = (mode: GameLaunchMode) => {
    setSelectedModeRaw(mode);
    localStorage.setItem('pz_last_launch_mode', mode);
  };
  // Where the game goes once it starts: the main menu, one save or one server ("Custom"),
  // remembered like the mode.
  const [target, setTargetRaw] = useState<LaunchTarget>(() => {
    if (localStorage.getItem('pz_remember_launch_mode') === 'false') return { kind: 'menu' };
    try {
      const saved = JSON.parse(localStorage.getItem('pz_launch_target') ?? 'null') as LaunchTarget | null;
      if (saved && (saved.kind === 'save' || saved.kind === 'server' || saved.kind === 'menu')) return saved;
    } catch {
      // a broken value: the main menu
    }
    return { kind: 'menu' };
  });
  const setTarget = (next: LaunchTarget) => {
    setTargetRaw(next);
    try {
      localStorage.setItem('pz_launch_target', JSON.stringify(next));
    } catch {
      // only a convenience
    }
  };
  const [customOpen, setCustomOpen] = useState(false);
  const targetName = target.kind === 'save' ? target.label : target.kind === 'server' ? target.name : '';
  const [isDropdownOpen, setIsDropdownOpen] = useState<boolean>(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Close dropdown on outside click
  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setIsDropdownOpen(false);
      }
    };
    if (isDropdownOpen) {
      document.addEventListener('mousedown', handleOutsideClick);
    }
    return () => {
      document.removeEventListener('mousedown', handleOutsideClick);
    };
  }, [isDropdownOpen]);

  const activeModeConfig = modeText(selectedMode);

  const handleSelectMode = (mode: GameLaunchMode, executeImmediately = false) => {
    setSelectedMode(mode);
    setIsDropdownOpen(false);
    if (executeImmediately) {
      onRunSandbox(mode, target);
    }
  };

  return (
    <header className="h-14 bg-slate-900 border-b border-slate-800 px-4 flex items-center justify-between select-none shrink-0 shadow-md relative z-40">
      {/* App Branding & Logo */}
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 rounded-lg bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shadow-inner">
          <Shield className="w-5 h-5" />
        </div>
        <div>
          <h1 className="text-sm font-bold text-slate-100 leading-none flex items-center gap-2">
            PZ Mod Studio
            <span className="text-[10px] font-normal text-emerald-400 bg-emerald-950/60 border border-emerald-800/80 px-1.5 py-0.5 rounded font-mono">
              v{__APP_VERSION__}
            </span>
          </h1>
          <p className="text-[11px] text-slate-400 leading-tight mt-0.5 font-mono">
            {t('header.tagline')}
          </p>
        </div>
      </div>

      {/* Center Status Indicators */}
      <div className="hidden md:flex items-center gap-3 text-xs">
        {/* Active Profile Badge */}
        <button
          onClick={onNavigateToProfiles}
          className="flex items-center gap-2 px-3 py-1 rounded-full bg-slate-950 border border-slate-800 hover:border-emerald-600/60 transition cursor-pointer group shadow-inner"
          title={t('header.profileTitle')}
        >
          <Layers className="w-3.5 h-3.5 text-emerald-400 group-hover:scale-110 transition-transform" />
          <span className="text-slate-400 font-mono text-[11px]">{t('header.profile')}</span>
          <span className="font-bold font-mono text-emerald-300 group-hover:text-emerald-200 truncate max-w-[150px]">
            {activeProfileName || t('header.noProfile')}
          </span>
        </button>

        {/* The installed build, and whether this profile was last played on it: saves do not cross builds. */}
        {installedBuild && (() => {
          const major = (v: string) => v.split('.')[0];
          // A build set by hand speaks for the profile; else the one it was last played on.
          const targetCrossed = !!profileTarget && profileTarget !== major(installedBuild);
          const other = targetCrossed || (!profileTarget && profileBuild && profileBuild !== installedBuild);
          const crossed = targetCrossed || (!profileTarget && other && major(profileBuild!) !== major(installedBuild));
          return (
            <span
              className={`flex items-center gap-1.5 px-3 py-1 rounded-full border font-mono text-[11px] shadow-inner ${
                crossed
                  ? 'bg-red-950/60 border-red-700/70 text-red-200'
                  : other
                  ? 'bg-amber-950/50 border-amber-700/60 text-amber-200'
                  : 'bg-slate-950 border-slate-800 text-slate-300'
              }`}
              title={(() => {
                const steam = install ? t('header.steamBranch', { branch: install.branch, updated: install.last_updated ? t('header.steamUpdated', { date: install.last_updated }) : '' }) : '';
                if (targetCrossed) return t('header.buildTargetCrossed', { target: profileTarget, installed: installedBuild });
                if (crossed) return t('header.buildCrossed', { played: profileBuild, installed: installedBuild, playedMajor: major(profileBuild!), installedMajor: major(installedBuild) });
                if (other) return t('header.buildOther', { played: profileBuild, installed: installedBuild });
                if (profileBuild) return t('header.buildSame', { installed: installedBuild, steam });
                return t('header.buildUnknown', { installed: installedBuild, steam });
              })()}
            >
              <span className="text-slate-400">{t('header.build')}</span>
              <span className="font-bold">{installedBuild}</span>
              {install && install.branch !== 'stable' && <span className="text-[10px] text-cyan-300">{install.branch}</span>}
              {other && <span className="text-[10px]">{t('header.profileOn', { build: profileBuild })}</span>}
            </span>
          );
        })()}

        <button
          type="button"
          onClick={onNavigateToMerger}
          className="flex items-center gap-2 px-3 py-1 rounded-full bg-slate-950 border border-slate-800 hover:border-emerald-600/60 transition cursor-pointer"
          title={t('header.conflictsTitle', { pending: conflictCount, resolved: resolvedConflictCount })}
        >
          <span className={`w-2 h-2 rounded-full ${conflictCount > 0 ? 'bg-amber-400 animate-pulse' : 'bg-emerald-400'}`}></span>
          <span className="text-slate-400 font-mono">{t('header.conflicts')}</span>
          <span className={`font-bold font-mono ${conflictCount > 0 ? 'text-amber-400' : 'text-emerald-300'}`}>{t('header.pending', { count: conflictCount })}</span>
          {resolvedConflictCount > 0 && <span className="font-mono text-slate-500">{t('header.resolved', { count: resolvedConflictCount })}</span>}
        </button>

      </div>

      {/* Right Actions: Launch Sandbox */}
      <div className="flex items-center gap-2.5">
        {/* Launch Game with Dropdown */}
        <div ref={dropdownRef} className="relative flex items-center">
          <div className="flex items-center shadow-lg rounded-lg overflow-hidden border border-emerald-600/40">
            {/* Main Action Button */}
            <button
              onClick={() => onRunSandbox(selectedMode, target)}
              className="flex items-center gap-2 px-3.5 py-2 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white text-xs font-bold transition cursor-pointer"
              title={target.kind === 'menu' ? t('header.executeIn', { mode: activeModeConfig.title }) : t('header.target.title', { name: targetName, mode: activeModeConfig.title })}
            >
              <Play className="w-3.5 h-3.5 fill-current" />
              <span className="max-w-[260px] truncate">{target.kind === 'menu' ? activeModeConfig.buttonLabel : t(`header.target.${target.kind}Button`, { name: targetName })}</span>
              {(selectedMode === 'DEBUG_FULLSCREEN' || selectedMode === 'MONITORED') && (
                <Activity className="w-3.5 h-3.5 text-emerald-200 animate-pulse" />
              )}
              {(selectedMode === 'DEBUG_WINDOWED' || selectedMode === 'WINDOWED') && (
                <Layout className="w-3.5 h-3.5 text-cyan-200" />
              )}
              {selectedMode === 'NORMAL_FULLSCREEN' && (
                <Zap className="w-3.5 h-3.5 text-amber-200" />
              )}
              {(selectedMode === 'NORMAL_WINDOWED' || selectedMode === 'NORMAL') && (
                <Layout className="w-3.5 h-3.5 text-purple-200" />
              )}
            </button>

            {/* Dropdown Chevron Button */}
            <button
              onClick={() => setIsDropdownOpen((prev) => !prev)}
              className="px-2 py-2 bg-teal-700 hover:bg-teal-600 text-teal-100 transition cursor-pointer border-l border-teal-600/50 flex items-center justify-center"
              title={t('header.selectMode')}
            >
              <ChevronDown className={`w-3.5 h-3.5 transition-transform ${isDropdownOpen ? 'rotate-180' : ''}`} />
            </button>
          </div>

          {/* Dropdown Menu Popup */}
          {isDropdownOpen && (
            <div className="absolute right-0 top-full mt-2 w-88 bg-slate-900/95 border border-slate-700/80 rounded-xl shadow-2xl p-1.5 z-50 backdrop-blur-md space-y-1 animate-fade-in">
              <div className="px-2.5 py-1.5 text-[11px] font-bold text-slate-400 border-b border-slate-800 flex items-center justify-between">
                <span>{t('header.launchModes')}</span>
                <span className="text-[10px] font-mono text-slate-500">Project Zomboid</span>
              </div>

              <div className="px-2.5 pt-1 text-[10px] font-bold text-emerald-400 uppercase tracking-wider font-mono">
                {t('header.debugModes')}
              </div>

              {LAUNCH_MODES.filter((m) => m.category === 'DEBUG').map((mode) => {
                const isSelected = selectedMode === mode.id;
                const words = modeText(mode.id);
                return (
                  <button
                    key={mode.id}
                    onClick={() => handleSelectMode(mode.id)}
                    className={`w-full text-left p-2.5 rounded-lg transition cursor-pointer flex items-start gap-2.5 ${
                      isSelected
                        ? 'bg-slate-800/90 border border-emerald-500/40 text-slate-100 shadow-inner'
                        : 'hover:bg-slate-800/50 text-slate-300 border border-transparent'
                    }`}
                  >
                    <div className="mt-0.5 shrink-0">
                      {mode.id === 'DEBUG_FULLSCREEN' && (
                        <div className="w-6 h-6 rounded-md bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
                          <Activity className="w-3.5 h-3.5" />
                        </div>
                      )}
                      {mode.id === 'DEBUG_WINDOWED' && (
                        <div className="w-6 h-6 rounded-md bg-cyan-500/10 border border-cyan-500/30 flex items-center justify-center text-cyan-400">
                          <Layout className="w-3.5 h-3.5" />
                        </div>
                      )}
                    </div>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between gap-1 mb-0.5">
                        <span className="text-xs font-bold text-slate-200 truncate">{words.title}</span>
                        <span className={`text-[10px] font-mono px-1.5 py-0.5 rounded border shrink-0 ${mode.badgeColor}`}>
                          {words.badge}
                        </span>
                      </div>
                      <p className="text-[10px] text-slate-400 leading-tight">{words.description}</p>
                    </div>

                    {isSelected && <Check className="w-4 h-4 text-emerald-400 shrink-0 mt-1" />}
                  </button>
                );
              })}

              <div className="px-2.5 pt-2 text-[10px] font-bold text-amber-400 uppercase tracking-wider font-mono border-t border-slate-800/80">
                {t('header.normalModes')}
              </div>

              {LAUNCH_MODES.filter((m) => m.category === 'NORMAL').map((mode) => {
                const isSelected = selectedMode === mode.id;
                const words = modeText(mode.id);
                return (
                  <button
                    key={mode.id}
                    onClick={() => handleSelectMode(mode.id)}
                    className={`w-full text-left p-2.5 rounded-lg transition cursor-pointer flex items-start gap-2.5 ${
                      isSelected
                        ? 'bg-slate-800/90 border border-emerald-500/40 text-slate-100 shadow-inner'
                        : 'hover:bg-slate-800/50 text-slate-300 border border-transparent'
                    }`}
                  >
                    <div className="mt-0.5 shrink-0">
                      {mode.id === 'NORMAL_FULLSCREEN' && (
                        <div className="w-6 h-6 rounded-md bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-amber-400">
                          <Zap className="w-3.5 h-3.5" />
                        </div>
                      )}
                      {mode.id === 'NORMAL_WINDOWED' && (
                        <div className="w-6 h-6 rounded-md bg-purple-500/10 border border-purple-500/30 flex items-center justify-center text-purple-400">
                          <Layout className="w-3.5 h-3.5" />
                        </div>
                      )}
                    </div>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between gap-1 mb-0.5">
                        <span className="text-xs font-bold text-slate-200 truncate">{words.title}</span>
                        <span className={`text-[10px] font-mono px-1.5 py-0.5 rounded border shrink-0 ${mode.badgeColor}`}>
                          {words.badge}
                        </span>
                      </div>
                      <p className="text-[10px] text-slate-400 leading-tight">{words.description}</p>
                    </div>

                    {isSelected && <Check className="w-4 h-4 text-emerald-400 shrink-0 mt-1" />}
                  </button>
                );
              })}

              <div className="px-2.5 pt-2 pb-1 text-[10px] font-bold text-sky-400 uppercase tracking-wider font-mono border-t border-slate-800/80 mt-1">
                {t('header.target.section')}
              </div>
              <button
                type="button"
                onClick={() => {
                  setTarget({ kind: 'menu' });
                  setIsDropdownOpen(false);
                }}
                className={`w-full text-left p-2.5 rounded-lg transition flex items-center gap-3 cursor-pointer border ${target.kind === 'menu' ? 'bg-slate-800/80 border-emerald-700/60' : 'hover:bg-slate-800/50 border-transparent'}`}
              >
                <div className="w-8 h-8 rounded-md flex items-center justify-center shrink-0 bg-slate-950 border border-slate-700 text-slate-300">
                  <Home className="w-4 h-4" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-bold text-slate-200">{t('header.target.menu')}</div>
                  <p className="text-[10px] text-slate-400 leading-tight">{t('header.target.menuBlurb')}</p>
                </div>
                {target.kind === 'menu' && <Check className="w-4 h-4 text-emerald-400 shrink-0" />}
              </button>
              <button
                type="button"
                onClick={() => {
                  setIsDropdownOpen(false);
                  setCustomOpen(true);
                }}
                disabled={!paths}
                className={`w-full text-left p-2.5 rounded-lg transition flex items-center gap-3 cursor-pointer border ${target.kind !== 'menu' ? 'bg-slate-800/80 border-emerald-700/60' : 'hover:bg-slate-800/50 border-transparent'}`}
              >
                <div className="w-8 h-8 rounded-md flex items-center justify-center shrink-0 bg-sky-950 border border-sky-800 text-sky-300">
                  <Crosshair className="w-4 h-4" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-bold text-slate-200">{t('header.target.custom')}</div>
                  <p className="text-[10px] text-slate-400 leading-tight truncate">
                    {target.kind === 'menu' ? t('header.target.customBlurb') : t(`header.target.${target.kind}Now`, { name: targetName })}
                  </p>
                </div>
                {target.kind !== 'menu' && <Check className="w-4 h-4 text-emerald-400 shrink-0" />}
              </button>
            </div>
          )}
          {customOpen && paths && (
            <CustomLaunchDialog
              paths={paths}
              activeProfileId={activeProfileId}
              initial={target}
              modes={LAUNCH_MODES.map((m) => m.id)}
              mode={selectedMode}
              onCancel={() => setCustomOpen(false)}
              onConfirm={(next, mode, start) => {
                setCustomOpen(false);
                setTarget(next);
                setSelectedMode(mode);
                if (start) onRunSandbox(mode, next);
              }}
            />
          )}
        </div>
      </div>
    </header>
  );
};
