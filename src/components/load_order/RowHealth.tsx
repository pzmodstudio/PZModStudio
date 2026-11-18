import { openRequiredItem } from '../../utils/workshopRequirement';
import React from 'react';
import { ArrowRight, Info, Wrench } from 'lucide-react';
import { HealthFix, HealthIssue, IssueState, ModInfo } from '../../types';
import { STATE_ORDER, STATE_STYLE, issueStateOf } from '../../utils/issueState';
import { TauriService } from '../../services/tauri';
import { StudioPathsUI } from '../settings/SettingsModule';
import { applyFix } from '../../utils/healthFix';
import i18next from 'i18next';
import { issueText } from '../../i18n/backendText';
import { Tip, TipBody } from '../common/Tip';
import { RichText } from '../common/RichText';
import { useUiMode } from '../../hooks/useUiMode';
import { UiMode, fixShownIn, simpleRowIssues } from '../../utils/uiMode';

/** What a row says for each code the health check can raise. Short: the row is narrow. */
/** What the Studio already fixed is Mod Health's record, not a row's problem: no chip. */
const onRow = (i: HealthIssue) => issueStateOf(i) !== 'handled';
/** Codes the row shows as its own tags; exported for the Mod List's tag filter. */
export const rowIssueCodes = (issues: HealthIssue[]) => issues.filter(onRow).map((i) => i.code);
/** A row has room for this many chips; the rest fold into one that opens Mod Health. */
const MAX_CHIPS = 2;
/** Issues about a group of files and mods, which only Mod Health shows whole. */
const HEALTH_ONLY_CODES = new Set(['script_definition_conflict', 'script_definition_stale', 'script_definition_settled', 'script_definition_conflict_more', 'translation_conflict', 'translation_stale', 'translation_settled']);

/** The short name of a finding; Simple says the ones in the Studio's own words in plain ones. */
export const labelFor = (issue: Pick<HealthIssue, 'code'>, mode: UiMode = 'advanced') => {
  const label = i18next.t(`modList.health.labels.${issue.code}`, { defaultValue: issue.code.replace(/_/g, ' ') });
  return mode === 'simple' ? i18next.t(`modList.health.simpleLabels.${issue.code}`, { defaultValue: label }) : label;
};

/** The same four states and colours as Mod Health (handled ones never reach a row). */
const STYLE: Record<IssueState, { chip: string; tone: 'red' | 'amber' | 'slate' }> = {
  act: { chip: 'bg-red-500/20 hover:bg-red-500/35 text-red-200 border-red-500/60', tone: 'red' },
  recommended: { chip: 'bg-amber-500/20 hover:bg-amber-500/35 text-amber-200 border-amber-500/60', tone: 'amber' },
  info: { chip: 'bg-slate-800/80 hover:bg-slate-700/80 text-slate-300 border-slate-600', tone: 'slate' },
  handled: { chip: 'bg-emerald-900/40 hover:bg-emerald-800/50 text-emerald-200 border-emerald-700/60', tone: 'slate' },
};

/** Fixes the row can apply by rewriting the list, the way Mod Health does. */
const LIST_FIX_ACTIONS = new Set<HealthFix['action']>(['enable_mod', 'disable_mod', 'move_before', 'remove_from_list', 'replace_in_list']);

/** `rowModId`: the mod whose row shows the fix; a move of another mod above it names that one. */
export const fixLabel = (fix: HealthFix, nameOf: (id: string) => string, rowModId?: string | null): string => {
  const t = i18next.t;
  switch (fix.action) {
    case 'enable_mod':
      return t('modList.health.fix.enable', { name: nameOf(fix.mod_id) });
    case 'disable_mod':
      return t('modList.health.fix.disable', { name: nameOf(fix.mod_id) });
    case 'move_before':
      return fix.before === rowModId && fix.mod_id !== rowModId
        ? t('modList.health.fix.moveAbove', { name: nameOf(fix.mod_id) })
        : t('modList.health.fix.moveBefore', { name: nameOf(fix.before) });
    case 'remove_from_list':
      return t('modList.health.fix.remove');
    case 'replace_in_list':
      return t('modList.health.fix.replace', { name: nameOf(fix.mod_id) });
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
    case 'dismiss_known_issue':
    case 'adopt_workshop_copy':
    case 'recopy_workshop':
    case 'update_package':
      return t('modList.health.fix.inHealth');
  }
};

/** What a fix needs to run from a row or the inspector. */
export interface FixContext {
  paths: StudioPathsUI;
  mods: ModInfo[];
  onApplyList: (mods: ModInfo[]) => void;
  onOpenMerger?: (relativePath: string) => void;
  onGoToHealth?: (issue?: HealthIssue) => void;
  onOpenInBrowser?: (workshopId: string) => void;
  onSearchInBrowser?: (text: string) => void;
  onNote?: (message: string) => void;
}

