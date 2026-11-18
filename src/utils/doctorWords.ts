/**
 * The words the Doctor's chat reads, in every language the app has (doctorChat.ts uses them).
 * Written as people type them; doctorChat folds case and accents on both sides, so "qué cambió"
 * and "que cambio", "geändert" and "geandert" are the same. A single word matches its endings
 * ("crashe" -> crasheó); Chinese has no spaces, so its words match anywhere in the text.
 */

import type { ChipKey } from './doctorChat';

/** Each question and the words or phrases that ask it, by language. */
export const INTENT_WORDS: Record<ChipKey, string[]> = {
  culprit: [
    // es / en
    'culpable', 'que mod hace', 'que mod causa', 'que mod rompe', 'quien rompe', 'quien causa', 'que lo causa', 'which mod causes', 'which mod makes', 'which mod breaks', 'culprit', 'sospechoso', 'suspect', 'who breaks',
    // pt
    'qual mod causa', 'qual mod quebra', 'qual mod faz', 'culpado', 'suspeito', 'quem quebra',
    // fr
    'quel mod cause', 'quel mod casse', 'quel mod fait', 'coupable', 'qui casse',
    // de
    'welche mod verursacht', 'welcher mod verursacht', 'schuldig', 'verursacht', 'verdächtig',
    // ru
    'какой мод вызывает', 'какой мод ломает', 'виноват', 'виновник', 'подозрева', 'кто ломает',
    // zh
    '哪个模组导致', '哪个模组引起', '罪魁', '元凶', '嫌疑',
  ],
  hunt: [
    'buscalo', 'busca el culpable', 'buscar el culpable', 'busquemos', 'por mitades', 'bisect', 'hunt', 'find the culprit', 'find it', 'sigue pasando', 'ya no pasa', 'still happens', 'no longer happens',
    'procure o culpado', 'procurar o culpado', 'encontre o culpado', 'pela metade', 'ainda acontece', 'não acontece mais',
    'trouve le coupable', 'trouver le coupable', 'par moitiés', 'ça arrive encore', 'ça arrive toujours', "n'arrive plus", 'arrive plus',
    'finde den schuldigen', 'schuldigen finden', 'halbieren', 'passiert immer noch', 'passiert noch', 'passiert nicht mehr',
    'найди виновника', 'найти виновника', 'пополам', 'всё ещё происходит', 'все еще происходит', 'больше не происходит',
    '找出罪魁', '找出元凶', '二分', '对半', '还在发生', '还会发生', '还是会', '不再发生', '没有再发生',
  ],
  week: [
    'que paso', 'what happened', 'como me fue', 'parte', 'novedades', 'report', 'que hubo', 'how did it go',
    'o que aconteceu', 'como foi', 'relatório', 'novidades',
    "qu'est-ce qui s'est passé", "que s'est-il passé", "comment ça s'est passé", 'rapport',
    'was ist passiert', 'wie lief', 'bericht', 'neuigkeiten',
    'что случилось', 'что произошло', 'что было', 'как прошло', 'отчёт',
    '发生了什么', '怎么样了', '报告', '这周怎么样',
  ],
  undo: [
    'deshace', 'deshacer', 'undo', 'volve atras', 'revert', 'revertir',
    'desfaz', 'desfazer', 'voltar atrás', 'reverter',
    'annule', 'annuler', 'revenir en arrière',
    'rückgängig', 'mach das rückgängig', 'widerrufe',
    'отмени', 'отменить', 'верни как было',
    '撤销', '撤回', '还原',
  ],
  marks: [
    'marca', 'marque', 'marcado', 'marcados', 'mark', 'marked', 'momento que marque', 'moments i marked',
    'marquei', 'momento que marquei',
    "j'ai marqué", 'moment marqué',
    'markiert', 'markierung', 'markiere',
    'отметил', 'отметка', 'отметки', 'отмеченн',
    '标记', '我标记',
  ],
  help: [
    'ayuda', 'help', 'que puedo', 'que podes', 'what can', 'como funciona', 'que haces', 'how does',
    'ajuda', 'o que você pode', 'o que você faz',
    'aide', 'que peux-tu', 'que pouvez-vous', 'comment ça marche',
    'hilfe', 'was kannst du', 'wie funktioniert',
    'помощь', 'помоги', 'что ты умеешь', 'как это работает',
    '帮助', '你能做什么', '怎么用',
  ],
  tools: ['herramienta', 'tool', 'mcp', 'ferramenta', 'outil', 'werkzeug', 'инструмент', '工具'],
  groups: [
    'suger grupo', 'suggest group', 'propone grupo', 'organiza', 'ordena mis mods', 'agrupar mis', 'organize',
    'sugira grupos', 'sugerir grupos', 'organizar meus mods',
    'suggère des groupes', 'propose des groupes', 'organiser', 'range mes mods',
    'gruppen vorschlagen', 'schlage gruppen', 'ordne meine mods', 'organisieren',
    'предложи группы', 'сгруппируй мои', 'упорядочи', 'организуй',
    '建议分组', '帮我分组', '整理模组',
  ],
  notes: [
    'anota lo aprendido', 'anotar lo aprendido', 'note what you learned', 'anota lo que aprendiste', 'notas de lo aprendido', 'write what you learned',
    'anote o que aprendeu', 'anotar o que aprendeu',
    'note ce que tu as appris', 'notez ce que vous avez appris',
    'notiere was du gelernt', 'schreib auf was du gelernt',
    'запиши что узнал', 'запиши выученное',
    '记下学到的', '记录学到的',
  ],
  changed: [
    'que cambio', 'cambio', 'cambie', 'changed', 'change', 'desde cuando', 'empezo', 'empieza', 'began', 'begin', 'since when', 'diferencia', 'distinto', 'different', 'difference',
    'o que mudou', 'mudou', 'mudei', 'desde quando', 'começou', 'diferença', 'diferente',
    "qu'est-ce qui a changé", 'ce qui a changé', 'changé', 'depuis quand', 'différence', 'différent',
    'was hat sich geändert', 'geändert', 'änderung', 'seit wann', 'angefangen', 'unterschied',
    'что изменилось', 'изменил', 'изменен', 'с каких пор', 'началось', 'разница', 'отличи',
    '改变', '变化', '变了', '什么时候开始', '从什么时候', '区别', '不同',
  ],
  crash: [
    'cerro', 'cierra', 'cierre', 'crash', 'crashe', 'no arranca', 'no carga', 'no inicia', 'no abre', 'no entra', 'se cae', 'cayo', 'caida', 'colgo', 'cuelga', 'tildo', 'tilda', 'congel', 'freez', 'froze', 'close', 'closed', 'exploto', 'revento', 'memoria', 'memory', 'ram', 'out of memory',
    'fechou', 'travou', 'trava', 'congelou', 'caiu', 'não abre', 'não inicia', 'não carrega', 'não entra',
    'fermé', 'se ferme', 'plante', 'planté', 'figé', 'gelé', 'ne démarre pas', 'ne se lance pas', 'ne charge pas', 'mémoire',
    'absturz', 'abgestürzt', 'stürzt ab', 'geschlossen', 'hängt', 'eingefroren', 'startet nicht', 'lädt nicht', 'speicher',
    'вылет', 'вылетает', 'вылетел', 'крашит', 'закрыл', 'зависа', 'завис', 'не запуска', 'не загружа', 'памят',
    '崩溃', '崩了', '闪退', '卡死', '卡住', '打不开', '无法启动', '启动不了', '内存', '自己关了',
  ],
  fixes: [
    'arreglo', 'funciono', 'sirvio', 'fix', 'worked', 'solucion', 'solucione',
    'consertou', 'funcionou', 'resolveu', 'correção', 'conserto',
    'réparation', 'corrigé', 'correctif', 'ça a marché', 'a fonctionné',
    'repariert', 'behoben', 'hat funktioniert', 'lösung',
    'исправлен', 'исправлени', 'помогло', 'сработало', 'починил',
    '修复', '有用吗', '解决了', '起作用',
  ],
  slow: [
    'lento', 'lenta', 'tarda', 'slow', 'carga', 'loading', 'fps', 'lag', 'pesad', 'rendimiento', 'performance', 'tiron', 'stutter', 'demora',
    'desempenho', 'engasga', 'carregamento',
    'lent', 'lente', 'ralenti', 'saccade', 'chargement', 'met du temps',
    'langsam', 'ruckel', 'dauert', 'leistung', 'ladezeit',
    'медленн', 'тормоз', 'лаг', 'долго', 'производительн', 'загрузк', 'фпс',
    '慢', '卡顿', '加载', '帧数', '性能', '掉帧',
  ],
  errors: [
    'error', 'falla', 'fail', 'problema', 'problem', 'bug', 'roto', 'broken', 'anda mal', 'no anda', 'no funciona', 'not work',
    'erro', 'falha', 'quebrado', 'não funciona',
    'erreur', 'échec', 'problème', 'cassé', 'ne marche pas', 'ne fonctionne pas',
    'fehler', 'kaputt', 'funktioniert nicht', 'geht nicht',
    'ошибк', 'сбой', 'проблем', 'сломан', 'не работает', 'баг',
    '错误', '报错', '出错', '问题', '坏了', '不能用', '故障',
  ],
  summary: [
    'que sabes', 'resumen', 'aprendiste', 'what do you know', 'summary', 'hola', 'hello', 'buenas', 'estado', 'como esta', 'como anda', 'status', 'que tal',
    'o que você sabe', 'resumo', 'olá', 'oi', 'como vai', 'tudo bem',
    'que sais-tu', 'que savez-vous', 'résumé', 'bonjour', 'salut', 'état', 'comment ça va',
    'was weißt du', 'zusammenfassung', 'hallo', 'wie geht', 'zustand',
    'что ты знаешь', 'что знаешь', 'сводка', 'итог', 'привет', 'здравствуй', 'статус', 'как дела',
    '你知道什么', '摘要', '总结', '你好', '状态',
  ],
};

