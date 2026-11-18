import { ModInfo } from '../types';

/**
 * How the Doctor's chat reads what the player types (components/health/DoctorChat): which
 * question it asks, which mods it names, when ("since yesterday"), whether it follows the last
 * answer ("and since when?", "switch it off"), and the orders that change groups or the Studio's
 * notes. By words, in Spanish and English alike, forgiving typos; no AI. What it did not
 * understand and the player then showed with a button is remembered on this PC.
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

const words = (s: string): string[] => plain(s).split(/[^a-z0-9]+/).filter(Boolean);

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
 * Each question and the words or phrases that ask it. A phrase (several words) weighs more than
 * one word; a stem matches its endings ("crashe" -> crasheó, crashea).
 */
const LEXICON: Record<ChipKey, string[]> = {
  culprit: ['culpable', 'que mod hace', 'que mod causa', 'que mod rompe', 'quien rompe', 'quien causa', 'que lo causa', 'which mod causes', 'which mod makes', 'which mod breaks', 'culprit', 'sospechoso', 'suspect', 'who breaks'],
  hunt: ['buscalo', 'busca el culpable', 'buscar el culpable', 'busquemos', 'por mitades', 'bisect', 'hunt', 'find the culprit', 'find it', 'sigue pasando', 'ya no pasa', 'still happens', 'no longer happens'],
  week: ['que paso', 'what happened', 'como me fue', 'parte', 'novedades', 'report', 'que hubo', 'how did it go'],
  undo: ['deshace', 'deshacer', 'undo', 'volve atras', 'revert', 'revertir'],
  marks: ['marca', 'marque', 'marcado', 'marcados', 'mark', 'marked', 'momento que marque', 'moments i marked'],
  help: ['ayuda', 'help', 'que puedo', 'que podes', 'what can', 'como funciona', 'que haces', 'how does'],
  tools: ['herramienta', 'tool', 'mcp'],
  groups: ['suger grupo', 'suggest group', 'propone grupo', 'organiza', 'ordena mis mods', 'agrupar mis', 'organize'],
  notes: ['anota lo aprendido', 'anotar lo aprendido', 'note what you learned', 'anota lo que aprendiste', 'notas de lo aprendido', 'write what you learned'],
  changed: ['que cambio', 'cambio', 'cambie', 'changed', 'change', 'desde cuando', 'empezo', 'empieza', 'began', 'begin', 'since when', 'diferencia', 'distinto', 'different', 'difference'],
  crash: ['cerro', 'cierra', 'cierre', 'crash', 'crashe', 'no arranca', 'no carga', 'no inicia', 'no abre', 'no entra', 'se cae', 'cayo', 'caida', 'colgo', 'cuelga', 'tildo', 'tilda', 'congel', 'freez', 'froze', 'close', 'closed', 'exploto', 'revento', 'memoria', 'memory', 'ram', 'out of memory'],
  fixes: ['arreglo', 'funciono', 'sirvio', 'fix', 'worked', 'solucion', 'solucione'],
  slow: ['lento', 'lenta', 'tarda', 'slow', 'carga', 'loading', 'fps', 'lag', 'pesad', 'rendimiento', 'performance', 'tiron', 'stutter', 'demora'],
  errors: ['error', 'falla', 'fail', 'problema', 'problem', 'bug', 'roto', 'broken', 'anda mal', 'no anda', 'no funciona', 'not work'],
  summary: ['que sabes', 'resumen', 'aprendiste', 'what do you know', 'summary', 'hola', 'hello', 'buenas', 'estado', 'como esta', 'como anda', 'status', 'que tal'],
};

