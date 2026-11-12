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