/** How a question about the app itself starts ("how do I…", "what is…"): the guide answers it. */
export const GUIDE_STARTS = [
  'como', 'para que', 'que es', 'que son', 'donde', 'cuando uso', 'how', 'what is', 'what are', 'where', 'what does', 'why does',
  'o que é', 'o que são', 'onde', 'pra que serve', 'para que serve',
  'comment', "qu'est-ce que", "qu'est-ce qu'", "c'est quoi", 'où', 'à quoi sert',
  'wie', 'was ist', 'was sind', 'wo', 'wozu',
  'как', 'что такое', 'где', 'зачем', 'для чего',
];
/** Chinese guide questions, anywhere in the text. */
export const GUIDE_ANYWHERE = ['怎么', '如何', '什么是', '是什么', '在哪'];

/** "and" followed by a new question, per language: [and, question words]. */
export const SPLIT_AND: [string, string[]][] = [
  ['y', ['que', 'qué', 'cual', 'cuál', 'como', 'cómo', 'por', 'cuando', 'cuándo', 'donde', 'dónde', 'hay', 'cuantas', 'cuántas']],
  ['and', ['what', 'which', 'how', 'why', 'when', 'where', 'is', 'are', 'did', 'any']],
  ['e', ['o que', 'qual', 'quais', 'como', 'por que', 'quando', 'onde', 'tem', 'quantos', 'quantas']],
  ['et', ['que', "qu'", 'quel', 'quels', 'quelle', 'quelles', 'comment', 'pourquoi', 'quand', 'où', 'est-ce', 'combien', 'y a-t-il']],
  ['und', ['was', 'welche', 'welcher', 'welches', 'wie', 'warum', 'wann', 'wo', 'gibt', 'wieviele']],
  ['и', ['что', 'какие', 'какой', 'как', 'почему', 'когда', 'где', 'есть', 'сколько']],
];
/** Chinese "and also", which needs no spaces around it. */
export const SPLIT_AND_ZH = ['还有', '另外', '以及'];

