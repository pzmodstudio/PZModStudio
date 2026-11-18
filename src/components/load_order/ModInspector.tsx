import React, { useEffect, useMemo, useRef, useState } from 'react';
import { AuthorLink, CatalogModSummary, HealthIssue, IssueState, ModChangelog, ModErrorStat, ModInfo, ModJavaPatches } from '../../types';
import { SourceBadge } from '../common/SourceBadge';
import { ModWorkshopPage } from './ModWorkshopPage';
import { ModBrainSection } from './ModBrainSection';
import { FixContext, IssueCards } from './RowHealth';
import { STATE_ORDER, STATE_STYLE, issueStateOf } from '../../utils/issueState';
import { SupportLinks } from '../common/SupportLinks';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { findInstalledDependencyInMods } from './modLookup';
import { Activity, BookOpen, Check, ChevronRight, Info, Layers, MessageSquare, Newspaper, Link2, ListOrdered, MapPin, Package, Pencil, Power, ShieldAlert, ShieldCheck, StickyNote, Wand2, Brain, X } from 'lucide-react';
import i18next from 'i18next';
import { useTranslation } from 'react-i18next';
import { tHtml } from '../../i18n/html';

/** The first problem of each kind the health check found for the selected mod, if any. */
export interface InspectorProblems {
  orderViolation?: { requiredModId: string; requiredModName: string; requiredModIndex: number };
  disabledDependency?: ModInfo;
  incompatibility?: { conflictingModId: string; conflictingModName: string };
}

interface ModInspectorProps {
  paths: StudioPathsUI;
  /** The mod to show, or null when the list is empty. */
  mod: ModInfo | null;
  /** Every installed mod, for what requires this one and what it requires. */
  mods: ModInfo[];
  /** Where the mod sits in the list (0-based), shown with a load-order warning. */
  position: number;
  problems: InspectorProblems;
  /** Badge classes when the mod shares its Workshop item with other mods, else undefined. */
  packageBadgeClass?: string;
  onJumpToMod: (modId: string) => void;
  onJumpToDependency: (requireEntry: string) => void;
  onFixLoadOrder: (modId: string, requiredModId: string) => void;
  onToggleMod: (modId: string) => void;
  onOpenInBrowser?: (workshopId: string) => void;
  /** The health check's findings for this mod, and what their fix buttons need. */
  issues: HealthIssue[];
  fixContext: FixContext;
  /** Where it comes from (Steam, a compatible copy, Nexus...). */
  catalogEntry?: CatalogModSummary;
  /** Last session: what the game logged about it, its loading time (ms), its share of the game's time (%). */
  errorStat?: ModErrorStat;
  loadMs?: number;
  gameShare?: number;
  /** The player's own: the profiles holding it, the active one, its note and groups. */
  profileNames: string[];
  activeProfileName?: string;
  note?: string;
  /** The Studio's note (the Doctor's), shown apart from the player's; it can be removed here. */
  studioNote?: string;
  onRemoveStudioNote?: (mod: ModInfo) => void;
  /** A mod id as its name, for the history's "what changed". */
  nameOf?: (id: string) => string;
  groups: string[];
  onEditNote?: (mod: ModInfo) => void;
  onEditGroups?: (mod: ModInfo) => void;
}

type InspectorTab = 'studio' | 'page' | 'comments';

/** "Needed by" shows this many before folding the rest (a library can have dozens). */
const DEPENDENTS_SHOWN = 6;

/** The game's own description markup: `<LINE>`, `<SIZE:large>`, `<RGB:r,g,b>`, `<CENTRE>`. */
const renderPZRichText = (text?: string): React.ReactNode => {
  if (!text) return <span className="text-slate-500 italic">{i18next.t('inspector.noDescription')}</span>;

  const cleanText = text.replace(/<LINE>/gi, '\n');
  const parts = cleanText.split(/(<[^>]+>)/g);

  let currentSize = 'small';
  let currentColor = 'inherit';
  let currentAlign = 'left';

  const elements: React.ReactNode[] = [];

  parts.forEach((part, index) => {
    if (!part) return;

    if (part.startsWith('<') && part.endsWith('>')) {
      const tagContent = part.slice(1, -1).trim();
      const upperTag = tagContent.toUpperCase();

      if (upperTag.startsWith('SIZE:')) {
        const sizeVal = upperTag.split(':')[1].toLowerCase();
        currentSize = sizeVal;
      } else if (upperTag.startsWith('RGB:')) {
        const rgbStr = tagContent.slice(4).trim();
        const rgbVals = rgbStr.split(',').map((v) => parseFloat(v.trim()));
        if (rgbVals.length >= 3 && !isNaN(rgbVals[0])) {
          const r = rgbVals[0] <= 1.0 ? Math.round(rgbVals[0] * 255) : Math.round(rgbVals[0]);
          const g = rgbVals[1] <= 1.0 ? Math.round(rgbVals[1] * 255) : Math.round(rgbVals[1]);
          const b = rgbVals[2] <= 1.0 ? Math.round(rgbVals[2] * 255) : Math.round(rgbVals[2]);
          const a = rgbVals.length >= 4 ? rgbVals[3] : 1;
          currentColor = `rgba(${r}, ${g}, ${b}, ${a})`;
        }
      } else if (upperTag === 'LEFT') {
        currentAlign = 'left';
      } else if (upperTag === 'CENTRE' || upperTag === 'CENTER') {
        currentAlign = 'center';
      } else if (upperTag === 'RIGHT') {
        currentAlign = 'right';
      }
    } else {
      const fontSizeClass =
        currentSize === 'large'
          ? 'text-sm font-bold tracking-wide block my-1'
          : currentSize === 'medium'
          ? 'text-xs font-bold tracking-wide block my-0.5'
          : 'text-[11px] leading-relaxed';

      const lines = part.split('\n');
      lines.forEach((line, lIdx) => {
        if (line) {
          elements.push(
            <span
              key={`${index}-${lIdx}`}
              className={`${fontSizeClass} inline-block`}
              style={{ color: currentColor === 'inherit' ? undefined : currentColor, textAlign: currentAlign as any }}
            >
              {line}
            </span>
          );
        }
        if (lIdx < lines.length - 1) {
          elements.push(<br key={`${index}-br-${lIdx}`} />);
        }
      });
    }
  });

  return <div className="space-y-0.5 font-sans leading-relaxed text-slate-200 select-text whitespace-pre-wrap">{elements}</div>;
};

