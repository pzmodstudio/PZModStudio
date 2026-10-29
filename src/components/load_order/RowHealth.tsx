import { openRequiredItem } from '../../utils/workshopRequirement';
import React from 'react';
import { AlertTriangle, Info, Wrench, XCircle } from 'lucide-react';
import { HealthFix, HealthIssue, HealthSeverity, ModInfo } from '../../types';
import { TauriService } from '../../services/tauri';
import { StudioPathsUI } from '../settings/SettingsModule';
import { applyFix } from '../../utils/healthFix';
import i18next from 'i18next';
import { issueText } from '../../i18n/backendText';
import { Tip, TipBody } from '../common/Tip';
import { RichText } from '../common/RichText';

/** What a row says for each code the health check can raise. Short: the row is narrow. */
/** Settled things are Mod Health's record, not a row's problem. */
const SETTLED_CODES = new Set(['script_definition_settled', 'translation_settled', 'compatible_copy']);
/** Codes the row shows as its own tags; exported for the Mod List's tag filter. */
export const rowIssueCodes = (issues: HealthIssue[]) => issues.filter((i) => !SETTLED_CODES.has(i.code)).map((i) => i.code);
/** A row has room for this many chips; the rest fold into one that opens Mod Health. */
const MAX_CHIPS = 2;
/** Issues about a group of files and mods, which only Mod Health shows whole. */
const HEALTH_ONLY_CODES = new Set(['script_definition_conflict', 'script_definition_stale', 'script_definition_settled', 'script_definition_conflict_more', 'translation_conflict', 'translation_stale', 'translation_settled']);

export const labelFor = (issue: Pick<HealthIssue, 'code'>) => i18next.t(`modList.health.labels.${issue.code}`, { defaultValue: issue.code.replace(/_/g, ' ') });

const STYLE: Record<HealthSeverity, { chip: string; icon: React.ElementType; iconColor: string; tone: 'red' | 'amber' | 'slate' }> = {
  error: { chip: 'bg-red-500/20 hover:bg-red-500/35 text-red-200 border-red-500/60', icon: XCircle, iconColor: 'text-red-400', tone: 'red' },
  warning: { chip: 'bg-amber-500/20 hover:bg-amber-500/35 text-amber-200 border-amber-500/60', icon: AlertTriangle, iconColor: 'text-amber-400', tone: 'amber' },
  info: { chip: 'bg-slate-800/80 hover:bg-slate-700/80 text-slate-300 border-slate-600', icon: Info, iconColor: 'text-slate-400', tone: 'slate' },
};

const ORDER: Record<HealthSeverity, number> = { error: 0, warning: 1, info: 2 };

/** Fixes the row can apply by rewriting the list, the way Mod Health does. */
const LIST_FIX_ACTIONS = new Set<HealthFix['action']>(['enable_mod', 'disable_mod', 'move_before', 'remove_from_list']);

const fixLabel = (fix: HealthFix, nameOf: (id: string) => string): string => {
  const t = i18next.t;
  switch (fix.action) {
    case 'enable_mod':
      return t('modList.health.fix.enable', { name: nameOf(fix.mod_id) });
    case 'disable_mod':
      return t('modList.health.fix.disable', { name: nameOf(fix.mod_id) });
    case 'move_before':
      return t('modList.health.fix.moveBefore', { name: nameOf(fix.before) });
    case 'remove_from_list':
      return t('modList.health.fix.remove');
    case 'subscribe_workshop':
      return t('modList.health.fix.searchWorkshop');
    case 'open_workshop_page':
      return t('modList.health.fix.openWorkshop');
    case 'open_in_merger':
      return t('modList.health.fix.openMerger');
    case 'keep_version':
      return t('modList.health.fix.keepVersion');
    case 'enable_shim_patch':
      return t('modList.health.fix.shimOn');
    case 'merge_files':
    case 'make_compatible_copy':
    case 'remove_compatible_copy':
    case 'remove_from_package':
      return t('modList.health.fix.inHealth');
  }
};