export const WORKSHOP_UPDATE = [
  'version nueva', 'nueva version', 'actualizacion', 'actualizo', 'se actualizo', 'update', 'updated', 'newer', 'new version',
  'versão nova', 'nova versão', 'atualização', 'atualizou',
  'nouvelle version', 'mise à jour', 'mis à jour',
  'neue version', 'aktualisier',
  'новая версия', 'новой версии', 'обновлени', 'обновил',
  '新版本', '更新',
];
export const WORKSHOP_COMMENTS = [
  'comentario', 'comments', 'que dicen', 'que opinan', 'opiniones', 'reviews', 'what do people say', 'what people say',
  'comentário', 'o que dizem', 'opiniões',
  'commentaire', "qu'en disent", 'avis',
  'kommentar', 'was sagen', 'meinungen',
  'комментари', 'что говорят', 'отзыв',
  '评论', '怎么说', '评价',
];

/** Words in Workshop comments that say something does not work (comments come in any language). */
export const TROUBLE = [
  'error', 'crash', 'broken', 'not work', "doesn't work", 'doesnt work', 'bug', 'roto', 'no funciona', 'no anda', 'falla',
  'erro', 'não funciona', 'quebrado', 'erreur', 'ne marche pas', 'ne fonctionne pas', 'cassé', 'fehler', 'funktioniert nicht', 'kaputt', 'absturz',
  'ошибк', 'не работает', 'вылет', '错误', '崩溃', '报错', '不能用',
];