/** What the fix button does beyond its label, when that needs saying. */
export const fixHint = (fix: HealthFix): string =>
    LIST_FIX_ACTIONS.has(fix.action)
      ? i18next.t('modList.health.listFix')
      : fix.action === 'open_in_merger'
      ? i18next.t('modList.health.mergerFix', { path: fix.relative_path })
      : fix.action === 'merge_files' || fix.action === 'make_compatible_copy' || fix.action === 'remove_compatible_copy' || fix.action === 'remove_from_package'
      ? i18next.t('modList.health.healthFix')
      : '';

/** What a chip's or a card's fix button runs: the list rewritten, a page opened, or Mod Health. */
export const makeFixRunner = ({ paths, mods, onApplyList, onOpenMerger, onGoToHealth, onOpenInBrowser, onSearchInBrowser, onNote }: FixContext) => {
  const nameOf = (id: string) => mods.find((m) => m.mod_id === id)?.name ?? id;
  return (fix: HealthFix, issue: HealthIssue) => {
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
      case 'dismiss_known_issue':
      case 'adopt_workshop_copy':
      case 'recopy_workshop':
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
export const RowHealthChips: React.FC<RowHealthChipsProps> = ({ paths, issues: allIssues, mods, onApplyList, onJumpToMod, onOpenMerger, onGoToHealth, onOpenInBrowser, onSearchInBrowser, onNote }) => {
  // Simple: only what needs something done gets a chip (the same rows its Diagnostics shows).
  const [mode] = useUiMode();
  const issues = mode === 'simple' ? simpleRowIssues(allIssues) : allIssues;
  if (issues.length === 0) return null;
  const nameOf = (id: string) => mods.find((m) => m.mod_id === id)?.name ?? id;
  // The names and ids a message may mention, set in bold in the tips.
  const modWords = mods.flatMap((m) => [m.name, m.mod_id]);

  // One chip per kind of problem, worst first; the message of the first is the tooltip. What is
  // already settled stays off the row, and past two chips the rest fold into one, so the mod's
  // name stays readable.
  const seen = new Set<string>();
  const all = [...issues]
    .filter(onRow)
    .sort((a, b) => STATE_ORDER.indexOf(issueStateOf(a)) - STATE_ORDER.indexOf(issueStateOf(b)))
    .filter((i) => (seen.has(i.code) ? false : (seen.add(i.code), true)));
  if (all.length === 0) return null;
  // Only what asks for something gets a chip of its own; "nothing to do" is the inspector's to
  // say, and folds with the rest into one quiet chip.
  const actionable = all.filter((i) => issueStateOf(i) !== 'info');
  const shown = actionable.slice(0, MAX_CHIPS);
  const folded = [...actionable.slice(MAX_CHIPS), ...all.filter((i) => issueStateOf(i) === 'info')];
  const onlyInfo = folded.every((i) => issueStateOf(i) === 'info');

  const runFix = makeFixRunner({ paths, mods, onApplyList, onOpenMerger, onGoToHealth, onOpenInBrowser, onSearchInBrowser, onNote });

  return (
    <>
      {shown.map((issue) => {
        const state = issueStateOf(issue);
        const style = STYLE[state];
        const Icon = STATE_STYLE[state].icon;
        // The chip goes to the other mod only when there is exactly one (a dependency, an order
        // pair); a group of mods and files is Mod Health's to show whole.
        const others = issue.related.filter((id) => mods.some((m) => m.mod_id === id));
        const other = others.length === 1 && !HEALTH_ONLY_CODES.has(issue.code) ? others[0] : undefined;
        // Only a row that wants something done offers its fix: on one that is only worth knowing, a
        // button reads as a thing to do.
        const firstFix = state === 'act' || state === 'recommended' ? issue.fixes[0] : undefined;
        // A fix that opens a Complete-only screen is not offered in Simple: the chip leads to Diagnostics.
        const fix = firstFix && fixShownIn(mode, firstFix) ? firstFix : undefined;
        return (
          <span key={issue.code} className="flex items-center shrink-0">
            <Tip
              content={
                <TipBody
                  title={`${i18next.t(`health.state.${state}.word`)}: ${labelFor(issue, mode)}`}
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
                <Icon className={`w-3 h-3 shrink-0 ${STATE_STYLE[state].iconColor}`} aria-hidden="true" />
                <span>{labelFor(issue, mode)}</span>
              </button>
            </Tip>
            {fix && (
              <Tip
                content={
                  <TipBody title={fixLabel(fix, nameOf, issue.mod_id)} tone="cyan">
                    <RichText mods={modWords} text={[i18next.t(`health.effect.${fix.action}`), fixHint(fix)].filter(Boolean).join(' ')} />
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
                  <span>{fixLabel(fix, nameOf, issue.mod_id)}</span>
                </button>
              </Tip>
            )}
          </span>
        );
      })}
      {folded.length > 0 && (
        <Tip
          content={
            <TipBody title={onlyInfo ? i18next.t('modList.health.infoTitle', { count: folded.length }) : i18next.t('modList.health.moreTitle', { count: folded.length })} action={i18next.t('modList.health.moreAction')} footnote={i18next.t('modList.health.moreInPanel')}>
              <ul className="space-y-0.5">
                {folded.map((i) => {
                  const FoldIcon = STATE_STYLE[issueStateOf(i)].icon;
                  return (
                    <li key={i.code} className="flex items-center gap-1.5">
                      <FoldIcon className={`w-3 h-3 shrink-0 ${STATE_STYLE[issueStateOf(i)].iconColor}`} aria-hidden="true" />
                      <span>{labelFor(i, mode)}</span>
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
            className="flex items-center gap-0.5 text-[9.5px] font-mono font-bold px-1.5 py-0.5 rounded border transition cursor-pointer shadow-sm bg-slate-800/80 hover:bg-slate-700/80 text-slate-300 border-slate-600 shrink-0"
          >
            {onlyInfo ? <Info className="w-3 h-3 text-slate-400" aria-hidden="true" /> : '+'}
            {folded.length}
          </button>
        </Tip>
      )}
    </>
  );
};

interface IssueCardsProps extends FixContext {
  issues: HealthIssue[];
  onJumpToMod: (modId: string) => void;
}

/**
 * The inspector's "what to do" list: every finding for the selected mod, worst first, as a card
 * with its state word (act / recommended / nothing to do), the whole message, and the buttons:
 * its fix when it asks for one, the other mod when there is exactly one, and Mod Health.
 */
export const IssueCards: React.FC<IssueCardsProps> = ({ issues: allIssues, onJumpToMod, ...ctx }) => {
  const { mods, onGoToHealth } = ctx;
  const [mode] = useUiMode();
  const issues = mode === 'simple' ? simpleRowIssues(allIssues) : allIssues;
  const nameOf = (id: string) => mods.find((m) => m.mod_id === id)?.name ?? id;
  const modWords = mods.flatMap((m) => [m.name, m.mod_id]);
  const seen = new Set<string>();
  const list = [...issues]
    .filter(onRow)
    .sort((a, b) => STATE_ORDER.indexOf(issueStateOf(a)) - STATE_ORDER.indexOf(issueStateOf(b)))
    .filter((i) => {
      const key = `${i.code}|${i.related.join(',')}`;
      return seen.has(key) ? false : (seen.add(key), true);
    });
  if (list.length === 0) return null;
  const runFix = makeFixRunner(ctx);
  return (
    <div className="flex flex-col gap-2">
      {list.map((issue, n) => {
        const state = issueStateOf(issue);
        const Icon = STATE_STYLE[state].icon;
        const others = issue.related.filter((id) => mods.some((m) => m.mod_id === id));
        const other = others.length === 1 && !HEALTH_ONLY_CODES.has(issue.code) ? others[0] : undefined;
        const firstFix = state === 'act' || state === 'recommended' ? issue.fixes[0] : undefined;
        const fix = firstFix && fixShownIn(mode, firstFix) ? firstFix : undefined;
        return (
          <div key={`${issue.code}-${n}`} className={`rounded-lg border px-3 py-2 text-[11.5px] ${STATE_STYLE[state].card}`}>
            <div className="flex items-start gap-2">
              <Icon className={`w-3.5 h-3.5 mt-0.5 shrink-0 ${STATE_STYLE[state].iconColor}`} aria-hidden="true" />
              <div className="min-w-0 flex-1">
                <div className="flex items-baseline gap-2 flex-wrap">
                  <span className={`font-bold ${STATE_STYLE[state].label}`}>{labelFor(issue, mode)}</span>
                  <span className="text-[10px] uppercase tracking-wider text-slate-500">{i18next.t(`health.state.${state}.word`)}</span>
                </div>
                <div className="mt-0.5 text-slate-300 leading-relaxed">
                  <RichText text={issueText(issue)} mods={modWords} foldAt={220} />
                </div>
                <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
                  {fix && (
                    <Tip content={<TipBody title={fixLabel(fix, nameOf, issue.mod_id)} tone="cyan"><RichText mods={modWords} text={[i18next.t(`health.effect.${fix.action}`), fixHint(fix)].filter(Boolean).join(' ')} /></TipBody>}>
                      <button
                        type="button"
                        onClick={() => runFix(fix, issue)}
                        className="flex items-center gap-1 px-2 py-1 rounded-md bg-emerald-700 hover:bg-emerald-600 text-white text-[11px] font-bold cursor-pointer"
                      >
                        <Wrench className="w-3 h-3" aria-hidden="true" />
                        {fixLabel(fix, nameOf, issue.mod_id)}
                      </button>
                    </Tip>
                  )}
                  {other && (
                    <button
                      type="button"
                      onClick={() => onJumpToMod(other)}
                      className="flex items-center gap-1 px-2 py-1 rounded-md border border-slate-700 hover:bg-slate-800 text-slate-300 text-[11px] cursor-pointer"
                    >
                      <ArrowRight className="w-3 h-3" aria-hidden="true" />
                      {i18next.t('inspector.issues.goTo', { name: nameOf(other) })}
                    </button>
                  )}
                  {onGoToHealth && (
                    <button
                      type="button"
                      onClick={() => onGoToHealth(issue)}
                      className="px-2 py-1 rounded-md text-cyan-300 hover:bg-slate-800 text-[11px] cursor-pointer"
                    >
                      {i18next.t('inspector.issues.inHealth')}
                    </button>
                  )}
                </div>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
};
