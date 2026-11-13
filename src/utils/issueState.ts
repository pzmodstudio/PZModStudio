import { AlertCircle, Lightbulb, Info, ShieldCheck } from 'lucide-react';
import type React from 'react';
import { HealthFix, HealthIssue, IssueState } from '../types';

/**
 * Whose move an issue is, in the four words every screen uses (Mod Health, the Mod List's chips):
 * the game breaks until you act; acting makes it better; nothing to do; the Studio already fixed
 * it. The backend sets `state`; an older one left it out, so the severity stands in.
 */
export const issueStateOf = (issue: HealthIssue): IssueState =>
  issue.state ?? (issue.severity === 'error' ? 'act' : issue.severity === 'warning' ? 'recommended' : 'info');

/** Most pressing first. */
export const STATE_ORDER: IssueState[] = ['act', 'recommended', 'info', 'handled'];

export const STATE_STYLE: Record<IssueState, { icon: React.ElementType; card: string; iconColor: string; label: string; chip: string; badge: string }> = {
  act: {
    icon: AlertCircle,
    card: 'border-red-500/50 bg-red-950/30',
    iconColor: 'text-red-400',
    label: 'text-red-300',
    chip: 'border-red-600/70 text-red-200 bg-red-950/60',
    badge: 'bg-red-600 text-white',
  },
  recommended: {
    icon: Lightbulb,
    card: 'border-amber-500/40 bg-amber-950/20',
    iconColor: 'text-amber-400',
    label: 'text-amber-300',
    chip: 'border-amber-600/70 text-amber-200 bg-amber-950/50',
    badge: 'bg-amber-500 text-slate-950',
  },
  info: {
    icon: Info,
    card: 'border-slate-700 bg-slate-900/50',
    iconColor: 'text-slate-400',
    label: 'text-slate-400',
    chip: 'border-slate-700 text-slate-300 bg-slate-900',
    badge: 'bg-slate-700 text-slate-200',
  },
  handled: {
    icon: ShieldCheck,
    card: 'border-emerald-700/50 bg-emerald-950/15',
    iconColor: 'text-emerald-400',
    label: 'text-emerald-300',
    chip: 'border-emerald-700/70 text-emerald-200 bg-emerald-950/50',
    badge: 'bg-emerald-700 text-white',
  },
};

/**
 * Fixes that undo something the Studio did. On a handled row they are not a thing to do: shown
 * quietly, as "undo".
 */
export const UNDO_ACTIONS = new Set<HealthFix['action']>(['remove_compatible_copy', 'remove_from_package', 'dismiss_known_issue']);

/**
 * Whose problem a finding is. "mod": the mod against the installed game, the same for everyone on
 * this build (worth telling its author). "combo": this mod beside the others on, which changes from
 * one player's list to the next. "install": this PC's own files and copies. "own": a mod the player
 * is making (its folder is linked from elsewhere), where "the mod" would send them to an author
 * who is themselves. Unknown codes get none.
 */
export type IssueScope = 'mod' | 'combo' | 'install' | 'own';
const SCOPE_MOD = new Set(['not_for_build', 'outside_version_range', 'invalid_mod_info', 'community_known_issue', 'api_missing', 'floods_console']);
const SCOPE_COMBO = new Set([
  'incompatible', 'community_incompatible', 'dependency_after', 'dependency_not_enabled', 'dependency_unavailable', 'missing_dependency',
  'load_after_order', 'load_before_order', 'community_rule_order', 'user_rule_order', 'translation_conflict', 'two_mod_managers', 'community_successor',
  'java_patch_overlap',
]);
const SCOPE_INSTALL = new Set([
  'compatible_copy', 'compatible_copy_unneeded', 'shadowed_copy', 'duplicate_entry', 'missing_mod', 'translation_settled', 'translation_stale',
  'server_item_unused', 'updated_since_last_play', 'workshop_update_pending', 'script_definition_settled',
]);
export const issueScopeOf = (code: string): IssueScope | null =>
  SCOPE_MOD.has(code) ? 'mod'
    : SCOPE_COMBO.has(code) || code.includes('conflict') ? 'combo'
    : SCOPE_INSTALL.has(code) || code.startsWith('package_') || code.startsWith('workshop_copy') ? 'install'
    : null;
/** The scope of an issue: its code's, with "the mod" said as "your mod" when the mod is the player's own. */
export const scopeOfIssue = (issue: { code: string; own_mod?: boolean }): IssueScope | null => {
  const scope = issueScopeOf(issue.code);
  return scope === 'mod' && issue.own_mod ? 'own' : scope;
};
export const SCOPE_CHIP: Record<IssueScope, string> = {
  own: 'border-emerald-700/70 text-emerald-300',
  mod: 'border-violet-700/70 text-violet-300',
  combo: 'border-amber-700/70 text-amber-300',
  install: 'border-slate-600 text-slate-400',
};