/** Follow-ups on the mod being talked about. */
export const FOLLOW = {
  disable: ['apagalo', 'desactivalo', 'sacalo de la lista', 'turn it off', 'disable it', 'switch it off', 'apagar ese', 'apaga ese',
    'desliga', 'desligue', 'desative', 'desativa ele', 'désactive', 'désactiver', 'éteins', 'schalte es aus', 'schalt es aus', 'schalt sie aus', 'schalte sie aus', 'schalt ihn aus', 'schalte ihn aus', 'deaktivier', 'ausschalten', 'выключи', 'отключи', '关掉', '禁用', '停用'],
  enable: ['prendelo', 'activalo', 'encendelo', 'turn it on', 'enable it', 'switch it on',
    'liga ele', 'ligue', 'ative', 'ativa ele', 'réactive', 'active-le', 'activer', 'allume', 'schalte es ein', 'schalt es an', 'schalt sie an', 'schalte sie ein', 'schalt ihn an', 'schalte ihn ein', 'aktivier', 'einschalten', 'включи', '启用', '开启'],
  note: ['anotalo', 'anota eso', 'note it', 'write it down', 'guarda eso',
    'anota isso', 'anote isso', 'note-le', 'note ça', 'notiere das', 'notier das', 'notier es', 'notiere es', 'schreib es auf', 'merk dir das', 'запиши это', 'запомни это', '记下来', '记一下'],
  show: ['mostramelo', 'mostrame', 'donde esta', 'ver en la lista', 'llevame', 'show it', 'show me', 'where is it', 'abrilo', 'open it',
    'mostra', 'mostre', 'onde está', 'me leva', 'montre', 'où est', 'emmène-moi', 'zeig', 'wo ist', 'bring mich', 'покажи', 'где он', 'открой его', '给我看', '在哪里', '显示它'],
  why: ['desde cuando', 'since when', 'por que', 'porque', 'why', 'cuando empezo',
    'desde quando', 'quando começou', 'depuis quand', 'pourquoi', 'seit wann', 'warum', 'wieso', 'с каких пор', 'почему', 'когда началось', '为什么', '从什么时候', '什么时候开始'],
  changed: ['que cambio', 'what changed', 'o que mudou', 'ce qui a changé', 'was hat sich geändert', 'что изменилось', '改变了什么', '变化'],
  errors: ['error', 'falla', 'problema', 'fail', 'erro', 'falha', 'erreur', 'problème', 'fehler', 'problem', 'ошибк', 'проблем', '错误', '问题'],
  history: ['historial', 'history', 'mas info', 'more', 'contame', 'tell me', 'histórico', 'mais', 'conta', 'historique', 'plus', 'raconte', 'verlauf', 'mehr', 'erzähl', 'истори', 'подробн', 'расскажи', '历史', '更多', '详细'],
};
/**
 * Follow-ups that are a whole message by themselves ("anota", "запиши"): as the start of a longer
 * text they would be another thing ("anotá lo aprendido" asks for the notes of what was learned).
 */
