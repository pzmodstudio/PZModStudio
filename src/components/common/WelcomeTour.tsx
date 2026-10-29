import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Activity,
  Archive,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Code2,
  FolderSearch,
  GitCompare,
  Globe,
  HeartPulse,
  HelpCircle,
  Layers,
  ListOrdered,
  Map as MapIcon,
  Newspaper,
  Puzzle,
  Server,
  Settings,
  Sparkles,
  X,
  XCircle,
} from 'lucide-react';
import { ActiveTab, HealthReport } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { LANGUAGES, setLanguage } from '../../i18n';

/** Seen once: the tour does not open by itself again (Help can open it). */
export const WELCOME_DONE_KEY = 'pz_welcome_done';
/** Anything can open the tour again by sending this event on window. */
export const SHOW_WELCOME_EVENT = 'pzms-show-welcome';

export const welcomeSeen = () => {
  try {
    return localStorage.getItem(WELCOME_DONE_KEY) === '1';
  } catch {
    return false;
  }
};

interface Step {
  key: string;
  icon: React.ElementType;
  /** The screen shown behind the card while this step is open. */
  tab?: ActiveTab;
}

const STEPS: Step[] = [
  { key: 'welcome', icon: Sparkles },
  { key: 'folders', icon: FolderSearch, tab: 'SETTINGS' },
  { key: 'profiles', icon: Layers, tab: 'PROFILES' },
  { key: 'modList', icon: ListOrdered, tab: 'MOD_LIST' },
  { key: 'health', icon: HeartPulse, tab: 'HEALTH' },
  { key: 'monitor', icon: Activity, tab: 'MONITOR' },
  { key: 'saves', icon: Archive, tab: 'SAVES' },
  { key: 'whatsNew', icon: Newspaper, tab: 'WHATS_NEW' },
  { key: 'browse', icon: Globe, tab: 'BROWSE' },
  { key: 'maps', icon: MapIcon, tab: 'MAPS' },
  { key: 'merger', icon: GitCompare, tab: 'MERGER' },
  { key: 'servers', icon: Server, tab: 'SERVERS' },
  { key: 'link', icon: Puzzle },
  { key: 'settings', icon: Settings, tab: 'SETTINGS' },
  { key: 'developer', icon: Code2 },
  { key: 'help', icon: HelpCircle },
];

interface WelcomeTourProps {
  open: boolean;
  onClose: () => void;
  paths: StudioPathsUI;
  /** The active profile's name, if one is active. */
  activeProfileName: string | null;
  profileCount: number;
  installedMods: number;
  health: HealthReport | null;
  /** Shows a screen behind the card (only screens open without a profile, when none is active). */
  onShowTab: (tab: ActiveTab) => void;
  /** The screen this step is about, for the sidebar to light up (null: none). */
  onHighlight: (tab: ActiveTab | null) => void;
}

/** Screens open without an active profile (the sidebar locks the rest). */
const OPEN_WITHOUT_PROFILE: ActiveTab[] = ['PROFILES', 'SETTINGS'];

/**
 * The first-run tour: one card per part of the Studio, the screen it is about shown behind it and
 * lit up in the sidebar, with what the Studio already knows about this install (the folders it
 * found, the profiles, the problems Mod Health sees). It opens once by itself; Help opens it again.
 */