/**
 * The right column of the Mod List, most pressing first: who the mod is (poster, kind, origin,
 * author), its switch, what to do about it (every health finding with its fix), the player's own
 * note, groups and profiles, the last session's errors and costs, what it needs and what needs
 * it, its description (folded), its mod.info facts and, folded, what its Java changes. The poster
 * is read from disk, or taken from the Workshop page when the mod ships none.
 */
export const ModInspector: React.FC<ModInspectorProps> = ({
  paths,
  mod,
  mods,
  position,
  problems,
  packageBadgeClass,
  onJumpToMod,
  onJumpToDependency,
  onFixLoadOrder,
  onToggleMod,
  onOpenInBrowser,
  issues,
  fixContext,
  catalogEntry,
  errorStat,
  loadMs,
  gameShare,
  profileNames,
  activeProfileName,
  note,
  studioNote,
  onRemoveStudioNote,
  nameOf,
  groups,
  onEditNote,
  onEditGroups,
}) => {
  const { t } = useTranslation();
  const [imageError, setImageError] = useState<boolean>(false);
  /** The poster shown full size over the screen. */
  const [posterBig, setPosterBig] = useState(false);
  /** The description unfolded. */
  const [descOpen, setDescOpen] = useState(false);
  /** Which tab is open; it stays as picked from one mod to the next (and between sessions). */
  const [tab, setTab] = useState<InspectorTab>(() => {
    const kept = localStorage.getItem('pz_inspector_tab');
    return kept === 'comments' ? 'comments' : kept === 'page' || kept === 'mod' ? 'page' : 'studio';
  });
  const pickTab = (id: InspectorTab) => {
    setTab(id);
    localStorage.setItem('pz_inspector_tab', id);
    scroller.current?.scrollTo({ top: 0 });
  };
  /** "Needed by" unfolded past the first few. */
  const [dependentsOpen, setDependentsOpen] = useState(false);
  /** The scrolling part: back to the top for each mod picked. */
  const scroller = useRef<HTMLDivElement>(null);
  useEffect(() => {
    setPosterBig(false);
    setDescOpen(false);
    setDependentsOpen(false);
    scroller.current?.scrollTo({ top: 0 });
  }, [mod?.mod_id]);
  /** The changelog the mod ships, for its newest entry and its author's support links. */
  const [changelog, setChangelog] = useState<ModChangelog | null>(null);
  useEffect(() => {
    setChangelog(null);
    if (!mod || mod.not_installed) return;
    let cancelled = false;
    void TauriService.modChangelog(paths, mod.mod_id).then((log) => {
      if (!cancelled) setChangelog(log);
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mod?.mod_id]);
  /** What the mod's Java declares it changes (ZombieBuddy patches in its jar); null: no Java. */
  const [javaPatches, setJavaPatches] = useState<ModJavaPatches | null>(null);
  useEffect(() => {
    setJavaPatches(null);
    if (!mod || mod.not_installed) return;
    let cancelled = false;
    void TauriService.javaPatchesOfMod(paths, mod.mod_id).then((found) => {
      if (!cancelled) setJavaPatches(found);
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mod?.mod_id]);
  /** Target class -> the method names patched there, for the patches ZombieBuddy applies. */
  const javaClasses = useMemo(() => {
    const byClass = new Map<string, Set<string>>();
    (javaPatches?.patches ?? []).forEach((p) => {
      if (p.not_applied || !p.target_class) return;
      const methods = byClass.get(p.target_class) ?? new Set<string>();
      if (p.target_method) methods.add(p.target_method);
      byClass.set(p.target_class, methods);
    });
    return [...byClass.entries()].map(([name, methods]) => ({ name, methods: [...methods].sort() })).sort((a, b) => a.name.localeCompare(b.name));
  }, [javaPatches]);
  /** The selected mod's poster, read from disk as a data URL (the webview cannot open files). */
  const [posterSrc, setPosterSrc] = useState<string | null>(null);
  /** Workshop id -> the picture on its Workshop page, for the mods that ship none. */
  const [workshopPreviews, setWorkshopPreviews] = useState<Map<string, string>>(new Map());
  /** Workshop id -> the support links (Ko-fi, Patreon...) in its Workshop description. */
  const [workshopSupport, setWorkshopSupport] = useState<Map<string, AuthorLink[]>>(new Map());

  // Most mods ship no picture of their own, so ask Steam once for all of them instead of
  // waiting for a request every time the user picks a different mod.
  const workshopIdsKey = useMemo(
    () => [...new Set(mods.map((m) => m.workshop_id).filter((id): id is string => !!id))].sort().join(','),
    [mods]
  );
  useEffect(() => {
    if (!workshopIdsKey) return;
    let cancelled = false;
    TauriService.workshopDetails(workshopIdsKey.split(','))
      .then((items) => {
        if (cancelled) return;
        setWorkshopPreviews(new Map(items.filter((i) => i.preview_url).map((i) => [i.workshop_id, i.preview_url!])));
        setWorkshopSupport(new Map(items.filter((i) => i.support?.length).map((i) => [i.workshop_id, i.support!])));
      })
      .catch(() => undefined); // offline: the mods that ship a poster still show one
    return () => {
      cancelled = true;
    };
  }, [workshopIdsKey]);

  // The selected mod's picture: its own poster read from disk, else the Workshop one.
  useEffect(() => {
    setImageError(false);
    setPosterSrc(null);
    const file = mod?.poster_url;
    const workshopId = mod?.workshop_id;
    if (!mod || !paths.is_valid) return;
    let cancelled = false;

    const fromWorkshop = async () => {
      const url = workshopId ? workshopPreviews.get(workshopId) : undefined;
      if (!url) throw new Error('no picture for this mod');
      return url;
    };

    (file ? TauriService.readModImage(paths, file).catch(fromWorkshop) : fromWorkshop())
      .then((src) => !cancelled && setPosterSrc(src))
      .catch(() => !cancelled && setImageError(true));
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mod?.mod_id, mod?.poster_url, mod?.workshop_id, paths.is_valid, workshopPreviews]);

  const reverseDependents = useMemo(() => {
    if (!mod) return [];
    return mods.filter(
      (m) => m.mod_id !== mod.mod_id && m.dependencies.some((dep) => findInstalledDependencyInMods(dep, [mod]) !== undefined)
    );
  }, [mod, mods]);

  const parentLibraries = useMemo(() => {
    if (!mod) return [];
    return (mod.dependencies ?? []).map((raw) => ({ raw, matchedMod: findInstalledDependencyInMods(raw, mods) }));
  }, [mod, mods]);

  if (!mod) {
    return (
      <div className="col-span-12 lg:col-span-5 bg-slate-900/80 border border-slate-800 rounded-xl flex flex-col overflow-hidden shadow p-4">
        <div className="flex-1 flex flex-col items-center justify-center gap-3 text-center text-slate-500 text-xs">
          <div className="w-12 h-12 rounded-xl bg-slate-950 border border-slate-800 flex items-center justify-center">
            <ListOrdered className="w-6 h-6 text-slate-600" />
          </div>
          <span className="max-w-[240px] leading-relaxed">{t('inspector.selectOne')}</span>
        </div>
      </div>
    );
  }

  /** Comments need a Workshop page: a mod without one shows its description instead. */
  const shownTab: InspectorTab = tab === 'comments' && !mod.workshop_id ? 'page' : tab;
  const { orderViolation, disabledDependency, incompatibility } = problems;
  // The health check's cards say the same with the fix: the older cards stay only for what it did not raise.
  const codes = new Set(issues.map((i) => i.code));
  const showOrderCard = orderViolation && !codes.has('dependency_after') && !codes.has('load_after_order');
  const showDisabledCard = disabledDependency && !codes.has('dependency_not_enabled');
  const showIncompatCard = incompatibility && !codes.has('incompatible') && !codes.has('community_incompatible');
  const visibleIssues = issues.filter((i) => issueStateOf(i) !== 'handled');
  const worst = visibleIssues.reduce<IssueState | null>(
    (w, i) => (w === null || STATE_ORDER.indexOf(issueStateOf(i)) < STATE_ORDER.indexOf(w) ? issueStateOf(i) : w),
    null
  );
  const problemCount = visibleIssues.length + (showOrderCard ? 1 : 0) + (showDisabledCard ? 1 : 0) + (showIncompatCard ? 1 : 0);
  const hasSession =
    (errorStat && (errorStat.exceptions > 0 || errorStat.warnings > 0)) || loadMs !== undefined || gameShare !== undefined;
  const KindIcon = mod.is_map_mod ? MapPin : mod.is_library ? Package : ListOrdered;
  const kindColor = mod.is_map_mod ? 'text-amber-400' : mod.is_library ? 'text-purple-400' : 'text-cyan-400';
  const showPoster = !!posterSrc && !imageError;

  /** Under a class of the Java list: the other active mods whose Java changes it too. */
  const javaSharedLine = (className: string) => {
    const shared = javaPatches?.shared?.find((s) => s.target_class === className);
    if (!shared || shared.mods.length === 0) return null;
    return (
      <div className="font-sans text-[10.5px] text-amber-200/80 pl-2">
        {shared.methods.length > 0
          ? t('inspector.java.sharedMethod', { methods: shared.methods.join(', '), mods: shared.mods.join(', ') })
          : t('inspector.java.sharedClass', { mods: shared.mods.join(', ') })}
      </div>
    );
  };
  const javaClassItem = (c: { name: string; methods: string[] }) => (
    <li key={c.name} className="break-all">
      <span className="text-slate-200">{c.name}</span>
      <span className="text-slate-500"> · {c.methods.length > 0 ? c.methods.join(', ') : t('inspector.java.wholeClass')}</span>
      {javaSharedLine(c.name)}
    </li>
  );
  const fact = (label: string, value: React.ReactNode) => (
    <React.Fragment key={label}>
      <dt className="text-slate-500">{label}</dt>
      <dd className="text-slate-200 min-w-0 break-words">{value}</dd>
    </React.Fragment>
  );

  return (
    <div className="col-span-12 lg:col-span-5 bg-slate-900/80 border border-slate-800 rounded-xl flex flex-col overflow-hidden shadow">
      {/* Header: picture, kind, name and the few facts that say what it is */}
      <div className="p-4 pb-3 border-b border-slate-800 flex gap-3">
        <button
          type="button"
          onClick={() => showPoster && setPosterBig(true)}
          className={`w-20 h-20 shrink-0 rounded-lg border border-slate-800 bg-slate-950 overflow-hidden flex items-center justify-center ${showPoster ? 'cursor-zoom-in hover:border-slate-600' : 'cursor-default'}`}
          title={showPoster ? t('inspector.posterZoom') : undefined}
        >
          {showPoster ? (
            <img src={posterSrc!} alt={mod.name} className="w-full h-full object-cover" onError={() => setImageError(true)} />
          ) : (
            <KindIcon className={`w-7 h-7 ${kindColor}`} />
          )}
        </button>
        <div className="min-w-0 flex-1 flex flex-col gap-1">
          <div className="flex items-center gap-1.5 flex-wrap">
            <span className={`flex items-center gap-1 text-[10px] font-mono px-1.5 py-0.5 rounded bg-slate-950 border border-slate-800 ${kindColor}`}>
              <KindIcon className="w-3 h-3" />
              {mod.is_map_mod ? t('inspector.mapMod') : mod.is_library ? t('inspector.baseLibrary') : t('inspector.scriptMod')}
            </span>
            {packageBadgeClass && (
              <span className={`text-[10px] font-mono font-bold px-1.5 py-0.5 rounded border ${packageBadgeClass} flex items-center gap-1`}>
                <Layers className="w-3 h-3" />
                {t('inspector.multiPackage')}
              </span>
            )}
            {!mod.not_installed && <SourceBadge modId={mod.mod_id} entry={catalogEntry} />}
          </div>
          <h3
            onClick={() => onJumpToMod(mod.mod_id)}
            className="text-base font-bold leading-snug text-slate-100 hover:text-emerald-400 transition cursor-pointer"
            title={t('inspector.centerTitle')}
          >
            {mod.name}
          </h3>
          <div className="text-[11px] font-mono text-emerald-400/90 truncate" title={mod.mod_id}>{mod.mod_id}</div>
          <div className="text-[11px] text-slate-400 truncate">
            {[mod.author || t('inspector.unknown'), mod.version ? `v${mod.version}` : null].filter(Boolean).join(' · ')}
          </div>
          <SupportLinks links={changelog?.links ?? []} more={mod.workshop_id ? workshopSupport.get(mod.workshop_id) : undefined} />
        </div>
      </div>

      {/* Its switch, and the Workshop page */}
      <div className="px-4 py-2 border-b border-slate-800 flex items-center gap-2">
        <button
          type="button"
          onClick={() => onToggleMod(mod.mod_id)}
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[11px] font-bold border cursor-pointer transition ${
            mod.enabled
              ? 'bg-emerald-600/20 text-emerald-300 border-emerald-600/60 hover:bg-emerald-600/30'
              : 'bg-slate-950 text-slate-400 border-slate-700 hover:border-slate-500'
          }`}
          title={mod.enabled ? t('modList.row.clickDisable') : t('modList.row.clickEnable')}
        >
          <Power className="w-3.5 h-3.5" />
          {mod.enabled ? t('inspector.onWithPlace', { n: position + 1 }) : t('inspector.offTurnOn')}
        </button>
        <div className="flex-1" />
        {mod.workshop_id && onOpenInBrowser && (
          <button
            onClick={() => onOpenInBrowser(mod.workshop_id!)}
            className="flex items-center gap-1 text-[11px] text-cyan-400 hover:text-cyan-300 bg-transparent border-0 cursor-pointer"
            title={t('inspector.openInBrowserTitle')}
          >
            <BookOpen className="w-3.5 h-3.5" />
            <span>{t('inspector.workshopPage')}</span>
          </button>
        )}
      </div>

      {/* Tabs: what the Studio knows about it in this list; its Workshop page (or the mod.info
          description for a mod with none); the Workshop comments */}
      <div className="px-4 border-b border-slate-800 flex items-end gap-1" role="tablist">
        {(mod.workshop_id ? (['studio', 'page', 'comments'] as const) : (['studio', 'page'] as const)).map((id) => (
          <button
            key={id}
            role="tab"
            aria-selected={shownTab === id}
            onClick={() => pickTab(id)}
            className={`flex items-center gap-1.5 px-3 py-2 text-[11.5px] font-bold border-b-2 -mb-px cursor-pointer transition ${
              shownTab === id ? 'border-emerald-400 text-emerald-300' : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            {id === 'studio' ? <ShieldCheck className="w-3.5 h-3.5" /> : id === 'page' ? <Newspaper className="w-3.5 h-3.5" /> : <MessageSquare className="w-3.5 h-3.5" />}
            {t(id === 'studio' ? 'inspector.tabs.studio' : id === 'comments' ? 'inspector.workshop.comments' : mod.workshop_id ? 'inspector.workshop.page' : 'inspector.overview')}
            {id === 'studio' && problemCount > 0 && worst && (
              <span className={`text-[10px] px-1.5 rounded-full ${STATE_STYLE[worst].badge}`}>{problemCount}</span>
            )}
          </button>
        ))}
      </div>

      <div ref={scroller} className="flex-1 min-h-0 overflow-y-auto p-4 space-y-5">
        {shownTab === 'studio' && (
          <>
        {/* The player's own: note, groups, profiles */}
        <Section icon={StickyNote} iconClass="text-sky-300" title={t('inspector.sections.notes')}>
          <div className="space-y-2.5">
            {note ? (
              <div className="rounded-lg border border-sky-800/60 bg-sky-950/30 px-3 py-2">
                <div className="text-[12px] text-sky-100 whitespace-pre-wrap break-words leading-relaxed">{note}</div>
                {onEditNote && (
                  <button onClick={() => onEditNote(mod)} className="mt-1.5 flex items-center gap-1 text-[11px] text-sky-300 hover:text-sky-200 cursor-pointer">
                    <Pencil className="w-3 h-3" />
                    {t('inspector.sections.editNote')}
                  </button>
                )}
              </div>
            ) : (
              onEditNote && (
                <button onClick={() => onEditNote(mod)} className="w-full flex items-center gap-1.5 px-3 py-2 rounded-lg border border-dashed border-slate-700 text-[11.5px] text-slate-400 hover:text-sky-300 hover:border-sky-700 cursor-pointer">
                  <StickyNote className="w-3.5 h-3.5" />
                  {t('inspector.sections.addNote')}
                </button>
              )
            )}
            {studioNote && (
              <div className="rounded-lg border border-violet-800/60 bg-violet-950/25 px-3 py-2" title={t('brain.studioNote.hint')}>
                <div className="flex items-center gap-1.5 text-[10px] font-bold uppercase tracking-wide text-violet-300">
                  <Brain className="w-3 h-3" aria-hidden="true" />
                  {t('brain.studioNote.label')}
                </div>
                <div className="mt-1 text-[12px] text-violet-100 whitespace-pre-wrap break-words leading-relaxed">{studioNote}</div>
                {onRemoveStudioNote && (
                  <button onClick={() => onRemoveStudioNote(mod)} className="mt-1.5 flex items-center gap-1 text-[11px] text-violet-300 hover:text-violet-200 cursor-pointer">
                    <X className="w-3 h-3" aria-hidden="true" />
                    {t('brain.studioNote.remove')}
                  </button>
                )}
              </div>
            )}
            <div className="grid grid-cols-[88px_1fr] gap-x-2 gap-y-1.5 text-[11.5px] items-start">
              <span className="text-slate-500 pt-0.5">{t('inspector.sections.groups')}</span>
              <div className="flex flex-wrap items-center gap-1">
                {groups.map((g) => (
                  <span key={g} className="text-[10.5px] px-1.5 py-0.5 rounded bg-violet-900/50 text-violet-200 border border-violet-700/50">{g}</span>
                ))}
                {onEditGroups && (
                  <button onClick={() => onEditGroups(mod)} className="text-[10.5px] px-1.5 py-0.5 rounded border border-dashed border-slate-700 text-slate-400 hover:text-violet-200 hover:border-violet-600 cursor-pointer">
                    {groups.length ? t('inspector.sections.editGroups') : t('inspector.sections.addGroup')}
                  </button>
                )}
              </div>
              <span className="text-slate-500 pt-0.5">{t('inspector.sections.profiles')}</span>
              <div className="flex flex-wrap items-center gap-1">
                {profileNames.length === 0 && <span className="text-slate-500 italic">{t('modList.row.noProfileYet')}</span>}
                {profileNames.map((p) => (
                  <span
                    key={p}
                    className={`flex items-center gap-1 text-[10.5px] px-1.5 py-0.5 rounded border ${
                      p === activeProfileName ? 'border-emerald-700/60 text-emerald-300 bg-emerald-950/30' : 'border-slate-700 text-slate-400'
                    }`}
                    title={p === activeProfileName ? t('inspector.sections.activeProfile') : undefined}
                  >
                    <Layers className="w-3 h-3" />
                    {p}
                  </span>
                ))}
              </div>
            </div>
          </div>
        </Section>

        {/* What to do: every finding of the health check, worst first, with its fix */}
        {problemCount > 0 ? (
          <Section
            icon={worst ? STATE_STYLE[worst].icon : ShieldAlert}
            iconClass={worst ? STATE_STYLE[worst].iconColor : 'text-orange-400'}
            title={t('inspector.sections.todo', { count: problemCount })}
            foldId="todo"
          >
            <div className="space-y-2">
              {showOrderCard && orderViolation && (
                <div className="rounded-lg border border-orange-500/50 bg-orange-950/30 px-3 py-2 text-[11.5px] space-y-1.5">
                  <div className="font-bold text-orange-300">{t('inspector.orderTitle')}</div>
                  <p className="text-slate-300 leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('inspector.orderBody', { name: orderViolation.requiredModName, n: orderViolation.requiredModIndex + 1 }) }} />
                  <div className="flex gap-1.5">
                    <button onClick={() => onFixLoadOrder(mod.mod_id, orderViolation.requiredModId)} className="flex items-center gap-1 px-2 py-1 rounded-md bg-emerald-700 hover:bg-emerald-600 text-white text-[11px] font-bold cursor-pointer">
                      <Wand2 className="w-3 h-3" />
                      {t('inspector.moveAfter', { name: orderViolation.requiredModName })}
                    </button>
                    <button onClick={() => onJumpToMod(orderViolation.requiredModId)} className="px-2 py-1 rounded-md border border-slate-700 hover:bg-slate-800 text-slate-300 text-[11px] cursor-pointer" title={t('inspector.jumpToDependency')}>
                      {t('inspector.viewDependency')}
                    </button>
                  </div>
                </div>
              )}
              {showDisabledCard && disabledDependency && (
                <div className="rounded-lg border border-rose-500/50 bg-rose-950/30 px-3 py-2 text-[11.5px] space-y-1.5">
                  <div className="font-bold text-rose-300">{t('inspector.disabledTitle')}</div>
                  <p className="text-slate-300 leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('inspector.disabledBody', { name: disabledDependency.name }) }} />
                  <div className="flex gap-1.5">
                    <button onClick={() => onToggleMod(disabledDependency.mod_id)} className="flex items-center gap-1 px-2 py-1 rounded-md bg-emerald-700 hover:bg-emerald-600 text-white text-[11px] font-bold cursor-pointer">
                      <Check className="w-3 h-3" />
                      {t('inspector.activateLibrary')}
                    </button>
                    <button onClick={() => onJumpToMod(disabledDependency.mod_id)} className="px-2 py-1 rounded-md border border-slate-700 hover:bg-slate-800 text-slate-300 text-[11px] cursor-pointer" title={t('inspector.jumpToLibrary')}>
                      {t('inspector.viewMod')}
                    </button>
                  </div>
                </div>
              )}
              {showIncompatCard && incompatibility && (
                <div className="rounded-lg border border-amber-500/50 bg-amber-950/30 px-3 py-2 text-[11.5px] space-y-1.5">
                  <div className="font-bold text-amber-300">{t('inspector.incompatTitle')}</div>
                  <p className="text-slate-300 leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('inspector.incompatBody', { name: incompatibility.conflictingModName }) }} />
                  <div className="flex gap-1.5">
                    <button onClick={() => onJumpToMod(incompatibility.conflictingModId)} className="px-2 py-1 rounded-md border border-amber-600/60 hover:bg-amber-900/30 text-amber-200 text-[11px] font-bold cursor-pointer">
                      {t('inspector.viewConflicting')}
                    </button>
                    <button onClick={() => onToggleMod(incompatibility.conflictingModId)} className="px-2 py-1 rounded-md border border-slate-700 hover:bg-slate-800 text-slate-300 text-[11px] cursor-pointer" title={t('inspector.deactivateTitle')}>
                      {t('inspector.deactivate')}
                    </button>
                  </div>
                </div>
              )}
              <IssueCards issues={issues} onJumpToMod={onJumpToMod} {...fixContext} />
            </div>
          </Section>
        ) : (
          <div className="flex items-center gap-2 text-[11.5px] text-emerald-300/90 bg-emerald-950/20 border border-emerald-900/50 rounded-lg px-3 py-2">
            <ShieldCheck className="w-4 h-4 shrink-0" />
            {t('inspector.sections.allGood')}
          </div>
        )}

        {/* On this PC: what the brain learned of it across sessions */}
        <Section icon={Brain} iconClass="text-violet-300" title={t('brain.history.title')} foldId="brain">
          <ModBrainSection paths={paths} modId={mod.mod_id} name={nameOf ?? ((id) => id)} />
        </Section>

        {/* Last session: what the game logged and what it cost */}
        {hasSession && (
          <Section icon={Activity} iconClass="text-cyan-300" title={t('inspector.sections.session')} foldId="session">
            <dl className="grid grid-cols-[120px_1fr] gap-x-2 gap-y-1.5 text-[11.5px]">
              {errorStat && (errorStat.exceptions > 0 || errorStat.warnings > 0) && (
                <>
                  <dt className="text-slate-500">{t('inspector.sections.logged')}</dt>
                  <dd className="min-w-0">
                    <span className={errorStat.exceptions > 0 ? 'text-red-300 font-bold' : 'text-slate-300'}>
                      {[
                        errorStat.exceptions > 0 ? t('modList.row.threw', { count: errorStat.exceptions }) : null,
                        errorStat.warnings > 0 ? t('modList.row.warn', { count: errorStat.warnings }) : null,
                      ]
                        .filter(Boolean)
                        .join(' · ')}
                    </span>
                    {(errorStat.last_exception ?? errorStat.last_warning) && (
                      <div className="mt-1 font-mono text-[10.5px] text-red-200/80 bg-slate-950 border border-slate-800 rounded px-1.5 py-1 break-all max-h-24 overflow-y-auto">
                        {errorStat.last_exception ?? errorStat.last_warning}
                      </div>
                    )}
                  </dd>
                </>
              )}
              {loadMs !== undefined && (
                <>
                  <dt className="text-slate-500">{t('inspector.sections.loadTime')}</dt>
                  <dd className={loadMs >= 1000 ? 'text-amber-300 font-bold' : 'text-slate-300'}>
                    {loadMs >= 1000 ? `${(loadMs / 1000).toFixed(1)} s` : `${loadMs} ms`}
                  </dd>
                </>
              )}
              {gameShare !== undefined && (
                <>
                  <dt className="text-slate-500">{t('inspector.sections.gameTime')}</dt>
                  <dd className={gameShare >= 5 ? 'text-amber-300 font-bold' : 'text-slate-300'}>{t('modList.row.gameTime', { pct: gameShare.toFixed(gameShare < 10 ? 1 : 0) })}</dd>
                </>
              )}
            </dl>
          </Section>
        )}

        {/* What it needs and what needs it */}
        <Section icon={Link2} iconClass="text-purple-300" title={t('inspector.sections.deps')}>
          <div className="grid grid-cols-[88px_1fr] gap-x-2 gap-y-2 text-[11.5px] items-start">
            <span className="text-slate-500 pt-1">{t('inspector.sections.needs')}</span>
            <div className="flex flex-wrap gap-1">
              {parentLibraries.length === 0 && <span className="text-slate-500 italic pt-1">{t('inspector.sections.none')}</span>}
              {parentLibraries.map((item, idx) => {
                const matched = item.matchedMod;
                const off = matched && !matched.enabled;
                return (
                  <button
                    key={idx}
                    onClick={() => onJumpToDependency(item.raw)}
                    className={`flex items-center gap-1.5 px-2 py-1 rounded-md border text-[11px] cursor-pointer transition ${
                      off ? 'border-rose-700 text-rose-200 bg-rose-950/50 hover:border-rose-400' : matched ? 'border-emerald-800 text-emerald-200 bg-emerald-950/40 hover:border-emerald-500' : 'border-red-700 text-red-200 bg-red-950/50 hover:border-red-400'
                    }`}
                    title={matched ? t('inspector.parentTitle', { name: matched.name, state: matched.enabled ? t('inspector.active') : t('inspector.disabledUpper') }) : t('inspector.notInstalledTitle', { name: item.raw })}
                  >
                    <span className={`w-1.5 h-1.5 rounded-full ${off ? 'bg-rose-400' : matched ? 'bg-emerald-400' : 'bg-red-400'}`} />
                    {matched ? matched.name : item.raw}
                  </button>
                );
              })}
            </div>
            <span className="text-slate-500 pt-1">{t('inspector.sections.neededBy')}</span>
            <div className="flex flex-wrap gap-1">
              {reverseDependents.length === 0 && <span className="text-slate-500 italic pt-1">{t('inspector.sections.none')}</span>}
              {(dependentsOpen ? reverseDependents : reverseDependents.slice(0, DEPENDENTS_SHOWN)).map((d) => (
                <button
                  key={d.mod_id}
                  onClick={() => onJumpToMod(d.mod_id)}
                  className={`flex items-center gap-1.5 px-2 py-1 rounded-md border text-[11px] cursor-pointer transition ${
                    d.enabled ? 'border-purple-800 text-purple-200 bg-purple-950/40 hover:border-purple-500' : 'border-slate-800 text-slate-500 hover:text-slate-300'
                  }`}
                  title={t('inspector.childTitle', { name: d.name, state: d.enabled ? t('inspector.active') : t('inspector.disabledUpper') })}
                >
                  <span className={`w-1.5 h-1.5 rounded-full ${d.enabled ? 'bg-emerald-400' : 'bg-slate-600'}`} />
                  {d.name}
                </button>
              ))}
              {reverseDependents.length > DEPENDENTS_SHOWN && (
                <button onClick={() => setDependentsOpen((o) => !o)} className="px-2 py-1 text-[11px] text-cyan-400 hover:text-cyan-300 cursor-pointer">
                  {dependentsOpen ? t('inspector.sections.less') : t('inspector.sections.andMore', { count: reverseDependents.length - DEPENDENTS_SHOWN })}
                </button>
              )}
            </div>
          </div>
        </Section>

        {/* mod.info facts, in two columns */}
        <Section icon={Info} iconClass="text-slate-400" title={t('inspector.metadata')} foldId="modinfo" startClosed>
          <dl className="grid grid-cols-[120px_1fr] gap-x-2 gap-y-1 text-[11.5px]">
            {fact(t('inspector.version'), mod.version || '1.0')}
            {fact(t('inspector.author'), mod.author || t('inspector.unknown'))}
            {fact(t('inspector.zomboidVersion'), mod.pzversion || '42.0+')}
            {fact(t('inspector.workshopId'), mod.workshop_id || t('inspector.builtIn'))}
            {fact(t('inspector.source'), mod.workshop_id ? t('inspector.steamWorkshop') : t('inspector.localDirectory'))}
            {mod.url &&
              fact(
                t('inspector.homepage'),
                <button onClick={() => TauriService.openExternalUrl(mod.url!)} className="text-cyan-400 hover:underline text-left break-all cursor-pointer">
                  {mod.url}
                </button>
              )}
            {mod.incompatible && mod.incompatible.length > 0 && fact(t('inspector.incompatibleWith'), <span className="text-amber-300">{mod.incompatible.join(', ')}</span>)}
          </dl>
          {changelog?.entries[0] && (
            <div className="mt-3 space-y-2">
                            {changelog?.entries[0] && (
                <details>
                  <summary className="text-[11px] text-sky-300 cursor-pointer select-none">
                    {t('inspector.changelog')}
                    {changelog.entries[0].title ? `: ${changelog.entries[0].title}` : ''}
                  </summary>
                  <div className="mt-1.5 text-[11px] text-slate-300 whitespace-pre-wrap break-words max-h-40 overflow-y-auto">{changelog.entries[0].text}</div>
                </details>
              )}
            </div>
          )}
        </Section>

        {/* What the mod's Java changes: folded, it is for the curious */}
        {javaPatches && (
          <details className="group">
            <summary className="list-none cursor-pointer select-none flex items-center gap-1.5 text-[11px] font-bold uppercase tracking-wider text-slate-400 hover:text-slate-200">
              <ChevronRight className="w-3.5 h-3.5 transition group-open:rotate-90" />
              <Layers className="w-3.5 h-3.5 text-sky-400" />
              <span className="text-sky-300">{t('inspector.java.title', { count: javaClasses.length })}</span>
              {(javaPatches.shared?.length ?? 0) > 0 && (
                <span className="ml-1 normal-case tracking-normal font-normal text-amber-300/90">{t('inspector.sections.javaShared', { count: javaPatches.shared!.length })}</span>
              )}
            </summary>
            <div className="mt-2 space-y-1">
              {javaPatches.unknown ? (
                <p className="text-[11px] text-amber-300 leading-relaxed">
                  {t(javaPatches.unknown === 'jar_missing' ? 'inspector.java.jarMissing' : javaPatches.unknown === 'no_package' ? 'inspector.java.noPackage' : 'inspector.java.unreadable', { jar: javaPatches.jar ?? '' })}
                </p>
              ) : (
                <>
                  <p className="text-[11px] text-slate-500 leading-relaxed">{t('inspector.java.how', { jar: javaPatches.jar ?? '', package: javaPatches.package ?? '' })}</p>
                  {(javaPatches.shared?.length ?? 0) > 0 && <p className="text-[11px] text-slate-400 leading-relaxed">{t('inspector.java.sharedNote')}</p>}
                  {javaClasses.length === 0 && <p className="text-[11px] text-slate-400">{t('inspector.java.none')}</p>}
                  {javaClasses.length > 0 && <ul className="text-[11px] font-mono leading-relaxed">{javaClasses.map(javaClassItem)}</ul>}
                  {[...new Set(javaPatches.patches.map((p) => p.not_applied).filter((w): w is string => !!w))].map((why) => (
                    <p key={why} className="text-[11px] text-slate-500 leading-relaxed">
                      {t('inspector.java.notApplied', { count: javaPatches.patches.filter((p) => p.not_applied === why).length, why: t(`inspector.java.why.${why}`, { defaultValue: why }) })}
                    </p>
                  ))}
                  {javaPatches.patches.some((p) => p.by_code && !p.not_applied) && <p className="text-[11px] text-slate-500 leading-relaxed">{t('inspector.java.byCode')}</p>}
                  {javaPatches.patches_in_code.length > 0 && (
                    <p className="text-[11px] text-amber-200/90 leading-relaxed">{t('inspector.java.inCode', { what: javaPatches.patches_in_code.join(', ') })}</p>
                  )}
                  {javaPatches.side && <p className="text-[11px] text-slate-500 leading-relaxed">{t(javaPatches.side === 'client' ? 'inspector.java.sideClient' : 'inspector.java.sideServer')}</p>}
                </>
              )}
            </div>
          </details>
        )}
          </>
        )}
        {shownTab !== 'studio' && (
          <>
        {mod.workshop_id ? (
          <ModWorkshopPage view={shownTab === 'comments' ? 'comments' : 'page'} workshopId={mod.workshop_id} mods={mods} fallback={plainDescription(mod.description)} onJumpToMod={onJumpToMod} onOpenInBrowser={onOpenInBrowser} />
        ) : (
          <>
        {/* The mod's own description, folded to a few lines */}
        <Section icon={BookOpen} iconClass="text-slate-300" title={t('inspector.overview')}>
          <div className={`relative text-xs text-slate-300 ${descOpen ? '' : 'max-h-32 overflow-hidden'}`}>
            {renderPZRichText(mod.description)}
            {!descOpen && (mod.description?.length ?? 0) > 280 && <div className="absolute inset-x-0 bottom-0 h-10 bg-gradient-to-t from-slate-900 to-transparent pointer-events-none" />}
          </div>
          {(mod.description?.length ?? 0) > 280 && (
            <button onClick={() => setDescOpen((o) => !o)} className="mt-1 text-[11px] text-cyan-400 hover:text-cyan-300 cursor-pointer">
              {descOpen ? t('inspector.sections.less') : t('inspector.sections.more')}
            </button>
          )}
        </Section>

          </>
        )}
          </>
        )}
      </div>

      {/* The poster, full size, over the screen */}
      {posterBig && showPoster && (
        <div className="fixed inset-0 z-50 bg-slate-950/85 backdrop-blur-sm flex items-center justify-center p-8 cursor-zoom-out" onClick={() => setPosterBig(false)}>
          <img src={posterSrc!} alt={mod.name} className="max-w-full max-h-full rounded-xl shadow-2xl border border-slate-700" />
        </div>
      )}
    </div>
  );
};

/**
 * One block of the inspector: a small heading with its icon, then its content. With `foldId` the
 * heading folds it, and the choice is kept (the same for every mod, and between sessions).
 */
const Section: React.FC<{ icon: React.ElementType; iconClass: string; title: React.ReactNode; children: React.ReactNode; foldId?: string; startClosed?: boolean }> = ({
  icon: Icon,
  iconClass,
  title,
  children,
  foldId,
  startClosed = false,
}) => {
  const key = foldId ? `pz_inspector_open_${foldId}` : '';
  const [open, setOpen] = useState(() => {
    if (!foldId) return true;
    const kept = localStorage.getItem(key);
    return kept === null ? !startClosed : kept === '1';
  });
  const heading = (
    <>
      {foldId && <ChevronRight className={`w-3.5 h-3.5 transition ${open ? 'rotate-90' : ''}`} />}
      <Icon className={`w-3.5 h-3.5 ${iconClass}`} />
      {title}
    </>
  );
  return (
    <section>
      {foldId ? (
        <button
          type="button"
          onClick={() => {
            setOpen(!open);
            localStorage.setItem(key, open ? '0' : '1');
          }}
          className={`w-full flex items-center gap-1.5 text-[11px] font-bold uppercase tracking-wider text-slate-400 hover:text-slate-200 cursor-pointer ${open ? 'mb-2' : ''}`}
        >
          {heading}
        </button>
      ) : (
        <h4 className="flex items-center gap-1.5 text-[11px] font-bold uppercase tracking-wider text-slate-400 mb-2">{heading}</h4>
      )}
      {open && children}
    </section>
  );
};

/** The game's description markup as plain text, for where Steam's page did not answer. */
const plainDescription = (text?: string) => (text ? text.replace(/<LINE>/gi, '\n').replace(/<[^>]+>/g, '').trim() : undefined);