export const FOLLOW_ALONE = {
  disable: ['desativa', 'desliga', 'désactive', 'deaktivieren', 'выключи', 'отключи', '关掉'],
  enable: ['ativa', 'liga', 'active', 'aktivieren', 'включи', '开启'],
  note: ['anota', 'anote', 'note', 'notiere', 'notier', 'запиши', '记下'],
  show: ['mostra', 'montre', 'zeig', 'покажи', '显示'],
};
/** "it", "that one", an opening "and": the text leans on the last answer. */
export const LEANS = ['ese', 'este', 'eso', 'esa', 'esto', 'it', 'that', 'this', 'lo', 'esse', 'isso', 'ele', 'dele', 'ce', 'ça', 'celui', 'celui-ci', 'das', 'dieser', 'diese', 'den', 'ihn', 'он', 'это', 'этот', 'его', 'него', '它', '这个', '那个'];
export const LEAN_STARTS = ['y', 'and', 'e', 'et', 'und', 'и', '那', '还有'];
/** Words that make a question about everything, not the mod just talked about. */
export const GENERAL = ['mods', 'juego', 'game', 'partida', 'partidas', 'sessions', 'todo', 'everything', 'jogo', 'sessões', 'tudo', 'jeu', 'parties', 'tout', 'spiel', 'sitzungen', 'alles', 'моды', 'игра', 'игры', 'игре', 'всё', 'все', '游戏', '全部', '所有'];

/** Time windows. */
export const TIME = {
  today: ['hoy', 'today', 'hoje', "aujourd'hui", 'heute', 'сегодня', '今天'],
  yesterday: ['ayer', 'yesterday', 'ontem', 'depuis hier', 'hier soir', 'gestern', 'вчера', '昨天'],
  week: ['esta semana', 'this week', 'ultima semana', 'last week', '7 dias', '7 days', 'essa semana', 'semana passada', 'cette semaine', 'semaine dernière', '7 jours', 'diese woche', 'letzte woche', 'letzten woche', '7 tage', 'эту неделю', 'этой неделе', 'за неделю', 'прошлой неделе', '7 дней', '本周', '这周', '这个星期', '上周', '7天'],
  month: ['este mes', 'this month', 'ultimo mes', 'last month', '30 dias', '30 days', 'esse mês', 'mês passado', 'ce mois', 'mois dernier', '30 jours', 'diesen monat', 'letzten monat', '30 tage', 'этот месяц', 'этом месяце', 'за месяц', 'прошлом месяце', '30 дней', '本月', '这个月', '上个月', '30天'],
  lastDays: ['ultimos', 'last', 'últimos', 'derniers', 'letzten', 'последние', 'за последние'],
  days: ['dias', 'days', 'jours', 'tagen', 'tage', 'дней', 'дня', 'день'],
  since: ['desde el', 'desde', 'since', 'depuis le', 'depuis', 'seit dem', 'seit', 'с', '从', '自'],
};
/** The last session ("what about my last game?"). */
export const LAST_SESSION = ['ultima partida', 'last session', 'ultima vez', 'last time', 'ultima sesion', 'última sessão', 'dernière partie', 'dernière session', 'letzte sitzung', 'letzte runde', 'последняя сессия', 'последнюю сессию', 'последняя игра', 'последнюю игру', '上一局', '上次'];
/** "profile X". */
export const PROFILE_WORDS = ['perfil', 'profile', 'profil', 'профиль', '配置方案'];

/** "Which one is the most…": [most, slow, errors]. */
export const SUPERLATIVE = {
  most: ['cual', 'que mod', 'which', 'the most', 'el mas', 'la mas', 'mas', 'peor', 'worst', 'top', 'qual', 'o mais', 'a mais', 'mais', 'pior', 'quel', 'le plus', 'la plus', 'pire', 'welche', 'welcher', 'am meisten', 'am längsten', 'meisten', 'schlimmste', 'какой', 'какие', 'самый', 'больше всего', 'дольше всего', 'худш', '哪个', '最'],
  slow: ['tarda', 'lento', 'slow', 'carga', 'load', 'pesad', 'demora', 'carrega', 'lent', 'charge', 'temps', 'langsam', 'dauert', 'laden', 'längsten', 'медлен', 'долго', 'дольше', 'грузит', 'загруз', '慢', '加载', '最久', '耗时'],
  errors: ['error', 'falla', 'fail', 'problema', 'problem', 'erro', 'falha', 'erreur', 'problème', 'fehler', 'ошиб', 'проблем', '错误', '报错', '问题'],
};

