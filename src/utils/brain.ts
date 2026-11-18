import type { TFunction } from 'i18next';
import { BrainChange, BrainFixOutcome, BrainJvmCrash, BrainOnset, BrainOverview, BrainPrelaunch, BrainSessionBrief } from '../types';

/**
 * How the brain's facts (rules::brain) are said: dates, what changed, a fix's outcome, a session.
 * Every sentence names what it was read from; "what changed" is said as what was different then,
 * never as the cause.
 */

/** A unix time as a short local date and time ("17/11 05:47"). */
export const when = (secs: number | null | undefined): string => {
  if (!secs) return '';
  const d = new Date(secs * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(d.getDate())}/${pad(d.getMonth() + 1)} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
};

/** A unix time as a day ("17/11"). */
export const day = (secs: number | null | undefined): string => when(secs).split(' ')[0] ?? '';

/** One change in words. */
export const changeText = (c: BrainChange, name: (id: string) => string, t: TFunction): string => {
  switch (c.kind) {
    case 'build':
      return t('brain.change.build', { from: c.from, to: c.to });
    case 'added':
      return t('brain.change.added', { mod: name(c.mod_id) });
    case 'removed':
      return t('brain.change.removed', { mod: name(c.mod_id) });
    case 'updated':
      return c.from && c.to && c.from !== c.to
        ? t('brain.change.updatedFromTo', { mod: name(c.mod_id), from: c.from, to: c.to })
        : t('brain.change.updated', { mod: name(c.mod_id) });
    case 'moved':
      return t('brain.change.moved', { mod: name(c.mod_id), from: c.from, to: c.to });
    default:
      return '';
  }
};

/** The changes, joined, with how many more. */
export const changesText = (changes: BrainChange[], more: number, name: (id: string) => string, t: TFunction): string => {
  if (changes.length === 0) return t('brain.change.none');
  // The first ones (the build, the mod, what it requires) are what matter; the rest is counted.
  const SHOWN = 8;
  const said = changes.slice(0, SHOWN).map((c) => changeText(c, name, t)).filter(Boolean).join('; ');
  const left = more + Math.max(0, changes.length - SHOWN);
  return left > 0 ? `${said}; ${t('brain.change.more', { count: left })}` : said;
};

/** When something began, and what was different then. */
export const onsetText = (o: BrainOnset, name: (id: string) => string, t: TFunction): string => {
  if (!o.last_good) return t('brain.onset.fromStart', { first: when(o.first_bad) });
  return t('brain.onset.text', { first: when(o.first_bad), good: when(o.last_good), changes: changesText(o.changes, o.more, name, t) });
};

/** A fix and what the sessions after it say. */
export const fixText = (f: BrainFixOutcome, name: (id: string) => string, t: TFunction): string =>
  t(`brain.fix.${f.verdict}`, {
    mod: name(f.fix.mod_id),
    action: t(`brain.fixAction.${f.fix.action}`, { defaultValue: f.fix.action }),
    when: day(f.fix.at),
    after: f.after,
    afterBad: f.after_bad,
    before: f.before,
    beforeBad: f.before_bad,
  });

/** The pre-launch warning. */
export const prelaunchText = (p: BrainPrelaunch, name: (id: string) => string, t: TFunction): string => {
  const parts = [
    t('brain.prelaunch.head', { bad: p.bad ?? p.failed_to_start + p.out_of_memory, of: p.looked_at, last: when(p.last_bad) }),
    p.failed_to_start > 0 ? t('brain.prelaunch.failed', { count: p.failed_to_start }) : '',
    p.out_of_memory > 0 ? t('brain.prelaunch.oom', { count: p.out_of_memory }) : '',
    (p.crashed ?? 0) > 0 ? t('brain.prelaunch.crashed', { count: p.crashed }) : '',
    p.last_good ? t('brain.prelaunch.diff', { good: when(p.last_good), changes: changesText(p.changes, p.more, name, t) }) : t('brain.prelaunch.noGood'),
  ];
  return parts.filter(Boolean).join(' ');
};

