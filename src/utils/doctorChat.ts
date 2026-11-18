import { ModInfo } from '../types';
import {
  FOLLOW, FOLLOW_ALONE, GENERAL, GUIDE_ANYWHERE, GUIDE_STARTS, INTENT_WORDS, LAST_SESSION, LEANS, LEAN_STARTS, LIST_AND, LIST_GROUPS, ORDER_WORDS, PROFILE_WORDS,
  SPLIT_AND, SPLIT_AND_ZH, SUPERLATIVE, TIME, TROUBLE, WORKSHOP_COMMENTS, WORKSHOP_UPDATE,
} from './doctorWords';

/**
 * How the Doctor's chat reads what the player types (components/health/DoctorChat): which
 * question it asks, which mods it names, when ("since yesterday"), whether it follows the last
 * answer ("and since when?", "switch it off"), and the orders that change groups or the Studio's
 * notes. By words, in every language the app has (doctorWords.ts), forgiving typos; no AI. What
 * it did not understand and the player then showed with a button is remembered on this PC.
 */

export type ChipKey = 'summary' | 'crash' | 'changed' | 'errors' | 'slow' | 'fixes' | 'groups' | 'notes' | 'tools' | 'help' | 'culprit' | 'hunt' | 'week' | 'undo' | 'marks';
export const ALL_CHIPS: ChipKey[] = ['summary', 'week', 'crash', 'culprit', 'changed', 'errors', 'slow', 'fixes', 'groups', 'notes', 'help'];

/** Lower case, without accents, so "qué cambió" and "que cambio" are one question. */
export const plain = (s: string): string =>
  s
    .toLowerCase()
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[¿¡]/g, '')
    .trim();