/**
 * Orders on groups, one set of words per language (so one language's small words never cut
 * another's mod names): the verbs and the words around them. Matched on the text as typed (group
 * and mod names keep their case and accents); the words here match with or without accents.
 */
export interface OrderWords {
  add: string[];
  into: string[];
  remove: string[];
  from: string[];
  rename: string[];
  renameTo: string[];
  drop: string[];
  the: string[];
  group: string[];
  /** a word after the new name (German "benenne … in … um") */
  tail?: string[];
}
export const ORDER_WORDS: OrderWords[] = [
  { add: ['agrupa', 'agrupar', 'pon', 'pone', 'mete', 'meter', 'agrega', 'agregar'], into: ['en', 'a', 'al'], remove: ['saca', 'sacar', 'quita', 'quitar'], from: ['del', 'de'],
    rename: ['renombra', 'renombrar'], renameTo: ['a', 'como'], drop: ['borra', 'borrar', 'elimina', 'eliminar'], the: ['el', 'la'], group: ['grupo'] },
  { add: ['group', 'add', 'put'], into: ['in', 'into', 'to'], remove: ['remove', 'take'], from: ['out of', 'from'],
    rename: ['rename'], renameTo: ['to', 'as'], drop: ['delete', 'remove'], the: ['the'], group: ['group'] },
  { add: ['agrupe', 'agrupa', 'coloque', 'coloca', 'ponha', 'põe', 'adicione', 'adiciona', 'bota'], into: ['em', 'no', 'na', 'ao'], remove: ['tire', 'tira', 'remova', 'remove'], from: ['do', 'da', 'de'],
    rename: ['renomeie', 'renomeia', 'renomear'], renameTo: ['para', 'como'], drop: ['apague', 'apaga', 'exclua', 'exclui', 'remova'], the: ['o', 'a'], group: ['grupo'] },
  { add: ['groupe', 'grouper', 'mets', 'mettre', 'ajoute', 'ajouter', 'place'], into: ['dans', 'en', 'au', 'à'], remove: ['retire', 'retirer', 'enlève', 'enlever', 'sors'], from: ['de la', 'du', 'de', 'des'],
    rename: ['renomme', 'renommer'], renameTo: ['en'], drop: ['supprime', 'supprimer', 'efface'], the: ['le', 'la', 'les'], group: ['groupe'] },
  { add: ['gruppiere', 'pack', 'packe', 'leg', 'lege', 'füge', 'setz', 'setze', 'tu'], into: ['in', 'zu', 'zur', 'zum'], remove: ['nimm', 'entferne', 'entfern'], from: ['aus', 'von', 'vom'],
    rename: ['benenne', 'nenne'], renameTo: ['in', 'zu'], drop: ['lösche', 'lösch', 'entferne'], the: ['die', 'der', 'den'], group: ['gruppe'], tail: ['um'] },
  { add: ['добавь', 'положи', 'помести', 'перенеси', 'сгруппируй'], into: ['в', 'во'], remove: ['убери', 'вынь', 'исключи', 'удали'], from: ['из'],
    rename: ['переименуй'], renameTo: ['в'], drop: ['удали'], the: [], group: ['группу', 'группа', 'группы'] },
];
/** Asking for the list of groups. */
export const LIST_GROUPS = ['que grupos', 'lista de grupos', 'listar grupos', 'list groups', 'my groups', 'groups', 'grupos', 'mis grupos', 'meus grupos', 'quais grupos', 'mes groupes', 'quels groupes', 'groupes', 'meine gruppen', 'welche gruppen', 'gruppen', 'мои группы', 'какие группы', 'группы', '我的分组', '有哪些分组', '分组列表'];
/** Between the mods of a list: "A, B and C". */
export const LIST_AND = ['y', 'and', 'e', 'et', 'und', 'и'];
