import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { Bot, ChevronDown, ChevronRight, Lightbulb, Loader2, Send } from 'lucide-react';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { noteChange } from '../../services/changes';
import { BisectSession, BrainOverview, BrainSessionReport, HealthReport, LearnedIssue, ModInfo, ModMeta, TranslatedErrorCard } from '../../types';
import { changesText, crashesText, day, fixText, onsetText, prelaunchText, sessionText, sessionWentBadly, when } from '../../utils/brain';
import { cardText } from '../../i18n/backendText';
import {
  ALL_CHIPS,
  ChipKey,
  FollowUp,
  Order,
  followUpOf,
  intentChoices,
  intentOf,
  modByName,
  modsNamedIn,
  orderOf,
  plain,
  rememberPhrase,
  sinceOf,
  suggestGroups,
  superlativeOf,
  withoutModNames,
  aliasedIn,
  asksTheGuide,
  rememberAlias,
  splitQuestions,
  TROUBLE_WORDS,
  workshopAskOf,
} from '../../utils/doctorChat';
import i18next from 'i18next';
import { crashAdvice, fixAdvice, loadAdvice, onsetAdvice } from '../../utils/doctorAdvice';

/** What the Doctor remembers between visits: the mod last talked about, and the last session it reported. */
const MEMORY_KEY = 'pz_doctor_memory';
interface Memory {
  focus?: string;
  at?: number;
  reported?: number;
  marksSeen?: number;
}
const readMemory = (): Memory => {
  try {
    return JSON.parse(localStorage.getItem(MEMORY_KEY) || '{}') as Memory;
  } catch {
    return {};
  }
};
const writeMemory = (patch: Memory) => localStorage.setItem(MEMORY_KEY, JSON.stringify({ ...readMemory(), ...patch }));

