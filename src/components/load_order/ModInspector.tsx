import React, { useEffect, useMemo, useState } from 'react';
import { AuthorLink, ModChangelog, ModInfo } from '../../types';
import { SupportLinks } from '../common/SupportLinks';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { findInstalledDependencyInMods } from './modLookup';
import {
  BookOpen,
  Check,
  ExternalLink,
  Layers,
  Link2,
  ListOrdered,
  MapPin,
  Package,
  ShieldAlert,
  Wand2,
} from 'lucide-react';
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
}

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
 * The right column of the Mod List: the selected mod, its problems with buttons that fix them,
 * its description and poster, its mod.info fields, and the mods on either side of its
 * requirements. The poster is read from disk, or taken from the Workshop page when the mod
 * ships none.
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
}) => {
  const { t } = useTranslation();
  const [imageError, setImageError] = useState<boolean>(false);
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
        <div className="flex-1 flex items-center justify-center text-slate-500 text-xs">
          {t('inspector.selectOne')}
        </div>
      </div>
    );
  }

  const { orderViolation, disabledDependency, incompatibility } = problems;

  return (
    <div className="col-span-12 lg:col-span-5 bg-slate-900/80 border border-slate-800 rounded-xl flex flex-col overflow-hidden shadow p-4">
      <div className="flex-1 flex flex-col overflow-y-auto space-y-4 pr-1">
        <div className="border-b border-slate-800 pb-3 space-y-1">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-1.5">
              <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-950 text-emerald-300 border border-emerald-800">
                {mod.is_map_mod ? t('inspector.mapMod') : mod.is_library ? t('inspector.baseLibrary') : t('inspector.scriptMod')}
              </span>
              {packageBadgeClass && (
                <span className={`text-[10px] font-mono font-bold px-2 py-0.5 rounded border ${packageBadgeClass} flex items-center gap-1`}>
                  <Layers className="w-3 h-3" />
                  {t('inspector.multiPackage')}
                </span>
              )}
            </div>

            {mod.workshop_id && onOpenInBrowser && (
              <button
                onClick={() => onOpenInBrowser(mod.workshop_id!)}
                className="flex items-center gap-1 text-[10px] text-cyan-400 hover:underline font-mono bg-transparent border-0 cursor-pointer"
                title={t('inspector.openInBrowserTitle')}
              >
                <span>{t('inspector.openInBrowser', { id: mod.workshop_id })}</span>
                <BookOpen className="w-3 h-3" />
              </button>
            )}
          </div>

          <h3
            onClick={() => onJumpToMod(mod.mod_id)}
            className="text-base font-bold text-slate-100 hover:text-emerald-400 transition cursor-pointer flex items-center gap-1.5"
            title={t('inspector.centerTitle')}
          >
            <span>{mod.name}</span>
          </h3>
          <div className="text-xs font-mono text-slate-400">{t('inspector.id')} <code className="text-emerald-400">{mod.mod_id}</code></div>
        </div>

        {/* Load Order Violation Warning Card */}
        {orderViolation && (
          <div className="p-3 bg-orange-950/70 border-2 border-orange-500 rounded-xl space-y-2 text-xs shadow-lg">
            <div className="flex items-center justify-between font-bold text-orange-300">
              <div className="flex items-center gap-2">
                <ShieldAlert className="w-4 h-4 text-orange-400 shrink-0" />
                <span>{t('inspector.orderTitle')}</span>
              </div>
              <span className="text-[10px] font-mono bg-orange-900/80 text-orange-200 px-2 py-0.5 rounded border border-orange-600">
                {t('inspector.position', { n: position + 1 })}
              </span>
            </div>
            <p className="text-[11px] text-slate-200 leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('inspector.orderBody', { name: orderViolation.requiredModName, n: orderViolation.requiredModIndex + 1 }) }} />
            <div className="flex items-center gap-2 pt-1">
              <button
                onClick={() => onFixLoadOrder(mod.mod_id, orderViolation.requiredModId)}
                className="flex-1 flex items-center justify-center gap-1.5 py-1.5 bg-gradient-to-r from-orange-600 to-amber-600 hover:from-orange-500 hover:to-amber-500 text-white rounded-lg text-xs font-bold transition cursor-pointer shadow"
              >
                <Wand2 className="w-4 h-4" />
                <span>{t('inspector.moveAfter', { name: orderViolation.requiredModName })}</span>
              </button>
              <button
                onClick={() => onJumpToMod(orderViolation.requiredModId)}
                className="px-3 py-1.5 bg-slate-900 hover:bg-slate-800 text-orange-300 border border-orange-800 rounded-lg text-xs font-bold transition cursor-pointer"
                title={t('inspector.jumpToDependency')}
              >
                <span>{t('inspector.viewDependency')}</span>
              </button>
            </div>
          </div>
        )}

        {/* Disabled Dependency Warning Card */}
        {disabledDependency && (
          <div className="p-3 bg-rose-950/60 border-2 border-rose-500/80 rounded-xl space-y-2 text-xs">
            <div className="flex items-center gap-2 font-bold text-rose-300">
              <ShieldAlert className="w-4 h-4 text-rose-400 shrink-0" />
              <span>{t('inspector.disabledTitle')}</span>
            </div>
            <p className="text-[11px] text-slate-300 leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('inspector.disabledBody', { name: disabledDependency.name }) }} />
            <div className="flex items-center gap-2 pt-1">
              <button
                onClick={() => onToggleMod(disabledDependency.mod_id)}
                className="flex-1 flex items-center justify-center gap-1.5 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-xs font-bold transition cursor-pointer shadow"
              >
                <Check className="w-4 h-4" />
                <span>{t('inspector.activateLibrary')}</span>
              </button>
              <button
                onClick={() => onJumpToMod(disabledDependency.mod_id)}
                className="px-3 py-1.5 bg-slate-900 hover:bg-slate-800 text-cyan-300 border border-cyan-800 rounded-lg text-xs font-bold transition cursor-pointer"
                title={t('inspector.jumpToLibrary')}
              >
                <span>{t('inspector.viewMod')}</span>
              </button>
            </div>
          </div>
        )}

        {/* Active Incompatibility Warning Card in Inspector */}
        {incompatibility && (
          <div className="p-3 bg-amber-950/60 border-2 border-amber-500/80 rounded-xl space-y-2 text-xs">
            <div className="flex items-center gap-2 font-bold text-amber-300">
              <ShieldAlert className="w-4 h-4 text-amber-400 shrink-0" />
              <span>{t('inspector.incompatTitle')}</span>
            </div>
            <p className="text-[11px] text-slate-300 leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('inspector.incompatBody', { name: incompatibility.conflictingModName }) }} />
            <div className="flex items-center gap-2 pt-1">
              <button
                onClick={() => onJumpToMod(incompatibility.conflictingModId)}
                className="flex-1 flex items-center justify-center gap-1.5 py-1.5 bg-amber-500/20 hover:bg-amber-500/40 text-amber-300 border border-amber-500/50 rounded-lg text-xs font-bold transition cursor-pointer"
              >
                <span>{t('inspector.viewConflicting')}</span>
              </button>
              <button
                onClick={() => onToggleMod(incompatibility.conflictingModId)}
                className="px-3 py-1.5 bg-slate-900 hover:bg-slate-800 text-slate-300 border border-slate-700 rounded-lg text-xs font-bold transition cursor-pointer"
                title={t('inspector.deactivateTitle')}
              >
                <span>{t('inspector.deactivate')}</span>
              </button>
            </div>
          </div>
        )}

        {((changelog && (changelog.links.length > 0 || changelog.entries.length > 0)) || (mod.workshop_id && workshopSupport.has(mod.workshop_id))) && (
          <div className="space-y-2 p-3 rounded-xl border border-slate-800 bg-slate-950/60">
            <SupportLinks links={changelog?.links ?? []} more={mod.workshop_id ? workshopSupport.get(mod.workshop_id) : undefined} />
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

        {/* Top Section: Split Description (Left) + Poster Thumbnail Frame (Right) */}
        <div className="grid grid-cols-12 gap-3 items-start">
          {/* Left Description Box with Parsed RichText */}
          <div className="col-span-12 sm:col-span-7 space-y-1">
            <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider">
              {t('inspector.overview')}
            </label>
            <div className="p-3 bg-slate-950/90 rounded-xl text-xs text-slate-300 leading-relaxed font-sans border border-slate-800 h-48 overflow-y-auto select-text shadow-inner">
              {renderPZRichText(mod.description)}
            </div>
          </div>

          {/* Right Poster Image Frame (256x256 image with fallback) */}
          <div className="col-span-12 sm:col-span-5 space-y-1">
            <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider">
              {t('inspector.poster')}
            </label>
            <div className="h-48 bg-slate-950 rounded-xl border border-slate-800 flex items-center justify-center overflow-hidden relative shadow-inner">
              {posterSrc && !imageError ? (
                <img
                  src={posterSrc}
                  alt={mod.name}
                  className="w-full h-full object-cover rounded-xl"
                  onError={() => setImageError(true)}
                />
              ) : (
                <div className="w-full h-full bg-gradient-to-br from-slate-900 via-slate-950 to-slate-900 border border-slate-800 rounded-xl flex flex-col items-center justify-center p-3 text-center space-y-2">
                  <div className="w-10 h-10 rounded-xl bg-slate-800/80 border border-slate-700/60 flex items-center justify-center shadow">
                    {mod.is_map_mod ? (
                      <MapPin className="w-5 h-5 text-amber-400" />
                    ) : mod.is_library ? (
                      <Package className="w-5 h-5 text-purple-400" />
                    ) : (
                      <ListOrdered className="w-5 h-5 text-cyan-400" />
                    )}
                  </div>
                  <span className="text-[11px] font-bold text-slate-200 line-clamp-1 max-w-[150px]">
                    {mod.name}
                  </span>
                  <div className="text-[10px] text-slate-400 font-mono">
                    <span>{mod.is_map_mod ? t('inspector.mapTile') : mod.is_library ? t('inspector.framework') : t('inspector.scriptModShort')}</span>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Bottom Section: In-Game Style Metadata Table Grid */}
        <div className="bg-slate-950/80 border border-slate-800 rounded-xl overflow-hidden text-xs shadow-md">
          <div className="bg-slate-900/90 px-3.5 py-2 border-b border-slate-800 text-[11px] font-bold uppercase tracking-wider text-slate-300 flex items-center justify-between">
            <span>{t('inspector.metadata')}</span>
            <span className="text-[10px] font-mono text-emerald-400 font-normal">{t('inspector.manifest')}</span>
          </div>
          <div className="divide-y divide-slate-800/60 font-mono text-[11px]">
            <div className="grid grid-cols-12 px-3.5 py-1.5 items-center">
              <div className="col-span-4 text-slate-400">{t('inspector.status')}</div>
              <div className={`col-span-8 font-bold ${mod.enabled ? 'text-emerald-400' : 'text-slate-500'}`}>
                {mod.enabled ? t('inspector.enabled') : t('inspector.disabled')}
              </div>
            </div>

            <div className="grid grid-cols-12 px-3.5 py-1.5 items-center">
              <div className="col-span-4 text-slate-400">{t('inspector.version')}</div>
              <div className="col-span-8 text-slate-200">{mod.version || '1.0'}</div>
            </div>

            <div className="grid grid-cols-12 px-3.5 py-1.5 items-center">
              <div className="col-span-4 text-slate-400">{t('inspector.author')}</div>
              <div className="col-span-8 text-cyan-300 font-bold">{mod.author || t('inspector.unknown')}</div>
            </div>

            {mod.url && (
              <div className="grid grid-cols-12 px-3.5 py-1.5 items-center">
                <div className="col-span-4 text-slate-400">{t('inspector.homepage')}</div>
                <div className="col-span-8">
                  <button
                    onClick={() => TauriService.openExternalUrl(mod.url!)}
                    className="text-cyan-400 hover:underline font-mono truncate max-w-full text-left"
                  >
                    {mod.url}
                  </button>
                </div>
              </div>
            )}

            <div className="grid grid-cols-12 px-3.5 py-1.5 items-center">
              <div className="col-span-4 text-slate-400">{t('inspector.modId')}</div>
              <div className="col-span-8 text-emerald-400 font-bold">{mod.mod_id}</div>
            </div>

            <div className="grid grid-cols-12 px-3.5 py-1.5 items-center">
              <div className="col-span-4 text-slate-400">{t('inspector.workshopId')}</div>
              <div className="col-span-8 text-slate-300">{mod.workshop_id || t('inspector.builtIn')}</div>
            </div>

            <div className="grid grid-cols-12 px-3.5 py-1.5 items-center">
              <div className="col-span-4 text-slate-400">{t('inspector.source')}</div>
              <div className="col-span-8 text-slate-300">{mod.workshop_id ? t('inspector.steamWorkshop') : t('inspector.localDirectory')}</div>
            </div>

            <div className="grid grid-cols-12 px-3.5 py-1.5 items-center">
              <div className="col-span-4 text-slate-400">{t('inspector.zomboidVersion')}</div>
              <div className="col-span-8 text-slate-300">{mod.pzversion || '42.0+'}</div>
            </div>

            <div className="grid grid-cols-12 px-3.5 py-1.5 items-start">
              <div className="col-span-4 text-slate-400 pt-0.5">{t('inspector.incompatibleWith')}</div>
              <div className="col-span-8">
                {mod.incompatible && mod.incompatible.length > 0 ? (
                  <div className="flex flex-wrap gap-1">
                    {mod.incompatible.map((incRaw, idx) => (
                      <span key={idx} className="text-amber-400 font-bold">{incRaw}</span>
                    ))}
                  </div>
                ) : (
                  <span className="text-slate-600 italic">-</span>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* SECTION 1: Base Libraries (Library Required by this Mod) */}
        <div className="space-y-1 border-t border-slate-800 pt-3">
          <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider flex items-center justify-between">
            <span className="flex items-center gap-1.5 text-emerald-400 font-bold">
              <Package className="w-3.5 h-3.5 text-emerald-400" />
              {t('inspector.libraryRequired', { count: parentLibraries.length })}
            </span>
          </label>
          <div className="flex flex-wrap gap-1.5 pt-1">
            {parentLibraries.length > 0 ? (
              parentLibraries.map((item, idx) => {
                const matched = item.matchedMod;
                const isDepDisabled = matched && !matched.enabled;
                return (
                  <button
                    key={idx}
                    onClick={() => onJumpToDependency(item.raw)}
                    className={`px-3 py-1.5 text-xs font-mono font-bold rounded-lg border transition cursor-pointer flex items-center gap-2 shadow-sm ${
                      isDepDisabled
                        ? 'bg-rose-950/80 border-rose-700 text-rose-200 hover:border-rose-400 shadow'
                        : matched
                        ? 'bg-emerald-950/80 border-emerald-700 text-emerald-200 hover:border-emerald-400 shadow'
                        : 'bg-red-950/80 border-red-700 text-red-200 hover:border-red-400 shadow'
                    }`}
                    title={
                      matched
                        ? t('inspector.parentTitle', { name: matched.name, state: matched.enabled ? t('inspector.active') : t('inspector.disabledUpper') })
                        : t('inspector.notInstalledTitle', { name: item.raw })
                    }
                  >
                    <span className={`w-2 h-2 rounded-full ${isDepDisabled ? 'bg-rose-400 animate-pulse' : matched ? 'bg-emerald-400' : 'bg-red-400'}`} />
                    <span>{matched ? matched.name : item.raw}</span>
                    <ExternalLink className="w-3 h-3 text-emerald-400 shrink-0 ml-0.5" />
                  </button>
                );
              })
            ) : (
              <span className="text-xs text-slate-500 italic">{t('inspector.noLibrary')}</span>
            )}
          </div>
        </div>

        {/* SECTION 2: Reverse Dependents (Mods Requiring This Library) */}
        <div className="space-y-1 border-t border-slate-800 pt-3">
          <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider flex items-center justify-between">
            <span className="flex items-center gap-1.5 text-purple-400 font-bold">
              <Link2 className="w-3.5 h-3.5 text-purple-400" />
              {t('inspector.modsRequiring', { count: reverseDependents.length })}
            </span>
          </label>
          <div className="flex flex-wrap gap-1.5 pt-1">
            {reverseDependents.length > 0 ? (
              reverseDependents.map((depMod) => (
                <button
                  key={depMod.mod_id}
                  onClick={() => onJumpToMod(depMod.mod_id)}
                  className={`px-3 py-1.5 text-xs font-mono font-bold rounded-lg border transition cursor-pointer flex items-center gap-2 shadow-sm ${
                    depMod.enabled
                      ? 'bg-purple-950/80 border-purple-700 text-purple-200 hover:border-purple-400 shadow'
                      : 'bg-slate-950 border-slate-800 text-slate-500 hover:text-slate-300'
                  }`}
                  title={t('inspector.childTitle', { name: depMod.name, state: depMod.enabled ? t('inspector.active') : t('inspector.disabledUpper') })}
                >
                  <span className={`w-2 h-2 rounded-full ${depMod.enabled ? 'bg-emerald-400' : 'bg-slate-600'}`} />
                  <span>{depMod.name}</span>
                </button>
              ))
            ) : (
              <span className="text-xs text-slate-500 italic">{t('inspector.noDependents')}</span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