/** Plain, with apostrophes and hyphens as spaces ("qu'est-ce" -> "qu est ce"): how words are compared. */
const flat = (s: string): string => plain(s).replace(/['’-]/g, ' ').replace(/\s+/g, ' ').trim();

/** Chinese (and Japanese) characters: words with no spaces between them. */
const CJK = /[぀-ヿ㐀-鿿]/;

const words = (s: string): string[] => plain(s).split(/[^\p{L}\p{N}]+/u).filter(Boolean);

const LETTER = /[\p{L}\p{N}]/u;

/** A key in the text: at the start of a word (a stem matches its endings); Chinese anywhere. */
const hasKey = (q: string, key: string): boolean => {
  if (CJK.test(key)) return q.includes(key);
  let at = q.indexOf(key);
  while (at >= 0) {
    if (at === 0 || !LETTER.test(q[at - 1])) return true;
    at = q.indexOf(key, at + 1);
  }
  return false;
};
/** A whole word or phrase in the text (Chinese anywhere). */
const hasWhole = (q: string, key: string): boolean => {
  if (CJK.test(key)) return q.includes(key);
  let at = q.indexOf(key);
  while (at >= 0) {
    const after = q[at + key.length];
    if ((at === 0 || !LETTER.test(q[at - 1])) && (after === undefined || !LETTER.test(after))) return true;
    at = q.indexOf(key, at + 1);
  }
  return false;
};
const anyKey = (q: string, keys: string[]) => keys.some((k) => hasKey(q, k));
const anyWhole = (q: string, keys: string[]) => keys.some((k) => hasWhole(q, k));
const flatAll = (keys: string[]) => [...new Set(keys.map(flat))];

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const ACCENTS: Record<string, string> = { a: 'aáàâãä', e: 'eéèêë', i: 'iíìîï', o: 'oóòôõö', u: 'uúùûü', c: 'cç', n: 'nñ', и: 'ий', е: 'её' };
/** A word as a pattern that takes it with or without accents, for text as typed. */
const loose = (word: string): string =>
  [...plain(word)].map((ch) => (ACCENTS[ch] ? `[${ACCENTS[ch]}]` : ch === ' ' ? '\\s+' : escape(ch))).join('');
const alts = (list: string[]) => [...new Set(list)].sort((a, b) => b.length - a.length).map(loose).join('|');

/** Edits between two words (insert, delete, change, swap two letters). */
export const editDistance = (a: string, b: string): number => {
  if (a === b) return 0;
  const d: number[][] = Array.from({ length: a.length + 1 }, (_, i) => [i, ...Array(b.length).fill(0)]);
  for (let j = 1; j <= b.length; j++) d[0][j] = j;
  for (let i = 1; i <= a.length; i++) {
    for (let j = 1; j <= b.length; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      d[i][j] = Math.min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + cost);
      if (i > 1 && j > 1 && a[i - 1] === b[j - 2] && a[i - 2] === b[j - 1]) d[i][j] = Math.min(d[i][j], d[i - 2][j - 2] + 1);
    }
  }
  return d[a.length][b.length];
};

/** A typed word is this word, or its start (stems), allowing a typo in longer words. */
const wordLike = (typed: string, stem: string): boolean => {
  if (typed.startsWith(stem)) return true;
  if (stem.length < 5) return false;
  const cut = typed.slice(0, stem.length + 1);
  return editDistance(cut.slice(0, stem.length), stem) <= (stem.length >= 8 ? 2 : 1) || editDistance(typed, stem) <= 1;
};

/**
 * Each question and the words or phrases that ask it (doctorWords.ts). A phrase (several words)
 * weighs more than one word; a stem matches its endings ("crashe" -> crasheó, crashea).
 */
const LEXICON = Object.fromEntries(Object.entries(INTENT_WORDS).map(([k, v]) => [k, flatAll(v)])) as Record<ChipKey, string[]>;

/** How strongly a text asks each question, best first. */
export const scoreIntents = (text: string): { intent: ChipKey; score: number }[] => {
  const q = ` ${flat(text)} `;
  const ws = words(text);
  const out: { intent: ChipKey; score: number }[] = [];
  for (const [intent, keys] of Object.entries(LEXICON) as [ChipKey, string[]][]) {
    let score = 0;
    // each typed word counts once per question, by the key it fits best: the same stem in two
    // languages ("error", "erro") is not two words
    const best = new Map<string, number>();
    for (const key of keys) {
      if (CJK.test(key)) {
        // no spaces in Chinese: a longer word found says more
        if (q.includes(key)) best.set(`zh:${key}`, key.length >= 2 ? 1 + (key.length - 2) * 0.5 : 0.8);
        continue;
      }
      const parts = key.split(' ');
      if (parts.length > 1) {
        if (q.includes(` ${key}`)) score += parts.length;
        else if (parts.every((p) => ws.some((w) => wordLike(w, p)))) score += parts.length * 0.7;
        continue;
      }
      for (const w of ws) {
        const s = w === key ? 1 : wordLike(w, key) ? 0.8 : 0;
        if (s > (best.get(w) ?? 0)) best.set(w, s);
      }
    }
    // a Chinese word inside a longer one found ("崩溃" in "崩溃了吗") is not counted twice
    const zh = [...best.keys()].filter((k) => k.startsWith('zh:')).map((k) => k.slice(3));
    for (const k of zh) if (zh.some((o) => o !== k && o.includes(k))) best.delete(`zh:${k}`);
    for (const s of best.values()) score += s;
    if (score > 0) out.push({ intent, score });
  }
  // "group" alone is an order, not a request for suggestions
  return out.sort((a, b) => b.score - a.score);
};

/** Which question a text asks, when one stands out. */
export const intentOf = (text: string): ChipKey | null => {
  const remembered = rememberedIntent(text);
  if (remembered) return remembered;
  const s = scoreIntents(text);
  if (s.length === 0 || s[0].score < 0.8) return null;
  return s[0].intent;
};

const GUIDE = flatAll(GUIDE_STARTS).sort((a, b) => b.length - a.length);
/** A question about the app itself ("how do I make a profile?", "what is a compatible copy?"): the guide answers it. */
export const asksTheGuide = (text: string): boolean => {
  const q = flat(text);
  return GUIDE.some((s) => q.startsWith(s) && !LETTER.test(q[s.length] ?? ' ')) || GUIDE_ANYWHERE.some((s) => q.includes(s));
};

const SPLIT = new RegExp(
  [
    '[?？]\\s*',
    ...SPLIT_AND.map(([and, qs]) => `\\s+${loose(and)}\\s+(?=(?:${alts(qs)})(?![\\p{L}\\p{N}]))`),
    `\\s*(?:${SPLIT_AND_ZH.join('|')})\\s*`,
  ].join('|'),
  'iu',
);
/** Several questions in one message, each on its own ("what changed and which mods fail?"). */
export const splitQuestions = (text: string): string[] => {
  const parts = text
    .split(SPLIT)
    .map((p) => p.trim())
    .filter((p) => p.length > 2 || (CJK.test(p) && p.length >= 2));
  return parts.length >= 2 ? parts : [text];
};

/** A question to the Workshop about a mod: is there an update, what do people say. */
export type WorkshopAsk = 'update' | 'comments';
const UPDATE = flatAll(WORKSHOP_UPDATE);
const COMMENTS = flatAll(WORKSHOP_COMMENTS);
export const workshopAskOf = (text: string): WorkshopAsk | null => {
  const q = flat(text);
  if (anyKey(q, UPDATE)) return 'update';
  if (anyKey(q, COMMENTS)) return 'comments';
  return null;
};

/** Words in comments that say something does not work (as written, and without accents). */
export const TROUBLE_WORDS = new RegExp([...new Set([...TROUBLE, ...TROUBLE.map(plain)])].map(escape).join('|').replace(/'/g, "['’]?") + '|42\\.2\\d', 'i');

/** Two questions it may be, when neither stands out (for "did you mean…?"). */
export const intentChoices = (text: string): ChipKey[] => {
  const s = scoreIntents(text);
  if (s.length < 2) return [];
  return s[1].score >= s[0].score * 0.85 ? [s[0].intent, s[1].intent] : [];
};

/** The installed mods a text names: by name or id, the longest first, none counted twice. */
export const modsNamedIn = (text: string, mods: ModInfo[]): ModInfo[] => {
  const q = ` ${plain(text)} `;
  const cands = mods
    .flatMap((m) => [
      { m, key: plain(m.name) },
      { m, key: plain(m.mod_id) },
    ])
    .filter((c) => c.key.length >= 3)
    .sort((a, b) => b.key.length - a.key.length);
  const found: ModInfo[] = [];
  let rest = q;
  for (const c of cands) {
    const at = rest.indexOf(c.key);
    if (at < 0) continue;
    const before = rest[at - 1] ?? ' ';
    const after = rest[at + c.key.length] ?? ' ';
    if (/[a-z0-9]/.test(before) || /[a-z0-9]/.test(after)) continue;
    if (!found.includes(c.m)) found.push(c.m);
    rest = rest.slice(0, at) + ' '.repeat(c.key.length) + rest.slice(at + c.key.length);
  }
  return found;
};

/** The text with the mods it names taken out: "Viewpoint - Light Fix" is a mod, not a question about fixes. */
export const withoutModNames = (text: string, named: ModInfo[]): string => {
  let q = plain(text);
  for (const m of named) for (const k of [plain(m.name), plain(m.mod_id)]) if (k.length >= 3) q = q.split(k).join(' ');
  return q;
};

/** One mod by a name the player typed: exactly, else the ones whose name holds it, else one a typo away. */
export const modByName = (typed: string, mods: ModInfo[]): ModInfo[] => {
  const q = plain(typed).replace(/^"|"$/g, '');
  if (!q) return [];
  const exact = mods.filter((m) => plain(m.name) === q || plain(m.mod_id) === q);
  if (exact.length) return exact.slice(0, 1);
  const holding = mods.filter((m) => plain(m.name).includes(q) || plain(m.mod_id).includes(q));
  if (holding.length) return holding;
  if (q.length < 5) return [];
  return mods.filter((m) => editDistance(plain(m.name), q) <= 2 || editDistance(plain(m.mod_id), q) <= 2);
};

const LIST_SPLIT = new RegExp(`\\s*(?:,|;|、|，|\\s(?:${LIST_AND.map(loose).join('|')})\\s)\\s*`, 'iu');
const splitList = (s: string): string[] =>
  s
    .split(LIST_SPLIT)
    .map((x) => x.trim().replace(/^"|"$/g, ''))
    .filter(Boolean);

/** An order to change groups or the Studio's notes, read from the words (original case kept). */
export type Order =
  | { kind: 'add'; mods: string[]; group: string }
  | { kind: 'remove'; mods: string[]; group: string }
  | { kind: 'rename'; group: string; to: string }
  | { kind: 'delete'; group: string }
  | { kind: 'note'; mod: string; text: string }
  | { kind: 'noteRemove'; mod: string }
  | { kind: 'listGroups' };

/** Each language's order patterns, built once from its words. */
const ORDER_PATTERNS = ORDER_WORDS.map((w) => {
  const the = w.the.length ? `(?:(?:${alts(w.the)})\\s+)?` : '';
  const group = `(?:${alts(w.group)})`;
  const tail = w.tail?.length ? `(?:\\s+(?:${alts(w.tail)}))?` : '';
  const re = (body: string) => new RegExp(`^${body}$`, 'iu');
  return {
    rename: re(`(?:${alts(w.rename)})\\s+${the}${group}\\s+"?(.+?)"?\\s+(?:${alts(w.renameTo)})\\s+"?(.+?)"?${tail}`),
    drop: re(`(?:${alts(w.drop)})\\s+${the}${group}\\s+"?(.+?)"?`),
    remove: re(`(?:${alts(w.remove)})\\s+(.+?)\\s+(?:${alts(w.from)})\\s+${the}${group}\\s+"?(.+?)"?`),
    add: re(`(?:${alts(w.add)})\\s+(.+?)\\s+(?:${alts(w.into)})\\s+${the}(?:${group}\\s+)?"?(.+?)"?`),
  };
});
const LIST_GROUPS_FLAT = new Set(flatAll(LIST_GROUPS));
/** The Studio's notes in the other languages: "nota em A: texto", "Notiz zu A: Text", "备注 A：文本". */
const NOTE = [
  /^nota\s+(?:em|sobre|para|no|na)\s+(.+?)\s*:\s*(.+)$/iu,
  /^note\s+(?:sur|pour|à)\s+(.+?)\s*:\s*(.+)$/iu,
  /^notiz\s+(?:zu|für|an|bei)\s+(.+?)\s*:\s*(.+)$/iu,
  /^(?:заметка|заметку|запиши)\s+(?:к|для|про)\s+(.+?)\s*:\s*(.+)$/iu,
  /^(?:备注|笔记)\s*(.+?)\s*[:：]\s*(.+)$/u,
];
const NOTE_REMOVE = [
  /^(?:apague|apaga|remova|tire)\s+a\s+nota\s+do\s+studio\s+(?:de|do|da|em)\s+(.+)$/iu,
  /^(?:supprime|retire|efface)\s+la\s+note\s+du\s+studio\s+(?:de|sur|pour)\s+(.+)$/iu,
  /^(?:l[öo]sche|entferne)\s+die\s+studio[- ]?notiz\s+(?:von|zu|für|bei)\s+(.+)$/iu,
  /^(?:удали|убери)\s+заметку\s+studio\s+(?:у|для|к|про)\s+(.+)$/iu,
  /^(?:删除|移除)\s*(.+?)\s*的\s*studio\s*备注$/iu,
];

export const orderOf = (text: string): Order | null => {
  const s = text.trim().replace(/[.!?。！？]+$/, '');
  let m = s.match(/^(?:nota|anot[aá]r?|note)\s+(?:en|on|a|para|for|in)\s+(.+?)\s*:\s*(.+)$/i);
  if (m) return { kind: 'note', mod: m[1], text: m[2] };
  m = s.match(/^(?:borr[aá]r?|quit[aá]r?|sac[aá]r?|elimin[aá]r?|remove|delete)\s+(?:la\s+)?(?:the\s+)?(?:nota|note)\s+(?:del\s+studio|studio|of\s+the\s+studio)\s+(?:de|en|of|on|from)\s+(.+)$/i)
    ?? s.match(/^(?:remove|delete)\s+the\s+studio\s+note\s+(?:of|on|from)\s+(.+)$/i);
  if (m) return { kind: 'noteRemove', mod: m[1] };
  for (const re of NOTE_REMOVE) {
    m = s.match(re);
    if (m) return { kind: 'noteRemove', mod: m[1] };
  }
  for (const re of NOTE) {
    m = s.match(re);
    if (m) return { kind: 'note', mod: m[1], text: m[2] };
  }
  // Chinese, with no spaces: "把分组 地图 改名为 世界", "删除 地图 分组", "把 A 从 地图 分组移出", "把 A、B 加到 地图 分组"
  m = s.match(/^把\s*(?:分组\s*)?(.+?)\s*(?:分组|组)\s*(?:重命名为|改名为|改成)\s*(.+)$/u);
  if (m) return { kind: 'rename', group: m[1], to: m[2] };
  m = s.match(/^删除\s*(?:分组\s*)?(.+?)\s*分组$/u) ?? s.match(/^删除分组\s*(.+)$/u);
  if (m) return { kind: 'delete', group: m[1] };
  m = s.match(/^把\s*(.+?)\s*从\s*(.+?)\s*(?:分组|组)?\s*(?:移出|拿出|删掉|去掉|移除)$/u);
  if (m) return { kind: 'remove', mods: splitList(m[1]), group: m[2] };
  m = s.match(/^把\s*(.+?)\s*(?:加到|加入|放进|放到|移到)\s*(.+?)\s*(?:分组|组)?$/u);
  if (m) return { kind: 'add', mods: splitList(m[1]), group: m[2] };
  for (const p of ORDER_PATTERNS) {
    m = s.match(p.rename);
    if (m) return { kind: 'rename', group: m[1], to: m[2] };
  }
  for (const p of ORDER_PATTERNS) {
    m = s.match(p.drop);
    if (m) return { kind: 'delete', group: m[1] };
  }
  for (const p of ORDER_PATTERNS) {
    m = s.match(p.remove);
    if (m) return { kind: 'remove', mods: splitList(m[1]), group: m[2] };
  }
  for (const p of ORDER_PATTERNS) {
    m = s.match(p.add);
    if (m) return { kind: 'add', mods: splitList(m[1]), group: m[2] };
  }
  if (LIST_GROUPS_FLAT.has(flat(s))) return { kind: 'listGroups' };
  return null;
};

/** A follow-up to the last answer: what to do with the mod being talked about. */
export type FollowUp = 'disable' | 'enable' | 'note' | 'show' | 'why' | 'errors' | 'history';

const F = Object.fromEntries(Object.entries(FOLLOW).map(([k, v]) => [k, flatAll(v)])) as Record<keyof typeof FOLLOW, string[]>;
const LEANS_FLAT = flatAll(LEANS);
const LEAN_STARTS_FLAT = flatAll(LEAN_STARTS);
const GENERAL_FLAT = flatAll(GENERAL);

/** The text leans on what was just talked about: "and…", "that one", "it". */
export const leansOnLast = (text: string): boolean => {
  const q = flat(text);
  const ws = words(text);
  return (
    LEAN_STARTS_FLAT.some((w) => (CJK.test(w) ? q.startsWith(w) : q === w || q.startsWith(`${w} `))) ||
    ws.some((w) => LEANS_FLAT.includes(w)) ||
    LEANS_FLAT.some((w) => CJK.test(w) && q.includes(w))
  );
};

export const followUpOf = (text: string): FollowUp | null => {
  const q = flat(text);
  for (const [kind, alone] of Object.entries(FOLLOW_ALONE) as [FollowUp, string[]][]) if (alone.some((w) => flat(w) === q)) return kind;
  if (anyKey(q, F.disable)) return 'disable';
  if (anyKey(q, F.enable)) return 'enable';
  if (anyKey(q, F.note)) return 'note';
  if (anyKey(q, F.show)) return 'show';
  // questions that lean on the last answer: "and…", "that one", or a short "since when?" / "why?".
  // A general question ("which mods fail?", "what changed?") is not a follow-up.
  const ws = words(text);
  const leans = leansOnLast(text);
  const general = ws.some((w) => GENERAL_FLAT.includes(w)) || GENERAL_FLAT.some((w) => CJK.test(w) && q.includes(w));
  if (general) return null;
  const short = ws.length <= 4;
  if ((leans || short) && anyKey(q, F.why)) return 'why';
  if (leans) {
    if (anyKey(q, F.changed)) return 'why';
    if (anyKey(q, F.errors)) return 'errors';
    if (anyKey(q, F.history)) return 'history';
  }
  return null;
};

const T = Object.fromEntries(Object.entries(TIME).map(([k, v]) => [k, flatAll(v)])) as Record<keyof typeof TIME, string[]>;
const LAST_DAYS = new RegExp(`(?:^|[^\\p{L}])(?:${alts(TIME.lastDays)})\\s+(\\d{1,3})\\s+(?:${alts(TIME.days)})(?![\\p{L}])|(?:最近|过去)\\s*(\\d{1,3})\\s*天`, 'u');
const SINCE_DATE = new RegExp(`(?:^|[^\\p{L}])(?:${alts(TIME.since.filter((x) => !CJK.test(x)))})\\s*(\\d{1,2})[/.](\\d{1,2})(?:[/.](\\d{2,4}))?|(?:从|自)\\s*(\\d{1,2})[/.](\\d{1,2})(?:[/.](\\d{2,4}))?`, 'u');

/** A time window the text names, as the unix second it starts at ("since yesterday", "this week", "last 3 days", "since 12/11"). */
export const sinceOf = (text: string, now: Date = new Date()): number | null => {
  const q = flat(text);
  const startOfDay = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate());
  const secs = (d: Date) => Math.floor(d.getTime() / 1000);
  const daysBack = (n: number) => secs(new Date(startOfDay(now).getTime() - n * 86400000));
  if (anyWhole(q, T.today)) return daysBack(0);
  if (anyWhole(q, T.yesterday)) return daysBack(1);
  if (anyWhole(q, T.week)) return daysBack(7);
  if (anyWhole(q, T.month)) return daysBack(30);
  const n = plain(text).match(LAST_DAYS);
  if (n) return daysBack(Number(n[1] ?? n[2]));
  const d = plain(text).match(SINCE_DATE);
  if (d) {
    const [day, month, y] = d[1] ? [d[1], d[2], d[3]] : [d[4], d[5], d[6]];
    const year = y ? (y.length === 2 ? 2000 + Number(y) : Number(y)) : now.getFullYear();
    return secs(new Date(year, Number(month) - 1, Number(day)));
  }
  return null;
};

/** "Which one is the slowest / gives the most errors?" */
export type Superlative = 'slowest' | 'most_errors';
const S = Object.fromEntries(Object.entries(SUPERLATIVE).map(([k, v]) => [k, flatAll(v)])) as Record<keyof typeof SUPERLATIVE, string[]>;
export const superlativeOf = (text: string): Superlative | null => {
  const q = flat(text);
  if (!anyKey(q, S.most)) return null;
  if (anyKey(q, S.slow)) return 'slowest';
  if (anyKey(q, S.errors)) return 'most_errors';
  return null;
};

const LAST_SESSION_FLAT = flatAll(LAST_SESSION);
/** "How did my last session go?" */
export const asksLastSession = (text: string): boolean => anyKey(flat(text), LAST_SESSION_FLAT);

const PROFILE = new RegExp(`(?:^|[^\\p{L}])(?:${alts(PROFILE_WORDS.filter((x) => !CJK.test(x)))})\\s+(.+)$|(?:${PROFILE_WORDS.filter((x) => CJK.test(x)).join('|')})\\s*(.+)$`, 'u');
/** "profile Knox Lives": the profile's name as typed (plain), or null. */
export const profileNamed = (text: string): string | null => {
  const m = plain(text).match(PROFILE);
  return m ? (m[1] ?? m[2] ?? '').trim() || null : null;
};

/** Groups worth suggesting among the active mods no group holds yet: the same first word in the
 * name (three or more), or three or more requiring the same mod. */
export const suggestGroups = (active: ModInfo[], grouped: Set<string>, all: ModInfo[]): { group: string; mods: ModInfo[] }[] => {
  const free = active.filter((m) => !grouped.has(m.mod_id));
  const out: { group: string; mods: ModInfo[] }[] = [];
  const taken = new Set<string>();
  const generic = new Set(['the', 'a', 'an', 'more', 'better', 'mod', 'mods', 'b42', 'b41', 'pz', 'project', 'zomboid', 'simple', 'improved', 'realistic', 'new']);
  const byWord = new Map<string, ModInfo[]>();
  for (const m of free) {
    const word = plain(m.name).split(/[^a-z0-9]+/).find((w) => w.length >= 4 && !generic.has(w));
    if (word) byWord.set(word, [...(byWord.get(word) ?? []), m]);
  }
  for (const [word, list] of [...byWord.entries()].sort((a, b) => b[1].length - a[1].length)) {
    if (list.length < 3) continue;
    const name = list[0].name.split(/[\s\-_:[\]()]+/).find((w) => plain(w) === word) ?? word;
    out.push({ group: name, mods: list });
    list.forEach((m) => taken.add(m.mod_id));
  }
  const byNeed = new Map<string, ModInfo[]>();
  for (const m of free.filter((x) => !taken.has(x.mod_id))) {
    for (const dep of m.dependencies ?? []) {
      const id = dep.replace(/^\\/, '');
      byNeed.set(id, [...(byNeed.get(id) ?? []), m]);
    }
  }
  for (const [dep, list] of [...byNeed.entries()].sort((a, b) => b[1].length - a[1].length)) {
    const rest = list.filter((m) => !taken.has(m.mod_id));
    if (rest.length < 3) continue;
    const lib = all.find((m) => m.mod_id === dep);
    out.push({ group: lib?.name ?? dep, mods: rest });
    rest.forEach((m) => taken.add(m.mod_id));
  }
  return out.slice(0, 4);
};

// ---------------------------------------------------------------------------------------------
// Nicknames learned on this PC: what the player typed for a mod, once picked among several.
// ---------------------------------------------------------------------------------------------

const ALIASES_KEY = 'pz_doctor_aliases';

const loadAliases = (): Record<string, string> => {
  try {
    return typeof localStorage === 'undefined' ? {} : (JSON.parse(localStorage.getItem(ALIASES_KEY) || '{}') as Record<string, string>);
  } catch {
    return {};
  }
};

export const rememberAlias = (typed: string, modId: string): void => {
  const key = plain(typed);
  if (key.length < 2 || typeof localStorage === 'undefined') return;
  const all = loadAliases();
  all[key] = modId;
  localStorage.setItem(ALIASES_KEY, JSON.stringify(Object.fromEntries(Object.entries(all).slice(-200))));
};

/** The mods a text names by a nickname learned before (whole words only). */
export const aliasedIn = (text: string, mods: ModInfo[]): ModInfo[] => {
  const q = ` ${plain(text)} `;
  const out: ModInfo[] = [];
  for (const [alias, id] of Object.entries(loadAliases())) {
    const at = q.indexOf(` ${alias}`);
    if (at < 0 || /[a-z0-9]/.test(q[at + alias.length + 1] ?? ' ')) continue;
    const m = mods.find((x) => x.mod_id === id);
    if (m && !out.includes(m)) out.push(m);
  }
  return out;
};

// ---------------------------------------------------------------------------------------------
// Phrases learned on this PC: what was not understood, then shown with a button.
// ---------------------------------------------------------------------------------------------

const PHRASES_KEY = 'pz_doctor_phrases';

const loadPhrases = (): Record<string, ChipKey> => {
  try {
    return typeof localStorage === 'undefined' ? {} : (JSON.parse(localStorage.getItem(PHRASES_KEY) || '{}') as Record<string, ChipKey>);
  } catch {
    return {};
  }
};

/** Remembers that this phrase asks this question. */
export const rememberPhrase = (text: string, intent: ChipKey): void => {
  const key = words(text).join(' ');
  if (!key || typeof localStorage === 'undefined') return;
  const all = loadPhrases();
  all[key] = intent;
  // the newest 200
  const entries = Object.entries(all).slice(-200);
  localStorage.setItem(PHRASES_KEY, JSON.stringify(Object.fromEntries(entries)));
};

/** A phrase learned before: the same words, or nearly (most of them shared). */
export const rememberedIntent = (text: string): ChipKey | null => {
  const ws = words(text);
  if (ws.length === 0) return null;
  const all = loadPhrases();
  const exact = all[ws.join(' ')];
  if (exact) return exact;
  const set = new Set(ws);
  for (const [phrase, intent] of Object.entries(all)) {
    const other = phrase.split(' ');
    const shared = other.filter((w) => set.has(w)).length;
    if (shared / Math.max(other.length, ws.length) >= 0.8) return intent;
  }
  return null;
};