/** One session in a line: when, how far it got, how it ended. */
export const sessionText = (s: BrainSessionBrief, t: TFunction): string => {
  const reached = s.live ? t('brain.session.live') : t(`brain.session.reached.${s.reached || 'none'}`);
  // Java's own crash report says it fell over; else how it was closed.
  const end = s.live
    ? ''
    : s.quit_to_menu && !s.crash && !s.oom
      ? t('brain.session.toMenu')
      : s.crash === 'fatal'
      ? s.crash_cause
        ? t('brain.session.crashIn', { what: t(`brain.crashCause.${s.crash_cause}`), frame: s.crash_frame || '?' })
        : t('brain.session.crashFatal', { reason: s.crash_reason || '?', frame: s.crash_frame || '?' })
      : s.crash === 'out_of_memory'
        ? t('brain.session.crashMemory')
        : s.oom
          ? t('brain.session.oom')
          : s.clean_exit
            ? t('brain.session.closed')
            : t('brain.session.cut');
  const load = s.load_secs ? t('brain.session.load', { secs: s.load_secs }) : '';
  const save = s.save ? t('brain.session.save', { save: s.save.split('/').pop() }) : '';
  return [when(s.started), t(`brain.session.kind.${s.kind}`), save, reached, end, load, t('brain.session.counts', { mods: s.mods, errors: s.errors })]
    .filter(Boolean)
    .join(' · ');
};

/** How sure a count makes the Doctor: 5+ is sure, 3+ likely, fewer can be chance. */
export const confidenceWord = (count: number, t: TFunction, ratio = 1): string =>
  t(count >= 5 && ratio >= 0.6 ? 'brain.confidence.strong' : count >= 3 ? 'brain.confidence.likely' : 'brain.confidence.weak');

/** Whether a session went badly: it did not get to the menu (or a server to its start), or ran out of memory. */
export const sessionWentBadly = (s: BrainSessionBrief): boolean => !s.live && (s.reached === '' || s.oom || !!s.crash);

/** What the Doctor says first, for the game's main menu card (at most three short lines, the
 * worst first): the warning before playing, a last session that went badly, the errors that repeat
 * in the active mods, a fix that worked. */
export const doctorBrief = (o: BrainOverview, activeIds: string[], name: (id: string) => string, t: TFunction): [string, string][] => {
  const cut = (s: string) => (s.length > 110 ? `${s.slice(0, 108).replace(/\s+\S*$/, '')}…` : s);
  const out: [string, string][] = [];
  if (o.prelaunch) out.push(['warn', cut(t('brain.brief.prelaunch', { bad: o.prelaunch.bad ?? o.prelaunch.failed_to_start + o.prelaunch.out_of_memory, of: o.prelaunch.looked_at }))]);
  const last = o.last_report?.brief;
  if (last && sessionWentBadly(last)) out.push(['warn', cut(t('brain.brief.lastBad', { how: sessionText(last, t) }))]);
  const open = o.summary.issues.filter((i) => !i.closed && activeIds.includes(i.mod_id));
  if (open.length) out.push(['line', cut(t('brain.brief.errors', { count: open.length, mod: name(open[0].mod_id) }))]);
  const worked = o.fixes.filter((f) => f.verdict === 'worked');
  if (worked.length) out.push(['line', cut(t('brain.brief.fixWorked', { mod: name(worked[0].fix.mod_id) }))]);
  if (out.length === 0) out.push(['line', cut(t('brain.brief.ok', { count: o.summary.sessions_on_build }))]);
  return out.slice(0, 3);
};

/** Java's crash reports in a line: how many, since when, of which kind. */
export const crashesText = (crashes: BrainJvmCrash[], t: TFunction): string => {
  if (crashes.length === 0) return t('brain.crash.none');
  const memory = crashes.filter((c) => c.kind === 'out_of_memory').length;
  const fatal = crashes.filter((c) => c.kind === 'fatal');
  const frames = [...new Set(fatal.map((c) => c.frame).filter(Boolean))].join(', ');
  const first = Math.min(...crashes.map((c) => c.at));
  const last = Math.max(...crashes.map((c) => c.at));
  return t('brain.crash.summary', { count: crashes.length, first: day(first), last: when(last), memory, fatal: fatal.length, frames: frames || '?', servers: crashes.filter((c) => c.role === 'server').length });
};