export const WelcomeTour: React.FC<WelcomeTourProps> = ({ open, onClose, paths, activeProfileName, profileCount, installedMods, health, onShowTab, onHighlight }) => {
  const { t, i18n } = useTranslation();
  const [index, setIndex] = useState(0);
  const step = STEPS[index];
  const reachable = (tab?: ActiveTab) => !!tab && (!!activeProfileName || OPEN_WITHOUT_PROFILE.includes(tab));

  useEffect(() => {
    if (open) setIndex(0);
  }, [open]);

  useEffect(() => {
    if (!open) {
      onHighlight(null);
      return;
    }
    onHighlight(step.tab && step.tab !== 'SETTINGS' ? step.tab : null);
    if (reachable(step.tab)) onShowTab(step.tab as ActiveTab);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, index, activeProfileName]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'ArrowRight') setIndex((i) => Math.min(STEPS.length - 1, i + 1));
      else if (e.key === 'ArrowLeft') setIndex((i) => Math.max(0, i - 1));
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open]);

  if (!open) return null;

  const finish = () => {
    try {
      localStorage.setItem(WELCOME_DONE_KEY, '1');
    } catch {
      // seen again next time: no harm
    }
    onClose();
  };

  const Icon = step.icon;
  const points = t(`welcome.steps.${step.key}.points`, { returnObjects: true }) as unknown;
  const list = Array.isArray(points) ? (points as string[]) : [];
  const last = index === STEPS.length - 1;

  const pathRow = (label: string, value: string) => (
    <div className="flex items-start gap-2 text-[11px]">
      {value ? <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 shrink-0 mt-0.5" aria-hidden="true" /> : <XCircle className="w-3.5 h-3.5 text-red-400 shrink-0 mt-0.5" aria-hidden="true" />}
      <div className="min-w-0">
        <div className="text-slate-300">{label}</div>
        <div className={`font-mono truncate ${value ? 'text-slate-500' : 'text-red-300'}`} title={value}>
          {value || t('welcome.notFound')}
        </div>
      </div>
    </div>
  );

  /** What the Studio already knows for this step, in a box under the text. */
  const live = (() => {
    switch (step.key) {
      case 'welcome':
        return (
          <div className="flex items-center gap-2 text-[11px]">
            <span className="text-slate-400">{t('welcome.language')}</span>
            {LANGUAGES.map((l) => (
              <button
                key={l.code}
                type="button"
                onClick={() => void setLanguage(l.code)}
                className={`px-2 py-0.5 rounded border cursor-pointer ${i18n.language.startsWith(l.code) ? 'border-emerald-600 text-emerald-200 bg-emerald-950/40' : 'border-slate-700 text-slate-300 hover:bg-slate-800'}`}
              >
                {l.name}
              </button>
            ))}
          </div>
        );
      case 'folders':
        return (
          <div className="flex flex-col gap-1.5">
            {pathRow(t('welcome.paths.game'), paths.pz_install_dir)}
            {pathRow(t('welcome.paths.workshop'), paths.workshop_dir)}
            {pathRow(t('welcome.paths.zomboid'), paths.user_zomboid_dir)}
            <div className="text-[11px] text-slate-400">{t('welcome.paths.mods', { count: installedMods })}</div>
          </div>
        );
      case 'profiles':
        return (
          <div className={`text-[11px] ${activeProfileName ? 'text-emerald-300' : 'text-amber-300'}`}>
            {activeProfileName ? t('welcome.profileActive', { name: activeProfileName, count: profileCount }) : t('welcome.profileNone')}
          </div>
        );
      case 'health':
        if (!health) return null;
        return (
          <div className={`text-[11px] ${health.errors > 0 ? 'text-red-300' : health.warnings > 0 ? 'text-amber-300' : 'text-emerald-300'}`}>
            {health.errors + health.warnings === 0
              ? t('welcome.healthClean', { count: health.active_count })
              : t('welcome.healthFound', { errors: health.errors, warnings: health.warnings, count: health.active_count })}
          </div>
        );
      default:
        return null;
    }
  })();

  const lockedHere = !!step.tab && !reachable(step.tab);

  return (
    <div
      className="fixed bottom-16 right-4 z-[60] w-[440px] max-w-[calc(100vw-2rem)] max-h-[calc(100vh-6rem)] flex flex-col rounded-xl border border-emerald-700/60 bg-slate-950/98 shadow-2xl"
      role="dialog"
      aria-label={t('welcome.title')}
    >
      <div className="flex items-center justify-between gap-2 px-4 pt-3">
        <div className="text-[10px] font-mono uppercase tracking-wider text-emerald-400">
          {t('welcome.title')} · {t('welcome.progress', { n: index + 1, total: STEPS.length })}
        </div>
        <button type="button" onClick={finish} className="p-1 rounded hover:bg-slate-800 text-slate-400 cursor-pointer" aria-label={t('welcome.skip')} title={t('welcome.skip')}>
          <X className="w-4 h-4" aria-hidden="true" />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-4 pb-3 pt-2 flex flex-col gap-2.5">
        <div className="flex items-start gap-3">
          <Icon className="w-6 h-6 text-emerald-400 shrink-0 mt-0.5" aria-hidden="true" />
          <div>
            <h2 className="text-sm font-bold text-slate-100">{t(`welcome.steps.${step.key}.title`)}</h2>
            <p className="text-xs text-slate-300 mt-1">{t(`welcome.steps.${step.key}.body`)}</p>
          </div>
        </div>
        {list.length > 0 && (
          <ul className="flex flex-col gap-1 pl-9 text-[11px] text-slate-400 list-disc marker:text-emerald-600">
            {list.map((p, i) => (
              <li key={i} className="ml-3">
                {p}
              </li>
            ))}
          </ul>
        )}
        {live && <div className="ml-9 rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2">{live}</div>}
        {lockedHere && <div className="ml-9 text-[11px] text-amber-300">{t('welcome.lockedHere')}</div>}
      </div>

      <div className="flex items-center gap-2 px-4 py-2.5 border-t border-slate-800">
        <div className="flex gap-1 flex-1 flex-wrap">
          {STEPS.map((s, i) => (
            <button
              key={s.key}
              type="button"
              onClick={() => setIndex(i)}
              className={`w-2 h-2 rounded-full cursor-pointer ${i === index ? 'bg-emerald-400' : i < index ? 'bg-emerald-800' : 'bg-slate-700'}`}
              aria-label={t(`welcome.steps.${s.key}.title`)}
              title={t(`welcome.steps.${s.key}.title`)}
            />
          ))}
        </div>
        <button
          type="button"
          onClick={() => setIndex((i) => Math.max(0, i - 1))}
          disabled={index === 0}
          className="flex items-center gap-1 px-2.5 py-1.5 rounded-md text-xs text-slate-300 hover:bg-slate-800 cursor-pointer disabled:opacity-30 disabled:cursor-default"
        >
          <ChevronLeft className="w-3.5 h-3.5" aria-hidden="true" />
          {t('welcome.back')}
        </button>
        <button
          type="button"
          onClick={() => (last ? finish() : setIndex((i) => i + 1))}
          className="flex items-center gap-1 px-3 py-1.5 rounded-md text-xs font-bold bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer"
        >
          {last ? t('welcome.done') : t('welcome.next')}
          {!last && <ChevronRight className="w-3.5 h-3.5" aria-hidden="true" />}
        </button>
      </div>
    </div>
  );
};
