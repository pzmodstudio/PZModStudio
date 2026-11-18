import React, { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { AlertCircle, Archive, CheckCircle2, Globe, Layers, ListOrdered, Loader2, Play, RotateCcw, Stethoscope } from 'lucide-react';
import { ActiveTab, AppProfile, GameLaunchMode, HealthReport, LaunchTarget, ModInfo, SaveSummary } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { healthVerdict } from '../../utils/healthVerdict';
import { launchModeFor, setUiMode } from '../../utils/uiMode';

interface SimpleHomeProps {
  paths: StudioPathsUI;
  profile: AppProfile;
  mods: ModInfo[];
  report: HealthReport | null;
  checking: boolean;
  onPlay: (mode: GameLaunchMode, target: LaunchTarget) => void;
  onGo: (tab: ActiveTab) => void;
}

/** The mode the header's launch button is on (the same rule), so both buttons start the game alike. */
const launchMode = (): GameLaunchMode => launchModeFor('simple');

/**
 * Simple mode's front page: the profile in use, Play (and Continue the last game played with it),
 * and the Doctor's verdict in one line: nothing to do, or what to fix first.
 */
export const SimpleHome: React.FC<SimpleHomeProps> = ({ paths, profile, mods, report, checking, onPlay, onGo }) => {
  const { t } = useTranslation();
  const [lastSave, setLastSave] = useState<SaveSummary | null>(null);

  // The newest single-player save played with this profile: what Continue opens.
  useEffect(() => {
    let alive = true;
    void Promise.all([TauriService.savesOverview(paths), TauriService.saveSessionDiffs(paths)]).then(([overview, diffs]) => {
      if (!alive) return;
      const mine = (overview?.saves ?? [])
        .filter((s) => s.mode !== 'Multiplayer' && diffs[s.id]?.profile === profile.id)
        .sort((a, b) => b.modified - a.modified);
      setLastSave(mine[0] ?? null);
    });
    return () => {
      alive = false;
    };
  }, [paths, profile.id]);

  const onCount = mods.filter((m) => m.enabled && !m.mod_id.startsWith('Z_PZModStudio_')).length;
  const verdict = useMemo(() => (report ? healthVerdict(report.issues) : null), [report]);

  const doctor = (() => {
    if (!verdict) {
      return (
        <div className="flex items-center gap-2 text-sm text-slate-300">
          <Loader2 className="w-4 h-4 animate-spin text-sky-300" aria-hidden="true" /> {t('simple.home.checking')}
        </div>
      );
    }
    if (verdict.level === 'act') {
      return (
        <div className="flex flex-wrap items-center gap-3">
          <AlertCircle className="w-6 h-6 text-red-400 shrink-0" aria-hidden="true" />
          <div className="flex-1 min-w-[220px]">
            <div className="text-sm font-bold text-red-200">{t('simple.home.act', { count: verdict.counts.act })}</div>
            <div className="text-[11px] text-slate-400">{t('simple.home.actHint')}</div>
          </div>
          <button
            type="button"
            onClick={() => onGo('HEALTH')}
            className="px-3 py-2 rounded-lg text-xs font-bold bg-red-600 hover:bg-red-500 text-white cursor-pointer"
          >
            {t('simple.home.seeWhat')}
          </button>
        </div>
      );
    }
    return (
      <div className="flex flex-wrap items-center gap-3">
        <CheckCircle2 className="w-6 h-6 text-emerald-400 shrink-0" aria-hidden="true" />
        <div className="flex-1 min-w-[220px]">
          <div className="text-sm font-bold text-emerald-200">{t('simple.home.ok')}</div>
          <div className="text-[11px] text-slate-400">
            {verdict.counts.recommended > 0 ? t('simple.home.optional', { count: verdict.counts.recommended }) : t('simple.home.okHint')}
          </div>
        </div>
        {checking && <Loader2 className="w-4 h-4 animate-spin text-sky-300" aria-label={t('simple.home.checking')} />}
      </div>
    );
  })();

  const shortcut = (tab: ActiveTab, Icon: React.ElementType, label: string, hint: string) => (
    <button
      key={tab}
      type="button"
      onClick={() => onGo(tab)}
      className="flex items-start gap-3 p-3 rounded-xl border border-slate-800 bg-slate-900/60 hover:border-emerald-700/70 hover:bg-slate-900 text-left cursor-pointer"
    >
      <Icon className="w-5 h-5 text-emerald-400 shrink-0 mt-0.5" aria-hidden="true" />
      <span className="min-w-0">
        <span className="block text-xs font-bold text-slate-100">{label}</span>
        <span className="block text-[11px] text-slate-400">{hint}</span>
      </span>
    </button>
  );

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-y-auto p-6 font-sans">
      <div className="max-w-3xl w-full mx-auto flex flex-col gap-4">
        <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-5 flex flex-col gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <Layers className="w-6 h-6 text-emerald-400 shrink-0" aria-hidden="true" />
            <div className="flex-1 min-w-0">
              <div className="text-[11px] uppercase tracking-wider text-slate-500 font-bold">{t('simple.home.profile')}</div>
              <div className="text-lg font-bold text-slate-100 truncate">{profile.name}</div>
              <div className="text-[11px] text-slate-400">{t('simple.home.modsOn', { count: onCount })}</div>
            </div>
            <button
              type="button"
              onClick={() => onGo('PROFILES')}
              className="px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 cursor-pointer"
            >
              {t('simple.home.changeProfile')}
            </button>
          </div>
          <div className="flex flex-wrap gap-3">
            <button
              type="button"
              onClick={() => onPlay(launchMode(), { kind: 'menu' })}
              className="flex items-center gap-2 px-6 py-3 rounded-xl text-sm font-bold bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white shadow-lg cursor-pointer"
            >
              <Play className="w-4 h-4 fill-current" aria-hidden="true" /> {t('simple.home.play')}
            </button>
            {lastSave && (
              <button
                type="button"
                onClick={() => onPlay(launchMode(), { kind: 'save', id: lastSave.id, label: lastSave.world })}
                title={t('simple.home.continueTitle', { world: lastSave.world })}
                className="flex items-center gap-2 px-5 py-3 rounded-xl text-sm font-bold bg-slate-800 hover:bg-slate-700 text-slate-100 border border-slate-700 cursor-pointer max-w-full"
              >
                <RotateCcw className="w-4 h-4 shrink-0" aria-hidden="true" />
                <span className="truncate">{t('simple.home.continue', { world: lastSave.world })}</span>
              </button>
            )}
          </div>
        </div>

        <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-4 flex flex-col gap-2">
          <div className="flex items-center gap-2 text-[11px] uppercase tracking-wider text-slate-500 font-bold">
            <Stethoscope className="w-3.5 h-3.5" aria-hidden="true" /> {t('simple.home.doctor')}
          </div>
          {doctor}
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {shortcut('MOD_LIST', ListOrdered, t('nav.modList'), t('simple.home.modsHint'))}
          {shortcut('BROWSE', Globe, t('nav.browse'), t('simple.home.browseHint'))}
          {shortcut('SAVES', Archive, t('nav.saves'), t('simple.home.savesHint'))}
          {shortcut('PROFILES', Layers, t('nav.profiles'), t('simple.home.profilesHint'))}
        </div>

        <div className="flex flex-wrap items-center gap-2 text-[11px] text-slate-500">
          <span>{t('simple.home.moreInComplete')}</span>
          <button type="button" onClick={() => setUiMode('advanced')} className="underline text-slate-300 hover:text-slate-100 cursor-pointer">
            {t('simple.switchToComplete')}
          </button>
        </div>
      </div>
    </div>
  );
};