/** How strongly a text asks each question, best first. */
export const scoreIntents = (text: string): { intent: ChipKey; score: number }[] => {
  const q = ` ${plain(text)} `;
  const ws = words(text);
  const out: { intent: ChipKey; score: number }[] = [];
  for (const [intent, keys] of Object.entries(LEXICON) as [ChipKey, string[]][]) {
    let score = 0;
    for (const key of keys) {
      const parts = key.split(' ');
      if (parts.length > 1) {
        if (q.includes(` ${key}`)) score += parts.length;
        else if (parts.every((p) => ws.some((w) => wordLike(w, p)))) score += parts.length * 0.7;
      } else if (ws.some((w) => w === key)) score += 1;
      else if (ws.some((w) => wordLike(w, key))) score += 0.8;
    }
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

/** A question about the app itself ("how do I make a profile?", "what is a compatible copy?"): the guide answers it. */
export const asksTheGuide = (text: string): boolean => /^(como|para que|que es|que son|donde|cuando uso|how|what is|what are|where|what does|why does)\b/.test(plain(text));

/** Several questions in one message, each on its own ("what changed and which mods fail?"). */
export const splitQuestions = (text: string): string[] => {
  const parts = text
    .split(/\?\s*|\s+y\s+(?=(?:que|qué|cual|cuál|como|cómo|por|cuando|cuándo|donde|dónde|hay|cuantas|cuántas)\s)|\s+and\s+(?=(?:what|which|how|why|when|where|is|are|did|any)\b)/i)
    .map((p) => p.trim())
    .filter((p) => p.length > 2);
  return parts.length >= 2 ? parts : [text];
};

/** A question to the Workshop about a mod: is there an update, what do people say. */
export type WorkshopAsk = 'update' | 'comments';
export const workshopAskOf = (text: string): WorkshopAsk | null => {
  const q = plain(text);
  if (/(version nueva|nueva version|actualizacion|actualizo|se actualizo|update|updated|newer)/.test(q)) return 'update';
  if (/(comentario|comments|que dicen|que opinan|opiniones|reviews|what do people say|what people say)/.test(q)) return 'comments';
  return null;
};

/** Words in comments that say something does not work. */
export const TROUBLE_WORDS = /(error|crash|broken|not work|doesn.?t work|bug|roto|no funciona|no anda|falla|42\.2\d)/i;

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

const splitList = (s: string): string[] =>
  s
    .split(/\s*(?:,|;|\sy\s|\sand\s)\s*/i)
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

export const orderOf = (text: string): Order | null => {
  const s = text.trim().replace(/[.!?]+$/, '');
  let m = s.match(/^(?:nota|anot[aá]r?|note)\s+(?:en|on|a|para|for|in)\s+(.+?)\s*:\s*(.+)$/i);
  if (m) return { kind: 'note', mod: m[1], text: m[2] };
  m = s.match(/^(?:borr[aá]r?|quit[aá]r?|sac[aá]r?|elimin[aá]r?|remove|delete)\s+(?:la\s+)?(?:the\s+)?(?:nota|note)\s+(?:del\s+studio|studio|of\s+the\s+studio)\s+(?:de|en|of|on|from)\s+(.+)$/i)
    ?? s.match(/^(?:remove|delete)\s+the\s+studio\s+note\s+(?:of|on|from)\s+(.+)$/i);
  if (m) return { kind: 'noteRemove', mod: m[1] };
  m = s.match(/^(?:renombr[aá]r?|rename)\s+(?:el\s+)?(?:grupo|group)\s+"?(.+?)"?\s+(?:a|to|como|as)\s+"?(.+?)"?$/i);
  if (m) return { kind: 'rename', group: m[1], to: m[2] };
  m = s.match(/^(?:borr[aá]r?|elimin[aá]r?|delete|remove)\s+(?:el\s+)?(?:grupo|group)\s+"?(.+?)"?$/i);
  if (m) return { kind: 'delete', group: m[1] };
  m = s.match(/^(?:sac[aá]r?|quit[aá]r?|remove|take)\s+(.+?)\s+(?:out\s+of|del|de|from)\s+(?:el\s+|the\s+)?(?:grupo|group)\s+"?(.+?)"?$/i);
  if (m) return { kind: 'remove', mods: splitList(m[1]), group: m[2] };
  m = s.match(/^(?:agrup[aá]r?|group|pon[eé]?|pone|met[eé]r?|agreg[aá]r?|add|put)\s+(.+?)\s+(?:en|in|into|to|al|a)\s+(?:el\s+|the\s+)?(?:grupo\s+|group\s+)?"?(.+?)"?$/i);
  if (m) return { kind: 'add', mods: splitList(m[1]), group: m[2] };
  if (/^(?:qu[eé]\s+grupos|list(?:a|ar)?\s+(?:de\s+)?(?:los\s+)?grupos|list\s+groups|(?:my\s+)?groups\??|grupos\??|mis\s+grupos)$/i.test(s)) return { kind: 'listGroups' };
  return null;
};

/** A follow-up to the last answer: what to do with the mod being talked about. */
export type FollowUp = 'disable' | 'enable' | 'note' | 'show' | 'why' | 'errors' | 'history';

export const followUpOf = (text: string): FollowUp | null => {
  const q = plain(text);
  const has = (...k: string[]) => k.some((x) => q.includes(x));
  if (has('apagalo', 'desactivalo', 'sacalo de la lista', 'turn it off', 'disable it', 'switch it off', 'apagar ese', 'apaga ese')) return 'disable';
  if (has('prendelo', 'activalo', 'encendelo', 'turn it on', 'enable it', 'switch it on')) return 'enable';
  if (has('anotalo', 'anota eso', 'note it', 'write it down', 'guarda eso')) return 'note';
  if (has('mostramelo', 'mostrame', 'donde esta', 'ver en la lista', 'llevame', 'show it', 'show me', 'where is it', 'abrilo', 'open it')) return 'show';
  // questions that lean on the last answer: "and…", "that one", or a short "since when?" / "why?".
  // A general question ("which mods fail?", "what changed?") is not a follow-up.
  const ws = words(text);
  const leans = /^(y|and)\b/.test(q) || ws.some((w) => ['ese', 'este', 'eso', 'esa', 'esto', 'it', 'that', 'this', 'lo'].includes(w));
  const general = ws.some((w) => ['mods', 'juego', 'game', 'partida', 'partidas', 'sessions', 'todo', 'everything'].includes(w));
  if (general) return null;
  const short = ws.length <= 4;
  if ((leans || short) && has('desde cuando', 'since when', 'por que', 'porque', 'why', 'cuando empezo')) return 'why';
  if (leans) {
    if (has('que cambio', 'what changed')) return 'why';
    if (has('error', 'falla', 'problema', 'fail')) return 'errors';
    if (has('historial', 'history', 'mas info', 'more', 'contame', 'tell me')) return 'history';
  }
  return null;
};

/** A time window the text names, as the unix second it starts at ("since yesterday", "this week", "last 3 days", "since 12/11"). */
export const sinceOf = (text: string, now: Date = new Date()): number | null => {
  const q = plain(text);
  const startOfDay = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate());
  const secs = (d: Date) => Math.floor(d.getTime() / 1000);
  if (/\b(hoy|today)\b/.test(q)) return secs(startOfDay(now));
  if (/\b(ayer|yesterday)\b/.test(q)) return secs(new Date(startOfDay(now).getTime() - 86400000));
  if (/(esta semana|this week|ultima semana|last week|7 dias|7 days)/.test(q)) return secs(new Date(startOfDay(now).getTime() - 7 * 86400000));
  if (/(este mes|this month|ultimo mes|last month|30 dias|30 days)/.test(q)) return secs(new Date(startOfDay(now).getTime() - 30 * 86400000));
  const n = q.match(/(?:ultimos|last)\s+(\d{1,3})\s+(?:dias|days)/);
  if (n) return secs(new Date(startOfDay(now).getTime() - Number(n[1]) * 86400000));
  const d = q.match(/(?:desde el|desde|since)\s+(\d{1,2})\/(\d{1,2})(?:\/(\d{2,4}))?/);
  if (d) {
    const year = d[3] ? (d[3].length === 2 ? 2000 + Number(d[3]) : Number(d[3])) : now.getFullYear();
    return secs(new Date(year, Number(d[2]) - 1, Number(d[1])));
  }
  return null;
};

/** "Which one is the slowest / gives the most errors?" */
export type Superlative = 'slowest' | 'most_errors';
export const superlativeOf = (text: string): Superlative | null => {
  const q = plain(text);
  const most = /(cual|que mod|which|the most|el mas|la mas|mas |peor|worst|top)/.test(q);
  if (!most) return null;
  if (/(tarda|lento|slow|carga|load|pesad)/.test(q)) return 'slowest';
  if (/(error|falla|fail|problema|problem)/.test(q)) return 'most_errors';
  return null;
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