/** Markdown of the guide, as plain text for a chat bubble. */
const guideText = (md: string, max = 600): string => {
  const plainText = md.replace(/\*\*|`/g, '').replace(/^\s*-\s+/gm, '• ').replace(/\n{2,}/g, '\n').trim();
  return plainText.length > max ? `${plainText.slice(0, max).replace(/\s+\S*$/, '')}…` : plainText;
};

/**
 * The Doctor's chat: a programmatic one, with no AI. What the player types is read by its words
 * (utils/doctorChat: typos, dates, follow-ups, "which one the most"); the answer is put together
 * from what the brain learned (rules::brain) and from the app's own tools (the MCP server's, run
 * through `doctor_tool_cmd`). Each answer leads with its conclusion, then one piece of advice with
 * its evidence (utils/doctorAdvice), the details folded, buttons to go there or act, and where it
 * came from. What changes something is done only on the player's yes; the player's own notes are
 * never written or removed.
 */

/** The tools the chat uses by itself / only with a yes (the same lists as the backend's). */
const READ_TOOLS = ['brain_overview', 'brain_mod_history', 'learned_on_this_pc', 'get_crash_diagnostics', 'get_game_status', 'get_mod_health', 'mod_notes_and_groups', 'community_index_lookup', 'list_mod_profiles', 'game_resources', 'server_status'];
const CHANGE_TOOLS = ['studio_note', 'mod_group'];

interface Action {
  label: string;
  /** "nav" goes somewhere at once; "change" asks first. */
  kind: 'nav' | 'change';
  run: () => Promise<string | void>;
  state: 'ask' | 'busy' | 'done' | 'no';
  result?: string;
}
interface Msg {
  from: 'me' | 'doctor';
  /** The conclusion, in a line or two. */
  text: string;
  advice?: string;
  /** The details, folded when more than two. */
  lines?: string[];
  source?: string;
  actions?: Action[];
  chips?: ChipKey[];
  /** Buttons that ask the next question ("which of these?"); `alias` is what the player typed, remembered for the mod picked. */
  choices?: { label: string; ask: string; alias?: string; modId?: string }[];
  /** A phrase not understood: the chip the player picks next is remembered for it. */
  learnFrom?: string;
}

const toolsText = (t: TFunction) => t('doctorChat.tools', { read: READ_TOOLS.join(', '), change: CHANGE_TOOLS.join(', ') });
const titleOf = (i: { code: string; params: Record<string, string>; title: string }) =>
  cardText({ code: i.code, params: i.params, title: i.title, explanation: '', recommended_action: '' }, 'title');

export const DoctorChat: React.FC<{
  paths: StudioPathsUI;
  mods: ModInfo[];
  report: HealthReport | null;
  onShowInModList?: (modId: string) => void;
  onToggleMod?: (modId: string) => void;
  onOpenSection?: (section: 'mods' | 'game' | 'server') => void;
  /** A page of the guide's screen: profiles, saves, monitor, mod-list, health, browse, maps, merger, servers, settings. */
  onOpenPage?: (page: string) => void;
  /** The mod list changed on disk (a hunt's trial): the screens read it again. */
  onModsChanged?: () => void;
}> = ({ paths, mods, report, onShowInModList, onToggleMod, onOpenSection, onOpenPage, onModsChanged }) => {
  const { t } = useTranslation();
  const [overview, setOverview] = useState<BrainOverview | null>(null);
  const [meta, setMeta] = useState<ModMeta | null>(null);
  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);
  const [open, setOpen] = useState<Set<number>>(new Set());
  /** The mod being talked about, for "switch it off", "and since when?". */
  const focus = useRef<ModInfo | null>(null);
  /** What undoes each change the Doctor made in this conversation, newest last. */
  const undo = useRef<{ label: string; calls: { tool: string; args: Record<string, unknown> }[]; toggle?: string }[]>([]);
  const setFocus = (m: ModInfo | null) => {
    focus.current = m;
    if (m) writeMemory({ focus: m.mod_id, at: Math.floor(Date.now() / 1000) });
  };
  const bottom = useRef<HTMLDivElement>(null);
  const modsRef = useRef(mods);
  modsRef.current = mods;

  const active = useMemo(() => mods.filter((m) => m.enabled), [mods]);
  const activeIds = useMemo(() => active.map((m) => m.mod_id), [active]);
  const nameOf = useMemo(() => {
    const names = new Map(mods.map((m) => [m.mod_id, m.name]));
    return (id: string) => names.get(id) ?? overview?.names[id] ?? id;
  }, [mods, overview]);

  const say = (m: Msg) => setMessages((list) => [...list, m]);
  const learnedOpen = (o: BrainOverview | null = overview): LearnedIssue[] => (o?.summary.issues ?? []).filter((i) => !i.closed && activeIds.includes(i.mod_id));

  // ---- actions -------------------------------------------------------------------------------
  const nav = (label: string, run: () => void): Action => ({ label, kind: 'nav', state: 'ask', run: async () => run() });
  const change = (label: string, calls: { tool: string; args: Record<string, unknown> }[], inverse?: { tool: string; args: Record<string, unknown> }[]): Action => ({
    label,
    kind: 'change',
    state: 'ask',
    run: async () => {
      for (const c of calls) await TauriService.doctorTool(c.tool, { user_zomboid_dir: paths.user_zomboid_dir, ...c.args }, true);
      if (inverse) undo.current.push({ label, calls: inverse });
      noteChange('other');
      setMeta(await TauriService.modMeta(paths).catch(() => null));
      return t('doctorChat.done');
    },
  });
  const showMod = (m: ModInfo): Action | null => (onShowInModList ? nav(t('doctorChat.nav.showInList', { mod: m.name }), () => onShowInModList(m.mod_id)) : null);
  const toggle = (m: ModInfo, on: boolean): Action | null => {
    if (!onToggleMod) return null;
    return {
      label: t(on ? 'doctorChat.nav.enable' : 'doctorChat.nav.disable', { mod: m.name }),
      kind: 'change',
      state: 'ask',
      run: async () => {
        const now = modsRef.current.find((x) => x.mod_id === m.mod_id);
        if (now && now.enabled !== on) {
          onToggleMod(m.mod_id);
          undo.current.push({ label: t(on ? 'doctorChat.nav.enable' : 'doctorChat.nav.disable', { mod: m.name }), calls: [], toggle: m.mod_id });
        }
        return t('doctorChat.done');
      },
    };
  };
  const openSection = (section: 'mods' | 'game' | 'server', label: string): Action | null => (onOpenSection ? nav(label, () => onOpenSection(section)) : null);
  const acts = (...a: (Action | null)[]) => a.filter((x): x is Action => !!x);

  // ---- greeting: what matters most, said first -----------------------------------------------
  const noticed = (o: BrainOverview | null): string | null => {
    if (!o) return null;
    if (o.prelaunch) return prelaunchText(o.prelaunch, nameOf, t);
    const last = o.recent.find((s) => !s.live);
    if (last && sessionWentBadly(last)) return t('doctorChat.noticedLast', { last: sessionText(last, t) });
    const open = learnedOpen(o);
    if (open.length > 0) return t('doctorChat.noticedErrors', { count: open.length, mod: nameOf(open[0].mod_id) });
    const update = o.insights.find((i) => i.code === 'game_update');
    if (update) return t('brain.insight.game_update', { ...update.params, since: day(Number(update.params.since)), mods: update.mods.map(nameOf).join(', ') });
    return null;
  };

  useEffect(() => {
    let live = true;
    void Promise.all([TauriService.brainOverview(paths, activeIds), TauriService.modMeta(paths).catch(() => null)]).then(([o, mm]) => {
      if (!live) return;
      setOverview(o);
      setMeta(mm);
      const last = o?.recent[0];
      const note = noticed(o);
      const mem = readMemory();
      const extra: Msg[] = [];
      // the report of the last session, once
      const r = o?.last_report;
      if (r && r.brief.started !== mem.reported) {
        extra.push(reportMsg(r, t('doctorChat.reportHead', { when: when(r.brief.started) })));
        writeMemory({ reported: r.brief.started });
      }
      // moments marked in the game since the Doctor last looked
      void TauriService.brainMarks(paths).then((marks) => {
        const fresh = marks.filter((m) => m.at > (readMemory().marksSeen ?? 0));
        if (fresh.length && live) say({ from: 'doctor', text: t('doctorChat.marksNew', { count: fresh.length }), chips: ['marks'] });
      });
      // what was talked about last time, and how it is now
      const before = mem.focus ? modsRef.current.find((m) => m.mod_id === mem.focus) : undefined;
      if (before && mem.at && Date.now() / 1000 - mem.at < 30 * 86400) {
        focus.current = before;
        const open = (o?.summary.issues ?? []).filter((i) => i.mod_id === before.mod_id && !i.closed).length;
        extra.push({ from: 'doctor', text: t(open ? 'doctorChat.lastTimeOpen' : 'doctorChat.lastTimeOk', { mod: before.name, count: open, when: day(mem.at) }), choices: [{ label: before.name, ask: before.name }] });
      }
      setMessages([
        {
          from: 'doctor',
          text: t(['doctorChat.hello', 'doctorChat.hello2', 'doctorChat.hello3'][Math.floor(Math.random() * 3)], {
            sessions: o?.summary.sessions_on_build ?? 0,
            build: o?.summary.build ?? '',
            status: last ? t('doctorChat.statusOk', { last: sessionText(last, t) }) : '',
          }),
          advice: note ? t('doctorChat.noticed', { what: note }) : undefined,
          chips: ALL_CHIPS,
          source: 'brain_overview',
        },
        ...extra,
      ]);
    });
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths]);

  useEffect(() => {
    bottom.current?.scrollIntoView({ block: 'nearest' });
  }, [messages]);

  // ---- answers --------------------------------------------------------------------------------
  const answer = async (text: string, chip?: ChipKey): Promise<Msg> => {
    if (chip) return answerIntent(chip, null);
    const order = orderOf(text);
    if (order) return answerOrder(order);
    const named = modsNamedIn(text, mods).concat(aliasedIn(text, mods).filter((m) => !modsNamedIn(text, mods).includes(m)));
    // a question to the Workshop about a mod
    const ws = workshopAskOf(withoutModNames(text, named));
    if (ws && named.length === 1) return answerWorkshop(ws, named[0]);
    if (ws && named.length === 0 && focus.current && /\b(ese|este|it|that|this)\b/.test(plain(text))) return answerWorkshop(ws, focus.current);
    // a profile or a group as the subject ("how do I make a profile?" is a question for the guide)
    const prof = asksTheGuide(text) ? null : plain(text).match(/(?:perfil|profile)\s+(.+)$/);
    if (prof && named.length === 0) return answerProfile(prof[1]);
    const group = [...(meta?.groups ?? [])]
      .sort((a, b) => b.name.length - a.name.length)
      .find((g) => g.name.length >= 2 && ` ${plain(text)} `.includes(` ${plain(g.name)} `));
    if (group && named.length === 0 && !asksTheGuide(text)) return answerGroup(group.name);
    const follow = followUpOf(text);
    // "switch off Realistic Temperature", or "switch it off" about the mod being talked about
    if (follow && named.length === 1) return answerFollow(follow, named[0]);
    if (follow && named.length === 0 && focus.current) return answerFollow(follow, focus.current);
    if (follow && named.length === 0 && !focus.current && ['disable', 'enable', 'note', 'show', 'why'].includes(follow)) return { from: 'doctor', text: t('doctorChat.noFocus') };
    const most = superlativeOf(text);
    if (most) return answerMost(most);
    const since = sinceOf(text);
    const intent = intentOf(withoutModNames(text, named));
    if (named.length === 1 && (!intent || ['changed', 'errors', 'fixes', 'crash', 'summary'].includes(intent))) return answerMod(named[0]);
    if (named.length > 1 && !intent) {
      return { from: 'doctor', text: t('doctorChat.whichMod'), choices: named.slice(0, 6).map((m) => ({ label: m.name, ask: m.name })) };
    }
    if (named.length === 0 && !intent && !asksTheGuide(text)) {
      // a mod a typo or a part of its name away: "which of these?", and what was typed is learned for the one picked
      const typed = withoutModNames(text, []).replace(/\b(historial|history|de|del|el|la|the|of|que|pasa|con|sobre|about)\b/g, ' ').trim();
      const near = typed.length >= 3 ? modByName(typed, mods) : [];
      if (near.length === 1) return answerMod(near[0]);
      if (near.length > 1 && near.length <= 8) return { from: 'doctor', text: t('doctorChat.whichMod'), choices: near.map((m) => ({ label: m.name, ask: m.name, alias: typed, modId: m.mod_id })) };
    }
    if (!intent && asksTheGuide(text)) return (await answerGuide(text)) ?? { from: 'doctor', text: t('doctorChat.notUnderstood'), chips: ALL_CHIPS, learnFrom: text };
    if (!intent) {
      const two = intentChoices(text);
      if (two.length) return { from: 'doctor', text: t('doctorChat.choose'), chips: two, learnFrom: text };
      // a date alone ("since yesterday") asks what changed
      if (since) return answerIntent('changed', since);
      return { from: 'doctor', text: t('doctorChat.notUnderstood'), chips: ALL_CHIPS, learnFrom: text };
    }
    // "how do I…" with a word that also names a question ("how do I fix…") goes to the guide when it has a clear answer
    if (asksTheGuide(text) && !['crash', 'changed', 'culprit', 'week', 'undo', 'hunt'].includes(intent)) {
      const guide = await answerGuide(text, true);
      if (guide) return guide;
    }
    return answerIntent(intent, since, text);
  };

  const answerIntent = async (intent: ChipKey, since: number | null, text = ''): Promise<Msg> => {
    const o = overview;
    const inWindow = (at: number) => !since || at >= since;
    switch (intent) {
      case 'culprit':
        return answerCulprit(since);
      case 'hunt':
        return answerHunt();
      case 'week':
        return answerWeek(since, text);
      case 'undo':
        return answerUndo();
      case 'marks':
        return answerMarks();
      case 'help':
        return { from: 'doctor', text: t('doctorChat.help'), chips: ALL_CHIPS };
      case 'tools':
        return { from: 'doctor', text: toolsText(t) };
      case 'summary': {
        const issues = report?.issues.filter((i) => i.code === 'learned_issue').length ?? 0;
        const insights = (o?.insights.length ?? 0) + (o?.prelaunch ? 1 : 0);
        const fixes = o?.fixes ?? [];
        return {
          from: 'doctor',
          text: t('doctorChat.summary', {
            sessions: o?.summary.sessions_on_build ?? 0,
            build: o?.summary.build ?? '',
            all: o?.summary.sessions ?? 0,
            issues: issues > 0 ? t('doctorChat.summaryIssues', { count: issues }) : t('doctorChat.summaryNoIssues'),
            insights: insights > 0 ? t('doctorChat.summaryInsights', { count: insights }) : '',
          }),
          advice: noticed(o) ?? undefined,
          lines: [
            fixes.length > 0
              ? t('brain.insight.fixes', {
                  worked: fixes.filter((f) => f.verdict === 'worked').length,
                  notWorked: fixes.filter((f) => f.verdict === 'not_worked').length,
                  waiting: fixes.filter((f) => f.verdict === 'waiting').length,
                })
              : '',
            crashesText(o?.crashes ?? [], t),
          ].filter(Boolean),
          source: 'brain_overview',
          chips: ['crash', 'changed', 'errors'],
        };
      }
      case 'crash': {
        if (!o || o.recent.length === 0) return { from: 'doctor', text: t('doctorChat.crashNone') };
        const recent = o.recent.filter((s) => inWindow(s.started)).slice(0, 8);
        const crashes = (o.crashes ?? []).filter((c) => inWindow(c.at));
        const bad = recent.filter(sessionWentBadly);
        const lines = recent.map((s) => sessionText(s, t));
        lines.push(crashesText(crashes, t));
        try {
          const diag = await TauriService.doctorTool<{ diagnosed_error_cards?: TranslatedErrorCard[] }>('get_crash_diagnostics', { user_zomboid_dir: paths.user_zomboid_dir });
          const cards = diag?.diagnosed_error_cards ?? [];
          if (cards.length > 0) {
            lines.push(
              t('doctorChat.crashLog', {
                count: cards.length,
                list: cards
                  .slice(0, 3)
                  .map((c) => {
                    const title = cardText(c, 'title');
                    return c.mod_name && !title.includes(c.mod_name) ? `${c.mod_name}: ${title}` : title;
                  })
                  .join('; '),
              }),
            );
          }
        } catch {
          // the brain's answer stands on its own
        }
        const conclusion = o.prelaunch
          ? prelaunchText(o.prelaunch, nameOf, t)
          : bad.length === 0
            ? t('doctorChat.crashAllGood', { count: recent.length })
            : t('doctorChat.crashBad', { count: recent.length, bad: bad.length, last: sessionText(bad[0], t) });
        return {
          from: 'doctor',
          text: since ? `${t('doctorChat.sinceLabel', { since: day(since) })} ${conclusion}` : conclusion,
          advice: crashAdvice(crashes, t) ?? undefined,
          lines,
          actions: acts(openSection('game', t('doctorChat.nav.openGame'))),
          source: 'brain_overview, get_crash_diagnostics',
          chips: ['changed', 'errors'],
        };
      }
      case 'changed': {
        if (since) {
          const onset = await TauriService.brainChangesSince(paths, since);
          if (!onset || !onset.last_good) return { from: 'doctor', text: t('doctorChat.sinceNone', { since: day(since) }) };
          return {
            from: 'doctor',
            text: t('doctorChat.sinceChanges', { since: day(since), good: when(onset.last_good), bad: when(onset.first_bad), changes: changesText(onset.changes, onset.more, nameOf, t) }),
            advice: loadAdvice(onset, nameOf, t) ?? undefined,
            source: 'brain_overview (changes since)',
          };
        }
        const msgs: { line: string; advice: string | null; mod?: string }[] = [];
        for (const i of o?.insights ?? []) {
          if (i.code === 'error_onset' && i.onset) {
            let cardParams: Record<string, string> = {};
            try {
              cardParams = JSON.parse(i.params.cardParams || '{}');
            } catch {
              cardParams = {};
            }
            const title = titleOf({ code: i.params.cardCode, params: cardParams, title: i.params.title });
            msgs.push({ line: `${nameOf(i.mod_id)} — ${title} (${i.params.seen}/${i.params.on}). ${onsetText(i.onset, nameOf, t)}`, advice: onsetAdvice(i.onset, i.mod_id, nameOf, t, i.params.cardCode), mod: i.mod_id });
          } else if (i.code === 'load_slower') {
            msgs.push({ line: `${t('brain.insight.load_slower', { ...i.params, since: day(Number(i.params.since)) })} ${i.onset ? onsetText(i.onset, nameOf, t) : ''}`, advice: loadAdvice(i.onset, nameOf, t) });
          } else if (i.code === 'game_update') {
            msgs.push({ line: t('brain.insight.game_update', { ...i.params, since: day(Number(i.params.since)), mods: i.mods.map(nameOf).join(', ') }), advice: null });
          }
        }
        if (o?.prelaunch) msgs.unshift({ line: prelaunchText(o.prelaunch, nameOf, t), advice: null });
        if (msgs.length === 0) return { from: 'doctor', text: t('doctorChat.changedNone'), chips: ['errors', 'crash'] };
        const first = msgs[0];
        const m = first.mod ? mods.find((x) => x.mod_id === first.mod) : undefined;
        if (m) setFocus(m);
        return {
          from: 'doctor',
          text: first.line,
          advice: msgs.find((x) => x.advice)?.advice ?? undefined,
          lines: msgs.slice(1).map((x) => x.line),
          actions: acts(m ? showMod(m) : null),
          source: 'brain_overview',
        };
      }
      case 'errors': {
        const list = learnedOpen().filter((i) => inWindow(i.last_seen));
        if (list.length === 0) return { from: 'doctor', text: t('doctorChat.errorsNone'), source: 'learned_on_this_pc' };
        const top = mods.find((x) => x.mod_id === list[0].mod_id);
        if (top) setFocus(top);
        return {
          from: 'doctor',
          text: t('doctorChat.errorsCount', { count: list.length, mod: nameOf(list[0].mod_id), title: titleOf(list[0]), seen: list[0].seen, on: list[0].on }),
          advice: onsetAdvice(null, list[0].mod_id, nameOf, t, list[0].code) ?? undefined,
          lines: list.map((i) => `${nameOf(i.mod_id)}: ${titleOf(i)} (${i.seen}/${i.on}, ${day(i.first_seen)}–${day(i.last_seen)})`),
          actions: acts(top ? showMod(top) : null, openSection('mods', t('doctorChat.nav.openMods'))),
          source: 'learned_on_this_pc',
          chips: ['changed', 'notes'],
        };
      }
      case 'slow': {
        const ins = o?.insights.find((i) => i.code === 'load_slower');
        if (ins) {
          return {
            from: 'doctor',
            text: t('brain.insight.load_slower', { ...ins.params, since: day(Number(ins.params.since)) }),
            advice: loadAdvice(ins.onset, nameOf, t) ?? undefined,
            lines: ins.onset ? [onsetText(ins.onset, nameOf, t)] : undefined,
            source: 'brain_overview',
          };
        }
        const loads = (o?.recent ?? []).filter((s) => s.load_secs && inWindow(s.started)).slice(0, 6).map((s) => s.load_secs);
        return { from: 'doctor', text: loads.length ? t('doctorChat.slowNone', { list: loads.join(', ') }) : t('doctorChat.slowNoData'), source: 'brain_overview', chips: ['summary'] };
      }
      case 'fixes': {
        const fixes = (o?.fixes ?? []).filter((f) => inWindow(f.fix.at));
        if (fixes.length === 0) return { from: 'doctor', text: t('doctorChat.fixesNone') };
        return {
          from: 'doctor',
          text: t('brain.insight.fixes', {
            worked: fixes.filter((f) => f.verdict === 'worked').length,
            notWorked: fixes.filter((f) => f.verdict === 'not_worked').length,
            waiting: fixes.filter((f) => f.verdict === 'waiting').length,
          }),
          advice: fixAdvice(fixes, t) ?? undefined,
          lines: fixes.slice(0, 8).map((f) => fixText(f, nameOf, t)),
          source: 'brain_overview',
        };
      }
      case 'groups': {
        const grouped = new Set((meta?.groups ?? []).flatMap((g) => g.mods));
        const sugg = suggestGroups(active, grouped, mods);
        if (sugg.length === 0) return { from: 'doctor', text: t('doctorChat.suggestNone') };
        return {
          from: 'doctor',
          text: t('doctorChat.suggestHead', { count: sugg.length }),
          lines: sugg.map((s) => t('doctorChat.suggest', { count: s.mods.length, group: s.group, mods: s.mods.slice(0, 10).map((m) => m.name).join(', ') + (s.mods.length > 10 ? '…' : '') })),
          actions: sugg.map((s) => change(`${t('doctorChat.confirm')}: ${s.group}`, [{ tool: 'mod_group', args: { action: 'add', group: s.group, mod_ids: s.mods.map((m) => m.mod_id) } }])),
          source: 'mod_notes_and_groups',
        };
      }
      case 'notes': {
        const byMod = new Map<string, string[]>();
        for (const i of learnedOpen()) {
          byMod.set(i.mod_id, [...(byMod.get(i.mod_id) ?? []), t('doctorChat.learnedNote', { title: titleOf(i), seen: i.seen, on: i.on, first: day(i.first_seen) })]);
        }
        if (byMod.size === 0) return { from: 'doctor', text: t('doctorChat.noLearnedNotes') };
        return {
          from: 'doctor',
          text: t('doctorChat.askLearnedNotes', { count: byMod.size }),
          lines: [...byMod.entries()].map(([id, notes]) => `${nameOf(id)}: ${notes.join(' ')}`),
          actions: [change(t('doctorChat.confirm'), [...byMod.entries()].map(([id, notes]) => ({ tool: 'studio_note', args: { mod_id: id, text: notes.join('\n') } })))],
          source: 'learned_on_this_pc → studio_note',
        };
      }
    }
    return { from: 'doctor', text: t('doctorChat.notUnderstood'), chips: ALL_CHIPS };
  };

  const answerMod = async (m: ModInfo): Promise<Msg> => {
    setFocus(m);
    const h = await TauriService.brainModHistory(paths, m.mod_id);
    if (!h || h.sessions === 0) return { from: 'doctor', text: `${m.name}: ${t('brain.history.none')}`, source: 'brain_mod_history', actions: acts(showMod(m)) };
    const open = h.errors.filter((e) => !e.stopped);
    const lines = [`${t('brain.history.sessions', { count: h.sessions, first: day(h.first), last: day(h.last) })} ${h.place ? t('brain.history.place', { place: h.place[0], of: h.place[1] }) : ''}`];
    if (h.versions.length) lines.push(t('brain.history.versions', { list: h.versions.map((v) => t('brain.history.version', { version: v.version, sessions: v.sessions })).join(', ') }));
    if (h.failed_to_start || h.out_of_memory || h.crashed) lines.push(t('brain.history.bad', { failed: h.failed_to_start, oom: h.out_of_memory, crashed: h.crashed ?? 0 }));
    for (const e of h.errors.slice(0, 4)) {
      lines.push(`${titleOf(e)}: ${t('brain.history.error', { seen: e.seen, on: e.on, first: day(e.first), last: day(e.last) })}${e.stopped ? ` (${t('brain.history.stopped')})` : ''}. ${e.onset ? onsetText(e.onset, nameOf, t) : ''}`);
    }
    h.fixes.forEach((f) => lines.push(fixText(f, nameOf, t)));
    const own = meta?.notes[m.mod_id];
    const studio = meta?.studio_notes?.[m.mod_id];
    if (own) lines.push(`${t('modList.row.noteHead')}: ${own}`);
    if (studio) lines.push(`${t('brain.studioNote.label')}: ${studio}`);
    const now = modsRef.current.find((x) => x.mod_id === m.mod_id);
    return {
      from: 'doctor',
      text: open.length
        ? t('doctorChat.modSummary', { mod: m.name, sessions: h.sessions, count: open.length, title: titleOf(open[0]) })
        : t('doctorChat.modSummaryOk', { mod: m.name, sessions: h.sessions }),
      advice: open.length ? (onsetAdvice(open[0].onset, m.mod_id, nameOf, t, open[0].code) ?? fixAdvice(h.fixes, t) ?? undefined) : undefined,
      lines,
      actions: acts(
        showMod(m),
        open.length && now?.enabled ? toggle(m, false) : null,
        open.length ? noteAction(m, open.map((e) => t('doctorChat.learnedNote', { title: titleOf(e), seen: e.seen, on: e.on, first: day(e.first) })).join('\n')) : null,
      ),
      source: 'brain_mod_history, mod_notes_and_groups',
    };
  };

  const noteAction = (m: ModInfo, text: string): Action =>
    change(t('doctorChat.nav.noteIt', { mod: m.name }), [{ tool: 'studio_note', args: { mod_id: m.mod_id, text } }], [{ tool: 'studio_note', args: { mod_id: m.mod_id, text: meta?.studio_notes?.[m.mod_id] ?? '' } }]);

  const answerFollow = async (f: FollowUp, m: ModInfo): Promise<Msg> => {
    setFocus(m);
    const now = modsRef.current.find((x) => x.mod_id === m.mod_id);
    switch (f) {
      case 'disable':
      case 'enable': {
        const on = f === 'enable';
        if (now && now.enabled === on) return { from: 'doctor', text: t(on ? 'doctorChat.alreadyOn' : 'doctorChat.alreadyOff', { mod: m.name }) };
        return { from: 'doctor', text: t(on ? 'doctorChat.askEnable' : 'doctorChat.askDisable', { mod: m.name }), actions: acts(toggle(m, on)) };
      }
      case 'show':
        onShowInModList?.(m.mod_id);
        return { from: 'doctor', text: t('doctorChat.shown', { mod: m.name }) };
      case 'note': {
        const h = await TauriService.brainModHistory(paths, m.mod_id);
        const open = (h?.errors ?? []).filter((e) => !e.stopped);
        const text = open.length
          ? open.map((e) => t('doctorChat.learnedNote', { title: titleOf(e), seen: e.seen, on: e.on, first: day(e.first) })).join('\n')
          : t('doctorChat.noteNothing', { sessions: h?.sessions ?? 0 });
        return { from: 'doctor', text: t('doctorChat.askNote', { mod: m.name, text }), actions: [noteAction(m, text)], source: 'brain_mod_history → studio_note' };
      }
      case 'why': {
        const h = await TauriService.brainModHistory(paths, m.mod_id);
        const e = (h?.errors ?? []).find((x) => !x.stopped) ?? h?.errors[0];
        if (!e || !e.onset) return { from: 'doctor', text: t('doctorChat.whyNone', { mod: m.name }) };
        return {
          from: 'doctor',
          text: `${m.name} — ${titleOf(e)}. ${onsetText(e.onset, nameOf, t)}`,
          advice: onsetAdvice(e.onset, m.mod_id, nameOf, t, e.code) ?? undefined,
          source: 'brain_mod_history',
        };
      }
      case 'errors':
      case 'history':
        return answerMod(m);
    }
  };

  const answerMost = async (most: 'slowest' | 'most_errors'): Promise<Msg> => {
    if (most === 'slowest') {
      const lt = await TauriService.modLoadTimes(paths).catch(() => null);
      const top = (lt?.mods ?? []).filter((x) => x.millis > 0).sort((a, b) => b.millis - a.millis).slice(0, 6);
      if (top.length === 0) return { from: 'doctor', text: t('doctorChat.slowestNone') };
      const m = mods.find((x) => x.mod_id === top[0].mod_id);
      if (m) setFocus(m);
      return {
        from: 'doctor',
        text: t('doctorChat.slowest', { mod: nameOf(top[0].mod_id), secs: (top[0].millis / 1000).toFixed(1) }),
        lines: top.map((x) => `${nameOf(x.mod_id)}: ${(x.millis / 1000).toFixed(1)} s`),
        actions: acts(m ? showMod(m) : null),
        source: 'mod_load_times',
      };
    }
    const byMod = new Map<string, number>();
    for (const i of learnedOpen()) byMod.set(i.mod_id, (byMod.get(i.mod_id) ?? 0) + i.seen);
    const top = [...byMod.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6);
    if (top.length === 0) return { from: 'doctor', text: t('doctorChat.errorsNone') };
    const m = mods.find((x) => x.mod_id === top[0][0]);
    if (m) setFocus(m);
    return {
      from: 'doctor',
      text: t('doctorChat.mostErrors', { mod: nameOf(top[0][0]), count: top[0][1] }),
      lines: top.map(([id, n]) => `${nameOf(id)}: ${n}`),
      actions: acts(m ? showMod(m) : null),
      source: 'learned_on_this_pc',
    };
  };

  const answerOrder = async (order: Order): Promise<Msg> => {
    const groups = meta?.groups ?? [];
    const findGroup = (name: string) => groups.find((g) => plain(g.name) === plain(name));
    const resolve = (names: string[]) => {
      const out = { ok: [] as ModInfo[], missing: [] as string[], many: [] as ModInfo[][] };
      for (const n of names) {
        const found = modByName(n, mods);
        if (found.length === 1) out.ok.push(found[0]);
        else if (found.length === 0) out.missing.push(n);
        else out.many.push(found);
      }
      return out;
    };
    switch (order.kind) {
      case 'listGroups':
        return groups.length
          ? { from: 'doctor', text: t('doctorChat.groupsList', { list: groups.map((g) => `${g.name} (${g.mods.length})`).join(', ') }), source: 'mod_notes_and_groups' }
          : { from: 'doctor', text: t('doctorChat.groupsNone'), chips: ['groups'] };
      case 'add':
      case 'remove': {
        if (order.mods.length === 0) return { from: 'doctor', text: t('doctorChat.needMods') };
        const r = resolve(order.mods);
        if (r.missing.length) return { from: 'doctor', text: `${t('doctorChat.modNotFound')} (${r.missing.join(', ')})` };
        if (r.many.length) {
          return { from: 'doctor', text: t('doctorChat.whichMod'), choices: r.many[0].slice(0, 6).map((m) => ({ label: m.name, ask: m.name })) };
        }
        const g = findGroup(order.group);
        if (order.kind === 'remove' && !g) return { from: 'doctor', text: t('doctorChat.groupNotFound', { group: order.group }) };
        const group = g?.name ?? order.group.trim();
        const names = r.ok.map((m) => m.name).join(', ');
        return {
          from: 'doctor',
          text: t(order.kind === 'add' ? 'doctorChat.askAdd' : 'doctorChat.askRemove', { mods: names, group }),
          actions: [
            change(t('doctorChat.confirm'), [{ tool: 'mod_group', args: { action: order.kind, group, mod_ids: r.ok.map((m) => m.mod_id) } }], [
              { tool: 'mod_group', args: { action: order.kind === 'add' ? 'remove' : 'add', group, mod_ids: r.ok.map((m) => m.mod_id).filter((id) => (order.kind === 'add' ? !g?.mods.includes(id) : g?.mods.includes(id))) } },
            ]),
          ],
          source: 'mod_group',
        };
      }
      case 'rename':
      case 'delete': {
        const g = findGroup(order.group);
        if (!g) return { from: 'doctor', text: t('doctorChat.groupNotFound', { group: order.group }) };
        return order.kind === 'rename'
          ? { from: 'doctor', text: t('doctorChat.askRename', { group: g.name, to: order.to }), actions: [change(t('doctorChat.confirm'), [{ tool: 'mod_group', args: { action: 'rename', group: g.name, to: order.to } }], [{ tool: 'mod_group', args: { action: 'rename', group: order.to, to: g.name } }])], source: 'mod_group' }
          : { from: 'doctor', text: t('doctorChat.askDelete', { group: g.name }), actions: [change(t('doctorChat.confirm'), [{ tool: 'mod_group', args: { action: 'delete', group: g.name } }], [{ tool: 'mod_group', args: { action: 'add', group: g.name, mod_ids: g.mods } }])], source: 'mod_group' };
      }
      case 'note':
      case 'noteRemove': {
        const found = modByName(order.mod, mods);
        if (found.length === 0) return { from: 'doctor', text: t('doctorChat.modNotFound') };
        if (found.length > 1) return { from: 'doctor', text: t('doctorChat.whichMod'), choices: found.slice(0, 6).map((m) => ({ label: m.name, ask: m.name })) };
        const m = found[0];
        setFocus(m);
        if (order.kind === 'noteRemove') {
          if (!meta?.studio_notes?.[m.mod_id]) return { from: 'doctor', text: t('doctorChat.noStudioNote', { mod: m.name }) };
          return { from: 'doctor', text: t('doctorChat.askNoteRemove', { mod: m.name }), actions: [change(t('doctorChat.confirm'), [{ tool: 'studio_note', args: { mod_id: m.mod_id, text: '' } }])], source: 'studio_note' };
        }
        return { from: 'doctor', text: t('doctorChat.askNote', { mod: m.name, text: order.text }), actions: [noteAction(m, order.text)], source: 'studio_note' };
      }
    }
  };

  // ---- reports, the culprit, the hunt, the guide, the Workshop, groups, profiles, undo -------
  const errorLine = (e: { mod_id: string; code: string; params: Record<string, string>; title: string }) => `${nameOf(e.mod_id)}: ${titleOf(e)}`;

  const reportMsg = (r: BrainSessionReport, head: string): Msg => {
    const b = r.brief;
    const lines = [
      t('doctorChat.reportLine', { minutes: r.minutes, how: sessionText(b, t) }),
      r.new_errors.length ? t('doctorChat.reportNew', { count: r.new_errors.length, list: r.new_errors.slice(0, 4).map(errorLine).join('; ') }) : t('doctorChat.reportNoNew', { repeated: r.repeated }),
      ...r.fixes.filter((f) => f.verdict === 'worked' || f.verdict === 'not_worked').map((f) => fixText(f, nameOf, t)),
    ];
    const m = r.new_errors[0] ? mods.find((x) => x.mod_id === r.new_errors[0].mod_id) : undefined;
    return { from: 'doctor', text: head, lines, actions: acts(m ? showMod(m) : null), source: 'brain_overview (last session)' };
  };

  const answerWeek = async (since: number | null, text: string): Promise<Msg> => {
    // "how did my last session go?": the last session; else the period (a week by default)
    if (!since && /(ultima partida|last session|ultima vez|last time|ultima sesion)/.test(plain(text)) && overview?.last_report) {
      return reportMsg(overview.last_report, t('doctorChat.reportHead', { when: when(overview.last_report.brief.started) }));
    }
    const from = since ?? Math.floor(Date.now() / 1000) - 7 * 86400;
    const p = await TauriService.brainPeriod(paths, from);
    if (!p || p.sessions + p.servers === 0) return { from: 'doctor', text: t('doctorChat.weekNone', { since: day(from) }) };
    const lines = [
      ...p.new_errors.slice(0, 8).map(errorLine),
      ...p.fixes.map((f) => fixText(f, nameOf, t)),
      p.changes && p.changes.last_good ? t('doctorChat.weekChanges', { changes: changesText(p.changes.changes, p.changes.more, nameOf, t) }) : '',
    ].filter(Boolean);
    const crashes = (overview?.crashes ?? []).filter((c) => c.at >= from);
    return {
      from: 'doctor',
      text: t('doctorChat.week', { since: day(from), sessions: p.sessions, hours: (p.minutes / 60).toFixed(1), servers: p.servers, bad: p.went_badly, crashes: p.crashes, errors: p.new_errors.length }),
      advice: crashAdvice(crashes, t) ?? fixAdvice(p.fixes, t) ?? undefined,
      lines,
      source: 'brain_period',
      chips: ['culprit', 'changed'],
    };
  };

  const answerCulprit = async (since: number | null): Promise<Msg> => {
    const list = await TauriService.brainSuspects(paths, 'any', since);
    if (list.length === 0) return { from: 'doctor', text: t('doctorChat.culpritNone'), chips: ['hunt', 'crash'], source: 'brain_suspects' };
    const top = list[0];
    const m = mods.find((x) => x.mod_id === top.mod_id);
    if (m) setFocus(m);
    const line = (x: typeof top) => t('doctorChat.culpritLine', { mod: nameOf(x.mod_id), withBad: x.with_bad, with: x.with, withoutBad: x.without_bad, without: x.without, sure: t(`brain.confidence.${x.confidence}`) });
    const now = m ? modsRef.current.find((x) => x.mod_id === m.mod_id) : undefined;
    return {
      from: 'doctor',
      text: t('doctorChat.culprit', { line: line(top) }),
      advice: t(top.confidence === 'strong' ? 'doctorChat.advice.culpritStrong' : 'doctorChat.advice.culpritCheck', { mod: nameOf(top.mod_id) }),
      lines: list.slice(1).map(line),
      actions: acts(m ? showMod(m) : null, m && now?.enabled ? toggle(m, false) : null),
      chips: ['hunt'],
      source: 'brain_suspects',
    };
  };

  /** The hunt (halving the list, the Monitor's bisect), driven from the chat. */
  const huntMsg = (sess: BisectSession | null, head?: string): Msg => {
    if (!sess) return { from: 'doctor', text: head ?? t('doctorChat.huntNone') };
    if (sess.phase === 'Done') {
      const c = sess.culprit ? mods.find((x) => x.mod_id === sess.culprit) : undefined;
      if (c) setFocus(c);
      return {
        from: 'doctor',
        text: c
          ? sess.pair_with
            ? t('doctorChat.huntDonePair', { mod: c.name, other: mods.find((x) => x.mod_id === sess.pair_with)?.name ?? sess.pair_with })
            : t('doctorChat.huntDone', { mod: c.name })
          : t('doctorChat.huntDoneNone'),
        advice: sess.note ?? undefined,
        actions: acts(c ? showMod(c) : null, c ? toggle(c, false) : null),
        source: 'bisect',
      };
    }
    const answerBtn = (label: string, still: boolean): Action =>
      nav(label, () => {
        void TauriService.bisectAnswer(paths, still)
          .then((next) => {
            onModsChanged?.();
            say(huntMsg(next));
          })
          .catch((err) => say({ from: 'doctor', text: t('doctorChat.failed', { reason: String(err) }) }));
      });
    return {
      from: 'doctor',
      text: head ?? t('doctorChat.huntStep', { round: sess.round, trial: sess.trial.length, candidates: sess.candidates.length, left: sess.launches_left }),
      advice: t('doctorChat.huntHow'),
      actions: [
        nav(t('doctorChat.huntPlay'), () => void TauriService.launchSandbox({ pz_install_dir: paths.pz_install_dir, user_zomboid_dir: paths.user_zomboid_dir, test_mode: 'NORMAL' })),
        answerBtn(t('doctorChat.huntStill'), true),
        answerBtn(t('doctorChat.huntGone'), false),
        nav(t('doctorChat.huntStop'), () => {
          void TauriService.bisectStop(paths, null).then(() => {
            onModsChanged?.();
            say({ from: 'doctor', text: t('doctorChat.huntStopped') });
          });
        }),
      ],
      source: 'bisect',
    };
  };

  const answerHunt = async (): Promise<Msg> => {
    const sess = await TauriService.bisectState(paths).catch(() => null);
    if (sess && sess.phase !== 'Done') return huntMsg(sess);
    return {
      from: 'doctor',
      text: t('doctorChat.huntOffer', { count: active.length }),
      actions: [
        {
          label: t('doctorChat.huntStart'),
          kind: 'change',
          state: 'ask',
          run: async () => {
            const started = await TauriService.bisectStart(paths, modsRef.current);
            onModsChanged?.();
            say(huntMsg(started));
            return t('doctorChat.done');
          },
        },
      ],
      source: 'bisect',
    };
  };

  const PAGE_LABEL: Record<string, string> = { profiles: 'nav.profiles', saves: 'nav.saves', monitor: 'nav.monitor', 'mod-list': 'nav.modList', health: 'nav.health', browse: 'nav.browse', maps: 'nav.maps', merger: 'nav.merger', servers: 'nav.servers', settings: 'nav.settings', 'whats-new': 'nav.whatsNew' };
  const answerGuide = async (text: string, onlyIfClear = false): Promise<Msg | null> => {
    // the guide in the app's language, else in the other one (a question typed in Spanish with the app in English)
    const lang = i18next.resolvedLanguage ?? 'es';
    let hits = await TauriService.appDocsSearch(text, lang);
    if (hits.length === 0 || hits[0].score < 1.5) {
      const other = await TauriService.appDocsSearch(text, lang.startsWith('es') ? 'en' : 'es');
      if (other.length > 0 && (hits.length === 0 || other[0].score > hits[0].score)) hits = other;
    }
    if (hits.length === 0 || (onlyIfClear && hits[0].score < 1.5)) {
      return onlyIfClear ? null : { from: 'doctor', text: t('doctorChat.notUnderstood'), chips: ALL_CHIPS, learnFrom: text };
    }
    const h = hits[0];
    const label = PAGE_LABEL[h.page];
    return {
      from: 'doctor',
      text: t('doctorChat.guideHead', { heading: h.heading }),
      advice: guideText(h.text),
      lines: hits.slice(1).map((x) => `${x.heading}: ${guideText(x.text, 160)}`),
      actions: acts(onOpenPage && label ? nav(t('doctorChat.nav.openPage', { page: t(label, { defaultValue: h.page }) }), () => onOpenPage(h.page)) : null),
      source: `docs/app/${h.page}.md`,
    } as Msg;
  };

  const answerWorkshop = async (kind: 'update' | 'comments', m: ModInfo): Promise<Msg> => {
    setFocus(m);
    if (!m.workshop_id) return { from: 'doctor', text: t('doctorChat.notWorkshop', { mod: m.name }) };
    if (kind === 'update') {
      const w = await TauriService.doctorTool<{ details?: { item?: { time_updated?: number } }; installed?: { installed_time?: number; update_pending?: boolean; updated_since_last_play?: boolean } }>('workshop_item', { id: m.workshop_id }).catch(() => null);
      if (!w) return { from: 'doctor', text: t('doctorChat.workshopFailed') };
      const latest = w.details?.item?.time_updated ?? 0;
      const installed = w.installed?.installed_time ?? 0;
      const text = w.installed?.update_pending
        ? t('doctorChat.updatePending', { mod: m.name, when: day(latest) })
        : latest > installed + 60 && installed > 0
          ? t('doctorChat.updateNewer', { mod: m.name, when: day(latest), have: day(installed) })
          : t('doctorChat.updateNone', { mod: m.name, when: day(latest || installed) });
      return { from: 'doctor', text, advice: w.installed?.updated_since_last_play ? t('doctorChat.updatedSincePlay', { mod: m.name }) : undefined, source: 'workshop_item' };
    }
    const c = await TauriService.doctorTool<{ total?: number; comments?: { text: string; when: number; by_author: boolean }[]; unavailable?: string }>('workshop_comments', { id: m.workshop_id, count: 40 }).catch(() => null);
    if (!c || c.unavailable) return { from: 'doctor', text: t('doctorChat.commentsNone', { mod: m.name }) };
    const list = c.comments ?? [];
    const trouble = list.filter((x) => TROUBLE_WORDS.test(x.text));
    const author = list.filter((x) => x.by_author).length;
    return {
      from: 'doctor',
      text: t('doctorChat.comments', { mod: m.name, read: list.length, total: c.total ?? list.length, trouble: trouble.length, author }),
      lines: trouble.slice(0, 4).map((x) => `${day(x.when)}: ${x.text.replace(/\s+/g, ' ').slice(0, 140)}${x.text.length > 140 ? '…' : ''}`),
      source: 'workshop_comments',
    };
  };

  const answerGroup = async (name: string): Promise<Msg> => {
    const g = (meta?.groups ?? []).find((x) => plain(x.name) === plain(name));
    if (!g) return { from: 'doctor', text: t('doctorChat.groupNotFound', { group: name }) };
    const inGroup = mods.filter((m) => g.mods.includes(m.mod_id));
    const on = inGroup.filter((m) => m.enabled);
    const issues = (overview?.summary.issues ?? []).filter((i) => !i.closed && g.mods.includes(i.mod_id));
    const suspects = (await TauriService.brainSuspects(paths, 'any')).filter((x) => g.mods.includes(x.mod_id));
    return {
      from: 'doctor',
      text: t('doctorChat.group', { group: g.name, count: inGroup.length, on: on.length, errors: issues.length, suspects: suspects.length }),
      advice: g.note || g.studio_note || undefined,
      lines: [...issues.map((i) => errorLine(i)), ...suspects.map((x) => t('doctorChat.culpritLine', { mod: nameOf(x.mod_id), withBad: x.with_bad, with: x.with, withoutBad: x.without_bad, without: x.without, sure: t(`brain.confidence.${x.confidence}`) }))],
      choices: inGroup.slice(0, 8).map((m) => ({ label: m.name, ask: m.name })),
      source: 'mod_notes_and_groups, learned_on_this_pc, brain_suspects',
    };
  };

  const answerProfile = async (typed: string): Promise<Msg> => {
    type Profile = { name: string; active_mod_ids: string[] };
    const got = await TauriService.doctorTool<{ profiles?: Profile[] } | Profile[]>('list_mod_profiles', { user_zomboid_dir: paths.user_zomboid_dir }).catch(() => null);
    const all: Profile[] = Array.isArray(got) ? got : (got?.profiles ?? []);
    const q = plain(typed).replace(/[?.!]+$/, '').trim();
    const exact = all.find((x) => plain(x.name) === q);
    const near = exact ? [exact] : all.filter((x) => plain(x.name).includes(q) || q.includes(plain(x.name)));
    if (near.length === 0) return { from: 'doctor', text: t('doctorChat.profileNotFound', { profile: typed }) };
    if (near.length > 1) return { from: 'doctor', text: t('doctorChat.whichProfile'), choices: near.slice(0, 6).map((x) => ({ label: x.name, ask: `${t('doctorChat.profileWord')} ${x.name}` })) };
    const p = near[0];
    const ids = p.active_mod_ids ?? [];
    const warn = await TauriService.brainPrelaunch(paths, ids);
    const issues = (overview?.summary.issues ?? []).filter((i) => !i.closed && ids.includes(i.mod_id));
    return {
      from: 'doctor',
      text: warn ? prelaunchText(warn, nameOf, t) : t('doctorChat.profileOk', { profile: p.name, count: ids.length, errors: issues.length }),
      lines: issues.map((i) => errorLine(i)),
      source: 'list_mod_profiles, brain_prelaunch',
    };
  };

  /** The moments marked in the game (the Link's pause menu button), with what the log says then. */
  const answerMarks = async (): Promise<Msg> => {
    const list = await TauriService.brainMarks(paths);
    writeMemory({ marksSeen: Math.floor(Date.now() / 1000) });
    if (list.length === 0) return { from: 'doctor', text: t('doctorChat.marksNone') };
    const lines: string[] = [];
    for (const m of list.slice(0, 5)) {
      const where = [m.save ? m.save.split('/').pop() : '', m.place].filter(Boolean).join(' · ');
      const said = m.cards.length
        ? m.cards.slice(0, 3).map((c) => `${c.mod_id ? `${c.mod_id}: ` : ''}${titleOf(c)}`).join('; ')
        : m.lines.length
          ? m.lines.slice(0, 2).join(' | ')
          : m.session
            ? t('doctorChat.markQuiet')
            : t('doctorChat.markNoLog');
      lines.push(`${when(m.at)}${where ? ` (${where})` : ''}: ${said}`);
    }
    const first = list[0];
    return {
      from: 'doctor',
      text: t('doctorChat.marks', { count: list.length, last: when(first.at) }),
      advice: first.cards.length ? t('doctorChat.markAdvice', { what: `${first.cards[0].mod_id ? `${first.cards[0].mod_id}: ` : ''}${titleOf(first.cards[0])}` }) : undefined,
      lines,
      source: 'brain_marks (Studio Link diary)',
    };
  };

  const answerUndo = async (): Promise<Msg> => {
    const last = undo.current[undo.current.length - 1];
    if (!last) return { from: 'doctor', text: t('doctorChat.undoNone') };
    return {
      from: 'doctor',
      text: t('doctorChat.undoAsk', { what: last.label }),
      actions: [
        {
          label: t('doctorChat.undoDo'),
          kind: 'change',
          state: 'ask',
          run: async () => {
            undo.current.pop();
            if (last.toggle) onToggleMod?.(last.toggle);
            for (const c of last.calls) await TauriService.doctorTool(c.tool, { user_zomboid_dir: paths.user_zomboid_dir, ...c.args }, true);
            noteChange('other');
            setMeta(await TauriService.modMeta(paths).catch(() => null));
            return t('doctorChat.done');
          },
        },
      ],
    };
  };

  // ---- the conversation ----------------------------------------------------------------------
  const ask = async (text: string, chip?: ChipKey, learnFrom?: string) => {
    if (busy || (!text.trim() && !chip)) return;
    if (chip && learnFrom) {
      // it did not understand that phrase and the player showed what it meant: remembered
      rememberPhrase(learnFrom, chip);
    }
    say({ from: 'me', text: chip ? t(`doctorChat.chips.${chip}`) : text });
    setInput('');
    setBusy(true);
    try {
      const parts = chip ? [text] : splitQuestions(text);
      if (parts.length > 1) {
        for (const p of parts) say(await answer(p));
        return;
      }
      const reply = await answer(text, chip);
      say(chip && learnFrom ? { ...reply, advice: [t('doctorChat.learned', { phrase: learnFrom }), reply.advice].filter(Boolean).join(' ') } : reply);
    } catch (err) {
      say({ from: 'doctor', text: t('doctorChat.failed', { reason: String(err) }) });
    } finally {
      setBusy(false);
    }
  };

  const runAction = async (mi: number, ai: number, yes: boolean) => {
    const a = messages[mi]?.actions?.[ai];
    if (!a) return;
    const set = (patch: Partial<Action>) =>
      setMessages((list) => list.map((m, i) => (i === mi ? { ...m, actions: m.actions?.map((x, j) => (j === ai ? { ...x, ...patch } : x)) } : m)));
    if (a.kind === 'nav') {
      await a.run();
      return;
    }
    if (a.state !== 'ask') return;
    if (!yes) return set({ state: 'no', result: t('doctorChat.cancelled') });
    set({ state: 'busy' });
    try {
      const r = await a.run();
      set({ state: 'done', result: r || t('doctorChat.done') });
    } catch (err) {
      set({ state: 'done', result: t('doctorChat.failed', { reason: String(err) }) });
    }
  };

  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-4 flex flex-col gap-3">
      <div className="flex items-start gap-2">
        <Bot className="w-5 h-5 text-violet-300 shrink-0" aria-hidden="true" />
        <div>
          <div className="text-sm font-bold text-slate-100">{t('doctorChat.title')}</div>
          <div className="text-[11px] text-slate-400 leading-relaxed">{t('doctorChat.blurb')}</div>
        </div>
      </div>
      <div className="max-h-[460px] overflow-y-auto flex flex-col gap-2 pr-1" aria-live="polite">
        {messages.length === 0 && (
          <div className="flex items-center gap-2 text-[11.5px] text-slate-400">
            <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />
            {t('brain.history.loading')}
          </div>
        )}
        {messages.map((m, mi) => {
          const many = (m.lines?.length ?? 0) > 2;
          const shown = !many || open.has(mi);
          return (
            <div key={mi} className={`flex ${m.from === 'me' ? 'justify-end' : 'justify-start'}`}>
              <div
                data-chat-msg={m.from}
                className={`max-w-[92%] rounded-lg px-3 py-2 text-[12px] leading-relaxed ${
                  m.from === 'me' ? 'bg-sky-900/40 border border-sky-800/60 text-sky-100' : 'bg-slate-950/60 border border-slate-800 text-slate-200'
                }`}
              >
                {m.text && <div className="whitespace-pre-wrap">{m.text}</div>}
                {m.advice && (
                  <div className="mt-1.5 flex items-start gap-1.5 rounded border border-amber-800/50 bg-amber-950/20 px-2 py-1 text-amber-100">
                    <Lightbulb className="w-3.5 h-3.5 mt-0.5 shrink-0 text-amber-300" aria-hidden="true" />
                    <span>{m.advice}</span>
                  </div>
                )}
                {m.lines && m.lines.length > 0 && (
                  <>
                    {many && (
                      <button
                        type="button"
                        onClick={() => setOpen((s) => new Set(s.has(mi) ? [...s].filter((x) => x !== mi) : [...s, mi]))}
                        className="mt-1 flex items-center gap-1 text-[11px] text-sky-300 hover:text-sky-200 cursor-pointer"
                      >
                        {shown ? <ChevronDown className="w-3 h-3" aria-hidden="true" /> : <ChevronRight className="w-3 h-3" aria-hidden="true" />}
                        {shown ? t('doctorChat.hide') : t('doctorChat.details', { count: m.lines.length })}
                      </button>
                    )}
                    {shown && (
                      <ul className="mt-1 space-y-1 list-disc pl-4 text-slate-300">
                        {m.lines.map((l, i) => (
                          <li key={i}>{l}</li>
                        ))}
                      </ul>
                    )}
                  </>
                )}
                {m.actions && m.actions.length > 0 && (
                  <div className="mt-2 flex flex-wrap items-center gap-1.5">
                    {m.actions.map((a, ai) =>
                      a.kind === 'nav' ? (
                        <button key={ai} type="button" onClick={() => void runAction(mi, ai, true)} className="px-2.5 py-1 rounded-md border border-sky-700/70 text-sky-200 hover:bg-sky-900/40 text-[11px] cursor-pointer">
                          {a.label}
                        </button>
                      ) : a.state === 'ask' ? (
                        <span key={ai} className="flex items-center gap-1">
                          <button type="button" onClick={() => void runAction(mi, ai, true)} className="px-2.5 py-1 rounded-md bg-violet-700 hover:bg-violet-600 text-white text-[11px] font-bold cursor-pointer">
                            {a.label}
                          </button>
                          <button type="button" onClick={() => void runAction(mi, ai, false)} className="px-2 py-1 rounded-md border border-slate-700 text-slate-300 hover:bg-slate-800 text-[11px] cursor-pointer">
                            {t('doctorChat.cancel')}
                          </button>
                        </span>
                      ) : a.state === 'busy' ? (
                        <Loader2 key={ai} className="w-3.5 h-3.5 animate-spin text-violet-300" aria-hidden="true" />
                      ) : (
                        <span key={ai} className={`text-[11px] ${a.state === 'done' ? 'text-emerald-300' : 'text-slate-400'}`}>
                          {a.label}: {a.result}
                        </span>
                      ),
                    )}
                  </div>
                )}
                {m.choices && (
                  <div className="mt-2 flex flex-wrap gap-1">
                    {m.choices.map((c) => (
                      <button
                        key={c.label}
                        type="button"
                        onClick={() => {
                          if (c.alias && c.modId) rememberAlias(c.alias, c.modId);
                          void ask(c.ask);
                        }}
                        disabled={busy} className="px-2 py-0.5 rounded-full border border-violet-700/70 text-[10.5px] text-violet-200 hover:bg-violet-900/40 cursor-pointer disabled:opacity-40">
                        {c.label}
                      </button>
                    ))}
                  </div>
                )}
                {m.source && <div className="mt-1 text-[10px] text-slate-500">{t('doctorChat.source', { source: m.source })}</div>}
                {m.chips && (
                  <div className="mt-2 flex flex-wrap gap-1">
                    {m.chips.map((c) => (
                      <button key={c} type="button" onClick={() => void ask('', c, m.learnFrom)} disabled={busy} className="px-2 py-0.5 rounded-full border border-slate-700 text-[10.5px] text-slate-300 hover:border-violet-600 hover:text-violet-200 cursor-pointer disabled:opacity-40">
                        {t(`doctorChat.chips.${c}`)}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>
          );
        })}
        {busy && (
          <div className="flex items-center gap-2 text-[11px] text-slate-400">
            <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />
            {t('doctorChat.thinking')}
          </div>
        )}
        <div ref={bottom} />
      </div>
      <form
        className="flex items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void ask(input);
        }}
      >
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={t('doctorChat.placeholder')}
          aria-label={t('doctorChat.title')}
          className="flex-1 min-w-0 rounded-md border border-slate-700 bg-slate-950 px-3 py-1.5 text-[12px] text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-violet-600"
        />
        <button type="submit" disabled={busy || !input.trim()} className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-violet-700 hover:bg-violet-600 text-white text-[12px] font-bold cursor-pointer disabled:opacity-40">
          <Send className="w-3.5 h-3.5" aria-hidden="true" />
          {t('doctorChat.send')}
        </button>
      </form>
    </div>
  );
};