interface RowHealthChipsProps {
  paths: StudioPathsUI;
  issues: HealthIssue[];
  mods: ModInfo[];
  /** Replaces the list (order + switches) and persists it. */
  onApplyList: (mods: ModInfo[]) => void;
  onJumpToMod: (modId: string) => void;
  onOpenMerger?: (relativePath: string) => void;
  /** Opens Mod Health, scrolled to the issue when one is given. */
  onGoToHealth?: (issue?: HealthIssue) => void;
  /** Opens a Workshop item / a Workshop search in Explore Mods. */
  onOpenInBrowser?: (workshopId: string) => void;
  onSearchInBrowser?: (text: string) => void;
  /** A line for the list's toast, for a fix that happened on disk. */
  onNote?: (message: string) => void;
}

/**
 * The health check's findings for one mod, on its row: one chip per problem, worst first, with
 * the fix beside it when the row can apply it. The chip itself goes to the other mod involved,
 * or to Mod Health when there is none.
 */
export const RowHealthChips: React.FC<RowHealthChipsProps> = ({ paths, issues, mods, onApplyList, onJumpToMod, onOpenMerger, onGoToHealth, onOpenInBrowser, onSearchInBrowser, onNote }) => {
  if (issues.length === 0) return null;
  const nameOf = (id: string) => mods.find((m) => m.mod_id === id)?.name ?? id;
  // The names and ids a message may mention, set in bold in the tips.
  const modWords = mods.flatMap((m) => [m.name, m.mod_id]);

  // One chip per kind of problem, worst first; the message of the first is the tooltip. What is
  // already settled stays off the row, and past two chips the rest fold into one, so the mod's
  // name stays readable.
  const seen = new Set<string>();
  const all = [...issues]
    .filter((i) => !SETTLED_CODES.has(i.code))
    .sort((a, b) => ORDER[a.severity] - ORDER[b.severity])
    .filter((i) => (seen.has(i.code) ? false : (seen.add(i.code), true)));
  if (all.length === 0) return null;
  const shown = all.slice(0, MAX_CHIPS);
  const folded = all.slice(MAX_CHIPS);

  /** What the fix button does beyond its label, when that needs saying. */
  const fixHint = (fix: HealthFix): string =>
    LIST_FIX_ACTIONS.has(fix.action)
      ? i18next.t('modList.health.listFix')
      : fix.action === 'open_in_merger'
      ? i18next.t('modList.health.mergerFix', { path: fix.relative_path })
      : fix.action === 'merge_files' || fix.action === 'make_compatible_copy' || fix.action === 'remove_compatible_copy' || fix.action === 'remove_from_package'
      ? i18next.t('modList.health.healthFix')
      : '';

  const runFix = (fix: HealthFix, issue: HealthIssue) => {
    switch (fix.action) {
      case 'subscribe_workshop':
        // The item that provides it, from the requiring mod's Required items; a search otherwise.
        void openRequiredItem(
          issue.mod_id,
          fix.hint,
          mods,
          (id) => (onOpenInBrowser ? onOpenInBrowser(id) : TauriService.openExternalUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${id}`)),
          (text) => (onSearchInBrowser ? onSearchInBrowser(text) : TauriService.openExternalUrl(`https://steamcommunity.com/workshop/browse/?appid=108600&searchtext=${encodeURIComponent(text)}`))
        );
        return;
      case 'open_workshop_page':
        if (onOpenInBrowser) onOpenInBrowser(fix.workshop_id);
        else TauriService.openExternalUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${fix.workshop_id}`);
        return;
      case 'open_in_merger':
        if (onOpenMerger) onOpenMerger(fix.relative_path);
        else onGoToHealth?.();
        return;
      case 'keep_version':
        void TauriService.keepModVersion(paths, fix.mod_id).then((kept) =>
          onNote?.(i18next.t('modList.health.kept', { name: kept.mod_name, when: kept.kept_at }))
        );
        return;
      case 'enable_shim_patch':
        void TauriService.setShimPatch(paths.user_zomboid_dir, fix.name, true).then((written) =>
          onNote?.(i18next.t('modList.health.shimOn', { name: fix.name, count: written }))
        );
        return;
      case 'merge_files':
      case 'make_compatible_copy':
      case 'remove_compatible_copy':
      case 'remove_from_package':
        onGoToHealth?.(issue);
        return;
      case 'disable_mod':
        if (!confirm(i18next.t('modList.health.confirmDisable', { name: nameOf(fix.mod_id) }))) return;
        break;
      default:
        break;
    }
    const next = applyFix(mods, fix);
    if (next) onApplyList(next);
  };

  return (
    <>
      {shown.map((issue) => {
        const style = STYLE[issue.severity];
        const Icon = style.icon;
        // The chip goes to the other mod only when there is exactly one (a dependency, an order
        // pair); a group of mods and files is Mod Health's to show whole.
        const others = issue.related.filter((id) => mods.some((m) => m.mod_id === id));
        const other = others.length === 1 && !HEALTH_ONLY_CODES.has(issue.code) ? others[0] : undefined;
        const fix = issue.fixes[0];
        return (
          <span key={issue.code} className="flex items-center shrink-0">
            <Tip
              content={
                <TipBody
                  title={labelFor(issue)}
                  tone={style.tone}
                  action={
                    other
                      ? i18next.t('modList.health.clickGoTo', { name: nameOf(other) })
                      : others.length > 1
                        ? i18next.t('modList.health.clickHealthAll', { count: others.length + 1 })
                        : i18next.t('modList.health.clickHealth')
                  }
                >
                  <RichText text={issueText(issue)} mods={modWords} foldAt={260} />
                </TipBody>
              }
            >
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  if (other) onJumpToMod(other);
                  else onGoToHealth?.(issue);
                }}
                className={`flex items-center gap-1 text-[9.5px] font-mono font-bold px-1.5 py-0.5 border transition cursor-pointer shadow-sm ${style.chip} ${
                  fix ? 'rounded-l' : 'rounded'
                }`}
              >
                <Icon className={`w-3 h-3 shrink-0 ${style.iconColor}`} aria-hidden="true" />
                <span>{labelFor(issue)}</span>
              </button>
            </Tip>
            {fix && (
              <Tip
                content={
                  <TipBody title={fixLabel(fix, nameOf)} tone="cyan">
                    {fixHint(fix) && <RichText mods={modWords} text={fixHint(fix)} />}
                  </TipBody>
                }
              >
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    runFix(fix, issue);
                  }}
                  className={`flex items-center gap-1 text-[9.5px] font-mono font-bold px-1.5 py-0.5 rounded-r border border-l-0 transition cursor-pointer shadow-sm ${style.chip}`}
                >
                  <Wrench className="w-3 h-3 shrink-0" aria-hidden="true" />
                  <span>{fixLabel(fix, nameOf)}</span>
                </button>
              </Tip>
            )}
          </span>
        );
      })}
      {folded.length > 0 && (
        <Tip
          content={
            <TipBody title={i18next.t('modList.health.moreTitle', { count: folded.length })} action={i18next.t('modList.health.moreAction')}>
              <ul className="space-y-0.5">
                {folded.map((i) => {
                  const FoldIcon = STYLE[i.severity].icon;
                  return (
                    <li key={i.code} className="flex items-center gap-1.5">
                      <FoldIcon className={`w-3 h-3 shrink-0 ${STYLE[i.severity].iconColor}`} aria-hidden="true" />
                      <span>{labelFor(i)}</span>
                    </li>
                  );
                })}
              </ul>
            </TipBody>
          }
        >
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onGoToHealth?.(folded[0]);
            }}
            className="flex items-center text-[9.5px] font-mono font-bold px-1.5 py-0.5 rounded border transition cursor-pointer shadow-sm bg-slate-800/80 hover:bg-slate-700/80 text-slate-300 border-slate-600 shrink-0"
          >
            +{folded.length}
          </button>
        </Tip>
      )}
    </>
  );
};
